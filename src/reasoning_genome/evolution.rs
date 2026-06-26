use crate::hierarchy::TaskProfile;
use crate::privacy_redaction::stable_redaction_digest;
use crate::writer_gate::{
    UNIFIED_WRITER_GATE_SCHEMA_VERSION, UnifiedWriterGateDecision, UnifiedWriterGateReport,
};

use super::model::profile_slug;
use super::{
    DnaSplicePreview, GeneScissorsIntent, GeneScissorsOperatorDecision,
    GeneScissorsTransactionJournal, GeneValidationStatus, GenomeExpression, MutationPlan,
};

pub const DNA_EVOLUTION_CONTROLLER_SCHEMA_VERSION: &str = "dna_evolution_controller_v1";
pub const DNA_EVOLUTION_APPLY_PLAN_SCHEMA_VERSION: &str = "dna_evolution_apply_plan_v1";
pub const DNA_EVOLUTION_APPLY_PLAN_TRACE_SCHEMA: &str = "rust-norion-dna-evolution-apply-plan-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DnaEvolutionPolicy {
    pub mutation_budget: usize,
    pub min_validation_artifacts: usize,
    pub require_rollback_replay: bool,
    pub require_operator_approval_for_activation: bool,
    pub max_fitness_regression_milli: i32,
}

