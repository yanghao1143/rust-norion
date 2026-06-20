use crate::budget::AgentBudget;
use crate::task::{AgentRole, AgentTask};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReflectionStage {
    Draft,
    Critique,
    Revision,
    MemoryNote,
    Done,
}

impl ReflectionStage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Critique => "critique",
            Self::Revision => "revision",
            Self::MemoryNote => "memory_note",
            Self::Done => "done",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReflectionEntry {
    pub stage: ReflectionStage,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReflectionError {
    WrongStage {
        expected: ReflectionStage,
        actual: ReflectionStage,
    },
    EmptyContent,
    AlreadyComplete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReflectionLoop {
    next_stage: ReflectionStage,
    entries: Vec<ReflectionEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReflectionLoopSummary {
    pub entries: usize,
    pub next_stage: ReflectionStage,
    pub is_complete: bool,
    pub memory_note_ready: bool,
    pub remaining_stages: Vec<ReflectionStage>,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReflectionLoopGateDecision {
    pub summary: ReflectionLoopSummary,
    pub can_continue_reflection: bool,
    pub can_promote_memory_note: bool,
    pub reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReflectionLoopHealthStatus {
    Stable,
    Watch,
    Repair,
}

impl ReflectionLoopHealthStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Watch => "watch",
            Self::Repair => "repair",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReflectionLoopSummaryHistory {
    summaries: Vec<ReflectionLoopSummary>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReflectionLoopDashboard {
    pub total_records: usize,
    pub complete_records: usize,
    pub incomplete_records: usize,
    pub memory_note_ready_records: usize,
    pub missing_memory_note_records: usize,
    pub stalled_stage_records: usize,
    pub completion_rate: f32,
    pub memory_note_ready_rate: f32,
    pub latest_next_stage: Option<ReflectionStage>,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReflectionLoopHealthPolicy {
    pub minimum_completion_rate: f32,
    pub minimum_memory_note_ready_rate: f32,
    pub maximum_incomplete_records: usize,
    pub maximum_missing_memory_note_records: usize,
    pub maximum_stalled_stage_records: usize,
}

impl Default for ReflectionLoopHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_completion_rate: 0.67,
            minimum_memory_note_ready_rate: 0.67,
            maximum_incomplete_records: 1,
            maximum_missing_memory_note_records: 0,
            maximum_stalled_stage_records: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReflectionLoopHealth {
    pub status: ReflectionLoopHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: ReflectionLoopDashboard,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReflectionLoopSummaryHistoryRecord {
    pub history: ReflectionLoopSummaryHistory,
    pub appended_summary: ReflectionLoopSummary,
    pub dashboard: ReflectionLoopDashboard,
    pub health: ReflectionLoopHealth,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ReflectionLoopSummaryHistoryRecorder;

impl Default for ReflectionLoop {
    fn default() -> Self {
        Self::new()
    }
}

impl ReflectionLoop {
    pub fn new() -> Self {
        Self {
            next_stage: ReflectionStage::Draft,
            entries: Vec::new(),
        }
    }

    pub fn next_stage(&self) -> ReflectionStage {
        self.next_stage
    }

    pub fn entries(&self) -> &[ReflectionEntry] {
        &self.entries
    }

    pub fn is_complete(&self) -> bool {
        self.next_stage == ReflectionStage::Done
    }

    pub fn submit(
        &mut self,
        stage: ReflectionStage,
        content: impl Into<String>,
    ) -> Result<(), ReflectionError> {
        if self.is_complete() {
            return Err(ReflectionError::AlreadyComplete);
        }
        if stage != self.next_stage {
            return Err(ReflectionError::WrongStage {
                expected: self.next_stage,
                actual: stage,
            });
        }
        let content = content.into();
        if content.trim().is_empty() {
            return Err(ReflectionError::EmptyContent);
        }

        self.entries.push(ReflectionEntry { stage, content });
        self.next_stage = match stage {
            ReflectionStage::Draft => ReflectionStage::Critique,
            ReflectionStage::Critique => ReflectionStage::Revision,
            ReflectionStage::Revision => ReflectionStage::MemoryNote,
            ReflectionStage::MemoryNote => ReflectionStage::Done,
            ReflectionStage::Done => ReflectionStage::Done,
        };
        Ok(())
    }

    pub fn memory_note(&self) -> Option<&str> {
        self.entries
            .iter()
            .rev()
            .find(|entry| entry.stage == ReflectionStage::MemoryNote)
            .map(|entry| entry.content.as_str())
    }

    pub fn summary(&self) -> ReflectionLoopSummary {
        let memory_note_ready = self.memory_note().is_some();
        let remaining_stages = remaining_reflection_stages(self.next_stage);
        let telemetry = reflection_loop_summary_telemetry(
            self.entries.len(),
            self.next_stage,
            self.is_complete(),
            memory_note_ready,
            remaining_stages.len(),
        );

        ReflectionLoopSummary {
            entries: self.entries.len(),
            next_stage: self.next_stage,
            is_complete: self.is_complete(),
            memory_note_ready,
            remaining_stages,
            telemetry,
        }
    }

    pub fn gate(&self) -> ReflectionLoopGateDecision {
        ReflectionLoopGateDecision::from_loop(self)
    }
}

impl ReflectionLoopGateDecision {
    pub fn from_loop(loop_state: &ReflectionLoop) -> Self {
        let summary = loop_state.summary();
        let mut reasons = Vec::new();

        if !summary.is_complete {
            reasons.push(format!(
                "reflection_incomplete_next_stage={}",
                summary.next_stage.as_str()
            ));
        }
        if summary.is_complete && !summary.memory_note_ready {
            reasons.push("reflection_memory_note_missing".to_owned());
        }

        let can_promote_memory_note = summary.is_complete && summary.memory_note_ready;
        let can_continue_reflection = !summary.is_complete;
        let telemetry = reflection_loop_gate_telemetry(
            can_continue_reflection,
            can_promote_memory_note,
            reasons.len(),
            &summary,
        );

        Self {
            summary,
            can_continue_reflection,
            can_promote_memory_note,
            reasons,
            telemetry,
        }
    }
}

impl ReflectionLoopSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(summaries: Vec<ReflectionLoopSummary>) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: ReflectionLoopSummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&ReflectionLoopSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[ReflectionLoopSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> ReflectionLoopDashboard {
        ReflectionLoopDashboard::from_summaries(&self.summaries)
    }

    pub fn health(&self, policy: ReflectionLoopHealthPolicy) -> ReflectionLoopHealth {
        self.dashboard().health(policy)
    }
}

impl ReflectionLoopDashboard {
    pub fn from_summaries(summaries: &[ReflectionLoopSummary]) -> Self {
        let total_records = summaries.len();
        let complete_records = summaries
            .iter()
            .filter(|summary| summary.is_complete)
            .count();
        let incomplete_records = total_records.saturating_sub(complete_records);
        let memory_note_ready_records = summaries
            .iter()
            .filter(|summary| summary.memory_note_ready)
            .count();
        let missing_memory_note_records = summaries
            .iter()
            .filter(|summary| summary.is_complete && !summary.memory_note_ready)
            .count();
        let stalled_stage_records = summaries
            .windows(2)
            .filter(|pair| {
                !pair[1].is_complete
                    && pair[0].next_stage == pair[1].next_stage
                    && pair[0].remaining_stages == pair[1].remaining_stages
            })
            .count();
        let completion_rate = rate(complete_records, total_records);
        let memory_note_ready_rate = rate(memory_note_ready_records, total_records);
        let latest_next_stage = summaries.last().map(|summary| summary.next_stage);
        let telemetry = reflection_loop_dashboard_telemetry(
            total_records,
            complete_records,
            incomplete_records,
            memory_note_ready_records,
            missing_memory_note_records,
            stalled_stage_records,
            completion_rate,
            memory_note_ready_rate,
            latest_next_stage,
        );

        Self {
            total_records,
            complete_records,
            incomplete_records,
            memory_note_ready_records,
            missing_memory_note_records,
            stalled_stage_records,
            completion_rate,
            memory_note_ready_rate,
            latest_next_stage,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    pub fn health(&self, policy: ReflectionLoopHealthPolicy) -> ReflectionLoopHealth {
        ReflectionLoopHealth::from_dashboard(self.clone(), policy)
    }
}

impl ReflectionLoopHealth {
    pub fn from_dashboard(
        dashboard: ReflectionLoopDashboard,
        policy: ReflectionLoopHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("reflection_loop_history_empty".to_owned());
        } else if dashboard.completion_rate < policy.minimum_completion_rate {
            watch_reasons.push(format!(
                "reflection_loop_completion_rate={:.3}<{}",
                dashboard.completion_rate, policy.minimum_completion_rate
            ));
        }

        if !dashboard.is_empty()
            && dashboard.memory_note_ready_rate < policy.minimum_memory_note_ready_rate
        {
            watch_reasons.push(format!(
                "reflection_loop_memory_note_ready_rate={:.3}<{}",
                dashboard.memory_note_ready_rate, policy.minimum_memory_note_ready_rate
            ));
        }

        if dashboard.incomplete_records > policy.maximum_incomplete_records {
            watch_reasons.push(format!(
                "reflection_loop_incomplete_records={}>{}",
                dashboard.incomplete_records, policy.maximum_incomplete_records
            ));
        }

        if dashboard.missing_memory_note_records > policy.maximum_missing_memory_note_records {
            repair_reasons.push(format!(
                "reflection_loop_missing_memory_note_records={}>{}",
                dashboard.missing_memory_note_records, policy.maximum_missing_memory_note_records
            ));
        }

        if dashboard.stalled_stage_records > policy.maximum_stalled_stage_records {
            repair_reasons.push(format!(
                "reflection_loop_stalled_stage_records={}>{}",
                dashboard.stalled_stage_records, policy.maximum_stalled_stage_records
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (ReflectionLoopHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (ReflectionLoopHealthStatus::Watch, watch_reasons)
        } else {
            (ReflectionLoopHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == ReflectionLoopHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != ReflectionLoopHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == ReflectionLoopHealthStatus::Repair
    }
}

impl ReflectionLoopSummaryHistoryRecord {
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

impl ReflectionLoopSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: ReflectionLoopSummaryHistory,
        summary: ReflectionLoopSummary,
        policy: ReflectionLoopHealthPolicy,
    ) -> ReflectionLoopSummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = reflection_loop_history_record_telemetry(&dashboard, &health);

        ReflectionLoopSummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_loop_with_health(
        &self,
        history: ReflectionLoopSummaryHistory,
        loop_state: &ReflectionLoop,
        policy: ReflectionLoopHealthPolicy,
    ) -> ReflectionLoopSummaryHistoryRecord {
        self.record_summary_with_health(history, loop_state.summary(), policy)
    }

    pub fn record_loop_with_health_gate(
        &self,
        history: ReflectionLoopSummaryHistory,
        loop_state: &ReflectionLoop,
        policy: ReflectionLoopHealthPolicy,
    ) -> ReflectionLoopHistoryGateRecord {
        let health_record = self.record_loop_with_health(history, loop_state, policy);
        let gate_decision = ReflectionLoopHistoryGate::new().gate(loop_state, &health_record);
        let telemetry =
            reflection_loop_history_gate_record_telemetry(&health_record, &gate_decision);

        ReflectionLoopHistoryGateRecord {
            health_record,
            gate_decision,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReflectionLoopHistoryGateDecision {
    pub loop_summary: ReflectionLoopSummary,
    pub reflection_health: ReflectionLoopHealth,
    pub can_continue_reflection: bool,
    pub can_promote_memory_note: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl ReflectionLoopHistoryGateDecision {
    pub fn is_memory_promotable(&self) -> bool {
        self.can_promote_memory_note && !self.requires_repair_first
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReflectionLoopHistoryGateRecord {
    pub health_record: ReflectionLoopSummaryHistoryRecord,
    pub gate_decision: ReflectionLoopHistoryGateDecision,
    pub telemetry: Vec<String>,
}

impl ReflectionLoopHistoryGateRecord {
    pub fn records(&self) -> usize {
        self.health_record.records()
    }

    pub fn allows_service_advance(&self) -> bool {
        self.health_record.allows_service_advance()
    }

    pub fn requires_repair_first(&self) -> bool {
        self.gate_decision.requires_repair_first
    }

    pub fn can_promote_memory_note(&self) -> bool {
        self.gate_decision.can_promote_memory_note
    }
}

#[derive(Debug, Clone, Default)]
pub struct ReflectionLoopHistoryGate;

impl ReflectionLoopHistoryGate {
    pub fn new() -> Self {
        Self
    }

    pub fn gate(
        &self,
        loop_state: &ReflectionLoop,
        history_record: &ReflectionLoopSummaryHistoryRecord,
    ) -> ReflectionLoopHistoryGateDecision {
        let loop_gate = loop_state.gate();
        let reflection_health = history_record.health.clone();
        let mut reasons = loop_gate.reasons.clone();
        extend_reflection_ordered_unique(
            &mut reasons,
            reflection_health
                .reasons
                .iter()
                .map(|reason| format!("reflection_loop_history:{reason}"))
                .collect::<Vec<_>>(),
        );
        let requires_repair_first = reflection_health.requires_repair_first();
        let can_continue_reflection = loop_gate.can_continue_reflection
            && reflection_health.allows_service_advance()
            && !requires_repair_first;
        let can_promote_memory_note = loop_gate.can_promote_memory_note
            && reflection_health.allows_service_advance()
            && !requires_repair_first;
        let repair_tasks =
            reflection_loop_history_gate_repair_tasks(requires_repair_first, &reasons);
        let telemetry = reflection_loop_history_gate_telemetry(
            can_continue_reflection,
            can_promote_memory_note,
            requires_repair_first,
            repair_tasks.len(),
            reasons.len(),
            &loop_gate.summary,
            reflection_health.status,
        );

        ReflectionLoopHistoryGateDecision {
            loop_summary: loop_gate.summary,
            reflection_health,
            can_continue_reflection,
            can_promote_memory_note,
            requires_repair_first,
            repair_tasks,
            reasons,
            telemetry,
        }
    }
}

fn remaining_reflection_stages(next_stage: ReflectionStage) -> Vec<ReflectionStage> {
    match next_stage {
        ReflectionStage::Draft => vec![
            ReflectionStage::Draft,
            ReflectionStage::Critique,
            ReflectionStage::Revision,
            ReflectionStage::MemoryNote,
        ],
        ReflectionStage::Critique => vec![
            ReflectionStage::Critique,
            ReflectionStage::Revision,
            ReflectionStage::MemoryNote,
        ],
        ReflectionStage::Revision => vec![ReflectionStage::Revision, ReflectionStage::MemoryNote],
        ReflectionStage::MemoryNote => vec![ReflectionStage::MemoryNote],
        ReflectionStage::Done => Vec::new(),
    }
}

fn reflection_loop_summary_telemetry(
    entries: usize,
    next_stage: ReflectionStage,
    is_complete: bool,
    memory_note_ready: bool,
    remaining_stages: usize,
) -> Vec<String> {
    vec![
        "agent_reflection_loop_summary=true".to_owned(),
        format!("agent_reflection_loop_summary_entries={entries}"),
        format!(
            "agent_reflection_loop_summary_next_stage={}",
            next_stage.as_str()
        ),
        format!("agent_reflection_loop_summary_complete={is_complete}"),
        format!("agent_reflection_loop_summary_memory_note_ready={memory_note_ready}"),
        format!("agent_reflection_loop_summary_remaining_stages={remaining_stages}"),
    ]
}

fn reflection_loop_gate_telemetry(
    can_continue_reflection: bool,
    can_promote_memory_note: bool,
    reasons: usize,
    summary: &ReflectionLoopSummary,
) -> Vec<String> {
    vec![
        "agent_reflection_loop_gate=true".to_owned(),
        format!("agent_reflection_loop_gate_continue={can_continue_reflection}"),
        format!("agent_reflection_loop_gate_memory_note={can_promote_memory_note}"),
        format!("agent_reflection_loop_gate_reasons={reasons}"),
        format!(
            "agent_reflection_loop_gate_next_stage={}",
            summary.next_stage.as_str()
        ),
        format!(
            "agent_reflection_loop_gate_remaining_stages={}",
            summary.remaining_stages.len()
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn reflection_loop_dashboard_telemetry(
    total_records: usize,
    complete_records: usize,
    incomplete_records: usize,
    memory_note_ready_records: usize,
    missing_memory_note_records: usize,
    stalled_stage_records: usize,
    completion_rate: f32,
    memory_note_ready_rate: f32,
    latest_next_stage: Option<ReflectionStage>,
) -> Vec<String> {
    vec![
        "agent_reflection_loop_dashboard=true".to_owned(),
        format!("agent_reflection_loop_dashboard_records={total_records}"),
        format!("agent_reflection_loop_dashboard_complete={complete_records}"),
        format!("agent_reflection_loop_dashboard_incomplete={incomplete_records}"),
        format!("agent_reflection_loop_dashboard_memory_note_ready={memory_note_ready_records}"),
        format!(
            "agent_reflection_loop_dashboard_missing_memory_note={missing_memory_note_records}"
        ),
        format!("agent_reflection_loop_dashboard_stalled_stage={stalled_stage_records}"),
        format!("agent_reflection_loop_dashboard_completion_rate={completion_rate:.3}"),
        format!(
            "agent_reflection_loop_dashboard_memory_note_ready_rate={memory_note_ready_rate:.3}"
        ),
        format!(
            "agent_reflection_loop_dashboard_latest_next_stage={}",
            latest_next_stage
                .map(ReflectionStage::as_str)
                .unwrap_or("none")
        ),
    ]
}

fn reflection_loop_history_record_telemetry(
    dashboard: &ReflectionLoopDashboard,
    health: &ReflectionLoopHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_reflection_loop_history_record=true".to_owned(),
        format!(
            "agent_reflection_loop_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_reflection_loop_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_reflection_loop_history_record_completion_rate={:.3}",
            dashboard.completion_rate
        ),
        format!(
            "agent_reflection_loop_history_record_memory_note_ready_rate={:.3}",
            dashboard.memory_note_ready_rate
        ),
        format!(
            "agent_reflection_loop_history_record_stalled_stage={}",
            dashboard.stalled_stage_records
        ),
    ];
    telemetry.extend(
        health
            .reasons
            .iter()
            .map(|reason| format!("agent_reflection_loop_history_record_reason={reason}")),
    );
    telemetry
}

fn reflection_loop_history_gate_repair_tasks(
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
                format!("reflection-loop-repair-{index}"),
                AgentRole::Reflector,
                format!("repair reflection loop: {reason}"),
                AgentBudget::new(12, 1, 1),
            )
            .with_lane("reflection-loop-repair")
            .with_priority(1)
        })
        .collect()
}

fn reflection_loop_history_gate_telemetry(
    can_continue_reflection: bool,
    can_promote_memory_note: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    reasons: usize,
    summary: &ReflectionLoopSummary,
    health_status: ReflectionLoopHealthStatus,
) -> Vec<String> {
    vec![
        "agent_reflection_loop_history_gate=true".to_owned(),
        format!(
            "agent_reflection_loop_history_gate_health={}",
            health_status.as_str()
        ),
        format!("agent_reflection_loop_history_gate_continue={can_continue_reflection}"),
        format!("agent_reflection_loop_history_gate_memory_note={can_promote_memory_note}"),
        format!("agent_reflection_loop_history_gate_requires_repair_first={requires_repair_first}"),
        format!("agent_reflection_loop_history_gate_repair_tasks={repair_tasks}"),
        format!("agent_reflection_loop_history_gate_reasons={reasons}"),
        format!(
            "agent_reflection_loop_history_gate_next_stage={}",
            summary.next_stage.as_str()
        ),
        format!(
            "agent_reflection_loop_history_gate_complete={}",
            summary.is_complete
        ),
        format!(
            "agent_reflection_loop_history_gate_memory_note_ready={}",
            summary.memory_note_ready
        ),
    ]
}

fn reflection_loop_history_gate_record_telemetry(
    health_record: &ReflectionLoopSummaryHistoryRecord,
    gate_decision: &ReflectionLoopHistoryGateDecision,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_reflection_loop_history_gate_record=true".to_owned(),
        format!(
            "agent_reflection_loop_history_gate_record_health={}",
            health_record.health.status.as_str()
        ),
        format!(
            "agent_reflection_loop_history_gate_record_records={}",
            health_record.records()
        ),
        format!(
            "agent_reflection_loop_history_gate_record_memory_note={}",
            gate_decision.can_promote_memory_note
        ),
        format!(
            "agent_reflection_loop_history_gate_record_requires_repair_first={}",
            gate_decision.requires_repair_first
        ),
        format!(
            "agent_reflection_loop_history_gate_record_repair_tasks={}",
            gate_decision.repair_tasks.len()
        ),
    ];
    telemetry.extend(gate_decision.telemetry.iter().cloned());
    telemetry
}

fn extend_reflection_ordered_unique(target: &mut Vec<String>, items: Vec<String>) {
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

    #[test]
    fn reflection_flow_moves_from_draft_to_memory_note() {
        let mut loop_state = ReflectionLoop::new();

        loop_state
            .submit(ReflectionStage::Draft, "draft answer")
            .unwrap();
        loop_state
            .submit(ReflectionStage::Critique, "needs budget evidence")
            .unwrap();
        loop_state
            .submit(ReflectionStage::Revision, "add budget evidence")
            .unwrap();
        loop_state
            .submit(ReflectionStage::MemoryNote, "remember budget isolation")
            .unwrap();

        assert!(loop_state.is_complete());
        assert_eq!(
            loop_state
                .entries()
                .iter()
                .map(|entry| entry.stage)
                .collect::<Vec<_>>(),
            vec![
                ReflectionStage::Draft,
                ReflectionStage::Critique,
                ReflectionStage::Revision,
                ReflectionStage::MemoryNote,
            ]
        );
        assert_eq!(loop_state.memory_note(), Some("remember budget isolation"));

        let gate = loop_state.gate();

        assert!(gate.can_promote_memory_note);
        assert!(!gate.can_continue_reflection);
        assert!(gate.reasons.is_empty());
        assert!(gate.summary.remaining_stages.is_empty());
        assert!(
            gate.telemetry
                .iter()
                .any(|line| { line == "agent_reflection_loop_gate_memory_note=true" })
        );
    }

    #[test]
    fn reflection_loop_reports_failure_paths_without_advancing() {
        let mut loop_state = ReflectionLoop::new();

        let wrong_stage = loop_state
            .submit(ReflectionStage::Critique, "too early")
            .unwrap_err();
        assert_eq!(
            wrong_stage,
            ReflectionError::WrongStage {
                expected: ReflectionStage::Draft,
                actual: ReflectionStage::Critique,
            }
        );
        assert_eq!(loop_state.next_stage(), ReflectionStage::Draft);

        let empty = loop_state
            .submit(ReflectionStage::Draft, "   ")
            .unwrap_err();
        assert_eq!(empty, ReflectionError::EmptyContent);
        assert_eq!(loop_state.next_stage(), ReflectionStage::Draft);
        let failed_gate = loop_state.gate();
        assert!(failed_gate.can_continue_reflection);
        assert!(!failed_gate.can_promote_memory_note);
        assert_eq!(
            failed_gate.reasons,
            vec!["reflection_incomplete_next_stage=draft"]
        );

        loop_state
            .submit(ReflectionStage::Draft, "draft answer")
            .unwrap();
        loop_state
            .submit(ReflectionStage::Critique, "needs evidence")
            .unwrap();
        loop_state
            .submit(ReflectionStage::Revision, "add evidence")
            .unwrap();
        loop_state
            .submit(ReflectionStage::MemoryNote, "remember evidence gate")
            .unwrap();

        let complete = loop_state
            .submit(ReflectionStage::MemoryNote, "late note")
            .unwrap_err();
        assert_eq!(complete, ReflectionError::AlreadyComplete);
        assert_eq!(loop_state.memory_note(), Some("remember evidence gate"));
    }

    #[test]
    fn reflection_loop_summary_tracks_remaining_stages_without_advancing() {
        let mut loop_state = ReflectionLoop::new();
        loop_state
            .submit(ReflectionStage::Draft, "draft answer")
            .unwrap();

        let summary = loop_state.summary();
        let gate = loop_state.gate();

        assert_eq!(summary.entries, 1);
        assert_eq!(summary.next_stage, ReflectionStage::Critique);
        assert!(!summary.is_complete);
        assert!(!summary.memory_note_ready);
        assert_eq!(
            summary.remaining_stages,
            vec![
                ReflectionStage::Critique,
                ReflectionStage::Revision,
                ReflectionStage::MemoryNote
            ]
        );
        assert!(
            summary
                .telemetry
                .iter()
                .any(|line| { line == "agent_reflection_loop_summary_remaining_stages=3" })
        );
        assert!(gate.can_continue_reflection);
        assert!(!gate.can_promote_memory_note);
        assert_eq!(
            gate.reasons,
            vec!["reflection_incomplete_next_stage=critique"]
        );
    }

    #[test]
    fn reflection_loop_history_watches_empty() {
        let health =
            ReflectionLoopSummaryHistory::new().health(ReflectionLoopHealthPolicy::default());

        assert_eq!(health.status, ReflectionLoopHealthStatus::Watch);
        assert_eq!(
            health.reasons,
            vec!["reflection_loop_history_empty".to_owned()]
        );
        assert_eq!(health.dashboard.total_records, 0);
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert!(
            health
                .dashboard
                .telemetry
                .iter()
                .any(|line| { line == "agent_reflection_loop_dashboard_latest_next_stage=none" })
        );
    }

    #[test]
    fn reflection_loop_history_marks_complete_memory_note_stable() {
        let mut loop_state = ReflectionLoop::new();
        loop_state
            .submit(ReflectionStage::Draft, "draft answer")
            .unwrap();
        loop_state
            .submit(ReflectionStage::Critique, "needs evidence")
            .unwrap();
        loop_state
            .submit(ReflectionStage::Revision, "add evidence")
            .unwrap();
        loop_state
            .submit(ReflectionStage::MemoryNote, "remember reflection evidence")
            .unwrap();

        let record = ReflectionLoopSummaryHistoryRecorder::new().record_loop_with_health(
            ReflectionLoopSummaryHistory::new(),
            &loop_state,
            ReflectionLoopHealthPolicy::default(),
        );

        assert_eq!(record.history.len(), 1);
        assert_eq!(record.records(), 1);
        assert!(record.appended_summary.is_complete);
        assert!(record.appended_summary.memory_note_ready);
        assert_eq!(record.dashboard.complete_records, 1);
        assert_eq!(record.dashboard.incomplete_records, 0);
        assert_eq!(record.dashboard.memory_note_ready_records, 1);
        assert_eq!(record.dashboard.completion_rate, 1.0);
        assert_eq!(record.dashboard.memory_note_ready_rate, 1.0);
        assert_eq!(record.health.status, ReflectionLoopHealthStatus::Stable);
        assert!(record.health.is_stable());
        assert!(record.health.allows_service_advance());
        assert!(!record.health.requires_repair_first());
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| { line == "agent_reflection_loop_history_record_status=stable" })
        );
    }

    #[test]
    fn reflection_loop_history_repairs_stalled_stage() {
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
        let history = ReflectionLoopSummaryHistory::from_summaries(vec![stalled.clone()]);

        let record = ReflectionLoopSummaryHistoryRecorder::new().record_summary_with_health(
            history,
            stalled,
            ReflectionLoopHealthPolicy::default(),
        );

        assert_eq!(record.history.len(), 2);
        assert_eq!(record.dashboard.incomplete_records, 2);
        assert_eq!(record.dashboard.stalled_stage_records, 1);
        assert_eq!(record.dashboard.completion_rate, 0.0);
        assert_eq!(record.health.status, ReflectionLoopHealthStatus::Repair);
        assert!(!record.health.allows_service_advance());
        assert!(record.health.requires_repair_first());
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert_eq!(
            record.health.reasons,
            vec![
                "reflection_loop_stalled_stage_records=1>0",
                "reflection_loop_completion_rate=0.000<0.67",
                "reflection_loop_memory_note_ready_rate=0.000<0.67",
                "reflection_loop_incomplete_records=2>1",
            ]
        );
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| { line == "agent_reflection_loop_history_record_stalled_stage=1" })
        );
    }

    #[test]
    fn reflection_loop_history_gate_promotes_stable_memory_note() {
        let mut loop_state = ReflectionLoop::new();
        loop_state
            .submit(ReflectionStage::Draft, "draft answer")
            .unwrap();
        loop_state
            .submit(ReflectionStage::Critique, "needs evidence")
            .unwrap();
        loop_state
            .submit(ReflectionStage::Revision, "add evidence")
            .unwrap();
        loop_state
            .submit(ReflectionStage::MemoryNote, "remember reflection evidence")
            .unwrap();
        let history_record = ReflectionLoopSummaryHistoryRecorder::new().record_loop_with_health(
            ReflectionLoopSummaryHistory::new(),
            &loop_state,
            ReflectionLoopHealthPolicy::default(),
        );

        let gate = ReflectionLoopHistoryGate::new().gate(&loop_state, &history_record);

        assert!(!gate.can_continue_reflection);
        assert!(gate.can_promote_memory_note);
        assert!(gate.is_memory_promotable());
        assert!(!gate.requires_repair_first);
        assert!(gate.repair_tasks.is_empty());
        assert!(gate.reasons.is_empty());
        assert_eq!(
            gate.reflection_health.status,
            ReflectionLoopHealthStatus::Stable
        );
        assert!(
            gate.telemetry
                .iter()
                .any(|line| { line == "agent_reflection_loop_history_gate_memory_note=true" })
        );
    }

    #[test]
    fn reflection_loop_history_gate_repairs_stalled_history_before_memory_note() {
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
            .submit(ReflectionStage::Draft, "draft answer")
            .unwrap();
        loop_state
            .submit(ReflectionStage::Critique, "needs evidence")
            .unwrap();
        loop_state
            .submit(ReflectionStage::Revision, "add evidence")
            .unwrap();
        loop_state
            .submit(ReflectionStage::MemoryNote, "remember reflection evidence")
            .unwrap();
        let history_record = ReflectionLoopSummaryHistoryRecorder::new().record_loop_with_health(
            ReflectionLoopSummaryHistory::from_summaries(vec![stalled.clone(), stalled]),
            &loop_state,
            ReflectionLoopHealthPolicy::default(),
        );

        let gate = ReflectionLoopHistoryGate::new().gate(&loop_state, &history_record);

        assert!(!gate.can_continue_reflection);
        assert!(!gate.can_promote_memory_note);
        assert!(!gate.is_memory_promotable());
        assert!(gate.requires_repair_first);
        assert_eq!(
            gate.reflection_health.status,
            ReflectionLoopHealthStatus::Repair
        );
        assert_eq!(gate.repair_tasks.len(), 4);
        assert_eq!(
            gate.repair_tasks
                .iter()
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "reflection-loop-repair-0",
                "reflection-loop-repair-1",
                "reflection-loop-repair-2",
                "reflection-loop-repair-3",
            ]
        );
        assert_eq!(
            gate.reasons,
            vec![
                "reflection_loop_history:reflection_loop_stalled_stage_records=1>0",
                "reflection_loop_history:reflection_loop_completion_rate=0.333<0.67",
                "reflection_loop_history:reflection_loop_memory_note_ready_rate=0.333<0.67",
                "reflection_loop_history:reflection_loop_incomplete_records=2>1",
            ]
        );
        assert!(
            gate.telemetry
                .iter()
                .any(|line| { line == "agent_reflection_loop_history_gate_repair_tasks=4" })
        );
    }

    #[test]
    fn reflection_loop_history_gate_continues_incomplete_clean_loop_without_memory_note() {
        let mut loop_state = ReflectionLoop::new();
        loop_state
            .submit(ReflectionStage::Draft, "draft answer")
            .unwrap();
        let history_record = ReflectionLoopSummaryHistoryRecorder::new().record_loop_with_health(
            ReflectionLoopSummaryHistory::new(),
            &loop_state,
            ReflectionLoopHealthPolicy {
                maximum_incomplete_records: 1,
                minimum_completion_rate: 0.0,
                minimum_memory_note_ready_rate: 0.0,
                maximum_missing_memory_note_records: 0,
                maximum_stalled_stage_records: 0,
            },
        );

        let gate = ReflectionLoopHistoryGate::new().gate(&loop_state, &history_record);

        assert!(gate.can_continue_reflection);
        assert!(!gate.can_promote_memory_note);
        assert!(!gate.is_memory_promotable());
        assert!(!gate.requires_repair_first);
        assert!(gate.repair_tasks.is_empty());
        assert_eq!(
            gate.reasons,
            vec!["reflection_incomplete_next_stage=critique"]
        );
    }

    #[test]
    fn reflection_loop_history_recorder_records_and_gates_memory_note() {
        let mut loop_state = ReflectionLoop::new();
        loop_state
            .submit(ReflectionStage::Draft, "draft answer")
            .unwrap();
        loop_state
            .submit(ReflectionStage::Critique, "needs evidence")
            .unwrap();
        loop_state
            .submit(ReflectionStage::Revision, "add evidence")
            .unwrap();
        loop_state
            .submit(ReflectionStage::MemoryNote, "remember reflection evidence")
            .unwrap();

        let record = ReflectionLoopSummaryHistoryRecorder::new().record_loop_with_health_gate(
            ReflectionLoopSummaryHistory::new(),
            &loop_state,
            ReflectionLoopHealthPolicy::default(),
        );

        assert_eq!(record.records(), 1);
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert!(record.can_promote_memory_note());
        assert!(record.gate_decision.is_memory_promotable());
        assert_eq!(
            record.health_record.health.status,
            ReflectionLoopHealthStatus::Stable
        );
        assert!(
            record.telemetry.iter().any(|line| {
                line == "agent_reflection_loop_history_gate_record_memory_note=true"
            })
        );
    }

    #[test]
    fn reflection_loop_history_recorder_records_and_gates_repair_first() {
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
            .submit(ReflectionStage::Draft, "draft answer")
            .unwrap();
        loop_state
            .submit(ReflectionStage::Critique, "needs evidence")
            .unwrap();
        loop_state
            .submit(ReflectionStage::Revision, "add evidence")
            .unwrap();
        loop_state
            .submit(ReflectionStage::MemoryNote, "remember reflection evidence")
            .unwrap();

        let record = ReflectionLoopSummaryHistoryRecorder::new().record_loop_with_health_gate(
            ReflectionLoopSummaryHistory::from_summaries(vec![stalled.clone(), stalled]),
            &loop_state,
            ReflectionLoopHealthPolicy::default(),
        );

        assert_eq!(record.records(), 3);
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert!(!record.can_promote_memory_note());
        assert_eq!(
            record.health_record.health.status,
            ReflectionLoopHealthStatus::Repair
        );
        assert_eq!(record.gate_decision.repair_tasks.len(), 4);
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_reflection_loop_history_gate_record_requires_repair_first=true"
        }));
    }
}
