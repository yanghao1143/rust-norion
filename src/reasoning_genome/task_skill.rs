use crate::hierarchy::TaskProfile;
use crate::privacy_redaction::{contains_private_or_executable_marker, stable_redaction_digest};

use super::model::{GeneValidationStatus, profile_slug};
use super::transaction::GeneScissorsOperatorDecision;

pub const TASK_SKILL_GENE_SCHEMA_VERSION: &str = "task_skill_gene_v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskSkillGeneDecision {
    AcceptPreview,
    HoldForEvidence,
    Reject,
    Quarantine,
    DuplicateSuppressed,
}

impl TaskSkillGeneDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AcceptPreview => "accept_preview",
            Self::HoldForEvidence => "hold_for_evidence",
            Self::Reject => "reject",
            Self::Quarantine => "quarantine",
            Self::DuplicateSuppressed => "duplicate_suppressed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskSkillGeneEvidence {
    pub compiler_passed: bool,
    pub tests_passed: bool,
    pub benchmark_passed: bool,
    pub multilingual_eval_passed: bool,
    pub privacy_gate_passed: bool,
    pub user_approved: bool,
    pub regression_failures: usize,
    pub age: u32,
}

impl Default for TaskSkillGeneEvidence {
    fn default() -> Self {
        Self {
            compiler_passed: false,
            tests_passed: false,
            benchmark_passed: false,
            multilingual_eval_passed: false,
            privacy_gate_passed: true,
            user_approved: false,
            regression_failures: 0,
            age: 0,
        }
    }
}

impl TaskSkillGeneEvidence {
    pub fn passing() -> Self {
        Self {
            compiler_passed: true,
            tests_passed: true,
            benchmark_passed: true,
            multilingual_eval_passed: true,
            privacy_gate_passed: true,
            user_approved: true,
            regression_failures: 0,
            age: 0,
        }
    }

