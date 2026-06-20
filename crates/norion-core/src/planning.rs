use crate::adapter::{
    AdapterExecutionContext, AdapterFallbackReason, AdapterObservation, AdapterSelection,
    AdapterSelectionReport,
};
use crate::engine::{
    InferenceError, InferenceRequest, RuntimeFailureBatchSummary, RuntimeFailureReport,
    RuntimeFailureSummary,
};
use crate::fht_dke::{
    FhtDkeBudget, FhtDkeBudgetSummary, FhtDkeBudgeter, FhtDkeInput, FhtDkePlanningCommitSummary,
    FhtDkePlanningReadinessSummary,
};
use crate::kv::{RuntimeKvImportManifestPlanSummary, RuntimeKvImportPlan};
use crate::manifest::RuntimeManifestDigest;
use crate::router::RouteBudget;
use crate::runtime::RuntimeGenerationBudget;
use crate::transformer::{RuntimeKvExportManifestPlanSummary, RuntimeKvExportPlan};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimePlanningDigest {
    pub generation_budget: RuntimeGenerationBudget,
    pub fht_dke_budget: FhtDkeBudget,
    pub adapter_selection: AdapterSelection,
    pub adapter_selection_report: AdapterSelectionReport,
    pub requested_kv_prefetch_blocks: usize,
    pub runtime_kv_prefetch_blocks: usize,
    pub planned_kv_import_blocks: usize,
    pub planned_kv_export_blocks: usize,
    pub hardware_pressure: f32,
    pub compute_headroom: f32,
    pub max_parallel_chunks: usize,
    pub latency_budget_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimePlanningSummary {
    pub generation_budget: RuntimeGenerationBudget,
    pub context_limited: bool,
    pub backend_max_tokens: usize,
    pub adapter_selection: AdapterSelection,
    pub adapter_fallback_reason: AdapterFallbackReason,
    pub allowed_adapter_count: usize,
    pub observation_count: usize,
    pub matching_observation_count: usize,
    pub matched_observation_fraction: f32,
    pub fht_dke: FhtDkeBudgetSummary,
    pub kv_exchange: RuntimePlanningKvExchange,
    pub kv_clamp: RuntimePlanningKvClampSummary,
    pub requested_kv_prefetch_blocks: usize,
    pub runtime_kv_prefetch_blocks: usize,
    pub hardware_pressure: f32,
    pub compute_headroom: f32,
    pub max_parallel_chunks: usize,
    pub latency_budget_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimePlanningReadinessStage {
    FhtDkePlanning,
    RuntimePreRequest,
    FhtDkeRuntimeBoundary,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimePlanningReadinessSummary {
    pub fht_dke_planning: FhtDkePlanningReadinessSummary,
    pub runtime_planning: RuntimePlanningSummary,
    pub fht_dke_planning_signal_component_count: usize,
    pub runtime_pre_request_signal_component_count: usize,
    pub fht_dke_runtime_boundary_signal_component_count: usize,
    pub fht_dke_planning_blocker_component_count: usize,
    pub runtime_pre_request_blocker_component_count: usize,
    pub fht_dke_runtime_boundary_blocker_component_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimePlanningManifestKvBridgeSummary {
    pub import: RuntimeKvImportManifestPlanSummary,
    pub export: RuntimeKvExportManifestPlanSummary,
    pub planned_import_blocks: usize,
    pub planned_export_blocks: usize,
}

impl RuntimePlanningReadinessSummary {
    pub fn new(
        fht_dke_planning: FhtDkePlanningReadinessSummary,
        runtime_planning: RuntimePlanningSummary,
    ) -> Self {
        Self {
            fht_dke_planning,
            runtime_planning,
            fht_dke_planning_signal_component_count: fht_dke_planning
                .fht_dke_planning_readiness_signal_component_count(),
            runtime_pre_request_signal_component_count: runtime_planning
                .pre_request_gate_signal_component_count(),
            fht_dke_runtime_boundary_signal_component_count: usize::from(
                Self::fht_dke_runtime_boundary_matches_parts(fht_dke_planning, runtime_planning),
            ),
            fht_dke_planning_blocker_component_count: fht_dke_planning
                .fht_dke_planning_readiness_blocker_component_count(),
            runtime_pre_request_blocker_component_count: runtime_planning
                .backend_request_commit_blocker_component_count(),
            fht_dke_runtime_boundary_blocker_component_count:
                Self::fht_dke_runtime_boundary_drift_component_count_parts(
                    fht_dke_planning,
                    runtime_planning,
                ),
        }
    }

    pub fn stage_order() -> [RuntimePlanningReadinessStage; 3] {
        [
            RuntimePlanningReadinessStage::FhtDkePlanning,
            RuntimePlanningReadinessStage::RuntimePreRequest,
            RuntimePlanningReadinessStage::FhtDkeRuntimeBoundary,
        ]
    }

    pub fn fht_dke_planning_ready(self) -> bool {
        self.fht_dke_planning
            .can_commit_fht_dke_planning_readiness()
    }

    pub fn runtime_pre_request_ready(self) -> bool {
        self.runtime_planning.can_commit_backend_request()
    }

    pub fn fht_dke_runtime_boundary_matches(self) -> bool {
        Self::fht_dke_runtime_boundary_matches_parts(self.fht_dke_planning, self.runtime_planning)
    }

    pub fn fht_dke_runtime_boundary_drift_component_count(self) -> usize {
        Self::fht_dke_runtime_boundary_drift_component_count_parts(
            self.fht_dke_planning,
            self.runtime_planning,
        )
    }

    pub fn fht_dke_runtime_boundary_ready(self) -> bool {
        self.fht_dke_runtime_boundary_matches()
            && self.fht_dke_runtime_boundary_drift_component_count() == 0
    }

    pub fn stage_ready(self, stage: RuntimePlanningReadinessStage) -> bool {
        match stage {
            RuntimePlanningReadinessStage::FhtDkePlanning => self.fht_dke_planning_ready(),
            RuntimePlanningReadinessStage::RuntimePreRequest => self.runtime_pre_request_ready(),
            RuntimePlanningReadinessStage::FhtDkeRuntimeBoundary => {
                self.fht_dke_runtime_boundary_ready()
            }
        }
    }

    pub fn stage_signal_component_count(self, stage: RuntimePlanningReadinessStage) -> usize {
        match stage {
            RuntimePlanningReadinessStage::FhtDkePlanning => {
                self.fht_dke_planning_signal_component_count
            }
            RuntimePlanningReadinessStage::RuntimePreRequest => {
                self.runtime_pre_request_signal_component_count
            }
            RuntimePlanningReadinessStage::FhtDkeRuntimeBoundary => {
                self.fht_dke_runtime_boundary_signal_component_count
            }
        }
    }

    pub fn stage_blocker_component_count(self, stage: RuntimePlanningReadinessStage) -> usize {
        match stage {
            RuntimePlanningReadinessStage::FhtDkePlanning => {
                self.fht_dke_planning_blocker_component_count
            }
            RuntimePlanningReadinessStage::RuntimePreRequest => {
                self.runtime_pre_request_blocker_component_count
            }
            RuntimePlanningReadinessStage::FhtDkeRuntimeBoundary => {
                self.fht_dke_runtime_boundary_blocker_component_count
            }
        }
    }

    pub fn first_unready_stage(self) -> Option<RuntimePlanningReadinessStage> {
        Self::stage_order()
            .into_iter()
            .find(|stage| !self.stage_ready(*stage))
    }

    pub fn first_blocking_stage(self) -> Option<RuntimePlanningReadinessStage> {
        Self::stage_order()
            .into_iter()
            .find(|stage| self.stage_blocker_component_count(*stage) > 0)
    }

    pub fn runtime_planning_readiness_signal_component_count(self) -> usize {
        self.fht_dke_planning_signal_component_count
            .saturating_add(self.runtime_pre_request_signal_component_count)
            .saturating_add(self.fht_dke_runtime_boundary_signal_component_count)
    }

    pub fn has_runtime_planning_readiness_signals(self) -> bool {
        self.runtime_planning_readiness_signal_component_count() > 0
    }

    pub fn runtime_planning_readiness_blocker_component_count(self) -> usize {
        self.fht_dke_planning_blocker_component_count
            .saturating_add(self.runtime_pre_request_blocker_component_count)
            .saturating_add(self.fht_dke_runtime_boundary_blocker_component_count)
    }

    pub fn has_runtime_planning_readiness_blockers(self) -> bool {
        self.runtime_planning_readiness_blocker_component_count() > 0
    }

    pub fn runtime_planning_readiness_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .fht_dke_planning_signal_component_count
            .saturating_add(self.runtime_pre_request_signal_component_count)
            .saturating_add(self.fht_dke_runtime_boundary_signal_component_count);
        let expected_blocker_count = self
            .fht_dke_planning_blocker_component_count
            .saturating_add(self.runtime_pre_request_blocker_component_count)
            .saturating_add(self.fht_dke_runtime_boundary_blocker_component_count);

        self.runtime_planning_readiness_signal_component_count() == expected_signal_count
            && self.has_runtime_planning_readiness_signals() == (expected_signal_count > 0)
            && self.runtime_planning_readiness_blocker_component_count() == expected_blocker_count
            && self.has_runtime_planning_readiness_blockers() == (expected_blocker_count > 0)
            && self
                .fht_dke_planning
                .fht_dke_planning_readiness_accounting_is_consistent()
            && self
                .runtime_planning
                .backend_request_commit_accounting_is_consistent()
    }

    pub fn runtime_planning_readiness_is_clean(self) -> bool {
        !self.has_runtime_planning_readiness_blockers()
            && self.runtime_planning_readiness_accounting_is_consistent()
    }

    pub fn can_commit_runtime_planning_readiness(self) -> bool {
        self.runtime_planning_readiness_is_clean()
            && self.fht_dke_planning_ready()
            && self.runtime_pre_request_ready()
            && self.fht_dke_runtime_boundary_ready()
    }

    pub fn fht_dke_planning_commit_summary(self) -> FhtDkePlanningCommitSummary {
        self.fht_dke_planning.commit_summary()
    }

    pub fn can_use_committed_fht_dke_runtime_planning_parts(self) -> bool {
        self.fht_dke_planning_commit_summary()
            .can_use_committed_runtime_planning_parts()
    }

    pub fn can_commit_runtime_planning_with_committed_parts(self) -> bool {
        self.can_commit_runtime_planning_readiness()
            && self.can_use_committed_fht_dke_runtime_planning_parts()
    }

    fn fht_dke_runtime_boundary_matches_parts(
        fht_dke_planning: FhtDkePlanningReadinessSummary,
        runtime_planning: RuntimePlanningSummary,
    ) -> bool {
        Self::fht_dke_runtime_boundary_drift_component_count_parts(
            fht_dke_planning,
            runtime_planning,
        ) == 0
    }

    fn fht_dke_runtime_boundary_drift_component_count_parts(
        fht_dke_planning: FhtDkePlanningReadinessSummary,
        runtime_planning: RuntimePlanningSummary,
    ) -> usize {
        let fht = fht_dke_planning.fht_dke_budget;
        let runtime = runtime_planning.fht_dke;

        usize::from(fht.enabled != runtime.enabled)
            + usize::from(fht.total_tokens != runtime.total_tokens)
            + usize::from(fht.dense_tokens != runtime.dense_tokens)
            + usize::from(fht.routed_tokens != runtime.routed_tokens)
            + usize::from(!float_close(fht.dense_fraction, runtime.dense_fraction))
            + usize::from(!float_close(fht.routed_fraction, runtime.routed_fraction))
            + usize::from(fht.kv_import_blocks != runtime.kv_import_blocks)
            + usize::from(fht.kv_export_blocks != runtime.kv_export_blocks)
            + usize::from(fht.kv_exchange_blocks != runtime.kv_exchange_blocks)
            + usize::from(fht.has_kv_exchange != runtime.has_kv_exchange)
            + usize::from(fht.token_split_is_valid != runtime.token_split_is_valid)
            + usize::from(!float_close(
                fht.attention_threshold,
                runtime.attention_threshold,
            ))
            + usize::from(!float_close(fht.route_pressure, runtime.route_pressure))
    }
}

impl RuntimePlanningSummary {
    pub fn adapter_used_fallback(self) -> bool {
        self.adapter_selection.used_fallback
    }

    pub fn adapter_selection_from_observation(self) -> bool {
        !self.adapter_used_fallback() && self.adapter_selection.experience_id.is_some()
    }

    pub fn adapter_missing_allowed_candidates(self) -> bool {
        self.allowed_adapter_count == 0
    }

    pub fn has_matching_adapter_observation(self) -> bool {
        self.matching_observation_count > 0
    }

    pub fn adapter_observations_all_rejected(self) -> bool {
        self.observation_count > 0 && !self.has_matching_adapter_observation()
    }

    pub fn adapter_observations_missing(self) -> bool {
        self.observation_count == 0
    }

    pub fn adapter_observation_gap(self) -> bool {
        self.adapter_observations_missing() || self.adapter_observations_all_rejected()
    }

    pub fn adapter_fallback_due_to_no_allowed_adapter(self) -> bool {
        self.adapter_used_fallback()
            && self.adapter_fallback_reason == AdapterFallbackReason::NoAllowedAdapter
    }

    pub fn adapter_fallback_due_to_no_matching_observation(self) -> bool {
        self.adapter_used_fallback()
            && self.adapter_fallback_reason == AdapterFallbackReason::NoMatchingObservation
    }

    pub fn adapter_selection_blocked(self) -> bool {
        self.adapter_missing_allowed_candidates()
            || self.adapter_fallback_due_to_no_allowed_adapter()
    }

    pub fn adapter_selection_blocker_component_count(self) -> usize {
        usize::from(self.adapter_selection_blocked())
    }

    pub fn adapter_observation_signal_component_count(self) -> usize {
        usize::from(self.adapter_observations_missing())
            + usize::from(self.adapter_observations_all_rejected())
            + usize::from(self.adapter_fallback_due_to_no_matching_observation())
    }

    pub fn adapter_planning_signal_component_count(self) -> usize {
        self.adapter_selection_blocker_component_count()
            + self.adapter_observation_signal_component_count()
    }

    pub fn has_adapter_planning_signals(self) -> bool {
        self.adapter_planning_signal_component_count() > 0
    }

    pub fn context_exhausted(self) -> bool {
        self.generation_budget.context_exhausted()
    }

    pub fn context_soft_limited(self) -> bool {
        self.generation_budget.truncated_but_can_generate()
    }

    pub fn kv_prefetch_was_clamped(self) -> bool {
        self.kv_exchange.prefetch_was_clamped
    }

    pub fn fht_dke_limited_kv_prefetch(self) -> bool {
        self.kv_clamp.has_fht_dke_clamp()
    }

    pub fn route_pressure_is_active(self) -> bool {
        self.fht_dke.has_route_pressure()
    }

    pub fn route_pressure_is_high(self) -> bool {
        self.fht_dke.route_pressure_is_high()
    }

    pub fn has_routed_kv_exchange(self) -> bool {
        self.fht_dke.has_routed_work() && self.fht_dke.has_kv_exchange
    }

    pub fn route_pressure_signal_component_count(self) -> usize {
        self.fht_dke.route_pressure_signal_component_count()
    }

    pub fn high_route_pressure_signal_component_count(self) -> usize {
        self.fht_dke.high_route_pressure_signal_component_count()
    }

    pub fn routed_kv_exchange_signal_component_count(self) -> usize {
        usize::from(self.has_routed_kv_exchange())
    }

    pub fn fht_dke_budget_shape_problem_component_count(self) -> usize {
        self.fht_dke.budget_shape_problem_component_count()
    }

    pub fn fht_dke_budget_commit_signal_component_count(self) -> usize {
        self.fht_dke.fht_dke_budget_commit_signal_component_count()
    }

    pub fn has_fht_dke_budget_commit_signals(self) -> bool {
        self.fht_dke.has_fht_dke_budget_commit_signals()
    }

    pub fn fht_dke_budget_commit_blocker_component_count(self) -> usize {
        self.fht_dke.fht_dke_budget_commit_blocker_component_count()
    }

    pub fn has_fht_dke_budget_commit_blockers(self) -> bool {
        self.fht_dke.has_fht_dke_budget_commit_blockers()
    }

    pub fn fht_dke_budget_commit_accounting_is_consistent(self) -> bool {
        self.fht_dke
            .fht_dke_budget_commit_accounting_is_consistent()
    }

    pub fn kv_clamp_is_consistent(self) -> bool {
        self.kv_clamp.is_consistent()
            && self.kv_exchange.prefetch_was_clamped == self.kv_clamp.prefetch_was_clamped
            && self.kv_exchange.clamp_reason == self.kv_clamp.clamp_reason
            && self.kv_exchange.import_blocks == self.kv_clamp.planned_kv_import_blocks
    }

    pub fn kv_clamp_consistency_problem_component_count(self) -> usize {
        usize::from(!self.kv_clamp_is_consistent())
    }

    pub fn generation_readiness_blocker_component_count(self) -> usize {
        usize::from(!self.generation_budget.can_generate())
    }

    pub fn parallelism_readiness_blocker_component_count(self) -> usize {
        usize::from(self.max_parallel_chunks == 0)
    }

    pub fn fht_dke_token_split_blocker_component_count(self) -> usize {
        self.fht_dke.token_split_invalid_component_count()
    }

    pub fn request_readiness_blocker_component_count(self) -> usize {
        self.generation_readiness_blocker_component_count()
            .saturating_add(self.parallelism_readiness_blocker_component_count())
            .saturating_add(self.adapter_selection_blocker_component_count())
            .saturating_add(self.fht_dke_token_split_blocker_component_count())
    }

    pub fn has_request_readiness_blockers(self) -> bool {
        self.request_readiness_blocker_component_count() > 0
    }

    pub fn request_readiness_accounting_is_consistent(self) -> bool {
        let expected_blocker_count = usize::from(!self.generation_budget.can_generate())
            .saturating_add(usize::from(self.max_parallel_chunks == 0))
            .saturating_add(self.adapter_selection_blocker_component_count())
            .saturating_add(usize::from(!self.fht_dke.token_split_is_valid));

        self.request_readiness_blocker_component_count() == expected_blocker_count
            && self.has_request_readiness_blockers() == (expected_blocker_count > 0)
            && self.is_request_ready() == (expected_blocker_count == 0)
    }

    pub fn pre_request_gate_problem_component_count(self) -> usize {
        self.generation_readiness_blocker_component_count()
            .saturating_add(self.parallelism_readiness_blocker_component_count())
            .saturating_add(self.adapter_selection_blocker_component_count())
            .saturating_add(self.fht_dke_budget_commit_blocker_component_count())
            .saturating_add(self.kv_clamp_consistency_problem_component_count())
    }

    pub fn has_pre_request_gate_problem_components(self) -> bool {
        self.pre_request_gate_problem_component_count() > 0
    }

    pub fn pre_request_gate_accounting_is_consistent(self) -> bool {
        let expected_problem_count = self
            .generation_readiness_blocker_component_count()
            .saturating_add(self.parallelism_readiness_blocker_component_count())
            .saturating_add(self.adapter_selection_blocker_component_count())
            .saturating_add(self.fht_dke_budget_commit_blocker_component_count())
            .saturating_add(self.kv_clamp_consistency_problem_component_count());

        self.pre_request_gate_problem_component_count() == expected_problem_count
            && self.has_pre_request_gate_problem_components() == (expected_problem_count > 0)
            && self.fht_dke_budget_commit_accounting_is_consistent()
    }

    pub fn pre_request_gate_shape_is_clean(self) -> bool {
        !self.has_pre_request_gate_problem_components()
            && self.request_readiness_accounting_is_consistent()
            && self.pre_request_gate_accounting_is_consistent()
    }

    pub fn backend_request_commit_signal_component_count(self) -> usize {
        self.pre_request_gate_signal_component_count()
    }

    pub fn has_backend_request_commit_signals(self) -> bool {
        self.backend_request_commit_signal_component_count() > 0
    }

    pub fn backend_request_commit_blocker_component_count(self) -> usize {
        self.pre_request_gate_problem_component_count()
    }

    pub fn has_backend_request_commit_blockers(self) -> bool {
        self.backend_request_commit_blocker_component_count() > 0
    }

    pub fn backend_request_commit_accounting_is_consistent(self) -> bool {
        self.pre_request_gate_accounting_is_consistent()
            && self.request_readiness_accounting_is_consistent()
            && self.fht_dke_budget_commit_accounting_is_consistent()
            && self.backend_request_commit_signal_component_count()
                == self.pre_request_gate_signal_component_count()
            && self.has_backend_request_commit_signals()
                == (self.backend_request_commit_signal_component_count() > 0)
            && self.backend_request_commit_blocker_component_count()
                == self.pre_request_gate_problem_component_count()
            && self.has_backend_request_commit_blockers()
                == (self.backend_request_commit_blocker_component_count() > 0)
    }

    pub fn backend_request_commit_is_clean(self) -> bool {
        !self.has_backend_request_commit_blockers()
            && self.backend_request_commit_accounting_is_consistent()
    }

    pub fn can_commit_backend_request(self) -> bool {
        self.is_request_ready()
            && self.backend_request_commit_is_clean()
            && self.pre_request_gate_shape_is_clean()
    }

    pub fn can_send_backend_request(self) -> bool {
        self.is_request_ready() && self.pre_request_gate_shape_is_clean()
    }

    pub fn planning_pressure_signal_component_count(self) -> usize {
        self.fht_dke_budget_commit_signal_component_count()
            .saturating_add(usize::from(self.kv_prefetch_was_clamped()))
            .saturating_add(usize::from(self.fht_dke_limited_kv_prefetch()))
    }

    pub fn pre_request_gate_signal_component_count(self) -> usize {
        self.adapter_observation_signal_component_count()
            .saturating_add(self.planning_pressure_signal_component_count())
    }

    pub fn has_pre_request_gate_signals(self) -> bool {
        self.pre_request_gate_signal_component_count() > 0
    }

    pub fn is_request_ready(self) -> bool {
        self.generation_budget.can_generate()
            && self.max_parallel_chunks > 0
            && self.adapter_fallback_reason != AdapterFallbackReason::NoAllowedAdapter
            && self.fht_dke.token_split_is_valid
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimePlanningAcceptanceReport {
    pub generation_budget: RuntimeGenerationBudget,
    pub planning_violations: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimePlanningAcceptanceSummary {
    pub accepted: bool,
    pub planning_violation_count: usize,
    pub contract_violation_count: usize,
    pub context_exhausted: bool,
    pub failure_report_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimePlanningAcceptanceCommitSummary {
    pub acceptance: RuntimePlanningAcceptanceSummary,
    pub action: RuntimePlanningAcceptanceCommitAction,
    pub can_commit: bool,
    pub should_return_failure: bool,
    pub failure_reports: Vec<RuntimeFailureReport>,
    pub primary_failure_report: Option<RuntimeFailureReport>,
    pub primary_failure_summary: Option<RuntimeFailureSummary>,
    pub failure_batch: RuntimeFailureBatchSummary,
    pub failure_report_count: usize,
    pub can_format_runtime_failures: bool,
    pub total_problem_component_count: usize,
    pub total_shape_problem_component_count: usize,
    pub component_accounting_consistent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimePlanningAcceptanceCommitAction {
    CommitBackendRequest,
    ReturnRuntimeFailure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimePlanningFailureReturnSource {
    PlanningAcceptance,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimePlanningFailureReturnSummary {
    pub source: RuntimePlanningFailureReturnSource,
    pub can_commit: bool,
    pub should_return_failure: bool,
    pub has_primary_failure_summary: bool,
    pub primary_failure_summary: Option<RuntimeFailureSummary>,
    pub failure_batch: RuntimeFailureBatchSummary,
    pub failure_report_count: usize,
    pub can_format_runtime_failures: bool,
    pub total_problem_component_count: usize,
    pub total_shape_problem_component_count: usize,
    pub commit_decision_accounting_consistent: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimePlanningFailureReturnReport {
    pub source: RuntimePlanningFailureReturnSource,
    pub primary_failure: RuntimeFailureReport,
    pub primary_failure_summary: RuntimeFailureSummary,
    pub failure_batch: RuntimeFailureBatchSummary,
    pub failure_report_count: usize,
    pub can_format_runtime_failures: bool,
    pub total_problem_component_count: usize,
    pub total_shape_problem_component_count: usize,
}

impl RuntimePlanningAcceptanceCommitAction {
    pub fn can_commit(self) -> bool {
        matches!(self, Self::CommitBackendRequest)
    }

    pub fn should_return_failure(self) -> bool {
        matches!(self, Self::ReturnRuntimeFailure)
    }
}

impl RuntimePlanningFailureReturnSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::PlanningAcceptance => "runtime_planning_acceptance",
        }
    }
}

impl RuntimePlanningFailureReturnSummary {
    pub fn new(
        source: RuntimePlanningFailureReturnSource,
        can_commit: bool,
        should_return_failure: bool,
        primary_failure_summary: Option<RuntimeFailureSummary>,
        failure_batch: RuntimeFailureBatchSummary,
        failure_report_count: usize,
        can_format_runtime_failures: bool,
        total_problem_component_count: usize,
        total_shape_problem_component_count: usize,
        commit_decision_accounting_consistent: bool,
    ) -> Self {
        Self {
            source,
            can_commit,
            should_return_failure,
            has_primary_failure_summary: primary_failure_summary.is_some(),
            primary_failure_summary,
            failure_batch,
            failure_report_count,
            can_format_runtime_failures,
            total_problem_component_count,
            total_shape_problem_component_count,
            commit_decision_accounting_consistent,
        }
    }

    pub fn has_failure_reports(self) -> bool {
        self.failure_report_count > 0
    }

    pub fn has_problem_components(self) -> bool {
        self.total_problem_component_count > 0 || self.total_shape_problem_component_count > 0
    }

    pub fn failure_return_accounting_is_consistent(self) -> bool {
        self.commit_decision_accounting_consistent
            && self.should_return_failure == (!self.can_commit && self.has_failure_reports())
            && self.has_primary_failure_summary == self.primary_failure_summary.is_some()
            && self.has_primary_failure_summary == self.has_failure_reports()
            && self.failure_batch.total_count == self.failure_report_count
            && self.can_format_runtime_failures == self.failure_batch.can_format_runtime_failures()
            && (!self.has_failure_reports() || self.has_problem_components())
    }

    pub fn can_return_runtime_failure(self) -> bool {
        self.should_return_failure
            && self.has_primary_failure_summary
            && self.can_format_runtime_failures
            && self.failure_return_accounting_is_consistent()
    }
}

impl RuntimePlanningFailureReturnReport {
    pub fn new(
        source: RuntimePlanningFailureReturnSource,
        primary_failure: RuntimeFailureReport,
        failure_batch: RuntimeFailureBatchSummary,
        failure_report_count: usize,
        can_format_runtime_failures: bool,
        total_problem_component_count: usize,
        total_shape_problem_component_count: usize,
    ) -> Self {
        let primary_failure_summary = primary_failure.failure_summary();
        Self {
            source,
            primary_failure,
            primary_failure_summary,
            failure_batch,
            failure_report_count,
            can_format_runtime_failures,
            total_problem_component_count,
            total_shape_problem_component_count,
        }
    }

    pub fn backend_message(&self) -> String {
        self.primary_failure.backend_message()
    }

    pub fn diagnostics_note(&self) -> String {
        self.primary_failure.diagnostics_note()
    }

    pub fn inference_error(&self) -> InferenceError {
        InferenceError::from_failure(self.primary_failure.clone())
    }

    pub fn failure_return_report_shape_is_clean(&self) -> bool {
        self.primary_failure_summary == self.primary_failure.failure_summary()
            && self.failure_report_count > 0
            && self.failure_batch.total_count == self.failure_report_count
            && self.can_format_runtime_failures == self.failure_batch.can_format_runtime_failures()
            && self.can_format_runtime_failures
            && (self.total_problem_component_count > 0
                || self.total_shape_problem_component_count > 0)
    }

    pub fn can_use_runtime_planning_failure_return_report(&self) -> bool {
        self.failure_return_report_shape_is_clean()
    }
}

impl RuntimePlanningAcceptanceSummary {
    pub fn total_violation_count(self) -> usize {
        self.planning_violation_count
    }

    pub fn has_planning_violations(self) -> bool {
        self.planning_violation_count > 0
    }

    pub fn has_context_exhaustion(self) -> bool {
        self.context_exhausted
    }

    pub fn has_contract_failures(self) -> bool {
        self.contract_violation_count > 0
    }

    pub fn has_failure_reports(self) -> bool {
        self.failure_report_count > 0
    }

    pub fn has_failures(self) -> bool {
        self.has_planning_violations()
            || self.has_context_exhaustion()
            || self.has_contract_failures()
    }

    pub fn accepted_state_matches_failures(self) -> bool {
        self.accepted == !self.has_failures()
    }

    pub fn planning_violation_component_count(self) -> usize {
        usize::from(self.has_planning_violations())
    }

    pub fn context_exhaustion_component_count(self) -> usize {
        usize::from(self.has_context_exhaustion())
    }

    pub fn contract_failure_component_count(self) -> usize {
        usize::from(self.has_contract_failures())
    }

    pub fn mapped_failure_report_component_count(self) -> usize {
        usize::from(self.has_failure_reports())
    }

    pub fn accepted_state_problem_component_count(self) -> usize {
        usize::from(!self.accepted_state_matches_failures())
    }

    pub fn failure_report_parity_problem_component_count(self) -> usize {
        usize::from(!self.failure_report_matches_failures())
    }

    pub fn planning_acceptance_problem_component_count(self) -> usize {
        self.planning_violation_component_count()
            .saturating_add(self.context_exhaustion_component_count())
            .saturating_add(self.contract_failure_component_count())
            .saturating_add(self.mapped_failure_report_component_count())
    }

    pub fn has_planning_acceptance_problem_components(self) -> bool {
        self.planning_acceptance_problem_component_count() > 0
    }

    pub fn planning_acceptance_accounting_is_consistent(self) -> bool {
        let expected_problem_count = self
            .planning_violation_component_count()
            .saturating_add(self.context_exhaustion_component_count())
            .saturating_add(self.contract_failure_component_count())
            .saturating_add(self.mapped_failure_report_component_count());

        self.planning_acceptance_problem_component_count() == expected_problem_count
            && self.has_planning_acceptance_problem_components() == (expected_problem_count > 0)
    }

    pub fn failure_report_matches_failures(self) -> bool {
        self.failure_report_count
            == self.context_exhaustion_component_count() + self.contract_failure_component_count()
    }

    pub fn planning_acceptance_shape_problem_component_count(self) -> usize {
        self.accepted_state_problem_component_count()
            .saturating_add(self.failure_report_parity_problem_component_count())
    }

    pub fn has_planning_acceptance_shape_problem_components(self) -> bool {
        self.planning_acceptance_shape_problem_component_count() > 0
    }

    pub fn planning_acceptance_shape_accounting_is_consistent(self) -> bool {
        let expected_shape_problem_count = self
            .accepted_state_problem_component_count()
            .saturating_add(self.failure_report_parity_problem_component_count());

        self.planning_acceptance_accounting_is_consistent()
            && self.planning_acceptance_shape_problem_component_count()
                == expected_shape_problem_count
            && self.has_planning_acceptance_shape_problem_components()
                == (expected_shape_problem_count > 0)
    }

    pub fn is_clean_acceptance(self) -> bool {
        self.accepted
            && !self.has_planning_violations()
            && !self.has_context_exhaustion()
            && !self.has_contract_failures()
            && !self.has_failure_reports()
            && self.accepted_state_matches_failures()
            && self.failure_report_matches_failures()
            && self.planning_acceptance_shape_accounting_is_consistent()
    }

    pub fn planning_acceptance_shape_is_clean(self) -> bool {
        self.is_clean_acceptance()
    }

    pub fn can_accept_runtime_planning(self) -> bool {
        self.planning_acceptance_shape_is_clean()
    }
}

impl RuntimePlanningAcceptanceCommitSummary {
    pub fn new(report: &RuntimePlanningAcceptanceReport) -> Self {
        let acceptance = report.acceptance_summary();
        let failure_reports = report.failure_reports();
        let primary_failure_report = failure_reports.first().cloned();
        let primary_failure_summary = primary_failure_report
            .as_ref()
            .map(|failure| failure.failure_summary());
        let failure_batch = RuntimeFailureReport::batch_summary(&failure_reports);
        let can_commit = acceptance.can_accept_runtime_planning();
        let failure_report_count = failure_reports.len();
        let can_format_runtime_failures = failure_batch.can_format_runtime_failures();
        let should_return_failure = !can_commit && failure_report_count > 0;
        let action = if can_commit {
            RuntimePlanningAcceptanceCommitAction::CommitBackendRequest
        } else {
            RuntimePlanningAcceptanceCommitAction::ReturnRuntimeFailure
        };

        Self {
            acceptance,
            action,
            can_commit,
            should_return_failure,
            failure_reports,
            primary_failure_report,
            primary_failure_summary,
            failure_batch,
            failure_report_count,
            can_format_runtime_failures,
            total_problem_component_count: acceptance.planning_acceptance_problem_component_count(),
            total_shape_problem_component_count: acceptance
                .planning_acceptance_shape_problem_component_count(),
            component_accounting_consistent: acceptance
                .planning_acceptance_shape_accounting_is_consistent(),
        }
    }

    pub fn action_can_commit(&self) -> bool {
        self.action.can_commit()
    }

    pub fn action_should_return_failure(&self) -> bool {
        self.action.should_return_failure()
    }

    pub fn has_primary_failure_summary(&self) -> bool {
        self.primary_failure_summary.is_some()
    }

    pub fn failure_batch_shape_is_clean(&self) -> bool {
        self.failure_batch.failure_batch_shape_is_clean()
    }

    pub fn failure_return_summary(&self) -> RuntimePlanningFailureReturnSummary {
        RuntimePlanningFailureReturnSummary::new(
            RuntimePlanningFailureReturnSource::PlanningAcceptance,
            self.can_commit,
            self.should_return_failure,
            self.primary_failure_summary,
            self.failure_batch,
            self.failure_report_count,
            self.can_format_runtime_failures,
            self.total_problem_component_count,
            self.total_shape_problem_component_count,
            self.commit_decision_accounting_is_consistent(),
        )
    }

    pub fn runtime_failure_return_report(&self) -> Option<RuntimePlanningFailureReturnReport> {
        if self.failure_return_summary().can_return_runtime_failure() {
            self.primary_failure_report.clone().map(|failure| {
                RuntimePlanningFailureReturnReport::new(
                    RuntimePlanningFailureReturnSource::PlanningAcceptance,
                    failure,
                    self.failure_batch,
                    self.failure_report_count,
                    self.can_format_runtime_failures,
                    self.total_problem_component_count,
                    self.total_shape_problem_component_count,
                )
            })
        } else {
            None
        }
    }

    pub fn commit_decision_accounting_is_consistent(&self) -> bool {
        self.can_commit == self.acceptance.can_accept_runtime_planning()
            && self.should_return_failure == (!self.can_commit && self.failure_report_count > 0)
            && self.action_can_commit() == self.can_commit
            && self.action_should_return_failure() == self.should_return_failure
            && self.failure_report_count == self.failure_reports.len()
            && self.failure_report_count == self.acceptance.failure_report_count
            && self.primary_failure_report.as_ref() == self.failure_reports.first()
            && self.primary_failure_summary
                == self
                    .primary_failure_report
                    .as_ref()
                    .map(|failure| failure.failure_summary())
            && self.failure_batch == RuntimeFailureReport::batch_summary(&self.failure_reports)
            && self.can_format_runtime_failures == self.failure_batch.can_format_runtime_failures()
            && self.total_problem_component_count
                == self
                    .acceptance
                    .planning_acceptance_problem_component_count()
            && self.total_shape_problem_component_count
                == self
                    .acceptance
                    .planning_acceptance_shape_problem_component_count()
            && self.component_accounting_consistent
                == self
                    .acceptance
                    .planning_acceptance_shape_accounting_is_consistent()
    }

    pub fn can_commit_backend_request(&self) -> bool {
        self.can_commit && self.commit_decision_accounting_is_consistent()
    }

    pub fn should_return_runtime_failure(&self) -> bool {
        self.should_return_failure && self.commit_decision_accounting_is_consistent()
    }
}

impl RuntimePlanningAcceptanceReport {
    pub fn is_accepted(&self) -> bool {
        self.planning_violations.is_empty()
    }

    pub fn violations(&self) -> &[String] {
        &self.planning_violations
    }

    pub fn acceptance_summary(&self) -> RuntimePlanningAcceptanceSummary {
        let context_exhausted = !self.generation_budget.can_generate();
        let contract_violation_count = self.contract_violation_count();

        RuntimePlanningAcceptanceSummary {
            accepted: self.is_accepted(),
            planning_violation_count: self.planning_violations.len(),
            contract_violation_count,
            context_exhausted,
            failure_report_count: usize::from(context_exhausted)
                + usize::from(contract_violation_count > 0),
        }
    }

    pub fn failure_reports(&self) -> Vec<RuntimeFailureReport> {
        let mut failures = Vec::new();
        let contract_violations = self.contract_violations();

        if !self.generation_budget.can_generate() {
            failures.push(RuntimeFailureReport::context_exhausted(
                self.generation_budget,
            ));
        }
        if !contract_violations.is_empty() {
            failures.push(RuntimeFailureReport::contract_violation(
                acceptance_message("runtime planning acceptance failed", &contract_violations),
            ));
        }

        failures
    }

    pub fn failure_batch_summary(&self) -> RuntimeFailureBatchSummary {
        RuntimeFailureReport::batch_summary(&self.failure_reports())
    }

    pub fn primary_failure_report(&self) -> Option<RuntimeFailureReport> {
        self.failure_reports().into_iter().next()
    }

    pub fn primary_failure_summary(&self) -> Option<RuntimeFailureSummary> {
        self.primary_failure_report()
            .map(|failure| failure.failure_summary())
    }

    pub fn commit_summary(&self) -> RuntimePlanningAcceptanceCommitSummary {
        RuntimePlanningAcceptanceCommitSummary::new(self)
    }

    fn contract_violations(&self) -> Vec<String> {
        self.planning_violations
            .iter()
            .filter(|violation| !violation.starts_with("runtime planning has no generation room"))
            .cloned()
            .collect()
    }

    fn contract_violation_count(&self) -> usize {
        self.planning_violations
            .iter()
            .filter(|violation| !violation.starts_with("runtime planning has no generation room"))
            .count()
    }
}

fn acceptance_message(prefix: &str, violations: &[String]) -> String {
    if violations.is_empty() {
        prefix.to_owned()
    } else {
        format!("{prefix}: {}", violations.join("; "))
    }
}

fn float_close(left: f32, right: f32) -> bool {
    (left - right).abs() <= 0.0001
}

impl RuntimePlanningDigest {
    pub fn from_request(
        request: &InferenceRequest,
        route_budget: RouteBudget,
        execution: &AdapterExecutionContext,
        observations: &[AdapterObservation],
        budgeter: &dyn FhtDkeBudgeter,
    ) -> Self {
        let generation_budget = request.generation_budget();
        let runtime_execution = execution.clone().clamp_for_runtime(&request.runtime);
        let adapter_selection_report = runtime_execution.select_adapter_report(observations);
        let adapter_selection = adapter_selection_report.selection;
        let fht_dke_budget = budgeter.budget(
            &FhtDkeInput::new(
                request.prompt_tokens,
                generation_budget.max_generated_tokens,
                request.runtime.clone(),
            )
            .with_route_budget(route_budget)
            .with_experiments(request.experiments),
        );
        let planned_kv_import_blocks = if fht_dke_budget.enabled {
            runtime_execution
                .kv_prefetch_blocks
                .min(fht_dke_budget.kv_import_blocks)
        } else {
            runtime_execution.kv_prefetch_blocks
        };

        Self {
            generation_budget,
            fht_dke_budget,
            adapter_selection,
            adapter_selection_report,
            requested_kv_prefetch_blocks: execution.kv_prefetch_blocks,
            runtime_kv_prefetch_blocks: runtime_execution.kv_prefetch_blocks,
            planned_kv_import_blocks,
            planned_kv_export_blocks: fht_dke_budget.kv_export_blocks,
            hardware_pressure: runtime_execution.hardware_pressure,
            compute_headroom: runtime_execution.compute_headroom,
            max_parallel_chunks: runtime_execution.max_parallel_chunks,
            latency_budget_ms: runtime_execution.latency_budget_ms,
        }
    }

    pub fn context_limited(self) -> bool {
        self.generation_budget.truncated_by_context || !self.generation_budget.can_generate()
    }

    pub fn backend_max_tokens(self) -> usize {
        self.generation_budget.max_generated_tokens
    }

    pub fn planned_kv_exchange(self) -> RuntimePlanningKvExchange {
        RuntimePlanningKvExchange {
            import_blocks: self.planned_kv_import_blocks,
            export_blocks: self.planned_kv_export_blocks,
            prefetch_was_clamped: self.kv_prefetch_was_clamped(),
            clamp_reason: self.kv_prefetch_clamp_reason(),
        }
    }

    pub fn manifest_kv_bridge_summary(
        self,
        manifest: &RuntimeManifestDigest,
    ) -> RuntimePlanningManifestKvBridgeSummary {
        let planned_kv = self.planned_kv_exchange();

        RuntimePlanningManifestKvBridgeSummary {
            import: RuntimeKvImportPlan::manifest_plan_summary(manifest, planned_kv.import_blocks),
            export: RuntimeKvExportPlan::manifest_plan_summary(manifest, planned_kv.export_blocks),
            planned_import_blocks: planned_kv.import_blocks,
            planned_export_blocks: planned_kv.export_blocks,
        }
    }

    pub fn fht_dke_summary(self) -> FhtDkeBudgetSummary {
        self.fht_dke_budget.budget_summary()
    }

    pub fn planning_summary(self) -> RuntimePlanningSummary {
        RuntimePlanningSummary {
            generation_budget: self.generation_budget,
            context_limited: self.context_limited(),
            backend_max_tokens: self.backend_max_tokens(),
            adapter_selection: self.adapter_selection,
            adapter_fallback_reason: self.adapter_fallback_reason(),
            allowed_adapter_count: self.adapter_selection_report.allowed_adapter_count,
            observation_count: self.adapter_selection_report.observation_count,
            matching_observation_count: self.adapter_selection_report.matching_observation_count,
            matched_observation_fraction: self.matched_adapter_observation_fraction(),
            fht_dke: self.fht_dke_summary(),
            kv_exchange: self.planned_kv_exchange(),
            kv_clamp: self.kv_prefetch_clamp_summary(),
            requested_kv_prefetch_blocks: self.requested_kv_prefetch_blocks,
            runtime_kv_prefetch_blocks: self.runtime_kv_prefetch_blocks,
            hardware_pressure: self.hardware_pressure,
            compute_headroom: self.compute_headroom,
            max_parallel_chunks: self.max_parallel_chunks,
            latency_budget_ms: self.latency_budget_ms,
        }
    }

    pub fn kv_prefetch_was_clamped(self) -> bool {
        self.planned_kv_import_blocks < self.requested_kv_prefetch_blocks
    }

    pub fn kv_prefetch_clamp_reason(self) -> RuntimePlanningKvClampReason {
        let runtime_clamped = self.runtime_kv_prefetch_blocks < self.requested_kv_prefetch_blocks;
        let fht_clamped = self.planned_kv_import_blocks < self.runtime_kv_prefetch_blocks;

        match (runtime_clamped, fht_clamped) {
            (false, false) => RuntimePlanningKvClampReason::NotClamped,
            (true, false) => RuntimePlanningKvClampReason::RuntimeMetadataLimit,
            (false, true) => RuntimePlanningKvClampReason::FhtDkeBudgetLimit,
            (true, true) => RuntimePlanningKvClampReason::RuntimeAndFhtDkeLimits,
        }
    }

    pub fn kv_prefetch_clamp_summary(self) -> RuntimePlanningKvClampSummary {
        RuntimePlanningKvClampSummary {
            requested_kv_prefetch_blocks: self.requested_kv_prefetch_blocks,
            runtime_kv_prefetch_blocks: self.runtime_kv_prefetch_blocks,
            planned_kv_import_blocks: self.planned_kv_import_blocks,
            runtime_metadata_reduction: self
                .requested_kv_prefetch_blocks
                .saturating_sub(self.runtime_kv_prefetch_blocks),
            fht_dke_reduction: self
                .runtime_kv_prefetch_blocks
                .saturating_sub(self.planned_kv_import_blocks),
            total_reduction: self
                .requested_kv_prefetch_blocks
                .saturating_sub(self.planned_kv_import_blocks),
            prefetch_was_clamped: self.kv_prefetch_was_clamped(),
            clamp_reason: self.kv_prefetch_clamp_reason(),
        }
    }

    pub fn adapter_used_fallback(self) -> bool {
        self.adapter_selection_report.used_fallback()
    }

    pub fn adapter_fallback_reason(self) -> AdapterFallbackReason {
        self.adapter_selection_report.fallback_reason
    }

    pub fn matched_adapter_observation_fraction(self) -> f32 {
        self.adapter_selection_report.matched_observation_fraction()
    }

    pub fn contract_violations(self) -> Vec<String> {
        let mut violations = Vec::new();

        if !self.generation_budget.can_generate() {
            violations.push(format!(
                "runtime planning has no generation room: prompt_tokens={} planned_context_tokens={}",
                self.generation_budget.prompt_tokens, self.generation_budget.planned_context_tokens
            ));
        }
        if self.fht_dke_budget.enabled
            && self.fht_dke_budget.dense_tokens + self.fht_dke_budget.routed_tokens
                != self.fht_dke_budget.total_tokens
        {
            violations.push(format!(
                "FHT-DKE token split {}+{} does not match total_tokens {}",
                self.fht_dke_budget.dense_tokens,
                self.fht_dke_budget.routed_tokens,
                self.fht_dke_budget.total_tokens
            ));
        }
        if self.fht_dke_budget.enabled
            && self.planned_kv_import_blocks > self.fht_dke_budget.kv_import_blocks
        {
            violations.push(format!(
                "planned KV imports {} exceed FHT-DKE KV import budget {}",
                self.planned_kv_import_blocks, self.fht_dke_budget.kv_import_blocks
            ));
        }
        if self.max_parallel_chunks == 0 {
            violations
                .push("runtime planning max_parallel_chunks must be greater than zero".into());
        }
        if self.adapter_selection_report.fallback_reason == AdapterFallbackReason::NoAllowedAdapter
        {
            violations
                .push("runtime planning has no allowed adapter execution candidates".to_owned());
        }

        violations
    }

    pub fn acceptance_report(self) -> RuntimePlanningAcceptanceReport {
        RuntimePlanningAcceptanceReport {
            generation_budget: self.generation_budget,
            planning_violations: self.contract_violations(),
        }
    }

    pub fn is_valid(self) -> bool {
        self.contract_violations().is_empty()
    }

    pub fn summary(self) -> String {
        format!(
            "prompt_tokens={} max_generated={} planned_context={} truncated={} adapter={} adapter_fallback={} matched_adapter_observations={}/{} fht_dke={} dense={} routed={} kv_prefetch_requested={} kv_prefetch_runtime={} kv_import={} kv_clamp={} kv_export={} route_pressure={:.3} hardware_pressure={:.3} parallel_chunks={}",
            self.generation_budget.prompt_tokens,
            self.generation_budget.max_generated_tokens,
            self.generation_budget.planned_context_tokens,
            self.generation_budget.truncated_by_context,
            self.adapter_selection.adapter.as_str(),
            self.adapter_selection_report.fallback_reason.as_str(),
            self.adapter_selection_report.matching_observation_count,
            self.adapter_selection_report.observation_count,
            self.fht_dke_budget.enabled,
            self.fht_dke_budget.dense_tokens,
            self.fht_dke_budget.routed_tokens,
            self.requested_kv_prefetch_blocks,
            self.runtime_kv_prefetch_blocks,
            self.planned_kv_import_blocks,
            self.kv_prefetch_clamp_reason().as_str(),
            self.planned_kv_export_blocks,
            self.fht_dke_budget.route_pressure,
            self.hardware_pressure,
            self.max_parallel_chunks
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimePlanningKvExchange {
    pub import_blocks: usize,
    pub export_blocks: usize,
    pub prefetch_was_clamped: bool,
    pub clamp_reason: RuntimePlanningKvClampReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimePlanningKvClampSummary {
    pub requested_kv_prefetch_blocks: usize,
    pub runtime_kv_prefetch_blocks: usize,
    pub planned_kv_import_blocks: usize,
    pub runtime_metadata_reduction: usize,
    pub fht_dke_reduction: usize,
    pub total_reduction: usize,
    pub prefetch_was_clamped: bool,
    pub clamp_reason: RuntimePlanningKvClampReason,
}

impl RuntimePlanningManifestKvBridgeSummary {
    pub fn import_bridge_is_clean(self) -> bool {
        self.import.can_use_manifest_runtime_kv_import_plan()
    }

    pub fn export_bridge_is_clean(self) -> bool {
        self.export.can_use_manifest_runtime_kv_export_plan()
    }

    pub fn import_plan_matches_planning(self) -> bool {
        self.import.import_plan_max_blocks == self.planned_import_blocks
    }

    pub fn export_plan_matches_planning(self) -> bool {
        self.export.export_plan_max_blocks == self.planned_export_blocks
    }

    pub fn manifest_plans_match_planning(self) -> bool {
        self.import_plan_matches_planning() && self.export_plan_matches_planning()
    }

    pub fn planning_import_drift_blocks(self) -> usize {
        self.import
            .import_plan_max_blocks
            .abs_diff(self.planned_import_blocks)
    }

    pub fn planning_export_drift_blocks(self) -> usize {
        self.export
            .export_plan_max_blocks
            .abs_diff(self.planned_export_blocks)
    }

    pub fn planning_kv_drift_blocks(self) -> usize {
        self.planning_import_drift_blocks()
            .saturating_add(self.planning_export_drift_blocks())
    }

    pub fn planning_kv_activity_signal_component_count(self) -> usize {
        usize::from(self.planned_import_blocks > 0)
            .saturating_add(usize::from(self.planned_export_blocks > 0))
    }

    pub fn manifest_kv_bridge_signal_component_count(self) -> usize {
        self.import
            .manifest_bridge_signal_component_count()
            .saturating_add(self.export.manifest_bridge_signal_component_count())
            .saturating_add(self.planning_kv_activity_signal_component_count())
    }

    pub fn has_manifest_kv_bridge_signals(self) -> bool {
        self.manifest_kv_bridge_signal_component_count() > 0
    }

    pub fn import_plan_planning_drift_component_count(self) -> usize {
        usize::from(!self.import_plan_matches_planning())
    }

    pub fn export_plan_planning_drift_component_count(self) -> usize {
        usize::from(!self.export_plan_matches_planning())
    }

    pub fn planning_kv_drift_component_count(self) -> usize {
        self.import_plan_planning_drift_component_count()
            .saturating_add(self.export_plan_planning_drift_component_count())
    }

    pub fn manifest_bridge_problem_component_count(self) -> usize {
        self.import
            .manifest_bridge_problem_component_count()
            .saturating_add(self.export.manifest_bridge_problem_component_count())
    }

    pub fn manifest_kv_bridge_problem_component_count(self) -> usize {
        self.manifest_bridge_problem_component_count()
            .saturating_add(self.planning_kv_drift_component_count())
    }

    pub fn has_manifest_kv_bridge_problem_components(self) -> bool {
        self.manifest_kv_bridge_problem_component_count() > 0
    }

    pub fn manifest_kv_bridge_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .import
            .manifest_bridge_signal_component_count()
            .saturating_add(self.export.manifest_bridge_signal_component_count())
            .saturating_add(self.planning_kv_activity_signal_component_count());
        let expected_problem_count = self
            .import
            .manifest_bridge_problem_component_count()
            .saturating_add(self.export.manifest_bridge_problem_component_count())
            .saturating_add(self.planning_kv_drift_component_count());

        self.import.manifest_bridge_accounting_is_consistent()
            && self.export.manifest_bridge_accounting_is_consistent()
            && self.manifest_kv_bridge_signal_component_count() == expected_signal_count
            && self.has_manifest_kv_bridge_signals() == (expected_signal_count > 0)
            && self.manifest_kv_bridge_problem_component_count() == expected_problem_count
            && self.has_manifest_kv_bridge_problem_components() == (expected_problem_count > 0)
    }

    pub fn manifest_kv_bridge_shape_is_clean(self) -> bool {
        !self.has_manifest_kv_bridge_problem_components()
            && self.manifest_kv_bridge_accounting_is_consistent()
    }

    pub fn can_use_runtime_planning_manifest_kv_bridge(self) -> bool {
        self.manifest_kv_bridge_shape_is_clean()
            && self.import_bridge_is_clean()
            && self.export_bridge_is_clean()
            && self.manifest_plans_match_planning()
    }
}

impl RuntimePlanningKvClampSummary {
    pub fn has_runtime_metadata_clamp(self) -> bool {
        self.runtime_metadata_reduction > 0
    }

    pub fn has_fht_dke_clamp(self) -> bool {
        self.fht_dke_reduction > 0
    }

    pub fn import_matches_runtime_prefetch(self) -> bool {
        self.planned_kv_import_blocks == self.runtime_kv_prefetch_blocks
    }

    pub fn runtime_prefetch_not_above_requested(self) -> bool {
        self.runtime_kv_prefetch_blocks <= self.requested_kv_prefetch_blocks
    }

    pub fn planned_import_not_above_runtime_prefetch(self) -> bool {
        self.planned_kv_import_blocks <= self.runtime_kv_prefetch_blocks
    }

    pub fn planned_import_not_above_requested(self) -> bool {
        self.planned_kv_import_blocks <= self.requested_kv_prefetch_blocks
    }

    pub fn clamp_counts_are_bounded(self) -> bool {
        self.runtime_prefetch_not_above_requested()
            && self.planned_import_not_above_runtime_prefetch()
            && self.planned_import_not_above_requested()
    }

    pub fn reductions_match_total(self) -> bool {
        self.runtime_metadata_reduction
            .saturating_add(self.fht_dke_reduction)
            == self.total_reduction
    }

    pub fn block_counts_match_reductions(self) -> bool {
        self.requested_kv_prefetch_blocks
            .saturating_sub(self.runtime_kv_prefetch_blocks)
            == self.runtime_metadata_reduction
            && self
                .runtime_kv_prefetch_blocks
                .saturating_sub(self.planned_kv_import_blocks)
                == self.fht_dke_reduction
            && self
                .requested_kv_prefetch_blocks
                .saturating_sub(self.planned_kv_import_blocks)
                == self.total_reduction
    }

    pub fn clamp_reason_matches_reductions(self) -> bool {
        match self.clamp_reason {
            RuntimePlanningKvClampReason::NotClamped => {
                !self.has_runtime_metadata_clamp() && !self.has_fht_dke_clamp()
            }
            RuntimePlanningKvClampReason::RuntimeMetadataLimit => {
                self.has_runtime_metadata_clamp() && !self.has_fht_dke_clamp()
            }
            RuntimePlanningKvClampReason::FhtDkeBudgetLimit => {
                !self.has_runtime_metadata_clamp() && self.has_fht_dke_clamp()
            }
            RuntimePlanningKvClampReason::RuntimeAndFhtDkeLimits => {
                self.has_runtime_metadata_clamp() && self.has_fht_dke_clamp()
            }
        }
    }

    pub fn prefetch_clamp_flag_matches_counts(self) -> bool {
        self.prefetch_was_clamped == (self.total_reduction > 0)
    }

    pub fn clamp_bound_problem_component_count(self) -> usize {
        usize::from(!self.runtime_prefetch_not_above_requested())
            + usize::from(!self.planned_import_not_above_runtime_prefetch())
            + usize::from(!self.planned_import_not_above_requested())
    }

    pub fn clamp_reduction_problem_component_count(self) -> usize {
        usize::from(!self.reductions_match_total())
            + usize::from(!self.block_counts_match_reductions())
    }

    pub fn clamp_reason_problem_component_count(self) -> usize {
        usize::from(!self.clamp_reason_matches_reductions())
    }

    pub fn clamp_flag_problem_component_count(self) -> usize {
        usize::from(!self.prefetch_clamp_flag_matches_counts())
    }

    pub fn clamp_shape_problem_component_count(self) -> usize {
        self.clamp_bound_problem_component_count()
            .saturating_add(self.clamp_reduction_problem_component_count())
            .saturating_add(self.clamp_reason_problem_component_count())
            .saturating_add(self.clamp_flag_problem_component_count())
    }

    pub fn has_clamp_shape_problem_components(self) -> bool {
        self.clamp_shape_problem_component_count() > 0
    }

    pub fn clamp_shape_accounting_is_consistent(self) -> bool {
        let expected_problem_count = self
            .clamp_bound_problem_component_count()
            .saturating_add(self.clamp_reduction_problem_component_count())
            .saturating_add(self.clamp_reason_problem_component_count())
            .saturating_add(self.clamp_flag_problem_component_count());

        self.clamp_shape_problem_component_count() == expected_problem_count
            && self.has_clamp_shape_problem_components() == (expected_problem_count > 0)
            && self.is_consistent() == (expected_problem_count == 0)
    }

    pub fn clamp_shape_is_clean(self) -> bool {
        !self.has_clamp_shape_problem_components() && self.clamp_shape_accounting_is_consistent()
    }

    pub fn can_use_runtime_planning_kv_clamp(self) -> bool {
        self.clamp_shape_is_clean()
    }

    pub fn is_unclamped(self) -> bool {
        !self.prefetch_was_clamped
            && self.clamp_reason == RuntimePlanningKvClampReason::NotClamped
            && self.total_reduction == 0
    }

    pub fn clamped_by_runtime_only(self) -> bool {
        self.clamp_reason == RuntimePlanningKvClampReason::RuntimeMetadataLimit
            && self.has_runtime_metadata_clamp()
            && !self.has_fht_dke_clamp()
    }

    pub fn clamped_by_fht_dke_only(self) -> bool {
        self.clamp_reason == RuntimePlanningKvClampReason::FhtDkeBudgetLimit
            && !self.has_runtime_metadata_clamp()
            && self.has_fht_dke_clamp()
    }

    pub fn clamped_by_runtime_and_fht_dke(self) -> bool {
        self.clamp_reason == RuntimePlanningKvClampReason::RuntimeAndFhtDkeLimits
            && self.has_runtime_metadata_clamp()
            && self.has_fht_dke_clamp()
    }

    pub fn is_consistent(self) -> bool {
        self.clamp_counts_are_bounded()
            && self.reductions_match_total()
            && self.block_counts_match_reductions()
            && self.clamp_reason_matches_reductions()
            && self.prefetch_clamp_flag_matches_counts()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimePlanningKvClampReason {
    NotClamped,
    RuntimeMetadataLimit,
    FhtDkeBudgetLimit,
    RuntimeAndFhtDkeLimits,
}

impl RuntimePlanningKvClampReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotClamped => "not-clamped",
            Self::RuntimeMetadataLimit => "runtime-metadata-limit",
            Self::FhtDkeBudgetLimit => "fht-dke-budget-limit",
            Self::RuntimeAndFhtDkeLimits => "runtime-and-fht-dke-limits",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::RuntimeAdapter;
    use crate::attention::{
        AttentionCandidate, AttentionCandidateBatchSummary, AttentionDecisionSummary,
        AttentionPolicy, AttentionSelectionReadinessSummary, ThresholdAttentionPolicy,
    };
    use crate::engine::RuntimeFailureKind;
    use crate::experiment::ExperimentSwitches;
    use crate::fht_dke::{DeterministicFhtDkeBudgeter, FhtDkePlanningReadinessSummary};
    use crate::hardware::{DeviceClass, HardwareAllocator, HardwareLoadSnapshot};
    use crate::manifest::{RuntimeKvPolicy, RuntimeManifestDigest, TransformerRuntimeArchitecture};
    use crate::profile::{HierarchyWeights, TaskProfile};
    use crate::router::{
        DefaultHierarchicalRouter, HierarchicalRouter, RouteBudgetReadinessSummary, RouteLayer,
        RouteLayerCounts, RoutingContext, RoutingDecisionSummary, TokenFeatures,
    };
    use crate::runtime::RuntimeMetadata;
    use crate::transformer::{
        TransformerAttentionKind, TransformerLayerBudget, TransformerPlanCounts,
        TransformerPlanDigest, TransformerPlanSummary, TransformerPlanningPressureSummary,
        TransformerPlanningReadinessSummary,
    };

    #[test]
    fn planning_digest_clamps_runtime_context_and_kv_prefetch() {
        let runtime = RuntimeMetadata::new("planning", "tok", 1024, 128)
            .with_kv_exchange(true, true)
            .with_kv_limits(2, 4);
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(900)
            .with_max_tokens(256)
            .with_runtime(runtime)
            .with_experiments(ExperimentSwitches::default().with_fht_dke(true));
        let execution =
            AdapterExecutionContext::new([RuntimeAdapter::Cuda, RuntimeAdapter::CpuSimd])
                .with_pressure(0.70, 0.30)
                .with_parallel_chunks(2)
                .with_kv_prefetch_blocks(8);
        let observations = [
            AdapterObservation::new(RuntimeAdapter::CpuSimd, 0.40, 0.5, 0.5, None, None, 7),
            AdapterObservation::new(RuntimeAdapter::Cuda, 0.90, 0.8, 0.9, None, None, 8),
        ];

        let digest = RuntimePlanningDigest::from_request(
            &request,
            RouteBudget {
                threshold: 0.45,
                attention_tokens: 9,
                fast_tokens: 1,
                attention_fraction: 0.90,
            },
            &execution,
            &observations,
            &DeterministicFhtDkeBudgeter::new(0.10, 0.60, 128),
        );

        assert!(digest.is_valid());
        assert_eq!(digest.generation_budget.max_generated_tokens, 124);
        assert_eq!(digest.backend_max_tokens(), 124);
        assert!(digest.context_limited());
        assert_eq!(digest.adapter_selection.adapter, RuntimeAdapter::Cuda);
        assert!(!digest.adapter_used_fallback());
        assert_eq!(
            digest.adapter_fallback_reason(),
            AdapterFallbackReason::NoFallback
        );
        assert_eq!(digest.adapter_selection_report.allowed_adapter_count, 2);
        assert_eq!(digest.adapter_selection_report.observation_count, 2);
        assert_eq!(
            digest.adapter_selection_report.matching_observation_count,
            2
        );
        assert_eq!(digest.matched_adapter_observation_fraction(), 1.0);
        assert_eq!(digest.requested_kv_prefetch_blocks, 8);
        assert_eq!(digest.runtime_kv_prefetch_blocks, 2);
        assert_eq!(digest.planned_kv_import_blocks, 2);
        assert!(digest.kv_prefetch_was_clamped());
        assert_eq!(
            digest.kv_prefetch_clamp_reason(),
            RuntimePlanningKvClampReason::RuntimeMetadataLimit
        );
        assert_eq!(
            digest.planned_kv_exchange(),
            RuntimePlanningKvExchange {
                import_blocks: 2,
                export_blocks: 4,
                prefetch_was_clamped: true,
                clamp_reason: RuntimePlanningKvClampReason::RuntimeMetadataLimit,
            }
        );
        assert_eq!(
            digest.kv_prefetch_clamp_summary(),
            RuntimePlanningKvClampSummary {
                requested_kv_prefetch_blocks: 8,
                runtime_kv_prefetch_blocks: 2,
                planned_kv_import_blocks: 2,
                runtime_metadata_reduction: 6,
                fht_dke_reduction: 0,
                total_reduction: 6,
                prefetch_was_clamped: true,
                clamp_reason: RuntimePlanningKvClampReason::RuntimeMetadataLimit,
            }
        );
        assert!(
            digest
                .kv_prefetch_clamp_summary()
                .has_runtime_metadata_clamp()
        );
        assert!(!digest.kv_prefetch_clamp_summary().has_fht_dke_clamp());
        assert!(
            digest
                .kv_prefetch_clamp_summary()
                .import_matches_runtime_prefetch()
        );
        assert!(digest.kv_prefetch_clamp_summary().is_consistent());
        assert!(digest.kv_prefetch_clamp_summary().clamped_by_runtime_only());
        assert!(!digest.kv_prefetch_clamp_summary().clamped_by_fht_dke_only());
        assert!(
            !digest
                .kv_prefetch_clamp_summary()
                .clamped_by_runtime_and_fht_dke()
        );
        assert_eq!(digest.fht_dke_budget.route_pressure, 0.90);
        assert_eq!(
            digest.fht_dke_summary(),
            digest.fht_dke_budget.budget_summary()
        );
        assert_eq!(digest.fht_dke_summary().route_pressure, 0.90);
        assert!(digest.fht_dke_summary().token_split_is_valid);
        assert!(digest.fht_dke_summary().has_kv_exchange);
        let summary = digest.planning_summary();
        assert_eq!(summary.generation_budget, digest.generation_budget);
        assert!(summary.context_limited);
        assert_eq!(summary.backend_max_tokens, 124);
        assert_eq!(summary.adapter_selection.adapter, RuntimeAdapter::Cuda);
        assert_eq!(
            summary.adapter_fallback_reason,
            AdapterFallbackReason::NoFallback
        );
        assert_eq!(summary.allowed_adapter_count, 2);
        assert_eq!(summary.observation_count, 2);
        assert_eq!(summary.matching_observation_count, 2);
        assert_eq!(summary.matched_observation_fraction, 1.0);
        assert!(!summary.adapter_used_fallback());
        assert!(summary.adapter_selection_from_observation());
        assert!(!summary.adapter_missing_allowed_candidates());
        assert!(summary.has_matching_adapter_observation());
        assert!(!summary.adapter_observations_all_rejected());
        assert!(!summary.adapter_observations_missing());
        assert!(!summary.adapter_observation_gap());
        assert!(!summary.adapter_fallback_due_to_no_allowed_adapter());
        assert!(!summary.adapter_fallback_due_to_no_matching_observation());
        assert!(!summary.adapter_selection_blocked());
        assert_eq!(summary.adapter_selection_blocker_component_count(), 0);
        assert_eq!(summary.adapter_observation_signal_component_count(), 0);
        assert_eq!(summary.adapter_planning_signal_component_count(), 0);
        assert!(!summary.has_adapter_planning_signals());
        assert!(!summary.context_exhausted());
        assert!(summary.context_soft_limited());
        assert_eq!(summary.fht_dke, digest.fht_dke_summary());
        assert_eq!(summary.kv_exchange, digest.planned_kv_exchange());
        assert_eq!(summary.kv_clamp, digest.kv_prefetch_clamp_summary());
        assert!(summary.kv_clamp_is_consistent());
        assert!(summary.kv_prefetch_was_clamped());
        assert!(!summary.fht_dke_limited_kv_prefetch());
        assert!(summary.route_pressure_is_active());
        assert!(summary.route_pressure_is_high());
        assert!(summary.has_routed_kv_exchange());
        assert_eq!(summary.route_pressure_signal_component_count(), 1);
        assert_eq!(summary.high_route_pressure_signal_component_count(), 1);
        assert_eq!(summary.routed_kv_exchange_signal_component_count(), 1);
        assert_eq!(summary.fht_dke_budget_shape_problem_component_count(), 0);
        assert_eq!(summary.fht_dke_budget_commit_signal_component_count(), 5);
        assert!(summary.has_fht_dke_budget_commit_signals());
        assert_eq!(summary.fht_dke_budget_commit_blocker_component_count(), 0);
        assert!(!summary.has_fht_dke_budget_commit_blockers());
        assert!(summary.fht_dke_budget_commit_accounting_is_consistent());
        assert_eq!(summary.kv_clamp_consistency_problem_component_count(), 0);
        assert_eq!(summary.generation_readiness_blocker_component_count(), 0);
        assert_eq!(summary.parallelism_readiness_blocker_component_count(), 0);
        assert_eq!(summary.fht_dke_token_split_blocker_component_count(), 0);
        assert_eq!(summary.request_readiness_blocker_component_count(), 0);
        assert!(!summary.has_request_readiness_blockers());
        assert!(summary.request_readiness_accounting_is_consistent());
        assert_eq!(summary.pre_request_gate_problem_component_count(), 0);
        assert!(!summary.has_pre_request_gate_problem_components());
        assert!(summary.pre_request_gate_accounting_is_consistent());
        assert!(summary.pre_request_gate_shape_is_clean());
        assert_eq!(summary.planning_pressure_signal_component_count(), 6);
        assert_eq!(summary.pre_request_gate_signal_component_count(), 6);
        assert!(summary.has_pre_request_gate_signals());
        assert_eq!(summary.backend_request_commit_signal_component_count(), 6);
        assert!(summary.has_backend_request_commit_signals());
        assert_eq!(summary.backend_request_commit_blocker_component_count(), 0);
        assert!(!summary.has_backend_request_commit_blockers());
        assert!(summary.backend_request_commit_accounting_is_consistent());
        assert!(summary.backend_request_commit_is_clean());
        assert!(summary.can_commit_backend_request());
        assert!(summary.can_send_backend_request());
        assert_eq!(summary.requested_kv_prefetch_blocks, 8);
        assert_eq!(summary.runtime_kv_prefetch_blocks, 2);
        assert_eq!(summary.hardware_pressure, 0.70);
        assert_eq!(summary.compute_headroom, 0.30);
        assert_eq!(summary.max_parallel_chunks, 2);
        assert_eq!(summary.latency_budget_ms, None);
        assert!(summary.is_request_ready());
        assert!(digest.summary().contains("adapter=cuda"));
        assert!(digest.summary().contains("adapter_fallback=none"));
        assert!(digest.summary().contains("kv_clamp=runtime-metadata-limit"));

        let report = digest.acceptance_report();
        let acceptance_summary = report.acceptance_summary();
        assert!(report.is_accepted());
        assert!(acceptance_summary.accepted);
        assert_eq!(acceptance_summary.total_violation_count(), 0);
        assert!(!acceptance_summary.has_planning_violations());
        assert_eq!(acceptance_summary.contract_violation_count, 0);
        assert!(!acceptance_summary.has_context_exhaustion());
        assert!(!acceptance_summary.has_contract_failures());
        assert!(!acceptance_summary.has_failure_reports());
        assert!(!acceptance_summary.has_failures());
        assert!(acceptance_summary.accepted_state_matches_failures());
        assert_eq!(acceptance_summary.planning_violation_component_count(), 0);
        assert_eq!(acceptance_summary.context_exhaustion_component_count(), 0);
        assert_eq!(acceptance_summary.contract_failure_component_count(), 0);
        assert_eq!(
            acceptance_summary.mapped_failure_report_component_count(),
            0
        );
        assert_eq!(
            acceptance_summary.planning_acceptance_problem_component_count(),
            0
        );
        assert!(!acceptance_summary.has_planning_acceptance_problem_components());
        assert!(acceptance_summary.planning_acceptance_accounting_is_consistent());
        assert!(acceptance_summary.failure_report_matches_failures());
        assert_eq!(
            acceptance_summary.accepted_state_problem_component_count(),
            0
        );
        assert_eq!(
            acceptance_summary.failure_report_parity_problem_component_count(),
            0
        );
        assert_eq!(
            acceptance_summary.planning_acceptance_shape_problem_component_count(),
            0
        );
        assert!(!acceptance_summary.has_planning_acceptance_shape_problem_components());
        assert!(acceptance_summary.planning_acceptance_shape_accounting_is_consistent());
        assert!(acceptance_summary.is_clean_acceptance());
        assert!(acceptance_summary.planning_acceptance_shape_is_clean());
        assert!(acceptance_summary.can_accept_runtime_planning());
        assert_eq!(acceptance_summary.failure_report_count, 0);
        assert!(report.violations().is_empty());
        assert!(report.failure_reports().is_empty());
        assert_eq!(report.failure_batch_summary().total_count, 0);
        assert!(!report.failure_batch_summary().has_failures());
        assert!(!report.failure_batch_summary().can_format_runtime_failures());
        assert!(report.primary_failure_report().is_none());
        assert!(report.primary_failure_summary().is_none());
        let commit = report.commit_summary();
        assert_eq!(
            commit.action,
            RuntimePlanningAcceptanceCommitAction::CommitBackendRequest
        );
        assert!(commit.action_can_commit());
        assert!(!commit.action_should_return_failure());
        assert!(commit.can_commit_backend_request());
        assert!(!commit.should_return_runtime_failure());
        assert_eq!(commit.acceptance, acceptance_summary);
        assert!(commit.failure_reports.is_empty());
        assert_eq!(commit.primary_failure_report, None);
        assert_eq!(commit.primary_failure_summary, None);
        assert_eq!(commit.failure_batch.total_count, 0);
        assert_eq!(commit.failure_report_count, 0);
        assert!(!commit.can_format_runtime_failures);
        assert_eq!(commit.total_problem_component_count, 0);
        assert_eq!(commit.total_shape_problem_component_count, 0);
        assert!(commit.component_accounting_consistent);
        assert!(!commit.has_primary_failure_summary());
        assert!(commit.failure_batch_shape_is_clean());
        assert!(commit.commit_decision_accounting_is_consistent());
        let failure_return = commit.failure_return_summary();
        assert_eq!(
            failure_return.source,
            RuntimePlanningFailureReturnSource::PlanningAcceptance
        );
        assert_eq!(failure_return.source.label(), "runtime_planning_acceptance");
        assert!(!failure_return.has_failure_reports());
        assert!(!failure_return.has_problem_components());
        assert!(failure_return.failure_return_accounting_is_consistent());
        assert!(!failure_return.can_return_runtime_failure());
        assert_eq!(commit.runtime_failure_return_report(), None);
    }

    #[test]
    fn planning_digest_reports_runtime_and_fht_dke_kv_prefetch_limits() {
        let runtime = RuntimeMetadata::new("planning", "tok", 4096, 1024)
            .with_kv_exchange(true, true)
            .with_kv_limits(6, 6);
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(1024)
            .with_max_tokens(1024)
            .with_runtime(runtime)
            .with_experiments(ExperimentSwitches::default().with_fht_dke(true));
        let execution =
            AdapterExecutionContext::new([RuntimeAdapter::Cuda]).with_kv_prefetch_blocks(8);

        let digest = RuntimePlanningDigest::from_request(
            &request,
            RouteBudget {
                threshold: 0.45,
                attention_tokens: 9,
                fast_tokens: 1,
                attention_fraction: 0.90,
            },
            &execution,
            &[],
            &DeterministicFhtDkeBudgeter::new(0.10, 0.60, 1024),
        );
        let clamp = digest.kv_prefetch_clamp_summary();
        let summary = digest.planning_summary();

        assert_eq!(digest.requested_kv_prefetch_blocks, 8);
        assert_eq!(digest.runtime_kv_prefetch_blocks, 6);
        assert_eq!(digest.fht_dke_budget.kv_import_blocks, 2);
        assert_eq!(digest.planned_kv_import_blocks, 2);
        assert_eq!(
            digest.kv_prefetch_clamp_reason(),
            RuntimePlanningKvClampReason::RuntimeAndFhtDkeLimits
        );
        assert!(digest.kv_prefetch_was_clamped());
        assert!(clamp.has_runtime_metadata_clamp());
        assert!(clamp.has_fht_dke_clamp());
        assert_eq!(clamp.runtime_metadata_reduction, 2);
        assert_eq!(clamp.fht_dke_reduction, 4);
        assert_eq!(clamp.total_reduction, 6);
        assert_eq!(
            clamp.clamp_reason,
            RuntimePlanningKvClampReason::RuntimeAndFhtDkeLimits
        );
        assert!(clamp.is_consistent());
        assert!(!clamp.clamped_by_runtime_only());
        assert!(!clamp.clamped_by_fht_dke_only());
        assert!(clamp.clamped_by_runtime_and_fht_dke());
        assert_eq!(
            digest.planned_kv_exchange(),
            RuntimePlanningKvExchange {
                import_blocks: 2,
                export_blocks: 2,
                prefetch_was_clamped: true,
                clamp_reason: RuntimePlanningKvClampReason::RuntimeAndFhtDkeLimits,
            }
        );
        assert_eq!(summary.kv_clamp, clamp);
        assert!(summary.kv_prefetch_was_clamped());
        assert!(summary.fht_dke_limited_kv_prefetch());
        assert!(summary.has_routed_kv_exchange());
        assert_eq!(summary.kv_clamp_consistency_problem_component_count(), 0);
        assert!(summary.can_commit_backend_request());
        assert!(digest.is_valid());
    }

    #[test]
    fn planning_digest_preserves_router_generated_high_route_pressure() {
        let router = DefaultHierarchicalRouter::new();
        let tokens = [TokenFeatures::new("borderline", 0.66, 0)];
        let routing_context = RoutingContext {
            hierarchy: HierarchyWeights::new(1.0, 0.0, 0.0),
            ..RoutingContext::default()
        };
        let decisions = router.route_many(&tokens, routing_context);
        let route_budget = router.budget(&tokens, routing_context);
        let runtime = RuntimeMetadata::new("planning", "tok", 2048, 128)
            .with_kv_exchange(true, true)
            .with_kv_limits(16, 16);
        let request = InferenceRequest::new("prompt", TaskProfile::General)
            .with_prompt_tokens(512)
            .with_max_tokens(128)
            .with_runtime(runtime)
            .with_experiments(ExperimentSwitches::default().with_fht_dke(true));
        let execution =
            AdapterExecutionContext::new([RuntimeAdapter::Cuda]).with_kv_prefetch_blocks(8);

        let digest = RuntimePlanningDigest::from_request(
            &request,
            route_budget,
            &execution,
            &[],
            &DeterministicFhtDkeBudgeter::new(0.10, 0.60, 128),
        );
        let fht_dke = digest.fht_dke_summary();
        let summary = digest.planning_summary();

        assert_eq!(decisions[0].layer, RouteLayer::LocalWindow);
        assert_eq!(route_budget.fast_tokens, 0);
        assert_eq!(route_budget.attention_tokens, 1);
        assert_eq!(route_budget.attention_fraction, 1.0);
        assert_eq!(fht_dke.attention_threshold, route_budget.threshold);
        assert_eq!(fht_dke.route_pressure, route_budget.attention_fraction);
        assert!(fht_dke.route_pressure_is_high());
        assert!(fht_dke.has_routed_work());
        assert!(fht_dke.can_use_fht_dke_budget());
        assert_eq!(
            summary.fht_dke.route_pressure,
            route_budget.attention_fraction
        );
        assert!(summary.route_pressure_is_active());
        assert!(summary.route_pressure_is_high());
        assert!(summary.fht_dke_limited_kv_prefetch());
        assert_eq!(
            digest.kv_prefetch_clamp_reason(),
            RuntimePlanningKvClampReason::FhtDkeBudgetLimit
        );
        assert!(digest.is_valid());
        assert!(summary.can_send_backend_request());
    }

    #[test]
    fn runtime_planning_readiness_commits_router_threshold_through_fht_dke_boundary() {
        let router = DefaultHierarchicalRouter::new();
        let tokens = [
            TokenFeatures::new("fast", 0.02, 0),
            TokenFeatures::new("local", 0.66, 1),
            TokenFeatures::new("global", 0.96, 2),
        ];
        let routing_context = RoutingContext {
            profile: TaskProfile::Coding,
            hierarchy: HierarchyWeights::new(1.0, 0.0, 0.0),
            ..RoutingContext::default()
        };
        let decisions = router.route_many(&tokens, routing_context);
        let decision_summary = RoutingDecisionSummary::from_decisions(
            router.threshold_for(TaskProfile::Coding),
            &decisions,
        );
        let route_readiness =
            RouteBudgetReadinessSummary::new(decision_summary, decision_summary.route_budget());
        let route_commit = route_readiness.commit_summary();
        let route_budget = route_commit
            .committed_route_budget
            .expect("router budget readiness commits a concrete budget");
        let candidates = decisions
            .iter()
            .enumerate()
            .map(|(position, decision)| AttentionCandidate::from_route(decision, position, 0.40))
            .collect::<Vec<_>>();
        let candidate_batch = AttentionCandidateBatchSummary::from_candidates(&candidates);
        let attention_decision = AttentionDecisionSummary {
            threshold: route_budget.threshold,
            max_selected: candidates.len(),
            candidate_count: candidates.len(),
            selected_count: route_budget.attention_tokens,
            rejected_count: route_budget.fast_tokens,
            selection_fraction: route_budget.attention_fraction,
            hit_selection_cap: false,
            selected_layer_counts: RouteLayerCounts {
                fast_projection: 0,
                local_window: decision_summary.layer_counts.local_window,
                global: decision_summary.layer_counts.global,
                fusion: decision_summary.layer_counts.fusion,
            },
            rejected_layer_counts: RouteLayerCounts {
                fast_projection: decision_summary.layer_counts.fast_projection,
                local_window: 0,
                global: 0,
                fusion: 0,
            },
        };
        let attention_readiness =
            AttentionSelectionReadinessSummary::new(candidate_batch, attention_decision);
        let transformer_summary = TransformerPlanSummary {
            layer_count: 3,
            counts: TransformerPlanCounts {
                global: 1,
                local: 1,
                fusion: 0,
            },
            average_compute_fraction: 0.58,
            min_window_size: 256,
            max_window_size: 4096,
        };
        let transformer_pressure = TransformerPlanningPressureSummary::from_parts(
            route_budget,
            attention_decision,
            transformer_summary,
        );
        let transformer_readiness = TransformerPlanningReadinessSummary::new(
            route_readiness,
            attention_readiness,
            transformer_pressure,
        );
        let runtime = RuntimeMetadata::new("planning", "tok", 2048, 256)
            .with_kv_exchange(true, true)
            .with_kv_limits(8, 8);
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(512)
            .with_max_tokens(128)
            .with_runtime(runtime)
            .with_experiments(ExperimentSwitches::default().with_fht_dke(true));
        let execution = AdapterExecutionContext::new([RuntimeAdapter::Cuda])
            .with_kv_prefetch_blocks(4)
            .with_parallel_chunks(2);
        let digest = RuntimePlanningDigest::from_request(
            &request,
            route_budget,
            &execution,
            &[],
            &DeterministicFhtDkeBudgeter::new(0.10, 0.60, 256),
        );
        let runtime_planning = digest.planning_summary();
        let fht_dke_planning =
            FhtDkePlanningReadinessSummary::new(transformer_readiness, runtime_planning.fht_dke);
        let runtime_readiness =
            RuntimePlanningReadinessSummary::new(fht_dke_planning, runtime_planning);

        assert_eq!(decisions[0].layer, RouteLayer::FastProjection);
        assert_eq!(decisions[1].layer, RouteLayer::LocalWindow);
        assert_eq!(decisions[2].layer, RouteLayer::Global);
        assert!(route_commit.can_use_committed_route_budget());
        assert_eq!(
            route_budget.threshold,
            router.threshold_for(TaskProfile::Coding)
        );
        assert_eq!(route_budget, decision_summary.route_budget());
        assert!(transformer_readiness.can_commit_transformer_planning_readiness());
        assert_eq!(
            transformer_readiness
                .planning_pressure
                .route_attention_fraction,
            route_budget.attention_fraction
        );
        assert!(fht_dke_planning.can_commit_fht_dke_planning_readiness());
        assert!(fht_dke_planning.route_pressure_matches_budget());
        assert!(fht_dke_planning.attention_threshold_matches_budget());
        assert_eq!(
            runtime_planning.fht_dke.route_pressure,
            route_budget.attention_fraction
        );
        assert_eq!(
            runtime_planning.fht_dke.attention_threshold,
            route_budget.threshold
        );
        assert_eq!(runtime_readiness.first_unready_stage(), None);
        assert_eq!(runtime_readiness.first_blocking_stage(), None);
        assert!(runtime_readiness.fht_dke_runtime_boundary_ready());
        assert!(runtime_readiness.runtime_planning_readiness_accounting_is_consistent());
        assert!(runtime_readiness.can_commit_runtime_planning_readiness());
    }

    #[test]
    fn runtime_planning_readiness_blocks_stale_low_pressure_route_budget_after_hardware_demote() {
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
        let low_pressure_summary = RoutingDecisionSummary::from_decisions(
            router.threshold_for(TaskProfile::General),
            &[low_pressure_decision.clone()],
        );
        let low_route_readiness =
            RouteBudgetReadinessSummary::new(low_pressure_summary, low_pressure_budget);
        let low_candidate = AttentionCandidate::from_route(&low_pressure_decision, 0, 0.80);
        let low_attention_decision = AttentionDecisionSummary {
            threshold: low_pressure_budget.threshold,
            max_selected: 1,
            candidate_count: 1,
            selected_count: 1,
            rejected_count: 0,
            selection_fraction: 1.0,
            hit_selection_cap: false,
            selected_layer_counts: RouteLayerCounts {
                fast_projection: 0,
                local_window: 1,
                global: 0,
                fusion: 0,
            },
            rejected_layer_counts: RouteLayerCounts::default(),
        };
        let low_attention_readiness = AttentionSelectionReadinessSummary::new(
            AttentionCandidateBatchSummary::from_candidates(&[low_candidate]),
            low_attention_decision,
        );
        let low_transformer_summary = TransformerPlanSummary {
            layer_count: 1,
            counts: TransformerPlanCounts {
                global: 0,
                local: 1,
                fusion: 0,
            },
            average_compute_fraction: 0.50,
            min_window_size: 256,
            max_window_size: 2048,
        };
        let low_transformer_pressure = TransformerPlanningPressureSummary::from_parts(
            low_pressure_budget,
            low_attention_decision,
            low_transformer_summary,
        );
        let low_transformer_readiness = TransformerPlanningReadinessSummary::new(
            low_route_readiness,
            low_attention_readiness,
            low_transformer_pressure,
        );
        let runtime = RuntimeMetadata::new("planning", "tok", 2048, 128)
            .with_kv_exchange(true, true)
            .with_kv_limits(8, 8);
        let switches = ExperimentSwitches::default().with_fht_dke(true);
        let request = InferenceRequest::new("prompt", TaskProfile::General)
            .with_prompt_tokens(512)
            .with_max_tokens(128)
            .with_runtime(runtime.clone())
            .with_experiments(switches);
        let budgeter = DeterministicFhtDkeBudgeter::new(0.10, 0.60, 128);
        let low_fht_dke_budget = budgeter
            .budget(
                &FhtDkeInput::new(512, 128, runtime)
                    .with_experiments(switches)
                    .with_route_budget(low_pressure_budget),
            )
            .budget_summary();
        let low_fht_dke_planning =
            FhtDkePlanningReadinessSummary::new(low_transformer_readiness, low_fht_dke_budget);
        let high_pressure_execution = AdapterExecutionContext::new([RuntimeAdapter::Cuda])
            .with_pressure(1.0, 0.20)
            .with_kv_prefetch_blocks(4);
        let digest = RuntimePlanningDigest::from_request(
            &request,
            high_pressure_budget,
            &high_pressure_execution,
            &[],
            &budgeter,
        );
        let runtime_planning = digest.planning_summary();
        let readiness =
            RuntimePlanningReadinessSummary::new(low_fht_dke_planning, runtime_planning);

        assert_eq!(low_pressure_decision.layer, RouteLayer::LocalWindow);
        assert_eq!(high_pressure_decision.layer, RouteLayer::FastProjection);
        assert!(low_pressure_budget.has_attention_pressure());
        assert_eq!(low_pressure_budget.attention_tokens, 1);
        assert_eq!(low_pressure_budget.fast_tokens, 0);
        assert!(!high_pressure_budget.has_attention_pressure());
        assert_eq!(high_pressure_budget.attention_tokens, 0);
        assert_eq!(high_pressure_budget.fast_tokens, 1);
        assert!(low_fht_dke_planning.can_commit_fht_dke_planning_readiness());
        assert!(low_fht_dke_planning.route_pressure_matches_budget());
        assert!(low_fht_dke_planning.attention_threshold_matches_budget());
        assert_eq!(
            runtime_planning.fht_dke.route_pressure,
            high_pressure_budget.attention_fraction
        );
        assert_eq!(runtime_planning.hardware_pressure, 1.0);
        assert!(runtime_planning.can_commit_backend_request());
        assert!(!readiness.fht_dke_runtime_boundary_ready());
        assert!(!readiness.fht_dke_runtime_boundary_matches());
        assert!(readiness.fht_dke_runtime_boundary_drift_component_count() >= 1);
        assert_eq!(
            readiness.first_unready_stage(),
            Some(RuntimePlanningReadinessStage::FhtDkeRuntimeBoundary)
        );
        assert_eq!(
            readiness.first_blocking_stage(),
            Some(RuntimePlanningReadinessStage::FhtDkeRuntimeBoundary)
        );
        assert_eq!(readiness.fht_dke_runtime_boundary_signal_component_count, 0);
        assert!(readiness.fht_dke_runtime_boundary_blocker_component_count >= 1);
        assert!(readiness.has_runtime_planning_readiness_blockers());
        assert!(readiness.runtime_planning_readiness_accounting_is_consistent());
        assert!(!readiness.runtime_planning_readiness_is_clean());
        assert!(!readiness.can_commit_runtime_planning_readiness());
    }

    #[test]
    fn planning_manifest_kv_bridge_summary_confirms_manifest_plans_match_runtime_planning() {
        let runtime = RuntimeMetadata::new("planning", "tok", 1024, 128)
            .with_kv_exchange(true, true)
            .with_kv_limits(2, 4);
        let manifest = RuntimeManifestDigest::from_metadata(runtime.clone())
            .with_architecture(TransformerRuntimeArchitecture::new(6, 128, 4, 2, 256))
            .with_kv_policy(RuntimeKvPolicy::import_export().with_limits(2, 4));
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(900)
            .with_max_tokens(256)
            .with_runtime(runtime)
            .with_experiments(ExperimentSwitches::default().with_fht_dke(true));
        let execution =
            AdapterExecutionContext::new([RuntimeAdapter::Cuda, RuntimeAdapter::CpuSimd])
                .with_pressure(0.70, 0.30)
                .with_parallel_chunks(2)
                .with_kv_prefetch_blocks(8);
        let observations = [
            AdapterObservation::new(RuntimeAdapter::CpuSimd, 0.40, 0.5, 0.5, None, None, 7),
            AdapterObservation::new(RuntimeAdapter::Cuda, 0.90, 0.8, 0.9, None, None, 8),
        ];

        let digest = RuntimePlanningDigest::from_request(
            &request,
            RouteBudget {
                threshold: 0.45,
                attention_tokens: 9,
                fast_tokens: 1,
                attention_fraction: 0.90,
            },
            &execution,
            &observations,
            &DeterministicFhtDkeBudgeter::new(0.10, 0.60, 128),
        );
        let bridge = digest.manifest_kv_bridge_summary(&manifest);

        assert_eq!(bridge.planned_import_blocks, 2);
        assert_eq!(bridge.planned_export_blocks, 4);
        assert_eq!(bridge.import.import_plan_max_blocks, 2);
        assert_eq!(bridge.export.export_plan_max_blocks, 4);
        assert!(bridge.import_bridge_is_clean());
        assert!(bridge.export_bridge_is_clean());
        assert!(bridge.import_plan_matches_planning());
        assert!(bridge.export_plan_matches_planning());
        assert!(bridge.manifest_plans_match_planning());
        assert_eq!(bridge.planning_import_drift_blocks(), 0);
        assert_eq!(bridge.planning_export_drift_blocks(), 0);
        assert_eq!(bridge.planning_kv_drift_blocks(), 0);
        assert_eq!(bridge.planning_kv_activity_signal_component_count(), 2);
        assert_eq!(bridge.manifest_kv_bridge_signal_component_count(), 19);
        assert!(bridge.has_manifest_kv_bridge_signals());
        assert_eq!(bridge.planning_kv_drift_component_count(), 0);
        assert_eq!(bridge.manifest_bridge_problem_component_count(), 0);
        assert_eq!(bridge.manifest_kv_bridge_problem_component_count(), 0);
        assert!(!bridge.has_manifest_kv_bridge_problem_components());
        assert!(bridge.manifest_kv_bridge_accounting_is_consistent());
        assert!(bridge.manifest_kv_bridge_shape_is_clean());
        assert!(bridge.can_use_runtime_planning_manifest_kv_bridge());
    }

    #[test]
    fn planning_manifest_kv_bridge_summary_reports_manifest_policy_and_planning_drift() {
        let runtime = RuntimeMetadata::new("planning", "tok", 1024, 128)
            .with_kv_exchange(true, true)
            .with_kv_limits(2, 4);
        let manifest = RuntimeManifestDigest::from_metadata(runtime.clone())
            .with_architecture(TransformerRuntimeArchitecture::new(6, 128, 4, 2, 256))
            .with_kv_policy(RuntimeKvPolicy {
                import_enabled: true,
                export_enabled: true,
                max_import_blocks: 1,
                max_export_blocks: 0,
            });
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(900)
            .with_max_tokens(256)
            .with_runtime(runtime)
            .with_experiments(ExperimentSwitches::default().with_fht_dke(true));
        let execution =
            AdapterExecutionContext::new([RuntimeAdapter::Cuda, RuntimeAdapter::CpuSimd])
                .with_pressure(0.70, 0.30)
                .with_parallel_chunks(2)
                .with_kv_prefetch_blocks(8);
        let observations = [
            AdapterObservation::new(RuntimeAdapter::CpuSimd, 0.40, 0.5, 0.5, None, None, 7),
            AdapterObservation::new(RuntimeAdapter::Cuda, 0.90, 0.8, 0.9, None, None, 8),
        ];

        let digest = RuntimePlanningDigest::from_request(
            &request,
            RouteBudget {
                threshold: 0.45,
                attention_tokens: 9,
                fast_tokens: 1,
                attention_fraction: 0.90,
            },
            &execution,
            &observations,
            &DeterministicFhtDkeBudgeter::new(0.10, 0.60, 128),
        );
        let bridge = digest.manifest_kv_bridge_summary(&manifest);

        assert_eq!(bridge.planned_import_blocks, 2);
        assert_eq!(bridge.planned_export_blocks, 4);
        assert_eq!(bridge.import.import_plan_max_blocks, 1);
        assert_eq!(bridge.export.export_plan_max_blocks, 0);
        assert!(bridge.import_bridge_is_clean());
        assert!(!bridge.export_bridge_is_clean());
        assert!(!bridge.import_plan_matches_planning());
        assert!(!bridge.export_plan_matches_planning());
        assert!(!bridge.manifest_plans_match_planning());
        assert_eq!(bridge.planning_import_drift_blocks(), 1);
        assert_eq!(bridge.planning_export_drift_blocks(), 4);
        assert_eq!(bridge.planning_kv_drift_blocks(), 5);
        assert_eq!(bridge.planning_kv_activity_signal_component_count(), 2);
        assert_eq!(bridge.manifest_kv_bridge_signal_component_count(), 17);
        assert!(bridge.has_manifest_kv_bridge_signals());
        assert_eq!(bridge.import_plan_planning_drift_component_count(), 1);
        assert_eq!(bridge.export_plan_planning_drift_component_count(), 1);
        assert_eq!(bridge.planning_kv_drift_component_count(), 2);
        assert_eq!(bridge.manifest_bridge_problem_component_count(), 2);
        assert_eq!(bridge.manifest_kv_bridge_problem_component_count(), 4);
        assert!(bridge.has_manifest_kv_bridge_problem_components());
        assert!(bridge.manifest_kv_bridge_accounting_is_consistent());
        assert!(!bridge.manifest_kv_bridge_shape_is_clean());
        assert!(!bridge.can_use_runtime_planning_manifest_kv_bridge());
    }

    #[test]
    fn planning_digest_reports_exhausted_context() {
        let runtime = RuntimeMetadata::new("planning", "tok", 128, 128);
        let request = InferenceRequest::new("prompt", TaskProfile::General)
            .with_prompt_tokens(128)
            .with_max_tokens(16)
            .with_runtime(runtime);
        let execution = AdapterExecutionContext::new([RuntimeAdapter::PortableRust]);

        let digest = RuntimePlanningDigest::from_request(
            &request,
            RouteBudget::default(),
            &execution,
            &[],
            &DeterministicFhtDkeBudgeter::default(),
        );
        let joined = digest.contract_violations().join("\n");

        assert!(!digest.is_valid());
        assert_eq!(digest.generation_budget.max_generated_tokens, 0);
        assert!(digest.context_limited());
        assert!(joined.contains("runtime planning has no generation room"));

        let report = digest.acceptance_report();
        let summary = report.acceptance_summary();
        let failures = report.failure_reports();
        let failure_batch = report.failure_batch_summary();
        let primary_summary = report.primary_failure_summary().unwrap();

        assert!(!report.is_accepted());
        assert!(!summary.accepted);
        assert_eq!(summary.planning_violation_count, 1);
        assert_eq!(summary.total_violation_count(), 1);
        assert!(summary.has_planning_violations());
        assert_eq!(summary.contract_violation_count, 0);
        assert!(summary.has_context_exhaustion());
        assert!(!summary.has_contract_failures());
        assert!(summary.has_failure_reports());
        assert!(summary.has_failures());
        assert!(summary.accepted_state_matches_failures());
        assert_eq!(summary.planning_violation_component_count(), 1);
        assert_eq!(summary.context_exhaustion_component_count(), 1);
        assert_eq!(summary.contract_failure_component_count(), 0);
        assert_eq!(summary.mapped_failure_report_component_count(), 1);
        assert_eq!(summary.planning_acceptance_problem_component_count(), 3);
        assert!(summary.has_planning_acceptance_problem_components());
        assert!(summary.planning_acceptance_accounting_is_consistent());
        assert!(summary.failure_report_matches_failures());
        assert_eq!(summary.accepted_state_problem_component_count(), 0);
        assert_eq!(summary.failure_report_parity_problem_component_count(), 0);
        assert_eq!(
            summary.planning_acceptance_shape_problem_component_count(),
            0
        );
        assert!(summary.planning_acceptance_shape_accounting_is_consistent());
        assert!(!summary.is_clean_acceptance());
        assert!(!summary.planning_acceptance_shape_is_clean());
        assert!(!summary.can_accept_runtime_planning());
        assert_eq!(summary.failure_report_count, failures.len());
        assert_eq!(failures.len(), 1);
        assert_eq!(
            failure_batch,
            RuntimeFailureReport::batch_summary(&failures)
        );
        assert_eq!(failure_batch.total_count, 1);
        assert_eq!(failure_batch.context_exhausted_count, 1);
        assert_eq!(failure_batch.contract_violation_count, 0);
        assert_eq!(failure_batch.backend_error_count, 1);
        assert!(!failure_batch.has_recoverable_failures());
        assert!(failure_batch.has_contract_failures());
        assert!(failure_batch.failure_batch_shape_is_clean());
        assert!(failure_batch.can_format_runtime_failures());
        assert_eq!(failures[0].kind, RuntimeFailureKind::ContextExhausted);
        assert!(failures[0].message.contains("leaves no generation room"));
        assert_eq!(report.primary_failure_report(), Some(failures[0].clone()));
        assert_eq!(primary_summary.kind, RuntimeFailureKind::ContextExhausted);
        assert_eq!(primary_summary.trace_label, "runtime_context_exhausted");
        assert!(!primary_summary.recoverable);
        assert!(primary_summary.backend_error);
        assert!(primary_summary.failure_summary_shape_is_clean());
        assert!(primary_summary.can_use_runtime_failure_report());
        let commit = report.commit_summary();
        assert_eq!(
            commit.action,
            RuntimePlanningAcceptanceCommitAction::ReturnRuntimeFailure
        );
        assert!(!commit.action_can_commit());
        assert!(commit.action_should_return_failure());
        assert!(!commit.can_commit_backend_request());
        assert!(commit.should_return_runtime_failure());
        assert_eq!(commit.acceptance, summary);
        assert_eq!(commit.failure_reports, failures);
        assert_eq!(commit.primary_failure_report, Some(failures[0].clone()));
        assert_eq!(commit.primary_failure_summary, Some(primary_summary));
        assert_eq!(commit.failure_batch, failure_batch);
        assert_eq!(commit.failure_report_count, 1);
        assert!(commit.can_format_runtime_failures);
        assert_eq!(commit.total_problem_component_count, 3);
        assert_eq!(commit.total_shape_problem_component_count, 0);
        assert!(commit.component_accounting_consistent);
        assert!(commit.has_primary_failure_summary());
        assert!(commit.failure_batch_shape_is_clean());
        assert!(commit.commit_decision_accounting_is_consistent());
        let failure_return = commit.failure_return_summary();
        assert_eq!(
            failure_return.source,
            RuntimePlanningFailureReturnSource::PlanningAcceptance
        );
        assert!(failure_return.has_failure_reports());
        assert!(failure_return.has_problem_components());
        assert!(failure_return.failure_return_accounting_is_consistent());
        assert!(failure_return.can_return_runtime_failure());
        let return_report = commit
            .runtime_failure_return_report()
            .expect("context planning return report");
        assert_eq!(
            return_report.source,
            RuntimePlanningFailureReturnSource::PlanningAcceptance
        );
        assert_eq!(return_report.primary_failure_summary, primary_summary);
        assert_eq!(return_report.failure_batch.context_exhausted_count, 1);
        assert!(return_report.failure_return_report_shape_is_clean());
        assert!(return_report.can_use_runtime_planning_failure_return_report());
        assert!(
            return_report
                .backend_message()
                .contains("leaves no generation room")
        );
        assert!(
            return_report
                .diagnostics_note()
                .starts_with("runtime_context_exhausted")
        );
        assert_eq!(
            return_report.inference_error().message,
            return_report.backend_message()
        );
    }

    #[test]
    fn planning_acceptance_returns_context_exhausted_for_zero_requested_tokens_at_full_context() {
        let runtime = RuntimeMetadata::new("planning", "tok", 128, 128);
        let request = InferenceRequest::new("prompt", TaskProfile::General)
            .with_prompt_tokens(128)
            .with_max_tokens(0)
            .with_runtime(runtime);
        let execution = AdapterExecutionContext::new([RuntimeAdapter::PortableRust]);

        let digest = RuntimePlanningDigest::from_request(
            &request,
            RouteBudget::default(),
            &execution,
            &[],
            &DeterministicFhtDkeBudgeter::default(),
        );
        let report = digest.acceptance_report();
        let summary = report.acceptance_summary();
        let commit = report.commit_summary();
        let return_report = commit
            .runtime_failure_return_report()
            .expect("zero max-token context exhaustion report");

        assert_eq!(request.max_tokens, 1);
        assert_eq!(digest.generation_budget.requested_max_tokens, 1);
        assert_eq!(digest.generation_budget.requested_context_tokens, 129);
        assert_eq!(digest.generation_budget.planned_context_tokens, 128);
        assert_eq!(digest.generation_budget.max_generated_tokens, 0);
        assert!(digest.generation_budget.context_exhausted());
        assert!(!digest.is_valid());
        assert!(!report.is_accepted());
        assert!(summary.has_context_exhaustion());
        assert_eq!(summary.contract_violation_count, 0);
        assert_eq!(summary.failure_report_count, 1);
        assert_eq!(
            commit.action,
            RuntimePlanningAcceptanceCommitAction::ReturnRuntimeFailure
        );
        assert!(!commit.can_commit_backend_request());
        assert!(commit.should_return_runtime_failure());
        assert!(commit.failure_return_summary().can_return_runtime_failure());
        assert_eq!(
            return_report.primary_failure_summary.kind,
            RuntimeFailureKind::ContextExhausted
        );
        assert_eq!(return_report.failure_batch.context_exhausted_count, 1);
        assert!(return_report.can_use_runtime_planning_failure_return_report());
    }

    #[test]
    fn planning_acceptance_report_maps_contract_failures() {
        let runtime = RuntimeMetadata::new("planning", "tok", 128, 16)
            .with_kv_exchange(true, true)
            .with_kv_limits(1, 1);
        let request = InferenceRequest::new("prompt", TaskProfile::General)
            .with_prompt_tokens(8)
            .with_max_tokens(8)
            .with_runtime(runtime);
        let execution = AdapterExecutionContext::new([RuntimeAdapter::PortableRust]);
        let mut digest = RuntimePlanningDigest::from_request(
            &request,
            RouteBudget::default(),
            &execution,
            &[],
            &DeterministicFhtDkeBudgeter::default(),
        );
        digest.max_parallel_chunks = 0;

        let report = digest.acceptance_report();
        let summary = report.acceptance_summary();
        let failures = report.failure_reports();
        let failure_batch = report.failure_batch_summary();
        let primary_summary = report.primary_failure_summary().unwrap();

        assert!(!report.is_accepted());
        assert!(!summary.accepted);
        assert_eq!(summary.planning_violation_count, 1);
        assert_eq!(summary.total_violation_count(), 1);
        assert!(summary.has_planning_violations());
        assert_eq!(summary.contract_violation_count, 1);
        assert!(!summary.has_context_exhaustion());
        assert!(summary.has_contract_failures());
        assert!(summary.has_failure_reports());
        assert!(summary.has_failures());
        assert!(summary.accepted_state_matches_failures());
        assert_eq!(summary.planning_violation_component_count(), 1);
        assert_eq!(summary.context_exhaustion_component_count(), 0);
        assert_eq!(summary.contract_failure_component_count(), 1);
        assert_eq!(summary.mapped_failure_report_component_count(), 1);
        assert_eq!(summary.planning_acceptance_problem_component_count(), 3);
        assert!(summary.has_planning_acceptance_problem_components());
        assert!(summary.planning_acceptance_accounting_is_consistent());
        assert!(summary.failure_report_matches_failures());
        assert_eq!(summary.accepted_state_problem_component_count(), 0);
        assert_eq!(summary.failure_report_parity_problem_component_count(), 0);
        assert_eq!(
            summary.planning_acceptance_shape_problem_component_count(),
            0
        );
        assert!(summary.planning_acceptance_shape_accounting_is_consistent());
        assert!(!summary.is_clean_acceptance());
        assert!(!summary.planning_acceptance_shape_is_clean());
        assert!(!summary.can_accept_runtime_planning());
        assert_eq!(summary.failure_report_count, failures.len());
        assert!(
            report
                .violations()
                .iter()
                .any(|violation| violation.contains("max_parallel_chunks"))
        );
        assert_eq!(failures.len(), 1);
        assert_eq!(
            failure_batch,
            RuntimeFailureReport::batch_summary(&failures)
        );
        assert_eq!(failure_batch.total_count, 1);
        assert_eq!(failure_batch.context_exhausted_count, 0);
        assert_eq!(failure_batch.contract_violation_count, 1);
        assert_eq!(failure_batch.backend_error_count, 0);
        assert!(failure_batch.has_recoverable_failures());
        assert!(failure_batch.has_contract_failures());
        assert!(failure_batch.failure_batch_shape_is_clean());
        assert!(failure_batch.can_format_runtime_failures());
        assert_eq!(failures[0].kind, RuntimeFailureKind::ContractViolation);
        assert!(
            failures[0]
                .message
                .contains("runtime planning acceptance failed")
        );
        assert_eq!(primary_summary.kind, RuntimeFailureKind::ContractViolation);
        assert_eq!(primary_summary.trace_label, "runtime_contract_violation");
        assert!(primary_summary.recoverable);
        assert!(!primary_summary.backend_error);
        assert!(primary_summary.failure_summary_shape_is_clean());
        assert!(primary_summary.can_use_runtime_failure_report());
        let commit = report.commit_summary();
        assert_eq!(
            commit.action,
            RuntimePlanningAcceptanceCommitAction::ReturnRuntimeFailure
        );
        assert!(!commit.action_can_commit());
        assert!(commit.action_should_return_failure());
        assert!(!commit.can_commit_backend_request());
        assert!(commit.should_return_runtime_failure());
        assert_eq!(commit.acceptance, summary);
        assert_eq!(commit.failure_reports, failures);
        assert_eq!(commit.primary_failure_report, Some(failures[0].clone()));
        assert_eq!(commit.primary_failure_summary, Some(primary_summary));
        assert_eq!(commit.failure_batch, failure_batch);
        assert_eq!(commit.failure_report_count, 1);
        assert!(commit.can_format_runtime_failures);
        assert_eq!(commit.total_problem_component_count, 3);
        assert_eq!(commit.total_shape_problem_component_count, 0);
        assert!(commit.component_accounting_consistent);
        assert!(commit.has_primary_failure_summary());
        assert!(commit.failure_batch_shape_is_clean());
        assert!(commit.commit_decision_accounting_is_consistent());
        let failure_return = commit.failure_return_summary();
        assert_eq!(
            failure_return.source,
            RuntimePlanningFailureReturnSource::PlanningAcceptance
        );
        assert!(failure_return.has_failure_reports());
        assert!(failure_return.has_problem_components());
        assert!(failure_return.failure_return_accounting_is_consistent());
        assert!(failure_return.can_return_runtime_failure());
        let return_report = commit
            .runtime_failure_return_report()
            .expect("contract planning return report");
        assert_eq!(
            return_report.source,
            RuntimePlanningFailureReturnSource::PlanningAcceptance
        );
        assert_eq!(return_report.primary_failure_summary, primary_summary);
        assert_eq!(return_report.failure_batch.contract_violation_count, 1);
        assert!(return_report.failure_return_report_shape_is_clean());
        assert!(return_report.can_use_runtime_planning_failure_return_report());
        assert!(
            return_report
                .backend_message()
                .contains("runtime planning acceptance failed")
        );
        assert!(
            return_report
                .diagnostics_note()
                .starts_with("runtime_contract_violation")
        );
        assert_eq!(
            return_report.inference_error().message,
            return_report.backend_message()
        );
    }

    #[test]
    fn planning_acceptance_summary_counts_public_shape_drift() {
        let summary = RuntimePlanningAcceptanceSummary {
            accepted: true,
            planning_violation_count: 1,
            contract_violation_count: 0,
            context_exhausted: false,
            failure_report_count: 1,
        };

        assert!(summary.accepted);
        assert!(summary.has_planning_violations());
        assert!(!summary.has_context_exhaustion());
        assert!(!summary.has_contract_failures());
        assert!(summary.has_failure_reports());
        assert!(summary.has_failures());
        assert!(!summary.accepted_state_matches_failures());
        assert!(!summary.failure_report_matches_failures());
        assert_eq!(summary.planning_violation_component_count(), 1);
        assert_eq!(summary.context_exhaustion_component_count(), 0);
        assert_eq!(summary.contract_failure_component_count(), 0);
        assert_eq!(summary.mapped_failure_report_component_count(), 1);
        assert_eq!(summary.planning_acceptance_problem_component_count(), 2);
        assert!(summary.has_planning_acceptance_problem_components());
        assert!(summary.planning_acceptance_accounting_is_consistent());
        assert_eq!(summary.accepted_state_problem_component_count(), 1);
        assert_eq!(summary.failure_report_parity_problem_component_count(), 1);
        assert_eq!(
            summary.planning_acceptance_shape_problem_component_count(),
            2
        );
        assert!(summary.has_planning_acceptance_shape_problem_components());
        assert!(summary.planning_acceptance_shape_accounting_is_consistent());
        assert!(!summary.is_clean_acceptance());
        assert!(!summary.planning_acceptance_shape_is_clean());
        assert!(!summary.can_accept_runtime_planning());
    }

    #[test]
    fn planning_digest_reports_empty_allowed_adapter_candidates() {
        let runtime = RuntimeMetadata::new("planning", "tok", 128, 16);
        let request = InferenceRequest::new("prompt", TaskProfile::General)
            .with_prompt_tokens(8)
            .with_max_tokens(8)
            .with_runtime(runtime);
        let execution = AdapterExecutionContext::new(Vec::new());

        let digest = RuntimePlanningDigest::from_request(
            &request,
            RouteBudget::default(),
            &execution,
            &[],
            &DeterministicFhtDkeBudgeter::default(),
        );
        let report = digest.acceptance_report();
        let failures = report.failure_reports();

        assert_eq!(
            digest.adapter_selection.adapter,
            RuntimeAdapter::PortableRust
        );
        assert!(digest.adapter_used_fallback());
        assert_eq!(
            digest.adapter_fallback_reason(),
            AdapterFallbackReason::NoAllowedAdapter
        );
        assert_eq!(digest.adapter_selection_report.allowed_adapter_count, 0);
        assert_eq!(digest.adapter_selection_report.observation_count, 0);
        assert_eq!(
            digest.adapter_selection_report.matching_observation_count,
            0
        );
        assert_eq!(digest.matched_adapter_observation_fraction(), 0.0);
        let summary = digest.planning_summary();
        assert!(summary.adapter_used_fallback());
        assert!(!summary.adapter_selection_from_observation());
        assert!(summary.adapter_missing_allowed_candidates());
        assert!(!summary.has_matching_adapter_observation());
        assert!(!summary.adapter_observations_all_rejected());
        assert!(summary.adapter_observations_missing());
        assert!(summary.adapter_observation_gap());
        assert_eq!(
            summary.adapter_fallback_reason,
            AdapterFallbackReason::NoAllowedAdapter
        );
        assert!(summary.adapter_fallback_due_to_no_allowed_adapter());
        assert!(!summary.adapter_fallback_due_to_no_matching_observation());
        assert!(summary.adapter_selection_blocked());
        assert_eq!(summary.adapter_selection_blocker_component_count(), 1);
        assert_eq!(summary.adapter_observation_signal_component_count(), 1);
        assert_eq!(summary.adapter_planning_signal_component_count(), 2);
        assert!(summary.has_adapter_planning_signals());
        assert_eq!(summary.allowed_adapter_count, 0);
        assert_eq!(summary.observation_count, 0);
        assert_eq!(summary.matching_observation_count, 0);
        assert_eq!(summary.matched_observation_fraction, 0.0);
        assert!(!summary.kv_prefetch_was_clamped());
        assert!(!summary.fht_dke_limited_kv_prefetch());
        assert!(!summary.route_pressure_is_active());
        assert!(!summary.route_pressure_is_high());
        assert!(!summary.has_routed_kv_exchange());
        assert_eq!(summary.request_readiness_blocker_component_count(), 1);
        assert!(summary.has_request_readiness_blockers());
        assert!(summary.request_readiness_accounting_is_consistent());
        assert_eq!(summary.pre_request_gate_problem_component_count(), 1);
        assert!(summary.has_pre_request_gate_problem_components());
        assert!(summary.pre_request_gate_accounting_is_consistent());
        assert_eq!(summary.planning_pressure_signal_component_count(), 0);
        assert_eq!(summary.pre_request_gate_signal_component_count(), 1);
        assert!(summary.has_pre_request_gate_signals());
        assert!(!summary.is_request_ready());
        assert!(!digest.is_valid());
        assert!(
            report
                .violations()
                .iter()
                .any(|violation| violation.contains("no allowed adapter"))
        );
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].kind, RuntimeFailureKind::ContractViolation);
    }

    #[test]
    fn planning_digest_keeps_no_matching_adapter_observations_as_fallback_signal() {
        let runtime = RuntimeMetadata::new("planning", "tok", 128, 16);
        let request = InferenceRequest::new("prompt", TaskProfile::General)
            .with_prompt_tokens(8)
            .with_max_tokens(8)
            .with_runtime(runtime);
        let execution = AdapterExecutionContext::new([RuntimeAdapter::CpuSimd]);
        let observations = [AdapterObservation::new(
            RuntimeAdapter::Cuda,
            0.95,
            0.9,
            0.8,
            None,
            None,
            42,
        )];

        let digest = RuntimePlanningDigest::from_request(
            &request,
            RouteBudget::default(),
            &execution,
            &observations,
            &DeterministicFhtDkeBudgeter::default(),
        );
        let summary = digest.planning_summary();

        assert!(digest.is_valid());
        assert!(summary.adapter_used_fallback());
        assert!(!summary.adapter_selection_from_observation());
        assert!(!summary.adapter_missing_allowed_candidates());
        assert!(!summary.has_matching_adapter_observation());
        assert!(summary.adapter_observations_all_rejected());
        assert!(!summary.adapter_observations_missing());
        assert!(summary.adapter_observation_gap());
        assert!(!summary.adapter_fallback_due_to_no_allowed_adapter());
        assert!(summary.adapter_fallback_due_to_no_matching_observation());
        assert!(!summary.adapter_selection_blocked());
        assert_eq!(summary.adapter_selection_blocker_component_count(), 0);
        assert_eq!(summary.adapter_observation_signal_component_count(), 2);
        assert_eq!(summary.adapter_planning_signal_component_count(), 2);
        assert!(summary.has_adapter_planning_signals());
        assert_eq!(
            summary.adapter_fallback_reason,
            AdapterFallbackReason::NoMatchingObservation
        );
        assert_eq!(summary.request_readiness_blocker_component_count(), 0);
        assert!(!summary.has_request_readiness_blockers());
        assert!(summary.request_readiness_accounting_is_consistent());
        assert_eq!(summary.pre_request_gate_problem_component_count(), 0);
        assert!(!summary.has_pre_request_gate_problem_components());
        assert!(summary.pre_request_gate_accounting_is_consistent());
        assert_eq!(summary.pre_request_gate_signal_component_count(), 2);
        assert!(summary.has_pre_request_gate_signals());
        assert!(summary.is_request_ready());
    }

    #[test]
    fn planning_summary_counts_pre_request_gate_problem_components() {
        let summary = RuntimePlanningSummary {
            generation_budget: RuntimeGenerationBudget::new(2048, 128, 1024),
            context_limited: true,
            backend_max_tokens: 0,
            adapter_selection: AdapterSelection::fallback(RuntimeAdapter::PortableRust),
            adapter_fallback_reason: AdapterFallbackReason::NoAllowedAdapter,
            allowed_adapter_count: 0,
            observation_count: 0,
            matching_observation_count: 0,
            matched_observation_fraction: 0.0,
            fht_dke: FhtDkeBudgetSummary {
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
            },
            kv_exchange: RuntimePlanningKvExchange {
                import_blocks: 2,
                export_blocks: 2,
                prefetch_was_clamped: false,
                clamp_reason: RuntimePlanningKvClampReason::NotClamped,
            },
            kv_clamp: RuntimePlanningKvClampSummary {
                requested_kv_prefetch_blocks: 4,
                runtime_kv_prefetch_blocks: 2,
                planned_kv_import_blocks: 1,
                runtime_metadata_reduction: 0,
                fht_dke_reduction: 0,
                total_reduction: 0,
                prefetch_was_clamped: false,
                clamp_reason: RuntimePlanningKvClampReason::NotClamped,
            },
            requested_kv_prefetch_blocks: 4,
            runtime_kv_prefetch_blocks: 2,
            hardware_pressure: 0.25,
            compute_headroom: 0.75,
            max_parallel_chunks: 0,
            latency_budget_ms: Some(120),
        };

        assert!(summary.context_exhausted());
        assert!(summary.adapter_selection_blocked());
        assert!(!summary.kv_clamp_is_consistent());
        assert_eq!(summary.kv_clamp_consistency_problem_component_count(), 1);
        assert_eq!(summary.generation_readiness_blocker_component_count(), 1);
        assert_eq!(summary.parallelism_readiness_blocker_component_count(), 1);
        assert_eq!(summary.adapter_selection_blocker_component_count(), 1);
        assert_eq!(summary.fht_dke_token_split_blocker_component_count(), 1);
        assert_eq!(summary.fht_dke_budget_shape_problem_component_count(), 3);
        assert_eq!(summary.fht_dke_budget_commit_signal_component_count(), 4);
        assert!(summary.has_fht_dke_budget_commit_signals());
        assert_eq!(summary.fht_dke_budget_commit_blocker_component_count(), 3);
        assert!(summary.has_fht_dke_budget_commit_blockers());
        assert!(summary.fht_dke_budget_commit_accounting_is_consistent());
        assert_eq!(summary.request_readiness_blocker_component_count(), 4);
        assert!(summary.has_request_readiness_blockers());
        assert!(summary.request_readiness_accounting_is_consistent());
        assert_eq!(summary.pre_request_gate_problem_component_count(), 7);
        assert!(summary.has_pre_request_gate_problem_components());
        assert!(summary.pre_request_gate_accounting_is_consistent());
        assert!(!summary.pre_request_gate_shape_is_clean());
        assert_eq!(summary.route_pressure_signal_component_count(), 1);
        assert_eq!(summary.high_route_pressure_signal_component_count(), 1);
        assert_eq!(summary.routed_kv_exchange_signal_component_count(), 0);
        assert_eq!(summary.planning_pressure_signal_component_count(), 4);
        assert_eq!(summary.pre_request_gate_signal_component_count(), 5);
        assert!(summary.has_pre_request_gate_signals());
        assert_eq!(summary.backend_request_commit_signal_component_count(), 5);
        assert!(summary.has_backend_request_commit_signals());
        assert_eq!(summary.backend_request_commit_blocker_component_count(), 7);
        assert!(summary.has_backend_request_commit_blockers());
        assert!(summary.backend_request_commit_accounting_is_consistent());
        assert!(!summary.backend_request_commit_is_clean());
        assert!(!summary.is_request_ready());
        assert!(!summary.can_commit_backend_request());
        assert!(!summary.can_send_backend_request());
    }

    #[test]
    fn runtime_planning_readiness_confirms_fht_dke_to_runtime_boundary() {
        let runtime = RuntimeMetadata::new("planning", "tok", 4096, 2048)
            .with_kv_exchange(true, true)
            .with_kv_limits(8, 8);
        let request = InferenceRequest::new("prompt", TaskProfile::LongDocument)
            .with_prompt_tokens(1800)
            .with_max_tokens(200)
            .with_runtime(runtime.clone())
            .with_experiments(ExperimentSwitches::default().with_fht_dke(true));
        let route_budget = clean_route_budget();
        let execution = AdapterExecutionContext::new([RuntimeAdapter::Cuda])
            .with_kv_prefetch_blocks(8)
            .with_parallel_chunks(2);
        let observations = [AdapterObservation::new(
            RuntimeAdapter::Cuda,
            0.95,
            0.90,
            0.80,
            None,
            None,
            16,
        )];
        let budgeter = DeterministicFhtDkeBudgeter::default();
        let digest = RuntimePlanningDigest::from_request(
            &request,
            route_budget,
            &execution,
            &observations,
            &budgeter,
        );
        let runtime_planning = digest.planning_summary();
        let fht_dke_planning =
            clean_fht_dke_planning_readiness(route_budget, runtime_planning.fht_dke);
        let readiness = RuntimePlanningReadinessSummary::new(fht_dke_planning, runtime_planning);

        assert_eq!(
            RuntimePlanningReadinessSummary::stage_order(),
            [
                RuntimePlanningReadinessStage::FhtDkePlanning,
                RuntimePlanningReadinessStage::RuntimePreRequest,
                RuntimePlanningReadinessStage::FhtDkeRuntimeBoundary,
            ]
        );
        assert!(readiness.fht_dke_planning_ready());
        assert!(readiness.runtime_pre_request_ready());
        assert!(readiness.fht_dke_runtime_boundary_ready());
        assert!(readiness.fht_dke_runtime_boundary_matches());
        assert_eq!(
            readiness.fht_dke_runtime_boundary_drift_component_count(),
            0
        );
        assert_eq!(readiness.first_unready_stage(), None);
        assert_eq!(readiness.first_blocking_stage(), None);
        assert_eq!(
            readiness.stage_signal_component_count(RuntimePlanningReadinessStage::FhtDkePlanning),
            readiness.fht_dke_planning_signal_component_count
        );
        assert_eq!(
            readiness
                .stage_blocker_component_count(RuntimePlanningReadinessStage::RuntimePreRequest),
            readiness.runtime_pre_request_blocker_component_count
        );
        assert_eq!(readiness.fht_dke_runtime_boundary_signal_component_count, 1);
        assert_eq!(
            readiness.fht_dke_runtime_boundary_blocker_component_count,
            0
        );
        assert_eq!(
            readiness.runtime_planning_readiness_signal_component_count(),
            readiness
                .fht_dke_planning_signal_component_count
                .saturating_add(readiness.runtime_pre_request_signal_component_count)
                .saturating_add(readiness.fht_dke_runtime_boundary_signal_component_count)
        );
        assert_eq!(
            readiness.runtime_planning_readiness_blocker_component_count(),
            0
        );
        assert!(readiness.has_runtime_planning_readiness_signals());
        assert!(!readiness.has_runtime_planning_readiness_blockers());
        assert!(readiness.runtime_planning_readiness_accounting_is_consistent());
        assert!(readiness.runtime_planning_readiness_is_clean());
        assert!(readiness.can_commit_runtime_planning_readiness());
    }

    #[test]
    fn runtime_planning_readiness_preserves_context_clamped_fht_dke_budget() {
        let runtime = RuntimeMetadata::new("planning", "tok", 1024, 2048)
            .with_kv_exchange(true, true)
            .with_kv_limits(8, 8);
        let request = InferenceRequest::new("prompt", TaskProfile::LongDocument)
            .with_prompt_tokens(900)
            .with_max_tokens(256)
            .with_runtime(runtime)
            .with_experiments(ExperimentSwitches::default().with_fht_dke(true));
        let route_budget = clean_route_budget();
        let execution = AdapterExecutionContext::new([RuntimeAdapter::Cuda])
            .with_kv_prefetch_blocks(8)
            .with_parallel_chunks(2);
        let observations = [AdapterObservation::new(
            RuntimeAdapter::Cuda,
            0.95,
            0.90,
            0.80,
            None,
            None,
            16,
        )];

        let digest = RuntimePlanningDigest::from_request(
            &request,
            route_budget,
            &execution,
            &observations,
            &DeterministicFhtDkeBudgeter::new(0.10, 0.60, 128),
        );
        let runtime_planning = digest.planning_summary();
        let fht_dke_planning =
            clean_fht_dke_planning_readiness(route_budget, runtime_planning.fht_dke);
        let readiness = RuntimePlanningReadinessSummary::new(fht_dke_planning, runtime_planning);

        assert_eq!(digest.generation_budget.requested_context_tokens, 1156);
        assert_eq!(digest.generation_budget.planned_context_tokens, 1024);
        assert_eq!(digest.generation_budget.max_generated_tokens, 124);
        assert!(digest.context_limited());
        assert_eq!(
            runtime_planning.fht_dke.total_tokens,
            digest.generation_budget.planned_context_tokens
        );
        assert_eq!(runtime_planning.backend_max_tokens, 124);
        assert!(runtime_planning.context_soft_limited());
        assert!(!runtime_planning.context_exhausted());
        assert!(runtime_planning.is_request_ready());
        assert!(runtime_planning.can_commit_backend_request());
        assert!(readiness.fht_dke_planning_ready());
        assert!(readiness.runtime_pre_request_ready());
        assert!(readiness.fht_dke_runtime_boundary_ready());
        assert_eq!(
            readiness.fht_dke_runtime_boundary_drift_component_count(),
            0
        );
        assert_eq!(readiness.first_unready_stage(), None);
        assert_eq!(readiness.first_blocking_stage(), None);
        assert!(readiness.runtime_planning_readiness_accounting_is_consistent());
        assert!(readiness.can_commit_runtime_planning_readiness());
    }

    #[test]
    fn runtime_planning_readiness_blocks_stale_runtime_fht_dke_summary() {
        let route_budget = clean_route_budget();
        let fht_dke_budget = FhtDkeBudgetSummary {
            enabled: true,
            total_tokens: 2000,
            dense_tokens: 1000,
            routed_tokens: 1000,
            dense_fraction: 0.50,
            routed_fraction: 0.50,
            kv_import_blocks: 4,
            kv_export_blocks: 4,
            kv_exchange_blocks: 8,
            has_kv_exchange: true,
            token_split_is_valid: true,
            attention_threshold: route_budget.threshold,
            route_pressure: route_budget.attention_fraction,
        };
        let fht_dke_planning = clean_fht_dke_planning_readiness(route_budget, fht_dke_budget);
        let mut runtime_planning = clean_runtime_planning_summary(fht_dke_budget);
        runtime_planning.fht_dke.route_pressure = 0.25;
        runtime_planning.fht_dke.attention_threshold = 0.42;
        let readiness = RuntimePlanningReadinessSummary::new(fht_dke_planning, runtime_planning);

        assert!(readiness.fht_dke_planning_ready());
        assert!(readiness.runtime_pre_request_ready());
        assert!(!readiness.fht_dke_runtime_boundary_ready());
        assert!(!readiness.fht_dke_runtime_boundary_matches());
        assert_eq!(
            readiness.fht_dke_runtime_boundary_drift_component_count(),
            2
        );
        assert_eq!(
            readiness.first_unready_stage(),
            Some(RuntimePlanningReadinessStage::FhtDkeRuntimeBoundary)
        );
        assert_eq!(
            readiness.first_blocking_stage(),
            Some(RuntimePlanningReadinessStage::FhtDkeRuntimeBoundary)
        );
        assert_eq!(readiness.fht_dke_runtime_boundary_signal_component_count, 0);
        assert_eq!(
            readiness.fht_dke_runtime_boundary_blocker_component_count,
            2
        );
        assert_eq!(
            readiness.runtime_planning_readiness_blocker_component_count(),
            2
        );
        assert!(readiness.has_runtime_planning_readiness_blockers());
        assert!(readiness.runtime_planning_readiness_accounting_is_consistent());
        assert!(!readiness.runtime_planning_readiness_is_clean());
        assert!(!readiness.can_commit_runtime_planning_readiness());
    }

    #[test]
    fn runtime_planning_readiness_commits_threshold_attention_decision_boundary() {
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
            AttentionCandidate::new("fast", 0, 0.20, 0.10, RouteLayer::FastProjection),
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
        let attention_decision = ThresholdAttentionPolicy::new(0.50).select(
            &candidates,
            RoutingContext::default(),
            switches,
        );
        let attention_readiness = AttentionSelectionReadinessSummary::from_decision(
            AttentionCandidate::batch_summary(&candidates),
            &attention_decision,
        );
        let transformer_summary = TransformerPlanDigest::new(
            Some("runtime-planning-threshold-attention"),
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
            attention_decision.decision_summary(),
            transformer_summary,
        );
        let transformer_planning = TransformerPlanningReadinessSummary::new(
            route_readiness,
            attention_readiness,
            pressure,
        );
        let runtime = RuntimeMetadata::new("planning", "tok", 4096, 2048)
            .with_kv_exchange(true, true)
            .with_kv_limits(8, 4);
        let request = InferenceRequest::new("prompt", TaskProfile::General)
            .with_prompt_tokens(1800)
            .with_max_tokens(200)
            .with_runtime(runtime)
            .with_experiments(switches);
        let execution = AdapterExecutionContext::new([RuntimeAdapter::Cuda])
            .with_kv_prefetch_blocks(4)
            .with_parallel_chunks(2);
        let digest = RuntimePlanningDigest::from_request(
            &request,
            route_budget,
            &execution,
            &[],
            &DeterministicFhtDkeBudgeter::default(),
        );
        let runtime_planning = digest.planning_summary();
        let fht_dke_planning =
            FhtDkePlanningReadinessSummary::new(transformer_planning, runtime_planning.fht_dke);
        let readiness = RuntimePlanningReadinessSummary::new(fht_dke_planning, runtime_planning);

        assert_eq!(
            attention_decision.selected_tokens(),
            vec!["keep-local", "keep-global"]
        );
        assert!(attention_decision.hit_selection_cap());
        assert!(attention_readiness.candidate_batch_ready());
        assert!(attention_readiness.decision_ready());
        assert!(attention_readiness.selection_boundary_ready());
        assert!(attention_readiness.can_commit_attention_selection_readiness());
        assert!(transformer_planning.route_budget_ready());
        assert!(transformer_planning.attention_selection_ready());
        assert!(transformer_planning.planning_pressure_ready());
        assert!(transformer_planning.can_commit_transformer_planning_readiness());
        assert!(fht_dke_planning.transformer_planning_ready());
        assert!(fht_dke_planning.fht_dke_budget_ready());
        assert!(fht_dke_planning.pressure_budget_boundary_ready());
        assert!(fht_dke_planning.can_commit_fht_dke_planning_readiness());
        assert_eq!(
            runtime_planning.fht_dke.route_pressure,
            route_budget.attention_fraction
        );
        assert_eq!(
            runtime_planning.fht_dke.attention_threshold,
            route_budget.threshold
        );
        assert!(readiness.fht_dke_planning_ready());
        assert!(readiness.runtime_pre_request_ready());
        assert!(readiness.fht_dke_runtime_boundary_ready());
        assert_eq!(readiness.first_unready_stage(), None);
        assert_eq!(readiness.first_blocking_stage(), None);
        assert!(readiness.runtime_planning_readiness_accounting_is_consistent());
        assert!(readiness.runtime_planning_readiness_is_clean());
        assert!(readiness.can_commit_runtime_planning_readiness());
    }

    #[test]
    fn runtime_planning_readiness_blocks_stale_adaptive_attention_boundary() {
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
        let attention_decision = ThresholdAttentionPolicy::new(0.50).select(
            &candidates,
            RoutingContext::default(),
            switches,
        );
        let attention_readiness = AttentionSelectionReadinessSummary::from_decision(
            AttentionCandidate::batch_summary(&candidates[..4]),
            &attention_decision,
        );
        let transformer_summary = TransformerPlanDigest::new(
            Some("runtime-planning-stale-adaptive-attention"),
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
            attention_decision.decision_summary(),
            transformer_summary,
        );
        let transformer_planning = TransformerPlanningReadinessSummary::new(
            route_readiness,
            attention_readiness,
            pressure,
        );
        let fht_dke_budget = DeterministicFhtDkeBudgeter::default()
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
        let fht_dke_planning =
            FhtDkePlanningReadinessSummary::new(transformer_planning, fht_dke_budget);
        let runtime_planning = clean_runtime_planning_summary(fht_dke_budget);
        let readiness = RuntimePlanningReadinessSummary::new(fht_dke_planning, runtime_planning);

        assert_eq!(
            attention_decision.selected_tokens(),
            vec!["keep-local", "keep-global"]
        );
        assert!(attention_readiness.candidate_batch_ready());
        assert!(attention_readiness.decision_ready());
        assert!(!attention_readiness.selection_boundary_ready());
        assert!(transformer_planning.route_budget_ready());
        assert!(!transformer_planning.attention_selection_ready());
        assert!(transformer_planning.planning_pressure_ready());
        assert!(!fht_dke_planning.transformer_planning_ready());
        assert!(fht_dke_planning.fht_dke_budget_ready());
        assert!(fht_dke_planning.pressure_budget_boundary_ready());
        assert!(!readiness.fht_dke_planning_ready());
        assert!(readiness.runtime_pre_request_ready());
        assert!(readiness.fht_dke_runtime_boundary_ready());
        assert!(readiness.fht_dke_runtime_boundary_matches());
        assert_eq!(
            readiness.fht_dke_runtime_boundary_drift_component_count(),
            0
        );
        assert_eq!(
            readiness.first_unready_stage(),
            Some(RuntimePlanningReadinessStage::FhtDkePlanning)
        );
        assert_eq!(
            readiness.first_blocking_stage(),
            Some(RuntimePlanningReadinessStage::FhtDkePlanning)
        );
        assert_eq!(readiness.runtime_pre_request_blocker_component_count, 0);
        assert_eq!(
            readiness.fht_dke_runtime_boundary_blocker_component_count,
            0
        );
        assert!(readiness.has_runtime_planning_readiness_blockers());
        assert!(readiness.runtime_planning_readiness_accounting_is_consistent());
        assert!(!readiness.runtime_planning_readiness_is_clean());
        assert!(!readiness.can_commit_runtime_planning_readiness());
    }

    #[test]
    fn runtime_planning_readiness_exposes_committed_fht_dke_parts_boundary() {
        let route_budget = clean_route_budget();
        let fht_dke_budget = FhtDkeBudgetSummary {
            enabled: true,
            total_tokens: 2000,
            dense_tokens: 1000,
            routed_tokens: 1000,
            dense_fraction: 0.50,
            routed_fraction: 0.50,
            kv_import_blocks: 4,
            kv_export_blocks: 4,
            kv_exchange_blocks: 8,
            has_kv_exchange: true,
            token_split_is_valid: true,
            attention_threshold: route_budget.threshold,
            route_pressure: route_budget.attention_fraction,
        };
        let fht_dke_planning = clean_fht_dke_planning_readiness(route_budget, fht_dke_budget);
        let runtime_planning = clean_runtime_planning_summary(fht_dke_budget);
        let clean = RuntimePlanningReadinessSummary::new(fht_dke_planning, runtime_planning);
        let clean_commit = clean.fht_dke_planning_commit_summary();

        assert!(clean.can_use_committed_fht_dke_runtime_planning_parts());
        assert!(clean.can_commit_runtime_planning_with_committed_parts());
        assert!(clean_commit.can_use_committed_runtime_planning_parts());
        assert_eq!(
            clean_commit.committed_transformer_planning,
            Some(fht_dke_planning.transformer_planning)
        );
        assert_eq!(clean_commit.committed_fht_dke_budget, Some(fht_dke_budget));
        assert!(clean_commit.commit_decision_accounting_is_consistent());

        let mut stale_runtime_planning = runtime_planning;
        stale_runtime_planning.fht_dke.route_pressure = 0.25;
        stale_runtime_planning.fht_dke.attention_threshold = 0.42;
        let boundary_drift =
            RuntimePlanningReadinessSummary::new(fht_dke_planning, stale_runtime_planning);
        let boundary_commit = boundary_drift.fht_dke_planning_commit_summary();

        assert!(boundary_drift.can_use_committed_fht_dke_runtime_planning_parts());
        assert!(!boundary_drift.can_commit_runtime_planning_with_committed_parts());
        assert!(boundary_commit.can_use_committed_runtime_planning_parts());
        assert!(!boundary_drift.fht_dke_runtime_boundary_ready());
        assert!(!boundary_drift.can_commit_runtime_planning_readiness());

        let mut stale_fht_dke_budget = fht_dke_budget;
        stale_fht_dke_budget.route_pressure = 0.25;
        stale_fht_dke_budget.attention_threshold = 0.42;
        let repair_fht_dke_planning =
            clean_fht_dke_planning_readiness(route_budget, stale_fht_dke_budget);
        let repair =
            RuntimePlanningReadinessSummary::new(repair_fht_dke_planning, runtime_planning);
        let repair_commit = repair.fht_dke_planning_commit_summary();

        assert!(!repair.can_use_committed_fht_dke_runtime_planning_parts());
        assert!(!repair.can_commit_runtime_planning_with_committed_parts());
        assert!(!repair_commit.can_use_committed_runtime_planning_parts());
        assert!(repair_commit.should_repair_fht_dke_planning());
        assert!(!repair.fht_dke_planning_ready());
        assert!(!repair.can_commit_runtime_planning_readiness());
    }

    fn clean_route_budget() -> RouteBudget {
        RouteBudget {
            threshold: 0.50,
            attention_tokens: 3,
            fast_tokens: 1,
            attention_fraction: 0.75,
        }
    }

    fn clean_fht_dke_planning_readiness(
        route_budget: RouteBudget,
        fht_dke_budget: FhtDkeBudgetSummary,
    ) -> FhtDkePlanningReadinessSummary {
        let layer_counts = RouteLayerCounts {
            fast_projection: 1,
            local_window: 1,
            global: 1,
            fusion: 1,
        };
        let decision_summary = RoutingDecisionSummary {
            threshold: route_budget.threshold,
            token_count: 4,
            layer_counts,
            attention_fraction: route_budget.attention_fraction,
            average_score: 0.65,
            min_score: 0.20,
            max_score: 0.90,
            above_threshold_tokens: route_budget.attention_tokens,
            below_threshold_tokens: route_budget.fast_tokens,
        };
        let route_readiness = RouteBudgetReadinessSummary::new(decision_summary, route_budget);
        let attention_decision = AttentionDecisionSummary {
            threshold: route_budget.threshold,
            max_selected: 4,
            candidate_count: 4,
            selected_count: 2,
            rejected_count: 2,
            selection_fraction: 0.50,
            hit_selection_cap: false,
            selected_layer_counts: RouteLayerCounts {
                fast_projection: 0,
                local_window: 1,
                global: 1,
                fusion: 0,
            },
            rejected_layer_counts: RouteLayerCounts {
                fast_projection: 1,
                local_window: 0,
                global: 0,
                fusion: 1,
            },
        };
        let attention_readiness = AttentionSelectionReadinessSummary::new(
            AttentionCandidateBatchSummary {
                candidate_count: 4,
                attention_candidate_count: route_budget.attention_tokens,
                fast_candidate_count: route_budget.fast_tokens,
                layer_counts,
                average_score: 0.65,
                average_entropy: 0.50,
                max_score: 0.90,
                max_entropy: 0.80,
            },
            attention_decision,
        );
        let transformer_summary = TransformerPlanSummary {
            layer_count: 4,
            counts: TransformerPlanCounts {
                global: 1,
                local: 2,
                fusion: 1,
            },
            average_compute_fraction: 0.60,
            min_window_size: 256,
            max_window_size: 4096,
        };
        let pressure = TransformerPlanningPressureSummary::from_parts(
            route_budget,
            attention_decision,
            transformer_summary,
        );
        let transformer_readiness = TransformerPlanningReadinessSummary::new(
            route_readiness,
            attention_readiness,
            pressure,
        );

        FhtDkePlanningReadinessSummary::new(transformer_readiness, fht_dke_budget)
    }

    fn clean_runtime_planning_summary(fht_dke: FhtDkeBudgetSummary) -> RuntimePlanningSummary {
        RuntimePlanningSummary {
            generation_budget: RuntimeGenerationBudget::new(1800, 200, 4096),
            context_limited: false,
            backend_max_tokens: 200,
            adapter_selection: AdapterSelection {
                adapter: RuntimeAdapter::Cuda,
                score: 0.95,
                experience_id: Some(16),
                used_fallback: false,
            },
            adapter_fallback_reason: AdapterFallbackReason::NoFallback,
            allowed_adapter_count: 1,
            observation_count: 1,
            matching_observation_count: 1,
            matched_observation_fraction: 1.0,
            fht_dke,
            kv_exchange: RuntimePlanningKvExchange {
                import_blocks: fht_dke.kv_import_blocks,
                export_blocks: fht_dke.kv_export_blocks,
                prefetch_was_clamped: false,
                clamp_reason: RuntimePlanningKvClampReason::NotClamped,
            },
            kv_clamp: RuntimePlanningKvClampSummary {
                requested_kv_prefetch_blocks: fht_dke.kv_import_blocks,
                runtime_kv_prefetch_blocks: fht_dke.kv_import_blocks,
                planned_kv_import_blocks: fht_dke.kv_import_blocks,
                runtime_metadata_reduction: 0,
                fht_dke_reduction: 0,
                total_reduction: 0,
                prefetch_was_clamped: false,
                clamp_reason: RuntimePlanningKvClampReason::NotClamped,
            },
            requested_kv_prefetch_blocks: fht_dke.kv_import_blocks,
            runtime_kv_prefetch_blocks: fht_dke.kv_import_blocks,
            hardware_pressure: 0.25,
            compute_headroom: 0.75,
            max_parallel_chunks: 2,
            latency_budget_ms: Some(120),
        }
    }

    #[test]
    fn planning_summary_blocks_backend_request_on_public_shape_drift() {
        let summary = RuntimePlanningSummary {
            generation_budget: RuntimeGenerationBudget::new(1024, 128, 2048),
            context_limited: false,
            backend_max_tokens: 128,
            adapter_selection: AdapterSelection {
                adapter: RuntimeAdapter::Cuda,
                score: 0.9,
                experience_id: Some(7),
                used_fallback: false,
            },
            adapter_fallback_reason: AdapterFallbackReason::NoFallback,
            allowed_adapter_count: 1,
            observation_count: 1,
            matching_observation_count: 1,
            matched_observation_fraction: 1.0,
            fht_dke: FhtDkeBudgetSummary {
                enabled: true,
                total_tokens: 12,
                dense_tokens: 4,
                routed_tokens: 8,
                dense_fraction: 4.0 / 12.0,
                routed_fraction: 8.0 / 12.0,
                kv_import_blocks: 2,
                kv_export_blocks: 1,
                kv_exchange_blocks: 3,
                has_kv_exchange: true,
                token_split_is_valid: true,
                attention_threshold: 0.55,
                route_pressure: 0.50,
            },
            kv_exchange: RuntimePlanningKvExchange {
                import_blocks: 4,
                export_blocks: 1,
                prefetch_was_clamped: false,
                clamp_reason: RuntimePlanningKvClampReason::NotClamped,
            },
            kv_clamp: RuntimePlanningKvClampSummary {
                requested_kv_prefetch_blocks: 4,
                runtime_kv_prefetch_blocks: 4,
                planned_kv_import_blocks: 3,
                runtime_metadata_reduction: 0,
                fht_dke_reduction: 0,
                total_reduction: 0,
                prefetch_was_clamped: false,
                clamp_reason: RuntimePlanningKvClampReason::NotClamped,
            },
            requested_kv_prefetch_blocks: 4,
            runtime_kv_prefetch_blocks: 4,
            hardware_pressure: 0.35,
            compute_headroom: 0.65,
            max_parallel_chunks: 2,
            latency_budget_ms: Some(120),
        };

        assert!(summary.is_request_ready());
        assert!(!summary.kv_clamp_is_consistent());
        assert_eq!(summary.kv_clamp_consistency_problem_component_count(), 1);
        assert_eq!(summary.request_readiness_blocker_component_count(), 0);
        assert!(summary.request_readiness_accounting_is_consistent());
        assert_eq!(summary.pre_request_gate_problem_component_count(), 1);
        assert!(summary.has_pre_request_gate_problem_components());
        assert!(summary.pre_request_gate_accounting_is_consistent());
        assert!(!summary.pre_request_gate_shape_is_clean());
        assert_eq!(
            summary.backend_request_commit_signal_component_count(),
            summary.pre_request_gate_signal_component_count()
        );
        assert_eq!(summary.backend_request_commit_blocker_component_count(), 1);
        assert!(summary.has_backend_request_commit_blockers());
        assert!(summary.backend_request_commit_accounting_is_consistent());
        assert!(!summary.backend_request_commit_is_clean());
        assert!(!summary.can_commit_backend_request());
        assert!(!summary.can_send_backend_request());
    }

    #[test]
    fn planning_digest_reports_fht_dke_kv_prefetch_clamp_reason() {
        let runtime = RuntimeMetadata::new("planning", "tok", 8192, 128)
            .with_kv_exchange(true, true)
            .with_kv_limits(8, 8);
        let request = InferenceRequest::new("prompt", TaskProfile::LongDocument)
            .with_prompt_tokens(1024)
            .with_max_tokens(512)
            .with_runtime(runtime)
            .with_experiments(ExperimentSwitches::default().with_fht_dke(true));
        let execution =
            AdapterExecutionContext::new([RuntimeAdapter::Cuda]).with_kv_prefetch_blocks(8);

        let digest = RuntimePlanningDigest::from_request(
            &request,
            RouteBudget {
                threshold: 0.45,
                attention_tokens: 1,
                fast_tokens: 9,
                attention_fraction: 0.10,
            },
            &execution,
            &[],
            &DeterministicFhtDkeBudgeter::new(0.10, 0.60, 4096),
        );

        assert!(digest.is_valid());
        assert_eq!(digest.requested_kv_prefetch_blocks, 8);
        assert_eq!(digest.runtime_kv_prefetch_blocks, 8);
        assert!(digest.planned_kv_import_blocks < digest.runtime_kv_prefetch_blocks);
        assert_eq!(
            digest.kv_prefetch_clamp_reason(),
            RuntimePlanningKvClampReason::FhtDkeBudgetLimit
        );
        assert_eq!(
            digest.planned_kv_exchange().clamp_reason.as_str(),
            "fht-dke-budget-limit"
        );
        let clamp = digest.kv_prefetch_clamp_summary();

        assert!(clamp.is_consistent());
        assert!(clamp.clamp_counts_are_bounded());
        assert_eq!(clamp.clamp_shape_problem_component_count(), 0);
        assert!(!clamp.has_clamp_shape_problem_components());
        assert!(clamp.clamp_shape_accounting_is_consistent());
        assert!(clamp.clamp_shape_is_clean());
        assert!(clamp.can_use_runtime_planning_kv_clamp());
        assert!(!clamp.has_runtime_metadata_clamp());
        assert!(clamp.has_fht_dke_clamp());
        assert!(clamp.clamped_by_fht_dke_only());
        assert!(!clamp.clamped_by_runtime_only());
        assert!(!clamp.clamped_by_runtime_and_fht_dke());
        assert!(digest.planning_summary().kv_clamp_is_consistent());
    }

    #[test]
    fn planning_digest_summarizes_runtime_and_fht_dke_kv_prefetch_reductions() {
        let runtime = RuntimeMetadata::new("planning", "tok", 8192, 128)
            .with_kv_exchange(true, true)
            .with_kv_limits(4, 8);
        let request = InferenceRequest::new("prompt", TaskProfile::LongDocument)
            .with_prompt_tokens(1024)
            .with_max_tokens(512)
            .with_runtime(runtime)
            .with_experiments(ExperimentSwitches::default().with_fht_dke(true));
        let execution =
            AdapterExecutionContext::new([RuntimeAdapter::Cuda]).with_kv_prefetch_blocks(8);

        let digest = RuntimePlanningDigest::from_request(
            &request,
            RouteBudget {
                threshold: 0.45,
                attention_tokens: 1,
                fast_tokens: 9,
                attention_fraction: 0.10,
            },
            &execution,
            &[],
            &DeterministicFhtDkeBudgeter::new(0.10, 0.60, 4096),
        );
        let clamp = digest.kv_prefetch_clamp_summary();

        assert!(digest.is_valid());
        assert_eq!(
            digest.kv_prefetch_clamp_reason(),
            RuntimePlanningKvClampReason::RuntimeAndFhtDkeLimits
        );
        assert_eq!(clamp.requested_kv_prefetch_blocks, 8);
        assert_eq!(clamp.runtime_kv_prefetch_blocks, 4);
        assert_eq!(clamp.planned_kv_import_blocks, 1);
        assert_eq!(clamp.runtime_metadata_reduction, 4);
        assert_eq!(clamp.fht_dke_reduction, 3);
        assert_eq!(clamp.total_reduction, 7);
        assert!(clamp.prefetch_was_clamped);
        assert_eq!(
            clamp.clamp_reason,
            RuntimePlanningKvClampReason::RuntimeAndFhtDkeLimits
        );
        assert!(clamp.has_runtime_metadata_clamp());
        assert!(clamp.has_fht_dke_clamp());
        assert!(!clamp.import_matches_runtime_prefetch());
        assert!(clamp.reductions_match_total());
        assert!(clamp.block_counts_match_reductions());
        assert!(clamp.clamp_reason_matches_reductions());
        assert!(clamp.prefetch_clamp_flag_matches_counts());
        assert!(clamp.is_consistent());
        assert!(clamp.clamp_counts_are_bounded());
        assert_eq!(clamp.clamp_bound_problem_component_count(), 0);
        assert_eq!(clamp.clamp_reduction_problem_component_count(), 0);
        assert_eq!(clamp.clamp_reason_problem_component_count(), 0);
        assert_eq!(clamp.clamp_flag_problem_component_count(), 0);
        assert_eq!(clamp.clamp_shape_problem_component_count(), 0);
        assert!(clamp.clamp_shape_accounting_is_consistent());
        assert!(clamp.clamp_shape_is_clean());
        assert!(clamp.can_use_runtime_planning_kv_clamp());
        assert!(clamp.clamped_by_runtime_and_fht_dke());
        assert!(!clamp.clamped_by_runtime_only());
        assert!(!clamp.clamped_by_fht_dke_only());
        let summary = digest.planning_summary();

        assert_eq!(summary.kv_clamp, clamp);
        assert!(summary.kv_clamp_is_consistent());
        assert!(summary.kv_prefetch_was_clamped());
        assert!(summary.fht_dke_limited_kv_prefetch());
        assert!(summary.route_pressure_is_active());
        assert!(!summary.route_pressure_is_high());
        assert!(summary.has_routed_kv_exchange());
        assert_eq!(summary.route_pressure_signal_component_count(), 1);
        assert_eq!(summary.high_route_pressure_signal_component_count(), 0);
        assert_eq!(summary.routed_kv_exchange_signal_component_count(), 1);
        assert_eq!(summary.fht_dke_budget_shape_problem_component_count(), 0);
        assert_eq!(summary.fht_dke_budget_commit_signal_component_count(), 3);
        assert!(summary.has_fht_dke_budget_commit_signals());
        assert_eq!(summary.fht_dke_budget_commit_blocker_component_count(), 0);
        assert!(!summary.has_fht_dke_budget_commit_blockers());
        assert!(summary.fht_dke_budget_commit_accounting_is_consistent());
        assert_eq!(summary.kv_clamp_consistency_problem_component_count(), 0);
        assert_eq!(summary.request_readiness_blocker_component_count(), 0);
        assert!(summary.request_readiness_accounting_is_consistent());
        assert_eq!(summary.pre_request_gate_problem_component_count(), 0);
        assert!(summary.pre_request_gate_accounting_is_consistent());
        assert_eq!(summary.planning_pressure_signal_component_count(), 5);
    }

    #[test]
    fn planning_digest_keeps_hardware_pressure_runtime_kv_clamp_as_signal() {
        let runtime = RuntimeMetadata::new("planning", "tok", 4096, 128)
            .with_kv_exchange(false, true)
            .with_kv_limits(0, 2);
        let request = InferenceRequest::new("prompt", TaskProfile::LongDocument)
            .with_prompt_tokens(1024)
            .with_max_tokens(256)
            .with_runtime(runtime)
            .with_experiments(ExperimentSwitches::default().with_fht_dke(true));
        let hardware = HardwareAllocator::new().plan(
            HardwareLoadSnapshot::new(DeviceClass::DiscreteGpu, 0.95, 0.95, 0.95, 0.95),
            TaskProfile::LongDocument,
            40_000,
            HierarchyWeights::for_profile(TaskProfile::LongDocument),
        );
        let execution = hardware.adapter_execution_context();
        let clamp = execution.runtime_clamp_summary(&request.runtime);

        assert_eq!(execution.kv_prefetch_blocks, 1);
        assert!(execution.hardware_pressure >= 0.72);
        assert!(clamp.kv_prefetch_was_clamped());
        assert_eq!(clamp.before.kv_prefetch_blocks, 1);
        assert_eq!(clamp.after.kv_prefetch_blocks, 0);
        assert_eq!(clamp.kv_prefetch_reduction, 1);
        assert_eq!(clamp.kv_prefetch_clamp_signal_component_count(), 1);
        assert_eq!(clamp.runtime_clamp_problem_component_count(), 0);
        assert!(clamp.runtime_clamp_shape_is_clean());
        assert!(clamp.can_commit_runtime_clamp());

        let digest = RuntimePlanningDigest::from_request(
            &request,
            RouteBudget {
                threshold: 0.50,
                attention_tokens: 4,
                fast_tokens: 4,
                attention_fraction: 0.50,
            },
            &execution,
            &[],
            &DeterministicFhtDkeBudgeter::new(0.10, 0.60, 4096),
        );
        let kv_clamp = digest.kv_prefetch_clamp_summary();
        let summary = digest.planning_summary();

        assert!(digest.is_valid());
        assert_eq!(digest.requested_kv_prefetch_blocks, 1);
        assert_eq!(digest.runtime_kv_prefetch_blocks, 0);
        assert_eq!(digest.planned_kv_import_blocks, 0);
        assert_eq!(digest.hardware_pressure, execution.hardware_pressure);
        assert_eq!(digest.compute_headroom, execution.compute_headroom);
        assert_eq!(digest.max_parallel_chunks, execution.max_parallel_chunks);
        assert_eq!(digest.latency_budget_ms, execution.latency_budget_ms);
        assert_eq!(
            digest.kv_prefetch_clamp_reason(),
            RuntimePlanningKvClampReason::RuntimeMetadataLimit
        );
        assert_eq!(kv_clamp.requested_kv_prefetch_blocks, 1);
        assert_eq!(kv_clamp.runtime_kv_prefetch_blocks, 0);
        assert_eq!(kv_clamp.planned_kv_import_blocks, 0);
        assert_eq!(kv_clamp.runtime_metadata_reduction, 1);
        assert_eq!(kv_clamp.fht_dke_reduction, 0);
        assert_eq!(kv_clamp.total_reduction, 1);
        assert!(kv_clamp.has_runtime_metadata_clamp());
        assert!(!kv_clamp.has_fht_dke_clamp());
        assert!(kv_clamp.clamped_by_runtime_only());
        assert!(kv_clamp.clamp_shape_is_clean());
        assert!(kv_clamp.can_use_runtime_planning_kv_clamp());
        assert_eq!(summary.kv_clamp, kv_clamp);
        assert_eq!(summary.hardware_pressure, execution.hardware_pressure);
        assert_eq!(summary.max_parallel_chunks, 1);
        assert!(summary.kv_prefetch_was_clamped());
        assert!(!summary.fht_dke_limited_kv_prefetch());
        assert_eq!(summary.kv_clamp_consistency_problem_component_count(), 0);
        assert_eq!(summary.request_readiness_blocker_component_count(), 0);
        assert_eq!(summary.pre_request_gate_problem_component_count(), 0);
        assert!(summary.pre_request_gate_shape_is_clean());
        assert!(summary.can_send_backend_request());
        assert!(summary.can_commit_backend_request());
        assert!(summary.planning_pressure_signal_component_count() > 0);
        assert!(summary.has_pre_request_gate_signals());
    }

    #[test]
    fn planning_summary_blocks_backend_request_on_empty_fht_dke_budget() {
        let summary = RuntimePlanningSummary {
            generation_budget: RuntimeGenerationBudget::new(1024, 128, 2048),
            context_limited: false,
            backend_max_tokens: 128,
            adapter_selection: AdapterSelection {
                adapter: RuntimeAdapter::Cuda,
                score: 0.9,
                experience_id: Some(7),
                used_fallback: false,
            },
            adapter_fallback_reason: AdapterFallbackReason::NoFallback,
            allowed_adapter_count: 1,
            observation_count: 1,
            matching_observation_count: 1,
            matched_observation_fraction: 1.0,
            fht_dke: FhtDkeBudget::disabled(0, RouteBudget::default().threshold).budget_summary(),
            kv_exchange: RuntimePlanningKvExchange {
                import_blocks: 0,
                export_blocks: 0,
                prefetch_was_clamped: false,
                clamp_reason: RuntimePlanningKvClampReason::NotClamped,
            },
            kv_clamp: RuntimePlanningKvClampSummary {
                requested_kv_prefetch_blocks: 0,
                runtime_kv_prefetch_blocks: 0,
                planned_kv_import_blocks: 0,
                runtime_metadata_reduction: 0,
                fht_dke_reduction: 0,
                total_reduction: 0,
                prefetch_was_clamped: false,
                clamp_reason: RuntimePlanningKvClampReason::NotClamped,
            },
            requested_kv_prefetch_blocks: 0,
            runtime_kv_prefetch_blocks: 0,
            hardware_pressure: 0.35,
            compute_headroom: 0.65,
            max_parallel_chunks: 1,
            latency_budget_ms: Some(120),
        };

        assert!(summary.is_request_ready());
        assert_eq!(summary.fht_dke_budget_shape_problem_component_count(), 0);
        assert_eq!(summary.fht_dke_budget_commit_signal_component_count(), 0);
        assert!(!summary.has_fht_dke_budget_commit_signals());
        assert_eq!(summary.fht_dke_budget_commit_blocker_component_count(), 1);
        assert!(summary.has_fht_dke_budget_commit_blockers());
        assert!(summary.fht_dke_budget_commit_accounting_is_consistent());
        assert_eq!(summary.request_readiness_blocker_component_count(), 0);
        assert!(summary.request_readiness_accounting_is_consistent());
        assert_eq!(summary.pre_request_gate_problem_component_count(), 1);
        assert!(summary.has_pre_request_gate_problem_components());
        assert!(summary.pre_request_gate_accounting_is_consistent());
        assert!(!summary.pre_request_gate_shape_is_clean());
        assert_eq!(summary.backend_request_commit_blocker_component_count(), 1);
        assert!(summary.has_backend_request_commit_blockers());
        assert!(summary.backend_request_commit_accounting_is_consistent());
        assert!(!summary.backend_request_commit_is_clean());
        assert!(!summary.can_send_backend_request());
        assert!(!summary.can_commit_backend_request());
    }

    #[test]
    fn planning_kv_clamp_summary_marks_unclamped_prefetch_as_consistent() {
        let runtime = RuntimeMetadata::new("planning", "tok", 8192, 128)
            .with_kv_exchange(true, true)
            .with_kv_limits(8, 8);
        let request = InferenceRequest::new("prompt", TaskProfile::General)
            .with_prompt_tokens(64)
            .with_max_tokens(64)
            .with_runtime(runtime);
        let execution =
            AdapterExecutionContext::new([RuntimeAdapter::CpuSimd]).with_kv_prefetch_blocks(2);

        let digest = RuntimePlanningDigest::from_request(
            &request,
            RouteBudget::default(),
            &execution,
            &[],
            &DeterministicFhtDkeBudgeter::default(),
        );
        let clamp = digest.kv_prefetch_clamp_summary();
        let summary = digest.planning_summary();

        assert_eq!(
            digest.kv_prefetch_clamp_reason(),
            RuntimePlanningKvClampReason::NotClamped
        );
        assert!(!digest.kv_prefetch_was_clamped());
        assert_eq!(clamp.requested_kv_prefetch_blocks, 2);
        assert_eq!(clamp.runtime_kv_prefetch_blocks, 2);
        assert_eq!(clamp.planned_kv_import_blocks, 2);
        assert_eq!(clamp.total_reduction, 0);
        assert!(clamp.import_matches_runtime_prefetch());
        assert!(!clamp.has_runtime_metadata_clamp());
        assert!(!clamp.has_fht_dke_clamp());
        assert!(clamp.is_unclamped());
        assert!(clamp.is_consistent());
        assert!(clamp.clamp_counts_are_bounded());
        assert_eq!(clamp.clamp_shape_problem_component_count(), 0);
        assert!(!clamp.has_clamp_shape_problem_components());
        assert!(clamp.clamp_shape_accounting_is_consistent());
        assert!(clamp.clamp_shape_is_clean());
        assert!(clamp.can_use_runtime_planning_kv_clamp());
        assert!(!clamp.clamped_by_runtime_only());
        assert!(!clamp.clamped_by_fht_dke_only());
        assert!(!clamp.clamped_by_runtime_and_fht_dke());
        assert!(summary.kv_clamp_is_consistent());
        assert!(!summary.kv_prefetch_was_clamped());
        assert!(summary.adapter_observations_missing());
        assert!(summary.adapter_observation_gap());
        assert!(summary.adapter_fallback_due_to_no_matching_observation());
        assert!(!summary.adapter_selection_blocked());
        assert_eq!(summary.adapter_selection_blocker_component_count(), 0);
        assert_eq!(summary.adapter_observation_signal_component_count(), 2);
        assert_eq!(summary.adapter_planning_signal_component_count(), 2);
        assert!(summary.has_adapter_planning_signals());
    }

    #[test]
    fn planning_kv_clamp_summary_counts_public_bound_drift() {
        let clamp = RuntimePlanningKvClampSummary {
            requested_kv_prefetch_blocks: 4,
            runtime_kv_prefetch_blocks: 6,
            planned_kv_import_blocks: 7,
            runtime_metadata_reduction: 0,
            fht_dke_reduction: 0,
            total_reduction: 0,
            prefetch_was_clamped: false,
            clamp_reason: RuntimePlanningKvClampReason::NotClamped,
        };

        assert!(!clamp.runtime_prefetch_not_above_requested());
        assert!(!clamp.planned_import_not_above_runtime_prefetch());
        assert!(!clamp.planned_import_not_above_requested());
        assert!(!clamp.clamp_counts_are_bounded());
        assert!(clamp.reductions_match_total());
        assert!(clamp.block_counts_match_reductions());
        assert!(clamp.clamp_reason_matches_reductions());
        assert!(clamp.prefetch_clamp_flag_matches_counts());
        assert_eq!(clamp.clamp_bound_problem_component_count(), 3);
        assert_eq!(clamp.clamp_reduction_problem_component_count(), 0);
        assert_eq!(clamp.clamp_reason_problem_component_count(), 0);
        assert_eq!(clamp.clamp_flag_problem_component_count(), 0);
        assert_eq!(clamp.clamp_shape_problem_component_count(), 3);
        assert!(clamp.has_clamp_shape_problem_components());
        assert!(!clamp.is_consistent());
        assert!(clamp.clamp_shape_accounting_is_consistent());
        assert!(!clamp.clamp_shape_is_clean());
        assert!(!clamp.can_use_runtime_planning_kv_clamp());
    }
}
