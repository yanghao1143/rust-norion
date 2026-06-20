use crate::aggregate::AggregationConflictReviewTrendGateDecision;
use crate::budget::AgentBudget;
use crate::cycle::AgentCycleHandoff;
use crate::ports::{MemoryNote, MemoryPort};
use crate::reflection::ReflectionLoopHistoryGateDecision;
use crate::task::{AgentRole, AgentTask};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemorySubmissionReport {
    pub submitted: Vec<MemoryNote>,
    pub failures: Vec<MemorySubmissionFailure>,
    pub blocked_reasons: Vec<String>,
}

impl MemorySubmissionReport {
    pub fn is_clean(&self) -> bool {
        self.failures.is_empty() && self.blocked_reasons.is_empty()
    }

    pub fn summary(&self) -> MemorySubmissionSummary {
        MemorySubmissionSummary::from_report(self)
    }

    pub fn gate(&self) -> MemorySubmissionGateDecision {
        MemorySubmissionGateDecision::from_report(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemorySubmissionSummary {
    pub submitted_notes: usize,
    pub failed_notes: usize,
    pub blocked_reasons: usize,
    pub attempted_notes: usize,
    pub clean: bool,
    pub port_attempted: bool,
    pub telemetry: Vec<String>,
}

impl MemorySubmissionSummary {
    pub fn from_report(report: &MemorySubmissionReport) -> Self {
        let submitted_notes = report.submitted.len();
        let failed_notes = report.failures.len();
        let blocked_reasons = report.blocked_reasons.len();
        let attempted_notes = submitted_notes + failed_notes;
        let clean = report.is_clean();
        let port_attempted = attempted_notes > 0;
        let telemetry = memory_submission_summary_telemetry(
            submitted_notes,
            failed_notes,
            blocked_reasons,
            attempted_notes,
            clean,
            port_attempted,
        );

        Self {
            submitted_notes,
            failed_notes,
            blocked_reasons,
            attempted_notes,
            clean,
            port_attempted,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemorySubmissionGateDecision {
    pub summary: MemorySubmissionSummary,
    pub can_continue_loop: bool,
    pub can_commit_submitted_notes: bool,
    pub requires_repair_first: bool,
    pub reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemorySubmissionHealthStatus {
    Stable,
    Watch,
    Repair,
}

impl MemorySubmissionHealthStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Watch => "watch",
            Self::Repair => "repair",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MemorySubmissionSummaryHistory {
    summaries: Vec<MemorySubmissionSummary>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemorySubmissionDashboard {
    pub total_records: usize,
    pub clean_records: usize,
    pub repair_first_records: usize,
    pub submitted_notes: usize,
    pub failed_notes: usize,
    pub blocked_reasons: usize,
    pub attempted_notes: usize,
    pub port_attempted_records: usize,
    pub no_note_records: usize,
    pub clean_rate: f32,
    pub port_attempt_rate: f32,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MemorySubmissionHealthPolicy {
    pub minimum_clean_rate: f32,
    pub minimum_port_attempt_rate: f32,
    pub maximum_failed_notes: usize,
    pub maximum_blocked_reasons: usize,
    pub maximum_no_note_records: usize,
}

impl Default for MemorySubmissionHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_clean_rate: 0.67,
            minimum_port_attempt_rate: 0.0,
            maximum_failed_notes: 0,
            maximum_blocked_reasons: 0,
            maximum_no_note_records: usize::MAX,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemorySubmissionHealth {
    pub status: MemorySubmissionHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: MemorySubmissionDashboard,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemorySubmissionSummaryHistoryRecord {
    pub history: MemorySubmissionSummaryHistory,
    pub appended_summary: MemorySubmissionSummary,
    pub dashboard: MemorySubmissionDashboard,
    pub health: MemorySubmissionHealth,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct MemorySubmissionSummaryHistoryRecorder;

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryPromotionGateDecision {
    pub candidate_notes: usize,
    pub reflection_gate: ReflectionLoopHistoryGateDecision,
    pub aggregation_conflict_gate: AggregationConflictReviewTrendGateDecision,
    pub memory_health: MemorySubmissionHealth,
    pub can_promote_memory_note: bool,
    pub can_submit_memory: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl MemoryPromotionGateDecision {
    pub fn is_memory_promotable(&self) -> bool {
        self.can_promote_memory_note && !self.requires_repair_first
    }
}

#[derive(Debug, Clone, Default)]
pub struct MemoryPromotionGate;

impl MemorySubmissionGateDecision {
    pub fn from_report(report: &MemorySubmissionReport) -> Self {
        let summary = report.summary();
        let mut reasons = Vec::new();

        reasons.extend(
            report
                .blocked_reasons
                .iter()
                .map(|reason| format!("memory_handoff_blocked reason={reason}")),
        );
        reasons.extend(report.failures.iter().map(|failure| {
            format!(
                "memory_submission_failed topic={} reason={}",
                failure.note.topic, failure.reason
            )
        }));

        let can_continue_loop = reasons.is_empty();
        let can_commit_submitted_notes = can_continue_loop && summary.submitted_notes > 0;
        let requires_repair_first = !can_continue_loop;
        let telemetry = memory_submission_gate_telemetry(
            can_continue_loop,
            can_commit_submitted_notes,
            requires_repair_first,
            reasons.len(),
            &summary,
        );

        Self {
            summary,
            can_continue_loop,
            can_commit_submitted_notes,
            requires_repair_first,
            reasons,
            telemetry,
        }
    }
}

impl MemorySubmissionSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(summaries: Vec<MemorySubmissionSummary>) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: MemorySubmissionSummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&MemorySubmissionSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[MemorySubmissionSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> MemorySubmissionDashboard {
        MemorySubmissionDashboard::from_summaries(&self.summaries)
    }

    pub fn health(&self, policy: MemorySubmissionHealthPolicy) -> MemorySubmissionHealth {
        self.dashboard().health(policy)
    }
}

impl MemorySubmissionDashboard {
    pub fn from_summaries(summaries: &[MemorySubmissionSummary]) -> Self {
        let total_records = summaries.len();
        let clean_records = summaries.iter().filter(|summary| summary.clean).count();
        let repair_first_records = summaries.iter().filter(|summary| !summary.clean).count();
        let submitted_notes = summaries
            .iter()
            .map(|summary| summary.submitted_notes)
            .sum::<usize>();
        let failed_notes = summaries
            .iter()
            .map(|summary| summary.failed_notes)
            .sum::<usize>();
        let blocked_reasons = summaries
            .iter()
            .map(|summary| summary.blocked_reasons)
            .sum::<usize>();
        let attempted_notes = summaries
            .iter()
            .map(|summary| summary.attempted_notes)
            .sum::<usize>();
        let port_attempted_records = summaries
            .iter()
            .filter(|summary| summary.port_attempted)
            .count();
        let no_note_records = summaries
            .iter()
            .filter(|summary| summary.submitted_notes == 0 && summary.attempted_notes == 0)
            .count();
        let clean_rate = rate(clean_records, total_records);
        let port_attempt_rate = rate(port_attempted_records, total_records);
        let telemetry = memory_submission_dashboard_telemetry(
            total_records,
            clean_records,
            repair_first_records,
            submitted_notes,
            failed_notes,
            blocked_reasons,
            attempted_notes,
            port_attempted_records,
            no_note_records,
            clean_rate,
            port_attempt_rate,
        );

        Self {
            total_records,
            clean_records,
            repair_first_records,
            submitted_notes,
            failed_notes,
            blocked_reasons,
            attempted_notes,
            port_attempted_records,
            no_note_records,
            clean_rate,
            port_attempt_rate,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    pub fn health(&self, policy: MemorySubmissionHealthPolicy) -> MemorySubmissionHealth {
        MemorySubmissionHealth::from_dashboard(self.clone(), policy)
    }
}

impl MemorySubmissionHealth {
    pub fn from_dashboard(
        dashboard: MemorySubmissionDashboard,
        policy: MemorySubmissionHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("memory_submission_history_empty".to_owned());
        } else if dashboard.clean_rate < policy.minimum_clean_rate {
            watch_reasons.push(format!(
                "memory_submission_clean_rate={:.3}<{}",
                dashboard.clean_rate, policy.minimum_clean_rate
            ));
        }

        if !dashboard.is_empty() && dashboard.port_attempt_rate < policy.minimum_port_attempt_rate {
            watch_reasons.push(format!(
                "memory_submission_port_attempt_rate={:.3}<{}",
                dashboard.port_attempt_rate, policy.minimum_port_attempt_rate
            ));
        }

        if dashboard.failed_notes > policy.maximum_failed_notes {
            repair_reasons.push(format!(
                "memory_submission_failed_notes={}>{}",
                dashboard.failed_notes, policy.maximum_failed_notes
            ));
        }

        if dashboard.blocked_reasons > policy.maximum_blocked_reasons {
            repair_reasons.push(format!(
                "memory_submission_blocked_reasons={}>{}",
                dashboard.blocked_reasons, policy.maximum_blocked_reasons
            ));
        }

        if dashboard.no_note_records > policy.maximum_no_note_records {
            watch_reasons.push(format!(
                "memory_submission_no_note_records={}>{}",
                dashboard.no_note_records, policy.maximum_no_note_records
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (MemorySubmissionHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (MemorySubmissionHealthStatus::Watch, watch_reasons)
        } else {
            (MemorySubmissionHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == MemorySubmissionHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != MemorySubmissionHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == MemorySubmissionHealthStatus::Repair
    }
}

impl MemorySubmissionSummaryHistoryRecord {
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

impl MemorySubmissionSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: MemorySubmissionSummaryHistory,
        summary: MemorySubmissionSummary,
        policy: MemorySubmissionHealthPolicy,
    ) -> MemorySubmissionSummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = memory_submission_history_record_telemetry(&dashboard, &health);

        MemorySubmissionSummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_report_with_health(
        &self,
        history: MemorySubmissionSummaryHistory,
        report: &MemorySubmissionReport,
        policy: MemorySubmissionHealthPolicy,
    ) -> MemorySubmissionSummaryHistoryRecord {
        self.record_summary_with_health(history, report.summary(), policy)
    }
}

impl MemoryPromotionGate {
    pub fn new() -> Self {
        Self
    }

    pub fn gate(
        &self,
        candidate_notes: &[MemoryNote],
        reflection_gate: &ReflectionLoopHistoryGateDecision,
        aggregation_conflict_gate: &AggregationConflictReviewTrendGateDecision,
        memory_health: &MemorySubmissionHealth,
    ) -> MemoryPromotionGateDecision {
        let mut reasons = Vec::new();

        if candidate_notes.is_empty() {
            reasons.push("memory_promotion_no_candidate_notes".to_owned());
        }

        if !reflection_gate.is_memory_promotable() {
            extend_memory_ordered_unique(
                &mut reasons,
                prefixed_or_default(
                    "memory_promotion_reflection",
                    &reflection_gate.reasons,
                    "memory_promotion_reflection_not_promotable",
                ),
            );
        }

        if !aggregation_conflict_gate.is_side_effect_safe() {
            extend_memory_ordered_unique(
                &mut reasons,
                prefixed_or_default(
                    "memory_promotion_aggregation_conflict",
                    &aggregation_conflict_gate.reasons,
                    "memory_promotion_aggregation_conflict_side_effect_closed",
                ),
            );
        }

        if !memory_health.is_stable() {
            extend_memory_ordered_unique(
                &mut reasons,
                prefixed_or_default(
                    "memory_promotion_submission_history",
                    &memory_health.reasons,
                    "memory_promotion_submission_history_not_stable",
                ),
            );
        }

        let requires_repair_first = reflection_gate.requires_repair_first
            || aggregation_conflict_gate.requires_repair_first
            || memory_health.requires_repair_first();
        let can_promote_memory_note = !candidate_notes.is_empty()
            && reflection_gate.is_memory_promotable()
            && aggregation_conflict_gate.is_side_effect_safe()
            && memory_health.is_stable()
            && !requires_repair_first;
        let can_submit_memory = can_promote_memory_note;
        let mut repair_tasks = Vec::new();
        if reflection_gate.requires_repair_first {
            repair_tasks.extend(reflection_gate.repair_tasks.clone());
        }
        if aggregation_conflict_gate.requires_repair_first {
            repair_tasks.extend(aggregation_conflict_gate.repair_tasks.clone());
        }
        repair_tasks.extend(memory_promotion_gate_repair_tasks(
            memory_health.requires_repair_first(),
            &reasons,
        ));
        let telemetry = memory_promotion_gate_telemetry(
            candidate_notes.len(),
            can_promote_memory_note,
            can_submit_memory,
            requires_repair_first,
            repair_tasks.len(),
            reasons.len(),
            memory_health.status,
        );

        MemoryPromotionGateDecision {
            candidate_notes: candidate_notes.len(),
            reflection_gate: reflection_gate.clone(),
            aggregation_conflict_gate: aggregation_conflict_gate.clone(),
            memory_health: memory_health.clone(),
            can_promote_memory_note,
            can_submit_memory,
            requires_repair_first,
            repair_tasks,
            reasons,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemorySubmissionFailure {
    pub note: MemoryNote,
    pub reason: String,
}

#[derive(Debug, Clone, Default)]
pub struct MemoryHandoffSubmitter;

impl MemoryHandoffSubmitter {
    pub fn new() -> Self {
        Self
    }

    pub fn submit<P>(&self, handoff: &AgentCycleHandoff, memory: &mut P) -> MemorySubmissionReport
    where
        P: MemoryPort,
        P::Error: ToString,
    {
        if !handoff.blocked_reasons.is_empty() {
            return MemorySubmissionReport {
                submitted: Vec::new(),
                failures: Vec::new(),
                blocked_reasons: handoff.blocked_reasons.clone(),
            };
        }

        let mut submitted = Vec::new();
        let mut failures = Vec::new();
        for note in &handoff.memory_notes {
            match memory.propose_note(note.clone()) {
                Ok(()) => submitted.push(note.clone()),
                Err(error) => failures.push(MemorySubmissionFailure {
                    note: note.clone(),
                    reason: error.to_string(),
                }),
            }
        }

        MemorySubmissionReport {
            submitted,
            failures,
            blocked_reasons: Vec::new(),
        }
    }
}

fn memory_submission_summary_telemetry(
    submitted_notes: usize,
    failed_notes: usize,
    blocked_reasons: usize,
    attempted_notes: usize,
    clean: bool,
    port_attempted: bool,
) -> Vec<String> {
    vec![
        "agent_memory_submission_summary=true".to_owned(),
        format!("agent_memory_submission_summary_submitted_notes={submitted_notes}"),
        format!("agent_memory_submission_summary_failed_notes={failed_notes}"),
        format!("agent_memory_submission_summary_blocked_reasons={blocked_reasons}"),
        format!("agent_memory_submission_summary_attempted_notes={attempted_notes}"),
        format!("agent_memory_submission_summary_clean={clean}"),
        format!("agent_memory_submission_summary_port_attempted={port_attempted}"),
    ]
}

fn memory_submission_gate_telemetry(
    can_continue_loop: bool,
    can_commit_submitted_notes: bool,
    requires_repair_first: bool,
    reasons: usize,
    summary: &MemorySubmissionSummary,
) -> Vec<String> {
    vec![
        "agent_memory_submission_gate=true".to_owned(),
        format!("agent_memory_submission_gate_continue={can_continue_loop}"),
        format!("agent_memory_submission_gate_commit_notes={can_commit_submitted_notes}"),
        format!("agent_memory_submission_gate_repair_first={requires_repair_first}"),
        format!("agent_memory_submission_gate_reasons={reasons}"),
        format!(
            "agent_memory_submission_gate_submitted_notes={}",
            summary.submitted_notes
        ),
        format!(
            "agent_memory_submission_gate_failed_notes={}",
            summary.failed_notes
        ),
        format!(
            "agent_memory_submission_gate_blocked_reasons={}",
            summary.blocked_reasons
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn memory_submission_dashboard_telemetry(
    total_records: usize,
    clean_records: usize,
    repair_first_records: usize,
    submitted_notes: usize,
    failed_notes: usize,
    blocked_reasons: usize,
    attempted_notes: usize,
    port_attempted_records: usize,
    no_note_records: usize,
    clean_rate: f32,
    port_attempt_rate: f32,
) -> Vec<String> {
    vec![
        "agent_memory_submission_dashboard=true".to_owned(),
        format!("agent_memory_submission_dashboard_records={total_records}"),
        format!("agent_memory_submission_dashboard_clean={clean_records}"),
        format!("agent_memory_submission_dashboard_repair_first={repair_first_records}"),
        format!("agent_memory_submission_dashboard_submitted_notes={submitted_notes}"),
        format!("agent_memory_submission_dashboard_failed_notes={failed_notes}"),
        format!("agent_memory_submission_dashboard_blocked_reasons={blocked_reasons}"),
        format!("agent_memory_submission_dashboard_attempted_notes={attempted_notes}"),
        format!("agent_memory_submission_dashboard_port_attempted={port_attempted_records}"),
        format!("agent_memory_submission_dashboard_no_note={no_note_records}"),
        format!("agent_memory_submission_dashboard_clean_rate={clean_rate:.3}"),
        format!("agent_memory_submission_dashboard_port_attempt_rate={port_attempt_rate:.3}"),
    ]
}

fn memory_submission_history_record_telemetry(
    dashboard: &MemorySubmissionDashboard,
    health: &MemorySubmissionHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_memory_submission_history_record=true".to_owned(),
        format!(
            "agent_memory_submission_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_memory_submission_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_memory_submission_history_record_clean_rate={:.3}",
            dashboard.clean_rate
        ),
        format!(
            "agent_memory_submission_history_record_failed_notes={}",
            dashboard.failed_notes
        ),
        format!(
            "agent_memory_submission_history_record_blocked_reasons={}",
            dashboard.blocked_reasons
        ),
    ];
    telemetry.extend(
        health
            .reasons
            .iter()
            .map(|reason| format!("agent_memory_submission_history_record_reason={reason}")),
    );
    telemetry
}

fn prefixed_or_default(prefix: &str, reasons: &[String], default_reason: &str) -> Vec<String> {
    if reasons.is_empty() {
        return vec![default_reason.to_owned()];
    }

    reasons
        .iter()
        .map(|reason| format!("{prefix}:{reason}"))
        .collect()
}

fn extend_memory_ordered_unique(target: &mut Vec<String>, items: Vec<String>) {
    for item in items {
        if !target.contains(&item) {
            target.push(item);
        }
    }
}

fn memory_promotion_gate_repair_tasks(
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
                format!("memory-promotion-repair-{index}"),
                AgentRole::MemoryCurator,
                format!("repair memory promotion gate: {reason}"),
                AgentBudget::new(16, 1, 1),
            )
            .with_lane("memory-promotion-repair")
            .with_priority(1)
        })
        .collect()
}

fn memory_promotion_gate_telemetry(
    candidate_notes: usize,
    can_promote_memory_note: bool,
    can_submit_memory: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    reasons: usize,
    memory_health_status: MemorySubmissionHealthStatus,
) -> Vec<String> {
    vec![
        "agent_memory_promotion_gate=true".to_owned(),
        format!("agent_memory_promotion_gate_candidate_notes={candidate_notes}"),
        format!("agent_memory_promotion_gate_promote={can_promote_memory_note}"),
        format!("agent_memory_promotion_gate_submit={can_submit_memory}"),
        format!("agent_memory_promotion_gate_requires_repair_first={requires_repair_first}"),
        format!("agent_memory_promotion_gate_repair_tasks={repair_tasks}"),
        format!("agent_memory_promotion_gate_reasons={reasons}"),
        format!(
            "agent_memory_promotion_gate_memory_health={}",
            memory_health_status.as_str()
        ),
    ]
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
    use crate::aggregate::{
        AggregationConflictReviewHealthPolicy, AggregationConflictReviewSummaryHistory,
        AggregationConflictReviewSummaryHistoryRecorder, AggregationConflictReviewTrendGate,
        AggregationConflictReviewer, AggregationHealthPolicy, AggregationSummaryHistory,
    };
    use crate::conflict::{ConflictReportHealthPolicy, ConflictReportSummaryHistory};
    use crate::message::{AgentMessage, AgentMessageKind};
    use crate::reflection::{
        ReflectionLoop, ReflectionLoopHealthPolicy, ReflectionLoopSummaryHistory,
        ReflectionLoopSummaryHistoryRecorder, ReflectionStage,
    };
    use crate::task::{AgentRole, AgentTask};

    #[derive(Debug, Default)]
    struct FakeMemoryPort {
        fail_topic: Option<String>,
        submitted: Vec<MemoryNote>,
    }

    impl MemoryPort for FakeMemoryPort {
        type Error = String;

        fn recall(
            &self,
            _query: &str,
            _limit: usize,
        ) -> Result<Vec<crate::ports::MemoryRecord>, Self::Error> {
            Ok(Vec::new())
        }

        fn propose_note(&mut self, note: MemoryNote) -> Result<(), Self::Error> {
            if self.fail_topic.as_deref() == Some(note.topic.as_str()) {
                return Err(format!("memory rejected {}", note.topic));
            }
            self.submitted.push(note);
            Ok(())
        }
    }

    #[test]
    fn submitter_writes_clean_handoff_notes_through_memory_port() {
        let handoff = AgentCycleHandoff {
            memory_notes: vec![MemoryNote::new("agent_cycle", "remember clean loop")],
            follow_up_tasks: Vec::new(),
            blocked_reasons: Vec::new(),
        };
        let mut memory = FakeMemoryPort::default();

        let report = MemoryHandoffSubmitter::new().submit(&handoff, &mut memory);

        assert!(report.is_clean());
        assert_eq!(report.submitted.len(), 1);
        assert_eq!(memory.submitted.len(), 1);
        assert_eq!(memory.submitted[0].content, "remember clean loop");

        let summary = report.summary();
        assert_eq!(summary.submitted_notes, 1);
        assert_eq!(summary.failed_notes, 0);
        assert_eq!(summary.attempted_notes, 1);
        assert!(summary.clean);
        assert!(summary.port_attempted);

        let gate = report.gate();
        assert!(gate.can_continue_loop);
        assert!(gate.can_commit_submitted_notes);
        assert!(!gate.requires_repair_first);
        assert!(gate.reasons.is_empty());
    }

    #[test]
    fn submitter_does_not_call_memory_port_when_handoff_is_blocked() {
        let handoff = AgentCycleHandoff {
            memory_notes: vec![MemoryNote::new("agent_cycle", "remember blocked loop")],
            follow_up_tasks: vec![AgentTask::new(
                "repair",
                AgentRole::Reviewer,
                "repair loop",
                crate::budget::AgentBudget::new(1, 1, 1),
            )],
            blocked_reasons: vec!["unresolved_conflicts=1".to_owned()],
        };
        let mut memory = FakeMemoryPort::default();

        let report = MemoryHandoffSubmitter::new().submit(&handoff, &mut memory);

        assert!(!report.is_clean());
        assert!(report.submitted.is_empty());
        assert!(memory.submitted.is_empty());
        assert_eq!(report.blocked_reasons, vec!["unresolved_conflicts=1"]);

        let summary = report.summary();
        assert_eq!(summary.submitted_notes, 0);
        assert_eq!(summary.failed_notes, 0);
        assert_eq!(summary.blocked_reasons, 1);
        assert_eq!(summary.attempted_notes, 0);
        assert!(!summary.clean);
        assert!(!summary.port_attempted);

        let gate = report.gate();
        assert!(!gate.can_continue_loop);
        assert!(!gate.can_commit_submitted_notes);
        assert!(gate.requires_repair_first);
        assert_eq!(
            gate.reasons,
            vec!["memory_handoff_blocked reason=unresolved_conflicts=1"]
        );
    }

    #[test]
    fn submitter_records_memory_port_failures_as_data() {
        let handoff = AgentCycleHandoff {
            memory_notes: vec![MemoryNote::new("agent_cycle", "remember clean loop")],
            follow_up_tasks: Vec::new(),
            blocked_reasons: Vec::new(),
        };
        let mut memory = FakeMemoryPort {
            fail_topic: Some("agent_cycle".to_owned()),
            submitted: Vec::new(),
        };

        let report = MemoryHandoffSubmitter::new().submit(&handoff, &mut memory);

        assert!(!report.is_clean());
        assert!(report.submitted.is_empty());
        assert_eq!(report.failures.len(), 1);
        assert_eq!(report.failures[0].reason, "memory rejected agent_cycle");

        let summary = report.summary();
        assert_eq!(summary.submitted_notes, 0);
        assert_eq!(summary.failed_notes, 1);
        assert_eq!(summary.blocked_reasons, 0);
        assert_eq!(summary.attempted_notes, 1);
        assert!(!summary.clean);
        assert!(summary.port_attempted);

        let gate = report.gate();
        assert!(!gate.can_continue_loop);
        assert!(!gate.can_commit_submitted_notes);
        assert!(gate.requires_repair_first);
        assert_eq!(
            gate.reasons,
            vec!["memory_submission_failed topic=agent_cycle reason=memory rejected agent_cycle"]
        );
    }

    #[test]
    fn memory_submission_history_watches_empty() {
        let health =
            MemorySubmissionSummaryHistory::new().health(MemorySubmissionHealthPolicy::default());

        assert_eq!(health.status, MemorySubmissionHealthStatus::Watch);
        assert_eq!(
            health.reasons,
            vec!["memory_submission_history_empty".to_owned()]
        );
        assert_eq!(health.dashboard.total_records, 0);
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert!(
            health
                .dashboard
                .telemetry
                .iter()
                .any(|line| { line == "agent_memory_submission_dashboard_records=0" })
        );
    }

    #[test]
    fn memory_submission_history_marks_clean_submission_stable() {
        let report = MemorySubmissionReport {
            submitted: vec![MemoryNote::new("agent_cycle", "remember clean loop")],
            failures: Vec::new(),
            blocked_reasons: Vec::new(),
        };

        let record = MemorySubmissionSummaryHistoryRecorder::new().record_report_with_health(
            MemorySubmissionSummaryHistory::new(),
            &report,
            MemorySubmissionHealthPolicy::default(),
        );

        assert_eq!(record.history.len(), 1);
        assert_eq!(record.records(), 1);
        assert!(record.appended_summary.clean);
        assert_eq!(record.dashboard.clean_records, 1);
        assert_eq!(record.dashboard.repair_first_records, 0);
        assert_eq!(record.dashboard.submitted_notes, 1);
        assert_eq!(record.dashboard.failed_notes, 0);
        assert_eq!(record.dashboard.blocked_reasons, 0);
        assert_eq!(record.dashboard.clean_rate, 1.0);
        assert_eq!(record.health.status, MemorySubmissionHealthStatus::Stable);
        assert!(record.health.is_stable());
        assert!(record.health.allows_service_advance());
        assert!(!record.health.requires_repair_first());
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| { line == "agent_memory_submission_history_record_status=stable" })
        );
    }

    #[test]
    fn memory_submission_history_repairs_failures_and_blockers() {
        let clean_summary = MemorySubmissionSummary {
            submitted_notes: 1,
            failed_notes: 0,
            blocked_reasons: 0,
            attempted_notes: 1,
            clean: true,
            port_attempted: true,
            telemetry: Vec::new(),
        };
        let dirty_summary = MemorySubmissionSummary {
            submitted_notes: 0,
            failed_notes: 1,
            blocked_reasons: 1,
            attempted_notes: 1,
            clean: false,
            port_attempted: true,
            telemetry: Vec::new(),
        };
        let history = MemorySubmissionSummaryHistory::from_summaries(vec![clean_summary]);

        let record = MemorySubmissionSummaryHistoryRecorder::new().record_summary_with_health(
            history,
            dirty_summary,
            MemorySubmissionHealthPolicy::default(),
        );

        assert_eq!(record.history.len(), 2);
        assert_eq!(record.dashboard.clean_records, 1);
        assert_eq!(record.dashboard.repair_first_records, 1);
        assert_eq!(record.dashboard.failed_notes, 1);
        assert_eq!(record.dashboard.blocked_reasons, 1);
        assert_eq!(record.dashboard.clean_rate, 0.5);
        assert_eq!(record.health.status, MemorySubmissionHealthStatus::Repair);
        assert!(!record.health.allows_service_advance());
        assert!(record.health.requires_repair_first());
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert_eq!(
            record.health.reasons,
            vec![
                "memory_submission_failed_notes=1>0",
                "memory_submission_blocked_reasons=1>0",
                "memory_submission_clean_rate=0.500<0.67",
            ]
        );
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| { line == "agent_memory_submission_history_record_status=repair" })
        );
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
            .submit(ReflectionStage::Revision, "keep memory evidence")
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

    fn review_trend_gate(
        messages: Vec<AgentMessage>,
    ) -> AggregationConflictReviewTrendGateDecision {
        let review = AggregationConflictReviewer::new().review_messages(
            messages,
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

    #[test]
    fn memory_promotion_gate_promotes_stable_reflection_review_and_memory_history() {
        let reflection_gate = stable_reflection_gate();
        let review_gate = review_trend_gate(vec![
            AgentMessage::new(
                "m1",
                AgentRole::Researcher,
                AgentMessageKind::Finding,
                "memory",
                "remember the clean handoff",
            ),
            AgentMessage::new(
                "m2",
                AgentRole::Reviewer,
                AgentMessageKind::Finding,
                "budget",
                "budget remained isolated",
            ),
        ]);
        let memory_health = stable_memory_submission_health();
        let notes = vec![MemoryNote::new("agent_cycle", "remember clean handoff")];

        let gate =
            MemoryPromotionGate::new().gate(&notes, &reflection_gate, &review_gate, &memory_health);

        assert_eq!(gate.candidate_notes, 1);
        assert!(gate.can_promote_memory_note);
        assert!(gate.can_submit_memory);
        assert!(gate.is_memory_promotable());
        assert!(!gate.requires_repair_first);
        assert!(gate.repair_tasks.is_empty());
        assert!(gate.reasons.is_empty());
        assert!(
            gate.telemetry
                .iter()
                .any(|line| line == "agent_memory_promotion_gate_promote=true")
        );
    }

    #[test]
    fn memory_promotion_gate_blocks_unresolved_conflict_before_memory_note() {
        let reflection_gate = stable_reflection_gate();
        let review_gate = review_trend_gate(vec![
            AgentMessage::new(
                "approve",
                AgentRole::Coder,
                AgentMessageKind::Decision,
                "memory",
                "approve memory note and proceed",
            ),
            AgentMessage::new(
                "block",
                AgentRole::Reviewer,
                AgentMessageKind::Risk,
                "memory",
                "reject memory note until validation passes",
            ),
        ]);
        let memory_health = stable_memory_submission_health();
        let notes = vec![MemoryNote::new("agent_cycle", "remember contested note")];

        let gate =
            MemoryPromotionGate::new().gate(&notes, &reflection_gate, &review_gate, &memory_health);

        assert!(!gate.can_promote_memory_note);
        assert!(!gate.can_submit_memory);
        assert!(!gate.is_memory_promotable());
        assert!(gate.requires_repair_first);
        assert!(!gate.repair_tasks.is_empty());
        assert!(gate.reasons.iter().any(|reason| {
            reason
                == "memory_promotion_aggregation_conflict:conflict_report:conflict_report_unresolved_conflicts=1"
        }));
        assert!(
            gate.telemetry
                .iter()
                .any(|line| { line == "agent_memory_promotion_gate_requires_repair_first=true" })
        );
    }

    #[test]
    fn memory_promotion_gate_blocks_empty_candidate_notes_without_repair_first() {
        let reflection_gate = stable_reflection_gate();
        let review_gate = review_trend_gate(vec![AgentMessage::new(
            "m1",
            AgentRole::Researcher,
            AgentMessageKind::Finding,
            "memory",
            "clean handoff had no durable lesson",
        )]);
        let memory_health = stable_memory_submission_health();

        let gate =
            MemoryPromotionGate::new().gate(&[], &reflection_gate, &review_gate, &memory_health);

        assert_eq!(gate.candidate_notes, 0);
        assert!(!gate.can_promote_memory_note);
        assert!(!gate.can_submit_memory);
        assert!(!gate.is_memory_promotable());
        assert!(!gate.requires_repair_first);
        assert!(gate.repair_tasks.is_empty());
        assert_eq!(gate.reasons, vec!["memory_promotion_no_candidate_notes"]);
        assert!(
            gate.telemetry
                .iter()
                .any(|line| line == "agent_memory_promotion_gate_candidate_notes=0")
        );
    }

    #[test]
    fn memory_promotion_gate_keeps_watch_history_observable_but_not_promotable() {
        let reflection_gate = stable_reflection_gate();
        let review_gate = review_trend_gate(vec![AgentMessage::new(
            "m1",
            AgentRole::Researcher,
            AgentMessageKind::Finding,
            "memory",
            "remember clean handoff",
        )]);
        let memory_health =
            MemorySubmissionSummaryHistory::new().health(MemorySubmissionHealthPolicy::default());
        let notes = vec![MemoryNote::new("agent_cycle", "remember clean handoff")];

        let gate =
            MemoryPromotionGate::new().gate(&notes, &reflection_gate, &review_gate, &memory_health);

        assert!(!gate.can_promote_memory_note);
        assert!(!gate.can_submit_memory);
        assert!(!gate.is_memory_promotable());
        assert!(!gate.requires_repair_first);
        assert!(gate.repair_tasks.is_empty());
        assert_eq!(
            gate.reasons,
            vec!["memory_promotion_submission_history:memory_submission_history_empty"]
        );
        assert!(
            gate.telemetry
                .iter()
                .any(|line| line == "agent_memory_promotion_gate_memory_health=watch")
        );
    }

    #[test]
    fn memory_promotion_gate_repairs_dirty_submission_history_before_clean_candidate() {
        let reflection_gate = stable_reflection_gate();
        let review_gate = review_trend_gate(vec![AgentMessage::new(
            "m1",
            AgentRole::Researcher,
            AgentMessageKind::Finding,
            "memory",
            "remember clean handoff after repair",
        )]);
        let dirty_submission = MemorySubmissionReport {
            submitted: Vec::new(),
            failures: Vec::new(),
            blocked_reasons: vec!["unresolved_conflicts=1".to_owned()],
        };
        let memory_health = MemorySubmissionSummaryHistoryRecorder::new()
            .record_report_with_health(
                MemorySubmissionSummaryHistory::new(),
                &dirty_submission,
                MemorySubmissionHealthPolicy::default(),
            )
            .health;
        let notes = vec![MemoryNote::new(
            "agent_cycle",
            "remember clean handoff after repair",
        )];

        let gate =
            MemoryPromotionGate::new().gate(&notes, &reflection_gate, &review_gate, &memory_health);

        assert_eq!(gate.candidate_notes, 1);
        assert_eq!(
            gate.memory_health.status,
            MemorySubmissionHealthStatus::Repair
        );
        assert!(!gate.can_promote_memory_note);
        assert!(!gate.can_submit_memory);
        assert!(!gate.is_memory_promotable());
        assert!(gate.requires_repair_first);
        assert_eq!(
            gate.repair_tasks
                .iter()
                .map(|task| task.id.clone())
                .collect::<Vec<_>>(),
            vec![
                "memory-promotion-repair-0".to_owned(),
                "memory-promotion-repair-1".to_owned()
            ]
        );
        assert_eq!(
            gate.reasons,
            vec![
                "memory_promotion_submission_history:memory_submission_blocked_reasons=1>0",
                "memory_promotion_submission_history:memory_submission_clean_rate=0.000<0.67"
            ]
        );
        assert!(
            gate.telemetry
                .iter()
                .any(|line| { line == "agent_memory_promotion_gate_requires_repair_first=true" })
        );
    }
}