    pub fn validation_status(&self) -> GeneValidationStatus {
        if !self.privacy_gate_passed || self.regression_failures > 0 {
            return GeneValidationStatus::Failed;
        }
        if self.compiler_passed
            && self.tests_passed
            && self.benchmark_passed
            && self.multilingual_eval_passed
        {
            GeneValidationStatus::Passed
        } else {
            GeneValidationStatus::Pending
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskSkillGeneInput {
    pub profile: TaskProfile,
    pub language: String,
    pub domain: String,
    pub tool_policy: String,
    pub prompt_policy_summary: String,
    pub validation_expectations: Vec<String>,
    pub failure_modes: Vec<String>,
    pub safe_activation_constraints: Vec<String>,
    pub clean_room_provenance: Option<String>,
    pub rollback_anchor_id: Option<String>,
    pub evidence: TaskSkillGeneEvidence,
    pub operator_decision: GeneScissorsOperatorDecision,
}

impl TaskSkillGeneInput {
    pub fn new(
        profile: TaskProfile,
        language: impl Into<String>,
        domain: impl Into<String>,
        prompt_policy_summary: impl Into<String>,
    ) -> Self {
        Self {
            profile,
            language: language.into(),
            domain: domain.into(),
            tool_policy: "local_tools_only".to_owned(),
            prompt_policy_summary: prompt_policy_summary.into(),
            validation_expectations: Vec::new(),
            failure_modes: Vec::new(),
            safe_activation_constraints: vec![
                "preview_only".to_owned(),
                "operator_approval_required".to_owned(),
                "rollback_anchor_required".to_owned(),
            ],
            clean_room_provenance: None,
            rollback_anchor_id: None,
            evidence: TaskSkillGeneEvidence::default(),
            operator_decision: GeneScissorsOperatorDecision::Pending,
        }
    }

    pub fn with_tool_policy(mut self, tool_policy: impl Into<String>) -> Self {
        self.tool_policy = tool_policy.into();
        self
    }

    pub fn with_validation_expectations(
        mut self,
        values: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.validation_expectations = values.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_failure_modes(
        mut self,
        values: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.failure_modes = values.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_clean_room_provenance(mut self, provenance: impl Into<String>) -> Self {
        self.clean_room_provenance = Some(provenance.into());
        self
    }

    pub fn with_rollback_anchor(mut self, anchor: impl Into<String>) -> Self {
        self.rollback_anchor_id = Some(anchor.into());
        self
    }

    pub fn with_evidence(mut self, evidence: TaskSkillGeneEvidence) -> Self {
        self.evidence = evidence;
        self
    }

    pub fn with_operator_decision(mut self, decision: GeneScissorsOperatorDecision) -> Self {
        self.operator_decision = decision;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskSkillGeneScoringPolicy {
    pub stale_age: u32,
    pub min_accept_score_milli: i32,
}

impl Default for TaskSkillGeneScoringPolicy {
    fn default() -> Self {
        Self {
            stale_age: 8,
            min_accept_score_milli: 760,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskSkillGeneCandidate {
    pub schema_version: &'static str,
    pub candidate_id: String,
    pub profile: TaskProfile,
    pub language: String,
    pub domain: String,
    pub tool_policy: String,
    pub skill_fingerprint_digest: String,
    pub prompt_policy_summary_digest: String,
    pub validation_expectations: Vec<String>,
    pub failure_modes: Vec<String>,
    pub safe_activation_constraints: Vec<String>,
    pub clean_room_provenance_digest: Option<String>,
    pub rollback_anchor_digest: Option<String>,
    pub duplicate_of: Option<String>,
    pub decision: TaskSkillGeneDecision,
    pub validation_status: GeneValidationStatus,
    pub operator_decision: GeneScissorsOperatorDecision,
    pub score_milli: i32,
    pub reason_codes: Vec<String>,
    pub activation_eligible: bool,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl TaskSkillGeneCandidate {
    pub fn is_read_only_preview(&self) -> bool {
        self.read_only && !self.write_allowed && !self.applied
    }

    pub fn summary_line(&self) -> String {
        format!(
            "task_skill_gene schema={} candidate={} profile={} decision={} score_milli={} validation={} approval={} activation_eligible={} reasons={} read_only={} write_allowed={} applied={}",
            self.schema_version,
            self.candidate_id,
            profile_slug(self.profile),
            self.decision.as_str(),
            self.score_milli,
            self.validation_status.as_str(),
            self.operator_decision.as_str(),
            self.activation_eligible,
            self.reason_codes.len(),
            self.read_only,
            self.write_allowed,
            self.applied
        )
    }

    pub fn redacted_trace_line(&self) -> String {
        format!(
            "{{\"schema\":\"{}\",\"candidate_id\":\"{}\",\"profile\":\"{}\",\"decision\":\"{}\",\"score_milli\":{},\"validation_status\":\"{}\",\"approval_status\":\"{}\",\"activation_eligible\":{},\"policy_digest\":\"{}\",\"raw_payload_included\":false,\"read_only\":{},\"write_allowed\":{},\"applied\":{}}}",
            TASK_SKILL_GENE_SCHEMA_VERSION,
            self.candidate_id,
            profile_slug(self.profile),
            self.decision.as_str(),
            self.score_milli,
            self.validation_status.as_str(),
            self.operator_decision.as_str(),
            self.activation_eligible,
            self.prompt_policy_summary_digest,
            self.read_only,
            self.write_allowed,
            self.applied
        )
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TaskSkillGeneScorer {
    pub policy: TaskSkillGeneScoringPolicy,
}

impl TaskSkillGeneScorer {
    pub fn new(policy: TaskSkillGeneScoringPolicy) -> Self {
        Self { policy }
    }

    pub fn score_candidate(
        &self,
        existing: &[TaskSkillGeneCandidate],
        input: TaskSkillGeneInput,
    ) -> TaskSkillGeneCandidate {
        let fingerprint = skill_fingerprint(&input);
        let duplicate_of = existing
            .iter()
            .find(|candidate| candidate.skill_fingerprint_digest == fingerprint)
            .map(|candidate| candidate.candidate_id.clone());
        let conflicts = existing.iter().any(|candidate| {
            candidate.profile == input.profile
                && eq_fold(&candidate.language, &input.language)
                && eq_fold(&candidate.domain, &input.domain)
                && !eq_fold(&candidate.tool_policy, &input.tool_policy)
                && !matches!(
                    candidate.decision,
                    TaskSkillGeneDecision::DuplicateSuppressed
                        | TaskSkillGeneDecision::Quarantine
                        | TaskSkillGeneDecision::Reject
                )
        });
        let blocked_payload = input_contains_blocked_payload(&input);
        let validation_status = input.evidence.validation_status();
        let score_milli = score_milli(&input.evidence, blocked_payload);
        let stale = input.evidence.age >= self.policy.stale_age;
        let mut reason_codes = Vec::new();

        if duplicate_of.is_some() {
            push_unique(&mut reason_codes, "task_skill_duplicate_suppressed");
        }
        if blocked_payload {
            push_unique(&mut reason_codes, "task_skill_blocked_payload");
        }
        if conflicts {
            push_unique(&mut reason_codes, "task_skill_policy_conflict");
        }
        if stale {
            push_unique(&mut reason_codes, "task_skill_stale_decay");
        }
        if input.evidence.regression_failures > 0 {
            push_unique(&mut reason_codes, "task_skill_regression_failure");
        }
        if validation_status == GeneValidationStatus::Pending {
            push_unique(&mut reason_codes, "task_skill_validation_pending");
        }
        if validation_status == GeneValidationStatus::Failed {
            push_unique(&mut reason_codes, "task_skill_validation_failed");
        }
        if input.operator_decision == GeneScissorsOperatorDecision::Pending {
            push_unique(&mut reason_codes, "task_skill_operator_approval_pending");
        }
        if input.rollback_anchor_id.is_none() {
            push_unique(&mut reason_codes, "task_skill_rollback_anchor_missing");
        }
        if input.clean_room_provenance.is_none() {
            push_unique(
                &mut reason_codes,
                "task_skill_clean_room_provenance_missing",
            );
        }

        let decision = if duplicate_of.is_some() {
            TaskSkillGeneDecision::DuplicateSuppressed
        } else if blocked_payload || conflicts {
            TaskSkillGeneDecision::Quarantine
        } else if validation_status == GeneValidationStatus::Failed {
            TaskSkillGeneDecision::Reject
        } else if stale
            || validation_status == GeneValidationStatus::Pending
            || score_milli < self.policy.min_accept_score_milli
        {
            TaskSkillGeneDecision::HoldForEvidence
        } else {
            TaskSkillGeneDecision::AcceptPreview
        };
        let activation_eligible = decision == TaskSkillGeneDecision::AcceptPreview
            && validation_status == GeneValidationStatus::Passed
            && input.operator_decision == GeneScissorsOperatorDecision::Approved
            && input.rollback_anchor_id.is_some();
        let candidate_id = stable_redaction_digest([
            TASK_SKILL_GENE_SCHEMA_VERSION,
            profile_slug(input.profile),
            &fingerprint,
            decision.as_str(),
            validation_status.as_str(),
            input.operator_decision.as_str(),
        ]);

        TaskSkillGeneCandidate {
            schema_version: TASK_SKILL_GENE_SCHEMA_VERSION,
            candidate_id,
            profile: input.profile,
            language: normalize_label(&input.language),
            domain: normalize_label(&input.domain),
            tool_policy: normalize_label(&input.tool_policy),
            skill_fingerprint_digest: fingerprint,
            prompt_policy_summary_digest: stable_redaction_digest([
                "task-skill-policy",
                input.prompt_policy_summary.trim(),
            ]),
            validation_expectations: sanitized_list(input.validation_expectations),
            failure_modes: sanitized_list(input.failure_modes),
            safe_activation_constraints: sanitized_list(input.safe_activation_constraints),
            clean_room_provenance_digest: input
                .clean_room_provenance
                .as_deref()
                .map(|value| stable_redaction_digest(["task-skill-provenance", value.trim()])),
            rollback_anchor_digest: input
                .rollback_anchor_id
                .as_deref()
                .map(|value| stable_redaction_digest(["task-skill-rollback", value.trim()])),
            duplicate_of,
            decision,
            validation_status,
            operator_decision: input.operator_decision,
            score_milli,
            reason_codes,
            activation_eligible,
            read_only: true,
            write_allowed: false,
            applied: false,
        }
    }
}

fn score_milli(evidence: &TaskSkillGeneEvidence, blocked_payload: bool) -> i32 {
    if blocked_payload || !evidence.privacy_gate_passed {
        return 0;
    }
    let mut score = 360;
    if evidence.compiler_passed {
        score += 120;
    }
    if evidence.tests_passed {
        score += 140;
    }
    if evidence.benchmark_passed {
        score += 120;
    }
    if evidence.multilingual_eval_passed {
        score += 100;
    }
    if evidence.user_approved {
        score += 80;
    }
    score -= (evidence.age.min(12) as i32) * 25;
    score -= (evidence.regression_failures.min(5) as i32) * 140;
    score.clamp(0, 1000)
}

fn skill_fingerprint(input: &TaskSkillGeneInput) -> String {
    stable_redaction_digest([
        "task-skill-fingerprint",
        profile_slug(input.profile),
        &normalize_label(&input.language),
        &normalize_label(&input.domain),
        &normalize_label(&input.tool_policy),
        input.prompt_policy_summary.trim(),
    ])
}

fn input_contains_blocked_payload(input: &TaskSkillGeneInput) -> bool {
    contains_private_or_executable_marker(&input.prompt_policy_summary)
        || input
            .validation_expectations
            .iter()
            .any(|value| contains_private_or_executable_marker(value))
        || input
            .failure_modes
            .iter()
            .any(|value| contains_private_or_executable_marker(value))
}

fn sanitized_list(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .map(|value| {
            if contains_private_or_executable_marker(&value) {
                stable_redaction_digest(["task-skill-redacted-list-item", value.trim()])
            } else {
                value.trim().to_owned()
            }
        })
        .filter(|value| !value.is_empty())
        .collect()
}

fn normalize_label(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace(' ', "_")
}

fn eq_fold(left: &str, right: &str) -> bool {
    normalize_label(left) == normalize_label(right)
}

fn push_unique(values: &mut Vec<String>, value: &str) {
    if !values.iter().any(|existing| existing == value) {
        values.push(value.to_owned());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hierarchy::TaskProfile;

    #[test]
    fn task_skill_candidate_stores_digest_only_policy_summary() {
        let candidate = TaskSkillGeneScorer::default().score_candidate(
            &[],
            base_input().with_operator_decision(GeneScissorsOperatorDecision::Approved),
        );

        assert_eq!(candidate.decision, TaskSkillGeneDecision::AcceptPreview);
        assert!(candidate.activation_eligible);
        assert!(candidate.is_read_only_preview());
        assert!(
            candidate
                .prompt_policy_summary_digest
                .starts_with("redaction-digest:")
        );
        assert!(!candidate.summary_line().contains("prefer compiler"));
        assert!(!candidate.redacted_trace_line().contains("prefer compiler"));
        assert!(
            candidate
                .redacted_trace_line()
                .contains("\"raw_payload_included\":false")
        );
    }

    #[test]
    fn task_skill_duplicate_is_suppressed_by_stable_fingerprint() {
        let scorer = TaskSkillGeneScorer::default();
        let first = scorer.score_candidate(&[], base_input());
        let duplicate = scorer.score_candidate(&[first.clone()], base_input());

        assert_eq!(
            duplicate.decision,
            TaskSkillGeneDecision::DuplicateSuppressed
        );
        assert_eq!(
            duplicate.duplicate_of.as_deref(),
            Some(first.candidate_id.as_str())
        );
        assert!(
            duplicate
                .reason_codes
                .contains(&"task_skill_duplicate_suppressed".to_owned())
        );
        assert!(!duplicate.activation_eligible);
    }

    #[test]
    fn stale_task_skill_decays_to_hold_for_more_evidence() {
        let mut evidence = TaskSkillGeneEvidence::passing();
        evidence.age = 12;
        let candidate = TaskSkillGeneScorer::default()
            .score_candidate(&[], base_input().with_evidence(evidence));

        assert_eq!(candidate.decision, TaskSkillGeneDecision::HoldForEvidence);
        assert!(candidate.score_milli < 760);
        assert!(
            candidate
                .reason_codes
                .contains(&"task_skill_stale_decay".to_owned())
        );
    }

    #[test]
    fn conflicting_task_skill_policy_is_quarantined() {
        let scorer = TaskSkillGeneScorer::default();
        let existing = scorer.score_candidate(&[], base_input());
        let conflict = scorer.score_candidate(
            &[existing],
            base_input().with_tool_policy("allow_remote_shell_tools"),
        );

        assert_eq!(conflict.decision, TaskSkillGeneDecision::Quarantine);
        assert!(
            conflict
                .reason_codes
                .contains(&"task_skill_policy_conflict".to_owned())
        );
        assert!(conflict.is_read_only_preview());
    }

    #[test]
    fn blocked_payload_is_quarantined_and_not_copied_into_trace() {
        let candidate = TaskSkillGeneScorer::default().score_candidate(
            &[],
            base_input()
                .with_validation_expectations(["prompt: copy this private user request"])
                .with_evidence(TaskSkillGeneEvidence::passing()),
        );

        assert_eq!(candidate.decision, TaskSkillGeneDecision::Quarantine);
        assert_eq!(candidate.score_milli, 0);
        assert!(
            candidate
                .validation_expectations
                .iter()
                .all(|value| value.starts_with("redaction-digest:"))
        );
        assert!(
            !candidate
                .redacted_trace_line()
                .contains("private user request")
        );
    }

    #[test]
    fn approval_and_rollback_anchor_gate_activation() {
        let pending = TaskSkillGeneScorer::default().score_candidate(&[], base_input());
        let approved_without_anchor = TaskSkillGeneScorer::default().score_candidate(
            &[],
            base_input_without_anchor()
                .with_operator_decision(GeneScissorsOperatorDecision::Approved),
        );
        let approved = TaskSkillGeneScorer::default().score_candidate(
            &[],
            base_input()
                .with_operator_decision(GeneScissorsOperatorDecision::Approved)
                .with_rollback_anchor("rollback:task-skill:coding"),
        );

        assert!(!pending.activation_eligible);
        assert!(!approved_without_anchor.activation_eligible);
        assert!(approved.activation_eligible);
        assert!(approved.write_allowed == false && approved.applied == false);
    }

    fn base_input() -> TaskSkillGeneInput {
        base_input_without_anchor()
            .with_rollback_anchor("rollback:task-skill:coding")
            .with_evidence(TaskSkillGeneEvidence::passing())
    }

    fn base_input_without_anchor() -> TaskSkillGeneInput {
        TaskSkillGeneInput::new(
            TaskProfile::Coding,
            "zh-CN",
            "rust-repair",
            "prefer compiler checked Rust fixes with short Chinese explanations",
        )
        .with_tool_policy("local_cargo_only")
        .with_validation_expectations(["cargo fmt", "focused cargo test", "trace gate"])
        .with_failure_modes(["compile_error", "test_regression"])
        .with_clean_room_provenance("owner-authored from local validation evidence")
        .with_evidence(TaskSkillGeneEvidence::passing())
    }
}
