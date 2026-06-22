use crate::experiment::ExperimentSwitches;
use crate::profile::TaskProfile;
use crate::router::{
    RouteLayer, RouteLayerCounts, RoutingContext, RoutingDecision, RoutingFeedback,
    RoutingFeedbackSummary,
};

#[derive(Debug, Clone, PartialEq)]
pub struct AttentionCandidate {
    pub token: String,
    pub position: usize,
    pub score: f32,
    pub entropy: f32,
    pub layer: RouteLayer,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttentionCandidateSummary {
    pub token: String,
    pub position: usize,
    pub score: f32,
    pub entropy: f32,
    pub layer: RouteLayer,
    pub uses_attention: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AttentionCandidateBatchSummary {
    pub candidate_count: usize,
    pub attention_candidate_count: usize,
    pub fast_candidate_count: usize,
    pub layer_counts: RouteLayerCounts,
    pub average_score: f32,
    pub average_entropy: f32,
    pub max_score: f32,
    pub max_entropy: f32,
}

impl AttentionCandidate {
    pub fn new(
        token: impl Into<String>,
        position: usize,
        score: f32,
        entropy: f32,
        layer: RouteLayer,
    ) -> Self {
        Self {
            token: token.into(),
            position,
            score: score.clamp(0.0, 1.0),
            entropy: entropy.clamp(0.0, 1.0),
            layer,
        }
    }

    pub fn from_route(decision: &RoutingDecision, position: usize, entropy: f32) -> Self {
        Self::new(
            decision.token.clone(),
            position,
            decision.score,
            entropy,
            decision.layer,
        )
    }

    pub fn candidate_summary(&self) -> AttentionCandidateSummary {
        AttentionCandidateSummary {
            token: self.token.clone(),
            position: self.position,
            score: self.score,
            entropy: self.entropy,
            layer: self.layer,
            uses_attention: self.layer.uses_attention(),
        }
    }

    pub fn batch_summary(candidates: &[AttentionCandidate]) -> AttentionCandidateBatchSummary {
        AttentionCandidateBatchSummary::from_candidates(candidates)
    }

    fn identity(&self) -> CandidateIdentity<'_> {
        CandidateIdentity {
            token: &self.token,
            position: self.position,
            layer: self.layer,
        }
    }
}

impl AttentionCandidateSummary {
    pub fn is_fast_projection(&self) -> bool {
        self.layer == RouteLayer::FastProjection
    }

    pub fn is_high_entropy(&self, threshold: f32) -> bool {
        self.entropy >= threshold.clamp(0.0, 1.0)
    }

    pub fn score_reaches(&self, threshold: f32) -> bool {
        self.score >= threshold.clamp(0.0, 1.0)
    }

    pub fn token_shape_is_valid(&self) -> bool {
        !self.token.is_empty()
    }

    pub fn score_shape_is_valid(&self) -> bool {
        finite_unit(self.score)
    }

    pub fn entropy_shape_is_valid(&self) -> bool {
        finite_unit(self.entropy)
    }

    pub fn uses_attention_matches_layer(&self) -> bool {
        self.uses_attention == self.layer.uses_attention()
    }

    pub fn candidate_signal_component_count(&self) -> usize {
        usize::from(self.token_shape_is_valid())
            + usize::from(self.uses_attention)
            + usize::from(self.is_fast_projection())
            + usize::from(self.score > 0.0 && self.score_shape_is_valid())
            + usize::from(self.entropy > 0.0 && self.entropy_shape_is_valid())
    }

    pub fn has_candidate_signal_components(&self) -> bool {
        self.candidate_signal_component_count() > 0
    }

    pub fn candidate_problem_component_count(&self) -> usize {
        usize::from(!self.token_shape_is_valid())
            + usize::from(!self.score_shape_is_valid())
            + usize::from(!self.entropy_shape_is_valid())
            + usize::from(!self.uses_attention_matches_layer())
    }

    pub fn has_candidate_problem_components(&self) -> bool {
        self.candidate_problem_component_count() > 0
    }

    pub fn candidate_accounting_is_consistent(&self) -> bool {
        let expected_signal_count = usize::from(self.token_shape_is_valid())
            .saturating_add(usize::from(self.uses_attention))
            .saturating_add(usize::from(self.is_fast_projection()))
            .saturating_add(usize::from(self.score > 0.0 && self.score_shape_is_valid()))
            .saturating_add(usize::from(
                self.entropy > 0.0 && self.entropy_shape_is_valid(),
            ));
        let expected_problem_count = usize::from(!self.token_shape_is_valid())
            .saturating_add(usize::from(!self.score_shape_is_valid()))
            .saturating_add(usize::from(!self.entropy_shape_is_valid()))
            .saturating_add(usize::from(!self.uses_attention_matches_layer()));

        self.candidate_signal_component_count() == expected_signal_count
            && self.candidate_problem_component_count() == expected_problem_count
    }

    pub fn candidate_shape_is_clean(&self) -> bool {
        !self.has_candidate_problem_components() && self.candidate_accounting_is_consistent()
    }

    pub fn can_use_attention_candidate(&self) -> bool {
        self.candidate_shape_is_clean()
    }
}

impl AttentionCandidateBatchSummary {
    pub fn from_candidates(candidates: &[AttentionCandidate]) -> Self {
        let layer_counts = layer_counts_from_candidates(candidates);
        let candidate_count = candidates.len();
        let (score_sum, entropy_sum, max_score, max_entropy) = candidates.iter().fold(
            (0.0_f32, 0.0_f32, 0.0_f32, 0.0_f32),
            |(score_sum, entropy_sum, max_score, max_entropy), candidate| {
                (
                    score_sum + candidate.score,
                    entropy_sum + candidate.entropy,
                    max_score.max(candidate.score),
                    max_entropy.max(candidate.entropy),
                )
            },
        );

        Self {
            candidate_count,
            attention_candidate_count: layer_counts.attention_total(),
            fast_candidate_count: layer_counts.fast_projection,
            layer_counts,
            average_score: if candidate_count == 0 {
                0.0
            } else {
                score_sum / candidate_count as f32
            },
            average_entropy: if candidate_count == 0 {
                0.0
            } else {
                entropy_sum / candidate_count as f32
            },
            max_score,
            max_entropy,
        }
    }

    pub fn is_empty(self) -> bool {
        self.candidate_count == 0
    }

    pub fn has_attention_candidates(self) -> bool {
        self.attention_candidate_count > 0
    }

    pub fn has_fast_candidates(self) -> bool {
        self.fast_candidate_count > 0
    }

    pub fn layer_counts_match_candidates(self) -> bool {
        self.layer_counts.total() == self.candidate_count
            && self.layer_counts.attention_total() == self.attention_candidate_count
            && self.layer_counts.fast_projection == self.fast_candidate_count
    }

    pub fn all_attention_candidates(self) -> bool {
        !self.is_empty() && self.fast_candidate_count == 0
    }

    pub fn uses_multiple_layers(self) -> bool {
        self.layer_counts.uses_multiple_layers()
    }

    pub fn attention_candidate_fraction(self) -> f32 {
        self.attention_candidate_count as f32 / self.candidate_count.max(1) as f32
    }

    pub fn attention_candidate_count_matches_total(self) -> bool {
        self.attention_candidate_count
            .saturating_add(self.fast_candidate_count)
            == self.candidate_count
    }

    pub fn score_shape_is_valid(self) -> bool {
        finite_unit(self.average_score)
            && finite_unit(self.max_score)
            && self.average_score <= self.max_score
            && (!self.is_empty()
                || (float_close(self.average_score, 0.0) && float_close(self.max_score, 0.0)))
    }

    pub fn entropy_shape_is_valid(self) -> bool {
        finite_unit(self.average_entropy)
            && finite_unit(self.max_entropy)
            && self.average_entropy <= self.max_entropy
            && (!self.is_empty()
                || (float_close(self.average_entropy, 0.0) && float_close(self.max_entropy, 0.0)))
    }

    pub fn attention_fraction_shape_is_valid(self) -> bool {
        finite_unit(self.attention_candidate_fraction())
    }

    pub fn candidate_batch_activity_signal_component_count(self) -> usize {
        usize::from(!self.is_empty())
            + usize::from(self.has_attention_candidates())
            + usize::from(self.has_fast_candidates())
            + usize::from(self.all_attention_candidates())
    }

    pub fn candidate_batch_layer_signal_component_count(self) -> usize {
        usize::from(self.uses_multiple_layers())
            + usize::from(self.layer_counts.has_fusion())
            + usize::from(self.layer_counts.has_attention_layers())
    }

    pub fn candidate_batch_score_signal_component_count(self) -> usize {
        usize::from(self.average_score > 0.0 && finite_unit(self.average_score))
            + usize::from(self.max_score > 0.0 && finite_unit(self.max_score))
            + usize::from(self.average_entropy > 0.0 && finite_unit(self.average_entropy))
            + usize::from(self.max_entropy > 0.0 && finite_unit(self.max_entropy))
    }

    pub fn candidate_batch_signal_component_count(self) -> usize {
        self.candidate_batch_activity_signal_component_count()
            .saturating_add(self.candidate_batch_layer_signal_component_count())
            .saturating_add(self.candidate_batch_score_signal_component_count())
    }

    pub fn has_candidate_batch_signal_components(self) -> bool {
        self.candidate_batch_signal_component_count() > 0
    }

    pub fn candidate_batch_count_problem_component_count(self) -> usize {
        usize::from(!self.layer_counts_match_candidates())
            + usize::from(!self.attention_candidate_count_matches_total())
    }

    pub fn candidate_batch_score_problem_component_count(self) -> usize {
        usize::from(!self.score_shape_is_valid())
            + usize::from(!self.entropy_shape_is_valid())
            + usize::from(!self.attention_fraction_shape_is_valid())
    }

    pub fn candidate_batch_problem_component_count(self) -> usize {
        self.candidate_batch_count_problem_component_count()
            .saturating_add(self.candidate_batch_score_problem_component_count())
    }

    pub fn has_candidate_batch_problem_components(self) -> bool {
        self.candidate_batch_problem_component_count() > 0
    }

    pub fn candidate_batch_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .candidate_batch_activity_signal_component_count()
            .saturating_add(self.candidate_batch_layer_signal_component_count())
            .saturating_add(self.candidate_batch_score_signal_component_count());
        let expected_problem_count = self
            .candidate_batch_count_problem_component_count()
            .saturating_add(self.candidate_batch_score_problem_component_count());

        self.candidate_batch_signal_component_count() == expected_signal_count
            && self.candidate_batch_problem_component_count() == expected_problem_count
    }

    pub fn candidate_batch_shape_is_clean(self) -> bool {
        !self.has_candidate_batch_problem_components()
            && self.candidate_batch_accounting_is_consistent()
    }