impl Default for DnaEvolutionPolicy {
    fn default() -> Self {
        Self {
            mutation_budget: 8,
            min_validation_artifacts: 4,
            require_rollback_replay: true,
            require_operator_approval_for_activation: true,
            max_fitness_regression_milli: -50,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnaEvolutionValidationStatus {
    Missing,
    Passed,
    Failed,
}

impl DnaEvolutionValidationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Passed => "passed",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnaEvolutionValidationEvidence {
    pub compiler_passed: bool,
    pub tests_passed: bool,
    pub benchmark_passed: bool,
    pub trace_gate_passed: bool,
    pub privacy_gate_passed: bool,
    pub canary_replay_passed: bool,
    pub rollback_replay_passed: bool,
    pub artifact_digests: Vec<String>,
}

impl Default for DnaEvolutionValidationEvidence {
    fn default() -> Self {
        Self {
            compiler_passed: false,
            tests_passed: false,
            benchmark_passed: false,
            trace_gate_passed: false,
            privacy_gate_passed: false,
            canary_replay_passed: false,
            rollback_replay_passed: false,
            artifact_digests: Vec::new(),
        }
    }
}

impl DnaEvolutionValidationEvidence {
    pub fn passing() -> Self {
        Self {
            compiler_passed: true,
            tests_passed: true,
            benchmark_passed: true,
            trace_gate_passed: true,
            privacy_gate_passed: true,
            canary_replay_passed: true,
            rollback_replay_passed: true,
            artifact_digests: vec![
                redacted_ref("cargo-check"),
                redacted_ref("focused-tests"),
                redacted_ref("benchmark-gate"),
                redacted_ref("trace-schema-gate"),
            ],
        }
    }

    pub fn failed_tests() -> Self {
        Self {
            compiler_passed: true,
            tests_passed: false,
            benchmark_passed: true,
            trace_gate_passed: true,
            privacy_gate_passed: true,
            canary_replay_passed: true,
            rollback_replay_passed: true,
            artifact_digests: vec![
                redacted_ref("cargo-check"),
                redacted_ref("focused-tests-failed"),
                redacted_ref("benchmark-gate"),
                redacted_ref("trace-schema-gate"),
            ],
        }
    }

    pub fn with_artifact_digest(mut self, digest: impl Into<String>) -> Self {
        push_unique(&mut self.artifact_digests, redacted_ref(&digest.into()));
        self
    }

    pub fn status(&self, policy: DnaEvolutionPolicy) -> DnaEvolutionValidationStatus {
        if self.artifact_digests.len() < policy.min_validation_artifacts {
            return DnaEvolutionValidationStatus::Missing;
        }
        if self.compiler_passed
            && self.tests_passed
            && self.benchmark_passed
            && self.trace_gate_passed
            && self.privacy_gate_passed
            && self.canary_replay_passed
            && self.rollback_replay_passed
        {
            DnaEvolutionValidationStatus::Passed
        } else {
            DnaEvolutionValidationStatus::Failed
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnaEvolutionCandidateDecision {
    CandidatePreview,
    Hold,
    Reject,
    Rollback,
}

impl DnaEvolutionCandidateDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CandidatePreview => "candidate_preview",
            Self::Hold => "hold",
            Self::Reject => "reject",
            Self::Rollback => "rollback",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnaEvolutionCandidate {
    pub candidate_id: String,
    pub generation_id: String,
    pub parent_anchor_ids: Vec<String>,
    pub stable_anchor_id: String,
    pub rollback_anchor_id: String,
    pub source_plan_id: String,
    pub target_gene_id: String,
    pub replacement_gene_id: Option<String>,
    pub intent: GeneScissorsIntent,
    pub decision: DnaEvolutionCandidateDecision,
    pub validation_status: DnaEvolutionValidationStatus,
    pub operator_decision: GeneScissorsOperatorDecision,
    pub fitness_delta_milli: i32,
    pub validation_artifact_digests: Vec<String>,
    pub reason_codes: Vec<String>,
    pub activation_eligible: bool,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl DnaEvolutionCandidate {
    pub fn is_read_only_preview(&self) -> bool {
        self.read_only && !self.write_allowed && !self.applied
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnaEvolutionControllerReport {
    pub schema_version: &'static str,
    pub profile: TaskProfile,
    pub generation_id: String,
    pub parent_anchor_ids: Vec<String>,
    pub stable_anchor_id: String,
    pub mutation_budget: usize,
    pub validation_status: DnaEvolutionValidationStatus,
    pub operator_decision: GeneScissorsOperatorDecision,
    pub transaction_replay_count: usize,
    pub transaction_replay_passed: bool,
    pub transaction_replay_blocked_count: usize,
    pub total_fitness_delta_milli: i32,
    pub min_fitness_delta_milli: i32,
    pub max_fitness_delta_milli: i32,
    pub candidates: Vec<DnaEvolutionCandidate>,
    pub blocked_reasons: Vec<String>,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl DnaEvolutionControllerReport {
    pub fn candidate_count(&self) -> usize {
        self.candidates.len()
    }

    pub fn decision_count(&self, decision: DnaEvolutionCandidateDecision) -> usize {
        self.candidates
            .iter()
            .filter(|candidate| candidate.decision == decision)
            .count()
    }

    pub fn intent_count(&self, intent: GeneScissorsIntent) -> usize {
        self.candidates
            .iter()
            .filter(|candidate| candidate.intent == intent)
            .count()
    }

    pub fn activation_eligible_count(&self) -> usize {
        self.candidates
            .iter()
            .filter(|candidate| candidate.activation_eligible)
            .count()
    }

    pub fn is_read_only_preview(&self) -> bool {
        self.read_only
            && !self.write_allowed
            && !self.applied
            && self
                .candidates
                .iter()
                .all(DnaEvolutionCandidate::is_read_only_preview)
    }

    pub fn fitness_delta_summary(&self) -> String {
        format!(
            "total={} min={} max={}",
            self.total_fitness_delta_milli,
            self.min_fitness_delta_milli,
            self.max_fitness_delta_milli
        )
    }

    pub fn summary_line(&self) -> String {
        format!(
            "dna_evolution_controller schema={} generation={} candidates={} candidate_preview={} hold={} reject={} rollback={} activation_eligible={} validation={} approval={} replay_count={} replay_passed={} fitness_delta_total={} blocked_reasons={} read_only={} write_allowed={} applied={}",
            self.schema_version,
            self.generation_id,
            self.candidate_count(),
            self.decision_count(DnaEvolutionCandidateDecision::CandidatePreview),
            self.decision_count(DnaEvolutionCandidateDecision::Hold),
            self.decision_count(DnaEvolutionCandidateDecision::Reject),
            self.decision_count(DnaEvolutionCandidateDecision::Rollback),
            self.activation_eligible_count(),
            self.validation_status.as_str(),
            self.operator_decision.as_str(),
            self.transaction_replay_count,
            self.transaction_replay_passed,
            self.total_fitness_delta_milli,
            self.blocked_reasons.len(),
            self.read_only,
            self.write_allowed,
            self.applied
        )
    }

    pub fn redacted_trace_line(&self) -> String {
        format!(
            "{{\"schema\":\"{}\",\"generation_id\":\"{}\",\"parent_anchors\":{},\"stable_anchor\":\"{}\",\"profile\":\"{}\",\"candidate_count\":{},\"candidate_preview\":{},\"hold\":{},\"reject\":{},\"rollback\":{},\"activation_eligible\":{},\"fitness_delta_summary\":\"{}\",\"validation_status\":\"{}\",\"approval_status\":\"{}\",\"transaction_replay\":{{\"count\":{},\"passed\":{},\"blocked\":{}}},\"read_only\":{},\"write_allowed\":{},\"applied\":{},\"raw_payload_included\":false}}",
            json_escape(self.schema_version),
            json_escape(&self.generation_id),
            json_string_array(&self.parent_anchor_ids),
            json_escape(&self.stable_anchor_id),
            profile_slug(self.profile),
            self.candidate_count(),
            self.decision_count(DnaEvolutionCandidateDecision::CandidatePreview),
            self.decision_count(DnaEvolutionCandidateDecision::Hold),
            self.decision_count(DnaEvolutionCandidateDecision::Reject),
            self.decision_count(DnaEvolutionCandidateDecision::Rollback),
            self.activation_eligible_count(),
            json_escape(&self.fitness_delta_summary()),
            self.validation_status.as_str(),
            self.operator_decision.as_str(),
            self.transaction_replay_count,
            self.transaction_replay_passed,
            self.transaction_replay_blocked_count,
            self.read_only,
            self.write_allowed,
            self.applied
        )
    }

    pub fn explicit_apply_plan(
        &self,
        writer_gate: &UnifiedWriterGateReport,
    ) -> DnaEvolutionApplyPlan {
        let activation_eligible = self.activation_eligible_count();
        let candidate_rejects = self.decision_count(DnaEvolutionCandidateDecision::Reject)
            + self.decision_count(DnaEvolutionCandidateDecision::Rollback);
        let source_safe = self.is_read_only_preview()
            && self.validation_status == DnaEvolutionValidationStatus::Passed
            && self.operator_decision == GeneScissorsOperatorDecision::Approved
            && self.transaction_replay_passed;
        let writer_ready = writer_gate.schema_version == UNIFIED_WRITER_GATE_SCHEMA_VERSION
            && writer_gate.decision == UnifiedWriterGateDecision::ReadyForExplicitApply
            && writer_gate.genome_records > 0
            && writer_gate.ready_records > 0
            && writer_gate.durable_write_allowed
            && writer_gate.explicit_apply_required
            && !writer_gate.applied;
        let mut reason_codes = Vec::new();
        if !self.is_read_only_preview() {
            push_unique(
                &mut reason_codes,
                "dna_evolution_source_not_preview_only".to_owned(),
            );
        }
        if self.validation_status != DnaEvolutionValidationStatus::Passed {
            push_unique(
                &mut reason_codes,
                "dna_evolution_validation_not_passed".to_owned(),
            );
        }
        if self.operator_decision != GeneScissorsOperatorDecision::Approved {
            push_unique(
                &mut reason_codes,
                "dna_evolution_operator_approval_missing".to_owned(),
            );
        }
        if !self.transaction_replay_passed {
            push_unique(
                &mut reason_codes,
                "dna_evolution_replay_not_passed".to_owned(),
            );
        }
        if activation_eligible == 0 {
            push_unique(
                &mut reason_codes,
                "dna_evolution_no_activation_eligible_candidate".to_owned(),
            );
        }
        if !writer_ready {
            push_unique(
                &mut reason_codes,
                "dna_evolution_writer_gate_not_ready".to_owned(),
            );
        }
        if writer_gate.applied || writer_gate.write_allowed != writer_gate.durable_write_allowed {
            push_unique(
                &mut reason_codes,
                "dna_evolution_writer_gate_state_invalid".to_owned(),
            );
        }

        let decision =
            if writer_gate.applied || !self.read_only || self.write_allowed || self.applied {
                DnaEvolutionApplyDecision::Rejected
            } else if source_safe && writer_ready && activation_eligible > 0 {
                DnaEvolutionApplyDecision::ReadyForExplicitApply
            } else if !source_safe || activation_eligible == 0 || candidate_rejects > 0 {
                DnaEvolutionApplyDecision::HeldForCandidateState
            } else {
                DnaEvolutionApplyDecision::HeldForWriterGate
            };
        let ready_candidates = if decision == DnaEvolutionApplyDecision::ReadyForExplicitApply {
            activation_eligible
        } else {
            0
        };
        let rejected_candidates = if decision == DnaEvolutionApplyDecision::Rejected {
            self.candidate_count()
        } else {
            candidate_rejects
        };
        let held_candidates = self
            .candidate_count()
            .saturating_sub(ready_candidates)
            .saturating_sub(rejected_candidates);
        let candidate_digest = stable_redaction_digest(
            self.candidates
                .iter()
                .map(|candidate| candidate.candidate_id.as_str())
                .chain([writer_gate.evidence_digest.as_str()]),
        );
        let apply_plan_digest = stable_redaction_digest([
            DNA_EVOLUTION_APPLY_PLAN_SCHEMA_VERSION,
            self.generation_id.as_str(),
            writer_gate.evidence_digest.as_str(),
            decision.as_str(),
            &ready_candidates.to_string(),
            &held_candidates.to_string(),
            &rejected_candidates.to_string(),
            candidate_digest.as_str(),
        ]);

        DnaEvolutionApplyPlan {
            schema_version: DNA_EVOLUTION_APPLY_PLAN_SCHEMA_VERSION,
            trace_schema: DNA_EVOLUTION_APPLY_PLAN_TRACE_SCHEMA,
            controller_schema_version: self.schema_version,
            writer_gate_schema_version: writer_gate.schema_version,
            writer_gate_decision: writer_gate.decision,
            generation_id: self.generation_id.clone(),
            decision,
            candidate_count: self.candidate_count(),
            ready_candidates,
            held_candidates,
            rejected_candidates,
            reason_code_count: reason_codes.len(),
            candidate_digest,
            apply_plan_digest,
            explicit_apply_required: writer_gate.explicit_apply_required || ready_candidates > 0,
            read_only: true,
            write_allowed: false,
            applied: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnaEvolutionApplyDecision {
    ReadyForExplicitApply,
    HeldForWriterGate,
    HeldForCandidateState,
    Rejected,
}

impl DnaEvolutionApplyDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReadyForExplicitApply => "ready_for_explicit_apply",
            Self::HeldForWriterGate => "held_for_writer_gate",
            Self::HeldForCandidateState => "held_for_candidate_state",
            Self::Rejected => "rejected",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnaEvolutionApplyPlan {
    pub schema_version: &'static str,
    pub trace_schema: &'static str,
    pub controller_schema_version: &'static str,
    pub writer_gate_schema_version: &'static str,
    pub writer_gate_decision: UnifiedWriterGateDecision,
    pub generation_id: String,
    pub decision: DnaEvolutionApplyDecision,
    pub candidate_count: usize,
    pub ready_candidates: usize,
    pub held_candidates: usize,
    pub rejected_candidates: usize,
    pub reason_code_count: usize,
    pub candidate_digest: String,
    pub apply_plan_digest: String,
    pub explicit_apply_required: bool,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl DnaEvolutionApplyPlan {
    pub fn is_preview_only(&self) -> bool {
        self.read_only && !self.write_allowed && !self.applied
    }

    pub fn passed(&self) -> bool {
        self.is_preview_only()
            && self.candidate_count
                == self
                    .ready_candidates
                    .saturating_add(self.held_candidates)
                    .saturating_add(self.rejected_candidates)
            && self.candidate_digest.starts_with("redaction-digest:")
            && self.apply_plan_digest.starts_with("redaction-digest:")
            && (self.ready_candidates == 0 || self.explicit_apply_required)
    }

    pub fn summary_line(&self) -> String {
        format!(
            "dna_evolution_apply_plan schema={} decision={} writer_gate_decision={} candidates={} ready={} held={} rejected={} reasons={} explicit_apply_required={} read_only={} write_allowed={} applied={} digest={}",
            self.schema_version,
            self.decision.as_str(),
            self.writer_gate_decision.as_str(),
            self.candidate_count,
            self.ready_candidates,
            self.held_candidates,
            self.rejected_candidates,
            self.reason_code_count,
            self.explicit_apply_required,
            self.read_only,
            self.write_allowed,
            self.applied,
            self.apply_plan_digest
        )
    }

    pub fn json_line(&self) -> String {
        format!(
            "{{\"schema\":\"{}\",\"plan_schema\":\"{}\",\"controller_schema\":\"{}\",\"writer_gate_schema\":\"{}\",\"decision\":\"{}\",\"writer_gate_decision\":\"{}\",\"generation_id\":\"{}\",\"candidates\":{},\"ready_candidates\":{},\"held_candidates\":{},\"rejected_candidates\":{},\"reason_code_count\":{},\"explicit_apply_required\":{},\"candidate_digest\":\"{}\",\"apply_plan_digest\":\"{}\",\"read_only\":{},\"write_allowed\":{},\"applied\":{},\"summary\":\"{}\"}}",
            json_escape(self.trace_schema),
            json_escape(self.schema_version),
            json_escape(self.controller_schema_version),
            json_escape(self.writer_gate_schema_version),
            json_escape(self.decision.as_str()),
            json_escape(self.writer_gate_decision.as_str()),
            json_escape(&self.generation_id),
            self.candidate_count,
            self.ready_candidates,
            self.held_candidates,
            self.rejected_candidates,
            self.reason_code_count,
            self.explicit_apply_required,
            json_escape(&self.candidate_digest),
            json_escape(&self.apply_plan_digest),
            self.read_only,
            self.write_allowed,
            self.applied,
            json_escape(&self.summary_line())
        )
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DnaEvolutionController {
    pub policy: DnaEvolutionPolicy,
}

impl DnaEvolutionController {
    pub fn new(policy: DnaEvolutionPolicy) -> Self {
        Self { policy }
    }

    pub fn preview_expression(
        &self,
        expression: &GenomeExpression,
        validation: &DnaEvolutionValidationEvidence,
        operator_decision: GeneScissorsOperatorDecision,
    ) -> DnaEvolutionControllerReport {
        let journal = GeneScissorsTransactionJournal::from_mutation_plans(
            expression.profile,
            expression.stable_anchor_id.clone(),
            &expression.mutation_plans,
        );
        self.preview_plans(
            expression.profile,
            expression.genome_id.as_str(),
            expression.stable_anchor_id.as_str(),
            &expression.mutation_plans,
            validation,
            operator_decision,
            Some(&journal),
        )
    }

    pub fn preview_splice(
        &self,
        preview: &DnaSplicePreview,
        validation: &DnaEvolutionValidationEvidence,
        operator_decision: GeneScissorsOperatorDecision,
    ) -> DnaEvolutionControllerReport {
        let journal = GeneScissorsTransactionJournal::from_splice_preview(preview);
        self.preview_plans(
            preview.profile,
            "splice-preview",
            preview.stable_anchor_id.as_str(),
            &preview.mutation_plans,
            validation,
            operator_decision,
            Some(&journal),
        )
    }

    pub fn preview_plans(
        &self,
        profile: TaskProfile,
        parent_anchor_id: impl AsRef<str>,
        stable_anchor_id: impl AsRef<str>,
        plans: &[MutationPlan],
        validation: &DnaEvolutionValidationEvidence,
        operator_decision: GeneScissorsOperatorDecision,
        journal: Option<&GeneScissorsTransactionJournal>,
    ) -> DnaEvolutionControllerReport {
        let parent_anchor_id = redacted_ref(parent_anchor_id.as_ref());
        let stable_anchor_id = redacted_ref(stable_anchor_id.as_ref());
        let validation_status = validation.status(self.policy);
        let replay = journal.map(GeneScissorsTransactionJournal::replay);
        let transaction_replay_count = replay
            .as_ref()
            .map(|report| report.transaction_count)
            .unwrap_or(0);
        let transaction_replay_passed = replay
            .as_ref()
            .map(|report| {
                report.passed_preview_gate()
                    && report.transaction_count >= plans.len()
                    && report.duplicate_suppressed_count == 0
            })
            .unwrap_or(plans.is_empty());
        let transaction_replay_blocked_count = replay
            .as_ref()
            .map(|report| report.active_expression_excluded_segments.len())
            .unwrap_or(0);
        let generation_id = stable_redaction_digest([
            DNA_EVOLUTION_CONTROLLER_SCHEMA_VERSION,
            profile_slug(profile),
            parent_anchor_id.as_str(),
            stable_anchor_id.as_str(),
            &plans.len().to_string(),
            validation_status.as_str(),
            operator_decision.as_str(),
        ]);
        let mut blocked_reasons = Vec::new();
        if self.policy.mutation_budget == 0 && !plans.is_empty() {
            push_unique(
                &mut blocked_reasons,
                "dna_evolution_mutation_budget_zero".to_owned(),
            );
        }
        if self.policy.require_rollback_replay && !transaction_replay_passed {
            push_unique(
                &mut blocked_reasons,
                "dna_evolution_transaction_replay_missing_or_failed".to_owned(),
            );
        }

        let mut candidates = Vec::with_capacity(plans.len());
        for (index, plan) in plans.iter().enumerate() {
            let budget_exhausted = index >= self.policy.mutation_budget;
            let candidate = self.candidate_from_plan(
                &generation_id,
                &parent_anchor_id,
                &stable_anchor_id,
                plan,
                index,
                budget_exhausted,
                validation_status,
                validation,
                operator_decision,
                transaction_replay_passed,
            );
            for reason in &candidate.reason_codes {
                if reason.starts_with("dna_evolution_") {
                    push_unique(&mut blocked_reasons, reason.clone());
                }
            }
            candidates.push(candidate);
        }

        let (total_fitness_delta_milli, min_fitness_delta_milli, max_fitness_delta_milli) =
            fitness_delta_bounds(&candidates);

        DnaEvolutionControllerReport {
            schema_version: DNA_EVOLUTION_CONTROLLER_SCHEMA_VERSION,
            profile,
            generation_id,
            parent_anchor_ids: vec![parent_anchor_id],
            stable_anchor_id,
            mutation_budget: self.policy.mutation_budget,
            validation_status,
            operator_decision,
            transaction_replay_count,
            transaction_replay_passed,
            transaction_replay_blocked_count,
            total_fitness_delta_milli,
            min_fitness_delta_milli,
            max_fitness_delta_milli,
            candidates,
            blocked_reasons,
            read_only: true,
            write_allowed: false,
            applied: false,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn candidate_from_plan(
        &self,
        generation_id: &str,
        parent_anchor_id: &str,
        stable_anchor_id: &str,
        plan: &MutationPlan,
        index: usize,
        budget_exhausted: bool,
        validation_status: DnaEvolutionValidationStatus,
        validation: &DnaEvolutionValidationEvidence,
        operator_decision: GeneScissorsOperatorDecision,
        transaction_replay_passed: bool,
    ) -> DnaEvolutionCandidate {
        let rollback_anchor_id = redacted_ref(&plan.rollback_anchor_id);
        let source_plan_id = redacted_ref(&plan.id);
        let target_gene_id = redacted_ref(&plan.target_gene_id);
        let replacement_gene_id = plan.replacement_gene_id.as_deref().map(redacted_ref);
        let mut reason_codes = Vec::new();
        let mut fitness_delta_milli = base_fitness_delta_milli(plan.intent);

        if budget_exhausted {
            push_unique(
                &mut reason_codes,
                "dna_evolution_mutation_budget_exhausted".to_owned(),
            );
            fitness_delta_milli = fitness_delta_milli.min(-30);
        }
        if validation_status == DnaEvolutionValidationStatus::Missing {
            push_unique(
                &mut reason_codes,
                "dna_evolution_validation_evidence_missing".to_owned(),
            );
        }
        if validation_status == DnaEvolutionValidationStatus::Failed {
            push_unique(
                &mut reason_codes,
                "dna_evolution_validation_failed".to_owned(),
            );
            fitness_delta_milli = fitness_delta_milli.min(-100);
        }
        if self.policy.require_rollback_replay && !transaction_replay_passed {
            push_unique(
                &mut reason_codes,
                "dna_evolution_transaction_replay_missing_or_failed".to_owned(),
            );
        }
        if operator_decision == GeneScissorsOperatorDecision::Pending {
            push_unique(
                &mut reason_codes,
                "dna_evolution_operator_approval_pending".to_owned(),
            );
        }
        if operator_decision == GeneScissorsOperatorDecision::Rejected {
            push_unique(
                &mut reason_codes,
                "dna_evolution_operator_rejected".to_owned(),
            );
        }
        if plan.validation_status == GeneValidationStatus::Failed {
            push_unique(
                &mut reason_codes,
                "dna_evolution_plan_validation_failed".to_owned(),
            );
            fitness_delta_milli = fitness_delta_milli.min(-100);
        }
        if fitness_delta_milli < self.policy.max_fitness_regression_milli {
            push_unique(
                &mut reason_codes,
                "dna_evolution_fitness_regression_over_budget".to_owned(),
            );
        }

        let decision = if budget_exhausted
            || validation_status == DnaEvolutionValidationStatus::Missing
            || (self.policy.require_rollback_replay && !transaction_replay_passed)
        {
            DnaEvolutionCandidateDecision::Hold
        } else if validation_status == DnaEvolutionValidationStatus::Failed
            || plan.validation_status == GeneValidationStatus::Failed
            || fitness_delta_milli < self.policy.max_fitness_regression_milli
        {
            if rollback_intent(plan.intent) {
                DnaEvolutionCandidateDecision::Rollback
            } else {
                DnaEvolutionCandidateDecision::Reject
            }
        } else if operator_decision == GeneScissorsOperatorDecision::Rejected {
            DnaEvolutionCandidateDecision::Reject
        } else {
            DnaEvolutionCandidateDecision::CandidatePreview
        };
        let approval_ready = !self.policy.require_operator_approval_for_activation
            || operator_decision == GeneScissorsOperatorDecision::Approved;
        let activation_eligible = decision == DnaEvolutionCandidateDecision::CandidatePreview
            && validation_status == DnaEvolutionValidationStatus::Passed
            && transaction_replay_passed
            && approval_ready;
        let candidate_id = stable_redaction_digest([
            "dna-evolution-candidate",
            generation_id,
            &index.to_string(),
            source_plan_id.as_str(),
            target_gene_id.as_str(),
            plan.intent.as_str(),
            decision.as_str(),
        ]);

        DnaEvolutionCandidate {
            candidate_id,
            generation_id: generation_id.to_owned(),
            parent_anchor_ids: vec![parent_anchor_id.to_owned()],
            stable_anchor_id: stable_anchor_id.to_owned(),
            rollback_anchor_id,
            source_plan_id,
            target_gene_id,
            replacement_gene_id,
            intent: plan.intent,
            decision,
            validation_status,
            operator_decision,
            fitness_delta_milli,
            validation_artifact_digests: validation.artifact_digests.clone(),
            reason_codes,
            activation_eligible,
            read_only: true,
            write_allowed: false,
            applied: false,
        }
    }
}

fn base_fitness_delta_milli(intent: GeneScissorsIntent) -> i32 {
    match intent {
        GeneScissorsIntent::Relabel => 70,
        GeneScissorsIntent::Cut => 130,
        GeneScissorsIntent::Splice => 85,
        GeneScissorsIntent::Quarantine => 120,
        GeneScissorsIntent::Repair => 80,
        GeneScissorsIntent::Crossover => 90,
        GeneScissorsIntent::Rollback => -20,
        GeneScissorsIntent::Regenerate => 150,
    }
}

fn rollback_intent(intent: GeneScissorsIntent) -> bool {
    matches!(
        intent,
        GeneScissorsIntent::Quarantine
            | GeneScissorsIntent::Cut
            | GeneScissorsIntent::Regenerate
            | GeneScissorsIntent::Rollback
    )
}

fn fitness_delta_bounds(candidates: &[DnaEvolutionCandidate]) -> (i32, i32, i32) {
    if candidates.is_empty() {
        return (0, 0, 0);
    }
    let total = candidates
        .iter()
        .map(|candidate| candidate.fitness_delta_milli)
        .sum();
    let min = candidates
        .iter()
        .map(|candidate| candidate.fitness_delta_milli)
        .min()
        .unwrap_or(0);
    let max = candidates
        .iter()
        .map(|candidate| candidate.fitness_delta_milli)
        .max()
        .unwrap_or(0);
    (total, min, max)
}

fn redacted_ref(value: &str) -> String {
    if value.trim().is_empty() {
        return stable_redaction_digest(["dna-evolution-empty-ref"]);
    }
    stable_redaction_digest(["dna-evolution-ref", value.trim()])
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn json_string_array(values: &[String]) -> String {
    let items = values
        .iter()
        .map(|value| format!("\"{}\"", json_escape(value)))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

fn json_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hierarchy::TaskProfile;
    use crate::privacy_redaction::contains_private_or_executable_marker;
    use crate::writer_gate::{
        UnifiedWriterGate, UnifiedWriterGateCandidate, UnifiedWriterGatePolicy,
    };

    #[test]
    fn dna_evolution_controller_produces_candidate_previews_for_mutation_intents() {
        let plans = vec![
            plan(GeneScissorsIntent::Relabel, "aged-label"),
            plan(GeneScissorsIntent::Splice, "schema-gap"),
            plan(GeneScissorsIntent::Repair, "format-drift"),
            plan(GeneScissorsIntent::Quarantine, "harmful-drift"),
            plan(GeneScissorsIntent::Cut, "malignant-cut"),
            plan(GeneScissorsIntent::Regenerate, "stable-regeneration")
                .with_replacement("young-stable-regeneration"),
        ];
        let journal = GeneScissorsTransactionJournal::from_mutation_plans(
            TaskProfile::Coding,
            "stable-anchor",
            &plans,
        );
        let report = DnaEvolutionController::default().preview_plans(
            TaskProfile::Coding,
            "parent-anchor",
            "stable-anchor",
            &plans,
            &DnaEvolutionValidationEvidence::passing(),
            GeneScissorsOperatorDecision::Pending,
            Some(&journal),
        );

        assert_eq!(report.candidate_count(), 6);
        assert_eq!(
            report.decision_count(DnaEvolutionCandidateDecision::CandidatePreview),
            6
        );
        assert_eq!(report.intent_count(GeneScissorsIntent::Relabel), 1);
        assert_eq!(report.intent_count(GeneScissorsIntent::Splice), 1);
        assert_eq!(report.intent_count(GeneScissorsIntent::Repair), 1);
        assert_eq!(report.intent_count(GeneScissorsIntent::Quarantine), 1);
        assert_eq!(report.intent_count(GeneScissorsIntent::Cut), 1);
        assert_eq!(report.intent_count(GeneScissorsIntent::Regenerate), 1);
        assert_eq!(report.activation_eligible_count(), 0);
        assert!(report.is_read_only_preview());
        assert!(report.summary_line().contains("validation=passed"));
        assert!(report.summary_line().contains("approval=pending"));
        assert!(report.redacted_trace_line().contains("generation_id"));
        assert!(
            report
                .redacted_trace_line()
                .contains("fitness_delta_summary")
        );
        assert!(!report.redacted_trace_line().contains("malignant-cut"));
        assert!(!contains_private_or_executable_marker(
            &report.redacted_trace_line()
        ));
    }

    #[test]
    fn approved_validation_marks_activation_eligible_without_write_apply() {
        let plans = vec![plan(GeneScissorsIntent::Repair, "repairable")];
        let journal = GeneScissorsTransactionJournal::from_mutation_plans(
            TaskProfile::General,
            "stable-anchor",
            &plans,
        );
        let report = DnaEvolutionController::default().preview_plans(
            TaskProfile::General,
            "parent-anchor",
            "stable-anchor",
            &plans,
            &DnaEvolutionValidationEvidence::passing(),
            GeneScissorsOperatorDecision::Approved,
            Some(&journal),
        );

        assert_eq!(report.activation_eligible_count(), 1);
        assert_eq!(
            report.decision_count(DnaEvolutionCandidateDecision::CandidatePreview),
            1
        );
        assert!(report.is_read_only_preview());
        assert!(!report.write_allowed);
        assert!(!report.applied);
        assert!(report.candidates.iter().all(|candidate| {
            candidate.activation_eligible && !candidate.write_allowed && !candidate.applied
        }));
    }

    #[test]
    fn apply_plan_reaches_ready_without_writing_when_writer_gate_is_ready() {
        let report = approved_report_fixture();
        let writer_gate = writer_gate_for_report(&report, true);
        let apply_plan = report.explicit_apply_plan(&writer_gate);
        let line = apply_plan.json_line();

        assert_eq!(
            writer_gate.decision,
            UnifiedWriterGateDecision::ReadyForExplicitApply
        );
        assert_eq!(
            apply_plan.decision,
            DnaEvolutionApplyDecision::ReadyForExplicitApply
        );
        assert_eq!(apply_plan.ready_candidates, 1);
        assert_eq!(apply_plan.held_candidates, 0);
        assert_eq!(apply_plan.rejected_candidates, 0);
        assert!(apply_plan.explicit_apply_required);
        assert!(apply_plan.passed(), "{}", apply_plan.summary_line());
        assert!(apply_plan.is_preview_only());
        assert!(!apply_plan.write_allowed);
        assert!(!apply_plan.applied);
        assert!(line.contains("\"schema\":\"rust-norion-dna-evolution-apply-plan-v1\""));
        assert!(line.contains("\"apply_plan_digest\":\"redaction-digest:"));
        assert!(!line.contains("\"records\":["));
        assert!(!line.contains("repairable"));
        assert!(!contains_private_or_executable_marker(&line));
        assert!(
            crate::trace::evaluate_trace_schema_line(&line).is_empty(),
            "{line}"
        );
    }

    #[test]
    fn apply_plan_holds_behind_default_writer_gate() {
        let report = approved_report_fixture();
        let writer_gate = writer_gate_for_report(&report, false);
        let apply_plan = report.explicit_apply_plan(&writer_gate);

        assert_eq!(writer_gate.decision, UnifiedWriterGateDecision::PreviewOnly);
        assert_eq!(
            apply_plan.decision,
            DnaEvolutionApplyDecision::HeldForWriterGate
        );
        assert_eq!(apply_plan.ready_candidates, 0);
        assert_eq!(apply_plan.held_candidates, 1);
        assert_eq!(apply_plan.rejected_candidates, 0);
        assert!(apply_plan.explicit_apply_required);
        assert!(apply_plan.passed(), "{}", apply_plan.summary_line());
        assert!(apply_plan.is_preview_only());
    }

    #[test]
    fn failed_validation_rolls_back_harmful_mutations_and_rejects_repair() {
        let plans = vec![
            plan(GeneScissorsIntent::Quarantine, "harmful-drift"),
            plan(GeneScissorsIntent::Cut, "malignant-cut"),
            plan(GeneScissorsIntent::Regenerate, "regen").with_replacement("regen-child"),
            plan(GeneScissorsIntent::Repair, "repairable"),
        ];
        let journal = GeneScissorsTransactionJournal::from_mutation_plans(
            TaskProfile::Coding,
            "stable-anchor",
            &plans,
        );
        let report = DnaEvolutionController::default().preview_plans(
            TaskProfile::Coding,
            "parent-anchor",
            "stable-anchor",
            &plans,
            &DnaEvolutionValidationEvidence::failed_tests(),
            GeneScissorsOperatorDecision::Pending,
            Some(&journal),
        );

        assert_eq!(
            report.validation_status,
            DnaEvolutionValidationStatus::Failed
        );
        assert_eq!(
            report.decision_count(DnaEvolutionCandidateDecision::Rollback),
            3
        );
        assert_eq!(
            report.decision_count(DnaEvolutionCandidateDecision::Reject),
            1
        );
        assert_eq!(report.activation_eligible_count(), 0);
        assert!(report.candidates.iter().all(|candidate| {
            candidate
                .reason_codes
                .contains(&"dna_evolution_validation_failed".to_owned())
                && !candidate.write_allowed
                && !candidate.applied
        }));
    }

    #[test]
    fn mutation_budget_exhaustion_holds_overflow_candidates() {
        let plans = vec![
            plan(GeneScissorsIntent::Relabel, "first"),
            plan(GeneScissorsIntent::Repair, "second"),
            plan(GeneScissorsIntent::Splice, "third"),
        ];
        let journal = GeneScissorsTransactionJournal::from_mutation_plans(
            TaskProfile::Writing,
            "stable-anchor",
            &plans,
        );
        let report = DnaEvolutionController::new(DnaEvolutionPolicy {
            mutation_budget: 2,
            ..DnaEvolutionPolicy::default()
        })
        .preview_plans(
            TaskProfile::Writing,
            "parent-anchor",
            "stable-anchor",
            &plans,
            &DnaEvolutionValidationEvidence::passing(),
            GeneScissorsOperatorDecision::Pending,
            Some(&journal),
        );

        assert_eq!(
            report.decision_count(DnaEvolutionCandidateDecision::CandidatePreview),
            2
        );
        assert_eq!(
            report.decision_count(DnaEvolutionCandidateDecision::Hold),
            1
        );
        assert!(
            report
                .blocked_reasons
                .contains(&"dna_evolution_mutation_budget_exhausted".to_owned())
        );
        assert!(
            report.candidates[2]
                .reason_codes
                .contains(&"dna_evolution_mutation_budget_exhausted".to_owned())
        );
    }

    #[test]
    fn missing_replay_or_validation_holds_without_durable_mutation() {
        let plans = vec![plan(GeneScissorsIntent::Cut, "malignant-cut")];
        let report = DnaEvolutionController::default().preview_plans(
            TaskProfile::LongDocument,
            "parent-anchor",
            "stable-anchor",
            &plans,
            &DnaEvolutionValidationEvidence::default(),
            GeneScissorsOperatorDecision::Approved,
            None,
        );

        assert_eq!(
            report.validation_status,
            DnaEvolutionValidationStatus::Missing
        );
        assert_eq!(
            report.decision_count(DnaEvolutionCandidateDecision::Hold),
            1
        );
        assert!(!report.transaction_replay_passed);
        assert!(
            report
                .blocked_reasons
                .contains(&"dna_evolution_validation_evidence_missing".to_owned())
        );
        assert!(
            report
                .blocked_reasons
                .contains(&"dna_evolution_transaction_replay_missing_or_failed".to_owned())
        );
        assert!(report.is_read_only_preview());
    }

    #[test]
    fn preview_expression_and_trace_are_digest_only() {
        let mut expression = GenomeExpression::empty(TaskProfile::Coding);
        expression.genome_id = "private prompt: raw_prompt should not leak".to_owned();
        expression.stable_anchor_id = "stable-anchor".to_owned();
        expression.mutation_plans = vec![plan(
            GeneScissorsIntent::Relabel,
            "tenant_id=private-target",
        )];
        let report = DnaEvolutionController::default().preview_expression(
            &expression,
            &DnaEvolutionValidationEvidence::passing(),
            GeneScissorsOperatorDecision::Approved,
        );
        let trace = report.redacted_trace_line();

        assert_eq!(report.candidate_count(), 1);
        assert_eq!(report.activation_eligible_count(), 1);
        assert!(trace.contains(DNA_EVOLUTION_CONTROLLER_SCHEMA_VERSION));
        assert!(!trace.contains("raw_prompt"));
        assert!(!trace.contains("tenant_id=private-target"));
        assert!(!contains_private_or_executable_marker(&trace));
    }

    fn plan(intent: GeneScissorsIntent, target: &str) -> MutationPlan {
        MutationPlan::preview(
            format!("plan-{target}-{}", intent.as_str()),
            intent,
            target,
            "redacted mutation reason",
            "redacted expected control-plane effect",
            "stable-anchor",
        )
    }

    fn approved_report_fixture() -> DnaEvolutionControllerReport {
        let plans = vec![plan(GeneScissorsIntent::Repair, "repairable")];
        let journal = GeneScissorsTransactionJournal::from_mutation_plans(
            TaskProfile::General,
            "stable-anchor",
            &plans,
        );
        DnaEvolutionController::default().preview_plans(
            TaskProfile::General,
            "parent-anchor",
            "stable-anchor",
            &plans,
            &DnaEvolutionValidationEvidence::passing(),
            GeneScissorsOperatorDecision::Approved,
            Some(&journal),
        )
    }

    fn writer_gate_for_report(
        report: &DnaEvolutionControllerReport,
        durable_writes_enabled: bool,
    ) -> crate::writer_gate::UnifiedWriterGateReport {
        let candidate = UnifiedWriterGateCandidate::dna_evolution_controller_report(report);
        let policy = UnifiedWriterGatePolicy {
            durable_writes_enabled,
            ..UnifiedWriterGatePolicy::default()
        };
        UnifiedWriterGate::new()
            .with_policy(policy)
            .evaluate([candidate])
    }
}
