use crate::experiment::ExperimentSwitches;
use crate::router::RouteBudget;
use crate::runtime::RuntimeMetadata;
use crate::transformer::TransformerPlanningReadinessSummary;

#[derive(Debug, Clone, PartialEq)]
pub struct FhtDkeInput {
    pub prompt_tokens: usize,
    pub max_generated_tokens: usize,
    pub route_budget: RouteBudget,
    pub runtime: RuntimeMetadata,
    pub experiments: ExperimentSwitches,
}

impl FhtDkeInput {
    pub fn new(
        prompt_tokens: usize,
        max_generated_tokens: usize,
        runtime: RuntimeMetadata,
    ) -> Self {
        Self {
            prompt_tokens,
            max_generated_tokens: max_generated_tokens.max(1),
            route_budget: RouteBudget::default(),
            runtime,
            experiments: ExperimentSwitches::default(),
        }
    }

    pub fn with_route_budget(mut self, route_budget: RouteBudget) -> Self {
        self.route_budget = route_budget;
        self
    }

    pub fn with_experiments(mut self, experiments: ExperimentSwitches) -> Self {
        self.experiments = experiments;
        self
    }

    pub fn requested_tokens(&self) -> usize {
        self.prompt_tokens.saturating_add(self.max_generated_tokens)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FhtDkeBudget {
    pub enabled: bool,
    pub total_tokens: usize,
    pub dense_tokens: usize,
    pub routed_tokens: usize,
    pub kv_import_blocks: usize,
    pub kv_export_blocks: usize,
    pub attention_threshold: f32,
    pub route_pressure: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FhtDkeBudgetSummary {
    pub enabled: bool,
    pub total_tokens: usize,
    pub dense_tokens: usize,
    pub routed_tokens: usize,
    pub dense_fraction: f32,
    pub routed_fraction: f32,
    pub kv_import_blocks: usize,
    pub kv_export_blocks: usize,
    pub kv_exchange_blocks: usize,
    pub has_kv_exchange: bool,
    pub token_split_is_valid: bool,
    pub attention_threshold: f32,
    pub route_pressure: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FhtDkePlanningReadinessStage {
    TransformerPlanning,
    BudgetCommit,
    PressureBudgetBoundary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FhtDkePlanningCommitAction {
    CommitFhtDkePlanning,
    WaitForFhtDkePlanning,
    RepairFhtDkePlanning,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FhtDkePlanningReadinessSummary {
    pub transformer_planning: TransformerPlanningReadinessSummary,
    pub fht_dke_budget: FhtDkeBudgetSummary,
    pub transformer_planning_signal_component_count: usize,
    pub fht_dke_budget_signal_component_count: usize,
    pub pressure_budget_boundary_signal_component_count: usize,
    pub transformer_planning_blocker_component_count: usize,
    pub fht_dke_budget_blocker_component_count: usize,
    pub pressure_budget_boundary_blocker_component_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FhtDkePlanningCommitSummary {
    pub readiness: FhtDkePlanningReadinessSummary,
    pub action: FhtDkePlanningCommitAction,
    pub committed_transformer_planning: Option<TransformerPlanningReadinessSummary>,
    pub committed_fht_dke_budget: Option<FhtDkeBudgetSummary>,
    pub can_commit: bool,
    pub should_wait_for_fht_dke_planning: bool,
    pub should_repair_fht_dke_planning: bool,
    pub first_unready_stage: Option<FhtDkePlanningReadinessStage>,
    pub first_blocking_stage: Option<FhtDkePlanningReadinessStage>,
    pub total_signal_component_count: usize,
    pub total_blocker_component_count: usize,
    pub component_accounting_consistent: bool,
}

impl FhtDkePlanningCommitAction {
    pub fn can_commit(self) -> bool {
        matches!(self, Self::CommitFhtDkePlanning)
    }

    pub fn should_wait_for_fht_dke_planning(self) -> bool {
        matches!(self, Self::WaitForFhtDkePlanning)
    }

    pub fn should_repair_fht_dke_planning(self) -> bool {
        matches!(self, Self::RepairFhtDkePlanning)
    }
}

impl FhtDkeBudgetSummary {
    pub fn has_route_pressure(self) -> bool {
        self.route_pressure > 0.0
    }

    pub fn route_pressure_signal_component_count(self) -> usize {
        usize::from(self.has_route_pressure())
    }

    pub fn route_pressure_is_high(self) -> bool {
        self.route_pressure >= 0.72
    }

    pub fn high_route_pressure_signal_component_count(self) -> usize {
        usize::from(self.route_pressure_is_high())
    }

    pub fn has_routed_work(self) -> bool {
        self.routed_tokens > 0
    }

    pub fn routed_work_signal_component_count(self) -> usize {
        usize::from(self.has_routed_work())
    }

    pub fn dense_tokens_dominate(self) -> bool {
        self.dense_tokens >= self.routed_tokens
    }

    pub fn kv_exchange_is_symmetric(self) -> bool {
        self.kv_import_blocks == self.kv_export_blocks
    }

    pub fn kv_exchange_blocks_match_parts(self) -> bool {
        self.kv_exchange_blocks == self.kv_import_blocks.saturating_add(self.kv_export_blocks)
    }

    pub fn kv_exchange_block_sum_drift(self) -> usize {
        self.kv_exchange_blocks
            .abs_diff(self.kv_import_blocks.saturating_add(self.kv_export_blocks))
    }

    pub fn kv_exchange_block_sum_drift_component_count(self) -> usize {
        usize::from(!self.kv_exchange_blocks_match_parts())
    }

    pub fn kv_exchange_flag_matches_blocks(self) -> bool {
        self.has_kv_exchange == (self.kv_exchange_blocks > 0)
    }

    pub fn kv_exchange_flag_drift_component_count(self) -> usize {
        usize::from(!self.kv_exchange_flag_matches_blocks())
    }

    pub fn kv_exchange_signal_component_count(self) -> usize {
        usize::from(self.has_kv_exchange)
    }

    pub fn kv_exchange_asymmetry_signal_component_count(self) -> usize {
        usize::from(!self.kv_exchange_is_symmetric())
    }

    pub fn token_split_invalid_component_count(self) -> usize {
        usize::from(!self.token_split_is_valid)
    }

    pub fn attention_threshold_is_valid(self) -> bool {
        finite_unit(self.attention_threshold)
    }

    pub fn attention_threshold_shape_problem_component_count(self) -> usize {
        usize::from(!self.attention_threshold_is_valid())
    }

    pub fn attention_threshold_admission_signal_component_count(self) -> usize {
        usize::from(self.attention_threshold_is_valid())
    }

    pub fn has_attention_threshold_admission_signals(self) -> bool {
        self.attention_threshold_admission_signal_component_count() > 0
    }

    pub fn attention_threshold_admission_blocker_component_count(self) -> usize {
        self.attention_threshold_shape_problem_component_count()
    }

    pub fn has_attention_threshold_admission_blockers(self) -> bool {
        self.attention_threshold_admission_blocker_component_count() > 0
    }

    pub fn attention_threshold_admission_accounting_is_consistent(self) -> bool {
        let expected_signal_count = usize::from(self.attention_threshold_is_valid());
        let expected_blocker_count = usize::from(!self.attention_threshold_is_valid());

        self.attention_threshold_admission_signal_component_count() == expected_signal_count
            && self.has_attention_threshold_admission_signals() == (expected_signal_count > 0)
            && self.attention_threshold_admission_blocker_component_count()
                == expected_blocker_count
            && self.has_attention_threshold_admission_blockers() == (expected_blocker_count > 0)
    }

    pub fn attention_threshold_admission_is_clean(self) -> bool {
        !self.has_attention_threshold_admission_blockers()
            && self.attention_threshold_admission_accounting_is_consistent()
    }

    pub fn can_admit_attention_threshold(self) -> bool {
        self.attention_threshold_admission_is_clean()
    }

    pub fn route_pressure_shape_is_valid(self) -> bool {
        finite_unit(self.route_pressure)
    }

    pub fn route_pressure_shape_problem_component_count(self) -> usize {
        usize::from(!self.route_pressure_shape_is_valid())
    }

    pub fn empty_budget_blocker_component_count(self) -> usize {
        usize::from(self.total_tokens == 0)
    }

    pub fn budget_shape_problem_component_count(self) -> usize {
        self.token_split_invalid_component_count()
            .saturating_add(self.kv_exchange_block_sum_drift_component_count())
            .saturating_add(self.kv_exchange_flag_drift_component_count())
            .saturating_add(self.attention_threshold_shape_problem_component_count())
            .saturating_add(self.route_pressure_shape_problem_component_count())
    }

    pub fn has_budget_shape_problem_components(self) -> bool {
        self.budget_shape_problem_component_count() > 0
    }

    pub fn budget_pressure_signal_component_count(self) -> usize {
        self.route_pressure_signal_component_count()
            .saturating_add(self.high_route_pressure_signal_component_count())
            .saturating_add(self.routed_work_signal_component_count())
            .saturating_add(self.kv_exchange_signal_component_count())
            .saturating_add(self.kv_exchange_asymmetry_signal_component_count())
    }

    pub fn fht_dke_budget_commit_signal_component_count(self) -> usize {
        self.budget_pressure_signal_component_count()
    }

    pub fn has_fht_dke_budget_commit_signals(self) -> bool {
        self.fht_dke_budget_commit_signal_component_count() > 0
    }

    pub fn fht_dke_budget_commit_blocker_component_count(self) -> usize {
        self.empty_budget_blocker_component_count()
            .saturating_add(self.budget_shape_problem_component_count())
    }

    pub fn has_fht_dke_budget_commit_blockers(self) -> bool {
        self.fht_dke_budget_commit_blocker_component_count() > 0
    }

    pub fn budget_shape_accounting_is_consistent(self) -> bool {
        let expected_problem_count = usize::from(!self.token_split_is_valid)
            .saturating_add(usize::from(!self.kv_exchange_blocks_match_parts()))
            .saturating_add(usize::from(!self.kv_exchange_flag_matches_blocks()))
            .saturating_add(usize::from(!self.attention_threshold_is_valid()))
            .saturating_add(usize::from(!self.route_pressure_shape_is_valid()));

        self.budget_shape_problem_component_count() == expected_problem_count
            && self.has_budget_shape_problem_components() == (expected_problem_count > 0)
    }

    pub fn fht_dke_budget_commit_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self.budget_pressure_signal_component_count();
        let expected_blocker_count = self
            .empty_budget_blocker_component_count()
            .saturating_add(self.budget_shape_problem_component_count());

        self.budget_shape_accounting_is_consistent()
            && self.fht_dke_budget_commit_signal_component_count() == expected_signal_count
            && self.has_fht_dke_budget_commit_signals() == (expected_signal_count > 0)
            && self.fht_dke_budget_commit_blocker_component_count() == expected_blocker_count
            && self.has_fht_dke_budget_commit_blockers() == (expected_blocker_count > 0)
    }

    pub fn budget_shape_is_clean(self) -> bool {
        !self.has_budget_shape_problem_components() && self.budget_shape_accounting_is_consistent()
    }

    pub fn can_use_fht_dke_budget(self) -> bool {
        self.total_tokens > 0 && self.budget_shape_is_clean()
    }

    pub fn fht_dke_budget_commit_is_clean(self) -> bool {
        !self.has_fht_dke_budget_commit_blockers()
            && self.fht_dke_budget_commit_accounting_is_consistent()
    }

    pub fn can_commit_fht_dke_budget(self) -> bool {
        self.fht_dke_budget_commit_is_clean()
    }

    pub fn routed_tokens_per_kv_exchange_block(self) -> Option<f32> {
        (self.kv_exchange_blocks > 0)
            .then_some(self.routed_tokens as f32 / self.kv_exchange_blocks as f32)
    }
}

impl FhtDkePlanningReadinessSummary {
    pub fn new(
        transformer_planning: TransformerPlanningReadinessSummary,
        fht_dke_budget: FhtDkeBudgetSummary,
    ) -> Self {
        Self {
            transformer_planning,
            fht_dke_budget,
            transformer_planning_signal_component_count: transformer_planning
                .transformer_planning_readiness_signal_component_count(),
            fht_dke_budget_signal_component_count: fht_dke_budget
                .fht_dke_budget_commit_signal_component_count(),
            pressure_budget_boundary_signal_component_count: usize::from(
                Self::pressure_budget_boundary_matches_parts(transformer_planning, fht_dke_budget),
            ),
            transformer_planning_blocker_component_count: transformer_planning
                .transformer_planning_readiness_blocker_component_count(),
            fht_dke_budget_blocker_component_count: fht_dke_budget
                .fht_dke_budget_commit_blocker_component_count(),
            pressure_budget_boundary_blocker_component_count:
                Self::pressure_budget_boundary_drift_component_count_parts(
                    transformer_planning,
                    fht_dke_budget,
                ),
        }
    }

    pub fn stage_order() -> [FhtDkePlanningReadinessStage; 3] {
        [
            FhtDkePlanningReadinessStage::TransformerPlanning,
            FhtDkePlanningReadinessStage::BudgetCommit,
            FhtDkePlanningReadinessStage::PressureBudgetBoundary,
        ]
    }

    pub fn route_pressure_matches_budget(self) -> bool {
        float_close(
            self.transformer_planning
                .planning_pressure
                .route_attention_fraction,
            self.fht_dke_budget.route_pressure,
        )
    }

    pub fn attention_threshold_matches_budget(self) -> bool {
        float_close(
            self.transformer_planning
                .route_budget
                .route_budget
                .threshold,
            self.fht_dke_budget.attention_threshold,
        )
    }

    pub fn pressure_budget_boundary_matches(self) -> bool {
        Self::pressure_budget_boundary_matches_parts(self.transformer_planning, self.fht_dke_budget)
    }

    pub fn pressure_budget_boundary_drift_component_count(self) -> usize {
        Self::pressure_budget_boundary_drift_component_count_parts(
            self.transformer_planning,
            self.fht_dke_budget,
        )
    }

    pub fn transformer_planning_ready(self) -> bool {
        self.transformer_planning
            .can_commit_transformer_planning_readiness()
    }

    pub fn fht_dke_budget_ready(self) -> bool {
        self.fht_dke_budget.can_commit_fht_dke_budget()
    }

    pub fn pressure_budget_boundary_ready(self) -> bool {
        self.pressure_budget_boundary_matches()
    }

    pub fn stage_ready(self, stage: FhtDkePlanningReadinessStage) -> bool {
        match stage {
            FhtDkePlanningReadinessStage::TransformerPlanning => self.transformer_planning_ready(),
            FhtDkePlanningReadinessStage::BudgetCommit => self.fht_dke_budget_ready(),
            FhtDkePlanningReadinessStage::PressureBudgetBoundary => {
                self.pressure_budget_boundary_ready()
            }
        }
    }

    pub fn stage_signal_component_count(self, stage: FhtDkePlanningReadinessStage) -> usize {
        match stage {
            FhtDkePlanningReadinessStage::TransformerPlanning => {
                self.transformer_planning_signal_component_count
            }
            FhtDkePlanningReadinessStage::BudgetCommit => {
                self.fht_dke_budget_signal_component_count
            }
            FhtDkePlanningReadinessStage::PressureBudgetBoundary => {
                self.pressure_budget_boundary_signal_component_count
            }
        }
    }

    pub fn stage_blocker_component_count(self, stage: FhtDkePlanningReadinessStage) -> usize {
        match stage {
            FhtDkePlanningReadinessStage::TransformerPlanning => {
                self.transformer_planning_blocker_component_count
            }
            FhtDkePlanningReadinessStage::BudgetCommit => {
                self.fht_dke_budget_blocker_component_count
            }
            FhtDkePlanningReadinessStage::PressureBudgetBoundary => {
                self.pressure_budget_boundary_blocker_component_count
            }
        }
    }

    pub fn first_unready_stage(self) -> Option<FhtDkePlanningReadinessStage> {
        Self::stage_order()
            .into_iter()
            .find(|stage| !self.stage_ready(*stage))
    }

    pub fn first_blocking_stage(self) -> Option<FhtDkePlanningReadinessStage> {
        Self::stage_order()
            .into_iter()
            .find(|stage| self.stage_blocker_component_count(*stage) > 0)
    }

    pub fn fht_dke_planning_readiness_signal_component_count(self) -> usize {
        self.transformer_planning_signal_component_count
            .saturating_add(self.fht_dke_budget_signal_component_count)
            .saturating_add(self.pressure_budget_boundary_signal_component_count)
    }

    pub fn has_fht_dke_planning_readiness_signals(self) -> bool {
        self.fht_dke_planning_readiness_signal_component_count() > 0
    }

    pub fn fht_dke_planning_readiness_blocker_component_count(self) -> usize {
        self.transformer_planning_blocker_component_count
            .saturating_add(self.fht_dke_budget_blocker_component_count)
            .saturating_add(self.pressure_budget_boundary_blocker_component_count)
    }

    pub fn has_fht_dke_planning_readiness_blockers(self) -> bool {
        self.fht_dke_planning_readiness_blocker_component_count() > 0
    }

    pub fn fht_dke_planning_readiness_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .transformer_planning_signal_component_count
            .saturating_add(self.fht_dke_budget_signal_component_count)
            .saturating_add(self.pressure_budget_boundary_signal_component_count);
        let expected_blocker_count = self
            .transformer_planning_blocker_component_count
            .saturating_add(self.fht_dke_budget_blocker_component_count)
            .saturating_add(self.pressure_budget_boundary_blocker_component_count);

        self.transformer_planning
            .transformer_planning_readiness_accounting_is_consistent()
            && self
                .fht_dke_budget
                .fht_dke_budget_commit_accounting_is_consistent()
            && self.transformer_planning_signal_component_count
                == self
                    .transformer_planning
                    .transformer_planning_readiness_signal_component_count()
            && self.fht_dke_budget_signal_component_count
                == self
                    .fht_dke_budget
                    .fht_dke_budget_commit_signal_component_count()
            && self.pressure_budget_boundary_signal_component_count
                == usize::from(self.pressure_budget_boundary_matches())
            && self.transformer_planning_blocker_component_count
                == self
                    .transformer_planning
                    .transformer_planning_readiness_blocker_component_count()
            && self.fht_dke_budget_blocker_component_count
                == self
                    .fht_dke_budget
                    .fht_dke_budget_commit_blocker_component_count()
            && self.pressure_budget_boundary_blocker_component_count
                == self.pressure_budget_boundary_drift_component_count()
            && self.fht_dke_planning_readiness_signal_component_count() == expected_signal_count
            && self.has_fht_dke_planning_readiness_signals() == (expected_signal_count > 0)
            && self.fht_dke_planning_readiness_blocker_component_count() == expected_blocker_count
            && self.has_fht_dke_planning_readiness_blockers() == (expected_blocker_count > 0)
    }

    pub fn fht_dke_planning_readiness_is_clean(self) -> bool {
        !self.has_fht_dke_planning_readiness_blockers()
            && self.fht_dke_planning_readiness_accounting_is_consistent()
    }

    pub fn can_commit_fht_dke_planning_readiness(self) -> bool {
        self.fht_dke_planning_readiness_is_clean()
            && self.transformer_planning_ready()
            && self.fht_dke_budget_ready()
            && self.pressure_budget_boundary_ready()
    }

    pub fn commit_summary(self) -> FhtDkePlanningCommitSummary {
        FhtDkePlanningCommitSummary::new(self)
    }

    fn pressure_budget_boundary_matches_parts(
        transformer_planning: TransformerPlanningReadinessSummary,
        fht_dke_budget: FhtDkeBudgetSummary,
    ) -> bool {
        float_close(
            transformer_planning
                .planning_pressure
                .route_attention_fraction,
            fht_dke_budget.route_pressure,
        ) && float_close(
            transformer_planning.route_budget.route_budget.threshold,
            fht_dke_budget.attention_threshold,
        )
    }

    fn pressure_budget_boundary_drift_component_count_parts(
        transformer_planning: TransformerPlanningReadinessSummary,
        fht_dke_budget: FhtDkeBudgetSummary,
    ) -> usize {
        usize::from(!float_close(
            transformer_planning
                .planning_pressure
                .route_attention_fraction,
            fht_dke_budget.route_pressure,
        ))
        .saturating_add(usize::from(!float_close(
            transformer_planning.route_budget.route_budget.threshold,
            fht_dke_budget.attention_threshold,
        )))
    }
}

impl FhtDkePlanningCommitSummary {
    pub fn new(readiness: FhtDkePlanningReadinessSummary) -> Self {
        let component_accounting_consistent =
            readiness.fht_dke_planning_readiness_accounting_is_consistent();
        let action = if readiness.can_commit_fht_dke_planning_readiness() {
            FhtDkePlanningCommitAction::CommitFhtDkePlanning
        } else if component_accounting_consistent
            && !readiness.has_fht_dke_planning_readiness_blockers()
        {
            FhtDkePlanningCommitAction::WaitForFhtDkePlanning
        } else {
            FhtDkePlanningCommitAction::RepairFhtDkePlanning
        };
        let committed_transformer_planning = action
            .can_commit()
            .then_some(readiness.transformer_planning);
        let committed_fht_dke_budget = action.can_commit().then_some(readiness.fht_dke_budget);

        Self {
            readiness,
            action,
            committed_transformer_planning,
            committed_fht_dke_budget,
            can_commit: action.can_commit(),
            should_wait_for_fht_dke_planning: action.should_wait_for_fht_dke_planning(),
            should_repair_fht_dke_planning: action.should_repair_fht_dke_planning(),
            first_unready_stage: readiness.first_unready_stage(),
            first_blocking_stage: readiness.first_blocking_stage(),
            total_signal_component_count: readiness
                .fht_dke_planning_readiness_signal_component_count(),
            total_blocker_component_count: readiness
                .fht_dke_planning_readiness_blocker_component_count(),
            component_accounting_consistent,
        }
    }

    pub fn action_can_commit(self) -> bool {
        self.action.can_commit()
    }

    pub fn action_should_wait_for_fht_dke_planning(self) -> bool {
        self.action.should_wait_for_fht_dke_planning()
    }

    pub fn action_should_repair_fht_dke_planning(self) -> bool {
        self.action.should_repair_fht_dke_planning()
    }

    pub fn can_commit_fht_dke_planning(self) -> bool {
        self.can_commit
    }

    pub fn should_wait_for_fht_dke_planning(self) -> bool {
        self.should_wait_for_fht_dke_planning
    }

    pub fn should_repair_fht_dke_planning(self) -> bool {
        self.should_repair_fht_dke_planning
    }

    pub fn can_use_committed_fht_dke_budget(self) -> bool {
        self.can_commit && self.committed_fht_dke_budget.is_some()
    }

    pub fn can_use_committed_transformer_planning(self) -> bool {
        self.can_commit && self.committed_transformer_planning.is_some()
    }

    pub fn can_use_committed_runtime_planning_parts(self) -> bool {
        self.can_use_committed_transformer_planning() && self.can_use_committed_fht_dke_budget()
    }

    pub fn commit_decision_accounting_is_consistent(self) -> bool {
        let expected_action = if self.readiness.can_commit_fht_dke_planning_readiness() {
            FhtDkePlanningCommitAction::CommitFhtDkePlanning
        } else if self.component_accounting_consistent
            && !self.readiness.has_fht_dke_planning_readiness_blockers()
        {
            FhtDkePlanningCommitAction::WaitForFhtDkePlanning
        } else {
            FhtDkePlanningCommitAction::RepairFhtDkePlanning
        };
        let expected_committed_transformer_planning = expected_action
            .can_commit()
            .then_some(self.readiness.transformer_planning);
        let expected_committed_fht_dke_budget = expected_action
            .can_commit()
            .then_some(self.readiness.fht_dke_budget);

        self.action == expected_action
            && self.committed_transformer_planning == expected_committed_transformer_planning
            && self.committed_fht_dke_budget == expected_committed_fht_dke_budget
            && self.can_commit == expected_action.can_commit()
            && self.should_wait_for_fht_dke_planning
                == expected_action.should_wait_for_fht_dke_planning()
            && self.should_repair_fht_dke_planning
                == expected_action.should_repair_fht_dke_planning()
            && self.first_unready_stage == self.readiness.first_unready_stage()
            && self.first_blocking_stage == self.readiness.first_blocking_stage()
            && self.total_signal_component_count
                == self
                    .readiness
                    .fht_dke_planning_readiness_signal_component_count()
            && self.total_blocker_component_count
                == self
                    .readiness
                    .fht_dke_planning_readiness_blocker_component_count()
            && self.component_accounting_consistent
                == self
                    .readiness
                    .fht_dke_planning_readiness_accounting_is_consistent()
    }
}

impl FhtDkeBudget {
    pub fn disabled(total_tokens: usize, attention_threshold: f32) -> Self {
        Self {
            enabled: false,
            total_tokens,
            dense_tokens: total_tokens,
            routed_tokens: 0,
            kv_import_blocks: 0,
            kv_export_blocks: 0,
            attention_threshold,
            route_pressure: 0.0,
        }
    }

    pub fn dense_fraction(self) -> f32 {
        self.dense_tokens as f32 / self.total_tokens.max(1) as f32
    }

    pub fn routed_fraction(self) -> f32 {
        self.routed_tokens as f32 / self.total_tokens.max(1) as f32
    }

    pub fn kv_exchange_blocks(self) -> usize {
        self.kv_import_blocks.saturating_add(self.kv_export_blocks)
    }

    pub fn has_kv_exchange(self) -> bool {
        self.kv_exchange_blocks() > 0
    }

    pub fn token_split_is_valid(self) -> bool {
        self.dense_tokens.saturating_add(self.routed_tokens) == self.total_tokens
            && (self.enabled || (self.routed_tokens == 0 && self.dense_tokens == self.total_tokens))
    }

    pub fn budget_summary(self) -> FhtDkeBudgetSummary {
        FhtDkeBudgetSummary {
            enabled: self.enabled,
            total_tokens: self.total_tokens,
            dense_tokens: self.dense_tokens,
            routed_tokens: self.routed_tokens,
            dense_fraction: self.dense_fraction(),
            routed_fraction: self.routed_fraction(),
            kv_import_blocks: self.kv_import_blocks,
            kv_export_blocks: self.kv_export_blocks,
            kv_exchange_blocks: self.kv_exchange_blocks(),
            has_kv_exchange: self.has_kv_exchange(),
            token_split_is_valid: self.token_split_is_valid(),
            attention_threshold: self.attention_threshold,
            route_pressure: self.route_pressure,
        }
    }
}

pub trait FhtDkeBudgeter {
    fn budget(&self, input: &FhtDkeInput) -> FhtDkeBudget;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeterministicFhtDkeBudgeter {
    pub min_dense_fraction: f32,
    pub max_dense_fraction: f32,
    pub tokens_per_kv_block: usize,
}

impl DeterministicFhtDkeBudgeter {
    pub fn new(
        min_dense_fraction: f32,
        max_dense_fraction: f32,
        tokens_per_kv_block: usize,
    ) -> Self {
        let min_dense_fraction = min_dense_fraction.clamp(0.01, 0.95);
        let max_dense_fraction = max_dense_fraction.clamp(min_dense_fraction, 1.0);
        Self {
            min_dense_fraction,
            max_dense_fraction,
            tokens_per_kv_block: tokens_per_kv_block.max(1),
        }
    }
}

impl Default for DeterministicFhtDkeBudgeter {
    fn default() -> Self {
        Self {
            min_dense_fraction: 0.12,
            max_dense_fraction: 0.58,
            tokens_per_kv_block: 256,
        }
    }
}

impl FhtDkeBudgeter for DeterministicFhtDkeBudgeter {
    fn budget(&self, input: &FhtDkeInput) -> FhtDkeBudget {
        let total_tokens = bounded_total_tokens(input);
        if !input.experiments.enable_fht_dke {
            return FhtDkeBudget::disabled(total_tokens, input.route_budget.threshold);
        }

        let context_window = context_window(input, total_tokens);
        let context_pressure = total_tokens as f32 / context_window as f32;
        let route_pressure = input.route_budget.attention_fraction.clamp(0.0, 1.0);
        let dense_fraction = (0.22 + route_pressure * 0.28 - context_pressure * 0.10)
            .clamp(self.min_dense_fraction, self.max_dense_fraction);
        let dense_tokens =
            ((total_tokens as f32 * dense_fraction).round() as usize).clamp(1, total_tokens.max(1));
        let routed_tokens = total_tokens.saturating_sub(dense_tokens);
        let needed_blocks = div_ceil(routed_tokens, self.tokens_per_kv_block);

        FhtDkeBudget {
            enabled: true,
            total_tokens,
            dense_tokens,
            routed_tokens,
            kv_import_blocks: if input.runtime.supports_kv_import {
                needed_blocks.min(input.runtime.max_kv_import_blocks)
            } else {
                0
            },
            kv_export_blocks: if input.runtime.supports_kv_export {
                needed_blocks.min(input.runtime.max_kv_export_blocks)
            } else {
                0
            },
            attention_threshold: input.route_budget.threshold,
            route_pressure,
        }
    }
}

fn bounded_total_tokens(input: &FhtDkeInput) -> usize {
    let requested = input.requested_tokens().max(1);
    if input.runtime.native_context_window == 0 {
        requested
    } else {
        requested.min(input.runtime.native_context_window)
    }
}

fn context_window(input: &FhtDkeInput, total_tokens: usize) -> usize {
    input.runtime.native_context_window.max(total_tokens).max(1)
}

fn div_ceil(value: usize, divisor: usize) -> usize {
    if value == 0 {
        0
    } else {
        1 + (value - 1) / divisor.max(1)
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
    use crate::attention::{
        AttentionCandidate, AttentionDecision, AttentionPolicy, AttentionSelectionReadinessSummary,
        ThresholdAttentionPolicy,
    };
    use crate::router::{
        RouteBudgetReadinessSummary, RouteLayer, RouteLayerCounts, RoutingContext,
        RoutingDecisionSummary,
    };
    use crate::transformer::{
        TransformerAttentionKind, TransformerLayerBudget, TransformerPlanDigest,
        TransformerPlanningPressureSummary, TransformerPlanningReadinessStage,
        TransformerPlanningReadinessSummary,
    };

    #[test]
    fn disabled_budget_keeps_all_tokens_dense() {
        let input = FhtDkeInput::new(100, 20, RuntimeMetadata::default());

        let budget = DeterministicFhtDkeBudgeter::default().budget(&input);

        assert!(!budget.enabled);
        assert_eq!(budget.dense_tokens, 120);
        assert_eq!(budget.routed_tokens, 0);
        assert_eq!(budget.route_pressure, 0.0);
        assert_eq!(budget.dense_fraction(), 1.0);
        assert_eq!(budget.routed_fraction(), 0.0);
        assert_eq!(budget.kv_exchange_blocks(), 0);
        assert!(!budget.has_kv_exchange());
        assert!(budget.token_split_is_valid());

        let summary = budget.budget_summary();
        assert_eq!(summary.enabled, budget.enabled);
        assert_eq!(summary.total_tokens, 120);
        assert_eq!(summary.dense_tokens, 120);
        assert_eq!(summary.routed_tokens, 0);
        assert_eq!(summary.dense_fraction, 1.0);
        assert_eq!(summary.routed_fraction, 0.0);
        assert_eq!(summary.kv_exchange_blocks, 0);
        assert!(!summary.has_kv_exchange);
        assert!(summary.token_split_is_valid);
        assert_eq!(
            summary.attention_threshold,
            RouteBudget::default().threshold
        );
        assert_eq!(summary.route_pressure, 0.0);
        assert!(!summary.has_route_pressure());
        assert_eq!(summary.route_pressure_signal_component_count(), 0);
        assert!(!summary.route_pressure_is_high());
        assert_eq!(summary.high_route_pressure_signal_component_count(), 0);
        assert!(!summary.has_routed_work());
        assert_eq!(summary.routed_work_signal_component_count(), 0);
        assert!(summary.dense_tokens_dominate());
        assert!(summary.kv_exchange_is_symmetric());
        assert!(summary.kv_exchange_blocks_match_parts());
        assert_eq!(summary.kv_exchange_block_sum_drift(), 0);
        assert_eq!(summary.kv_exchange_block_sum_drift_component_count(), 0);
        assert!(summary.kv_exchange_flag_matches_blocks());
        assert_eq!(summary.kv_exchange_flag_drift_component_count(), 0);
        assert_eq!(summary.kv_exchange_signal_component_count(), 0);
        assert_eq!(summary.kv_exchange_asymmetry_signal_component_count(), 0);
        assert_eq!(summary.token_split_invalid_component_count(), 0);
        assert_eq!(summary.empty_budget_blocker_component_count(), 0);
        assert_eq!(summary.budget_shape_problem_component_count(), 0);
        assert!(!summary.has_budget_shape_problem_components());
        assert_eq!(summary.budget_pressure_signal_component_count(), 0);
        assert_eq!(summary.fht_dke_budget_commit_signal_component_count(), 0);
        assert!(!summary.has_fht_dke_budget_commit_signals());
        assert_eq!(summary.fht_dke_budget_commit_blocker_component_count(), 0);
        assert!(!summary.has_fht_dke_budget_commit_blockers());
        assert!(summary.budget_shape_accounting_is_consistent());
        assert!(summary.fht_dke_budget_commit_accounting_is_consistent());
        assert!(summary.budget_shape_is_clean());
        assert!(summary.can_use_fht_dke_budget());
        assert!(summary.fht_dke_budget_commit_is_clean());
        assert!(summary.can_commit_fht_dke_budget());
        assert_eq!(summary.routed_tokens_per_kv_exchange_block(), None);
    }

    #[test]
    fn enabled_budget_splits_dense_and_routed_tokens() {
        let runtime = RuntimeMetadata::new("local", "tok", 4096, 2048)
            .with_kv_exchange(true, true)
            .with_kv_limits(8, 4);
        let switches = ExperimentSwitches {
            enable_fht_dke: true,
            ..ExperimentSwitches::default()
        };
        let input = FhtDkeInput::new(1800, 200, runtime)
            .with_experiments(switches)
            .with_route_budget(RouteBudget {
                threshold: 0.50,
                attention_tokens: 32,
                fast_tokens: 32,
                attention_fraction: 0.50,
            });

        let budget = DeterministicFhtDkeBudgeter::default().budget(&input);

        assert!(budget.enabled);
        assert_eq!(budget.total_tokens, 2000);
        assert!(budget.dense_tokens > 0);
        assert!(budget.routed_tokens > budget.dense_tokens);
        assert!(budget.kv_import_blocks > 0);
        assert!(budget.kv_export_blocks > 0);
        assert_eq!(budget.route_pressure, 0.50);
        assert!(budget.dense_fraction() > 0.0);
        assert!(budget.routed_fraction() > budget.dense_fraction());
        assert_eq!(
            budget.kv_exchange_blocks(),
            budget.kv_import_blocks + budget.kv_export_blocks
        );
        assert!(budget.has_kv_exchange());
        assert!(budget.token_split_is_valid());

        let summary = budget.budget_summary();
        assert!(summary.enabled);
        assert_eq!(summary.total_tokens, budget.total_tokens);
        assert_eq!(summary.dense_tokens, budget.dense_tokens);
        assert_eq!(summary.routed_tokens, budget.routed_tokens);
        assert_eq!(summary.kv_import_blocks, budget.kv_import_blocks);
        assert_eq!(summary.kv_export_blocks, budget.kv_export_blocks);
        assert_eq!(summary.kv_exchange_blocks, budget.kv_exchange_blocks());
        assert!(summary.has_kv_exchange);
        assert!(summary.token_split_is_valid);
        assert_eq!(summary.attention_threshold, 0.50);
        assert_eq!(summary.route_pressure, 0.50);
        assert!((summary.dense_fraction - budget.dense_fraction()).abs() < 0.0001);
        assert!((summary.routed_fraction - budget.routed_fraction()).abs() < 0.0001);
        assert!(summary.has_route_pressure());
        assert_eq!(summary.route_pressure_signal_component_count(), 1);
        assert!(!summary.route_pressure_is_high());
        assert_eq!(summary.high_route_pressure_signal_component_count(), 0);
        assert!(summary.has_routed_work());
        assert_eq!(summary.routed_work_signal_component_count(), 1);
        assert!(!summary.dense_tokens_dominate());
        assert!(!summary.kv_exchange_is_symmetric());
        assert!(summary.kv_exchange_blocks_match_parts());
        assert_eq!(summary.kv_exchange_block_sum_drift(), 0);
        assert_eq!(summary.kv_exchange_block_sum_drift_component_count(), 0);
        assert!(summary.kv_exchange_flag_matches_blocks());
        assert_eq!(summary.kv_exchange_flag_drift_component_count(), 0);
        assert_eq!(summary.kv_exchange_signal_component_count(), 1);
        assert_eq!(summary.kv_exchange_asymmetry_signal_component_count(), 1);
        assert_eq!(summary.token_split_invalid_component_count(), 0);
        assert_eq!(summary.empty_budget_blocker_component_count(), 0);
        assert_eq!(summary.budget_shape_problem_component_count(), 0);
        assert!(!summary.has_budget_shape_problem_components());
        assert_eq!(summary.budget_pressure_signal_component_count(), 4);
        assert_eq!(summary.fht_dke_budget_commit_signal_component_count(), 4);
        assert!(summary.has_fht_dke_budget_commit_signals());
        assert_eq!(summary.fht_dke_budget_commit_blocker_component_count(), 0);
        assert!(!summary.has_fht_dke_budget_commit_blockers());
        assert!(summary.budget_shape_accounting_is_consistent());
        assert!(summary.fht_dke_budget_commit_accounting_is_consistent());
        assert!(summary.budget_shape_is_clean());
        assert!(summary.can_use_fht_dke_budget());
        assert!(summary.fht_dke_budget_commit_is_clean());
        assert!(summary.can_commit_fht_dke_budget());
        assert!(summary.routed_tokens_per_kv_exchange_block().is_some());
    }

    #[test]
    fn fht_dke_planning_readiness_confirms_pressure_budget_boundary() {
        let runtime = RuntimeMetadata::new("local", "tok", 4096, 2048)
            .with_kv_exchange(true, true)
            .with_kv_limits(8, 4);
        let switches = ExperimentSwitches {
            enable_fht_dke: true,
            ..ExperimentSwitches::default()
        };
        let transformer_planning = transformer_planning_readiness();
        let budget = DeterministicFhtDkeBudgeter::default()
            .budget(
                &FhtDkeInput::new(1800, 200, runtime)
                    .with_experiments(switches)
                    .with_route_budget(transformer_planning.route_budget.route_budget),
            )
            .budget_summary();
        let readiness = FhtDkePlanningReadinessSummary::new(transformer_planning, budget);

        assert_eq!(
            FhtDkePlanningReadinessSummary::stage_order(),
            [
                FhtDkePlanningReadinessStage::TransformerPlanning,
                FhtDkePlanningReadinessStage::BudgetCommit,
                FhtDkePlanningReadinessStage::PressureBudgetBoundary,
            ]
        );
        assert!(readiness.transformer_planning_ready());
        assert!(readiness.fht_dke_budget_ready());
        assert!(readiness.pressure_budget_boundary_ready());
        assert!(readiness.route_pressure_matches_budget());
        assert!(readiness.attention_threshold_matches_budget());
        assert!(readiness.pressure_budget_boundary_matches());
        assert_eq!(
            readiness.pressure_budget_boundary_drift_component_count(),
            0
        );
        assert_eq!(readiness.first_unready_stage(), None);
        assert_eq!(readiness.first_blocking_stage(), None);
        assert_eq!(readiness.transformer_planning_signal_component_count, 45);
        assert_eq!(readiness.fht_dke_budget_signal_component_count, 5);
        assert_eq!(readiness.pressure_budget_boundary_signal_component_count, 1);
        assert_eq!(
            readiness.stage_signal_component_count(FhtDkePlanningReadinessStage::BudgetCommit),
            readiness.fht_dke_budget_signal_component_count
        );
        assert_eq!(
            readiness.stage_blocker_component_count(
                FhtDkePlanningReadinessStage::PressureBudgetBoundary
            ),
            readiness.pressure_budget_boundary_blocker_component_count
        );
        assert_eq!(
            readiness.fht_dke_planning_readiness_signal_component_count(),
            51
        );
        assert!(readiness.has_fht_dke_planning_readiness_signals());
        assert_eq!(
            readiness.fht_dke_planning_readiness_blocker_component_count(),
            0
        );
        assert!(!readiness.has_fht_dke_planning_readiness_blockers());
        assert!(readiness.fht_dke_planning_readiness_accounting_is_consistent());
        assert!(readiness.fht_dke_planning_readiness_is_clean());
        assert!(readiness.can_commit_fht_dke_planning_readiness());
        let commit = readiness.commit_summary();
        assert_eq!(
            commit.action,
            FhtDkePlanningCommitAction::CommitFhtDkePlanning
        );
        assert!(commit.action_can_commit());
        assert!(!commit.action_should_wait_for_fht_dke_planning());
        assert!(!commit.action_should_repair_fht_dke_planning());
        assert_eq!(commit.committed_fht_dke_budget, Some(budget));
        assert!(commit.can_commit_fht_dke_planning());
        assert!(!commit.should_wait_for_fht_dke_planning());
        assert!(!commit.should_repair_fht_dke_planning());
        assert!(commit.can_use_committed_fht_dke_budget());
        assert_eq!(commit.first_unready_stage, None);
        assert_eq!(commit.first_blocking_stage, None);
        assert_eq!(commit.total_signal_component_count, 51);
        assert_eq!(commit.total_blocker_component_count, 0);
        assert!(commit.component_accounting_consistent);
        assert!(commit.commit_decision_accounting_is_consistent());
    }

    #[test]
    fn fht_dke_planning_commit_summary_exposes_runtime_planning_parts() {
        let runtime = RuntimeMetadata::new("local", "tok", 4096, 2048)
            .with_kv_exchange(true, true)
            .with_kv_limits(8, 4);
        let switches = ExperimentSwitches {
            enable_fht_dke: true,
            ..ExperimentSwitches::default()
        };
        let transformer_planning = transformer_planning_readiness();
        let budget = DeterministicFhtDkeBudgeter::default()
            .budget(
                &FhtDkeInput::new(1800, 200, runtime.clone())
                    .with_experiments(switches)
                    .with_route_budget(transformer_planning.route_budget.route_budget),
            )
            .budget_summary();
        let ready =
            FhtDkePlanningReadinessSummary::new(transformer_planning, budget).commit_summary();

        assert!(ready.can_commit_fht_dke_planning());
        assert_eq!(
            ready.committed_transformer_planning,
            Some(transformer_planning)
        );
        assert_eq!(ready.committed_fht_dke_budget, Some(budget));
        assert!(ready.can_use_committed_transformer_planning());
        assert!(ready.can_use_committed_fht_dke_budget());
        assert!(ready.can_use_committed_runtime_planning_parts());
        assert!(ready.commit_decision_accounting_is_consistent());

        let stale_route_budget = RouteBudget {
            threshold: 0.42,
            attention_tokens: 2,
            fast_tokens: 6,
            attention_fraction: 0.25,
        };
        let stale_budget = DeterministicFhtDkeBudgeter::default()
            .budget(
                &FhtDkeInput::new(1800, 200, runtime)
                    .with_experiments(switches)
                    .with_route_budget(stale_route_budget),
            )
            .budget_summary();
        let repair = FhtDkePlanningReadinessSummary::new(transformer_planning, stale_budget)
            .commit_summary();

        assert!(!repair.can_commit_fht_dke_planning());
        assert_eq!(repair.committed_transformer_planning, None);
        assert_eq!(repair.committed_fht_dke_budget, None);
        assert!(!repair.can_use_committed_transformer_planning());
        assert!(!repair.can_use_committed_fht_dke_budget());
        assert!(!repair.can_use_committed_runtime_planning_parts());
        assert!(repair.should_repair_fht_dke_planning());
        assert!(repair.commit_decision_accounting_is_consistent());
    }

    #[test]
    fn fht_dke_planning_readiness_blocks_stale_budget_pressure() {
        let runtime = RuntimeMetadata::new("local", "tok", 4096, 2048)
            .with_kv_exchange(true, true)
            .with_kv_limits(8, 4);
        let switches = ExperimentSwitches {
            enable_fht_dke: true,
            ..ExperimentSwitches::default()
        };
        let transformer_planning = transformer_planning_readiness();
        let stale_route_budget = RouteBudget {
            threshold: 0.42,
            attention_tokens: 2,
            fast_tokens: 6,
            attention_fraction: 0.25,
        };
        let budget = DeterministicFhtDkeBudgeter::default()
            .budget(
                &FhtDkeInput::new(1800, 200, runtime)
                    .with_experiments(switches)
                    .with_route_budget(stale_route_budget),
            )
            .budget_summary();
        let readiness = FhtDkePlanningReadinessSummary::new(transformer_planning, budget);

        assert!(readiness.transformer_planning_ready());
        assert!(readiness.fht_dke_budget_ready());
        assert!(!readiness.pressure_budget_boundary_ready());
        assert!(!readiness.route_pressure_matches_budget());
        assert!(!readiness.attention_threshold_matches_budget());
        assert!(!readiness.pressure_budget_boundary_matches());
        assert_eq!(
            readiness.pressure_budget_boundary_drift_component_count(),
            2
        );
        assert_eq!(
            readiness.first_unready_stage(),
            Some(FhtDkePlanningReadinessStage::PressureBudgetBoundary)
        );
        assert_eq!(
            readiness.first_blocking_stage(),
            Some(FhtDkePlanningReadinessStage::PressureBudgetBoundary)
        );
        assert_eq!(readiness.transformer_planning_signal_component_count, 45);
        assert_eq!(readiness.fht_dke_budget_signal_component_count, 4);
        assert_eq!(readiness.pressure_budget_boundary_signal_component_count, 0);
        assert_eq!(readiness.transformer_planning_blocker_component_count, 0);
        assert_eq!(readiness.fht_dke_budget_blocker_component_count, 0);
        assert_eq!(
            readiness.pressure_budget_boundary_blocker_component_count,
            2
        );
        assert_eq!(
            readiness.fht_dke_planning_readiness_signal_component_count(),
            49
        );
        assert_eq!(
            readiness.fht_dke_planning_readiness_blocker_component_count(),
            2
        );
        assert!(readiness.has_fht_dke_planning_readiness_blockers());
        assert!(readiness.fht_dke_planning_readiness_accounting_is_consistent());
        assert!(!readiness.fht_dke_planning_readiness_is_clean());
        assert!(!readiness.can_commit_fht_dke_planning_readiness());
        let commit = readiness.commit_summary();
        assert_eq!(
            commit.action,
            FhtDkePlanningCommitAction::RepairFhtDkePlanning
        );
        assert!(!commit.action_can_commit());
        assert!(!commit.action_should_wait_for_fht_dke_planning());
        assert!(commit.action_should_repair_fht_dke_planning());
        assert_eq!(commit.committed_fht_dke_budget, None);
        assert!(!commit.can_commit_fht_dke_planning());
        assert!(!commit.should_wait_for_fht_dke_planning());
        assert!(commit.should_repair_fht_dke_planning());
        assert!(!commit.can_use_committed_fht_dke_budget());
        assert_eq!(
            commit.first_unready_stage,
            Some(FhtDkePlanningReadinessStage::PressureBudgetBoundary)
        );
        assert_eq!(
            commit.first_blocking_stage,
            Some(FhtDkePlanningReadinessStage::PressureBudgetBoundary)
        );
        assert_eq!(commit.total_signal_component_count, 49);
        assert_eq!(commit.total_blocker_component_count, 2);
        assert!(commit.component_accounting_consistent);
        assert!(commit.commit_decision_accounting_is_consistent());
    }

    #[test]
    fn fht_dke_planning_readiness_blocks_stale_adaptive_attention_selection_boundary() {
        let route_summary = RoutingDecisionSummary {
            threshold: 0.50,
            token_count: 5,
            layer_counts: RouteLayerCounts {
                fast_projection: 1,
                local_window: 1,
                global: 2,
                fusion: 1,
            },
            attention_fraction: 0.80,
            average_score: 0.74,
            min_score: 0.20,
            max_score: 0.96,
            above_threshold_tokens: 4,
            below_threshold_tokens: 1,
        };
        let route_budget = route_summary.route_budget();
        let route_readiness = RouteBudgetReadinessSummary::new(route_summary, route_budget);
        let candidates = [
            AttentionCandidate::new("fast", 0, 0.99, 0.10, RouteLayer::FastProjection),
            AttentionCandidate::new("keep-local", 1, 0.74, 0.40, RouteLayer::LocalWindow),
            AttentionCandidate::new("drop-low", 2, 0.49, 0.90, RouteLayer::Global),
            AttentionCandidate::new("keep-global", 3, 0.96, 0.80, RouteLayer::Global),
            AttentionCandidate::new("over-budget", 4, 0.60, 0.50, RouteLayer::Fusion),
        ];
        let switches = ExperimentSwitches {
            enable_fht_dke: true,
            max_attention_tokens: 2,
            ..ExperimentSwitches::default()
        };
        let decision = ThresholdAttentionPolicy::new(0.50).select(
            &candidates,
            RoutingContext::default(),
            switches,
        );
        let attention_readiness = AttentionSelectionReadinessSummary::from_decision(
            AttentionCandidate::batch_summary(&candidates[..4]),
            &decision,
        );
        let transformer_summary = TransformerPlanDigest::new(
            Some("stale-adaptive-attention-selection"),
            vec![
                TransformerLayerBudget::new(0, TransformerAttentionKind::Global, 0.88, 4096),
                TransformerLayerBudget::new(1, TransformerAttentionKind::LocalWindow, 0.60, 2048),
                TransformerLayerBudget::new(2, TransformerAttentionKind::Fusion, 0.42, 1024),
                TransformerLayerBudget::new(3, TransformerAttentionKind::LocalWindow, 0.55, 2048),
            ],
        )
        .plan_summary();
        let pressure = TransformerPlanningPressureSummary::from_parts(
            route_budget,
            decision.decision_summary(),
            transformer_summary,
        );
        let transformer_planning = TransformerPlanningReadinessSummary::new(
            route_readiness,
            attention_readiness,
            pressure,
        );
        let budget = DeterministicFhtDkeBudgeter::default()
            .budget(
                &FhtDkeInput::new(
                    1800,
                    200,
                    RuntimeMetadata::new("local", "tok", 4096, 2048)
                        .with_kv_exchange(true, true)
                        .with_kv_limits(8, 4),
                )
                .with_experiments(switches)
                .with_route_budget(route_budget),
            )
            .budget_summary();
        let readiness = FhtDkePlanningReadinessSummary::new(transformer_planning, budget);

        assert_eq!(
            decision.selected_tokens(),
            vec!["keep-local", "keep-global"]
        );
        assert!(decision.hit_selection_cap());
        assert!(attention_readiness.candidate_batch_ready());
        assert!(attention_readiness.decision_ready());
        assert!(!attention_readiness.selection_boundary_ready());
        assert!(!attention_readiness.can_commit_attention_selection_readiness());
        assert!(transformer_planning.route_budget_ready());
        assert!(!transformer_planning.attention_selection_ready());
        assert!(transformer_planning.planning_pressure_ready());
        assert_eq!(
            transformer_planning.first_unready_stage(),
            Some(TransformerPlanningReadinessStage::AttentionSelection)
        );
        assert_eq!(
            transformer_planning.first_blocking_stage(),
            Some(TransformerPlanningReadinessStage::AttentionSelection)
        );
        assert!(readiness.fht_dke_budget_ready());
        assert!(readiness.pressure_budget_boundary_ready());
        assert!(!readiness.transformer_planning_ready());
        assert_eq!(
            readiness.first_unready_stage(),
            Some(FhtDkePlanningReadinessStage::TransformerPlanning)
        );
        assert_eq!(
            readiness.first_blocking_stage(),
            Some(FhtDkePlanningReadinessStage::TransformerPlanning)
        );
        assert_eq!(readiness.fht_dke_budget_blocker_component_count, 0);
        assert_eq!(
            readiness.pressure_budget_boundary_blocker_component_count,
            0
        );
        assert_eq!(
            readiness
                .stage_blocker_component_count(FhtDkePlanningReadinessStage::TransformerPlanning),
            transformer_planning.transformer_planning_readiness_blocker_component_count()
        );
        assert!(readiness.fht_dke_planning_readiness_accounting_is_consistent());
        assert!(!readiness.fht_dke_planning_readiness_is_clean());
        assert!(!readiness.can_commit_fht_dke_planning_readiness());
        let commit = readiness.commit_summary();
        assert_eq!(
            commit.action,
            FhtDkePlanningCommitAction::RepairFhtDkePlanning
        );
        assert_eq!(commit.committed_fht_dke_budget, None);
        assert!(!commit.can_use_committed_fht_dke_budget());
        assert_eq!(
            commit.first_unready_stage,
            Some(FhtDkePlanningReadinessStage::TransformerPlanning)
        );
        assert_eq!(
            commit.first_blocking_stage,
            Some(FhtDkePlanningReadinessStage::TransformerPlanning)
        );
        assert!(commit.component_accounting_consistent);
        assert!(commit.commit_decision_accounting_is_consistent());
    }

    #[test]
    fn budget_summary_counts_shape_accounting_drift() {
        let summary = FhtDkeBudgetSummary {
            enabled: true,
            total_tokens: 12,
            dense_tokens: 4,
            routed_tokens: 9,
            dense_fraction: 4.0 / 12.0,
            routed_fraction: 9.0 / 12.0,
            kv_import_blocks: 1,
            kv_export_blocks: 2,
            kv_exchange_blocks: 4,
            has_kv_exchange: false,
            token_split_is_valid: false,
            attention_threshold: 0.55,
            route_pressure: 0.80,
        };

        assert!(summary.has_route_pressure());
        assert_eq!(summary.route_pressure_signal_component_count(), 1);
        assert!(summary.route_pressure_is_high());
        assert_eq!(summary.high_route_pressure_signal_component_count(), 1);
        assert!(summary.has_routed_work());
        assert_eq!(summary.routed_work_signal_component_count(), 1);
        assert!(!summary.kv_exchange_is_symmetric());
        assert!(!summary.kv_exchange_blocks_match_parts());
        assert_eq!(summary.kv_exchange_block_sum_drift(), 1);
        assert_eq!(summary.kv_exchange_block_sum_drift_component_count(), 1);
        assert!(!summary.kv_exchange_flag_matches_blocks());
        assert_eq!(summary.kv_exchange_flag_drift_component_count(), 1);
        assert_eq!(summary.kv_exchange_signal_component_count(), 0);
        assert_eq!(summary.kv_exchange_asymmetry_signal_component_count(), 1);
        assert_eq!(summary.token_split_invalid_component_count(), 1);
        assert!(summary.attention_threshold_is_valid());
        assert_eq!(
            summary.attention_threshold_shape_problem_component_count(),
            0
        );
        assert!(summary.route_pressure_shape_is_valid());
        assert_eq!(summary.route_pressure_shape_problem_component_count(), 0);
        assert_eq!(summary.empty_budget_blocker_component_count(), 0);
        assert_eq!(summary.budget_shape_problem_component_count(), 3);
        assert!(summary.has_budget_shape_problem_components());
        assert_eq!(summary.budget_pressure_signal_component_count(), 4);
        assert_eq!(summary.fht_dke_budget_commit_signal_component_count(), 4);
        assert!(summary.has_fht_dke_budget_commit_signals());
        assert_eq!(summary.fht_dke_budget_commit_blocker_component_count(), 3);
        assert!(summary.has_fht_dke_budget_commit_blockers());
        assert!(summary.budget_shape_accounting_is_consistent());
        assert!(summary.fht_dke_budget_commit_accounting_is_consistent());
        assert!(!summary.budget_shape_is_clean());
        assert!(!summary.can_use_fht_dke_budget());
        assert!(!summary.fht_dke_budget_commit_is_clean());
        assert!(!summary.can_commit_fht_dke_budget());
    }

    #[test]
    fn budget_summary_blocks_invalid_threshold_and_route_pressure_shape() {
        let summary = FhtDkeBudgetSummary {
            enabled: true,
            total_tokens: 16,
            dense_tokens: 8,
            routed_tokens: 8,
            dense_fraction: 0.5,
            routed_fraction: 0.5,
            kv_import_blocks: 0,
            kv_export_blocks: 0,
            kv_exchange_blocks: 0,
            has_kv_exchange: false,
            token_split_is_valid: true,
            attention_threshold: 1.25,
            route_pressure: f32::NAN,
        };

        assert!(!summary.attention_threshold_is_valid());
        assert_eq!(
            summary.attention_threshold_shape_problem_component_count(),
            1
        );
        assert!(!summary.route_pressure_shape_is_valid());
        assert_eq!(summary.route_pressure_shape_problem_component_count(), 1);
        assert_eq!(summary.token_split_invalid_component_count(), 0);
        assert_eq!(summary.kv_exchange_block_sum_drift_component_count(), 0);
        assert_eq!(summary.kv_exchange_flag_drift_component_count(), 0);
        assert_eq!(summary.budget_shape_problem_component_count(), 2);
        assert!(summary.has_budget_shape_problem_components());
        assert_eq!(summary.fht_dke_budget_commit_blocker_component_count(), 2);
        assert!(summary.has_fht_dke_budget_commit_blockers());
        assert!(summary.budget_shape_accounting_is_consistent());
        assert!(summary.fht_dke_budget_commit_accounting_is_consistent());
        assert!(!summary.budget_shape_is_clean());
        assert!(!summary.can_use_fht_dke_budget());
        assert!(!summary.fht_dke_budget_commit_is_clean());
        assert!(!summary.can_commit_fht_dke_budget());
    }

    #[test]
    fn budget_summary_exposes_attention_threshold_admission_boundary() {
        let accepted = FhtDkeBudgetSummary {
            enabled: true,
            total_tokens: 16,
            dense_tokens: 8,
            routed_tokens: 8,
            dense_fraction: 0.5,
            routed_fraction: 0.5,
            kv_import_blocks: 0,
            kv_export_blocks: 0,
            kv_exchange_blocks: 0,
            has_kv_exchange: false,
            token_split_is_valid: true,
            attention_threshold: 0.72,
            route_pressure: 0.50,
        };
        let rejected = FhtDkeBudgetSummary {
            attention_threshold: f32::INFINITY,
            ..accepted
        };

        assert!(accepted.attention_threshold_is_valid());
        assert_eq!(
            accepted.attention_threshold_admission_signal_component_count(),
            1
        );
        assert!(accepted.has_attention_threshold_admission_signals());
        assert_eq!(
            accepted.attention_threshold_admission_blocker_component_count(),
            0
        );
        assert!(!accepted.has_attention_threshold_admission_blockers());
        assert!(accepted.attention_threshold_admission_accounting_is_consistent());
        assert!(accepted.attention_threshold_admission_is_clean());
        assert!(accepted.can_admit_attention_threshold());
        assert_eq!(
            accepted.attention_threshold_shape_problem_component_count(),
            accepted.attention_threshold_admission_blocker_component_count()
        );
        assert_eq!(accepted.budget_shape_problem_component_count(), 0);

        assert!(!rejected.attention_threshold_is_valid());
        assert_eq!(
            rejected.attention_threshold_admission_signal_component_count(),
            0
        );
        assert!(!rejected.has_attention_threshold_admission_signals());
        assert_eq!(
            rejected.attention_threshold_admission_blocker_component_count(),
            1
        );
        assert!(rejected.has_attention_threshold_admission_blockers());
        assert!(rejected.attention_threshold_admission_accounting_is_consistent());
        assert!(!rejected.attention_threshold_admission_is_clean());
        assert!(!rejected.can_admit_attention_threshold());
        assert_eq!(
            rejected.attention_threshold_shape_problem_component_count(),
            rejected.attention_threshold_admission_blocker_component_count()
        );
        assert_eq!(rejected.budget_shape_problem_component_count(), 1);
        assert_eq!(rejected.fht_dke_budget_commit_blocker_component_count(), 1);
        assert!(!rejected.can_commit_fht_dke_budget());
    }

    #[test]
    fn route_pressure_increases_dense_budget() {
        let runtime =
            RuntimeMetadata::new("local", "tok", 4096, 2048).with_kv_exchange(false, false);
        let switches = ExperimentSwitches {
            enable_fht_dke: true,
            ..ExperimentSwitches::default()
        };
        let low_pressure = FhtDkeInput::new(1000, 200, runtime.clone())
            .with_experiments(switches)
            .with_route_budget(RouteBudget {
                threshold: 0.65,
                attention_tokens: 1,
                fast_tokens: 9,
                attention_fraction: 0.10,
            });
        let high_pressure = FhtDkeInput::new(1000, 200, runtime)
            .with_experiments(switches)
            .with_route_budget(RouteBudget {
                threshold: 0.45,
                attention_tokens: 9,
                fast_tokens: 1,
                attention_fraction: 0.90,
            });
        let budgeter = DeterministicFhtDkeBudgeter::default();

        let low = budgeter.budget(&low_pressure);
        let high = budgeter.budget(&high_pressure);

        assert!(high.dense_tokens > low.dense_tokens);
        assert!(high.routed_tokens < low.routed_tokens);
        assert_eq!(low.route_pressure, 0.10);
        assert_eq!(high.route_pressure, 0.90);
        assert!(high.dense_fraction() > low.dense_fraction());
        assert!(high.routed_fraction() < low.routed_fraction());
    }

    #[test]
    fn route_pressure_reduces_routed_kv_exchange_demand() {
        let runtime = RuntimeMetadata::new("local", "tok", 8192, 2048)
            .with_kv_exchange(true, true)
            .with_kv_limits(128, 128);
        let switches = ExperimentSwitches {
            enable_fht_dke: true,
            ..ExperimentSwitches::default()
        };
        let low_pressure = FhtDkeInput::new(3584, 512, runtime.clone())
            .with_experiments(switches)
            .with_route_budget(RouteBudget {
                threshold: 0.65,
                attention_tokens: 1,
                fast_tokens: 9,
                attention_fraction: 0.10,
            });
        let high_pressure = FhtDkeInput::new(3584, 512, runtime)
            .with_experiments(switches)
            .with_route_budget(RouteBudget {
                threshold: 0.45,
                attention_tokens: 9,
                fast_tokens: 1,
                attention_fraction: 0.90,
            });
        let budgeter = DeterministicFhtDkeBudgeter::new(0.10, 0.60, 128);

        let low = budgeter.budget(&low_pressure);
        let high = budgeter.budget(&high_pressure);
        let low_summary = low.budget_summary();
        let high_summary = high.budget_summary();

        assert_eq!(low.total_tokens, high.total_tokens);
        assert_eq!(low.total_tokens, 4096);
        assert!(high.dense_tokens > low.dense_tokens);
        assert!(high.routed_tokens < low.routed_tokens);
        assert!(high.kv_import_blocks < low.kv_import_blocks);
        assert!(high.kv_export_blocks < low.kv_export_blocks);
        assert!(high_summary.kv_exchange_blocks < low_summary.kv_exchange_blocks);
        assert!(low_summary.has_kv_exchange);
        assert!(high_summary.has_kv_exchange);
        assert!(low_summary.token_split_is_valid);
        assert!(high_summary.token_split_is_valid);
        assert_eq!(low_summary.route_pressure, 0.10);
        assert_eq!(high_summary.route_pressure, 0.90);
        assert!(low_summary.has_route_pressure());
        assert!(high_summary.has_route_pressure());
        assert!(!low_summary.route_pressure_is_high());
        assert!(high_summary.route_pressure_is_high());
        assert_eq!(low_summary.high_route_pressure_signal_component_count(), 0);
        assert_eq!(high_summary.high_route_pressure_signal_component_count(), 1);
        assert!(low_summary.has_routed_work());
        assert!(high_summary.has_routed_work());
        assert_eq!(low_summary.budget_shape_problem_component_count(), 0);
        assert_eq!(high_summary.budget_shape_problem_component_count(), 0);
        assert_eq!(low_summary.budget_pressure_signal_component_count(), 3);
        assert_eq!(high_summary.budget_pressure_signal_component_count(), 4);
        assert!(low_summary.budget_shape_accounting_is_consistent());
        assert!(high_summary.budget_shape_accounting_is_consistent());
        assert!(low_summary.budget_shape_is_clean());
        assert!(high_summary.budget_shape_is_clean());
        assert!(low_summary.can_use_fht_dke_budget());
        assert!(high_summary.can_use_fht_dke_budget());
        assert!(
            high_summary.routed_tokens_per_kv_exchange_block().unwrap()
                <= low_summary.routed_tokens_per_kv_exchange_block().unwrap()
        );
    }

    #[test]
    fn budgeter_clamps_total_tokens_to_known_runtime_context_window() {
        let runtime = RuntimeMetadata::new("local", "tok", 1024, 2048)
            .with_kv_exchange(true, true)
            .with_kv_limits(3, 2);
        let input = FhtDkeInput::new(1000, 1000, runtime)
            .with_experiments(ExperimentSwitches {
                enable_fht_dke: true,
                ..ExperimentSwitches::default()
            })
            .with_route_budget(RouteBudget {
                threshold: 0.50,
                attention_tokens: 5,
                fast_tokens: 5,
                attention_fraction: 0.50,
            });
        let budgeter = DeterministicFhtDkeBudgeter::new(0.10, 0.60, 128);

        let budget = budgeter.budget(&input);
        let summary = budget.budget_summary();

        assert_eq!(input.requested_tokens(), 2000);
        assert_eq!(budget.total_tokens, 1024);
        assert_eq!(
            budget.dense_tokens.saturating_add(budget.routed_tokens),
            1024
        );
        assert_eq!(budget.kv_import_blocks, 3);
        assert_eq!(budget.kv_export_blocks, 2);
        assert_eq!(summary.kv_exchange_blocks, 5);
        assert!(summary.token_split_is_valid);
        assert_eq!(summary.attention_threshold, 0.50);
        assert_eq!(summary.route_pressure, 0.50);
        assert!(summary.has_kv_exchange);
        assert!(summary.kv_exchange_is_symmetric() == false);
        assert_eq!(summary.budget_shape_problem_component_count(), 0);
        assert_eq!(summary.fht_dke_budget_commit_signal_component_count(), 4);
        assert_eq!(summary.fht_dke_budget_commit_blocker_component_count(), 0);
        assert!(summary.fht_dke_budget_commit_accounting_is_consistent());
        assert!(summary.can_commit_fht_dke_budget());
    }

    #[test]
    fn empty_budget_summary_is_clean_but_not_usable() {
        let summary = FhtDkeBudget::disabled(0, RouteBudget::default().threshold).budget_summary();

        assert_eq!(summary.total_tokens, 0);
        assert_eq!(summary.empty_budget_blocker_component_count(), 1);
        assert!(!summary.has_budget_shape_problem_components());
        assert!(summary.budget_shape_accounting_is_consistent());
        assert_eq!(summary.fht_dke_budget_commit_signal_component_count(), 0);
        assert!(!summary.has_fht_dke_budget_commit_signals());
        assert_eq!(summary.fht_dke_budget_commit_blocker_component_count(), 1);
        assert!(summary.has_fht_dke_budget_commit_blockers());
        assert!(summary.fht_dke_budget_commit_accounting_is_consistent());
        assert!(summary.budget_shape_is_clean());
        assert!(!summary.can_use_fht_dke_budget());
        assert!(!summary.fht_dke_budget_commit_is_clean());
        assert!(!summary.can_commit_fht_dke_budget());
    }

    fn transformer_planning_readiness() -> TransformerPlanningReadinessSummary {
        let route_summary = RoutingDecisionSummary {
            threshold: 0.50,
            token_count: 8,
            layer_counts: RouteLayerCounts {
                fast_projection: 2,
                local_window: 2,
                global: 2,
                fusion: 2,
            },
            attention_fraction: 0.75,
            average_score: 0.60,
            min_score: 0.10,
            max_score: 0.90,
            above_threshold_tokens: 6,
            below_threshold_tokens: 2,
        };
        let route_budget = route_summary.route_budget();
        let route_readiness = RouteBudgetReadinessSummary::new(route_summary, route_budget);
        let candidates = [
            AttentionCandidate::new("local", 0, 0.70, 0.30, RouteLayer::LocalWindow),
            AttentionCandidate::new("global", 1, 0.90, 0.60, RouteLayer::Global),
            AttentionCandidate::new("fast", 2, 0.95, 0.10, RouteLayer::FastProjection),
            AttentionCandidate::new("fusion", 3, 0.80, 0.50, RouteLayer::Fusion),
        ];
        let attention_decision = AttentionDecision {
            threshold: 0.50,
            max_selected: 2,
            selected: candidates[..2].to_vec(),
            rejected: candidates[2..].to_vec(),
        };
        let attention_readiness = AttentionSelectionReadinessSummary::from_decision(
            AttentionCandidate::batch_summary(&candidates),
            &attention_decision,
        );
        let transformer_summary = TransformerPlanDigest::new(
            Some("adapter-pressure-test"),
            vec![
                TransformerLayerBudget::new(0, TransformerAttentionKind::Global, 0.90, 4096),
                TransformerLayerBudget::new(1, TransformerAttentionKind::LocalWindow, 0.60, 1024),
                TransformerLayerBudget::new(2, TransformerAttentionKind::Fusion, 0.40, 512),
                TransformerLayerBudget::new(3, TransformerAttentionKind::LocalWindow, 0.50, 1024),
            ],
        )
        .plan_summary();
        let pressure = TransformerPlanningPressureSummary::from_parts(
            route_budget,
            attention_decision.decision_summary(),
            transformer_summary,
        );

        let readiness = TransformerPlanningReadinessSummary::new(
            route_readiness,
            attention_readiness,
            pressure,
        );

        assert_eq!(readiness.first_unready_stage(), None);
        assert_eq!(readiness.first_blocking_stage(), None);
        assert_eq!(
            readiness
                .stage_signal_component_count(TransformerPlanningReadinessStage::PlanningPressure),
            12
        );
        assert!(readiness.can_commit_transformer_planning_readiness());

        readiness
    }
}