    pub fn can_use_attention_candidate_batch(self) -> bool {
        self.candidate_count > 0 && self.candidate_batch_shape_is_clean()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttentionDecision {
    pub threshold: f32,
    pub max_selected: usize,
    pub selected: Vec<AttentionCandidate>,
    pub rejected: Vec<AttentionCandidate>,
}

impl AttentionDecision {
    pub fn selected_tokens(&self) -> Vec<&str> {
        self.selected
            .iter()
            .map(|candidate| candidate.token.as_str())
            .collect()
    }

    pub fn selected_count(&self) -> usize {
        self.selected.len()
    }

    pub fn rejected_count(&self) -> usize {
        self.rejected.len()
    }

    pub fn candidate_count(&self) -> usize {
        self.selected_count().saturating_add(self.rejected_count())
    }

    pub fn selection_fraction(&self) -> f32 {
        self.selected_count() as f32 / self.candidate_count().max(1) as f32
    }

    pub fn hit_selection_cap(&self) -> bool {
        self.selected_count() >= self.max_selected
            && self.rejected.iter().any(|candidate| {
                candidate.layer.uses_attention() && candidate.score >= self.threshold
            })
    }

    pub fn decision_summary(&self) -> AttentionDecisionSummary {
        AttentionDecisionSummary::from_decision(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AttentionDecisionSummary {
    pub threshold: f32,
    pub max_selected: usize,
    pub candidate_count: usize,
    pub selected_count: usize,
    pub rejected_count: usize,
    pub selection_fraction: f32,
    pub hit_selection_cap: bool,
    pub selected_layer_counts: RouteLayerCounts,
    pub rejected_layer_counts: RouteLayerCounts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttentionSelectionReadinessStage {
    CandidateBatch,
    Decision,
    SelectionBoundary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttentionSelectionReadinessCommitAction {
    CommitAttentionSelection,
    WaitForAttentionSelection,
    RepairAttentionSelection,
}

impl AttentionSelectionReadinessCommitAction {
    pub fn can_commit(self) -> bool {
        matches!(self, Self::CommitAttentionSelection)
    }

    pub fn should_wait(self) -> bool {
        matches!(self, Self::WaitForAttentionSelection)
    }

    pub fn should_repair(self) -> bool {
        matches!(self, Self::RepairAttentionSelection)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AttentionSelectionReadinessSummary {
    pub candidate_batch: AttentionCandidateBatchSummary,
    pub decision: AttentionDecisionSummary,
    pub candidate_batch_signal_component_count: usize,
    pub decision_signal_component_count: usize,
    pub selection_boundary_signal_component_count: usize,
    pub candidate_batch_blocker_component_count: usize,
    pub decision_blocker_component_count: usize,
    pub selection_boundary_blocker_component_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AttentionSelectionReadinessCommitSummary {
    pub readiness: AttentionSelectionReadinessSummary,
    pub action: AttentionSelectionReadinessCommitAction,
    pub committed_attention_decision: Option<AttentionDecisionSummary>,
    pub can_commit: bool,
    pub should_wait_for_attention_selection: bool,
    pub should_repair_attention_selection: bool,
    pub first_unready_stage: Option<AttentionSelectionReadinessStage>,
    pub first_blocking_stage: Option<AttentionSelectionReadinessStage>,
    pub total_signal_component_count: usize,
    pub total_blocker_component_count: usize,
    pub component_accounting_consistent: bool,
}

impl AttentionDecisionSummary {
    pub fn from_decision(decision: &AttentionDecision) -> Self {
        let selected_layer_counts = layer_counts_from_candidates(&decision.selected);
        let rejected_layer_counts = layer_counts_from_candidates(&decision.rejected);

        Self {
            threshold: decision.threshold,
            max_selected: decision.max_selected,
            candidate_count: decision.candidate_count(),
            selected_count: decision.selected_count(),
            rejected_count: decision.rejected_count(),
            selection_fraction: decision.selection_fraction(),
            hit_selection_cap: decision.hit_selection_cap(),
            selected_layer_counts,
            rejected_layer_counts,
        }
    }

    pub fn selected_attention_tokens(self) -> usize {
        self.selected_layer_counts.attention_total()
    }

    pub fn rejected_attention_tokens(self) -> usize {
        self.rejected_layer_counts.attention_total()
    }

    pub fn is_empty(self) -> bool {
        self.candidate_count == 0
    }

    pub fn selected_counts_match_layers(self) -> bool {
        self.selected_layer_counts.total() == self.selected_count
    }

    pub fn rejected_counts_match_layers(self) -> bool {
        self.rejected_layer_counts.total() == self.rejected_count
    }

    pub fn candidate_accounting_balanced(self) -> bool {
        self.selected_count.saturating_add(self.rejected_count) == self.candidate_count
            && self.selected_counts_match_layers()
            && self.rejected_counts_match_layers()
    }

    pub fn has_selected_attention(self) -> bool {
        self.selected_attention_tokens() > 0
    }

    pub fn has_rejected_attention(self) -> bool {
        self.rejected_attention_tokens() > 0
    }

    pub fn selected_attention_fraction(self) -> f32 {
        self.selected_attention_tokens() as f32 / self.selected_count.max(1) as f32
    }

    pub fn rejected_attention_fraction(self) -> f32 {
        self.rejected_attention_tokens() as f32 / self.rejected_count.max(1) as f32
    }

    pub fn has_selection_pressure(self) -> bool {
        self.hit_selection_cap || self.has_rejected_attention()
    }

    pub fn threshold_shape_is_valid(self) -> bool {
        finite_unit(self.threshold)
    }

    pub fn selection_fraction_matches_counts(self) -> bool {
        let expected = self.selected_count as f32 / self.candidate_count.max(1) as f32;
        finite_unit(self.selection_fraction) && float_close(self.selection_fraction, expected)
    }

    pub fn selected_count_within_cap(self) -> bool {
        self.selected_count <= self.max_selected
    }

    pub fn selection_cap_hit_shape_is_valid(self) -> bool {
        !self.hit_selection_cap
            || (self.selected_count >= self.max_selected && self.has_rejected_attention())
    }

    pub fn selection_cap_hit_shape_problem_component_count(self) -> usize {
        usize::from(!self.selection_cap_hit_shape_is_valid())
    }

    pub fn decision_activity_signal_component_count(self) -> usize {
        usize::from(!self.is_empty())
            + usize::from(self.selected_count > 0)
            + usize::from(self.rejected_count > 0)
            + usize::from(self.hit_selection_cap)
    }

    pub fn decision_attention_signal_component_count(self) -> usize {
        usize::from(self.has_selected_attention())
            + usize::from(self.has_rejected_attention())
            + usize::from(self.has_selection_pressure())
    }

    pub fn decision_signal_component_count(self) -> usize {
        self.decision_activity_signal_component_count()
            .saturating_add(self.decision_attention_signal_component_count())
    }

    pub fn has_decision_signal_components(self) -> bool {
        self.decision_signal_component_count() > 0
    }

    pub fn decision_count_problem_component_count(self) -> usize {
        usize::from(!self.candidate_accounting_balanced())
            + usize::from(!self.selected_count_within_cap())
    }

    pub fn decision_shape_problem_component_count(self) -> usize {
        usize::from(!self.threshold_shape_is_valid())
            + usize::from(!self.selection_fraction_matches_counts())
            + self.selection_cap_hit_shape_problem_component_count()
    }

    pub fn decision_problem_component_count(self) -> usize {
        self.decision_count_problem_component_count()
            .saturating_add(self.decision_shape_problem_component_count())
    }

    pub fn has_decision_problem_components(self) -> bool {
        self.decision_problem_component_count() > 0
    }

    pub fn decision_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .decision_activity_signal_component_count()
            .saturating_add(self.decision_attention_signal_component_count());
        let expected_problem_count = self
            .decision_count_problem_component_count()
            .saturating_add(self.decision_shape_problem_component_count());

        self.decision_signal_component_count() == expected_signal_count
            && self.decision_problem_component_count() == expected_problem_count
    }

    pub fn decision_shape_is_clean(self) -> bool {
        !self.has_decision_problem_components() && self.decision_accounting_is_consistent()
    }

    pub fn can_use_attention_decision(self) -> bool {
        self.candidate_count > 0 && self.decision_shape_is_clean()
    }
}

impl AttentionSelectionReadinessSummary {
    pub fn new(
        candidate_batch: AttentionCandidateBatchSummary,
        decision: AttentionDecisionSummary,
    ) -> Self {
        Self {
            candidate_batch,
            decision,
            candidate_batch_signal_component_count: candidate_batch
                .candidate_batch_signal_component_count(),
            decision_signal_component_count: decision.decision_signal_component_count(),
            selection_boundary_signal_component_count: usize::from(
                !candidate_batch.is_empty()
                    && !decision.is_empty()
                    && Self::selection_boundary_matches_parts(candidate_batch, decision),
            ),
            candidate_batch_blocker_component_count: candidate_batch
                .candidate_batch_problem_component_count(),
            decision_blocker_component_count: decision.decision_problem_component_count(),
            selection_boundary_blocker_component_count:
                Self::selection_boundary_drift_component_count_parts(candidate_batch, decision),
        }
    }

    pub fn from_decision(
        candidate_batch: AttentionCandidateBatchSummary,
        decision: &AttentionDecision,
    ) -> Self {
        Self::new(candidate_batch, decision.decision_summary())
    }

    pub fn stage_order() -> [AttentionSelectionReadinessStage; 3] {
        [
            AttentionSelectionReadinessStage::CandidateBatch,
            AttentionSelectionReadinessStage::Decision,
            AttentionSelectionReadinessStage::SelectionBoundary,
        ]
    }

    pub fn selected_and_rejected_layer_counts(self) -> RouteLayerCounts {
        combined_layer_counts(
            self.decision.selected_layer_counts,
            self.decision.rejected_layer_counts,
        )
    }

    pub fn candidate_count_matches_decision(self) -> bool {
        self.candidate_batch.candidate_count == self.decision.candidate_count
    }

    pub fn attention_candidate_count_matches_decision(self) -> bool {
        self.candidate_batch.attention_candidate_count
            == self.selected_and_rejected_layer_counts().attention_total()
    }

    pub fn fast_candidate_count_matches_decision(self) -> bool {
        self.candidate_batch.fast_candidate_count
            == self.selected_and_rejected_layer_counts().fast_projection
    }

    pub fn layer_counts_match_decision(self) -> bool {
        self.candidate_batch.layer_counts == self.selected_and_rejected_layer_counts()
    }

    pub fn selection_boundary_matches(self) -> bool {
        Self::selection_boundary_matches_parts(self.candidate_batch, self.decision)
    }

    pub fn selection_boundary_drift_component_count(self) -> usize {
        Self::selection_boundary_drift_component_count_parts(self.candidate_batch, self.decision)
    }

    pub fn candidate_batch_ready(self) -> bool {
        self.candidate_batch.can_use_attention_candidate_batch()
    }

    pub fn decision_ready(self) -> bool {
        self.decision.can_use_attention_decision()
    }

    pub fn selection_boundary_ready(self) -> bool {
        self.selection_boundary_matches()
    }

    pub fn stage_ready(self, stage: AttentionSelectionReadinessStage) -> bool {
        match stage {
            AttentionSelectionReadinessStage::CandidateBatch => self.candidate_batch_ready(),
            AttentionSelectionReadinessStage::Decision => self.decision_ready(),
            AttentionSelectionReadinessStage::SelectionBoundary => self.selection_boundary_ready(),
        }
    }

    pub fn stage_signal_component_count(self, stage: AttentionSelectionReadinessStage) -> usize {
        match stage {
            AttentionSelectionReadinessStage::CandidateBatch => {
                self.candidate_batch_signal_component_count
            }
            AttentionSelectionReadinessStage::Decision => self.decision_signal_component_count,
            AttentionSelectionReadinessStage::SelectionBoundary => {
                self.selection_boundary_signal_component_count
            }
        }
    }

    pub fn stage_blocker_component_count(self, stage: AttentionSelectionReadinessStage) -> usize {
        match stage {
            AttentionSelectionReadinessStage::CandidateBatch => {
                self.candidate_batch_blocker_component_count
            }
            AttentionSelectionReadinessStage::Decision => self.decision_blocker_component_count,
            AttentionSelectionReadinessStage::SelectionBoundary => {
                self.selection_boundary_blocker_component_count
            }
        }
    }

    pub fn first_unready_stage(self) -> Option<AttentionSelectionReadinessStage> {
        Self::stage_order()
            .into_iter()
            .find(|stage| !self.stage_ready(*stage))
    }

    pub fn first_blocking_stage(self) -> Option<AttentionSelectionReadinessStage> {
        Self::stage_order()
            .into_iter()
            .find(|stage| self.stage_blocker_component_count(*stage) > 0)
    }

    pub fn attention_selection_readiness_signal_component_count(self) -> usize {
        self.candidate_batch_signal_component_count
            .saturating_add(self.decision_signal_component_count)
            .saturating_add(self.selection_boundary_signal_component_count)
    }

    pub fn has_attention_selection_readiness_signals(self) -> bool {
        self.attention_selection_readiness_signal_component_count() > 0
    }

    pub fn attention_selection_readiness_blocker_component_count(self) -> usize {
        self.candidate_batch_blocker_component_count
            .saturating_add(self.decision_blocker_component_count)
            .saturating_add(self.selection_boundary_blocker_component_count)
    }

    pub fn has_attention_selection_readiness_blockers(self) -> bool {
        self.attention_selection_readiness_blocker_component_count() > 0
    }

    pub fn attention_selection_readiness_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .candidate_batch_signal_component_count
            .saturating_add(self.decision_signal_component_count)
            .saturating_add(self.selection_boundary_signal_component_count);
        let expected_blocker_count = self
            .candidate_batch_blocker_component_count
            .saturating_add(self.decision_blocker_component_count)
            .saturating_add(self.selection_boundary_blocker_component_count);

        self.candidate_batch
            .candidate_batch_accounting_is_consistent()
            && self.decision.decision_accounting_is_consistent()
            && self.candidate_batch_signal_component_count
                == self
                    .candidate_batch
                    .candidate_batch_signal_component_count()
            && self.decision_signal_component_count
                == self.decision.decision_signal_component_count()
            && self.selection_boundary_signal_component_count
                == usize::from(
                    !self.candidate_batch.is_empty()
                        && !self.decision.is_empty()
                        && self.selection_boundary_matches(),
                )
            && self.candidate_batch_blocker_component_count
                == self
                    .candidate_batch
                    .candidate_batch_problem_component_count()
            && self.decision_blocker_component_count
                == self.decision.decision_problem_component_count()
            && self.selection_boundary_blocker_component_count
                == self.selection_boundary_drift_component_count()
            && self.attention_selection_readiness_signal_component_count() == expected_signal_count
            && self.has_attention_selection_readiness_signals() == (expected_signal_count > 0)
            && self.attention_selection_readiness_blocker_component_count()
                == expected_blocker_count
            && self.has_attention_selection_readiness_blockers() == (expected_blocker_count > 0)
    }

    pub fn attention_selection_readiness_is_clean(self) -> bool {
        !self.has_attention_selection_readiness_blockers()
            && self.attention_selection_readiness_accounting_is_consistent()
    }

    pub fn can_commit_attention_selection_readiness(self) -> bool {
        self.attention_selection_readiness_is_clean()
            && self.candidate_batch_ready()
            && self.decision_ready()
            && self.selection_boundary_ready()
    }

    pub fn attention_selection_readiness_commit_action(
        self,
    ) -> AttentionSelectionReadinessCommitAction {
        if self.can_commit_attention_selection_readiness() {
            AttentionSelectionReadinessCommitAction::CommitAttentionSelection
        } else if self.has_attention_selection_readiness_blockers() {
            AttentionSelectionReadinessCommitAction::RepairAttentionSelection
        } else {
            AttentionSelectionReadinessCommitAction::WaitForAttentionSelection
        }
    }

    pub fn commit_summary(self) -> AttentionSelectionReadinessCommitSummary {
        AttentionSelectionReadinessCommitSummary::new(self)
    }

    fn selection_boundary_matches_parts(
        candidate_batch: AttentionCandidateBatchSummary,
        decision: AttentionDecisionSummary,
    ) -> bool {
        let selected_and_rejected = combined_layer_counts(
            decision.selected_layer_counts,
            decision.rejected_layer_counts,
        );

        candidate_batch.candidate_count == decision.candidate_count
            && candidate_batch.attention_candidate_count == selected_and_rejected.attention_total()
            && candidate_batch.fast_candidate_count == selected_and_rejected.fast_projection
            && candidate_batch.layer_counts == selected_and_rejected
    }

    fn selection_boundary_drift_component_count_parts(
        candidate_batch: AttentionCandidateBatchSummary,
        decision: AttentionDecisionSummary,
    ) -> usize {
        let selected_and_rejected = combined_layer_counts(
            decision.selected_layer_counts,
            decision.rejected_layer_counts,
        );

        usize::from(candidate_batch.candidate_count != decision.candidate_count)
            .saturating_add(usize::from(
                candidate_batch.attention_candidate_count
                    != selected_and_rejected.attention_total(),
            ))
            .saturating_add(usize::from(
                candidate_batch.fast_candidate_count != selected_and_rejected.fast_projection,
            ))
            .saturating_add(usize::from(
                candidate_batch.layer_counts != selected_and_rejected,
            ))
    }
}

impl AttentionSelectionReadinessCommitSummary {
    pub fn new(readiness: AttentionSelectionReadinessSummary) -> Self {
        let component_accounting_consistent =
            readiness.attention_selection_readiness_accounting_is_consistent();
        let action = readiness.attention_selection_readiness_commit_action();
        let committed_attention_decision = action.can_commit().then_some(readiness.decision);

        Self {
            readiness,
            action,
            committed_attention_decision,
            can_commit: action.can_commit(),
            should_wait_for_attention_selection: action.should_wait(),
            should_repair_attention_selection: action.should_repair(),
            first_unready_stage: readiness.first_unready_stage(),
            first_blocking_stage: readiness.first_blocking_stage(),
            total_signal_component_count: readiness
                .attention_selection_readiness_signal_component_count(),
            total_blocker_component_count: readiness
                .attention_selection_readiness_blocker_component_count(),
            component_accounting_consistent,
        }
    }

    pub fn action_can_commit(self) -> bool {
        self.action.can_commit()
    }

    pub fn action_should_wait_for_attention_selection(self) -> bool {
        self.action.should_wait()
    }

    pub fn action_should_repair_attention_selection(self) -> bool {
        self.action.should_repair()
    }

    pub fn can_commit_attention_selection(self) -> bool {
        self.can_commit
    }

    pub fn should_wait_for_attention_selection(self) -> bool {
        self.should_wait_for_attention_selection
    }

    pub fn should_repair_attention_selection(self) -> bool {
        self.should_repair_attention_selection
    }

    pub fn can_use_committed_attention_decision(self) -> bool {
        self.can_commit && self.committed_attention_decision.is_some()
    }

    pub fn commit_decision_accounting_is_consistent(self) -> bool {
        let expected_action = self.readiness.attention_selection_readiness_commit_action();
        let expected_committed_attention_decision = expected_action
            .can_commit()
            .then_some(self.readiness.decision);

        self.action == expected_action
            && self.committed_attention_decision == expected_committed_attention_decision
            && self.can_commit == expected_action.can_commit()
            && self.should_wait_for_attention_selection == expected_action.should_wait()
            && self.should_repair_attention_selection == expected_action.should_repair()
            && self.first_unready_stage == self.readiness.first_unready_stage()
            && self.first_blocking_stage == self.readiness.first_blocking_stage()
            && self.total_signal_component_count
                == self
                    .readiness
                    .attention_selection_readiness_signal_component_count()
            && self.total_blocker_component_count
                == self
                    .readiness
                    .attention_selection_readiness_blocker_component_count()
            && self.component_accounting_consistent
                == self
                    .readiness
                    .attention_selection_readiness_accounting_is_consistent()
    }
}

pub trait AttentionPolicy {
    fn select(
        &self,
        candidates: &[AttentionCandidate],
        context: RoutingContext,
        switches: ExperimentSwitches,
    ) -> AttentionDecision;

    fn observe(&mut self, feedback: RoutingFeedback);
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThresholdAttentionPolicy {
    thresholds: AttentionThresholds,
    base_threshold: f32,
    min_threshold: f32,
    max_threshold: f32,
    learning_rate: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThresholdAttentionPolicySummary {
    pub base_threshold: f32,
    pub min_threshold: f32,
    pub max_threshold: f32,
    pub learning_rate: f32,
    pub general_threshold: f32,
    pub coding_threshold: f32,
    pub writing_threshold: f32,
    pub long_document_threshold: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThresholdAttentionAdjustmentReport {
    pub profile: TaskProfile,
    pub feedback: RoutingFeedbackSummary,
    pub previous_threshold: f32,
    pub adjusted_threshold: f32,
    pub threshold_delta: f32,
    pub min_threshold: f32,
    pub max_threshold: f32,
    pub action: ThresholdAttentionAdjustmentAction,
    pub can_commit: bool,
    pub requires_repair_first: bool,
    pub reason_codes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThresholdAttentionAdjustmentAction {
    LowerThresholdForQualityRepair,
    RaiseThresholdForComputeSavings,
    KeepThreshold,
    RepairThresholdAdjustment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThresholdAttentionPolicyAction {
    UseThresholdPolicy,
    RepairThresholdPolicy,
}

impl ThresholdAttentionPolicyAction {
    pub fn can_use(self) -> bool {
        matches!(self, Self::UseThresholdPolicy)
    }

    pub fn should_repair(self) -> bool {
        matches!(self, Self::RepairThresholdPolicy)
    }
}

impl ThresholdAttentionAdjustmentAction {
    pub fn can_commit(self) -> bool {
        !matches!(self, Self::RepairThresholdAdjustment)
    }

    pub fn should_repair(self) -> bool {
        matches!(self, Self::RepairThresholdAdjustment)
    }

    pub fn changes_threshold(self) -> bool {
        matches!(
            self,
            Self::LowerThresholdForQualityRepair | Self::RaiseThresholdForComputeSavings
        )
    }
}

impl ThresholdAttentionAdjustmentReport {
    pub fn threshold_changed(&self) -> bool {
        !float_close(self.previous_threshold, self.adjusted_threshold)
    }

    pub fn threshold_lowered(&self) -> bool {
        self.adjusted_threshold < self.previous_threshold && self.threshold_changed()
    }

    pub fn threshold_raised(&self) -> bool {
        self.adjusted_threshold > self.previous_threshold && self.threshold_changed()
    }

    pub fn expected_to_restore_quality(&self) -> bool {
        self.can_commit
            && self.threshold_lowered()
            && matches!(
                self.action,
                ThresholdAttentionAdjustmentAction::LowerThresholdForQualityRepair
            )
    }

    pub fn expected_to_reduce_attention_compute(&self) -> bool {
        self.can_commit
            && self.threshold_raised()
            && matches!(
                self.action,
                ThresholdAttentionAdjustmentAction::RaiseThresholdForComputeSavings
            )
    }

    pub fn threshold_bounds_are_valid(&self) -> bool {
        self.min_threshold.is_finite()
            && self.max_threshold.is_finite()
            && self.min_threshold <= self.max_threshold
            && self.previous_threshold.is_finite()
            && self.adjusted_threshold.is_finite()
            && self.previous_threshold >= self.min_threshold
            && self.previous_threshold <= self.max_threshold
            && self.adjusted_threshold >= self.min_threshold
            && self.adjusted_threshold <= self.max_threshold
    }

    pub fn threshold_delta_matches_thresholds(&self) -> bool {
        self.threshold_delta.is_finite()
            && float_close(
                self.threshold_delta,
                self.adjusted_threshold - self.previous_threshold,
            )
    }

    pub fn action_matches_delta(&self) -> bool {
        match self.action {
            ThresholdAttentionAdjustmentAction::LowerThresholdForQualityRepair => {
                self.threshold_lowered()
            }
            ThresholdAttentionAdjustmentAction::RaiseThresholdForComputeSavings => {
                self.threshold_raised()
            }
            ThresholdAttentionAdjustmentAction::KeepThreshold => !self.threshold_changed(),
            ThresholdAttentionAdjustmentAction::RepairThresholdAdjustment => {
                self.requires_repair_first
            }
        }
    }

    pub fn threshold_adjustment_signal_component_count(&self) -> usize {
        usize::from(self.feedback.has_feedback_signal_components())
            + usize::from(self.threshold_changed())
            + usize::from(self.expected_to_restore_quality())
            + usize::from(self.expected_to_reduce_attention_compute())
            + usize::from(self.threshold_bounds_are_valid())
    }

    pub fn has_threshold_adjustment_signal_components(&self) -> bool {
        self.threshold_adjustment_signal_component_count() > 0
    }

    pub fn threshold_adjustment_blocker_component_count(&self) -> usize {
        usize::from(!self.feedback.can_use_routing_feedback())
            + usize::from(!self.threshold_bounds_are_valid())
            + usize::from(!self.threshold_delta_matches_thresholds())
            + usize::from(!self.action_matches_delta())
            + usize::from(self.action.should_repair() && !self.requires_repair_first)
            + usize::from(self.can_commit != self.action.can_commit())
    }

    pub fn has_threshold_adjustment_blockers(&self) -> bool {
        self.threshold_adjustment_blocker_component_count() > 0
    }

    pub fn threshold_adjustment_accounting_is_consistent(&self) -> bool {
        let expected_signal_count = usize::from(self.feedback.has_feedback_signal_components())
            .saturating_add(usize::from(self.threshold_changed()))
            .saturating_add(usize::from(self.expected_to_restore_quality()))
            .saturating_add(usize::from(self.expected_to_reduce_attention_compute()))
            .saturating_add(usize::from(self.threshold_bounds_are_valid()));
        let expected_blocker_count = usize::from(!self.feedback.can_use_routing_feedback())
            .saturating_add(usize::from(!self.threshold_bounds_are_valid()))
            .saturating_add(usize::from(!self.threshold_delta_matches_thresholds()))
            .saturating_add(usize::from(!self.action_matches_delta()))
            .saturating_add(usize::from(
                self.action.should_repair() && !self.requires_repair_first,
            ))
            .saturating_add(usize::from(self.can_commit != self.action.can_commit()));

        self.threshold_adjustment_signal_component_count() == expected_signal_count
            && self.threshold_adjustment_blocker_component_count() == expected_blocker_count
    }

    pub fn threshold_adjustment_is_clean(&self) -> bool {
        !self.has_threshold_adjustment_blockers()
            && self.threshold_adjustment_accounting_is_consistent()
    }

    pub fn can_commit_threshold_adjustment(&self) -> bool {
        self.can_commit && self.threshold_adjustment_is_clean()
    }
}

impl ThresholdAttentionPolicySummary {
    pub fn thresholds_are_finite(self) -> bool {
        self.base_threshold.is_finite()
            && self.min_threshold.is_finite()
            && self.max_threshold.is_finite()
            && self.general_threshold.is_finite()
            && self.coding_threshold.is_finite()
            && self.writing_threshold.is_finite()
            && self.long_document_threshold.is_finite()
    }

    pub fn learning_rate_is_valid(self) -> bool {
        self.learning_rate.is_finite() && self.learning_rate >= 0.0
    }

    pub fn thresholds_are_bounded(self) -> bool {
        self.min_threshold <= self.max_threshold
            && self.base_threshold >= self.min_threshold
            && self.base_threshold <= self.max_threshold
            && self.general_threshold >= self.min_threshold
            && self.general_threshold <= self.max_threshold
            && self.coding_threshold >= self.min_threshold
            && self.coding_threshold <= self.max_threshold
            && self.writing_threshold >= self.min_threshold
            && self.writing_threshold <= self.max_threshold
            && self.long_document_threshold >= self.min_threshold
            && self.long_document_threshold <= self.max_threshold
    }

    pub fn threshold_for(self, profile: TaskProfile) -> f32 {
        match profile {
            TaskProfile::General => self.general_threshold,
            TaskProfile::Coding => self.coding_threshold,
            TaskProfile::Writing => self.writing_threshold,
            TaskProfile::LongDocument => self.long_document_threshold,
        }
    }

    pub fn threshold_spread(self) -> f32 {
        let min = self
            .general_threshold
            .min(self.coding_threshold)
            .min(self.writing_threshold)
            .min(self.long_document_threshold);
        let max = self
            .general_threshold
            .max(self.coding_threshold)
            .max(self.writing_threshold)
            .max(self.long_document_threshold);

        max - min
    }

    pub fn profile_thresholds_match_base(self) -> bool {
        float_close(self.general_threshold, self.base_threshold)
            && float_close(self.coding_threshold, self.base_threshold)
            && float_close(self.writing_threshold, self.base_threshold)
            && float_close(self.long_document_threshold, self.base_threshold)
    }

    pub fn adapted_profile_count(self) -> usize {
        [
            self.general_threshold,
            self.coding_threshold,
            self.writing_threshold,
            self.long_document_threshold,
        ]
        .into_iter()
        .filter(|threshold| !float_close(*threshold, self.base_threshold))
        .count()
    }

    pub fn threshold_policy_signal_component_count(self) -> usize {
        usize::from(self.thresholds_are_bounded())
            + usize::from(self.profile_thresholds_match_base())
            + usize::from(self.threshold_spread() > 0.0 && self.thresholds_are_finite())
            + usize::from(self.adapted_profile_count() > 0)
    }

    pub fn has_threshold_policy_signal_components(self) -> bool {
        self.threshold_policy_signal_component_count() > 0
    }

    pub fn threshold_policy_problem_component_count(self) -> usize {
        usize::from(!self.thresholds_are_finite())
            + usize::from(!self.learning_rate_is_valid())
            + usize::from(!self.thresholds_are_bounded())
    }

    pub fn has_threshold_policy_problem_components(self) -> bool {
        self.threshold_policy_problem_component_count() > 0
    }

    pub fn threshold_policy_accounting_is_consistent(self) -> bool {
        let expected_signal_count = usize::from(self.thresholds_are_bounded())
            .saturating_add(usize::from(self.profile_thresholds_match_base()))
            .saturating_add(usize::from(
                self.threshold_spread() > 0.0 && self.thresholds_are_finite(),
            ))
            .saturating_add(usize::from(self.adapted_profile_count() > 0));
        let expected_problem_count = usize::from(!self.thresholds_are_finite())
            .saturating_add(usize::from(!self.learning_rate_is_valid()))
            .saturating_add(usize::from(!self.thresholds_are_bounded()));

        self.threshold_policy_signal_component_count() == expected_signal_count
            && self.threshold_policy_problem_component_count() == expected_problem_count
    }

    pub fn threshold_policy_shape_is_clean(self) -> bool {
        !self.has_threshold_policy_problem_components()
            && self.threshold_policy_accounting_is_consistent()
    }

    pub fn can_use_threshold_policy(self) -> bool {
        self.threshold_policy_shape_is_clean()
    }

    pub fn threshold_policy_action(self) -> ThresholdAttentionPolicyAction {
        if self.can_use_threshold_policy() {
            ThresholdAttentionPolicyAction::UseThresholdPolicy
        } else {
            ThresholdAttentionPolicyAction::RepairThresholdPolicy
        }
    }

    pub fn threshold_policy_admission_signal_component_count(self) -> usize {
        self.threshold_policy_signal_component_count()
    }

    pub fn has_threshold_policy_admission_signals(self) -> bool {
        self.threshold_policy_admission_signal_component_count() > 0
    }

    pub fn threshold_policy_admission_blocker_component_count(self) -> usize {
        self.threshold_policy_problem_component_count()
    }

    pub fn has_threshold_policy_admission_blockers(self) -> bool {
        self.threshold_policy_admission_blocker_component_count() > 0
    }

    pub fn threshold_policy_admission_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self.threshold_policy_signal_component_count();
        let expected_blocker_count = self.threshold_policy_problem_component_count();

        self.threshold_policy_accounting_is_consistent()
            && self.threshold_policy_admission_signal_component_count() == expected_signal_count
            && self.has_threshold_policy_admission_signals() == (expected_signal_count > 0)
            && self.threshold_policy_admission_blocker_component_count() == expected_blocker_count
            && self.has_threshold_policy_admission_blockers() == (expected_blocker_count > 0)
    }

    pub fn threshold_policy_admission_is_clean(self) -> bool {
        !self.has_threshold_policy_admission_blockers()
            && self.threshold_policy_admission_accounting_is_consistent()
    }

    pub fn can_admit_threshold_policy(self) -> bool {
        self.can_use_threshold_policy() && self.threshold_policy_admission_is_clean()
    }
}

impl ThresholdAttentionPolicy {
    pub fn new(base_threshold: f32) -> Self {
        let defaults = Self::default();
        let base_threshold = base_threshold.clamp(defaults.min_threshold, defaults.max_threshold);
        Self {
            thresholds: AttentionThresholds::from_single(base_threshold),
            base_threshold,
            ..defaults
        }
    }

    pub fn threshold_for(&self, profile: TaskProfile) -> f32 {
        self.thresholds
            .get(profile)
            .clamp(self.min_threshold, self.max_threshold)
    }

    pub fn policy_summary(&self) -> ThresholdAttentionPolicySummary {
        ThresholdAttentionPolicySummary {
            base_threshold: self.base_threshold,
            min_threshold: self.min_threshold,
            max_threshold: self.max_threshold,
            learning_rate: self.learning_rate,
            general_threshold: self.threshold_for(TaskProfile::General),
            coding_threshold: self.threshold_for(TaskProfile::Coding),
            writing_threshold: self.threshold_for(TaskProfile::Writing),
            long_document_threshold: self.threshold_for(TaskProfile::LongDocument),
        }
    }

    fn threshold_for_context(&self, profile: TaskProfile, switches: ExperimentSwitches) -> f32 {
        if switches.enable_adaptive_attention_thresholds {
            self.threshold_for(profile)
        } else {
            self.base_threshold
        }
    }

    pub fn preview_adjustment(
        &self,
        feedback: RoutingFeedback,
    ) -> ThresholdAttentionAdjustmentReport {
        let feedback_summary = feedback.feedback_summary();
        let policy_summary = self.policy_summary();
        let previous_threshold = self.threshold_for(feedback.profile);
        let mut target_threshold = previous_threshold;
        let mut reason_codes = Vec::new();

        let feedback_is_clean = feedback_summary.can_use_routing_feedback();
        let policy_is_clean = policy_summary.can_use_threshold_policy();

        if !feedback_is_clean {
            reason_codes.push("attention_threshold_feedback_requires_repair".to_owned());
        }

        if !policy_is_clean {
            reason_codes.push("attention_threshold_policy_requires_repair".to_owned());
        }

        if feedback_is_clean && policy_is_clean {
            let contradiction_pressure = (feedback.contradiction_count as f32 * 0.02).min(0.10);

            if feedback.quality < 0.60 {
                target_threshold -=
                    self.learning_rate * (0.60 - feedback.quality) + contradiction_pressure;
            } else if feedback.quality > 0.84 && feedback.perplexity <= 8.0 {
                target_threshold += self.learning_rate * (feedback.quality - 0.84);
            }
        }

        let adjusted_threshold = target_threshold.clamp(self.min_threshold, self.max_threshold);
        let threshold_delta = adjusted_threshold - previous_threshold;
        let action = if !feedback_is_clean || !policy_is_clean {
            ThresholdAttentionAdjustmentAction::RepairThresholdAdjustment
        } else if adjusted_threshold < previous_threshold {
            ThresholdAttentionAdjustmentAction::LowerThresholdForQualityRepair
        } else if adjusted_threshold > previous_threshold {
            ThresholdAttentionAdjustmentAction::RaiseThresholdForComputeSavings
        } else {
            ThresholdAttentionAdjustmentAction::KeepThreshold
        };
        let requires_repair_first = action.should_repair();
        let can_commit = action.can_commit();

        if action == ThresholdAttentionAdjustmentAction::LowerThresholdForQualityRepair {
            reason_codes.push("attention_threshold_lowered_for_quality_repair".to_owned());
        } else if action == ThresholdAttentionAdjustmentAction::RaiseThresholdForComputeSavings {
            reason_codes.push("attention_threshold_raised_for_compute_savings".to_owned());
        } else if action == ThresholdAttentionAdjustmentAction::KeepThreshold {
            reason_codes.push("attention_threshold_kept".to_owned());
        }

        if feedback_is_clean
            && policy_is_clean
            && !float_close(target_threshold, adjusted_threshold)
        {
            reason_codes.push("attention_threshold_adjustment_clamped".to_owned());
        }

        ThresholdAttentionAdjustmentReport {
            profile: feedback.profile,
            feedback: feedback_summary,
            previous_threshold,
            adjusted_threshold,
            threshold_delta,
            min_threshold: self.min_threshold,
            max_threshold: self.max_threshold,
            action,
            can_commit,
            requires_repair_first,
            reason_codes,
        }
    }

    pub fn observe_with_report(
        &mut self,
        feedback: RoutingFeedback,
    ) -> ThresholdAttentionAdjustmentReport {
        let report = self.preview_adjustment(feedback);

        if report.can_commit_threshold_adjustment() {
            self.thresholds
                .set(report.profile, report.adjusted_threshold);
        }

        report
    }
}

impl Default for ThresholdAttentionPolicy {
    fn default() -> Self {
        Self {
            thresholds: AttentionThresholds::from_single(0.55),
            base_threshold: 0.55,
            min_threshold: 0.15,
            max_threshold: 0.90,
            learning_rate: 0.07,
        }
    }
}

impl AttentionPolicy for ThresholdAttentionPolicy {
    fn select(
        &self,
        candidates: &[AttentionCandidate],
        context: RoutingContext,
        switches: ExperimentSwitches,
    ) -> AttentionDecision {
        let threshold = self.threshold_for_context(context.profile, switches);
        let max_selected = switches.max_attention_tokens.max(1);
        let mut selected = candidates
            .iter()
            .filter(|candidate| candidate.layer.uses_attention() && candidate.score >= threshold)
            .cloned()
            .collect::<Vec<_>>();

        selected.sort_by(|left, right| {
            right
                .score
                .total_cmp(&left.score)
                .then_with(|| left.position.cmp(&right.position))
        });
        selected.truncate(max_selected);
        selected.sort_by_key(|candidate| candidate.position);

        let rejected = candidates
            .iter()
            .filter(|candidate| {
                !selected
                    .iter()
                    .any(|selected| selected.identity() == candidate.identity())
            })
            .cloned()
            .collect();

        AttentionDecision {
            threshold,
            max_selected,
            selected,
            rejected,
        }
    }

    fn observe(&mut self, feedback: RoutingFeedback) {
        self.observe_with_report(feedback);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct AttentionThresholds {
    general: f32,
    coding: f32,
    writing: f32,
    long_document: f32,
}

impl AttentionThresholds {
    fn from_single(threshold: f32) -> Self {
        Self {
            general: threshold,
            coding: threshold,
            writing: threshold,
            long_document: threshold,
        }
    }

    fn get(self, profile: TaskProfile) -> f32 {
        match profile {
            TaskProfile::General => self.general,
            TaskProfile::Coding => self.coding,
            TaskProfile::Writing => self.writing,
            TaskProfile::LongDocument => self.long_document,
        }
    }

    fn set(&mut self, profile: TaskProfile, threshold: f32) {
        match profile {
            TaskProfile::General => self.general = threshold,
            TaskProfile::Coding => self.coding = threshold,
            TaskProfile::Writing => self.writing = threshold,
            TaskProfile::LongDocument => self.long_document = threshold,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CandidateIdentity<'a> {
    token: &'a str,
    position: usize,
    layer: RouteLayer,
}

fn layer_counts_from_candidates(candidates: &[AttentionCandidate]) -> RouteLayerCounts {
    let mut counts = RouteLayerCounts::default();
    for candidate in candidates {
        counts.bump(candidate.layer);
    }
    counts
}

fn combined_layer_counts(left: RouteLayerCounts, right: RouteLayerCounts) -> RouteLayerCounts {
    RouteLayerCounts {
        fast_projection: left.fast_projection.saturating_add(right.fast_projection),
        local_window: left.local_window.saturating_add(right.local_window),
        global: left.global.saturating_add(right.global),
        fusion: left.fusion.saturating_add(right.fusion),
    }
}

fn float_close(left: f32, right: f32) -> bool {
    (left - right).abs() <= 0.0001
}

fn finite_unit(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attention_candidate_summary_preserves_route_boundary_shape() {
        let decision = RoutingDecision {
            token: "borrow_checker".to_string(),
            score: 0.74,
            layer: RouteLayer::LocalWindow,
        };

        let candidate = AttentionCandidate::from_route(&decision, 7, 0.83);
        let summary = candidate.candidate_summary();

        assert_eq!(summary.token, "borrow_checker");
        assert_eq!(summary.position, 7);
        assert_eq!(summary.score, 0.74);
        assert_eq!(summary.entropy, 0.83);
        assert_eq!(summary.layer, RouteLayer::LocalWindow);
        assert!(summary.uses_attention);
        assert!(!summary.is_fast_projection());
        assert!(summary.is_high_entropy(0.80));
        assert!(summary.score_reaches(0.70));
        assert!(!summary.score_reaches(0.80));
        assert!(summary.token_shape_is_valid());
        assert!(summary.score_shape_is_valid());
        assert!(summary.entropy_shape_is_valid());
        assert!(summary.uses_attention_matches_layer());
        assert_eq!(summary.candidate_signal_component_count(), 4);
        assert!(summary.has_candidate_signal_components());
        assert_eq!(summary.candidate_problem_component_count(), 0);
        assert!(!summary.has_candidate_problem_components());
        assert!(summary.candidate_accounting_is_consistent());
        assert!(summary.candidate_shape_is_clean());
        assert!(summary.can_use_attention_candidate());
    }

    #[test]
    fn attention_candidate_batch_summary_counts_layer_mix_and_pressure() {
        let candidates = [
            AttentionCandidate::new("fast", 0, 0.20, 0.10, RouteLayer::FastProjection),
            AttentionCandidate::new("local", 1, 0.70, 0.60, RouteLayer::LocalWindow),
            AttentionCandidate::new("global", 2, 0.90, 0.80, RouteLayer::Global),
            AttentionCandidate::new("fusion", 3, 0.80, 0.70, RouteLayer::Fusion),
        ];

        let summary = AttentionCandidate::batch_summary(&candidates);

        assert_eq!(summary.candidate_count, 4);
        assert_eq!(summary.attention_candidate_count, 3);
        assert_eq!(summary.fast_candidate_count, 1);
        assert_eq!(summary.layer_counts.fast_projection, 1);
        assert_eq!(summary.layer_counts.local_window, 1);
        assert_eq!(summary.layer_counts.global, 1);
        assert_eq!(summary.layer_counts.fusion, 1);
        assert!((summary.average_score - 0.65).abs() < 0.0001);
        assert!((summary.average_entropy - 0.55).abs() < 0.0001);
        assert_eq!(summary.max_score, 0.90);
        assert_eq!(summary.max_entropy, 0.80);
        assert!(!summary.is_empty());
        assert!(summary.has_attention_candidates());
        assert!(summary.has_fast_candidates());
        assert!(summary.layer_counts_match_candidates());
        assert!(!summary.all_attention_candidates());
        assert!(summary.uses_multiple_layers());
        assert!((summary.attention_candidate_fraction() - 0.75).abs() < 0.0001);
        assert!(summary.attention_candidate_count_matches_total());
        assert!(summary.score_shape_is_valid());
        assert!(summary.entropy_shape_is_valid());
        assert!(summary.attention_fraction_shape_is_valid());
        assert_eq!(summary.candidate_batch_activity_signal_component_count(), 3);
        assert_eq!(summary.candidate_batch_layer_signal_component_count(), 3);
        assert_eq!(summary.candidate_batch_score_signal_component_count(), 4);
        assert_eq!(summary.candidate_batch_signal_component_count(), 10);
        assert!(summary.has_candidate_batch_signal_components());
        assert_eq!(summary.candidate_batch_count_problem_component_count(), 0);
        assert_eq!(summary.candidate_batch_score_problem_component_count(), 0);
        assert_eq!(summary.candidate_batch_problem_component_count(), 0);
        assert!(!summary.has_candidate_batch_problem_components());
        assert!(summary.candidate_batch_accounting_is_consistent());
        assert!(summary.candidate_batch_shape_is_clean());
        assert!(summary.can_use_attention_candidate_batch());
    }

    #[test]
    fn empty_attention_candidate_batch_summary_is_noop() {
        let summary = AttentionCandidateBatchSummary::from_candidates(&[]);

        assert_eq!(summary.candidate_count, 0);
        assert_eq!(summary.attention_candidate_count, 0);
        assert_eq!(summary.fast_candidate_count, 0);
        assert_eq!(summary.layer_counts.total(), 0);
        assert_eq!(summary.average_score, 0.0);
        assert_eq!(summary.average_entropy, 0.0);
        assert_eq!(summary.max_score, 0.0);
        assert_eq!(summary.max_entropy, 0.0);
        assert!(summary.is_empty());
        assert!(!summary.has_attention_candidates());
        assert!(!summary.has_fast_candidates());
        assert!(summary.layer_counts_match_candidates());
        assert!(!summary.all_attention_candidates());
        assert!(!summary.uses_multiple_layers());
        assert_eq!(summary.attention_candidate_fraction(), 0.0);
        assert_eq!(summary.candidate_batch_signal_component_count(), 0);
        assert!(!summary.has_candidate_batch_signal_components());
        assert_eq!(summary.candidate_batch_problem_component_count(), 0);
        assert!(!summary.has_candidate_batch_problem_components());
        assert!(summary.candidate_batch_accounting_is_consistent());
        assert!(summary.candidate_batch_shape_is_clean());
        assert!(!summary.can_use_attention_candidate_batch());
    }

    #[test]
    fn attention_candidate_batch_summary_marks_all_attention_sets() {
        let candidates = [
            AttentionCandidate::new("local", 1, 0.70, 0.60, RouteLayer::LocalWindow),
            AttentionCandidate::new("fusion", 2, 0.80, 0.70, RouteLayer::Fusion),
        ];

        let summary = AttentionCandidate::batch_summary(&candidates);

        assert_eq!(summary.candidate_count, 2);
        assert_eq!(summary.attention_candidate_count, 2);
        assert_eq!(summary.fast_candidate_count, 0);
        assert!(summary.has_attention_candidates());
        assert!(!summary.has_fast_candidates());
        assert!(summary.layer_counts_match_candidates());
        assert!(summary.all_attention_candidates());
        assert!(summary.uses_multiple_layers());
        assert_eq!(summary.attention_candidate_fraction(), 1.0);
        assert_eq!(summary.candidate_batch_activity_signal_component_count(), 3);
        assert_eq!(summary.candidate_batch_layer_signal_component_count(), 3);
        assert_eq!(summary.candidate_batch_score_signal_component_count(), 4);
        assert_eq!(summary.candidate_batch_signal_component_count(), 10);
        assert_eq!(summary.candidate_batch_problem_component_count(), 0);
        assert!(summary.candidate_batch_accounting_is_consistent());
        assert!(summary.candidate_batch_shape_is_clean());
        assert!(summary.can_use_attention_candidate_batch());
    }

    #[test]
    fn attention_candidate_summaries_count_public_shape_drift() {
        let candidate = AttentionCandidateSummary {
            token: String::new(),
            position: 0,
            score: 1.2,
            entropy: f32::NAN,
            layer: RouteLayer::FastProjection,
            uses_attention: true,
        };

        assert!(!candidate.token_shape_is_valid());
        assert!(!candidate.score_shape_is_valid());
        assert!(!candidate.entropy_shape_is_valid());
        assert!(!candidate.uses_attention_matches_layer());
        assert_eq!(candidate.candidate_signal_component_count(), 2);
        assert_eq!(candidate.candidate_problem_component_count(), 4);
        assert!(candidate.has_candidate_problem_components());
        assert!(candidate.candidate_accounting_is_consistent());
        assert!(!candidate.candidate_shape_is_clean());
        assert!(!candidate.can_use_attention_candidate());

        let batch = AttentionCandidateBatchSummary {
            candidate_count: 2,
            attention_candidate_count: 3,
            fast_candidate_count: 1,
            layer_counts: RouteLayerCounts {
                fast_projection: 1,
                local_window: 1,
                global: 0,
                fusion: 0,
            },
            average_score: 0.8,
            average_entropy: f32::NAN,
            max_score: 0.7,
            max_entropy: 0.5,
        };

        assert!(!batch.layer_counts_match_candidates());
        assert!(!batch.attention_candidate_count_matches_total());
        assert!(!batch.score_shape_is_valid());
        assert!(!batch.entropy_shape_is_valid());
        assert!(!batch.attention_fraction_shape_is_valid());
        assert_eq!(batch.candidate_batch_count_problem_component_count(), 2);
        assert_eq!(batch.candidate_batch_score_problem_component_count(), 3);
        assert_eq!(batch.candidate_batch_problem_component_count(), 5);
        assert!(batch.has_candidate_batch_problem_components());
        assert!(batch.candidate_batch_accounting_is_consistent());
        assert!(!batch.candidate_batch_shape_is_clean());
        assert!(!batch.can_use_attention_candidate_batch());
    }

    #[test]
    fn threshold_policy_filters_and_caps_candidates() {
        let policy = ThresholdAttentionPolicy::new(0.50);
        let switches = ExperimentSwitches {
            max_attention_tokens: 2,
            ..ExperimentSwitches::default()
        };
        let candidates = [
            AttentionCandidate::new("fast", 0, 0.99, 0.10, RouteLayer::FastProjection),
            AttentionCandidate::new("keep-local", 1, 0.74, 0.40, RouteLayer::LocalWindow),
            AttentionCandidate::new("drop-low", 2, 0.49, 0.90, RouteLayer::Global),
            AttentionCandidate::new("keep-global", 3, 0.96, 0.80, RouteLayer::Global),
            AttentionCandidate::new("over-budget", 4, 0.60, 0.50, RouteLayer::Fusion),
        ];

        let decision = policy.select(&candidates, RoutingContext::default(), switches);

        assert_eq!(
            decision.selected_tokens(),
            vec!["keep-local", "keep-global"]
        );
        assert_eq!(decision.selected.len(), 2);
        assert_eq!(decision.rejected.len(), 3);
        assert_eq!(decision.selected_count(), 2);
        assert_eq!(decision.rejected_count(), 3);
        assert_eq!(decision.candidate_count(), 5);
        assert!((decision.selection_fraction() - 0.4).abs() < 0.0001);
        assert!(decision.hit_selection_cap());

        let summary = decision.decision_summary();
        assert_eq!(summary.threshold, 0.50);
        assert_eq!(summary.max_selected, 2);
        assert_eq!(summary.candidate_count, 5);
        assert_eq!(summary.selected_count, 2);
        assert_eq!(summary.rejected_count, 3);
        assert!((summary.selection_fraction - 0.4).abs() < 0.0001);
        assert!(summary.hit_selection_cap);
        assert_eq!(summary.selected_layer_counts.local_window, 1);
        assert_eq!(summary.selected_layer_counts.global, 1);
        assert_eq!(summary.selected_attention_tokens(), 2);
        assert_eq!(summary.rejected_layer_counts.fast_projection, 1);
        assert_eq!(summary.rejected_layer_counts.global, 1);
        assert_eq!(summary.rejected_layer_counts.fusion, 1);
        assert_eq!(summary.rejected_attention_tokens(), 2);
        assert!(!summary.is_empty());
        assert!(summary.selected_counts_match_layers());
        assert!(summary.rejected_counts_match_layers());
        assert!(summary.candidate_accounting_balanced());
        assert!(summary.has_selected_attention());
        assert!(summary.has_rejected_attention());
        assert_eq!(summary.selected_attention_fraction(), 1.0);
        assert!((summary.rejected_attention_fraction() - (2.0 / 3.0)).abs() < 0.0001);
        assert!(summary.has_selection_pressure());
        assert!(summary.threshold_shape_is_valid());
        assert!(summary.selection_fraction_matches_counts());
        assert!(summary.selected_count_within_cap());
        assert_eq!(summary.decision_activity_signal_component_count(), 4);
        assert_eq!(summary.decision_attention_signal_component_count(), 3);
        assert_eq!(summary.decision_signal_component_count(), 7);
        assert!(summary.has_decision_signal_components());
        assert_eq!(summary.decision_count_problem_component_count(), 0);
        assert_eq!(summary.decision_shape_problem_component_count(), 0);
        assert_eq!(summary.decision_problem_component_count(), 0);
        assert!(!summary.has_decision_problem_components());
        assert!(summary.decision_accounting_is_consistent());
        assert!(summary.decision_shape_is_clean());
        assert!(summary.can_use_attention_decision());

        let candidate_batch = AttentionCandidate::batch_summary(&candidates);
        let readiness =
            AttentionSelectionReadinessSummary::from_decision(candidate_batch, &decision);
        assert_eq!(
            AttentionSelectionReadinessSummary::stage_order(),
            [
                AttentionSelectionReadinessStage::CandidateBatch,
                AttentionSelectionReadinessStage::Decision,
                AttentionSelectionReadinessStage::SelectionBoundary,
            ]
        );
        assert_eq!(
            readiness.selected_and_rejected_layer_counts(),
            candidate_batch.layer_counts
        );
        assert!(readiness.candidate_batch_ready());
        assert!(readiness.decision_ready());
        assert!(readiness.selection_boundary_ready());
        assert!(readiness.candidate_count_matches_decision());
        assert!(readiness.attention_candidate_count_matches_decision());
        assert!(readiness.fast_candidate_count_matches_decision());
        assert!(readiness.layer_counts_match_decision());
        assert!(readiness.selection_boundary_matches());
        assert_eq!(readiness.selection_boundary_drift_component_count(), 0);
        assert_eq!(readiness.first_unready_stage(), None);
        assert_eq!(readiness.first_blocking_stage(), None);
        assert_eq!(readiness.candidate_batch_signal_component_count, 10);
        assert_eq!(readiness.decision_signal_component_count, 7);
        assert_eq!(readiness.selection_boundary_signal_component_count, 1);
        assert_eq!(
            readiness
                .stage_signal_component_count(AttentionSelectionReadinessStage::CandidateBatch),
            readiness.candidate_batch_signal_component_count
        );
        assert_eq!(
            readiness
                .stage_blocker_component_count(AttentionSelectionReadinessStage::SelectionBoundary),
            readiness.selection_boundary_blocker_component_count
        );
        assert_eq!(
            readiness.attention_selection_readiness_signal_component_count(),
            18
        );
        assert!(readiness.has_attention_selection_readiness_signals());
        assert_eq!(
            readiness.attention_selection_readiness_blocker_component_count(),
            0
        );
        assert!(!readiness.has_attention_selection_readiness_blockers());
        assert!(readiness.attention_selection_readiness_accounting_is_consistent());
        assert!(readiness.attention_selection_readiness_is_clean());
        assert!(readiness.can_commit_attention_selection_readiness());
        assert_eq!(
            readiness.attention_selection_readiness_commit_action(),
            AttentionSelectionReadinessCommitAction::CommitAttentionSelection
        );
        assert!(
            readiness
                .attention_selection_readiness_commit_action()
                .can_commit()
        );
        assert!(
            !readiness
                .attention_selection_readiness_commit_action()
                .should_wait()
        );
        assert!(
            !readiness
                .attention_selection_readiness_commit_action()
                .should_repair()
        );
    }

    #[test]
    fn attention_selection_readiness_commit_summary_exposes_admission_boundary() {
        let policy = ThresholdAttentionPolicy::new(0.50);
        let switches = ExperimentSwitches {
            max_attention_tokens: 2,
            ..ExperimentSwitches::default()
        };
        let candidates = [
            AttentionCandidate::new("fast", 0, 0.99, 0.10, RouteLayer::FastProjection),
            AttentionCandidate::new("keep-local", 1, 0.74, 0.40, RouteLayer::LocalWindow),
            AttentionCandidate::new("drop-low", 2, 0.49, 0.90, RouteLayer::Global),
            AttentionCandidate::new("keep-global", 3, 0.96, 0.80, RouteLayer::Global),
            AttentionCandidate::new("over-budget", 4, 0.60, 0.50, RouteLayer::Fusion),
        ];
        let decision = policy.select(&candidates, RoutingContext::default(), switches);
        let candidate_batch = AttentionCandidate::batch_summary(&candidates);
        let ready = AttentionSelectionReadinessSummary::from_decision(candidate_batch, &decision)
            .commit_summary();

        assert_eq!(
            ready.action,
            AttentionSelectionReadinessCommitAction::CommitAttentionSelection
        );
        assert!(ready.action_can_commit());
        assert!(!ready.action_should_wait_for_attention_selection());
        assert!(!ready.action_should_repair_attention_selection());
        assert!(ready.can_commit_attention_selection());
        assert!(!ready.should_wait_for_attention_selection());
        assert!(!ready.should_repair_attention_selection());
        assert_eq!(
            ready.committed_attention_decision,
            Some(decision.decision_summary())
        );
        assert!(ready.can_use_committed_attention_decision());
        assert_eq!(ready.first_unready_stage, None);
        assert_eq!(ready.first_blocking_stage, None);
        assert_eq!(ready.total_signal_component_count, 18);
        assert_eq!(ready.total_blocker_component_count, 0);
        assert!(ready.component_accounting_consistent);
        assert!(ready.commit_decision_accounting_is_consistent());

        let empty = AttentionSelectionReadinessSummary::new(
            AttentionCandidateBatchSummary::from_candidates(&[]),
            AttentionDecision {
                threshold: 0.50,
                max_selected: 2,
                selected: Vec::new(),
                rejected: Vec::new(),
            }
            .decision_summary(),
        )
        .commit_summary();

        assert_eq!(
            empty.action,
            AttentionSelectionReadinessCommitAction::WaitForAttentionSelection
        );
        assert!(!empty.action_can_commit());
        assert!(empty.action_should_wait_for_attention_selection());
        assert!(!empty.action_should_repair_attention_selection());
        assert!(!empty.can_commit_attention_selection());
        assert!(empty.should_wait_for_attention_selection());
        assert!(!empty.should_repair_attention_selection());
        assert_eq!(empty.committed_attention_decision, None);
        assert!(!empty.can_use_committed_attention_decision());
        assert_eq!(
            empty.first_unready_stage,
            Some(AttentionSelectionReadinessStage::CandidateBatch)
        );
        assert_eq!(empty.first_blocking_stage, None);
        assert_eq!(empty.total_signal_component_count, 0);
        assert_eq!(empty.total_blocker_component_count, 0);
        assert!(empty.component_accounting_consistent);
        assert!(empty.commit_decision_accounting_is_consistent());

        let repair = AttentionSelectionReadinessSummary::from_decision(
            AttentionCandidate::batch_summary(&candidates[..4]),
            &decision,
        )
        .commit_summary();

        assert_eq!(
            repair.action,
            AttentionSelectionReadinessCommitAction::RepairAttentionSelection
        );
        assert!(!repair.action_can_commit());
        assert!(!repair.action_should_wait_for_attention_selection());
        assert!(repair.action_should_repair_attention_selection());
        assert!(!repair.can_commit_attention_selection());
        assert!(!repair.should_wait_for_attention_selection());
        assert!(repair.should_repair_attention_selection());
        assert_eq!(repair.committed_attention_decision, None);
        assert!(!repair.can_use_committed_attention_decision());
        assert_eq!(
            repair.first_unready_stage,
            Some(AttentionSelectionReadinessStage::SelectionBoundary)
        );
        assert_eq!(
            repair.first_blocking_stage,
            Some(AttentionSelectionReadinessStage::SelectionBoundary)
        );
        assert_eq!(repair.total_signal_component_count, 16);
        assert_eq!(repair.total_blocker_component_count, 3);
        assert!(repair.component_accounting_consistent);
        assert!(repair.commit_decision_accounting_is_consistent());
    }

    #[test]
    fn threshold_policy_selects_equal_threshold_attention_without_fast_layer_leak() {
        let policy = ThresholdAttentionPolicy::new(0.50);
        let switches = ExperimentSwitches {
            max_attention_tokens: 4,
            ..ExperimentSwitches::default()
        };
        let candidates = [
            AttentionCandidate::new(
                "fast-at-threshold",
                0,
                0.50,
                0.20,
                RouteLayer::FastProjection,
            ),
            AttentionCandidate::new("equal-local", 1, 0.50, 0.40, RouteLayer::LocalWindow),
            AttentionCandidate::new("below-global", 2, 0.49, 0.80, RouteLayer::Global),
            AttentionCandidate::new("above-fusion", 3, 0.70, 0.60, RouteLayer::Fusion),
        ];

        let decision = policy.select(&candidates, RoutingContext::default(), switches);
        let summary = decision.decision_summary();
        let candidate_batch = AttentionCandidate::batch_summary(&candidates);
        let readiness =
            AttentionSelectionReadinessSummary::from_decision(candidate_batch, &decision);

        assert_eq!(
            decision.selected_tokens(),
            vec!["equal-local", "above-fusion"]
        );
        assert_eq!(decision.selected_count(), 2);
        assert_eq!(decision.rejected_count(), 2);
        assert!(!decision.hit_selection_cap());
        assert_eq!(summary.threshold, 0.50);
        assert_eq!(summary.max_selected, 4);
        assert_eq!(summary.candidate_count, 4);
        assert_eq!(summary.selected_attention_tokens(), 2);
        assert_eq!(summary.rejected_attention_tokens(), 1);
        assert_eq!(summary.selected_layer_counts.local_window, 1);
        assert_eq!(summary.selected_layer_counts.fusion, 1);
        assert_eq!(summary.rejected_layer_counts.fast_projection, 1);
        assert_eq!(summary.rejected_layer_counts.global, 1);
        assert!((summary.selection_fraction - 0.5).abs() < 0.0001);
        assert!(summary.selected_counts_match_layers());
        assert!(summary.rejected_counts_match_layers());
        assert!(summary.candidate_accounting_balanced());
        assert!(summary.threshold_shape_is_valid());
        assert!(summary.selection_fraction_matches_counts());
        assert!(summary.selected_count_within_cap());
        assert!(summary.decision_accounting_is_consistent());
        assert!(summary.can_use_attention_decision());
        assert_eq!(candidate_batch.fast_candidate_count, 1);
        assert_eq!(candidate_batch.attention_candidate_count, 3);
        assert!(readiness.selection_boundary_matches());
        assert_eq!(readiness.selection_boundary_drift_component_count(), 0);
        assert!(readiness.attention_selection_readiness_accounting_is_consistent());
        assert!(readiness.can_commit_attention_selection_readiness());
        assert_eq!(
            readiness.attention_selection_readiness_commit_action(),
            AttentionSelectionReadinessCommitAction::CommitAttentionSelection
        );
        assert!(
            readiness
                .attention_selection_readiness_commit_action()
                .can_commit()
        );
    }

    #[test]
    fn empty_attention_decision_summary_is_balanced_noop() {
        let decision = AttentionDecision {
            threshold: 0.50,
            max_selected: 4,
            selected: Vec::new(),
            rejected: Vec::new(),
        };

        let summary = decision.decision_summary();

        assert_eq!(summary.candidate_count, 0);
        assert_eq!(summary.selected_count, 0);
        assert_eq!(summary.rejected_count, 0);
        assert_eq!(summary.selected_attention_tokens(), 0);
        assert_eq!(summary.rejected_attention_tokens(), 0);
        assert!(summary.is_empty());
        assert!(summary.selected_counts_match_layers());
        assert!(summary.rejected_counts_match_layers());
        assert!(summary.candidate_accounting_balanced());
        assert!(!summary.has_selected_attention());
        assert!(!summary.has_rejected_attention());
        assert_eq!(summary.selected_attention_fraction(), 0.0);
        assert_eq!(summary.rejected_attention_fraction(), 0.0);
        assert!(!summary.has_selection_pressure());
        assert_eq!(summary.decision_signal_component_count(), 0);
        assert_eq!(summary.decision_problem_component_count(), 0);
        assert!(!summary.has_decision_signal_components());
        assert!(!summary.has_decision_problem_components());
        assert!(summary.decision_accounting_is_consistent());
        assert!(summary.decision_shape_is_clean());
        assert!(!summary.can_use_attention_decision());

        let readiness = AttentionSelectionReadinessSummary::new(
            AttentionCandidateBatchSummary::from_candidates(&[]),
            summary,
        );

        assert!(!readiness.candidate_batch_ready());
        assert!(!readiness.decision_ready());
        assert!(readiness.selection_boundary_ready());
        assert_eq!(
            readiness.first_unready_stage(),
            Some(AttentionSelectionReadinessStage::CandidateBatch)
        );
        assert_eq!(readiness.first_blocking_stage(), None);
        assert_eq!(readiness.candidate_batch_signal_component_count, 0);
        assert_eq!(readiness.decision_signal_component_count, 0);
        assert_eq!(readiness.selection_boundary_signal_component_count, 0);
        assert_eq!(
            readiness.attention_selection_readiness_blocker_component_count(),
            0
        );
        assert!(readiness.attention_selection_readiness_accounting_is_consistent());
        assert!(readiness.attention_selection_readiness_is_clean());
        assert!(!readiness.can_commit_attention_selection_readiness());
        assert_eq!(
            readiness.attention_selection_readiness_commit_action(),
            AttentionSelectionReadinessCommitAction::WaitForAttentionSelection
        );
        assert!(
            !readiness
                .attention_selection_readiness_commit_action()
                .can_commit()
        );
        assert!(
            readiness
                .attention_selection_readiness_commit_action()
                .should_wait()
        );
        assert!(
            !readiness
                .attention_selection_readiness_commit_action()
                .should_repair()
        );
    }

    #[test]
    fn attention_selection_readiness_blocks_candidate_decision_drift() {
        let policy = ThresholdAttentionPolicy::new(0.50);
        let switches = ExperimentSwitches {
            max_attention_tokens: 2,
            ..ExperimentSwitches::default()
        };
        let candidates = [
            AttentionCandidate::new("fast", 0, 0.99, 0.10, RouteLayer::FastProjection),
            AttentionCandidate::new("keep-local", 1, 0.74, 0.40, RouteLayer::LocalWindow),
            AttentionCandidate::new("drop-low", 2, 0.49, 0.90, RouteLayer::Global),
            AttentionCandidate::new("keep-global", 3, 0.96, 0.80, RouteLayer::Global),
            AttentionCandidate::new("over-budget", 4, 0.60, 0.50, RouteLayer::Fusion),
        ];
        let decision = policy.select(&candidates, RoutingContext::default(), switches);
        let stale_candidate_batch = AttentionCandidate::batch_summary(&candidates[..4]);
        let readiness =
            AttentionSelectionReadinessSummary::from_decision(stale_candidate_batch, &decision);

        assert!(readiness.candidate_batch_ready());
        assert!(readiness.decision_ready());
        assert!(!readiness.selection_boundary_ready());
        assert!(!readiness.candidate_count_matches_decision());
        assert!(!readiness.attention_candidate_count_matches_decision());
        assert!(readiness.fast_candidate_count_matches_decision());
        assert!(!readiness.layer_counts_match_decision());
        assert_eq!(readiness.selection_boundary_drift_component_count(), 3);
        assert_eq!(
            readiness.first_unready_stage(),
            Some(AttentionSelectionReadinessStage::SelectionBoundary)
        );
        assert_eq!(
            readiness.first_blocking_stage(),
            Some(AttentionSelectionReadinessStage::SelectionBoundary)
        );
        assert_eq!(readiness.candidate_batch_signal_component_count, 9);
        assert_eq!(readiness.decision_signal_component_count, 7);
        assert_eq!(readiness.selection_boundary_signal_component_count, 0);
        assert_eq!(readiness.candidate_batch_blocker_component_count, 0);
        assert_eq!(readiness.decision_blocker_component_count, 0);
        assert_eq!(readiness.selection_boundary_blocker_component_count, 3);
        assert_eq!(
            readiness.attention_selection_readiness_signal_component_count(),
            16
        );
        assert_eq!(
            readiness.attention_selection_readiness_blocker_component_count(),
            3
        );
        assert!(readiness.has_attention_selection_readiness_blockers());
        assert!(readiness.attention_selection_readiness_accounting_is_consistent());
        assert!(!readiness.attention_selection_readiness_is_clean());
        assert!(!readiness.can_commit_attention_selection_readiness());
        assert_eq!(
            readiness.attention_selection_readiness_commit_action(),
            AttentionSelectionReadinessCommitAction::RepairAttentionSelection
        );
        assert!(
            !readiness
                .attention_selection_readiness_commit_action()
                .can_commit()
        );
        assert!(
            !readiness
                .attention_selection_readiness_commit_action()
                .should_wait()
        );
        assert!(
            readiness
                .attention_selection_readiness_commit_action()
                .should_repair()
        );
    }

    #[test]
    fn attention_decision_summary_counts_public_shape_drift() {
        let summary = AttentionDecisionSummary {
            threshold: -0.1,
            max_selected: 1,
            candidate_count: 2,
            selected_count: 3,
            rejected_count: 0,
            selection_fraction: 0.2,
            hit_selection_cap: false,
            selected_layer_counts: RouteLayerCounts {
                fast_projection: 0,
                local_window: 2,
                global: 0,
                fusion: 0,
            },
            rejected_layer_counts: RouteLayerCounts::default(),
        };

        assert!(!summary.candidate_accounting_balanced());
        assert!(!summary.threshold_shape_is_valid());
        assert!(!summary.selection_fraction_matches_counts());
        assert!(!summary.selected_count_within_cap());
        assert_eq!(summary.decision_activity_signal_component_count(), 2);
        assert_eq!(summary.decision_attention_signal_component_count(), 1);
        assert_eq!(summary.decision_signal_component_count(), 3);
        assert_eq!(summary.decision_count_problem_component_count(), 2);
        assert_eq!(summary.decision_shape_problem_component_count(), 2);
        assert_eq!(summary.decision_problem_component_count(), 4);
        assert!(summary.has_decision_problem_components());
        assert!(summary.decision_accounting_is_consistent());
        assert!(!summary.decision_shape_is_clean());
        assert!(!summary.can_use_attention_decision());
    }

    #[test]
    fn attention_decision_summary_blocks_selection_cap_hit_flag_drift() {
        let summary = AttentionDecisionSummary {
            threshold: 0.5,
            max_selected: 2,
            candidate_count: 2,
            selected_count: 1,
            rejected_count: 1,
            selection_fraction: 0.5,
            hit_selection_cap: true,
            selected_layer_counts: RouteLayerCounts {
                fast_projection: 0,
                local_window: 1,
                global: 0,
                fusion: 0,
            },
            rejected_layer_counts: RouteLayerCounts {
                fast_projection: 1,
                local_window: 0,
                global: 0,
                fusion: 0,
            },
        };

        assert!(summary.candidate_accounting_balanced());
        assert!(summary.threshold_shape_is_valid());
        assert!(summary.selection_fraction_matches_counts());
        assert!(summary.selected_count_within_cap());
        assert!(!summary.has_rejected_attention());
        assert!(!summary.selection_cap_hit_shape_is_valid());
        assert_eq!(summary.selection_cap_hit_shape_problem_component_count(), 1);
        assert_eq!(summary.decision_count_problem_component_count(), 0);
        assert_eq!(summary.decision_shape_problem_component_count(), 1);
        assert_eq!(summary.decision_problem_component_count(), 1);
        assert!(summary.has_decision_problem_components());
        assert!(summary.decision_accounting_is_consistent());
        assert!(!summary.decision_shape_is_clean());
        assert!(!summary.can_use_attention_decision());
    }

    #[test]
    fn adaptive_observe_changes_only_target_profile() {
        let mut policy = ThresholdAttentionPolicy::default();
        let coding_before = policy.threshold_for(TaskProfile::Coding);
        let writing_before = policy.threshold_for(TaskProfile::Writing);
        let before = policy.policy_summary();

        assert!(before.thresholds_are_bounded());
        assert!(before.profile_thresholds_match_base());
        assert_eq!(before.adapted_profile_count(), 0);
        assert_eq!(before.threshold_for(TaskProfile::Coding), coding_before);
        assert_eq!(before.threshold_spread(), 0.0);
        assert!(before.thresholds_are_finite());
        assert!(before.learning_rate_is_valid());
        assert_eq!(before.threshold_policy_signal_component_count(), 2);
        assert!(!before.has_threshold_policy_problem_components());
        assert!(before.threshold_policy_accounting_is_consistent());
        assert!(before.threshold_policy_shape_is_clean());
        assert!(before.can_use_threshold_policy());
        assert_eq!(
            before.threshold_policy_action(),
            ThresholdAttentionPolicyAction::UseThresholdPolicy
        );
        assert!(before.threshold_policy_action().can_use());
        assert!(!before.threshold_policy_action().should_repair());

        policy.observe(RoutingFeedback {
            profile: TaskProfile::Coding,
            quality: 0.30,
            perplexity: 32.0,
            contradiction_count: 2,
        });

        let after = policy.policy_summary();

        assert!(policy.threshold_for(TaskProfile::Coding) < coding_before);
        assert_eq!(policy.threshold_for(TaskProfile::Writing), writing_before);
        assert!(after.thresholds_are_bounded());
        assert!(!after.profile_thresholds_match_base());
        assert_eq!(after.adapted_profile_count(), 1);
        assert_eq!(
            after.threshold_for(TaskProfile::Coding),
            policy.threshold_for(TaskProfile::Coding)
        );
        assert!(after.threshold_spread() > 0.0);
        assert_eq!(after.threshold_policy_signal_component_count(), 3);
        assert_eq!(after.threshold_policy_problem_component_count(), 0);
        assert!(after.threshold_policy_accounting_is_consistent());
        assert!(after.threshold_policy_shape_is_clean());
        assert!(after.can_use_threshold_policy());
        assert_eq!(
            after.threshold_policy_action(),
            ThresholdAttentionPolicyAction::UseThresholdPolicy
        );
        assert!(after.threshold_policy_action().can_use());
        assert!(!after.threshold_policy_action().should_repair());
    }

    #[test]
    fn threshold_policy_preview_reports_quality_repair_adjustment() {
        let mut policy = ThresholdAttentionPolicy::default();
        let coding_before = policy.threshold_for(TaskProfile::Coding);
        let feedback = RoutingFeedback {
            profile: TaskProfile::Coding,
            quality: 0.30,
            perplexity: 32.0,
            contradiction_count: 2,
        };

        let preview = policy.preview_adjustment(feedback);

        assert_eq!(preview.profile, TaskProfile::Coding);
        assert_eq!(preview.feedback, feedback.feedback_summary());
        assert_eq!(preview.previous_threshold, coding_before);
        assert!((preview.adjusted_threshold - 0.489).abs() < 0.0001);
        assert!((preview.threshold_delta + 0.061).abs() < 0.0001);
        assert_eq!(
            preview.action,
            ThresholdAttentionAdjustmentAction::LowerThresholdForQualityRepair
        );
        assert!(preview.action.can_commit());
        assert!(preview.action.changes_threshold());
        assert!(!preview.action.should_repair());
        assert!(preview.can_commit);
        assert!(!preview.requires_repair_first);
        assert!(preview.threshold_changed());
        assert!(preview.threshold_lowered());
        assert!(!preview.threshold_raised());
        assert!(preview.expected_to_restore_quality());
        assert!(!preview.expected_to_reduce_attention_compute());
        assert!(preview.threshold_bounds_are_valid());
        assert!(preview.threshold_delta_matches_thresholds());
        assert!(preview.action_matches_delta());
        assert_eq!(preview.threshold_adjustment_signal_component_count(), 4);
        assert_eq!(preview.threshold_adjustment_blocker_component_count(), 0);
        assert!(preview.has_threshold_adjustment_signal_components());
        assert!(!preview.has_threshold_adjustment_blockers());
        assert!(preview.threshold_adjustment_accounting_is_consistent());
        assert!(preview.threshold_adjustment_is_clean());
        assert!(preview.can_commit_threshold_adjustment());
        assert_eq!(
            preview.reason_codes,
            vec!["attention_threshold_lowered_for_quality_repair"]
        );
        assert_eq!(policy.threshold_for(TaskProfile::Coding), coding_before);

        let committed = policy.observe_with_report(feedback);

        assert_eq!(committed, preview);
        assert_eq!(
            policy.threshold_for(TaskProfile::Coding),
            preview.adjusted_threshold
        );
    }

    #[test]
    fn threshold_policy_preview_reports_compute_saving_adjustment() {
        let policy = ThresholdAttentionPolicy::default();
        let writing_before = policy.threshold_for(TaskProfile::Writing);
        let feedback = RoutingFeedback {
            profile: TaskProfile::Writing,
            quality: 0.95,
            perplexity: 4.0,
            contradiction_count: 0,
        };

        let report = policy.preview_adjustment(feedback);

        assert_eq!(
            report.action,
            ThresholdAttentionAdjustmentAction::RaiseThresholdForComputeSavings
        );
        assert!(report.action.can_commit());
        assert!(report.action.changes_threshold());
        assert!(report.can_commit);
        assert!(!report.requires_repair_first);
        assert_eq!(report.previous_threshold, writing_before);
        assert!(report.adjusted_threshold > writing_before);
        assert!((report.adjusted_threshold - 0.5577).abs() < 0.0001);
        assert!((report.threshold_delta - 0.0077).abs() < 0.0001);
        assert!(report.threshold_changed());
        assert!(!report.threshold_lowered());
        assert!(report.threshold_raised());
        assert!(!report.expected_to_restore_quality());
        assert!(report.expected_to_reduce_attention_compute());
        assert!(report.threshold_bounds_are_valid());
        assert!(report.threshold_delta_matches_thresholds());
        assert!(report.action_matches_delta());
        assert_eq!(report.threshold_adjustment_signal_component_count(), 4);
        assert_eq!(report.threshold_adjustment_blocker_component_count(), 0);
        assert!(report.threshold_adjustment_is_clean());
        assert!(report.can_commit_threshold_adjustment());
        assert_eq!(
            report.reason_codes,
            vec!["attention_threshold_raised_for_compute_savings"]
        );
    }

    #[test]
    fn threshold_policy_rejects_invalid_adjustment_feedback_without_mutating() {
        let mut policy = ThresholdAttentionPolicy::default();
        let general_before = policy.threshold_for(TaskProfile::General);
        let feedback = RoutingFeedback {
            profile: TaskProfile::General,
            quality: f32::NAN,
            perplexity: -1.0,
            contradiction_count: 0,
        };

        let report = policy.observe_with_report(feedback);

        assert_eq!(
            report.action,
            ThresholdAttentionAdjustmentAction::RepairThresholdAdjustment
        );
        assert!(!report.action.can_commit());
        assert!(!report.action.changes_threshold());
        assert!(report.action.should_repair());
        assert!(!report.can_commit);
        assert!(report.requires_repair_first);
        assert_eq!(report.previous_threshold, general_before);
        assert_eq!(report.adjusted_threshold, general_before);
        assert_eq!(report.threshold_delta, 0.0);
        assert!(!report.threshold_changed());
        assert!(!report.expected_to_restore_quality());
        assert!(!report.expected_to_reduce_attention_compute());
        assert!(report.threshold_bounds_are_valid());
        assert!(report.threshold_delta_matches_thresholds());
        assert!(report.action_matches_delta());
        assert_eq!(report.threshold_adjustment_signal_component_count(), 1);
        assert_eq!(report.threshold_adjustment_blocker_component_count(), 1);
        assert!(report.has_threshold_adjustment_signal_components());
        assert!(report.has_threshold_adjustment_blockers());
        assert!(report.threshold_adjustment_accounting_is_consistent());
        assert!(!report.threshold_adjustment_is_clean());
        assert!(!report.can_commit_threshold_adjustment());
        assert_eq!(
            report.reason_codes,
            vec!["attention_threshold_feedback_requires_repair"]
        );
        assert_eq!(policy.threshold_for(TaskProfile::General), general_before);
    }

    #[test]
    fn adaptive_threshold_profile_state_requires_enabled_switch_for_selection() {
        let mut policy = ThresholdAttentionPolicy::default();
        let base = policy.policy_summary().base_threshold;
        policy.observe(RoutingFeedback {
            profile: TaskProfile::Coding,
            quality: 0.30,
            perplexity: 32.0,
            contradiction_count: 2,
        });
        let adapted = policy.threshold_for(TaskProfile::Coding);
        let candidates = [AttentionCandidate::new(
            "borderline-coding",
            0,
            (base + adapted) / 2.0,
            0.55,
            RouteLayer::LocalWindow,
        )];
        let context = RoutingContext {
            profile: TaskProfile::Coding,
            ..RoutingContext::default()
        };
        let disabled_switches =
            ExperimentSwitches::default().with_adaptive_attention_thresholds(false);
        let enabled_switches =
            ExperimentSwitches::default().with_adaptive_attention_thresholds(true);

        let disabled = policy.select(&candidates, context, disabled_switches);
        let enabled = policy.select(&candidates, context, enabled_switches);
        let disabled_readiness = AttentionSelectionReadinessSummary::from_decision(
            AttentionCandidate::batch_summary(&candidates),
            &disabled,
        );
        let enabled_readiness = AttentionSelectionReadinessSummary::from_decision(
            AttentionCandidate::batch_summary(&candidates),
            &enabled,
        );

        assert!(adapted < base);
        assert_eq!(disabled.threshold, base);
        assert_eq!(enabled.threshold, adapted);
        assert!(disabled.selected.is_empty());
        assert_eq!(disabled.rejected.len(), 1);
        assert_eq!(enabled.selected_tokens(), vec!["borderline-coding"]);
        assert_eq!(enabled.rejected.len(), 0);
        assert!(disabled_readiness.can_commit_attention_selection_readiness());
        assert!(enabled_readiness.can_commit_attention_selection_readiness());
        assert!(disabled_readiness.selection_boundary_matches());
        assert!(enabled_readiness.selection_boundary_matches());
    }

    #[test]
    fn threshold_policy_constructor_keeps_summary_inside_policy_bounds() {
        let low = ThresholdAttentionPolicy::new(-1.0).policy_summary();
        let high = ThresholdAttentionPolicy::new(2.0).policy_summary();

        assert!(low.thresholds_are_bounded());
        assert_eq!(low.base_threshold, low.min_threshold);
        assert!(low.profile_thresholds_match_base());
        assert_eq!(low.adapted_profile_count(), 0);
        assert_eq!(
            low.threshold_policy_action(),
            ThresholdAttentionPolicyAction::UseThresholdPolicy
        );
        assert!(low.threshold_policy_action().can_use());

        assert!(high.thresholds_are_bounded());
        assert_eq!(high.base_threshold, high.max_threshold);
        assert!(high.profile_thresholds_match_base());
        assert_eq!(high.adapted_profile_count(), 0);
        assert_eq!(
            high.threshold_policy_action(),
            ThresholdAttentionPolicyAction::UseThresholdPolicy
        );
        assert!(high.threshold_policy_action().can_use());
    }

    #[test]
    fn threshold_policy_summary_exposes_admission_boundary() {
        let clean = ThresholdAttentionPolicy::default().policy_summary();

        assert_eq!(clean.threshold_policy_admission_signal_component_count(), 2);
        assert!(clean.has_threshold_policy_admission_signals());
        assert_eq!(
            clean.threshold_policy_admission_blocker_component_count(),
            0
        );
        assert!(!clean.has_threshold_policy_admission_blockers());
        assert!(clean.threshold_policy_admission_accounting_is_consistent());
        assert!(clean.threshold_policy_admission_is_clean());
        assert!(clean.can_admit_threshold_policy());

        let mut policy = ThresholdAttentionPolicy::default();
        policy.observe(RoutingFeedback {
            profile: TaskProfile::Coding,
            quality: 0.30,
            perplexity: 32.0,
            contradiction_count: 2,
        });
        let adapted = policy.policy_summary();

        assert_eq!(adapted.adapted_profile_count(), 1);
        assert_eq!(
            adapted.threshold_policy_admission_signal_component_count(),
            3
        );
        assert!(adapted.has_threshold_policy_admission_signals());
        assert_eq!(
            adapted.threshold_policy_admission_blocker_component_count(),
            0
        );
        assert!(!adapted.has_threshold_policy_admission_blockers());
        assert!(adapted.threshold_policy_admission_accounting_is_consistent());
        assert!(adapted.threshold_policy_admission_is_clean());
        assert!(adapted.can_admit_threshold_policy());

        let repair = ThresholdAttentionPolicySummary {
            base_threshold: f32::NAN,
            min_threshold: 0.9,
            max_threshold: 0.1,
            learning_rate: -1.0,
            general_threshold: 2.0,
            coding_threshold: 0.5,
            writing_threshold: f32::INFINITY,
            long_document_threshold: 0.2,
        };

        assert_eq!(
            repair.threshold_policy_admission_signal_component_count(),
            1
        );
        assert!(repair.has_threshold_policy_admission_signals());
        assert_eq!(
            repair.threshold_policy_admission_blocker_component_count(),
            3
        );
        assert!(repair.has_threshold_policy_admission_blockers());
        assert!(repair.threshold_policy_admission_accounting_is_consistent());
        assert!(!repair.threshold_policy_admission_is_clean());
        assert!(!repair.can_admit_threshold_policy());
    }

    #[test]
    fn threshold_policy_summary_counts_public_shape_drift() {
        let summary = ThresholdAttentionPolicySummary {
            base_threshold: f32::NAN,
            min_threshold: 0.9,
            max_threshold: 0.1,
            learning_rate: -1.0,
            general_threshold: 2.0,
            coding_threshold: 0.5,
            writing_threshold: f32::INFINITY,
            long_document_threshold: 0.2,
        };

        assert!(!summary.thresholds_are_finite());
        assert!(!summary.learning_rate_is_valid());
        assert!(!summary.thresholds_are_bounded());
        assert!(!summary.profile_thresholds_match_base());
        assert_eq!(summary.adapted_profile_count(), 4);
        assert_eq!(summary.threshold_policy_signal_component_count(), 1);
        assert_eq!(summary.threshold_policy_problem_component_count(), 3);
        assert!(summary.has_threshold_policy_signal_components());
        assert!(summary.has_threshold_policy_problem_components());
        assert!(summary.threshold_policy_accounting_is_consistent());
        assert!(!summary.threshold_policy_shape_is_clean());
        assert!(!summary.can_use_threshold_policy());
        assert_eq!(
            summary.threshold_policy_action(),
            ThresholdAttentionPolicyAction::RepairThresholdPolicy
        );
        assert!(!summary.threshold_policy_action().can_use());
        assert!(summary.threshold_policy_action().should_repair());
    }
}
