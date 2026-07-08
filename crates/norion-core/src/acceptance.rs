use crate::adapter::{AdapterSelectionRuntimeSummary, RuntimeAdapter};
use crate::diagnostics::{
    InferenceDiagnostics, RuntimeDeviceExecutionEnvelopeSummary, RuntimeDiagnostics,
};
use crate::engine::{
    InferenceError, InferenceOutcome, InferenceRequest, RuntimeFailureBatchSummary,
    RuntimeFailureKind, RuntimeFailureReport, RuntimeFailureSummary,
};
use crate::hardware::HardwarePlan;
use crate::kv::{
    KvBlock, KvNamespaceCounts, RuntimeKvImportReadinessCommitAction,
    RuntimeKvImportReadinessSummary, RuntimeKvValidationSummary,
};
use crate::manifest::{RuntimeManifestDigest, TransformerRuntimeArchitecture};
use crate::planning::{
    RuntimePlanningDigest, RuntimePlanningManifestKvBridgeSummary, RuntimePlanningReadinessSummary,
};
use crate::profile::HierarchyWeights;
use crate::recursive::RecursiveScheduleSummary;
use crate::request::{
    RuntimeRequestAcceptanceReport, RuntimeRequestAcceptanceSummary, RuntimeRequestEnvelope,
    RuntimeRequestEnvelopeSummary, RuntimeRequestGateSummary,
    RuntimeRequestManifestPlanningReadinessSummary, RuntimeRequestPlanningReadinessSummary,
};
use crate::response::{
    RuntimeResponseAcceptanceReport, RuntimeResponseAcceptanceSummary, RuntimeResponseEnvelope,
    RuntimeResponseEnvelopeSummary, RuntimeResponseGateSummary, RuntimeResponseManifestKvSummary,
    RuntimeResponseReadinessSummary,
};
use crate::router::RouteBudget;
use crate::transformer::{
    RuntimeKvExportReadinessCommitAction, RuntimeKvExportReadinessSummary, TransformerPlanDigest,
};

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeAcceptanceContext {
    pub request: RuntimeRequestEnvelope,
    pub hardware: HardwarePlan,
    pub imported_kv_blocks: Vec<KvBlock>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeBoundaryAcceptanceSummary {
    pub accepted: bool,
    pub request: RuntimeRequestAcceptanceSummary,
    pub response: RuntimeResponseAcceptanceSummary,
    pub total_violation_count: usize,
    pub total_failure_report_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeBoundaryEnvelopeSummary {
    pub request: RuntimeRequestEnvelopeSummary,
    pub response: RuntimeResponseEnvelopeSummary,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeBoundaryAdapterSummary {
    pub request_selected_adapter: Option<RuntimeAdapter>,
    pub runtime_selected_adapter: Option<RuntimeAdapter>,
    pub adapter_candidate_count: usize,
    pub has_planning_selection: bool,
    pub selection: Option<AdapterSelectionRuntimeSummary>,
    pub request_adapter_reported: bool,
    pub runtime_adapter_reported: bool,
    pub runtime_adapter_matches_request: bool,
    pub runtime_adapter_allowed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeBoundaryDeviceExecutionReadinessSummary {
    pub runtime_reported_metadata_ready: bool,
    pub device_execution_envelope: RuntimeDeviceExecutionEnvelopeSummary,
    pub device_execution_envelope_ready: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeBoundaryDeviceExecutionCommitAction {
    CommitRuntimeBoundaryDeviceExecution,
    WaitForRuntimeReportedDeviceExecutionMetadata,
    RepairDeviceExecutionEnvelope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeBoundaryDeviceExecutionCommitSummary {
    pub readiness: RuntimeBoundaryDeviceExecutionReadinessSummary,
    pub action: RuntimeBoundaryDeviceExecutionCommitAction,
    pub can_commit: bool,
    pub should_wait_for_runtime_reported_metadata: bool,
    pub should_repair_device_execution_envelope: bool,
    pub total_signal_component_count: usize,
    pub total_blocker_component_count: usize,
    pub component_accounting_consistent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeBoundaryKvSummary {
    pub request_imported_kv_blocks: usize,
    pub concrete_imported_kv_blocks: usize,
    pub accepted_imported_kv_blocks: usize,
    pub imported_kv_violation_count: usize,
    pub response_imported_kv_blocks: usize,
    pub response_exported_kv_blocks: usize,
    pub diagnostics_imported_kv_blocks: usize,
    pub diagnostics_exported_kv_blocks: usize,
    pub diagnostics_weak_runtime_kv_imports_skipped: usize,
    pub accepted_exported_kv_blocks: usize,
    pub exported_kv_violation_count: usize,
    pub imported_namespace_counts: KvNamespaceCounts,
    pub exported_namespace_counts: KvNamespaceCounts,
    pub runtime_import_enabled: bool,
    pub runtime_export_enabled: bool,
    pub runtime_max_import_blocks: usize,
    pub runtime_max_export_blocks: usize,
    pub planned_imported_kv_blocks: Option<usize>,
    pub planned_exported_kv_blocks: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeManifestBoundaryKvStage {
    RequestManifestPlanning,
    ResponseManifestKv,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeManifestBoundaryKvSummary {
    pub request_manifest_planning: RuntimeRequestManifestPlanningReadinessSummary,
    pub response_manifest_kv: RuntimeResponseManifestKvSummary,
    pub request_manifest_planning_signal_component_count: usize,
    pub response_manifest_kv_signal_component_count: usize,
    pub request_manifest_planning_blocker_component_count: usize,
    pub response_manifest_kv_blocker_component_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeBoundaryGateSummary {
    pub request_accepted: bool,
    pub response_accepted: bool,
    pub envelope_consistent: bool,
    pub adapter_consistent: bool,
    pub kv_consistent: bool,
    pub request_backend_wire_problem_count: usize,
    pub request_planning_pre_request_problem_count: usize,
    pub request_planning_pressure_signal_count: usize,
    pub request_planning_dense_compute_avoided_tokens: usize,
    pub response_wire_problem_count: usize,
    pub planning_pre_request_problem_count: usize,
    pub planning_pressure_signal_count: usize,
    pub response_planning_dense_compute_avoided_tokens: usize,
    pub kv_boundary_signal_count: usize,
    pub response_uncertainty_coverage_signal_count: usize,
    pub response_uncertainty_metric_problem_count: usize,
    pub response_uncertainty_accounting_consistent: bool,
    pub total_violation_count: usize,
    pub total_failure_report_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeBoundaryCommitReadinessSummary {
    pub request_acceptance_ready: bool,
    pub response_acceptance_ready: bool,
    pub boundary_acceptance_ready: bool,
    pub boundary_envelope_ready: bool,
    pub boundary_adapter_ready: bool,
    pub boundary_kv_ready: bool,
    pub boundary_gate_ready: bool,
    pub runtime_response_ready: bool,
    pub request_acceptance_signal_component_count: usize,
    pub response_acceptance_signal_component_count: usize,
    pub boundary_acceptance_signal_component_count: usize,
    pub acceptance_signal_component_count: usize,
    pub boundary_envelope_signal_component_count: usize,
    pub boundary_adapter_signal_component_count: usize,
    pub boundary_kv_signal_component_count: usize,
    pub boundary_gate_signal_component_count: usize,
    pub planning_dense_compute_avoided_tokens: usize,
    pub envelope_signal_component_count: usize,
    pub adapter_signal_component_count: usize,
    pub kv_signal_component_count: usize,
    pub gate_signal_component_count: usize,
    pub runtime_response_signal_component_count: usize,
    pub total_signal_component_count: usize,
    pub request_acceptance_blocker_component_count: usize,
    pub response_acceptance_blocker_component_count: usize,
    pub boundary_acceptance_blocker_component_count: usize,
    pub acceptance_blocker_component_count: usize,
    pub boundary_envelope_blocker_component_count: usize,
    pub boundary_adapter_blocker_component_count: usize,
    pub boundary_kv_blocker_component_count: usize,
    pub boundary_gate_blocker_component_count: usize,
    pub envelope_blocker_component_count: usize,
    pub adapter_blocker_component_count: usize,
    pub kv_blocker_component_count: usize,
    pub gate_blocker_component_count: usize,
    pub runtime_response_blocker_component_count: usize,
    pub total_blocker_component_count: usize,
    pub component_accounting_consistent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeBoundaryCommitSummary {
    pub readiness: RuntimeBoundaryCommitReadinessSummary,
    pub action: RuntimeBoundaryCommitAction,
    pub can_commit: bool,
    pub should_return_failure: bool,
    pub first_unready_stage: Option<RuntimeBoundaryCommitStage>,
    pub first_blocking_stage: Option<RuntimeBoundaryCommitStage>,
    pub primary_failure_summary: Option<RuntimeFailureSummary>,
    pub failure_batch: RuntimeFailureBatchSummary,
    pub failure_report_count: usize,
    pub can_format_runtime_failures: bool,
    pub planning_dense_compute_avoided_tokens: usize,
    pub total_signal_component_count: usize,
    pub total_blocker_component_count: usize,
    pub component_accounting_consistent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeManifestBoundaryCommitReadinessSummary {
    pub boundary_commit: RuntimeBoundaryCommitReadinessSummary,
    pub manifest_boundary_kv: RuntimeManifestBoundaryKvSummary,
    pub boundary_commit_action: RuntimeBoundaryCommitAction,
    pub boundary_commit_ready: bool,
    pub manifest_boundary_kv_ready: bool,
    pub boundary_commit_signal_component_count: usize,
    pub manifest_boundary_kv_signal_component_count: usize,
    pub total_signal_component_count: usize,
    pub boundary_commit_blocker_component_count: usize,
    pub manifest_boundary_kv_blocker_component_count: usize,
    pub total_blocker_component_count: usize,
    pub component_accounting_consistent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeManifestBoundaryCommitSummary {
    pub readiness: RuntimeManifestBoundaryCommitReadinessSummary,
    pub action: RuntimeManifestBoundaryCommitAction,
    pub boundary_commit_action: RuntimeBoundaryCommitAction,
    pub can_commit: bool,
    pub should_return_failure: bool,
    pub first_unready_stage: Option<RuntimeManifestBoundaryCommitStage>,
    pub first_blocking_stage: Option<RuntimeManifestBoundaryCommitStage>,
    pub first_problem_kind: Option<RuntimeManifestBoundaryCommitProblemKind>,
    pub primary_failure_summary: Option<RuntimeFailureSummary>,
    pub failure_batch: RuntimeFailureBatchSummary,
    pub failure_report_count: usize,
    pub can_format_runtime_failures: bool,
    pub total_signal_component_count: usize,
    pub total_blocker_component_count: usize,
    pub component_accounting_consistent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeKvSideEffectReadinessSummary {
    pub import: RuntimeKvImportReadinessSummary,
    pub manifest_boundary_commit: RuntimeManifestBoundaryCommitReadinessSummary,
    pub export: RuntimeKvExportReadinessSummary,
    pub import_commit_action: RuntimeKvImportReadinessCommitAction,
    pub manifest_boundary_commit_action: RuntimeManifestBoundaryCommitAction,
    pub export_commit_action: RuntimeKvExportReadinessCommitAction,
    pub import_ready: bool,
    pub manifest_boundary_commit_ready: bool,
    pub export_ready: bool,
    pub import_signal_component_count: usize,
    pub manifest_boundary_commit_signal_component_count: usize,
    pub export_signal_component_count: usize,
    pub total_signal_component_count: usize,
    pub import_blocker_component_count: usize,
    pub manifest_boundary_commit_blocker_component_count: usize,
    pub export_blocker_component_count: usize,
    pub total_blocker_component_count: usize,
    pub component_accounting_consistent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeKvSideEffectCommitSummary {
    pub readiness: RuntimeKvSideEffectReadinessSummary,
    pub action: RuntimeKvSideEffectCommitAction,
    pub import_commit_action: RuntimeKvImportReadinessCommitAction,
    pub manifest_boundary_commit_action: RuntimeManifestBoundaryCommitAction,
    pub export_commit_action: RuntimeKvExportReadinessCommitAction,
    pub can_commit: bool,
    pub should_return_failure: bool,
    pub first_unready_stage: Option<RuntimeKvSideEffectStage>,
    pub first_blocking_stage: Option<RuntimeKvSideEffectStage>,
    pub first_problem_kind: Option<RuntimeKvSideEffectProblemKind>,
    pub primary_failure_summary: Option<RuntimeFailureSummary>,
    pub failure_batch: RuntimeFailureBatchSummary,
    pub failure_report_count: usize,
    pub can_format_runtime_failures: bool,
    pub total_signal_component_count: usize,
    pub total_blocker_component_count: usize,
    pub component_accounting_consistent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeFailureReturnSource {
    BoundaryCommit,
    ManifestBoundaryCommit,
    KvSideEffectCommit,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeFailureReturnSummary {
    pub source: RuntimeFailureReturnSource,
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
pub struct RuntimeFailureReturnReport {
    pub source: RuntimeFailureReturnSource,
    pub primary_failure: RuntimeFailureReport,
    pub primary_failure_summary: RuntimeFailureSummary,
    pub failure_batch: RuntimeFailureBatchSummary,
    pub failure_report_count: usize,
    pub can_format_runtime_failures: bool,
    pub total_blocker_component_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeBoundaryCommitStage {
    RequestAcceptance,
    ResponseAcceptance,
    BoundaryAcceptance,
    BoundaryEnvelope,
    BoundaryAdapter,
    BoundaryKv,
    BoundaryGate,
    RuntimeResponse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeBoundaryCommitAction {
    CommitBoundary,
    ReturnRuntimeFailure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeManifestBoundaryCommitStage {
    BoundaryCommit,
    ManifestBoundaryKv,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeManifestBoundaryCommitProblemKind {
    BoundaryCommit,
    RequestManifestPlanning,
    ResponseManifestKv,
    ComponentAccounting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeManifestBoundaryCommitAction {
    CommitManifestBoundary,
    ReturnRuntimeFailure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeKvSideEffectStage {
    RuntimeKvImport,
    ManifestBoundaryCommit,
    RuntimeKvExport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeKvSideEffectProblemKind {
    RuntimeKvImport,
    BoundaryCommit,
    RequestManifestPlanning,
    ResponseManifestKv,
    ManifestBoundaryAccounting,
    RuntimeKvExport,
    ComponentAccounting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeKvSideEffectCommitAction {
    CommitSideEffects,
    ReturnRuntimeFailure,
}

impl RuntimeManifestBoundaryCommitProblemKind {
    pub fn failure_kind(self) -> RuntimeFailureKind {
        RuntimeFailureKind::ContractViolation
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::BoundaryCommit => "boundary_commit",
            Self::RequestManifestPlanning => "request_manifest_planning",
            Self::ResponseManifestKv => "response_manifest_kv",
            Self::ComponentAccounting => "manifest_boundary_commit_accounting",
        }
    }

    pub fn failure_message(self, component_count: usize) -> String {
        format!(
            "{} blocked runtime manifest boundary commit with {} problem component(s)",
            self.label(),
            component_count
        )
    }
}

impl RuntimeBoundaryCommitStage {
    pub fn failure_kind(self) -> RuntimeFailureKind {
        match self {
            Self::RuntimeResponse => RuntimeFailureKind::Runtime,
            Self::RequestAcceptance
            | Self::ResponseAcceptance
            | Self::BoundaryAcceptance
            | Self::BoundaryEnvelope
            | Self::BoundaryAdapter
            | Self::BoundaryKv
            | Self::BoundaryGate => RuntimeFailureKind::ContractViolation,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::RequestAcceptance => "request_acceptance",
            Self::ResponseAcceptance => "response_acceptance",
            Self::BoundaryAcceptance => "boundary_acceptance",
            Self::BoundaryEnvelope => "boundary_envelope",
            Self::BoundaryAdapter => "boundary_adapter",
            Self::BoundaryKv => "boundary_kv",
            Self::BoundaryGate => "boundary_gate",
            Self::RuntimeResponse => "runtime_response",
        }
    }

    pub fn failure_message(self, component_count: usize) -> String {
        format!(
            "{} blocked runtime boundary commit with {} problem component(s)",
            self.label(),
            component_count
        )
    }
}

impl RuntimeBoundaryCommitAction {
    pub fn can_commit(self) -> bool {
        matches!(self, Self::CommitBoundary)
    }

    pub fn should_return_failure(self) -> bool {
        matches!(self, Self::ReturnRuntimeFailure)
    }
}

impl RuntimeBoundaryDeviceExecutionCommitAction {
    pub fn can_commit(self) -> bool {
        matches!(self, Self::CommitRuntimeBoundaryDeviceExecution)
    }

    pub fn should_wait_for_runtime_reported_metadata(self) -> bool {
        matches!(self, Self::WaitForRuntimeReportedDeviceExecutionMetadata)
    }

    pub fn should_repair_device_execution_envelope(self) -> bool {
        matches!(self, Self::RepairDeviceExecutionEnvelope)
    }
}

impl RuntimeManifestBoundaryCommitAction {
    pub fn can_commit(self) -> bool {
        matches!(self, Self::CommitManifestBoundary)
    }

    pub fn should_return_failure(self) -> bool {
        matches!(self, Self::ReturnRuntimeFailure)
    }
}

impl RuntimeKvSideEffectProblemKind {
    pub fn failure_kind(self) -> RuntimeFailureKind {
        match self {
            Self::RuntimeKvImport => RuntimeFailureKind::KvImport,
            Self::RuntimeKvExport => RuntimeFailureKind::KvExport,
            Self::BoundaryCommit
            | Self::RequestManifestPlanning
            | Self::ResponseManifestKv
            | Self::ManifestBoundaryAccounting
            | Self::ComponentAccounting => RuntimeFailureKind::ContractViolation,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::RuntimeKvImport => "runtime_kv_import",
            Self::BoundaryCommit => "boundary_commit",
            Self::RequestManifestPlanning => "request_manifest_planning",
            Self::ResponseManifestKv => "response_manifest_kv",
            Self::ManifestBoundaryAccounting => "manifest_boundary_accounting",
            Self::RuntimeKvExport => "runtime_kv_export",
            Self::ComponentAccounting => "runtime_kv_side_effect_accounting",
        }
    }

    pub fn failure_message(self, component_count: usize) -> String {
        format!(
            "{} blocked runtime KV side effects with {} problem component(s)",
            self.label(),
            component_count
        )
    }
}

impl RuntimeKvSideEffectCommitAction {
    pub fn can_commit(self) -> bool {
        matches!(self, Self::CommitSideEffects)
    }

    pub fn should_return_failure(self) -> bool {
        matches!(self, Self::ReturnRuntimeFailure)
    }
}

impl RuntimeBoundaryEnvelopeSummary {
    pub fn response_exceeds_request_tokens(self) -> bool {
        self.response.token_count > self.request.max_tokens
    }

    pub fn response_imported_kv_matches_request(self) -> bool {
        self.response.imported_kv_blocks == self.request.imported_kv_blocks
    }

    pub fn response_kv_counts_match_diagnostics(self) -> bool {
        self.response.kv_counts_match_diagnostics()
    }

    pub fn response_has_token_uncertainty(self) -> bool {
        self.response.has_token_uncertainty()
    }

    pub fn response_uncertainty_coverage_signal_component_count(self) -> usize {
        self.response
            .token_uncertainty_coverage_signal_component_count()
    }

    pub fn response_has_uncertainty_coverage_signals(self) -> bool {
        self.response.has_token_uncertainty_coverage_signals()
    }

    pub fn response_uncertainty_metric_problem_component_count(self) -> usize {
        self.response
            .token_uncertainty_metric_problem_component_count()
    }

    pub fn response_has_uncertainty_metric_problem_components(self) -> bool {
        self.response
            .has_token_uncertainty_metric_problem_components()
    }

    pub fn response_uncertainty_accounting_is_consistent(self) -> bool {
        self.response.token_uncertainty_accounting_is_consistent()
    }

    pub fn has_runtime_execution_signal(self) -> bool {
        self.response.has_runtime_execution_signal
    }

    pub fn has_request_adapter_candidate(self) -> bool {
        self.request.has_adapter_candidates()
    }

    pub fn request_was_context_limited(self) -> bool {
        self.request.context_limited_generation()
    }

    pub fn response_token_drift_component_count(self) -> usize {
        usize::from(self.response_exceeds_request_tokens())
    }

    pub fn imported_kv_drift_component_count(self) -> usize {
        usize::from(!self.response_imported_kv_matches_request())
    }

    pub fn diagnostics_kv_drift_component_count(self) -> usize {
        usize::from(!self.response_kv_counts_match_diagnostics())
    }

    pub fn runtime_execution_signal_missing_component_count(self) -> usize {
        usize::from(!self.has_runtime_execution_signal())
    }

    pub fn request_adapter_signal_missing_component_count(self) -> usize {
        usize::from(!self.has_request_adapter_candidate())
    }

    pub fn context_pressure_signal_component_count(self) -> usize {
        usize::from(self.request_was_context_limited())
    }

    pub fn boundary_shape_drift_component_count(self) -> usize {
        self.response_token_drift_component_count()
            .saturating_add(self.imported_kv_drift_component_count())
            .saturating_add(self.diagnostics_kv_drift_component_count())
    }

    pub fn boundary_envelope_signal_component_count(self) -> usize {
        self.boundary_shape_drift_component_count()
            .saturating_add(self.runtime_execution_signal_missing_component_count())
            .saturating_add(self.request_adapter_signal_missing_component_count())
            .saturating_add(self.context_pressure_signal_component_count())
            .saturating_add(self.response_uncertainty_coverage_signal_component_count())
    }

    pub fn boundary_shape_is_consistent(self) -> bool {
        !self.response_exceeds_request_tokens()
            && self.response_imported_kv_matches_request()
            && self.response_kv_counts_match_diagnostics()
    }

    pub fn boundary_envelope_is_consistent(self) -> bool {
        self.boundary_shape_is_consistent() && self.response_uncertainty_accounting_is_consistent()
    }

    pub fn boundary_envelope_shape_is_clean(self) -> bool {
        self.boundary_envelope_commit_is_clean()
    }

    pub fn can_use_runtime_boundary_envelope(self) -> bool {
        self.can_commit_runtime_boundary_envelope()
    }

    pub fn runtime_boundary_envelope_commit_signal_component_count(self) -> usize {
        self.request
            .request_envelope_commit_signal_component_count()
            .saturating_add(
                self.response
                    .runtime_response_envelope_commit_signal_component_count(),
            )
            .saturating_add(self.boundary_envelope_signal_component_count())
    }

    pub fn has_runtime_boundary_envelope_commit_signals(self) -> bool {
        self.runtime_boundary_envelope_commit_signal_component_count() > 0
    }

    pub fn runtime_boundary_envelope_commit_blocker_component_count(self) -> usize {
        self.request
            .request_envelope_commit_blocker_component_count()
            .saturating_add(
                self.response
                    .runtime_response_envelope_commit_blocker_component_count(),
            )
            .saturating_add(self.boundary_shape_drift_component_count())
            .saturating_add(usize::from(
                self.request.request_envelope_commit_is_clean()
                    && !self.request.can_commit_runtime_request_envelope(),
            ))
            .saturating_add(usize::from(
                self.response.runtime_response_envelope_commit_is_clean()
                    && !self.response.can_commit_runtime_response_envelope(),
            ))
    }

    pub fn has_runtime_boundary_envelope_commit_blockers(self) -> bool {
        self.runtime_boundary_envelope_commit_blocker_component_count() > 0
    }

    pub fn runtime_boundary_envelope_commit_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .request
            .request_envelope_commit_signal_component_count()
            .saturating_add(
                self.response
                    .runtime_response_envelope_commit_signal_component_count(),
            )
            .saturating_add(self.boundary_envelope_signal_component_count());
        let expected_blocker_count = self
            .request
            .request_envelope_commit_blocker_component_count()
            .saturating_add(
                self.response
                    .runtime_response_envelope_commit_blocker_component_count(),
            )
            .saturating_add(self.boundary_shape_drift_component_count())
            .saturating_add(usize::from(
                self.request.request_envelope_commit_is_clean()
                    && !self.request.can_commit_runtime_request_envelope(),
            ))
            .saturating_add(usize::from(
                self.response.runtime_response_envelope_commit_is_clean()
                    && !self.response.can_commit_runtime_response_envelope(),
            ));

        self.request
            .request_envelope_commit_accounting_is_consistent()
            && self
                .response
                .runtime_response_envelope_commit_accounting_is_consistent()
            && self.runtime_boundary_envelope_commit_signal_component_count()
                == expected_signal_count
            && self.has_runtime_boundary_envelope_commit_signals() == (expected_signal_count > 0)
            && self.runtime_boundary_envelope_commit_blocker_component_count()
                == expected_blocker_count
            && self.has_runtime_boundary_envelope_commit_blockers() == (expected_blocker_count > 0)
    }

    pub fn boundary_envelope_commit_is_clean(self) -> bool {
        self.runtime_boundary_envelope_commit_blocker_component_count() == 0
            && self.runtime_boundary_envelope_commit_accounting_is_consistent()
    }

    pub fn can_commit_runtime_boundary_envelope(self) -> bool {
        self.boundary_envelope_commit_is_clean()
            && self.request.can_commit_runtime_request_envelope()
            && self.response.can_commit_runtime_response_envelope()
            && self.boundary_envelope_is_consistent()
    }
}

impl RuntimeBoundaryAdapterSummary {
    pub fn request_adapter_missing(self) -> bool {
        !self.request_adapter_reported
    }

    pub fn runtime_adapter_missing(self) -> bool {
        !self.runtime_adapter_reported
    }

    pub fn runtime_adapter_drifted_from_request(self) -> bool {
        self.request_adapter_reported
            && self.runtime_adapter_reported
            && !self.runtime_adapter_matches_request
    }

    pub fn runtime_adapter_outside_execution_context(self) -> bool {
        self.runtime_adapter_reported && !self.runtime_adapter_allowed
    }

    pub fn runtime_adapter_problem(self) -> bool {
        self.request_adapter_missing()
            || self.runtime_adapter_missing()
            || self.runtime_adapter_drifted_from_request()
            || self.runtime_adapter_outside_execution_context()
            || self
                .selection
                .map(|selection| selection.runtime_adapter_problem())
                .unwrap_or(false)
    }

    pub fn request_adapter_boundary_signal_component_count(self) -> usize {
        usize::from(self.request_adapter_reported) + usize::from(self.adapter_candidate_count > 0)
    }

    pub fn runtime_adapter_boundary_signal_component_count(self) -> usize {
        usize::from(self.runtime_adapter_reported)
            + usize::from(self.runtime_adapter_matches_request)
            + usize::from(self.runtime_adapter_allowed)
    }

    pub fn planning_selection_boundary_signal_component_count(self) -> usize {
        usize::from(self.has_planning_selection) + usize::from(self.runtime_selection_confirmed())
    }

    pub fn adapter_boundary_signal_component_count(self) -> usize {
        self.request_adapter_boundary_signal_component_count()
            .saturating_add(self.runtime_adapter_boundary_signal_component_count())
            .saturating_add(self.planning_selection_boundary_signal_component_count())
    }

    pub fn has_adapter_boundary_signals(self) -> bool {
        self.adapter_boundary_signal_component_count() > 0
    }

    pub fn request_adapter_problem_component_count(self) -> usize {
        usize::from(self.request_adapter_missing())
    }

    pub fn runtime_adapter_problem_component_count(self) -> usize {
        usize::from(self.runtime_adapter_missing())
            + usize::from(self.runtime_adapter_drifted_from_request())
            + usize::from(self.runtime_adapter_outside_execution_context())
    }

    pub fn planning_selection_problem_component_count(self) -> usize {
        self.selection
            .map(|selection| selection.runtime_adapter_problem_component_count())
            .unwrap_or(0)
    }

    pub fn adapter_boundary_problem_component_count(self) -> usize {
        self.request_adapter_problem_component_count()
            .saturating_add(self.runtime_adapter_problem_component_count())
            .saturating_add(self.planning_selection_problem_component_count())
    }

    pub fn has_adapter_boundary_problem_components(self) -> bool {
        self.adapter_boundary_problem_component_count() > 0
    }

    pub fn runtime_selection_confirmed(self) -> bool {
        self.selection
            .map(|selection| selection.runtime_selection_confirmed())
            .unwrap_or(
                self.request_adapter_reported
                    && self.runtime_adapter_reported
                    && self.runtime_adapter_matches_request
                    && self.runtime_adapter_allowed,
            )
    }

    pub fn adapter_boundary_is_consistent(self) -> bool {
        !self.request_adapter_missing()
            && !self.runtime_adapter_missing()
            && !self.runtime_adapter_drifted_from_request()
            && !self.runtime_adapter_outside_execution_context()
            && self
                .selection
                .map(|selection| selection.fallback_has_reason())
                .unwrap_or(true)
    }

    pub fn adapter_boundary_shape_is_clean(self) -> bool {
        !self.has_adapter_boundary_problem_components() && self.adapter_boundary_is_consistent()
    }

    pub fn adapter_boundary_commit_signal_component_count(self) -> usize {
        self.adapter_boundary_signal_component_count()
    }

    pub fn adapter_boundary_commit_blocker_component_count(self) -> usize {
        self.adapter_boundary_problem_component_count()
    }

    pub fn adapter_boundary_commit_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .request_adapter_boundary_signal_component_count()
            .saturating_add(self.runtime_adapter_boundary_signal_component_count())
            .saturating_add(self.planning_selection_boundary_signal_component_count());
        let expected_blocker_count = self
            .request_adapter_problem_component_count()
            .saturating_add(self.runtime_adapter_problem_component_count())
            .saturating_add(self.planning_selection_problem_component_count());

        self.adapter_boundary_signal_component_count() == expected_signal_count
            && self.has_adapter_boundary_signals() == (expected_signal_count > 0)
            && self.adapter_boundary_commit_signal_component_count() == expected_signal_count
            && self.adapter_boundary_problem_component_count() == expected_blocker_count
            && self.has_adapter_boundary_problem_components() == (expected_blocker_count > 0)
            && self.adapter_boundary_commit_blocker_component_count() == expected_blocker_count
            && self.runtime_adapter_problem() == (expected_blocker_count > 0)
    }

    pub fn adapter_boundary_commit_is_clean(self) -> bool {
        self.adapter_boundary_commit_blocker_component_count() == 0
            && self.adapter_boundary_commit_accounting_is_consistent()
    }

    pub fn can_commit_runtime_boundary_adapter(self) -> bool {
        self.runtime_selection_confirmed() && self.adapter_boundary_commit_is_clean()
    }

    pub fn can_use_runtime_boundary_adapter(self) -> bool {
        self.can_commit_runtime_boundary_adapter()
    }
}

impl RuntimeBoundaryDeviceExecutionReadinessSummary {
    pub fn new(
        runtime_reported_metadata_ready: bool,
        device_execution_envelope: RuntimeDeviceExecutionEnvelopeSummary,
    ) -> Self {
        Self {
            runtime_reported_metadata_ready,
            device_execution_envelope,
            device_execution_envelope_ready: device_execution_envelope
                .can_submit_runtime_device_execution_envelope(),
        }
    }

    pub fn device_execution_signal_component_count(self) -> usize {
        usize::from(self.runtime_reported_metadata_ready).saturating_add(
            self.device_execution_envelope
                .runtime_device_execution_envelope_admission_signal_component_count(),
        )
    }

    pub fn has_device_execution_signals(self) -> bool {
        self.device_execution_signal_component_count() > 0
    }

    pub fn device_execution_blocker_component_count(self) -> usize {
        usize::from(!self.runtime_reported_metadata_ready).saturating_add(
            self.device_execution_envelope
                .runtime_device_execution_envelope_admission_blocker_component_count(),
        )
    }

    pub fn has_device_execution_blockers(self) -> bool {
        self.device_execution_blocker_component_count() > 0
    }

    pub fn device_execution_readiness_accounting_is_consistent(self) -> bool {
        let expected_signal_count = usize::from(self.runtime_reported_metadata_ready)
            .saturating_add(
                self.device_execution_envelope
                    .runtime_device_execution_envelope_admission_signal_component_count(),
            );
        let expected_blocker_count = usize::from(!self.runtime_reported_metadata_ready)
            .saturating_add(
                self.device_execution_envelope
                    .runtime_device_execution_envelope_admission_blocker_component_count(),
            );

        self.device_execution_envelope
            .runtime_device_execution_envelope_admission_accounting_is_consistent()
            && self.device_execution_signal_component_count() == expected_signal_count
            && self.has_device_execution_signals() == (expected_signal_count > 0)
            && self.device_execution_blocker_component_count() == expected_blocker_count
            && self.has_device_execution_blockers() == (expected_blocker_count > 0)
            && self.device_execution_envelope_ready
                == self
                    .device_execution_envelope
                    .can_submit_runtime_device_execution_envelope()
    }

    pub fn device_execution_readiness_is_clean(self) -> bool {
        !self.has_device_execution_blockers()
            && self.device_execution_readiness_accounting_is_consistent()
    }

    pub fn can_commit_runtime_boundary_device_execution(self) -> bool {
        self.runtime_reported_metadata_ready
            && self.device_execution_envelope_ready
            && self.device_execution_readiness_is_clean()
    }

    pub fn runtime_boundary_device_execution_commit_action(
        self,
    ) -> RuntimeBoundaryDeviceExecutionCommitAction {
        if self.can_commit_runtime_boundary_device_execution() {
            RuntimeBoundaryDeviceExecutionCommitAction::CommitRuntimeBoundaryDeviceExecution
        } else if !self.runtime_reported_metadata_ready {
            RuntimeBoundaryDeviceExecutionCommitAction::WaitForRuntimeReportedDeviceExecutionMetadata
        } else {
            RuntimeBoundaryDeviceExecutionCommitAction::RepairDeviceExecutionEnvelope
        }
    }

    pub fn commit_action_matches_readiness(self) -> bool {
        let action = self.runtime_boundary_device_execution_commit_action();
        action.can_commit() == self.can_commit_runtime_boundary_device_execution()
            && action.should_wait_for_runtime_reported_metadata()
                == !self.runtime_reported_metadata_ready
            && action.should_repair_device_execution_envelope()
                == (self.runtime_reported_metadata_ready
                    && !self.can_commit_runtime_boundary_device_execution())
    }

    pub fn commit_summary(self) -> RuntimeBoundaryDeviceExecutionCommitSummary {
        RuntimeBoundaryDeviceExecutionCommitSummary::new(self)
    }
}

impl RuntimeBoundaryDeviceExecutionCommitSummary {
    pub fn new(readiness: RuntimeBoundaryDeviceExecutionReadinessSummary) -> Self {
        let action = readiness.runtime_boundary_device_execution_commit_action();

        Self {
            readiness,
            action,
            can_commit: readiness.can_commit_runtime_boundary_device_execution(),
            should_wait_for_runtime_reported_metadata: action
                .should_wait_for_runtime_reported_metadata(),
            should_repair_device_execution_envelope: action
                .should_repair_device_execution_envelope(),
            total_signal_component_count: readiness.device_execution_signal_component_count(),
            total_blocker_component_count: readiness.device_execution_blocker_component_count(),
            component_accounting_consistent: readiness
                .device_execution_readiness_accounting_is_consistent(),
        }
    }

    pub fn action_can_commit(self) -> bool {
        self.action.can_commit()
    }

    pub fn action_should_wait_for_runtime_reported_metadata(self) -> bool {
        self.action.should_wait_for_runtime_reported_metadata()
    }

    pub fn action_should_repair_device_execution_envelope(self) -> bool {
        self.action.should_repair_device_execution_envelope()
    }

    pub fn has_blocker_components(self) -> bool {
        self.total_blocker_component_count > 0
    }

    pub fn commit_decision_accounting_is_consistent(self) -> bool {
        self.can_commit
            == self
                .readiness
                .can_commit_runtime_boundary_device_execution()
            && self.action
                == self
                    .readiness
                    .runtime_boundary_device_execution_commit_action()
            && self.action_can_commit() == self.can_commit
            && self.should_wait_for_runtime_reported_metadata
                == self.action_should_wait_for_runtime_reported_metadata()
            && self.should_repair_device_execution_envelope
                == self.action_should_repair_device_execution_envelope()
            && self.total_signal_component_count
                == self.readiness.device_execution_signal_component_count()
            && self.total_blocker_component_count
                == self.readiness.device_execution_blocker_component_count()
            && self.has_blocker_components() == self.readiness.has_device_execution_blockers()
            && self.component_accounting_consistent
                == self
                    .readiness
                    .device_execution_readiness_accounting_is_consistent()
            && self.readiness.commit_action_matches_readiness()
    }
}

impl RuntimeBoundaryKvSummary {
    pub fn concrete_imports_match_request(self) -> bool {
        self.concrete_imported_kv_blocks == self.request_imported_kv_blocks
    }

    pub fn concrete_import_count_drifted(self) -> bool {
        !self.concrete_imports_match_request()
    }

    pub fn response_imports_match_request(self) -> bool {
        self.response_imported_kv_blocks == self.request_imported_kv_blocks
    }

    pub fn response_import_count_drifted(self) -> bool {
        !self.response_imports_match_request()
    }

    pub fn diagnostics_match_response(self) -> bool {
        self.response_imported_kv_blocks == self.diagnostics_imported_kv_blocks
            && self.response_exported_kv_blocks == self.diagnostics_exported_kv_blocks
    }

    pub fn diagnostics_count_drifted(self) -> bool {
        !self.diagnostics_match_response()
    }

    pub fn imports_within_planning(self) -> bool {
        self.planned_imported_kv_blocks
            .map(|planned| {
                self.request_imported_kv_blocks == planned
                    && self.concrete_imported_kv_blocks == planned
                    && self.response_imported_kv_blocks <= planned
            })
            .unwrap_or(true)
    }

    pub fn exports_within_runtime(self) -> bool {
        if self.runtime_export_enabled {
            self.runtime_max_export_blocks == 0
                || self.response_exported_kv_blocks <= self.runtime_max_export_blocks
        } else {
            self.response_exported_kv_blocks == 0
        }
    }

    pub fn exports_within_planning(self) -> bool {
        self.planned_exported_kv_blocks
            .map(|planned| self.response_exported_kv_blocks <= planned)
            .unwrap_or(true)
    }

    pub fn namespaces_are_runtime_exchange(self) -> bool {
        self.imported_namespace_counts.non_runtime_total() == 0
            && self.exported_namespace_counts.non_runtime_total() == 0
    }

    pub fn has_kv_violations(self) -> bool {
        self.imported_kv_violation_count > 0 || self.exported_kv_violation_count > 0
    }

    pub fn imported_kv_activity_signal_component_count(self) -> usize {
        usize::from(self.request_imported_kv_blocks > 0)
            .saturating_add(usize::from(self.concrete_imported_kv_blocks > 0))
            .saturating_add(usize::from(self.accepted_imported_kv_blocks > 0))
            .saturating_add(usize::from(self.response_imported_kv_blocks > 0))
    }

    pub fn exported_kv_activity_signal_component_count(self) -> usize {
        usize::from(self.response_exported_kv_blocks > 0)
            .saturating_add(usize::from(self.accepted_exported_kv_blocks > 0))
    }

    pub fn diagnostics_kv_activity_signal_component_count(self) -> usize {
        usize::from(self.diagnostics_imported_kv_blocks > 0)
            .saturating_add(usize::from(self.diagnostics_exported_kv_blocks > 0))
            .saturating_add(usize::from(
                self.diagnostics_weak_runtime_kv_imports_skipped > 0,
            ))
    }

    pub fn runtime_kv_capability_signal_component_count(self) -> usize {
        usize::from(self.runtime_import_enabled)
            .saturating_add(usize::from(self.runtime_export_enabled))
            .saturating_add(usize::from(self.runtime_max_import_blocks > 0))
            .saturating_add(usize::from(self.runtime_max_export_blocks > 0))
    }

    pub fn planning_kv_boundary_signal_component_count(self) -> usize {
        usize::from(
            self.planned_imported_kv_blocks
                .map(|planned| planned > 0)
                .unwrap_or(false),
        )
        .saturating_add(usize::from(
            self.planned_exported_kv_blocks
                .map(|planned| planned > 0)
                .unwrap_or(false),
        ))
    }

    pub fn namespace_kv_activity_signal_component_count(self) -> usize {
        self.imported_namespace_counts
            .namespace_boundary_signal_component_count()
            .saturating_add(
                self.exported_namespace_counts
                    .namespace_boundary_signal_component_count(),
            )
    }

    pub fn kv_boundary_signal_component_count(self) -> usize {
        self.imported_kv_activity_signal_component_count()
            .saturating_add(self.exported_kv_activity_signal_component_count())
            .saturating_add(self.diagnostics_kv_activity_signal_component_count())
            .saturating_add(self.runtime_kv_capability_signal_component_count())
            .saturating_add(self.planning_kv_boundary_signal_component_count())
            .saturating_add(self.namespace_kv_activity_signal_component_count())
    }

    pub fn has_kv_boundary_signals(self) -> bool {
        self.kv_boundary_signal_component_count() > 0
    }

    pub fn runtime_exchange_count_drift_component_count(self) -> usize {
        usize::from(self.concrete_import_count_drifted())
            + usize::from(self.response_import_count_drifted())
            + usize::from(self.diagnostics_count_drifted())
    }

    pub fn planning_bound_drift_component_count(self) -> usize {
        usize::from(!self.imports_within_planning()) + usize::from(!self.exports_within_planning())
    }

    pub fn runtime_bound_drift_component_count(self) -> usize {
        usize::from(!self.exports_within_runtime())
    }

    pub fn namespace_drift_component_count(self) -> usize {
        usize::from(!self.namespaces_are_runtime_exchange())
    }

    pub fn validation_failure_component_count(self) -> usize {
        usize::from(self.imported_kv_violation_count > 0)
            + usize::from(self.exported_kv_violation_count > 0)
    }

    pub fn kv_boundary_problem_component_count(self) -> usize {
        self.runtime_exchange_count_drift_component_count()
            .saturating_add(self.planning_bound_drift_component_count())
            .saturating_add(self.runtime_bound_drift_component_count())
            .saturating_add(self.namespace_drift_component_count())
            .saturating_add(self.validation_failure_component_count())
    }

    pub fn has_kv_boundary_problem_components(self) -> bool {
        self.kv_boundary_problem_component_count() > 0
    }

    pub fn kv_boundary_is_consistent(self) -> bool {
        self.concrete_imports_match_request()
            && self.response_imports_match_request()
            && self.diagnostics_match_response()
            && self.imports_within_planning()
            && self.exports_within_runtime()
            && self.exports_within_planning()
            && self.namespaces_are_runtime_exchange()
            && !self.has_kv_violations()
    }

    pub fn kv_boundary_shape_is_clean(self) -> bool {
        !self.has_kv_boundary_problem_components() && self.kv_boundary_is_consistent()
    }

    pub fn can_use_runtime_boundary_kv(self) -> bool {
        self.kv_boundary_shape_is_clean()
    }
}

impl RuntimeManifestBoundaryKvSummary {
    pub fn new(
        request_manifest_planning: RuntimeRequestManifestPlanningReadinessSummary,
        response_manifest_kv: RuntimeResponseManifestKvSummary,
    ) -> Self {
        Self {
            request_manifest_planning,
            response_manifest_kv,
            request_manifest_planning_signal_component_count: request_manifest_planning
                .manifest_request_planning_signal_component_count(),
            response_manifest_kv_signal_component_count: response_manifest_kv
                .response_manifest_kv_signal_component_count(),
            request_manifest_planning_blocker_component_count: request_manifest_planning
                .manifest_request_planning_blocker_component_count(),
            response_manifest_kv_blocker_component_count: response_manifest_kv
                .response_manifest_kv_blocker_component_count(),
        }
    }

    pub fn stage_order() -> [RuntimeManifestBoundaryKvStage; 2] {
        [
            RuntimeManifestBoundaryKvStage::RequestManifestPlanning,
            RuntimeManifestBoundaryKvStage::ResponseManifestKv,
        ]
    }

    pub fn request_manifest_planning_ready(self) -> bool {
        self.request_manifest_planning
            .can_commit_manifest_request_planning()
    }

    pub fn response_manifest_kv_ready(self) -> bool {
        self.response_manifest_kv.can_commit_response_manifest_kv()
    }

    pub fn stage_ready(self, stage: RuntimeManifestBoundaryKvStage) -> bool {
        match stage {
            RuntimeManifestBoundaryKvStage::RequestManifestPlanning => {
                self.request_manifest_planning_ready()
            }
            RuntimeManifestBoundaryKvStage::ResponseManifestKv => self.response_manifest_kv_ready(),
        }
    }

    pub fn stage_signal_component_count(self, stage: RuntimeManifestBoundaryKvStage) -> usize {
        match stage {
            RuntimeManifestBoundaryKvStage::RequestManifestPlanning => {
                self.request_manifest_planning_signal_component_count
            }
            RuntimeManifestBoundaryKvStage::ResponseManifestKv => {
                self.response_manifest_kv_signal_component_count
            }
        }
    }

    pub fn stage_blocker_component_count(self, stage: RuntimeManifestBoundaryKvStage) -> usize {
        match stage {
            RuntimeManifestBoundaryKvStage::RequestManifestPlanning => {
                self.request_manifest_planning_blocker_component_count
            }
            RuntimeManifestBoundaryKvStage::ResponseManifestKv => {
                self.response_manifest_kv_blocker_component_count
            }
        }
    }

    pub fn first_unready_stage(self) -> Option<RuntimeManifestBoundaryKvStage> {
        Self::stage_order()
            .into_iter()
            .find(|stage| !self.stage_ready(*stage))
    }

    pub fn first_blocking_stage(self) -> Option<RuntimeManifestBoundaryKvStage> {
        Self::stage_order()
            .into_iter()
            .find(|stage| self.stage_blocker_component_count(*stage) > 0)
    }

    pub fn manifest_boundary_kv_signal_component_count(self) -> usize {
        self.request_manifest_planning_signal_component_count
            .saturating_add(self.response_manifest_kv_signal_component_count)
    }

    pub fn has_manifest_boundary_kv_signals(self) -> bool {
        self.manifest_boundary_kv_signal_component_count() > 0
    }

    pub fn manifest_boundary_kv_blocker_component_count(self) -> usize {
        self.request_manifest_planning_blocker_component_count
            .saturating_add(self.response_manifest_kv_blocker_component_count)
    }

    pub fn has_manifest_boundary_kv_blockers(self) -> bool {
        self.manifest_boundary_kv_blocker_component_count() > 0
    }

    pub fn manifest_boundary_kv_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .request_manifest_planning_signal_component_count
            .saturating_add(self.response_manifest_kv_signal_component_count);
        let expected_blocker_count = self
            .request_manifest_planning_blocker_component_count
            .saturating_add(self.response_manifest_kv_blocker_component_count);

        self.request_manifest_planning
            .manifest_request_planning_accounting_is_consistent()
            && self
                .response_manifest_kv
                .response_manifest_kv_accounting_is_consistent()
            && self.manifest_boundary_kv_signal_component_count() == expected_signal_count
            && self.has_manifest_boundary_kv_signals() == (expected_signal_count > 0)
            && self.manifest_boundary_kv_blocker_component_count() == expected_blocker_count
            && self.has_manifest_boundary_kv_blockers() == (expected_blocker_count > 0)
    }

    pub fn manifest_boundary_kv_is_clean(self) -> bool {
        !self.has_manifest_boundary_kv_blockers()
            && self.manifest_boundary_kv_accounting_is_consistent()
    }

    pub fn can_commit_manifest_boundary_kv(self) -> bool {
        self.manifest_boundary_kv_is_clean()
            && self.request_manifest_planning_ready()
            && self.response_manifest_kv_ready()
    }
}

impl RuntimeBoundaryGateSummary {
    pub fn request_acceptance_failed(self) -> bool {
        !self.request_accepted
    }

    pub fn response_acceptance_failed(self) -> bool {
        !self.response_accepted
    }

    pub fn has_acceptance_failures(self) -> bool {
        self.request_acceptance_failed() || self.response_acceptance_failed()
    }

    pub fn envelope_drifted(self) -> bool {
        !self.envelope_consistent || !self.response_uncertainty_accounting_is_consistent()
    }

    pub fn adapter_drifted(self) -> bool {
        !self.adapter_consistent
    }

    pub fn kv_drifted(self) -> bool {
        !self.kv_consistent
    }

    pub fn has_response_wire_problem_components(self) -> bool {
        self.response_wire_problem_count > 0
    }

    pub fn has_request_backend_wire_problem_components(self) -> bool {
        self.request_backend_wire_problem_count > 0
    }

    pub fn has_planning_pre_request_gate_problems(self) -> bool {
        self.planning_pre_request_problem_count > 0
    }

    pub fn has_request_planning_pre_request_gate_problems(self) -> bool {
        self.request_planning_pre_request_problem_count > 0
    }

    pub fn has_planning_pressure_signals(self) -> bool {
        self.planning_pressure_signal_count > 0
    }

    pub fn has_request_planning_pressure_signals(self) -> bool {
        self.request_planning_pressure_signal_count > 0
    }

    pub fn has_request_planning_dense_compute_savings(self) -> bool {
        self.request_planning_dense_compute_avoided_tokens > 0
    }

    pub fn has_response_planning_dense_compute_savings(self) -> bool {
        self.response_planning_dense_compute_avoided_tokens > 0
    }

    pub fn has_planning_dense_compute_savings(self) -> bool {
        self.planning_dense_compute_avoided_tokens() > 0
    }

    pub fn planning_dense_compute_avoided_tokens(self) -> usize {
        self.request_planning_dense_compute_avoided_tokens
            .max(self.response_planning_dense_compute_avoided_tokens)
    }

    pub fn response_wire_problem_component_count(self) -> usize {
        self.response_wire_problem_count
    }

    pub fn request_backend_wire_problem_component_count(self) -> usize {
        self.request_backend_wire_problem_count
    }

    pub fn direct_response_wire_problem_component_count(self) -> usize {
        self.response_wire_problem_count
            .saturating_sub(self.planning_pre_request_problem_count)
    }

    pub fn direct_request_backend_wire_problem_component_count(self) -> usize {
        self.request_backend_wire_problem_count
            .saturating_sub(self.request_planning_pre_request_problem_count)
    }

    pub fn planning_pre_request_gate_problem_component_count(self) -> usize {
        usize::from(self.has_planning_pre_request_gate_problems())
    }

    pub fn request_planning_pre_request_gate_problem_component_count(self) -> usize {
        usize::from(self.has_request_planning_pre_request_gate_problems())
    }

    pub fn planning_pressure_signal_component_count(self) -> usize {
        usize::from(self.has_planning_pressure_signals())
    }

    pub fn request_planning_pressure_signal_component_count(self) -> usize {
        usize::from(self.has_request_planning_pressure_signals())
    }

    pub fn kv_boundary_signal_component_count(self) -> usize {
        self.kv_boundary_signal_count
    }

    pub fn has_kv_boundary_signals(self) -> bool {
        self.kv_boundary_signal_count > 0
    }

    pub fn response_uncertainty_coverage_signal_component_count(self) -> usize {
        self.response_uncertainty_coverage_signal_count
    }

    pub fn has_response_uncertainty_coverage_signals(self) -> bool {
        self.response_uncertainty_coverage_signal_count > 0
    }

    pub fn commit_gate_signal_component_count(self) -> usize {
        self.request_planning_pressure_signal_count
            .saturating_add(self.planning_pressure_signal_count)
            .saturating_add(self.kv_boundary_signal_count)
            .saturating_add(self.response_uncertainty_coverage_signal_count)
    }

    pub fn commit_gate_has_signal_components(self) -> bool {
        self.commit_gate_signal_component_count() > 0
    }

    pub fn response_uncertainty_metric_problem_component_count(self) -> usize {
        self.response_uncertainty_metric_problem_count
    }

    pub fn has_response_uncertainty_metric_problem_components(self) -> bool {
        self.response_uncertainty_metric_problem_count > 0
    }

    pub fn response_uncertainty_accounting_is_consistent(self) -> bool {
        self.response_uncertainty_accounting_consistent
            && !self.has_response_uncertainty_metric_problem_components()
    }

    pub fn total_wire_problem_component_count(self) -> usize {
        self.request_backend_wire_problem_component_count()
            .saturating_add(self.response_wire_problem_component_count())
    }

    pub fn has_wire_problem_components(self) -> bool {
        self.total_wire_problem_component_count() > 0
    }

    pub fn response_wire_accounting_is_consistent(self) -> bool {
        self.response_wire_problem_count >= self.planning_pre_request_problem_count
    }

    pub fn request_backend_wire_accounting_is_consistent(self) -> bool {
        self.request_backend_wire_problem_count >= self.request_planning_pre_request_problem_count
    }

    pub fn wire_accounting_is_consistent(self) -> bool {
        self.request_backend_wire_accounting_is_consistent()
            && self.response_wire_accounting_is_consistent()
            && self.has_wire_problem_components() == (self.total_wire_problem_component_count() > 0)
    }

    pub fn has_boundary_drift(self) -> bool {
        self.envelope_drifted() || self.adapter_drifted() || self.kv_drifted()
    }

    pub fn has_failure_reports(self) -> bool {
        self.total_failure_report_count > 0
    }

    pub fn has_total_violations(self) -> bool {
        self.total_violation_count > 0
    }

    pub fn request_acceptance_blocker_component_count(self) -> usize {
        usize::from(self.request_acceptance_failed())
    }

    pub fn response_acceptance_blocker_component_count(self) -> usize {
        usize::from(self.response_acceptance_failed())
    }

    pub fn acceptance_failure_component_count(self) -> usize {
        self.request_acceptance_blocker_component_count()
            + self.response_acceptance_blocker_component_count()
    }

    pub fn envelope_blocker_component_count(self) -> usize {
        usize::from(self.envelope_drifted())
    }

    pub fn adapter_blocker_component_count(self) -> usize {
        usize::from(self.adapter_drifted())
    }

    pub fn kv_blocker_component_count(self) -> usize {
        usize::from(self.kv_drifted())
    }

    pub fn boundary_drift_component_count(self) -> usize {
        self.envelope_blocker_component_count()
            + self.adapter_blocker_component_count()
            + self.kv_blocker_component_count()
    }

    pub fn mapped_failure_report_component_count(self) -> usize {
        usize::from(self.has_failure_reports())
    }

    pub fn commit_blocker_component_count(self) -> usize {
        self.acceptance_failure_component_count()
            .saturating_add(self.boundary_drift_component_count())
            .saturating_add(self.mapped_failure_report_component_count())
    }

    pub fn commit_gate_has_problem_components(self) -> bool {
        self.commit_blocker_component_count() > 0
    }

    pub fn commit_gate_accounting_is_consistent(self) -> bool {
        self.commit_blocker_component_count()
            == self
                .acceptance_failure_component_count()
                .saturating_add(self.boundary_drift_component_count())
                .saturating_add(self.mapped_failure_report_component_count())
    }

    pub fn can_commit_response(self) -> bool {
        !self.has_acceptance_failures() && !self.has_boundary_drift()
    }

    pub fn is_clean_commit_gate(self) -> bool {
        self.can_commit_response()
            && self.total_violation_count == 0
            && self.total_failure_report_count == 0
    }

    pub fn boundary_gate_shape_is_clean(self) -> bool {
        self.is_clean_commit_gate()
            && !self.has_wire_problem_components()
            && self.wire_accounting_is_consistent()
            && self.commit_gate_accounting_is_consistent()
            && self.response_uncertainty_accounting_is_consistent()
    }

    pub fn runtime_boundary_commit_signal_component_count(self) -> usize {
        self.commit_gate_signal_component_count()
    }

    pub fn has_runtime_boundary_commit_signals(self) -> bool {
        self.runtime_boundary_commit_signal_component_count() > 0
    }

    pub fn runtime_boundary_commit_blocker_component_count(self) -> usize {
        self.commit_blocker_component_count()
            .saturating_add(self.total_wire_problem_component_count())
    }

    pub fn has_runtime_boundary_commit_blockers(self) -> bool {
        self.runtime_boundary_commit_blocker_component_count() > 0
    }

    pub fn runtime_boundary_commit_accounting_is_consistent(self) -> bool {
        let expected_blocker_count = self
            .commit_blocker_component_count()
            .saturating_add(self.total_wire_problem_component_count());

        self.wire_accounting_is_consistent()
            && self.commit_gate_accounting_is_consistent()
            && self.response_uncertainty_accounting_is_consistent()
            && self.runtime_boundary_commit_signal_component_count()
                == self.commit_gate_signal_component_count()
            && self.has_runtime_boundary_commit_signals()
                == (self.runtime_boundary_commit_signal_component_count() > 0)
            && self.runtime_boundary_commit_blocker_component_count() == expected_blocker_count
            && self.has_runtime_boundary_commit_blockers() == (expected_blocker_count > 0)
    }

    pub fn runtime_boundary_commit_is_clean(self) -> bool {
        !self.has_runtime_boundary_commit_blockers()
            && self.runtime_boundary_commit_accounting_is_consistent()
    }

    pub fn can_commit_runtime_boundary(self) -> bool {
        self.can_commit_response()
            && self.boundary_gate_shape_is_clean()
            && self.runtime_boundary_commit_is_clean()
    }

    pub fn can_commit_runtime_response(self) -> bool {
        self.can_commit_response() && self.boundary_gate_shape_is_clean()
    }
}

impl RuntimeBoundaryCommitReadinessSummary {
    pub fn stage_order() -> [RuntimeBoundaryCommitStage; 8] {
        [
            RuntimeBoundaryCommitStage::RequestAcceptance,
            RuntimeBoundaryCommitStage::ResponseAcceptance,
            RuntimeBoundaryCommitStage::BoundaryAcceptance,
            RuntimeBoundaryCommitStage::BoundaryEnvelope,
            RuntimeBoundaryCommitStage::BoundaryAdapter,
            RuntimeBoundaryCommitStage::BoundaryKv,
            RuntimeBoundaryCommitStage::BoundaryGate,
            RuntimeBoundaryCommitStage::RuntimeResponse,
        ]
    }

    pub fn has_commit_signals(self) -> bool {
        self.total_signal_component_count > 0
    }

    pub fn has_commit_blockers(self) -> bool {
        self.total_blocker_component_count > 0
    }

    pub fn has_planning_dense_compute_savings(self) -> bool {
        self.planning_dense_compute_avoided_tokens > 0
    }

    pub fn all_acceptance_ready(self) -> bool {
        self.request_acceptance_ready
            && self.response_acceptance_ready
            && self.boundary_acceptance_ready
    }

    pub fn all_boundary_summaries_ready(self) -> bool {
        self.boundary_envelope_ready && self.boundary_adapter_ready && self.boundary_kv_ready
    }

    pub fn stage_ready(self, stage: RuntimeBoundaryCommitStage) -> bool {
        match stage {
            RuntimeBoundaryCommitStage::RequestAcceptance => self.request_acceptance_ready,
            RuntimeBoundaryCommitStage::ResponseAcceptance => self.response_acceptance_ready,
            RuntimeBoundaryCommitStage::BoundaryAcceptance => self.boundary_acceptance_ready,
            RuntimeBoundaryCommitStage::BoundaryEnvelope => self.boundary_envelope_ready,
            RuntimeBoundaryCommitStage::BoundaryAdapter => self.boundary_adapter_ready,
            RuntimeBoundaryCommitStage::BoundaryKv => self.boundary_kv_ready,
            RuntimeBoundaryCommitStage::BoundaryGate => self.boundary_gate_ready,
            RuntimeBoundaryCommitStage::RuntimeResponse => self.runtime_response_ready,
        }
    }

    pub fn stage_signal_component_count(self, stage: RuntimeBoundaryCommitStage) -> usize {
        match stage {
            RuntimeBoundaryCommitStage::RequestAcceptance => {
                self.request_acceptance_signal_component_count
            }
            RuntimeBoundaryCommitStage::ResponseAcceptance => {
                self.response_acceptance_signal_component_count
            }
            RuntimeBoundaryCommitStage::BoundaryAcceptance => {
                self.boundary_acceptance_signal_component_count
            }
            RuntimeBoundaryCommitStage::BoundaryEnvelope => {
                self.boundary_envelope_signal_component_count
            }
            RuntimeBoundaryCommitStage::BoundaryAdapter => {
                self.boundary_adapter_signal_component_count
            }
            RuntimeBoundaryCommitStage::BoundaryKv => self.boundary_kv_signal_component_count,
            RuntimeBoundaryCommitStage::BoundaryGate => self.boundary_gate_signal_component_count,
            RuntimeBoundaryCommitStage::RuntimeResponse => {
                self.runtime_response_signal_component_count
            }
        }
    }

    pub fn stage_blocker_component_count(self, stage: RuntimeBoundaryCommitStage) -> usize {
        match stage {
            RuntimeBoundaryCommitStage::RequestAcceptance => {
                self.request_acceptance_blocker_component_count
            }
            RuntimeBoundaryCommitStage::ResponseAcceptance => {
                self.response_acceptance_blocker_component_count
            }
            RuntimeBoundaryCommitStage::BoundaryAcceptance => {
                self.boundary_acceptance_blocker_component_count
            }
            RuntimeBoundaryCommitStage::BoundaryEnvelope => {
                self.boundary_envelope_blocker_component_count
            }
            RuntimeBoundaryCommitStage::BoundaryAdapter => {
                self.boundary_adapter_blocker_component_count
            }
            RuntimeBoundaryCommitStage::BoundaryKv => self.boundary_kv_blocker_component_count,
            RuntimeBoundaryCommitStage::BoundaryGate => self.boundary_gate_blocker_component_count,
            RuntimeBoundaryCommitStage::RuntimeResponse => {
                self.runtime_response_blocker_component_count
            }
        }
    }

    pub fn first_unready_stage(self) -> Option<RuntimeBoundaryCommitStage> {
        Self::stage_order()
            .into_iter()
            .find(|stage| !self.stage_ready(*stage))
    }

    pub fn first_blocking_stage(self) -> Option<RuntimeBoundaryCommitStage> {
        Self::stage_order()
            .into_iter()
            .find(|stage| self.stage_blocker_component_count(*stage) > 0)
    }

    pub fn failure_report_for(
        self,
        stage: RuntimeBoundaryCommitStage,
    ) -> Option<RuntimeFailureReport> {
        let component_count = self.stage_blocker_component_count(stage);
        if component_count == 0 {
            None
        } else {
            Some(RuntimeFailureReport::new(
                stage.failure_kind(),
                stage.failure_message(component_count),
            ))
        }
    }

    pub fn failure_reports(self) -> Vec<RuntimeFailureReport> {
        Self::stage_order()
            .into_iter()
            .filter_map(|stage| self.failure_report_for(stage))
            .collect()
    }

    pub fn failure_report_count(self) -> usize {
        Self::stage_order()
            .into_iter()
            .filter(|stage| self.stage_blocker_component_count(*stage) > 0)
            .count()
    }

    pub fn has_failure_reports(self) -> bool {
        self.failure_report_count() > 0
    }

    pub fn failure_batch_summary(self) -> RuntimeFailureBatchSummary {
        RuntimeFailureReport::batch_summary(&self.failure_reports())
    }

    pub fn can_format_runtime_failures(self) -> bool {
        self.failure_batch_summary().can_format_runtime_failures()
    }

    pub fn primary_failure_report(self) -> Option<RuntimeFailureReport> {
        self.first_blocking_stage()
            .and_then(|stage| self.failure_report_for(stage))
    }

    pub fn primary_failure_summary(self) -> Option<RuntimeFailureSummary> {
        self.primary_failure_report()
            .map(|failure| failure.failure_summary())
    }

    pub fn commit_summary(self) -> RuntimeBoundaryCommitSummary {
        RuntimeBoundaryCommitSummary::new(self)
    }

    pub fn readiness_accounting_is_consistent(self) -> bool {
        self.component_accounting_consistent
            && self.acceptance_signal_component_count
                == self
                    .request_acceptance_signal_component_count
                    .saturating_add(self.response_acceptance_signal_component_count)
                    .saturating_add(self.boundary_acceptance_signal_component_count)
            && self.envelope_signal_component_count == self.boundary_envelope_signal_component_count
            && self.adapter_signal_component_count == self.boundary_adapter_signal_component_count
            && self.kv_signal_component_count == self.boundary_kv_signal_component_count
            && self.gate_signal_component_count == self.boundary_gate_signal_component_count
            && self.runtime_response_signal_component_count
                == usize::from(self.runtime_response_ready)
            && self.total_signal_component_count
                == self
                    .acceptance_signal_component_count
                    .saturating_add(self.envelope_signal_component_count)
                    .saturating_add(self.adapter_signal_component_count)
                    .saturating_add(self.kv_signal_component_count)
                    .saturating_add(self.gate_signal_component_count)
                    .saturating_add(self.runtime_response_signal_component_count)
            && self.acceptance_blocker_component_count
                == self
                    .request_acceptance_blocker_component_count
                    .saturating_add(self.response_acceptance_blocker_component_count)
                    .saturating_add(self.boundary_acceptance_blocker_component_count)
            && self.envelope_blocker_component_count
                == self.boundary_envelope_blocker_component_count
            && self.adapter_blocker_component_count == self.boundary_adapter_blocker_component_count
            && self.kv_blocker_component_count == self.boundary_kv_blocker_component_count
            && self.gate_blocker_component_count == self.boundary_gate_blocker_component_count
            && self.runtime_response_blocker_component_count
                == usize::from(!self.runtime_response_ready)
            && self.total_blocker_component_count
                == self
                    .acceptance_blocker_component_count
                    .saturating_add(self.envelope_blocker_component_count)
                    .saturating_add(self.adapter_blocker_component_count)
                    .saturating_add(self.kv_blocker_component_count)
                    .saturating_add(self.gate_blocker_component_count)
                    .saturating_add(self.runtime_response_blocker_component_count)
            && self.has_commit_signals() == (self.total_signal_component_count > 0)
            && self.has_commit_blockers() == (self.total_blocker_component_count > 0)
    }

    pub fn readiness_commit_is_clean(self) -> bool {
        !self.has_commit_blockers() && self.readiness_accounting_is_consistent()
    }

    pub fn can_commit_runtime_boundary(self) -> bool {
        self.all_acceptance_ready()
            && self.all_boundary_summaries_ready()
            && self.boundary_gate_ready
            && self.runtime_response_ready
            && self.readiness_commit_is_clean()
    }

    pub fn runtime_boundary_commit_action(self) -> RuntimeBoundaryCommitAction {
        if self.can_commit_runtime_boundary() {
            RuntimeBoundaryCommitAction::CommitBoundary
        } else {
            RuntimeBoundaryCommitAction::ReturnRuntimeFailure
        }
    }
}

impl RuntimeBoundaryCommitSummary {
    pub fn new(readiness: RuntimeBoundaryCommitReadinessSummary) -> Self {
        let can_commit = readiness.can_commit_runtime_boundary();
        let failure_report_count = readiness.failure_report_count();
        let failure_batch = readiness.failure_batch_summary();
        let can_format_runtime_failures = readiness.can_format_runtime_failures();
        let should_return_failure = !can_commit && failure_report_count > 0;
        let action = readiness.runtime_boundary_commit_action();

        Self {
            readiness,
            action,
            can_commit,
            should_return_failure,
            first_unready_stage: readiness.first_unready_stage(),
            first_blocking_stage: readiness.first_blocking_stage(),
            primary_failure_summary: readiness.primary_failure_summary(),
            failure_batch,
            failure_report_count,
            can_format_runtime_failures,
            planning_dense_compute_avoided_tokens: readiness.planning_dense_compute_avoided_tokens,
            total_signal_component_count: readiness.total_signal_component_count,
            total_blocker_component_count: readiness.total_blocker_component_count,
            component_accounting_consistent: readiness.readiness_accounting_is_consistent(),
        }
    }

    pub fn action_can_commit(self) -> bool {
        self.action.can_commit()
    }

    pub fn action_should_return_failure(self) -> bool {
        self.action.should_return_failure()
    }

    pub fn has_primary_failure_summary(self) -> bool {
        self.primary_failure_summary.is_some()
    }

    pub fn has_planning_dense_compute_savings(self) -> bool {
        self.planning_dense_compute_avoided_tokens > 0
    }

    pub fn failure_report_for(
        self,
        stage: RuntimeBoundaryCommitStage,
    ) -> Option<RuntimeFailureReport> {
        self.readiness.failure_report_for(stage)
    }

    pub fn failure_reports(self) -> Vec<RuntimeFailureReport> {
        self.readiness.failure_reports()
    }

    pub fn primary_failure_report(self) -> Option<RuntimeFailureReport> {
        self.readiness.primary_failure_report()
    }

    pub fn failure_batch_shape_is_clean(self) -> bool {
        self.failure_batch.failure_batch_shape_is_clean()
    }

    pub fn failure_return_summary(self) -> RuntimeFailureReturnSummary {
        RuntimeFailureReturnSummary::new(
            RuntimeFailureReturnSource::BoundaryCommit,
            self.can_commit,
            self.should_return_failure,
            self.primary_failure_summary,
            self.failure_batch,
            self.failure_report_count,
            self.can_format_runtime_failures,
            self.total_blocker_component_count,
            self.commit_decision_accounting_is_consistent(),
        )
    }

    pub fn runtime_failure_return_report(self) -> Option<RuntimeFailureReturnReport> {
        let failure_return = self.failure_return_summary();
        if failure_return.can_return_runtime_failure() {
            self.primary_failure_report().map(|failure| {
                RuntimeFailureReturnReport::new(
                    RuntimeFailureReturnSource::BoundaryCommit,
                    failure,
                    self.failure_batch,
                    self.failure_report_count,
                    self.can_format_runtime_failures,
                    self.total_blocker_component_count,
                )
            })
        } else {
            None
        }
    }

    pub fn commit_decision_accounting_is_consistent(self) -> bool {
        self.can_commit == self.readiness.can_commit_runtime_boundary()
            && self.should_return_failure == (!self.can_commit && self.failure_report_count > 0)
            && self.action == self.readiness.runtime_boundary_commit_action()
            && self.action_can_commit() == self.can_commit
            && self.action_should_return_failure() == self.should_return_failure
            && self.first_unready_stage == self.readiness.first_unready_stage()
            && self.first_blocking_stage == self.readiness.first_blocking_stage()
            && self.primary_failure_summary == self.readiness.primary_failure_summary()
            && self.failure_batch == self.readiness.failure_batch_summary()
            && self.failure_report_count == self.readiness.failure_report_count()
            && self.failure_report_count == self.failure_reports().len()
            && self.can_format_runtime_failures == self.readiness.can_format_runtime_failures()
            && self.planning_dense_compute_avoided_tokens
                == self.readiness.planning_dense_compute_avoided_tokens
            && self.total_signal_component_count == self.readiness.total_signal_component_count
            && self.total_blocker_component_count == self.readiness.total_blocker_component_count
            && self.component_accounting_consistent
                == self.readiness.readiness_accounting_is_consistent()
    }

    pub fn can_consume_runtime_boundary_device_execution_commit_summary(
        self,
        device_execution: RuntimeBoundaryDeviceExecutionCommitSummary,
    ) -> bool {
        self.commit_decision_accounting_is_consistent()
            && device_execution.commit_decision_accounting_is_consistent()
    }

    pub fn consumes_runtime_boundary_device_execution_without_boundary_commit(
        self,
        device_execution: RuntimeBoundaryDeviceExecutionCommitSummary,
    ) -> bool {
        self.can_consume_runtime_boundary_device_execution_commit_summary(device_execution)
            && !self.can_commit_runtime_boundary()
            && self.action_should_return_failure()
    }

    pub fn can_commit_runtime_boundary(self) -> bool {
        self.can_commit && self.commit_decision_accounting_is_consistent()
    }

    pub fn should_return_runtime_failure(self) -> bool {
        self.should_return_failure && self.commit_decision_accounting_is_consistent()
    }
}

impl RuntimeManifestBoundaryCommitReadinessSummary {
    pub fn new(
        boundary_commit: RuntimeBoundaryCommitReadinessSummary,
        manifest_boundary_kv: RuntimeManifestBoundaryKvSummary,
    ) -> Self {
        let boundary_commit_ready = boundary_commit.can_commit_runtime_boundary();
        let manifest_boundary_kv_ready = manifest_boundary_kv.can_commit_manifest_boundary_kv();
        let boundary_commit_action = boundary_commit.commit_summary().action;
        let boundary_commit_signal_component_count = boundary_commit.total_signal_component_count;
        let manifest_boundary_kv_signal_component_count =
            manifest_boundary_kv.manifest_boundary_kv_signal_component_count();
        let total_signal_component_count = boundary_commit_signal_component_count
            .saturating_add(manifest_boundary_kv_signal_component_count);
        let boundary_commit_blocker_component_count = boundary_commit.total_blocker_component_count;
        let manifest_boundary_kv_blocker_component_count =
            manifest_boundary_kv.manifest_boundary_kv_blocker_component_count();
        let total_blocker_component_count = boundary_commit_blocker_component_count
            .saturating_add(manifest_boundary_kv_blocker_component_count);
        let component_accounting_consistent = boundary_commit.readiness_accounting_is_consistent()
            && manifest_boundary_kv.manifest_boundary_kv_accounting_is_consistent();

        Self {
            boundary_commit,
            manifest_boundary_kv,
            boundary_commit_action,
            boundary_commit_ready,
            manifest_boundary_kv_ready,
            boundary_commit_signal_component_count,
            manifest_boundary_kv_signal_component_count,
            total_signal_component_count,
            boundary_commit_blocker_component_count,
            manifest_boundary_kv_blocker_component_count,
            total_blocker_component_count,
            component_accounting_consistent,
        }
    }

    pub fn stage_order() -> [RuntimeManifestBoundaryCommitStage; 2] {
        [
            RuntimeManifestBoundaryCommitStage::BoundaryCommit,
            RuntimeManifestBoundaryCommitStage::ManifestBoundaryKv,
        ]
    }

    pub fn problem_kind_order() -> [RuntimeManifestBoundaryCommitProblemKind; 4] {
        [
            RuntimeManifestBoundaryCommitProblemKind::BoundaryCommit,
            RuntimeManifestBoundaryCommitProblemKind::RequestManifestPlanning,
            RuntimeManifestBoundaryCommitProblemKind::ResponseManifestKv,
            RuntimeManifestBoundaryCommitProblemKind::ComponentAccounting,
        ]
    }

    pub fn has_commit_signals(self) -> bool {
        self.total_signal_component_count > 0
    }

    pub fn has_commit_blockers(self) -> bool {
        self.total_blocker_component_count > 0
    }

    pub fn component_accounting_drift_count(self) -> usize {
        usize::from(!self.readiness_accounting_is_consistent())
    }

    pub fn problem_kind_component_count(
        self,
        kind: RuntimeManifestBoundaryCommitProblemKind,
    ) -> usize {
        match kind {
            RuntimeManifestBoundaryCommitProblemKind::BoundaryCommit => {
                self.boundary_commit_blocker_component_count
            }
            RuntimeManifestBoundaryCommitProblemKind::RequestManifestPlanning => {
                self.manifest_boundary_kv
                    .request_manifest_planning_blocker_component_count
            }
            RuntimeManifestBoundaryCommitProblemKind::ResponseManifestKv => {
                self.manifest_boundary_kv
                    .response_manifest_kv_blocker_component_count
            }
            RuntimeManifestBoundaryCommitProblemKind::ComponentAccounting => {
                self.component_accounting_drift_count()
            }
        }
    }

    pub fn manifest_boundary_commit_problem_component_count(self) -> usize {
        self.boundary_commit_blocker_component_count
            .saturating_add(
                self.manifest_boundary_kv
                    .manifest_boundary_kv_blocker_component_count(),
            )
            .saturating_add(self.component_accounting_drift_count())
    }

    pub fn has_manifest_boundary_commit_problem_components(self) -> bool {
        self.manifest_boundary_commit_problem_component_count() > 0
    }

    pub fn all_boundary_commit_stages_ready(self) -> bool {
        self.boundary_commit_ready && self.manifest_boundary_kv_ready
    }

    pub fn stage_ready(self, stage: RuntimeManifestBoundaryCommitStage) -> bool {
        match stage {
            RuntimeManifestBoundaryCommitStage::BoundaryCommit => self.boundary_commit_ready,
            RuntimeManifestBoundaryCommitStage::ManifestBoundaryKv => {
                self.manifest_boundary_kv_ready
            }
        }
    }

    pub fn boundary_commit_action_matches_readiness(self) -> bool {
        self.boundary_commit_action.can_commit() == self.boundary_commit_ready
            && self.boundary_commit_action == self.boundary_commit.commit_summary().action
    }

    pub fn stage_signal_component_count(self, stage: RuntimeManifestBoundaryCommitStage) -> usize {
        match stage {
            RuntimeManifestBoundaryCommitStage::BoundaryCommit => {
                self.boundary_commit_signal_component_count
            }
            RuntimeManifestBoundaryCommitStage::ManifestBoundaryKv => {
                self.manifest_boundary_kv_signal_component_count
            }
        }
    }

    pub fn stage_blocker_component_count(self, stage: RuntimeManifestBoundaryCommitStage) -> usize {
        match stage {
            RuntimeManifestBoundaryCommitStage::BoundaryCommit => {
                self.boundary_commit_blocker_component_count
            }
            RuntimeManifestBoundaryCommitStage::ManifestBoundaryKv => {
                self.manifest_boundary_kv_blocker_component_count
            }
        }
    }

    pub fn first_unready_stage(self) -> Option<RuntimeManifestBoundaryCommitStage> {
        Self::stage_order()
            .into_iter()
            .find(|stage| !self.stage_ready(*stage))
    }

    pub fn first_blocking_stage(self) -> Option<RuntimeManifestBoundaryCommitStage> {
        Self::stage_order()
            .into_iter()
            .find(|stage| self.stage_blocker_component_count(*stage) > 0)
    }

    pub fn first_problem_kind(self) -> Option<RuntimeManifestBoundaryCommitProblemKind> {
        Self::problem_kind_order()
            .into_iter()
            .find(|kind| self.problem_kind_component_count(*kind) > 0)
    }

    pub fn failure_report_for(
        self,
        kind: RuntimeManifestBoundaryCommitProblemKind,
    ) -> Option<RuntimeFailureReport> {
        let component_count = self.problem_kind_component_count(kind);
        if component_count == 0 {
            None
        } else {
            Some(RuntimeFailureReport::new(
                kind.failure_kind(),
                kind.failure_message(component_count),
            ))
        }
    }

    pub fn failure_reports(self) -> Vec<RuntimeFailureReport> {
        Self::problem_kind_order()
            .into_iter()
            .filter_map(|kind| self.failure_report_for(kind))
            .collect()
    }

    pub fn failure_report_count(self) -> usize {
        Self::problem_kind_order()
            .into_iter()
            .filter(|kind| self.problem_kind_component_count(*kind) > 0)
            .count()
    }

    pub fn has_failure_reports(self) -> bool {
        self.failure_report_count() > 0
    }

    pub fn failure_batch_summary(self) -> RuntimeFailureBatchSummary {
        RuntimeFailureReport::batch_summary(&self.failure_reports())
    }

    pub fn can_format_runtime_failures(self) -> bool {
        self.failure_batch_summary().can_format_runtime_failures()
    }

    pub fn primary_failure_report(self) -> Option<RuntimeFailureReport> {
        self.first_problem_kind()
            .and_then(|kind| self.failure_report_for(kind))
    }

    pub fn primary_failure_summary(self) -> Option<RuntimeFailureSummary> {
        self.primary_failure_report()
            .map(|failure| failure.failure_summary())
    }

    pub fn commit_summary(self) -> RuntimeManifestBoundaryCommitSummary {
        RuntimeManifestBoundaryCommitSummary::new(self)
    }

    pub fn readiness_accounting_is_consistent(self) -> bool {
        self.component_accounting_consistent
            && self.boundary_commit_signal_component_count
                == self.boundary_commit.total_signal_component_count
            && self.manifest_boundary_kv_signal_component_count
                == self
                    .manifest_boundary_kv
                    .manifest_boundary_kv_signal_component_count()
            && self.total_signal_component_count
                == self
                    .boundary_commit_signal_component_count
                    .saturating_add(self.manifest_boundary_kv_signal_component_count)
            && self.boundary_commit_blocker_component_count
                == self.boundary_commit.total_blocker_component_count
            && self.manifest_boundary_kv_blocker_component_count
                == self
                    .manifest_boundary_kv
                    .manifest_boundary_kv_blocker_component_count()
            && self.total_blocker_component_count
                == self
                    .boundary_commit_blocker_component_count
                    .saturating_add(self.manifest_boundary_kv_blocker_component_count)
            && self.boundary_commit_action_matches_readiness()
            && self.has_commit_signals() == (self.total_signal_component_count > 0)
            && self.has_commit_blockers() == (self.total_blocker_component_count > 0)
    }

    pub fn readiness_commit_is_clean(self) -> bool {
        !self.has_commit_blockers() && self.readiness_accounting_is_consistent()
    }

    pub fn can_commit_runtime_manifest_boundary(self) -> bool {
        self.all_boundary_commit_stages_ready() && self.readiness_commit_is_clean()
    }

    pub fn runtime_manifest_boundary_commit_action(self) -> RuntimeManifestBoundaryCommitAction {
        if self.can_commit_runtime_manifest_boundary() {
            RuntimeManifestBoundaryCommitAction::CommitManifestBoundary
        } else {
            RuntimeManifestBoundaryCommitAction::ReturnRuntimeFailure
        }
    }
}

impl RuntimeManifestBoundaryCommitSummary {
    pub fn new(readiness: RuntimeManifestBoundaryCommitReadinessSummary) -> Self {
        let can_commit = readiness.can_commit_runtime_manifest_boundary();
        let failure_report_count = readiness.failure_report_count();
        let failure_batch = readiness.failure_batch_summary();
        let can_format_runtime_failures = readiness.can_format_runtime_failures();
        let should_return_failure = !can_commit && failure_report_count > 0;
        let action = readiness.runtime_manifest_boundary_commit_action();

        Self {
            readiness,
            action,
            boundary_commit_action: readiness.boundary_commit_action,
            can_commit,
            should_return_failure,
            first_unready_stage: readiness.first_unready_stage(),
            first_blocking_stage: readiness.first_blocking_stage(),
            first_problem_kind: readiness.first_problem_kind(),
            primary_failure_summary: readiness.primary_failure_summary(),
            failure_batch,
            failure_report_count,
            can_format_runtime_failures,
            total_signal_component_count: readiness.total_signal_component_count,
            total_blocker_component_count: readiness.total_blocker_component_count,
            component_accounting_consistent: readiness.readiness_accounting_is_consistent(),
        }
    }

    pub fn action_can_commit(self) -> bool {
        self.action.can_commit()
    }

    pub fn action_should_return_failure(self) -> bool {
        self.action.should_return_failure()
    }

    pub fn has_primary_failure_summary(self) -> bool {
        self.primary_failure_summary.is_some()
    }

    pub fn failure_report_for(
        self,
        kind: RuntimeManifestBoundaryCommitProblemKind,
    ) -> Option<RuntimeFailureReport> {
        self.readiness.failure_report_for(kind)
    }

    pub fn failure_reports(self) -> Vec<RuntimeFailureReport> {
        self.readiness.failure_reports()
    }

    pub fn primary_failure_report(self) -> Option<RuntimeFailureReport> {
        self.readiness.primary_failure_report()
    }

    pub fn failure_batch_shape_is_clean(self) -> bool {
        self.failure_batch.failure_batch_shape_is_clean()
    }

    pub fn failure_return_summary(self) -> RuntimeFailureReturnSummary {
        RuntimeFailureReturnSummary::new(
            RuntimeFailureReturnSource::ManifestBoundaryCommit,
            self.can_commit,
            self.should_return_failure,
            self.primary_failure_summary,
            self.failure_batch,
            self.failure_report_count,
            self.can_format_runtime_failures,
            self.total_blocker_component_count,
            self.commit_decision_accounting_is_consistent(),
        )
    }

    pub fn runtime_failure_return_report(self) -> Option<RuntimeFailureReturnReport> {
        let failure_return = self.failure_return_summary();
        if failure_return.can_return_runtime_failure() {
            self.primary_failure_report().map(|failure| {
                RuntimeFailureReturnReport::new(
                    RuntimeFailureReturnSource::ManifestBoundaryCommit,
                    failure,
                    self.failure_batch,
                    self.failure_report_count,
                    self.can_format_runtime_failures,
                    self.total_blocker_component_count,
                )
            })
        } else {
            None
        }
    }

    pub fn commit_decision_accounting_is_consistent(self) -> bool {
        self.can_commit == self.readiness.can_commit_runtime_manifest_boundary()
            && self.should_return_failure == (!self.can_commit && self.failure_report_count > 0)
            && self.action == self.readiness.runtime_manifest_boundary_commit_action()
            && self.action_can_commit() == self.can_commit
            && self.action_should_return_failure() == self.should_return_failure
            && self.boundary_commit_action == self.readiness.boundary_commit_action
            && self.readiness.boundary_commit_action_matches_readiness()
            && self.first_unready_stage == self.readiness.first_unready_stage()
            && self.first_blocking_stage == self.readiness.first_blocking_stage()
            && self.first_problem_kind == self.readiness.first_problem_kind()
            && self.primary_failure_summary == self.readiness.primary_failure_summary()
            && self.failure_batch == self.readiness.failure_batch_summary()
            && self.failure_report_count == self.readiness.failure_report_count()
            && self.failure_report_count == self.failure_reports().len()
            && self.can_format_runtime_failures == self.readiness.can_format_runtime_failures()
            && self.total_signal_component_count == self.readiness.total_signal_component_count
            && self.total_blocker_component_count == self.readiness.total_blocker_component_count
            && self.component_accounting_consistent
                == self.readiness.readiness_accounting_is_consistent()
    }

    pub fn can_commit_runtime_manifest_boundary(self) -> bool {
        self.can_commit && self.commit_decision_accounting_is_consistent()
    }

    pub fn should_return_runtime_failure(self) -> bool {
        self.should_return_failure && self.commit_decision_accounting_is_consistent()
    }
}

impl RuntimeKvSideEffectReadinessSummary {
    pub fn new(
        import: RuntimeKvImportReadinessSummary,
        manifest_boundary_commit: RuntimeManifestBoundaryCommitReadinessSummary,
        export: RuntimeKvExportReadinessSummary,
    ) -> Self {
        let import_ready = import.can_commit_runtime_kv_import_readiness();
        let manifest_boundary_commit_ready =
            manifest_boundary_commit.can_commit_runtime_manifest_boundary();
        let export_ready = export.can_commit_runtime_kv_export_readiness();
        let import_commit_action = import.commit_summary().action;
        let manifest_boundary_commit_action = manifest_boundary_commit.commit_summary().action;
        let export_commit_action = export.commit_summary().action;
        let import_signal_component_count =
            import.runtime_kv_import_readiness_signal_component_count();
        let manifest_boundary_commit_signal_component_count =
            manifest_boundary_commit.total_signal_component_count;
        let export_signal_component_count =
            export.runtime_kv_export_readiness_signal_component_count();
        let total_signal_component_count = import_signal_component_count
            .saturating_add(manifest_boundary_commit_signal_component_count)
            .saturating_add(export_signal_component_count);
        let import_blocker_component_count =
            import.runtime_kv_import_readiness_blocker_component_count();
        let manifest_boundary_commit_blocker_component_count =
            manifest_boundary_commit.manifest_boundary_commit_problem_component_count();
        let export_blocker_component_count =
            export.runtime_kv_export_readiness_blocker_component_count();
        let total_blocker_component_count = import_blocker_component_count
            .saturating_add(manifest_boundary_commit_blocker_component_count)
            .saturating_add(export_blocker_component_count);
        let component_accounting_consistent = import
            .runtime_kv_import_readiness_accounting_is_consistent()
            && manifest_boundary_commit.readiness_accounting_is_consistent()
            && export.runtime_kv_export_readiness_accounting_is_consistent();

        Self {
            import,
            manifest_boundary_commit,
            export,
            import_commit_action,
            manifest_boundary_commit_action,
            export_commit_action,
            import_ready,
            manifest_boundary_commit_ready,
            export_ready,
            import_signal_component_count,
            manifest_boundary_commit_signal_component_count,
            export_signal_component_count,
            total_signal_component_count,
            import_blocker_component_count,
            manifest_boundary_commit_blocker_component_count,
            export_blocker_component_count,
            total_blocker_component_count,
            component_accounting_consistent,
        }
    }

    pub fn stage_order() -> [RuntimeKvSideEffectStage; 3] {
        [
            RuntimeKvSideEffectStage::RuntimeKvImport,
            RuntimeKvSideEffectStage::ManifestBoundaryCommit,
            RuntimeKvSideEffectStage::RuntimeKvExport,
        ]
    }

    pub fn problem_kind_order() -> [RuntimeKvSideEffectProblemKind; 7] {
        [
            RuntimeKvSideEffectProblemKind::RuntimeKvImport,
            RuntimeKvSideEffectProblemKind::BoundaryCommit,
            RuntimeKvSideEffectProblemKind::RequestManifestPlanning,
            RuntimeKvSideEffectProblemKind::ResponseManifestKv,
            RuntimeKvSideEffectProblemKind::ManifestBoundaryAccounting,
            RuntimeKvSideEffectProblemKind::RuntimeKvExport,
            RuntimeKvSideEffectProblemKind::ComponentAccounting,
        ]
    }

    pub fn stage_ready(self, stage: RuntimeKvSideEffectStage) -> bool {
        match stage {
            RuntimeKvSideEffectStage::RuntimeKvImport => self.import_ready,
            RuntimeKvSideEffectStage::ManifestBoundaryCommit => self.manifest_boundary_commit_ready,
            RuntimeKvSideEffectStage::RuntimeKvExport => self.export_ready,
        }
    }

    pub fn import_commit_action_matches_readiness(self) -> bool {
        self.import_commit_action.can_commit() == self.import_ready
            && self.import_commit_action == self.import.commit_summary().action
    }

    pub fn export_commit_action_matches_readiness(self) -> bool {
        self.export_commit_action.can_commit() == self.export_ready
            && self.export_commit_action == self.export.commit_summary().action
    }

    pub fn manifest_boundary_commit_action_matches_readiness(self) -> bool {
        self.manifest_boundary_commit_action.can_commit() == self.manifest_boundary_commit_ready
            && self.manifest_boundary_commit_action
                == self.manifest_boundary_commit.commit_summary().action
    }

    pub fn child_commit_actions_match_readiness(self) -> bool {
        self.import_commit_action_matches_readiness()
            && self.manifest_boundary_commit_action_matches_readiness()
            && self.export_commit_action_matches_readiness()
    }

    pub fn child_commit_action_drift_component_count(self) -> usize {
        usize::from(!self.import_commit_action_matches_readiness())
            .saturating_add(usize::from(
                !self.manifest_boundary_commit_action_matches_readiness(),
            ))
            .saturating_add(usize::from(!self.export_commit_action_matches_readiness()))
    }

    pub fn component_accounting_drift_count(self) -> usize {
        usize::from(!self.runtime_kv_side_effect_accounting_is_consistent())
    }

    pub fn problem_kind_component_count(self, kind: RuntimeKvSideEffectProblemKind) -> usize {
        match kind {
            RuntimeKvSideEffectProblemKind::RuntimeKvImport => self.import_blocker_component_count,
            RuntimeKvSideEffectProblemKind::BoundaryCommit => {
                self.manifest_boundary_commit.problem_kind_component_count(
                    RuntimeManifestBoundaryCommitProblemKind::BoundaryCommit,
                )
            }
            RuntimeKvSideEffectProblemKind::RequestManifestPlanning => {
                self.manifest_boundary_commit.problem_kind_component_count(
                    RuntimeManifestBoundaryCommitProblemKind::RequestManifestPlanning,
                )
            }
            RuntimeKvSideEffectProblemKind::ResponseManifestKv => {
                self.manifest_boundary_commit.problem_kind_component_count(
                    RuntimeManifestBoundaryCommitProblemKind::ResponseManifestKv,
                )
            }
            RuntimeKvSideEffectProblemKind::ManifestBoundaryAccounting => {
                self.manifest_boundary_commit.problem_kind_component_count(
                    RuntimeManifestBoundaryCommitProblemKind::ComponentAccounting,
                )
            }
            RuntimeKvSideEffectProblemKind::RuntimeKvExport => self.export_blocker_component_count,
            RuntimeKvSideEffectProblemKind::ComponentAccounting => {
                self.component_accounting_drift_count()
            }
        }
    }

    pub fn stage_signal_component_count(self, stage: RuntimeKvSideEffectStage) -> usize {
        match stage {
            RuntimeKvSideEffectStage::RuntimeKvImport => self.import_signal_component_count,
            RuntimeKvSideEffectStage::ManifestBoundaryCommit => {
                self.manifest_boundary_commit_signal_component_count
            }
            RuntimeKvSideEffectStage::RuntimeKvExport => self.export_signal_component_count,
        }
    }

    pub fn stage_blocker_component_count(self, stage: RuntimeKvSideEffectStage) -> usize {
        match stage {
            RuntimeKvSideEffectStage::RuntimeKvImport => self.import_blocker_component_count,
            RuntimeKvSideEffectStage::ManifestBoundaryCommit => {
                self.manifest_boundary_commit_blocker_component_count
            }
            RuntimeKvSideEffectStage::RuntimeKvExport => self.export_blocker_component_count,
        }
    }

    pub fn first_unready_stage(self) -> Option<RuntimeKvSideEffectStage> {
        Self::stage_order()
            .into_iter()
            .find(|stage| !self.stage_ready(*stage))
    }

    pub fn first_blocking_stage(self) -> Option<RuntimeKvSideEffectStage> {
        Self::stage_order()
            .into_iter()
            .find(|stage| self.stage_blocker_component_count(*stage) > 0)
    }

    pub fn first_problem_kind(self) -> Option<RuntimeKvSideEffectProblemKind> {
        Self::problem_kind_order()
            .into_iter()
            .find(|kind| self.problem_kind_component_count(*kind) > 0)
    }

    pub fn failure_report_for(
        self,
        kind: RuntimeKvSideEffectProblemKind,
    ) -> Option<RuntimeFailureReport> {
        let component_count = self.problem_kind_component_count(kind);
        if component_count == 0 {
            None
        } else {
            Some(RuntimeFailureReport::new(
                kind.failure_kind(),
                kind.failure_message(component_count),
            ))
        }
    }

    pub fn failure_reports(self) -> Vec<RuntimeFailureReport> {
        Self::problem_kind_order()
            .into_iter()
            .filter_map(|kind| self.failure_report_for(kind))
            .collect()
    }

    pub fn failure_report_count(self) -> usize {
        Self::problem_kind_order()
            .into_iter()
            .filter(|kind| self.problem_kind_component_count(*kind) > 0)
            .count()
    }

    pub fn has_failure_reports(self) -> bool {
        self.failure_report_count() > 0
    }

    pub fn failure_batch_summary(self) -> RuntimeFailureBatchSummary {
        RuntimeFailureReport::batch_summary(&self.failure_reports())
    }

    pub fn can_format_runtime_failures(self) -> bool {
        self.failure_batch_summary().can_format_runtime_failures()
    }

    pub fn primary_failure_report(self) -> Option<RuntimeFailureReport> {
        self.first_problem_kind()
            .and_then(|kind| self.failure_report_for(kind))
    }

    pub fn primary_failure_summary(self) -> Option<RuntimeFailureSummary> {
        self.primary_failure_report()
            .map(|failure| failure.failure_summary())
    }

    pub fn commit_summary(self) -> RuntimeKvSideEffectCommitSummary {
        RuntimeKvSideEffectCommitSummary::new(self)
    }

    pub fn runtime_kv_side_effect_signal_component_count(self) -> usize {
        self.total_signal_component_count
    }

    pub fn has_runtime_kv_side_effect_signals(self) -> bool {
        self.total_signal_component_count > 0
    }

    pub fn runtime_kv_side_effect_blocker_component_count(self) -> usize {
        self.total_blocker_component_count
    }

    pub fn runtime_kv_side_effect_problem_component_count(self) -> usize {
        self.import_blocker_component_count
            .saturating_add(
                self.manifest_boundary_commit
                    .manifest_boundary_commit_problem_component_count(),
            )
            .saturating_add(self.export_blocker_component_count)
            .saturating_add(self.component_accounting_drift_count())
    }

    pub fn has_runtime_kv_side_effect_blockers(self) -> bool {
        self.total_blocker_component_count > 0
    }

    pub fn has_runtime_kv_side_effect_problem_components(self) -> bool {
        self.runtime_kv_side_effect_problem_component_count() > 0
    }

    pub fn runtime_kv_side_effect_accounting_is_consistent(self) -> bool {
        self.component_accounting_consistent
            && self.import_signal_component_count
                == self
                    .import
                    .runtime_kv_import_readiness_signal_component_count()
            && self.manifest_boundary_commit_signal_component_count
                == self.manifest_boundary_commit.total_signal_component_count
            && self.export_signal_component_count
                == self
                    .export
                    .runtime_kv_export_readiness_signal_component_count()
            && self.total_signal_component_count
                == self
                    .import_signal_component_count
                    .saturating_add(self.manifest_boundary_commit_signal_component_count)
                    .saturating_add(self.export_signal_component_count)
            && self.import_blocker_component_count
                == self
                    .import
                    .runtime_kv_import_readiness_blocker_component_count()
            && self.manifest_boundary_commit_blocker_component_count
                == self
                    .manifest_boundary_commit
                    .manifest_boundary_commit_problem_component_count()
            && self.export_blocker_component_count
                == self
                    .export
                    .runtime_kv_export_readiness_blocker_component_count()
            && self.child_commit_actions_match_readiness()
            && self.total_blocker_component_count
                == self
                    .import_blocker_component_count
                    .saturating_add(self.manifest_boundary_commit_blocker_component_count)
                    .saturating_add(self.export_blocker_component_count)
            && self.has_runtime_kv_side_effect_signals() == (self.total_signal_component_count > 0)
            && self.has_runtime_kv_side_effect_blockers()
                == (self.total_blocker_component_count > 0)
    }

    pub fn runtime_kv_side_effect_commit_is_clean(self) -> bool {
        !self.has_runtime_kv_side_effect_blockers()
            && self.runtime_kv_side_effect_accounting_is_consistent()
    }

    pub fn can_commit_runtime_kv_side_effects(self) -> bool {
        self.import_ready
            && self.manifest_boundary_commit_ready
            && self.export_ready
            && self.runtime_kv_side_effect_commit_is_clean()
    }

    pub fn runtime_kv_side_effect_commit_action(self) -> RuntimeKvSideEffectCommitAction {
        if self.can_commit_runtime_kv_side_effects() {
            RuntimeKvSideEffectCommitAction::CommitSideEffects
        } else {
            RuntimeKvSideEffectCommitAction::ReturnRuntimeFailure
        }
    }
}

impl RuntimeKvSideEffectCommitSummary {
    pub fn new(readiness: RuntimeKvSideEffectReadinessSummary) -> Self {
        let can_commit = readiness.can_commit_runtime_kv_side_effects();
        let failure_report_count = readiness.failure_report_count();
        let failure_batch = readiness.failure_batch_summary();
        let can_format_runtime_failures = readiness.can_format_runtime_failures();
        let should_return_failure = !can_commit && failure_report_count > 0;
        let action = readiness.runtime_kv_side_effect_commit_action();

        Self {
            readiness,
            action,
            import_commit_action: readiness.import_commit_action,
            manifest_boundary_commit_action: readiness.manifest_boundary_commit_action,
            export_commit_action: readiness.export_commit_action,
            can_commit,
            should_return_failure,
            first_unready_stage: readiness.first_unready_stage(),
            first_blocking_stage: readiness.first_blocking_stage(),
            first_problem_kind: readiness.first_problem_kind(),
            primary_failure_summary: readiness.primary_failure_summary(),
            failure_batch,
            failure_report_count,
            can_format_runtime_failures,
            total_signal_component_count: readiness.total_signal_component_count,
            total_blocker_component_count: readiness.total_blocker_component_count,
            component_accounting_consistent: readiness
                .runtime_kv_side_effect_accounting_is_consistent(),
        }
    }

    pub fn action_can_commit(self) -> bool {
        self.action.can_commit()
    }

    pub fn action_should_return_failure(self) -> bool {
        self.action.should_return_failure()
    }

    pub fn has_primary_failure_summary(self) -> bool {
        self.primary_failure_summary.is_some()
    }

    pub fn failure_report_for(
        self,
        kind: RuntimeKvSideEffectProblemKind,
    ) -> Option<RuntimeFailureReport> {
        self.readiness.failure_report_for(kind)
    }

    pub fn failure_reports(self) -> Vec<RuntimeFailureReport> {
        self.readiness.failure_reports()
    }

    pub fn primary_failure_report(self) -> Option<RuntimeFailureReport> {
        self.readiness.primary_failure_report()
    }

    pub fn failure_batch_shape_is_clean(self) -> bool {
        self.failure_batch.failure_batch_shape_is_clean()
    }

    pub fn failure_return_summary(self) -> RuntimeFailureReturnSummary {
        RuntimeFailureReturnSummary::new(
            RuntimeFailureReturnSource::KvSideEffectCommit,
            self.can_commit,
            self.should_return_failure,
            self.primary_failure_summary,
            self.failure_batch,
            self.failure_report_count,
            self.can_format_runtime_failures,
            self.total_blocker_component_count,
            self.commit_decision_accounting_is_consistent(),
        )
    }

    pub fn runtime_failure_return_report(self) -> Option<RuntimeFailureReturnReport> {
        let failure_return = self.failure_return_summary();
        if failure_return.can_return_runtime_failure() {
            self.primary_failure_report().map(|failure| {
                RuntimeFailureReturnReport::new(
                    RuntimeFailureReturnSource::KvSideEffectCommit,
                    failure,
                    self.failure_batch,
                    self.failure_report_count,
                    self.can_format_runtime_failures,
                    self.total_blocker_component_count,
                )
            })
        } else {
            None
        }
    }

    pub fn commit_decision_accounting_is_consistent(self) -> bool {
        self.can_commit == self.readiness.can_commit_runtime_kv_side_effects()
            && self.should_return_failure == (!self.can_commit && self.failure_report_count > 0)
            && self.action == self.readiness.runtime_kv_side_effect_commit_action()
            && self.action_can_commit() == self.can_commit
            && self.action_should_return_failure() == self.should_return_failure
            && self.import_commit_action == self.readiness.import_commit_action
            && self.manifest_boundary_commit_action
                == self.readiness.manifest_boundary_commit_action
            && self.export_commit_action == self.readiness.export_commit_action
            && self.readiness.child_commit_actions_match_readiness()
            && self.first_unready_stage == self.readiness.first_unready_stage()
            && self.first_blocking_stage == self.readiness.first_blocking_stage()
            && self.first_problem_kind == self.readiness.first_problem_kind()
            && self.primary_failure_summary == self.readiness.primary_failure_summary()
            && self.failure_batch == self.readiness.failure_batch_summary()
            && self.failure_report_count == self.readiness.failure_report_count()
            && self.failure_report_count == self.failure_reports().len()
            && self.can_format_runtime_failures == self.readiness.can_format_runtime_failures()
            && self.total_signal_component_count == self.readiness.total_signal_component_count
            && self.total_blocker_component_count == self.readiness.total_blocker_component_count
            && self.component_accounting_consistent
                == self
                    .readiness
                    .runtime_kv_side_effect_accounting_is_consistent()
    }

    pub fn can_commit_runtime_kv_side_effects(self) -> bool {
        self.can_commit && self.commit_decision_accounting_is_consistent()
    }

    pub fn should_return_runtime_failure(self) -> bool {
        self.should_return_failure && self.commit_decision_accounting_is_consistent()
    }
}

impl RuntimeFailureReturnSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::BoundaryCommit => "boundary_commit",
            Self::ManifestBoundaryCommit => "manifest_boundary_commit",
            Self::KvSideEffectCommit => "kv_side_effect_commit",
        }
    }
}

impl RuntimeFailureReturnSummary {
    pub fn new(
        source: RuntimeFailureReturnSource,
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

impl RuntimeFailureReturnReport {
    pub fn new(
        source: RuntimeFailureReturnSource,
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

    pub fn can_use_runtime_failure_return_report(&self) -> bool {
        self.failure_return_report_shape_is_clean()
    }
}

impl RuntimeBoundaryAcceptanceSummary {
    pub fn has_request_failures(self) -> bool {
        self.request.total_violation_count() > 0
    }

    pub fn has_response_failures(self) -> bool {
        self.response.total_violation_count() > 0
    }

    pub fn has_kv_failures(self) -> bool {
        self.request.has_imported_kv_failures() || self.response.has_exported_kv_failures()
    }

    pub fn has_request_parity_failures(self) -> bool {
        self.response.has_request_parity_failures()
    }

    pub fn has_failures(self) -> bool {
        self.has_request_failures()
            || self.has_response_failures()
            || self.has_kv_failures()
            || self.has_request_parity_failures()
    }

    pub fn request_acceptance_failure_component_count(self) -> usize {
        usize::from(self.has_request_failures())
    }

    pub fn response_acceptance_failure_component_count(self) -> usize {
        usize::from(self.has_response_failures())
    }

    pub fn kv_failure_component_count(self) -> usize {
        usize::from(self.has_kv_failures())
    }

    pub fn request_parity_failure_component_count(self) -> usize {
        usize::from(self.has_request_parity_failures())
    }

    pub fn boundary_failure_component_count(self) -> usize {
        self.request_acceptance_failure_component_count()
            .saturating_add(self.response_acceptance_failure_component_count())
            .saturating_add(self.kv_failure_component_count())
            .saturating_add(self.request_parity_failure_component_count())
    }

    pub fn has_failure_reports(self) -> bool {
        self.total_failure_report_count > 0
    }

    pub fn boundary_acceptance_problem_component_count(self) -> usize {
        self.boundary_failure_component_count()
            .saturating_add(usize::from(self.has_failure_reports()))
    }

    pub fn has_boundary_acceptance_problem_components(self) -> bool {
        self.boundary_acceptance_problem_component_count() > 0
    }

    pub fn failure_report_matches_parts(self) -> bool {
        self.total_failure_report_count
            == self
                .request
                .failure_report_count
                .saturating_add(self.response.failure_report_count)
    }

    pub fn total_violation_matches_parts(self) -> bool {
        self.total_violation_count
            == self
                .request
                .total_violation_count()
                .saturating_add(self.response.total_violation_count())
    }

    pub fn boundary_acceptance_accounting_is_consistent(self) -> bool {
        let expected_failure_count = self
            .request_acceptance_failure_component_count()
            .saturating_add(self.response_acceptance_failure_component_count())
            .saturating_add(self.kv_failure_component_count())
            .saturating_add(self.request_parity_failure_component_count());
        let expected_problem_count =
            expected_failure_count.saturating_add(usize::from(self.has_failure_reports()));

        self.boundary_failure_component_count() == expected_failure_count
            && self.boundary_acceptance_problem_component_count() == expected_problem_count
            && self.has_boundary_acceptance_problem_components() == (expected_problem_count > 0)
            && self.has_failures() == (expected_failure_count > 0)
            && self.total_violation_matches_parts()
            && self.failure_report_matches_parts()
            && self.request.request_acceptance_accounting_is_consistent()
            && self.response.response_acceptance_accounting_is_consistent()
            && self.accepted == (self.request.accepted && self.response.accepted)
    }

    pub fn is_clean_acceptance(self) -> bool {
        self.accepted
            && !self.has_failures()
            && self.total_violation_count == 0
            && self.total_failure_report_count == 0
            && self.total_violation_matches_parts()
            && self.request.is_clean_acceptance()
            && self.response.is_clean_acceptance()
            && self.boundary_acceptance_accounting_is_consistent()
    }

    pub fn runtime_boundary_acceptance_commit_signal_component_count(self) -> usize {
        self.request
            .runtime_request_acceptance_commit_signal_component_count()
            .saturating_add(
                self.response
                    .runtime_response_acceptance_commit_signal_component_count(),
            )
            .saturating_add(usize::from(self.accepted))
    }

    pub fn has_runtime_boundary_acceptance_commit_signals(self) -> bool {
        self.runtime_boundary_acceptance_commit_signal_component_count() > 0
    }

    pub fn runtime_boundary_acceptance_commit_blocker_component_count(self) -> usize {
        self.request
            .runtime_request_acceptance_commit_blocker_component_count()
            .saturating_add(
                self.response
                    .runtime_response_acceptance_commit_blocker_component_count(),
            )
            .saturating_add(self.boundary_acceptance_problem_component_count())
    }

    pub fn has_runtime_boundary_acceptance_commit_blockers(self) -> bool {
        self.runtime_boundary_acceptance_commit_blocker_component_count() > 0
    }

    pub fn runtime_boundary_acceptance_commit_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .request
            .runtime_request_acceptance_commit_signal_component_count()
            .saturating_add(
                self.response
                    .runtime_response_acceptance_commit_signal_component_count(),
            )
            .saturating_add(usize::from(self.accepted));
        let expected_blocker_count = self
            .request
            .runtime_request_acceptance_commit_blocker_component_count()
            .saturating_add(
                self.response
                    .runtime_response_acceptance_commit_blocker_component_count(),
            )
            .saturating_add(self.boundary_acceptance_problem_component_count());

        self.boundary_acceptance_accounting_is_consistent()
            && self
                .request
                .runtime_request_acceptance_commit_accounting_is_consistent()
            && self
                .response
                .runtime_response_acceptance_commit_accounting_is_consistent()
            && self.runtime_boundary_acceptance_commit_signal_component_count()
                == expected_signal_count
            && self.has_runtime_boundary_acceptance_commit_signals() == (expected_signal_count > 0)
            && self.runtime_boundary_acceptance_commit_blocker_component_count()
                == expected_blocker_count
            && self.has_runtime_boundary_acceptance_commit_blockers()
                == (expected_blocker_count > 0)
    }

    pub fn runtime_boundary_acceptance_commit_is_clean(self) -> bool {
        self.runtime_boundary_acceptance_commit_blocker_component_count() == 0
            && self.runtime_boundary_acceptance_commit_accounting_is_consistent()
    }

    pub fn can_commit_runtime_boundary_acceptance(self) -> bool {
        self.accepted
            && self.request.can_commit_runtime_request_acceptance()
            && self.response.can_commit_runtime_response_acceptance()
            && self.runtime_boundary_acceptance_commit_is_clean()
    }
}

impl RuntimeAcceptanceContext {
    pub fn new(
        request: RuntimeRequestEnvelope,
        hardware: HardwarePlan,
        imported_kv_blocks: impl Into<Vec<KvBlock>>,
    ) -> Self {
        Self {
            request,
            hardware,
            imported_kv_blocks: imported_kv_blocks.into(),
        }
    }

    pub fn from_request_parts(
        request: &InferenceRequest,
        architecture: TransformerRuntimeArchitecture,
        route_budget: RouteBudget,
        hierarchy: HierarchyWeights,
        transformer_plan: &TransformerPlanDigest,
        hardware: HardwarePlan,
        imported_kv_blocks: impl Into<Vec<KvBlock>>,
    ) -> Self {
        let imported_kv_blocks = imported_kv_blocks.into();
        let execution = hardware
            .adapter_execution_context()
            .clamp_for_runtime(&request.runtime);
        let request = RuntimeRequestEnvelope::from_parts(
            request,
            architecture,
            route_budget,
            hierarchy,
            transformer_plan,
            &execution,
            imported_kv_blocks.len(),
        );

        Self::new(request, hardware, imported_kv_blocks)
    }

    pub fn with_planning_digest(mut self, planning: RuntimePlanningDigest) -> Self {
        self.request = self.request.with_planning_digest(planning);
        self
    }

    pub fn with_recursive_schedule(mut self, recursive: RecursiveScheduleSummary) -> Self {
        self.request = self.request.with_recursive_schedule(recursive);
        self
    }

    pub fn request(&self) -> &RuntimeRequestEnvelope {
        &self.request
    }

    pub fn hardware(&self) -> &HardwarePlan {
        &self.hardware
    }

    pub fn imported_kv_blocks(&self) -> &[KvBlock] {
        &self.imported_kv_blocks
    }

    pub fn runtime_diagnostics_seed(&self) -> RuntimeDiagnostics {
        RuntimeDiagnostics::from_request_envelope(&self.request)
    }

    pub fn inference_diagnostics_seed(&self) -> InferenceDiagnostics {
        InferenceDiagnostics::from_request_envelope(&self.request).with_hardware(
            self.hardware.pressure,
            self.hardware.compute_headroom(),
            self.hardware.latency_budget_ms,
        )
    }

    pub fn runtime_reported_device_execution_envelope_summary(
        &self,
        runtime: &RuntimeDiagnostics,
    ) -> RuntimeDeviceExecutionEnvelopeSummary {
        let execution = self
            .hardware
            .adapter_execution_context()
            .clamp_for_runtime(&self.request.runtime);
        runtime.device_execution_envelope_summary(
            &self.request.runtime,
            self.request.architecture,
            &execution,
            &self.hardware,
        )
    }

    pub fn runtime_boundary_device_execution_readiness_summary(
        &self,
        runtime: &RuntimeDiagnostics,
    ) -> RuntimeBoundaryDeviceExecutionReadinessSummary {
        RuntimeBoundaryDeviceExecutionReadinessSummary::new(
            runtime.can_admit_runtime_reported_device_execution_metadata(),
            self.runtime_reported_device_execution_envelope_summary(runtime),
        )
    }

    pub fn runtime_boundary_device_execution_commit_summary(
        &self,
        runtime: &RuntimeDiagnostics,
    ) -> RuntimeBoundaryDeviceExecutionCommitSummary {
        self.runtime_boundary_device_execution_readiness_summary(runtime)
            .commit_summary()
    }

    pub fn can_submit_runtime_reported_device_execution_envelope(
        &self,
        runtime: &RuntimeDiagnostics,
    ) -> bool {
        self.runtime_boundary_device_execution_readiness_summary(runtime)
            .can_commit_runtime_boundary_device_execution()
    }

    pub fn request_acceptance_report(&self) -> RuntimeRequestAcceptanceReport {
        self.request.acceptance_report(&self.imported_kv_blocks)
    }

    pub fn request_acceptance_summary(&self) -> RuntimeRequestAcceptanceSummary {
        self.request_acceptance_report().acceptance_summary()
    }

    pub fn request_gate_summary(&self) -> RuntimeRequestGateSummary {
        self.request.request_gate_summary(&self.imported_kv_blocks)
    }

    pub fn request_planning_readiness_summary(
        &self,
        runtime_planning: RuntimePlanningReadinessSummary,
    ) -> RuntimeRequestPlanningReadinessSummary {
        self.request
            .request_planning_readiness_summary(runtime_planning, &self.imported_kv_blocks)
    }

    pub fn can_commit_request_planning_with_committed_parts(
        &self,
        runtime_planning: RuntimePlanningReadinessSummary,
    ) -> bool {
        self.request_planning_readiness_summary(runtime_planning)
            .can_commit_runtime_request_planning_with_committed_parts()
    }

    pub fn manifest_request_planning_readiness_summary(
        &self,
        runtime_planning: RuntimePlanningReadinessSummary,
        manifest: &RuntimeManifestDigest,
    ) -> Option<RuntimeRequestManifestPlanningReadinessSummary> {
        self.request.manifest_request_planning_readiness_summary(
            runtime_planning,
            manifest,
            &self.imported_kv_blocks,
        )
    }

    pub fn request_envelope_summary(&self) -> RuntimeRequestEnvelopeSummary {
        self.request.envelope_summary()
    }

    pub fn response_envelope(&self, outcome: &InferenceOutcome) -> RuntimeResponseEnvelope {
        RuntimeResponseEnvelope::from_outcome(outcome)
    }

    pub fn response_envelope_summary(
        &self,
        outcome: &InferenceOutcome,
    ) -> RuntimeResponseEnvelopeSummary {
        self.response_envelope(outcome).envelope_summary()
    }

    pub fn boundary_envelope_summary(
        &self,
        outcome: &InferenceOutcome,
    ) -> RuntimeBoundaryEnvelopeSummary {
        RuntimeBoundaryEnvelopeSummary {
            request: self.request_envelope_summary(),
            response: self.response_envelope_summary(outcome),
        }
    }

    pub fn boundary_adapter_summary(
        &self,
        outcome: &InferenceOutcome,
    ) -> RuntimeBoundaryAdapterSummary {
        let execution = self
            .hardware
            .adapter_execution_context()
            .clamp_for_runtime(&self.request.runtime);
        let runtime_selected_adapter = outcome.diagnostics.runtime.selected_adapter;
        let selection = self.request.planning.map(|planning| {
            execution.selection_runtime_summary(
                planning.adapter_selection_report,
                runtime_selected_adapter,
            )
        });
        let request_selected_adapter = self.request.selected_adapter;
        let request_adapter_reported = request_selected_adapter.is_some();
        let runtime_adapter_reported = runtime_selected_adapter.is_some();
        let runtime_adapter_matches_request =
            runtime_adapter_reported && runtime_selected_adapter == request_selected_adapter;
        let runtime_adapter_allowed = runtime_selected_adapter
            .map(|adapter| execution.adapters.contains(&adapter))
            .unwrap_or(false);

        RuntimeBoundaryAdapterSummary {
            request_selected_adapter,
            runtime_selected_adapter,
            adapter_candidate_count: self.request.adapter_count,
            has_planning_selection: selection.is_some(),
            selection,
            request_adapter_reported,
            runtime_adapter_reported,
            runtime_adapter_matches_request,
            runtime_adapter_allowed,
        }
    }

    pub fn boundary_kv_summary(&self, outcome: &InferenceOutcome) -> RuntimeBoundaryKvSummary {
        let request_import_report = self.request_acceptance_report();
        let request_import_summary: RuntimeKvValidationSummary = request_import_report
            .imported_kv_report
            .validation_summary();
        let response = self.response_envelope(outcome);
        let exported_report = response.validate_exported_kv_blocks(outcome, &self.request);
        let exported_summary = exported_report.validation_summary();
        let planned_kv = self
            .request
            .planning
            .map(|planning| planning.planned_kv_exchange());

        RuntimeBoundaryKvSummary {
            request_imported_kv_blocks: self.request.imported_kv_blocks,
            concrete_imported_kv_blocks: self.imported_kv_blocks.len(),
            accepted_imported_kv_blocks: request_import_summary.accepted_count,
            imported_kv_violation_count: request_import_summary.violation_count,
            response_imported_kv_blocks: response.imported_kv_blocks,
            response_exported_kv_blocks: response.exported_kv_blocks,
            diagnostics_imported_kv_blocks: response.diagnostics_imported_kv_blocks,
            diagnostics_exported_kv_blocks: response.diagnostics_exported_kv_blocks,
            diagnostics_weak_runtime_kv_imports_skipped: response
                .diagnostics_weak_runtime_kv_imports_skipped,
            accepted_exported_kv_blocks: exported_summary.accepted_count,
            exported_kv_violation_count: exported_summary.violation_count,
            imported_namespace_counts: KvNamespaceCounts::from_blocks(&self.imported_kv_blocks),
            exported_namespace_counts: KvNamespaceCounts::from_blocks(&outcome.exported_kv),
            runtime_import_enabled: self.request.runtime.supports_kv_import,
            runtime_export_enabled: self.request.runtime.supports_kv_export,
            runtime_max_import_blocks: self.request.runtime.max_kv_import_blocks,
            runtime_max_export_blocks: self.request.runtime.max_kv_export_blocks,
            planned_imported_kv_blocks: planned_kv.map(|planned| planned.import_blocks),
            planned_exported_kv_blocks: planned_kv.map(|planned| planned.export_blocks),
        }
    }

    pub fn response_acceptance_report(
        &self,
        outcome: &InferenceOutcome,
    ) -> RuntimeResponseAcceptanceReport {
        self.response_envelope(outcome)
            .acceptance_report(outcome, &self.request, &self.hardware)
    }

    pub fn response_acceptance_summary(
        &self,
        outcome: &InferenceOutcome,
    ) -> RuntimeResponseAcceptanceSummary {
        self.response_acceptance_report(outcome)
            .acceptance_summary()
    }

    pub fn response_gate_summary(&self, outcome: &InferenceOutcome) -> RuntimeResponseGateSummary {
        self.response_envelope(outcome).response_gate_summary(
            outcome,
            &self.request,
            &self.hardware,
        )
    }

    pub fn response_manifest_kv_summary(
        &self,
        outcome: &InferenceOutcome,
        manifest_kv_bridge: RuntimePlanningManifestKvBridgeSummary,
    ) -> RuntimeResponseManifestKvSummary {
        self.response_envelope(outcome)
            .request_parity_summary(outcome, &self.request)
            .manifest_kv_summary(manifest_kv_bridge)
    }

    pub fn manifest_boundary_kv_summary(
        &self,
        outcome: &InferenceOutcome,
        runtime_planning: RuntimePlanningReadinessSummary,
        manifest: &RuntimeManifestDigest,
    ) -> Option<RuntimeManifestBoundaryKvSummary> {
        let request_manifest_planning =
            self.manifest_request_planning_readiness_summary(runtime_planning, manifest)?;
        let response_manifest_kv = self
            .response_manifest_kv_summary(outcome, request_manifest_planning.manifest_kv_bridge);

        Some(RuntimeManifestBoundaryKvSummary::new(
            request_manifest_planning,
            response_manifest_kv,
        ))
    }

    pub fn manifest_boundary_commit_readiness_summary(
        &self,
        outcome: &InferenceOutcome,
        runtime_planning: RuntimePlanningReadinessSummary,
        manifest: &RuntimeManifestDigest,
    ) -> Option<RuntimeManifestBoundaryCommitReadinessSummary> {
        let boundary_commit = self.boundary_commit_readiness_summary(outcome);
        let manifest_boundary_kv =
            self.manifest_boundary_kv_summary(outcome, runtime_planning, manifest)?;

        Some(RuntimeManifestBoundaryCommitReadinessSummary::new(
            boundary_commit,
            manifest_boundary_kv,
        ))
    }

    pub fn response_readiness_summary(
        &self,
        outcome: &InferenceOutcome,
    ) -> RuntimeResponseReadinessSummary {
        let envelope = self.response_envelope(outcome);

        RuntimeResponseReadinessSummary::new(
            envelope.envelope_summary(),
            envelope.request_parity_summary(outcome, &self.request),
            envelope.response_gate_summary(outcome, &self.request, &self.hardware),
        )
    }

    pub fn boundary_acceptance_summary(
        &self,
        outcome: &InferenceOutcome,
    ) -> RuntimeBoundaryAcceptanceSummary {
        let request = self.request_acceptance_summary();
        let response = self.response_acceptance_summary(outcome);

        RuntimeBoundaryAcceptanceSummary {
            accepted: request.accepted && response.accepted,
            request,
            response,
            total_violation_count: request
                .total_violation_count()
                .saturating_add(response.total_violation_count()),
            total_failure_report_count: request
                .failure_report_count
                .saturating_add(response.failure_report_count),
        }
    }

    pub fn boundary_gate_summary(&self, outcome: &InferenceOutcome) -> RuntimeBoundaryGateSummary {
        let acceptance = self.boundary_acceptance_summary(outcome);
        let envelope = self.boundary_envelope_summary(outcome);
        let adapter = self.boundary_adapter_summary(outcome);
        let kv = self.boundary_kv_summary(outcome);
        let request_gate = self.request_gate_summary();
        let response_gate = self.response_gate_summary(outcome);

        RuntimeBoundaryGateSummary {
            request_accepted: request_gate.request_accepted,
            response_accepted: response_gate.response_accepted,
            envelope_consistent: envelope.boundary_envelope_is_consistent(),
            adapter_consistent: adapter.adapter_boundary_is_consistent(),
            kv_consistent: kv.kv_boundary_is_consistent(),
            request_backend_wire_problem_count: request_gate.backend_wire_problem_count,
            request_planning_pre_request_problem_count: request_gate
                .planning_pre_request_problem_count,
            request_planning_pressure_signal_count: request_gate.planning_pressure_signal_count,
            request_planning_dense_compute_avoided_tokens: request_gate
                .planning_dense_compute_avoided_tokens,
            response_wire_problem_count: response_gate.response_wire_problem_count,
            planning_pre_request_problem_count: response_gate.planning_pre_request_problem_count,
            planning_pressure_signal_count: response_gate.planning_pressure_signal_count,
            response_planning_dense_compute_avoided_tokens: response_gate
                .planning_dense_compute_avoided_tokens,
            kv_boundary_signal_count: kv.kv_boundary_signal_component_count(),
            response_uncertainty_coverage_signal_count: envelope
                .response_uncertainty_coverage_signal_component_count(),
            response_uncertainty_metric_problem_count: envelope
                .response_uncertainty_metric_problem_component_count(),
            response_uncertainty_accounting_consistent: envelope
                .response_uncertainty_accounting_is_consistent(),
            total_violation_count: acceptance.total_violation_count,
            total_failure_report_count: acceptance.total_failure_report_count,
        }
    }

    pub fn boundary_commit_readiness_summary(
        &self,
        outcome: &InferenceOutcome,
    ) -> RuntimeBoundaryCommitReadinessSummary {
        let acceptance = self.boundary_acceptance_summary(outcome);
        let envelope = self.boundary_envelope_summary(outcome);
        let adapter = self.boundary_adapter_summary(outcome);
        let kv = self.boundary_kv_summary(outcome);
        let gate = self.boundary_gate_summary(outcome);

        let request_acceptance_signal_component_count = acceptance
            .request
            .runtime_request_acceptance_commit_signal_component_count();
        let response_acceptance_signal_component_count = acceptance
            .response
            .runtime_response_acceptance_commit_signal_component_count();
        let boundary_acceptance_signal_component_count = usize::from(acceptance.accepted);
        let acceptance_signal_component_count = request_acceptance_signal_component_count
            .saturating_add(response_acceptance_signal_component_count)
            .saturating_add(boundary_acceptance_signal_component_count);
        let boundary_envelope_signal_component_count =
            envelope.runtime_boundary_envelope_commit_signal_component_count();
        let boundary_adapter_signal_component_count =
            adapter.adapter_boundary_commit_signal_component_count();
        let boundary_kv_signal_component_count = kv.kv_boundary_signal_component_count();
        let boundary_gate_signal_component_count =
            gate.runtime_boundary_commit_signal_component_count();
        let planning_dense_compute_avoided_tokens = gate.planning_dense_compute_avoided_tokens();
        let runtime_response_signal_component_count =
            usize::from(gate.can_commit_runtime_response());
        let envelope_signal_component_count = boundary_envelope_signal_component_count;
        let adapter_signal_component_count = boundary_adapter_signal_component_count;
        let kv_signal_component_count = boundary_kv_signal_component_count;
        let gate_signal_component_count = boundary_gate_signal_component_count;
        let total_signal_component_count = acceptance_signal_component_count
            .saturating_add(envelope_signal_component_count)
            .saturating_add(adapter_signal_component_count)
            .saturating_add(kv_signal_component_count)
            .saturating_add(gate_signal_component_count)
            .saturating_add(runtime_response_signal_component_count);

        let request_acceptance_blocker_component_count = acceptance
            .request
            .runtime_request_acceptance_commit_blocker_component_count();
        let response_acceptance_blocker_component_count = acceptance
            .response
            .runtime_response_acceptance_commit_blocker_component_count();
        let boundary_acceptance_blocker_component_count =
            acceptance.boundary_acceptance_problem_component_count();
        let acceptance_blocker_component_count = request_acceptance_blocker_component_count
            .saturating_add(response_acceptance_blocker_component_count)
            .saturating_add(boundary_acceptance_blocker_component_count);
        let boundary_envelope_blocker_component_count =
            envelope.runtime_boundary_envelope_commit_blocker_component_count();
        let boundary_adapter_blocker_component_count =
            adapter.adapter_boundary_commit_blocker_component_count();
        let boundary_kv_blocker_component_count = kv.kv_boundary_problem_component_count();
        let boundary_gate_blocker_component_count =
            gate.runtime_boundary_commit_blocker_component_count();
        let runtime_response_blocker_component_count =
            usize::from(!gate.can_commit_runtime_response());
        let envelope_blocker_component_count = boundary_envelope_blocker_component_count;
        let adapter_blocker_component_count = boundary_adapter_blocker_component_count;
        let kv_blocker_component_count = boundary_kv_blocker_component_count;
        let gate_blocker_component_count = boundary_gate_blocker_component_count;
        let total_blocker_component_count = acceptance_blocker_component_count
            .saturating_add(envelope_blocker_component_count)
            .saturating_add(adapter_blocker_component_count)
            .saturating_add(kv_blocker_component_count)
            .saturating_add(gate_blocker_component_count)
            .saturating_add(runtime_response_blocker_component_count);

        RuntimeBoundaryCommitReadinessSummary {
            request_acceptance_ready: acceptance.request.can_commit_runtime_request_acceptance(),
            response_acceptance_ready: acceptance.response.can_commit_runtime_response_acceptance(),
            boundary_acceptance_ready: acceptance.can_commit_runtime_boundary_acceptance(),
            boundary_envelope_ready: envelope.can_commit_runtime_boundary_envelope(),
            boundary_adapter_ready: adapter.can_commit_runtime_boundary_adapter(),
            boundary_kv_ready: kv.can_use_runtime_boundary_kv(),
            boundary_gate_ready: gate.can_commit_runtime_boundary(),
            runtime_response_ready: gate.can_commit_runtime_response(),
            request_acceptance_signal_component_count,
            response_acceptance_signal_component_count,
            boundary_acceptance_signal_component_count,
            acceptance_signal_component_count,
            boundary_envelope_signal_component_count,
            boundary_adapter_signal_component_count,
            boundary_kv_signal_component_count,
            boundary_gate_signal_component_count,
            planning_dense_compute_avoided_tokens,
            envelope_signal_component_count,
            adapter_signal_component_count,
            kv_signal_component_count,
            gate_signal_component_count,
            runtime_response_signal_component_count,
            total_signal_component_count,
            request_acceptance_blocker_component_count,
            response_acceptance_blocker_component_count,
            boundary_acceptance_blocker_component_count,
            acceptance_blocker_component_count,
            boundary_envelope_blocker_component_count,
            boundary_adapter_blocker_component_count,
            boundary_kv_blocker_component_count,
            boundary_gate_blocker_component_count,
            envelope_blocker_component_count,
            adapter_blocker_component_count,
            kv_blocker_component_count,
            gate_blocker_component_count,
            runtime_response_blocker_component_count,
            total_blocker_component_count,
            component_accounting_consistent: acceptance
                .runtime_boundary_acceptance_commit_accounting_is_consistent()
                && envelope.runtime_boundary_envelope_commit_accounting_is_consistent()
                && adapter.adapter_boundary_commit_accounting_is_consistent()
                && kv.has_kv_boundary_signals() == (kv_signal_component_count > 0)
                && kv.has_kv_boundary_problem_components() == (kv_blocker_component_count > 0)
                && gate.runtime_boundary_commit_accounting_is_consistent(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::{
        AdapterFallbackReason, AdapterSelection, AdapterSelectionRuntimeSummary, RuntimeAdapter,
    };
    use crate::attention::{
        AttentionCandidateBatchSummary, AttentionDecisionSummary,
        AttentionSelectionReadinessSummary,
    };
    use crate::diagnostics::{DeviceExecutionSource, InferenceDiagnostics, RuntimeDiagnostics};
    use crate::engine::{GeneratedToken, InferenceOutcome, InferenceRequest, RuntimeFailureKind};
    use crate::experiment::ExperimentSwitches;
    use crate::fht_dke::{
        DeterministicFhtDkeBudgeter, FhtDkeBudgetSummary, FhtDkePlanningReadinessSummary,
    };
    use crate::hardware::{DeviceClass, HardwareAllocator, HardwareLoadSnapshot};
    use crate::kv::{
        KvNamespace, RuntimeKvCandidate, RuntimeKvImportBlockSummary, RuntimeKvImportPlan,
        RuntimeKvImportReadinessSummary,
    };
    use crate::manifest::{RuntimeKvPolicy, RuntimeManifestDigest, TransformerRuntimeArchitecture};
    use crate::planning::{
        RuntimePlanningKvClampReason, RuntimePlanningKvClampSummary, RuntimePlanningKvExchange,
        RuntimePlanningReadinessSummary, RuntimePlanningSummary,
    };
    use crate::profile::{HierarchyWeights, TaskProfile};
    use crate::request::{
        RuntimeRequestEnvelope, RuntimeRequestManifestPlanningReadinessStage,
        RuntimeRequestPlanningReadinessStage,
    };
    use crate::response::RuntimeResponseReadinessStage;
    use crate::router::{
        RouteBudget, RouteBudgetReadinessSummary, RouteLayerCounts, RoutingDecisionSummary,
    };
    use crate::runtime::{RuntimeGenerationBudget, RuntimeMetadata};
    use crate::transformer::{
        RuntimeKvExportPlan, RuntimeKvExportPlanningSummary, RuntimeKvExportReadinessSummary,
        RuntimeKvExportSummary, TransformerAttentionKind, TransformerForwardBatchSummary,
        TransformerForwardSummary, TransformerLayerBudget, TransformerPlanCounts,
        TransformerPlanDigest, TransformerPlanSummary, TransformerPlanningPressureSummary,
        TransformerPlanningReadinessSummary,
    };

    #[test]
    fn acceptance_context_reuses_saved_request_and_hardware_for_response_gate() {
        let metadata = RuntimeMetadata::new("model", "tok", 128, 16)
            .with_kv_exchange(true, true)
            .with_kv_limits(1, 1);
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(16)
            .with_max_tokens(2)
            .with_runtime(metadata);
        let architecture = TransformerRuntimeArchitecture::new(1, 16, 2, 1, 64);
        let transformer_plan = TransformerPlanDigest::new(
            Some("acceptance-context"),
            vec![TransformerLayerBudget::new(
                0,
                TransformerAttentionKind::Global,
                0.5,
                64,
            )],
        );
        let hardware = HardwareAllocator::new().plan(
            HardwareLoadSnapshot::new(DeviceClass::DiscreteGpu, 0.1, 0.1, 0.1, 0.1),
            TaskProfile::Coding,
            512,
            HierarchyWeights::for_profile(TaskProfile::Coding),
        );
        let execution = hardware
            .adapter_execution_context()
            .clamp_for_runtime(&request.runtime);
        let planning = RuntimePlanningDigest::from_request(
            &request,
            RouteBudget::default(),
            &execution,
            &[],
            &DeterministicFhtDkeBudgeter::default(),
        );
        let imported = runtime_block(1);
        let context = RuntimeAcceptanceContext::from_request_parts(
            &request,
            architecture,
            RouteBudget::default(),
            HierarchyWeights::for_profile(TaskProfile::Coding),
            &transformer_plan,
            hardware,
            vec![imported.clone()],
        );
        let request_context = context.clone().with_planning_digest(planning);
        let runtime_seed = request_context.runtime_diagnostics_seed();
        assert_eq!(runtime_seed.model_id.as_deref(), Some("model"));
        assert_eq!(runtime_seed.selected_adapter, Some(RuntimeAdapter::Cuda));
        assert_eq!(runtime_seed.layer_count, 1);
        assert_eq!(runtime_seed.hidden_size, 16);
        assert_eq!(runtime_seed.local_window_tokens, 64);
        assert_eq!(runtime_seed.imported_kv_blocks, 1);

        let diagnostics_seed = request_context.inference_diagnostics_seed();
        let diagnostics_parity = diagnostics_seed.request_parity_summary(request_context.request());
        assert!(diagnostics_parity.can_accept_inference_diagnostics_request_parity());
        assert_eq!(
            diagnostics_seed.generation_budget,
            Some(request_context.request().generation_budget)
        );
        assert_eq!(
            diagnostics_seed.hardware_pressure,
            context.hardware.pressure
        );
        assert_eq!(
            diagnostics_seed.compute_headroom,
            context.hardware.compute_headroom()
        );
        assert_eq!(
            diagnostics_seed.latency_budget_ms,
            context.hardware.latency_budget_ms
        );

        let runtime = request_context
            .runtime_diagnostics_seed()
            .with_device_execution(
                "gpu",
                "gpu",
                "cpu",
                "gpu-resident",
                DeviceExecutionSource::RuntimeReported,
            );
        let planned_runtime = runtime.clone().with_kv_exchange(1, 0);
        let runtime = runtime.with_kv_exchange(1, 1);
        let mut planned_outcome = InferenceOutcome::empty().with_diagnostics(
            request_context
                .inference_diagnostics_seed()
                .with_runtime(planned_runtime),
        );
        planned_outcome.answer = "ok".to_owned();
        planned_outcome.tokens.push(GeneratedToken::new("o"));
        planned_outcome.tokens.push(GeneratedToken::new("k"));
        planned_outcome.imported_kv.push(imported.clone());
        let mut outcome = InferenceOutcome::empty().with_diagnostics(
            request_context
                .inference_diagnostics_seed()
                .with_runtime(runtime),
        );
        outcome.answer = "ok".to_owned();
        outcome.tokens.push(GeneratedToken::new("o"));
        outcome.tokens.push(GeneratedToken::new("k"));
        outcome.imported_kv.push(imported);
        outcome.exported_kv.push(runtime_block(2));

        assert!(context.request_acceptance_report().is_accepted());
        assert!(context.request_acceptance_summary().accepted);
        assert_eq!(context.imported_kv_blocks().len(), 1);
        let request_gate = context.request_gate_summary();
        assert_eq!(
            request_gate,
            context
                .request()
                .request_gate_summary(context.imported_kv_blocks())
        );
        assert!(request_gate.can_send_request());
        assert_eq!(request_gate.backend_wire_problem_count, 0);
        let runtime_planning = clean_runtime_planning_readiness();
        let request_readiness =
            request_context.request_planning_readiness_summary(runtime_planning);
        assert_eq!(
            RuntimeRequestPlanningReadinessSummary::stage_order(),
            [
                RuntimeRequestPlanningReadinessStage::RuntimePlanning,
                RuntimeRequestPlanningReadinessStage::RequestPlanningParity,
                RuntimeRequestPlanningReadinessStage::RequestGate,
            ]
        );
        assert!(request_readiness.runtime_planning_ready());
        assert!(request_readiness.request_planning_ready());
        assert!(request_readiness.request_gate_ready());
        assert_eq!(request_readiness.first_unready_stage(), None);
        assert_eq!(request_readiness.first_blocking_stage(), None);
        assert_eq!(
            request_readiness,
            request_context
                .request()
                .request_planning_readiness_summary(
                    runtime_planning,
                    request_context.imported_kv_blocks(),
                )
        );
        assert!(request_readiness.can_commit_runtime_request_planning());
        let planned_kv = planning.planned_kv_exchange();
        let manifest =
            RuntimeManifestDigest::from_metadata(request_context.request().runtime.clone())
                .with_architecture(architecture)
                .with_kv_policy(
                    RuntimeKvPolicy::import_export()
                        .with_limits(planned_kv.import_blocks, planned_kv.export_blocks),
                );
        let manifest_request_readiness = request_context
            .manifest_request_planning_readiness_summary(runtime_planning, &manifest)
            .expect("planning digest is attached");
        assert_eq!(
            manifest_request_readiness,
            request_context
                .request()
                .manifest_request_planning_readiness_summary(
                    runtime_planning,
                    &manifest,
                    request_context.imported_kv_blocks(),
                )
                .expect("planning digest is attached")
        );
        assert!(manifest_request_readiness.manifest_kv_bridge_ready());
        assert!(manifest_request_readiness.request_planning_ready());
        assert!(manifest_request_readiness.can_commit_manifest_request_planning());
        let planned_response_report = request_context.response_acceptance_report(&planned_outcome);
        assert!(
            planned_response_report.is_accepted(),
            "{planned_response_report:?}"
        );
        assert!(
            request_context
                .response_readiness_summary(&planned_outcome)
                .can_commit_runtime_response_readiness()
        );
        assert!(context.response_acceptance_report(&outcome).is_accepted());
        assert!(context.response_acceptance_summary(&outcome).accepted);
        let response_gate = context.response_gate_summary(&outcome);
        assert!(response_gate.response_accepted);
        assert!(response_gate.envelope_consistent);
        assert!(response_gate.request_parity_consistent);
        assert!(response_gate.exported_kv_accepted);
        assert_eq!(response_gate.accepted_exported_kv_blocks, 1);
        assert!(response_gate.can_accept_response());
        assert_eq!(
            response_gate,
            context.response_envelope(&outcome).response_gate_summary(
                &outcome,
                context.request(),
                context.hardware()
            )
        );
        let response_readiness = context.response_readiness_summary(&outcome);
        assert_eq!(
            RuntimeResponseReadinessSummary::stage_order(),
            [
                RuntimeResponseReadinessStage::ResponseEnvelope,
                RuntimeResponseReadinessStage::ResponseRequestParity,
                RuntimeResponseReadinessStage::ResponseGate,
            ]
        );
        assert!(response_readiness.response_envelope_ready());
        assert!(response_readiness.response_request_ready());
        assert!(response_readiness.response_gate_ready());
        assert_eq!(response_readiness.first_unready_stage(), None);
        assert_eq!(response_readiness.first_blocking_stage(), None);
        assert_eq!(
            response_readiness.response_gate_blocker_component_count,
            response_gate.runtime_response_commit_blocker_component_count()
        );
        assert_eq!(
            response_readiness,
            RuntimeResponseReadinessSummary::new(
                context.response_envelope_summary(&outcome),
                context
                    .response_envelope(&outcome)
                    .request_parity_summary(&outcome, context.request()),
                response_gate
            )
        );
        assert!(response_readiness.can_commit_runtime_response_readiness());
        let manifest_kv_bridge = manifest_request_readiness.manifest_kv_bridge;
        let response_manifest_kv =
            request_context.response_manifest_kv_summary(&planned_outcome, manifest_kv_bridge);
        assert_eq!(
            response_manifest_kv,
            request_context
                .response_envelope(&planned_outcome)
                .request_parity_summary(&planned_outcome, request_context.request())
                .manifest_kv_summary(manifest_kv_bridge)
        );
        assert!(response_manifest_kv.manifest_bridge_ready());
        assert!(response_manifest_kv.response_planned_kv_ready());
        assert!(response_manifest_kv.response_kv_within_manifest_plan());
        assert!(response_manifest_kv.response_manifest_kv_accounting_is_consistent());
        assert!(response_manifest_kv.can_commit_response_manifest_kv());
        let manifest_boundary_kv = request_context
            .manifest_boundary_kv_summary(&planned_outcome, runtime_planning, &manifest)
            .expect("planning digest is attached");
        assert_eq!(
            manifest_boundary_kv,
            RuntimeManifestBoundaryKvSummary::new(manifest_request_readiness, response_manifest_kv)
        );
        assert_eq!(
            RuntimeManifestBoundaryKvSummary::stage_order(),
            [
                RuntimeManifestBoundaryKvStage::RequestManifestPlanning,
                RuntimeManifestBoundaryKvStage::ResponseManifestKv,
            ]
        );
        assert!(manifest_boundary_kv.request_manifest_planning_ready());
        assert!(manifest_boundary_kv.response_manifest_kv_ready());
        assert_eq!(manifest_boundary_kv.first_unready_stage(), None);
        assert_eq!(manifest_boundary_kv.first_blocking_stage(), None);
        assert_eq!(
            manifest_boundary_kv.stage_signal_component_count(
                RuntimeManifestBoundaryKvStage::RequestManifestPlanning
            ),
            manifest_request_readiness.manifest_request_planning_signal_component_count()
        );
        assert_eq!(
            manifest_boundary_kv
                .stage_blocker_component_count(RuntimeManifestBoundaryKvStage::ResponseManifestKv),
            response_manifest_kv.response_manifest_kv_blocker_component_count()
        );
        assert!(manifest_boundary_kv.has_manifest_boundary_kv_signals());
        assert!(!manifest_boundary_kv.has_manifest_boundary_kv_blockers());
        assert_eq!(
            manifest_boundary_kv.manifest_boundary_kv_blocker_component_count(),
            0
        );
        assert!(manifest_boundary_kv.manifest_boundary_kv_accounting_is_consistent());
        assert!(manifest_boundary_kv.manifest_boundary_kv_is_clean());
        assert!(manifest_boundary_kv.can_commit_manifest_boundary_kv());
        let manifest_boundary_commit = request_context
            .manifest_boundary_commit_readiness_summary(
                &planned_outcome,
                runtime_planning,
                &manifest,
            )
            .expect("planning digest is attached");
        assert_eq!(
            manifest_boundary_commit,
            RuntimeManifestBoundaryCommitReadinessSummary::new(
                request_context.boundary_commit_readiness_summary(&planned_outcome),
                manifest_boundary_kv,
            )
        );
        assert_eq!(
            RuntimeManifestBoundaryCommitReadinessSummary::stage_order(),
            [
                RuntimeManifestBoundaryCommitStage::BoundaryCommit,
                RuntimeManifestBoundaryCommitStage::ManifestBoundaryKv,
            ]
        );
        assert_eq!(
            RuntimeManifestBoundaryCommitReadinessSummary::problem_kind_order(),
            [
                RuntimeManifestBoundaryCommitProblemKind::BoundaryCommit,
                RuntimeManifestBoundaryCommitProblemKind::RequestManifestPlanning,
                RuntimeManifestBoundaryCommitProblemKind::ResponseManifestKv,
                RuntimeManifestBoundaryCommitProblemKind::ComponentAccounting,
            ]
        );
        assert!(manifest_boundary_commit.boundary_commit_ready);
        assert!(manifest_boundary_commit.manifest_boundary_kv_ready);
        assert_eq!(
            manifest_boundary_commit.boundary_commit_action,
            RuntimeBoundaryCommitAction::CommitBoundary
        );
        assert!(manifest_boundary_commit.boundary_commit_action_matches_readiness());
        assert!(manifest_boundary_commit.all_boundary_commit_stages_ready());
        assert_eq!(manifest_boundary_commit.first_unready_stage(), None);
        assert_eq!(manifest_boundary_commit.first_blocking_stage(), None);
        assert_eq!(manifest_boundary_commit.first_problem_kind(), None);
        assert_eq!(
            manifest_boundary_commit.stage_signal_component_count(
                RuntimeManifestBoundaryCommitStage::ManifestBoundaryKv
            ),
            manifest_boundary_kv.manifest_boundary_kv_signal_component_count()
        );
        assert_eq!(
            manifest_boundary_commit
                .stage_blocker_component_count(RuntimeManifestBoundaryCommitStage::BoundaryCommit),
            0
        );
        assert!(manifest_boundary_commit.has_commit_signals());
        assert!(!manifest_boundary_commit.has_commit_blockers());
        assert_eq!(manifest_boundary_commit.total_blocker_component_count, 0);
        assert_eq!(
            manifest_boundary_commit.problem_kind_component_count(
                RuntimeManifestBoundaryCommitProblemKind::ResponseManifestKv
            ),
            0
        );
        assert_eq!(
            manifest_boundary_commit.manifest_boundary_commit_problem_component_count(),
            0
        );
        assert!(!manifest_boundary_commit.has_manifest_boundary_commit_problem_components());
        assert!(manifest_boundary_commit.readiness_accounting_is_consistent());
        assert!(manifest_boundary_commit.readiness_commit_is_clean());
        assert!(manifest_boundary_commit.can_commit_runtime_manifest_boundary());
        assert_eq!(
            manifest_boundary_commit.runtime_manifest_boundary_commit_action(),
            RuntimeManifestBoundaryCommitAction::CommitManifestBoundary
        );
        assert_eq!(manifest_boundary_commit.failure_reports(), Vec::new());
        assert_eq!(manifest_boundary_commit.failure_report_count(), 0);
        assert!(!manifest_boundary_commit.has_failure_reports());
        assert_eq!(
            manifest_boundary_commit.failure_batch_summary().total_count,
            0
        );
        assert!(!manifest_boundary_commit.can_format_runtime_failures());
        assert_eq!(manifest_boundary_commit.primary_failure_report(), None);
        assert_eq!(manifest_boundary_commit.primary_failure_summary(), None);
        let manifest_boundary_commit_summary = manifest_boundary_commit.commit_summary();
        assert_eq!(
            manifest_boundary_commit_summary.action,
            RuntimeManifestBoundaryCommitAction::CommitManifestBoundary
        );
        assert_eq!(
            manifest_boundary_commit_summary.action,
            manifest_boundary_commit.runtime_manifest_boundary_commit_action()
        );
        assert_eq!(
            manifest_boundary_commit_summary.boundary_commit_action,
            RuntimeBoundaryCommitAction::CommitBoundary
        );
        assert!(manifest_boundary_commit_summary.action_can_commit());
        assert!(!manifest_boundary_commit_summary.action_should_return_failure());
        assert!(manifest_boundary_commit_summary.can_commit_runtime_manifest_boundary());
        assert!(!manifest_boundary_commit_summary.should_return_runtime_failure());
        assert_eq!(manifest_boundary_commit_summary.first_unready_stage, None);
        assert_eq!(manifest_boundary_commit_summary.first_blocking_stage, None);
        assert_eq!(manifest_boundary_commit_summary.first_problem_kind, None);
        assert!(!manifest_boundary_commit_summary.has_primary_failure_summary());
        assert_eq!(
            manifest_boundary_commit_summary.primary_failure_report(),
            None
        );
        assert_eq!(
            manifest_boundary_commit_summary.failure_reports(),
            Vec::new()
        );
        assert_eq!(manifest_boundary_commit_summary.failure_report_count, 0);
        assert!(!manifest_boundary_commit_summary.can_format_runtime_failures);
        assert_eq!(
            manifest_boundary_commit_summary.total_signal_component_count,
            manifest_boundary_commit.total_signal_component_count
        );
        assert_eq!(
            manifest_boundary_commit_summary.total_blocker_component_count,
            0
        );
        assert!(manifest_boundary_commit_summary.component_accounting_consistent);
        assert!(manifest_boundary_commit_summary.failure_batch_shape_is_clean());
        assert!(manifest_boundary_commit_summary.commit_decision_accounting_is_consistent());
        let manifest_failure_return = manifest_boundary_commit_summary.failure_return_summary();
        assert_eq!(
            manifest_failure_return.source,
            RuntimeFailureReturnSource::ManifestBoundaryCommit
        );
        assert!(!manifest_failure_return.can_return_runtime_failure());
        assert!(manifest_failure_return.failure_return_accounting_is_consistent());
        assert_eq!(
            manifest_boundary_commit_summary.runtime_failure_return_report(),
            None
        );
        let import_plan = RuntimeKvImportPlan::new(
            &request_context.request().runtime,
            architecture,
            planned_kv.import_blocks,
        );
        let import_candidates = vec![RuntimeKvCandidate::new(1, vec![0.1; 16], 1.0)];
        let import_blocks = import_plan.build_blocks(&import_candidates);
        let import_summary = import_plan.import_summary(&import_candidates);
        let import_readiness = RuntimeKvImportReadinessSummary::new(
            import_summary,
            RuntimeKvImportBlockSummary::from_blocks(import_summary.planned_blocks, &import_blocks),
        );
        assert!(import_readiness.can_commit_runtime_kv_import_readiness());
        let export_plan = RuntimeKvExportPlan::new(
            &request_context.request().runtime,
            architecture,
            planned_kv.export_blocks,
        );
        let forward_summaries = transformer_plan
            .layers
            .iter()
            .map(|layer| TransformerForwardSummary::from_layer_budget(layer, 0.5))
            .collect::<Vec<_>>();
        let export_readiness =
            export_plan.readiness_summary(planning, &[0.1; 16], &forward_summaries);
        assert!(export_readiness.can_commit_runtime_kv_export_readiness());
        let side_effects = RuntimeKvSideEffectReadinessSummary::new(
            import_readiness,
            manifest_boundary_commit,
            export_readiness,
        );
        assert_eq!(
            RuntimeKvSideEffectReadinessSummary::stage_order(),
            [
                RuntimeKvSideEffectStage::RuntimeKvImport,
                RuntimeKvSideEffectStage::ManifestBoundaryCommit,
                RuntimeKvSideEffectStage::RuntimeKvExport,
            ]
        );
        assert_eq!(
            RuntimeKvSideEffectReadinessSummary::problem_kind_order(),
            [
                RuntimeKvSideEffectProblemKind::RuntimeKvImport,
                RuntimeKvSideEffectProblemKind::BoundaryCommit,
                RuntimeKvSideEffectProblemKind::RequestManifestPlanning,
                RuntimeKvSideEffectProblemKind::ResponseManifestKv,
                RuntimeKvSideEffectProblemKind::ManifestBoundaryAccounting,
                RuntimeKvSideEffectProblemKind::RuntimeKvExport,
                RuntimeKvSideEffectProblemKind::ComponentAccounting,
            ]
        );
        assert!(side_effects.import_ready);
        assert!(side_effects.manifest_boundary_commit_ready);
        assert!(side_effects.export_ready);
        assert_eq!(
            side_effects.import_commit_action,
            RuntimeKvImportReadinessCommitAction::CommitRuntimeKvImport
        );
        assert_eq!(
            side_effects.manifest_boundary_commit_action,
            RuntimeManifestBoundaryCommitAction::CommitManifestBoundary
        );
        assert_eq!(
            side_effects.export_commit_action,
            RuntimeKvExportReadinessCommitAction::CommitRuntimeKvExport
        );
        assert!(side_effects.import_commit_action_matches_readiness());
        assert!(side_effects.manifest_boundary_commit_action_matches_readiness());
        assert!(side_effects.export_commit_action_matches_readiness());
        assert!(side_effects.child_commit_actions_match_readiness());
        assert_eq!(side_effects.child_commit_action_drift_component_count(), 0);
        assert_eq!(side_effects.first_unready_stage(), None);
        assert_eq!(side_effects.first_blocking_stage(), None);
        assert_eq!(side_effects.first_problem_kind(), None);
        assert_eq!(
            side_effects.stage_signal_component_count(RuntimeKvSideEffectStage::RuntimeKvImport),
            import_readiness.runtime_kv_import_readiness_signal_component_count()
        );
        assert_eq!(
            side_effects.stage_blocker_component_count(RuntimeKvSideEffectStage::RuntimeKvExport),
            0
        );
        assert!(side_effects.has_runtime_kv_side_effect_signals());
        assert!(!side_effects.has_runtime_kv_side_effect_blockers());
        assert_eq!(
            side_effects.runtime_kv_side_effect_blocker_component_count(),
            0
        );
        assert_eq!(
            side_effects
                .problem_kind_component_count(RuntimeKvSideEffectProblemKind::RuntimeKvExport),
            0
        );
        assert_eq!(
            side_effects.runtime_kv_side_effect_problem_component_count(),
            0
        );
        assert!(!side_effects.has_runtime_kv_side_effect_problem_components());
        assert_eq!(side_effects.failure_reports(), Vec::new());
        assert_eq!(side_effects.failure_report_count(), 0);
        assert!(!side_effects.has_failure_reports());
        assert_eq!(side_effects.failure_batch_summary().total_count, 0);
        assert!(!side_effects.failure_batch_summary().has_failures());
        assert!(!side_effects.can_format_runtime_failures());
        assert_eq!(side_effects.primary_failure_report(), None);
        assert_eq!(side_effects.primary_failure_summary(), None);
        assert!(side_effects.runtime_kv_side_effect_accounting_is_consistent());
        assert!(side_effects.runtime_kv_side_effect_commit_is_clean());
        assert!(side_effects.can_commit_runtime_kv_side_effects());
        assert_eq!(
            side_effects.runtime_kv_side_effect_commit_action(),
            RuntimeKvSideEffectCommitAction::CommitSideEffects
        );
        let side_effect_commit = side_effects.commit_summary();
        assert_eq!(
            side_effect_commit.action,
            RuntimeKvSideEffectCommitAction::CommitSideEffects
        );
        assert_eq!(
            side_effect_commit.action,
            side_effects.runtime_kv_side_effect_commit_action()
        );
        assert_eq!(
            side_effect_commit.import_commit_action,
            RuntimeKvImportReadinessCommitAction::CommitRuntimeKvImport
        );
        assert_eq!(
            side_effect_commit.manifest_boundary_commit_action,
            RuntimeManifestBoundaryCommitAction::CommitManifestBoundary
        );
        assert_eq!(
            side_effect_commit.export_commit_action,
            RuntimeKvExportReadinessCommitAction::CommitRuntimeKvExport
        );
        assert!(side_effect_commit.action_can_commit());
        assert!(!side_effect_commit.action_should_return_failure());
        assert!(side_effect_commit.can_commit_runtime_kv_side_effects());
        assert!(!side_effect_commit.should_return_runtime_failure());
        assert_eq!(side_effect_commit.first_unready_stage, None);
        assert_eq!(side_effect_commit.first_blocking_stage, None);
        assert_eq!(side_effect_commit.first_problem_kind, None);
        assert!(!side_effect_commit.has_primary_failure_summary());
        assert_eq!(side_effect_commit.primary_failure_report(), None);
        assert_eq!(side_effect_commit.failure_reports(), Vec::new());
        assert_eq!(side_effect_commit.failure_report_count, 0);
        assert!(!side_effect_commit.can_format_runtime_failures);
        assert_eq!(
            side_effect_commit.total_signal_component_count,
            side_effects.total_signal_component_count
        );
        assert_eq!(side_effect_commit.total_blocker_component_count, 0);
        assert!(side_effect_commit.component_accounting_consistent);
        assert!(side_effect_commit.commit_decision_accounting_is_consistent());
        let side_effect_failure_return = side_effect_commit.failure_return_summary();
        assert_eq!(
            side_effect_failure_return.source,
            RuntimeFailureReturnSource::KvSideEffectCommit
        );
        assert!(!side_effect_failure_return.can_return_runtime_failure());
        assert!(side_effect_failure_return.failure_return_accounting_is_consistent());
        assert_eq!(side_effect_commit.runtime_failure_return_report(), None);
        let mut blocked_planned_kv = response_manifest_kv.response_planned_kv;
        blocked_planned_kv.exported_kv_blocks = 1;
        blocked_planned_kv.exported_kv_within_planning = Some(false);
        let blocked_response_manifest_kv =
            RuntimeResponseManifestKvSummary::new(manifest_kv_bridge, blocked_planned_kv);
        let blocked_manifest_boundary_kv = RuntimeManifestBoundaryKvSummary::new(
            manifest_request_readiness,
            blocked_response_manifest_kv,
        );
        assert!(blocked_manifest_boundary_kv.request_manifest_planning_ready());
        assert!(!blocked_manifest_boundary_kv.response_manifest_kv_ready());
        assert_eq!(
            blocked_manifest_boundary_kv.first_unready_stage(),
            Some(RuntimeManifestBoundaryKvStage::ResponseManifestKv)
        );
        assert_eq!(
            blocked_manifest_boundary_kv.first_blocking_stage(),
            Some(RuntimeManifestBoundaryKvStage::ResponseManifestKv)
        );
        assert_eq!(
            blocked_manifest_boundary_kv
                .stage_blocker_component_count(RuntimeManifestBoundaryKvStage::ResponseManifestKv),
            2
        );
        assert_eq!(
            blocked_manifest_boundary_kv.manifest_boundary_kv_blocker_component_count(),
            2
        );
        assert!(blocked_manifest_boundary_kv.manifest_boundary_kv_accounting_is_consistent());
        assert!(!blocked_manifest_boundary_kv.manifest_boundary_kv_is_clean());
        assert!(!blocked_manifest_boundary_kv.can_commit_manifest_boundary_kv());
        let blocked_manifest_boundary_commit = RuntimeManifestBoundaryCommitReadinessSummary::new(
            manifest_boundary_commit.boundary_commit,
            blocked_manifest_boundary_kv,
        );
        assert!(blocked_manifest_boundary_commit.boundary_commit_ready);
        assert!(!blocked_manifest_boundary_commit.manifest_boundary_kv_ready);
        assert_eq!(
            blocked_manifest_boundary_commit.boundary_commit_action,
            RuntimeBoundaryCommitAction::CommitBoundary
        );
        assert!(blocked_manifest_boundary_commit.boundary_commit_action_matches_readiness());
        assert_eq!(
            blocked_manifest_boundary_commit.first_unready_stage(),
            Some(RuntimeManifestBoundaryCommitStage::ManifestBoundaryKv)
        );
        assert_eq!(
            blocked_manifest_boundary_commit.first_blocking_stage(),
            Some(RuntimeManifestBoundaryCommitStage::ManifestBoundaryKv)
        );
        assert_eq!(
            blocked_manifest_boundary_commit.first_problem_kind(),
            Some(RuntimeManifestBoundaryCommitProblemKind::ResponseManifestKv)
        );
        assert_eq!(
            blocked_manifest_boundary_commit.stage_blocker_component_count(
                RuntimeManifestBoundaryCommitStage::ManifestBoundaryKv
            ),
            2
        );
        assert_eq!(
            blocked_manifest_boundary_commit.problem_kind_component_count(
                RuntimeManifestBoundaryCommitProblemKind::ResponseManifestKv
            ),
            2
        );
        assert_eq!(
            blocked_manifest_boundary_commit.manifest_boundary_commit_problem_component_count(),
            2
        );
        assert!(blocked_manifest_boundary_commit.has_manifest_boundary_commit_problem_components());
        assert!(blocked_manifest_boundary_commit.has_commit_signals());
        assert!(blocked_manifest_boundary_commit.has_commit_blockers());
        assert!(blocked_manifest_boundary_commit.readiness_accounting_is_consistent());
        assert!(!blocked_manifest_boundary_commit.readiness_commit_is_clean());
        assert!(!blocked_manifest_boundary_commit.can_commit_runtime_manifest_boundary());
        assert_eq!(blocked_manifest_boundary_commit.failure_report_count(), 1);
        assert!(blocked_manifest_boundary_commit.has_failure_reports());
        let boundary_failure = blocked_manifest_boundary_commit
            .primary_failure_report()
            .expect("manifest boundary commit failure is reported");
        let boundary_failure_summary = blocked_manifest_boundary_commit
            .primary_failure_summary()
            .expect("manifest boundary commit failure summary is reported");
        assert_eq!(boundary_failure.kind, RuntimeFailureKind::ContractViolation);
        assert_eq!(
            boundary_failure_summary.kind,
            RuntimeFailureKind::ContractViolation
        );
        assert_eq!(
            blocked_manifest_boundary_commit
                .failure_report_for(RuntimeManifestBoundaryCommitProblemKind::ResponseManifestKv),
            Some(boundary_failure.clone())
        );
        assert_eq!(
            blocked_manifest_boundary_commit.failure_reports(),
            vec![boundary_failure.clone()]
        );
        let boundary_failure_batch = blocked_manifest_boundary_commit.failure_batch_summary();
        assert_eq!(boundary_failure_batch.total_count, 1);
        assert_eq!(boundary_failure_batch.contract_violation_count, 1);
        assert!(boundary_failure_batch.can_format_runtime_failures());
        assert!(blocked_manifest_boundary_commit.can_format_runtime_failures());
        assert_eq!(
            blocked_manifest_boundary_commit.runtime_manifest_boundary_commit_action(),
            RuntimeManifestBoundaryCommitAction::ReturnRuntimeFailure
        );
        let blocked_manifest_boundary_commit_summary =
            blocked_manifest_boundary_commit.commit_summary();
        assert_eq!(
            blocked_manifest_boundary_commit_summary.action,
            RuntimeManifestBoundaryCommitAction::ReturnRuntimeFailure
        );
        assert_eq!(
            blocked_manifest_boundary_commit_summary.action,
            blocked_manifest_boundary_commit.runtime_manifest_boundary_commit_action()
        );
        assert_eq!(
            blocked_manifest_boundary_commit_summary.boundary_commit_action,
            RuntimeBoundaryCommitAction::CommitBoundary
        );
        assert!(!blocked_manifest_boundary_commit_summary.action_can_commit());
        assert!(blocked_manifest_boundary_commit_summary.action_should_return_failure());
        assert!(!blocked_manifest_boundary_commit_summary.can_commit_runtime_manifest_boundary());
        assert!(blocked_manifest_boundary_commit_summary.should_return_runtime_failure());
        assert_eq!(
            blocked_manifest_boundary_commit_summary.first_unready_stage,
            Some(RuntimeManifestBoundaryCommitStage::ManifestBoundaryKv)
        );
        assert_eq!(
            blocked_manifest_boundary_commit_summary.first_problem_kind,
            Some(RuntimeManifestBoundaryCommitProblemKind::ResponseManifestKv)
        );
        assert_eq!(
            blocked_manifest_boundary_commit_summary.primary_failure_summary,
            Some(boundary_failure_summary)
        );
        assert_eq!(
            blocked_manifest_boundary_commit_summary.primary_failure_report(),
            Some(boundary_failure.clone())
        );
        assert_eq!(
            blocked_manifest_boundary_commit_summary.failure_reports(),
            vec![boundary_failure.clone()]
        );
        assert_eq!(
            blocked_manifest_boundary_commit_summary
                .failure_batch
                .contract_violation_count,
            1
        );
        assert_eq!(
            blocked_manifest_boundary_commit_summary.failure_report_count,
            1
        );
        assert!(blocked_manifest_boundary_commit_summary.can_format_runtime_failures);
        assert!(blocked_manifest_boundary_commit_summary.failure_batch_shape_is_clean());
        assert!(
            blocked_manifest_boundary_commit_summary.commit_decision_accounting_is_consistent()
        );
        let blocked_manifest_failure_return =
            blocked_manifest_boundary_commit_summary.failure_return_summary();
        assert_eq!(
            blocked_manifest_failure_return.source,
            RuntimeFailureReturnSource::ManifestBoundaryCommit
        );
        assert!(blocked_manifest_failure_return.should_return_failure);
        assert!(blocked_manifest_failure_return.has_primary_failure_summary);
        assert_eq!(
            blocked_manifest_failure_return.primary_failure_summary,
            Some(boundary_failure_summary)
        );
        assert!(blocked_manifest_failure_return.can_return_runtime_failure());
        assert!(blocked_manifest_failure_return.failure_return_accounting_is_consistent());
        let blocked_manifest_return_report = blocked_manifest_boundary_commit_summary
            .runtime_failure_return_report()
            .expect("manifest boundary failure return report is materialized");
        assert_eq!(
            blocked_manifest_return_report.source,
            RuntimeFailureReturnSource::ManifestBoundaryCommit
        );
        assert_eq!(
            blocked_manifest_return_report.primary_failure,
            boundary_failure.clone()
        );
        assert_eq!(
            blocked_manifest_return_report.primary_failure_summary,
            boundary_failure_summary
        );
        assert_eq!(
            blocked_manifest_return_report.backend_message(),
            boundary_failure.backend_message()
        );
        assert_eq!(
            blocked_manifest_return_report.inference_error().message,
            boundary_failure.backend_message()
        );
        assert!(blocked_manifest_return_report.can_use_runtime_failure_return_report());
        let manifest_blocked_side_effects = RuntimeKvSideEffectReadinessSummary::new(
            import_readiness,
            blocked_manifest_boundary_commit,
            export_readiness,
        );
        assert_eq!(
            manifest_blocked_side_effects.import_commit_action,
            RuntimeKvImportReadinessCommitAction::CommitRuntimeKvImport
        );
        assert_eq!(
            manifest_blocked_side_effects.manifest_boundary_commit_action,
            RuntimeManifestBoundaryCommitAction::ReturnRuntimeFailure
        );
        assert_eq!(
            manifest_blocked_side_effects.export_commit_action,
            RuntimeKvExportReadinessCommitAction::CommitRuntimeKvExport
        );
        assert!(manifest_blocked_side_effects.import_commit_action_matches_readiness());
        assert!(manifest_blocked_side_effects.manifest_boundary_commit_action_matches_readiness());
        assert!(manifest_blocked_side_effects.export_commit_action_matches_readiness());
        assert!(manifest_blocked_side_effects.child_commit_actions_match_readiness());
        assert_eq!(
            manifest_blocked_side_effects.child_commit_action_drift_component_count(),
            0
        );
        assert_eq!(
            manifest_blocked_side_effects.first_problem_kind(),
            Some(RuntimeKvSideEffectProblemKind::ResponseManifestKv)
        );
        let manifest_failure = manifest_blocked_side_effects
            .primary_failure_report()
            .expect("manifest boundary KV drift is reported");
        let manifest_primary_summary = manifest_blocked_side_effects
            .primary_failure_summary()
            .expect("manifest boundary KV drift summary is reported");
        assert_eq!(manifest_failure.kind, RuntimeFailureKind::ContractViolation);
        assert_eq!(
            manifest_primary_summary.kind,
            RuntimeFailureKind::ContractViolation
        );
        assert_eq!(
            manifest_failure.failure_summary().trace_label,
            RuntimeFailureKind::ContractViolation.trace_label()
        );
        let manifest_failure_batch = manifest_blocked_side_effects.failure_batch_summary();
        assert_eq!(manifest_blocked_side_effects.failure_reports().len(), 1);
        assert_eq!(manifest_blocked_side_effects.failure_report_count(), 1);
        assert!(manifest_blocked_side_effects.has_failure_reports());
        assert_eq!(manifest_failure_batch.total_count, 1);
        assert_eq!(manifest_failure_batch.contract_violation_count, 1);
        assert_eq!(manifest_failure_batch.kv_export_count, 0);
        assert!(manifest_failure_batch.failure_batch_shape_is_clean());
        assert!(manifest_blocked_side_effects.can_format_runtime_failures());
        let manifest_commit = manifest_blocked_side_effects.commit_summary();
        assert_eq!(
            manifest_commit.action,
            RuntimeKvSideEffectCommitAction::ReturnRuntimeFailure
        );
        assert_eq!(
            manifest_commit.import_commit_action,
            RuntimeKvImportReadinessCommitAction::CommitRuntimeKvImport
        );
        assert_eq!(
            manifest_commit.manifest_boundary_commit_action,
            RuntimeManifestBoundaryCommitAction::ReturnRuntimeFailure
        );
        assert_eq!(
            manifest_commit.export_commit_action,
            RuntimeKvExportReadinessCommitAction::CommitRuntimeKvExport
        );
        assert!(!manifest_commit.action_can_commit());
        assert!(manifest_commit.action_should_return_failure());
        assert!(!manifest_commit.can_commit_runtime_kv_side_effects());
        assert!(manifest_commit.should_return_runtime_failure());
        assert_eq!(
            manifest_commit.first_unready_stage,
            Some(RuntimeKvSideEffectStage::ManifestBoundaryCommit)
        );
        assert_eq!(
            manifest_commit.first_problem_kind,
            Some(RuntimeKvSideEffectProblemKind::ResponseManifestKv)
        );
        assert_eq!(
            manifest_commit.primary_failure_summary,
            Some(manifest_primary_summary)
        );
        assert_eq!(
            manifest_commit.primary_failure_report(),
            Some(manifest_failure.clone())
        );
        assert_eq!(
            manifest_commit.failure_report_for(RuntimeKvSideEffectProblemKind::ResponseManifestKv),
            Some(manifest_failure.clone())
        );
        assert_eq!(
            manifest_commit.failure_reports(),
            vec![manifest_failure.clone()]
        );
        assert!(manifest_commit.has_primary_failure_summary());
        assert_eq!(manifest_commit.failure_batch.contract_violation_count, 1);
        assert_eq!(manifest_commit.failure_report_count, 1);
        assert!(manifest_commit.can_format_runtime_failures);
        assert!(manifest_commit.failure_batch_shape_is_clean());
        assert!(manifest_commit.commit_decision_accounting_is_consistent());
        let manifest_side_effect_failure_return = manifest_commit.failure_return_summary();
        assert_eq!(
            manifest_side_effect_failure_return.source,
            RuntimeFailureReturnSource::KvSideEffectCommit
        );
        assert!(manifest_side_effect_failure_return.should_return_failure);
        assert!(manifest_side_effect_failure_return.has_primary_failure_summary);
        assert_eq!(
            manifest_side_effect_failure_return.primary_failure_summary,
            Some(manifest_primary_summary)
        );
        assert!(manifest_side_effect_failure_return.can_return_runtime_failure());
        assert!(manifest_side_effect_failure_return.failure_return_accounting_is_consistent());
        let manifest_side_effect_return_report = manifest_commit
            .runtime_failure_return_report()
            .expect("manifest side-effect failure return report is materialized");
        assert_eq!(
            manifest_side_effect_return_report.source,
            RuntimeFailureReturnSource::KvSideEffectCommit
        );
        assert_eq!(
            manifest_side_effect_return_report.primary_failure,
            manifest_failure.clone()
        );
        assert_eq!(
            manifest_side_effect_return_report.diagnostics_note(),
            manifest_failure.diagnostics_note()
        );
        assert!(manifest_side_effect_return_report.can_use_runtime_failure_return_report());
        let blocked_export_summary = RuntimeKvExportSummary {
            enabled: true,
            max_blocks: 1,
            planned_blocks: 1,
            forward_value_len: 16,
            forward_summary_count: forward_summaries.len(),
            forward_batch: TransformerForwardBatchSummary::from_summaries(&forward_summaries),
            hit_export_limit: true,
        };
        let blocked_export_planning = RuntimeKvExportPlanningSummary {
            planning_export_blocks: 1,
            export_plan_max_blocks: 1,
            export_summary: blocked_export_summary,
            export_plan_matches_planning_limit: true,
            planned_export_within_planning: true,
        };
        let blocked_side_effects = RuntimeKvSideEffectReadinessSummary::new(
            import_readiness,
            manifest_boundary_commit,
            RuntimeKvExportReadinessSummary::from_blocks(blocked_export_planning, &[]),
        );
        assert!(blocked_side_effects.import_ready);
        assert!(blocked_side_effects.manifest_boundary_commit_ready);
        assert!(!blocked_side_effects.export_ready);
        assert_eq!(
            blocked_side_effects.import_commit_action,
            RuntimeKvImportReadinessCommitAction::CommitRuntimeKvImport
        );
        assert_eq!(
            blocked_side_effects.manifest_boundary_commit_action,
            RuntimeManifestBoundaryCommitAction::CommitManifestBoundary
        );
        assert_eq!(
            blocked_side_effects.export_commit_action,
            RuntimeKvExportReadinessCommitAction::ReturnRuntimeFailure
        );
        assert!(blocked_side_effects.import_commit_action_matches_readiness());
        assert!(blocked_side_effects.manifest_boundary_commit_action_matches_readiness());
        assert!(blocked_side_effects.export_commit_action_matches_readiness());
        assert!(blocked_side_effects.child_commit_actions_match_readiness());
        assert_eq!(
            blocked_side_effects.child_commit_action_drift_component_count(),
            0
        );
        assert_eq!(
            blocked_side_effects.first_unready_stage(),
            Some(RuntimeKvSideEffectStage::RuntimeKvExport)
        );
        assert_eq!(
            blocked_side_effects.first_blocking_stage(),
            Some(RuntimeKvSideEffectStage::RuntimeKvExport)
        );
        assert_eq!(
            blocked_side_effects.first_problem_kind(),
            Some(RuntimeKvSideEffectProblemKind::RuntimeKvExport)
        );
        assert_eq!(
            blocked_side_effects
                .stage_blocker_component_count(RuntimeKvSideEffectStage::RuntimeKvExport),
            blocked_side_effects.export_blocker_component_count
        );
        assert_eq!(
            blocked_side_effects
                .problem_kind_component_count(RuntimeKvSideEffectProblemKind::RuntimeKvExport),
            blocked_side_effects.export_blocker_component_count
        );
        assert_eq!(
            blocked_side_effects.runtime_kv_side_effect_problem_component_count(),
            blocked_side_effects.runtime_kv_side_effect_blocker_component_count()
        );
        let export_failure = blocked_side_effects
            .primary_failure_report()
            .expect("export block drift is reported");
        let export_primary_summary = blocked_side_effects
            .primary_failure_summary()
            .expect("export block drift summary is reported");
        assert_eq!(export_failure.kind, RuntimeFailureKind::KvExport);
        assert_eq!(export_primary_summary.kind, RuntimeFailureKind::KvExport);
        assert_eq!(
            export_failure.failure_summary().trace_label,
            RuntimeFailureKind::KvExport.trace_label()
        );
        let export_failure_batch = blocked_side_effects.failure_batch_summary();
        assert_eq!(blocked_side_effects.failure_reports().len(), 1);
        assert_eq!(blocked_side_effects.failure_report_count(), 1);
        assert!(blocked_side_effects.has_failure_reports());
        assert_eq!(export_failure_batch.total_count, 1);
        assert_eq!(export_failure_batch.kv_export_count, 1);
        assert_eq!(export_failure_batch.contract_violation_count, 0);
        assert!(export_failure_batch.failure_batch_shape_is_clean());
        assert!(blocked_side_effects.can_format_runtime_failures());
        assert!(blocked_side_effects.has_runtime_kv_side_effect_blockers());
        assert!(blocked_side_effects.has_runtime_kv_side_effect_problem_components());
        assert!(blocked_side_effects.runtime_kv_side_effect_accounting_is_consistent());
        assert!(!blocked_side_effects.runtime_kv_side_effect_commit_is_clean());
        assert!(!blocked_side_effects.can_commit_runtime_kv_side_effects());
        assert_eq!(
            blocked_side_effects.runtime_kv_side_effect_commit_action(),
            RuntimeKvSideEffectCommitAction::ReturnRuntimeFailure
        );
        let export_commit = blocked_side_effects.commit_summary();
        assert_eq!(
            export_commit.action,
            RuntimeKvSideEffectCommitAction::ReturnRuntimeFailure
        );
        assert_eq!(
            export_commit.action,
            blocked_side_effects.runtime_kv_side_effect_commit_action()
        );
        assert_eq!(
            export_commit.import_commit_action,
            RuntimeKvImportReadinessCommitAction::CommitRuntimeKvImport
        );
        assert_eq!(
            export_commit.manifest_boundary_commit_action,
            RuntimeManifestBoundaryCommitAction::CommitManifestBoundary
        );
        assert_eq!(
            export_commit.export_commit_action,
            RuntimeKvExportReadinessCommitAction::ReturnRuntimeFailure
        );
        assert!(!export_commit.action_can_commit());
        assert!(export_commit.action_should_return_failure());
        assert!(!export_commit.can_commit_runtime_kv_side_effects());
        assert!(export_commit.should_return_runtime_failure());
        assert_eq!(
            export_commit.first_unready_stage,
            Some(RuntimeKvSideEffectStage::RuntimeKvExport)
        );
        assert_eq!(
            export_commit.first_problem_kind,
            Some(RuntimeKvSideEffectProblemKind::RuntimeKvExport)
        );
        assert_eq!(
            export_commit.primary_failure_summary,
            Some(export_primary_summary)
        );
        assert_eq!(
            export_commit.primary_failure_report(),
            Some(export_failure.clone())
        );
        assert_eq!(
            export_commit.failure_report_for(RuntimeKvSideEffectProblemKind::RuntimeKvExport),
            Some(export_failure.clone())
        );
        assert_eq!(
            export_commit.failure_reports(),
            vec![export_failure.clone()]
        );
        assert_eq!(export_commit.failure_batch.kv_export_count, 1);
        assert_eq!(export_commit.failure_report_count, 1);
        assert!(export_commit.can_format_runtime_failures);
        assert!(export_commit.failure_batch_shape_is_clean());
        assert!(export_commit.commit_decision_accounting_is_consistent());
        let export_side_effect_failure_return = export_commit.failure_return_summary();
        assert_eq!(
            export_side_effect_failure_return.source,
            RuntimeFailureReturnSource::KvSideEffectCommit
        );
        assert!(export_side_effect_failure_return.should_return_failure);
        assert!(export_side_effect_failure_return.has_primary_failure_summary);
        assert_eq!(
            export_side_effect_failure_return.primary_failure_summary,
            Some(export_primary_summary)
        );
        assert!(export_side_effect_failure_return.can_return_runtime_failure());
        assert!(export_side_effect_failure_return.failure_return_accounting_is_consistent());
        let export_side_effect_return_report = export_commit
            .runtime_failure_return_report()
            .expect("export side-effect failure return report is materialized");
        assert_eq!(
            export_side_effect_return_report.source,
            RuntimeFailureReturnSource::KvSideEffectCommit
        );
        assert_eq!(
            export_side_effect_return_report.primary_failure,
            export_failure.clone()
        );
        assert_eq!(
            export_side_effect_return_report.inference_error().message,
            export_failure.backend_message()
        );
        assert!(export_side_effect_return_report.can_use_runtime_failure_return_report());
        let boundary = context.boundary_acceptance_summary(&outcome);
        assert!(boundary.accepted);
        assert!(!boundary.has_request_failures());
        assert!(!boundary.has_response_failures());
        assert!(!boundary.has_kv_failures());
        assert!(!boundary.has_request_parity_failures());
        assert_eq!(boundary.total_violation_count, 0);
        assert_eq!(boundary.total_failure_report_count, 0);
        assert_eq!(boundary.request_acceptance_failure_component_count(), 0);
        assert_eq!(boundary.response_acceptance_failure_component_count(), 0);
        assert_eq!(boundary.kv_failure_component_count(), 0);
        assert_eq!(boundary.request_parity_failure_component_count(), 0);
        assert_eq!(boundary.boundary_failure_component_count(), 0);
        assert!(!boundary.has_failure_reports());
        assert_eq!(boundary.boundary_acceptance_problem_component_count(), 0);
        assert!(boundary.total_violation_matches_parts());
        assert!(boundary.failure_report_matches_parts());
        assert!(boundary.is_clean_acceptance());
        assert_eq!(
            boundary.runtime_boundary_acceptance_commit_signal_component_count(),
            boundary
                .request
                .runtime_request_acceptance_commit_signal_component_count()
                .saturating_add(
                    boundary
                        .response
                        .runtime_response_acceptance_commit_signal_component_count(),
                )
                .saturating_add(1)
        );
        assert!(boundary.has_runtime_boundary_acceptance_commit_signals());
        assert_eq!(
            boundary.runtime_boundary_acceptance_commit_blocker_component_count(),
            0
        );
        assert!(!boundary.has_runtime_boundary_acceptance_commit_blockers());
        assert!(boundary.runtime_boundary_acceptance_commit_accounting_is_consistent());
        assert!(boundary.runtime_boundary_acceptance_commit_is_clean());
        assert!(boundary.can_commit_runtime_boundary_acceptance());
        assert_eq!(boundary.request, context.request_acceptance_summary());
        assert_eq!(
            boundary.response,
            context.response_acceptance_summary(&outcome)
        );
        assert_eq!(
            context.response_envelope(&outcome).summary(),
            "schema=rust-norion-runtime-response-v1 answer_chars=2 tokens=2 imported_kv=1 exported_kv=1 weak_runtime_kv_imports_skipped=0 runtime_signal=true"
        );
        let envelope_summary = context.boundary_envelope_summary(&outcome);
        assert!(envelope_summary.boundary_shape_is_consistent());
        assert!(envelope_summary.has_request_adapter_candidate());
        assert!(envelope_summary.has_runtime_execution_signal());
        assert!(!envelope_summary.request_was_context_limited());
        assert_eq!(envelope_summary.request.imported_kv_blocks, 1);
        assert_eq!(envelope_summary.response.imported_kv_blocks, 1);
        assert_eq!(envelope_summary.response_token_drift_component_count(), 0);
        assert_eq!(envelope_summary.imported_kv_drift_component_count(), 0);
        assert_eq!(envelope_summary.diagnostics_kv_drift_component_count(), 0);
        assert_eq!(
            envelope_summary.runtime_execution_signal_missing_component_count(),
            0
        );
        assert_eq!(
            envelope_summary.request_adapter_signal_missing_component_count(),
            0
        );
        assert_eq!(
            envelope_summary.context_pressure_signal_component_count(),
            0
        );
        assert!(!envelope_summary.response_has_token_uncertainty());
        assert_eq!(
            envelope_summary.response_uncertainty_coverage_signal_component_count(),
            2
        );
        assert!(envelope_summary.response_has_uncertainty_coverage_signals());
        assert_eq!(
            envelope_summary.response_uncertainty_metric_problem_component_count(),
            0
        );
        assert!(!envelope_summary.response_has_uncertainty_metric_problem_components());
        assert!(envelope_summary.response_uncertainty_accounting_is_consistent());
        assert_eq!(envelope_summary.boundary_shape_drift_component_count(), 0);
        assert_eq!(
            envelope_summary.boundary_envelope_signal_component_count(),
            2
        );
        let expected_envelope_commit_signals = envelope_summary
            .request
            .request_envelope_commit_signal_component_count()
            .saturating_add(
                envelope_summary
                    .response
                    .runtime_response_envelope_commit_signal_component_count(),
            )
            .saturating_add(envelope_summary.boundary_envelope_signal_component_count());
        assert_eq!(
            envelope_summary.runtime_boundary_envelope_commit_signal_component_count(),
            expected_envelope_commit_signals
        );
        assert!(envelope_summary.has_runtime_boundary_envelope_commit_signals());
        assert_eq!(
            envelope_summary.runtime_boundary_envelope_commit_blocker_component_count(),
            0
        );
        assert!(!envelope_summary.has_runtime_boundary_envelope_commit_blockers());
        assert!(envelope_summary.runtime_boundary_envelope_commit_accounting_is_consistent());
        assert!(envelope_summary.boundary_envelope_commit_is_clean());
        assert!(envelope_summary.can_commit_runtime_boundary_envelope());
        assert!(envelope_summary.boundary_envelope_is_consistent());
        assert!(envelope_summary.boundary_envelope_shape_is_clean());
        assert!(envelope_summary.can_use_runtime_boundary_envelope());
        let kv_summary = context.boundary_kv_summary(&outcome);

        assert_eq!(kv_summary.request_imported_kv_blocks, 1);
        assert_eq!(kv_summary.concrete_imported_kv_blocks, 1);
        assert_eq!(kv_summary.accepted_imported_kv_blocks, 1);
        assert_eq!(kv_summary.response_imported_kv_blocks, 1);
        assert_eq!(kv_summary.response_exported_kv_blocks, 1);
        assert_eq!(kv_summary.accepted_exported_kv_blocks, 1);
        assert_eq!(kv_summary.imported_namespace_counts.runtime, 1);
        assert_eq!(kv_summary.exported_namespace_counts.runtime, 1);
        assert!(kv_summary.runtime_import_enabled);
        assert!(kv_summary.runtime_export_enabled);
        assert!(kv_summary.concrete_imports_match_request());
        assert!(kv_summary.response_imports_match_request());
        assert!(kv_summary.diagnostics_match_response());
        assert!(kv_summary.imports_within_planning());
        assert!(kv_summary.exports_within_runtime());
        assert!(kv_summary.exports_within_planning());
        assert!(kv_summary.namespaces_are_runtime_exchange());
        assert!(!kv_summary.has_kv_violations());
        assert_eq!(kv_summary.imported_kv_activity_signal_component_count(), 4);
        assert_eq!(kv_summary.exported_kv_activity_signal_component_count(), 2);
        assert_eq!(
            kv_summary.diagnostics_kv_activity_signal_component_count(),
            2
        );
        assert_eq!(kv_summary.runtime_kv_capability_signal_component_count(), 4);
        assert_eq!(kv_summary.planning_kv_boundary_signal_component_count(), 0);
        assert_eq!(kv_summary.namespace_kv_activity_signal_component_count(), 2);
        assert_eq!(kv_summary.kv_boundary_signal_component_count(), 14);
        assert!(kv_summary.has_kv_boundary_signals());
        assert_eq!(kv_summary.runtime_exchange_count_drift_component_count(), 0);
        assert_eq!(kv_summary.planning_bound_drift_component_count(), 0);
        assert_eq!(kv_summary.runtime_bound_drift_component_count(), 0);
        assert_eq!(kv_summary.namespace_drift_component_count(), 0);
        assert_eq!(kv_summary.validation_failure_component_count(), 0);
        assert_eq!(kv_summary.kv_boundary_problem_component_count(), 0);
        assert!(!kv_summary.has_kv_boundary_problem_components());
        assert!(kv_summary.kv_boundary_is_consistent());
        assert!(kv_summary.kv_boundary_shape_is_clean());
        assert!(kv_summary.can_use_runtime_boundary_kv());
        let gate = context.boundary_gate_summary(&outcome);

        assert!(gate.request_accepted);
        assert!(gate.response_accepted);
        assert!(gate.envelope_consistent);
        assert!(gate.adapter_consistent);
        assert!(gate.kv_consistent);
        assert_eq!(gate.request_backend_wire_problem_count, 0);
        assert_eq!(gate.request_planning_pre_request_problem_count, 0);
        assert_eq!(gate.request_planning_pressure_signal_count, 0);
        assert_eq!(gate.request_planning_dense_compute_avoided_tokens, 0);
        assert_eq!(gate.response_wire_problem_count, 0);
        assert_eq!(gate.planning_pre_request_problem_count, 0);
        assert_eq!(gate.planning_pressure_signal_count, 0);
        assert_eq!(gate.response_planning_dense_compute_avoided_tokens, 0);
        assert_eq!(gate.planning_dense_compute_avoided_tokens(), 0);
        assert!(!gate.has_request_planning_dense_compute_savings());
        assert!(!gate.has_response_planning_dense_compute_savings());
        assert!(!gate.has_planning_dense_compute_savings());
        assert_eq!(gate.kv_boundary_signal_count, 14);
        assert_eq!(gate.kv_boundary_signal_component_count(), 14);
        assert!(gate.has_kv_boundary_signals());
        assert_eq!(gate.response_uncertainty_coverage_signal_count, 2);
        assert_eq!(gate.response_uncertainty_metric_problem_count, 0);
        assert!(gate.response_uncertainty_accounting_consistent);
        assert_eq!(gate.total_violation_count, 0);
        assert_eq!(gate.total_failure_report_count, 0);
        assert_eq!(gate.request_backend_wire_problem_component_count(), 0);
        assert_eq!(
            gate.direct_request_backend_wire_problem_component_count(),
            0
        );
        assert_eq!(
            gate.request_planning_pre_request_gate_problem_component_count(),
            0
        );
        assert_eq!(gate.request_planning_pressure_signal_component_count(), 0);
        assert_eq!(gate.total_wire_problem_component_count(), 0);
        assert!(!gate.has_request_backend_wire_problem_components());
        assert!(!gate.has_request_planning_pre_request_gate_problems());
        assert!(!gate.has_request_planning_pressure_signals());
        assert!(!gate.has_wire_problem_components());
        assert!(gate.request_backend_wire_accounting_is_consistent());
        assert_eq!(gate.response_wire_problem_component_count(), 0);
        assert_eq!(gate.direct_response_wire_problem_component_count(), 0);
        assert_eq!(gate.planning_pre_request_gate_problem_component_count(), 0);
        assert_eq!(gate.planning_pressure_signal_component_count(), 0);
        assert_eq!(
            gate.response_uncertainty_coverage_signal_component_count(),
            2
        );
        assert!(gate.has_response_uncertainty_coverage_signals());
        assert_eq!(gate.commit_gate_signal_component_count(), 16);
        assert!(gate.commit_gate_has_signal_components());
        assert_eq!(
            gate.response_uncertainty_metric_problem_component_count(),
            0
        );
        assert!(!gate.has_response_uncertainty_metric_problem_components());
        assert!(gate.response_uncertainty_accounting_is_consistent());
        assert!(!gate.has_response_wire_problem_components());
        assert!(!gate.has_planning_pre_request_gate_problems());
        assert!(!gate.has_planning_pressure_signals());
        assert!(gate.response_wire_accounting_is_consistent());
        assert!(gate.wire_accounting_is_consistent());
        assert!(!gate.request_acceptance_failed());
        assert!(!gate.response_acceptance_failed());
        assert!(!gate.has_acceptance_failures());
        assert!(!gate.envelope_drifted());
        assert!(!gate.adapter_drifted());
        assert!(!gate.kv_drifted());
        assert!(!gate.has_boundary_drift());
        assert!(!gate.has_total_violations());
        assert_eq!(gate.request_acceptance_blocker_component_count(), 0);
        assert_eq!(gate.response_acceptance_blocker_component_count(), 0);
        assert_eq!(gate.acceptance_failure_component_count(), 0);
        assert_eq!(gate.envelope_blocker_component_count(), 0);
        assert_eq!(gate.adapter_blocker_component_count(), 0);
        assert_eq!(gate.kv_blocker_component_count(), 0);
        assert_eq!(gate.boundary_drift_component_count(), 0);
        assert_eq!(gate.mapped_failure_report_component_count(), 0);
        assert_eq!(gate.commit_blocker_component_count(), 0);
        assert!(!gate.commit_gate_has_problem_components());
        assert!(gate.commit_gate_accounting_is_consistent());
        assert!(gate.can_commit_response());
        assert!(gate.is_clean_commit_gate());
        assert!(gate.boundary_gate_shape_is_clean());
        assert_eq!(gate.runtime_boundary_commit_signal_component_count(), 16);
        assert!(gate.has_runtime_boundary_commit_signals());
        assert_eq!(gate.runtime_boundary_commit_blocker_component_count(), 0);
        assert!(!gate.has_runtime_boundary_commit_blockers());
        assert!(gate.runtime_boundary_commit_accounting_is_consistent());
        assert!(gate.runtime_boundary_commit_is_clean());
        assert!(gate.can_commit_runtime_boundary());
        assert!(gate.can_commit_runtime_response());
        let readiness = context.boundary_commit_readiness_summary(&outcome);

        assert!(readiness.request_acceptance_ready);
        assert!(readiness.response_acceptance_ready);
        assert!(readiness.boundary_acceptance_ready);
        assert!(readiness.boundary_envelope_ready);
        assert!(readiness.boundary_adapter_ready);
        assert!(readiness.boundary_kv_ready);
        assert!(readiness.boundary_gate_ready);
        assert!(readiness.runtime_response_ready);
        assert!(readiness.all_acceptance_ready());
        assert!(readiness.all_boundary_summaries_ready());
        assert_eq!(
            readiness.total_signal_component_count,
            readiness
                .acceptance_signal_component_count
                .saturating_add(readiness.envelope_signal_component_count)
                .saturating_add(readiness.adapter_signal_component_count)
                .saturating_add(readiness.kv_signal_component_count)
                .saturating_add(readiness.gate_signal_component_count)
                .saturating_add(readiness.runtime_response_signal_component_count)
        );
        assert_eq!(
            readiness.stage_signal_component_count(RuntimeBoundaryCommitStage::RequestAcceptance),
            readiness.request_acceptance_signal_component_count
        );
        assert_eq!(
            readiness.stage_blocker_component_count(RuntimeBoundaryCommitStage::BoundaryGate),
            0
        );
        assert_eq!(readiness.first_unready_stage(), None);
        assert_eq!(readiness.first_blocking_stage(), None);
        assert!(readiness.has_commit_signals());
        assert_eq!(readiness.total_blocker_component_count, 0);
        assert!(!readiness.has_commit_blockers());
        assert!(readiness.readiness_accounting_is_consistent());
        assert!(readiness.readiness_commit_is_clean());
        assert!(readiness.can_commit_runtime_boundary());
        assert_eq!(
            readiness.runtime_boundary_commit_action(),
            RuntimeBoundaryCommitAction::CommitBoundary
        );
        assert_eq!(readiness.failure_reports(), Vec::new());
        assert_eq!(readiness.failure_report_count(), 0);
        assert!(!readiness.has_failure_reports());
        assert_eq!(readiness.failure_batch_summary().total_count, 0);
        assert!(!readiness.can_format_runtime_failures());
        assert_eq!(readiness.primary_failure_report(), None);
        assert_eq!(readiness.primary_failure_summary(), None);
        let boundary_commit = readiness.commit_summary();
        assert_eq!(
            boundary_commit.action,
            RuntimeBoundaryCommitAction::CommitBoundary
        );
        assert_eq!(
            boundary_commit.action,
            readiness.runtime_boundary_commit_action()
        );
        assert!(boundary_commit.action_can_commit());
        assert!(!boundary_commit.action_should_return_failure());
        assert!(boundary_commit.can_commit_runtime_boundary());
        assert!(!boundary_commit.should_return_runtime_failure());
        assert_eq!(boundary_commit.first_unready_stage, None);
        assert_eq!(boundary_commit.first_blocking_stage, None);
        assert!(!boundary_commit.has_primary_failure_summary());
        assert_eq!(boundary_commit.primary_failure_report(), None);
        assert_eq!(boundary_commit.failure_reports(), Vec::new());
        assert_eq!(boundary_commit.failure_report_count, 0);
        assert!(!boundary_commit.can_format_runtime_failures);
        assert_eq!(
            boundary_commit.total_signal_component_count,
            readiness.total_signal_component_count
        );
        assert_eq!(boundary_commit.total_blocker_component_count, 0);
        assert!(boundary_commit.component_accounting_consistent);
        assert!(boundary_commit.failure_batch_shape_is_clean());
        assert!(boundary_commit.commit_decision_accounting_is_consistent());
        let failure_return = boundary_commit.failure_return_summary();
        assert_eq!(
            failure_return.source,
            RuntimeFailureReturnSource::BoundaryCommit
        );
        assert_eq!(failure_return.source.label(), "boundary_commit");
        assert!(!failure_return.should_return_failure);
        assert!(!failure_return.can_return_runtime_failure());
        assert!(!failure_return.has_failure_reports());
        assert!(!failure_return.has_primary_failure_summary);
        assert!(failure_return.failure_return_accounting_is_consistent());
        assert_eq!(boundary_commit.runtime_failure_return_report(), None);
    }

    #[test]
    fn acceptance_context_request_planning_commit_requires_committed_runtime_parts() {
        let metadata = RuntimeMetadata::new("model", "tok", 128, 16)
            .with_kv_exchange(true, true)
            .with_kv_limits(1, 1);
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(16)
            .with_max_tokens(2)
            .with_runtime(metadata);
        let architecture = TransformerRuntimeArchitecture::new(1, 16, 2, 1, 64);
        let transformer_plan = TransformerPlanDigest::new(
            Some("acceptance-context-request-planning-commit"),
            vec![TransformerLayerBudget::new(
                0,
                TransformerAttentionKind::Global,
                0.5,
                64,
            )],
        );
        let hardware = HardwareAllocator::new().plan(
            HardwareLoadSnapshot::new(DeviceClass::DiscreteGpu, 0.1, 0.1, 0.1, 0.1),
            TaskProfile::Coding,
            512,
            HierarchyWeights::for_profile(TaskProfile::Coding),
        );
        let execution = hardware
            .adapter_execution_context()
            .clamp_for_runtime(&request.runtime);
        let planning = RuntimePlanningDigest::from_request(
            &request,
            RouteBudget::default(),
            &execution,
            &[],
            &DeterministicFhtDkeBudgeter::default(),
        );
        let context = RuntimeAcceptanceContext::from_request_parts(
            &request,
            architecture,
            RouteBudget::default(),
            HierarchyWeights::for_profile(TaskProfile::Coding),
            &transformer_plan,
            hardware,
            vec![runtime_block(1)],
        )
        .with_planning_digest(planning);
        let clean_runtime_planning = clean_runtime_planning_readiness();

        assert!(context.can_commit_request_planning_with_committed_parts(clean_runtime_planning));

        let route_budget = clean_route_budget();
        let stale_fht_dke_budget = FhtDkeBudgetSummary {
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
            attention_threshold: 0.42,
            route_pressure: 0.25,
        };
        let stale_runtime_planning = RuntimePlanningReadinessSummary::new(
            clean_fht_dke_planning_readiness(route_budget, stale_fht_dke_budget),
            clean_runtime_planning_summary(stale_fht_dke_budget),
        );
        let stale_request_readiness =
            context.request_planning_readiness_summary(stale_runtime_planning);

        assert!(!stale_request_readiness.runtime_planning_committed_parts_ready());
        assert!(stale_request_readiness.request_planning_ready());
        assert!(stale_request_readiness.request_gate_ready());
        assert!(!stale_request_readiness.can_commit_runtime_request_planning());
        assert!(!context.can_commit_request_planning_with_committed_parts(stale_runtime_planning));
    }

    #[test]
    fn acceptance_boundary_gate_surfaces_planning_dense_compute_savings() {
        let metadata = RuntimeMetadata::new("model", "tok", 128, 16)
            .with_kv_exchange(true, true)
            .with_kv_limits(4, 4);
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(96)
            .with_max_tokens(16)
            .with_runtime(metadata)
            .with_experiments(ExperimentSwitches::default().with_fht_dke(true));
        let architecture = TransformerRuntimeArchitecture::new(1, 16, 2, 1, 64);
        let transformer_plan = TransformerPlanDigest::new(
            Some("acceptance-boundary-dense-compute"),
            vec![TransformerLayerBudget::new(
                0,
                TransformerAttentionKind::Global,
                0.5,
                64,
            )],
        );
        let hardware = HardwareAllocator::new().plan(
            HardwareLoadSnapshot::new(DeviceClass::DiscreteGpu, 0.1, 0.1, 0.1, 0.1),
            TaskProfile::Coding,
            512,
            HierarchyWeights::for_profile(TaskProfile::Coding),
        );
        let execution = hardware.adapter_execution_context();
        let planning = RuntimePlanningDigest::from_request(
            &request,
            RouteBudget {
                threshold: 0.50,
                attention_tokens: 8,
                fast_tokens: 2,
                attention_fraction: 0.80,
            },
            &execution,
            &[],
            &DeterministicFhtDkeBudgeter::new(0.10, 0.60, 64),
        );
        let imported = (0..planning.planned_kv_exchange().import_blocks)
            .map(|id| runtime_block(id as u64))
            .collect::<Vec<_>>();
        let context = RuntimeAcceptanceContext::from_request_parts(
            &request,
            architecture,
            RouteBudget::default(),
            HierarchyWeights::for_profile(TaskProfile::Coding),
            &transformer_plan,
            hardware,
            imported.clone(),
        )
        .with_planning_digest(planning);
        let runtime = context.runtime_diagnostics_seed().with_device_execution(
            "gpu",
            "gpu",
            "cpu",
            "gpu-resident",
            DeviceExecutionSource::RuntimeReported,
        );
        let mut outcome = InferenceOutcome::empty()
            .with_diagnostics(context.inference_diagnostics_seed().with_runtime(runtime));
        outcome.answer = "ok".to_owned();
        outcome.tokens.push(GeneratedToken::new("ok"));
        outcome.imported_kv = imported;

        let gate = context.boundary_gate_summary(&outcome);
        let avoided = planning.planning_summary().dense_compute_avoided_tokens();

        assert!(avoided > 0);
        assert_eq!(gate.request_planning_dense_compute_avoided_tokens, avoided);
        assert_eq!(gate.response_planning_dense_compute_avoided_tokens, avoided);
        assert_eq!(gate.planning_dense_compute_avoided_tokens(), avoided);
        assert!(gate.has_planning_dense_compute_savings());
        let readiness = context.boundary_commit_readiness_summary(&outcome);
        assert_eq!(readiness.planning_dense_compute_avoided_tokens, avoided);
        assert!(readiness.has_planning_dense_compute_savings());
        let commit = readiness.commit_summary();
        assert_eq!(commit.planning_dense_compute_avoided_tokens, avoided);
        assert!(commit.has_planning_dense_compute_savings());
        assert!(commit.commit_decision_accounting_is_consistent());
    }

    #[test]
    fn acceptance_context_submits_only_runtime_reported_device_execution_envelope() {
        let metadata = RuntimeMetadata::new("model", "tok", 128, 16)
            .with_kv_exchange(true, true)
            .with_kv_limits(1, 1)
            .with_kv_precision(8, 4);
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(16)
            .with_max_tokens(2)
            .with_runtime(metadata);
        let architecture = TransformerRuntimeArchitecture::new(1, 16, 2, 1, 64);
        let transformer_plan = TransformerPlanDigest::new(
            Some("acceptance-context-device-execution"),
            vec![TransformerLayerBudget::new(
                0,
                TransformerAttentionKind::Global,
                0.5,
                64,
            )],
        );
        let hardware = HardwareAllocator::new().plan(
            HardwareLoadSnapshot::new(DeviceClass::DiscreteGpu, 0.1, 0.1, 0.1, 0.1),
            TaskProfile::Coding,
            512,
            HierarchyWeights::for_profile(TaskProfile::Coding),
        );
        let context = RuntimeAcceptanceContext::from_request_parts(
            &request,
            architecture,
            RouteBudget::default(),
            HierarchyWeights::for_profile(TaskProfile::Coding),
            &transformer_plan,
            hardware,
            vec![runtime_block(1)],
        );
        let runtime_reported = context.runtime_diagnostics_seed().with_device_execution(
            "gpu",
            "gpu",
            "cpu",
            "gpu-resident",
            DeviceExecutionSource::RuntimeReported,
        );
        let control_plane_filled = context.runtime_diagnostics_seed().with_device_execution(
            "gpu",
            "gpu",
            "cpu",
            "gpu-resident",
            DeviceExecutionSource::ControlPlaneFilled,
        );
        let drifted_hardware = context.runtime_diagnostics_seed().with_device_execution(
            "cpu",
            "cpu",
            "gpu",
            "tiered",
            DeviceExecutionSource::RuntimeReported,
        );
        let context_before_commit_projection = context.clone();
        let runtime_reported_before_commit_projection = runtime_reported.clone();
        let control_plane_before_commit_projection = control_plane_filled.clone();
        let drifted_hardware_before_commit_projection = drifted_hardware.clone();
        let runtime_reported_summary =
            context.runtime_reported_device_execution_envelope_summary(&runtime_reported);
        let control_plane_summary =
            context.runtime_reported_device_execution_envelope_summary(&control_plane_filled);
        let drifted_hardware_summary =
            context.runtime_reported_device_execution_envelope_summary(&drifted_hardware);
        let runtime_reported_readiness =
            context.runtime_boundary_device_execution_readiness_summary(&runtime_reported);
        let control_plane_readiness =
            context.runtime_boundary_device_execution_readiness_summary(&control_plane_filled);
        let drifted_hardware_readiness =
            context.runtime_boundary_device_execution_readiness_summary(&drifted_hardware);
        let runtime_reported_commit = runtime_reported_readiness.commit_summary();
        let control_plane_commit = control_plane_readiness.commit_summary();
        let drifted_hardware_commit = drifted_hardware_readiness.commit_summary();
        let runtime_reported_context_commit =
            context.runtime_boundary_device_execution_commit_summary(&runtime_reported);
        let control_plane_context_commit =
            context.runtime_boundary_device_execution_commit_summary(&control_plane_filled);
        let drifted_hardware_context_commit =
            context.runtime_boundary_device_execution_commit_summary(&drifted_hardware);

        assert_eq!(context, context_before_commit_projection);
        assert_eq!(runtime_reported, runtime_reported_before_commit_projection);
        assert_eq!(control_plane_filled, control_plane_before_commit_projection);
        assert_eq!(drifted_hardware, drifted_hardware_before_commit_projection);

        assert!(runtime_reported_summary.runtime_diagnostics_contract_admitted);
        assert!(runtime_reported_summary.hardware_contract_admitted);
        assert!(runtime_reported_summary.hardware_diagnostics_admitted);
        assert!(runtime_reported_summary.can_submit_runtime_device_execution_envelope());
        assert!(runtime_reported_readiness.runtime_reported_metadata_ready);
        assert!(runtime_reported_readiness.device_execution_envelope_ready);
        assert_eq!(
            runtime_reported_readiness.device_execution_signal_component_count(),
            4
        );
        assert_eq!(
            runtime_reported_readiness.device_execution_blocker_component_count(),
            0
        );
        assert!(runtime_reported_readiness.device_execution_readiness_accounting_is_consistent());
        assert!(runtime_reported_readiness.device_execution_readiness_is_clean());
        assert!(runtime_reported_readiness.can_commit_runtime_boundary_device_execution());
        assert_eq!(
            runtime_reported_readiness.runtime_boundary_device_execution_commit_action(),
            RuntimeBoundaryDeviceExecutionCommitAction::CommitRuntimeBoundaryDeviceExecution
        );
        assert!(runtime_reported_readiness.commit_action_matches_readiness());
        assert!(context.can_submit_runtime_reported_device_execution_envelope(&runtime_reported));
        assert_eq!(
            runtime_reported_commit.readiness,
            runtime_reported_readiness
        );
        assert_eq!(runtime_reported_context_commit, runtime_reported_commit);
        assert_eq!(
            runtime_reported_commit.action,
            RuntimeBoundaryDeviceExecutionCommitAction::CommitRuntimeBoundaryDeviceExecution
        );
        assert!(runtime_reported_commit.can_commit);
        assert!(runtime_reported_commit.action_can_commit());
        assert!(!runtime_reported_commit.should_wait_for_runtime_reported_metadata);
        assert!(!runtime_reported_commit.should_repair_device_execution_envelope);
        assert_eq!(runtime_reported_commit.total_signal_component_count, 4);
        assert_eq!(runtime_reported_commit.total_blocker_component_count, 0);
        assert!(!runtime_reported_commit.has_blocker_components());
        assert!(runtime_reported_commit.component_accounting_consistent);
        assert!(runtime_reported_commit.commit_decision_accounting_is_consistent());
        assert!(control_plane_summary.runtime_diagnostics_contract_admitted);
        assert!(control_plane_summary.hardware_contract_admitted);
        assert!(control_plane_summary.hardware_diagnostics_admitted);
        assert!(control_plane_summary.can_submit_runtime_device_execution_envelope());
        assert!(!control_plane_readiness.runtime_reported_metadata_ready);
        assert!(control_plane_readiness.device_execution_envelope_ready);
        assert_eq!(
            control_plane_readiness.device_execution_signal_component_count(),
            3
        );
        assert_eq!(
            control_plane_readiness.device_execution_blocker_component_count(),
            1
        );
        assert!(control_plane_readiness.device_execution_readiness_accounting_is_consistent());
        assert!(!control_plane_readiness.device_execution_readiness_is_clean());
        assert!(!control_plane_readiness.can_commit_runtime_boundary_device_execution());
        assert_eq!(
            control_plane_readiness.runtime_boundary_device_execution_commit_action(),
            RuntimeBoundaryDeviceExecutionCommitAction::WaitForRuntimeReportedDeviceExecutionMetadata
        );
        assert!(control_plane_readiness.commit_action_matches_readiness());
        assert!(
            !context.can_submit_runtime_reported_device_execution_envelope(&control_plane_filled)
        );
        assert_eq!(control_plane_commit.readiness, control_plane_readiness);
        assert_eq!(control_plane_context_commit, control_plane_commit);
        assert_eq!(
            control_plane_commit.action,
            RuntimeBoundaryDeviceExecutionCommitAction::WaitForRuntimeReportedDeviceExecutionMetadata
        );
        assert!(!control_plane_commit.can_commit);
        assert!(!control_plane_commit.action_can_commit());
        assert!(control_plane_commit.should_wait_for_runtime_reported_metadata);
        assert!(control_plane_commit.action_should_wait_for_runtime_reported_metadata());
        assert!(!control_plane_commit.should_repair_device_execution_envelope);
        assert_eq!(control_plane_commit.total_signal_component_count, 3);
        assert_eq!(control_plane_commit.total_blocker_component_count, 1);
        assert!(control_plane_commit.has_blocker_components());
        assert!(control_plane_commit.component_accounting_consistent);
        assert!(control_plane_commit.commit_decision_accounting_is_consistent());
        assert!(drifted_hardware_summary.runtime_diagnostics_contract_admitted);
        assert!(!drifted_hardware_summary.hardware_contract_admitted);
        assert!(!drifted_hardware_summary.hardware_diagnostics_admitted);
        assert!(!drifted_hardware_summary.can_submit_runtime_device_execution_envelope());
        assert!(drifted_hardware_readiness.runtime_reported_metadata_ready);
        assert!(!drifted_hardware_readiness.device_execution_envelope_ready);
        assert_eq!(
            drifted_hardware_readiness.device_execution_signal_component_count(),
            2
        );
        assert_eq!(
            drifted_hardware_readiness.device_execution_blocker_component_count(),
            2
        );
        assert!(drifted_hardware_readiness.device_execution_readiness_accounting_is_consistent());
        assert!(!drifted_hardware_readiness.device_execution_readiness_is_clean());
        assert!(!drifted_hardware_readiness.can_commit_runtime_boundary_device_execution());
        assert_eq!(
            drifted_hardware_readiness.runtime_boundary_device_execution_commit_action(),
            RuntimeBoundaryDeviceExecutionCommitAction::RepairDeviceExecutionEnvelope
        );
        assert!(drifted_hardware_readiness.commit_action_matches_readiness());
        assert!(!context.can_submit_runtime_reported_device_execution_envelope(&drifted_hardware));
        assert_eq!(
            drifted_hardware_commit.readiness,
            drifted_hardware_readiness
        );
        assert_eq!(drifted_hardware_context_commit, drifted_hardware_commit);
        assert_eq!(
            drifted_hardware_commit.action,
            RuntimeBoundaryDeviceExecutionCommitAction::RepairDeviceExecutionEnvelope
        );
        assert!(!drifted_hardware_commit.can_commit);
        assert!(!drifted_hardware_commit.action_can_commit());
        assert!(!drifted_hardware_commit.should_wait_for_runtime_reported_metadata);
        assert!(drifted_hardware_commit.should_repair_device_execution_envelope);
        assert!(drifted_hardware_commit.action_should_repair_device_execution_envelope());
        assert_eq!(drifted_hardware_commit.total_signal_component_count, 2);
        assert_eq!(drifted_hardware_commit.total_blocker_component_count, 2);
        assert!(drifted_hardware_commit.has_blocker_components());
        assert!(drifted_hardware_commit.component_accounting_consistent);
        assert!(drifted_hardware_commit.commit_decision_accounting_is_consistent());
    }

    #[test]
    fn boundary_commit_report_consumes_device_execution_commit_summary_without_committing() {
        let metadata = RuntimeMetadata::new("model", "tok", 128, 16)
            .with_kv_exchange(true, true)
            .with_kv_limits(1, 1)
            .with_kv_precision(8, 4);
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(16)
            .with_max_tokens(2)
            .with_runtime(metadata);
        let architecture = TransformerRuntimeArchitecture::new(1, 16, 2, 1, 64);
        let transformer_plan = TransformerPlanDigest::new(
            Some("boundary-device-execution-evidence"),
            vec![TransformerLayerBudget::new(
                0,
                TransformerAttentionKind::Global,
                0.5,
                64,
            )],
        );
        let hardware = HardwareAllocator::new().plan(
            HardwareLoadSnapshot::new(DeviceClass::DiscreteGpu, 0.1, 0.1, 0.1, 0.1),
            TaskProfile::Coding,
            512,
            HierarchyWeights::for_profile(TaskProfile::Coding),
        );
        let context = RuntimeAcceptanceContext::from_request_parts(
            &request,
            architecture,
            RouteBudget::default(),
            HierarchyWeights::for_profile(TaskProfile::Coding),
            &transformer_plan,
            hardware,
            vec![runtime_block(1)],
        );
        let runtime_reported = context.runtime_diagnostics_seed().with_device_execution(
            "gpu",
            "gpu",
            "cpu",
            "gpu-resident",
            DeviceExecutionSource::RuntimeReported,
        );
        let device_execution_commit =
            context.runtime_boundary_device_execution_commit_summary(&runtime_reported);
        let boundary_readiness = RuntimeBoundaryCommitReadinessSummary {
            request_acceptance_ready: true,
            response_acceptance_ready: false,
            boundary_acceptance_ready: true,
            boundary_envelope_ready: true,
            boundary_adapter_ready: true,
            boundary_kv_ready: true,
            boundary_gate_ready: true,
            runtime_response_ready: false,
            request_acceptance_signal_component_count: 1,
            response_acceptance_signal_component_count: 0,
            boundary_acceptance_signal_component_count: 1,
            acceptance_signal_component_count: 2,
            boundary_envelope_signal_component_count: 1,
            boundary_adapter_signal_component_count: 1,
            boundary_kv_signal_component_count: 1,
            boundary_gate_signal_component_count: 1,
            planning_dense_compute_avoided_tokens: 0,
            envelope_signal_component_count: 1,
            adapter_signal_component_count: 1,
            kv_signal_component_count: 1,
            gate_signal_component_count: 1,
            runtime_response_signal_component_count: 0,
            total_signal_component_count: 6,
            request_acceptance_blocker_component_count: 0,
            response_acceptance_blocker_component_count: 1,
            boundary_acceptance_blocker_component_count: 0,
            acceptance_blocker_component_count: 1,
            boundary_envelope_blocker_component_count: 0,
            boundary_adapter_blocker_component_count: 0,
            boundary_kv_blocker_component_count: 0,
            boundary_gate_blocker_component_count: 0,
            envelope_blocker_component_count: 0,
            adapter_blocker_component_count: 0,
            kv_blocker_component_count: 0,
            gate_blocker_component_count: 0,
            runtime_response_blocker_component_count: 1,
            total_blocker_component_count: 2,
            component_accounting_consistent: true,
        };
        let boundary_commit = boundary_readiness.commit_summary();

        assert!(device_execution_commit.can_commit);
        assert!(device_execution_commit.commit_decision_accounting_is_consistent());
        assert_eq!(
            device_execution_commit.action,
            RuntimeBoundaryDeviceExecutionCommitAction::CommitRuntimeBoundaryDeviceExecution
        );
        assert_eq!(
            boundary_commit.action,
            RuntimeBoundaryCommitAction::ReturnRuntimeFailure
        );
        assert!(!boundary_commit.can_commit_runtime_boundary());
        assert!(boundary_commit.should_return_runtime_failure());
        assert!(
            boundary_commit.can_consume_runtime_boundary_device_execution_commit_summary(
                device_execution_commit
            )
        );
        assert!(
            boundary_commit.consumes_runtime_boundary_device_execution_without_boundary_commit(
                device_execution_commit
            )
        );
        assert_eq!(
            boundary_commit.failure_report_count,
            boundary_readiness.failure_report_count()
        );
        assert_eq!(
            context.runtime_boundary_device_execution_commit_summary(&runtime_reported),
            device_execution_commit
        );
    }

    #[test]
    fn acceptance_context_boundary_adapter_summary_confirms_planned_runtime_adapter() {
        let metadata = RuntimeMetadata::new("model", "tok", 128, 16)
            .with_kv_exchange(true, true)
            .with_kv_limits(1, 1);
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(16)
            .with_max_tokens(2)
            .with_runtime(metadata);
        let architecture = TransformerRuntimeArchitecture::new(1, 16, 2, 1, 64);
        let transformer_plan = TransformerPlanDigest::new(
            Some("acceptance-adapter-summary"),
            vec![TransformerLayerBudget::new(
                0,
                TransformerAttentionKind::Global,
                0.5,
                64,
            )],
        );
        let hardware = HardwareAllocator::new().plan(
            HardwareLoadSnapshot::new(DeviceClass::DiscreteGpu, 0.1, 0.1, 0.1, 0.1),
            TaskProfile::Coding,
            512,
            HierarchyWeights::for_profile(TaskProfile::Coding),
        );
        let execution = hardware
            .adapter_execution_context()
            .clamp_for_runtime(&request.runtime);
        let planning = crate::planning::RuntimePlanningDigest::from_request(
            &request,
            RouteBudget::default(),
            &execution,
            &[],
            &DeterministicFhtDkeBudgeter::default(),
        );
        let context = RuntimeAcceptanceContext::from_request_parts(
            &request,
            architecture,
            RouteBudget::default(),
            HierarchyWeights::for_profile(TaskProfile::Coding),
            &transformer_plan,
            hardware,
            vec![runtime_block(1)],
        )
        .with_planning_digest(planning);
        let runtime = RuntimeDiagnostics::empty()
            .with_selected_adapter(planning.adapter_selection.adapter)
            .with_kv_exchange(1, 0);
        let outcome = InferenceOutcome::empty().with_diagnostics(
            InferenceDiagnostics::new(RouteBudget::default()).with_runtime(runtime),
        );

        let summary = context.boundary_adapter_summary(&outcome);
        let selection = summary.selection.expect("planning selection summary");

        assert_eq!(
            summary.request_selected_adapter,
            Some(planning.adapter_selection.adapter)
        );
        assert_eq!(
            summary.runtime_selected_adapter,
            Some(planning.adapter_selection.adapter)
        );
        assert_eq!(summary.adapter_candidate_count, execution.adapters.len());
        assert!(summary.has_planning_selection);
        assert!(summary.runtime_adapter_matches_request);
        assert!(summary.runtime_adapter_allowed);
        assert!(summary.runtime_selection_confirmed());
        assert!(!summary.runtime_adapter_problem());
        assert_eq!(summary.request_adapter_boundary_signal_component_count(), 2);
        assert_eq!(summary.runtime_adapter_boundary_signal_component_count(), 3);
        assert_eq!(
            summary.planning_selection_boundary_signal_component_count(),
            2
        );
        assert_eq!(summary.adapter_boundary_signal_component_count(), 7);
        assert!(summary.has_adapter_boundary_signals());
        assert_eq!(summary.request_adapter_problem_component_count(), 0);
        assert_eq!(summary.runtime_adapter_problem_component_count(), 0);
        assert_eq!(summary.planning_selection_problem_component_count(), 0);
        assert_eq!(summary.adapter_boundary_problem_component_count(), 0);
        assert!(!summary.has_adapter_boundary_problem_components());
        assert!(summary.adapter_boundary_is_consistent());
        assert_eq!(summary.adapter_boundary_commit_signal_component_count(), 7);
        assert_eq!(summary.adapter_boundary_commit_blocker_component_count(), 0);
        assert!(summary.adapter_boundary_commit_accounting_is_consistent());
        assert!(summary.adapter_boundary_commit_is_clean());
        assert!(summary.adapter_boundary_shape_is_clean());
        assert!(summary.can_commit_runtime_boundary_adapter());
        assert!(summary.can_use_runtime_boundary_adapter());
        assert!(selection.runtime_selection_confirmed());
        assert_eq!(selection.runtime_adapter_problem_component_count(), 0);
    }

    #[test]
    fn acceptance_context_boundary_adapter_summary_reports_runtime_adapter_drift() {
        let metadata = RuntimeMetadata::new("model", "tok", 128, 16)
            .with_kv_exchange(true, true)
            .with_kv_limits(1, 1);
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(16)
            .with_max_tokens(2)
            .with_runtime(metadata);
        let architecture = TransformerRuntimeArchitecture::new(1, 16, 2, 1, 64);
        let transformer_plan = TransformerPlanDigest::new(
            Some("acceptance-adapter-drift"),
            vec![TransformerLayerBudget::new(
                0,
                TransformerAttentionKind::Global,
                0.5,
                64,
            )],
        );
        let hardware = HardwareAllocator::new().plan(
            HardwareLoadSnapshot::new(DeviceClass::CpuOnly, 0.1, 0.1, 0.1, 0.1),
            TaskProfile::Coding,
            512,
            HierarchyWeights::for_profile(TaskProfile::Coding),
        );
        let context = RuntimeAcceptanceContext::from_request_parts(
            &request,
            architecture,
            RouteBudget::default(),
            HierarchyWeights::for_profile(TaskProfile::Coding),
            &transformer_plan,
            hardware,
            vec![],
        );
        let runtime = RuntimeDiagnostics::empty().with_selected_adapter(RuntimeAdapter::Cuda);
        let outcome = InferenceOutcome::empty().with_diagnostics(
            InferenceDiagnostics::new(RouteBudget::default()).with_runtime(runtime),
        );

        let summary = context.boundary_adapter_summary(&outcome);

        assert!(summary.request_adapter_reported);
        assert!(summary.runtime_adapter_reported);
        assert!(!summary.has_planning_selection);
        assert!(summary.runtime_adapter_drifted_from_request());
        assert!(summary.runtime_adapter_outside_execution_context());
        assert!(!summary.runtime_selection_confirmed());
        assert!(summary.runtime_adapter_problem());
        assert_eq!(summary.request_adapter_boundary_signal_component_count(), 2);
        assert_eq!(summary.runtime_adapter_boundary_signal_component_count(), 1);
        assert_eq!(
            summary.planning_selection_boundary_signal_component_count(),
            0
        );
        assert_eq!(summary.adapter_boundary_signal_component_count(), 3);
        assert!(summary.has_adapter_boundary_signals());
        assert_eq!(summary.request_adapter_problem_component_count(), 0);
        assert_eq!(summary.runtime_adapter_problem_component_count(), 2);
        assert_eq!(summary.planning_selection_problem_component_count(), 0);
        assert_eq!(summary.adapter_boundary_problem_component_count(), 2);
        assert!(summary.has_adapter_boundary_problem_components());
        assert!(!summary.adapter_boundary_is_consistent());
        assert_eq!(summary.adapter_boundary_commit_signal_component_count(), 3);
        assert_eq!(summary.adapter_boundary_commit_blocker_component_count(), 2);
        assert!(summary.adapter_boundary_commit_accounting_is_consistent());
        assert!(!summary.adapter_boundary_commit_is_clean());
        assert!(!summary.adapter_boundary_shape_is_clean());
        assert!(!summary.can_commit_runtime_boundary_adapter());
        assert!(!summary.can_use_runtime_boundary_adapter());
    }

    #[test]
    fn runtime_boundary_adapter_summary_counts_planning_selection_problems() {
        let summary = RuntimeBoundaryAdapterSummary {
            request_selected_adapter: Some(RuntimeAdapter::CpuSimd),
            runtime_selected_adapter: Some(RuntimeAdapter::CpuSimd),
            adapter_candidate_count: 1,
            has_planning_selection: true,
            selection: Some(AdapterSelectionRuntimeSummary {
                selection: AdapterSelection {
                    adapter: RuntimeAdapter::Cuda,
                    score: 0.80,
                    experience_id: Some(7),
                    used_fallback: true,
                },
                fallback_reason: AdapterFallbackReason::NoFallback,
                allowed_adapter_count: 1,
                matching_observation_count: 0,
                runtime_selected_adapter: Some(RuntimeAdapter::CpuSimd),
                runtime_adapter_reported: true,
                runtime_adapter_matches_selection: false,
                runtime_adapter_allowed: true,
            }),
            request_adapter_reported: true,
            runtime_adapter_reported: true,
            runtime_adapter_matches_request: true,
            runtime_adapter_allowed: true,
        };

        let selection = summary.selection.expect("selection summary");

        assert!(!summary.request_adapter_missing());
        assert!(!summary.runtime_adapter_missing());
        assert!(!summary.runtime_adapter_drifted_from_request());
        assert!(!summary.runtime_adapter_outside_execution_context());
        assert!(selection.runtime_selection_drifted());
        assert!(selection.fallback_without_reason());
        assert_eq!(selection.runtime_adapter_problem_component_count(), 2);
        assert_eq!(summary.request_adapter_boundary_signal_component_count(), 2);
        assert_eq!(summary.runtime_adapter_boundary_signal_component_count(), 3);
        assert_eq!(
            summary.planning_selection_boundary_signal_component_count(),
            1
        );
        assert_eq!(summary.adapter_boundary_signal_component_count(), 6);
        assert!(summary.has_adapter_boundary_signals());
        assert_eq!(summary.request_adapter_problem_component_count(), 0);
        assert_eq!(summary.runtime_adapter_problem_component_count(), 0);
        assert_eq!(summary.planning_selection_problem_component_count(), 2);
        assert_eq!(summary.adapter_boundary_problem_component_count(), 2);
        assert!(summary.has_adapter_boundary_problem_components());
        assert!(summary.runtime_adapter_problem());
        assert_eq!(summary.adapter_boundary_commit_signal_component_count(), 6);
        assert_eq!(summary.adapter_boundary_commit_blocker_component_count(), 2);
        assert!(summary.adapter_boundary_commit_accounting_is_consistent());
        assert!(!summary.adapter_boundary_commit_is_clean());
        assert!(!summary.adapter_boundary_shape_is_clean());
        assert!(!summary.can_commit_runtime_boundary_adapter());
        assert!(!summary.can_use_runtime_boundary_adapter());
    }

    #[test]
    fn acceptance_context_boundary_envelope_summary_reports_shape_drift() {
        let metadata = RuntimeMetadata::new("model", "tok", 128, 16)
            .with_kv_exchange(true, true)
            .with_kv_limits(1, 1);
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(120)
            .with_max_tokens(16)
            .with_runtime(metadata);
        let architecture = TransformerRuntimeArchitecture::new(1, 16, 2, 1, 64);
        let transformer_plan = TransformerPlanDigest::new(
            Some("acceptance-envelope-summary"),
            vec![TransformerLayerBudget::new(
                0,
                TransformerAttentionKind::Global,
                0.5,
                64,
            )],
        );
        let hardware = HardwareAllocator::new().plan(
            HardwareLoadSnapshot::new(DeviceClass::DiscreteGpu, 0.1, 0.1, 0.1, 0.1),
            TaskProfile::Coding,
            512,
            HierarchyWeights::for_profile(TaskProfile::Coding),
        );
        let context = RuntimeAcceptanceContext::from_request_parts(
            &request,
            architecture,
            RouteBudget::default(),
            HierarchyWeights::for_profile(TaskProfile::Coding),
            &transformer_plan,
            hardware,
            vec![runtime_block(1)],
        );
        let runtime = RuntimeDiagnostics::empty()
            .with_selected_adapter(RuntimeAdapter::Cuda)
            .with_kv_exchange(1, 0);
        let mut outcome = InferenceOutcome::empty().with_diagnostics(
            InferenceDiagnostics::new(RouteBudget::default()).with_runtime(runtime),
        );
        outcome.answer = "too many tokens".to_owned();
        outcome.tokens.push(GeneratedToken::new("too"));
        outcome.tokens.push(GeneratedToken::new("many"));
        outcome.tokens.push(GeneratedToken::new("tokens"));
        outcome.imported_kv.push(runtime_block(1));
        outcome.imported_kv.push(runtime_block(2));

        let summary = context.boundary_envelope_summary(&outcome);

        assert!(summary.request_was_context_limited());
        assert!(summary.has_request_adapter_candidate());
        assert_eq!(summary.request.max_tokens, 16);
        assert_eq!(summary.request.max_generated_tokens, 8);
        assert!(!summary.response_exceeds_request_tokens());
        assert!(!summary.response_imported_kv_matches_request());
        assert!(!summary.response_kv_counts_match_diagnostics());
        assert_eq!(summary.response_token_drift_component_count(), 0);
        assert_eq!(summary.imported_kv_drift_component_count(), 1);
        assert_eq!(summary.diagnostics_kv_drift_component_count(), 1);
        assert_eq!(
            summary.runtime_execution_signal_missing_component_count(),
            0
        );
        assert_eq!(summary.request_adapter_signal_missing_component_count(), 0);
        assert_eq!(summary.context_pressure_signal_component_count(), 1);
        assert_eq!(
            summary.response_uncertainty_coverage_signal_component_count(),
            2
        );
        assert!(summary.response_has_uncertainty_coverage_signals());
        assert_eq!(
            summary.response_uncertainty_metric_problem_component_count(),
            0
        );
        assert!(summary.response_uncertainty_accounting_is_consistent());
        assert_eq!(summary.boundary_shape_drift_component_count(), 2);
        assert_eq!(summary.boundary_envelope_signal_component_count(), 5);
        let expected_blockers = summary
            .request
            .request_envelope_commit_blocker_component_count()
            .saturating_add(
                summary
                    .response
                    .runtime_response_envelope_commit_blocker_component_count(),
            )
            .saturating_add(summary.boundary_shape_drift_component_count())
            .saturating_add(usize::from(
                summary.request.request_envelope_commit_is_clean()
                    && !summary.request.can_commit_runtime_request_envelope(),
            ))
            .saturating_add(usize::from(
                summary.response.runtime_response_envelope_commit_is_clean()
                    && !summary.response.can_commit_runtime_response_envelope(),
            ));
        assert_eq!(
            summary.runtime_boundary_envelope_commit_blocker_component_count(),
            expected_blockers
        );
        assert!(summary.has_runtime_boundary_envelope_commit_blockers());
        assert!(summary.runtime_boundary_envelope_commit_accounting_is_consistent());
        assert!(!summary.boundary_envelope_commit_is_clean());
        assert!(!summary.can_commit_runtime_boundary_envelope());
        assert!(!summary.boundary_shape_is_consistent());
        assert!(!summary.boundary_envelope_is_consistent());
        assert!(!summary.boundary_envelope_shape_is_clean());
        assert!(!summary.can_use_runtime_boundary_envelope());
        let readiness = context.boundary_commit_readiness_summary(&outcome);

        assert!(readiness.request_acceptance_ready);
        assert!(!readiness.response_acceptance_ready);
        assert!(!readiness.boundary_acceptance_ready);
        assert!(!readiness.boundary_envelope_ready);
        assert!(readiness.boundary_adapter_ready);
        assert!(!readiness.boundary_kv_ready);
        assert!(!readiness.boundary_gate_ready);
        assert!(!readiness.runtime_response_ready);
        assert!(!readiness.all_acceptance_ready());
        assert!(!readiness.all_boundary_summaries_ready());
        assert_eq!(
            readiness.total_blocker_component_count,
            readiness
                .acceptance_blocker_component_count
                .saturating_add(readiness.envelope_blocker_component_count)
                .saturating_add(readiness.adapter_blocker_component_count)
                .saturating_add(readiness.kv_blocker_component_count)
                .saturating_add(readiness.gate_blocker_component_count)
                .saturating_add(readiness.runtime_response_blocker_component_count)
        );
        assert_eq!(
            readiness.first_unready_stage(),
            Some(RuntimeBoundaryCommitStage::ResponseAcceptance)
        );
        assert_eq!(
            readiness.first_blocking_stage(),
            Some(RuntimeBoundaryCommitStage::ResponseAcceptance)
        );
        assert_eq!(
            readiness.stage_blocker_component_count(RuntimeBoundaryCommitStage::BoundaryEnvelope),
            readiness.boundary_envelope_blocker_component_count
        );
        assert!(readiness.has_commit_blockers());
        assert!(readiness.readiness_accounting_is_consistent());
        assert!(!readiness.readiness_commit_is_clean());
        assert!(!readiness.can_commit_runtime_boundary());
        assert!(readiness.has_failure_reports());
        assert_eq!(
            readiness.failure_report_count(),
            readiness.failure_reports().len()
        );
        let primary_failure = readiness
            .primary_failure_report()
            .expect("boundary commit failure is reported");
        let primary_failure_summary = readiness
            .primary_failure_summary()
            .expect("boundary commit failure summary is reported");
        assert_eq!(primary_failure.kind, RuntimeFailureKind::ContractViolation);
        assert_eq!(
            primary_failure_summary.kind,
            RuntimeFailureKind::ContractViolation
        );
        assert_eq!(
            readiness.failure_report_for(RuntimeBoundaryCommitStage::ResponseAcceptance),
            Some(primary_failure.clone())
        );
        assert_eq!(readiness.failure_reports().first(), Some(&primary_failure));
        let failure_batch = readiness.failure_batch_summary();
        assert_eq!(failure_batch.total_count, readiness.failure_report_count());
        assert_eq!(
            failure_batch.runtime_count,
            usize::from(
                readiness
                    .stage_blocker_component_count(RuntimeBoundaryCommitStage::RuntimeResponse)
                    > 0
            )
        );
        assert_eq!(
            failure_batch.contract_violation_count,
            readiness
                .failure_report_count()
                .saturating_sub(failure_batch.runtime_count)
        );
        assert!(failure_batch.can_format_runtime_failures());
        assert!(readiness.can_format_runtime_failures());
        assert_eq!(
            readiness.runtime_boundary_commit_action(),
            RuntimeBoundaryCommitAction::ReturnRuntimeFailure
        );
        let boundary_commit = readiness.commit_summary();
        assert_eq!(
            boundary_commit.action,
            RuntimeBoundaryCommitAction::ReturnRuntimeFailure
        );
        assert_eq!(
            boundary_commit.action,
            readiness.runtime_boundary_commit_action()
        );
        assert!(!boundary_commit.action_can_commit());
        assert!(boundary_commit.action_should_return_failure());
        assert!(!boundary_commit.can_commit_runtime_boundary());
        assert!(boundary_commit.should_return_runtime_failure());
        assert_eq!(
            boundary_commit.first_unready_stage,
            Some(RuntimeBoundaryCommitStage::ResponseAcceptance)
        );
        assert_eq!(
            boundary_commit.first_blocking_stage,
            Some(RuntimeBoundaryCommitStage::ResponseAcceptance)
        );
        assert_eq!(
            boundary_commit.primary_failure_summary,
            Some(primary_failure_summary)
        );
        assert_eq!(
            boundary_commit.primary_failure_report(),
            Some(primary_failure.clone())
        );
        assert_eq!(
            boundary_commit.failure_report_count,
            readiness.failure_report_count()
        );
        assert_eq!(boundary_commit.failure_batch, failure_batch);
        assert!(boundary_commit.can_format_runtime_failures);
        assert!(boundary_commit.failure_batch_shape_is_clean());
        assert!(boundary_commit.commit_decision_accounting_is_consistent());
        let failure_return = boundary_commit.failure_return_summary();
        assert_eq!(
            failure_return.source,
            RuntimeFailureReturnSource::BoundaryCommit
        );
        assert!(failure_return.should_return_failure);
        assert!(failure_return.has_primary_failure_summary);
        assert_eq!(
            failure_return.primary_failure_summary,
            Some(primary_failure_summary)
        );
        assert_eq!(
            failure_return.failure_report_count,
            readiness.failure_report_count()
        );
        assert!(failure_return.can_return_runtime_failure());
        assert!(failure_return.failure_return_accounting_is_consistent());
        let failure_return_report = boundary_commit
            .runtime_failure_return_report()
            .expect("boundary failure return report is materialized");
        assert_eq!(
            failure_return_report.source,
            RuntimeFailureReturnSource::BoundaryCommit
        );
        assert_eq!(failure_return_report.primary_failure, primary_failure);
        assert_eq!(
            failure_return_report.primary_failure_summary,
            primary_failure_summary
        );
        assert_eq!(
            failure_return_report.inference_error().message,
            failure_return_report.backend_message()
        );
        assert!(failure_return_report.can_use_runtime_failure_return_report());
    }

    #[test]
    fn acceptance_context_boundary_kv_summary_reports_namespace_and_count_drift() {
        let metadata = RuntimeMetadata::new("model", "tok", 128, 16)
            .with_kv_exchange(true, true)
            .with_kv_limits(1, 1);
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(16)
            .with_max_tokens(2)
            .with_runtime(metadata);
        let architecture = TransformerRuntimeArchitecture::new(1, 16, 2, 1, 64);
        let transformer_plan = TransformerPlanDigest::new(
            Some("acceptance-kv-summary"),
            vec![TransformerLayerBudget::new(
                0,
                TransformerAttentionKind::Global,
                0.5,
                64,
            )],
        );
        let hardware = HardwareAllocator::new().plan(
            HardwareLoadSnapshot::new(DeviceClass::DiscreteGpu, 0.1, 0.1, 0.1, 0.1),
            TaskProfile::Coding,
            512,
            HierarchyWeights::for_profile(TaskProfile::Coding),
        );
        let context = RuntimeAcceptanceContext::from_request_parts(
            &request,
            architecture,
            RouteBudget::default(),
            HierarchyWeights::for_profile(TaskProfile::Coding),
            &transformer_plan,
            hardware,
            vec![KvBlock::new(
                7,
                KvNamespace::Gist,
                0,
                0,
                0..1,
                vec![0.1],
                vec![0.2],
            )],
        );
        let runtime = RuntimeDiagnostics::empty()
            .with_selected_adapter(RuntimeAdapter::Cuda)
            .with_kv_exchange(0, 0)
            .with_weak_runtime_kv_imports_skipped(3);
        let mut outcome = InferenceOutcome::empty().with_diagnostics(
            InferenceDiagnostics::new(RouteBudget::default()).with_runtime(runtime),
        );
        outcome.answer = "ok".to_owned();
        outcome.tokens.push(GeneratedToken::new("ok"));
        outcome.imported_kv.push(runtime_block(1));
        outcome.imported_kv.push(runtime_block(2));
        outcome.exported_kv.push(KvBlock::new(
            8,
            KvNamespace::Gist,
            0,
            0,
            0..1,
            vec![0.3],
            vec![0.4],
        ));

        let response = context.response_envelope(&outcome);
        let summary = context.boundary_kv_summary(&outcome);

        assert_eq!(summary.request_imported_kv_blocks, 1);
        assert_eq!(summary.concrete_imported_kv_blocks, 1);
        assert_eq!(summary.response_imported_kv_blocks, 2);
        assert_eq!(summary.diagnostics_imported_kv_blocks, 0);
        assert_eq!(summary.diagnostics_exported_kv_blocks, 0);
        assert_eq!(response.diagnostics_weak_runtime_kv_imports_skipped, 3);
        assert_eq!(
            summary.diagnostics_weak_runtime_kv_imports_skipped,
            response.diagnostics_weak_runtime_kv_imports_skipped
        );
        assert_eq!(summary.imported_namespace_counts.gist, 1);
        assert_eq!(summary.exported_namespace_counts.gist, 1);
        assert!(summary.imported_kv_violation_count > 0);
        assert!(summary.exported_kv_violation_count > 0);
        assert!(summary.concrete_imports_match_request());
        assert!(!summary.concrete_import_count_drifted());
        assert!(!summary.response_imports_match_request());
        assert!(summary.response_import_count_drifted());
        assert!(!summary.diagnostics_match_response());
        assert!(summary.diagnostics_count_drifted());
        assert!(summary.imports_within_planning());
        assert!(summary.exports_within_runtime());
        assert!(summary.exports_within_planning());
        assert!(!summary.namespaces_are_runtime_exchange());
        assert!(summary.has_kv_violations());
        assert_eq!(summary.runtime_exchange_count_drift_component_count(), 2);
        assert_eq!(summary.planning_bound_drift_component_count(), 0);
        assert_eq!(summary.runtime_bound_drift_component_count(), 0);
        assert_eq!(summary.namespace_drift_component_count(), 1);
        assert_eq!(summary.validation_failure_component_count(), 2);
        assert_eq!(summary.kv_boundary_problem_component_count(), 5);
        assert!(summary.has_kv_boundary_problem_components());
        assert!(!summary.kv_boundary_is_consistent());
        assert!(!summary.kv_boundary_shape_is_clean());
        assert!(!summary.can_use_runtime_boundary_kv());

        let gate = context.boundary_gate_summary(&outcome);

        assert!(!gate.request_accepted);
        assert!(!gate.response_accepted);
        assert!(!gate.envelope_consistent);
        assert!(gate.adapter_consistent);
        assert!(!gate.kv_consistent);
        assert!(gate.total_violation_count > 0);
        assert!(gate.total_failure_report_count > 0);
        assert!(gate.request_acceptance_failed());
        assert!(gate.response_acceptance_failed());
        assert!(gate.has_acceptance_failures());
        assert!(gate.envelope_drifted());
        assert!(!gate.adapter_drifted());
        assert!(gate.kv_drifted());
        assert!(gate.has_boundary_drift());
        assert!(gate.has_failure_reports());
        assert!(gate.has_total_violations());
        assert_eq!(gate.request_acceptance_blocker_component_count(), 1);
        assert_eq!(gate.response_acceptance_blocker_component_count(), 1);
        assert_eq!(gate.acceptance_failure_component_count(), 2);
        assert_eq!(gate.envelope_blocker_component_count(), 1);
        assert_eq!(gate.adapter_blocker_component_count(), 0);
        assert_eq!(gate.kv_blocker_component_count(), 1);
        assert_eq!(gate.boundary_drift_component_count(), 2);
        assert_eq!(gate.mapped_failure_report_component_count(), 1);
        assert_eq!(gate.commit_blocker_component_count(), 5);
        assert!(gate.commit_gate_has_problem_components());
        assert!(gate.commit_gate_accounting_is_consistent());
        assert!(!gate.can_commit_response());
        assert!(!gate.is_clean_commit_gate());
        assert!(!gate.boundary_gate_shape_is_clean());
        assert!(!gate.can_commit_runtime_response());
    }

    #[test]
    fn runtime_boundary_kv_summary_counts_planning_and_runtime_bound_drift() {
        let summary = RuntimeBoundaryKvSummary {
            request_imported_kv_blocks: 3,
            concrete_imported_kv_blocks: 2,
            accepted_imported_kv_blocks: 2,
            imported_kv_violation_count: 0,
            response_imported_kv_blocks: 4,
            response_exported_kv_blocks: 3,
            diagnostics_imported_kv_blocks: 1,
            diagnostics_exported_kv_blocks: 2,
            diagnostics_weak_runtime_kv_imports_skipped: 5,
            accepted_exported_kv_blocks: 0,
            exported_kv_violation_count: 1,
            imported_namespace_counts: KvNamespaceCounts {
                runtime: 2,
                semantic: 0,
                gist: 0,
                agent: 0,
                custom: 0,
            },
            exported_namespace_counts: KvNamespaceCounts {
                runtime: 1,
                semantic: 1,
                gist: 0,
                agent: 0,
                custom: 0,
            },
            runtime_import_enabled: true,
            runtime_export_enabled: true,
            runtime_max_import_blocks: 4,
            runtime_max_export_blocks: 2,
            planned_imported_kv_blocks: Some(2),
            planned_exported_kv_blocks: Some(1),
        };

        assert!(summary.concrete_import_count_drifted());
        assert!(summary.response_import_count_drifted());
        assert!(summary.diagnostics_count_drifted());
        assert!(!summary.imports_within_planning());
        assert!(!summary.exports_within_runtime());
        assert!(!summary.exports_within_planning());
        assert!(!summary.namespaces_are_runtime_exchange());
        assert!(summary.has_kv_violations());
        assert_eq!(summary.imported_kv_activity_signal_component_count(), 4);
        assert_eq!(summary.exported_kv_activity_signal_component_count(), 1);
        assert_eq!(summary.diagnostics_kv_activity_signal_component_count(), 3);
        assert_eq!(summary.runtime_kv_capability_signal_component_count(), 4);
        assert_eq!(summary.planning_kv_boundary_signal_component_count(), 2);
        assert_eq!(summary.namespace_kv_activity_signal_component_count(), 5);
        assert_eq!(summary.kv_boundary_signal_component_count(), 19);
        assert!(summary.has_kv_boundary_signals());
        assert_eq!(summary.runtime_exchange_count_drift_component_count(), 3);
        assert_eq!(summary.planning_bound_drift_component_count(), 2);
        assert_eq!(summary.runtime_bound_drift_component_count(), 1);
        assert_eq!(summary.namespace_drift_component_count(), 1);
        assert_eq!(summary.validation_failure_component_count(), 1);
        assert_eq!(summary.kv_boundary_problem_component_count(), 8);
        assert!(summary.has_kv_boundary_problem_components());
        assert!(!summary.kv_boundary_is_consistent());
        assert!(!summary.kv_boundary_shape_is_clean());
        assert!(!summary.can_use_runtime_boundary_kv());
    }

    #[test]
    fn runtime_boundary_kv_summary_counts_weak_skip_as_activity_not_drift() {
        let summary = RuntimeBoundaryKvSummary {
            request_imported_kv_blocks: 0,
            concrete_imported_kv_blocks: 0,
            accepted_imported_kv_blocks: 0,
            imported_kv_violation_count: 0,
            response_imported_kv_blocks: 0,
            response_exported_kv_blocks: 0,
            diagnostics_imported_kv_blocks: 0,
            diagnostics_exported_kv_blocks: 0,
            diagnostics_weak_runtime_kv_imports_skipped: 2,
            accepted_exported_kv_blocks: 0,
            exported_kv_violation_count: 0,
            imported_namespace_counts: KvNamespaceCounts::default(),
            exported_namespace_counts: KvNamespaceCounts::default(),
            runtime_import_enabled: false,
            runtime_export_enabled: false,
            runtime_max_import_blocks: 0,
            runtime_max_export_blocks: 0,
            planned_imported_kv_blocks: None,
            planned_exported_kv_blocks: None,
        };

        assert_eq!(summary.imported_kv_activity_signal_component_count(), 0);
        assert_eq!(summary.exported_kv_activity_signal_component_count(), 0);
        assert_eq!(summary.diagnostics_kv_activity_signal_component_count(), 1);
        assert_eq!(summary.kv_boundary_signal_component_count(), 1);
        assert!(summary.has_kv_boundary_signals());
        assert_eq!(summary.runtime_exchange_count_drift_component_count(), 0);
        assert_eq!(summary.kv_boundary_problem_component_count(), 0);
        assert!(!summary.has_kv_boundary_problem_components());
        assert!(summary.kv_boundary_is_consistent());
        assert!(summary.kv_boundary_shape_is_clean());
        assert!(summary.can_use_runtime_boundary_kv());
    }

    #[test]
    fn acceptance_context_constructor_clamps_hardware_execution_for_runtime_limits() {
        let metadata = RuntimeMetadata::new("model", "tok", 128, 4)
            .with_kv_exchange(true, false)
            .with_kv_limits(1, 0);
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(16)
            .with_max_tokens(8)
            .with_runtime(metadata);
        let architecture = TransformerRuntimeArchitecture::new(1, 16, 2, 1, 64);
        let transformer_plan = TransformerPlanDigest::new(
            Some("clamped-acceptance-context"),
            vec![TransformerLayerBudget::new(
                0,
                TransformerAttentionKind::Global,
                0.5,
                64,
            )],
        );
        let hardware = HardwareAllocator::new().plan(
            HardwareLoadSnapshot::new(DeviceClass::DiscreteGpu, 0.1, 0.1, 0.1, 0.1),
            TaskProfile::Coding,
            4096,
            HierarchyWeights::for_profile(TaskProfile::Coding),
        );

        let context = RuntimeAcceptanceContext::from_request_parts(
            &request,
            architecture,
            RouteBudget::default(),
            HierarchyWeights::for_profile(TaskProfile::Coding),
            &transformer_plan,
            hardware,
            vec![runtime_block(1)],
        );

        assert_eq!(context.request.imported_kv_blocks, 1);
        assert_eq!(context.request.kv_prefetch_blocks, 1);
        assert!(context.request_acceptance_report().is_accepted());
    }

    #[test]
    fn acceptance_context_clamps_prefetch_but_blocks_excess_import_commit() {
        let metadata = RuntimeMetadata::new("model", "tok", 128, 4)
            .with_kv_exchange(true, true)
            .with_kv_limits(1, 1);
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(16)
            .with_max_tokens(8)
            .with_runtime(metadata);
        let architecture = TransformerRuntimeArchitecture::new(1, 16, 2, 1, 64);
        let transformer_plan = TransformerPlanDigest::new(
            Some("clamped-prefetch-excess-import"),
            vec![TransformerLayerBudget::new(
                0,
                TransformerAttentionKind::Global,
                0.5,
                64,
            )],
        );
        let hardware = HardwareAllocator::new().plan(
            HardwareLoadSnapshot::new(DeviceClass::Server, 0.1, 0.1, 0.1, 0.1),
            TaskProfile::Coding,
            4096,
            HierarchyWeights::for_profile(TaskProfile::Coding),
        );
        let requested_execution = hardware.adapter_execution_context();
        let clamp = requested_execution.runtime_clamp_summary(&request.runtime);
        let runtime_execution = requested_execution.clamp_for_runtime(&request.runtime);
        let planning = crate::planning::RuntimePlanningDigest::from_request(
            &request,
            RouteBudget {
                threshold: 0.50,
                attention_tokens: 8,
                fast_tokens: 2,
                attention_fraction: 0.80,
            },
            &runtime_execution,
            &[],
            &DeterministicFhtDkeBudgeter::default(),
        );
        let context = RuntimeAcceptanceContext::from_request_parts(
            &request,
            architecture,
            RouteBudget::default(),
            HierarchyWeights::for_profile(TaskProfile::Coding),
            &transformer_plan,
            hardware,
            vec![runtime_block(1), runtime_block(2)],
        )
        .with_planning_digest(planning);

        let report = context.request_acceptance_report();
        let gate = context.request_gate_summary();
        let envelope = context.request_envelope_summary();
        let parity = context.request.planning_parity_summary();
        let joined = report.violations().join("\n");

        assert!(clamp.kv_prefetch_was_clamped());
        assert_eq!(clamp.after.kv_prefetch_blocks, 1);
        assert_eq!(
            planning.kv_prefetch_clamp_reason(),
            RuntimePlanningKvClampReason::NotClamped
        );
        assert_eq!(planning.planned_kv_exchange().import_blocks, 1);
        assert_eq!(context.request.imported_kv_blocks, 2);
        assert_eq!(context.request.kv_prefetch_blocks, 1);
        assert_eq!(envelope.imported_kv_blocks, 2);
        assert_eq!(envelope.kv_prefetch_blocks, 1);
        assert!(parity.kv_prefetch_matches_planning.unwrap_or(false));
        assert_eq!(parity.imported_kv_matches_planning, Some(false));
        assert!(!report.is_accepted());
        assert!(!gate.request_accepted);
        assert!(!gate.can_commit_runtime_request());
        assert!(gate.has_request_contract_failures());
        assert!(!gate.has_imported_kv_failures());
        assert!(gate.has_runtime_request_commit_blockers());
        assert!(joined.contains("runtime request imports 2 KV blocks above runtime limit 1"));
        assert!(
            joined
                .contains("runtime request imported KV count 2 differs from planned KV imports 1")
        );
    }

    #[test]
    fn acceptance_context_reports_imported_kv_count_mismatches_against_planning() {
        let metadata = RuntimeMetadata::new("model", "tok", 128, 4)
            .with_kv_exchange(true, true)
            .with_kv_limits(2, 2);
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(16)
            .with_max_tokens(8)
            .with_runtime(metadata);
        let architecture = TransformerRuntimeArchitecture::new(1, 16, 2, 1, 64);
        let transformer_plan = TransformerPlanDigest::new(
            Some("planning-import-parity"),
            vec![TransformerLayerBudget::new(
                0,
                TransformerAttentionKind::Global,
                0.5,
                64,
            )],
        );
        let hardware = HardwareAllocator::new().plan(
            HardwareLoadSnapshot::new(DeviceClass::DiscreteGpu, 0.1, 0.1, 0.1, 0.1),
            TaskProfile::Coding,
            4096,
            HierarchyWeights::for_profile(TaskProfile::Coding),
        );
        let execution = hardware
            .adapter_execution_context()
            .clamp_for_runtime(&request.runtime);
        let planning = crate::planning::RuntimePlanningDigest::from_request(
            &request,
            RouteBudget {
                threshold: 0.50,
                attention_tokens: 8,
                fast_tokens: 2,
                attention_fraction: 0.80,
            },
            &execution,
            &[],
            &DeterministicFhtDkeBudgeter::default(),
        );
        let context = RuntimeAcceptanceContext::from_request_parts(
            &request,
            architecture,
            RouteBudget::default(),
            HierarchyWeights::for_profile(TaskProfile::Coding),
            &transformer_plan,
            hardware,
            vec![runtime_block(1)],
        )
        .with_planning_digest(planning);

        let report = context.request_acceptance_report();
        let summary = context.request_acceptance_summary();
        let joined = report.violations().join("\n");

        assert_eq!(planning.planned_kv_exchange().import_blocks, 2);
        assert!(!report.is_accepted());
        assert!(!summary.accepted);
        assert!(summary.has_request_contract_failures());
        assert_eq!(
            summary.request_violation_count,
            report.request_violations.len()
        );
        assert!(joined.contains("imported KV count 1 differs from planned KV imports 2"));
    }

    #[test]
    fn acceptance_context_accepts_planned_imported_kv_blocks() {
        let metadata = RuntimeMetadata::new("model", "tok", 128, 4)
            .with_kv_exchange(true, true)
            .with_kv_limits(2, 2);
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(16)
            .with_max_tokens(8)
            .with_runtime(metadata);
        let architecture = TransformerRuntimeArchitecture::new(1, 16, 2, 1, 64);
        let transformer_plan = TransformerPlanDigest::new(
            Some("planned-import-parity"),
            vec![TransformerLayerBudget::new(
                0,
                TransformerAttentionKind::Global,
                0.5,
                64,
            )],
        );
        let hardware = HardwareAllocator::new().plan(
            HardwareLoadSnapshot::new(DeviceClass::DiscreteGpu, 0.1, 0.1, 0.1, 0.1),
            TaskProfile::Coding,
            4096,
            HierarchyWeights::for_profile(TaskProfile::Coding),
        );
        let execution = hardware
            .adapter_execution_context()
            .clamp_for_runtime(&request.runtime);
        let planning = crate::planning::RuntimePlanningDigest::from_request(
            &request,
            RouteBudget {
                threshold: 0.50,
                attention_tokens: 8,
                fast_tokens: 2,
                attention_fraction: 0.80,
            },
            &execution,
            &[],
            &DeterministicFhtDkeBudgeter::default(),
        );
        let context = RuntimeAcceptanceContext::from_request_parts(
            &request,
            architecture,
            RouteBudget::default(),
            HierarchyWeights::for_profile(TaskProfile::Coding),
            &transformer_plan,
            hardware,
            vec![runtime_block(1), runtime_block(2)],
        )
        .with_planning_digest(planning);

        let report = context.request_acceptance_report();

        assert_eq!(planning.planned_kv_exchange().import_blocks, 2);
        assert!(report.is_accepted());
        assert_eq!(report.accepted_imported_kv_blocks().len(), 2);
    }

    #[test]
    fn acceptance_context_manifest_readiness_blocks_manifest_policy_drift_after_clean_request() {
        let metadata = RuntimeMetadata::new("model", "tok", 128, 4)
            .with_kv_exchange(true, true)
            .with_kv_limits(2, 2);
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(16)
            .with_max_tokens(8)
            .with_runtime(metadata.clone());
        let architecture = TransformerRuntimeArchitecture::new(1, 16, 2, 1, 64);
        let transformer_plan = TransformerPlanDigest::new(
            Some("manifest-policy-drift-after-clean-request"),
            vec![TransformerLayerBudget::new(
                0,
                TransformerAttentionKind::Global,
                0.5,
                64,
            )],
        );
        let hardware = HardwareAllocator::new().plan(
            HardwareLoadSnapshot::new(DeviceClass::DiscreteGpu, 0.1, 0.1, 0.1, 0.1),
            TaskProfile::Coding,
            4096,
            HierarchyWeights::for_profile(TaskProfile::Coding),
        );
        let execution = hardware
            .adapter_execution_context()
            .clamp_for_runtime(&request.runtime);
        let planning = RuntimePlanningDigest::from_request(
            &request,
            RouteBudget {
                threshold: 0.50,
                attention_tokens: 8,
                fast_tokens: 2,
                attention_fraction: 0.80,
            },
            &execution,
            &[],
            &DeterministicFhtDkeBudgeter::default(),
        );
        let planned_kv = planning.planned_kv_exchange();
        let manifest = RuntimeManifestDigest::from_metadata(metadata)
            .with_architecture(architecture)
            .with_kv_policy(RuntimeKvPolicy {
                import_enabled: true,
                export_enabled: true,
                max_import_blocks: planned_kv.import_blocks.saturating_sub(1),
                max_export_blocks: 0,
            });
        let context = RuntimeAcceptanceContext::from_request_parts(
            &request,
            architecture,
            RouteBudget::default(),
            HierarchyWeights::for_profile(TaskProfile::Coding),
            &transformer_plan,
            hardware,
            vec![runtime_block(1), runtime_block(2)],
        )
        .with_planning_digest(planning);
        let runtime_planning = clean_runtime_planning_readiness();

        let request_readiness = context.request_planning_readiness_summary(runtime_planning);
        let manifest_readiness = context
            .manifest_request_planning_readiness_summary(runtime_planning, &manifest)
            .expect("planning digest is attached");

        assert_eq!(planned_kv.import_blocks, 2);
        assert!(context.request_acceptance_report().is_accepted());
        assert!(context.request_gate_summary().can_commit_runtime_request());
        assert!(request_readiness.request_planning_ready());
        assert!(request_readiness.request_gate_ready());
        assert!(request_readiness.can_commit_runtime_request_planning());
        assert!(!manifest_readiness.manifest_kv_bridge_ready());
        assert!(manifest_readiness.request_planning_ready());
        assert_eq!(
            manifest_readiness.first_unready_stage(),
            Some(RuntimeRequestManifestPlanningReadinessStage::ManifestKvBridge)
        );
        assert_eq!(
            manifest_readiness.first_blocking_stage(),
            Some(RuntimeRequestManifestPlanningReadinessStage::ManifestKvBridge)
        );
        assert_eq!(
            manifest_readiness.request_planning_blocker_component_count,
            0
        );
        assert!(manifest_readiness.manifest_kv_bridge_blocker_component_count > 0);
        assert!(manifest_readiness.has_manifest_request_planning_blockers());
        assert!(manifest_readiness.manifest_request_planning_accounting_is_consistent());
        assert!(!manifest_readiness.manifest_request_planning_is_clean());
        assert!(!manifest_readiness.can_commit_manifest_request_planning());
    }

    #[test]
    fn acceptance_context_manifest_bridge_blocks_export_disabled_after_clean_request() {
        let metadata = RuntimeMetadata::new("model", "tok", 128, 4)
            .with_kv_exchange(true, true)
            .with_kv_limits(2, 2);
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(16)
            .with_max_tokens(8)
            .with_runtime(metadata.clone())
            .with_experiments(ExperimentSwitches::default().with_fht_dke(true));
        let architecture = TransformerRuntimeArchitecture::new(1, 16, 2, 1, 64);
        let transformer_plan = TransformerPlanDigest::new(
            Some("manifest-export-disabled-after-clean-request"),
            vec![TransformerLayerBudget::new(
                0,
                TransformerAttentionKind::Global,
                0.5,
                64,
            )],
        );
        let hardware = HardwareAllocator::new().plan(
            HardwareLoadSnapshot::new(DeviceClass::DiscreteGpu, 0.1, 0.1, 0.1, 0.1),
            TaskProfile::Coding,
            4096,
            HierarchyWeights::for_profile(TaskProfile::Coding),
        );
        let execution = hardware
            .adapter_execution_context()
            .clamp_for_runtime(&request.runtime);
        let planning = RuntimePlanningDigest::from_request(
            &request,
            RouteBudget {
                threshold: 0.50,
                attention_tokens: 8,
                fast_tokens: 2,
                attention_fraction: 0.80,
            },
            &execution,
            &[],
            &DeterministicFhtDkeBudgeter::new(0.10, 0.60, 4096),
        );
        let planned_kv = planning.planned_kv_exchange();
        let manifest = RuntimeManifestDigest::from_metadata(metadata)
            .with_architecture(architecture)
            .with_kv_policy(
                RuntimeKvPolicy::from_capabilities(true, false)
                    .with_limits(planned_kv.import_blocks, planned_kv.export_blocks),
            );
        let context = RuntimeAcceptanceContext::from_request_parts(
            &request,
            architecture,
            RouteBudget::default(),
            HierarchyWeights::for_profile(TaskProfile::Coding),
            &transformer_plan,
            hardware,
            vec![runtime_block(1)],
        )
        .with_planning_digest(planning);
        let runtime_planning = clean_runtime_planning_readiness();

        let request_readiness = context.request_planning_readiness_summary(runtime_planning);
        let manifest_readiness = context
            .manifest_request_planning_readiness_summary(runtime_planning, &manifest)
            .expect("planning digest is attached");
        let bridge = planning.manifest_kv_bridge_summary(&manifest);

        assert_eq!(planned_kv.import_blocks, 1);
        assert_eq!(planned_kv.export_blocks, 1);
        assert!(context.request_acceptance_report().is_accepted());
        assert!(context.request_gate_summary().can_commit_runtime_request());
        assert!(request_readiness.can_commit_runtime_request_planning());
        assert!(bridge.import_bridge_is_clean());
        assert!(bridge.import_plan_matches_planning());
        assert!(!bridge.export_bridge_is_clean());
        assert!(!bridge.export_plan_matches_planning());
        assert!(bridge.export.requested_export_without_manifest_capacity());
        assert_eq!(
            bridge.planning_export_drift_blocks(),
            planned_kv.export_blocks
        );
        assert!(bridge.manifest_kv_bridge_accounting_is_consistent());
        assert!(!bridge.can_use_runtime_planning_manifest_kv_bridge());
        assert!(manifest_readiness.request_planning_ready());
        assert!(manifest_readiness.request_planning.request_gate_ready());
        assert!(!manifest_readiness.manifest_kv_bridge_ready());
        assert_eq!(
            manifest_readiness.first_unready_stage(),
            Some(RuntimeRequestManifestPlanningReadinessStage::ManifestKvBridge)
        );
        assert_eq!(
            manifest_readiness.first_blocking_stage(),
            Some(RuntimeRequestManifestPlanningReadinessStage::ManifestKvBridge)
        );
        assert_eq!(
            manifest_readiness.request_planning_blocker_component_count,
            0
        );
        assert!(manifest_readiness.manifest_kv_bridge_blocker_component_count > 0);
        assert!(manifest_readiness.has_manifest_request_planning_blockers());
        assert!(manifest_readiness.manifest_request_planning_accounting_is_consistent());
        assert!(!manifest_readiness.manifest_request_planning_is_clean());
        assert!(!manifest_readiness.can_commit_manifest_request_planning());
    }

    #[test]
    fn acceptance_context_manifest_boundary_blocks_response_export_above_manifest_plan() {
        let metadata = RuntimeMetadata::new("model", "tok", 128, 4)
            .with_kv_exchange(true, true)
            .with_kv_limits(2, 4);
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(16)
            .with_max_tokens(8)
            .with_runtime(metadata.clone())
            .with_experiments(ExperimentSwitches::default().with_fht_dke(true));
        let architecture = TransformerRuntimeArchitecture::new(1, 16, 2, 1, 64);
        let transformer_plan = TransformerPlanDigest::new(
            Some("response-export-above-manifest-plan"),
            vec![TransformerLayerBudget::new(
                0,
                TransformerAttentionKind::Global,
                0.5,
                64,
            )],
        );
        let hardware = HardwareAllocator::new().plan(
            HardwareLoadSnapshot::new(DeviceClass::DiscreteGpu, 0.1, 0.1, 0.1, 0.1),
            TaskProfile::Coding,
            4096,
            HierarchyWeights::for_profile(TaskProfile::Coding),
        );
        let execution = hardware
            .adapter_execution_context()
            .clamp_for_runtime(&request.runtime);
        let planning = RuntimePlanningDigest::from_request(
            &request,
            RouteBudget {
                threshold: 0.50,
                attention_tokens: 8,
                fast_tokens: 2,
                attention_fraction: 0.80,
            },
            &execution,
            &[],
            &DeterministicFhtDkeBudgeter::new(0.10, 0.60, 4096),
        );
        let planned_kv = planning.planned_kv_exchange();
        let manifest = RuntimeManifestDigest::from_metadata(metadata)
            .with_architecture(architecture)
            .with_kv_policy(
                RuntimeKvPolicy::import_export()
                    .with_limits(planned_kv.import_blocks, planned_kv.export_blocks),
            );
        let context = RuntimeAcceptanceContext::from_request_parts(
            &request,
            architecture,
            RouteBudget::default(),
            HierarchyWeights::for_profile(TaskProfile::Coding),
            &transformer_plan,
            hardware,
            vec![runtime_block(1)],
        )
        .with_planning_digest(planning);
        let runtime_planning = clean_runtime_planning_readiness();
        let mut outcome = InferenceOutcome::empty().with_diagnostics(
            context
                .inference_diagnostics_seed()
                .with_runtime(context.runtime_diagnostics_seed().with_kv_exchange(1, 2)),
        );
        outcome.answer = "ok".to_owned();
        outcome.tokens.push(GeneratedToken::new("o"));
        outcome.tokens.push(GeneratedToken::new("k"));
        outcome.imported_kv.push(runtime_block(1));
        outcome.exported_kv.push(runtime_block(2));
        outcome.exported_kv.push(runtime_block(3));

        let request_manifest = context
            .manifest_request_planning_readiness_summary(runtime_planning, &manifest)
            .expect("planning digest is attached");
        let boundary_kv = context
            .manifest_boundary_kv_summary(&outcome, runtime_planning, &manifest)
            .expect("planning digest is attached");

        assert_eq!(planned_kv.import_blocks, 1);
        assert_eq!(planned_kv.export_blocks, 1);
        assert!(request_manifest.can_commit_manifest_request_planning());
        assert!(boundary_kv.request_manifest_planning_ready());
        assert!(!boundary_kv.response_manifest_kv_ready());
        assert_eq!(
            boundary_kv.first_unready_stage(),
            Some(RuntimeManifestBoundaryKvStage::ResponseManifestKv)
        );
        assert_eq!(
            boundary_kv.first_blocking_stage(),
            Some(RuntimeManifestBoundaryKvStage::ResponseManifestKv)
        );
        assert!(boundary_kv.response_manifest_kv.manifest_bridge_ready());
        assert!(!boundary_kv.response_manifest_kv.response_planned_kv_ready());
        assert!(
            !boundary_kv
                .response_manifest_kv
                .manifest_export_plan_covers_response()
        );
        assert_eq!(
            boundary_kv.request_manifest_planning_blocker_component_count,
            0
        );
        assert!(boundary_kv.response_manifest_kv_blocker_component_count > 0);
        assert!(boundary_kv.has_manifest_boundary_kv_blockers());
        assert!(boundary_kv.manifest_boundary_kv_accounting_is_consistent());
        assert!(!boundary_kv.manifest_boundary_kv_is_clean());
        assert!(!boundary_kv.can_commit_manifest_boundary_kv());
    }

    #[test]
    fn acceptance_context_blocks_side_effect_commit_when_export_blocks_are_missing() {
        let metadata = RuntimeMetadata::new("model", "tok", 128, 4)
            .with_kv_exchange(true, true)
            .with_kv_limits(1, 1);
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(16)
            .with_max_tokens(8)
            .with_runtime(metadata.clone())
            .with_experiments(ExperimentSwitches::default().with_fht_dke(true));
        let architecture = TransformerRuntimeArchitecture::new(1, 16, 2, 1, 64);
        let transformer_plan = TransformerPlanDigest::new(
            Some("missing-export-block-side-effect"),
            vec![TransformerLayerBudget::new(
                0,
                TransformerAttentionKind::Global,
                0.5,
                64,
            )],
        );
        let hardware = HardwareAllocator::new().plan(
            HardwareLoadSnapshot::new(DeviceClass::DiscreteGpu, 0.1, 0.1, 0.1, 0.1),
            TaskProfile::Coding,
            4096,
            HierarchyWeights::for_profile(TaskProfile::Coding),
        );
        let execution = hardware
            .adapter_execution_context()
            .clamp_for_runtime(&request.runtime);
        let planning = RuntimePlanningDigest::from_request(
            &request,
            RouteBudget {
                threshold: 0.50,
                attention_tokens: 8,
                fast_tokens: 2,
                attention_fraction: 0.80,
            },
            &execution,
            &[],
            &DeterministicFhtDkeBudgeter::new(0.10, 0.60, 4096),
        );
        let planned_kv = planning.planned_kv_exchange();
        let manifest = RuntimeManifestDigest::from_metadata(metadata)
            .with_architecture(architecture)
            .with_kv_policy(
                RuntimeKvPolicy::import_export()
                    .with_limits(planned_kv.import_blocks, planned_kv.export_blocks),
            );
        let context = RuntimeAcceptanceContext::from_request_parts(
            &request,
            architecture,
            RouteBudget::default(),
            HierarchyWeights::for_profile(TaskProfile::Coding),
            &transformer_plan,
            hardware,
            vec![runtime_block(1)],
        )
        .with_planning_digest(planning);
        let runtime_planning = clean_runtime_planning_readiness();
        let mut outcome = InferenceOutcome::empty().with_diagnostics(
            context
                .inference_diagnostics_seed()
                .with_runtime(context.runtime_diagnostics_seed().with_kv_exchange(1, 1)),
        );
        outcome.answer = "ok".to_owned();
        outcome.tokens.push(GeneratedToken::new("o"));
        outcome.tokens.push(GeneratedToken::new("k"));
        outcome.imported_kv.push(runtime_block(1));
        outcome.exported_kv.push(runtime_block(2));

        let import_plan = RuntimeKvImportPlan::new(
            &context.request().runtime,
            architecture,
            planned_kv.import_blocks,
        );
        let import_candidates = vec![RuntimeKvCandidate::new(1, vec![0.1; 16], 1.0)];
        let import_blocks = import_plan.build_blocks(&import_candidates);
        let import_summary = import_plan.import_summary(&import_candidates);
        let import_readiness = RuntimeKvImportReadinessSummary::new(
            import_summary,
            RuntimeKvImportBlockSummary::from_blocks(import_summary.planned_blocks, &import_blocks),
        );
        let manifest_boundary_commit = context
            .manifest_boundary_commit_readiness_summary(&outcome, runtime_planning, &manifest)
            .expect("planning digest is attached");
        let export_plan = RuntimeKvExportPlan::new(
            &context.request().runtime,
            architecture,
            planned_kv.export_blocks,
        );
        let forward_vector = [0.1; 16];
        let forward_summaries = transformer_plan
            .layers
            .iter()
            .map(|layer| TransformerForwardSummary::from_layer_budget(layer, 0.5))
            .collect::<Vec<_>>();
        let planned_export_blocks = export_plan.build_blocks(&forward_vector, &forward_summaries);
        let export_readiness = export_plan.readiness_summary_for_blocks(
            planning,
            &forward_vector,
            &forward_summaries,
            &[],
        );
        let side_effects = RuntimeKvSideEffectReadinessSummary::new(
            import_readiness,
            manifest_boundary_commit,
            export_readiness,
        );

        assert_eq!(planned_kv.import_blocks, 1);
        assert_eq!(planned_kv.export_blocks, 1);
        assert_eq!(planned_export_blocks.len(), 1);
        assert!(import_readiness.can_commit_runtime_kv_import_readiness());
        assert!(manifest_boundary_commit.can_commit_runtime_manifest_boundary());
        assert!(manifest_boundary_commit.readiness_commit_is_clean());
        assert!(!export_readiness.can_commit_runtime_kv_export_readiness());
        assert!(side_effects.import_ready);
        assert!(side_effects.manifest_boundary_commit_ready);
        assert!(!side_effects.export_ready);
        assert_eq!(
            side_effects.export_commit_action,
            RuntimeKvExportReadinessCommitAction::ReturnRuntimeFailure
        );
        assert_eq!(
            side_effects.first_unready_stage(),
            Some(RuntimeKvSideEffectStage::RuntimeKvExport)
        );
        assert_eq!(
            side_effects.first_blocking_stage(),
            Some(RuntimeKvSideEffectStage::RuntimeKvExport)
        );
        assert_eq!(
            side_effects.first_problem_kind(),
            Some(RuntimeKvSideEffectProblemKind::RuntimeKvExport)
        );
        assert_eq!(
            side_effects
                .problem_kind_component_count(RuntimeKvSideEffectProblemKind::RuntimeKvExport),
            side_effects.export_blocker_component_count
        );
        assert!(side_effects.runtime_kv_side_effect_accounting_is_consistent());
        assert!(!side_effects.runtime_kv_side_effect_commit_is_clean());
        assert!(!side_effects.can_commit_runtime_kv_side_effects());

        let export_failure = side_effects
            .primary_failure_report()
            .expect("missing exported KV block is reported");
        let export_summary = side_effects
            .primary_failure_summary()
            .expect("missing exported KV block summary is reported");
        assert_eq!(export_failure.kind, RuntimeFailureKind::KvExport);
        assert_eq!(export_summary.kind, RuntimeFailureKind::KvExport);
        let failure_batch = side_effects.failure_batch_summary();
        assert_eq!(failure_batch.total_count, 1);
        assert_eq!(failure_batch.kv_export_count, 1);
        assert_eq!(failure_batch.contract_violation_count, 0);
        assert!(failure_batch.failure_batch_shape_is_clean());

        let commit = side_effects.commit_summary();
        assert_eq!(
            commit.action,
            RuntimeKvSideEffectCommitAction::ReturnRuntimeFailure
        );
        assert_eq!(
            commit.export_commit_action,
            RuntimeKvExportReadinessCommitAction::ReturnRuntimeFailure
        );
        assert_eq!(
            commit.first_unready_stage,
            Some(RuntimeKvSideEffectStage::RuntimeKvExport)
        );
        assert_eq!(
            commit.first_problem_kind,
            Some(RuntimeKvSideEffectProblemKind::RuntimeKvExport)
        );
        assert_eq!(commit.primary_failure_summary, Some(export_summary));
        assert_eq!(
            commit.failure_report_for(RuntimeKvSideEffectProblemKind::RuntimeKvExport),
            Some(export_failure.clone())
        );
        assert_eq!(commit.failure_batch.kv_export_count, 1);
        assert!(commit.should_return_runtime_failure());
        assert!(!commit.can_commit_runtime_kv_side_effects());
        assert!(commit.commit_decision_accounting_is_consistent());
        let return_report = commit
            .runtime_failure_return_report()
            .expect("KV export side-effect failure return report is materialized");
        assert_eq!(
            return_report.source,
            RuntimeFailureReturnSource::KvSideEffectCommit
        );
        assert_eq!(return_report.primary_failure, export_failure);
        assert_eq!(
            return_report.primary_failure_summary.kind,
            RuntimeFailureKind::KvExport
        );
        assert!(return_report.can_use_runtime_failure_return_report());
    }

    #[test]
    fn acceptance_context_keeps_request_failures_available_before_runtime_execution() {
        let metadata = RuntimeMetadata::new("model", "tok", 128, 4)
            .with_kv_exchange(true, false)
            .with_kv_limits(1, 0);
        let request = InferenceRequest::new("prompt", TaskProfile::General)
            .with_prompt_tokens(8)
            .with_max_tokens(8)
            .with_runtime(metadata);
        let architecture = TransformerRuntimeArchitecture::new(1, 8, 1, 1, 64);
        let transformer_plan = TransformerPlanDigest::new(
            Some("bad-acceptance-context"),
            vec![TransformerLayerBudget::new(
                0,
                TransformerAttentionKind::Global,
                0.5,
                64,
            )],
        );
        let hardware = HardwareAllocator::new().plan(
            HardwareLoadSnapshot::new(DeviceClass::CpuOnly, 0.1, 0.1, 0.1, 0.1),
            TaskProfile::General,
            512,
            HierarchyWeights::default(),
        );
        let request_envelope = RuntimeRequestEnvelope::from_parts(
            &request,
            architecture,
            RouteBudget::default(),
            HierarchyWeights::default(),
            &transformer_plan,
            &crate::adapter::AdapterExecutionContext::new(Vec::new()),
            1,
        );
        let context = RuntimeAcceptanceContext::new(
            request_envelope,
            hardware,
            vec![KvBlock::new(
                1,
                KvNamespace::Gist,
                0,
                0,
                0..1,
                vec![0.1],
                vec![0.2],
            )],
        );
        let report = context.request_acceptance_report();
        let failures = report.failure_reports();
        let mut outcome = InferenceOutcome::empty();
        outcome.answer = "ok".to_owned();
        outcome.tokens.push(GeneratedToken::new("ok"));
        let boundary = context.boundary_acceptance_summary(&outcome);

        assert!(!report.is_accepted());
        assert_eq!(failures.len(), 2);
        assert!(failures[0].message.contains("request acceptance failed"));
        assert!(failures[1].message.contains("imported KV rejected"));
        assert!(!boundary.accepted);
        assert!(boundary.has_request_failures());
        assert!(boundary.has_response_failures());
        assert!(boundary.has_kv_failures());
        assert!(boundary.has_request_parity_failures());
        assert!(boundary.has_failures());
        assert_eq!(boundary.request_acceptance_failure_component_count(), 1);
        assert_eq!(boundary.response_acceptance_failure_component_count(), 1);
        assert_eq!(boundary.kv_failure_component_count(), 1);
        assert_eq!(boundary.request_parity_failure_component_count(), 1);
        assert_eq!(boundary.boundary_failure_component_count(), 4);
        assert!(boundary.has_failure_reports());
        assert_eq!(boundary.boundary_acceptance_problem_component_count(), 5);
        assert!(boundary.total_violation_matches_parts());
        assert!(boundary.failure_report_matches_parts());
        assert!(!boundary.is_clean_acceptance());
        assert!(boundary.total_violation_count >= boundary.request.total_violation_count());
        assert_eq!(
            boundary.total_failure_report_count,
            boundary.request.failure_report_count + boundary.response.failure_report_count
        );
    }

    #[test]
    fn boundary_gate_summary_counts_request_and_adapter_commit_blockers() {
        let gate = RuntimeBoundaryGateSummary {
            request_accepted: false,
            response_accepted: true,
            envelope_consistent: true,
            adapter_consistent: false,
            kv_consistent: true,
            request_backend_wire_problem_count: 4,
            request_planning_pre_request_problem_count: 1,
            request_planning_pressure_signal_count: 2,
            request_planning_dense_compute_avoided_tokens: 8,
            response_wire_problem_count: 2,
            planning_pre_request_problem_count: 1,
            planning_pressure_signal_count: 3,
            response_planning_dense_compute_avoided_tokens: 8,
            kv_boundary_signal_count: 7,
            response_uncertainty_coverage_signal_count: 0,
            response_uncertainty_metric_problem_count: 0,
            response_uncertainty_accounting_consistent: true,
            total_violation_count: 1,
            total_failure_report_count: 1,
        };

        assert!(gate.request_acceptance_failed());
        assert!(!gate.response_acceptance_failed());
        assert!(gate.has_acceptance_failures());
        assert!(!gate.envelope_drifted());
        assert!(gate.adapter_drifted());
        assert!(!gate.kv_drifted());
        assert!(gate.has_request_backend_wire_problem_components());
        assert!(gate.has_request_planning_pre_request_gate_problems());
        assert!(gate.has_request_planning_pressure_signals());
        assert!(gate.has_request_planning_dense_compute_savings());
        assert!(gate.has_response_planning_dense_compute_savings());
        assert!(gate.has_planning_dense_compute_savings());
        assert_eq!(gate.planning_dense_compute_avoided_tokens(), 8);
        assert_eq!(gate.request_backend_wire_problem_component_count(), 4);
        assert_eq!(
            gate.direct_request_backend_wire_problem_component_count(),
            3
        );
        assert_eq!(
            gate.request_planning_pre_request_gate_problem_component_count(),
            1
        );
        assert_eq!(gate.request_planning_pressure_signal_component_count(), 1);
        assert!(gate.request_backend_wire_accounting_is_consistent());
        assert!(gate.has_response_wire_problem_components());
        assert!(gate.has_planning_pre_request_gate_problems());
        assert!(gate.has_planning_pressure_signals());
        assert_eq!(gate.response_wire_problem_component_count(), 2);
        assert_eq!(gate.direct_response_wire_problem_component_count(), 1);
        assert_eq!(gate.planning_pre_request_gate_problem_component_count(), 1);
        assert_eq!(gate.planning_pressure_signal_component_count(), 1);
        assert_eq!(gate.kv_boundary_signal_component_count(), 7);
        assert!(gate.has_kv_boundary_signals());
        assert_eq!(
            gate.response_uncertainty_coverage_signal_component_count(),
            0
        );
        assert!(!gate.has_response_uncertainty_coverage_signals());
        assert_eq!(gate.commit_gate_signal_component_count(), 12);
        assert!(gate.commit_gate_has_signal_components());
        assert_eq!(
            gate.response_uncertainty_metric_problem_component_count(),
            0
        );
        assert!(!gate.has_response_uncertainty_metric_problem_components());
        assert!(gate.response_uncertainty_accounting_is_consistent());
        assert!(gate.response_wire_accounting_is_consistent());
        assert_eq!(gate.total_wire_problem_component_count(), 6);
        assert!(gate.has_wire_problem_components());
        assert!(gate.wire_accounting_is_consistent());
        assert!(gate.has_boundary_drift());
        assert!(gate.has_failure_reports());
        assert!(gate.has_total_violations());
        assert_eq!(gate.request_acceptance_blocker_component_count(), 1);
        assert_eq!(gate.response_acceptance_blocker_component_count(), 0);
        assert_eq!(gate.acceptance_failure_component_count(), 1);
        assert_eq!(gate.envelope_blocker_component_count(), 0);
        assert_eq!(gate.adapter_blocker_component_count(), 1);
        assert_eq!(gate.kv_blocker_component_count(), 0);
        assert_eq!(gate.boundary_drift_component_count(), 1);
        assert_eq!(gate.mapped_failure_report_component_count(), 1);
        assert_eq!(gate.commit_blocker_component_count(), 3);
        assert!(gate.commit_gate_has_problem_components());
        assert!(gate.commit_gate_accounting_is_consistent());
        assert!(!gate.can_commit_response());
        assert!(!gate.is_clean_commit_gate());
        assert_eq!(gate.runtime_boundary_commit_signal_component_count(), 12);
        assert!(gate.has_runtime_boundary_commit_signals());
        assert_eq!(gate.runtime_boundary_commit_blocker_component_count(), 9);
        assert!(gate.has_runtime_boundary_commit_blockers());
        assert!(gate.runtime_boundary_commit_accounting_is_consistent());
        assert!(!gate.runtime_boundary_commit_is_clean());
        assert!(!gate.can_commit_runtime_boundary());
    }

    #[test]
    fn boundary_gate_summary_counts_uncertainty_accounting_drift() {
        let gate = RuntimeBoundaryGateSummary {
            request_accepted: true,
            response_accepted: true,
            envelope_consistent: true,
            adapter_consistent: true,
            kv_consistent: true,
            request_backend_wire_problem_count: 0,
            request_planning_pre_request_problem_count: 0,
            request_planning_pressure_signal_count: 0,
            request_planning_dense_compute_avoided_tokens: 0,
            response_wire_problem_count: 0,
            planning_pre_request_problem_count: 0,
            planning_pressure_signal_count: 0,
            response_planning_dense_compute_avoided_tokens: 0,
            kv_boundary_signal_count: 0,
            response_uncertainty_coverage_signal_count: 2,
            response_uncertainty_metric_problem_count: 4,
            response_uncertainty_accounting_consistent: true,
            total_violation_count: 0,
            total_failure_report_count: 0,
        };

        assert_eq!(
            gate.response_uncertainty_coverage_signal_component_count(),
            2
        );
        assert!(gate.has_response_uncertainty_coverage_signals());
        assert_eq!(gate.commit_gate_signal_component_count(), 2);
        assert!(gate.commit_gate_has_signal_components());
        assert_eq!(
            gate.response_uncertainty_metric_problem_component_count(),
            4
        );
        assert!(gate.has_response_uncertainty_metric_problem_components());
        assert!(!gate.response_uncertainty_accounting_is_consistent());
        assert!(gate.envelope_drifted());
        assert!(gate.has_boundary_drift());
        assert_eq!(gate.envelope_blocker_component_count(), 1);
        assert_eq!(gate.boundary_drift_component_count(), 1);
        assert_eq!(gate.commit_blocker_component_count(), 1);
        assert!(gate.commit_gate_has_problem_components());
        assert!(gate.commit_gate_accounting_is_consistent());
        assert!(!gate.can_commit_response());
        assert!(!gate.boundary_gate_shape_is_clean());
        assert_eq!(gate.runtime_boundary_commit_signal_component_count(), 2);
        assert!(gate.has_runtime_boundary_commit_signals());
        assert_eq!(gate.runtime_boundary_commit_blocker_component_count(), 1);
        assert!(gate.has_runtime_boundary_commit_blockers());
        assert!(!gate.runtime_boundary_commit_accounting_is_consistent());
        assert!(!gate.runtime_boundary_commit_is_clean());
        assert!(!gate.can_commit_runtime_boundary());
        assert!(!gate.can_commit_runtime_response());
    }

    #[test]
    fn boundary_gate_summary_blocks_public_wire_accounting_drift() {
        let gate = RuntimeBoundaryGateSummary {
            request_accepted: true,
            response_accepted: true,
            envelope_consistent: true,
            adapter_consistent: true,
            kv_consistent: true,
            request_backend_wire_problem_count: 0,
            request_planning_pre_request_problem_count: 1,
            request_planning_pressure_signal_count: 0,
            request_planning_dense_compute_avoided_tokens: 0,
            response_wire_problem_count: 0,
            planning_pre_request_problem_count: 0,
            planning_pressure_signal_count: 0,
            response_planning_dense_compute_avoided_tokens: 0,
            kv_boundary_signal_count: 0,
            response_uncertainty_coverage_signal_count: 0,
            response_uncertainty_metric_problem_count: 0,
            response_uncertainty_accounting_consistent: true,
            total_violation_count: 0,
            total_failure_report_count: 0,
        };

        assert!(gate.can_commit_response());
        assert!(gate.is_clean_commit_gate());
        assert!(gate.commit_gate_accounting_is_consistent());
        assert!(gate.response_uncertainty_accounting_is_consistent());
        assert!(!gate.request_backend_wire_accounting_is_consistent());
        assert!(!gate.wire_accounting_is_consistent());
        assert!(!gate.boundary_gate_shape_is_clean());
        assert_eq!(gate.runtime_boundary_commit_signal_component_count(), 0);
        assert!(!gate.has_runtime_boundary_commit_signals());
        assert_eq!(gate.runtime_boundary_commit_blocker_component_count(), 0);
        assert!(!gate.has_runtime_boundary_commit_blockers());
        assert!(!gate.runtime_boundary_commit_accounting_is_consistent());
        assert!(!gate.runtime_boundary_commit_is_clean());
        assert!(!gate.can_commit_runtime_boundary());
        assert!(!gate.can_commit_runtime_response());
    }

    #[test]
    fn boundary_gate_summary_blocks_public_wire_problem_drift() {
        let gate = RuntimeBoundaryGateSummary {
            request_accepted: true,
            response_accepted: true,
            envelope_consistent: true,
            adapter_consistent: true,
            kv_consistent: true,
            request_backend_wire_problem_count: 1,
            request_planning_pre_request_problem_count: 0,
            request_planning_pressure_signal_count: 0,
            request_planning_dense_compute_avoided_tokens: 0,
            response_wire_problem_count: 0,
            planning_pre_request_problem_count: 0,
            planning_pressure_signal_count: 0,
            response_planning_dense_compute_avoided_tokens: 0,
            kv_boundary_signal_count: 0,
            response_uncertainty_coverage_signal_count: 0,
            response_uncertainty_metric_problem_count: 0,
            response_uncertainty_accounting_consistent: true,
            total_violation_count: 0,
            total_failure_report_count: 0,
        };

        assert!(gate.can_commit_response());
        assert!(gate.is_clean_commit_gate());
        assert!(gate.request_backend_wire_accounting_is_consistent());
        assert!(gate.wire_accounting_is_consistent());
        assert!(gate.has_wire_problem_components());
        assert!(!gate.boundary_gate_shape_is_clean());
        assert_eq!(gate.runtime_boundary_commit_signal_component_count(), 0);
        assert!(!gate.has_runtime_boundary_commit_signals());
        assert_eq!(gate.runtime_boundary_commit_blocker_component_count(), 1);
        assert!(gate.has_runtime_boundary_commit_blockers());
        assert!(gate.runtime_boundary_commit_accounting_is_consistent());
        assert!(!gate.runtime_boundary_commit_is_clean());
        assert!(!gate.can_commit_runtime_boundary());
        assert!(!gate.can_commit_runtime_response());
    }

    #[test]
    fn boundary_acceptance_summary_counts_public_shape_drift() {
        let request = RuntimeRequestAcceptanceSummary {
            accepted: false,
            request_violation_count: 1,
            imported_kv_violation_count: 0,
            accepted_imported_kv_blocks: 0,
            failure_report_count: 1,
        };
        let response = RuntimeResponseAcceptanceSummary {
            accepted: true,
            response_violation_count: 0,
            request_violation_count: 0,
            exported_kv_violation_count: 0,
            accepted_exported_kv_blocks: 0,
            failure_report_count: 0,
        };
        let clean = RuntimeBoundaryAcceptanceSummary {
            accepted: true,
            request: RuntimeRequestAcceptanceSummary {
                accepted: true,
                request_violation_count: 0,
                imported_kv_violation_count: 0,
                accepted_imported_kv_blocks: 1,
                failure_report_count: 0,
            },
            response,
            total_violation_count: 0,
            total_failure_report_count: 0,
        };
        let drift = RuntimeBoundaryAcceptanceSummary {
            accepted: true,
            request,
            response,
            total_violation_count: 1,
            total_failure_report_count: 0,
        };

        assert_eq!(clean.boundary_acceptance_problem_component_count(), 0);
        assert!(!clean.has_boundary_acceptance_problem_components());
        assert!(clean.boundary_acceptance_accounting_is_consistent());
        assert!(clean.is_clean_acceptance());
        assert_eq!(
            clean.runtime_boundary_acceptance_commit_signal_component_count(),
            clean
                .request
                .runtime_request_acceptance_commit_signal_component_count()
                .saturating_add(
                    clean
                        .response
                        .runtime_response_acceptance_commit_signal_component_count(),
                )
                .saturating_add(1)
        );
        assert_eq!(
            clean.runtime_boundary_acceptance_commit_blocker_component_count(),
            0
        );
        assert!(clean.runtime_boundary_acceptance_commit_accounting_is_consistent());
        assert!(clean.runtime_boundary_acceptance_commit_is_clean());
        assert!(clean.can_commit_runtime_boundary_acceptance());

        assert!(drift.has_request_failures());
        assert!(drift.has_failures());
        assert_eq!(drift.request_acceptance_failure_component_count(), 1);
        assert_eq!(drift.boundary_failure_component_count(), 1);
        assert_eq!(drift.boundary_acceptance_problem_component_count(), 1);
        assert!(drift.has_boundary_acceptance_problem_components());
        assert!(!drift.failure_report_matches_parts());
        assert!(!drift.boundary_acceptance_accounting_is_consistent());
        assert!(!drift.is_clean_acceptance());
        assert_eq!(
            drift.runtime_boundary_acceptance_commit_blocker_component_count(),
            drift
                .request
                .runtime_request_acceptance_commit_blocker_component_count()
                .saturating_add(
                    drift
                        .response
                        .runtime_response_acceptance_commit_blocker_component_count(),
                )
                .saturating_add(drift.boundary_acceptance_problem_component_count())
        );
        assert!(drift.has_runtime_boundary_acceptance_commit_blockers());
        assert!(!drift.runtime_boundary_acceptance_commit_accounting_is_consistent());
        assert!(!drift.runtime_boundary_acceptance_commit_is_clean());
        assert!(!drift.can_commit_runtime_boundary_acceptance());
    }

    fn clean_route_budget() -> RouteBudget {
        RouteBudget {
            threshold: 0.50,
            attention_tokens: 3,
            fast_tokens: 1,
            attention_fraction: 0.75,
        }
    }

    fn clean_runtime_planning_readiness() -> RuntimePlanningReadinessSummary {
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
        RuntimePlanningReadinessSummary::new(
            clean_fht_dke_planning_readiness(route_budget, fht_dke_budget),
            clean_runtime_planning_summary(fht_dke_budget),
        )
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

    fn runtime_block(id: u64) -> KvBlock {
        KvBlock::new(id, KvNamespace::Runtime, 0, 0, 0..1, vec![0.1], vec![0.2])
    }
}
