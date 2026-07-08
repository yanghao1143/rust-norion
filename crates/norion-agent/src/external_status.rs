//! Clean-room external agent status boundary.
//!
//! This module models only sanitized status facts. It must not own PTYs,
//! terminal multiplexers, Herdr clients, copied AGPL code, prompts, answers,
//! terminal scrollback, or durable memory writes.

pub const EXTERNAL_AGENT_LIFECYCLE_TRACE_SCHEMA: &str = "rust-norion-external-agent-lifecycle-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalAgentStatusAuthority {
    LifecycleHook,
    ScreenManifest,
    ManualReport,
    Unknown,
}

impl ExternalAgentStatusAuthority {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LifecycleHook => "lifecycle_hook",
            Self::ScreenManifest => "screen_manifest",
            Self::ManualReport => "manual_report",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalAgentState {
    Working,
    Blocked,
    Done,
    Idle,
    Unknown,
}

impl ExternalAgentState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Working => "working",
            Self::Blocked => "blocked",
            Self::Done => "done",
            Self::Idle => "idle",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalAgentStatusSnapshot {
    pub authority: ExternalAgentStatusAuthority,
    pub state: ExternalAgentState,
    pub evidence_id: Option<String>,
    pub evidence_digest: Option<String>,
    pub sanitized_session_ref: Option<String>,
    pub observed_at_ms: Option<u64>,
    pub fresh_for_ms: Option<u64>,
}

impl ExternalAgentStatusSnapshot {
    pub fn new(authority: ExternalAgentStatusAuthority, state: ExternalAgentState) -> Self {
        Self {
            authority,
            state,
            evidence_id: None,
            evidence_digest: None,
            sanitized_session_ref: None,
            observed_at_ms: None,
            fresh_for_ms: None,
        }
    }

    pub fn with_evidence(
        mut self,
        evidence_id: impl Into<String>,
        evidence_digest: impl Into<String>,
    ) -> Self {
        self.evidence_id = Some(evidence_id.into());
        self.evidence_digest = Some(evidence_digest.into());
        self
    }

    pub fn with_sanitized_session_ref(mut self, session_ref: impl Into<String>) -> Self {
        self.sanitized_session_ref = Some(session_ref.into());
        self
    }

    pub fn observed_at(mut self, observed_at_ms: u64, fresh_for_ms: u64) -> Self {
        self.observed_at_ms = Some(observed_at_ms);
        self.fresh_for_ms = Some(fresh_for_ms);
        self
    }

    pub fn has_evidence(&self) -> bool {
        self.evidence_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
            && self
                .evidence_digest
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
    }

