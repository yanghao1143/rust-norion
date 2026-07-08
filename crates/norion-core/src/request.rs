use crate::adapter::{AdapterExecutionContext, RuntimeAdapter};
use crate::diagnostics::DiagnosticsPressureBand;
use crate::engine::{
    InferenceError, InferenceRequest, RuntimeFailureBatchSummary, RuntimeFailureReport,
    RuntimeFailureSummary,
};
use crate::kv::{KvBlock, RuntimeKvBlockContract, RuntimeKvValidationReport};
use crate::manifest::{RuntimeManifestDigest, TransformerRuntimeArchitecture};
use crate::planning::{
    RuntimePlanningDigest, RuntimePlanningManifestKvBridgeSummary, RuntimePlanningReadinessSummary,
};
use crate::profile::{HierarchyWeights, TaskProfile};
use crate::recursive::RecursiveScheduleSummary;
use crate::router::RouteBudget;
use crate::runtime::{RuntimeGenerationBudget, RuntimeMetadata};
use crate::transformer::TransformerPlanDigest;

pub const RUNTIME_REQUEST_SCHEMA: &str = "rust-norion-runtime-request-v1";

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeRequestEnvelope {
    pub schema: &'static str,
    pub profile: TaskProfile,
    pub prompt_chars: usize,
    pub prompt_tokens: usize,
    pub max_tokens: usize,
    pub generation_budget: RuntimeGenerationBudget,
    pub runtime: RuntimeMetadata,
    pub architecture: TransformerRuntimeArchitecture,
    pub route_budget: RouteBudget,
    pub hierarchy: HierarchyWeights,
    pub transformer_layer_count: usize,
    pub imported_kv_blocks: usize,
    pub adapter_count: usize,
    pub selected_adapter: Option<RuntimeAdapter>,
    pub hardware_pressure: f32,
    pub kv_prefetch_blocks: usize,
    pub planning: Option<RuntimePlanningDigest>,
    pub recursive: Option<RecursiveScheduleSummary>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeRequestEnvelopeSummary {
    pub schema: &'static str,
    pub profile: TaskProfile,
    pub prompt_chars: usize,
    pub prompt_tokens: usize,
    pub requested_max_tokens: usize,
    pub max_tokens: usize,
    pub max_generated_tokens: usize,
    pub planned_context_tokens: usize,
    pub truncated_by_context: bool,
    pub can_generate: bool,
    pub model_context_window: usize,
    pub runtime_embedding_dimensions: usize,
    pub runtime_metadata_adapter_ready: bool,
    pub runtime_import_enabled: bool,
    pub runtime_export_enabled: bool,
    pub max_kv_import_blocks: usize,
    pub max_kv_export_blocks: usize,
    pub architecture_layer_count: usize,
    pub architecture_hidden_size: usize,
    pub transformer_layer_count: usize,
    pub imported_kv_blocks: usize,
    pub adapter_count: usize,
    pub selected_adapter: Option<RuntimeAdapter>,
    pub hardware_pressure: f32,
    pub hardware_pressure_band: DiagnosticsPressureBand,
    pub kv_prefetch_blocks: usize,
    pub has_planning_digest: bool,
    pub planning_backend_max_tokens: Option<usize>,
    pub planning_import_blocks: Option<usize>,
    pub planning_export_blocks: Option<usize>,
    pub has_recursive_schedule: bool,
    pub recursive_requires_recursion: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeRequestPlanningParitySummary {
    pub has_planning_digest: bool,
    pub request_max_tokens: usize,
    pub planned_backend_max_tokens: Option<usize>,
    pub max_tokens_match_planning: Option<bool>,
    pub generation_budget_matches_planning: Option<bool>,
    pub request_selected_adapter: Option<RuntimeAdapter>,
    pub planned_adapter: Option<RuntimeAdapter>,
    pub selected_adapter_matches_planning: Option<bool>,
    pub imported_kv_blocks: usize,
    pub kv_prefetch_blocks: usize,
    pub planned_import_blocks: Option<usize>,
    pub planned_export_blocks: Option<usize>,
    pub imported_kv_matches_planning: Option<bool>,
    pub kv_prefetch_matches_planning: Option<bool>,
    pub planning_violation_count: usize,
    pub planning_pre_request_problem_count: usize,
    pub planning_pressure_signal_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeRequestGateSummary {
    pub request_accepted: bool,
    pub envelope_consistent: bool,
    pub planning_attached: bool,
    pub planning_consistent: bool,
    pub accepted_imported_kv_blocks: usize,
    pub backend_wire_problem_count: usize,
    pub planning_pre_request_problem_count: usize,
    pub planning_pressure_signal_count: usize,
    pub request_violation_count: usize,
    pub imported_kv_violation_count: usize,
    pub failure_report_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeRequestPlanningReadinessStage {
    RuntimePlanning,
    RequestPlanningParity,
    RequestGate,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeRequestPlanningReadinessSummary {
    pub runtime_planning: RuntimePlanningReadinessSummary,
    pub request_planning: RuntimeRequestPlanningParitySummary,
    pub request_gate: RuntimeRequestGateSummary,
    pub runtime_planning_signal_component_count: usize,
    pub request_planning_signal_component_count: usize,
    pub request_gate_signal_component_count: usize,
    pub runtime_planning_blocker_component_count: usize,
    pub request_planning_blocker_component_count: usize,
    pub request_gate_blocker_component_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeRequestManifestPlanningReadinessStage {
    ManifestKvBridge,
    RequestPlanning,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeRequestManifestPlanningReadinessSummary {
    pub manifest_kv_bridge: RuntimePlanningManifestKvBridgeSummary,
    pub request_planning: RuntimeRequestPlanningReadinessSummary,
    pub manifest_kv_bridge_signal_component_count: usize,
    pub request_planning_signal_component_count: usize,
    pub manifest_kv_bridge_blocker_component_count: usize,
    pub request_planning_blocker_component_count: usize,
}

impl RuntimeRequestPlanningReadinessSummary {
    pub fn new(
        runtime_planning: RuntimePlanningReadinessSummary,
        request_planning: RuntimeRequestPlanningParitySummary,
        request_gate: RuntimeRequestGateSummary,
    ) -> Self {
        Self {
            runtime_planning,
            request_planning,
            request_gate,
            runtime_planning_signal_component_count: runtime_planning
                .runtime_planning_readiness_signal_component_count(),
            request_planning_signal_component_count: request_planning
                .planning_pressure_signal_component_count(),
            request_gate_signal_component_count: request_gate
                .runtime_request_commit_signal_component_count(),
            runtime_planning_blocker_component_count: runtime_planning
                .runtime_planning_readiness_blocker_component_count(),
            request_planning_blocker_component_count: request_planning
                .backend_wire_problem_component_count(),
            request_gate_blocker_component_count: request_gate
                .runtime_request_commit_blocker_component_count(),
        }
    }

    pub fn stage_order() -> [RuntimeRequestPlanningReadinessStage; 3] {
        [
            RuntimeRequestPlanningReadinessStage::RuntimePlanning,
            RuntimeRequestPlanningReadinessStage::RequestPlanningParity,
            RuntimeRequestPlanningReadinessStage::RequestGate,
        ]
    }

    pub fn runtime_planning_ready(self) -> bool {
        self.runtime_planning
            .can_commit_runtime_planning_readiness()
    }

    pub fn runtime_planning_committed_parts_ready(self) -> bool {
        self.runtime_planning
            .can_commit_runtime_planning_with_committed_parts()
    }

    pub fn request_planning_ready(self) -> bool {
        self.request_planning.can_use_backend_wire_request()
    }

    pub fn request_gate_ready(self) -> bool {
        self.request_gate.can_commit_runtime_request()
    }

    pub fn stage_ready(self, stage: RuntimeRequestPlanningReadinessStage) -> bool {
        match stage {
            RuntimeRequestPlanningReadinessStage::RuntimePlanning => self.runtime_planning_ready(),
            RuntimeRequestPlanningReadinessStage::RequestPlanningParity => {
                self.request_planning_ready()
            }
            RuntimeRequestPlanningReadinessStage::RequestGate => self.request_gate_ready(),
        }
    }

    pub fn stage_signal_component_count(
        self,
        stage: RuntimeRequestPlanningReadinessStage,
    ) -> usize {
        match stage {
            RuntimeRequestPlanningReadinessStage::RuntimePlanning => {
                self.runtime_planning_signal_component_count
            }
            RuntimeRequestPlanningReadinessStage::RequestPlanningParity => {
                self.request_planning_signal_component_count
            }
            RuntimeRequestPlanningReadinessStage::RequestGate => {
                self.request_gate_signal_component_count
            }
        }
    }

    pub fn stage_blocker_component_count(
        self,
        stage: RuntimeRequestPlanningReadinessStage,
    ) -> usize {
        match stage {
            RuntimeRequestPlanningReadinessStage::RuntimePlanning => {
                self.runtime_planning_blocker_component_count
            }
            RuntimeRequestPlanningReadinessStage::RequestPlanningParity => {
                self.request_planning_blocker_component_count
            }
            RuntimeRequestPlanningReadinessStage::RequestGate => {
                self.request_gate_blocker_component_count
            }
        }
    }

    pub fn first_unready_stage(self) -> Option<RuntimeRequestPlanningReadinessStage> {
        Self::stage_order()
            .into_iter()
            .find(|stage| !self.stage_ready(*stage))
    }

    pub fn first_blocking_stage(self) -> Option<RuntimeRequestPlanningReadinessStage> {
        Self::stage_order()
            .into_iter()
            .find(|stage| self.stage_blocker_component_count(*stage) > 0)
    }

    pub fn runtime_request_planning_signal_component_count(self) -> usize {
        self.runtime_planning_signal_component_count
            .saturating_add(self.request_planning_signal_component_count)
            .saturating_add(self.request_gate_signal_component_count)
    }

    pub fn has_runtime_request_planning_signals(self) -> bool {
        self.runtime_request_planning_signal_component_count() > 0
    }

    pub fn runtime_request_planning_blocker_component_count(self) -> usize {
        self.runtime_planning_blocker_component_count
            .saturating_add(self.request_planning_blocker_component_count)
            .saturating_add(self.request_gate_blocker_component_count)
    }

    pub fn has_runtime_request_planning_blockers(self) -> bool {
        self.runtime_request_planning_blocker_component_count() > 0
    }

    pub fn runtime_request_planning_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .runtime_planning_signal_component_count
            .saturating_add(self.request_planning_signal_component_count)
            .saturating_add(self.request_gate_signal_component_count);
        let expected_blocker_count = self
            .runtime_planning_blocker_component_count
            .saturating_add(self.request_planning_blocker_component_count)
            .saturating_add(self.request_gate_blocker_component_count);

        self.runtime_request_planning_signal_component_count() == expected_signal_count
            && self.has_runtime_request_planning_signals() == (expected_signal_count > 0)
            && self.runtime_request_planning_blocker_component_count() == expected_blocker_count
            && self.has_runtime_request_planning_blockers() == (expected_blocker_count > 0)
            && self
                .runtime_planning
                .runtime_planning_readiness_accounting_is_consistent()
            && self
                .request_planning
                .backend_wire_accounting_is_consistent()
            && self
                .request_gate
                .runtime_request_commit_accounting_is_consistent()
    }

    pub fn runtime_request_planning_is_clean(self) -> bool {
        !self.has_runtime_request_planning_blockers()
            && self.runtime_request_planning_accounting_is_consistent()
    }

    pub fn can_commit_runtime_request_planning(self) -> bool {
        self.runtime_request_planning_is_clean()
            && self.runtime_planning_ready()
            && self.request_planning_ready()
            && self.request_gate_ready()
    }

    pub fn can_commit_runtime_request_planning_with_committed_parts(self) -> bool {
        self.can_commit_runtime_request_planning() && self.runtime_planning_committed_parts_ready()
    }
}

impl RuntimeRequestManifestPlanningReadinessSummary {
    pub fn new(
        manifest_kv_bridge: RuntimePlanningManifestKvBridgeSummary,
        request_planning: RuntimeRequestPlanningReadinessSummary,
    ) -> Self {
        Self {
            manifest_kv_bridge,
            request_planning,
            manifest_kv_bridge_signal_component_count: manifest_kv_bridge
                .manifest_kv_bridge_signal_component_count(),
            request_planning_signal_component_count: request_planning
                .runtime_request_planning_signal_component_count(),
            manifest_kv_bridge_blocker_component_count: manifest_kv_bridge
                .manifest_kv_bridge_problem_component_count(),
            request_planning_blocker_component_count: request_planning
                .runtime_request_planning_blocker_component_count(),
        }
    }

    pub fn stage_order() -> [RuntimeRequestManifestPlanningReadinessStage; 2] {
        [
            RuntimeRequestManifestPlanningReadinessStage::ManifestKvBridge,
            RuntimeRequestManifestPlanningReadinessStage::RequestPlanning,
        ]
    }

    pub fn manifest_kv_bridge_ready(self) -> bool {
        self.manifest_kv_bridge
            .can_use_runtime_planning_manifest_kv_bridge()
    }

    pub fn request_planning_ready(self) -> bool {
        self.request_planning.can_commit_runtime_request_planning()
    }

    pub fn stage_ready(self, stage: RuntimeRequestManifestPlanningReadinessStage) -> bool {
        match stage {
            RuntimeRequestManifestPlanningReadinessStage::ManifestKvBridge => {
                self.manifest_kv_bridge_ready()
            }
            RuntimeRequestManifestPlanningReadinessStage::RequestPlanning => {
                self.request_planning_ready()
            }
        }
    }

    pub fn stage_signal_component_count(
        self,
        stage: RuntimeRequestManifestPlanningReadinessStage,
    ) -> usize {
        match stage {
            RuntimeRequestManifestPlanningReadinessStage::ManifestKvBridge => {
                self.manifest_kv_bridge_signal_component_count
            }
            RuntimeRequestManifestPlanningReadinessStage::RequestPlanning => {
                self.request_planning_signal_component_count
            }
        }
    }

    pub fn stage_blocker_component_count(
        self,
        stage: RuntimeRequestManifestPlanningReadinessStage,
    ) -> usize {
        match stage {
            RuntimeRequestManifestPlanningReadinessStage::ManifestKvBridge => {
                self.manifest_kv_bridge_blocker_component_count
            }
            RuntimeRequestManifestPlanningReadinessStage::RequestPlanning => {
                self.request_planning_blocker_component_count
            }
        }
    }

    pub fn first_unready_stage(self) -> Option<RuntimeRequestManifestPlanningReadinessStage> {
        Self::stage_order()
            .into_iter()
            .find(|stage| !self.stage_ready(*stage))
    }

    pub fn first_blocking_stage(self) -> Option<RuntimeRequestManifestPlanningReadinessStage> {
        Self::stage_order()
            .into_iter()
            .find(|stage| self.stage_blocker_component_count(*stage) > 0)
    }

    pub fn manifest_request_planning_signal_component_count(self) -> usize {
        self.manifest_kv_bridge_signal_component_count
            .saturating_add(self.request_planning_signal_component_count)
    }

    pub fn has_manifest_request_planning_signals(self) -> bool {
        self.manifest_request_planning_signal_component_count() > 0
    }

    pub fn manifest_request_planning_blocker_component_count(self) -> usize {
        self.manifest_kv_bridge_blocker_component_count
            .saturating_add(self.request_planning_blocker_component_count)
    }

    pub fn has_manifest_request_planning_blockers(self) -> bool {
        self.manifest_request_planning_blocker_component_count() > 0
    }

    pub fn manifest_request_planning_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .manifest_kv_bridge_signal_component_count
            .saturating_add(self.request_planning_signal_component_count);
        let expected_blocker_count = self
            .manifest_kv_bridge_blocker_component_count
            .saturating_add(self.request_planning_blocker_component_count);

        self.manifest_kv_bridge
            .manifest_kv_bridge_accounting_is_consistent()
            && self
                .request_planning
                .runtime_request_planning_accounting_is_consistent()
            && self.manifest_request_planning_signal_component_count() == expected_signal_count
            && self.has_manifest_request_planning_signals() == (expected_signal_count > 0)
            && self.manifest_request_planning_blocker_component_count() == expected_blocker_count
            && self.has_manifest_request_planning_blockers() == (expected_blocker_count > 0)
    }

    pub fn manifest_request_planning_is_clean(self) -> bool {
        !self.has_manifest_request_planning_blockers()
            && self.manifest_request_planning_accounting_is_consistent()
    }

    pub fn can_commit_manifest_request_planning(self) -> bool {
        self.manifest_request_planning_is_clean()
            && self.manifest_kv_bridge_ready()
            && self.request_planning_ready()
    }
}

impl RuntimeRequestEnvelopeSummary {
    pub fn schema_matches_runtime_request(self) -> bool {
        self.schema == RUNTIME_REQUEST_SCHEMA
    }

    pub fn has_adapter_candidates(self) -> bool {
        self.adapter_count > 0
    }

    pub fn has_selected_adapter(self) -> bool {
        self.selected_adapter.is_some()
    }

    pub fn context_limited_generation(self) -> bool {
        self.truncated_by_context || !self.can_generate
    }

    pub fn runtime_metadata_adapter_blocked(self) -> bool {
        !self.runtime_metadata_adapter_ready
    }

    pub fn transformer_layers_match_architecture(self) -> bool {
        self.transformer_layer_count == 0
            || self.architecture_layer_count == 0
            || self.transformer_layer_count == self.architecture_layer_count
    }

    pub fn has_kv_exchange_capacity(self) -> bool {
        self.runtime_import_enabled || self.runtime_export_enabled
    }

    pub fn has_kv_import_pressure(self) -> bool {
        self.imported_kv_blocks > 0 || self.kv_prefetch_blocks > 0
    }

    pub fn kv_imports_exceed_runtime_limit(self) -> bool {
        self.runtime_import_enabled
            && self.max_kv_import_blocks > 0
            && self.imported_kv_blocks > self.max_kv_import_blocks
    }

    pub fn kv_prefetch_exceeds_runtime_limit(self) -> bool {
        self.runtime_import_enabled
            && self.max_kv_import_blocks > 0
            && self.kv_prefetch_blocks > self.max_kv_import_blocks
    }

    pub fn planning_attached(self) -> bool {
        self.has_planning_digest
    }

    pub fn recursive_attached(self) -> bool {
        self.has_recursive_schedule
    }

    pub fn request_envelope_commit_signal_component_count(self) -> usize {
        usize::from(self.has_adapter_candidates())
            .saturating_add(usize::from(self.has_selected_adapter()))
            .saturating_add(usize::from(self.has_kv_exchange_capacity()))
            .saturating_add(usize::from(self.has_kv_import_pressure()))
            .saturating_add(usize::from(self.hardware_pressure_band.is_constrained()))
            .saturating_add(usize::from(self.planning_attached()))
            .saturating_add(usize::from(self.recursive_attached()))
            .saturating_add(usize::from(self.context_limited_generation()))
    }

    pub fn has_request_envelope_commit_signals(self) -> bool {
        self.request_envelope_commit_signal_component_count() > 0
    }

    pub fn request_envelope_commit_blocker_component_count(self) -> usize {
        usize::from(!self.schema_matches_runtime_request())
            .saturating_add(usize::from(!self.transformer_layers_match_architecture()))
            .saturating_add(usize::from(self.runtime_metadata_adapter_blocked()))
            .saturating_add(usize::from(self.kv_imports_exceed_runtime_limit()))
            .saturating_add(usize::from(self.kv_prefetch_exceeds_runtime_limit()))
    }

    pub fn has_request_envelope_commit_blockers(self) -> bool {
        self.request_envelope_commit_blocker_component_count() > 0
    }

    pub fn request_envelope_commit_accounting_is_consistent(self) -> bool {
        let expected_signal_count = usize::from(self.has_adapter_candidates())
            .saturating_add(usize::from(self.has_selected_adapter()))
            .saturating_add(usize::from(self.has_kv_exchange_capacity()))
            .saturating_add(usize::from(self.has_kv_import_pressure()))
            .saturating_add(usize::from(self.hardware_pressure_band.is_constrained()))
            .saturating_add(usize::from(self.planning_attached()))
            .saturating_add(usize::from(self.recursive_attached()))
            .saturating_add(usize::from(self.context_limited_generation()));
        let expected_blocker_count = usize::from(!self.schema_matches_runtime_request())
            .saturating_add(usize::from(!self.transformer_layers_match_architecture()))
            .saturating_add(usize::from(self.runtime_metadata_adapter_blocked()))
            .saturating_add(usize::from(self.kv_imports_exceed_runtime_limit()))
            .saturating_add(usize::from(self.kv_prefetch_exceeds_runtime_limit()));

        self.request_envelope_commit_signal_component_count() == expected_signal_count
            && self.has_request_envelope_commit_signals() == (expected_signal_count > 0)
            && self.request_envelope_commit_blocker_component_count() == expected_blocker_count
            && self.has_request_envelope_commit_blockers() == (expected_blocker_count > 0)
    }

    pub fn request_envelope_commit_is_clean(self) -> bool {
        self.request_envelope_commit_blocker_component_count() == 0
            && self.request_envelope_commit_accounting_is_consistent()
    }

    pub fn request_envelope_shape_is_clean(self) -> bool {
        self.request_envelope_commit_is_clean()
    }

    pub fn can_commit_runtime_request_envelope(self) -> bool {
        self.request_envelope_commit_is_clean()
            && self.can_generate
            && self.runtime_metadata_adapter_ready
            && self.has_adapter_candidates()
            && self.has_selected_adapter()
    }

    pub fn can_use_runtime_request_envelope(self) -> bool {
        self.can_commit_runtime_request_envelope()
    }
}

impl RuntimeRequestPlanningParitySummary {
    pub fn planning_attached(self) -> bool {
        self.has_planning_digest
    }

    pub fn planning_missing_from_request(self) -> bool {
        !self.has_planning_digest
    }

    pub fn max_tokens_drifted_from_planning(self) -> bool {
        self.max_tokens_match_planning == Some(false)
    }

    pub fn generation_budget_drifted_from_planning(self) -> bool {
        self.generation_budget_matches_planning == Some(false)
    }

    pub fn adapter_drifted_from_planning(self) -> bool {
        self.selected_adapter_matches_planning == Some(false)
    }

    pub fn imported_kv_drifted_from_planning(self) -> bool {
        self.imported_kv_matches_planning == Some(false)
    }

    pub fn kv_prefetch_drifted_from_planning(self) -> bool {
        self.kv_prefetch_matches_planning == Some(false)
    }

    pub fn token_drift_component_count(self) -> usize {
        usize::from(self.max_tokens_drifted_from_planning())
            + usize::from(self.generation_budget_drifted_from_planning())
    }

    pub fn max_token_drift_component_count(self) -> usize {
        usize::from(self.max_tokens_drifted_from_planning())
    }

    pub fn generation_budget_drift_component_count(self) -> usize {
        usize::from(self.generation_budget_drifted_from_planning())
    }

    pub fn adapter_drift_component_count(self) -> usize {
        usize::from(self.adapter_drifted_from_planning())
    }

    pub fn kv_drift_component_count(self) -> usize {
        usize::from(self.imported_kv_drifted_from_planning())
            + usize::from(self.kv_prefetch_drifted_from_planning())
    }

    pub fn imported_kv_drift_component_count(self) -> usize {
        usize::from(self.imported_kv_drifted_from_planning())
    }

    pub fn kv_prefetch_drift_component_count(self) -> usize {
        usize::from(self.kv_prefetch_drifted_from_planning())
    }

    pub fn planning_attachment_drift_component_count(self) -> usize {
        usize::from(self.planning_missing_from_request())
    }

    pub fn planning_contract_drift_component_count(self) -> usize {
        usize::from(self.planning_has_contract_violations())
    }

    pub fn request_planning_drift_component_count(self) -> usize {
        self.planning_attachment_drift_component_count()
            .saturating_add(self.token_drift_component_count())
            .saturating_add(self.adapter_drift_component_count())
            .saturating_add(self.kv_drift_component_count())
            .saturating_add(self.planning_contract_drift_component_count())
    }

    pub fn token_budget_matches(self) -> bool {
        self.max_tokens_match_planning.unwrap_or(false)
            && self.generation_budget_matches_planning.unwrap_or(false)
    }

    pub fn adapter_matches(self) -> bool {
        self.selected_adapter_matches_planning.unwrap_or(false)
    }

    pub fn kv_import_matches(self) -> bool {
        self.imported_kv_matches_planning.unwrap_or(false)
            && self.kv_prefetch_matches_planning.unwrap_or(false)
    }

    pub fn planning_has_contract_violations(self) -> bool {
        self.planning_violation_count > 0
    }

    pub fn planning_has_pre_request_gate_problems(self) -> bool {
        self.planning_pre_request_problem_count > 0
    }

    pub fn planning_has_pressure_signals(self) -> bool {
        self.planning_pressure_signal_count > 0
    }

    pub fn planning_pre_request_gate_problem_component_count(self) -> usize {
        usize::from(self.planning_has_pre_request_gate_problems())
    }

    pub fn planning_pressure_signal_component_count(self) -> usize {
        usize::from(self.planning_has_pressure_signals())
    }

    pub fn request_matches_planning(self) -> bool {
        self.planning_attached()
            && self.token_budget_matches()
            && self.adapter_matches()
            && self.kv_import_matches()
            && !self.planning_has_contract_violations()
            && !self.planning_has_pre_request_gate_problems()
    }

    pub fn backend_wire_problem_component_count(self) -> usize {
        self.request_planning_drift_component_count()
            .saturating_add(self.planning_pre_request_gate_problem_component_count())
    }

    pub fn has_backend_wire_problem_components(self) -> bool {
        self.backend_wire_problem_component_count() > 0
    }

    pub fn backend_wire_accounting_is_consistent(self) -> bool {
        let expected_problem_count = self
            .planning_attachment_drift_component_count()
            .saturating_add(self.token_drift_component_count())
            .saturating_add(self.adapter_drift_component_count())
            .saturating_add(self.kv_drift_component_count())
            .saturating_add(self.planning_contract_drift_component_count())
            .saturating_add(self.planning_pre_request_gate_problem_component_count());

        self.backend_wire_problem_component_count() == expected_problem_count
            && self.has_backend_wire_problem_components() == (expected_problem_count > 0)
            && self.request_matches_planning() == (expected_problem_count == 0)
    }

    pub fn backend_wire_shape_is_clean(self) -> bool {
        !self.has_backend_wire_problem_components() && self.backend_wire_accounting_is_consistent()
    }

    pub fn can_use_backend_wire_request(self) -> bool {
        self.request_matches_planning() && self.backend_wire_shape_is_clean()
    }
}

impl RuntimeRequestGateSummary {
    pub fn has_acceptance_failures(self) -> bool {
        !self.request_accepted
    }

    pub fn has_request_contract_failures(self) -> bool {
        self.request_violation_count > 0
    }

    pub fn has_imported_kv_failures(self) -> bool {
        self.imported_kv_violation_count > 0
    }

    pub fn envelope_drifted(self) -> bool {
        !self.envelope_consistent
    }

    pub fn planning_drifted(self) -> bool {
        !self.planning_consistent
    }

    pub fn has_backend_wire_problem_components(self) -> bool {
        self.backend_wire_problem_count > 0
    }

    pub fn has_planning_pre_request_gate_problems(self) -> bool {
        self.planning_pre_request_problem_count > 0
    }

    pub fn has_planning_pressure_signals(self) -> bool {
        self.planning_pressure_signal_count > 0
    }

    pub fn backend_wire_problem_component_count(self) -> usize {
        self.backend_wire_problem_count
    }

    pub fn direct_backend_wire_problem_component_count(self) -> usize {
        self.backend_wire_problem_count
            .saturating_sub(self.planning_pre_request_problem_count)
    }

    pub fn planning_pre_request_gate_problem_component_count(self) -> usize {
        usize::from(self.has_planning_pre_request_gate_problems())
    }

    pub fn planning_pressure_signal_component_count(self) -> usize {
        usize::from(self.has_planning_pressure_signals())
    }

    pub fn imported_kv_activity_signal_component_count(self) -> usize {
        usize::from(self.accepted_imported_kv_blocks > 0)
    }

    pub fn send_gate_signal_component_count(self) -> usize {
        self.planning_pressure_signal_count
            .saturating_add(self.imported_kv_activity_signal_component_count())
    }

    pub fn send_gate_has_signal_components(self) -> bool {
        self.send_gate_signal_component_count() > 0
    }

    pub fn backend_wire_accounting_is_consistent(self) -> bool {
        self.backend_wire_problem_count >= self.planning_pre_request_problem_count
            && self.planning_consistent == !self.has_backend_wire_problem_components()
    }

    pub fn has_boundary_drift(self) -> bool {
        self.envelope_drifted() || self.planning_drifted()
    }

    pub fn has_failure_reports(self) -> bool {
        self.failure_report_count > 0
    }

    pub fn has_total_violations(self) -> bool {
        self.request_violation_count
            .saturating_add(self.imported_kv_violation_count)
            > 0
    }

    pub fn request_contract_failure_component_count(self) -> usize {
        usize::from(self.has_request_contract_failures())
    }

    pub fn imported_kv_failure_component_count(self) -> usize {
        usize::from(self.has_imported_kv_failures())
    }

    pub fn acceptance_failure_component_count(self) -> usize {
        self.request_contract_failure_component_count() + self.imported_kv_failure_component_count()
    }

    pub fn envelope_blocker_component_count(self) -> usize {
        usize::from(self.envelope_drifted())
    }

    pub fn planning_blocker_component_count(self) -> usize {
        usize::from(self.planning_drifted())
    }

    pub fn boundary_drift_component_count(self) -> usize {
        self.envelope_blocker_component_count() + self.planning_blocker_component_count()
    }

    pub fn mapped_failure_report_component_count(self) -> usize {
        usize::from(self.has_failure_reports())
    }

    pub fn send_blocker_component_count(self) -> usize {
        self.acceptance_failure_component_count()
            .saturating_add(self.boundary_drift_component_count())
            .saturating_add(self.mapped_failure_report_component_count())
    }

    pub fn send_gate_has_problem_components(self) -> bool {
        self.send_blocker_component_count() > 0
    }

    pub fn send_gate_accounting_is_consistent(self) -> bool {
        self.send_blocker_component_count()
            == self
                .acceptance_failure_component_count()
                .saturating_add(self.boundary_drift_component_count())
                .saturating_add(self.mapped_failure_report_component_count())
    }

    pub fn failure_report_matches_failures(self) -> bool {
        self.failure_report_count
            == self.request_contract_failure_component_count()
                + self.imported_kv_failure_component_count()
    }

    pub fn can_send_request(self) -> bool {
        !self.has_acceptance_failures() && !self.has_boundary_drift()
    }

    pub fn is_clean_send_gate(self) -> bool {
        self.can_send_request()
            && !self.has_request_contract_failures()
            && !self.has_imported_kv_failures()
            && self.failure_report_count == 0
    }

    pub fn request_gate_shape_is_clean(self) -> bool {
        self.is_clean_send_gate()
            && self.backend_wire_accounting_is_consistent()
            && self.send_gate_accounting_is_consistent()
            && self.failure_report_matches_failures()
    }

    pub fn runtime_request_commit_signal_component_count(self) -> usize {
        self.send_gate_signal_component_count()
    }

    pub fn has_runtime_request_commit_signals(self) -> bool {
        self.runtime_request_commit_signal_component_count() > 0
    }

    pub fn runtime_request_commit_blocker_component_count(self) -> usize {
        self.send_blocker_component_count()
            .saturating_add(self.direct_backend_wire_problem_component_count())
    }

    pub fn has_runtime_request_commit_blockers(self) -> bool {
        self.runtime_request_commit_blocker_component_count() > 0
    }

    pub fn runtime_request_commit_accounting_is_consistent(self) -> bool {
        let expected_blocker_count = self
            .send_blocker_component_count()
            .saturating_add(self.direct_backend_wire_problem_component_count());

        self.backend_wire_accounting_is_consistent()
            && self.send_gate_accounting_is_consistent()
            && self.failure_report_matches_failures()
            && self.runtime_request_commit_signal_component_count()
                == self.send_gate_signal_component_count()
            && self.has_runtime_request_commit_signals()
                == (self.runtime_request_commit_signal_component_count() > 0)
            && self.runtime_request_commit_blocker_component_count() == expected_blocker_count
            && self.has_runtime_request_commit_blockers() == (expected_blocker_count > 0)
    }

    pub fn runtime_request_commit_is_clean(self) -> bool {
        !self.has_runtime_request_commit_blockers()
            && self.runtime_request_commit_accounting_is_consistent()
    }

    pub fn can_commit_runtime_request(self) -> bool {
        self.can_send_request()
            && self.request_gate_shape_is_clean()
            && self.runtime_request_commit_is_clean()
    }

    pub fn can_send_runtime_request(self) -> bool {
        self.can_send_request() && self.request_gate_shape_is_clean()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeRequestAcceptanceReport {
    pub request_violations: Vec<String>,
    pub imported_kv_report: RuntimeKvValidationReport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeRequestAcceptanceSummary {
    pub accepted: bool,
    pub request_violation_count: usize,
    pub imported_kv_violation_count: usize,
    pub accepted_imported_kv_blocks: usize,
    pub failure_report_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeRequestFailureReturnSource {
    RequestAcceptance,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeRequestFailureReturnSummary {
    pub source: RuntimeRequestFailureReturnSource,
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
pub struct RuntimeRequestFailureReturnReport {
    pub source: RuntimeRequestFailureReturnSource,
    pub primary_failure: RuntimeFailureReport,
    pub primary_failure_summary: RuntimeFailureSummary,
    pub failure_batch: RuntimeFailureBatchSummary,
    pub failure_report_count: usize,
    pub can_format_runtime_failures: bool,
    pub total_blocker_component_count: usize,
}

impl RuntimeRequestFailureReturnSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::RequestAcceptance => "runtime_request_acceptance",
        }
    }
}

impl RuntimeRequestFailureReturnSummary {
    pub fn new(
        source: RuntimeRequestFailureReturnSource,
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

impl RuntimeRequestFailureReturnReport {
    pub fn new(
        source: RuntimeRequestFailureReturnSource,
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

    pub fn can_use_runtime_request_failure_return_report(&self) -> bool {
        self.failure_return_report_shape_is_clean()
    }
}

impl RuntimeRequestAcceptanceSummary {
    pub fn total_violation_count(self) -> usize {
        self.request_violation_count
            .saturating_add(self.imported_kv_violation_count)
    }

    pub fn has_request_contract_failures(self) -> bool {
        self.request_violation_count > 0
    }

    pub fn has_imported_kv_failures(self) -> bool {
        self.imported_kv_violation_count > 0
    }

    pub fn has_failures(self) -> bool {
        self.has_request_contract_failures() || self.has_imported_kv_failures()
    }

    pub fn request_contract_failure_component_count(self) -> usize {
        usize::from(self.has_request_contract_failures())
    }

    pub fn imported_kv_failure_component_count(self) -> usize {
        usize::from(self.has_imported_kv_failures())
    }

    pub fn acceptance_failure_component_count(self) -> usize {
        self.request_contract_failure_component_count()
            .saturating_add(self.imported_kv_failure_component_count())
    }

    pub fn has_failure_reports(self) -> bool {
        self.failure_report_count > 0
    }

    pub fn request_acceptance_problem_component_count(self) -> usize {
        self.acceptance_failure_component_count()
            .saturating_add(usize::from(self.has_failure_reports()))
    }

    pub fn has_request_acceptance_problem_components(self) -> bool {
        self.request_acceptance_problem_component_count() > 0
    }

    pub fn failure_report_matches_failures(self) -> bool {
        self.failure_report_count
            == usize::from(self.has_request_contract_failures())
                + usize::from(self.has_imported_kv_failures())
    }

    pub fn request_acceptance_accounting_is_consistent(self) -> bool {
        let expected_failure_count = self
            .request_contract_failure_component_count()
            .saturating_add(self.imported_kv_failure_component_count());
        let expected_problem_count =
            expected_failure_count.saturating_add(usize::from(self.has_failure_reports()));

        self.acceptance_failure_component_count() == expected_failure_count
            && self.request_acceptance_problem_component_count() == expected_problem_count
            && self.has_request_acceptance_problem_components() == (expected_problem_count > 0)
            && self.has_failures() == (expected_failure_count > 0)
            && self.failure_report_matches_failures()
            && self.accepted == (self.total_violation_count() == 0)
    }

    pub fn is_clean_acceptance(self) -> bool {
        self.accepted
            && !self.has_failures()
            && self.failure_report_count == 0
            && self.request_acceptance_accounting_is_consistent()
    }

    pub fn runtime_request_acceptance_commit_signal_component_count(self) -> usize {
        usize::from(self.accepted) + usize::from(self.accepted_imported_kv_blocks > 0)
    }

    pub fn runtime_request_acceptance_commit_blocker_component_count(self) -> usize {
        self.request_acceptance_problem_component_count()
    }

    pub fn runtime_request_acceptance_commit_accounting_is_consistent(self) -> bool {
        let expected_signal_count =
            usize::from(self.accepted) + usize::from(self.accepted_imported_kv_blocks > 0);

        self.request_acceptance_accounting_is_consistent()
            && self.runtime_request_acceptance_commit_signal_component_count()
                == expected_signal_count
            && self.runtime_request_acceptance_commit_blocker_component_count()
                == self.request_acceptance_problem_component_count()
    }

    pub fn runtime_request_acceptance_commit_is_clean(self) -> bool {
        self.runtime_request_acceptance_commit_blocker_component_count() == 0
            && self.runtime_request_acceptance_commit_accounting_is_consistent()
    }

    pub fn can_commit_runtime_request_acceptance(self) -> bool {
        self.accepted && self.runtime_request_acceptance_commit_is_clean()
    }

    pub fn request_acceptance_shape_is_clean(self) -> bool {
        self.runtime_request_acceptance_commit_is_clean()
    }

    pub fn can_accept_runtime_request(self) -> bool {
        self.can_commit_runtime_request_acceptance()
    }
}

impl RuntimeRequestAcceptanceReport {
    pub fn is_accepted(&self) -> bool {
        self.request_violations.is_empty() && self.imported_kv_report.is_valid()
    }

    pub fn acceptance_summary(&self) -> RuntimeRequestAcceptanceSummary {
        RuntimeRequestAcceptanceSummary {
            accepted: self.is_accepted(),
            request_violation_count: self.request_violations.len(),
            imported_kv_violation_count: self.imported_kv_report.violations.len(),
            accepted_imported_kv_blocks: self.imported_kv_report.accepted.len(),
            failure_report_count: usize::from(!self.request_violations.is_empty())
                + usize::from(!self.imported_kv_report.violations.is_empty()),
        }
    }

    pub fn violations(&self) -> Vec<String> {
        let mut violations = self.request_violations.clone();
        violations.extend(self.imported_kv_report.violations.clone());
        violations
    }

    pub fn accepted_imported_kv_blocks(&self) -> &[KvBlock] {
        &self.imported_kv_report.accepted
    }

    pub fn failure_reports(&self) -> Vec<RuntimeFailureReport> {
        let mut failures = Vec::new();

        if !self.request_violations.is_empty() {
            failures.push(RuntimeFailureReport::contract_violation(
                acceptance_message(
                    "runtime request acceptance failed",
                    &self.request_violations,
                ),
            ));
        }
        if !self.imported_kv_report.violations.is_empty() {
            failures.push(RuntimeFailureReport::kv_import(acceptance_message(
                "runtime request imported KV rejected",
                &self.imported_kv_report.violations,
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

    pub fn failure_return_summary(&self) -> RuntimeRequestFailureReturnSummary {
        let acceptance = self.acceptance_summary();
        let failure_batch = self.failure_batch_summary();
        let failure_report_count = failure_batch.total_count;
        let can_commit = acceptance.can_commit_runtime_request_acceptance();
        RuntimeRequestFailureReturnSummary::new(
            RuntimeRequestFailureReturnSource::RequestAcceptance,
            can_commit,
            !can_commit && failure_report_count > 0,
            self.primary_failure_summary(),
            failure_batch,
            failure_report_count,
            failure_batch.can_format_runtime_failures(),
            acceptance.runtime_request_acceptance_commit_blocker_component_count(),
            acceptance.runtime_request_acceptance_commit_accounting_is_consistent(),
        )
    }

    pub fn runtime_failure_return_report(&self) -> Option<RuntimeRequestFailureReturnReport> {
        let failure_return = self.failure_return_summary();
        if failure_return.can_return_runtime_failure() {
            self.primary_failure_report().map(|failure| {
                RuntimeRequestFailureReturnReport::new(
                    RuntimeRequestFailureReturnSource::RequestAcceptance,
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

fn acceptance_message(prefix: &str, violations: &[String]) -> String {
    if violations.is_empty() {
        prefix.to_owned()
    } else {
        format!("{prefix}: {}", violations.join("; "))
    }
}

impl RuntimeRequestEnvelope {
    pub fn from_parts(
        request: &InferenceRequest,
        architecture: TransformerRuntimeArchitecture,
        route_budget: RouteBudget,
        hierarchy: HierarchyWeights,
        transformer_plan: &TransformerPlanDigest,
        execution: &AdapterExecutionContext,
        imported_kv_blocks: usize,
    ) -> Self {
        Self {
            schema: RUNTIME_REQUEST_SCHEMA,
            profile: request.profile,
            prompt_chars: request.prompt.chars().count(),
            prompt_tokens: request.prompt_tokens,
            max_tokens: request.max_tokens,
            generation_budget: request.generation_budget(),
            runtime: request.runtime.clone(),
            architecture,
            route_budget,
            hierarchy,
            transformer_layer_count: transformer_plan.layers.len(),
            imported_kv_blocks,
            adapter_count: execution.adapters.len(),
            selected_adapter: execution.adapters.first().copied(),
            hardware_pressure: execution.hardware_pressure,
            kv_prefetch_blocks: execution.kv_prefetch_blocks,
            planning: None,
            recursive: None,
        }
    }

    pub fn with_planning_digest(mut self, planning: RuntimePlanningDigest) -> Self {
        self.max_tokens = planning.backend_max_tokens();
        self.generation_budget = planning.generation_budget;
        self.selected_adapter = Some(planning.adapter_selection.adapter);
        self.kv_prefetch_blocks = planning.planned_kv_exchange().import_blocks;
        self.hardware_pressure = planning.hardware_pressure;
        self.planning = Some(planning);
        self
    }

    pub fn with_recursive_schedule(mut self, recursive: RecursiveScheduleSummary) -> Self {
        self.recursive = Some(recursive);
        self
    }

    pub fn imported_kv_contract(&self) -> RuntimeKvBlockContract {
        RuntimeKvBlockContract::for_request_imports(self)
    }

    pub fn validate_imported_kv_blocks(&self, blocks: &[KvBlock]) -> RuntimeKvValidationReport {
        self.imported_kv_contract()
            .validate_blocks(blocks, &self.runtime, self.architecture)
    }

    pub fn acceptance_report(
        &self,
        imported_kv_blocks: &[KvBlock],
    ) -> RuntimeRequestAcceptanceReport {
        RuntimeRequestAcceptanceReport {
            request_violations: self.contract_violations(),
            imported_kv_report: self.validate_imported_kv_blocks(imported_kv_blocks),
        }
    }

    pub fn contract_violations(&self) -> Vec<String> {
        let mut violations = Vec::new();

        if self.schema != RUNTIME_REQUEST_SCHEMA {
            violations.push(format!(
                "runtime request schema {} does not match {}",
                self.schema, RUNTIME_REQUEST_SCHEMA
            ));
        }
        if !self.generation_budget.can_generate() {
            violations.push(format!(
                "runtime request cannot generate: prompt_tokens={} native_context_window={}",
                self.prompt_tokens, self.runtime.native_context_window
            ));
        }
        let runtime_metadata = self.runtime.shape_summary();
        if !runtime_metadata.can_commit_runtime_metadata_adapter() {
            violations.push(format!(
                "runtime metadata adapter is not committable: embedding_dimensions={} missing_components={}",
                runtime_metadata.embedding_dimensions,
                runtime_metadata.runtime_metadata_adapter_missing_component_count()
            ));
        }
        if self.architecture.layer_count == 0 {
            violations
                .push("runtime architecture layer_count must be greater than zero".to_owned());
        }
        if self.architecture.hidden_size == 0 {
            violations
                .push("runtime architecture hidden_size must be greater than zero".to_owned());
        }
        if self.architecture.attention_heads == 0 {
            violations
                .push("runtime architecture attention_heads must be greater than zero".to_owned());
        }
        if self.architecture.kv_heads == 0 {
            violations.push("runtime architecture kv_heads must be greater than zero".to_owned());
        }
        if self.architecture.kv_heads > self.architecture.attention_heads
            && self.architecture.attention_heads > 0
        {
            violations.push(format!(
                "runtime architecture kv_heads {} must not exceed attention_heads {}",
                self.architecture.kv_heads, self.architecture.attention_heads
            ));
        }
        if self.transformer_layer_count > 0
            && self.architecture.layer_count > 0
            && self.transformer_layer_count != self.architecture.layer_count
        {
            violations.push(format!(
                "transformer layer count {} does not match runtime architecture layer_count {}",
                self.transformer_layer_count, self.architecture.layer_count
            ));
        }
        if self.adapter_count == 0 {
            violations.push("runtime request has no adapter execution candidates".to_owned());
        }
        if !self.runtime.supports_kv_import && self.imported_kv_blocks > 0 {
            violations.push(format!(
                "runtime request imports {} KV blocks but runtime KV import is disabled",
                self.imported_kv_blocks
            ));
        }
        if self.runtime.supports_kv_import
            && self.runtime.max_kv_import_blocks > 0
            && self.imported_kv_blocks > self.runtime.max_kv_import_blocks
        {
            violations.push(format!(
                "runtime request imports {} KV blocks above runtime limit {}",
                self.imported_kv_blocks, self.runtime.max_kv_import_blocks
            ));
        }
        if self.runtime.supports_kv_import
            && self.runtime.max_kv_import_blocks > 0
            && self.kv_prefetch_blocks > self.runtime.max_kv_import_blocks
        {
            violations.push(format!(
                "runtime request prefetches {} KV blocks above runtime limit {}",
                self.kv_prefetch_blocks, self.runtime.max_kv_import_blocks
            ));
        }
        if let Some(planning) = self.planning {
            if self.max_tokens != planning.backend_max_tokens() {
                violations.push(format!(
                    "runtime request max_tokens {} differs from planned backend max_tokens {}",
                    self.max_tokens,
                    planning.backend_max_tokens()
                ));
            }
            if self.generation_budget != planning.generation_budget {
                violations.push(
                    "runtime request generation budget differs from planning digest".to_owned(),
                );
            }
            if self.selected_adapter != Some(planning.adapter_selection.adapter) {
                violations.push(format!(
                    "runtime request selected adapter {:?} differs from planned adapter {}",
                    self.selected_adapter,
                    planning.adapter_selection.adapter.as_str()
                ));
            }
            let planned_kv = planning.planned_kv_exchange();
            if self.imported_kv_blocks != planned_kv.import_blocks {
                violations.push(format!(
                    "runtime request imported KV count {} differs from planned KV imports {}",
                    self.imported_kv_blocks, planned_kv.import_blocks
                ));
            }
            if self.kv_prefetch_blocks != planned_kv.import_blocks {
                violations.push(format!(
                    "runtime request KV prefetch {} differs from planned KV imports {}",
                    self.kv_prefetch_blocks, planned_kv.import_blocks
                ));
            }
            violations.extend(planning.contract_violations());
        }
        if let Some(recursive) = self.recursive {
            violations.extend(recursive.contract_violations(self.prompt_tokens));
        }

        violations
    }

    pub fn is_valid(&self) -> bool {
        self.contract_violations().is_empty()
    }

    pub fn planning_parity_summary(&self) -> RuntimeRequestPlanningParitySummary {
        let planned_kv = self.planning.map(|planning| planning.planned_kv_exchange());

        RuntimeRequestPlanningParitySummary {
            has_planning_digest: self.planning.is_some(),
            request_max_tokens: self.max_tokens,
            planned_backend_max_tokens: self.planning.map(|planning| planning.backend_max_tokens()),
            max_tokens_match_planning: self
                .planning
                .map(|planning| self.max_tokens == planning.backend_max_tokens()),
            generation_budget_matches_planning: self
                .planning
                .map(|planning| self.generation_budget == planning.generation_budget),
            request_selected_adapter: self.selected_adapter,
            planned_adapter: self
                .planning
                .map(|planning| planning.adapter_selection.adapter),
            selected_adapter_matches_planning: self
                .planning
                .map(|planning| self.selected_adapter == Some(planning.adapter_selection.adapter)),
            imported_kv_blocks: self.imported_kv_blocks,
            kv_prefetch_blocks: self.kv_prefetch_blocks,
            planned_import_blocks: planned_kv.map(|kv| kv.import_blocks),
            planned_export_blocks: planned_kv.map(|kv| kv.export_blocks),
            imported_kv_matches_planning: planned_kv
                .map(|kv| self.imported_kv_blocks == kv.import_blocks),
            kv_prefetch_matches_planning: planned_kv
                .map(|kv| self.kv_prefetch_blocks == kv.import_blocks),
            planning_violation_count: self
                .planning
                .map(|planning| planning.contract_violations().len())
                .unwrap_or(0),
            planning_pre_request_problem_count: self
                .planning
                .map(|planning| {
                    planning
                        .planning_summary()
                        .pre_request_gate_problem_component_count()
                })
                .unwrap_or(0),
            planning_pressure_signal_count: self
                .planning
                .map(|planning| {
                    planning
                        .planning_summary()
                        .planning_pressure_signal_component_count()
                })
                .unwrap_or(0),
        }
    }

    pub fn request_gate_summary(
        &self,
        imported_kv_blocks: &[KvBlock],
    ) -> RuntimeRequestGateSummary {
        let acceptance = self
            .acceptance_report(imported_kv_blocks)
            .acceptance_summary();
        let envelope = self.envelope_summary();
        let planning = self.planning_parity_summary();
        let envelope_consistent = envelope.can_commit_runtime_request_envelope();
        let planning_consistent =
            !planning.planning_attached() || planning.request_matches_planning();

        RuntimeRequestGateSummary {
            request_accepted: acceptance.accepted,
            envelope_consistent,
            planning_attached: planning.planning_attached(),
            planning_consistent,
            accepted_imported_kv_blocks: acceptance.accepted_imported_kv_blocks,
            backend_wire_problem_count: if planning.planning_attached() {
                planning.backend_wire_problem_component_count()
            } else {
                0
            },
            planning_pre_request_problem_count: if planning.planning_attached() {
                planning.planning_pre_request_gate_problem_component_count()
            } else {
                0
            },
            planning_pressure_signal_count: if planning.planning_attached() {
                planning.planning_pressure_signal_count
            } else {
                0
            },
            request_violation_count: acceptance.request_violation_count,
            imported_kv_violation_count: acceptance.imported_kv_violation_count,
            failure_report_count: acceptance.failure_report_count,
        }
    }

    pub fn request_planning_readiness_summary(
        &self,
        runtime_planning: RuntimePlanningReadinessSummary,
        imported_kv_blocks: &[KvBlock],
    ) -> RuntimeRequestPlanningReadinessSummary {
        RuntimeRequestPlanningReadinessSummary::new(
            runtime_planning,
            self.planning_parity_summary(),
            self.request_gate_summary(imported_kv_blocks),
        )
    }

    pub fn manifest_request_planning_readiness_summary(
        &self,
        runtime_planning: RuntimePlanningReadinessSummary,
        manifest: &RuntimeManifestDigest,
        imported_kv_blocks: &[KvBlock],
    ) -> Option<RuntimeRequestManifestPlanningReadinessSummary> {
        self.planning.map(|planning| {
            RuntimeRequestManifestPlanningReadinessSummary::new(
                planning.manifest_kv_bridge_summary(manifest),
                self.request_planning_readiness_summary(runtime_planning, imported_kv_blocks),
            )
        })
    }

    pub fn envelope_summary(&self) -> RuntimeRequestEnvelopeSummary {
        let planned_kv = self.planning.map(|planning| planning.planned_kv_exchange());

        RuntimeRequestEnvelopeSummary {
            schema: self.schema,
            profile: self.profile,
            prompt_chars: self.prompt_chars,
            prompt_tokens: self.prompt_tokens,
            requested_max_tokens: self.generation_budget.requested_max_tokens,
            max_tokens: self.max_tokens,
            max_generated_tokens: self.generation_budget.max_generated_tokens,
            planned_context_tokens: self.generation_budget.planned_context_tokens,
            truncated_by_context: self.generation_budget.truncated_by_context,
            can_generate: self.generation_budget.can_generate(),
            model_context_window: self.runtime.native_context_window,
            runtime_embedding_dimensions: self.runtime.embedding_dimensions,
            runtime_metadata_adapter_ready: self
                .runtime
                .shape_summary()
                .can_commit_runtime_metadata_adapter(),
            runtime_import_enabled: self.runtime.supports_kv_import,
            runtime_export_enabled: self.runtime.supports_kv_export,
            max_kv_import_blocks: self.runtime.max_kv_import_blocks,
            max_kv_export_blocks: self.runtime.max_kv_export_blocks,
            architecture_layer_count: self.architecture.layer_count,
            architecture_hidden_size: self.architecture.hidden_size,
            transformer_layer_count: self.transformer_layer_count,
            imported_kv_blocks: self.imported_kv_blocks,
            adapter_count: self.adapter_count,
            selected_adapter: self.selected_adapter,
            hardware_pressure: self.hardware_pressure,
            hardware_pressure_band: DiagnosticsPressureBand::from_pressure(self.hardware_pressure),
            kv_prefetch_blocks: self.kv_prefetch_blocks,
            has_planning_digest: self.planning.is_some(),
            planning_backend_max_tokens: self
                .planning
                .map(|planning| planning.backend_max_tokens()),
            planning_import_blocks: planned_kv.map(|kv| kv.import_blocks),
            planning_export_blocks: planned_kv.map(|kv| kv.export_blocks),
            has_recursive_schedule: self.recursive.is_some(),
            recursive_requires_recursion: self
                .recursive
                .map(|recursive| recursive.requires_recursion)
                .unwrap_or(false),
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "schema={} profile={:?} prompt_tokens={} max_tokens={} planned_context={} model_id={} layers={} imported_kv={} adapters={} pressure={:.3} planning={} recursive={}",
            self.schema,
            self.profile,
            self.prompt_tokens,
            self.max_tokens,
            self.generation_budget.planned_context_tokens,
            self.runtime.model_id,
            self.transformer_layer_count,
            self.imported_kv_blocks,
            self.adapter_count,
            self.hardware_pressure,
            self.planning
                .map(|planning| planning.adapter_selection.adapter.as_str().to_owned())
                .unwrap_or_else(|| "none".to_owned()),
            self.recursive
                .map(|recursive| recursive.requires_recursion.to_string())
                .unwrap_or_else(|| "none".to_owned())
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::RuntimeAdapter;
    use crate::attention::{
        AttentionCandidateBatchSummary, AttentionDecisionSummary,
        AttentionSelectionReadinessSummary,
    };
    use crate::engine::RuntimeFailureKind;
    use crate::experiment::ExperimentSwitches;
    use crate::fht_dke::{
        DeterministicFhtDkeBudgeter, FhtDkeBudgetSummary, FhtDkePlanningReadinessSummary,
    };
    use crate::fusion::{KvFusionPolicy, ReinforcedKvFusionPolicy};
    use crate::manifest::{RuntimeKvPolicy, RuntimeManifestDigest};
    use crate::planning::{
        RuntimePlanningDigest, RuntimePlanningKvClampReason, RuntimePlanningKvClampSummary,
        RuntimePlanningKvExchange, RuntimePlanningReadinessStage, RuntimePlanningReadinessSummary,
        RuntimePlanningSummary,
    };
    use crate::recursive::RecursiveSchedulerConfig;
    use crate::router::{
        DefaultHierarchicalRouter, HierarchicalRouter, RouteBudgetReadinessSummary, RouteLayer,
        RouteLayerCounts, RoutingContext, RoutingDecisionSummary, TokenFeatures,
    };
    use crate::transformer::{
        TransformerAttentionKind, TransformerLayerBudget, TransformerPlanCounts,
        TransformerPlanSummary, TransformerPlanningPressureSummary,
        TransformerPlanningReadinessSummary,
    };

    #[test]
    fn request_envelope_summarizes_runtime_wire_contract() {
        let runtime = RuntimeMetadata::new("model", "tok", 128, 16)
            .with_kv_exchange(true, true)
            .with_kv_limits(2, 2);
        let request = InferenceRequest::new("hello", TaskProfile::Coding)
            .with_prompt_tokens(32)
            .with_max_tokens(16)
            .with_runtime(runtime);
        let architecture = TransformerRuntimeArchitecture::new(2, 16, 4, 2, 64);
        let transformer_plan = TransformerPlanDigest::new(
            Some("test"),
            vec![
                TransformerLayerBudget::new(0, TransformerAttentionKind::Global, 0.8, 64),
                TransformerLayerBudget::new(1, TransformerAttentionKind::LocalWindow, 0.6, 32),
            ],
        );
        let execution = AdapterExecutionContext::new([RuntimeAdapter::Cuda])
            .with_pressure(0.25, 0.80)
            .with_kv_prefetch_blocks(2);

        let envelope = RuntimeRequestEnvelope::from_parts(
            &request,
            architecture,
            RouteBudget::default(),
            HierarchyWeights::for_profile(TaskProfile::Coding),
            &transformer_plan,
            &execution,
            2,
        )
        .with_recursive_schedule(
            RecursiveSchedulerConfig::new(128, 64, 8, 2)
                .plan_tokens(32)
                .schedule_summary(),
        );

        assert!(envelope.is_valid());
        assert_eq!(envelope.schema, RUNTIME_REQUEST_SCHEMA);
        assert_eq!(envelope.prompt_chars, 5);
        assert_eq!(envelope.generation_budget.max_generated_tokens, 16);
        assert_eq!(envelope.transformer_layer_count, 2);
        assert_eq!(envelope.selected_adapter, Some(RuntimeAdapter::Cuda));
        assert_eq!(envelope.planning, None);
        assert_eq!(envelope.recursive.unwrap().prompt_tokens, 32);
        assert!(
            envelope
                .summary()
                .contains("schema=rust-norion-runtime-request-v1")
        );
    }

    #[test]
    fn request_envelope_reports_context_architecture_and_adapter_violations() {
        let runtime = RuntimeMetadata::new("model", "tok", 16, 8);
        let request = InferenceRequest::new("prompt", TaskProfile::General)
            .with_prompt_tokens(32)
            .with_max_tokens(8)
            .with_runtime(runtime);
        let architecture = TransformerRuntimeArchitecture::new(0, 0, 1, 2, 64);
        let transformer_plan = TransformerPlanDigest::new(
            Some("bad"),
            vec![TransformerLayerBudget::new(
                0,
                TransformerAttentionKind::Fusion,
                0.5,
                16,
            )],
        );
        let execution = AdapterExecutionContext::new(Vec::new());

        let envelope = RuntimeRequestEnvelope::from_parts(
            &request,
            architecture,
            RouteBudget::default(),
            HierarchyWeights::default(),
            &transformer_plan,
            &execution,
            1,
        );
        let joined = envelope.contract_violations().join("\n");

        assert!(!envelope.is_valid());
        assert!(joined.contains("runtime request cannot generate"));
        assert!(joined.contains("layer_count must be greater than zero"));
        assert!(joined.contains("hidden_size must be greater than zero"));
        assert!(joined.contains("kv_heads 2 must not exceed attention_heads 1"));
        assert!(joined.contains("runtime request has no adapter execution candidates"));
        assert!(joined.contains("runtime KV import is disabled"));
    }

    #[test]
    fn request_envelope_reports_kv_import_limit_violations() {
        let runtime = RuntimeMetadata::new("model", "tok", 128, 8)
            .with_kv_exchange(true, false)
            .with_kv_limits(1, 0);
        let request = InferenceRequest::new("prompt", TaskProfile::General)
            .with_prompt_tokens(8)
            .with_max_tokens(8)
            .with_runtime(runtime);
        let architecture = TransformerRuntimeArchitecture::new(1, 8, 1, 1, 64);
        let transformer_plan = TransformerPlanDigest::new(
            Some("one"),
            vec![TransformerLayerBudget::new(
                0,
                TransformerAttentionKind::Global,
                0.5,
                64,
            )],
        );
        let execution =
            AdapterExecutionContext::new([RuntimeAdapter::CpuSimd]).with_kv_prefetch_blocks(3);

        let envelope = RuntimeRequestEnvelope::from_parts(
            &request,
            architecture,
            RouteBudget::default(),
            HierarchyWeights::default(),
            &transformer_plan,
            &execution,
            2,
        );
        let joined = envelope.contract_violations().join("\n");
        let summary = envelope.envelope_summary();

        assert!(joined.contains("imports 2 KV blocks above runtime limit 1"));
        assert!(joined.contains("prefetches 3 KV blocks above runtime limit 1"));
        assert_eq!(summary.request_envelope_commit_blocker_component_count(), 2);
        assert!(summary.has_request_envelope_commit_blockers());
        assert!(summary.request_envelope_commit_accounting_is_consistent());
        assert!(!summary.request_envelope_commit_is_clean());
        assert!(!summary.can_commit_runtime_request_envelope());
    }

    #[test]
    fn request_envelope_validates_imported_kv_blocks_from_origin_contract() {
        let runtime = RuntimeMetadata::new("model", "tok", 128, 4)
            .with_kv_exchange(true, false)
            .with_kv_limits(1, 0);
        let request = InferenceRequest::new("prompt", TaskProfile::General)
            .with_prompt_tokens(8)
            .with_max_tokens(8)
            .with_runtime(runtime);
        let architecture = TransformerRuntimeArchitecture::new(1, 8, 1, 1, 64);
        let transformer_plan = TransformerPlanDigest::new(
            Some("one"),
            vec![TransformerLayerBudget::new(
                0,
                TransformerAttentionKind::Global,
                0.5,
                64,
            )],
        );
        let execution =
            AdapterExecutionContext::new([RuntimeAdapter::CpuSimd]).with_kv_prefetch_blocks(1);
        let envelope = RuntimeRequestEnvelope::from_parts(
            &request,
            architecture,
            RouteBudget::default(),
            HierarchyWeights::default(),
            &transformer_plan,
            &execution,
            1,
        );
        let valid = KvBlock::new(
            1,
            crate::kv::KvNamespace::Runtime,
            0,
            0,
            0..1,
            vec![0.1],
            vec![0.2],
        );
        let extra = KvBlock::new(
            2,
            crate::kv::KvNamespace::Gist,
            0,
            0,
            0..1,
            vec![0.3],
            vec![0.4],
        );

        let report = envelope.validate_imported_kv_blocks(&[valid.clone(), extra]);
        let joined = report.violations.join("\n");

        assert_eq!(envelope.imported_kv_contract().max_blocks, 1);
        assert_eq!(report.accepted, vec![valid]);
        assert!(joined.contains("imported KV block count 2 exceeds contract max_blocks 1"));
    }

    #[test]
    fn request_acceptance_report_combines_wire_and_imported_kv_violations() {
        let runtime = RuntimeMetadata::new("model", "tok", 128, 4)
            .with_kv_exchange(true, false)
            .with_kv_limits(1, 0);
        let request = InferenceRequest::new("prompt", TaskProfile::General)
            .with_prompt_tokens(8)
            .with_max_tokens(8)
            .with_runtime(runtime);
        let architecture = TransformerRuntimeArchitecture::new(1, 8, 1, 1, 64);
        let transformer_plan = TransformerPlanDigest::new(
            Some("request-acceptance"),
            vec![TransformerLayerBudget::new(
                0,
                TransformerAttentionKind::Global,
                0.5,
                64,
            )],
        );
        let execution = AdapterExecutionContext::new(Vec::new());
        let envelope = RuntimeRequestEnvelope::from_parts(
            &request,
            architecture,
            RouteBudget::default(),
            HierarchyWeights::default(),
            &transformer_plan,
            &execution,
            1,
        );
        let block = KvBlock::new(
            1,
            crate::kv::KvNamespace::Gist,
            0,
            0,
            0..1,
            vec![0.1],
            vec![0.2],
        );

        let report = envelope.acceptance_report(&[block]);
        let joined = report.violations().join("\n");
        let summary = report.acceptance_summary();

        assert!(!report.is_accepted());
        assert!(!summary.accepted);
        assert_eq!(
            summary.request_violation_count,
            report.request_violations.len()
        );
        assert_eq!(
            summary.imported_kv_violation_count,
            report.imported_kv_report.violations.len()
        );
        assert_eq!(
            summary.total_violation_count(),
            report.request_violations.len() + report.imported_kv_report.violations.len()
        );
        assert_eq!(summary.accepted_imported_kv_blocks, 0);
        assert_eq!(summary.failure_report_count, report.failure_reports().len());
        assert!(summary.has_request_contract_failures());
        assert!(summary.has_imported_kv_failures());
        assert!(summary.has_failures());
        assert_eq!(summary.request_contract_failure_component_count(), 1);
        assert_eq!(summary.imported_kv_failure_component_count(), 1);
        assert_eq!(summary.acceptance_failure_component_count(), 2);
        assert!(summary.has_failure_reports());
        assert_eq!(summary.request_acceptance_problem_component_count(), 3);
        assert!(summary.failure_report_matches_failures());
        assert_eq!(
            summary.runtime_request_acceptance_commit_signal_component_count(),
            0
        );
        assert_eq!(
            summary.runtime_request_acceptance_commit_blocker_component_count(),
            3
        );
        assert!(summary.runtime_request_acceptance_commit_accounting_is_consistent());
        assert!(!summary.runtime_request_acceptance_commit_is_clean());
        assert!(!summary.can_commit_runtime_request_acceptance());
        assert!(!summary.is_clean_acceptance());
        assert!(report.accepted_imported_kv_blocks().is_empty());
        assert!(joined.contains("runtime request has no adapter execution candidates"));
        assert!(joined.contains("namespace gist is not runtime"));
    }

    #[test]
    fn request_acceptance_report_maps_failures_to_runtime_failure_reports() {
        let clean_report = RuntimeRequestAcceptanceReport {
            request_violations: Vec::new(),
            imported_kv_report: RuntimeKvValidationReport {
                accepted: Vec::new(),
                violations: Vec::new(),
            },
        };
        let clean_failure_return = clean_report.failure_return_summary();
        assert_eq!(
            clean_failure_return.source,
            RuntimeRequestFailureReturnSource::RequestAcceptance
        );
        assert_eq!(
            clean_failure_return.source.label(),
            "runtime_request_acceptance"
        );
        assert!(!clean_failure_return.has_failure_reports());
        assert!(!clean_failure_return.has_blocker_components());
        assert!(clean_failure_return.failure_return_accounting_is_consistent());
        assert!(!clean_failure_return.can_return_runtime_failure());
        assert_eq!(clean_report.runtime_failure_return_report(), None);

        let runtime = RuntimeMetadata::new("model", "tok", 128, 4)
            .with_kv_exchange(true, false)
            .with_kv_limits(1, 0);
        let request = InferenceRequest::new("prompt", TaskProfile::General)
            .with_prompt_tokens(8)
            .with_max_tokens(8)
            .with_runtime(runtime);
        let architecture = TransformerRuntimeArchitecture::new(1, 8, 1, 1, 64);
        let transformer_plan = TransformerPlanDigest::new(
            Some("request-failure"),
            vec![TransformerLayerBudget::new(
                0,
                TransformerAttentionKind::Global,
                0.5,
                64,
            )],
        );
        let execution = AdapterExecutionContext::new(Vec::new());
        let envelope = RuntimeRequestEnvelope::from_parts(
            &request,
            architecture,
            RouteBudget::default(),
            HierarchyWeights::default(),
            &transformer_plan,
            &execution,
            1,
        );
        let block = KvBlock::new(
            1,
            crate::kv::KvNamespace::Gist,
            0,
            0,
            0..1,
            vec![0.1],
            vec![0.2],
        );

        let report = envelope.acceptance_report(&[block]);
        let failures = report.failure_reports();
        let failure_batch = report.failure_batch_summary();
        let primary_summary = report.primary_failure_summary().unwrap();

        assert_eq!(failures.len(), 2);
        assert_eq!(
            failure_batch,
            RuntimeFailureReport::batch_summary(&failures)
        );
        assert_eq!(failure_batch.total_count, 2);
        assert_eq!(failure_batch.contract_violation_count, 1);
        assert_eq!(failure_batch.kv_import_count, 1);
        assert_eq!(failure_batch.recoverable_count, 1);
        assert_eq!(failure_batch.backend_error_count, 1);
        assert!(failure_batch.has_kv_failures());
        assert!(failure_batch.has_contract_failures());
        assert!(failure_batch.failure_batch_shape_is_clean());
        assert!(failure_batch.can_format_runtime_failures());
        assert_eq!(failures[0].kind, RuntimeFailureKind::ContractViolation);
        assert_eq!(failures[0].kind.trace_label(), "runtime_contract_violation");
        assert!(failures[0].is_recoverable());
        assert!(failures[0].message.contains("request acceptance failed"));
        assert_eq!(failures[1].kind, RuntimeFailureKind::KvImport);
        assert_eq!(failures[1].kind.trace_label(), "runtime_kv_import_error");
        assert!(!failures[1].is_recoverable());
        assert!(failures[1].diagnostics_note().contains("namespace gist"));
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
            RuntimeRequestFailureReturnSource::RequestAcceptance
        );
        assert!(failure_return.has_failure_reports());
        assert!(failure_return.has_blocker_components());
        assert!(failure_return.failure_return_accounting_is_consistent());
        assert!(failure_return.can_return_runtime_failure());
        let return_report = report
            .runtime_failure_return_report()
            .expect("request acceptance return report");
        assert_eq!(
            return_report.source,
            RuntimeRequestFailureReturnSource::RequestAcceptance
        );
        assert_eq!(return_report.primary_failure_summary, primary_summary);
        assert_eq!(return_report.failure_batch.total_count, 2);
        assert_eq!(return_report.failure_batch.contract_violation_count, 1);
        assert!(return_report.failure_return_report_shape_is_clean());
        assert!(return_report.can_use_runtime_request_failure_return_report());
        assert!(
            return_report
                .backend_message()
                .contains("runtime request acceptance failed")
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
    fn request_envelope_reports_recursive_schedule_violations() {
        let runtime = RuntimeMetadata::new("model", "tok", 128, 8);
        let request = InferenceRequest::new("prompt", TaskProfile::General)
            .with_prompt_tokens(8)
            .with_max_tokens(8)
            .with_runtime(runtime);
        let architecture = TransformerRuntimeArchitecture::new(1, 8, 1, 1, 64);
        let transformer_plan = TransformerPlanDigest::new(
            Some("one"),
            vec![TransformerLayerBudget::new(
                0,
                TransformerAttentionKind::Global,
                0.5,
                64,
            )],
        );
        let execution = AdapterExecutionContext::new([RuntimeAdapter::CpuSimd]);
        let mut recursive = RecursiveSchedulerConfig::new(128, 64, 8, 2)
            .plan_tokens(9)
            .schedule_summary();
        recursive.execution_wave_count = 0;

        let envelope = RuntimeRequestEnvelope::from_parts(
            &request,
            architecture,
            RouteBudget::default(),
            HierarchyWeights::default(),
            &transformer_plan,
            &execution,
            0,
        )
        .with_recursive_schedule(recursive);
        let joined = envelope.contract_violations().join("\n");

        assert!(joined.contains("prompt_tokens 9 differ from request prompt_tokens 8"));
        assert!(joined.contains("has chunks but no execution waves"));
    }

    #[test]
    fn request_envelope_uses_planning_digest_for_backend_wire_fields() {
        let runtime = RuntimeMetadata::new("model", "tok", 128, 16)
            .with_kv_exchange(true, true)
            .with_kv_limits(1, 4);
        let request = InferenceRequest::new("hello", TaskProfile::Coding)
            .with_prompt_tokens(120)
            .with_max_tokens(32)
            .with_runtime(runtime)
            .with_experiments(ExperimentSwitches::default().with_fht_dke(true));
        let architecture = TransformerRuntimeArchitecture::new(2, 16, 4, 2, 64);
        let transformer_plan = TransformerPlanDigest::new(
            Some("test"),
            vec![
                TransformerLayerBudget::new(0, TransformerAttentionKind::Global, 0.8, 64),
                TransformerLayerBudget::new(1, TransformerAttentionKind::LocalWindow, 0.6, 32),
            ],
        );
        let execution =
            AdapterExecutionContext::new([RuntimeAdapter::CpuSimd, RuntimeAdapter::Cuda])
                .with_pressure(0.35, 0.60)
                .with_kv_prefetch_blocks(4);
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

        let envelope = RuntimeRequestEnvelope::from_parts(
            &request,
            architecture,
            RouteBudget::default(),
            HierarchyWeights::for_profile(TaskProfile::Coding),
            &transformer_plan,
            &execution,
            planning.planned_kv_exchange().import_blocks,
        )
        .with_planning_digest(planning);

        let parity = envelope.planning_parity_summary();
        let imported = (0..envelope.imported_kv_blocks)
            .map(|index| {
                KvBlock::new(
                    index as u64,
                    crate::kv::KvNamespace::Runtime,
                    0,
                    0,
                    index..index + 1,
                    vec![0.1],
                    vec![0.2],
                )
            })
            .collect::<Vec<_>>();
        let gate = envelope.request_gate_summary(&imported);

        assert!(envelope.is_valid());
        assert_eq!(envelope.max_tokens, 8);
        assert_eq!(envelope.generation_budget, planning.generation_budget);
        assert_eq!(envelope.selected_adapter, Some(RuntimeAdapter::CpuSimd));
        assert_eq!(
            envelope.kv_prefetch_blocks,
            planning.planned_kv_exchange().import_blocks
        );
        assert!(envelope.summary().contains("planning=cpu-simd"));
        assert!(parity.planning_attached());
        assert_eq!(parity.request_max_tokens, planning.backend_max_tokens());
        assert_eq!(
            parity.planned_backend_max_tokens,
            Some(planning.backend_max_tokens())
        );
        assert_eq!(parity.max_tokens_match_planning, Some(true));
        assert_eq!(parity.generation_budget_matches_planning, Some(true));
        assert_eq!(
            parity.request_selected_adapter,
            Some(RuntimeAdapter::CpuSimd)
        );
        assert_eq!(parity.planned_adapter, Some(RuntimeAdapter::CpuSimd));
        assert_eq!(parity.selected_adapter_matches_planning, Some(true));
        assert_eq!(
            parity.planned_import_blocks,
            Some(planning.planned_kv_exchange().import_blocks)
        );
        assert_eq!(
            parity.planned_export_blocks,
            Some(planning.planned_kv_exchange().export_blocks)
        );
        assert_eq!(parity.imported_kv_matches_planning, Some(true));
        assert_eq!(parity.kv_prefetch_matches_planning, Some(true));
        assert_eq!(parity.planning_violation_count, 0);
        assert_eq!(parity.planning_pre_request_problem_count, 0);
        assert_eq!(parity.planning_pressure_signal_count, 7);
        assert!(!parity.planning_missing_from_request());
        assert!(!parity.max_tokens_drifted_from_planning());
        assert!(!parity.generation_budget_drifted_from_planning());
        assert!(!parity.adapter_drifted_from_planning());
        assert!(!parity.imported_kv_drifted_from_planning());
        assert!(!parity.kv_prefetch_drifted_from_planning());
        assert!(!parity.planning_has_pre_request_gate_problems());
        assert!(parity.planning_has_pressure_signals());
        assert_eq!(parity.token_drift_component_count(), 0);
        assert_eq!(parity.max_token_drift_component_count(), 0);
        assert_eq!(parity.generation_budget_drift_component_count(), 0);
        assert_eq!(parity.adapter_drift_component_count(), 0);
        assert_eq!(parity.kv_drift_component_count(), 0);
        assert_eq!(parity.imported_kv_drift_component_count(), 0);
        assert_eq!(parity.kv_prefetch_drift_component_count(), 0);
        assert_eq!(parity.planning_attachment_drift_component_count(), 0);
        assert_eq!(parity.planning_contract_drift_component_count(), 0);
        assert_eq!(
            parity.planning_pre_request_gate_problem_component_count(),
            0
        );
        assert_eq!(parity.planning_pressure_signal_component_count(), 1);
        assert_eq!(parity.request_planning_drift_component_count(), 0);
        assert_eq!(parity.backend_wire_problem_component_count(), 0);
        assert!(!parity.has_backend_wire_problem_components());
        assert!(parity.backend_wire_accounting_is_consistent());
        assert!(parity.backend_wire_shape_is_clean());
        assert!(parity.can_use_backend_wire_request());
        assert!(parity.token_budget_matches());
        assert!(parity.adapter_matches());
        assert!(parity.kv_import_matches());
        assert!(!parity.planning_has_contract_violations());
        assert!(parity.request_matches_planning());
        assert!(gate.request_accepted);
        assert!(gate.envelope_consistent);
        assert!(gate.planning_attached);
        assert!(gate.planning_consistent);
        assert_eq!(gate.accepted_imported_kv_blocks, imported.len());
        assert_eq!(gate.backend_wire_problem_count, 0);
        assert_eq!(gate.planning_pre_request_problem_count, 0);
        assert_eq!(gate.planning_pressure_signal_count, 7);
        assert_eq!(gate.request_violation_count, 0);
        assert_eq!(gate.imported_kv_violation_count, 0);
        assert_eq!(gate.failure_report_count, 0);
        assert_eq!(gate.backend_wire_problem_component_count(), 0);
        assert_eq!(gate.direct_backend_wire_problem_component_count(), 0);
        assert_eq!(gate.planning_pre_request_gate_problem_component_count(), 0);
        assert_eq!(gate.planning_pressure_signal_component_count(), 1);
        assert_eq!(gate.imported_kv_activity_signal_component_count(), 1);
        assert_eq!(gate.send_gate_signal_component_count(), 8);
        assert!(gate.send_gate_has_signal_components());
        assert!(!gate.has_backend_wire_problem_components());
        assert!(!gate.has_planning_pre_request_gate_problems());
        assert!(gate.has_planning_pressure_signals());
        assert!(gate.backend_wire_accounting_is_consistent());
        assert!(!gate.has_acceptance_failures());
        assert!(!gate.has_request_contract_failures());
        assert!(!gate.has_imported_kv_failures());
        assert!(!gate.envelope_drifted());
        assert!(!gate.planning_drifted());
        assert!(!gate.has_boundary_drift());
        assert!(!gate.has_failure_reports());
        assert!(!gate.has_total_violations());
        assert_eq!(gate.request_contract_failure_component_count(), 0);
        assert_eq!(gate.imported_kv_failure_component_count(), 0);
        assert_eq!(gate.acceptance_failure_component_count(), 0);
        assert_eq!(gate.envelope_blocker_component_count(), 0);
        assert_eq!(gate.planning_blocker_component_count(), 0);
        assert_eq!(gate.boundary_drift_component_count(), 0);
        assert_eq!(gate.mapped_failure_report_component_count(), 0);
        assert_eq!(gate.send_blocker_component_count(), 0);
        assert!(!gate.send_gate_has_problem_components());
        assert!(gate.send_gate_accounting_is_consistent());
        assert!(gate.failure_report_matches_failures());
        assert!(gate.can_send_request());
        assert!(gate.is_clean_send_gate());
        assert!(gate.request_gate_shape_is_clean());
        assert_eq!(gate.runtime_request_commit_signal_component_count(), 8);
        assert!(gate.has_runtime_request_commit_signals());
        assert_eq!(gate.runtime_request_commit_blocker_component_count(), 0);
        assert!(!gate.has_runtime_request_commit_blockers());
        assert!(gate.runtime_request_commit_accounting_is_consistent());
        assert!(gate.runtime_request_commit_is_clean());
        assert!(gate.can_commit_runtime_request());
        assert!(gate.can_send_runtime_request());
        let runtime_planning = clean_runtime_planning_readiness();
        let readiness = envelope.request_planning_readiness_summary(runtime_planning, &imported);

        assert_eq!(
            readiness,
            RuntimeRequestPlanningReadinessSummary::new(runtime_planning, parity, gate)
        );
        assert!(readiness.runtime_planning_ready());
        assert!(readiness.request_planning_ready());
        assert!(readiness.request_gate_ready());
        assert_eq!(readiness.first_unready_stage(), None);
        assert_eq!(readiness.first_blocking_stage(), None);
        assert_eq!(
            readiness.request_planning_signal_component_count,
            parity.planning_pressure_signal_component_count()
        );
        assert_eq!(
            readiness.request_gate_signal_component_count,
            gate.runtime_request_commit_signal_component_count()
        );
        assert!(readiness.can_commit_runtime_request_planning());
    }

    #[test]
    fn request_envelope_blocks_missing_runtime_metadata_adapter_after_generation_degrade() {
        let runtime = RuntimeMetadata::new("model", "tok", 4096, 0)
            .with_kv_exchange(true, false)
            .with_kv_limits(2, 0);
        let metadata = runtime.shape_summary();
        let request = InferenceRequest::new("hello", TaskProfile::Coding)
            .with_prompt_tokens(4000)
            .with_max_tokens(256)
            .with_runtime(runtime)
            .with_experiments(ExperimentSwitches::default().with_fht_dke(true));
        let route_budget = clean_route_budget();
        let architecture = TransformerRuntimeArchitecture::new(2, 128, 4, 2, 64);
        let transformer_plan = TransformerPlanDigest::new(
            Some("metadata-degrade"),
            vec![
                TransformerLayerBudget::new(0, TransformerAttentionKind::Global, 0.8, 64),
                TransformerLayerBudget::new(1, TransformerAttentionKind::LocalWindow, 0.6, 32),
            ],
        );
        let execution =
            AdapterExecutionContext::new([RuntimeAdapter::CpuSimd]).with_pressure(0.30, 0.80);
        let planning = RuntimePlanningDigest::from_request(
            &request,
            route_budget,
            &execution,
            &[],
            &DeterministicFhtDkeBudgeter::new(0.10, 0.60, 64),
        );
        let runtime_planning = RuntimePlanningReadinessSummary::new(
            clean_fht_dke_planning_readiness(route_budget, planning.fht_dke_summary()),
            planning.planning_summary(),
        );
        let envelope = RuntimeRequestEnvelope::from_parts(
            &request,
            architecture,
            route_budget,
            HierarchyWeights::for_profile(TaskProfile::Coding),
            &transformer_plan,
            &execution,
            0,
        )
        .with_planning_digest(planning);
        let summary = envelope.envelope_summary();
        let parity = envelope.planning_parity_summary();
        let gate = envelope.request_gate_summary(&[]);
        let readiness = envelope.request_planning_readiness_summary(runtime_planning, &[]);
        let joined = envelope.contract_violations().join("\n");

        assert!(metadata.has_known_context_window());
        assert!(!metadata.has_embedding_dimensions());
        assert!(metadata.can_use_runtime_metadata_contract());
        assert!(!metadata.can_commit_runtime_metadata_adapter());
        assert_eq!(
            metadata.runtime_metadata_adapter_blocker_component_count(),
            1
        );
        assert_eq!(planning.backend_max_tokens(), 96);
        assert!(planning.generation_budget.truncated_but_can_generate());
        assert!(planning.generation_budget.can_commit_backend_max_tokens());
        assert!(planning.contract_violations().is_empty());
        assert!(runtime_planning.can_commit_runtime_planning_readiness());
        assert_eq!(summary.runtime_embedding_dimensions, 0);
        assert!(!summary.runtime_metadata_adapter_ready);
        assert!(summary.can_generate);
        assert!(summary.context_limited_generation());
        assert_eq!(summary.request_envelope_commit_blocker_component_count(), 1);
        assert!(summary.has_request_envelope_commit_blockers());
        assert!(summary.request_envelope_commit_accounting_is_consistent());
        assert!(!summary.can_commit_runtime_request_envelope());
        assert!(!envelope.is_valid());
        assert!(joined.contains("runtime metadata adapter is not committable"));
        assert!(parity.can_use_backend_wire_request());
        assert_eq!(parity.planning_pre_request_problem_count, 0);
        assert!(gate.request_accepted == false);
        assert!(!gate.envelope_consistent);
        assert!(gate.planning_consistent);
        assert_eq!(gate.backend_wire_problem_count, 0);
        assert_eq!(gate.request_violation_count, 1);
        assert!(!gate.can_commit_runtime_request());
        assert!(readiness.runtime_planning_ready());
        assert!(readiness.request_planning_ready());
        assert!(!readiness.request_gate_ready());
        assert_eq!(
            readiness.first_unready_stage(),
            Some(RuntimeRequestPlanningReadinessStage::RequestGate)
        );
        assert_eq!(
            readiness.first_blocking_stage(),
            Some(RuntimeRequestPlanningReadinessStage::RequestGate)
        );
        assert_eq!(readiness.runtime_planning_blocker_component_count, 0);
        assert_eq!(readiness.request_planning_blocker_component_count, 0);
        assert!(readiness.request_gate_blocker_component_count > 0);
        assert!(readiness.runtime_request_planning_accounting_is_consistent());
        assert!(!readiness.can_commit_runtime_request_planning());
    }

    #[test]
    fn request_envelope_preserves_router_budget_but_blocks_missing_runtime_metadata_adapter() {
        let router = DefaultHierarchicalRouter::new();
        let tokens = [
            TokenFeatures::new("fast", 0.10, 0),
            TokenFeatures::new("local-a", 0.70, 1),
            TokenFeatures::new("local-b", 0.80, 2),
            TokenFeatures::new("global", 0.90, 3),
        ];
        let routing_context = RoutingContext {
            profile: TaskProfile::Coding,
            hierarchy: HierarchyWeights::for_profile(TaskProfile::Coding),
            ..RoutingContext::default()
        };
        let route_budget = router.budget(&tokens, routing_context);
        let runtime = RuntimeMetadata::new("model", "tok", 4096, 0)
            .with_kv_exchange(true, false)
            .with_kv_limits(2, 0);
        let metadata = runtime.shape_summary();
        let request = InferenceRequest::new("hello", TaskProfile::Coding)
            .with_prompt_tokens(512)
            .with_max_tokens(128)
            .with_runtime(runtime)
            .with_experiments(ExperimentSwitches::default().with_fht_dke(true));
        let architecture = TransformerRuntimeArchitecture::new(2, 128, 4, 2, 64);
        let transformer_plan = TransformerPlanDigest::new(
            Some("metadata-router-budget"),
            vec![
                TransformerLayerBudget::new(0, TransformerAttentionKind::Global, 0.8, 64),
                TransformerLayerBudget::new(1, TransformerAttentionKind::LocalWindow, 0.6, 32),
            ],
        );
        let execution =
            AdapterExecutionContext::new([RuntimeAdapter::CpuSimd]).with_pressure(0.30, 0.80);
        let planning = RuntimePlanningDigest::from_request(
            &request,
            route_budget,
            &execution,
            &[],
            &DeterministicFhtDkeBudgeter::new(0.10, 0.60, 64),
        );
        let runtime_planning = RuntimePlanningReadinessSummary::new(
            clean_fht_dke_planning_readiness(route_budget, planning.fht_dke_summary()),
            planning.planning_summary(),
        );
        let envelope = RuntimeRequestEnvelope::from_parts(
            &request,
            architecture,
            route_budget,
            HierarchyWeights::for_profile(TaskProfile::Coding),
            &transformer_plan,
            &execution,
            0,
        )
        .with_planning_digest(planning);
        let summary = envelope.envelope_summary();
        let parity = envelope.planning_parity_summary();
        let gate = envelope.request_gate_summary(&[]);
        let readiness = envelope.request_planning_readiness_summary(runtime_planning, &[]);
        let joined = envelope.contract_violations().join("\n");

        assert_eq!(route_budget.fast_tokens, 1);
        assert_eq!(route_budget.attention_tokens, 3);
        assert_eq!(route_budget.attention_fraction, 0.75);
        assert!(route_budget.can_use_route_budget());
        assert_eq!(envelope.route_budget, route_budget);
        assert_eq!(
            planning.fht_dke_summary().attention_threshold,
            route_budget.threshold
        );
        assert_eq!(
            planning.fht_dke_summary().route_pressure,
            route_budget.attention_fraction
        );
        assert!(metadata.can_use_runtime_metadata_contract());
        assert!(!metadata.can_commit_runtime_metadata_adapter());
        assert!(planning.generation_budget.can_commit_backend_max_tokens());
        assert!(runtime_planning.can_commit_runtime_planning_readiness());
        assert_eq!(summary.runtime_embedding_dimensions, 0);
        assert!(!summary.runtime_metadata_adapter_ready);
        assert!(summary.can_generate);
        assert_eq!(summary.request_envelope_commit_blocker_component_count(), 1);
        assert!(summary.has_request_envelope_commit_blockers());
        assert!(summary.request_envelope_commit_accounting_is_consistent());
        assert!(!summary.can_commit_runtime_request_envelope());
        assert!(parity.can_use_backend_wire_request());
        assert_eq!(parity.planning_pre_request_problem_count, 0);
        assert_eq!(
            parity.planning_pressure_signal_count,
            planning
                .planning_summary()
                .planning_pressure_signal_component_count()
        );
        assert!(!gate.request_accepted);
        assert!(!gate.envelope_consistent);
        assert!(gate.planning_consistent);
        assert_eq!(gate.request_violation_count, 1);
        assert_eq!(gate.backend_wire_problem_count, 0);
        assert!(joined.contains("runtime metadata adapter is not committable"));
        assert!(readiness.runtime_planning_ready());
        assert!(readiness.request_planning_ready());
        assert!(!readiness.request_gate_ready());
        assert_eq!(
            readiness.first_blocking_stage(),
            Some(RuntimeRequestPlanningReadinessStage::RequestGate)
        );
        assert_eq!(readiness.runtime_planning_blocker_component_count, 0);
        assert_eq!(readiness.request_planning_blocker_component_count, 0);
        assert!(readiness.request_gate_blocker_component_count > 0);
        assert!(readiness.runtime_request_planning_accounting_is_consistent());
        assert!(!readiness.can_commit_runtime_request_planning());
    }

    #[test]
    fn request_envelope_blocks_unknown_context_runtime_metadata_after_budget_normalizes() {
        let router = DefaultHierarchicalRouter::new();
        let tokens = [
            TokenFeatures::new("fast", 0.10, 0),
            TokenFeatures::new("local-a", 0.70, 1),
            TokenFeatures::new("local-b", 0.80, 2),
            TokenFeatures::new("global", 0.90, 3),
        ];
        let routing_context = RoutingContext {
            profile: TaskProfile::Coding,
            hierarchy: HierarchyWeights::for_profile(TaskProfile::Coding),
            ..RoutingContext::default()
        };
        let route_budget = router.budget(&tokens, routing_context);
        let runtime = RuntimeMetadata::new("model", "tok", 0, 2048)
            .with_kv_exchange(true, true)
            .with_kv_limits(2, 1);
        let metadata = runtime.shape_summary();
        let request = InferenceRequest::new("hello", TaskProfile::Coding)
            .with_prompt_tokens(512)
            .with_max_tokens(64)
            .with_runtime(runtime)
            .with_experiments(ExperimentSwitches::default().with_fht_dke(true));
        let architecture = TransformerRuntimeArchitecture::new(2, 128, 4, 2, 64);
        let transformer_plan = TransformerPlanDigest::new(
            Some("unknown-context-metadata"),
            vec![
                TransformerLayerBudget::new(0, TransformerAttentionKind::Global, 0.8, 64),
                TransformerLayerBudget::new(1, TransformerAttentionKind::LocalWindow, 0.6, 32),
            ],
        );
        let execution = AdapterExecutionContext::new([RuntimeAdapter::Cuda])
            .with_pressure(0.45, 0.55)
            .with_kv_prefetch_blocks(1);
        let planning = RuntimePlanningDigest::from_request(
            &request,
            route_budget,
            &execution,
            &[],
            &DeterministicFhtDkeBudgeter::new(0.10, 0.60, 64),
        );
        let runtime_planning = RuntimePlanningReadinessSummary::new(
            clean_fht_dke_planning_readiness(route_budget, planning.fht_dke_summary()),
            planning.planning_summary(),
        );
        let envelope = RuntimeRequestEnvelope::from_parts(
            &request,
            architecture,
            route_budget,
            HierarchyWeights::for_profile(TaskProfile::Coding),
            &transformer_plan,
            &execution,
            planning.planned_kv_exchange().import_blocks,
        )
        .with_planning_digest(planning);
        let summary = envelope.envelope_summary();
        let parity = envelope.planning_parity_summary();
        let gate = envelope.request_gate_summary(&[]);
        let readiness = envelope.request_planning_readiness_summary(runtime_planning, &[]);
        let joined = envelope.contract_violations().join("\n");

        assert_eq!(route_budget.fast_tokens, 1);
        assert_eq!(route_budget.attention_tokens, 3);
        assert_eq!(route_budget.attention_fraction, 0.75);
        assert!(route_budget.can_use_route_budget());
        assert!(!metadata.has_known_context_window());
        assert!(metadata.has_embedding_dimensions());
        assert!(metadata.can_use_runtime_metadata_contract());
        assert!(!metadata.can_commit_runtime_metadata_adapter());
        assert_eq!(
            metadata.runtime_metadata_adapter_missing_component_count(),
            1
        );
        assert_eq!(planning.backend_max_tokens(), 64);
        assert!(!planning.context_limited());
        assert!(planning.generation_budget.can_generate());
        assert!(!planning.generation_budget.truncated_by_context);
        assert!(planning.generation_budget.can_commit_backend_max_tokens());
        assert!(planning.contract_violations().is_empty());
        assert!(runtime_planning.can_commit_runtime_planning_readiness());
        assert_eq!(summary.model_context_window, 0);
        assert_eq!(summary.runtime_embedding_dimensions, 2048);
        assert!(!summary.runtime_metadata_adapter_ready);
        assert!(summary.can_generate);
        assert!(!summary.context_limited_generation());
        assert_eq!(summary.request_envelope_commit_blocker_component_count(), 1);
        assert!(summary.has_request_envelope_commit_blockers());
        assert!(summary.request_envelope_commit_accounting_is_consistent());
        assert!(!summary.can_commit_runtime_request_envelope());
        assert!(!envelope.is_valid());
        assert!(joined.contains("runtime metadata adapter is not committable"));
        assert!(parity.can_use_backend_wire_request());
        assert_eq!(parity.planning_pre_request_problem_count, 0);
        assert!(!gate.request_accepted);
        assert!(!gate.envelope_consistent);
        assert!(gate.planning_consistent);
        assert_eq!(gate.request_violation_count, 1);
        assert_eq!(gate.backend_wire_problem_count, 0);
        assert!(readiness.runtime_planning_ready());
        assert!(readiness.request_planning_ready());
        assert!(!readiness.request_gate_ready());
        assert_eq!(
            readiness.first_blocking_stage(),
            Some(RuntimeRequestPlanningReadinessStage::RequestGate)
        );
        assert_eq!(readiness.runtime_planning_blocker_component_count, 0);
        assert_eq!(readiness.request_planning_blocker_component_count, 0);
        assert!(readiness.request_gate_blocker_component_count > 0);
        assert!(readiness.runtime_request_planning_accounting_is_consistent());
        assert!(!readiness.can_commit_runtime_request_planning());
    }

    #[test]
    fn request_envelope_blocks_stale_kv_import_after_runtime_metadata_clamp() {
        let runtime = RuntimeMetadata::new("model", "tok", 128, 16)
            .with_kv_exchange(true, true)
            .with_kv_limits(1, 4);
        let request = InferenceRequest::new("hello", TaskProfile::Coding)
            .with_prompt_tokens(120)
            .with_max_tokens(32)
            .with_runtime(runtime)
            .with_experiments(ExperimentSwitches::default().with_fht_dke(true));
        let architecture = TransformerRuntimeArchitecture::new(2, 16, 4, 2, 64);
        let transformer_plan = TransformerPlanDigest::new(
            Some("metadata-clamp"),
            vec![
                TransformerLayerBudget::new(0, TransformerAttentionKind::Global, 0.8, 64),
                TransformerLayerBudget::new(1, TransformerAttentionKind::LocalWindow, 0.6, 32),
            ],
        );
        let execution =
            AdapterExecutionContext::new([RuntimeAdapter::CpuSimd, RuntimeAdapter::Cuda])
                .with_pressure(0.35, 0.60)
                .with_kv_prefetch_blocks(4);
        let route_budget = RouteBudget {
            threshold: 0.50,
            attention_tokens: 8,
            fast_tokens: 2,
            attention_fraction: 0.80,
        };
        let planning = RuntimePlanningDigest::from_request(
            &request,
            route_budget,
            &execution,
            &[],
            &DeterministicFhtDkeBudgeter::new(0.10, 0.60, 64),
        );
        let planned_kv = planning.planned_kv_exchange();

        let mut envelope = RuntimeRequestEnvelope::from_parts(
            &request,
            architecture,
            route_budget,
            HierarchyWeights::for_profile(TaskProfile::Coding),
            &transformer_plan,
            &execution,
            execution.kv_prefetch_blocks,
        )
        .with_planning_digest(planning);
        envelope.kv_prefetch_blocks = execution.kv_prefetch_blocks;
        let imported = (0..execution.kv_prefetch_blocks)
            .map(|index| {
                KvBlock::new(
                    index as u64,
                    crate::kv::KvNamespace::Runtime,
                    0,
                    0,
                    index..index + 1,
                    vec![0.1],
                    vec![0.2],
                )
            })
            .collect::<Vec<_>>();

        let joined = envelope.contract_violations().join("\n");
        let parity = envelope.planning_parity_summary();
        let gate = envelope.request_gate_summary(&imported);
        let readiness = envelope
            .request_planning_readiness_summary(clean_runtime_planning_readiness(), &imported);

        assert_eq!(planning.runtime_kv_prefetch_blocks, 1);
        assert_eq!(planned_kv.import_blocks, 1);
        assert!(planning.kv_prefetch_was_clamped());
        assert!(
            planning
                .kv_prefetch_clamp_summary()
                .has_runtime_metadata_clamp()
        );
        assert!(
            planning
                .planning_summary()
                .pre_request_gate_shape_is_clean()
        );
        assert!(joined.contains("imports 4 KV blocks above runtime limit 1"));
        assert!(joined.contains("prefetches 4 KV blocks above runtime limit 1"));
        assert!(joined.contains("imported KV count 4 differs from planned KV imports 1"));
        assert!(joined.contains("KV prefetch 4 differs from planned KV imports 1"));
        assert!(parity.planning_attached());
        assert_eq!(parity.planned_import_blocks, Some(planned_kv.import_blocks));
        assert_eq!(parity.imported_kv_matches_planning, Some(false));
        assert_eq!(parity.kv_prefetch_matches_planning, Some(false));
        assert!(parity.imported_kv_drifted_from_planning());
        assert!(parity.kv_prefetch_drifted_from_planning());
        assert_eq!(parity.kv_drift_component_count(), 2);
        assert!(!parity.request_matches_planning());
        assert!(!parity.can_use_backend_wire_request());
        assert!(!gate.request_accepted);
        assert!(!gate.envelope_consistent);
        assert!(gate.planning_attached);
        assert!(!gate.planning_consistent);
        assert!(gate.has_request_contract_failures());
        assert!(!gate.has_imported_kv_failures());
        assert!(gate.has_backend_wire_problem_components());
        assert!(!gate.can_commit_runtime_request());
        assert!(readiness.runtime_planning_ready());
        assert!(!readiness.request_planning_ready());
        assert!(!readiness.request_gate_ready());
        assert_eq!(
            readiness.first_blocking_stage(),
            Some(RuntimeRequestPlanningReadinessStage::RequestPlanningParity)
        );
        assert!(!readiness.can_commit_runtime_request_planning());
    }

    #[test]
    fn request_manifest_planning_readiness_confirms_manifest_bridge_before_request_gate() {
        let runtime = RuntimeMetadata::new("model", "tok", 1024, 128)
            .with_kv_exchange(true, true)
            .with_kv_limits(2, 4);
        let request = InferenceRequest::new("hello", TaskProfile::Coding)
            .with_prompt_tokens(900)
            .with_max_tokens(256)
            .with_runtime(runtime.clone())
            .with_experiments(ExperimentSwitches::default().with_fht_dke(true));
        let architecture = TransformerRuntimeArchitecture::new(6, 128, 4, 2, 256);
        let transformer_plan = TransformerPlanDigest::new(
            Some("manifest-request"),
            (0..6)
                .map(|layer| {
                    TransformerLayerBudget::new(layer, TransformerAttentionKind::Global, 0.8, 64)
                })
                .collect(),
        );
        let execution =
            AdapterExecutionContext::new([RuntimeAdapter::Cuda, RuntimeAdapter::CpuSimd])
                .with_pressure(0.70, 0.30)
                .with_parallel_chunks(2)
                .with_kv_prefetch_blocks(8);
        let observations = [
            crate::adapter::AdapterObservation::new(
                RuntimeAdapter::CpuSimd,
                0.40,
                0.5,
                0.5,
                None,
                None,
                7,
            ),
            crate::adapter::AdapterObservation::new(
                RuntimeAdapter::Cuda,
                0.90,
                0.8,
                0.9,
                None,
                None,
                8,
            ),
        ];
        let planning = RuntimePlanningDigest::from_request(
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
        let planned_kv = planning.planned_kv_exchange();
        let manifest = RuntimeManifestDigest::from_metadata(runtime)
            .with_architecture(architecture)
            .with_kv_policy(
                RuntimeKvPolicy::import_export()
                    .with_limits(planned_kv.import_blocks, planned_kv.export_blocks),
            );
        let envelope = RuntimeRequestEnvelope::from_parts(
            &request,
            architecture,
            RouteBudget::default(),
            HierarchyWeights::for_profile(TaskProfile::Coding),
            &transformer_plan,
            &execution,
            planned_kv.import_blocks,
        )
        .with_planning_digest(planning);
        let imported = (0..planned_kv.import_blocks)
            .map(|index| {
                KvBlock::new(
                    index as u64,
                    crate::kv::KvNamespace::Runtime,
                    index % architecture.layer_count,
                    index % architecture.kv_heads,
                    index..index + 1,
                    vec![0.1; 128],
                    vec![0.2; 128],
                )
            })
            .collect::<Vec<_>>();
        let runtime_planning = clean_runtime_planning_readiness();
        let request_planning =
            envelope.request_planning_readiness_summary(runtime_planning, &imported);
        let manifest_bridge = planning.manifest_kv_bridge_summary(&manifest);
        let readiness = envelope
            .manifest_request_planning_readiness_summary(runtime_planning, &manifest, &imported)
            .expect("planning digest is attached");

        assert_eq!(
            readiness,
            RuntimeRequestManifestPlanningReadinessSummary::new(manifest_bridge, request_planning)
        );
        assert_eq!(
            RuntimeRequestManifestPlanningReadinessSummary::stage_order(),
            [
                RuntimeRequestManifestPlanningReadinessStage::ManifestKvBridge,
                RuntimeRequestManifestPlanningReadinessStage::RequestPlanning,
            ]
        );
        assert!(readiness.manifest_kv_bridge_ready());
        assert!(readiness.request_planning_ready());
        assert_eq!(readiness.first_unready_stage(), None);
        assert_eq!(readiness.first_blocking_stage(), None);
        assert_eq!(
            readiness.stage_signal_component_count(
                RuntimeRequestManifestPlanningReadinessStage::ManifestKvBridge
            ),
            readiness.manifest_kv_bridge_signal_component_count
        );
        assert_eq!(
            readiness.stage_blocker_component_count(
                RuntimeRequestManifestPlanningReadinessStage::RequestPlanning
            ),
            readiness.request_planning_blocker_component_count
        );
        assert_eq!(
            readiness.manifest_kv_bridge_signal_component_count,
            manifest_bridge.manifest_kv_bridge_signal_component_count()
        );
        assert_eq!(
            readiness.request_planning_signal_component_count,
            request_planning.runtime_request_planning_signal_component_count()
        );
        assert_eq!(readiness.manifest_kv_bridge_blocker_component_count, 0);
        assert_eq!(readiness.request_planning_blocker_component_count, 0);
        assert!(readiness.has_manifest_request_planning_signals());
        assert!(!readiness.has_manifest_request_planning_blockers());
        assert_eq!(
            readiness.manifest_request_planning_blocker_component_count(),
            0
        );
        assert!(readiness.manifest_request_planning_accounting_is_consistent());
        assert!(readiness.manifest_request_planning_is_clean());
        assert!(readiness.can_commit_manifest_request_planning());
    }

    #[test]
    fn request_manifest_planning_readiness_preserves_router_budget_kv_degrade() {
        let router = DefaultHierarchicalRouter::new();
        let tokens = [TokenFeatures::new("borderline", 0.66, 0)];
        let routing_context = RoutingContext {
            hierarchy: HierarchyWeights::new(1.0, 0.0, 0.0),
            ..RoutingContext::default()
        };
        let decisions = router.route_many(&tokens, routing_context);
        let route_budget = router.budget(&tokens, routing_context);
        let runtime = RuntimeMetadata::new("model", "tok", 2048, 128)
            .with_kv_exchange(true, true)
            .with_kv_limits(16, 16);
        let request = InferenceRequest::new("hello", TaskProfile::General)
            .with_prompt_tokens(512)
            .with_max_tokens(128)
            .with_runtime(runtime.clone())
            .with_experiments(ExperimentSwitches::default().with_fht_dke(true));
        let architecture = TransformerRuntimeArchitecture::new(4, 128, 4, 2, 256);
        let transformer_plan = TransformerPlanDigest::new(
            Some("router-manifest-request"),
            (0..4)
                .map(|layer| {
                    TransformerLayerBudget::new(layer, TransformerAttentionKind::Global, 0.8, 64)
                })
                .collect(),
        );
        let execution =
            AdapterExecutionContext::new([RuntimeAdapter::Cuda]).with_kv_prefetch_blocks(8);
        let planning = RuntimePlanningDigest::from_request(
            &request,
            route_budget,
            &execution,
            &[],
            &DeterministicFhtDkeBudgeter::new(0.10, 0.60, 128),
        );
        let planned_kv = planning.planned_kv_exchange();
        let manifest = RuntimeManifestDigest::from_metadata(runtime)
            .with_architecture(architecture)
            .with_kv_policy(
                RuntimeKvPolicy::import_export()
                    .with_limits(planned_kv.import_blocks, planned_kv.export_blocks),
            );
        let envelope = RuntimeRequestEnvelope::from_parts(
            &request,
            architecture,
            route_budget,
            HierarchyWeights::for_profile(TaskProfile::General),
            &transformer_plan,
            &execution,
            planned_kv.import_blocks,
        )
        .with_planning_digest(planning);
        let imported = (0..planned_kv.import_blocks)
            .map(|index| {
                KvBlock::new(
                    index as u64,
                    crate::kv::KvNamespace::Runtime,
                    index % architecture.layer_count,
                    index % architecture.kv_heads,
                    index..index + 1,
                    vec![0.1; 128],
                    vec![0.2; 128],
                )
            })
            .collect::<Vec<_>>();

        let readiness = envelope
            .manifest_request_planning_readiness_summary(
                clean_runtime_planning_readiness(),
                &manifest,
                &imported,
            )
            .expect("planning digest is attached");
        let envelope_summary = envelope.envelope_summary();

        assert_eq!(decisions[0].layer, RouteLayer::LocalWindow);
        assert_eq!(route_budget.fast_tokens, 0);
        assert_eq!(route_budget.attention_tokens, 1);
        assert_eq!(route_budget.attention_fraction, 1.0);
        assert_eq!(envelope.route_budget, route_budget);
        assert!(planning.fht_dke_summary().route_pressure_is_high());
        assert!(planning.kv_prefetch_was_clamped());
        assert_eq!(
            planning.kv_prefetch_clamp_reason(),
            RuntimePlanningKvClampReason::FhtDkeBudgetLimit
        );
        assert_eq!(planned_kv.import_blocks, imported.len());
        assert!(planned_kv.import_blocks < planning.requested_kv_prefetch_blocks);
        assert_eq!(envelope.imported_kv_blocks, planned_kv.import_blocks);
        assert_eq!(envelope.kv_prefetch_blocks, planned_kv.import_blocks);
        assert!(envelope_summary.can_commit_runtime_request_envelope());
        assert!(readiness.manifest_kv_bridge_ready());
        assert!(readiness.request_planning_ready());
        assert_eq!(readiness.first_unready_stage(), None);
        assert_eq!(readiness.first_blocking_stage(), None);
        assert!(readiness.manifest_request_planning_is_clean());
        assert!(readiness.can_commit_manifest_request_planning());
    }

    #[test]
    fn request_manifest_planning_readiness_commits_runtime_and_fht_dke_kv_prefetch_limits() {
        let route_budget = RouteBudget {
            threshold: 0.45,
            attention_tokens: 9,
            fast_tokens: 1,
            attention_fraction: 0.90,
        };
        let runtime = RuntimeMetadata::new("model", "tok", 4096, 1024)
            .with_kv_exchange(true, true)
            .with_kv_limits(6, 6);
        let request = InferenceRequest::new("hello", TaskProfile::Coding)
            .with_prompt_tokens(1024)
            .with_max_tokens(1024)
            .with_runtime(runtime.clone())
            .with_experiments(ExperimentSwitches::default().with_fht_dke(true));
        let architecture = TransformerRuntimeArchitecture::new(4, 128, 4, 2, 256);
        let transformer_plan = TransformerPlanDigest::new(
            Some("runtime-fht-dke-kv-prefetch-limits"),
            (0..4)
                .map(|layer| {
                    TransformerLayerBudget::new(layer, TransformerAttentionKind::Global, 0.8, 64)
                })
                .collect(),
        );
        let execution =
            AdapterExecutionContext::new([RuntimeAdapter::Cuda]).with_kv_prefetch_blocks(8);
        let planning = RuntimePlanningDigest::from_request(
            &request,
            route_budget,
            &execution,
            &[],
            &DeterministicFhtDkeBudgeter::new(0.10, 0.60, 1024),
        );
        let planned_kv = planning.planned_kv_exchange();
        let manifest = RuntimeManifestDigest::from_metadata(runtime)
            .with_architecture(architecture)
            .with_kv_policy(
                RuntimeKvPolicy::import_export()
                    .with_limits(planned_kv.import_blocks, planned_kv.export_blocks),
            );
        let envelope = RuntimeRequestEnvelope::from_parts(
            &request,
            architecture,
            route_budget,
            HierarchyWeights::for_profile(TaskProfile::Coding),
            &transformer_plan,
            &execution,
            planned_kv.import_blocks,
        )
        .with_planning_digest(planning);
        let imported = (0..planned_kv.import_blocks)
            .map(|index| {
                KvBlock::new(
                    index as u64,
                    crate::kv::KvNamespace::Runtime,
                    index % architecture.layer_count,
                    index % architecture.kv_heads,
                    index..index + 1,
                    vec![0.1; 128],
                    vec![0.2; 128],
                )
            })
            .collect::<Vec<_>>();

        let clamp = planning.kv_prefetch_clamp_summary();
        let envelope_summary = envelope.envelope_summary();
        let parity = envelope.planning_parity_summary();
        let gate = envelope.request_gate_summary(&imported);
        let readiness = envelope
            .manifest_request_planning_readiness_summary(
                clean_runtime_planning_readiness(),
                &manifest,
                &imported,
            )
            .expect("planning digest is attached");

        assert_eq!(planning.requested_kv_prefetch_blocks, 8);
        assert_eq!(planning.runtime_kv_prefetch_blocks, 6);
        assert_eq!(planning.fht_dke_budget.kv_import_blocks, 2);
        assert_eq!(planned_kv.import_blocks, 2);
        assert_eq!(planned_kv.export_blocks, 2);
        assert_eq!(
            planning.kv_prefetch_clamp_reason(),
            RuntimePlanningKvClampReason::RuntimeAndFhtDkeLimits
        );
        assert!(clamp.has_runtime_metadata_clamp());
        assert!(clamp.has_fht_dke_clamp());
        assert!(clamp.clamped_by_runtime_and_fht_dke());
        assert_eq!(envelope.imported_kv_blocks, planned_kv.import_blocks);
        assert_eq!(envelope.kv_prefetch_blocks, planned_kv.import_blocks);
        assert!(envelope_summary.can_commit_runtime_request_envelope());
        assert!(parity.planning_attached());
        assert_eq!(parity.imported_kv_matches_planning, Some(true));
        assert_eq!(parity.kv_prefetch_matches_planning, Some(true));
        assert!(!parity.imported_kv_drifted_from_planning());
        assert!(!parity.kv_prefetch_drifted_from_planning());
        assert_eq!(parity.kv_drift_component_count(), 0);
        assert!(parity.can_use_backend_wire_request());
        assert!(gate.planning_consistent);
        assert_eq!(gate.backend_wire_problem_count, 0);
        assert!(gate.can_commit_runtime_request());
        assert!(readiness.manifest_kv_bridge_ready());
        assert!(readiness.request_planning_ready());
        assert_eq!(readiness.first_unready_stage(), None);
        assert_eq!(readiness.first_blocking_stage(), None);
        assert!(readiness.manifest_request_planning_accounting_is_consistent());
        assert!(readiness.manifest_request_planning_is_clean());
        assert!(readiness.can_commit_manifest_request_planning());
    }

    #[test]
    fn request_envelope_blocks_fusion_skip_pressure_as_imported_kv_commit() {
        let route_budget = RouteBudget {
            threshold: 0.45,
            attention_tokens: 9,
            fast_tokens: 1,
            attention_fraction: 0.90,
        };
        let runtime = RuntimeMetadata::new("model", "tok", 4096, 1024)
            .with_kv_exchange(true, true)
            .with_kv_limits(6, 6);
        let request = InferenceRequest::new("hello", TaskProfile::Coding)
            .with_prompt_tokens(1024)
            .with_max_tokens(1024)
            .with_runtime(runtime.clone())
            .with_experiments(ExperimentSwitches::default().with_fht_dke(true));
        let architecture = TransformerRuntimeArchitecture::new(4, 128, 4, 2, 256);
        let transformer_plan = TransformerPlanDigest::new(
            Some("fusion-skip-request-boundary"),
            (0..4)
                .map(|layer| {
                    TransformerLayerBudget::new(layer, TransformerAttentionKind::Global, 0.8, 64)
                })
                .collect(),
        );
        let execution =
            AdapterExecutionContext::new([RuntimeAdapter::Cuda]).with_kv_prefetch_blocks(8);
        let planning = RuntimePlanningDigest::from_request(
            &request,
            route_budget,
            &execution,
            &[],
            &DeterministicFhtDkeBudgeter::new(0.10, 0.60, 1024),
        );
        let planned_kv = planning.planned_kv_exchange();
        let fusion_existing = vec![KvBlock::new(
            10,
            crate::kv::KvNamespace::Runtime,
            0,
            0,
            0..4,
            vec![1.0, 0.0],
            vec![1.0, 0.0],
        )];
        let fusion_incoming = vec![
            KvBlock::new(
                11,
                crate::kv::KvNamespace::Runtime,
                0,
                0,
                0..4,
                vec![1.0, 0.0],
                vec![1.0, 0.0],
            ),
            KvBlock::new(
                12,
                crate::kv::KvNamespace::Runtime,
                0,
                1,
                4..8,
                vec![0.0, 1.0],
                vec![0.0, 1.0],
            ),
        ];
        let fusion =
            ReinforcedKvFusionPolicy::new(0.92, 1).fuse(&fusion_existing, &fusion_incoming);
        let fusion_summary = fusion.merge_summary();
        let fusion_commit = fusion_summary.commit_summary();
        let imported = (0..planned_kv.import_blocks)
            .map(|index| {
                KvBlock::new(
                    index as u64,
                    crate::kv::KvNamespace::Runtime,
                    index % architecture.layer_count,
                    index % architecture.kv_heads,
                    index..index + 1,
                    vec![0.1; 128],
                    vec![0.2; 128],
                )
            })
            .collect::<Vec<_>>();
        let envelope = RuntimeRequestEnvelope::from_parts(
            &request,
            architecture,
            route_budget,
            HierarchyWeights::for_profile(TaskProfile::Coding),
            &transformer_plan,
            &execution,
            planned_kv.import_blocks,
        )
        .with_planning_digest(planning);
        let stale_envelope = RuntimeRequestEnvelope::from_parts(
            &request,
            architecture,
            route_budget,
            HierarchyWeights::for_profile(TaskProfile::Coding),
            &transformer_plan,
            &execution,
            planned_kv.import_blocks + fusion_summary.skipped_count,
        )
        .with_planning_digest(planning);
        let clean_parity = envelope.planning_parity_summary();
        let clean_gate = envelope.request_gate_summary(&imported);
        let stale_parity = stale_envelope.planning_parity_summary();
        let stale_gate = stale_envelope.request_gate_summary(&imported);

        assert_eq!(planning.requested_kv_prefetch_blocks, 8);
        assert_eq!(planning.runtime_kv_prefetch_blocks, 6);
        assert_eq!(planned_kv.import_blocks, 2);
        assert_eq!(
            planning.kv_prefetch_clamp_reason(),
            RuntimePlanningKvClampReason::RuntimeAndFhtDkeLimits
        );
        assert_eq!(fusion.skipped, 1);
        assert_eq!(fusion_summary.skipped_count, 1);
        assert!(fusion_summary.has_skips());
        assert!(fusion_summary.changed_due_to_skips());
        assert_eq!(fusion_summary.runtime_block_count, 1);
        assert!(fusion_summary.can_commit_kv_fusion_persistence());
        assert!(fusion_commit.can_commit_kv_fusion_persistence());
        assert_eq!(envelope.imported_kv_blocks, planned_kv.import_blocks);
        assert_eq!(envelope.kv_prefetch_blocks, planned_kv.import_blocks);
        assert_eq!(clean_parity.imported_kv_matches_planning, Some(true));
        assert_eq!(clean_parity.kv_prefetch_matches_planning, Some(true));
        assert_eq!(clean_parity.kv_drift_component_count(), 0);
        assert!(clean_parity.can_use_backend_wire_request());
        assert_eq!(clean_gate.backend_wire_problem_count, 0);
        assert!(clean_gate.can_commit_runtime_request());
        assert_eq!(
            stale_envelope.imported_kv_blocks,
            planned_kv.import_blocks + fusion_summary.skipped_count
        );
        assert_eq!(stale_envelope.kv_prefetch_blocks, planned_kv.import_blocks);
        assert_eq!(stale_parity.imported_kv_matches_planning, Some(false));
        assert_eq!(stale_parity.kv_prefetch_matches_planning, Some(true));
        assert_eq!(stale_parity.kv_drift_component_count(), 1);
        assert!(!stale_parity.can_use_backend_wire_request());
        assert!(!stale_gate.planning_consistent);
        assert_eq!(stale_gate.backend_wire_problem_count, 1);
        assert!(!stale_gate.can_commit_runtime_request());
    }

    #[test]
    fn request_manifest_planning_readiness_blocks_uncommitted_kv_degrade_drift() {
        let router = DefaultHierarchicalRouter::new();
        let tokens = [TokenFeatures::new("borderline", 0.66, 0)];
        let routing_context = RoutingContext {
            hierarchy: HierarchyWeights::new(1.0, 0.0, 0.0),
            ..RoutingContext::default()
        };
        let route_budget = router.budget(&tokens, routing_context);
        let runtime = RuntimeMetadata::new("model", "tok", 2048, 128)
            .with_kv_exchange(true, true)
            .with_kv_limits(16, 16);
        let request = InferenceRequest::new("hello", TaskProfile::General)
            .with_prompt_tokens(512)
            .with_max_tokens(128)
            .with_runtime(runtime.clone())
            .with_experiments(ExperimentSwitches::default().with_fht_dke(true));
        let architecture = TransformerRuntimeArchitecture::new(4, 128, 4, 2, 256);
        let transformer_plan = TransformerPlanDigest::new(
            Some("router-manifest-request-drift"),
            (0..4)
                .map(|layer| {
                    TransformerLayerBudget::new(layer, TransformerAttentionKind::Global, 0.8, 64)
                })
                .collect(),
        );
        let execution =
            AdapterExecutionContext::new([RuntimeAdapter::Cuda]).with_kv_prefetch_blocks(8);
        let planning = RuntimePlanningDigest::from_request(
            &request,
            route_budget,
            &execution,
            &[],
            &DeterministicFhtDkeBudgeter::new(0.10, 0.60, 128),
        );
        let planned_kv = planning.planned_kv_exchange();
        let manifest = RuntimeManifestDigest::from_metadata(runtime)
            .with_architecture(architecture)
            .with_kv_policy(
                RuntimeKvPolicy::import_export()
                    .with_limits(planned_kv.import_blocks, planned_kv.export_blocks),
            );
        let envelope = RuntimeRequestEnvelope::from_parts(
            &request,
            architecture,
            route_budget,
            HierarchyWeights::for_profile(TaskProfile::General),
            &transformer_plan,
            &execution,
            execution.kv_prefetch_blocks,
        )
        .with_planning_digest(planning);
        let imported = (0..planned_kv.import_blocks)
            .map(|index| {
                KvBlock::new(
                    index as u64,
                    crate::kv::KvNamespace::Runtime,
                    index % architecture.layer_count,
                    index % architecture.kv_heads,
                    index..index + 1,
                    vec![0.1; 128],
                    vec![0.2; 128],
                )
            })
            .collect::<Vec<_>>();

        let envelope_summary = envelope.envelope_summary();
        let parity = envelope.planning_parity_summary();
        let gate = envelope.request_gate_summary(&imported);
        let request_readiness = envelope
            .request_planning_readiness_summary(clean_runtime_planning_readiness(), &imported);
        let manifest_readiness = envelope
            .manifest_request_planning_readiness_summary(
                clean_runtime_planning_readiness(),
                &manifest,
                &imported,
            )
            .expect("planning digest is attached");

        assert_eq!(route_budget.fast_tokens, 0);
        assert_eq!(route_budget.attention_tokens, 1);
        assert_eq!(route_budget.attention_fraction, 1.0);
        assert!(planning.fht_dke_summary().route_pressure_is_high());
        assert!(planning.kv_prefetch_was_clamped());
        assert_eq!(
            planning.kv_prefetch_clamp_reason(),
            RuntimePlanningKvClampReason::FhtDkeBudgetLimit
        );
        assert!(planned_kv.import_blocks < execution.kv_prefetch_blocks);
        assert_eq!(envelope.imported_kv_blocks, execution.kv_prefetch_blocks);
        assert_eq!(envelope.kv_prefetch_blocks, planned_kv.import_blocks);
        assert!(envelope_summary.can_commit_runtime_request_envelope());
        assert!(parity.planning_attached());
        assert!(parity.imported_kv_drifted_from_planning());
        assert!(!parity.kv_prefetch_drifted_from_planning());
        assert_eq!(parity.imported_kv_drift_component_count(), 1);
        assert_eq!(parity.kv_prefetch_drift_component_count(), 0);
        assert_eq!(parity.backend_wire_problem_component_count(), 1);
        assert!(!parity.can_use_backend_wire_request());
        assert!(!gate.planning_consistent);
        assert_eq!(gate.backend_wire_problem_count, 1);
        assert_eq!(gate.imported_kv_violation_count, 0);
        assert!(gate.has_backend_wire_problem_components());
        assert!(gate.has_boundary_drift());
        assert!(!gate.can_commit_runtime_request());
        assert!(request_readiness.runtime_planning_ready());
        assert!(!request_readiness.request_planning_ready());
        assert!(!request_readiness.request_gate_ready());
        assert_eq!(
            request_readiness.first_unready_stage(),
            Some(RuntimeRequestPlanningReadinessStage::RequestPlanningParity)
        );
        assert_eq!(
            request_readiness.first_blocking_stage(),
            Some(RuntimeRequestPlanningReadinessStage::RequestPlanningParity)
        );
        assert!(!request_readiness.can_commit_runtime_request_planning());
        assert!(manifest_readiness.manifest_kv_bridge_ready());
        assert!(!manifest_readiness.request_planning_ready());
        assert_eq!(
            manifest_readiness.first_unready_stage(),
            Some(RuntimeRequestManifestPlanningReadinessStage::RequestPlanning)
        );
        assert!(!manifest_readiness.can_commit_manifest_request_planning());
    }

    #[test]
    fn request_manifest_planning_readiness_blocks_manifest_bridge_drift() {
        let runtime = RuntimeMetadata::new("model", "tok", 1024, 128)
            .with_kv_exchange(true, true)
            .with_kv_limits(2, 4);
        let request = InferenceRequest::new("hello", TaskProfile::Coding)
            .with_prompt_tokens(900)
            .with_max_tokens(256)
            .with_runtime(runtime.clone())
            .with_experiments(ExperimentSwitches::default().with_fht_dke(true));
        let architecture = TransformerRuntimeArchitecture::new(6, 128, 4, 2, 256);
        let transformer_plan = TransformerPlanDigest::new(
            Some("manifest-request-drift"),
            (0..6)
                .map(|layer| {
                    TransformerLayerBudget::new(layer, TransformerAttentionKind::Global, 0.8, 64)
                })
                .collect(),
        );
        let execution =
            AdapterExecutionContext::new([RuntimeAdapter::Cuda, RuntimeAdapter::CpuSimd])
                .with_pressure(0.70, 0.30)
                .with_parallel_chunks(2)
                .with_kv_prefetch_blocks(8);
        let observations = [
            crate::adapter::AdapterObservation::new(
                RuntimeAdapter::CpuSimd,
                0.40,
                0.5,
                0.5,
                None,
                None,
                7,
            ),
            crate::adapter::AdapterObservation::new(
                RuntimeAdapter::Cuda,
                0.90,
                0.8,
                0.9,
                None,
                None,
                8,
            ),
        ];
        let planning = RuntimePlanningDigest::from_request(
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
        let planned_kv = planning.planned_kv_exchange();
        let manifest = RuntimeManifestDigest::from_metadata(runtime)
            .with_architecture(architecture)
            .with_kv_policy(RuntimeKvPolicy {
                import_enabled: true,
                export_enabled: true,
                max_import_blocks: 1,
                max_export_blocks: 0,
            });
        let envelope = RuntimeRequestEnvelope::from_parts(
            &request,
            architecture,
            RouteBudget::default(),
            HierarchyWeights::for_profile(TaskProfile::Coding),
            &transformer_plan,
            &execution,
            planned_kv.import_blocks,
        )
        .with_planning_digest(planning);
        let imported = (0..planned_kv.import_blocks)
            .map(|index| {
                KvBlock::new(
                    index as u64,
                    crate::kv::KvNamespace::Runtime,
                    index % architecture.layer_count,
                    index % architecture.kv_heads,
                    index..index + 1,
                    vec![0.1; 128],
                    vec![0.2; 128],
                )
            })
            .collect::<Vec<_>>();
        let readiness = envelope
            .manifest_request_planning_readiness_summary(
                clean_runtime_planning_readiness(),
                &manifest,
                &imported,
            )
            .expect("planning digest is attached");

        assert!(!readiness.manifest_kv_bridge_ready());
        assert!(readiness.request_planning_ready());
        assert_eq!(
            readiness.first_unready_stage(),
            Some(RuntimeRequestManifestPlanningReadinessStage::ManifestKvBridge)
        );
        assert_eq!(
            readiness.first_blocking_stage(),
            Some(RuntimeRequestManifestPlanningReadinessStage::ManifestKvBridge)
        );
        assert_eq!(readiness.manifest_kv_bridge_blocker_component_count, 4);
        assert_eq!(readiness.request_planning_blocker_component_count, 0);
        assert_eq!(
            readiness.manifest_request_planning_blocker_component_count(),
            4
        );
        assert!(readiness.has_manifest_request_planning_blockers());
        assert!(readiness.manifest_request_planning_accounting_is_consistent());
        assert!(!readiness.manifest_request_planning_is_clean());
        assert!(!readiness.can_commit_manifest_request_planning());
    }

    #[test]
    fn request_envelope_summary_reports_adapter_context_and_kv_limits() {
        let runtime = RuntimeMetadata::new("model", "tok", 128, 16)
            .with_kv_exchange(true, true)
            .with_kv_limits(2, 3);
        let request = InferenceRequest::new("hello", TaskProfile::Coding)
            .with_prompt_tokens(32)
            .with_max_tokens(16)
            .with_runtime(runtime);
        let architecture = TransformerRuntimeArchitecture::new(2, 16, 4, 2, 64);
        let transformer_plan = TransformerPlanDigest::new(
            Some("summary"),
            vec![
                TransformerLayerBudget::new(0, TransformerAttentionKind::Global, 0.8, 64),
                TransformerLayerBudget::new(1, TransformerAttentionKind::LocalWindow, 0.6, 32),
            ],
        );
        let execution = AdapterExecutionContext::new([RuntimeAdapter::Cuda])
            .with_pressure(0.76, 0.40)
            .with_kv_prefetch_blocks(2);
        let envelope = RuntimeRequestEnvelope::from_parts(
            &request,
            architecture,
            RouteBudget::default(),
            HierarchyWeights::for_profile(TaskProfile::Coding),
            &transformer_plan,
            &execution,
            2,
        );

        let summary = envelope.envelope_summary();

        assert_eq!(summary.schema, RUNTIME_REQUEST_SCHEMA);
        assert_eq!(summary.profile, TaskProfile::Coding);
        assert_eq!(summary.prompt_chars, 5);
        assert_eq!(summary.requested_max_tokens, 16);
        assert_eq!(summary.max_generated_tokens, 16);
        assert_eq!(summary.planned_context_tokens, 48);
        assert!(summary.can_generate);
        assert!(!summary.context_limited_generation());
        assert_eq!(summary.model_context_window, 128);
        assert!(summary.has_kv_exchange_capacity());
        assert_eq!(summary.max_kv_import_blocks, 2);
        assert_eq!(summary.max_kv_export_blocks, 3);
        assert_eq!(summary.architecture_layer_count, 2);
        assert_eq!(summary.architecture_hidden_size, 16);
        assert!(summary.transformer_layers_match_architecture());
        assert!(summary.has_adapter_candidates());
        assert!(summary.has_selected_adapter());
        assert_eq!(summary.selected_adapter, Some(RuntimeAdapter::Cuda));
        assert_eq!(
            summary.hardware_pressure_band,
            DiagnosticsPressureBand::High
        );
        assert!(summary.has_kv_import_pressure());
        assert!(!summary.kv_imports_exceed_runtime_limit());
        assert!(!summary.kv_prefetch_exceeds_runtime_limit());
        assert!(!summary.planning_attached());
        assert!(!summary.recursive_attached());
        assert_eq!(summary.request_envelope_commit_signal_component_count(), 5);
        assert!(summary.has_request_envelope_commit_signals());
        assert_eq!(summary.request_envelope_commit_blocker_component_count(), 0);
        assert!(!summary.has_request_envelope_commit_blockers());
        assert!(summary.request_envelope_commit_accounting_is_consistent());
        assert!(summary.request_envelope_commit_is_clean());
        assert!(summary.request_envelope_shape_is_clean());
        assert!(summary.can_commit_runtime_request_envelope());
        assert!(summary.can_use_runtime_request_envelope());
    }

    #[test]
    fn request_envelope_blocks_commit_when_zero_requested_tokens_exhaust_context() {
        let runtime = RuntimeMetadata::new("model", "tok", 128, 16)
            .with_kv_exchange(true, true)
            .with_kv_limits(2, 3);
        let request = InferenceRequest::new("hello", TaskProfile::Coding)
            .with_prompt_tokens(128)
            .with_max_tokens(0)
            .with_runtime(runtime);
        let architecture = TransformerRuntimeArchitecture::new(2, 16, 4, 2, 64);
        let transformer_plan = TransformerPlanDigest::new(
            Some("context-exhausted"),
            vec![
                TransformerLayerBudget::new(0, TransformerAttentionKind::Global, 0.8, 64),
                TransformerLayerBudget::new(1, TransformerAttentionKind::LocalWindow, 0.6, 32),
            ],
        );
        let execution = AdapterExecutionContext::new([RuntimeAdapter::Cuda]);
        let envelope = RuntimeRequestEnvelope::from_parts(
            &request,
            architecture,
            RouteBudget::default(),
            HierarchyWeights::for_profile(TaskProfile::Coding),
            &transformer_plan,
            &execution,
            0,
        );

        let summary = envelope.envelope_summary();
        let joined = envelope.contract_violations().join("\n");

        assert_eq!(summary.requested_max_tokens, 1);
        assert_eq!(summary.max_tokens, 1);
        assert_eq!(summary.max_generated_tokens, 0);
        assert_eq!(summary.planned_context_tokens, 128);
        assert!(summary.truncated_by_context);
        assert!(!summary.can_generate);
        assert!(summary.context_limited_generation());
        assert_eq!(summary.request_envelope_commit_signal_component_count(), 4);
        assert_eq!(summary.request_envelope_commit_blocker_component_count(), 0);
        assert!(!summary.has_request_envelope_commit_blockers());
        assert!(summary.request_envelope_commit_accounting_is_consistent());
        assert!(summary.request_envelope_commit_is_clean());
        assert!(summary.request_envelope_shape_is_clean());
        assert!(!summary.can_commit_runtime_request_envelope());
        assert!(!summary.can_use_runtime_request_envelope());
        assert!(joined.contains("runtime request cannot generate"));
    }

    #[test]
    fn request_envelope_summary_reports_planning_recursive_and_context_pressure() {
        let runtime = RuntimeMetadata::new("model", "tok", 128, 16)
            .with_kv_exchange(true, true)
            .with_kv_limits(2, 4);
        let request = InferenceRequest::new("hello", TaskProfile::Coding)
            .with_prompt_tokens(120)
            .with_max_tokens(32)
            .with_runtime(runtime)
            .with_experiments(ExperimentSwitches::default().with_fht_dke(true));
        let architecture = TransformerRuntimeArchitecture::new(1, 16, 2, 1, 64);
        let transformer_plan = TransformerPlanDigest::new(
            Some("summary-planning"),
            vec![TransformerLayerBudget::new(
                0,
                TransformerAttentionKind::Global,
                0.5,
                64,
            )],
        );
        let execution = AdapterExecutionContext::new([RuntimeAdapter::CpuSimd])
            .with_pressure(0.35, 0.60)
            .with_kv_prefetch_blocks(4);
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
        let recursive = RecursiveSchedulerConfig::new(128, 64, 8, 2)
            .plan_tokens(120)
            .schedule_summary();
        let envelope = RuntimeRequestEnvelope::from_parts(
            &request,
            architecture,
            RouteBudget::default(),
            HierarchyWeights::default(),
            &transformer_plan,
            &execution,
            planning.planned_kv_exchange().import_blocks,
        )
        .with_planning_digest(planning)
        .with_recursive_schedule(recursive);

        let summary = envelope.envelope_summary();

        assert!(summary.context_limited_generation());
        assert_eq!(summary.max_tokens, planning.backend_max_tokens());
        assert_eq!(
            summary.planning_backend_max_tokens,
            Some(planning.backend_max_tokens())
        );
        assert_eq!(
            summary.planning_import_blocks,
            Some(planning.planned_kv_exchange().import_blocks)
        );
        assert_eq!(
            summary.planning_export_blocks,
            Some(planning.planned_kv_exchange().export_blocks)
        );
        assert!(summary.planning_attached());
        assert!(summary.recursive_attached());
        assert!(!summary.recursive_requires_recursion);
        assert_eq!(summary.kv_prefetch_blocks, summary.imported_kv_blocks);
        assert!(summary.transformer_layers_match_architecture());
        assert_eq!(summary.request_envelope_commit_signal_component_count(), 7);
        assert_eq!(summary.request_envelope_commit_blocker_component_count(), 0);
        assert!(!summary.has_request_envelope_commit_blockers());
        assert!(summary.request_envelope_commit_accounting_is_consistent());
        assert!(summary.request_envelope_commit_is_clean());
        assert!(summary.can_commit_runtime_request_envelope());
    }

    #[test]
    fn request_envelope_reports_planning_digest_mismatches() {
        let runtime = RuntimeMetadata::new("model", "tok", 128, 16)
            .with_kv_exchange(true, true)
            .with_kv_limits(2, 4);
        let request = InferenceRequest::new("hello", TaskProfile::Coding)
            .with_prompt_tokens(120)
            .with_max_tokens(32)
            .with_runtime(runtime);
        let architecture = TransformerRuntimeArchitecture::new(1, 16, 2, 1, 64);
        let transformer_plan = TransformerPlanDigest::new(
            Some("one"),
            vec![TransformerLayerBudget::new(
                0,
                TransformerAttentionKind::Global,
                0.5,
                64,
            )],
        );
        let execution =
            AdapterExecutionContext::new([RuntimeAdapter::Cuda]).with_kv_prefetch_blocks(2);
        let planning = RuntimePlanningDigest::from_request(
            &request,
            RouteBudget::default(),
            &execution,
            &[],
            &DeterministicFhtDkeBudgeter::default(),
        );
        let mut envelope = RuntimeRequestEnvelope::from_parts(
            &request,
            architecture,
            RouteBudget::default(),
            HierarchyWeights::default(),
            &transformer_plan,
            &execution,
            0,
        )
        .with_planning_digest(planning);
        envelope.max_tokens = request.max_tokens;
        envelope.selected_adapter = Some(RuntimeAdapter::PortableRust);
        envelope.imported_kv_blocks = planning.planned_kv_exchange().import_blocks + 1;
        envelope.kv_prefetch_blocks = planning.planned_kv_exchange().import_blocks + 1;

        let joined = envelope.contract_violations().join("\n");
        let parity = envelope.planning_parity_summary();
        let gate = envelope.request_gate_summary(&[]);

        assert!(joined.contains("max_tokens 32 differs from planned backend max_tokens"));
        assert!(joined.contains("selected adapter"));
        assert!(joined.contains("imported KV count"));
        assert!(joined.contains("KV prefetch"));
        assert!(parity.planning_attached());
        assert_eq!(parity.max_tokens_match_planning, Some(false));
        assert_eq!(parity.generation_budget_matches_planning, Some(true));
        assert_eq!(parity.selected_adapter_matches_planning, Some(false));
        assert_eq!(parity.imported_kv_matches_planning, Some(false));
        assert_eq!(parity.kv_prefetch_matches_planning, Some(false));
        assert_eq!(parity.planning_violation_count, 0);
        assert_eq!(parity.planning_pre_request_problem_count, 0);
        assert_eq!(parity.planning_pressure_signal_count, 0);
        assert!(!parity.planning_missing_from_request());
        assert!(parity.max_tokens_drifted_from_planning());
        assert!(!parity.generation_budget_drifted_from_planning());
        assert!(parity.adapter_drifted_from_planning());
        assert!(parity.imported_kv_drifted_from_planning());
        assert!(parity.kv_prefetch_drifted_from_planning());
        assert!(!parity.planning_has_pre_request_gate_problems());
        assert!(!parity.planning_has_pressure_signals());
        assert_eq!(parity.token_drift_component_count(), 1);
        assert_eq!(parity.max_token_drift_component_count(), 1);
        assert_eq!(parity.generation_budget_drift_component_count(), 0);
        assert_eq!(parity.adapter_drift_component_count(), 1);
        assert_eq!(parity.kv_drift_component_count(), 2);
        assert_eq!(parity.imported_kv_drift_component_count(), 1);
        assert_eq!(parity.kv_prefetch_drift_component_count(), 1);
        assert_eq!(parity.planning_attachment_drift_component_count(), 0);
        assert_eq!(parity.planning_contract_drift_component_count(), 0);
        assert_eq!(
            parity.planning_pre_request_gate_problem_component_count(),
            0
        );
        assert_eq!(parity.planning_pressure_signal_component_count(), 0);
        assert_eq!(parity.request_planning_drift_component_count(), 4);
        assert_eq!(parity.backend_wire_problem_component_count(), 4);
        assert!(parity.has_backend_wire_problem_components());
        assert!(parity.backend_wire_accounting_is_consistent());
        assert!(!parity.backend_wire_shape_is_clean());
        assert!(!parity.can_use_backend_wire_request());
        assert!(!parity.token_budget_matches());
        assert!(!parity.adapter_matches());
        assert!(!parity.kv_import_matches());
        assert!(!parity.planning_has_contract_violations());
        assert!(!parity.request_matches_planning());
        assert!(!gate.request_accepted);
        assert!(!gate.envelope_consistent);
        assert!(gate.planning_attached);
        assert!(!gate.planning_consistent);
        assert_eq!(gate.accepted_imported_kv_blocks, 0);
        assert_eq!(gate.backend_wire_problem_count, 4);
        assert_eq!(gate.planning_pre_request_problem_count, 0);
        assert_eq!(gate.planning_pressure_signal_count, 0);
        assert!(gate.request_violation_count > 0);
        assert_eq!(gate.imported_kv_violation_count, 0);
        assert_eq!(gate.failure_report_count, 1);
        assert_eq!(gate.backend_wire_problem_component_count(), 4);
        assert_eq!(gate.direct_backend_wire_problem_component_count(), 4);
        assert_eq!(gate.planning_pre_request_gate_problem_component_count(), 0);
        assert_eq!(gate.planning_pressure_signal_component_count(), 0);
        assert_eq!(gate.imported_kv_activity_signal_component_count(), 0);
        assert_eq!(gate.send_gate_signal_component_count(), 0);
        assert!(!gate.send_gate_has_signal_components());
        assert!(gate.has_backend_wire_problem_components());
        assert!(!gate.has_planning_pre_request_gate_problems());
        assert!(!gate.has_planning_pressure_signals());
        assert!(gate.backend_wire_accounting_is_consistent());
        assert!(gate.has_acceptance_failures());
        assert!(gate.has_request_contract_failures());
        assert!(!gate.has_imported_kv_failures());
        assert!(gate.envelope_drifted());
        assert!(gate.planning_drifted());
        assert!(gate.has_boundary_drift());
        assert!(gate.has_failure_reports());
        assert!(gate.has_total_violations());
        assert_eq!(gate.request_contract_failure_component_count(), 1);
        assert_eq!(gate.imported_kv_failure_component_count(), 0);
        assert_eq!(gate.acceptance_failure_component_count(), 1);
        assert_eq!(gate.envelope_blocker_component_count(), 1);
        assert_eq!(gate.planning_blocker_component_count(), 1);
        assert_eq!(gate.boundary_drift_component_count(), 2);
        assert_eq!(gate.mapped_failure_report_component_count(), 1);
        assert_eq!(gate.send_blocker_component_count(), 4);
        assert!(gate.send_gate_has_problem_components());
        assert!(gate.send_gate_accounting_is_consistent());
        assert!(gate.failure_report_matches_failures());
        assert!(!gate.can_send_request());
        assert!(!gate.is_clean_send_gate());
        assert!(!gate.request_gate_shape_is_clean());
        assert_eq!(gate.runtime_request_commit_signal_component_count(), 0);
        assert!(!gate.has_runtime_request_commit_signals());
        assert_eq!(gate.runtime_request_commit_blocker_component_count(), 8);
        assert!(gate.has_runtime_request_commit_blockers());
        assert!(gate.runtime_request_commit_accounting_is_consistent());
        assert!(!gate.runtime_request_commit_is_clean());
        assert!(!gate.can_commit_runtime_request());
        assert!(!gate.can_send_runtime_request());
    }

    #[test]
    fn request_planning_parity_classifies_missing_and_contract_drift() {
        let missing = RuntimeRequestPlanningParitySummary {
            has_planning_digest: false,
            request_max_tokens: 8,
            planned_backend_max_tokens: None,
            max_tokens_match_planning: None,
            generation_budget_matches_planning: None,
            request_selected_adapter: Some(RuntimeAdapter::Cuda),
            planned_adapter: None,
            selected_adapter_matches_planning: None,
            imported_kv_blocks: 0,
            kv_prefetch_blocks: 0,
            planned_import_blocks: None,
            planned_export_blocks: None,
            imported_kv_matches_planning: None,
            kv_prefetch_matches_planning: None,
            planning_violation_count: 0,
            planning_pre_request_problem_count: 0,
            planning_pressure_signal_count: 0,
        };
        let contract_drift = RuntimeRequestPlanningParitySummary {
            has_planning_digest: true,
            request_max_tokens: 8,
            planned_backend_max_tokens: Some(8),
            max_tokens_match_planning: Some(true),
            generation_budget_matches_planning: Some(true),
            request_selected_adapter: Some(RuntimeAdapter::Cuda),
            planned_adapter: Some(RuntimeAdapter::Cuda),
            selected_adapter_matches_planning: Some(true),
            imported_kv_blocks: 1,
            kv_prefetch_blocks: 1,
            planned_import_blocks: Some(1),
            planned_export_blocks: Some(0),
            imported_kv_matches_planning: Some(true),
            kv_prefetch_matches_planning: Some(true),
            planning_violation_count: 2,
            planning_pre_request_problem_count: 3,
            planning_pressure_signal_count: 4,
        };

        assert!(missing.planning_missing_from_request());
        assert_eq!(missing.planning_attachment_drift_component_count(), 1);
        assert_eq!(missing.planning_contract_drift_component_count(), 0);
        assert_eq!(missing.max_token_drift_component_count(), 0);
        assert_eq!(missing.generation_budget_drift_component_count(), 0);
        assert_eq!(missing.token_drift_component_count(), 0);
        assert_eq!(missing.adapter_drift_component_count(), 0);
        assert_eq!(missing.imported_kv_drift_component_count(), 0);
        assert_eq!(missing.kv_prefetch_drift_component_count(), 0);
        assert_eq!(missing.kv_drift_component_count(), 0);
        assert_eq!(missing.request_planning_drift_component_count(), 1);
        assert_eq!(
            missing.planning_pre_request_gate_problem_component_count(),
            0
        );
        assert_eq!(missing.planning_pressure_signal_component_count(), 0);
        assert_eq!(missing.backend_wire_problem_component_count(), 1);
        assert!(missing.has_backend_wire_problem_components());
        assert!(missing.backend_wire_accounting_is_consistent());
        assert!(!missing.backend_wire_shape_is_clean());
        assert!(!missing.can_use_backend_wire_request());
        assert!(!missing.token_budget_matches());
        assert!(!missing.request_matches_planning());
        assert!(!contract_drift.planning_missing_from_request());
        assert!(contract_drift.planning_has_contract_violations());
        assert!(contract_drift.planning_has_pre_request_gate_problems());
        assert!(contract_drift.planning_has_pressure_signals());
        assert_eq!(
            contract_drift.planning_attachment_drift_component_count(),
            0
        );
        assert_eq!(contract_drift.planning_contract_drift_component_count(), 1);
        assert_eq!(contract_drift.max_token_drift_component_count(), 0);
        assert_eq!(contract_drift.generation_budget_drift_component_count(), 0);
        assert_eq!(contract_drift.token_drift_component_count(), 0);
        assert_eq!(contract_drift.adapter_drift_component_count(), 0);
        assert_eq!(contract_drift.imported_kv_drift_component_count(), 0);
        assert_eq!(contract_drift.kv_prefetch_drift_component_count(), 0);
        assert_eq!(contract_drift.kv_drift_component_count(), 0);
        assert_eq!(contract_drift.request_planning_drift_component_count(), 1);
        assert_eq!(
            contract_drift.planning_pre_request_gate_problem_component_count(),
            1
        );
        assert_eq!(contract_drift.planning_pressure_signal_component_count(), 1);
        assert_eq!(contract_drift.backend_wire_problem_component_count(), 2);
        assert!(contract_drift.has_backend_wire_problem_components());
        assert!(contract_drift.backend_wire_accounting_is_consistent());
        assert!(!contract_drift.backend_wire_shape_is_clean());
        assert!(!contract_drift.can_use_backend_wire_request());
        assert!(!contract_drift.request_matches_planning());
    }

    #[test]
    fn request_gate_summary_splits_backend_wire_problems_from_pressure_signals() {
        let gate = RuntimeRequestGateSummary {
            request_accepted: false,
            envelope_consistent: true,
            planning_attached: true,
            planning_consistent: false,
            accepted_imported_kv_blocks: 1,
            backend_wire_problem_count: 3,
            planning_pre_request_problem_count: 1,
            planning_pressure_signal_count: 4,
            request_violation_count: 1,
            imported_kv_violation_count: 0,
            failure_report_count: 1,
        };

        assert!(gate.has_backend_wire_problem_components());
        assert!(gate.has_planning_pre_request_gate_problems());
        assert!(gate.has_planning_pressure_signals());
        assert_eq!(gate.backend_wire_problem_component_count(), 3);
        assert_eq!(gate.direct_backend_wire_problem_component_count(), 2);
        assert_eq!(gate.planning_pre_request_gate_problem_component_count(), 1);
        assert_eq!(gate.planning_pressure_signal_component_count(), 1);
        assert!(gate.backend_wire_accounting_is_consistent());
        assert!(!gate.request_gate_shape_is_clean());
        assert_eq!(gate.runtime_request_commit_signal_component_count(), 5);
        assert!(gate.has_runtime_request_commit_signals());
        assert_eq!(gate.runtime_request_commit_blocker_component_count(), 5);
        assert!(gate.has_runtime_request_commit_blockers());
        assert!(gate.runtime_request_commit_accounting_is_consistent());
        assert!(!gate.runtime_request_commit_is_clean());
        assert!(!gate.can_commit_runtime_request());
        assert!(!gate.can_send_runtime_request());
        assert!(gate.has_request_contract_failures());
        assert!(gate.planning_drifted());
        assert_eq!(gate.request_contract_failure_component_count(), 1);
        assert_eq!(gate.planning_blocker_component_count(), 1);
    }

    #[test]
    fn request_gate_summary_counts_public_shape_drift() {
        let gate = RuntimeRequestGateSummary {
            request_accepted: true,
            envelope_consistent: true,
            planning_attached: true,
            planning_consistent: true,
            accepted_imported_kv_blocks: 0,
            backend_wire_problem_count: 1,
            planning_pre_request_problem_count: 0,
            planning_pressure_signal_count: 0,
            request_violation_count: 0,
            imported_kv_violation_count: 0,
            failure_report_count: 0,
        };

        assert!(gate.can_send_request());
        assert!(gate.is_clean_send_gate());
        assert!(gate.send_gate_accounting_is_consistent());
        assert!(gate.failure_report_matches_failures());
        assert!(!gate.backend_wire_accounting_is_consistent());
        assert!(!gate.request_gate_shape_is_clean());
        assert_eq!(gate.runtime_request_commit_signal_component_count(), 0);
        assert!(!gate.has_runtime_request_commit_signals());
        assert_eq!(gate.runtime_request_commit_blocker_component_count(), 1);
        assert!(gate.has_runtime_request_commit_blockers());
        assert!(!gate.runtime_request_commit_accounting_is_consistent());
        assert!(!gate.runtime_request_commit_is_clean());
        assert!(!gate.can_commit_runtime_request());
        assert!(!gate.can_send_runtime_request());
    }

    #[test]
    fn runtime_request_planning_readiness_confirms_request_wire_boundary() {
        let readiness = RuntimeRequestPlanningReadinessSummary::new(
            clean_runtime_planning_readiness(),
            clean_request_planning_parity(),
            clean_request_gate(),
        );

        assert_eq!(
            RuntimeRequestPlanningReadinessSummary::stage_order(),
            [
                RuntimeRequestPlanningReadinessStage::RuntimePlanning,
                RuntimeRequestPlanningReadinessStage::RequestPlanningParity,
                RuntimeRequestPlanningReadinessStage::RequestGate,
            ]
        );
        assert!(readiness.runtime_planning_ready());
        assert!(readiness.request_planning_ready());
        assert!(readiness.request_gate_ready());
        assert_eq!(readiness.first_unready_stage(), None);
        assert_eq!(readiness.first_blocking_stage(), None);
        assert_eq!(
            readiness.stage_signal_component_count(
                RuntimeRequestPlanningReadinessStage::RuntimePlanning
            ),
            readiness.runtime_planning_signal_component_count
        );
        assert_eq!(
            readiness
                .stage_blocker_component_count(RuntimeRequestPlanningReadinessStage::RequestGate),
            readiness.request_gate_blocker_component_count
        );
        assert_eq!(readiness.request_planning_blocker_component_count, 0);
        assert_eq!(readiness.request_gate_blocker_component_count, 0);
        assert!(readiness.has_runtime_request_planning_signals());
        assert!(!readiness.has_runtime_request_planning_blockers());
        assert_eq!(
            readiness.runtime_request_planning_blocker_component_count(),
            0
        );
        assert!(readiness.runtime_request_planning_accounting_is_consistent());
        assert!(readiness.runtime_request_planning_is_clean());
        assert!(readiness.can_commit_runtime_request_planning());
    }

    #[test]
    fn runtime_request_planning_readiness_requires_committed_runtime_parts() {
        let clean = RuntimeRequestPlanningReadinessSummary::new(
            clean_runtime_planning_readiness(),
            clean_request_planning_parity(),
            clean_request_gate(),
        );

        assert!(clean.runtime_planning_ready());
        assert!(clean.runtime_planning_committed_parts_ready());
        assert!(clean.can_commit_runtime_request_planning());
        assert!(clean.can_commit_runtime_request_planning_with_committed_parts());

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
        let stale_fht_dke_planning =
            clean_fht_dke_planning_readiness(route_budget, stale_fht_dke_budget);
        let stale_runtime_planning = RuntimePlanningReadinessSummary::new(
            stale_fht_dke_planning,
            clean_runtime_planning_summary(stale_fht_dke_budget),
        );
        let stale = RuntimeRequestPlanningReadinessSummary::new(
            stale_runtime_planning,
            clean_request_planning_parity(),
            clean_request_gate(),
        );

        assert!(!stale_runtime_planning.can_use_committed_fht_dke_runtime_planning_parts());
        assert!(!stale.runtime_planning_ready());
        assert!(!stale.runtime_planning_committed_parts_ready());
        assert!(stale.request_planning_ready());
        assert!(stale.request_gate_ready());
        assert!(!stale.can_commit_runtime_request_planning());
        assert!(!stale.can_commit_runtime_request_planning_with_committed_parts());
        assert_eq!(
            stale.first_unready_stage(),
            Some(RuntimeRequestPlanningReadinessStage::RuntimePlanning)
        );
        assert_eq!(
            stale.first_blocking_stage(),
            Some(RuntimeRequestPlanningReadinessStage::RuntimePlanning)
        );
    }

    #[test]
    fn runtime_request_planning_readiness_blocks_context_exhausted_runtime_planning() {
        let route_budget = clean_route_budget();
        let fht_dke_budget = FhtDkeBudgetSummary {
            enabled: true,
            total_tokens: 128,
            dense_tokens: 128,
            routed_tokens: 0,
            dense_fraction: 1.0,
            routed_fraction: 0.0,
            kv_import_blocks: 0,
            kv_export_blocks: 0,
            kv_exchange_blocks: 0,
            has_kv_exchange: false,
            token_split_is_valid: true,
            attention_threshold: route_budget.threshold,
            route_pressure: route_budget.attention_fraction,
        };
        let fht_dke_planning = clean_fht_dke_planning_readiness(route_budget, fht_dke_budget);
        let mut runtime_planning = clean_runtime_planning_summary(fht_dke_budget);
        runtime_planning.generation_budget = RuntimeGenerationBudget::new(128, 1, 128);
        runtime_planning.context_limited = true;
        runtime_planning.backend_max_tokens = 0;
        let runtime_readiness =
            RuntimePlanningReadinessSummary::new(fht_dke_planning, runtime_planning);
        let readiness = RuntimeRequestPlanningReadinessSummary::new(
            runtime_readiness,
            clean_request_planning_parity(),
            clean_request_gate(),
        );

        assert!(runtime_planning.context_exhausted());
        assert!(!runtime_readiness.runtime_pre_request_ready());
        assert_eq!(
            runtime_readiness.first_unready_stage(),
            Some(RuntimePlanningReadinessStage::RuntimePreRequest)
        );
        assert_eq!(
            runtime_readiness.runtime_pre_request_blocker_component_count,
            1
        );
        assert!(!readiness.runtime_planning_ready());
        assert!(readiness.request_planning_ready());
        assert!(readiness.request_gate_ready());
        assert_eq!(
            readiness.first_unready_stage(),
            Some(RuntimeRequestPlanningReadinessStage::RuntimePlanning)
        );
        assert_eq!(
            readiness.first_blocking_stage(),
            Some(RuntimeRequestPlanningReadinessStage::RuntimePlanning)
        );
        assert_eq!(readiness.runtime_planning_blocker_component_count, 1);
        assert_eq!(readiness.request_planning_blocker_component_count, 0);
        assert_eq!(readiness.request_gate_blocker_component_count, 0);
        assert_eq!(
            readiness.runtime_request_planning_blocker_component_count(),
            1
        );
        assert!(readiness.has_runtime_request_planning_blockers());
        assert!(readiness.runtime_request_planning_accounting_is_consistent());
        assert!(!readiness.runtime_request_planning_is_clean());
        assert!(!readiness.can_commit_runtime_request_planning());
    }

    #[test]
    fn runtime_request_planning_readiness_blocks_request_planning_parity_drift() {
        let mut parity = clean_request_planning_parity();
        parity.max_tokens_match_planning = Some(false);
        parity.kv_prefetch_matches_planning = Some(false);
        let mut gate = clean_request_gate();
        gate.planning_consistent = false;
        gate.backend_wire_problem_count = parity.backend_wire_problem_component_count();
        let readiness = RuntimeRequestPlanningReadinessSummary::new(
            clean_runtime_planning_readiness(),
            parity,
            gate,
        );

        assert!(readiness.runtime_planning_ready());
        assert!(!readiness.request_planning_ready());
        assert!(!readiness.request_gate_ready());
        assert_eq!(
            readiness.first_unready_stage(),
            Some(RuntimeRequestPlanningReadinessStage::RequestPlanningParity)
        );
        assert_eq!(
            readiness.first_blocking_stage(),
            Some(RuntimeRequestPlanningReadinessStage::RequestPlanningParity)
        );
        assert_eq!(readiness.request_planning_blocker_component_count, 2);
        assert_eq!(readiness.request_gate_blocker_component_count, 3);
        assert_eq!(
            readiness.runtime_request_planning_blocker_component_count(),
            5
        );
        assert!(readiness.has_runtime_request_planning_blockers());
        assert!(readiness.runtime_request_planning_accounting_is_consistent());
        assert!(!readiness.runtime_request_planning_is_clean());
        assert!(!readiness.can_commit_runtime_request_planning());
    }

    #[test]
    fn request_acceptance_summary_counts_public_shape_drift() {
        let clean = RuntimeRequestAcceptanceSummary {
            accepted: true,
            request_violation_count: 0,
            imported_kv_violation_count: 0,
            accepted_imported_kv_blocks: 2,
            failure_report_count: 0,
        };
        let drift = RuntimeRequestAcceptanceSummary {
            accepted: true,
            request_violation_count: 1,
            imported_kv_violation_count: 0,
            accepted_imported_kv_blocks: 0,
            failure_report_count: 0,
        };

        assert_eq!(clean.request_acceptance_problem_component_count(), 0);
        assert!(!clean.has_request_acceptance_problem_components());
        assert!(clean.request_acceptance_accounting_is_consistent());
        assert_eq!(
            clean.runtime_request_acceptance_commit_signal_component_count(),
            2
        );
        assert_eq!(
            clean.runtime_request_acceptance_commit_blocker_component_count(),
            0
        );
        assert!(clean.runtime_request_acceptance_commit_accounting_is_consistent());
        assert!(clean.runtime_request_acceptance_commit_is_clean());
        assert!(clean.can_commit_runtime_request_acceptance());
        assert!(clean.is_clean_acceptance());
        assert!(clean.request_acceptance_shape_is_clean());
        assert!(clean.can_accept_runtime_request());

        assert_eq!(drift.total_violation_count(), 1);
        assert!(drift.has_request_contract_failures());
        assert_eq!(drift.acceptance_failure_component_count(), 1);
        assert_eq!(drift.request_acceptance_problem_component_count(), 1);
        assert!(drift.has_request_acceptance_problem_components());
        assert!(!drift.failure_report_matches_failures());
        assert!(!drift.request_acceptance_accounting_is_consistent());
        assert_eq!(
            drift.runtime_request_acceptance_commit_signal_component_count(),
            1
        );
        assert_eq!(
            drift.runtime_request_acceptance_commit_blocker_component_count(),
            1
        );
        assert!(!drift.runtime_request_acceptance_commit_accounting_is_consistent());
        assert!(!drift.runtime_request_acceptance_commit_is_clean());
        assert!(!drift.can_commit_runtime_request_acceptance());
        assert!(!drift.is_clean_acceptance());
        assert!(!drift.request_acceptance_shape_is_clean());
        assert!(!drift.can_accept_runtime_request());
    }

    fn clean_request_planning_parity() -> RuntimeRequestPlanningParitySummary {
        RuntimeRequestPlanningParitySummary {
            has_planning_digest: true,
            request_max_tokens: 200,
            planned_backend_max_tokens: Some(200),
            max_tokens_match_planning: Some(true),
            generation_budget_matches_planning: Some(true),
            request_selected_adapter: Some(RuntimeAdapter::Cuda),
            planned_adapter: Some(RuntimeAdapter::Cuda),
            selected_adapter_matches_planning: Some(true),
            imported_kv_blocks: 4,
            kv_prefetch_blocks: 4,
            planned_import_blocks: Some(4),
            planned_export_blocks: Some(4),
            imported_kv_matches_planning: Some(true),
            kv_prefetch_matches_planning: Some(true),
            planning_violation_count: 0,
            planning_pre_request_problem_count: 0,
            planning_pressure_signal_count: 3,
        }
    }

    fn clean_request_gate() -> RuntimeRequestGateSummary {
        RuntimeRequestGateSummary {
            request_accepted: true,
            envelope_consistent: true,
            planning_attached: true,
            planning_consistent: true,
            accepted_imported_kv_blocks: 4,
            backend_wire_problem_count: 0,
            planning_pre_request_problem_count: 0,
            planning_pressure_signal_count: 3,
            request_violation_count: 0,
            imported_kv_violation_count: 0,
            failure_report_count: 0,
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
        let fht_dke_planning = clean_fht_dke_planning_readiness(route_budget, fht_dke_budget);
        RuntimePlanningReadinessSummary::new(
            fht_dke_planning,
            clean_runtime_planning_summary(fht_dke_budget),
        )
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
            adapter_selection: crate::adapter::AdapterSelection {
                adapter: RuntimeAdapter::Cuda,
                score: 0.95,
                experience_id: Some(16),
                used_fallback: false,
            },
            adapter_fallback_reason: crate::adapter::AdapterFallbackReason::NoFallback,
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
}
