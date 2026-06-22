use std::collections::BTreeSet;

use crate::self_evolution::{SelfEvolutionValidationEvidence, SelfEvolutionValidationLane};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NoWeightImprovementLane {
    Memory,
    Gene,
    Routing,
    Tool,
    Runtime,
    AdapterTrainingHandoff,
    ModelWeightMutation,
}

impl NoWeightImprovementLane {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Memory => "memory",
            Self::Gene => "gene",
            Self::Routing => "routing",
            Self::Tool => "tool",
            Self::Runtime => "runtime",
            Self::AdapterTrainingHandoff => "adapter_training_handoff",
            Self::ModelWeightMutation => "model_weight_mutation",
        }
    }

    pub fn is_no_weight_lane(self) -> bool {
        matches!(
            self,
            Self::Memory | Self::Gene | Self::Routing | Self::Tool | Self::Runtime
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoWeightRetrainDecision {
    PromoteNoWeight,
    HoldForEvidence,
    HoldForApproval,
    HandoffProposed,
    Rejected,
}

impl NoWeightRetrainDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PromoteNoWeight => "promote_no_weight",
            Self::HoldForEvidence => "hold_for_evidence",
            Self::HoldForApproval => "hold_for_approval",
            Self::HandoffProposed => "handoff_proposed",
            Self::Rejected => "rejected",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterTrainingHandoffState {
    NotApplicable,
    Disabled,
    ProposedForApproval,
    ApprovedForExternalRun,
    Rejected,
}

impl AdapterTrainingHandoffState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotApplicable => "not_applicable",
            Self::Disabled => "disabled",
            Self::ProposedForApproval => "proposed_for_approval",
            Self::ApprovedForExternalRun => "approved_for_external_run",
            Self::Rejected => "rejected",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NoWeightRetrainPolicy {
    pub allow_adapter_training_handoff: bool,
    pub allow_model_weight_mutation: bool,
    pub require_operator_approval: bool,
    pub require_privacy_evidence: bool,
    pub require_rollback_anchor: bool,
    pub require_validation_passed: bool,
    pub require_experiment_validation: bool,
    pub max_regression_budget: f32,
    pub min_benchmark_delta: f32,
    pub min_saturation_for_adapter_handoff: f32,
}

impl Default for NoWeightRetrainPolicy {
    fn default() -> Self {
        Self {
            allow_adapter_training_handoff: false,
            allow_model_weight_mutation: false,
            require_operator_approval: true,
            require_privacy_evidence: true,
            require_rollback_anchor: true,
            require_validation_passed: true,
            require_experiment_validation: false,
            max_regression_budget: 0.02,
            min_benchmark_delta: 0.0,
            min_saturation_for_adapter_handoff: 0.80,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NoWeightImprovementCandidate {
    pub candidate_id: String,
    pub lane: NoWeightImprovementLane,
    pub rationale_digest: String,
    pub benchmark_delta: f32,
    pub regression_budget: f32,
    pub saturation_score: f32,
    pub rollback_anchor_id: String,
    pub privacy_evidence_id: String,
    pub validation: SelfEvolutionValidationEvidence,
    pub operator_approved: bool,
    pub evidence_ids: Vec<String>,
}

impl NoWeightImprovementCandidate {
    pub fn new(candidate_id: impl Into<String>, lane: NoWeightImprovementLane) -> Self {
        let candidate_id = candidate_id.into();
        Self {
            rationale_digest: stable_digest(&candidate_id),
            candidate_id,
            lane,
            benchmark_delta: 0.0,
            regression_budget: 0.0,
            saturation_score: 0.0,
            rollback_anchor_id: String::new(),
            privacy_evidence_id: String::new(),
            validation: SelfEvolutionValidationEvidence::default(),
            operator_approved: false,
            evidence_ids: Vec::new(),
        }
    }

    pub fn with_rationale(mut self, rationale: impl AsRef<str>) -> Self {
        self.rationale_digest = stable_digest(rationale.as_ref());
        self
    }

    pub fn with_benchmark_delta(mut self, benchmark_delta: f32) -> Self {
        self.benchmark_delta = finite_or_zero(benchmark_delta);
        self
    }

    pub fn with_regression_budget(mut self, regression_budget: f32) -> Self {
        self.regression_budget = finite_or_zero(regression_budget).max(0.0);
        self
    }

    pub fn with_saturation_score(mut self, saturation_score: f32) -> Self {
        self.saturation_score = finite_or_zero(saturation_score).clamp(0.0, 1.0);
        self
    }

    pub fn with_rollback_anchor(mut self, rollback_anchor_id: impl Into<String>) -> Self {
        self.rollback_anchor_id = rollback_anchor_id.into();
        self
    }

    pub fn with_privacy_evidence(mut self, privacy_evidence_id: impl Into<String>) -> Self {
        self.privacy_evidence_id = privacy_evidence_id.into();
        self
    }

    pub fn with_validation(mut self, validation: SelfEvolutionValidationEvidence) -> Self {
        self.validation = validation;
        self
    }

    pub fn with_operator_approval(mut self, operator_approved: bool) -> Self {
        self.operator_approved = operator_approved;
        self
    }

    pub fn with_evidence_id(mut self, evidence_id: impl Into<String>) -> Self {
        push_unique_string(&mut self.evidence_ids, evidence_id);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NoWeightRetrainScorecard {
    pub candidate_id: String,
    pub lane: NoWeightImprovementLane,
    pub decision: NoWeightRetrainDecision,
    pub no_weight_retrain: bool,
    pub adapter_handoff_state: AdapterTrainingHandoffState,
    pub benchmark_delta: f32,
    pub regression_budget: f32,
    pub saturation_score: f32,
    pub rollback_anchor_id: String,
    pub privacy_evidence_present: bool,
    pub validation_passed: bool,
    pub operator_approved: bool,
    pub evidence_digest: String,
    pub evidence_ids: Vec<String>,
    pub blocked_reasons: Vec<String>,
    pub read_only: bool,
    pub report_only: bool,
    pub preview_only: bool,
    pub write_allowed: bool,
    pub adapter_training_execution_allowed: bool,
    pub model_weight_write_allowed: bool,
}

impl NoWeightRetrainScorecard {
    pub fn accepted(&self) -> bool {
        matches!(
            self.decision,
            NoWeightRetrainDecision::PromoteNoWeight | NoWeightRetrainDecision::HandoffProposed
        )
    }

    pub fn summary_line(&self) -> String {
        format!(
            "no_weight_retrain_scorecard candidate={} lane={} decision={} no_weight_retrain={} adapter_handoff={} benchmark_delta={:.6} regression_budget={:.6} saturation={:.6} rollback={} privacy_evidence={} validation_passed={} operator_approved={} blocked={} read_only={} report_only={} preview_only={} write_allowed={} adapter_training_execution_allowed={} model_weight_write_allowed={} evidence_digest={}",
            self.candidate_id,
            self.lane.as_str(),
            self.decision.as_str(),
            self.no_weight_retrain,
            self.adapter_handoff_state.as_str(),
            self.benchmark_delta,
            self.regression_budget,
            self.saturation_score,
            redact_anchor(&self.rollback_anchor_id),
            self.privacy_evidence_present,
            self.validation_passed,
            self.operator_approved,
            self.blocked_reasons.join("|"),
            self.read_only,
            self.report_only,
            self.preview_only,
            self.write_allowed,
            self.adapter_training_execution_allowed,
            self.model_weight_write_allowed,
            self.evidence_digest
        )
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct NoWeightRetrainGate {
    pub policy: NoWeightRetrainPolicy,
}

impl NoWeightRetrainGate {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_policy(mut self, policy: NoWeightRetrainPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn evaluate(&self, candidate: &NoWeightImprovementCandidate) -> NoWeightRetrainScorecard {
        let mut blocked_reasons = Vec::new();

        if candidate.candidate_id.trim().is_empty() {
            blocked_reasons.push("candidate_id_missing".to_owned());
        }
        if self.policy.require_rollback_anchor && candidate.rollback_anchor_id.trim().is_empty() {
            blocked_reasons.push("rollback_anchor_missing".to_owned());
        }
        let privacy_evidence_present = !candidate.privacy_evidence_id.trim().is_empty();
        if self.policy.require_privacy_evidence && !privacy_evidence_present {
            blocked_reasons.push("privacy_evidence_missing".to_owned());
        }
        let validation_passed = validation_passed(
            candidate.validation,
            self.policy.require_experiment_validation,
        );
        if self.policy.require_validation_passed && !validation_passed {
            push_validation_blockers(&mut blocked_reasons, candidate.validation, &self.policy);
        }
        if candidate.benchmark_delta < self.policy.min_benchmark_delta {
            blocked_reasons.push(format!(
                "benchmark_delta_below_min:{:.6}<{}",
                candidate.benchmark_delta, self.policy.min_benchmark_delta
            ));
        }
        if candidate.regression_budget > self.policy.max_regression_budget {
            blocked_reasons.push(format!(
                "regression_budget_exceeded:{:.6}>{:.6}",
                candidate.regression_budget, self.policy.max_regression_budget
            ));
        }

        let mut adapter_handoff_state = AdapterTrainingHandoffState::NotApplicable;
        let mut no_weight_retrain = candidate.lane.is_no_weight_lane();
        match candidate.lane {
            NoWeightImprovementLane::AdapterTrainingHandoff => {
                no_weight_retrain = false;
                if !self.policy.allow_adapter_training_handoff {
                    adapter_handoff_state = AdapterTrainingHandoffState::Disabled;
                    blocked_reasons.push("adapter_training_handoff_disabled".to_owned());
                } else if candidate.saturation_score
                    < self.policy.min_saturation_for_adapter_handoff
                {
                    adapter_handoff_state = AdapterTrainingHandoffState::ProposedForApproval;
                    blocked_reasons.push(format!(
                        "adapter_handoff_saturation_below_min:{:.6}<{}",
                        candidate.saturation_score, self.policy.min_saturation_for_adapter_handoff
                    ));
                } else if self.policy.require_operator_approval && !candidate.operator_approved {
                    adapter_handoff_state = AdapterTrainingHandoffState::ProposedForApproval;
                    blocked_reasons.push("operator_approval_missing".to_owned());
                } else {
                    adapter_handoff_state = AdapterTrainingHandoffState::ApprovedForExternalRun;
                }
            }
            NoWeightImprovementLane::ModelWeightMutation => {
                no_weight_retrain = false;
                adapter_handoff_state = AdapterTrainingHandoffState::Rejected;
                if !self.policy.allow_model_weight_mutation {
                    blocked_reasons.push("model_weight_mutation_disabled".to_owned());
                }
                blocked_reasons
                    .push("direct_model_weight_mutation_forbidden_use_handoff".to_owned());
            }
            _ => {
                if self.policy.require_operator_approval && !candidate.operator_approved {
                    blocked_reasons.push("operator_approval_missing".to_owned());
                }
            }
        }

        let decision = decision_for(candidate, adapter_handoff_state, &blocked_reasons);
        let evidence_digest = scorecard_digest(candidate, &blocked_reasons, decision);

        NoWeightRetrainScorecard {
            candidate_id: sanitize_id(&candidate.candidate_id),
            lane: candidate.lane,
            decision,
            no_weight_retrain,
            adapter_handoff_state,
            benchmark_delta: candidate.benchmark_delta,
            regression_budget: candidate.regression_budget,
            saturation_score: candidate.saturation_score,
            rollback_anchor_id: redact_anchor(&candidate.rollback_anchor_id),
            privacy_evidence_present,
            validation_passed,
            operator_approved: candidate.operator_approved,
            evidence_digest,
            evidence_ids: redacted_evidence_ids(&candidate.evidence_ids),
            blocked_reasons,
            read_only: true,
            report_only: true,
            preview_only: true,
            write_allowed: false,
            adapter_training_execution_allowed: false,
            model_weight_write_allowed: false,
        }
    }
}

fn decision_for(
    candidate: &NoWeightImprovementCandidate,
    adapter_handoff_state: AdapterTrainingHandoffState,
    blocked_reasons: &[String],
) -> NoWeightRetrainDecision {
    match candidate.lane {
        NoWeightImprovementLane::AdapterTrainingHandoff => match adapter_handoff_state {
            AdapterTrainingHandoffState::ApprovedForExternalRun if blocked_reasons.is_empty() => {
                NoWeightRetrainDecision::HandoffProposed
            }
            AdapterTrainingHandoffState::ProposedForApproval => {
                NoWeightRetrainDecision::HoldForApproval
            }
            AdapterTrainingHandoffState::Disabled | AdapterTrainingHandoffState::Rejected => {
                NoWeightRetrainDecision::Rejected
            }
            AdapterTrainingHandoffState::NotApplicable
            | AdapterTrainingHandoffState::ApprovedForExternalRun => {
                NoWeightRetrainDecision::HoldForEvidence
            }
        },
        NoWeightImprovementLane::ModelWeightMutation => NoWeightRetrainDecision::Rejected,
        _ if blocked_reasons.is_empty() => NoWeightRetrainDecision::PromoteNoWeight,
        _ if blocked_reasons
            .iter()
            .any(|reason| reason == "operator_approval_missing") =>
        {
            NoWeightRetrainDecision::HoldForApproval
        }
        _ => NoWeightRetrainDecision::HoldForEvidence,
    }
}

fn validation_passed(
    validation: SelfEvolutionValidationEvidence,
    require_experiment: bool,
) -> bool {
    lane_passed(validation.compiler)
        && lane_passed(validation.tests)
        && lane_passed(validation.benchmarks)
        && (!require_experiment || lane_passed(validation.experiments))
        && validation.experiments.failed == 0
}

fn lane_passed(lane: SelfEvolutionValidationLane) -> bool {
    lane.items > 0
        && lane.passed > 0
        && lane.failed == 0
        && lane.passed.saturating_add(lane.failed) <= lane.items
}

fn push_validation_blockers(
    blocked_reasons: &mut Vec<String>,
    validation: SelfEvolutionValidationEvidence,
    policy: &NoWeightRetrainPolicy,
) {
    push_lane_blocker(blocked_reasons, "compiler", validation.compiler);
    push_lane_blocker(blocked_reasons, "tests", validation.tests);
    push_lane_blocker(blocked_reasons, "benchmarks", validation.benchmarks);
    if policy.require_experiment_validation || validation.experiments.failed > 0 {
        push_lane_blocker(blocked_reasons, "experiments", validation.experiments);
    }
}

fn push_lane_blocker(
    blocked_reasons: &mut Vec<String>,
    lane_name: &str,
    lane: SelfEvolutionValidationLane,
) {
    if lane.items == 0 || lane.passed == 0 {
        blocked_reasons.push(format!("{lane_name}_validation_missing"));
    }
    if lane.failed > 0 {
        blocked_reasons.push(format!("{lane_name}_validation_failed:{}", lane.failed));
    }
    if lane.passed.saturating_add(lane.failed) > lane.items {
        blocked_reasons.push(format!("{lane_name}_validation_accounting_invalid"));
    }
}

fn scorecard_digest(
    candidate: &NoWeightImprovementCandidate,
    blocked_reasons: &[String],
    decision: NoWeightRetrainDecision,
) -> String {
    stable_digest(&format!(
        "{}:{}:{}:{:.6}:{:.6}:{:.6}:{}:{}:{}:{}",
        candidate.candidate_id,
        candidate.lane.as_str(),
        candidate.rationale_digest,
        candidate.benchmark_delta,
        candidate.regression_budget,
        candidate.saturation_score,
        candidate.rollback_anchor_id,
        candidate.privacy_evidence_id,
        decision.as_str(),
        blocked_reasons.join("|")
    ))
}

fn redacted_evidence_ids(ids: &[String]) -> Vec<String> {
    ids.iter()
        .map(|id| {
            if let Some((prefix, _)) = id.split_once(':') {
                format!("{}:{}", sanitize_id(prefix), stable_digest(id))
            } else {
                format!("evidence:{}", stable_digest(id))
            }
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn redact_anchor(value: &str) -> String {
    if value.trim().is_empty() {
        "missing".to_owned()
    } else if value.contains(':') {
        sanitize_id(value)
    } else {
        stable_digest(value)
    }
}

fn sanitize_id(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, ':' | '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .chars()
        .take(120)
        .collect()
}

fn push_unique_string(values: &mut Vec<String>, value: impl Into<String>) {
    let value = value.into();
    if !value.trim().is_empty() && !values.contains(&value) {
        values.push(value);
    }
}

fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

fn stable_digest(value: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("digest:{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn promotes_approved_no_weight_memory_candidate() {
        let candidate = valid_candidate("candidate:memory", NoWeightImprovementLane::Memory)
            .with_operator_approval(true)
            .with_benchmark_delta(0.06)
            .with_regression_budget(0.01);

        let scorecard = NoWeightRetrainGate::new().evaluate(&candidate);

        assert_eq!(scorecard.decision, NoWeightRetrainDecision::PromoteNoWeight);
        assert!(scorecard.no_weight_retrain);
        assert!(scorecard.accepted());
        assert_eq!(scorecard.lane, NoWeightImprovementLane::Memory);
        assert!(!scorecard.write_allowed);
        assert!(!scorecard.model_weight_write_allowed);
        assert!(!scorecard.adapter_training_execution_allowed);
        assert!(scorecard.summary_line().contains("no_weight_retrain=true"));
    }

    #[test]
    fn classifies_no_weight_lanes() {
        for lane in [
            NoWeightImprovementLane::Memory,
            NoWeightImprovementLane::Gene,
            NoWeightImprovementLane::Routing,
            NoWeightImprovementLane::Tool,
            NoWeightImprovementLane::Runtime,
        ] {
            let candidate = valid_candidate(format!("candidate:{}", lane.as_str()), lane)
                .with_operator_approval(true);
            let scorecard = NoWeightRetrainGate::new().evaluate(&candidate);
            assert_eq!(scorecard.decision, NoWeightRetrainDecision::PromoteNoWeight);
            assert!(scorecard.no_weight_retrain);
            assert_eq!(scorecard.lane.as_str(), lane.as_str());
        }
    }

    #[test]
    fn default_policy_rejects_adapter_training_handoff() {
        let candidate = valid_candidate(
            "candidate:adapter",
            NoWeightImprovementLane::AdapterTrainingHandoff,
        )
        .with_operator_approval(true)
        .with_saturation_score(0.95);

        let scorecard = NoWeightRetrainGate::new().evaluate(&candidate);

        assert_eq!(scorecard.decision, NoWeightRetrainDecision::Rejected);
        assert_eq!(
            scorecard.adapter_handoff_state,
            AdapterTrainingHandoffState::Disabled
        );
        assert!(!scorecard.no_weight_retrain);
        assert!(
            scorecard
                .blocked_reasons
                .contains(&"adapter_training_handoff_disabled".to_owned())
        );
        assert!(!scorecard.adapter_training_execution_allowed);
    }

    #[test]
    fn adapter_handoff_requires_saturation_and_approval_when_enabled() {
        let policy = NoWeightRetrainPolicy {
            allow_adapter_training_handoff: true,
            ..NoWeightRetrainPolicy::default()
        };
        let low_saturation = valid_candidate(
            "candidate:adapter-low",
            NoWeightImprovementLane::AdapterTrainingHandoff,
        )
        .with_operator_approval(true)
        .with_saturation_score(0.30);
        let missing_approval = valid_candidate(
            "candidate:adapter-approval",
            NoWeightImprovementLane::AdapterTrainingHandoff,
        )
        .with_saturation_score(0.95);
        let approved = valid_candidate(
            "candidate:adapter-approved",
            NoWeightImprovementLane::AdapterTrainingHandoff,
        )
        .with_operator_approval(true)
        .with_saturation_score(0.95);

        let gate = NoWeightRetrainGate::new().with_policy(policy);
        let low = gate.evaluate(&low_saturation);
        let held = gate.evaluate(&missing_approval);
        let ready = gate.evaluate(&approved);

        assert_eq!(low.decision, NoWeightRetrainDecision::HoldForApproval);
        assert!(
            low.blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("adapter_handoff_saturation_below_min"))
        );
        assert_eq!(held.decision, NoWeightRetrainDecision::HoldForApproval);
        assert!(
            held.blocked_reasons
                .contains(&"operator_approval_missing".to_owned())
        );
        assert_eq!(ready.decision, NoWeightRetrainDecision::HandoffProposed);
        assert_eq!(
            ready.adapter_handoff_state,
            AdapterTrainingHandoffState::ApprovedForExternalRun
        );
        assert!(!ready.adapter_training_execution_allowed);
        assert!(!ready.write_allowed);
    }

    #[test]
    fn rejects_direct_weight_mutation_even_with_approval() {
        let candidate = valid_candidate(
            "candidate:weight",
            NoWeightImprovementLane::ModelWeightMutation,
        )
        .with_operator_approval(true);

        let scorecard = NoWeightRetrainGate::new().evaluate(&candidate);

        assert_eq!(scorecard.decision, NoWeightRetrainDecision::Rejected);
        assert!(!scorecard.model_weight_write_allowed);
        assert!(
            scorecard
                .blocked_reasons
                .contains(&"model_weight_mutation_disabled".to_owned())
        );
        assert!(
            scorecard
                .blocked_reasons
                .contains(&"direct_model_weight_mutation_forbidden_use_handoff".to_owned())
        );
    }

    #[test]
    fn holds_for_evidence_without_validation_or_rollback() {
        let candidate =
            NoWeightImprovementCandidate::new("candidate:weak", NoWeightImprovementLane::Routing)
                .with_privacy_evidence("privacy:redacted")
                .with_operator_approval(true);

        let scorecard = NoWeightRetrainGate::new().evaluate(&candidate);

        assert_eq!(scorecard.decision, NoWeightRetrainDecision::HoldForEvidence);
        assert!(
            scorecard
                .blocked_reasons
                .contains(&"rollback_anchor_missing".to_owned())
        );
        assert!(
            scorecard
                .blocked_reasons
                .contains(&"compiler_validation_missing".to_owned())
        );
        assert!(
            scorecard
                .blocked_reasons
                .contains(&"tests_validation_missing".to_owned())
        );
        assert!(
            scorecard
                .blocked_reasons
                .contains(&"benchmarks_validation_missing".to_owned())
        );
    }

    #[test]
    fn scorecard_redacts_raw_rationale_and_payload_like_evidence() {
        let secret = "SECRET_TRAINING_PAYLOAD_DO_NOT_LOG";
        let candidate = valid_candidate("candidate:redact", NoWeightImprovementLane::Tool)
            .with_operator_approval(true)
            .with_rationale(secret)
            .with_evidence_id(secret);

        let scorecard = NoWeightRetrainGate::new().evaluate(&candidate);
        let summary = scorecard.summary_line();

        assert_eq!(scorecard.decision, NoWeightRetrainDecision::PromoteNoWeight);
        assert!(!summary.contains(secret));
        assert!(!scorecard.evidence_digest.contains(secret));
        assert!(scorecard.evidence_ids.iter().all(|id| !id.contains(secret)));
    }

    fn valid_candidate(
        id: impl Into<String>,
        lane: NoWeightImprovementLane,
    ) -> NoWeightImprovementCandidate {
        NoWeightImprovementCandidate::new(id, lane)
            .with_rollback_anchor("rollback:no-weight")
            .with_privacy_evidence("privacy:redacted")
            .with_validation(passing_validation())
            .with_benchmark_delta(0.03)
            .with_regression_budget(0.01)
            .with_evidence_id("compiler:passed")
            .with_evidence_id("tests:passed")
            .with_evidence_id("benchmarks:passed")
    }

    fn passing_validation() -> SelfEvolutionValidationEvidence {
        SelfEvolutionValidationEvidence::from_lanes(
            SelfEvolutionValidationLane::new(1, 1, 0),
            SelfEvolutionValidationLane::new(1, 1, 0),
            SelfEvolutionValidationLane::new(1, 1, 0),
            SelfEvolutionValidationLane::default(),
        )
    }
}
