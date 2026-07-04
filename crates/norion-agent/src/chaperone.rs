use std::collections::{BTreeMap, BTreeSet};

use crate::adapter::AgentAdapterBoundaryStatus;
use crate::budget::AgentBudget;
use crate::task::{AgentRole, AgentTask, AgentTaskQueue};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReasoningChaperoneFoldGuard;

impl ReasoningChaperoneFoldGuard {
    pub fn evaluate(trace: &ReasoningChaperoneStructuredTrace) -> ReasoningChaperoneFoldSummary {
        let declared_capabilities = normalized_set(&trace.declared_capabilities);
        let evidence_refs = normalized_set(&trace.evidence_refs);
        let mut reasons = Vec::new();

        let undefined_capability_count = trace
            .used_capabilities
            .iter()
            .filter(|capability| {
                let capability = capability.trim();
                !capability.is_empty() && !declared_capabilities.contains(capability)
            })
            .count();
        if undefined_capability_count > 0 {
            reasons.push("undefined_capability_before_service_execution".to_owned());
        }

        let ungated_side_effect_count = trace
            .side_effects
            .iter()
            .filter(|side_effect| side_effect.executed && !side_effect.gate_passed)
            .count();
        if ungated_side_effect_count > 0 {
            reasons.push("ungated_side_effect_before_service_execution".to_owned());
        }

        let missing_evidence_count = trace
            .promotions
            .iter()
            .map(|promotion| {
                if promotion.evidence_refs.is_empty() {
                    1
                } else {
                    promotion
                        .evidence_refs
                        .iter()
                        .filter(|evidence_ref| !evidence_refs.contains(evidence_ref.trim()))
                        .count()
                }
            })
            .sum::<usize>();
        if missing_evidence_count > 0 {
            reasons.push("missing_evidence_before_admission".to_owned());
        }

        let mut contradiction_count = state_transition_contradictions(&trace.state_transitions);
        if trace
            .continuation_task_id
            .as_deref()
            .is_some_and(|continuation_task_id| continuation_task_id != trace.task_id)
        {
            contradiction_count += 1;
            reasons.push("task_id_continuity_contradiction".to_owned());
        }
        if trace.continuation_budget_overspent {
            contradiction_count += 1;
            reasons.push("budget_overspend_before_continuation".to_owned());
        }
        let predictive_surprise_count = trace
            .predictive_surprise
            .as_ref()
            .filter(|surprise| surprise.surprise_milli > surprise.repair_threshold_milli)
            .map(|_| 1)
            .unwrap_or(0);
        if predictive_surprise_count > 0 {
            contradiction_count += predictive_surprise_count;
            reasons.push("predictive_surprise_exceeds_repair_threshold".to_owned());
        }
        if contradiction_count > 0 {
            push_unique(&mut reasons, "structured_state_contradiction");
        }

        let raw_capture_rejected_count = usize::from(trace.raw_cot_capture_attempted)
            + usize::from(trace.raw_prompt_capture_attempted);
        if raw_capture_rejected_count > 0 {
            reasons.push("raw_reasoning_capture_forbidden".to_owned());
        }

        let watch_only = trace.predictive_surprise.as_ref().is_some_and(|surprise| {
            surprise.surprise_milli > 0
                && surprise.surprise_milli <= surprise.repair_threshold_milli
        });
        let status = if !reasons.is_empty() {
            AgentAdapterBoundaryStatus::Repair
        } else if watch_only {
            AgentAdapterBoundaryStatus::Watch
        } else {
            AgentAdapterBoundaryStatus::Stable
        };
        let repair_tasks = if status == AgentAdapterBoundaryStatus::Repair {
            vec![repair_task(&trace.task_id, &reasons)]
        } else {
            Vec::new()
        };

        ReasoningChaperoneFoldSummary {
            task_id: trace.task_id.clone(),
            fold_status: status,
            undefined_capability_count,
            contradiction_count,
            ungated_side_effect_count,
            missing_evidence_count,
            predictive_surprise_count,
            raw_capture_rejected_count,
            repair_task_count: repair_tasks.len(),
            raw_cot_captured: false,
            raw_prompt_captured: false,
            service_execution_allowed: status != AgentAdapterBoundaryStatus::Repair
                && undefined_capability_count == 0
                && ungated_side_effect_count == 0,
            admission_allowed: status != AgentAdapterBoundaryStatus::Repair
                && missing_evidence_count == 0,
            consumed_structured_fields: consumed_structured_fields(),
            blocked_reasons: reasons,
            repair_tasks,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReasoningChaperoneStructuredTrace {
    pub task_id: String,
    pub continuation_task_id: Option<String>,
    pub declared_capabilities: Vec<String>,
    pub used_capabilities: Vec<String>,
    pub side_effects: Vec<ReasoningChaperoneSideEffect>,
    pub evidence_refs: Vec<String>,
    pub promotions: Vec<ReasoningChaperonePromotion>,
    pub state_transitions: Vec<ReasoningChaperoneStateTransition>,
    pub continuation_budget_overspent: bool,
    pub predictive_surprise: Option<ReasoningChaperonePredictiveSurprise>,
    pub raw_cot_capture_attempted: bool,
    pub raw_prompt_capture_attempted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReasoningChaperoneSideEffect {
    pub capability: String,
    pub gate_passed: bool,
    pub executed: bool,
}

impl ReasoningChaperoneSideEffect {
    pub fn executed(capability: impl Into<String>, gate_passed: bool) -> Self {
        Self {
            capability: capability.into(),
            gate_passed,
            executed: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReasoningChaperonePromotion {
    pub kind: String,
    pub evidence_refs: Vec<String>,
}

impl ReasoningChaperonePromotion {
    pub fn new(kind: impl Into<String>, evidence_refs: Vec<String>) -> Self {
        Self {
            kind: kind.into(),
            evidence_refs,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReasoningChaperoneStateTransition {
    pub key: String,
    pub from: String,
    pub to: String,
}

impl ReasoningChaperoneStateTransition {
    pub fn new(key: impl Into<String>, from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            from: from.into(),
            to: to.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReasoningChaperonePredictiveSurprise {
    pub surprise_milli: usize,
    pub repair_threshold_milli: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReasoningChaperoneFoldSummary {
    pub task_id: String,
    pub fold_status: AgentAdapterBoundaryStatus,
    pub undefined_capability_count: usize,
    pub contradiction_count: usize,
    pub ungated_side_effect_count: usize,
    pub missing_evidence_count: usize,
    pub predictive_surprise_count: usize,
    pub raw_capture_rejected_count: usize,
    pub repair_task_count: usize,
    pub raw_cot_captured: bool,
    pub raw_prompt_captured: bool,
    pub service_execution_allowed: bool,
    pub admission_allowed: bool,
    pub consumed_structured_fields: Vec<&'static str>,
    pub blocked_reasons: Vec<String>,
    pub repair_tasks: Vec<AgentTask>,
}

impl ReasoningChaperoneFoldSummary {
    pub fn requires_repair_first(&self) -> bool {
        self.fold_status == AgentAdapterBoundaryStatus::Repair
    }

    pub fn repair_first_queue(&self, queue: AgentTaskQueue) -> AgentTaskQueue {
        queue.with_repair_first(&self.repair_tasks)
    }

    pub fn summary_line(&self) -> String {
        format!(
            "reasoning_chaperone_fold_guard: fold_status={} undefined_capability_count={} contradiction_count={} ungated_side_effect_count={} missing_evidence_count={} repair_task_count={} raw_cot_captured={} raw_prompt_captured={} service_execution_allowed={} admission_allowed={} consumed_structured_field_count={} blocked_reason_count={}",
            self.fold_status.as_str(),
            self.undefined_capability_count,
            self.contradiction_count,
            self.ungated_side_effect_count,
            self.missing_evidence_count,
            self.repair_task_count,
            self.raw_cot_captured,
            self.raw_prompt_captured,
            self.service_execution_allowed,
            self.admission_allowed,
            self.consumed_structured_fields.len(),
            self.blocked_reasons.len()
        )
    }
}

fn state_transition_contradictions(transitions: &[ReasoningChaperoneStateTransition]) -> usize {
    let mut states = BTreeMap::new();
    let mut contradictions = 0;

    for transition in transitions {
        let key = transition.key.trim();
        if key.is_empty() {
            continue;
        }
        match states.get(key) {
            Some(current) if current != transition.from.trim() => contradictions += 1,
            _ => {}
        }
        states.insert(key.to_owned(), transition.to.trim().to_owned());
    }

    contradictions
}

fn normalized_set(values: &[String]) -> BTreeSet<String> {
    values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect()
}

fn repair_task(task_id: &str, reasons: &[String]) -> AgentTask {
    AgentTask::new(
        format!(
            "reasoning-chaperone-repair-{}",
            safe_task_id_suffix(task_id)
        ),
        AgentRole::Reviewer,
        format!(
            "repair structured reasoning trace before service execution/admission: {}",
            reasons
                .first()
                .map(String::as_str)
                .unwrap_or("fold_guard_repair")
        ),
        AgentBudget::new(16, 1, 1),
    )
    .with_lane("reasoning-chaperone-repair")
    .with_priority(1)
}

fn safe_task_id_suffix(task_id: &str) -> String {
    let suffix = task_id
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
        .collect::<String>();
    if suffix.is_empty() {
        "unknown".to_owned()
    } else {
        suffix
    }
}

fn push_unique(reasons: &mut Vec<String>, reason: &str) {
    if !reasons.iter().any(|existing| existing == reason) {
        reasons.push(reason.to_owned());
    }
}

fn consumed_structured_fields() -> Vec<&'static str> {
    vec![
        "task_id",
        "continuation_task_id",
        "declared_capabilities",
        "used_capabilities",
        "side_effects.gate_passed",
        "side_effects.executed",
        "evidence_refs",
        "promotions.evidence_refs",
        "state_transitions",
        "continuation_budget_overspent",
        "predictive_surprise",
        "raw_cot_capture_attempted",
        "raw_prompt_capture_attempted",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stable_trace() -> ReasoningChaperoneStructuredTrace {
        ReasoningChaperoneStructuredTrace {
            task_id: "task-1".to_owned(),
            continuation_task_id: Some("task-1".to_owned()),
            declared_capabilities: vec!["search".to_owned(), "memory_promote".to_owned()],
            used_capabilities: vec!["search".to_owned()],
            side_effects: vec![ReasoningChaperoneSideEffect::executed("search", true)],
            evidence_refs: vec!["digest:evidence-1".to_owned()],
            promotions: vec![ReasoningChaperonePromotion::new(
                "memory",
                vec!["digest:evidence-1".to_owned()],
            )],
            state_transitions: vec![
                ReasoningChaperoneStateTransition::new("phase", "planned", "running"),
                ReasoningChaperoneStateTransition::new("phase", "running", "complete"),
            ],
            continuation_budget_overspent: false,
            predictive_surprise: None,
            raw_cot_capture_attempted: false,
            raw_prompt_capture_attempted: false,
        }
    }

    #[test]
    fn reasoning_chaperone_stable_trace_passes_without_repair_tasks() {
        let summary = ReasoningChaperoneFoldGuard::evaluate(&stable_trace());

        assert_eq!(summary.fold_status, AgentAdapterBoundaryStatus::Stable);
        assert!(summary.service_execution_allowed);
        assert!(summary.admission_allowed);
        assert_eq!(summary.repair_task_count, 0);
        assert!(!summary.raw_cot_captured);
        assert!(summary.summary_line().contains("fold_status=stable"));
    }

    #[test]
    fn reasoning_chaperone_undefined_capability_blocks_service_execution() {
        let mut trace = stable_trace();
        trace.used_capabilities.push("shell_write".to_owned());

        let summary = ReasoningChaperoneFoldGuard::evaluate(&trace);

        assert_eq!(summary.fold_status, AgentAdapterBoundaryStatus::Repair);
        assert_eq!(summary.undefined_capability_count, 1);
        assert!(!summary.service_execution_allowed);
        assert_eq!(summary.repair_task_count, 1);
    }

    #[test]
    fn reasoning_chaperone_contradiction_emits_repair_first_task() {
        let mut trace = stable_trace();
        trace
            .state_transitions
            .push(ReasoningChaperoneStateTransition::new(
                "phase", "planned", "promoted",
            ));
        let business = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "business",
            AgentRole::Coder,
            "continue business task",
            AgentBudget::new(4, 1, 1),
        )]);

        let summary = ReasoningChaperoneFoldGuard::evaluate(&trace);
        let guarded_queue = summary.repair_first_queue(business);
        let business_task = guarded_queue
            .tasks()
            .into_iter()
            .find(|task| task.id == "business")
            .unwrap();

        assert_eq!(summary.fold_status, AgentAdapterBoundaryStatus::Repair);
        assert_eq!(summary.contradiction_count, 1);
        assert_eq!(summary.repair_tasks.len(), 1);
        assert!(
            business_task
                .dependencies
                .contains(&summary.repair_tasks[0].id)
        );
    }

    #[test]
    fn reasoning_chaperone_missing_evidence_rejects_admission() {
        let mut trace = stable_trace();
        trace.promotions = vec![ReasoningChaperonePromotion::new(
            "genome",
            vec!["digest:missing".to_owned()],
        )];

        let summary = ReasoningChaperoneFoldGuard::evaluate(&trace);

        assert_eq!(summary.fold_status, AgentAdapterBoundaryStatus::Repair);
        assert_eq!(summary.missing_evidence_count, 1);
        assert!(!summary.admission_allowed);
    }

    #[test]
    fn reasoning_chaperone_rejects_raw_cot_and_prompt_capture_without_storing_them() {
        let mut trace = stable_trace();
        trace.raw_cot_capture_attempted = true;
        trace.raw_prompt_capture_attempted = true;

        let summary = ReasoningChaperoneFoldGuard::evaluate(&trace);

        assert_eq!(summary.fold_status, AgentAdapterBoundaryStatus::Repair);
        assert_eq!(summary.raw_capture_rejected_count, 2);
        assert!(!summary.raw_cot_captured);
        assert!(!summary.raw_prompt_captured);
        assert!(!summary.summary_line().contains("prompt="));
    }

    #[test]
    fn reasoning_chaperone_predictive_surprise_folds_into_repair_path() {
        let mut trace = stable_trace();
        trace.predictive_surprise = Some(ReasoningChaperonePredictiveSurprise {
            surprise_milli: 900,
            repair_threshold_milli: 700,
        });

        let summary = ReasoningChaperoneFoldGuard::evaluate(&trace);

        assert_eq!(summary.fold_status, AgentAdapterBoundaryStatus::Repair);
        assert_eq!(summary.predictive_surprise_count, 1);
        assert_eq!(summary.repair_task_count, 1);
    }
}
