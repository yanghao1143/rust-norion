use crate::adapter::RuntimeAdapter;
use crate::engine::{
    GeneratedTokenMetrics, InferenceError, InferenceOutcome, RuntimeFailureBatchSummary,
    RuntimeFailureReport, RuntimeFailureSummary,
};
use crate::hardware::HardwarePlan;
use crate::kv::{RuntimeKvBlockContract, RuntimeKvValidationReport};
use crate::manifest::TransformerRuntimeArchitecture;
use crate::planning::RuntimePlanningManifestKvBridgeSummary;
use crate::request::RuntimeRequestEnvelope;
use crate::runtime::RuntimeMetadata;

pub const RUNTIME_RESPONSE_SCHEMA: &str = "rust-norion-runtime-response-v1";

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeResponseEnvelope {
    pub schema: &'static str,
    pub answer_chars: usize,
    pub token_metrics: GeneratedTokenMetrics,
    pub imported_kv_blocks: usize,
    pub exported_kv_blocks: usize,
    pub diagnostics_imported_kv_blocks: usize,
    pub diagnostics_exported_kv_blocks: usize,
    pub diagnostics_weak_runtime_kv_imports_skipped: usize,
    pub has_runtime_execution_signal: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeResponseEnvelopeSummary {
    pub schema: &'static str,
    pub answer_chars: usize,
    pub token_count: usize,
    pub entropy_count: usize,
    pub logprob_count: usize,
    pub has_uncertainty_signal: bool,
    pub token_uncertainty_coverage_signal_count: usize,
    pub token_uncertainty_metric_problem_count: usize,
    pub token_uncertainty_accounting_consistent: bool,
    pub imported_kv_blocks: usize,
    pub exported_kv_blocks: usize,
    pub diagnostics_imported_kv_blocks: usize,
    pub diagnostics_exported_kv_blocks: usize,
    pub diagnostics_weak_runtime_kv_imports_skipped: usize,
    pub has_runtime_execution_signal: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeResponseRequestParitySummary {
    pub token_count: usize,
    pub request_max_tokens: usize,
    pub planned_backend_max_tokens: Option<usize>,
    pub token_count_within_request: bool,
    pub token_count_within_planning: Option<bool>,
    pub imported_kv_blocks: usize,
    pub request_imported_kv_blocks: usize,
    pub planned_imported_kv_blocks: Option<usize>,
    pub imported_kv_matches_request: bool,
    pub imported_kv_within_planning: Option<bool>,
    pub exported_kv_blocks: usize,
    pub runtime_export_enabled: bool,
    pub runtime_max_export_blocks: usize,
    pub planned_exported_kv_blocks: Option<usize>,
    pub exported_kv_within_runtime: bool,
    pub exported_kv_within_planning: Option<bool>,
    pub request_selected_adapter: Option<RuntimeAdapter>,
    pub runtime_selected_adapter: Option<RuntimeAdapter>,
    pub runtime_adapter_reported: bool,
    pub selected_adapter_matches_request: bool,
    pub generation_budget_reported: bool,
    pub generation_budget_matches_request: bool,
    pub route_budget_matches_request: bool,
    pub hardware_pressure_matches_request: bool,
    pub compute_headroom_matches_planning: Option<bool>,
    pub latency_budget_matches_planning: Option<bool>,
    pub planning_pre_request_problem_count: usize,
    pub planning_pressure_signal_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeResponsePlannedKvSummary {
    pub has_planning_digest: bool,
    pub imported_kv_blocks: usize,
    pub exported_kv_blocks: usize,
    pub planned_imported_kv_blocks: Option<usize>,
    pub planned_exported_kv_blocks: Option<usize>,
    pub imported_kv_within_planning: Option<bool>,
    pub exported_kv_within_planning: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeResponseManifestKvSummary {
    pub manifest_kv_bridge: RuntimePlanningManifestKvBridgeSummary,
    pub response_planned_kv: RuntimeResponsePlannedKvSummary,
    pub response_imported_kv_within_manifest_plan: bool,
    pub response_exported_kv_within_manifest_plan: bool,
    pub manifest_kv_bridge_signal_component_count: usize,
    pub response_planned_kv_signal_component_count: usize,
    pub manifest_kv_bridge_blocker_component_count: usize,
    pub response_planned_kv_blocker_component_count: usize,
    pub response_manifest_kv_blocker_component_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeResponseGateSummary {
    pub response_accepted: bool,
    pub envelope_consistent: bool,
    pub request_parity_consistent: bool,
    pub exported_kv_accepted: bool,
    pub accepted_exported_kv_blocks: usize,
    pub response_wire_problem_count: usize,
    pub planning_pre_request_problem_count: usize,
    pub planning_pressure_signal_count: usize,
    pub response_violation_count: usize,
    pub request_violation_count: usize,
    pub exported_kv_violation_count: usize,
    pub failure_report_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeResponseReadinessStage {
    ResponseEnvelope,
    ResponseRequestParity,
    ResponseGate,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeResponseReadinessSummary {
    pub response_envelope: RuntimeResponseEnvelopeSummary,
    pub response_request: RuntimeResponseRequestParitySummary,
    pub response_gate: RuntimeResponseGateSummary,
    pub response_envelope_signal_component_count: usize,
    pub response_request_signal_component_count: usize,
    pub response_gate_signal_component_count: usize,
    pub response_envelope_blocker_component_count: usize,
    pub response_request_blocker_component_count: usize,
    pub response_gate_blocker_component_count: usize,
}

impl RuntimeResponseReadinessSummary {
    pub fn new(
        response_envelope: RuntimeResponseEnvelopeSummary,
        response_request: RuntimeResponseRequestParitySummary,
        response_gate: RuntimeResponseGateSummary,
    ) -> Self {
        Self {
            response_envelope,
            response_request,
            response_gate,
            response_envelope_signal_component_count: response_envelope
                .runtime_response_envelope_commit_signal_component_count(),
            response_request_signal_component_count: response_request
                .planning_pressure_signal_component_count(),
            response_gate_signal_component_count: response_gate
                .runtime_response_commit_signal_component_count(),
            response_envelope_blocker_component_count: response_envelope
                .runtime_response_envelope_commit_blocker_component_count(),
            response_request_blocker_component_count: response_request
                .response_wire_problem_component_count(),
            response_gate_blocker_component_count: response_gate
                .runtime_response_commit_blocker_component_count(),
        }
    }

    pub fn stage_order() -> [RuntimeResponseReadinessStage; 3] {
        [
            RuntimeResponseReadinessStage::ResponseEnvelope,
            RuntimeResponseReadinessStage::ResponseRequestParity,
            RuntimeResponseReadinessStage::ResponseGate,
        ]
    }

    pub fn response_envelope_ready(self) -> bool {
        self.response_envelope
            .can_commit_runtime_response_envelope()
    }

    pub fn response_request_ready(self) -> bool {
        self.response_request.can_use_response_wire()
    }

    pub fn response_gate_ready(self) -> bool {
        self.response_gate.can_commit_runtime_response()
    }

    pub fn stage_ready(self, stage: RuntimeResponseReadinessStage) -> bool {
        match stage {
            RuntimeResponseReadinessStage::ResponseEnvelope => self.response_envelope_ready(),
            RuntimeResponseReadinessStage::ResponseRequestParity => self.response_request_ready(),
            RuntimeResponseReadinessStage::ResponseGate => self.response_gate_ready(),
        }
    }

    pub fn stage_signal_component_count(self, stage: RuntimeResponseReadinessStage) -> usize {
        match stage {
            RuntimeResponseReadinessStage::ResponseEnvelope => {
                self.response_envelope_signal_component_count
            }
            RuntimeResponseReadinessStage::ResponseRequestParity => {
                self.response_request_signal_component_count
            }
            RuntimeResponseReadinessStage::ResponseGate => {
                self.response_gate_signal_component_count
            }
        }
    }

    pub fn stage_blocker_component_count(self, stage: RuntimeResponseReadinessStage) -> usize {
        match stage {
            RuntimeResponseReadinessStage::ResponseEnvelope => {
                self.response_envelope_blocker_component_count
            }
            RuntimeResponseReadinessStage::ResponseRequestParity => {
                self.response_request_blocker_component_count
            }
            RuntimeResponseReadinessStage::ResponseGate => {
                self.response_gate_blocker_component_count
            }
        }
    }

    pub fn first_unready_stage(self) -> Option<RuntimeResponseReadinessStage> {
        Self::stage_order()
            .into_iter()
            .find(|stage| !self.stage_ready(*stage))
    }

    pub fn first_blocking_stage(self) -> Option<RuntimeResponseReadinessStage> {
        Self::stage_order()
            .into_iter()
            .find(|stage| self.stage_blocker_component_count(*stage) > 0)
    }

    pub fn runtime_response_readiness_signal_component_count(self) -> usize {
        self.response_envelope_signal_component_count
            .saturating_add(self.response_request_signal_component_count)
            .saturating_add(self.response_gate_signal_component_count)
    }

    pub fn has_runtime_response_readiness_signals(self) -> bool {
        self.runtime_response_readiness_signal_component_count() > 0
    }

    pub fn runtime_response_readiness_blocker_component_count(self) -> usize {
        self.response_envelope_blocker_component_count
            .saturating_add(self.response_request_blocker_component_count)
            .saturating_add(self.response_gate_blocker_component_count)
    }

    pub fn has_runtime_response_readiness_blockers(self) -> bool {
        self.runtime_response_readiness_blocker_component_count() > 0
    }

    pub fn runtime_response_readiness_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .response_envelope_signal_component_count
            .saturating_add(self.response_request_signal_component_count)
            .saturating_add(self.response_gate_signal_component_count);
        let expected_blocker_count = self
            .response_envelope_blocker_component_count
            .saturating_add(self.response_request_blocker_component_count)
            .saturating_add(self.response_gate_blocker_component_count);

        self.runtime_response_readiness_signal_component_count() == expected_signal_count
            && self.has_runtime_response_readiness_signals() == (expected_signal_count > 0)
            && self.runtime_response_readiness_blocker_component_count() == expected_blocker_count
            && self.has_runtime_response_readiness_blockers() == (expected_blocker_count > 0)
            && self
                .response_envelope
                .runtime_response_envelope_commit_accounting_is_consistent()
            && self
                .response_request
                .response_wire_accounting_is_consistent()
            && self
                .response_gate
                .runtime_response_commit_accounting_is_consistent()
    }

    pub fn runtime_response_readiness_is_clean(self) -> bool {
        !self.has_runtime_response_readiness_blockers()
            && self.runtime_response_readiness_accounting_is_consistent()
    }

    pub fn can_commit_runtime_response_readiness(self) -> bool {
        self.runtime_response_readiness_is_clean()
            && self.response_envelope_ready()
            && self.response_request_ready()
            && self.response_gate_ready()
    }
}

impl RuntimeResponseEnvelopeSummary {
    pub fn has_answer(self) -> bool {
        self.answer_chars > 0
    }

    pub fn has_generated_tokens(self) -> bool {
        self.token_count > 0
    }

    pub fn has_kv_exchange(self) -> bool {
        self.imported_kv_blocks > 0 || self.exported_kv_blocks > 0
    }

    pub fn has_runtime_kv_activity(self) -> bool {
        self.has_kv_exchange() || self.diagnostics_weak_runtime_kv_imports_skipped > 0
    }

    pub fn has_token_uncertainty(self) -> bool {
        self.has_uncertainty_signal
    }

    pub fn token_uncertainty_coverage_signal_component_count(self) -> usize {
        self.token_uncertainty_coverage_signal_count
    }

    pub fn has_token_uncertainty_coverage_signals(self) -> bool {
        self.token_uncertainty_coverage_signal_count > 0
    }

    pub fn token_uncertainty_metric_problem_component_count(self) -> usize {
        self.token_uncertainty_metric_problem_count
    }

    pub fn has_token_uncertainty_metric_problem_components(self) -> bool {
        self.token_uncertainty_metric_problem_count > 0
    }

    pub fn token_uncertainty_accounting_is_consistent(self) -> bool {
        self.token_uncertainty_accounting_consistent
            && self.token_uncertainty_metric_problem_count == 0
            && !self.has_token_uncertainty_metric_problem_components()
    }

    pub fn schema_matches_runtime_response(self) -> bool {
        self.schema == RUNTIME_RESPONSE_SCHEMA
    }

    pub fn envelope_kv_exchange_total(self) -> usize {
        self.imported_kv_blocks
            .saturating_add(self.exported_kv_blocks)
    }

    pub fn diagnostics_kv_exchange_total(self) -> usize {
        self.diagnostics_imported_kv_blocks
            .saturating_add(self.diagnostics_exported_kv_blocks)
    }

    pub fn diagnostics_kv_activity_total(self) -> usize {
        self.diagnostics_kv_exchange_total()
            .saturating_add(self.diagnostics_weak_runtime_kv_imports_skipped)
    }

    pub fn kv_counts_match_diagnostics(self) -> bool {
        self.imported_kv_blocks == self.diagnostics_imported_kv_blocks
            && self.exported_kv_blocks == self.diagnostics_exported_kv_blocks
    }

    pub fn runtime_response_envelope_commit_signal_component_count(self) -> usize {
        usize::from(self.has_answer())
            .saturating_add(usize::from(self.has_generated_tokens()))
            .saturating_add(usize::from(self.has_runtime_kv_activity()))
            .saturating_add(usize::from(self.has_token_uncertainty()))
            .saturating_add(self.token_uncertainty_coverage_signal_component_count())
            .saturating_add(usize::from(self.has_runtime_execution_signal))
    }

    pub fn has_runtime_response_envelope_commit_signals(self) -> bool {
        self.runtime_response_envelope_commit_signal_component_count() > 0
    }

    pub fn runtime_response_envelope_commit_blocker_component_count(self) -> usize {
        usize::from(!self.schema_matches_runtime_response())
            .saturating_add(usize::from(!self.has_answer()))
            .saturating_add(usize::from(!self.kv_counts_match_diagnostics()))
            .saturating_add(usize::from(
                !self.token_uncertainty_accounting_is_consistent(),
            ))
    }

    pub fn has_runtime_response_envelope_commit_blockers(self) -> bool {
        self.runtime_response_envelope_commit_blocker_component_count() > 0
    }

    pub fn runtime_response_envelope_commit_accounting_is_consistent(self) -> bool {
        let expected_signal_count = usize::from(self.has_answer())
            .saturating_add(usize::from(self.has_generated_tokens()))
            .saturating_add(usize::from(self.has_runtime_kv_activity()))
            .saturating_add(usize::from(self.has_token_uncertainty()))
            .saturating_add(self.token_uncertainty_coverage_signal_component_count())
            .saturating_add(usize::from(self.has_runtime_execution_signal));
        let expected_blocker_count = usize::from(!self.schema_matches_runtime_response())
            .saturating_add(usize::from(!self.has_answer()))
            .saturating_add(usize::from(!self.kv_counts_match_diagnostics()))
            .saturating_add(usize::from(
                !self.token_uncertainty_accounting_is_consistent(),
            ));

        self.runtime_response_envelope_commit_signal_component_count() == expected_signal_count
            && self.has_runtime_response_envelope_commit_signals() == (expected_signal_count > 0)
            && self.runtime_response_envelope_commit_blocker_component_count()
                == expected_blocker_count
            && self.has_runtime_response_envelope_commit_blockers() == (expected_blocker_count > 0)
    }

    pub fn runtime_response_envelope_commit_is_clean(self) -> bool {
        self.runtime_response_envelope_commit_blocker_component_count() == 0
            && self.runtime_response_envelope_commit_accounting_is_consistent()
    }

    pub fn response_envelope_shape_is_clean(self) -> bool {
        self.runtime_response_envelope_commit_is_clean()
    }

    pub fn can_commit_runtime_response_envelope(self) -> bool {
        self.runtime_response_envelope_commit_is_clean()
            && self.has_generated_tokens()
            && self.has_runtime_execution_signal
    }

    pub fn can_use_runtime_response_envelope(self) -> bool {
        self.can_commit_runtime_response_envelope()
    }
}

impl RuntimeResponseRequestParitySummary {
    pub fn planned_kv_summary(self) -> RuntimeResponsePlannedKvSummary {
        RuntimeResponsePlannedKvSummary {
            has_planning_digest: self.has_planning_digest(),
            imported_kv_blocks: self.imported_kv_blocks,
            exported_kv_blocks: self.exported_kv_blocks,
            planned_imported_kv_blocks: self.planned_imported_kv_blocks,
            planned_exported_kv_blocks: self.planned_exported_kv_blocks,
            imported_kv_within_planning: self.imported_kv_within_planning,
            exported_kv_within_planning: self.exported_kv_within_planning,
        }
    }

    pub fn manifest_kv_summary(
        self,
        manifest_kv_bridge: RuntimePlanningManifestKvBridgeSummary,
    ) -> RuntimeResponseManifestKvSummary {
        RuntimeResponseManifestKvSummary::new(manifest_kv_bridge, self.planned_kv_summary())
    }

    pub fn has_planning_digest(self) -> bool {
        self.planned_backend_max_tokens.is_some()
    }

    pub fn token_drifted_from_request(self) -> bool {
        !self.token_count_within_request
    }

    pub fn token_drifted_from_planning(self) -> bool {
        self.token_count_within_planning == Some(false)
    }

    pub fn imported_kv_drifted_from_request(self) -> bool {
        !self.imported_kv_matches_request
    }

    pub fn imported_kv_exceeds_planning(self) -> bool {
        self.imported_kv_within_planning == Some(false)
    }

    pub fn exported_kv_exceeds_runtime(self) -> bool {
        !self.exported_kv_within_runtime
    }

    pub fn exported_kv_exceeds_planning(self) -> bool {
        self.exported_kv_within_planning == Some(false)
    }

    pub fn adapter_missing_from_runtime(self) -> bool {
        self.request_selected_adapter.is_some() && !self.runtime_adapter_reported
    }

    pub fn adapter_drifted_from_request(self) -> bool {
        self.request_selected_adapter.is_some()
            && self.runtime_adapter_reported
            && !self.selected_adapter_matches_request
    }

    pub fn generation_budget_missing(self) -> bool {
        !self.generation_budget_reported
    }

    pub fn generation_budget_drifted(self) -> bool {
        self.generation_budget_reported && !self.generation_budget_matches_request
    }

    pub fn route_budget_drifted(self) -> bool {
        !self.route_budget_matches_request
    }

    pub fn hardware_pressure_drifted(self) -> bool {
        !self.hardware_pressure_matches_request
    }

    pub fn planning_diagnostics_drifted(self) -> bool {
        self.compute_headroom_matches_planning == Some(false)
            || self.latency_budget_matches_planning == Some(false)
    }

    pub fn planning_has_pre_request_gate_problems(self) -> bool {
        self.planning_pre_request_problem_count > 0
    }

    pub fn planning_has_pressure_signals(self) -> bool {
        self.planning_pressure_signal_count > 0
    }

    pub fn token_drift_component_count(self) -> usize {
        usize::from(self.token_drifted_from_request())
            + usize::from(self.token_drifted_from_planning())
    }

    pub fn request_token_drift_component_count(self) -> usize {
        usize::from(self.token_drifted_from_request())
    }

    pub fn planning_token_drift_component_count(self) -> usize {
        usize::from(self.token_drifted_from_planning())
    }

    pub fn kv_drift_component_count(self) -> usize {
        usize::from(self.imported_kv_drifted_from_request())
            + usize::from(self.imported_kv_exceeds_planning())
            + usize::from(self.exported_kv_exceeds_runtime())
            + usize::from(self.exported_kv_exceeds_planning())
    }

    pub fn request_kv_drift_component_count(self) -> usize {
        usize::from(self.imported_kv_drifted_from_request())
    }

    pub fn runtime_kv_drift_component_count(self) -> usize {
        usize::from(self.exported_kv_exceeds_runtime())
    }

    pub fn planning_kv_drift_component_count(self) -> usize {
        usize::from(self.imported_kv_exceeds_planning())
            + usize::from(self.exported_kv_exceeds_planning())
    }

    pub fn adapter_drift_component_count(self) -> usize {
        usize::from(self.adapter_missing_from_runtime())
            + usize::from(self.adapter_drifted_from_request())
    }

    pub fn diagnostics_drift_component_count(self) -> usize {
        usize::from(self.generation_budget_missing())
            + usize::from(self.generation_budget_drifted())
            + usize::from(self.route_budget_drifted())
            + usize::from(self.hardware_pressure_drifted())
            + usize::from(self.planning_diagnostics_drifted())
    }

    pub fn request_diagnostics_drift_component_count(self) -> usize {
        usize::from(self.generation_budget_missing())
            + usize::from(self.generation_budget_drifted())
            + usize::from(self.route_budget_drifted())
            + usize::from(self.hardware_pressure_drifted())
    }

    pub fn planning_diagnostics_drift_component_count(self) -> usize {
        usize::from(self.compute_headroom_matches_planning == Some(false))
            + usize::from(self.latency_budget_matches_planning == Some(false))
    }

    pub fn planning_pre_request_gate_problem_component_count(self) -> usize {
        usize::from(self.planning_has_pre_request_gate_problems())
    }

    pub fn planning_pressure_signal_component_count(self) -> usize {
        usize::from(self.planning_has_pressure_signals())
    }

    pub fn request_drift_component_count(self) -> usize {
        self.token_drift_component_count()
            .saturating_add(self.kv_drift_component_count())
            .saturating_add(self.adapter_drift_component_count())
            .saturating_add(self.diagnostics_drift_component_count())
    }

    pub fn token_parity_ok(self) -> bool {
        self.token_count_within_request && self.token_count_within_planning.unwrap_or(true)
    }

    pub fn kv_parity_ok(self) -> bool {
        self.imported_kv_matches_request
            && self.imported_kv_within_planning.unwrap_or(true)
            && self.exported_kv_within_runtime
            && self.exported_kv_within_planning.unwrap_or(true)
    }

    pub fn adapter_parity_ok(self) -> bool {
        match self.request_selected_adapter {
            Some(_) => self.runtime_adapter_reported && self.selected_adapter_matches_request,
            None => true,
        }
    }

    pub fn diagnostics_parity_ok(self) -> bool {
        self.generation_budget_reported
            && self.generation_budget_matches_request
            && self.route_budget_matches_request
            && self.hardware_pressure_matches_request
            && self.compute_headroom_matches_planning.unwrap_or(true)
            && self.latency_budget_matches_planning.unwrap_or(true)
    }

    pub fn response_wire_problem_component_count(self) -> usize {
        self.request_drift_component_count()
            .saturating_add(self.planning_pre_request_gate_problem_component_count())
    }

    pub fn has_response_wire_problem_components(self) -> bool {
        self.response_wire_problem_component_count() > 0
    }

    pub fn response_wire_accounting_is_consistent(self) -> bool {
        let expected_problem_count = self
            .token_drift_component_count()
            .saturating_add(self.kv_drift_component_count())
            .saturating_add(self.adapter_drift_component_count())
            .saturating_add(self.diagnostics_drift_component_count())
            .saturating_add(self.planning_pre_request_gate_problem_component_count());

        self.response_wire_problem_component_count() == expected_problem_count
            && self.has_response_wire_problem_components() == (expected_problem_count > 0)
            && self.request_parity_is_consistent() == (expected_problem_count == 0)
    }

    pub fn response_wire_shape_is_clean(self) -> bool {
        !self.has_response_wire_problem_components()
            && self.response_wire_accounting_is_consistent()
    }

    pub fn can_use_response_wire(self) -> bool {
        self.request_parity_is_consistent() && self.response_wire_shape_is_clean()
    }

    pub fn request_parity_is_consistent(self) -> bool {
        self.token_parity_ok()
            && self.kv_parity_ok()
            && self.adapter_parity_ok()
            && self.diagnostics_parity_ok()
            && !self.planning_has_pre_request_gate_problems()
    }
}

impl RuntimeResponsePlannedKvSummary {
    pub fn has_response_kv_activity(self) -> bool {
        self.imported_kv_blocks > 0 || self.exported_kv_blocks > 0
    }

    pub fn has_planned_kv_activity(self) -> bool {
        self.planned_imported_kv_blocks.unwrap_or(0) > 0
            || self.planned_exported_kv_blocks.unwrap_or(0) > 0
    }

    pub fn planning_limits_reported(self) -> bool {
        self.planned_imported_kv_blocks.is_some() && self.planned_exported_kv_blocks.is_some()
    }

    pub fn imported_kv_exceeds_planning(self) -> bool {
        self.imported_kv_within_planning == Some(false)
    }

    pub fn exported_kv_exceeds_planning(self) -> bool {
        self.exported_kv_within_planning == Some(false)
    }

    pub fn response_kv_within_planning(self) -> bool {
        self.imported_kv_within_planning.unwrap_or(true)
            && self.exported_kv_within_planning.unwrap_or(true)
    }

    pub fn response_kv_matches_planned_zero_export(self) -> bool {
        self.planned_exported_kv_blocks == Some(0) && self.exported_kv_blocks == 0
    }

    pub fn response_kv_exceeds_planning(self) -> bool {
        self.imported_kv_exceeds_planning() || self.exported_kv_exceeds_planning()
    }

    pub fn response_kv_activity_signal_component_count(self) -> usize {
        usize::from(self.imported_kv_blocks > 0) + usize::from(self.exported_kv_blocks > 0)
    }

    pub fn planned_kv_activity_signal_component_count(self) -> usize {
        usize::from(self.planned_imported_kv_blocks.unwrap_or(0) > 0)
            + usize::from(self.planned_exported_kv_blocks.unwrap_or(0) > 0)
    }

    pub fn response_planned_kv_signal_component_count(self) -> usize {
        usize::from(self.has_planning_digest)
            .saturating_add(self.response_kv_activity_signal_component_count())
            .saturating_add(self.planned_kv_activity_signal_component_count())
    }

    pub fn has_response_planned_kv_signals(self) -> bool {
        self.response_planned_kv_signal_component_count() > 0
    }

    pub fn planning_limit_missing_component_count(self) -> usize {
        usize::from(self.has_planning_digest && self.planned_imported_kv_blocks.is_none())
            .saturating_add(usize::from(
                self.has_planning_digest && self.planned_exported_kv_blocks.is_none(),
            ))
    }

    pub fn planning_kv_exceeded_component_count(self) -> usize {
        usize::from(self.imported_kv_exceeds_planning())
            .saturating_add(usize::from(self.exported_kv_exceeds_planning()))
    }

    pub fn response_planned_kv_problem_component_count(self) -> usize {
        self.planning_limit_missing_component_count()
            .saturating_add(self.planning_kv_exceeded_component_count())
    }

    pub fn has_response_planned_kv_problem_components(self) -> bool {
        self.response_planned_kv_problem_component_count() > 0
    }

    pub fn response_planned_kv_accounting_is_consistent(self) -> bool {
        let expected_signal_count = usize::from(self.has_planning_digest)
            .saturating_add(usize::from(self.imported_kv_blocks > 0))
            .saturating_add(usize::from(self.exported_kv_blocks > 0))
            .saturating_add(usize::from(
                self.planned_imported_kv_blocks.unwrap_or(0) > 0,
            ))
            .saturating_add(usize::from(
                self.planned_exported_kv_blocks.unwrap_or(0) > 0,
            ));
        let expected_problem_count =
            usize::from(self.has_planning_digest && self.planned_imported_kv_blocks.is_none())
                .saturating_add(usize::from(
                    self.has_planning_digest && self.planned_exported_kv_blocks.is_none(),
                ))
                .saturating_add(usize::from(self.imported_kv_exceeds_planning()))
                .saturating_add(usize::from(self.exported_kv_exceeds_planning()));

        self.response_planned_kv_signal_component_count() == expected_signal_count
            && self.has_response_planned_kv_signals() == (expected_signal_count > 0)
            && self.response_planned_kv_problem_component_count() == expected_problem_count
            && self.has_response_planned_kv_problem_components() == (expected_problem_count > 0)
    }

    pub fn response_planned_kv_shape_is_clean(self) -> bool {
        !self.has_response_planned_kv_problem_components()
            && self.response_planned_kv_accounting_is_consistent()
    }

    pub fn can_use_response_planned_kv(self) -> bool {
        self.response_planned_kv_shape_is_clean()
    }

    pub fn can_commit_planned_kv_response(self) -> bool {
        self.has_planning_digest
            && self.planning_limits_reported()
            && self.response_kv_within_planning()
            && self.response_planned_kv_shape_is_clean()
    }
}

impl RuntimeResponseManifestKvSummary {
    pub fn new(
        manifest_kv_bridge: RuntimePlanningManifestKvBridgeSummary,
        response_planned_kv: RuntimeResponsePlannedKvSummary,
    ) -> Self {
        let response_imported_kv_within_manifest_plan = response_planned_kv.imported_kv_blocks
            <= manifest_kv_bridge.import.import_plan_max_blocks;
        let response_exported_kv_within_manifest_plan = response_planned_kv.exported_kv_blocks
            <= manifest_kv_bridge.export.export_plan_max_blocks;

        Self {
            manifest_kv_bridge,
            response_planned_kv,
            response_imported_kv_within_manifest_plan,
            response_exported_kv_within_manifest_plan,
            manifest_kv_bridge_signal_component_count: manifest_kv_bridge
                .manifest_kv_bridge_signal_component_count(),
            response_planned_kv_signal_component_count: response_planned_kv
                .response_planned_kv_signal_component_count(),
            manifest_kv_bridge_blocker_component_count: manifest_kv_bridge
                .manifest_kv_bridge_problem_component_count(),
            response_planned_kv_blocker_component_count: response_planned_kv
                .response_planned_kv_problem_component_count(),
            response_manifest_kv_blocker_component_count: usize::from(
                !response_imported_kv_within_manifest_plan,
            )
            .saturating_add(usize::from(!response_exported_kv_within_manifest_plan)),
        }
    }

    pub fn manifest_bridge_ready(self) -> bool {
        self.manifest_kv_bridge
            .can_use_runtime_planning_manifest_kv_bridge()
    }

    pub fn response_planned_kv_ready(self) -> bool {
        self.response_planned_kv.can_commit_planned_kv_response()
    }

    pub fn manifest_import_plan_covers_response(self) -> bool {
        self.response_imported_kv_within_manifest_plan
    }

    pub fn manifest_export_plan_covers_response(self) -> bool {
        self.response_exported_kv_within_manifest_plan
    }

    pub fn response_kv_within_manifest_plan(self) -> bool {
        self.manifest_import_plan_covers_response() && self.manifest_export_plan_covers_response()
    }

    pub fn response_manifest_kv_signal_component_count(self) -> usize {
        self.manifest_kv_bridge_signal_component_count
            .saturating_add(self.response_planned_kv_signal_component_count)
    }

    pub fn has_response_manifest_kv_signals(self) -> bool {
        self.response_manifest_kv_signal_component_count() > 0
    }

    pub fn manifest_bridge_blocker_component_count(self) -> usize {
        self.manifest_kv_bridge_blocker_component_count
    }

    pub fn response_planned_kv_blocker_component_count(self) -> usize {
        self.response_planned_kv_blocker_component_count
    }

    pub fn response_kv_exceeds_manifest_plan_component_count(self) -> usize {
        self.response_manifest_kv_blocker_component_count
    }

    pub fn response_manifest_kv_blocker_component_count(self) -> usize {
        self.manifest_bridge_blocker_component_count()
            .saturating_add(self.response_planned_kv_blocker_component_count())
            .saturating_add(self.response_kv_exceeds_manifest_plan_component_count())
    }

    pub fn has_response_manifest_kv_blockers(self) -> bool {
        self.response_manifest_kv_blocker_component_count() > 0
    }

    pub fn response_manifest_kv_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .manifest_kv_bridge
            .manifest_kv_bridge_signal_component_count()
            .saturating_add(
                self.response_planned_kv
                    .response_planned_kv_signal_component_count(),
            );
        let expected_response_manifest_blockers =
            usize::from(!self.response_imported_kv_within_manifest_plan)
                .saturating_add(usize::from(!self.response_exported_kv_within_manifest_plan));
        let expected_blocker_count = self
            .manifest_kv_bridge
            .manifest_kv_bridge_problem_component_count()
            .saturating_add(
                self.response_planned_kv
                    .response_planned_kv_problem_component_count(),
            )
            .saturating_add(expected_response_manifest_blockers);

        self.manifest_kv_bridge
            .manifest_kv_bridge_accounting_is_consistent()
            && self
                .response_planned_kv
                .response_planned_kv_accounting_is_consistent()
            && self.response_manifest_kv_signal_component_count() == expected_signal_count
            && self.has_response_manifest_kv_signals() == (expected_signal_count > 0)
            && self.response_manifest_kv_blocker_component_count() == expected_blocker_count
            && self.has_response_manifest_kv_blockers() == (expected_blocker_count > 0)
            && self.response_manifest_kv_blocker_component_count
                == expected_response_manifest_blockers
    }

    pub fn response_manifest_kv_shape_is_clean(self) -> bool {
        !self.has_response_manifest_kv_blockers()
            && self.response_manifest_kv_accounting_is_consistent()
    }

    pub fn can_use_response_manifest_kv(self) -> bool {
        self.response_manifest_kv_shape_is_clean()
            && self.manifest_bridge_ready()
            && self.response_planned_kv_ready()
            && self.response_kv_within_manifest_plan()
    }

    pub fn can_commit_response_manifest_kv(self) -> bool {
        self.can_use_response_manifest_kv()
    }
}

impl RuntimeResponseGateSummary {
    pub fn has_acceptance_failures(self) -> bool {
        !self.response_accepted
    }

    pub fn has_response_contract_failures(self) -> bool {
        self.response_violation_count > 0
    }

    pub fn has_request_parity_failures(self) -> bool {
        self.request_violation_count > 0
    }

    pub fn has_exported_kv_failures(self) -> bool {
        self.exported_kv_violation_count > 0
    }

    pub fn envelope_drifted(self) -> bool {
        !self.envelope_consistent
    }

    pub fn request_parity_drifted(self) -> bool {
        !self.request_parity_consistent
    }

    pub fn has_response_wire_problem_components(self) -> bool {
        self.response_wire_problem_count > 0
    }

    pub fn has_planning_pre_request_gate_problems(self) -> bool {
        self.planning_pre_request_problem_count > 0
    }

    pub fn has_planning_pressure_signals(self) -> bool {
        self.planning_pressure_signal_count > 0
    }

    pub fn response_wire_problem_component_count(self) -> usize {
        self.response_wire_problem_count
    }

    pub fn direct_response_wire_problem_component_count(self) -> usize {
        self.response_wire_problem_count
            .saturating_sub(self.planning_pre_request_problem_count)
    }

    pub fn planning_pre_request_gate_problem_component_count(self) -> usize {
        usize::from(self.has_planning_pre_request_gate_problems())
    }

    pub fn planning_pressure_signal_component_count(self) -> usize {
        usize::from(self.has_planning_pressure_signals())
    }

    pub fn exported_kv_activity_signal_component_count(self) -> usize {
        usize::from(self.accepted_exported_kv_blocks > 0)
    }

    pub fn response_gate_signal_component_count(self) -> usize {
        self.planning_pressure_signal_count
            .saturating_add(self.exported_kv_activity_signal_component_count())
    }

    pub fn response_gate_has_signal_components(self) -> bool {
        self.response_gate_signal_component_count() > 0
    }

    pub fn response_wire_accounting_is_consistent(self) -> bool {
        self.response_wire_problem_count >= self.planning_pre_request_problem_count
            && self.request_parity_consistent == !self.has_response_wire_problem_components()
    }

    pub fn exported_kv_drifted(self) -> bool {
        !self.exported_kv_accepted
    }

    pub fn has_boundary_drift(self) -> bool {
        self.envelope_drifted() || self.request_parity_drifted() || self.exported_kv_drifted()
    }

    pub fn has_failure_reports(self) -> bool {
        self.failure_report_count > 0
    }

    pub fn has_total_violations(self) -> bool {
        self.response_violation_count
            .saturating_add(self.request_violation_count)
            .saturating_add(self.exported_kv_violation_count)
            > 0
    }

    pub fn response_contract_failure_component_count(self) -> usize {
        usize::from(self.has_response_contract_failures())
    }

    pub fn request_parity_failure_component_count(self) -> usize {
        usize::from(self.has_request_parity_failures())
    }

    pub fn exported_kv_failure_component_count(self) -> usize {
        usize::from(self.has_exported_kv_failures())
    }

    pub fn acceptance_failure_component_count(self) -> usize {
        self.response_contract_failure_component_count()
            + self.request_parity_failure_component_count()
            + self.exported_kv_failure_component_count()
    }

    pub fn envelope_blocker_component_count(self) -> usize {
        usize::from(self.envelope_drifted())
    }

    pub fn request_parity_blocker_component_count(self) -> usize {
        usize::from(self.request_parity_drifted())
    }

    pub fn exported_kv_blocker_component_count(self) -> usize {
        usize::from(self.exported_kv_drifted())
    }

    pub fn boundary_drift_component_count(self) -> usize {
        self.envelope_blocker_component_count()
            + self.request_parity_blocker_component_count()
            + self.exported_kv_blocker_component_count()
    }

    pub fn mapped_failure_report_component_count(self) -> usize {
        usize::from(self.has_failure_reports())
    }

    pub fn response_blocker_component_count(self) -> usize {
        self.acceptance_failure_component_count()
            .saturating_add(self.boundary_drift_component_count())
            .saturating_add(self.mapped_failure_report_component_count())
    }

    pub fn response_gate_has_problem_components(self) -> bool {
        self.response_blocker_component_count() > 0
    }

    pub fn response_gate_accounting_is_consistent(self) -> bool {
        self.response_blocker_component_count()
            == self
                .acceptance_failure_component_count()
                .saturating_add(self.boundary_drift_component_count())
                .saturating_add(self.mapped_failure_report_component_count())
    }

    pub fn failure_report_matches_failures(self) -> bool {
        self.failure_report_count
            == self.response_contract_failure_component_count()
                + self.request_parity_failure_component_count()
                + self.exported_kv_failure_component_count()
    }

    pub fn can_accept_response(self) -> bool {
        !self.has_acceptance_failures() && !self.has_boundary_drift()
    }

    pub fn is_clean_response_gate(self) -> bool {
        self.can_accept_response()
            && !self.has_response_contract_failures()
            && !self.has_request_parity_failures()
            && !self.has_exported_kv_failures()
            && self.failure_report_count == 0
    }

    pub fn response_gate_shape_is_clean(self) -> bool {
        self.is_clean_response_gate()
            && self.response_wire_accounting_is_consistent()
            && self.response_gate_accounting_is_consistent()
            && self.failure_report_matches_failures()
    }

    pub fn runtime_response_commit_signal_component_count(self) -> usize {
        self.response_gate_signal_component_count()
    }

    pub fn has_runtime_response_commit_signals(self) -> bool {
        self.runtime_response_commit_signal_component_count() > 0
    }

    pub fn runtime_response_commit_blocker_component_count(self) -> usize {
        self.response_blocker_component_count()
            .saturating_add(self.direct_response_wire_problem_component_count())
    }

    pub fn has_runtime_response_commit_blockers(self) -> bool {
        self.runtime_response_commit_blocker_component_count() > 0
    }

    pub fn runtime_response_commit_accounting_is_consistent(self) -> bool {
        let expected_blocker_count = self
            .response_blocker_component_count()
            .saturating_add(self.direct_response_wire_problem_component_count());

        self.response_wire_accounting_is_consistent()
            && self.response_gate_accounting_is_consistent()
            && self.failure_report_matches_failures()
            && self.runtime_response_commit_signal_component_count()
                == self.response_gate_signal_component_count()
            && self.has_runtime_response_commit_signals()
                == (self.runtime_response_commit_signal_component_count() > 0)
            && self.runtime_response_commit_blocker_component_count() == expected_blocker_count
            && self.has_runtime_response_commit_blockers() == (expected_blocker_count > 0)
    }

    pub fn runtime_response_commit_is_clean(self) -> bool {
        !self.has_runtime_response_commit_blockers()
            && self.runtime_response_commit_accounting_is_consistent()
    }

    pub fn can_commit_runtime_response(self) -> bool {
        self.can_accept_response()
            && self.response_gate_shape_is_clean()
            && self.runtime_response_commit_is_clean()
    }

    pub fn can_accept_runtime_response(self) -> bool {
        self.can_accept_response() && self.response_gate_shape_is_clean()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeResponseAcceptanceReport {
    pub response_violations: Vec<String>,
    pub request_violations: Vec<String>,
    pub exported_kv_report: RuntimeKvValidationReport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeResponseAcceptanceSummary {
    pub accepted: bool,
    pub response_violation_count: usize,
    pub request_violation_count: usize,
    pub exported_kv_violation_count: usize,
    pub accepted_exported_kv_blocks: usize,
    pub failure_report_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeResponseFailureReturnSource {
    ResponseAcceptance,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeResponseFailureReturnSummary {
    pub source: RuntimeResponseFailureReturnSource,
    pub can_commit: bool,
    pub should_return_failure: bool,
    pub has_primary_failure_summary: bool,
    pub primary_failure_summary: Option<RuntimeFailureSummary>,
    pub failure_batch: RuntimeFailureBatchSummary,
    pub failure_report_count: usize,
    pub can_format_runtime_failures: bool,
    pub total_blocker_component_count: usize,
    pub commit_decision_accounting_consistent: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeResponseFailureReturnReport {
    pub source: RuntimeResponseFailureReturnSource,
    pub primary_failure: RuntimeFailureReport,
    pub primary_failure_summary: RuntimeFailureSummary,
    pub failure_batch: RuntimeFailureBatchSummary,
    pub failure_report_count: usize,
    pub can_format_runtime_failures: bool,
    pub total_blocker_component_count: usize,
}

impl RuntimeResponseFailureReturnSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::ResponseAcceptance => "runtime_response_acceptance",
        }
    }
}

impl RuntimeResponseFailureReturnSummary {
    pub fn new(
        source: RuntimeResponseFailureReturnSource,
        can_commit: bool,
        should_return_failure: bool,
        primary_failure_summary: Option<RuntimeFailureSummary>,
        failure_batch: RuntimeFailureBatchSummary,
        failure_report_count: usize,
        can_format_runtime_failures: bool,
        total_blocker_component_count: usize,
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
            total_blocker_component_count,
            commit_decision_accounting_consistent,
        }
    }

    pub fn has_failure_reports(self) -> bool {
        self.failure_report_count > 0
    }

    pub fn has_blocker_components(self) -> bool {
        self.total_blocker_component_count > 0
    }

    pub fn failure_return_accounting_is_consistent(self) -> bool {
        self.commit_decision_accounting_consistent
            && self.should_return_failure == (!self.can_commit && self.has_failure_reports())
            && self.has_primary_failure_summary == self.primary_failure_summary.is_some()
            && self.has_primary_failure_summary == self.has_failure_reports()
            && self.failure_batch.total_count == self.failure_report_count
            && self.can_format_runtime_failures == self.failure_batch.can_format_runtime_failures()
            && (!self.has_failure_reports() || self.has_blocker_components())
    }

    pub fn can_return_runtime_failure(self) -> bool {
        self.should_return_failure
            && self.has_primary_failure_summary
            && self.can_format_runtime_failures
            && self.failure_return_accounting_is_consistent()
    }
}

impl RuntimeResponseFailureReturnReport {
    pub fn new(
        source: RuntimeResponseFailureReturnSource,
        primary_failure: RuntimeFailureReport,
        failure_batch: RuntimeFailureBatchSummary,
        failure_report_count: usize,
        can_format_runtime_failures: bool,
        total_blocker_component_count: usize,
    ) -> Self {
        let primary_failure_summary = primary_failure.failure_summary();
        Self {
            source,
            primary_failure,
            primary_failure_summary,
            failure_batch,
            failure_report_count,
            can_format_runtime_failures,
            total_blocker_component_count,
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
            && self.total_blocker_component_count > 0
    }

    pub fn can_use_runtime_response_failure_return_report(&self) -> bool {
        self.failure_return_report_shape_is_clean()
    }
}

impl RuntimeResponseAcceptanceSummary {
    pub fn total_violation_count(self) -> usize {
        self.response_violation_count
            .saturating_add(self.request_violation_count)
            .saturating_add(self.exported_kv_violation_count)
    }

    pub fn has_response_contract_failures(self) -> bool {
        self.response_violation_count > 0
    }

    pub fn has_request_parity_failures(self) -> bool {
        self.request_violation_count > 0
    }

    pub fn has_exported_kv_failures(self) -> bool {
        self.exported_kv_violation_count > 0
    }

    pub fn has_failures(self) -> bool {
        self.has_response_contract_failures()
            || self.has_request_parity_failures()
            || self.has_exported_kv_failures()
    }

    pub fn response_contract_failure_component_count(self) -> usize {
        usize::from(self.has_response_contract_failures())
    }

    pub fn request_parity_failure_component_count(self) -> usize {
        usize::from(self.has_request_parity_failures())
    }

    pub fn exported_kv_failure_component_count(self) -> usize {
        usize::from(self.has_exported_kv_failures())
    }

    pub fn acceptance_failure_component_count(self) -> usize {
        self.response_contract_failure_component_count()
            .saturating_add(self.request_parity_failure_component_count())
            .saturating_add(self.exported_kv_failure_component_count())
    }

    pub fn has_failure_reports(self) -> bool {
        self.failure_report_count > 0
    }

    pub fn response_acceptance_problem_component_count(self) -> usize {
        self.acceptance_failure_component_count()
            .saturating_add(usize::from(self.has_failure_reports()))
    }

    pub fn has_response_acceptance_problem_components(self) -> bool {
        self.response_acceptance_problem_component_count() > 0
    }

    pub fn failure_report_matches_failures(self) -> bool {
        self.failure_report_count
            == usize::from(self.has_response_contract_failures())
                + usize::from(self.has_request_parity_failures())
                + usize::from(self.has_exported_kv_failures())
    }

    pub fn response_acceptance_accounting_is_consistent(self) -> bool {
        let expected_failure_count = self
            .response_contract_failure_component_count()
            .saturating_add(self.request_parity_failure_component_count())
            .saturating_add(self.exported_kv_failure_component_count());
        let expected_problem_count =
            expected_failure_count.saturating_add(usize::from(self.has_failure_reports()));

        self.acceptance_failure_component_count() == expected_failure_count
            && self.response_acceptance_problem_component_count() == expected_problem_count
            && self.has_response_acceptance_problem_components() == (expected_problem_count > 0)
            && self.has_failures() == (expected_failure_count > 0)
            && self.failure_report_matches_failures()
            && self.accepted == (self.total_violation_count() == 0)
    }

    pub fn is_clean_acceptance(self) -> bool {
        self.accepted
            && !self.has_failures()
            && self.failure_report_count == 0
            && self.response_acceptance_accounting_is_consistent()
    }

    pub fn runtime_response_acceptance_commit_signal_component_count(self) -> usize {
        usize::from(self.accepted) + usize::from(self.accepted_exported_kv_blocks > 0)
    }

    pub fn runtime_response_acceptance_commit_blocker_component_count(self) -> usize {
        self.response_acceptance_problem_component_count()
    }

    pub fn runtime_response_acceptance_commit_accounting_is_consistent(self) -> bool {
        let expected_signal_count =
            usize::from(self.accepted) + usize::from(self.accepted_exported_kv_blocks > 0);

        self.response_acceptance_accounting_is_consistent()
            && self.runtime_response_acceptance_commit_signal_component_count()
                == expected_signal_count
            && self.runtime_response_acceptance_commit_blocker_component_count()
                == self.response_acceptance_problem_component_count()
    }

    pub fn runtime_response_acceptance_commit_is_clean(self) -> bool {
        self.runtime_response_acceptance_commit_blocker_component_count() == 0
            && self.runtime_response_acceptance_commit_accounting_is_consistent()
    }

    pub fn response_acceptance_shape_is_clean(self) -> bool {
        self.runtime_response_acceptance_commit_is_clean()
    }

    pub fn can_commit_runtime_response_acceptance(self) -> bool {
        self.accepted && self.runtime_response_acceptance_commit_is_clean()
    }
}

impl RuntimeResponseAcceptanceReport {
    pub fn is_accepted(&self) -> bool {
        self.response_violations.is_empty()
            && self.request_violations.is_empty()
            && self.exported_kv_report.is_valid()
    }

    pub fn acceptance_summary(&self) -> RuntimeResponseAcceptanceSummary {
        RuntimeResponseAcceptanceSummary {
            accepted: self.is_accepted(),
            response_violation_count: self.response_violations.len(),
            request_violation_count: self.request_violations.len(),
            exported_kv_violation_count: self.exported_kv_report.violations.len(),
            accepted_exported_kv_blocks: self.exported_kv_report.accepted.len(),
            failure_report_count: usize::from(!self.response_violations.is_empty())
                + usize::from(!self.request_violations.is_empty())
                + usize::from(!self.exported_kv_report.violations.is_empty()),
        }
    }

    pub fn violations(&self) -> Vec<String> {
        let mut violations = self.response_violations.clone();
        violations.extend(self.request_violations.clone());
        violations.extend(self.exported_kv_report.violations.clone());
        violations
    }

    pub fn accepted_exported_kv_blocks(&self) -> &[crate::kv::KvBlock] {
        &self.exported_kv_report.accepted
    }

    pub fn failure_reports(&self) -> Vec<RuntimeFailureReport> {
        let mut failures = Vec::new();

        if !self.response_violations.is_empty() {
            failures.push(RuntimeFailureReport::contract_violation(
                acceptance_message(
                    "runtime response acceptance failed",
                    &self.response_violations,
                ),
            ));
        }
        if !self.request_violations.is_empty() {
            failures.push(RuntimeFailureReport::contract_violation(
                acceptance_message(
                    "runtime response request parity failed",
                    &self.request_violations,
                ),
            ));
        }
        if !self.exported_kv_report.violations.is_empty() {
            failures.push(RuntimeFailureReport::kv_export(acceptance_message(
                "runtime response exported KV rejected",
                &self.exported_kv_report.violations,
            )));
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

    pub fn failure_return_summary(&self) -> RuntimeResponseFailureReturnSummary {
        let acceptance = self.acceptance_summary();
        let failure_batch = self.failure_batch_summary();
        let failure_report_count = failure_batch.total_count;
        let can_commit = acceptance.can_commit_runtime_response_acceptance();
        RuntimeResponseFailureReturnSummary::new(
            RuntimeResponseFailureReturnSource::ResponseAcceptance,
            can_commit,
            !can_commit && failure_report_count > 0,
            self.primary_failure_summary(),
            failure_batch,
            failure_report_count,
            failure_batch.can_format_runtime_failures(),
            acceptance.runtime_response_acceptance_commit_blocker_component_count(),
            acceptance.runtime_response_acceptance_commit_accounting_is_consistent(),
        )
    }

    pub fn runtime_failure_return_report(&self) -> Option<RuntimeResponseFailureReturnReport> {
        let failure_return = self.failure_return_summary();
        if failure_return.can_return_runtime_failure() {
            self.primary_failure_report().map(|failure| {
                RuntimeResponseFailureReturnReport::new(
                    RuntimeResponseFailureReturnSource::ResponseAcceptance,
                    failure,
                    failure_return.failure_batch,
                    failure_return.failure_report_count,
                    failure_return.can_format_runtime_failures,
                    failure_return.total_blocker_component_count,
                )
            })
        } else {
            None
        }
    }
}

impl RuntimeResponseEnvelope {
    pub fn from_outcome(outcome: &InferenceOutcome) -> Self {
        Self {
            schema: RUNTIME_RESPONSE_SCHEMA,
            answer_chars: outcome.answer.trim().chars().count(),
            token_metrics: outcome.token_metrics(),
            imported_kv_blocks: outcome.imported_kv.len(),
            exported_kv_blocks: outcome.exported_kv.len(),
            diagnostics_imported_kv_blocks: outcome.diagnostics.runtime.imported_kv_blocks,
            diagnostics_exported_kv_blocks: outcome.diagnostics.runtime.exported_kv_blocks,
            diagnostics_weak_runtime_kv_imports_skipped: outcome
                .diagnostics
                .runtime
                .weak_runtime_kv_imports_skipped,
            has_runtime_execution_signal: outcome.diagnostics.has_runtime_execution_signal(),
        }
    }

    pub fn contract_violations(
        &self,
        outcome: &InferenceOutcome,
        metadata: &RuntimeMetadata,
        architecture: TransformerRuntimeArchitecture,
        hardware: &HardwarePlan,
    ) -> Vec<String> {
        let mut violations = Vec::new();

        if self.schema != RUNTIME_RESPONSE_SCHEMA {
            violations.push(format!(
                "runtime response schema {} does not match {}",
                self.schema, RUNTIME_RESPONSE_SCHEMA
            ));
        }
        if self.answer_chars == 0 {
            violations.push("runtime response answer must be non-empty".to_owned());
        }
        let empty_token_count = outcome
            .tokens
            .iter()
            .filter(|token| token.text.trim().is_empty())
            .count();
        if empty_token_count > 0 {
            violations.push(format!(
                "runtime response contains {empty_token_count} empty generated tokens"
            ));
        }
        if self.imported_kv_blocks != self.diagnostics_imported_kv_blocks {
            violations.push(format!(
                "runtime response imported KV count {} differs from diagnostics {}",
                self.imported_kv_blocks, self.diagnostics_imported_kv_blocks
            ));
        }
        if self.exported_kv_blocks != self.diagnostics_exported_kv_blocks {
            violations.push(format!(
                "runtime response exported KV count {} differs from diagnostics {}",
                self.exported_kv_blocks, self.diagnostics_exported_kv_blocks
            ));
        }
        violations.extend(outcome.diagnostics.runtime.contract_violations(
            metadata,
            architecture,
            &hardware.adapter_execution_context(),
        ));
        violations.extend(
            outcome
                .diagnostics
                .runtime
                .hardware_contract_violations(hardware),
        );

        violations
    }

    pub fn is_valid(
        &self,
        outcome: &InferenceOutcome,
        metadata: &RuntimeMetadata,
        architecture: TransformerRuntimeArchitecture,
        hardware: &HardwarePlan,
    ) -> bool {
        self.contract_violations(outcome, metadata, architecture, hardware)
            .is_empty()
    }

    pub fn request_contract_violations(
        &self,
        outcome: &InferenceOutcome,
        request: &RuntimeRequestEnvelope,
    ) -> Vec<String> {
        let mut violations = Vec::new();

        if self.token_metrics.token_count > request.max_tokens {
            violations.push(format!(
                "runtime response generated {} tokens above request max_tokens {}",
                self.token_metrics.token_count, request.max_tokens
            ));
        }
        if self.imported_kv_blocks != request.imported_kv_blocks {
            violations.push(format!(
                "runtime response imported KV count {} differs from request imported KV count {}",
                self.imported_kv_blocks, request.imported_kv_blocks
            ));
        }
        if !request.runtime.supports_kv_export && self.exported_kv_blocks > 0 {
            violations.push(format!(
                "runtime response exports {} KV blocks but request runtime KV export is disabled",
                self.exported_kv_blocks
            ));
        }
        if request.runtime.supports_kv_export
            && request.runtime.max_kv_export_blocks > 0
            && self.exported_kv_blocks > request.runtime.max_kv_export_blocks
        {
            violations.push(format!(
                "runtime response exports {} KV blocks above request runtime limit {}",
                self.exported_kv_blocks, request.runtime.max_kv_export_blocks
            ));
        }
        if let Some(expected_adapter) = request.selected_adapter
            && let Some(actual_adapter) = outcome.diagnostics.runtime.selected_adapter
            && actual_adapter != expected_adapter
        {
            violations.push(format!(
                "runtime response selected adapter {} differs from request adapter {}",
                actual_adapter.as_str(),
                expected_adapter.as_str()
            ));
        }
        if let Some(generation_budget) = outcome.diagnostics.generation_budget
            && generation_budget != request.generation_budget
        {
            violations.push(
                "runtime response generation budget differs from request generation budget"
                    .to_owned(),
            );
        }
        if outcome.diagnostics.route_budget != request.route_budget {
            violations
                .push("runtime response route budget differs from request route budget".to_owned());
        }
        if !float_close(
            outcome.diagnostics.hardware_pressure,
            request.hardware_pressure,
        ) {
            violations.push(format!(
                "runtime response hardware pressure {:.3} differs from request hardware pressure {:.3}",
                outcome.diagnostics.hardware_pressure, request.hardware_pressure
            ));
        }
        if let Some(planning) = request.planning {
            let planned_kv = planning.planned_kv_exchange();
            if self.token_metrics.token_count > planning.backend_max_tokens() {
                violations.push(format!(
                    "runtime response generated {} tokens above planned backend max_tokens {}",
                    self.token_metrics.token_count,
                    planning.backend_max_tokens()
                ));
            }
            if self.imported_kv_blocks > planned_kv.import_blocks {
                violations.push(format!(
                    "runtime response imported {} KV blocks above planned KV imports {}",
                    self.imported_kv_blocks, planned_kv.import_blocks
                ));
            }
            if self.exported_kv_blocks > planned_kv.export_blocks {
                violations.push(format!(
                    "runtime response exported {} KV blocks above planned KV exports {}",
                    self.exported_kv_blocks, planned_kv.export_blocks
                ));
            }
            if !float_close(
                outcome.diagnostics.compute_headroom,
                planning.compute_headroom,
            ) {
                violations.push(format!(
                    "runtime response compute headroom {:.3} differs from planned compute headroom {:.3}",
                    outcome.diagnostics.compute_headroom, planning.compute_headroom
                ));
            }
            if outcome.diagnostics.latency_budget_ms != planning.latency_budget_ms {
                violations.push(format!(
                    "runtime response latency budget {:?} differs from planned latency budget {:?}",
                    outcome.diagnostics.latency_budget_ms, planning.latency_budget_ms
                ));
            }
        }

        violations
    }

    pub fn request_parity_summary(
        &self,
        outcome: &InferenceOutcome,
        request: &RuntimeRequestEnvelope,
    ) -> RuntimeResponseRequestParitySummary {
        let planning = request.planning;
        let planned_kv = planning.map(|planning| planning.planned_kv_exchange());
        let planned_backend_max_tokens = planning.map(|planning| planning.backend_max_tokens());
        let token_count_within_planning =
            planned_backend_max_tokens.map(|limit| self.token_metrics.token_count <= limit);
        let imported_kv_within_planning =
            planned_kv.map(|planned| self.imported_kv_blocks <= planned.import_blocks);
        let exported_kv_within_planning =
            planned_kv.map(|planned| self.exported_kv_blocks <= planned.export_blocks);
        let runtime_selected_adapter = outcome.diagnostics.runtime.selected_adapter;
        let runtime_adapter_reported = runtime_selected_adapter.is_some();
        let selected_adapter_matches_request =
            runtime_adapter_reported && runtime_selected_adapter == request.selected_adapter;
        let generation_budget_reported = outcome.diagnostics.generation_budget.is_some();
        let generation_budget_matches_request =
            outcome.diagnostics.generation_budget == Some(request.generation_budget);
        let compute_headroom_matches_planning = planning.map(|planning| {
            float_close(
                outcome.diagnostics.compute_headroom,
                planning.compute_headroom,
            )
        });
        let latency_budget_matches_planning = planning
            .map(|planning| outcome.diagnostics.latency_budget_ms == planning.latency_budget_ms);

        RuntimeResponseRequestParitySummary {
            token_count: self.token_metrics.token_count,
            request_max_tokens: request.max_tokens,
            planned_backend_max_tokens,
            token_count_within_request: self.token_metrics.token_count <= request.max_tokens,
            token_count_within_planning,
            imported_kv_blocks: self.imported_kv_blocks,
            request_imported_kv_blocks: request.imported_kv_blocks,
            planned_imported_kv_blocks: planned_kv.map(|planned| planned.import_blocks),
            imported_kv_matches_request: self.imported_kv_blocks == request.imported_kv_blocks,
            imported_kv_within_planning,
            exported_kv_blocks: self.exported_kv_blocks,
            runtime_export_enabled: request.runtime.supports_kv_export,
            runtime_max_export_blocks: request.runtime.max_kv_export_blocks,
            planned_exported_kv_blocks: planned_kv.map(|planned| planned.export_blocks),
            exported_kv_within_runtime: if request.runtime.supports_kv_export {
                request.runtime.max_kv_export_blocks == 0
                    || self.exported_kv_blocks <= request.runtime.max_kv_export_blocks
            } else {
                self.exported_kv_blocks == 0
            },
            exported_kv_within_planning,
            request_selected_adapter: request.selected_adapter,
            runtime_selected_adapter,
            runtime_adapter_reported,
            selected_adapter_matches_request,
            generation_budget_reported,
            generation_budget_matches_request,
            route_budget_matches_request: outcome.diagnostics.route_budget == request.route_budget,
            hardware_pressure_matches_request: float_close(
                outcome.diagnostics.hardware_pressure,
                request.hardware_pressure,
            ),
            compute_headroom_matches_planning,
            latency_budget_matches_planning,
            planning_pre_request_problem_count: planning
                .map(|planning| {
                    planning
                        .planning_summary()
                        .pre_request_gate_problem_component_count()
                })
                .unwrap_or(0),
            planning_pressure_signal_count: planning
                .map(|planning| {
                    planning
                        .planning_summary()
                        .planning_pressure_signal_component_count()
                })
                .unwrap_or(0),
        }
    }

    pub fn validate_exported_kv_blocks(
        &self,
        outcome: &InferenceOutcome,
        request: &RuntimeRequestEnvelope,
    ) -> RuntimeKvValidationReport {
        let mut report = RuntimeKvBlockContract::for_request_exports(request).validate_blocks(
            &outcome.exported_kv,
            &request.runtime,
            request.architecture,
        );

        if self.exported_kv_blocks != outcome.exported_kv.len() {
            report.violations.push(format!(
                "runtime response exported KV envelope count {} differs from outcome block count {}",
                self.exported_kv_blocks,
                outcome.exported_kv.len()
            ));
        }

        report
    }

    pub fn acceptance_report(
        &self,
        outcome: &InferenceOutcome,
        request: &RuntimeRequestEnvelope,
        hardware: &HardwarePlan,
    ) -> RuntimeResponseAcceptanceReport {
        RuntimeResponseAcceptanceReport {
            response_violations: self.contract_violations(
                outcome,
                &request.runtime,
                request.architecture,
                hardware,
            ),
            request_violations: self.request_contract_violations(outcome, request),
            exported_kv_report: self.validate_exported_kv_blocks(outcome, request),
        }
    }

    pub fn response_gate_summary(
        &self,
        outcome: &InferenceOutcome,
        request: &RuntimeRequestEnvelope,
        hardware: &HardwarePlan,
    ) -> RuntimeResponseGateSummary {
        let acceptance = self
            .acceptance_report(outcome, request, hardware)
            .acceptance_summary();
        let envelope = self.envelope_summary();
        let request_parity = self.request_parity_summary(outcome, request);
        let envelope_consistent = envelope.has_answer() && envelope.kv_counts_match_diagnostics();
        let exported_kv_accepted = acceptance.exported_kv_violation_count == 0;

        RuntimeResponseGateSummary {
            response_accepted: acceptance.accepted,
            envelope_consistent,
            request_parity_consistent: request_parity.request_parity_is_consistent(),
            exported_kv_accepted,
            accepted_exported_kv_blocks: acceptance.accepted_exported_kv_blocks,
            response_wire_problem_count: request_parity.response_wire_problem_component_count(),
            planning_pre_request_problem_count: request_parity
                .planning_pre_request_gate_problem_component_count(),
            planning_pressure_signal_count: request_parity.planning_pressure_signal_count,
            response_violation_count: acceptance.response_violation_count,
            request_violation_count: acceptance.request_violation_count,
            exported_kv_violation_count: acceptance.exported_kv_violation_count,
            failure_report_count: acceptance.failure_report_count,
        }
    }

    pub fn envelope_summary(&self) -> RuntimeResponseEnvelopeSummary {
        RuntimeResponseEnvelopeSummary {
            schema: self.schema,
            answer_chars: self.answer_chars,
            token_count: self.token_metrics.token_count,
            entropy_count: self.token_metrics.entropy_count,
            logprob_count: self.token_metrics.logprob_count,
            has_uncertainty_signal: self.token_metrics.has_uncertainty_signal(),
            token_uncertainty_coverage_signal_count: self
                .token_metrics
                .uncertainty_coverage_signal_component_count(),
            token_uncertainty_metric_problem_count: self
                .token_metrics
                .uncertainty_metric_problem_component_count(),
            token_uncertainty_accounting_consistent: self
                .token_metrics
                .uncertainty_accounting_is_consistent(),
            imported_kv_blocks: self.imported_kv_blocks,
            exported_kv_blocks: self.exported_kv_blocks,
            diagnostics_imported_kv_blocks: self.diagnostics_imported_kv_blocks,
            diagnostics_exported_kv_blocks: self.diagnostics_exported_kv_blocks,
            diagnostics_weak_runtime_kv_imports_skipped: self
                .diagnostics_weak_runtime_kv_imports_skipped,
            has_runtime_execution_signal: self.has_runtime_execution_signal,
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "schema={} answer_chars={} tokens={} imported_kv={} exported_kv={} weak_runtime_kv_imports_skipped={} runtime_signal={}",
            self.schema,
            self.answer_chars,
            self.token_metrics.token_count,
            self.imported_kv_blocks,
            self.exported_kv_blocks,
            self.diagnostics_weak_runtime_kv_imports_skipped,
            self.has_runtime_execution_signal
        )
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::RuntimeAdapter;
    use crate::diagnostics::{DeviceExecutionSource, InferenceDiagnostics, RuntimeDiagnostics};
    use crate::engine::{GeneratedToken, InferenceOutcome, InferenceRequest, RuntimeFailureKind};
    use crate::hardware::{DeviceClass, HardwareAllocator, HardwareLoadSnapshot};
    use crate::kv::{KvBlock, KvNamespace, RuntimeKvImportManifestPlanSummary};
    use crate::profile::{HierarchyWeights, TaskProfile};
    use crate::request::RuntimeRequestEnvelope;
    use crate::router::{
        DefaultHierarchicalRouter, HierarchicalRouter, RouteBudget, RoutingContext, TokenFeatures,
    };
    use crate::transformer::{
        RuntimeKvExportManifestPlanSummary, TransformerAttentionKind, TransformerLayerBudget,
        TransformerPlanDigest,
    };

    #[test]
    fn response_envelope_summarizes_valid_outcome() {
        let metadata = RuntimeMetadata::new("model", "tok", 128, 16);
        let architecture = TransformerRuntimeArchitecture::new(2, 16, 2, 2, 64);
        let hardware = HardwareAllocator::new().plan(
            HardwareLoadSnapshot::new(DeviceClass::DiscreteGpu, 0.1, 0.1, 0.1, 0.1),
            TaskProfile::Coding,
            512,
            HierarchyWeights::for_profile(TaskProfile::Coding),
        );
        let runtime = RuntimeDiagnostics::empty()
            .with_model_id("model")
            .with_selected_adapter(RuntimeAdapter::Cuda)
            .with_architecture(2, 16, 64)
            .with_device_execution(
                "gpu",
                "gpu",
                "cpu",
                "gpu-resident",
                DeviceExecutionSource::RuntimeReported,
            )
            .with_kv_exchange(1, 1);
        let mut outcome = InferenceOutcome::empty().with_diagnostics(
            InferenceDiagnostics::new(RouteBudget::default()).with_runtime(runtime),
        );
        outcome.answer = "ok".to_owned();
        outcome.tokens.push(GeneratedToken::new("ok"));
        outcome.imported_kv.push(runtime_block(1));
        outcome.exported_kv.push(runtime_block(2));

        let envelope = RuntimeResponseEnvelope::from_outcome(&outcome);

        assert!(envelope.is_valid(&outcome, &metadata, architecture, &hardware));
        assert_eq!(envelope.schema, RUNTIME_RESPONSE_SCHEMA);
        assert_eq!(envelope.answer_chars, 2);
        assert_eq!(envelope.token_metrics.token_count, 1);
        assert!(envelope.has_runtime_execution_signal);
        assert!(
            envelope
                .summary()
                .contains("schema=rust-norion-runtime-response-v1")
        );
    }

    #[test]
    fn response_envelope_summary_reports_uncertainty_and_kv_parity() {
        let runtime = RuntimeDiagnostics::empty()
            .with_selected_adapter(RuntimeAdapter::Cuda)
            .with_kv_exchange(1, 1);
        let mut outcome = InferenceOutcome::empty().with_diagnostics(
            InferenceDiagnostics::new(RouteBudget::default()).with_runtime(runtime),
        );
        outcome.answer = "ok".to_owned();
        outcome.tokens.push(GeneratedToken {
            text: "ok".to_owned(),
            logprob: Some(-0.5),
            entropy: Some(0.25),
        });
        outcome.imported_kv.push(runtime_block(1));
        outcome.exported_kv.push(runtime_block(2));

        let summary = RuntimeResponseEnvelope::from_outcome(&outcome).envelope_summary();

        assert_eq!(summary.schema, RUNTIME_RESPONSE_SCHEMA);
        assert_eq!(summary.answer_chars, 2);
        assert!(summary.has_answer());
        assert_eq!(summary.token_count, 1);
        assert!(summary.has_generated_tokens());
        assert_eq!(summary.entropy_count, 1);
        assert_eq!(summary.logprob_count, 1);
        assert!(summary.has_token_uncertainty());
        assert_eq!(
            summary.token_uncertainty_coverage_signal_component_count(),
            0
        );
        assert!(!summary.has_token_uncertainty_coverage_signals());
        assert_eq!(
            summary.token_uncertainty_metric_problem_component_count(),
            0
        );
        assert!(!summary.has_token_uncertainty_metric_problem_components());
        assert!(summary.token_uncertainty_accounting_is_consistent());
        assert!(summary.schema_matches_runtime_response());
        assert!(summary.has_kv_exchange());
        assert!(summary.has_runtime_kv_activity());
        assert_eq!(summary.envelope_kv_exchange_total(), 2);
        assert_eq!(summary.diagnostics_kv_exchange_total(), 2);
        assert_eq!(summary.diagnostics_kv_activity_total(), 2);
        assert!(summary.kv_counts_match_diagnostics());
        assert!(summary.has_runtime_execution_signal);
        assert_eq!(
            summary.runtime_response_envelope_commit_signal_component_count(),
            5
        );
        assert!(summary.has_runtime_response_envelope_commit_signals());
        assert_eq!(
            summary.runtime_response_envelope_commit_blocker_component_count(),
            0
        );
        assert!(!summary.has_runtime_response_envelope_commit_blockers());
        assert!(summary.runtime_response_envelope_commit_accounting_is_consistent());
        assert!(summary.runtime_response_envelope_commit_is_clean());
        assert!(summary.response_envelope_shape_is_clean());
        assert!(summary.can_commit_runtime_response_envelope());
        assert!(summary.can_use_runtime_response_envelope());
    }

    #[test]
    fn response_envelope_summary_reports_diagnostics_kv_drift() {
        let runtime = RuntimeDiagnostics::empty().with_kv_exchange(2, 0);
        let mut outcome = InferenceOutcome::empty().with_diagnostics(
            InferenceDiagnostics::new(RouteBudget::default()).with_runtime(runtime),
        );
        outcome.answer = "ok".to_owned();
        outcome.tokens.push(GeneratedToken::new("ok"));
        outcome.imported_kv.push(runtime_block(1));
        outcome.exported_kv.push(runtime_block(2));

        let summary = RuntimeResponseEnvelope::from_outcome(&outcome).envelope_summary();

        assert_eq!(summary.imported_kv_blocks, 1);
        assert_eq!(summary.exported_kv_blocks, 1);
        assert_eq!(summary.diagnostics_imported_kv_blocks, 2);
        assert_eq!(summary.diagnostics_exported_kv_blocks, 0);
        assert_eq!(summary.diagnostics_weak_runtime_kv_imports_skipped, 0);
        assert_eq!(summary.envelope_kv_exchange_total(), 2);
        assert_eq!(summary.diagnostics_kv_exchange_total(), 2);
        assert_eq!(summary.diagnostics_kv_activity_total(), 2);
        assert!(!summary.kv_counts_match_diagnostics());
        assert!(!summary.has_token_uncertainty());
        assert_eq!(
            summary.token_uncertainty_coverage_signal_component_count(),
            2
        );
        assert!(summary.has_token_uncertainty_coverage_signals());
        assert_eq!(
            summary.token_uncertainty_metric_problem_component_count(),
            0
        );
        assert!(summary.token_uncertainty_accounting_is_consistent());
        assert!(summary.schema_matches_runtime_response());
        assert_eq!(
            summary.runtime_response_envelope_commit_signal_component_count(),
            6
        );
        assert_eq!(
            summary.runtime_response_envelope_commit_blocker_component_count(),
            1
        );
        assert!(summary.has_runtime_response_envelope_commit_blockers());
        assert!(summary.runtime_response_envelope_commit_accounting_is_consistent());
        assert!(!summary.runtime_response_envelope_commit_is_clean());
        assert!(!summary.response_envelope_shape_is_clean());
        assert!(!summary.can_commit_runtime_response_envelope());
        assert!(!summary.can_use_runtime_response_envelope());
    }

    #[test]
    fn response_envelope_summary_counts_weak_skip_as_activity_not_exchange() {
        let runtime = RuntimeDiagnostics::empty().with_weak_runtime_kv_imports_skipped(2);
        let mut outcome = InferenceOutcome::empty().with_diagnostics(
            InferenceDiagnostics::new(RouteBudget::default()).with_runtime(runtime),
        );
        outcome.answer = "ok".to_owned();
        outcome.tokens.push(GeneratedToken::new("ok"));

        let envelope = RuntimeResponseEnvelope::from_outcome(&outcome);
        let summary = envelope.envelope_summary();

        assert_eq!(envelope.imported_kv_blocks, 0);
        assert_eq!(envelope.exported_kv_blocks, 0);
        assert_eq!(envelope.diagnostics_imported_kv_blocks, 0);
        assert_eq!(envelope.diagnostics_exported_kv_blocks, 0);
        assert_eq!(envelope.diagnostics_weak_runtime_kv_imports_skipped, 2);
        assert!(
            envelope
                .summary()
                .contains("weak_runtime_kv_imports_skipped=2")
        );
        assert_eq!(summary.diagnostics_weak_runtime_kv_imports_skipped, 2);
        assert!(!summary.has_kv_exchange());
        assert!(summary.has_runtime_kv_activity());
        assert_eq!(summary.envelope_kv_exchange_total(), 0);
        assert_eq!(summary.diagnostics_kv_exchange_total(), 0);
        assert_eq!(summary.diagnostics_kv_activity_total(), 2);
        assert!(summary.kv_counts_match_diagnostics());
        assert!(summary.has_runtime_execution_signal);
        assert_eq!(
            summary.runtime_response_envelope_commit_signal_component_count(),
            6
        );
        assert_eq!(
            summary.runtime_response_envelope_commit_blocker_component_count(),
            0
        );
        assert!(summary.runtime_response_envelope_commit_accounting_is_consistent());
        assert!(summary.runtime_response_envelope_commit_is_clean());
        assert!(summary.response_envelope_shape_is_clean());
        assert!(summary.can_commit_runtime_response_envelope());
        assert!(summary.can_use_runtime_response_envelope());
    }

    #[test]
    fn response_envelope_summary_reports_uncertainty_accounting_drift() {
        let summary = RuntimeResponseEnvelopeSummary {
            schema: RUNTIME_RESPONSE_SCHEMA,
            answer_chars: 2,
            token_count: 1,
            entropy_count: 2,
            logprob_count: 0,
            has_uncertainty_signal: true,
            token_uncertainty_coverage_signal_count: 2,
            token_uncertainty_metric_problem_count: 4,
            token_uncertainty_accounting_consistent: true,
            imported_kv_blocks: 0,
            exported_kv_blocks: 0,
            diagnostics_imported_kv_blocks: 0,
            diagnostics_exported_kv_blocks: 0,
            diagnostics_weak_runtime_kv_imports_skipped: 0,
            has_runtime_execution_signal: true,
        };

        assert!(summary.has_token_uncertainty());
        assert_eq!(
            summary.token_uncertainty_coverage_signal_component_count(),
            2
        );
        assert!(summary.has_token_uncertainty_coverage_signals());
        assert_eq!(
            summary.token_uncertainty_metric_problem_component_count(),
            4
        );
        assert!(summary.has_token_uncertainty_metric_problem_components());
        assert!(!summary.token_uncertainty_accounting_is_consistent());
        assert!(summary.schema_matches_runtime_response());
        assert_eq!(
            summary.runtime_response_envelope_commit_signal_component_count(),
            6
        );
        assert_eq!(
            summary.runtime_response_envelope_commit_blocker_component_count(),
            1
        );
        assert!(summary.runtime_response_envelope_commit_accounting_is_consistent());
        assert!(!summary.runtime_response_envelope_commit_is_clean());
        assert!(!summary.response_envelope_shape_is_clean());
        assert!(!summary.can_commit_runtime_response_envelope());
        assert!(!summary.can_use_runtime_response_envelope());
    }

    #[test]
    fn response_envelope_reports_answer_token_and_kv_count_violations() {
        let metadata = RuntimeMetadata::new("model", "tok", 128, 16);
        let architecture = TransformerRuntimeArchitecture::new(1, 16, 1, 1, 64);
        let hardware = HardwareAllocator::new().plan(
            HardwareLoadSnapshot::new(DeviceClass::CpuOnly, 0.1, 0.1, 0.1, 0.1),
            TaskProfile::General,
            512,
            HierarchyWeights::default(),
        );
        let runtime = RuntimeDiagnostics::empty().with_kv_exchange(2, 3);
        let mut outcome = InferenceOutcome::empty().with_diagnostics(
            InferenceDiagnostics::new(RouteBudget::default()).with_runtime(runtime),
        );
        outcome.answer = "   ".to_owned();
        outcome.tokens.push(GeneratedToken::new(" "));
        outcome.imported_kv.push(runtime_block(1));
        outcome.exported_kv.push(runtime_block(2));

        let envelope = RuntimeResponseEnvelope::from_outcome(&outcome);
        let joined = envelope
            .contract_violations(&outcome, &metadata, architecture, &hardware)
            .join("\n");

        assert!(joined.contains("answer must be non-empty"));
        assert!(joined.contains("contains 1 empty generated tokens"));
        assert!(joined.contains("imported KV count 1 differs from diagnostics 2"));
        assert!(joined.contains("exported KV count 1 differs from diagnostics 3"));
    }

    #[test]
    fn response_envelope_reuses_runtime_diagnostics_contract_checks() {
        let metadata = RuntimeMetadata::new("model", "tok", 128, 16).with_kv_precision(4, 4);
        let architecture = TransformerRuntimeArchitecture::new(2, 16, 2, 2, 64);
        let hardware = HardwareAllocator::new().plan(
            HardwareLoadSnapshot::new(DeviceClass::CpuOnly, 0.1, 0.1, 0.1, 0.1),
            TaskProfile::General,
            512,
            HierarchyWeights::default(),
        );
        let runtime = RuntimeDiagnostics::empty()
            .with_model_id("other")
            .with_selected_adapter(RuntimeAdapter::Cuda)
            .with_architecture(3, 32, 64)
            .with_kv_precision(8, 4);
        let mut outcome = InferenceOutcome::empty().with_diagnostics(
            InferenceDiagnostics::new(RouteBudget::default()).with_runtime(runtime),
        );
        outcome.answer = "answer".to_owned();

        let envelope = RuntimeResponseEnvelope::from_outcome(&outcome);
        let joined = envelope
            .contract_violations(&outcome, &metadata, architecture, &hardware)
            .join("\n");

        assert!(joined.contains("model_id"));
        assert!(joined.contains("selected adapter"));
        assert!(joined.contains("layer_count"));
        assert!(joined.contains("hidden_size"));
        assert!(joined.contains("hot KV precision"));
    }

    #[test]
    fn response_envelope_matches_request_contract() {
        let metadata = RuntimeMetadata::new("model", "tok", 128, 16)
            .with_kv_exchange(true, true)
            .with_kv_limits(2, 2);
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(16)
            .with_max_tokens(2)
            .with_runtime(metadata);
        let runtime_request = request_envelope(&request, RuntimeAdapter::Cuda, 1);
        let runtime = RuntimeDiagnostics::empty()
            .with_selected_adapter(RuntimeAdapter::Cuda)
            .with_kv_exchange(1, 1);
        let mut outcome = InferenceOutcome::empty().with_diagnostics(
            InferenceDiagnostics::new(RouteBudget::default())
                .with_runtime(runtime)
                .with_generation_budget(runtime_request.generation_budget),
        );
        outcome.answer = "ok".to_owned();
        outcome.tokens.push(GeneratedToken::new("o"));
        outcome.tokens.push(GeneratedToken::new("k"));
        outcome.imported_kv.push(runtime_block(1));
        outcome.exported_kv.push(runtime_block(2));

        let envelope = RuntimeResponseEnvelope::from_outcome(&outcome);
        let summary = envelope.request_parity_summary(&outcome, &runtime_request);

        assert!(
            envelope
                .request_contract_violations(&outcome, &runtime_request)
                .is_empty()
        );
        assert_eq!(summary.token_count, 2);
        assert_eq!(summary.request_max_tokens, 2);
        assert!(summary.token_count_within_request);
        assert!(summary.imported_kv_matches_request);
        assert!(summary.exported_kv_within_runtime);
        assert!(summary.selected_adapter_matches_request);
        assert!(summary.generation_budget_reported);
        assert!(summary.generation_budget_matches_request);
        assert!(summary.route_budget_matches_request);
        assert!(summary.hardware_pressure_matches_request);
        assert!(!summary.has_planning_digest());
        assert_eq!(summary.token_drift_component_count(), 0);
        assert_eq!(summary.kv_drift_component_count(), 0);
        assert_eq!(summary.adapter_drift_component_count(), 0);
        assert_eq!(summary.diagnostics_drift_component_count(), 0);
        assert_eq!(summary.request_drift_component_count(), 0);
        assert!(!summary.planning_has_pre_request_gate_problems());
        assert!(!summary.planning_has_pressure_signals());
        let planned_kv = summary.planned_kv_summary();
        assert!(!planned_kv.has_planning_digest);
        assert_eq!(planned_kv.imported_kv_blocks, 1);
        assert_eq!(planned_kv.exported_kv_blocks, 1);
        assert_eq!(planned_kv.planned_imported_kv_blocks, None);
        assert_eq!(planned_kv.planned_exported_kv_blocks, None);
        assert!(planned_kv.has_response_kv_activity());
        assert!(!planned_kv.has_planned_kv_activity());
        assert!(!planned_kv.planning_limits_reported());
        assert!(planned_kv.response_kv_within_planning());
        assert!(!planned_kv.response_kv_exceeds_planning());
        assert_eq!(planned_kv.response_kv_activity_signal_component_count(), 2);
        assert_eq!(planned_kv.planned_kv_activity_signal_component_count(), 0);
        assert_eq!(planned_kv.response_planned_kv_signal_component_count(), 2);
        assert!(planned_kv.has_response_planned_kv_signals());
        assert_eq!(planned_kv.planning_limit_missing_component_count(), 0);
        assert_eq!(planned_kv.planning_kv_exceeded_component_count(), 0);
        assert_eq!(planned_kv.response_planned_kv_problem_component_count(), 0);
        assert!(!planned_kv.has_response_planned_kv_problem_components());
        assert!(planned_kv.response_planned_kv_accounting_is_consistent());
        assert!(planned_kv.response_planned_kv_shape_is_clean());
        assert!(planned_kv.can_use_response_planned_kv());
        assert!(!planned_kv.can_commit_planned_kv_response());
        assert_eq!(
            summary.planning_pre_request_gate_problem_component_count(),
            0
        );
        assert_eq!(summary.planning_pressure_signal_component_count(), 0);
        assert_eq!(summary.response_wire_problem_component_count(), 0);
        assert!(!summary.has_response_wire_problem_components());
        assert!(summary.response_wire_accounting_is_consistent());
        assert!(summary.response_wire_shape_is_clean());
        assert!(summary.can_use_response_wire());
        assert!(summary.token_parity_ok());
        assert!(summary.kv_parity_ok());
        assert!(summary.adapter_parity_ok());
        assert!(summary.diagnostics_parity_ok());
        assert!(summary.request_parity_is_consistent());
    }

    #[test]
    fn response_request_parity_blocks_stale_router_budget_from_runtime_diagnostics() {
        let metadata = RuntimeMetadata::new("model", "tok", 128, 16);
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(16)
            .with_max_tokens(2)
            .with_runtime(metadata);
        let router = DefaultHierarchicalRouter::new();
        let token = TokenFeatures::new("response-route", 0.80, 0);
        let routing_context = RoutingContext {
            profile: TaskProfile::Coding,
            hierarchy: HierarchyWeights::for_profile(TaskProfile::Coding),
            ..RoutingContext::default()
        };
        let request_route_budget = router.budget(std::slice::from_ref(&token), routing_context);
        let runtime_request = request_envelope_with_route_budget(
            &request,
            RuntimeAdapter::Cuda,
            0,
            request_route_budget,
        );
        let hardware = HardwareAllocator::new().plan(
            HardwareLoadSnapshot::new(DeviceClass::DiscreteGpu, 0.1, 0.1, 0.1, 0.1),
            TaskProfile::Coding,
            512,
            HierarchyWeights::for_profile(TaskProfile::Coding),
        );
        let runtime = RuntimeDiagnostics::empty()
            .with_model_id("model")
            .with_selected_adapter(RuntimeAdapter::Cuda)
            .with_architecture(1, 16, 64)
            .with_device_execution(
                "gpu",
                "gpu",
                "cpu",
                "gpu-resident",
                DeviceExecutionSource::RuntimeReported,
            );
        let mut outcome = InferenceOutcome::empty().with_diagnostics(
            InferenceDiagnostics::new(RouteBudget::default())
                .with_runtime(runtime)
                .with_generation_budget(runtime_request.generation_budget),
        );
        outcome.answer = "ok".to_owned();
        outcome.tokens.push(GeneratedToken::new("ok"));

        let envelope = RuntimeResponseEnvelope::from_outcome(&outcome);
        let summary = envelope.request_parity_summary(&outcome, &runtime_request);
        let gate = envelope.response_gate_summary(&outcome, &runtime_request, &hardware);
        let joined = envelope
            .request_contract_violations(&outcome, &runtime_request)
            .join("\n");

        assert!(request_route_budget.can_use_route_budget());
        assert_ne!(request_route_budget, RouteBudget::default());
        assert!(summary.token_parity_ok());
        assert!(summary.kv_parity_ok());
        assert!(summary.adapter_parity_ok());
        assert!(!summary.route_budget_matches_request);
        assert!(summary.route_budget_drifted());
        assert!(!summary.generation_budget_drifted());
        assert!(!summary.hardware_pressure_drifted());
        assert_eq!(summary.request_diagnostics_drift_component_count(), 1);
        assert_eq!(summary.response_wire_problem_component_count(), 1);
        assert!(summary.has_response_wire_problem_components());
        assert!(summary.response_wire_accounting_is_consistent());
        assert!(!summary.response_wire_shape_is_clean());
        assert!(!summary.can_use_response_wire());
        assert!(!summary.request_parity_is_consistent());
        assert!(!gate.response_accepted);
        assert!(gate.envelope_consistent);
        assert!(!gate.request_parity_consistent);
        assert_eq!(gate.response_violation_count, 0);
        assert_eq!(gate.request_violation_count, 1);
        assert_eq!(gate.exported_kv_violation_count, 0);
        assert_eq!(gate.response_wire_problem_count, 1);
        assert!(gate.has_request_parity_failures());
        assert!(!gate.can_commit_runtime_response());
        assert!(joined.contains("runtime response route budget differs from request route budget"));
    }

    #[test]
    fn response_request_parity_blocks_stale_low_pressure_route_after_hardware_demote() {
        let metadata = RuntimeMetadata::new("model", "tok", 128, 16);
        let request = InferenceRequest::new("prompt", TaskProfile::General)
            .with_prompt_tokens(16)
            .with_max_tokens(2)
            .with_runtime(metadata);
        let router = DefaultHierarchicalRouter::new();
        let token = TokenFeatures::new("response-hardware-borderline", 0.80, 0);
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
        let architecture = TransformerRuntimeArchitecture::new(1, 16, 2, 1, 64);
        let transformer_plan = TransformerPlanDigest::new(
            Some("response-hardware-demote"),
            vec![TransformerLayerBudget::new(
                0,
                TransformerAttentionKind::Global,
                0.5,
                64,
            )],
        );
        let execution = crate::adapter::AdapterExecutionContext::new([RuntimeAdapter::Cuda])
            .with_pressure(1.0, 0.20);
        let runtime_request = RuntimeRequestEnvelope::from_parts(
            &request,
            architecture,
            high_pressure_budget,
            HierarchyWeights::for_profile(TaskProfile::General),
            &transformer_plan,
            &execution,
            0,
        );
        let hardware = HardwareAllocator::new().plan(
            HardwareLoadSnapshot::new(DeviceClass::DiscreteGpu, 0.1, 0.1, 0.1, 0.1),
            TaskProfile::General,
            512,
            HierarchyWeights::for_profile(TaskProfile::General),
        );
        let runtime = RuntimeDiagnostics::empty()
            .with_model_id("model")
            .with_selected_adapter(RuntimeAdapter::Cuda)
            .with_architecture(1, 16, 64)
            .with_device_execution(
                "gpu",
                "gpu",
                "cpu",
                "gpu-resident",
                DeviceExecutionSource::RuntimeReported,
            );
        let mut outcome = InferenceOutcome::empty().with_diagnostics(
            InferenceDiagnostics::new(low_pressure_budget)
                .with_runtime(runtime)
                .with_generation_budget(runtime_request.generation_budget)
                .with_hardware(0.0, 0.20, None),
        );
        outcome.answer = "ok".to_owned();
        outcome.tokens.push(GeneratedToken::new("ok"));

        let envelope = RuntimeResponseEnvelope::from_outcome(&outcome);
        let summary = envelope.request_parity_summary(&outcome, &runtime_request);
        let gate = envelope.response_gate_summary(&outcome, &runtime_request, &hardware);
        let joined = envelope
            .request_contract_violations(&outcome, &runtime_request)
            .join("\n");

        assert_eq!(
            low_pressure_decision.layer,
            crate::router::RouteLayer::LocalWindow
        );
        assert_eq!(
            high_pressure_decision.layer,
            crate::router::RouteLayer::FastProjection
        );
        assert!(low_pressure_budget.has_attention_pressure());
        assert_eq!(low_pressure_budget.attention_tokens, 1);
        assert_eq!(low_pressure_budget.fast_tokens, 0);
        assert!(!high_pressure_budget.has_attention_pressure());
        assert_eq!(high_pressure_budget.attention_tokens, 0);
        assert_eq!(high_pressure_budget.fast_tokens, 1);
        assert_eq!(runtime_request.route_budget, high_pressure_budget);
        assert_eq!(runtime_request.hardware_pressure, 1.0);
        assert!(summary.token_parity_ok());
        assert!(summary.kv_parity_ok());
        assert!(summary.adapter_parity_ok());
        assert!(!summary.route_budget_matches_request);
        assert!(!summary.hardware_pressure_matches_request);
        assert!(summary.route_budget_drifted());
        assert!(summary.hardware_pressure_drifted());
        assert!(!summary.generation_budget_drifted());
        assert_eq!(summary.request_diagnostics_drift_component_count(), 2);
        assert_eq!(summary.response_wire_problem_component_count(), 2);
        assert!(summary.has_response_wire_problem_components());
        assert!(summary.response_wire_accounting_is_consistent());
        assert!(!summary.response_wire_shape_is_clean());
        assert!(!summary.can_use_response_wire());
        assert!(!summary.request_parity_is_consistent());
        assert!(!gate.response_accepted);
        assert!(gate.envelope_consistent);
        assert!(!gate.request_parity_consistent);
        assert_eq!(gate.response_violation_count, 0);
        assert_eq!(gate.request_violation_count, 2);
        assert_eq!(gate.exported_kv_violation_count, 0);
        assert_eq!(gate.response_wire_problem_count, 2);
        assert!(gate.has_request_parity_failures());
        assert!(!gate.can_commit_runtime_response());
        assert!(joined.contains("runtime response route budget differs from request route budget"));
        assert!(joined.contains(
            "runtime response hardware pressure 0.000 differs from request hardware pressure 1.000"
        ));
    }

    #[test]
    fn response_gate_summary_accepts_consistent_runtime_response() {
        let metadata = RuntimeMetadata::new("model", "tok", 128, 16)
            .with_kv_exchange(true, true)
            .with_kv_limits(2, 2);
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(16)
            .with_max_tokens(2)
            .with_runtime(metadata);
        let runtime_request = request_envelope(&request, RuntimeAdapter::Cuda, 1);
        let hardware = HardwareAllocator::new().plan(
            HardwareLoadSnapshot::new(DeviceClass::DiscreteGpu, 0.1, 0.1, 0.1, 0.1),
            TaskProfile::Coding,
            512,
            HierarchyWeights::for_profile(TaskProfile::Coding),
        );
        let runtime = RuntimeDiagnostics::empty()
            .with_model_id("model")
            .with_selected_adapter(RuntimeAdapter::Cuda)
            .with_architecture(1, 16, 64)
            .with_device_execution(
                "gpu",
                "gpu",
                "cpu",
                "gpu-resident",
                DeviceExecutionSource::RuntimeReported,
            )
            .with_kv_exchange(1, 1);
        let mut outcome = InferenceOutcome::empty().with_diagnostics(
            InferenceDiagnostics::new(RouteBudget::default())
                .with_runtime(runtime)
                .with_generation_budget(runtime_request.generation_budget),
        );
        outcome.answer = "ok".to_owned();
        outcome.tokens.push(GeneratedToken::new("o"));
        outcome.tokens.push(GeneratedToken::new("k"));
        outcome.imported_kv.push(runtime_block(1));
        outcome.exported_kv.push(runtime_block(2));

        let envelope = RuntimeResponseEnvelope::from_outcome(&outcome);
        let gate = envelope.response_gate_summary(&outcome, &runtime_request, &hardware);

        assert!(gate.response_accepted);
        assert!(gate.envelope_consistent);
        assert!(gate.request_parity_consistent);
        assert!(gate.exported_kv_accepted);
        assert_eq!(gate.accepted_exported_kv_blocks, 1);
        assert_eq!(gate.response_wire_problem_count, 0);
        assert_eq!(gate.planning_pre_request_problem_count, 0);
        assert_eq!(gate.planning_pressure_signal_count, 0);
        assert_eq!(gate.response_violation_count, 0);
        assert_eq!(gate.request_violation_count, 0);
        assert_eq!(gate.exported_kv_violation_count, 0);
        assert_eq!(gate.failure_report_count, 0);
        assert_eq!(gate.response_wire_problem_component_count(), 0);
        assert_eq!(gate.direct_response_wire_problem_component_count(), 0);
        assert_eq!(gate.planning_pre_request_gate_problem_component_count(), 0);
        assert_eq!(gate.planning_pressure_signal_component_count(), 0);
        assert_eq!(gate.exported_kv_activity_signal_component_count(), 1);
        assert_eq!(gate.response_gate_signal_component_count(), 1);
        assert!(gate.response_gate_has_signal_components());
        assert!(!gate.has_response_wire_problem_components());
        assert!(!gate.has_planning_pre_request_gate_problems());
        assert!(!gate.has_planning_pressure_signals());
        assert!(gate.response_wire_accounting_is_consistent());
        assert!(!gate.has_acceptance_failures());
        assert!(!gate.has_response_contract_failures());
        assert!(!gate.has_request_parity_failures());
        assert!(!gate.has_exported_kv_failures());
        assert!(!gate.envelope_drifted());
        assert!(!gate.request_parity_drifted());
        assert!(!gate.exported_kv_drifted());
        assert!(!gate.has_boundary_drift());
        assert!(!gate.has_failure_reports());
        assert!(!gate.has_total_violations());
        assert_eq!(gate.response_contract_failure_component_count(), 0);
        assert_eq!(gate.request_parity_failure_component_count(), 0);
        assert_eq!(gate.exported_kv_failure_component_count(), 0);
        assert_eq!(gate.acceptance_failure_component_count(), 0);
        assert_eq!(gate.envelope_blocker_component_count(), 0);
        assert_eq!(gate.request_parity_blocker_component_count(), 0);
        assert_eq!(gate.exported_kv_blocker_component_count(), 0);
        assert_eq!(gate.boundary_drift_component_count(), 0);
        assert_eq!(gate.mapped_failure_report_component_count(), 0);
        assert_eq!(gate.response_blocker_component_count(), 0);
        assert!(!gate.response_gate_has_problem_components());
        assert!(gate.response_gate_accounting_is_consistent());
        assert!(gate.failure_report_matches_failures());
        assert!(gate.can_accept_response());
        assert!(gate.is_clean_response_gate());
        assert!(gate.response_gate_shape_is_clean());
        assert_eq!(gate.runtime_response_commit_signal_component_count(), 1);
        assert!(gate.has_runtime_response_commit_signals());
        assert_eq!(gate.runtime_response_commit_blocker_component_count(), 0);
        assert!(!gate.has_runtime_response_commit_blockers());
        assert!(gate.runtime_response_commit_accounting_is_consistent());
        assert!(gate.runtime_response_commit_is_clean());
        assert!(gate.can_commit_runtime_response());
        assert!(gate.can_accept_runtime_response());
    }

    #[test]
    fn response_envelope_reports_request_contract_mismatches() {
        let metadata = RuntimeMetadata::new("model", "tok", 128, 16)
            .with_kv_exchange(true, true)
            .with_kv_limits(1, 1);
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(16)
            .with_max_tokens(1)
            .with_runtime(metadata);
        let runtime_request = request_envelope(&request, RuntimeAdapter::Cuda, 1);
        let runtime = RuntimeDiagnostics::empty()
            .with_selected_adapter(RuntimeAdapter::CpuSimd)
            .with_kv_exchange(2, 2);
        let mut outcome = InferenceOutcome::empty().with_diagnostics(
            InferenceDiagnostics::new(RouteBudget {
                threshold: 0.22,
                attention_tokens: 1,
                fast_tokens: 9,
                attention_fraction: 0.10,
            })
            .with_runtime(runtime)
            .with_generation_budget(
                RuntimeMetadata::new("other", "tok", 64, 16).generation_budget(8, 1),
            )
            .with_hardware(0.90, 0.10, Some(5)),
        );
        outcome.answer = "too many".to_owned();
        outcome.tokens.push(GeneratedToken::new("too"));
        outcome.tokens.push(GeneratedToken::new("many"));
        outcome.imported_kv.push(runtime_block(1));
        outcome.imported_kv.push(runtime_block(2));
        outcome.exported_kv.push(runtime_block(3));
        outcome.exported_kv.push(runtime_block(4));

        let envelope = RuntimeResponseEnvelope::from_outcome(&outcome);
        let joined = envelope
            .request_contract_violations(&outcome, &runtime_request)
            .join("\n");
        let summary = envelope.request_parity_summary(&outcome, &runtime_request);

        assert!(joined.contains("generated 2 tokens above request max_tokens 1"));
        assert!(joined.contains("imported KV count 2 differs from request imported KV count 1"));
        assert!(joined.contains("exports 2 KV blocks above request runtime limit 1"));
        assert!(joined.contains("selected adapter"));
        assert!(joined.contains("generation budget differs"));
        assert!(joined.contains("route budget differs"));
        assert!(joined.contains("hardware pressure"));
        assert!(!summary.token_count_within_request);
        assert!(!summary.imported_kv_matches_request);
        assert!(!summary.exported_kv_within_runtime);
        assert!(!summary.selected_adapter_matches_request);
        assert!(!summary.generation_budget_matches_request);
        assert!(!summary.route_budget_matches_request);
        assert!(!summary.hardware_pressure_matches_request);
        assert!(summary.token_drifted_from_request());
        assert!(!summary.token_drifted_from_planning());
        assert!(summary.imported_kv_drifted_from_request());
        assert!(!summary.imported_kv_exceeds_planning());
        assert!(summary.exported_kv_exceeds_runtime());
        assert!(!summary.exported_kv_exceeds_planning());
        assert!(!summary.adapter_missing_from_runtime());
        assert!(summary.adapter_drifted_from_request());
        assert!(!summary.generation_budget_missing());
        assert!(summary.generation_budget_drifted());
        assert!(summary.route_budget_drifted());
        assert!(summary.hardware_pressure_drifted());
        assert!(!summary.planning_diagnostics_drifted());
        assert_eq!(summary.token_drift_component_count(), 1);
        assert_eq!(summary.request_token_drift_component_count(), 1);
        assert_eq!(summary.planning_token_drift_component_count(), 0);
        assert_eq!(summary.kv_drift_component_count(), 2);
        assert_eq!(summary.request_kv_drift_component_count(), 1);
        assert_eq!(summary.runtime_kv_drift_component_count(), 1);
        assert_eq!(summary.planning_kv_drift_component_count(), 0);
        assert_eq!(summary.adapter_drift_component_count(), 1);
        assert_eq!(summary.diagnostics_drift_component_count(), 3);
        assert_eq!(summary.request_diagnostics_drift_component_count(), 3);
        assert_eq!(summary.planning_diagnostics_drift_component_count(), 0);
        assert_eq!(summary.request_drift_component_count(), 7);
        assert!(!summary.planning_has_pre_request_gate_problems());
        assert!(!summary.planning_has_pressure_signals());
        assert_eq!(
            summary.planning_pre_request_gate_problem_component_count(),
            0
        );
        assert_eq!(summary.planning_pressure_signal_component_count(), 0);
        assert_eq!(summary.response_wire_problem_component_count(), 7);
        assert!(summary.has_response_wire_problem_components());
        assert!(summary.response_wire_accounting_is_consistent());
        assert!(!summary.response_wire_shape_is_clean());
        assert!(!summary.can_use_response_wire());
        assert!(!summary.token_parity_ok());
        assert!(!summary.kv_parity_ok());
        assert!(!summary.adapter_parity_ok());
        assert!(!summary.diagnostics_parity_ok());
        assert!(!summary.request_parity_is_consistent());
    }

    #[test]
    fn response_boundary_blocks_adapter_and_precision_drift_with_device_signal() {
        let metadata = RuntimeMetadata::new("model", "tok", 128, 16)
            .with_kv_exchange(true, true)
            .with_kv_limits(1, 1)
            .with_kv_precision(4, 4);
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(16)
            .with_max_tokens(2)
            .with_runtime(metadata);
        let runtime_request = request_envelope(&request, RuntimeAdapter::Cuda, 1);
        let hardware = HardwareAllocator::new().plan(
            HardwareLoadSnapshot::new(DeviceClass::DiscreteGpu, 0.1, 0.1, 0.1, 0.1),
            TaskProfile::Coding,
            512,
            HierarchyWeights::for_profile(TaskProfile::Coding),
        );
        let runtime = RuntimeDiagnostics::empty()
            .with_model_id("model")
            .with_selected_adapter(RuntimeAdapter::PortableRust)
            .with_architecture(1, 16, 64)
            .with_device_execution(
                "gpu",
                "gpu",
                "cpu",
                "gpu-resident",
                DeviceExecutionSource::RuntimeReported,
            )
            .with_kv_exchange(1, 1)
            .with_kv_precision(8, 4);
        let mut outcome = InferenceOutcome::empty().with_diagnostics(
            InferenceDiagnostics::new(runtime_request.route_budget)
                .with_runtime(runtime)
                .with_generation_budget(runtime_request.generation_budget),
        );
        outcome.answer = "ok".to_owned();
        outcome.tokens.push(GeneratedToken::new("ok"));
        outcome.imported_kv.push(runtime_block(1));
        outcome.exported_kv.push(runtime_block(2));

        let envelope = RuntimeResponseEnvelope::from_outcome(&outcome);
        let response_summary = envelope.envelope_summary();
        let response_parity = envelope.request_parity_summary(&outcome, &runtime_request);
        let diagnostics_summary = outcome.diagnostics.runtime.diagnostics_summary();
        let diagnostics_parity = outcome
            .diagnostics
            .runtime
            .request_parity_summary(&runtime_request);
        let report = envelope.acceptance_report(&outcome, &runtime_request, &hardware);
        let gate = envelope.response_gate_summary(&outcome, &runtime_request, &hardware);
        let joined = report.violations().join("\n");

        assert!(response_summary.response_envelope_shape_is_clean());
        assert!(response_summary.has_runtime_execution_signal);
        assert!(diagnostics_summary.has_runtime_reported_device_execution());
        assert_eq!(
            diagnostics_summary.device_execution_source,
            Some(DeviceExecutionSource::RuntimeReported)
        );
        assert!(diagnostics_summary.has_valid_kv_precision);
        assert!(!response_parity.selected_adapter_matches_request);
        assert!(response_parity.adapter_drifted_from_request());
        assert!(!diagnostics_parity.selected_adapter_matches_request);
        assert!(diagnostics_parity.kv_precision_reported);
        assert!(diagnostics_parity.kv_precision_valid);
        assert!(!diagnostics_parity.kv_precision_within_request);
        assert!(!diagnostics_parity.precision_parity_ok());
        assert!(gate.envelope_consistent);
        assert!(!gate.response_accepted);
        assert!(!gate.request_parity_consistent);
        assert!(gate.exported_kv_accepted);
        assert!(gate.has_response_contract_failures());
        assert!(gate.has_request_parity_failures());
        assert!(!gate.has_exported_kv_failures());
        assert!(!gate.can_commit_runtime_response());
        assert!(joined.contains("hot KV precision 8 exceeds metadata 4"));
        assert!(
            joined.contains("selected adapter portable-rust differs from request adapter cuda")
        );
    }

    #[test]
    fn response_request_parity_classifies_missing_runtime_reports() {
        let metadata = RuntimeMetadata::new("model", "tok", 128, 16);
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(16)
            .with_max_tokens(2)
            .with_runtime(metadata);
        let runtime_request = request_envelope(&request, RuntimeAdapter::Cuda, 0);
        let mut outcome = InferenceOutcome::empty()
            .with_diagnostics(InferenceDiagnostics::new(runtime_request.route_budget));
        outcome.answer = "ok".to_owned();
        outcome.tokens.push(GeneratedToken::new("ok"));

        let summary = RuntimeResponseEnvelope::from_outcome(&outcome)
            .request_parity_summary(&outcome, &runtime_request);

        assert!(!summary.token_drifted_from_request());
        assert!(!summary.imported_kv_drifted_from_request());
        assert!(!summary.exported_kv_exceeds_runtime());
        assert!(summary.adapter_missing_from_runtime());
        assert!(!summary.adapter_drifted_from_request());
        assert!(summary.generation_budget_missing());
        assert!(!summary.generation_budget_drifted());
        assert!(!summary.route_budget_drifted());
        assert!(!summary.hardware_pressure_drifted());
        assert_eq!(summary.token_drift_component_count(), 0);
        assert_eq!(summary.request_token_drift_component_count(), 0);
        assert_eq!(summary.planning_token_drift_component_count(), 0);
        assert_eq!(summary.kv_drift_component_count(), 0);
        assert_eq!(summary.request_kv_drift_component_count(), 0);
        assert_eq!(summary.runtime_kv_drift_component_count(), 0);
        assert_eq!(summary.planning_kv_drift_component_count(), 0);
        assert_eq!(summary.adapter_drift_component_count(), 1);
        assert_eq!(summary.diagnostics_drift_component_count(), 1);
        assert_eq!(summary.request_diagnostics_drift_component_count(), 1);
        assert_eq!(summary.planning_diagnostics_drift_component_count(), 0);
        assert_eq!(summary.request_drift_component_count(), 2);
        assert!(!summary.planning_has_pre_request_gate_problems());
        assert!(!summary.planning_has_pressure_signals());
        assert_eq!(
            summary.planning_pre_request_gate_problem_component_count(),
            0
        );
        assert_eq!(summary.planning_pressure_signal_component_count(), 0);
        assert_eq!(summary.response_wire_problem_component_count(), 2);
        assert!(summary.has_response_wire_problem_components());
        assert!(summary.response_wire_accounting_is_consistent());
        assert!(!summary.response_wire_shape_is_clean());
        assert!(!summary.can_use_response_wire());
        assert!(summary.token_parity_ok());
        assert!(summary.kv_parity_ok());
        assert!(!summary.adapter_parity_ok());
        assert!(!summary.diagnostics_parity_ok());
        assert!(!summary.request_parity_is_consistent());
    }

    #[test]
    fn response_request_parity_classifies_planning_drift_components() {
        let summary = RuntimeResponseRequestParitySummary {
            token_count: 4,
            request_max_tokens: 8,
            planned_backend_max_tokens: Some(2),
            token_count_within_request: true,
            token_count_within_planning: Some(false),
            imported_kv_blocks: 3,
            request_imported_kv_blocks: 3,
            planned_imported_kv_blocks: Some(2),
            imported_kv_matches_request: true,
            imported_kv_within_planning: Some(false),
            exported_kv_blocks: 2,
            runtime_export_enabled: true,
            runtime_max_export_blocks: 4,
            planned_exported_kv_blocks: Some(1),
            exported_kv_within_runtime: true,
            exported_kv_within_planning: Some(false),
            request_selected_adapter: Some(RuntimeAdapter::Cuda),
            runtime_selected_adapter: Some(RuntimeAdapter::Cuda),
            runtime_adapter_reported: true,
            selected_adapter_matches_request: true,
            generation_budget_reported: true,
            generation_budget_matches_request: true,
            route_budget_matches_request: true,
            hardware_pressure_matches_request: true,
            compute_headroom_matches_planning: Some(false),
            latency_budget_matches_planning: Some(false),
            planning_pre_request_problem_count: 2,
            planning_pressure_signal_count: 3,
        };

        assert!(summary.has_planning_digest());
        assert!(!summary.token_drifted_from_request());
        assert!(summary.token_drifted_from_planning());
        assert!(!summary.imported_kv_drifted_from_request());
        assert!(summary.imported_kv_exceeds_planning());
        assert!(!summary.exported_kv_exceeds_runtime());
        assert!(summary.exported_kv_exceeds_planning());
        let planned_kv = summary.planned_kv_summary();
        assert!(planned_kv.has_planning_digest);
        assert_eq!(planned_kv.imported_kv_blocks, 3);
        assert_eq!(planned_kv.exported_kv_blocks, 2);
        assert_eq!(planned_kv.planned_imported_kv_blocks, Some(2));
        assert_eq!(planned_kv.planned_exported_kv_blocks, Some(1));
        assert!(planned_kv.has_response_kv_activity());
        assert!(planned_kv.has_planned_kv_activity());
        assert!(planned_kv.planning_limits_reported());
        assert!(planned_kv.imported_kv_exceeds_planning());
        assert!(planned_kv.exported_kv_exceeds_planning());
        assert!(!planned_kv.response_kv_within_planning());
        assert!(planned_kv.response_kv_exceeds_planning());
        assert_eq!(planned_kv.response_kv_activity_signal_component_count(), 2);
        assert_eq!(planned_kv.planned_kv_activity_signal_component_count(), 2);
        assert_eq!(planned_kv.response_planned_kv_signal_component_count(), 5);
        assert!(planned_kv.has_response_planned_kv_signals());
        assert_eq!(planned_kv.planning_limit_missing_component_count(), 0);
        assert_eq!(planned_kv.planning_kv_exceeded_component_count(), 2);
        assert_eq!(planned_kv.response_planned_kv_problem_component_count(), 2);
        assert!(planned_kv.has_response_planned_kv_problem_components());
        assert!(planned_kv.response_planned_kv_accounting_is_consistent());
        assert!(!planned_kv.response_planned_kv_shape_is_clean());
        assert!(!planned_kv.can_use_response_planned_kv());
        assert!(!planned_kv.can_commit_planned_kv_response());
        let planned_zero_export = RuntimeResponseRequestParitySummary {
            imported_kv_blocks: 1,
            planned_imported_kv_blocks: Some(1),
            imported_kv_within_planning: Some(true),
            exported_kv_blocks: 0,
            planned_exported_kv_blocks: Some(0),
            exported_kv_within_planning: Some(true),
            ..summary
        }
        .planned_kv_summary();
        assert!(planned_zero_export.response_kv_matches_planned_zero_export());
        assert!(planned_zero_export.response_kv_within_planning());
        assert_eq!(
            planned_zero_export.response_planned_kv_signal_component_count(),
            3
        );
        assert_eq!(
            planned_zero_export.response_planned_kv_problem_component_count(),
            0
        );
        assert!(planned_zero_export.response_planned_kv_shape_is_clean());
        assert!(planned_zero_export.can_use_response_planned_kv());
        assert!(planned_zero_export.can_commit_planned_kv_response());
        assert!(!summary.adapter_missing_from_runtime());
        assert!(!summary.adapter_drifted_from_request());
        assert!(!summary.generation_budget_missing());
        assert!(!summary.generation_budget_drifted());
        assert!(summary.planning_diagnostics_drifted());
        assert!(summary.planning_has_pre_request_gate_problems());
        assert!(summary.planning_has_pressure_signals());
        assert_eq!(summary.token_drift_component_count(), 1);
        assert_eq!(summary.request_token_drift_component_count(), 0);
        assert_eq!(summary.planning_token_drift_component_count(), 1);
        assert_eq!(summary.kv_drift_component_count(), 2);
        assert_eq!(summary.request_kv_drift_component_count(), 0);
        assert_eq!(summary.runtime_kv_drift_component_count(), 0);
        assert_eq!(summary.planning_kv_drift_component_count(), 2);
        assert_eq!(summary.adapter_drift_component_count(), 0);
        assert_eq!(summary.diagnostics_drift_component_count(), 1);
        assert_eq!(summary.request_diagnostics_drift_component_count(), 0);
        assert_eq!(summary.planning_diagnostics_drift_component_count(), 2);
        assert_eq!(summary.request_drift_component_count(), 4);
        assert_eq!(
            summary.planning_pre_request_gate_problem_component_count(),
            1
        );
        assert_eq!(summary.planning_pressure_signal_component_count(), 1);
        assert_eq!(summary.response_wire_problem_component_count(), 5);
        assert!(summary.has_response_wire_problem_components());
        assert!(summary.response_wire_accounting_is_consistent());
        assert!(!summary.response_wire_shape_is_clean());
        assert!(!summary.can_use_response_wire());
        assert!(!summary.token_parity_ok());
        assert!(!summary.kv_parity_ok());
        assert!(summary.adapter_parity_ok());
        assert!(!summary.diagnostics_parity_ok());
        assert!(!summary.request_parity_is_consistent());
    }

    #[test]
    fn response_manifest_kv_summary_allows_clean_manifest_planned_response() {
        let parity = clean_response_request_parity();
        let manifest_bridge = clean_manifest_kv_bridge();
        let summary = parity.manifest_kv_summary(manifest_bridge);

        assert_eq!(summary.manifest_kv_bridge, manifest_bridge);
        assert_eq!(summary.response_planned_kv, parity.planned_kv_summary());
        assert!(summary.manifest_bridge_ready());
        assert!(summary.response_planned_kv_ready());
        assert!(summary.manifest_import_plan_covers_response());
        assert!(summary.manifest_export_plan_covers_response());
        assert!(summary.response_kv_within_manifest_plan());
        assert_eq!(
            summary.manifest_kv_bridge_signal_component_count,
            manifest_bridge.manifest_kv_bridge_signal_component_count()
        );
        assert_eq!(
            summary.response_planned_kv_signal_component_count,
            parity
                .planned_kv_summary()
                .response_planned_kv_signal_component_count()
        );
        assert_eq!(summary.manifest_kv_bridge_blocker_component_count, 0);
        assert_eq!(summary.response_planned_kv_blocker_component_count, 0);
        assert_eq!(summary.response_manifest_kv_blocker_component_count, 0);
        assert_eq!(summary.manifest_bridge_blocker_component_count(), 0);
        assert_eq!(summary.response_planned_kv_blocker_component_count(), 0);
        assert_eq!(
            summary.response_kv_exceeds_manifest_plan_component_count(),
            0
        );
        assert!(summary.has_response_manifest_kv_signals());
        assert!(!summary.has_response_manifest_kv_blockers());
        assert_eq!(summary.response_manifest_kv_blocker_component_count(), 0);
        assert!(summary.response_manifest_kv_accounting_is_consistent());
        assert!(summary.response_manifest_kv_shape_is_clean());
        assert!(summary.can_use_response_manifest_kv());
        assert!(summary.can_commit_response_manifest_kv());
    }

    #[test]
    fn response_manifest_kv_summary_blocks_manifest_plan_drift() {
        let mut manifest_bridge = clean_manifest_kv_bridge();
        manifest_bridge.planned_export_blocks = 2;
        let parity = RuntimeResponseRequestParitySummary {
            exported_kv_blocks: 2,
            runtime_max_export_blocks: 2,
            planned_exported_kv_blocks: Some(2),
            exported_kv_within_runtime: true,
            exported_kv_within_planning: Some(true),
            ..clean_response_request_parity()
        };
        let summary = parity.manifest_kv_summary(manifest_bridge);

        assert!(!summary.manifest_bridge_ready());
        assert!(summary.response_planned_kv_ready());
        assert!(summary.manifest_import_plan_covers_response());
        assert!(!summary.manifest_export_plan_covers_response());
        assert!(!summary.response_kv_within_manifest_plan());
        assert_eq!(summary.manifest_kv_bridge_blocker_component_count, 1);
        assert_eq!(summary.response_planned_kv_blocker_component_count, 0);
        assert_eq!(summary.response_manifest_kv_blocker_component_count, 1);
        assert_eq!(summary.manifest_bridge_blocker_component_count(), 1);
        assert_eq!(summary.response_planned_kv_blocker_component_count(), 0);
        assert_eq!(
            summary.response_kv_exceeds_manifest_plan_component_count(),
            1
        );
        assert_eq!(summary.response_manifest_kv_blocker_component_count(), 2);
        assert!(summary.has_response_manifest_kv_blockers());
        assert!(summary.response_manifest_kv_accounting_is_consistent());
        assert!(!summary.response_manifest_kv_shape_is_clean());
        assert!(!summary.can_use_response_manifest_kv());
        assert!(!summary.can_commit_response_manifest_kv());
    }

    #[test]
    fn response_gate_summary_splits_wire_problems_from_pressure_signals() {
        let gate = RuntimeResponseGateSummary {
            response_accepted: false,
            envelope_consistent: true,
            request_parity_consistent: false,
            exported_kv_accepted: true,
            accepted_exported_kv_blocks: 0,
            response_wire_problem_count: 4,
            planning_pre_request_problem_count: 1,
            planning_pressure_signal_count: 3,
            response_violation_count: 0,
            request_violation_count: 1,
            exported_kv_violation_count: 0,
            failure_report_count: 1,
        };

        assert!(gate.has_response_wire_problem_components());
        assert!(gate.has_planning_pre_request_gate_problems());
        assert!(gate.has_planning_pressure_signals());
        assert_eq!(gate.response_wire_problem_component_count(), 4);
        assert_eq!(gate.direct_response_wire_problem_component_count(), 3);
        assert_eq!(gate.planning_pre_request_gate_problem_component_count(), 1);
        assert_eq!(gate.planning_pressure_signal_component_count(), 1);
        assert_eq!(gate.exported_kv_activity_signal_component_count(), 0);
        assert_eq!(gate.response_gate_signal_component_count(), 3);
        assert!(gate.response_gate_has_signal_components());
        assert!(gate.response_wire_accounting_is_consistent());
        assert!(!gate.response_gate_shape_is_clean());
        assert!(!gate.can_accept_runtime_response());
        assert!(gate.has_request_parity_failures());
        assert!(gate.request_parity_drifted());
        assert_eq!(gate.request_parity_failure_component_count(), 1);
        assert_eq!(gate.request_parity_blocker_component_count(), 1);
        assert_eq!(gate.runtime_response_commit_signal_component_count(), 3);
        assert!(gate.has_runtime_response_commit_signals());
        assert_eq!(gate.runtime_response_commit_blocker_component_count(), 6);
        assert!(gate.has_runtime_response_commit_blockers());
        assert!(gate.runtime_response_commit_accounting_is_consistent());
        assert!(!gate.runtime_response_commit_is_clean());
        assert!(!gate.can_commit_runtime_response());
    }

    #[test]
    fn response_gate_summary_counts_public_shape_drift() {
        let gate = RuntimeResponseGateSummary {
            response_accepted: true,
            envelope_consistent: true,
            request_parity_consistent: true,
            exported_kv_accepted: true,
            accepted_exported_kv_blocks: 0,
            response_wire_problem_count: 1,
            planning_pre_request_problem_count: 0,
            planning_pressure_signal_count: 0,
            response_violation_count: 0,
            request_violation_count: 0,
            exported_kv_violation_count: 0,
            failure_report_count: 0,
        };

        assert!(gate.can_accept_response());
        assert!(gate.is_clean_response_gate());
        assert!(gate.response_gate_accounting_is_consistent());
        assert!(gate.failure_report_matches_failures());
        assert!(!gate.response_wire_accounting_is_consistent());
        assert!(!gate.response_gate_shape_is_clean());
        assert_eq!(gate.runtime_response_commit_signal_component_count(), 0);
        assert!(!gate.has_runtime_response_commit_signals());
        assert_eq!(gate.runtime_response_commit_blocker_component_count(), 1);
        assert!(gate.has_runtime_response_commit_blockers());
        assert!(!gate.runtime_response_commit_accounting_is_consistent());
        assert!(!gate.runtime_response_commit_is_clean());
        assert!(!gate.can_commit_runtime_response());
        assert!(!gate.can_accept_runtime_response());
    }

    #[test]
    fn runtime_response_readiness_confirms_response_wire_boundary() {
        let envelope = clean_response_envelope_summary();
        let parity = clean_response_request_parity();
        let gate = clean_response_gate();
        let readiness = RuntimeResponseReadinessSummary::new(envelope, parity, gate);

        assert_eq!(
            RuntimeResponseReadinessSummary::stage_order(),
            [
                RuntimeResponseReadinessStage::ResponseEnvelope,
                RuntimeResponseReadinessStage::ResponseRequestParity,
                RuntimeResponseReadinessStage::ResponseGate,
            ]
        );
        assert!(readiness.response_envelope_ready());
        assert!(readiness.response_request_ready());
        assert!(readiness.response_gate_ready());
        assert_eq!(readiness.first_unready_stage(), None);
        assert_eq!(readiness.first_blocking_stage(), None);
        assert_eq!(
            readiness.stage_signal_component_count(RuntimeResponseReadinessStage::ResponseEnvelope),
            readiness.response_envelope_signal_component_count
        );
        assert_eq!(
            readiness.stage_blocker_component_count(RuntimeResponseReadinessStage::ResponseGate),
            readiness.response_gate_blocker_component_count
        );
        assert_eq!(readiness.response_envelope_blocker_component_count, 0);
        assert_eq!(readiness.response_request_blocker_component_count, 0);
        assert_eq!(readiness.response_gate_blocker_component_count, 0);
        assert_eq!(
            readiness.runtime_response_readiness_signal_component_count(),
            envelope
                .runtime_response_envelope_commit_signal_component_count()
                .saturating_add(parity.planning_pressure_signal_component_count())
                .saturating_add(gate.runtime_response_commit_signal_component_count())
        );
        assert!(readiness.has_runtime_response_readiness_signals());
        assert_eq!(
            readiness.runtime_response_readiness_blocker_component_count(),
            0
        );
        assert!(!readiness.has_runtime_response_readiness_blockers());
        assert!(readiness.runtime_response_readiness_accounting_is_consistent());
        assert!(readiness.runtime_response_readiness_is_clean());
        assert!(readiness.can_commit_runtime_response_readiness());
    }

    #[test]
    fn runtime_response_readiness_blocks_response_request_parity_drift() {
        let envelope = clean_response_envelope_summary();
        let mut parity = clean_response_request_parity();
        parity.token_count_within_planning = Some(false);
        parity.imported_kv_within_planning = Some(false);
        parity.planning_pre_request_problem_count = 2;
        let mut gate = clean_response_gate();
        gate.response_accepted = false;
        gate.request_parity_consistent = false;
        gate.response_wire_problem_count = parity.response_wire_problem_component_count();
        gate.planning_pre_request_problem_count = parity.planning_pre_request_problem_count;
        gate.request_violation_count = 1;
        gate.failure_report_count = 1;
        let readiness = RuntimeResponseReadinessSummary::new(envelope, parity, gate);

        assert!(readiness.response_envelope_ready());
        assert!(!readiness.response_request_ready());
        assert!(!readiness.response_gate_ready());
        assert_eq!(
            readiness.first_unready_stage(),
            Some(RuntimeResponseReadinessStage::ResponseRequestParity)
        );
        assert_eq!(
            readiness.first_blocking_stage(),
            Some(RuntimeResponseReadinessStage::ResponseRequestParity)
        );
        assert_eq!(readiness.response_request_blocker_component_count, 3);
        assert_eq!(readiness.response_gate_blocker_component_count, 4);
        assert_eq!(
            readiness.runtime_response_readiness_blocker_component_count(),
            7
        );
        assert!(readiness.has_runtime_response_readiness_blockers());
        assert!(readiness.runtime_response_readiness_accounting_is_consistent());
        assert!(!readiness.runtime_response_readiness_is_clean());
        assert!(!readiness.can_commit_runtime_response_readiness());
    }

    #[test]
    fn response_acceptance_summary_counts_public_shape_drift() {
        let clean = RuntimeResponseAcceptanceSummary {
            accepted: true,
            response_violation_count: 0,
            request_violation_count: 0,
            exported_kv_violation_count: 0,
            accepted_exported_kv_blocks: 1,
            failure_report_count: 0,
        };
        let drift = RuntimeResponseAcceptanceSummary {
            accepted: false,
            response_violation_count: 1,
            request_violation_count: 1,
            exported_kv_violation_count: 0,
            accepted_exported_kv_blocks: 0,
            failure_report_count: 1,
        };

        assert_eq!(clean.response_acceptance_problem_component_count(), 0);
        assert!(!clean.has_response_acceptance_problem_components());
        assert!(clean.response_acceptance_accounting_is_consistent());
        assert_eq!(
            clean.runtime_response_acceptance_commit_signal_component_count(),
            2
        );
        assert_eq!(
            clean.runtime_response_acceptance_commit_blocker_component_count(),
            0
        );
        assert!(clean.runtime_response_acceptance_commit_accounting_is_consistent());
        assert!(clean.runtime_response_acceptance_commit_is_clean());
        assert!(clean.response_acceptance_shape_is_clean());
        assert!(clean.can_commit_runtime_response_acceptance());
        assert!(clean.is_clean_acceptance());

        assert_eq!(drift.total_violation_count(), 2);
        assert!(drift.has_response_contract_failures());
        assert!(drift.has_request_parity_failures());
        assert_eq!(drift.acceptance_failure_component_count(), 2);
        assert_eq!(drift.response_acceptance_problem_component_count(), 3);
        assert!(drift.has_response_acceptance_problem_components());
        assert!(!drift.failure_report_matches_failures());
        assert!(!drift.response_acceptance_accounting_is_consistent());
        assert_eq!(
            drift.runtime_response_acceptance_commit_signal_component_count(),
            0
        );
        assert_eq!(
            drift.runtime_response_acceptance_commit_blocker_component_count(),
            3
        );
        assert!(!drift.runtime_response_acceptance_commit_accounting_is_consistent());
        assert!(!drift.runtime_response_acceptance_commit_is_clean());
        assert!(!drift.response_acceptance_shape_is_clean());
        assert!(!drift.can_commit_runtime_response_acceptance());
        assert!(!drift.is_clean_acceptance());
    }

    #[test]
    fn response_envelope_validates_exported_kv_blocks_from_request_contract() {
        let metadata = RuntimeMetadata::new("model", "tok", 128, 16)
            .with_kv_exchange(true, true)
            .with_kv_limits(0, 1);
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(16)
            .with_max_tokens(2)
            .with_runtime(metadata);
        let runtime_request = request_envelope(&request, RuntimeAdapter::Cuda, 0);
        let runtime = RuntimeDiagnostics::empty()
            .with_selected_adapter(RuntimeAdapter::Cuda)
            .with_kv_exchange(0, 2);
        let mut outcome = InferenceOutcome::empty().with_diagnostics(
            InferenceDiagnostics::new(RouteBudget::default())
                .with_runtime(runtime)
                .with_generation_budget(runtime_request.generation_budget),
        );
        outcome.answer = "ok".to_owned();
        outcome.tokens.push(GeneratedToken::new("ok"));
        outcome.exported_kv.push(runtime_block(1));
        outcome.exported_kv.push(KvBlock::new(
            2,
            KvNamespace::Gist,
            0,
            0,
            0..1,
            vec![0.3],
            vec![0.4],
        ));

        let envelope = RuntimeResponseEnvelope::from_outcome(&outcome);
        let report = envelope.validate_exported_kv_blocks(&outcome, &runtime_request);
        let joined = report.violations.join("\n");

        assert_eq!(report.accepted, vec![runtime_block(1)]);
        assert!(joined.contains("exported KV block count 2 exceeds contract max_blocks 1"));
    }

    #[test]
    fn response_acceptance_report_combines_response_request_and_exported_kv_violations() {
        let metadata = RuntimeMetadata::new("model", "tok", 128, 16)
            .with_kv_exchange(true, true)
            .with_kv_limits(0, 2);
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(16)
            .with_max_tokens(1)
            .with_runtime(metadata);
        let runtime_request = request_envelope(&request, RuntimeAdapter::Cuda, 0);
        let hardware = HardwareAllocator::new().plan(
            HardwareLoadSnapshot::new(DeviceClass::CpuOnly, 0.1, 0.1, 0.1, 0.1),
            TaskProfile::Coding,
            512,
            HierarchyWeights::for_profile(TaskProfile::Coding),
        );
        let runtime = RuntimeDiagnostics::empty()
            .with_selected_adapter(RuntimeAdapter::Cuda)
            .with_kv_exchange(0, 1);
        let mut outcome = InferenceOutcome::empty().with_diagnostics(
            InferenceDiagnostics::new(RouteBudget::default())
                .with_runtime(runtime)
                .with_generation_budget(runtime_request.generation_budget),
        );
        outcome.answer = " ".to_owned();
        outcome.tokens.push(GeneratedToken::new("too"));
        outcome.tokens.push(GeneratedToken::new("many"));
        outcome.exported_kv.push(KvBlock::new(
            1,
            KvNamespace::Gist,
            0,
            0,
            0..1,
            vec![0.1],
            vec![0.2],
        ));

        let envelope = RuntimeResponseEnvelope::from_outcome(&outcome);
        let report = envelope.acceptance_report(&outcome, &runtime_request, &hardware);
        let gate = envelope.response_gate_summary(&outcome, &runtime_request, &hardware);
        let joined = report.violations().join("\n");
        let summary = report.acceptance_summary();

        assert!(!report.is_accepted());
        assert!(!summary.accepted);
        assert!(!gate.response_accepted);
        assert!(!gate.envelope_consistent);
        assert!(!gate.request_parity_consistent);
        assert!(!gate.exported_kv_accepted);
        assert!(gate.has_acceptance_failures());
        assert!(gate.has_response_wire_problem_components());
        assert_eq!(gate.planning_pre_request_problem_count, 0);
        assert_eq!(gate.planning_pressure_signal_count, 0);
        assert_eq!(gate.planning_pre_request_gate_problem_component_count(), 0);
        assert_eq!(gate.planning_pressure_signal_component_count(), 0);
        assert_eq!(
            gate.response_wire_problem_component_count(),
            gate.direct_response_wire_problem_component_count()
        );
        assert!(gate.response_wire_accounting_is_consistent());
        assert!(gate.envelope_drifted());
        assert!(gate.request_parity_drifted());
        assert!(gate.exported_kv_drifted());
        assert!(gate.has_boundary_drift());
        assert!(gate.has_failure_reports());
        assert!(gate.has_total_violations());
        assert_eq!(gate.response_contract_failure_component_count(), 1);
        assert_eq!(gate.request_parity_failure_component_count(), 1);
        assert_eq!(gate.exported_kv_failure_component_count(), 1);
        assert_eq!(gate.acceptance_failure_component_count(), 3);
        assert_eq!(gate.envelope_blocker_component_count(), 1);
        assert_eq!(gate.request_parity_blocker_component_count(), 1);
        assert_eq!(gate.exported_kv_blocker_component_count(), 1);
        assert_eq!(gate.boundary_drift_component_count(), 3);
        assert_eq!(gate.mapped_failure_report_component_count(), 1);
        assert_eq!(gate.response_blocker_component_count(), 7);
        assert!(gate.response_gate_has_problem_components());
        assert!(gate.response_gate_accounting_is_consistent());
        assert_eq!(
            gate.runtime_response_commit_signal_component_count(),
            gate.response_gate_signal_component_count()
        );
        assert_eq!(
            gate.runtime_response_commit_blocker_component_count(),
            gate.response_blocker_component_count()
                .saturating_add(gate.direct_response_wire_problem_component_count())
        );
        assert!(gate.has_runtime_response_commit_blockers());
        assert!(gate.runtime_response_commit_accounting_is_consistent());
        assert!(!gate.runtime_response_commit_is_clean());
        assert!(!gate.can_commit_runtime_response());
        assert!(!gate.can_accept_response());
        assert!(!gate.response_gate_shape_is_clean());
        assert!(!gate.can_accept_runtime_response());
        assert_eq!(
            summary.response_violation_count,
            report.response_violations.len()
        );
        assert_eq!(
            summary.request_violation_count,
            report.request_violations.len()
        );
        assert_eq!(
            summary.exported_kv_violation_count,
            report.exported_kv_report.violations.len()
        );
        assert_eq!(
            summary.total_violation_count(),
            report.response_violations.len()
                + report.request_violations.len()
                + report.exported_kv_report.violations.len()
        );
        assert_eq!(summary.accepted_exported_kv_blocks, 0);
        assert_eq!(summary.failure_report_count, report.failure_reports().len());
        assert_eq!(
            gate.response_violation_count,
            summary.response_violation_count
        );
        assert_eq!(
            gate.request_violation_count,
            summary.request_violation_count
        );
        assert_eq!(
            gate.exported_kv_violation_count,
            summary.exported_kv_violation_count
        );
        assert_eq!(gate.failure_report_count, summary.failure_report_count);
        assert!(summary.has_response_contract_failures());
        assert!(summary.has_request_parity_failures());
        assert!(summary.has_exported_kv_failures());
        assert!(summary.has_failures());
        assert_eq!(summary.response_contract_failure_component_count(), 1);
        assert_eq!(summary.request_parity_failure_component_count(), 1);
        assert_eq!(summary.exported_kv_failure_component_count(), 1);
        assert_eq!(summary.acceptance_failure_component_count(), 3);
        assert!(summary.has_failure_reports());
        assert_eq!(summary.response_acceptance_problem_component_count(), 4);
        assert!(summary.failure_report_matches_failures());
        assert!(!summary.is_clean_acceptance());
        assert!(gate.has_response_contract_failures());
        assert!(gate.has_request_parity_failures());
        assert!(gate.has_exported_kv_failures());
        assert!(gate.failure_report_matches_failures());
        assert!(!gate.is_clean_response_gate());
        assert!(report.accepted_exported_kv_blocks().is_empty());
        assert!(joined.contains("runtime response answer must be non-empty"));
        assert!(joined.contains("generated 2 tokens above request max_tokens 1"));
        assert!(joined.contains("namespace gist is not runtime"));
    }

    #[test]
    fn response_acceptance_report_maps_failures_to_runtime_failure_reports() {
        let clean_report = RuntimeResponseAcceptanceReport {
            response_violations: Vec::new(),
            request_violations: Vec::new(),
            exported_kv_report: RuntimeKvValidationReport {
                accepted: Vec::new(),
                violations: Vec::new(),
            },
        };
        let clean_failure_return = clean_report.failure_return_summary();
        assert_eq!(
            clean_failure_return.source,
            RuntimeResponseFailureReturnSource::ResponseAcceptance
        );
        assert_eq!(
            clean_failure_return.source.label(),
            "runtime_response_acceptance"
        );
        assert!(!clean_failure_return.has_failure_reports());
        assert!(!clean_failure_return.has_blocker_components());
        assert!(clean_failure_return.failure_return_accounting_is_consistent());
        assert!(!clean_failure_return.can_return_runtime_failure());
        assert_eq!(clean_report.runtime_failure_return_report(), None);

        let metadata = RuntimeMetadata::new("model", "tok", 128, 16)
            .with_kv_exchange(true, true)
            .with_kv_limits(0, 2);
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(16)
            .with_max_tokens(1)
            .with_runtime(metadata);
        let runtime_request = request_envelope(&request, RuntimeAdapter::Cuda, 0);
        let hardware = HardwareAllocator::new().plan(
            HardwareLoadSnapshot::new(DeviceClass::CpuOnly, 0.1, 0.1, 0.1, 0.1),
            TaskProfile::Coding,
            512,
            HierarchyWeights::for_profile(TaskProfile::Coding),
        );
        let runtime = RuntimeDiagnostics::empty()
            .with_selected_adapter(RuntimeAdapter::Cuda)
            .with_kv_exchange(0, 1);
        let mut outcome = InferenceOutcome::empty().with_diagnostics(
            InferenceDiagnostics::new(RouteBudget::default())
                .with_runtime(runtime)
                .with_generation_budget(runtime_request.generation_budget),
        );
        outcome.answer = " ".to_owned();
        outcome.tokens.push(GeneratedToken::new("too"));
        outcome.tokens.push(GeneratedToken::new("many"));
        outcome.exported_kv.push(KvBlock::new(
            1,
            KvNamespace::Gist,
            0,
            0,
            0..1,
            vec![0.1],
            vec![0.2],
        ));

        let envelope = RuntimeResponseEnvelope::from_outcome(&outcome);
        let report = envelope.acceptance_report(&outcome, &runtime_request, &hardware);
        let failures = report.failure_reports();
        let failure_batch = report.failure_batch_summary();
        let primary_summary = report.primary_failure_summary().unwrap();

        assert_eq!(failures.len(), 3);
        assert_eq!(
            failure_batch,
            RuntimeFailureReport::batch_summary(&failures)
        );
        assert_eq!(failure_batch.total_count, 3);
        assert_eq!(failure_batch.contract_violation_count, 2);
        assert_eq!(failure_batch.kv_export_count, 1);
        assert_eq!(failure_batch.recoverable_count, 3);
        assert_eq!(failure_batch.backend_error_count, 0);
        assert!(failure_batch.has_kv_failures());
        assert!(failure_batch.has_contract_failures());
        assert!(failure_batch.all_failures_are_recoverable());
        assert!(failure_batch.failure_batch_shape_is_clean());
        assert!(failure_batch.can_format_runtime_failures());
        assert_eq!(failures[0].kind, RuntimeFailureKind::ContractViolation);
        assert!(failures[0].message.contains("response acceptance failed"));
        assert_eq!(failures[1].kind, RuntimeFailureKind::ContractViolation);
        assert!(failures[1].message.contains("request parity failed"));
        assert_eq!(failures[2].kind, RuntimeFailureKind::KvExport);
        assert_eq!(failures[2].kind.trace_label(), "runtime_kv_export_error");
        assert!(failures[2].is_recoverable());
        assert!(failures[2].diagnostics_note().contains("namespace gist"));
        assert_eq!(
            report.primary_failure_report().unwrap().kind,
            RuntimeFailureKind::ContractViolation
        );
        assert_eq!(primary_summary.kind, RuntimeFailureKind::ContractViolation);
        assert_eq!(primary_summary.trace_label, "runtime_contract_violation");
        assert!(primary_summary.recoverable);
        assert!(!primary_summary.backend_error);
        assert!(primary_summary.failure_summary_shape_is_clean());
        assert!(primary_summary.can_use_runtime_failure_report());
        let failure_return = report.failure_return_summary();
        assert_eq!(
            failure_return.source,
            RuntimeResponseFailureReturnSource::ResponseAcceptance
        );
        assert!(failure_return.has_failure_reports());
        assert!(failure_return.has_blocker_components());
        assert!(failure_return.failure_return_accounting_is_consistent());
        assert!(failure_return.can_return_runtime_failure());
        let return_report = report
            .runtime_failure_return_report()
            .expect("response acceptance return report");
        assert_eq!(
            return_report.source,
            RuntimeResponseFailureReturnSource::ResponseAcceptance
        );
        assert_eq!(return_report.primary_failure_summary, primary_summary);
        assert_eq!(return_report.failure_batch.total_count, 3);
        assert_eq!(return_report.failure_batch.contract_violation_count, 2);
        assert_eq!(return_report.failure_batch.kv_export_count, 1);
        assert!(return_report.failure_return_report_shape_is_clean());
        assert!(return_report.can_use_runtime_response_failure_return_report());
        assert!(
            return_report
                .backend_message()
                .contains("runtime response acceptance failed")
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

    fn clean_response_envelope_summary() -> RuntimeResponseEnvelopeSummary {
        RuntimeResponseEnvelopeSummary {
            schema: RUNTIME_RESPONSE_SCHEMA,
            answer_chars: 2,
            token_count: 2,
            entropy_count: 0,
            logprob_count: 0,
            has_uncertainty_signal: false,
            token_uncertainty_coverage_signal_count: 0,
            token_uncertainty_metric_problem_count: 0,
            token_uncertainty_accounting_consistent: true,
            imported_kv_blocks: 1,
            exported_kv_blocks: 1,
            diagnostics_imported_kv_blocks: 1,
            diagnostics_exported_kv_blocks: 1,
            diagnostics_weak_runtime_kv_imports_skipped: 0,
            has_runtime_execution_signal: true,
        }
    }

    fn clean_response_request_parity() -> RuntimeResponseRequestParitySummary {
        RuntimeResponseRequestParitySummary {
            token_count: 2,
            request_max_tokens: 4,
            planned_backend_max_tokens: Some(4),
            token_count_within_request: true,
            token_count_within_planning: Some(true),
            imported_kv_blocks: 1,
            request_imported_kv_blocks: 1,
            planned_imported_kv_blocks: Some(1),
            imported_kv_matches_request: true,
            imported_kv_within_planning: Some(true),
            exported_kv_blocks: 1,
            runtime_export_enabled: true,
            runtime_max_export_blocks: 1,
            planned_exported_kv_blocks: Some(1),
            exported_kv_within_runtime: true,
            exported_kv_within_planning: Some(true),
            request_selected_adapter: Some(RuntimeAdapter::Cuda),
            runtime_selected_adapter: Some(RuntimeAdapter::Cuda),
            runtime_adapter_reported: true,
            selected_adapter_matches_request: true,
            generation_budget_reported: true,
            generation_budget_matches_request: true,
            route_budget_matches_request: true,
            hardware_pressure_matches_request: true,
            compute_headroom_matches_planning: Some(true),
            latency_budget_matches_planning: Some(true),
            planning_pre_request_problem_count: 0,
            planning_pressure_signal_count: 1,
        }
    }

    fn clean_response_gate() -> RuntimeResponseGateSummary {
        RuntimeResponseGateSummary {
            response_accepted: true,
            envelope_consistent: true,
            request_parity_consistent: true,
            exported_kv_accepted: true,
            accepted_exported_kv_blocks: 1,
            response_wire_problem_count: 0,
            planning_pre_request_problem_count: 0,
            planning_pressure_signal_count: 1,
            response_violation_count: 0,
            request_violation_count: 0,
            exported_kv_violation_count: 0,
            failure_report_count: 0,
        }
    }

    fn clean_manifest_kv_bridge() -> RuntimePlanningManifestKvBridgeSummary {
        RuntimePlanningManifestKvBridgeSummary {
            import: RuntimeKvImportManifestPlanSummary {
                manifest_import_enabled: true,
                manifest_max_import_blocks: 1,
                runtime_import_enabled: true,
                runtime_max_import_blocks: 1,
                requested_prefetch_blocks: 1,
                import_plan_max_blocks: 1,
                embedding_dimensions: Some(16),
                architecture_layer_count: 1,
                architecture_kv_heads: 2,
            },
            export: RuntimeKvExportManifestPlanSummary {
                manifest_export_enabled: true,
                manifest_max_export_blocks: 1,
                runtime_export_enabled: true,
                runtime_max_export_blocks: 1,
                requested_export_blocks: 1,
                export_plan_max_blocks: 1,
                architecture_layer_count: 1,
                architecture_kv_heads: 2,
            },
            planned_import_blocks: 1,
            planned_export_blocks: 1,
        }
    }

    fn runtime_block(id: u64) -> KvBlock {
        KvBlock::new(id, KvNamespace::Runtime, 0, 0, 0..1, vec![0.1], vec![0.2])
    }

    fn request_envelope(
        request: &InferenceRequest,
        adapter: RuntimeAdapter,
        imported_kv_blocks: usize,
    ) -> RuntimeRequestEnvelope {
        request_envelope_with_route_budget(
            request,
            adapter,
            imported_kv_blocks,
            RouteBudget::default(),
        )
    }

    fn request_envelope_with_route_budget(
        request: &InferenceRequest,
        adapter: RuntimeAdapter,
        imported_kv_blocks: usize,
        route_budget: RouteBudget,
    ) -> RuntimeRequestEnvelope {
        let architecture = TransformerRuntimeArchitecture::new(1, 16, 2, 1, 64);
        let transformer_plan = TransformerPlanDigest::new(
            Some("response-test"),
            vec![TransformerLayerBudget::new(
                0,
                TransformerAttentionKind::Global,
                0.5,
                64,
            )],
        );
        let execution = crate::adapter::AdapterExecutionContext::new([adapter]);

        RuntimeRequestEnvelope::from_parts(
            request,
            architecture,
            route_budget,
            HierarchyWeights::for_profile(request.profile),
            &transformer_plan,
            &execution,
            imported_kv_blocks,
        )
    }
}
