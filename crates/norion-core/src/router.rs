use crate::profile::{HierarchyWeights, TaskProfile};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteLayer {
    FastProjection,
    LocalWindow,
    Global,
    Fusion,
}

impl RouteLayer {
    pub fn uses_attention(self) -> bool {
        self != Self::FastProjection
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::FastProjection => "fast_projection",
            Self::LocalWindow => "local_window",
            Self::Global => "global",
            Self::Fusion => "fusion",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TokenFeatures {
    pub text: String,
    pub entropy: f32,
    pub position: usize,
}

impl TokenFeatures {
    pub fn new(text: impl Into<String>, entropy: f32, position: usize) -> Self {
        Self {
            text: text.into(),
            entropy: entropy.clamp(0.0, 1.0),
            position,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoutingContext {
    pub profile: TaskProfile,
    pub context_tokens: usize,
    pub cache_hit_rate: f32,
    pub hardware_pressure: f32,
    pub compute_headroom: f32,
    pub hierarchy: HierarchyWeights,
}

impl Default for RoutingContext {
    fn default() -> Self {
        Self {
            profile: TaskProfile::General,
            context_tokens: 0,
            cache_hit_rate: 0.0,
            hardware_pressure: 0.0,
            compute_headroom: 0.5,
            hierarchy: HierarchyWeights::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RoutingDecision {
    pub token: String,
    pub score: f32,
    pub layer: RouteLayer,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RouteLayerCounts {
    pub fast_projection: usize,
    pub local_window: usize,
    pub global: usize,
    pub fusion: usize,
}

impl RouteLayerCounts {
    pub fn from_decisions(decisions: &[RoutingDecision]) -> Self {
        let mut counts = Self::default();
        for decision in decisions {
            counts.bump(decision.layer);
        }
        counts
    }

    pub fn bump(&mut self, layer: RouteLayer) {
        match layer {
            RouteLayer::FastProjection => {
                self.fast_projection = self.fast_projection.saturating_add(1);
            }
            RouteLayer::LocalWindow => {
                self.local_window = self.local_window.saturating_add(1);
            }
            RouteLayer::Global => {
                self.global = self.global.saturating_add(1);
            }
            RouteLayer::Fusion => {
                self.fusion = self.fusion.saturating_add(1);
            }
        }
    }

    pub fn total(self) -> usize {
        self.fast_projection
            .saturating_add(self.local_window)
            .saturating_add(self.global)
            .saturating_add(self.fusion)
    }

    pub fn attention_total(self) -> usize {
        self.local_window
            .saturating_add(self.global)
            .saturating_add(self.fusion)
    }

    pub fn is_empty(self) -> bool {
        self.total() == 0
    }

    pub fn has_fast_projection(self) -> bool {
        self.fast_projection > 0
    }

    pub fn has_attention_layers(self) -> bool {
        self.attention_total() > 0
    }

    pub fn has_fusion(self) -> bool {
        self.fusion > 0
    }

    pub fn all_attention(self) -> bool {
        !self.is_empty() && self.fast_projection == 0
    }

    pub fn uses_multiple_layers(self) -> bool {
        [
            self.fast_projection,
            self.local_window,
            self.global,
            self.fusion,
        ]
        .into_iter()
        .filter(|count| *count > 0)
        .count()
            > 1
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoutingDecisionSummary {
    pub threshold: f32,
    pub token_count: usize,
    pub layer_counts: RouteLayerCounts,
    pub attention_fraction: f32,
    pub average_score: f32,
    pub min_score: f32,
    pub max_score: f32,
    pub above_threshold_tokens: usize,
    pub below_threshold_tokens: usize,
}

impl RoutingDecisionSummary {
    pub fn from_decisions(threshold: f32, decisions: &[RoutingDecision]) -> Self {
        let layer_counts = RouteLayerCounts::from_decisions(decisions);
        let token_count = decisions.len();
        let attention_fraction = layer_counts.attention_total() as f32 / token_count.max(1) as f32;
        let above_threshold_tokens = decisions
            .iter()
            .filter(|decision| decision.score >= threshold)
            .count();
        let below_threshold_tokens = token_count.saturating_sub(above_threshold_tokens);

        let (score_sum, min_score, max_score) = decisions.iter().fold(
            (0.0_f32, f32::INFINITY, f32::NEG_INFINITY),
            |(sum, min_score, max_score), decision| {
                (
                    sum + decision.score,
                    min_score.min(decision.score),
                    max_score.max(decision.score),
                )
            },
        );

        Self {
            threshold,
            token_count,
            layer_counts,
            attention_fraction,
            average_score: if token_count == 0 {
                0.0
            } else {
                score_sum / token_count as f32
            },
            min_score: if token_count == 0 { 0.0 } else { min_score },
            max_score: if token_count == 0 { 0.0 } else { max_score },
            above_threshold_tokens,
            below_threshold_tokens,
        }
    }

    pub fn route_budget(self) -> RouteBudget {
        RouteBudget {
            threshold: self.threshold,
            attention_tokens: self.layer_counts.attention_total(),
            fast_tokens: self.layer_counts.fast_projection,
            attention_fraction: self.attention_fraction,
        }
    }

    pub fn has_threshold_crossings(self) -> bool {
        self.above_threshold_tokens > 0
    }

    pub fn is_empty(self) -> bool {
        self.token_count == 0
    }

    pub fn layer_counts_match_tokens(self) -> bool {
        self.layer_counts.total() == self.token_count
    }

    pub fn has_fast_path(self) -> bool {
        self.layer_counts.has_fast_projection()
    }

    pub fn has_attention_route(self) -> bool {
        self.layer_counts.has_attention_layers()
    }

    pub fn all_attention_route(self) -> bool {
        self.layer_counts.all_attention()
    }

    pub fn uses_multiple_layers(self) -> bool {
        self.layer_counts.uses_multiple_layers()
    }

    pub fn has_score_spread(self) -> bool {
        !self.is_empty() && !float_close(self.min_score, self.max_score)
    }

    pub fn route_budget_matches(self, budget: RouteBudget) -> bool {
        float_close(self.threshold, budget.threshold)
            && self.layer_counts.attention_total() == budget.attention_tokens
            && self.layer_counts.fast_projection == budget.fast_tokens
            && float_close(self.attention_fraction, budget.attention_fraction)
    }

    pub fn threshold_partition_matches_tokens(self) -> bool {
        self.above_threshold_tokens
            .saturating_add(self.below_threshold_tokens)
            == self.token_count
    }

    pub fn attention_fraction_matches_layers(self) -> bool {
        let total = self.layer_counts.total();
        if total == 0 {
            return float_close(self.attention_fraction, 0.0);
        }

        let expected = self.layer_counts.attention_total() as f32 / total as f32;
        float_close(self.attention_fraction, expected)
    }

    pub fn score_range_is_valid(self) -> bool {
        if self.is_empty() {
            return float_close(self.average_score, 0.0)
                && float_close(self.min_score, 0.0)
                && float_close(self.max_score, 0.0);
        }

        finite_unit(self.average_score)
            && finite_unit(self.min_score)
            && finite_unit(self.max_score)
            && self.min_score <= self.average_score
            && self.average_score <= self.max_score
    }

    pub fn threshold_is_valid(self) -> bool {
        finite_unit(self.threshold)
    }

    pub fn route_activity_signal_component_count(self) -> usize {
        usize::from(!self.is_empty())
            + usize::from(self.has_threshold_crossings())
            + usize::from(self.below_threshold_tokens > 0)
    }

    pub fn route_layer_signal_component_count(self) -> usize {
        usize::from(self.has_fast_path())
            + usize::from(self.has_attention_route())
            + usize::from(self.layer_counts.has_fusion())
            + usize::from(self.uses_multiple_layers())
            + usize::from(self.all_attention_route())
    }

    pub fn route_score_signal_component_count(self) -> usize {
        usize::from(self.has_score_spread())
            + usize::from(self.attention_fraction > 0.0)
            + usize::from(self.average_score > 0.0)
    }

    pub fn routing_signal_component_count(self) -> usize {
        self.route_activity_signal_component_count()
            .saturating_add(self.route_layer_signal_component_count())
            .saturating_add(self.route_score_signal_component_count())
    }

    pub fn has_routing_signal_components(self) -> bool {
        self.routing_signal_component_count() > 0
    }

    pub fn route_count_problem_component_count(self) -> usize {
        usize::from(!self.layer_counts_match_tokens())
            + usize::from(!self.threshold_partition_matches_tokens())
    }

    pub fn route_score_problem_component_count(self) -> usize {
        usize::from(!self.threshold_is_valid())
            + usize::from(!finite_unit(self.attention_fraction))
            + usize::from(!self.attention_fraction_matches_layers())
            + usize::from(!self.score_range_is_valid())
    }

    pub fn routing_problem_component_count(self) -> usize {
        self.route_count_problem_component_count()
            .saturating_add(self.route_score_problem_component_count())
    }

    pub fn has_routing_problem_components(self) -> bool {
        self.routing_problem_component_count() > 0
    }

    pub fn routing_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .route_activity_signal_component_count()
            .saturating_add(self.route_layer_signal_component_count())
            .saturating_add(self.route_score_signal_component_count());
        let expected_problem_count = self
            .route_count_problem_component_count()
            .saturating_add(self.route_score_problem_component_count());

        self.routing_signal_component_count() == expected_signal_count
            && self.routing_problem_component_count() == expected_problem_count
    }

    pub fn routing_shape_is_clean(self) -> bool {
        !self.has_routing_problem_components() && self.routing_accounting_is_consistent()
    }

    pub fn can_use_route_summary(self) -> bool {
        !self.is_empty() && self.routing_shape_is_clean()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RouteBudget {
    pub threshold: f32,
    pub attention_tokens: usize,
    pub fast_tokens: usize,
    pub attention_fraction: f32,
}

impl RouteBudget {
    pub fn from_decisions(threshold: f32, decisions: &[RoutingDecision]) -> Self {
        RoutingDecisionSummary::from_decisions(threshold, decisions).route_budget()
    }

    pub fn total_tokens(self) -> usize {
        self.attention_tokens.saturating_add(self.fast_tokens)
    }

    pub fn is_empty(self) -> bool {
        self.total_tokens() == 0
    }

    pub fn has_attention_pressure(self) -> bool {
        self.attention_tokens > 0
    }

    pub fn attention_dominates(self) -> bool {
        self.attention_tokens > self.fast_tokens
    }

    pub fn token_counts_match_fraction(self) -> bool {
        let total_tokens = self.total_tokens();
        if total_tokens == 0 {
            return float_close(self.attention_fraction, 0.0);
        }

        let expected = self.attention_tokens as f32 / total_tokens as f32;
        float_close(self.attention_fraction, expected)
    }

    pub fn route_budget_signal_component_count(self) -> usize {
        usize::from(!self.is_empty())
            + usize::from(self.has_attention_pressure())
            + usize::from(self.fast_tokens > 0)
            + usize::from(self.attention_dominates())
    }

    pub fn has_route_budget_signal_components(self) -> bool {
        self.route_budget_signal_component_count() > 0
    }

    pub fn route_budget_problem_component_count(self) -> usize {
        usize::from(!finite_unit(self.threshold))
            + usize::from(!finite_unit(self.attention_fraction))
            + usize::from(!self.token_counts_match_fraction())
    }

    pub fn has_route_budget_problem_components(self) -> bool {
        self.route_budget_problem_component_count() > 0
    }

    pub fn route_budget_accounting_is_consistent(self) -> bool {
        let expected_signal_count = usize::from(!self.is_empty())
            .saturating_add(usize::from(self.has_attention_pressure()))
            .saturating_add(usize::from(self.fast_tokens > 0))
            .saturating_add(usize::from(self.attention_dominates()));
        let expected_problem_count = usize::from(!finite_unit(self.threshold))
            .saturating_add(usize::from(!finite_unit(self.attention_fraction)))
            .saturating_add(usize::from(!self.token_counts_match_fraction()));

        self.route_budget_signal_component_count() == expected_signal_count
            && self.route_budget_problem_component_count() == expected_problem_count
    }

    pub fn route_budget_shape_is_clean(self) -> bool {
        !self.has_route_budget_problem_components() && self.route_budget_accounting_is_consistent()
    }

    pub fn can_use_route_budget(self) -> bool {
        !self.is_empty() && self.route_budget_shape_is_clean()
    }
}

impl Default for RouteBudget {
    fn default() -> Self {
        Self {
            threshold: 0.52,
            attention_tokens: 0,
            fast_tokens: 0,
            attention_fraction: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteBudgetReadinessStage {
    DecisionSummary,
    RouteBudget,
    BudgetParity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteBudgetReadinessCommitAction {
    CommitRouteBudget,
    WaitForRouteBudget,
    RepairRouteBudget,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RouteBudgetReadinessSummary {
    pub decision_summary: RoutingDecisionSummary,
    pub route_budget: RouteBudget,
    pub decision_signal_component_count: usize,
    pub budget_signal_component_count: usize,
    pub parity_signal_component_count: usize,
    pub decision_blocker_component_count: usize,
    pub budget_blocker_component_count: usize,
    pub parity_blocker_component_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RouteBudgetReadinessCommitSummary {
    pub readiness: RouteBudgetReadinessSummary,
    pub action: RouteBudgetReadinessCommitAction,
    pub committed_route_budget: Option<RouteBudget>,
    pub can_commit: bool,
    pub should_wait_for_route_budget: bool,
    pub should_repair_route_budget: bool,
    pub first_unready_stage: Option<RouteBudgetReadinessStage>,
    pub first_blocking_stage: Option<RouteBudgetReadinessStage>,
    pub total_signal_component_count: usize,
    pub total_blocker_component_count: usize,
    pub component_accounting_consistent: bool,
}

impl RouteBudgetReadinessCommitAction {
    pub fn can_commit(self) -> bool {
        matches!(self, Self::CommitRouteBudget)
    }

    pub fn should_wait_for_route_budget(self) -> bool {
        matches!(self, Self::WaitForRouteBudget)
    }

    pub fn should_repair_route_budget(self) -> bool {
        matches!(self, Self::RepairRouteBudget)
    }
}

impl RouteBudgetReadinessSummary {
    pub fn new(decision_summary: RoutingDecisionSummary, route_budget: RouteBudget) -> Self {
        Self {
            decision_summary,
            route_budget,
            decision_signal_component_count: decision_summary.routing_signal_component_count(),
            budget_signal_component_count: route_budget.route_budget_signal_component_count(),
            parity_signal_component_count: usize::from(
                !decision_summary.is_empty()
                    && !route_budget.is_empty()
                    && decision_summary.route_budget_matches(route_budget),
            ),
            decision_blocker_component_count: decision_summary.routing_problem_component_count(),
            budget_blocker_component_count: route_budget.route_budget_problem_component_count(),
            parity_blocker_component_count: usize::from(
                !decision_summary.route_budget_matches(route_budget),
            ),
        }
    }

    pub fn from_decisions(
        threshold: f32,
        decisions: &[RoutingDecision],
        route_budget: RouteBudget,
    ) -> Self {
        Self::new(
            RoutingDecisionSummary::from_decisions(threshold, decisions),
            route_budget,
        )
    }

    pub fn stage_order() -> [RouteBudgetReadinessStage; 3] {
        [
            RouteBudgetReadinessStage::DecisionSummary,
            RouteBudgetReadinessStage::RouteBudget,
            RouteBudgetReadinessStage::BudgetParity,
        ]
    }

    pub fn decision_summary_ready(self) -> bool {
        self.decision_summary.can_use_route_summary()
    }

    pub fn route_budget_ready(self) -> bool {
        self.route_budget.can_use_route_budget()
    }

    pub fn budget_parity_ready(self) -> bool {
        self.decision_summary
            .route_budget_matches(self.route_budget)
    }

    pub fn stage_ready(self, stage: RouteBudgetReadinessStage) -> bool {
        match stage {
            RouteBudgetReadinessStage::DecisionSummary => self.decision_summary_ready(),
            RouteBudgetReadinessStage::RouteBudget => self.route_budget_ready(),
            RouteBudgetReadinessStage::BudgetParity => self.budget_parity_ready(),
        }
    }

    pub fn stage_signal_component_count(self, stage: RouteBudgetReadinessStage) -> usize {
        match stage {
            RouteBudgetReadinessStage::DecisionSummary => self.decision_signal_component_count,
            RouteBudgetReadinessStage::RouteBudget => self.budget_signal_component_count,
            RouteBudgetReadinessStage::BudgetParity => self.parity_signal_component_count,
        }
    }

    pub fn stage_blocker_component_count(self, stage: RouteBudgetReadinessStage) -> usize {
        match stage {
            RouteBudgetReadinessStage::DecisionSummary => self.decision_blocker_component_count,
            RouteBudgetReadinessStage::RouteBudget => self.budget_blocker_component_count,
            RouteBudgetReadinessStage::BudgetParity => self.parity_blocker_component_count,
        }
    }

    pub fn first_unready_stage(self) -> Option<RouteBudgetReadinessStage> {
        Self::stage_order()
            .into_iter()
            .find(|stage| !self.stage_ready(*stage))
    }

    pub fn first_blocking_stage(self) -> Option<RouteBudgetReadinessStage> {
        Self::stage_order()
            .into_iter()
            .find(|stage| self.stage_blocker_component_count(*stage) > 0)
    }

    pub fn route_budget_readiness_signal_component_count(self) -> usize {
        self.decision_signal_component_count
            .saturating_add(self.budget_signal_component_count)
            .saturating_add(self.parity_signal_component_count)
    }

    pub fn has_route_budget_readiness_signals(self) -> bool {
        self.route_budget_readiness_signal_component_count() > 0
    }

    pub fn route_budget_readiness_blocker_component_count(self) -> usize {
        self.decision_blocker_component_count
            .saturating_add(self.budget_blocker_component_count)
            .saturating_add(self.parity_blocker_component_count)
    }

    pub fn has_route_budget_readiness_blockers(self) -> bool {
        self.route_budget_readiness_blocker_component_count() > 0
    }

    pub fn route_budget_readiness_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .decision_signal_component_count
            .saturating_add(self.budget_signal_component_count)
            .saturating_add(self.parity_signal_component_count);
        let expected_blocker_count = self
            .decision_blocker_component_count
            .saturating_add(self.budget_blocker_component_count)
            .saturating_add(self.parity_blocker_component_count);

        self.decision_summary.routing_accounting_is_consistent()
            && self.route_budget.route_budget_accounting_is_consistent()
            && self.decision_signal_component_count
                == self.decision_summary.routing_signal_component_count()
            && self.budget_signal_component_count
                == self.route_budget.route_budget_signal_component_count()
            && self.parity_signal_component_count
                == usize::from(
                    !self.decision_summary.is_empty()
                        && !self.route_budget.is_empty()
                        && self
                            .decision_summary
                            .route_budget_matches(self.route_budget),
                )
            && self.decision_blocker_component_count
                == self.decision_summary.routing_problem_component_count()
            && self.budget_blocker_component_count
                == self.route_budget.route_budget_problem_component_count()
            && self.parity_blocker_component_count
                == usize::from(
                    !self
                        .decision_summary
                        .route_budget_matches(self.route_budget),
                )
            && self.route_budget_readiness_signal_component_count() == expected_signal_count
            && self.has_route_budget_readiness_signals() == (expected_signal_count > 0)
            && self.route_budget_readiness_blocker_component_count() == expected_blocker_count
            && self.has_route_budget_readiness_blockers() == (expected_blocker_count > 0)
    }

    pub fn route_budget_readiness_is_clean(self) -> bool {
        !self.has_route_budget_readiness_blockers()
            && self.route_budget_readiness_accounting_is_consistent()
    }

    pub fn can_commit_route_budget_readiness(self) -> bool {
        self.route_budget_readiness_is_clean()
            && self.decision_summary_ready()
            && self.route_budget_ready()
            && self.budget_parity_ready()
    }

    pub fn route_budget_commit_action(self) -> RouteBudgetReadinessCommitAction {
        if self.can_commit_route_budget_readiness() {
            RouteBudgetReadinessCommitAction::CommitRouteBudget
        } else if self.route_budget_readiness_accounting_is_consistent()
            && !self.has_route_budget_readiness_blockers()
        {
            RouteBudgetReadinessCommitAction::WaitForRouteBudget
        } else {
            RouteBudgetReadinessCommitAction::RepairRouteBudget
        }
    }

    pub fn commit_summary(self) -> RouteBudgetReadinessCommitSummary {
        RouteBudgetReadinessCommitSummary::new(self)
    }
}

impl RouteBudgetReadinessCommitSummary {
    pub fn new(readiness: RouteBudgetReadinessSummary) -> Self {
        let component_accounting_consistent =
            readiness.route_budget_readiness_accounting_is_consistent();
        let action = readiness.route_budget_commit_action();
        let committed_route_budget = action.can_commit().then_some(readiness.route_budget);

        Self {
            readiness,
            action,
            committed_route_budget,
            can_commit: action.can_commit(),
            should_wait_for_route_budget: action.should_wait_for_route_budget(),
            should_repair_route_budget: action.should_repair_route_budget(),
            first_unready_stage: readiness.first_unready_stage(),
            first_blocking_stage: readiness.first_blocking_stage(),
            total_signal_component_count: readiness.route_budget_readiness_signal_component_count(),
            total_blocker_component_count: readiness
                .route_budget_readiness_blocker_component_count(),
            component_accounting_consistent,
        }
    }

    pub fn action_can_commit(self) -> bool {
        self.action.can_commit()
    }

    pub fn action_should_wait_for_route_budget(self) -> bool {
        self.action.should_wait_for_route_budget()
    }

    pub fn action_should_repair_route_budget(self) -> bool {
        self.action.should_repair_route_budget()
    }

    pub fn can_commit_route_budget(self) -> bool {
        self.can_commit
    }

    pub fn should_wait_for_route_budget(self) -> bool {
        self.should_wait_for_route_budget
    }

    pub fn should_repair_route_budget(self) -> bool {
        self.should_repair_route_budget
    }

    pub fn can_use_committed_route_budget(self) -> bool {
        self.can_commit && self.committed_route_budget.is_some()
    }

    pub fn route_budget_admission_signal_component_count(self) -> usize {
        self.total_signal_component_count
    }

    pub fn has_route_budget_admission_signals(self) -> bool {
        self.route_budget_admission_signal_component_count() > 0
    }

    pub fn missing_committed_route_budget_component_count(self) -> usize {
        usize::from(self.committed_route_budget.is_none())
    }

    pub fn route_budget_admission_blocker_component_count(self) -> usize {
        self.total_blocker_component_count
            .saturating_add(self.missing_committed_route_budget_component_count())
    }

    pub fn has_route_budget_admission_blockers(self) -> bool {
        self.route_budget_admission_blocker_component_count() > 0
    }

    pub fn route_budget_admission_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self.total_signal_component_count;
        let expected_blocker_count = self
            .total_blocker_component_count
            .saturating_add(usize::from(self.committed_route_budget.is_none()));

        self.commit_decision_accounting_is_consistent()
            && self.route_budget_admission_signal_component_count() == expected_signal_count
            && self.has_route_budget_admission_signals() == (expected_signal_count > 0)
            && self.missing_committed_route_budget_component_count()
                == usize::from(self.committed_route_budget.is_none())
            && self.route_budget_admission_blocker_component_count() == expected_blocker_count
            && self.has_route_budget_admission_blockers() == (expected_blocker_count > 0)
    }

    pub fn route_budget_admission_is_clean(self) -> bool {
        !self.has_route_budget_admission_blockers()
            && self.route_budget_admission_accounting_is_consistent()
    }

    pub fn can_admit_committed_route_budget(self) -> bool {
        self.can_use_committed_route_budget() && self.route_budget_admission_is_clean()
    }

    pub fn commit_decision_accounting_is_consistent(self) -> bool {
        let expected_action = self.readiness.route_budget_commit_action();
        let expected_committed_route_budget = expected_action
            .can_commit()
            .then_some(self.readiness.route_budget);

        self.action == expected_action
            && self.committed_route_budget == expected_committed_route_budget
            && self.can_commit == self.action.can_commit()
            && self.should_wait_for_route_budget == self.action.should_wait_for_route_budget()
            && self.should_repair_route_budget == self.action.should_repair_route_budget()
            && self.first_unready_stage == self.readiness.first_unready_stage()
            && self.first_blocking_stage == self.readiness.first_blocking_stage()
            && self.total_signal_component_count
                == self
                    .readiness
                    .route_budget_readiness_signal_component_count()
            && self.total_blocker_component_count
                == self
                    .readiness
                    .route_budget_readiness_blocker_component_count()
            && self.component_accounting_consistent
                == self
                    .readiness
                    .route_budget_readiness_accounting_is_consistent()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoutingFeedback {
    pub profile: TaskProfile,
    pub quality: f32,
    pub perplexity: f32,
    pub contradiction_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoutingFeedbackSummary {
    pub profile: TaskProfile,
    pub quality: f32,
    pub perplexity: f32,
    pub contradiction_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoutingFeedbackBatchSummary {
    pub feedback_count: usize,
    pub profile_counts: ProfileObservations,
    pub average_quality: f32,
    pub average_perplexity: f32,
    pub contradiction_total: usize,
    pub low_quality_count: usize,
    pub high_quality_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RouterState {
    pub threshold: f32,
    pub observations: u64,
    pub profile_thresholds: ProfileThresholds,
    pub profile_observations: ProfileObservations,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProfileThresholds {
    pub general: f32,
    pub coding: f32,
    pub writing: f32,
    pub long_document: f32,
}

impl ProfileThresholds {
    pub fn from_single(threshold: f32) -> Self {
        Self {
            general: threshold,
            coding: threshold,
            writing: threshold,
            long_document: threshold,
        }
    }

    pub fn get(self, profile: TaskProfile) -> f32 {
        match profile {
            TaskProfile::General => self.general,
            TaskProfile::Coding => self.coding,
            TaskProfile::Writing => self.writing,
            TaskProfile::LongDocument => self.long_document,
        }
    }

    pub fn set(&mut self, profile: TaskProfile, threshold: f32) {
        match profile {
            TaskProfile::General => self.general = threshold,
            TaskProfile::Coding => self.coding = threshold,
            TaskProfile::Writing => self.writing = threshold,
            TaskProfile::LongDocument => self.long_document = threshold,
        }
    }

    pub fn clamp(self, min_threshold: f32, max_threshold: f32) -> Self {
        Self {
            general: self.general.clamp(min_threshold, max_threshold),
            coding: self.coding.clamp(min_threshold, max_threshold),
            writing: self.writing.clamp(min_threshold, max_threshold),
            long_document: self.long_document.clamp(min_threshold, max_threshold),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProfileObservations {
    pub general: u64,
    pub coding: u64,
    pub writing: u64,
    pub long_document: u64,
}

impl ProfileObservations {
    pub fn from_single(observations: u64) -> Self {
        Self {
            general: observations,
            coding: 0,
            writing: 0,
            long_document: 0,
        }
    }

    pub fn get(self, profile: TaskProfile) -> u64 {
        match profile {
            TaskProfile::General => self.general,
            TaskProfile::Coding => self.coding,
            TaskProfile::Writing => self.writing,
            TaskProfile::LongDocument => self.long_document,
        }
    }

    pub fn bump(&mut self, profile: TaskProfile) {
        match profile {
            TaskProfile::General => self.general = self.general.saturating_add(1),
            TaskProfile::Coding => self.coding = self.coding.saturating_add(1),
            TaskProfile::Writing => self.writing = self.writing.saturating_add(1),
            TaskProfile::LongDocument => {
                self.long_document = self.long_document.saturating_add(1);
            }
        }
    }

    pub fn total(self) -> u64 {
        self.general
            .saturating_add(self.coding)
            .saturating_add(self.writing)
            .saturating_add(self.long_document)
    }

    pub fn active_profile_count(self) -> usize {
        [self.general, self.coding, self.writing, self.long_document]
            .into_iter()
            .filter(|observations| *observations > 0)
            .count()
    }
}

impl RouterState {
    pub fn profile_observation_total(self) -> u64 {
        self.profile_observations.total()
    }

    pub fn observation_count_drift(self) -> i128 {
        self.observations as i128 - self.profile_observation_total() as i128
    }

    pub fn has_observation_drift(self) -> bool {
        self.observation_count_drift() != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GenerationMetrics {
    pub perplexity: f32,
    pub semantic_consistency: f32,
    pub contradiction_count: usize,
    pub token_count: usize,
}

impl GenerationMetrics {
    pub fn quality_score(self) -> f32 {
        let perplexity_score = (1.0 / (1.0 + self.perplexity / 12.0)).clamp(0.0, 1.0);
        let consistency_score = self.semantic_consistency.clamp(0.0, 1.0);
        let contradiction_penalty = (self.contradiction_count as f32 * 0.18).min(0.72);

        ((perplexity_score * 0.35) + (consistency_score * 0.65) - contradiction_penalty)
            .clamp(0.0, 1.0)
    }

    pub fn routing_feedback(self, profile: TaskProfile) -> RoutingFeedback {
        RoutingFeedback::from_generation_metrics(profile, self)
    }
}

impl RoutingFeedback {
    pub fn from_generation_metrics(profile: TaskProfile, metrics: GenerationMetrics) -> Self {
        Self {
            profile,
            quality: metrics.quality_score(),
            perplexity: metrics.perplexity,
            contradiction_count: metrics.contradiction_count,
        }
    }

    pub fn feedback_summary(self) -> RoutingFeedbackSummary {
        RoutingFeedbackSummary {
            profile: self.profile,
            quality: self.quality,
            perplexity: self.perplexity,
            contradiction_count: self.contradiction_count,
        }
    }

    pub fn batch_summary(feedback: &[RoutingFeedback]) -> RoutingFeedbackBatchSummary {
        RoutingFeedbackBatchSummary::from_feedback(feedback)
    }
}

impl RoutingFeedbackSummary {
    pub fn is_low_quality(self) -> bool {
        self.quality < 0.58
    }

    pub fn is_high_quality(self) -> bool {
        self.quality > 0.82 && self.perplexity <= 9.0 && !self.has_contradictions()
    }

    pub fn has_contradictions(self) -> bool {
        self.contradiction_count > 0
    }

    pub fn quality_shape_is_valid(self) -> bool {
        finite_unit(self.quality)
    }

    pub fn perplexity_shape_is_valid(self) -> bool {
        finite_nonnegative(self.perplexity)
    }

    pub fn feedback_signal_component_count(self) -> usize {
        usize::from(self.is_low_quality())
            + usize::from(self.is_high_quality())
            + usize::from(self.has_contradictions())
            + usize::from(self.perplexity > 0.0)
    }

    pub fn has_feedback_signal_components(self) -> bool {
        self.feedback_signal_component_count() > 0
    }

    pub fn feedback_problem_component_count(self) -> usize {
        usize::from(!self.quality_shape_is_valid()) + usize::from(!self.perplexity_shape_is_valid())
    }

    pub fn has_feedback_problem_components(self) -> bool {
        self.feedback_problem_component_count() > 0
    }

    pub fn feedback_accounting_is_consistent(self) -> bool {
        let expected_signal_count = usize::from(self.is_low_quality())
            .saturating_add(usize::from(self.is_high_quality()))
            .saturating_add(usize::from(self.has_contradictions()))
            .saturating_add(usize::from(self.perplexity > 0.0));
        let expected_problem_count = usize::from(!self.quality_shape_is_valid())
            .saturating_add(usize::from(!self.perplexity_shape_is_valid()));

        self.feedback_signal_component_count() == expected_signal_count
            && self.feedback_problem_component_count() == expected_problem_count
    }

    pub fn feedback_shape_is_clean(self) -> bool {
        !self.has_feedback_problem_components() && self.feedback_accounting_is_consistent()
    }

    pub fn can_use_routing_feedback(self) -> bool {
        self.feedback_shape_is_clean()
    }
}

impl RoutingFeedbackBatchSummary {
    pub fn from_feedback(feedback: &[RoutingFeedback]) -> Self {
        let mut profile_counts = ProfileObservations::default();
        let mut quality_total = 0.0;
        let mut perplexity_total = 0.0;
        let mut contradiction_total = 0usize;
        let mut low_quality_count = 0usize;
        let mut high_quality_count = 0usize;

        for item in feedback {
            let summary = item.feedback_summary();
            profile_counts.bump(summary.profile);
            quality_total += summary.quality;
            perplexity_total += summary.perplexity;
            contradiction_total = contradiction_total.saturating_add(summary.contradiction_count);
            if summary.is_low_quality() {
                low_quality_count += 1;
            }
            if summary.is_high_quality() {
                high_quality_count += 1;
            }
        }

        let feedback_count = feedback.len();
        Self {
            feedback_count,
            profile_counts,
            average_quality: if feedback_count == 0 {
                0.0
            } else {
                quality_total / feedback_count as f32
            },
            average_perplexity: if feedback_count == 0 {
                0.0
            } else {
                perplexity_total / feedback_count as f32
            },
            contradiction_total,
            low_quality_count,
            high_quality_count,
        }
    }

    pub fn is_empty(self) -> bool {
        self.feedback_count == 0
    }

    pub fn has_mixed_profiles(self) -> bool {
        self.profile_counts.active_profile_count() > 1
    }

    pub fn has_quality_pressure(self) -> bool {
        self.low_quality_count > 0 || self.contradiction_total > 0
    }

    pub fn profile_count_matches_feedback(self) -> bool {
        self.profile_counts.total() == self.feedback_count as u64
    }

    pub fn quality_bucket_counts_are_bounded(self) -> bool {
        self.low_quality_count <= self.feedback_count
            && self.high_quality_count <= self.feedback_count
    }

    pub fn average_quality_shape_is_valid(self) -> bool {
        finite_unit(self.average_quality)
    }

    pub fn average_perplexity_shape_is_valid(self) -> bool {
        finite_nonnegative(self.average_perplexity)
    }

    pub fn feedback_batch_signal_component_count(self) -> usize {
        usize::from(!self.is_empty())
            + usize::from(self.has_mixed_profiles())
            + usize::from(self.has_quality_pressure())
            + usize::from(self.high_quality_count > 0)
    }

    pub fn has_feedback_batch_signal_components(self) -> bool {
        self.feedback_batch_signal_component_count() > 0
    }

    pub fn feedback_batch_problem_component_count(self) -> usize {
        usize::from(!self.profile_count_matches_feedback())
            + usize::from(!self.quality_bucket_counts_are_bounded())
            + usize::from(!self.average_quality_shape_is_valid())
            + usize::from(!self.average_perplexity_shape_is_valid())
    }

    pub fn has_feedback_batch_problem_components(self) -> bool {
        self.feedback_batch_problem_component_count() > 0
    }

    pub fn feedback_batch_accounting_is_consistent(self) -> bool {
        let expected_signal_count = usize::from(!self.is_empty())
            .saturating_add(usize::from(self.has_mixed_profiles()))
            .saturating_add(usize::from(self.has_quality_pressure()))
            .saturating_add(usize::from(self.high_quality_count > 0));
        let expected_problem_count = usize::from(!self.profile_count_matches_feedback())
            .saturating_add(usize::from(!self.quality_bucket_counts_are_bounded()))
            .saturating_add(usize::from(!self.average_quality_shape_is_valid()))
            .saturating_add(usize::from(!self.average_perplexity_shape_is_valid()));

        self.feedback_batch_signal_component_count() == expected_signal_count
            && self.feedback_batch_problem_component_count() == expected_problem_count
    }

    pub fn feedback_batch_shape_is_clean(self) -> bool {
        !self.has_feedback_batch_problem_components()
            && self.feedback_batch_accounting_is_consistent()
    }

    pub fn can_use_routing_feedback_batch(self) -> bool {
        !self.is_empty() && self.feedback_batch_shape_is_clean()
    }
}

pub trait HierarchicalRouter {
    fn route(&self, token: &TokenFeatures, context: RoutingContext) -> RoutingDecision;

    fn route_many(
        &self,
        tokens: &[TokenFeatures],
        context: RoutingContext,
    ) -> Vec<RoutingDecision> {
        tokens
            .iter()
            .map(|token| self.route(token, context))
            .collect()
    }

    fn budget(&self, tokens: &[TokenFeatures], context: RoutingContext) -> RouteBudget;

    fn observe(&mut self, feedback: RoutingFeedback);
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DefaultHierarchicalRouter {
    threshold: f32,
    thresholds: ProfileThresholds,
    min_threshold: f32,
    max_threshold: f32,
    learning_rate: f32,
    observations: u64,
    profile_observations: ProfileObservations,
}

impl Default for DefaultHierarchicalRouter {
    fn default() -> Self {
        let threshold = 0.52;
        Self {
            threshold,
            thresholds: ProfileThresholds::from_single(threshold),
            min_threshold: 0.18,
            max_threshold: 0.88,
            learning_rate: 0.08,
            observations: 0,
            profile_observations: ProfileObservations::default(),
        }
    }
}

impl DefaultHierarchicalRouter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn threshold(&self) -> f32 {
        self.threshold.clamp(self.min_threshold, self.max_threshold)
    }

    pub fn threshold_for(&self, profile: TaskProfile) -> f32 {
        self.thresholds
            .get(profile)
            .clamp(self.min_threshold, self.max_threshold)
    }

    pub fn state(&self) -> RouterState {
        RouterState {
            threshold: self.threshold(),
            observations: self.observations,
            profile_thresholds: self
                .thresholds
                .clamp(self.min_threshold, self.max_threshold),
            profile_observations: self.profile_observations,
        }
    }

    pub fn restore_state(&mut self, state: RouterState) {
        self.threshold = state
            .threshold
            .clamp(self.min_threshold, self.max_threshold);
        self.thresholds = state
            .profile_thresholds
            .clamp(self.min_threshold, self.max_threshold);
        self.observations = state.observations;
        self.profile_observations = state.profile_observations;
    }
}

impl HierarchicalRouter for DefaultHierarchicalRouter {
    fn route(&self, token: &TokenFeatures, context: RoutingContext) -> RoutingDecision {
        let score = routing_score(token.entropy, context);
        let threshold = self.threshold_for(context.profile);
        let layer = choose_layer(score, threshold, context);

        RoutingDecision {
            token: token.text.clone(),
            score,
            layer,
        }
    }

    fn budget(&self, tokens: &[TokenFeatures], context: RoutingContext) -> RouteBudget {
        let decisions = self.route_many(tokens, context);
        RouteBudget::from_decisions(self.threshold_for(context.profile), &decisions)
    }

    fn observe(&mut self, feedback: RoutingFeedback) {
        let mut threshold = self.threshold_for(feedback.profile);
        let contradiction_pressure = (feedback.contradiction_count as f32 * 0.025).min(0.12);

        if feedback.quality < 0.58 {
            threshold -= self.learning_rate * (0.58 - feedback.quality) + contradiction_pressure;
        } else if feedback.quality > 0.82
            && feedback.perplexity <= 9.0
            && feedback.contradiction_count == 0
        {
            threshold += self.learning_rate * (feedback.quality - 0.82);
        }

        self.thresholds.set(
            feedback.profile,
            threshold.clamp(self.min_threshold, self.max_threshold),
        );
        self.threshold = self.threshold_for(feedback.profile);
        self.observations = self.observations.saturating_add(1);
        self.profile_observations.bump(feedback.profile);
    }
}

fn routing_score(entropy: f32, context: RoutingContext) -> f32 {
    let profile_pressure = match context.profile {
        TaskProfile::General => 0.0,
        TaskProfile::Coding => 0.05,
        TaskProfile::Writing => 0.08,
        TaskProfile::LongDocument => 0.10,
    };
    let context_pressure = (context.context_tokens as f32 / 32_000.0).min(0.18);
    let cache_discount = context.cache_hit_rate.clamp(0.0, 1.0) * 0.10;
    let hardware_discount = context.hardware_pressure.clamp(0.0, 1.0) * 0.16;
    let compute_bonus = (context.compute_headroom.clamp(0.0, 1.0) - 0.5).max(0.0) * 0.12;
    let hierarchy_bias = context.hierarchy.global * 0.05
        + context.hierarchy.local * 0.03
        + context.hierarchy.fusion * 0.04;

    (entropy * 0.72 + profile_pressure + context_pressure + compute_bonus + hierarchy_bias
        - cache_discount
        - hardware_discount)
        .clamp(0.0, 1.0)
}

fn choose_layer(score: f32, threshold: f32, context: RoutingContext) -> RouteLayer {
    if score < threshold {
        return RouteLayer::FastProjection;
    }

    match context.profile {
        TaskProfile::LongDocument if context.context_tokens >= 8_192 => RouteLayer::Fusion,
        TaskProfile::LongDocument if score < threshold + 0.22 => RouteLayer::Fusion,
        TaskProfile::Coding if score < threshold + 0.24 => RouteLayer::LocalWindow,
        TaskProfile::Writing => RouteLayer::Global,
        _ if score >= threshold + 0.24 => RouteLayer::Global,
        _ => RouteLayer::LocalWindow,
    }
}

fn float_close(left: f32, right: f32) -> bool {
    (left - right).abs() <= 0.0001
}

fn finite_unit(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn finite_nonnegative(value: f32) -> bool {
    value.is_finite() && value >= 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routing_selects_profile_specific_layers() {
        let router = DefaultHierarchicalRouter::new();
        let token = TokenFeatures::new("borrow_checker", 0.82, 0);
        let coding = router.route(
            &token,
            RoutingContext {
                profile: TaskProfile::Coding,
                ..RoutingContext::default()
            },
        );
        let long = router.route(
            &token,
            RoutingContext {
                profile: TaskProfile::LongDocument,
                context_tokens: 16_384,
                ..RoutingContext::default()
            },
        );

        assert_eq!(coding.layer, RouteLayer::LocalWindow);
        assert_eq!(long.layer, RouteLayer::Fusion);
    }

    #[test]
    fn route_budget_counts_attention_layers() {
        let router = DefaultHierarchicalRouter::new();
        let tokens = [
            TokenFeatures::new("easy", 0.05, 0),
            TokenFeatures::new("complex", 0.90, 1),
        ];

        let budget = router.budget(&tokens, RoutingContext::default());

        assert_eq!(budget.fast_tokens, 1);
        assert_eq!(budget.attention_tokens, 1);
        assert!((budget.attention_fraction - 0.5).abs() < 0.0001);
    }

    #[test]
    fn route_budget_threshold_includes_equal_score_as_attention_pressure() {
        let decisions = [
            RoutingDecision {
                token: "equal".to_string(),
                score: 0.52,
                layer: RouteLayer::LocalWindow,
            },
            RoutingDecision {
                token: "below".to_string(),
                score: 0.519,
                layer: RouteLayer::FastProjection,
            },
        ];

        let summary = RoutingDecisionSummary::from_decisions(0.52, &decisions);
        let budget = summary.route_budget();
        let readiness = RouteBudgetReadinessSummary::new(summary, budget);

        assert_eq!(summary.above_threshold_tokens, 1);
        assert_eq!(summary.below_threshold_tokens, 1);
        assert!(summary.threshold_partition_matches_tokens());
        assert!(summary.has_threshold_crossings());
        assert_eq!(budget.attention_tokens, 1);
        assert_eq!(budget.fast_tokens, 1);
        assert!((budget.attention_fraction - 0.5).abs() < 0.0001);
        assert!(budget.has_attention_pressure());
        assert!(summary.route_budget_matches(budget));
        assert!(readiness.can_commit_route_budget_readiness());
        assert_eq!(
            readiness.commit_summary().committed_route_budget,
            Some(budget)
        );
    }

    #[test]
    fn hierarchy_bias_can_promote_borderline_tokens_into_route_budget() {
        let router = DefaultHierarchicalRouter::new();
        let token = TokenFeatures::new("borderline", 0.66, 0);
        let local_context = RoutingContext {
            hierarchy: HierarchyWeights::new(0.0, 1.0, 0.0),
            ..RoutingContext::default()
        };
        let global_context = RoutingContext {
            hierarchy: HierarchyWeights::new(1.0, 0.0, 0.0),
            ..RoutingContext::default()
        };

        let local = router.route(&token, local_context);
        let global = router.route(&token, global_context);

        assert!(local.score < router.threshold_for(TaskProfile::General));
        assert_eq!(local.layer, RouteLayer::FastProjection);
        assert!(global.score >= router.threshold_for(TaskProfile::General));
        assert_eq!(global.layer, RouteLayer::LocalWindow);

        let budget = router.budget(&[token], global_context);
        let summary = RoutingDecisionSummary::from_decisions(
            router.threshold_for(TaskProfile::General),
            &[global],
        );
        let readiness = RouteBudgetReadinessSummary::new(summary, budget);

        assert_eq!(budget.fast_tokens, 0);
        assert_eq!(budget.attention_tokens, 1);
        assert_eq!(budget.attention_fraction, 1.0);
        assert!(budget.attention_dominates());
        assert!(budget.route_budget_shape_is_clean());
        assert!(readiness.budget_parity_ready());
        assert!(readiness.can_commit_route_budget_readiness());
        assert!(readiness.commit_summary().can_use_committed_route_budget());
    }

    #[test]
    fn hardware_pressure_discount_demotes_borderline_route_budget_and_blocks_stale_attention_budget()
     {
        let router = DefaultHierarchicalRouter::new();
        let token = TokenFeatures::new("hardware-borderline", 0.80, 0);
        let low_pressure_context = RoutingContext::default();
        let high_pressure_context = RoutingContext {
            hardware_pressure: 1.0,
            ..RoutingContext::default()
        };

        let low_pressure_decision = router.route(&token, low_pressure_context);
        let high_pressure_decision = router.route(&token, high_pressure_context);
        let low_pressure_budget = router.budget(std::slice::from_ref(&token), low_pressure_context);
        let high_pressure_budget =
            router.budget(std::slice::from_ref(&token), high_pressure_context);
        let high_pressure_summary = RoutingDecisionSummary::from_decisions(
            router.threshold_for(TaskProfile::General),
            &[high_pressure_decision.clone()],
        );
        let stale_readiness =
            RouteBudgetReadinessSummary::new(high_pressure_summary, low_pressure_budget);
        let commit = stale_readiness.commit_summary();

        assert!(low_pressure_decision.score >= router.threshold_for(TaskProfile::General));
        assert_eq!(low_pressure_decision.layer, RouteLayer::LocalWindow);
        assert!(high_pressure_decision.score < router.threshold_for(TaskProfile::General));
        assert_eq!(high_pressure_decision.layer, RouteLayer::FastProjection);
        assert!(low_pressure_budget.has_attention_pressure());
        assert_eq!(low_pressure_budget.attention_tokens, 1);
        assert_eq!(low_pressure_budget.fast_tokens, 0);
        assert!(!high_pressure_budget.has_attention_pressure());
        assert_eq!(high_pressure_budget.attention_tokens, 0);
        assert_eq!(high_pressure_budget.fast_tokens, 1);
        assert!(high_pressure_summary.route_budget_matches(high_pressure_budget));
        assert!(!high_pressure_summary.route_budget_matches(low_pressure_budget));
        assert!(stale_readiness.decision_summary_ready());
        assert!(stale_readiness.route_budget_ready());
        assert!(!stale_readiness.budget_parity_ready());
        assert_eq!(
            stale_readiness.first_unready_stage(),
            Some(RouteBudgetReadinessStage::BudgetParity)
        );
        assert_eq!(
            stale_readiness.first_blocking_stage(),
            Some(RouteBudgetReadinessStage::BudgetParity)
        );
        assert!(stale_readiness.has_route_budget_readiness_blockers());
        assert!(stale_readiness.route_budget_readiness_accounting_is_consistent());
        assert!(!stale_readiness.can_commit_route_budget_readiness());
        assert_eq!(
            commit.action,
            RouteBudgetReadinessCommitAction::RepairRouteBudget
        );
        assert_eq!(commit.committed_route_budget, None);
        assert!(commit.should_repair_route_budget());
        assert!(!commit.can_use_committed_route_budget());
        assert!(commit.commit_decision_accounting_is_consistent());
    }

    #[test]
    fn routing_decision_summary_counts_layers_thresholds_and_scores() {
        let decisions = [
            RoutingDecision {
                token: "fast".to_string(),
                score: 0.18,
                layer: RouteLayer::FastProjection,
            },
            RoutingDecision {
                token: "local".to_string(),
                score: 0.62,
                layer: RouteLayer::LocalWindow,
            },
            RoutingDecision {
                token: "global".to_string(),
                score: 0.86,
                layer: RouteLayer::Global,
            },
            RoutingDecision {
                token: "fusion".to_string(),
                score: 0.74,
                layer: RouteLayer::Fusion,
            },
        ];

        let summary = RoutingDecisionSummary::from_decisions(0.60, &decisions);

        assert_eq!(RouteLayer::Global.as_str(), "global");
        assert_eq!(summary.token_count, 4);
        assert_eq!(summary.layer_counts.fast_projection, 1);
        assert_eq!(summary.layer_counts.local_window, 1);
        assert_eq!(summary.layer_counts.global, 1);
        assert_eq!(summary.layer_counts.fusion, 1);
        assert_eq!(summary.layer_counts.total(), 4);
        assert_eq!(summary.layer_counts.attention_total(), 3);
        assert!(!summary.layer_counts.is_empty());
        assert!(summary.layer_counts.has_fast_projection());
        assert!(summary.layer_counts.has_attention_layers());
        assert!(summary.layer_counts.has_fusion());
        assert!(!summary.layer_counts.all_attention());
        assert!(summary.layer_counts.uses_multiple_layers());
        assert_eq!(summary.above_threshold_tokens, 3);
        assert_eq!(summary.below_threshold_tokens, 1);
        assert!(summary.has_threshold_crossings());
        assert!(!summary.is_empty());
        assert!(summary.layer_counts_match_tokens());
        assert!(summary.has_fast_path());
        assert!(summary.has_attention_route());
        assert!(!summary.all_attention_route());
        assert!(summary.uses_multiple_layers());
        assert!(summary.has_score_spread());
        assert!((summary.attention_fraction - 0.75).abs() < 0.0001);
        assert!((summary.average_score - 0.60).abs() < 0.0001);
        assert!((summary.min_score - 0.18).abs() < 0.0001);
        assert!((summary.max_score - 0.86).abs() < 0.0001);
        assert!(summary.threshold_partition_matches_tokens());
        assert!(summary.attention_fraction_matches_layers());
        assert!(summary.score_range_is_valid());
        assert!(summary.threshold_is_valid());
        assert_eq!(summary.route_activity_signal_component_count(), 3);
        assert_eq!(summary.route_layer_signal_component_count(), 4);
        assert_eq!(summary.route_score_signal_component_count(), 3);
        assert_eq!(summary.routing_signal_component_count(), 10);
        assert!(summary.has_routing_signal_components());
        assert_eq!(summary.route_count_problem_component_count(), 0);
        assert_eq!(summary.route_score_problem_component_count(), 0);
        assert_eq!(summary.routing_problem_component_count(), 0);
        assert!(!summary.has_routing_problem_components());
        assert!(summary.routing_accounting_is_consistent());
        assert!(summary.routing_shape_is_clean());
        assert!(summary.can_use_route_summary());

        let budget = summary.route_budget();
        assert_eq!(budget.attention_tokens, 3);
        assert_eq!(budget.fast_tokens, 1);
        assert_eq!(budget.total_tokens(), 4);
        assert!(!budget.is_empty());
        assert!(budget.has_attention_pressure());
        assert!(budget.attention_dominates());
        assert!(budget.token_counts_match_fraction());
        assert_eq!(budget.route_budget_signal_component_count(), 4);
        assert!(budget.has_route_budget_signal_components());
        assert_eq!(budget.route_budget_problem_component_count(), 0);
        assert!(!budget.has_route_budget_problem_components());
        assert!(budget.route_budget_accounting_is_consistent());
        assert!(budget.route_budget_shape_is_clean());
        assert!(budget.can_use_route_budget());
        assert!(summary.route_budget_matches(budget));
        assert!((budget.attention_fraction - summary.attention_fraction).abs() < 0.0001);

        let readiness = RouteBudgetReadinessSummary::new(summary, budget);
        assert_eq!(
            RouteBudgetReadinessSummary::stage_order(),
            [
                RouteBudgetReadinessStage::DecisionSummary,
                RouteBudgetReadinessStage::RouteBudget,
                RouteBudgetReadinessStage::BudgetParity,
            ]
        );
        assert!(readiness.decision_summary_ready());
        assert!(readiness.route_budget_ready());
        assert!(readiness.budget_parity_ready());
        assert_eq!(readiness.first_unready_stage(), None);
        assert_eq!(readiness.first_blocking_stage(), None);
        assert_eq!(readiness.decision_signal_component_count, 10);
        assert_eq!(readiness.budget_signal_component_count, 4);
        assert_eq!(readiness.parity_signal_component_count, 1);
        assert_eq!(
            readiness.stage_signal_component_count(RouteBudgetReadinessStage::DecisionSummary),
            readiness.decision_signal_component_count
        );
        assert_eq!(
            readiness.stage_blocker_component_count(RouteBudgetReadinessStage::BudgetParity),
            readiness.parity_blocker_component_count
        );
        assert_eq!(
            readiness.route_budget_readiness_signal_component_count(),
            15
        );
        assert!(readiness.has_route_budget_readiness_signals());
        assert_eq!(
            readiness.route_budget_readiness_blocker_component_count(),
            0
        );
        assert!(!readiness.has_route_budget_readiness_blockers());
        assert!(readiness.route_budget_readiness_accounting_is_consistent());
        assert!(readiness.route_budget_readiness_is_clean());
        assert!(readiness.can_commit_route_budget_readiness());
        let commit = readiness.commit_summary();
        assert_eq!(
            commit.action,
            RouteBudgetReadinessCommitAction::CommitRouteBudget
        );
        assert!(commit.action_can_commit());
        assert!(!commit.action_should_wait_for_route_budget());
        assert!(!commit.action_should_repair_route_budget());
        assert_eq!(commit.committed_route_budget, Some(budget));
        assert!(commit.can_commit_route_budget());
        assert!(!commit.should_wait_for_route_budget());
        assert!(!commit.should_repair_route_budget());
        assert!(commit.can_use_committed_route_budget());
        assert_eq!(commit.first_unready_stage, None);
        assert_eq!(commit.first_blocking_stage, None);
        assert_eq!(commit.total_signal_component_count, 15);
        assert_eq!(commit.total_blocker_component_count, 0);
        assert!(commit.component_accounting_consistent);
        assert!(commit.commit_decision_accounting_is_consistent());
    }

    #[test]
    fn routing_decision_summary_handles_empty_decision_sets() {
        let summary = RoutingDecisionSummary::from_decisions(0.52, &[]);

        assert_eq!(summary.token_count, 0);
        assert!(summary.layer_counts.is_empty());
        assert!(!summary.layer_counts.has_fast_projection());
        assert!(!summary.layer_counts.has_attention_layers());
        assert!(!summary.layer_counts.has_fusion());
        assert!(!summary.layer_counts.all_attention());
        assert!(!summary.layer_counts.uses_multiple_layers());
        assert_eq!(summary.layer_counts.attention_total(), 0);
        assert_eq!(summary.above_threshold_tokens, 0);
        assert_eq!(summary.below_threshold_tokens, 0);
        assert_eq!(summary.attention_fraction, 0.0);
        assert_eq!(summary.average_score, 0.0);
        assert_eq!(summary.min_score, 0.0);
        assert_eq!(summary.max_score, 0.0);
        assert!(!summary.has_threshold_crossings());
        assert!(summary.is_empty());
        assert!(summary.layer_counts_match_tokens());
        assert!(!summary.has_fast_path());
        assert!(!summary.has_attention_route());
        assert!(!summary.all_attention_route());
        assert!(!summary.uses_multiple_layers());
        assert!(!summary.has_score_spread());
        assert_eq!(summary.routing_signal_component_count(), 0);
        assert!(!summary.has_routing_signal_components());
        assert_eq!(summary.routing_problem_component_count(), 0);
        assert!(!summary.has_routing_problem_components());
        assert!(summary.routing_accounting_is_consistent());
        assert!(summary.routing_shape_is_clean());
        assert!(!summary.can_use_route_summary());
        assert_eq!(summary.route_budget(), RouteBudget::default());
        assert!(summary.route_budget_matches(RouteBudget::default()));
        assert!(RouteBudget::default().is_empty());
        assert!(!RouteBudget::default().has_attention_pressure());
        assert!(!RouteBudget::default().attention_dominates());
        assert!(RouteBudget::default().token_counts_match_fraction());
        assert_eq!(
            RouteBudget::default().route_budget_signal_component_count(),
            0
        );
        assert_eq!(
            RouteBudget::default().route_budget_problem_component_count(),
            0
        );
        assert!(RouteBudget::default().route_budget_accounting_is_consistent());
        assert!(RouteBudget::default().route_budget_shape_is_clean());
        assert!(!RouteBudget::default().can_use_route_budget());

        let readiness = RouteBudgetReadinessSummary::new(summary, RouteBudget::default());

        assert!(!readiness.decision_summary_ready());
        assert!(!readiness.route_budget_ready());
        assert!(readiness.budget_parity_ready());
        assert_eq!(
            readiness.first_unready_stage(),
            Some(RouteBudgetReadinessStage::DecisionSummary)
        );
        assert_eq!(readiness.first_blocking_stage(), None);
        assert_eq!(readiness.decision_signal_component_count, 0);
        assert_eq!(readiness.budget_signal_component_count, 0);
        assert_eq!(readiness.parity_signal_component_count, 0);
        assert_eq!(
            readiness.route_budget_readiness_blocker_component_count(),
            0
        );
        assert!(readiness.route_budget_readiness_accounting_is_consistent());
        assert!(readiness.route_budget_readiness_is_clean());
        assert!(!readiness.can_commit_route_budget_readiness());
        let commit = readiness.commit_summary();
        assert_eq!(
            commit.action,
            RouteBudgetReadinessCommitAction::WaitForRouteBudget
        );
        assert!(!commit.action_can_commit());
        assert!(commit.action_should_wait_for_route_budget());
        assert!(!commit.action_should_repair_route_budget());
        assert_eq!(commit.committed_route_budget, None);
        assert!(!commit.can_commit_route_budget());
        assert!(commit.should_wait_for_route_budget());
        assert!(!commit.should_repair_route_budget());
        assert!(!commit.can_use_committed_route_budget());
        assert_eq!(
            commit.first_unready_stage,
            Some(RouteBudgetReadinessStage::DecisionSummary)
        );
        assert_eq!(commit.first_blocking_stage, None);
        assert_eq!(commit.total_signal_component_count, 0);
        assert_eq!(commit.total_blocker_component_count, 0);
        assert!(commit.component_accounting_consistent);
        assert!(commit.commit_decision_accounting_is_consistent());
    }

    #[test]
    fn route_budget_readiness_blocks_budget_parity_drift() {
        let decisions = [
            RoutingDecision {
                token: "fast".to_string(),
                score: 0.18,
                layer: RouteLayer::FastProjection,
            },
            RoutingDecision {
                token: "local".to_string(),
                score: 0.62,
                layer: RouteLayer::LocalWindow,
            },
            RoutingDecision {
                token: "global".to_string(),
                score: 0.86,
                layer: RouteLayer::Global,
            },
            RoutingDecision {
                token: "fusion".to_string(),
                score: 0.74,
                layer: RouteLayer::Fusion,
            },
        ];
        let route_budget = RouteBudget {
            threshold: 0.60,
            attention_tokens: 2,
            fast_tokens: 3,
            attention_fraction: 0.40,
        };
        let readiness = RouteBudgetReadinessSummary::from_decisions(0.60, &decisions, route_budget);

        assert!(readiness.decision_summary_ready());
        assert!(readiness.route_budget_ready());
        assert!(!readiness.budget_parity_ready());
        assert_eq!(
            readiness.first_unready_stage(),
            Some(RouteBudgetReadinessStage::BudgetParity)
        );
        assert_eq!(
            readiness.first_blocking_stage(),
            Some(RouteBudgetReadinessStage::BudgetParity)
        );
        assert_eq!(readiness.decision_signal_component_count, 10);
        assert_eq!(readiness.budget_signal_component_count, 3);
        assert_eq!(readiness.parity_signal_component_count, 0);
        assert_eq!(readiness.decision_blocker_component_count, 0);
        assert_eq!(readiness.budget_blocker_component_count, 0);
        assert_eq!(readiness.parity_blocker_component_count, 1);
        assert_eq!(
            readiness.route_budget_readiness_signal_component_count(),
            13
        );
        assert_eq!(
            readiness.route_budget_readiness_blocker_component_count(),
            1
        );
        assert!(readiness.has_route_budget_readiness_blockers());
        assert!(readiness.route_budget_readiness_accounting_is_consistent());
        assert!(!readiness.route_budget_readiness_is_clean());
        assert!(!readiness.can_commit_route_budget_readiness());
        let commit = readiness.commit_summary();
        assert_eq!(
            commit.action,
            RouteBudgetReadinessCommitAction::RepairRouteBudget
        );
        assert!(!commit.action_can_commit());
        assert!(!commit.action_should_wait_for_route_budget());
        assert!(commit.action_should_repair_route_budget());
        assert_eq!(commit.committed_route_budget, None);
        assert!(!commit.can_commit_route_budget());
        assert!(!commit.should_wait_for_route_budget());
        assert!(commit.should_repair_route_budget());
        assert!(!commit.can_use_committed_route_budget());
        assert_eq!(
            commit.first_unready_stage,
            Some(RouteBudgetReadinessStage::BudgetParity)
        );
        assert_eq!(
            commit.first_blocking_stage,
            Some(RouteBudgetReadinessStage::BudgetParity)
        );
        assert_eq!(commit.total_signal_component_count, 13);
        assert_eq!(commit.total_blocker_component_count, 1);
        assert!(commit.component_accounting_consistent);
        assert!(commit.commit_decision_accounting_is_consistent());
    }

    #[test]
    fn route_budget_readiness_exposes_commit_action_boundary() {
        let decisions = [
            RoutingDecision {
                token: "fast".to_string(),
                score: 0.18,
                layer: RouteLayer::FastProjection,
            },
            RoutingDecision {
                token: "local".to_string(),
                score: 0.62,
                layer: RouteLayer::LocalWindow,
            },
        ];
        let ready_summary = RoutingDecisionSummary::from_decisions(0.60, &decisions);
        let ready_budget = ready_summary.route_budget();
        let ready = RouteBudgetReadinessSummary::new(ready_summary, ready_budget);
        assert_eq!(
            ready.route_budget_commit_action(),
            RouteBudgetReadinessCommitAction::CommitRouteBudget
        );
        assert_eq!(
            ready.commit_summary().action,
            ready.route_budget_commit_action()
        );

        let waiting_summary = RoutingDecisionSummary::from_decisions(0.60, &[]);
        let waiting =
            RouteBudgetReadinessSummary::new(waiting_summary, waiting_summary.route_budget());
        assert_eq!(
            waiting.route_budget_commit_action(),
            RouteBudgetReadinessCommitAction::WaitForRouteBudget
        );
        assert_eq!(
            waiting.commit_summary().action,
            waiting.route_budget_commit_action()
        );

        let stale_budget = RouteBudget {
            threshold: 0.60,
            attention_tokens: ready_budget.attention_tokens,
            fast_tokens: ready_budget.fast_tokens.saturating_add(1),
            attention_fraction: ready_budget.attention_fraction,
        };
        let repair = RouteBudgetReadinessSummary::new(ready_summary, stale_budget);
        assert_eq!(
            repair.route_budget_commit_action(),
            RouteBudgetReadinessCommitAction::RepairRouteBudget
        );
        assert_eq!(
            repair.commit_summary().action,
            repair.route_budget_commit_action()
        );
    }

    #[test]
    fn route_budget_commit_summary_exposes_admission_boundary() {
        let decisions = [
            RoutingDecision {
                token: "fast".to_string(),
                score: 0.18,
                layer: RouteLayer::FastProjection,
            },
            RoutingDecision {
                token: "local".to_string(),
                score: 0.62,
                layer: RouteLayer::LocalWindow,
            },
        ];
        let ready_summary = RoutingDecisionSummary::from_decisions(0.60, &decisions);
        let ready_budget = ready_summary.route_budget();
        let ready = RouteBudgetReadinessSummary::new(ready_summary, ready_budget).commit_summary();
        let waiting_summary = RoutingDecisionSummary::from_decisions(0.60, &[]);
        let waiting =
            RouteBudgetReadinessSummary::new(waiting_summary, waiting_summary.route_budget())
                .commit_summary();
        let repair_budget = RouteBudget {
            threshold: 0.60,
            attention_tokens: ready_budget.attention_tokens,
            fast_tokens: ready_budget.fast_tokens.saturating_add(1),
            attention_fraction: ready_budget.attention_fraction,
        };
        let repair =
            RouteBudgetReadinessSummary::new(ready_summary, repair_budget).commit_summary();

        assert_eq!(
            ready.action,
            RouteBudgetReadinessCommitAction::CommitRouteBudget
        );
        assert_eq!(ready.route_budget_admission_signal_component_count(), 13);
        assert!(ready.has_route_budget_admission_signals());
        assert_eq!(ready.missing_committed_route_budget_component_count(), 0);
        assert_eq!(ready.route_budget_admission_blocker_component_count(), 0);
        assert!(!ready.has_route_budget_admission_blockers());
        assert!(ready.route_budget_admission_accounting_is_consistent());
        assert!(ready.route_budget_admission_is_clean());
        assert!(ready.can_admit_committed_route_budget());
        assert_eq!(ready.committed_route_budget, Some(ready_budget));

        assert_eq!(
            waiting.action,
            RouteBudgetReadinessCommitAction::WaitForRouteBudget
        );
        assert_eq!(waiting.route_budget_admission_signal_component_count(), 0);
        assert!(!waiting.has_route_budget_admission_signals());
        assert_eq!(waiting.missing_committed_route_budget_component_count(), 1);
        assert_eq!(waiting.route_budget_admission_blocker_component_count(), 1);
        assert!(waiting.has_route_budget_admission_blockers());
        assert!(waiting.route_budget_admission_accounting_is_consistent());
        assert!(!waiting.route_budget_admission_is_clean());
        assert!(!waiting.can_admit_committed_route_budget());
        assert_eq!(waiting.committed_route_budget, None);

        assert_eq!(
            repair.action,
            RouteBudgetReadinessCommitAction::RepairRouteBudget
        );
        assert_eq!(repair.route_budget_admission_signal_component_count(), 12);
        assert!(repair.has_route_budget_admission_signals());
        assert_eq!(repair.missing_committed_route_budget_component_count(), 1);
        assert_eq!(repair.total_blocker_component_count, 2);
        assert_eq!(repair.route_budget_admission_blocker_component_count(), 3);
        assert!(repair.has_route_budget_admission_blockers());
        assert!(repair.route_budget_admission_accounting_is_consistent());
        assert!(!repair.route_budget_admission_is_clean());
        assert!(!repair.can_admit_committed_route_budget());
        assert_eq!(repair.committed_route_budget, None);
    }

    #[test]
    fn routing_decision_summary_marks_all_attention_routes() {
        let decisions = [
            RoutingDecision {
                token: "global".to_string(),
                score: 0.86,
                layer: RouteLayer::Global,
            },
            RoutingDecision {
                token: "fusion".to_string(),
                score: 0.74,
                layer: RouteLayer::Fusion,
            },
        ];

        let summary = RoutingDecisionSummary::from_decisions(0.60, &decisions);

        assert_eq!(summary.layer_counts.total(), 2);
        assert!(!summary.layer_counts.has_fast_projection());
        assert!(summary.layer_counts.all_attention());
        assert!(summary.layer_counts.uses_multiple_layers());
        assert!(summary.has_attention_route());
        assert!(summary.all_attention_route());
        assert!(summary.has_score_spread());
        assert_eq!(summary.route_activity_signal_component_count(), 2);
        assert_eq!(summary.route_layer_signal_component_count(), 4);
        assert_eq!(summary.route_score_signal_component_count(), 3);
        assert_eq!(summary.routing_signal_component_count(), 9);
        assert_eq!(summary.routing_problem_component_count(), 0);
        assert!(summary.routing_accounting_is_consistent());
        assert!(summary.routing_shape_is_clean());
        assert!(summary.can_use_route_summary());

        let budget = summary.route_budget();
        assert_eq!(budget.fast_tokens, 0);
        assert_eq!(budget.attention_tokens, 2);
        assert!(budget.attention_dominates());
        assert!(budget.token_counts_match_fraction());
        assert_eq!(budget.route_budget_signal_component_count(), 3);
        assert_eq!(budget.route_budget_problem_component_count(), 0);
        assert!(budget.route_budget_accounting_is_consistent());
        assert!(budget.route_budget_shape_is_clean());
        assert!(budget.can_use_route_budget());
    }

    #[test]
    fn routing_decision_summary_counts_public_shape_drift() {
        let summary = RoutingDecisionSummary {
            threshold: 1.4,
            token_count: 2,
            layer_counts: RouteLayerCounts {
                fast_projection: 1,
                local_window: 1,
                global: 0,
                fusion: 1,
            },
            attention_fraction: 2.0,
            average_score: 0.80,
            min_score: 0.90,
            max_score: 0.70,
            above_threshold_tokens: 3,
            below_threshold_tokens: 0,
        };

        assert!(!summary.layer_counts_match_tokens());
        assert!(!summary.threshold_partition_matches_tokens());
        assert!(!summary.attention_fraction_matches_layers());
        assert!(!summary.score_range_is_valid());
        assert!(!summary.threshold_is_valid());
        assert_eq!(summary.route_activity_signal_component_count(), 2);
        assert_eq!(summary.route_layer_signal_component_count(), 4);
        assert_eq!(summary.route_score_signal_component_count(), 3);
        assert_eq!(summary.routing_signal_component_count(), 9);
        assert_eq!(summary.route_count_problem_component_count(), 2);
        assert_eq!(summary.route_score_problem_component_count(), 4);
        assert_eq!(summary.routing_problem_component_count(), 6);
        assert!(summary.has_routing_problem_components());
        assert!(summary.routing_accounting_is_consistent());
        assert!(!summary.routing_shape_is_clean());
        assert!(!summary.can_use_route_summary());

        let budget = RouteBudget {
            threshold: -0.2,
            attention_tokens: 2,
            fast_tokens: 1,
            attention_fraction: 0.10,
        };

        assert!(!budget.token_counts_match_fraction());
        assert_eq!(budget.route_budget_signal_component_count(), 4);
        assert_eq!(budget.route_budget_problem_component_count(), 2);
        assert!(budget.has_route_budget_problem_components());
        assert!(budget.route_budget_accounting_is_consistent());
        assert!(!budget.route_budget_shape_is_clean());
        assert!(!budget.can_use_route_budget());
    }

    #[test]
    fn adaptive_feedback_changes_profile_threshold() {
        let mut router = DefaultHierarchicalRouter::new();
        let before = router.threshold_for(TaskProfile::Writing);

        router.observe(RoutingFeedback {
            profile: TaskProfile::Writing,
            quality: 0.25,
            perplexity: 30.0,
            contradiction_count: 2,
        });

        assert!(router.threshold_for(TaskProfile::Writing) < before);
        assert_eq!(router.threshold_for(TaskProfile::Coding), before);
    }

    #[test]
    fn adaptive_feedback_does_not_raise_threshold_when_high_quality_has_contradictions() {
        let mut router = DefaultHierarchicalRouter::new();
        let before = router.threshold_for(TaskProfile::Writing);

        router.observe(RoutingFeedback {
            profile: TaskProfile::Writing,
            quality: 0.95,
            perplexity: 4.0,
            contradiction_count: 1,
        });

        assert_eq!(router.threshold_for(TaskProfile::Writing), before);
        assert_eq!(
            router
                .state()
                .profile_observations
                .get(TaskProfile::Writing),
            1
        );
    }

    #[test]
    fn generation_metrics_score_quality_with_contradiction_penalty() {
        let clean = GenerationMetrics {
            perplexity: 6.0,
            semantic_consistency: 0.90,
            contradiction_count: 0,
            token_count: 24,
        };
        let contradictory = GenerationMetrics {
            contradiction_count: 3,
            ..clean
        };

        assert!(clean.quality_score() > 0.75);
        assert!(contradictory.quality_score() < clean.quality_score());
    }

    #[test]
    fn generation_metrics_clamp_quality_inputs() {
        let metrics = GenerationMetrics {
            perplexity: 0.0,
            semantic_consistency: 3.0,
            contradiction_count: 99,
            token_count: 8,
        };

        assert!((0.0..=1.0).contains(&metrics.quality_score()));
    }

    #[test]
    fn generation_metrics_convert_to_routing_feedback() {
        let metrics = GenerationMetrics {
            perplexity: 18.0,
            semantic_consistency: 0.40,
            contradiction_count: 2,
            token_count: 128,
        };

        let feedback = metrics.routing_feedback(TaskProfile::Coding);

        assert_eq!(feedback.profile, TaskProfile::Coding);
        assert_eq!(feedback.perplexity, metrics.perplexity);
        assert_eq!(feedback.contradiction_count, metrics.contradiction_count);
        assert!((feedback.quality - metrics.quality_score()).abs() < 0.0001);
    }

    #[test]
    fn routing_feedback_summary_reports_threshold_pressure_inputs() {
        let feedback = RoutingFeedback {
            profile: TaskProfile::Writing,
            quality: 0.42,
            perplexity: 24.0,
            contradiction_count: 3,
        };

        let summary = feedback.feedback_summary();

        assert_eq!(summary.profile, TaskProfile::Writing);
        assert_eq!(summary.quality, 0.42);
        assert_eq!(summary.perplexity, 24.0);
        assert_eq!(summary.contradiction_count, 3);
        assert!(summary.is_low_quality());
        assert!(!summary.is_high_quality());
        assert!(summary.has_contradictions());
        assert!(summary.quality_shape_is_valid());
        assert!(summary.perplexity_shape_is_valid());
        assert_eq!(summary.feedback_signal_component_count(), 3);
        assert!(summary.has_feedback_signal_components());
        assert_eq!(summary.feedback_problem_component_count(), 0);
        assert!(!summary.has_feedback_problem_components());
        assert!(summary.feedback_accounting_is_consistent());
        assert!(summary.feedback_shape_is_clean());
        assert!(summary.can_use_routing_feedback());
    }

    #[test]
    fn routing_feedback_summary_does_not_mark_contradictory_feedback_high_quality() {
        let feedback = RoutingFeedback {
            profile: TaskProfile::Writing,
            quality: 0.95,
            perplexity: 4.0,
            contradiction_count: 1,
        };

        let summary = feedback.feedback_summary();

        assert!(!summary.is_low_quality());
        assert!(!summary.is_high_quality());
        assert!(summary.has_contradictions());
        assert_eq!(summary.feedback_signal_component_count(), 2);
        assert!(summary.feedback_accounting_is_consistent());
        assert!(summary.can_use_routing_feedback());
    }

    #[test]
    fn routing_feedback_summary_counts_invalid_public_shape() {
        let summary = RoutingFeedbackSummary {
            profile: TaskProfile::General,
            quality: -0.10,
            perplexity: f32::NAN,
            contradiction_count: 1,
        };

        assert!(!summary.quality_shape_is_valid());
        assert!(!summary.perplexity_shape_is_valid());
        assert_eq!(summary.feedback_signal_component_count(), 2);
        assert_eq!(summary.feedback_problem_component_count(), 2);
        assert!(summary.has_feedback_problem_components());
        assert!(summary.feedback_accounting_is_consistent());
        assert!(!summary.feedback_shape_is_clean());
        assert!(!summary.can_use_routing_feedback());
    }

    #[test]
    fn routing_feedback_batch_summary_counts_profiles_and_quality_pressure() {
        let feedback = [
            RoutingFeedback {
                profile: TaskProfile::Coding,
                quality: 0.90,
                perplexity: 7.0,
                contradiction_count: 0,
            },
            RoutingFeedback {
                profile: TaskProfile::Coding,
                quality: 0.52,
                perplexity: 18.0,
                contradiction_count: 1,
            },
            RoutingFeedback {
                profile: TaskProfile::LongDocument,
                quality: 0.40,
                perplexity: 26.0,
                contradiction_count: 2,
            },
        ];

        let summary = RoutingFeedback::batch_summary(&feedback);

        assert_eq!(summary.feedback_count, 3);
        assert_eq!(summary.profile_counts.get(TaskProfile::Coding), 2);
        assert_eq!(summary.profile_counts.get(TaskProfile::LongDocument), 1);
        assert_eq!(summary.profile_counts.active_profile_count(), 2);
        assert!((summary.average_quality - (1.82 / 3.0)).abs() < 0.0001);
        assert!((summary.average_perplexity - 17.0).abs() < 0.0001);
        assert_eq!(summary.contradiction_total, 3);
        assert_eq!(summary.low_quality_count, 2);
        assert_eq!(summary.high_quality_count, 1);
        assert!(!summary.is_empty());
        assert!(summary.has_mixed_profiles());
        assert!(summary.has_quality_pressure());
        assert!(summary.profile_count_matches_feedback());
        assert!(summary.quality_bucket_counts_are_bounded());
        assert!(summary.average_quality_shape_is_valid());
        assert!(summary.average_perplexity_shape_is_valid());
        assert_eq!(summary.feedback_batch_signal_component_count(), 4);
        assert!(summary.has_feedback_batch_signal_components());
        assert_eq!(summary.feedback_batch_problem_component_count(), 0);
        assert!(!summary.has_feedback_batch_problem_components());
        assert!(summary.feedback_batch_accounting_is_consistent());
        assert!(summary.feedback_batch_shape_is_clean());
        assert!(summary.can_use_routing_feedback_batch());
    }

    #[test]
    fn empty_routing_feedback_batch_summary_is_noop() {
        let summary = RoutingFeedbackBatchSummary::from_feedback(&[]);

        assert_eq!(summary.feedback_count, 0);
        assert_eq!(summary.profile_counts.total(), 0);
        assert_eq!(summary.average_quality, 0.0);
        assert_eq!(summary.average_perplexity, 0.0);
        assert_eq!(summary.contradiction_total, 0);
        assert_eq!(summary.low_quality_count, 0);
        assert_eq!(summary.high_quality_count, 0);
        assert!(summary.is_empty());
        assert!(!summary.has_mixed_profiles());
        assert!(!summary.has_quality_pressure());
        assert_eq!(summary.feedback_batch_signal_component_count(), 0);
        assert_eq!(summary.feedback_batch_problem_component_count(), 0);
        assert!(!summary.has_feedback_batch_signal_components());
        assert!(!summary.has_feedback_batch_problem_components());
        assert!(summary.feedback_batch_accounting_is_consistent());
        assert!(summary.feedback_batch_shape_is_clean());
        assert!(!summary.can_use_routing_feedback_batch());
    }

    #[test]
    fn routing_feedback_batch_summary_counts_public_shape_drift() {
        let summary = RoutingFeedbackBatchSummary {
            feedback_count: 2,
            profile_counts: ProfileObservations {
                general: 1,
                coding: 1,
                writing: 1,
                long_document: 0,
            },
            average_quality: 1.20,
            average_perplexity: f32::NEG_INFINITY,
            contradiction_total: 1,
            low_quality_count: 3,
            high_quality_count: 4,
        };

        assert!(!summary.profile_count_matches_feedback());
        assert!(!summary.quality_bucket_counts_are_bounded());
        assert!(!summary.average_quality_shape_is_valid());
        assert!(!summary.average_perplexity_shape_is_valid());
        assert_eq!(summary.feedback_batch_signal_component_count(), 4);
        assert_eq!(summary.feedback_batch_problem_component_count(), 4);
        assert!(summary.has_feedback_batch_problem_components());
        assert!(summary.feedback_batch_accounting_is_consistent());
        assert!(!summary.feedback_batch_shape_is_clean());
        assert!(!summary.can_use_routing_feedback_batch());
    }

    #[test]
    fn router_state_tracks_profile_observations() {
        let mut router = DefaultHierarchicalRouter::new();

        router.observe(
            GenerationMetrics {
                perplexity: 28.0,
                semantic_consistency: 0.30,
                contradiction_count: 2,
                token_count: 64,
            }
            .routing_feedback(TaskProfile::Writing),
        );

        let state = router.state();

        assert_eq!(state.observations, 1);
        assert_eq!(state.profile_observations.get(TaskProfile::Writing), 1);
        assert_eq!(state.profile_observations.get(TaskProfile::Coding), 0);
        assert_eq!(state.profile_observation_total(), 1);
        assert_eq!(state.profile_observations.active_profile_count(), 1);
        assert!(!state.has_observation_drift());
        assert_eq!(state.threshold, router.threshold_for(TaskProfile::Writing));
    }

    #[test]
    fn router_state_restore_clamps_profile_thresholds() {
        let mut router = DefaultHierarchicalRouter::new();
        let mut thresholds = ProfileThresholds::from_single(0.52);
        thresholds.set(TaskProfile::Coding, 2.0);
        thresholds.set(TaskProfile::Writing, -1.0);

        router.restore_state(RouterState {
            threshold: 9.0,
            observations: 7,
            profile_thresholds: thresholds,
            profile_observations: ProfileObservations {
                coding: 3,
                writing: 4,
                ..ProfileObservations::default()
            },
        });

        let state = router.state();

        assert_eq!(state.threshold, 0.88);
        assert_eq!(router.threshold_for(TaskProfile::Coding), 0.88);
        assert_eq!(router.threshold_for(TaskProfile::Writing), 0.18);
        assert_eq!(state.observations, 7);
        assert_eq!(state.profile_observations.get(TaskProfile::Coding), 3);
        assert_eq!(state.profile_observation_total(), 7);
        assert!(!state.has_observation_drift());
    }

    #[test]
    fn router_state_reports_observation_count_drift() {
        let state = RouterState {
            threshold: 0.52,
            observations: 9,
            profile_thresholds: ProfileThresholds::from_single(0.52),
            profile_observations: ProfileObservations {
                general: 2,
                coding: 3,
                writing: 1,
                long_document: 0,
            },
        };

        assert_eq!(state.profile_observation_total(), 6);
        assert_eq!(state.profile_observations.active_profile_count(), 3);
        assert_eq!(state.observation_count_drift(), 3);
        assert!(state.has_observation_drift());
    }
}