    pub fn is_stale_at(&self, now_ms: u64) -> bool {
        match (self.observed_at_ms, self.fresh_for_ms) {
            (Some(observed_at_ms), Some(fresh_for_ms)) => {
                observed_at_ms.saturating_add(fresh_for_ms) < now_ms
            }
            _ => true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalAgentWaitAction {
    HoldDependentTask,
    RequireOperatorAttention,
    EligibleToContinue,
    ObserveOnly,
}

impl ExternalAgentWaitAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::HoldDependentTask => "hold_dependent_task",
            Self::RequireOperatorAttention => "require_operator_attention",
            Self::EligibleToContinue => "eligible_to_continue",
            Self::ObserveOnly => "observe_only",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalAgentStatusWaitDecision {
    pub action: ExternalAgentWaitAction,
    pub reason: &'static str,
    pub evidence_required: bool,
    pub validation_success: bool,
    pub starts_process: bool,
    pub sends_prompt: bool,
    pub writes_memory: bool,
}

impl ExternalAgentStatusWaitDecision {
    fn new(action: ExternalAgentWaitAction, reason: &'static str, evidence_required: bool) -> Self {
        Self {
            action,
            reason,
            evidence_required,
            validation_success: false,
            starts_process: false,
            sends_prompt: false,
            writes_memory: false,
        }
    }

    pub fn report_only(&self) -> bool {
        !self.starts_process && !self.sends_prompt && !self.writes_memory
    }
}

pub fn agent_status_wait_gate(
    snapshot: &ExternalAgentStatusSnapshot,
    now_ms: u64,
) -> ExternalAgentStatusWaitDecision {
    if !snapshot.has_evidence() {
        return ExternalAgentStatusWaitDecision::new(
            ExternalAgentWaitAction::ObserveOnly,
            "missing_evidence",
            true,
        );
    }
    if snapshot.is_stale_at(now_ms) {
        return ExternalAgentStatusWaitDecision::new(
            ExternalAgentWaitAction::ObserveOnly,
            "stale_evidence",
            true,
        );
    }
    match snapshot.state {
        ExternalAgentState::Working => ExternalAgentStatusWaitDecision::new(
            ExternalAgentWaitAction::HoldDependentTask,
            "external_agent_working",
            false,
        ),
        ExternalAgentState::Blocked => ExternalAgentStatusWaitDecision::new(
            ExternalAgentWaitAction::RequireOperatorAttention,
            "external_agent_blocked",
            false,
        ),
        ExternalAgentState::Done => ExternalAgentStatusWaitDecision::new(
            ExternalAgentWaitAction::EligibleToContinue,
            "external_agent_done_requires_separate_validation",
            false,
        ),
        ExternalAgentState::Idle => ExternalAgentStatusWaitDecision::new(
            ExternalAgentWaitAction::ObserveOnly,
            "external_agent_idle",
            false,
        ),
        ExternalAgentState::Unknown => ExternalAgentStatusWaitDecision::new(
            ExternalAgentWaitAction::ObserveOnly,
            "external_agent_unknown",
            true,
        ),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExternalAgentLifecycleReport {
    pub agents: usize,
    pub evidence_ready: usize,
    pub missing_evidence: usize,
    pub stale_evidence: usize,
    pub working: usize,
    pub blocked: usize,
    pub done: usize,
    pub idle: usize,
    pub unknown: usize,
    pub hold_dependent_task: usize,
    pub require_operator_attention: usize,
    pub eligible_to_continue: usize,
    pub observe_only: usize,
    pub validation_success: usize,
    pub report_only: usize,
    pub starts_process: usize,
    pub sends_prompt: usize,
    pub writes_memory: usize,
    pub cleanup_required: usize,
}

impl ExternalAgentLifecycleReport {
    pub fn from_snapshots(snapshots: &[ExternalAgentStatusSnapshot], now_ms: u64) -> Self {
        let mut report = Self {
            agents: snapshots.len(),
            ..Self::default()
        };
        for snapshot in snapshots {
            let has_evidence = snapshot.has_evidence();
            let stale = snapshot.is_stale_at(now_ms);
            if has_evidence && !stale {
                report.evidence_ready += 1;
            }
            if !has_evidence {
                report.missing_evidence += 1;
            }
            if stale {
                report.stale_evidence += 1;
            }
            match snapshot.state {
                ExternalAgentState::Working => report.working += 1,
                ExternalAgentState::Blocked => report.blocked += 1,
                ExternalAgentState::Done => report.done += 1,
                ExternalAgentState::Idle => report.idle += 1,
                ExternalAgentState::Unknown => report.unknown += 1,
            }

            let decision = agent_status_wait_gate(snapshot, now_ms);
            match decision.action {
                ExternalAgentWaitAction::HoldDependentTask => report.hold_dependent_task += 1,
                ExternalAgentWaitAction::RequireOperatorAttention => {
                    report.require_operator_attention += 1
                }
                ExternalAgentWaitAction::EligibleToContinue => report.eligible_to_continue += 1,
                ExternalAgentWaitAction::ObserveOnly => report.observe_only += 1,
            }
            report.validation_success += usize::from(decision.validation_success);
            report.report_only += usize::from(decision.report_only());
            report.starts_process += usize::from(decision.starts_process);
            report.sends_prompt += usize::from(decision.sends_prompt);
            report.writes_memory += usize::from(decision.writes_memory);
            if matches!(
                decision.action,
                ExternalAgentWaitAction::HoldDependentTask
                    | ExternalAgentWaitAction::RequireOperatorAttention
            ) || !has_evidence
                || stale
                || snapshot.state == ExternalAgentState::Unknown
            {
                report.cleanup_required += 1;
            }
        }
        report
    }

    pub fn ready(&self) -> bool {
        self.agents > 0
            && self.evidence_ready == self.agents
            && self.missing_evidence == 0
            && self.stale_evidence == 0
            && self.working == 0
            && self.blocked == 0
            && self.unknown == 0
            && self.done + self.idle == self.agents
            && self.hold_dependent_task == 0
            && self.require_operator_attention == 0
            && self.validation_success == 0
            && self.report_only == self.agents
            && self.starts_process == 0
            && self.sends_prompt == 0
            && self.writes_memory == 0
            && self.cleanup_required == 0
    }

    pub fn report_digest(&self) -> String {
        format!(
            "redaction-digest:external-agent-lifecycle:{}:{}:{}:{}:{}:{}",
            self.agents,
            self.evidence_ready,
            self.done,
            self.idle,
            self.cleanup_required,
            self.validation_success
        )
    }

    pub fn trace_json_line(&self) -> String {
        format!(
            "{{\"schema\":\"{}\",\"report_kind\":\"lifecycle_gate\",\"agents\":{},\"evidence_ready\":{},\"missing_evidence\":{},\"stale_evidence\":{},\"working\":{},\"blocked\":{},\"done\":{},\"idle\":{},\"unknown\":{},\"hold_dependent_task\":{},\"require_operator_attention\":{},\"eligible_to_continue\":{},\"observe_only\":{},\"validation_success\":{},\"report_only\":{},\"starts_process\":{},\"sends_prompt\":{},\"writes_memory\":{},\"cleanup_required\":{},\"ready\":{},\"report_digest\":\"{}\",\"read_only\":true,\"write_allowed\":false,\"applied\":false}}",
            EXTERNAL_AGENT_LIFECYCLE_TRACE_SCHEMA,
            self.agents,
            self.evidence_ready,
            self.missing_evidence,
            self.stale_evidence,
            self.working,
            self.blocked,
            self.done,
            self.idle,
            self.unknown,
            self.hold_dependent_task,
            self.require_operator_attention,
            self.eligible_to_continue,
            self.observe_only,
            self.validation_success,
            self.report_only,
            self.starts_process,
            self.sends_prompt,
            self.writes_memory,
            self.cleanup_required,
            self.ready(),
            self.report_digest()
        )
    }
}

pub fn default_external_agent_lifecycle_report() -> ExternalAgentLifecycleReport {
    let snapshots = [
        ExternalAgentStatusSnapshot::new(
            ExternalAgentStatusAuthority::LifecycleHook,
            ExternalAgentState::Done,
        )
        .with_evidence(
            "evidence:external-agent-lifecycle:done",
            "redaction-digest:external-agent:done",
        )
        .with_sanitized_session_ref("agent:done:1")
        .observed_at(1_000, 500),
        ExternalAgentStatusSnapshot::new(
            ExternalAgentStatusAuthority::LifecycleHook,
            ExternalAgentState::Idle,
        )
        .with_evidence(
            "evidence:external-agent-lifecycle:idle",
            "redaction-digest:external-agent:idle",
        )
        .with_sanitized_session_ref("agent:idle:1")
        .observed_at(1_000, 500),
    ];
    ExternalAgentLifecycleReport::from_snapshots(&snapshots, 1_100)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(state: ExternalAgentState) -> ExternalAgentStatusSnapshot {
        ExternalAgentStatusSnapshot::new(ExternalAgentStatusAuthority::ManualReport, state)
            .with_evidence("evidence:status:1", "redaction-digest:external-agent")
            .with_sanitized_session_ref("session:panel:1")
            .observed_at(1_000, 500)
    }

    fn assert_report_only(decision: &ExternalAgentStatusWaitDecision) {
        assert!(decision.report_only());
        assert!(!decision.starts_process);
        assert!(!decision.sends_prompt);
        assert!(!decision.writes_memory);
        assert!(!decision.validation_success);
    }

    #[test]
    fn working_status_holds_dependent_task_without_side_effects() {
        let decision = agent_status_wait_gate(&snapshot(ExternalAgentState::Working), 1_100);

        assert_eq!(decision.action, ExternalAgentWaitAction::HoldDependentTask);
        assert_eq!(decision.reason, "external_agent_working");
        assert_report_only(&decision);
    }

    #[test]
    fn blocked_status_requires_operator_attention_without_side_effects() {
        let decision = agent_status_wait_gate(&snapshot(ExternalAgentState::Blocked), 1_100);

        assert_eq!(
            decision.action,
            ExternalAgentWaitAction::RequireOperatorAttention
        );
        assert_eq!(decision.reason, "external_agent_blocked");
        assert_report_only(&decision);
    }

    #[test]
    fn done_status_allows_scheduling_but_not_validation_success() {
        let decision = agent_status_wait_gate(&snapshot(ExternalAgentState::Done), 1_100);

        assert_eq!(decision.action, ExternalAgentWaitAction::EligibleToContinue);
        assert_eq!(
            decision.reason,
            "external_agent_done_requires_separate_validation"
        );
        assert_report_only(&decision);
    }

    #[test]
    fn idle_and_unknown_statuses_observe_only() {
        let idle = agent_status_wait_gate(&snapshot(ExternalAgentState::Idle), 1_100);
        let unknown = agent_status_wait_gate(&snapshot(ExternalAgentState::Unknown), 1_100);

        assert_eq!(idle.action, ExternalAgentWaitAction::ObserveOnly);
        assert_eq!(idle.reason, "external_agent_idle");
        assert_eq!(unknown.action, ExternalAgentWaitAction::ObserveOnly);
        assert_eq!(unknown.reason, "external_agent_unknown");
        assert!(unknown.evidence_required);
        assert_report_only(&idle);
        assert_report_only(&unknown);
    }

    #[test]
    fn stale_or_missing_evidence_observes_only() {
        let stale = agent_status_wait_gate(&snapshot(ExternalAgentState::Done), 1_600);
        let missing = agent_status_wait_gate(
            &ExternalAgentStatusSnapshot::new(
                ExternalAgentStatusAuthority::Unknown,
                ExternalAgentState::Done,
            )
            .observed_at(1_000, 500),
            1_100,
        );

        assert_eq!(stale.action, ExternalAgentWaitAction::ObserveOnly);
        assert_eq!(stale.reason, "stale_evidence");
        assert!(stale.evidence_required);
        assert_eq!(missing.action, ExternalAgentWaitAction::ObserveOnly);
        assert_eq!(missing.reason, "missing_evidence");
        assert!(missing.evidence_required);
        assert_report_only(&stale);
        assert_report_only(&missing);
    }

    #[test]
    fn partial_evidence_observes_only() {
        let mut snapshot = ExternalAgentStatusSnapshot::new(
            ExternalAgentStatusAuthority::LifecycleHook,
            ExternalAgentState::Done,
        )
        .with_sanitized_session_ref("session:panel:1")
        .observed_at(1_000, 500);
        snapshot.evidence_id = Some("evidence:status:partial".to_owned());

        let partial = agent_status_wait_gate(&snapshot, 1_100);

        assert_eq!(partial.action, ExternalAgentWaitAction::ObserveOnly);
        assert_eq!(partial.reason, "missing_evidence");
        assert_report_only(&partial);
    }

    #[test]
    fn lifecycle_report_requires_zero_active_or_unclean_agents() {
        let report = default_external_agent_lifecycle_report();

        assert!(report.ready());
        assert_eq!(report.agents, 2);
        assert_eq!(report.evidence_ready, 2);
        assert_eq!(report.done, 1);
        assert_eq!(report.idle, 1);
        assert_eq!(report.working, 0);
        assert_eq!(report.blocked, 0);
        assert_eq!(report.cleanup_required, 0);
        assert_eq!(report.validation_success, 0);
        assert_eq!(report.report_only, 2);
        assert!(report.trace_json_line().contains("\"ready\":true"));
        assert!(
            report
                .trace_json_line()
                .contains("\"report_digest\":\"redaction-digest:")
        );
    }

    #[test]
    fn lifecycle_report_marks_working_agent_as_cleanup_required() {
        let snapshots = [snapshot(ExternalAgentState::Working)];
        let report = ExternalAgentLifecycleReport::from_snapshots(&snapshots, 1_100);

        assert!(!report.ready());
        assert_eq!(report.working, 1);
        assert_eq!(report.hold_dependent_task, 1);
        assert_eq!(report.cleanup_required, 1);
    }
}
