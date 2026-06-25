use std::str::FromStr;

use crate::adapter::{AdapterExecutionContext, RuntimeAdapter};
use crate::engine::{RuntimeFailureBatchSummary, RuntimeFailureReport, RuntimeFailureSummary};
use crate::hardware::{ComputeLane, DeviceClass, DeviceMemoryMode, HardwarePlan};
use crate::manifest::TransformerRuntimeArchitecture;
use crate::request::RuntimeRequestEnvelope;
use crate::router::RouteBudget;
use crate::runtime::{RuntimeGenerationBudget, RuntimeMetadata};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceExecutionSource {
    RuntimeReported,
    ControlPlaneFilled,
}

impl DeviceExecutionSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RuntimeReported => "runtime-reported",
            Self::ControlPlaneFilled => "control-plane-filled",
        }
    }
}

impl FromStr for DeviceExecutionSource {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "runtime-reported" => Ok(Self::RuntimeReported),
            "control-plane-filled" => Ok(Self::ControlPlaneFilled),
            other => Err(format!("unknown device execution source: {other}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RuntimeDiagnostics {
    pub model_id: Option<String>,
    pub selected_adapter: Option<RuntimeAdapter>,
    pub device_profile: Option<String>,
    pub primary_lane: Option<String>,
    pub fallback_lane: Option<String>,
    pub memory_mode: Option<String>,
    pub device_execution_source: Option<DeviceExecutionSource>,
    pub layer_count: usize,
    pub global_layers: usize,
    pub local_window_layers: usize,
    pub fusion_layers: usize,
    pub hidden_size: usize,
    pub local_window_tokens: usize,
    pub forward_energy: Option<f32>,
    pub kv_influence: Option<f32>,
    pub imported_kv_blocks: usize,
    pub exported_kv_blocks: usize,
    pub weak_runtime_kv_imports_skipped: usize,
    pub hot_kv_precision_bits: Option<u8>,
    pub cold_kv_precision_bits: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeDiagnosticsSummary {
    pub has_model_id: bool,
    pub has_selected_adapter: bool,
    pub has_runtime_architecture: bool,
    pub layer_count: usize,
    pub layer_mode_count: usize,
    pub has_all_layer_modes: bool,
    pub has_device_execution: bool,
    pub device_execution_source: Option<DeviceExecutionSource>,
    pub has_forward_energy: bool,
    pub has_kv_influence: bool,
    pub imported_kv_blocks: usize,
    pub exported_kv_blocks: usize,
    pub weak_runtime_kv_imports_skipped: usize,
    pub has_valid_kv_precision: bool,
    pub has_forward_signal: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeDiagnosticsRequestParitySummary {
    pub model_id_reported: bool,
    pub model_id_matches_request: bool,
    pub selected_adapter_reported: bool,
    pub selected_adapter_matches_request: bool,
    pub architecture_reported: bool,
    pub layer_count_matches_request: bool,
    pub hidden_size_matches_request: bool,
    pub local_window_tokens_within_request: bool,
    pub imported_kv_blocks: usize,
    pub request_imported_kv_blocks: usize,
    pub imported_kv_matches_request: bool,
    pub exported_kv_blocks: usize,
    pub runtime_export_enabled: bool,
    pub runtime_max_export_blocks: usize,
    pub exported_kv_within_runtime: bool,
    pub kv_precision_reported: bool,
    pub kv_precision_valid: bool,
    pub kv_precision_within_request: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeDiagnosticsContractSummary {
    pub model_id_reported: bool,
    pub model_id_matches_metadata: bool,
    pub layer_count_reported: bool,
    pub layer_count_matches_architecture: bool,
    pub hidden_size_reported: bool,
    pub hidden_size_matches_architecture: bool,
    pub local_window_tokens_reported: bool,
    pub local_window_tokens_within_context: bool,
    pub selected_adapter_reported: bool,
    pub selected_adapter_within_execution: bool,
    pub hot_kv_precision_reported: bool,
    pub hot_kv_precision_within_metadata: bool,
    pub cold_kv_precision_reported: bool,
    pub cold_kv_precision_within_metadata: bool,
    pub kv_precision_pair_reported: bool,
    pub kv_precision_pair_valid: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeHardwareDiagnosticsContractSummary {
    pub device_profile_reported: bool,
    pub device_profile_known: bool,
    pub device_profile_matches_hardware: bool,
    pub primary_lane_reported: bool,
    pub primary_lane_known: bool,
    pub primary_lane_matches_hardware: bool,
    pub fallback_lane_reported: bool,
    pub fallback_lane_known: bool,
    pub fallback_lane_matches_hardware: bool,
    pub memory_mode_reported: bool,
    pub memory_mode_known: bool,
    pub memory_mode_matches_hardware: bool,
}

impl RuntimeDiagnosticsSummary {
    pub fn has_runtime_identity(self) -> bool {
        self.has_model_id && self.has_selected_adapter
    }

    pub fn missing_runtime_identity(self) -> bool {
        !self.has_runtime_identity()
    }

    pub fn has_runtime_reported_device_execution(self) -> bool {
        self.has_device_execution
            && self.device_execution_source == Some(DeviceExecutionSource::RuntimeReported)
    }

    pub fn has_control_plane_filled_device_execution(self) -> bool {
        self.has_device_execution
            && self.device_execution_source == Some(DeviceExecutionSource::ControlPlaneFilled)
    }

    pub fn has_device_execution_for_hardware_diagnostics(self) -> bool {
        self.has_device_execution && self.device_execution_source_matches_execution()
    }

    pub fn can_use_device_execution_for_hardware_diagnostics(self) -> bool {
        self.can_use_runtime_diagnostics() && self.has_device_execution_for_hardware_diagnostics()
    }

    pub fn has_device_execution_source(self) -> bool {
        self.device_execution_source.is_some()
    }

    pub fn device_execution_source_matches_execution(self) -> bool {
        self.has_device_execution == self.has_device_execution_source()
    }

    pub fn kv_exchange_total(self) -> usize {
        self.imported_kv_blocks
            .saturating_add(self.exported_kv_blocks)
    }

    pub fn has_runtime_kv_exchange(self) -> bool {
        self.kv_exchange_total() > 0
    }

    pub fn runtime_kv_activity_total(self) -> usize {
        self.kv_exchange_total()
            .saturating_add(self.weak_runtime_kv_imports_skipped)
    }

    pub fn has_runtime_kv_activity(self) -> bool {
        self.runtime_kv_activity_total() > 0
    }

    pub fn has_runtime_forward_or_kv_signal(self) -> bool {
        self.has_forward_signal || self.has_runtime_kv_activity()
    }

    pub fn missing_runtime_architecture(self) -> bool {
        !self.has_runtime_architecture
    }

    pub fn missing_valid_kv_precision(self) -> bool {
        !self.has_valid_kv_precision
    }

    pub fn has_complete_runtime_signal(self) -> bool {
        self.has_runtime_identity()
            && self.has_runtime_architecture
            && self.has_forward_signal
            && self.has_valid_kv_precision
    }

    pub fn runtime_identity_signal_component_count(self) -> usize {
        usize::from(self.has_model_id) + usize::from(self.has_selected_adapter)
    }

    pub fn runtime_architecture_signal_component_count(self) -> usize {
        usize::from(self.has_runtime_architecture)
            + usize::from(self.layer_mode_count > 0)
            + usize::from(self.has_all_layer_modes)
    }

    pub fn device_execution_signal_component_count(self) -> usize {
        usize::from(self.has_device_execution)
            + usize::from(self.has_runtime_reported_device_execution())
            + usize::from(self.has_control_plane_filled_device_execution())
    }

    pub fn runtime_forward_signal_component_count(self) -> usize {
        usize::from(self.has_forward_energy) + usize::from(self.has_kv_influence)
    }

    pub fn runtime_kv_activity_signal_component_count(self) -> usize {
        usize::from(self.imported_kv_blocks > 0)
            + usize::from(self.exported_kv_blocks > 0)
            + usize::from(self.weak_runtime_kv_imports_skipped > 0)
    }

    pub fn runtime_precision_signal_component_count(self) -> usize {
        usize::from(self.has_valid_kv_precision)
    }

    pub fn runtime_diagnostics_signal_component_count(self) -> usize {
        self.runtime_identity_signal_component_count()
            .saturating_add(self.runtime_architecture_signal_component_count())
            .saturating_add(self.device_execution_signal_component_count())
            .saturating_add(self.runtime_forward_signal_component_count())
            .saturating_add(self.runtime_kv_activity_signal_component_count())
            .saturating_add(self.runtime_precision_signal_component_count())
    }

    pub fn has_runtime_diagnostics_signals(self) -> bool {
        self.runtime_diagnostics_signal_component_count() > 0
    }

    pub fn runtime_identity_problem_component_count(self) -> usize {
        usize::from(!self.has_model_id) + usize::from(!self.has_selected_adapter)
    }

    pub fn runtime_architecture_problem_component_count(self) -> usize {
        usize::from(self.missing_runtime_architecture())
            + usize::from(
                self.has_runtime_architecture
                    && self.layer_mode_count > 0
                    && !self.has_all_layer_modes,
            )
    }

    pub fn device_execution_source_problem_component_count(self) -> usize {
        usize::from(!self.device_execution_source_matches_execution())
    }

    pub fn runtime_activity_problem_component_count(self) -> usize {
        usize::from(!self.has_runtime_forward_or_kv_signal())
    }

    pub fn runtime_precision_problem_component_count(self) -> usize {
        usize::from(self.missing_valid_kv_precision())
    }

    pub fn runtime_diagnostics_problem_component_count(self) -> usize {
        self.runtime_identity_problem_component_count()
            .saturating_add(self.runtime_architecture_problem_component_count())
            .saturating_add(self.device_execution_source_problem_component_count())
            .saturating_add(self.runtime_activity_problem_component_count())
            .saturating_add(self.runtime_precision_problem_component_count())
    }

    pub fn has_runtime_diagnostics_problem_components(self) -> bool {
        self.runtime_diagnostics_problem_component_count() > 0
    }

    pub fn runtime_diagnostics_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .runtime_identity_signal_component_count()
            .saturating_add(self.runtime_architecture_signal_component_count())
            .saturating_add(self.device_execution_signal_component_count())
            .saturating_add(self.runtime_forward_signal_component_count())
            .saturating_add(self.runtime_kv_activity_signal_component_count())
            .saturating_add(self.runtime_precision_signal_component_count());
        let expected_problem_count = self
            .runtime_identity_problem_component_count()
            .saturating_add(self.runtime_architecture_problem_component_count())
            .saturating_add(self.device_execution_source_problem_component_count())
            .saturating_add(self.runtime_activity_problem_component_count())
            .saturating_add(self.runtime_precision_problem_component_count());

        self.runtime_diagnostics_signal_component_count() == expected_signal_count
            && self.has_runtime_diagnostics_signals() == (expected_signal_count > 0)
            && self.runtime_diagnostics_problem_component_count() == expected_problem_count
            && self.has_runtime_diagnostics_problem_components() == (expected_problem_count > 0)
    }

    pub fn runtime_diagnostics_shape_is_clean(self) -> bool {
        !self.has_runtime_diagnostics_problem_components()
            && self.runtime_diagnostics_accounting_is_consistent()
    }

    pub fn can_use_runtime_diagnostics(self) -> bool {
        self.runtime_diagnostics_shape_is_clean()
    }

    pub fn hardware_adapter_metadata_signal_component_count(self) -> usize {
        self.device_execution_signal_component_count()
    }

    pub fn has_hardware_adapter_metadata_signals(self) -> bool {
        self.hardware_adapter_metadata_signal_component_count() > 0
    }

    pub fn missing_device_execution_metadata_component_count(self) -> usize {
        usize::from(!self.has_device_execution)
    }

    pub fn control_plane_filled_metadata_component_count(self) -> usize {
        usize::from(self.has_control_plane_filled_device_execution())
    }

    pub fn hardware_adapter_metadata_blocker_component_count(self) -> usize {
        self.device_execution_source_problem_component_count()
            .saturating_add(self.missing_device_execution_metadata_component_count())
            .saturating_add(self.control_plane_filled_metadata_component_count())
    }

    pub fn has_hardware_adapter_metadata_blockers(self) -> bool {
        self.hardware_adapter_metadata_blocker_component_count() > 0
    }

    pub fn hardware_adapter_metadata_admission_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self.device_execution_signal_component_count();
        let expected_blocker_count = self
            .device_execution_source_problem_component_count()
            .saturating_add(usize::from(!self.has_device_execution))
            .saturating_add(usize::from(
                self.has_control_plane_filled_device_execution(),
            ));

        self.runtime_diagnostics_accounting_is_consistent()
            && self.hardware_adapter_metadata_signal_component_count() == expected_signal_count
            && self.has_hardware_adapter_metadata_signals() == (expected_signal_count > 0)
            && self.missing_device_execution_metadata_component_count()
                == usize::from(!self.has_device_execution)
            && self.control_plane_filled_metadata_component_count()
                == usize::from(self.has_control_plane_filled_device_execution())
            && self.hardware_adapter_metadata_blocker_component_count() == expected_blocker_count
            && self.has_hardware_adapter_metadata_blockers() == (expected_blocker_count > 0)
    }

    pub fn hardware_adapter_metadata_admission_is_clean(self) -> bool {
        !self.has_hardware_adapter_metadata_blockers()
            && self.hardware_adapter_metadata_admission_accounting_is_consistent()
    }

    pub fn can_admit_hardware_adapter_metadata(self) -> bool {
        self.can_use_runtime_diagnostics()
            && self.has_runtime_reported_device_execution()
            && self.hardware_adapter_metadata_admission_is_clean()
    }
}

impl RuntimeDiagnosticsRequestParitySummary {
    pub fn model_parity_ok(self) -> bool {
        self.model_id_reported && self.model_id_matches_request
    }

    pub fn adapter_parity_ok(self) -> bool {
        self.selected_adapter_reported && self.selected_adapter_matches_request
    }

    pub fn architecture_parity_ok(self) -> bool {
        self.architecture_reported
            && self.layer_count_matches_request
            && self.hidden_size_matches_request
            && self.local_window_tokens_within_request
    }

    pub fn kv_parity_ok(self) -> bool {
        self.imported_kv_matches_request && self.exported_kv_within_runtime
    }

    pub fn precision_parity_ok(self) -> bool {
        self.kv_precision_reported && self.kv_precision_valid && self.kv_precision_within_request
    }

    pub fn missing_model_id_report(self) -> bool {
        !self.model_id_reported
    }

    pub fn missing_selected_adapter_report(self) -> bool {
        !self.selected_adapter_reported
    }

    pub fn missing_architecture_report(self) -> bool {
        !self.architecture_reported
    }

    pub fn missing_kv_precision_report(self) -> bool {
        !self.kv_precision_reported
    }

    pub fn request_parity_is_consistent(self) -> bool {
        self.model_parity_ok()
            && self.adapter_parity_ok()
            && self.architecture_parity_ok()
            && self.kv_parity_ok()
            && self.precision_parity_ok()
    }

    pub fn model_drifted(self) -> bool {
        self.model_id_reported && !self.model_id_matches_request
    }

    pub fn adapter_drifted(self) -> bool {
        self.selected_adapter_reported && !self.selected_adapter_matches_request
    }

    pub fn architecture_drifted(self) -> bool {
        self.architecture_reported && !self.architecture_parity_ok()
    }

    pub fn imported_kv_drifted(self) -> bool {
        !self.imported_kv_matches_request
    }

    pub fn exported_kv_exceeds_runtime(self) -> bool {
        !self.exported_kv_within_runtime
    }

    pub fn exported_kv_block_overflow(self) -> usize {
        if self.runtime_export_enabled {
            if self.runtime_max_export_blocks == 0 {
                0
            } else {
                self.exported_kv_blocks
                    .saturating_sub(self.runtime_max_export_blocks)
            }
        } else {
            self.exported_kv_blocks
        }
    }

    pub fn kv_count_drifted(self) -> bool {
        self.imported_kv_drifted() || self.exported_kv_exceeds_runtime()
    }

    pub fn precision_drifted(self) -> bool {
        self.kv_precision_reported && !self.kv_precision_within_request
    }

    pub fn missing_required_runtime_report(self) -> bool {
        !self.model_id_reported
            || !self.selected_adapter_reported
            || !self.architecture_reported
            || !self.kv_precision_reported
    }

    pub fn missing_report_component_count(self) -> usize {
        usize::from(self.missing_model_id_report())
            + usize::from(self.missing_selected_adapter_report())
            + usize::from(self.missing_architecture_report())
            + usize::from(self.missing_kv_precision_report())
    }

    pub fn identity_drift_component_count(self) -> usize {
        usize::from(self.model_drifted()) + usize::from(self.adapter_drifted())
    }

    pub fn architecture_drift_component_count(self) -> usize {
        usize::from(self.architecture_drifted())
    }

    pub fn kv_drift_component_count(self) -> usize {
        usize::from(self.imported_kv_drifted()) + usize::from(self.exported_kv_exceeds_runtime())
    }

    pub fn precision_drift_component_count(self) -> usize {
        usize::from(self.precision_drifted())
            + usize::from(self.kv_precision_reported && !self.kv_precision_valid)
    }

    pub fn runtime_drift_component_count(self) -> usize {
        self.missing_report_component_count()
            .saturating_add(self.identity_drift_component_count())
            .saturating_add(self.architecture_drift_component_count())
            .saturating_add(self.kv_drift_component_count())
            .saturating_add(self.precision_drift_component_count())
    }

    pub fn has_runtime_drift_components(self) -> bool {
        self.runtime_drift_component_count() > 0
    }

    pub fn runtime_drift_accounting_is_consistent(self) -> bool {
        let expected_problem_count = self
            .missing_report_component_count()
            .saturating_add(self.identity_drift_component_count())
            .saturating_add(self.architecture_drift_component_count())
            .saturating_add(self.kv_drift_component_count())
            .saturating_add(self.precision_drift_component_count());

        self.runtime_drift_component_count() == expected_problem_count
            && self.has_runtime_drift_components() == (expected_problem_count > 0)
            && self.request_parity_is_consistent() == (expected_problem_count == 0)
    }

    pub fn runtime_request_parity_shape_is_clean(self) -> bool {
        !self.has_runtime_drift_components() && self.runtime_drift_accounting_is_consistent()
    }

    pub fn can_accept_runtime_diagnostics_request_parity(self) -> bool {
        self.runtime_request_parity_shape_is_clean()
    }

    pub fn runtime_request_parity_admission_signal_component_count(self) -> usize {
        usize::from(self.model_parity_ok())
            .saturating_add(usize::from(self.adapter_parity_ok()))
            .saturating_add(usize::from(self.architecture_parity_ok()))
            .saturating_add(usize::from(self.kv_parity_ok()))
            .saturating_add(usize::from(self.precision_parity_ok()))
    }

    pub fn has_runtime_request_parity_admission_signals(self) -> bool {
        self.runtime_request_parity_admission_signal_component_count() > 0
    }

    pub fn runtime_request_parity_admission_blocker_component_count(self) -> usize {
        self.runtime_drift_component_count()
            .saturating_add(usize::from(!self.request_parity_is_consistent()))
    }

    pub fn has_runtime_request_parity_admission_blockers(self) -> bool {
        self.runtime_request_parity_admission_blocker_component_count() > 0
    }

    pub fn runtime_request_parity_admission_accounting_is_consistent(self) -> bool {
        let expected_signal_count = usize::from(self.model_parity_ok())
            .saturating_add(usize::from(self.adapter_parity_ok()))
            .saturating_add(usize::from(self.architecture_parity_ok()))
            .saturating_add(usize::from(self.kv_parity_ok()))
            .saturating_add(usize::from(self.precision_parity_ok()));
        let expected_blocker_count = self
            .runtime_drift_component_count()
            .saturating_add(usize::from(!self.request_parity_is_consistent()));

        self.runtime_drift_accounting_is_consistent()
            && self.runtime_request_parity_admission_signal_component_count()
                == expected_signal_count
            && self.has_runtime_request_parity_admission_signals() == (expected_signal_count > 0)
            && self.runtime_request_parity_admission_blocker_component_count()
                == expected_blocker_count
            && self.has_runtime_request_parity_admission_blockers() == (expected_blocker_count > 0)
    }

    pub fn runtime_request_parity_admission_is_clean(self) -> bool {
        !self.has_runtime_request_parity_admission_blockers()
            && self.runtime_request_parity_admission_accounting_is_consistent()
    }

    pub fn can_admit_runtime_diagnostics_request_parity(self) -> bool {
        self.can_accept_runtime_diagnostics_request_parity()
            && self.runtime_request_parity_admission_is_clean()
    }
}

impl RuntimeDiagnosticsContractSummary {
    pub fn identity_contract_is_ready(self) -> bool {
        self.model_id_reported && self.model_id_matches_metadata
    }

    pub fn architecture_contract_is_ready(self) -> bool {
        self.layer_count_reported
            && self.layer_count_matches_architecture
            && self.hidden_size_reported
            && self.hidden_size_matches_architecture
            && self.local_window_tokens_reported
            && self.local_window_tokens_within_context
    }

    pub fn adapter_contract_is_ready(self) -> bool {
        self.selected_adapter_reported && self.selected_adapter_within_execution
    }

    pub fn precision_contract_is_ready(self) -> bool {
        self.hot_kv_precision_reported
            && self.hot_kv_precision_within_metadata
            && self.cold_kv_precision_reported
            && self.cold_kv_precision_within_metadata
            && self.kv_precision_pair_reported
            && self.kv_precision_pair_valid
    }

    pub fn identity_contract_problem_component_count(self) -> usize {
        usize::from(self.model_id_reported && !self.model_id_matches_metadata)
    }

    pub fn architecture_contract_problem_component_count(self) -> usize {
        usize::from(self.layer_count_reported && !self.layer_count_matches_architecture)
            .saturating_add(usize::from(
                self.hidden_size_reported && !self.hidden_size_matches_architecture,
            ))
            .saturating_add(usize::from(
                self.local_window_tokens_reported && !self.local_window_tokens_within_context,
            ))
    }

    pub fn adapter_contract_problem_component_count(self) -> usize {
        usize::from(self.selected_adapter_reported && !self.selected_adapter_within_execution)
    }

    pub fn precision_contract_problem_component_count(self) -> usize {
        usize::from(self.hot_kv_precision_reported && !self.hot_kv_precision_within_metadata)
            .saturating_add(usize::from(
                self.cold_kv_precision_reported && !self.cold_kv_precision_within_metadata,
            ))
            .saturating_add(usize::from(
                self.kv_precision_pair_reported && !self.kv_precision_pair_valid,
            ))
    }

    pub fn diagnostics_contract_problem_component_count(self) -> usize {
        self.identity_contract_problem_component_count()
            .saturating_add(self.architecture_contract_problem_component_count())
            .saturating_add(self.adapter_contract_problem_component_count())
            .saturating_add(self.precision_contract_problem_component_count())
    }

    pub fn has_diagnostics_contract_problem_components(self) -> bool {
        self.diagnostics_contract_problem_component_count() > 0
    }

    pub fn diagnostics_contract_signal_component_count(self) -> usize {
        usize::from(self.model_id_reported)
            .saturating_add(usize::from(self.layer_count_reported))
            .saturating_add(usize::from(self.hidden_size_reported))
            .saturating_add(usize::from(self.local_window_tokens_reported))
            .saturating_add(usize::from(self.selected_adapter_reported))
            .saturating_add(usize::from(self.hot_kv_precision_reported))
            .saturating_add(usize::from(self.cold_kv_precision_reported))
            .saturating_add(usize::from(self.kv_precision_pair_reported))
    }

    pub fn has_diagnostics_contract_signals(self) -> bool {
        self.diagnostics_contract_signal_component_count() > 0
    }

    pub fn diagnostics_contract_accounting_is_consistent(self) -> bool {
        let expected_problem_count = self
            .identity_contract_problem_component_count()
            .saturating_add(self.architecture_contract_problem_component_count())
            .saturating_add(self.adapter_contract_problem_component_count())
            .saturating_add(self.precision_contract_problem_component_count());
        let expected_signal_count = usize::from(self.model_id_reported)
            .saturating_add(usize::from(self.layer_count_reported))
            .saturating_add(usize::from(self.hidden_size_reported))
            .saturating_add(usize::from(self.local_window_tokens_reported))
            .saturating_add(usize::from(self.selected_adapter_reported))
            .saturating_add(usize::from(self.hot_kv_precision_reported))
            .saturating_add(usize::from(self.cold_kv_precision_reported))
            .saturating_add(usize::from(self.kv_precision_pair_reported));

        self.diagnostics_contract_problem_component_count() == expected_problem_count
            && self.has_diagnostics_contract_problem_components() == (expected_problem_count > 0)
            && self.diagnostics_contract_signal_component_count() == expected_signal_count
            && self.has_diagnostics_contract_signals() == (expected_signal_count > 0)
    }

    pub fn diagnostics_contract_shape_is_clean(self) -> bool {
        !self.has_diagnostics_contract_problem_components()
            && self.diagnostics_contract_accounting_is_consistent()
    }

    pub fn can_accept_runtime_diagnostics_contract(self) -> bool {
        self.diagnostics_contract_shape_is_clean()
    }

    pub fn missing_contract_report_component_count(self) -> usize {
        usize::from(!self.model_id_reported)
            .saturating_add(usize::from(!self.layer_count_reported))
            .saturating_add(usize::from(!self.hidden_size_reported))
            .saturating_add(usize::from(!self.local_window_tokens_reported))
            .saturating_add(usize::from(!self.selected_adapter_reported))
            .saturating_add(usize::from(!self.hot_kv_precision_reported))
            .saturating_add(usize::from(!self.cold_kv_precision_reported))
    }

    pub fn runtime_diagnostics_contract_admission_signal_component_count(self) -> usize {
        usize::from(self.identity_contract_is_ready())
            .saturating_add(usize::from(self.architecture_contract_is_ready()))
            .saturating_add(usize::from(self.adapter_contract_is_ready()))
            .saturating_add(usize::from(self.precision_contract_is_ready()))
    }

    pub fn has_runtime_diagnostics_contract_admission_signals(self) -> bool {
        self.runtime_diagnostics_contract_admission_signal_component_count() > 0
    }

    pub fn runtime_diagnostics_contract_admission_blocker_component_count(self) -> usize {
        self.missing_contract_report_component_count()
            .saturating_add(self.diagnostics_contract_problem_component_count())
    }

    pub fn has_runtime_diagnostics_contract_admission_blockers(self) -> bool {
        self.runtime_diagnostics_contract_admission_blocker_component_count() > 0
    }

    pub fn runtime_diagnostics_contract_admission_accounting_is_consistent(self) -> bool {
        let expected_signal_count = usize::from(self.identity_contract_is_ready())
            .saturating_add(usize::from(self.architecture_contract_is_ready()))
            .saturating_add(usize::from(self.adapter_contract_is_ready()))
            .saturating_add(usize::from(self.precision_contract_is_ready()));
        let expected_blocker_count = self
            .missing_contract_report_component_count()
            .saturating_add(self.diagnostics_contract_problem_component_count());

        self.diagnostics_contract_accounting_is_consistent()
            && self.runtime_diagnostics_contract_admission_signal_component_count()
                == expected_signal_count
            && self.has_runtime_diagnostics_contract_admission_signals()
                == (expected_signal_count > 0)
            && self.runtime_diagnostics_contract_admission_blocker_component_count()
                == expected_blocker_count
            && self.has_runtime_diagnostics_contract_admission_blockers()
                == (expected_blocker_count > 0)
    }

    pub fn runtime_diagnostics_contract_admission_is_clean(self) -> bool {
        !self.has_runtime_diagnostics_contract_admission_blockers()
            && self.runtime_diagnostics_contract_admission_accounting_is_consistent()
    }

    pub fn can_admit_runtime_diagnostics_contract(self) -> bool {
        self.can_accept_runtime_diagnostics_contract()
            && self.runtime_diagnostics_contract_admission_is_clean()
    }
}

impl RuntimeHardwareDiagnosticsContractSummary {
    pub fn device_profile_contract_is_clean(self) -> bool {
        if self.device_profile_reported {
            self.device_profile_known && self.device_profile_matches_hardware
        } else {
            !self.device_profile_known && !self.device_profile_matches_hardware
        }
    }

    pub fn primary_lane_contract_is_clean(self) -> bool {
        if self.primary_lane_reported {
            self.primary_lane_known && self.primary_lane_matches_hardware
        } else {
            !self.primary_lane_known && !self.primary_lane_matches_hardware
        }
    }

    pub fn fallback_lane_contract_is_clean(self) -> bool {
        if self.fallback_lane_reported {
            self.fallback_lane_known && self.fallback_lane_matches_hardware
        } else {
            !self.fallback_lane_known && !self.fallback_lane_matches_hardware
        }
    }

    pub fn memory_mode_contract_is_clean(self) -> bool {
        if self.memory_mode_reported {
            self.memory_mode_known && self.memory_mode_matches_hardware
        } else {
            !self.memory_mode_known && !self.memory_mode_matches_hardware
        }
    }

    pub fn device_profile_contract_is_ready(self) -> bool {
        self.device_profile_reported
            && self.device_profile_known
            && self.device_profile_matches_hardware
    }

    pub fn primary_lane_contract_is_ready(self) -> bool {
        self.primary_lane_reported && self.primary_lane_known && self.primary_lane_matches_hardware
    }

    pub fn fallback_lane_contract_is_ready(self) -> bool {
        self.fallback_lane_reported
            && self.fallback_lane_known
            && self.fallback_lane_matches_hardware
    }

    pub fn memory_mode_contract_is_ready(self) -> bool {
        self.memory_mode_reported && self.memory_mode_known && self.memory_mode_matches_hardware
    }

    pub fn device_profile_problem_component_count(self) -> usize {
        usize::from(!self.device_profile_contract_is_clean())
    }

    pub fn lane_problem_component_count(self) -> usize {
        usize::from(!self.primary_lane_contract_is_clean())
            .saturating_add(usize::from(!self.fallback_lane_contract_is_clean()))
    }

    pub fn memory_mode_problem_component_count(self) -> usize {
        usize::from(!self.memory_mode_contract_is_clean())
    }

    pub fn hardware_contract_problem_component_count(self) -> usize {
        self.device_profile_problem_component_count()
            .saturating_add(self.lane_problem_component_count())
            .saturating_add(self.memory_mode_problem_component_count())
    }

    pub fn has_hardware_contract_problem_components(self) -> bool {
        self.hardware_contract_problem_component_count() > 0
    }

    pub fn hardware_contract_signal_component_count(self) -> usize {
        usize::from(self.device_profile_reported)
            .saturating_add(usize::from(self.device_profile_known))
            .saturating_add(usize::from(self.device_profile_matches_hardware))
            .saturating_add(usize::from(self.primary_lane_reported))
            .saturating_add(usize::from(self.primary_lane_known))
            .saturating_add(usize::from(self.primary_lane_matches_hardware))
            .saturating_add(usize::from(self.fallback_lane_reported))
            .saturating_add(usize::from(self.fallback_lane_known))
            .saturating_add(usize::from(self.fallback_lane_matches_hardware))
            .saturating_add(usize::from(self.memory_mode_reported))
            .saturating_add(usize::from(self.memory_mode_known))
            .saturating_add(usize::from(self.memory_mode_matches_hardware))
    }

    pub fn has_hardware_contract_signals(self) -> bool {
        self.hardware_contract_signal_component_count() > 0
    }

    pub fn hardware_contract_accounting_is_consistent(self) -> bool {
        let expected_problem_count = self
            .device_profile_problem_component_count()
            .saturating_add(self.lane_problem_component_count())
            .saturating_add(self.memory_mode_problem_component_count());
        let expected_signal_count = usize::from(self.device_profile_reported)
            .saturating_add(usize::from(self.device_profile_known))
            .saturating_add(usize::from(self.device_profile_matches_hardware))
            .saturating_add(usize::from(self.primary_lane_reported))
            .saturating_add(usize::from(self.primary_lane_known))
            .saturating_add(usize::from(self.primary_lane_matches_hardware))
            .saturating_add(usize::from(self.fallback_lane_reported))
            .saturating_add(usize::from(self.fallback_lane_known))
            .saturating_add(usize::from(self.fallback_lane_matches_hardware))
            .saturating_add(usize::from(self.memory_mode_reported))
            .saturating_add(usize::from(self.memory_mode_known))
            .saturating_add(usize::from(self.memory_mode_matches_hardware));

        self.hardware_contract_problem_component_count() == expected_problem_count
            && self.has_hardware_contract_problem_components() == (expected_problem_count > 0)
            && self.hardware_contract_signal_component_count() == expected_signal_count
            && self.has_hardware_contract_signals() == (expected_signal_count > 0)
    }

    pub fn hardware_contract_shape_is_clean(self) -> bool {
        !self.has_hardware_contract_problem_components()
            && self.hardware_contract_accounting_is_consistent()
    }

    pub fn can_accept_runtime_hardware_contract(self) -> bool {
        self.hardware_contract_shape_is_clean()
    }

    pub fn missing_hardware_contract_report_component_count(self) -> usize {
        usize::from(!self.device_profile_reported)
            .saturating_add(usize::from(!self.primary_lane_reported))
            .saturating_add(usize::from(!self.fallback_lane_reported))
            .saturating_add(usize::from(!self.memory_mode_reported))
    }

    pub fn runtime_hardware_contract_admission_signal_component_count(self) -> usize {
        usize::from(self.device_profile_contract_is_ready())
            .saturating_add(usize::from(self.primary_lane_contract_is_ready()))
            .saturating_add(usize::from(self.fallback_lane_contract_is_ready()))
            .saturating_add(usize::from(self.memory_mode_contract_is_ready()))
    }

    pub fn has_runtime_hardware_contract_admission_signals(self) -> bool {
        self.runtime_hardware_contract_admission_signal_component_count() > 0
    }

    pub fn runtime_hardware_contract_admission_blocker_component_count(self) -> usize {
        self.missing_hardware_contract_report_component_count()
            .saturating_add(self.hardware_contract_problem_component_count())
    }

    pub fn has_runtime_hardware_contract_admission_blockers(self) -> bool {
        self.runtime_hardware_contract_admission_blocker_component_count() > 0
    }

    pub fn runtime_hardware_contract_admission_accounting_is_consistent(self) -> bool {
        let expected_signal_count = usize::from(self.device_profile_contract_is_ready())
            .saturating_add(usize::from(self.primary_lane_contract_is_ready()))
            .saturating_add(usize::from(self.fallback_lane_contract_is_ready()))
            .saturating_add(usize::from(self.memory_mode_contract_is_ready()));
        let expected_blocker_count = self
            .missing_hardware_contract_report_component_count()
            .saturating_add(self.hardware_contract_problem_component_count());

        self.hardware_contract_accounting_is_consistent()
            && self.runtime_hardware_contract_admission_signal_component_count()
                == expected_signal_count
            && self.has_runtime_hardware_contract_admission_signals() == (expected_signal_count > 0)
            && self.runtime_hardware_contract_admission_blocker_component_count()
                == expected_blocker_count
            && self.has_runtime_hardware_contract_admission_blockers()
                == (expected_blocker_count > 0)
    }

    pub fn runtime_hardware_contract_admission_is_clean(self) -> bool {
        !self.has_runtime_hardware_contract_admission_blockers()
            && self.runtime_hardware_contract_admission_accounting_is_consistent()
    }

    pub fn can_admit_runtime_hardware_contract(self) -> bool {
        self.can_accept_runtime_hardware_contract()
            && self.runtime_hardware_contract_admission_is_clean()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeHardwareDiagnosticsReport {
    pub hardware_violations: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeHardwareDiagnosticsSummary {
    pub accepted: bool,
    pub hardware_violation_count: usize,
    pub failure_report_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeDeviceExecutionEnvelopeSummary {
    pub runtime_diagnostics_contract_admitted: bool,
    pub hardware_contract_admitted: bool,
    pub hardware_diagnostics_admitted: bool,
}

impl RuntimeHardwareDiagnosticsSummary {
    pub fn has_hardware_violations(self) -> bool {
        self.hardware_violation_count > 0
    }

    pub fn hardware_failure_component_count(self) -> usize {
        usize::from(self.has_hardware_violations())
    }

    pub fn has_failure_reports(self) -> bool {
        self.failure_report_count > 0
    }

    pub fn mapped_failure_report_component_count(self) -> usize {
        usize::from(self.has_failure_reports())
    }

    pub fn hardware_acceptance_problem_component_count(self) -> usize {
        self.hardware_failure_component_count()
            .saturating_add(self.mapped_failure_report_component_count())
    }

    pub fn has_hardware_acceptance_problem_components(self) -> bool {
        self.hardware_acceptance_problem_component_count() > 0
    }

    pub fn failure_report_matches_violations(self) -> bool {
        self.failure_report_count == usize::from(self.hardware_violation_count > 0)
    }

    pub fn hardware_acceptance_accounting_is_consistent(self) -> bool {
        let expected_failure_count = usize::from(self.has_hardware_violations());
        let expected_problem_count =
            expected_failure_count.saturating_add(usize::from(self.has_failure_reports()));

        self.hardware_failure_component_count() == expected_failure_count
            && self.hardware_acceptance_problem_component_count() == expected_problem_count
            && self.has_hardware_acceptance_problem_components() == (expected_problem_count > 0)
            && self.failure_report_matches_violations()
            && self.accepted == (self.hardware_violation_count == 0)
    }

    pub fn is_clean_acceptance(self) -> bool {
        self.accepted
            && self.hardware_violation_count == 0
            && self.failure_report_count == 0
            && self.hardware_acceptance_accounting_is_consistent()
    }

    pub fn hardware_acceptance_shape_is_clean(self) -> bool {
        self.is_clean_acceptance()
    }

    pub fn can_accept_runtime_hardware_diagnostics(self) -> bool {
        self.hardware_acceptance_shape_is_clean()
    }

    pub fn runtime_hardware_admission_signal_component_count(self) -> usize {
        usize::from(self.accepted)
    }

    pub fn has_runtime_hardware_admission_signals(self) -> bool {
        self.runtime_hardware_admission_signal_component_count() > 0
    }

    pub fn runtime_hardware_admission_blocker_component_count(self) -> usize {
        self.hardware_acceptance_problem_component_count()
            .saturating_add(usize::from(!self.accepted))
    }

    pub fn has_runtime_hardware_admission_blockers(self) -> bool {
        self.runtime_hardware_admission_blocker_component_count() > 0
    }

    pub fn runtime_hardware_admission_accounting_is_consistent(self) -> bool {
        let expected_signal_count = usize::from(self.accepted);
        let expected_blocker_count = self
            .hardware_acceptance_problem_component_count()
            .saturating_add(usize::from(!self.accepted));

        self.hardware_acceptance_accounting_is_consistent()
            && self.runtime_hardware_admission_signal_component_count() == expected_signal_count
            && self.has_runtime_hardware_admission_signals() == (expected_signal_count > 0)
            && self.runtime_hardware_admission_blocker_component_count() == expected_blocker_count
            && self.has_runtime_hardware_admission_blockers() == (expected_blocker_count > 0)
    }

    pub fn runtime_hardware_admission_is_clean(self) -> bool {
        !self.has_runtime_hardware_admission_blockers()
            && self.runtime_hardware_admission_accounting_is_consistent()
    }

    pub fn can_admit_runtime_hardware_diagnostics(self) -> bool {
        self.can_accept_runtime_hardware_diagnostics() && self.runtime_hardware_admission_is_clean()
    }
}

impl RuntimeDeviceExecutionEnvelopeSummary {
    pub fn from_admission_summaries(
        runtime_contract: RuntimeDiagnosticsContractSummary,
        hardware_contract: RuntimeHardwareDiagnosticsContractSummary,
        hardware_diagnostics: RuntimeHardwareDiagnosticsSummary,
    ) -> Self {
        Self {
            runtime_diagnostics_contract_admitted: runtime_contract
                .can_admit_runtime_diagnostics_contract(),
            hardware_contract_admitted: hardware_contract.can_admit_runtime_hardware_contract(),
            hardware_diagnostics_admitted: hardware_diagnostics
                .can_admit_runtime_hardware_diagnostics(),
        }
    }

    pub fn runtime_device_execution_envelope_admission_signal_component_count(self) -> usize {
        usize::from(self.runtime_diagnostics_contract_admitted)
            .saturating_add(usize::from(self.hardware_contract_admitted))
            .saturating_add(usize::from(self.hardware_diagnostics_admitted))
    }

    pub fn has_runtime_device_execution_envelope_admission_signals(self) -> bool {
        self.runtime_device_execution_envelope_admission_signal_component_count() > 0
    }

    pub fn runtime_device_execution_envelope_admission_blocker_component_count(self) -> usize {
        usize::from(!self.runtime_diagnostics_contract_admitted)
            .saturating_add(usize::from(!self.hardware_contract_admitted))
            .saturating_add(usize::from(!self.hardware_diagnostics_admitted))
    }

    pub fn has_runtime_device_execution_envelope_admission_blockers(self) -> bool {
        self.runtime_device_execution_envelope_admission_blocker_component_count() > 0
    }

    pub fn runtime_device_execution_envelope_admission_accounting_is_consistent(self) -> bool {
        let expected_signal_count = usize::from(self.runtime_diagnostics_contract_admitted)
            .saturating_add(usize::from(self.hardware_contract_admitted))
            .saturating_add(usize::from(self.hardware_diagnostics_admitted));
        let expected_blocker_count = usize::from(!self.runtime_diagnostics_contract_admitted)
            .saturating_add(usize::from(!self.hardware_contract_admitted))
            .saturating_add(usize::from(!self.hardware_diagnostics_admitted));

        self.runtime_device_execution_envelope_admission_signal_component_count()
            == expected_signal_count
            && self.has_runtime_device_execution_envelope_admission_signals()
                == (expected_signal_count > 0)
            && self.runtime_device_execution_envelope_admission_blocker_component_count()
                == expected_blocker_count
            && self.has_runtime_device_execution_envelope_admission_blockers()
                == (expected_blocker_count > 0)
            && expected_signal_count.saturating_add(expected_blocker_count) == 3
    }

    pub fn runtime_device_execution_envelope_admission_is_clean(self) -> bool {
        !self.has_runtime_device_execution_envelope_admission_blockers()
            && self.runtime_device_execution_envelope_admission_accounting_is_consistent()
    }

    pub fn can_submit_runtime_device_execution_envelope(self) -> bool {
        self.runtime_device_execution_envelope_admission_is_clean()
    }
}

impl RuntimeHardwareDiagnosticsReport {
    pub fn is_accepted(&self) -> bool {
        self.hardware_violations.is_empty()
    }

    pub fn violations(&self) -> &[String] {
        &self.hardware_violations
    }

    pub fn failure_reports(&self) -> Vec<RuntimeFailureReport> {
        if self.hardware_violations.is_empty() {
            Vec::new()
        } else {
            vec![RuntimeFailureReport::contract_violation(
                acceptance_message(
                    "runtime hardware diagnostics acceptance failed",
                    &self.hardware_violations,
                ),
            )]
        }
    }

    pub fn failure_batch_summary(&self) -> RuntimeFailureBatchSummary {
        RuntimeFailureReport::batch_summary(&self.failure_reports())
    }

    pub fn primary_failure_report(&self) -> Option<RuntimeFailureReport> {
        self.failure_reports().into_iter().next()
    }

    pub fn primary_failure_summary(&self) -> Option<RuntimeFailureSummary> {
        self.primary_failure_report()
            .map(|report| report.failure_summary())
    }

    pub fn diagnostics_summary(&self) -> RuntimeHardwareDiagnosticsSummary {
        RuntimeHardwareDiagnosticsSummary {
            accepted: self.is_accepted(),
            hardware_violation_count: self.hardware_violations.len(),
            failure_report_count: usize::from(!self.hardware_violations.is_empty()),
        }
    }
}

impl RuntimeDiagnostics {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_request_envelope(request: &RuntimeRequestEnvelope) -> Self {
        Self::default().with_request_envelope(request)
    }

    pub fn with_request_envelope(mut self, request: &RuntimeRequestEnvelope) -> Self {
        if self.model_id.is_none() {
            self.model_id = non_empty_string(request.runtime.model_id.clone());
        }
        if self.selected_adapter.is_none() {
            self.selected_adapter = request.selected_adapter;
        }
        if self.layer_count == 0 {
            self.layer_count = request.architecture.layer_count;
        }
        if self.hidden_size == 0 {
            self.hidden_size = request.architecture.hidden_size;
        }
        if self.local_window_tokens == 0 {
            self.local_window_tokens = request.architecture.local_window_tokens;
        }
        if self.imported_kv_blocks == 0 {
            self.imported_kv_blocks = request.imported_kv_blocks;
        }
        if self.hot_kv_precision_bits.is_none() {
            self.hot_kv_precision_bits = Some(request.runtime.hot_kv_precision_bits);
        }
        if self.cold_kv_precision_bits.is_none() {
            self.cold_kv_precision_bits = Some(
                request
                    .runtime
                    .cold_kv_precision_bits
                    .min(request.runtime.hot_kv_precision_bits),
            );
        }

        self
    }

    pub fn with_model_id(mut self, model_id: impl Into<String>) -> Self {
        self.model_id = non_empty_string(model_id.into());
        self
    }

    pub fn with_selected_adapter(mut self, adapter: RuntimeAdapter) -> Self {
        self.selected_adapter = Some(adapter);
        self
    }

    pub fn with_layer_modes(mut self, global: usize, local_window: usize, fusion: usize) -> Self {
        self.global_layers = global;
        self.local_window_layers = local_window;
        self.fusion_layers = fusion;
        self
    }

    pub fn with_architecture(
        mut self,
        layer_count: usize,
        hidden_size: usize,
        local_window_tokens: usize,
    ) -> Self {
        self.layer_count = layer_count;
        self.hidden_size = hidden_size;
        self.local_window_tokens = local_window_tokens;
        self
    }

    pub fn with_device_execution(
        mut self,
        device_profile: impl Into<String>,
        primary_lane: impl Into<String>,
        fallback_lane: impl Into<String>,
        memory_mode: impl Into<String>,
        source: DeviceExecutionSource,
    ) -> Self {
        self.device_profile = non_empty_string(device_profile.into());
        self.primary_lane = non_empty_string(primary_lane.into());
        self.fallback_lane = non_empty_string(fallback_lane.into());
        self.memory_mode = non_empty_string(memory_mode.into());
        self.device_execution_source = self.has_device_execution_signal().then_some(source);
        self
    }

    pub fn clear_device_execution(mut self) -> Self {
        self.device_profile = None;
        self.primary_lane = None;
        self.fallback_lane = None;
        self.memory_mode = None;
        self.device_execution_source = None;
        self
    }

    pub fn with_forward_signals(
        mut self,
        forward_energy: Option<f32>,
        kv_influence: Option<f32>,
    ) -> Self {
        self.forward_energy = finite_option(forward_energy);
        self.kv_influence = finite_option(kv_influence);
        self
    }

    pub fn with_kv_exchange(mut self, imported: usize, exported: usize) -> Self {
        self.imported_kv_blocks = imported;
        self.exported_kv_blocks = exported;
        self
    }

    pub fn with_weak_runtime_kv_imports_skipped(mut self, skipped: usize) -> Self {
        self.weak_runtime_kv_imports_skipped = skipped;
        self
    }

    pub fn with_kv_precision(mut self, hot_bits: u8, cold_bits: u8) -> Self {
        if valid_kv_precision(hot_bits, cold_bits) {
            self.hot_kv_precision_bits = Some(hot_bits);
            self.cold_kv_precision_bits = Some(cold_bits);
        }
        self
    }

    pub fn clear_kv_precision(mut self) -> Self {
        self.hot_kv_precision_bits = None;
        self.cold_kv_precision_bits = None;
        self
    }

    pub fn layer_mode_count(&self) -> usize {
        self.global_layers
            .saturating_add(self.local_window_layers)
            .saturating_add(self.fusion_layers)
    }

    pub fn has_layer_mode_signal(&self) -> bool {
        self.layer_mode_count() > 0
    }

    pub fn has_all_layer_modes(&self) -> bool {
        self.global_layers > 0 && self.local_window_layers > 0 && self.fusion_layers > 0
    }

    pub fn has_device_profile_signal(&self) -> bool {
        has_text(self.device_profile.as_deref())
    }

    pub fn has_device_execution_signal(&self) -> bool {
        self.has_device_profile_signal()
            && has_text(self.primary_lane.as_deref())
            && has_text(self.fallback_lane.as_deref())
            && has_text(self.memory_mode.as_deref())
    }

    pub fn has_runtime_reported_device_execution_signal(&self) -> bool {
        self.has_device_execution_signal()
            && self.device_execution_source == Some(DeviceExecutionSource::RuntimeReported)
    }

    pub fn has_control_plane_filled_device_execution_signal(&self) -> bool {
        self.has_device_execution_signal()
            && self.device_execution_source == Some(DeviceExecutionSource::ControlPlaneFilled)
    }

    pub fn has_runtime_architecture_signal(&self) -> bool {
        has_text(self.model_id.as_deref())
            && self.layer_count > 0
            && self.hidden_size > 0
            && self.local_window_tokens > 0
    }

    pub fn has_valid_kv_precision_signal(&self) -> bool {
        match (self.hot_kv_precision_bits, self.cold_kv_precision_bits) {
            (Some(hot), Some(cold)) => valid_kv_precision(hot, cold),
            _ => false,
        }
    }

    pub fn has_forward_signal(&self) -> bool {
        self.layer_count > 0
            || self.has_layer_mode_signal()
            || self.has_device_execution_signal()
            || self.forward_energy.is_some()
            || self.kv_influence.is_some()
            || self.imported_kv_blocks > 0
            || self.exported_kv_blocks > 0
            || self.weak_runtime_kv_imports_skipped > 0
    }

    pub fn contract_summary(
        &self,
        metadata: &RuntimeMetadata,
        architecture: TransformerRuntimeArchitecture,
        execution: &AdapterExecutionContext,
    ) -> RuntimeDiagnosticsContractSummary {
        let model_id_reported = self
            .model_id
            .as_ref()
            .is_some_and(|model_id| !model_id.is_empty());
        let layer_count_reported = self.layer_count > 0;
        let hidden_size_reported = self.hidden_size > 0;
        let local_window_tokens_reported = self.local_window_tokens > 0;
        let selected_adapter_reported = self.selected_adapter.is_some();
        let hot_kv_precision_reported = self.hot_kv_precision_bits.is_some();
        let cold_kv_precision_reported = self.cold_kv_precision_bits.is_some();
        let kv_precision_pair_reported =
            self.hot_kv_precision_bits.is_some() && self.cold_kv_precision_bits.is_some();

        RuntimeDiagnosticsContractSummary {
            model_id_reported,
            model_id_matches_metadata: self.model_id.as_deref() == Some(metadata.model_id.as_str()),
            layer_count_reported,
            layer_count_matches_architecture: self.layer_count == architecture.layer_count,
            hidden_size_reported,
            hidden_size_matches_architecture: self.hidden_size == architecture.hidden_size,
            local_window_tokens_reported,
            local_window_tokens_within_context: metadata.native_context_window == 0
                || self.local_window_tokens <= metadata.native_context_window,
            selected_adapter_reported,
            selected_adapter_within_execution: self
                .selected_adapter
                .is_some_and(|adapter| execution.adapters.contains(&adapter)),
            hot_kv_precision_reported,
            hot_kv_precision_within_metadata: self
                .hot_kv_precision_bits
                .is_some_and(|hot_bits| hot_bits <= metadata.hot_kv_precision_bits),
            cold_kv_precision_reported,
            cold_kv_precision_within_metadata: self
                .cold_kv_precision_bits
                .is_some_and(|cold_bits| cold_bits <= metadata.cold_kv_precision_bits),
            kv_precision_pair_reported,
            kv_precision_pair_valid: self.has_valid_kv_precision_signal(),
        }
    }

    pub fn contract_violations(
        &self,
        metadata: &RuntimeMetadata,
        architecture: TransformerRuntimeArchitecture,
        execution: &AdapterExecutionContext,
    ) -> Vec<String> {
        let mut violations = Vec::new();

        if let Some(model_id) = self.model_id.as_deref()
            && model_id != metadata.model_id
        {
            violations.push(format!(
                "runtime diagnostics model_id {model_id} differs from metadata {}",
                metadata.model_id
            ));
        }
        if self.layer_count > 0 && self.layer_count != architecture.layer_count {
            violations.push(format!(
                "runtime diagnostics layer_count {} differs from architecture {}",
                self.layer_count, architecture.layer_count
            ));
        }
        if self.hidden_size > 0 && self.hidden_size != architecture.hidden_size {
            violations.push(format!(
                "runtime diagnostics hidden_size {} differs from architecture {}",
                self.hidden_size, architecture.hidden_size
            ));
        }
        if self.local_window_tokens > metadata.native_context_window
            && metadata.native_context_window > 0
        {
            violations.push(format!(
                "runtime diagnostics local_window_tokens {} exceeds native context {}",
                self.local_window_tokens, metadata.native_context_window
            ));
        }
        if let Some(adapter) = self.selected_adapter
            && !execution.adapters.contains(&adapter)
        {
            violations.push(format!(
                "runtime diagnostics selected adapter {} is outside execution context",
                adapter.as_str()
            ));
        }
        if let Some(hot_bits) = self.hot_kv_precision_bits
            && hot_bits > metadata.hot_kv_precision_bits
        {
            violations.push(format!(
                "runtime diagnostics hot KV precision {hot_bits} exceeds metadata {}",
                metadata.hot_kv_precision_bits
            ));
        }
        if let Some(cold_bits) = self.cold_kv_precision_bits
            && cold_bits > metadata.cold_kv_precision_bits
        {
            violations.push(format!(
                "runtime diagnostics cold KV precision {cold_bits} exceeds metadata {}",
                metadata.cold_kv_precision_bits
            ));
        }
        if let (Some(hot), Some(cold)) = (self.hot_kv_precision_bits, self.cold_kv_precision_bits)
            && !valid_kv_precision(hot, cold)
        {
            violations.push("runtime diagnostics KV precision is invalid".to_owned());
        }

        violations
    }

    pub fn hardware_contract_summary(
        &self,
        hardware: &HardwarePlan,
    ) -> RuntimeHardwareDiagnosticsContractSummary {
        let device_profile_reported = has_text(self.device_profile.as_deref());
        let device_profile = self
            .device_profile
            .as_deref()
            .and_then(|device_profile| device_profile.parse::<DeviceClass>().ok());
        let primary_lane_reported = has_text(self.primary_lane.as_deref());
        let primary_lane = self
            .primary_lane
            .as_deref()
            .and_then(|primary_lane| primary_lane.parse::<ComputeLane>().ok());
        let fallback_lane_reported = has_text(self.fallback_lane.as_deref());
        let fallback_lane = self
            .fallback_lane
            .as_deref()
            .and_then(|fallback_lane| fallback_lane.parse::<ComputeLane>().ok());
        let memory_mode_reported = has_text(self.memory_mode.as_deref());
        let memory_mode = self
            .memory_mode
            .as_deref()
            .and_then(|memory_mode| memory_mode.parse::<DeviceMemoryMode>().ok());

        RuntimeHardwareDiagnosticsContractSummary {
            device_profile_reported,
            device_profile_known: device_profile.is_some(),
            device_profile_matches_hardware: device_profile == Some(hardware.device),
            primary_lane_reported,
            primary_lane_known: primary_lane.is_some(),
            primary_lane_matches_hardware: primary_lane == Some(hardware.execution.primary_lane),
            fallback_lane_reported,
            fallback_lane_known: fallback_lane.is_some(),
            fallback_lane_matches_hardware: fallback_lane == Some(hardware.execution.fallback_lane),
            memory_mode_reported,
            memory_mode_known: memory_mode.is_some(),
            memory_mode_matches_hardware: memory_mode == Some(hardware.execution.memory_mode),
        }
    }

    pub fn hardware_contract_violations(&self, hardware: &HardwarePlan) -> Vec<String> {
        let mut violations = Vec::new();

        if let Some(device_profile) = self.device_profile.as_deref() {
            match device_profile.parse::<DeviceClass>() {
                Ok(device) if device != hardware.device => violations.push(format!(
                    "runtime diagnostics device_profile {} differs from request device {}",
                    device.as_str(),
                    hardware.device.as_str()
                )),
                Ok(_) => {}
                Err(_) => violations.push(format!(
                    "runtime diagnostics unknown device_profile {device_profile}"
                )),
            }
        }

        if let Some(primary_lane) = self.primary_lane.as_deref() {
            match primary_lane.parse::<ComputeLane>() {
                Ok(lane) if lane != hardware.execution.primary_lane => violations.push(format!(
                    "runtime diagnostics primary_lane {} differs from request primary {}",
                    lane.as_str(),
                    hardware.execution.primary_lane.as_str()
                )),
                Ok(_) => {}
                Err(_) => violations.push(format!(
                    "runtime diagnostics unknown primary_lane {primary_lane}"
                )),
            }
        }

        if let Some(fallback_lane) = self.fallback_lane.as_deref() {
            match fallback_lane.parse::<ComputeLane>() {
                Ok(lane) if lane != hardware.execution.fallback_lane => violations.push(format!(
                    "runtime diagnostics fallback_lane {} differs from request fallback {}",
                    lane.as_str(),
                    hardware.execution.fallback_lane.as_str()
                )),
                Ok(_) => {}
                Err(_) => violations.push(format!(
                    "runtime diagnostics unknown fallback_lane {fallback_lane}"
                )),
            }
        }

        if let Some(memory_mode) = self.memory_mode.as_deref() {
            match memory_mode.parse::<DeviceMemoryMode>() {
                Ok(mode) if mode != hardware.execution.memory_mode => violations.push(format!(
                    "runtime diagnostics memory_mode {} differs from request memory {}",
                    mode.as_str(),
                    hardware.execution.memory_mode.as_str()
                )),
                Ok(_) => {}
                Err(_) => violations.push(format!(
                    "runtime diagnostics unknown memory_mode {memory_mode}"
                )),
            }
        }

        violations
    }

    pub fn hardware_acceptance_report(
        &self,
        hardware: &HardwarePlan,
    ) -> RuntimeHardwareDiagnosticsReport {
        RuntimeHardwareDiagnosticsReport {
            hardware_violations: self.hardware_contract_violations(hardware),
        }
    }

    pub fn device_execution_envelope_summary(
        &self,
        metadata: &RuntimeMetadata,
        architecture: TransformerRuntimeArchitecture,
        execution: &AdapterExecutionContext,
        hardware: &HardwarePlan,
    ) -> RuntimeDeviceExecutionEnvelopeSummary {
        let runtime_contract = self.contract_summary(metadata, architecture, execution);
        let hardware_contract = self.hardware_contract_summary(hardware);
        let hardware_diagnostics = self
            .hardware_acceptance_report(hardware)
            .diagnostics_summary();

        RuntimeDeviceExecutionEnvelopeSummary::from_admission_summaries(
            runtime_contract,
            hardware_contract,
            hardware_diagnostics,
        )
    }

    pub fn can_submit_runtime_reported_device_execution_envelope(
        &self,
        metadata: &RuntimeMetadata,
        architecture: TransformerRuntimeArchitecture,
        execution: &AdapterExecutionContext,
        hardware: &HardwarePlan,
    ) -> bool {
        self.can_admit_runtime_reported_device_execution_metadata()
            && self
                .device_execution_envelope_summary(metadata, architecture, execution, hardware)
                .can_submit_runtime_device_execution_envelope()
    }

    pub fn request_parity_summary(
        &self,
        request: &RuntimeRequestEnvelope,
    ) -> RuntimeDiagnosticsRequestParitySummary {
        let model_id_reported = self
            .model_id
            .as_ref()
            .is_some_and(|model_id| !model_id.is_empty());
        let selected_adapter_reported = self.selected_adapter.is_some();
        let architecture_reported =
            self.layer_count > 0 && self.hidden_size > 0 && self.local_window_tokens > 0;
        let runtime_export_enabled = request.runtime.supports_kv_export;
        let exported_kv_within_runtime = if runtime_export_enabled {
            request.runtime.max_kv_export_blocks == 0
                || self.exported_kv_blocks <= request.runtime.max_kv_export_blocks
        } else {
            self.exported_kv_blocks == 0
        };
        let kv_precision_reported =
            self.hot_kv_precision_bits.is_some() && self.cold_kv_precision_bits.is_some();
        let kv_precision_valid = self.has_valid_kv_precision_signal();
        let kv_precision_within_request =
            match (self.hot_kv_precision_bits, self.cold_kv_precision_bits) {
                (Some(hot), Some(cold)) => {
                    hot <= request.runtime.hot_kv_precision_bits
                        && cold <= request.runtime.cold_kv_precision_bits
                }
                _ => false,
            };

        RuntimeDiagnosticsRequestParitySummary {
            model_id_reported,
            model_id_matches_request: self.model_id.as_deref()
                == Some(request.runtime.model_id.as_str()),
            selected_adapter_reported,
            selected_adapter_matches_request: self.selected_adapter == request.selected_adapter,
            architecture_reported,
            layer_count_matches_request: self.layer_count == request.architecture.layer_count,
            hidden_size_matches_request: self.hidden_size == request.architecture.hidden_size,
            local_window_tokens_within_request: self.local_window_tokens > 0
                && (request.runtime.native_context_window == 0
                    || self.local_window_tokens <= request.runtime.native_context_window),
            imported_kv_blocks: self.imported_kv_blocks,
            request_imported_kv_blocks: request.imported_kv_blocks,
            imported_kv_matches_request: self.imported_kv_blocks == request.imported_kv_blocks,
            exported_kv_blocks: self.exported_kv_blocks,
            runtime_export_enabled,
            runtime_max_export_blocks: request.runtime.max_kv_export_blocks,
            exported_kv_within_runtime,
            kv_precision_reported,
            kv_precision_valid,
            kv_precision_within_request,
        }
    }

    pub fn diagnostics_summary(&self) -> RuntimeDiagnosticsSummary {
        RuntimeDiagnosticsSummary {
            has_model_id: has_text(self.model_id.as_deref()),
            has_selected_adapter: self.selected_adapter.is_some(),
            has_runtime_architecture: self.has_runtime_architecture_signal(),
            layer_count: self.layer_count,
            layer_mode_count: self.layer_mode_count(),
            has_all_layer_modes: self.has_all_layer_modes(),
            has_device_execution: self.has_device_execution_signal(),
            device_execution_source: self.device_execution_source,
            has_forward_energy: self.forward_energy.is_some(),
            has_kv_influence: self.kv_influence.is_some(),
            imported_kv_blocks: self.imported_kv_blocks,
            exported_kv_blocks: self.exported_kv_blocks,
            weak_runtime_kv_imports_skipped: self.weak_runtime_kv_imports_skipped,
            has_valid_kv_precision: self.has_valid_kv_precision_signal(),
            has_forward_signal: self.has_forward_signal(),
        }
    }

    pub fn can_admit_runtime_reported_device_execution_metadata(&self) -> bool {
        self.diagnostics_summary()
            .can_admit_hardware_adapter_metadata()
    }
}

fn acceptance_message(prefix: &str, violations: &[String]) -> String {
    if violations.is_empty() {
        prefix.to_owned()
    } else {
        format!("{prefix}: {}", violations.join("; "))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingSource {
    Runtime,
    Fallback,
}

impl EmbeddingSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Runtime => "runtime",
            Self::Fallback => "fallback",
        }
    }
}

impl Default for EmbeddingSource {
    fn default() -> Self {
        Self::Fallback
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EmbeddingCallDiagnostics {
    pub source: EmbeddingSource,
    pub dimensions: usize,
}

impl EmbeddingCallDiagnostics {
    pub fn new(source: EmbeddingSource, dimensions: usize) -> Self {
        Self { source, dimensions }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EmbeddingDiagnostics {
    pub query: EmbeddingCallDiagnostics,
    pub memory_write: Option<EmbeddingCallDiagnostics>,
    pub gist_writes: Vec<EmbeddingCallDiagnostics>,
    pub runtime_calls: usize,
    pub fallback_calls: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmbeddingDiagnosticsSummary {
    pub query_source: EmbeddingSource,
    pub query_dimensions: usize,
    pub has_memory_write: bool,
    pub memory_write_source: Option<EmbeddingSource>,
    pub memory_write_dimensions: usize,
    pub gist_write_count: usize,
    pub gist_runtime_write_count: usize,
    pub gist_fallback_write_count: usize,
    pub runtime_calls: usize,
    pub fallback_calls: usize,
    pub total_calls: usize,
}

impl EmbeddingDiagnostics {
    pub fn from_query(query: EmbeddingCallDiagnostics) -> Self {
        let mut diagnostics = Self {
            query,
            ..Self::default()
        };
        diagnostics.record_call(query);
        diagnostics
    }

    pub fn record_memory_write(&mut self, call: EmbeddingCallDiagnostics) {
        self.memory_write = Some(call);
        self.record_call(call);
    }

    pub fn record_gist_write(&mut self, call: EmbeddingCallDiagnostics) {
        self.gist_writes.push(call);
        self.record_call(call);
    }

    pub fn runtime_embedding_available(&self) -> bool {
        self.runtime_calls > 0
    }

    pub fn fallback_embedding_used(&self) -> bool {
        self.fallback_calls > 0
    }

    pub fn total_calls(&self) -> usize {
        1 + usize::from(self.memory_write.is_some()) + self.gist_writes.len()
    }

    pub fn gist_write_runtime_calls(&self) -> usize {
        self.gist_writes
            .iter()
            .filter(|call| call.source == EmbeddingSource::Runtime)
            .count()
    }

    pub fn gist_write_fallback_calls(&self) -> usize {
        self.gist_writes
            .iter()
            .filter(|call| call.source == EmbeddingSource::Fallback)
            .count()
    }

    pub fn diagnostics_summary(&self) -> EmbeddingDiagnosticsSummary {
        EmbeddingDiagnosticsSummary {
            query_source: self.query.source,
            query_dimensions: self.query.dimensions,
            has_memory_write: self.memory_write.is_some(),
            memory_write_source: self.memory_write.map(|call| call.source),
            memory_write_dimensions: self.memory_write.map(|call| call.dimensions).unwrap_or(0),
            gist_write_count: self.gist_writes.len(),
            gist_runtime_write_count: self.gist_write_runtime_calls(),
            gist_fallback_write_count: self.gist_write_fallback_calls(),
            runtime_calls: self.runtime_calls,
            fallback_calls: self.fallback_calls,
            total_calls: self.total_calls(),
        }
    }

    fn record_call(&mut self, call: EmbeddingCallDiagnostics) {
        match call.source {
            EmbeddingSource::Runtime => self.runtime_calls += 1,
            EmbeddingSource::Fallback => self.fallback_calls += 1,
        }
    }
}

impl EmbeddingDiagnosticsSummary {
    pub fn has_query_dimensions(self) -> bool {
        self.query_dimensions > 0
    }

    pub fn runtime_embedding_available(self) -> bool {
        self.runtime_calls > 0
    }

    pub fn fallback_embedding_used(self) -> bool {
        self.fallback_calls > 0
    }

    pub fn has_gist_writes(self) -> bool {
        self.gist_write_count > 0
    }

    pub fn has_memory_write_source(self) -> bool {
        self.memory_write_source.is_some()
    }

    pub fn uses_mixed_embedding_sources(self) -> bool {
        self.runtime_embedding_available() && self.fallback_embedding_used()
    }

    pub fn query_dimensions_shape_is_valid(self) -> bool {
        self.query_dimensions > 0
    }

    pub fn memory_write_shape_is_valid(self) -> bool {
        self.has_memory_write == self.has_memory_write_source()
            && if self.has_memory_write {
                self.memory_write_dimensions > 0
            } else {
                self.memory_write_dimensions == 0
            }
    }

    pub fn gist_writes_match_total(self) -> bool {
        self.gist_runtime_write_count
            .saturating_add(self.gist_fallback_write_count)
            == self.gist_write_count
    }

    pub fn call_counts_match_total(self) -> bool {
        self.runtime_calls.saturating_add(self.fallback_calls) == self.total_calls
    }

    pub fn embedding_signal_component_count(self) -> usize {
        usize::from(self.has_query_dimensions())
            .saturating_add(usize::from(self.has_memory_write))
            .saturating_add(usize::from(self.has_gist_writes()))
            .saturating_add(usize::from(self.runtime_embedding_available()))
            .saturating_add(usize::from(self.fallback_embedding_used()))
            .saturating_add(usize::from(self.uses_mixed_embedding_sources()))
    }

    pub fn has_embedding_signals(self) -> bool {
        self.embedding_signal_component_count() > 0
    }

    pub fn embedding_shape_problem_component_count(self) -> usize {
        usize::from(!self.query_dimensions_shape_is_valid())
            .saturating_add(usize::from(!self.memory_write_shape_is_valid()))
            .saturating_add(usize::from(!self.gist_writes_match_total()))
            .saturating_add(usize::from(!self.call_counts_match_total()))
    }

    pub fn has_embedding_shape_problem_components(self) -> bool {
        self.embedding_shape_problem_component_count() > 0
    }

    pub fn embedding_accounting_is_consistent(self) -> bool {
        let expected_signal_count = usize::from(self.has_query_dimensions())
            .saturating_add(usize::from(self.has_memory_write))
            .saturating_add(usize::from(self.has_gist_writes()))
            .saturating_add(usize::from(self.runtime_embedding_available()))
            .saturating_add(usize::from(self.fallback_embedding_used()))
            .saturating_add(usize::from(self.uses_mixed_embedding_sources()));
        let expected_problem_count = usize::from(!self.query_dimensions_shape_is_valid())
            .saturating_add(usize::from(!self.memory_write_shape_is_valid()))
            .saturating_add(usize::from(!self.gist_writes_match_total()))
            .saturating_add(usize::from(!self.call_counts_match_total()));

        self.embedding_signal_component_count() == expected_signal_count
            && self.embedding_shape_problem_component_count() == expected_problem_count
            && self.has_embedding_shape_problem_components() == (expected_problem_count > 0)
            && self.memory_write_shape_is_valid()
            && self.gist_writes_match_total()
            && self.call_counts_match_total()
    }

    pub fn embedding_summary_is_clean(self) -> bool {
        !self.has_embedding_shape_problem_components() && self.embedding_accounting_is_consistent()
    }

    pub fn can_use_embedding_diagnostics(self) -> bool {
        self.embedding_summary_is_clean() && self.has_embedding_signals()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct InferenceDiagnostics {
    pub runtime: RuntimeDiagnostics,
    pub embeddings: EmbeddingDiagnostics,
    pub route_budget: RouteBudget,
    pub generation_budget: Option<RuntimeGenerationBudget>,
    pub hardware_pressure: f32,
    pub compute_headroom: f32,
    pub latency_budget_ms: Option<u64>,
    pub recursive_runtime_calls: usize,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InferenceDiagnosticsSummary {
    pub has_generation_budget: bool,
    pub generation_truncated_by_context: bool,
    pub route_attention_tokens: usize,
    pub route_fast_tokens: usize,
    pub runtime_kv_exchange_total: usize,
    pub weak_runtime_kv_imports_skipped: usize,
    pub has_runtime_execution_signal: bool,
    pub runtime_embedding_available: bool,
    pub fallback_embedding_used: bool,
    pub hardware_pressure_band: DiagnosticsPressureBand,
    pub has_latency_budget: bool,
    pub recursive_runtime_calls: usize,
    pub note_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InferenceDiagnosticsRequestParitySummary {
    pub route_budget_matches_request: bool,
    pub generation_budget_reported: bool,
    pub generation_budget_matches_request: bool,
    pub hardware_pressure_matches_request: bool,
    pub has_planning_digest: bool,
    pub compute_headroom_matches_planning: Option<bool>,
    pub latency_budget_matches_planning: Option<bool>,
    pub runtime: RuntimeDiagnosticsRequestParitySummary,
}

impl InferenceDiagnosticsSummary {
    pub fn has_route_activity(self) -> bool {
        self.route_token_total() > 0
    }

    pub fn has_runtime_kv_exchange(self) -> bool {
        self.runtime_kv_exchange_total > 0
    }

    pub fn has_weak_runtime_kv_import_skips(self) -> bool {
        self.weak_runtime_kv_imports_skipped > 0
    }

    pub fn used_any_embedding_fallback(self) -> bool {
        self.fallback_embedding_used
    }

    pub fn has_runtime_or_embedding_execution(self) -> bool {
        self.has_runtime_execution_signal || self.runtime_embedding_available
    }

    pub fn has_recursive_runtime(self) -> bool {
        self.recursive_runtime_calls > 0
    }

    pub fn has_notes(self) -> bool {
        self.note_count > 0
    }

    pub fn route_token_total(self) -> usize {
        self.route_attention_tokens
            .saturating_add(self.route_fast_tokens)
    }

    pub fn has_complete_diagnostics_signal(self) -> bool {
        self.has_generation_budget && self.has_runtime_or_embedding_execution()
    }
}

impl InferenceDiagnosticsRequestParitySummary {
    pub fn routing_parity_ok(self) -> bool {
        self.route_budget_matches_request
    }

    pub fn generation_parity_ok(self) -> bool {
        self.generation_budget_reported && self.generation_budget_matches_request
    }

    pub fn hardware_parity_ok(self) -> bool {
        self.hardware_pressure_matches_request
            && self.compute_headroom_matches_planning.unwrap_or(true)
            && self.latency_budget_matches_planning.unwrap_or(true)
    }

    pub fn runtime_parity_ok(self) -> bool {
        self.runtime.request_parity_is_consistent()
    }

    pub fn routing_drifted(self) -> bool {
        !self.route_budget_matches_request
    }

    pub fn generation_budget_missing(self) -> bool {
        !self.generation_budget_reported
    }

    pub fn generation_budget_drifted(self) -> bool {
        self.generation_budget_reported && !self.generation_budget_matches_request
    }

    pub fn hardware_pressure_drifted(self) -> bool {
        !self.hardware_pressure_matches_request
    }

    pub fn compute_headroom_drifted(self) -> bool {
        self.compute_headroom_matches_planning == Some(false)
    }

    pub fn latency_budget_drifted(self) -> bool {
        self.latency_budget_matches_planning == Some(false)
    }

    pub fn planning_hardware_drifted(self) -> bool {
        self.compute_headroom_drifted() || self.latency_budget_drifted()
    }

    pub fn runtime_drifted(self) -> bool {
        !self.runtime_parity_ok()
    }

    pub fn missing_required_diagnostics_report(self) -> bool {
        self.generation_budget_missing() || self.runtime.missing_required_runtime_report()
    }

    pub fn routing_drift_component_count(self) -> usize {
        usize::from(self.routing_drifted())
    }

    pub fn generation_drift_component_count(self) -> usize {
        usize::from(self.generation_budget_missing())
            + usize::from(self.generation_budget_drifted())
    }

    pub fn hardware_drift_component_count(self) -> usize {
        usize::from(self.hardware_pressure_drifted())
            + usize::from(self.compute_headroom_drifted())
            + usize::from(self.latency_budget_drifted())
    }

    pub fn diagnostics_request_drift_component_count(self) -> usize {
        self.routing_drift_component_count()
            .saturating_add(self.generation_drift_component_count())
            .saturating_add(self.hardware_drift_component_count())
            .saturating_add(self.runtime.runtime_drift_component_count())
    }

    pub fn has_request_drift(self) -> bool {
        self.routing_drifted()
            || self.generation_budget_missing()
            || self.generation_budget_drifted()
            || self.hardware_pressure_drifted()
            || self.planning_hardware_drifted()
            || self.runtime_drifted()
    }

    pub fn has_diagnostics_request_drift_components(self) -> bool {
        self.diagnostics_request_drift_component_count() > 0
    }

    pub fn diagnostics_request_accounting_is_consistent(self) -> bool {
        let expected_problem_count = self
            .routing_drift_component_count()
            .saturating_add(self.generation_drift_component_count())
            .saturating_add(self.hardware_drift_component_count())
            .saturating_add(self.runtime.runtime_drift_component_count());

        self.diagnostics_request_drift_component_count() == expected_problem_count
            && self.has_diagnostics_request_drift_components() == (expected_problem_count > 0)
            && self.request_parity_is_consistent() == (expected_problem_count == 0)
    }

    pub fn request_parity_is_consistent(self) -> bool {
        self.routing_parity_ok()
            && self.generation_parity_ok()
            && self.hardware_parity_ok()
            && self.runtime_parity_ok()
    }

    pub fn diagnostics_request_parity_shape_is_clean(self) -> bool {
        !self.has_diagnostics_request_drift_components()
            && self.diagnostics_request_accounting_is_consistent()
    }

    pub fn can_accept_inference_diagnostics_request_parity(self) -> bool {
        self.diagnostics_request_parity_shape_is_clean()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticsPressureBand {
    Low,
    Medium,
    High,
    Critical,
}

impl DiagnosticsPressureBand {
    pub fn from_pressure(pressure: f32) -> Self {
        let pressure = pressure.clamp(0.0, 1.0);
        if pressure >= 0.88 {
            Self::Critical
        } else if pressure >= 0.72 {
            Self::High
        } else if pressure >= 0.45 {
            Self::Medium
        } else {
            Self::Low
        }
    }

    pub fn is_constrained(self) -> bool {
        matches!(self, Self::High | Self::Critical)
    }
}

impl InferenceDiagnostics {
    pub fn new(route_budget: RouteBudget) -> Self {
        Self {
            route_budget,
            ..Self::default()
        }
    }

    pub fn from_request_envelope(request: &RuntimeRequestEnvelope) -> Self {
        Self::default().with_request_envelope(request)
    }

    pub fn with_request_envelope(mut self, request: &RuntimeRequestEnvelope) -> Self {
        self.route_budget = request.route_budget;
        self.generation_budget = Some(request.generation_budget);
        self.hardware_pressure = request.hardware_pressure.clamp(0.0, 1.0);

        if let Some(planning) = request.planning {
            self.compute_headroom = planning.compute_headroom.clamp(0.0, 1.0);
            self.latency_budget_ms = planning.latency_budget_ms;
        }
        self.runtime = self.runtime.with_request_envelope(request);

        self
    }

    pub fn with_runtime(mut self, runtime: RuntimeDiagnostics) -> Self {
        self.runtime = runtime;
        self
    }

    pub fn with_embeddings(mut self, embeddings: EmbeddingDiagnostics) -> Self {
        self.embeddings = embeddings;
        self
    }

    pub fn with_generation_budget(mut self, generation_budget: RuntimeGenerationBudget) -> Self {
        self.generation_budget = Some(generation_budget);
        self
    }

    pub fn with_hardware(
        mut self,
        hardware_pressure: f32,
        compute_headroom: f32,
        latency_budget_ms: Option<u64>,
    ) -> Self {
        self.hardware_pressure = hardware_pressure.clamp(0.0, 1.0);
        self.compute_headroom = compute_headroom.clamp(0.0, 1.0);
        self.latency_budget_ms = latency_budget_ms;
        self
    }

    pub fn with_recursive_runtime_calls(mut self, recursive_runtime_calls: usize) -> Self {
        self.recursive_runtime_calls = recursive_runtime_calls;
        self
    }

    pub fn push_note(&mut self, note: impl Into<String>) {
        let note = note.into();
        if !note.trim().is_empty() {
            self.notes.push(note);
        }
    }

    pub fn request_parity_summary(
        &self,
        request: &RuntimeRequestEnvelope,
    ) -> InferenceDiagnosticsRequestParitySummary {
        let planning = request.planning;

        InferenceDiagnosticsRequestParitySummary {
            route_budget_matches_request: self.route_budget == request.route_budget,
            generation_budget_reported: self.generation_budget.is_some(),
            generation_budget_matches_request: self.generation_budget
                == Some(request.generation_budget),
            hardware_pressure_matches_request: float_close(
                self.hardware_pressure,
                request.hardware_pressure,
            ),
            has_planning_digest: planning.is_some(),
            compute_headroom_matches_planning: planning
                .map(|planning| float_close(self.compute_headroom, planning.compute_headroom)),
            latency_budget_matches_planning: planning
                .map(|planning| self.latency_budget_ms == planning.latency_budget_ms),
            runtime: self.runtime.request_parity_summary(request),
        }
    }

    pub fn kv_exchange_total(&self) -> usize {
        self.runtime
            .imported_kv_blocks
            .saturating_add(self.runtime.exported_kv_blocks)
    }

    pub fn has_runtime_execution_signal(&self) -> bool {
        self.runtime.has_forward_signal()
            || self.embeddings.runtime_embedding_available()
            || self.recursive_runtime_calls > 0
    }

    pub fn diagnostics_summary(&self) -> InferenceDiagnosticsSummary {
        InferenceDiagnosticsSummary {
            has_generation_budget: self.generation_budget.is_some(),
            generation_truncated_by_context: self
                .generation_budget
                .is_some_and(|budget| budget.truncated_by_context),
            route_attention_tokens: self.route_budget.attention_tokens,
            route_fast_tokens: self.route_budget.fast_tokens,
            runtime_kv_exchange_total: self.kv_exchange_total(),
            weak_runtime_kv_imports_skipped: self.runtime.weak_runtime_kv_imports_skipped,
            has_runtime_execution_signal: self.has_runtime_execution_signal(),
            runtime_embedding_available: self.embeddings.runtime_embedding_available(),
            fallback_embedding_used: self.embeddings.fallback_embedding_used(),
            hardware_pressure_band: DiagnosticsPressureBand::from_pressure(self.hardware_pressure),
            has_latency_budget: self.latency_budget_ms.is_some(),
            recursive_runtime_calls: self.recursive_runtime_calls,
            note_count: self.notes.len(),
        }
    }
}

impl Default for InferenceDiagnostics {
    fn default() -> Self {
        Self {
            runtime: RuntimeDiagnostics::default(),
            embeddings: EmbeddingDiagnostics::default(),
            route_budget: RouteBudget::default(),
            generation_budget: None,
            hardware_pressure: 0.0,
            compute_headroom: 0.5,
            latency_budget_ms: None,
            recursive_runtime_calls: 0,
            notes: Vec::new(),
        }
    }
}

fn valid_kv_precision(hot_bits: u8, cold_bits: u8) -> bool {
    matches!(hot_bits, 4 | 8) && matches!(cold_bits, 4 | 8) && cold_bits <= hot_bits
}

fn float_close(left: f32, right: f32) -> bool {
    (left - right).abs() <= 0.0001
}

fn finite_option(value: Option<f32>) -> Option<f32> {
    value.filter(|value| value.is_finite())
}

fn has_text(value: Option<&str>) -> bool {
    value.is_some_and(|value| !value.trim().is_empty())
}

fn non_empty_string(value: String) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::RuntimeAdapter;
    use crate::engine::{InferenceRequest, RuntimeFailureKind};
    use crate::fht_dke::DeterministicFhtDkeBudgeter;
    use crate::manifest::TransformerRuntimeArchitecture;
    use crate::planning::RuntimePlanningDigest;
    use crate::profile::{HierarchyWeights, TaskProfile};
    use crate::request::RuntimeRequestEnvelope;
    use crate::router::{
        DefaultHierarchicalRouter, HierarchicalRouter, RoutingContext, TokenFeatures,
    };
    use crate::transformer::{
        TransformerAttentionKind, TransformerLayerBudget, TransformerPlanDigest,
    };

    #[test]
    fn runtime_diagnostics_reports_signal_quality() {
        let diagnostics = RuntimeDiagnostics::empty()
            .with_model_id("model")
            .with_architecture(24, 4096, 2048)
            .with_layer_modes(8, 12, 4)
            .with_device_execution(
                "discrete-gpu",
                "gpu",
                "cpu",
                "gpu-resident",
                DeviceExecutionSource::RuntimeReported,
            )
            .with_forward_signals(Some(0.42), Some(f32::NAN))
            .with_kv_exchange(2, 3)
            .with_kv_precision(8, 4);

        assert!(diagnostics.has_forward_signal());
        assert!(diagnostics.has_runtime_architecture_signal());
        assert!(diagnostics.has_all_layer_modes());
        assert!(diagnostics.has_runtime_reported_device_execution_signal());
        assert!(diagnostics.has_valid_kv_precision_signal());
        assert_eq!(diagnostics.layer_mode_count(), 24);
        assert_eq!(diagnostics.kv_influence, None);

        let summary = diagnostics.diagnostics_summary();

        assert!(summary.has_model_id);
        assert!(!summary.has_selected_adapter);
        assert!(!summary.has_runtime_identity());
        assert!(summary.missing_runtime_identity());
        assert!(summary.has_runtime_architecture);
        assert_eq!(summary.layer_count, 24);
        assert_eq!(summary.layer_mode_count, 24);
        assert!(summary.has_all_layer_modes);
        assert!(summary.has_device_execution);
        assert_eq!(
            summary.device_execution_source,
            Some(DeviceExecutionSource::RuntimeReported)
        );
        assert!(summary.has_runtime_reported_device_execution());
        assert!(!summary.has_control_plane_filled_device_execution());
        assert!(summary.has_forward_energy);
        assert!(!summary.has_kv_influence);
        assert_eq!(summary.imported_kv_blocks, 2);
        assert_eq!(summary.exported_kv_blocks, 3);
        assert_eq!(summary.weak_runtime_kv_imports_skipped, 0);
        assert_eq!(summary.kv_exchange_total(), 5);
        assert!(summary.has_runtime_kv_exchange());
        assert_eq!(summary.runtime_kv_activity_total(), 5);
        assert!(summary.has_runtime_kv_activity());
        assert!(summary.has_valid_kv_precision);
        assert!(summary.has_forward_signal);
        assert!(summary.has_runtime_forward_or_kv_signal());
        assert!(!summary.missing_runtime_architecture());
        assert!(!summary.missing_valid_kv_precision());
        assert!(!summary.has_complete_runtime_signal());
        assert_eq!(summary.runtime_identity_signal_component_count(), 1);
        assert_eq!(summary.runtime_architecture_signal_component_count(), 3);
        assert_eq!(summary.device_execution_signal_component_count(), 2);
        assert_eq!(summary.runtime_forward_signal_component_count(), 1);
        assert_eq!(summary.runtime_kv_activity_signal_component_count(), 2);
        assert_eq!(summary.runtime_precision_signal_component_count(), 1);
        assert_eq!(summary.runtime_diagnostics_signal_component_count(), 10);
        assert!(summary.has_runtime_diagnostics_signals());
        assert_eq!(summary.runtime_identity_problem_component_count(), 1);
        assert_eq!(summary.runtime_architecture_problem_component_count(), 0);
        assert_eq!(summary.runtime_activity_problem_component_count(), 0);
        assert_eq!(summary.runtime_precision_problem_component_count(), 0);
        assert_eq!(summary.runtime_diagnostics_problem_component_count(), 1);
        assert!(summary.has_runtime_diagnostics_problem_components());
        assert!(summary.runtime_diagnostics_accounting_is_consistent());
        assert!(!summary.runtime_diagnostics_shape_is_clean());
        assert!(!summary.can_use_runtime_diagnostics());
    }

    #[test]
    fn runtime_diagnostics_summary_counts_missing_runtime_shape() {
        let summary = RuntimeDiagnostics::empty().diagnostics_summary();

        assert!(!summary.has_runtime_diagnostics_signals());
        assert_eq!(summary.runtime_diagnostics_signal_component_count(), 0);
        assert_eq!(summary.runtime_identity_problem_component_count(), 2);
        assert_eq!(summary.runtime_architecture_problem_component_count(), 1);
        assert_eq!(summary.runtime_activity_problem_component_count(), 1);
        assert_eq!(summary.runtime_precision_problem_component_count(), 1);
        assert_eq!(summary.runtime_diagnostics_problem_component_count(), 5);
        assert!(summary.has_runtime_diagnostics_problem_components());
        assert!(!summary.has_complete_runtime_signal());
        assert!(summary.runtime_diagnostics_accounting_is_consistent());
        assert!(!summary.runtime_diagnostics_shape_is_clean());
        assert!(!summary.can_use_runtime_diagnostics());
    }

    #[test]
    fn runtime_diagnostics_summary_exposes_adapter_use_gate() {
        let summary = RuntimeDiagnostics::empty()
            .with_model_id("model")
            .with_selected_adapter(RuntimeAdapter::CpuSimd)
            .with_architecture(24, 4096, 2048)
            .with_forward_signals(Some(0.42), None)
            .with_kv_precision(8, 4)
            .diagnostics_summary();

        assert!(summary.has_complete_runtime_signal());
        assert_eq!(summary.runtime_diagnostics_problem_component_count(), 0);
        assert!(!summary.has_runtime_diagnostics_problem_components());
        assert!(summary.runtime_diagnostics_accounting_is_consistent());
        assert!(summary.runtime_diagnostics_shape_is_clean());
        assert!(summary.can_use_runtime_diagnostics());
    }

    #[test]
    fn runtime_diagnostics_summary_counts_weak_runtime_kv_skip_as_activity_not_exchange() {
        let summary = RuntimeDiagnostics::empty()
            .with_model_id("model")
            .with_selected_adapter(RuntimeAdapter::CpuSimd)
            .with_architecture(24, 4096, 2048)
            .with_weak_runtime_kv_imports_skipped(2)
            .with_kv_precision(8, 4)
            .diagnostics_summary();

        assert_eq!(summary.imported_kv_blocks, 0);
        assert_eq!(summary.exported_kv_blocks, 0);
        assert_eq!(summary.weak_runtime_kv_imports_skipped, 2);
        assert_eq!(summary.kv_exchange_total(), 0);
        assert!(!summary.has_runtime_kv_exchange());
        assert_eq!(summary.runtime_kv_activity_total(), 2);
        assert!(summary.has_runtime_kv_activity());
        assert!(summary.has_runtime_forward_or_kv_signal());
        assert!(summary.has_forward_signal);
        assert_eq!(summary.runtime_kv_activity_signal_component_count(), 1);
        assert_eq!(summary.runtime_activity_problem_component_count(), 0);
        assert!(summary.runtime_diagnostics_accounting_is_consistent());
        assert!(summary.can_use_runtime_diagnostics());
    }

    #[test]
    fn runtime_diagnostics_summary_exposes_hardware_adapter_metadata_admission() {
        let runtime_reported = RuntimeDiagnostics::empty()
            .with_model_id("model")
            .with_selected_adapter(RuntimeAdapter::CpuSimd)
            .with_architecture(24, 4096, 2048)
            .with_device_execution(
                "discrete-gpu",
                "gpu",
                "cpu",
                "gpu-resident",
                DeviceExecutionSource::RuntimeReported,
            )
            .with_kv_precision(8, 4)
            .diagnostics_summary();
        let control_plane_filled = RuntimeDiagnostics::empty()
            .with_model_id("model")
            .with_selected_adapter(RuntimeAdapter::CpuSimd)
            .with_architecture(24, 4096, 2048)
            .with_device_execution(
                "discrete-gpu",
                "gpu",
                "cpu",
                "gpu-resident",
                DeviceExecutionSource::ControlPlaneFilled,
            )
            .with_kv_precision(8, 4)
            .diagnostics_summary();
        let missing_device_execution = RuntimeDiagnostics::empty()
            .with_model_id("model")
            .with_selected_adapter(RuntimeAdapter::CpuSimd)
            .with_architecture(24, 4096, 2048)
            .with_kv_precision(8, 4)
            .diagnostics_summary();

        assert!(runtime_reported.can_use_runtime_diagnostics());
        assert!(runtime_reported.has_runtime_reported_device_execution());
        assert!(runtime_reported.has_device_execution_for_hardware_diagnostics());
        assert!(runtime_reported.can_use_device_execution_for_hardware_diagnostics());
        assert_eq!(
            runtime_reported.hardware_adapter_metadata_signal_component_count(),
            2
        );
        assert!(runtime_reported.has_hardware_adapter_metadata_signals());
        assert_eq!(
            runtime_reported.missing_device_execution_metadata_component_count(),
            0
        );
        assert_eq!(
            runtime_reported.control_plane_filled_metadata_component_count(),
            0
        );
        assert_eq!(
            runtime_reported.hardware_adapter_metadata_blocker_component_count(),
            0
        );
        assert!(!runtime_reported.has_hardware_adapter_metadata_blockers());
        assert!(runtime_reported.hardware_adapter_metadata_admission_accounting_is_consistent());
        assert!(runtime_reported.hardware_adapter_metadata_admission_is_clean());
        assert!(runtime_reported.can_admit_hardware_adapter_metadata());

        assert!(control_plane_filled.can_use_runtime_diagnostics());
        assert!(control_plane_filled.has_control_plane_filled_device_execution());
        assert!(!control_plane_filled.has_runtime_reported_device_execution());
        assert!(control_plane_filled.has_device_execution_for_hardware_diagnostics());
        assert!(control_plane_filled.can_use_device_execution_for_hardware_diagnostics());
        assert_eq!(
            control_plane_filled.hardware_adapter_metadata_signal_component_count(),
            2
        );
        assert_eq!(
            control_plane_filled.control_plane_filled_metadata_component_count(),
            1
        );
        assert_eq!(
            control_plane_filled.hardware_adapter_metadata_blocker_component_count(),
            1
        );
        assert!(control_plane_filled.has_hardware_adapter_metadata_blockers());
        assert!(
            control_plane_filled.hardware_adapter_metadata_admission_accounting_is_consistent()
        );
        assert!(!control_plane_filled.hardware_adapter_metadata_admission_is_clean());
        assert!(!control_plane_filled.can_admit_hardware_adapter_metadata());

        assert!(missing_device_execution.can_use_runtime_diagnostics());
        assert!(!missing_device_execution.has_device_execution);
        assert!(!missing_device_execution.has_device_execution_for_hardware_diagnostics());
        assert!(!missing_device_execution.can_use_device_execution_for_hardware_diagnostics());
        assert_eq!(
            missing_device_execution.hardware_adapter_metadata_signal_component_count(),
            0
        );
        assert!(!missing_device_execution.has_hardware_adapter_metadata_signals());
        assert_eq!(
            missing_device_execution.missing_device_execution_metadata_component_count(),
            1
        );
        assert_eq!(
            missing_device_execution.hardware_adapter_metadata_blocker_component_count(),
            1
        );
        assert!(missing_device_execution.has_hardware_adapter_metadata_blockers());
        assert!(
            missing_device_execution.hardware_adapter_metadata_admission_accounting_is_consistent()
        );
        assert!(!missing_device_execution.hardware_adapter_metadata_admission_is_clean());
        assert!(!missing_device_execution.can_admit_hardware_adapter_metadata());
    }

    #[test]
    fn runtime_diagnostics_admits_only_runtime_reported_device_execution_metadata() {
        let runtime_reported = RuntimeDiagnostics::empty()
            .with_model_id("model")
            .with_selected_adapter(RuntimeAdapter::CpuSimd)
            .with_architecture(24, 4096, 2048)
            .with_device_execution(
                "discrete-gpu",
                "gpu",
                "cpu",
                "gpu-resident",
                DeviceExecutionSource::RuntimeReported,
            )
            .with_kv_precision(8, 4);
        let control_plane_filled = RuntimeDiagnostics::empty()
            .with_model_id("model")
            .with_selected_adapter(RuntimeAdapter::CpuSimd)
            .with_architecture(24, 4096, 2048)
            .with_device_execution(
                "discrete-gpu",
                "gpu",
                "cpu",
                "gpu-resident",
                DeviceExecutionSource::ControlPlaneFilled,
            )
            .with_kv_precision(8, 4);
        let missing_device_execution = RuntimeDiagnostics::empty()
            .with_model_id("model")
            .with_selected_adapter(RuntimeAdapter::CpuSimd)
            .with_architecture(24, 4096, 2048)
            .with_kv_precision(8, 4);

        assert!(runtime_reported.can_admit_runtime_reported_device_execution_metadata());
        assert_eq!(
            runtime_reported.can_admit_runtime_reported_device_execution_metadata(),
            runtime_reported
                .diagnostics_summary()
                .can_admit_hardware_adapter_metadata()
        );
        assert!(
            control_plane_filled
                .diagnostics_summary()
                .can_use_device_execution_for_hardware_diagnostics()
        );
        assert!(!control_plane_filled.can_admit_runtime_reported_device_execution_metadata());
        assert!(!missing_device_execution.can_admit_runtime_reported_device_execution_metadata());
    }

    #[test]
    fn runtime_diagnostics_contract_summary_accepts_matching_runtime_claims() {
        let metadata = RuntimeMetadata::new("model", "tok", 2048, 4096).with_kv_precision(8, 4);
        let architecture = TransformerRuntimeArchitecture::new(24, 4096, 16, 16, 2048);
        let execution =
            AdapterExecutionContext::new([RuntimeAdapter::CpuSimd, RuntimeAdapter::PortableRust]);
        let diagnostics = RuntimeDiagnostics::empty()
            .with_model_id("model")
            .with_selected_adapter(RuntimeAdapter::CpuSimd)
            .with_architecture(24, 4096, 2048)
            .with_kv_precision(8, 4);

        let summary = diagnostics.contract_summary(&metadata, architecture, &execution);

        assert!(summary.model_id_reported);
        assert!(summary.model_id_matches_metadata);
        assert!(summary.layer_count_reported);
        assert!(summary.layer_count_matches_architecture);
        assert!(summary.hidden_size_reported);
        assert!(summary.hidden_size_matches_architecture);
        assert!(summary.local_window_tokens_reported);
        assert!(summary.local_window_tokens_within_context);
        assert!(summary.selected_adapter_reported);
        assert!(summary.selected_adapter_within_execution);
        assert!(summary.hot_kv_precision_reported);
        assert!(summary.hot_kv_precision_within_metadata);
        assert!(summary.cold_kv_precision_reported);
        assert!(summary.cold_kv_precision_within_metadata);
        assert!(summary.kv_precision_pair_reported);
        assert!(summary.kv_precision_pair_valid);
        assert_eq!(summary.diagnostics_contract_signal_component_count(), 8);
        assert!(summary.has_diagnostics_contract_signals());
        assert_eq!(summary.identity_contract_problem_component_count(), 0);
        assert_eq!(summary.architecture_contract_problem_component_count(), 0);
        assert_eq!(summary.adapter_contract_problem_component_count(), 0);
        assert_eq!(summary.precision_contract_problem_component_count(), 0);
        assert_eq!(summary.diagnostics_contract_problem_component_count(), 0);
        assert!(!summary.has_diagnostics_contract_problem_components());
        assert!(summary.diagnostics_contract_accounting_is_consistent());
        assert!(summary.diagnostics_contract_shape_is_clean());
        assert!(summary.can_accept_runtime_diagnostics_contract());
    }

    #[test]
    fn runtime_diagnostics_contract_violations_find_bad_adapter_and_precision() {
        let metadata = RuntimeMetadata::new("model", "tok", 2048, 4096).with_kv_precision(4, 4);
        let architecture = TransformerRuntimeArchitecture::new(24, 4096, 16, 16, 2048);
        let execution = AdapterExecutionContext::new([RuntimeAdapter::CpuSimd]);
        let diagnostics = RuntimeDiagnostics::empty()
            .with_model_id("other")
            .with_selected_adapter(RuntimeAdapter::Cuda)
            .with_architecture(24, 4096, 4096)
            .with_kv_precision(8, 4);

        let violations = diagnostics.contract_violations(&metadata, architecture, &execution);
        let summary = diagnostics.contract_summary(&metadata, architecture, &execution);

        assert!(
            violations
                .iter()
                .any(|violation| violation.contains("model_id"))
        );
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains("selected adapter"))
        );
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains("hot KV precision"))
        );
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains("local_window_tokens"))
        );
        assert!(summary.model_id_reported);
        assert!(!summary.model_id_matches_metadata);
        assert!(summary.layer_count_matches_architecture);
        assert!(summary.hidden_size_matches_architecture);
        assert!(!summary.local_window_tokens_within_context);
        assert!(summary.selected_adapter_reported);
        assert!(!summary.selected_adapter_within_execution);
        assert!(summary.hot_kv_precision_reported);
        assert!(!summary.hot_kv_precision_within_metadata);
        assert!(summary.cold_kv_precision_within_metadata);
        assert!(summary.kv_precision_pair_valid);
        assert_eq!(summary.identity_contract_problem_component_count(), 1);
        assert_eq!(summary.architecture_contract_problem_component_count(), 1);
        assert_eq!(summary.adapter_contract_problem_component_count(), 1);
        assert_eq!(summary.precision_contract_problem_component_count(), 1);
        assert_eq!(summary.diagnostics_contract_problem_component_count(), 4);
        assert!(summary.has_diagnostics_contract_problem_components());
        assert_eq!(summary.diagnostics_contract_signal_component_count(), 8);
        assert!(summary.has_diagnostics_contract_signals());
        assert!(summary.diagnostics_contract_accounting_is_consistent());
        assert!(!summary.diagnostics_contract_shape_is_clean());
        assert!(!summary.can_accept_runtime_diagnostics_contract());
    }

    #[test]
    fn runtime_diagnostics_contract_summary_counts_precision_pair_drift() {
        let metadata = RuntimeMetadata::new("model", "tok", 2048, 4096).with_kv_precision(8, 8);
        let architecture = TransformerRuntimeArchitecture::new(24, 4096, 16, 16, 2048);
        let execution = AdapterExecutionContext::new([RuntimeAdapter::CpuSimd]);
        let mut diagnostics = RuntimeDiagnostics::empty()
            .with_model_id("model")
            .with_selected_adapter(RuntimeAdapter::CpuSimd)
            .with_architecture(24, 4096, 2048);
        diagnostics.hot_kv_precision_bits = Some(4);
        diagnostics.cold_kv_precision_bits = Some(8);

        let summary = diagnostics.contract_summary(&metadata, architecture, &execution);

        assert!(summary.hot_kv_precision_within_metadata);
        assert!(summary.cold_kv_precision_within_metadata);
        assert!(summary.kv_precision_pair_reported);
        assert!(!summary.kv_precision_pair_valid);
        assert_eq!(summary.precision_contract_problem_component_count(), 1);
        assert_eq!(summary.diagnostics_contract_problem_component_count(), 1);
        assert!(summary.diagnostics_contract_accounting_is_consistent());
        assert!(!summary.diagnostics_contract_shape_is_clean());
    }

    #[test]
    fn runtime_diagnostics_contract_summary_exposes_admission_boundary() {
        let clean = RuntimeDiagnosticsContractSummary {
            model_id_reported: true,
            model_id_matches_metadata: true,
            layer_count_reported: true,
            layer_count_matches_architecture: true,
            hidden_size_reported: true,
            hidden_size_matches_architecture: true,
            local_window_tokens_reported: true,
            local_window_tokens_within_context: true,
            selected_adapter_reported: true,
            selected_adapter_within_execution: true,
            hot_kv_precision_reported: true,
            hot_kv_precision_within_metadata: true,
            cold_kv_precision_reported: true,
            cold_kv_precision_within_metadata: true,
            kv_precision_pair_reported: true,
            kv_precision_pair_valid: true,
        };
        let drift = RuntimeDiagnosticsContractSummary {
            model_id_matches_metadata: false,
            layer_count_matches_architecture: false,
            selected_adapter_within_execution: false,
            hot_kv_precision_within_metadata: false,
            kv_precision_pair_valid: false,
            ..clean
        };
        let missing = RuntimeDiagnosticsContractSummary {
            model_id_reported: false,
            model_id_matches_metadata: false,
            layer_count_reported: false,
            layer_count_matches_architecture: false,
            hidden_size_reported: false,
            hidden_size_matches_architecture: false,
            local_window_tokens_reported: false,
            local_window_tokens_within_context: false,
            selected_adapter_reported: false,
            selected_adapter_within_execution: false,
            hot_kv_precision_reported: false,
            hot_kv_precision_within_metadata: false,
            cold_kv_precision_reported: false,
            cold_kv_precision_within_metadata: false,
            kv_precision_pair_reported: false,
            kv_precision_pair_valid: false,
        };

        assert_eq!(
            clean.runtime_diagnostics_contract_admission_signal_component_count(),
            4
        );
        assert!(clean.has_runtime_diagnostics_contract_admission_signals());
        assert_eq!(clean.missing_contract_report_component_count(), 0);
        assert_eq!(
            clean.runtime_diagnostics_contract_admission_blocker_component_count(),
            0
        );
        assert!(!clean.has_runtime_diagnostics_contract_admission_blockers());
        assert!(clean.runtime_diagnostics_contract_admission_accounting_is_consistent());
        assert!(clean.runtime_diagnostics_contract_admission_is_clean());
        assert!(clean.can_admit_runtime_diagnostics_contract());
        assert!(clean.can_accept_runtime_diagnostics_contract());

        assert_eq!(
            drift.runtime_diagnostics_contract_admission_signal_component_count(),
            0
        );
        assert!(!drift.has_runtime_diagnostics_contract_admission_signals());
        assert_eq!(drift.missing_contract_report_component_count(), 0);
        assert_eq!(drift.diagnostics_contract_problem_component_count(), 5);
        assert_eq!(
            drift.runtime_diagnostics_contract_admission_blocker_component_count(),
            5
        );
        assert!(drift.has_runtime_diagnostics_contract_admission_blockers());
        assert!(drift.runtime_diagnostics_contract_admission_accounting_is_consistent());
        assert!(!drift.runtime_diagnostics_contract_admission_is_clean());
        assert!(!drift.can_admit_runtime_diagnostics_contract());
        assert!(!drift.can_accept_runtime_diagnostics_contract());

        assert_eq!(
            missing.runtime_diagnostics_contract_admission_signal_component_count(),
            0
        );
        assert!(!missing.has_runtime_diagnostics_contract_admission_signals());
        assert_eq!(missing.missing_contract_report_component_count(), 7);
        assert_eq!(missing.diagnostics_contract_problem_component_count(), 0);
        assert_eq!(
            missing.runtime_diagnostics_contract_admission_blocker_component_count(),
            7
        );
        assert!(missing.has_runtime_diagnostics_contract_admission_blockers());
        assert!(missing.runtime_diagnostics_contract_admission_accounting_is_consistent());
        assert!(!missing.runtime_diagnostics_contract_admission_is_clean());
        assert!(!missing.can_admit_runtime_diagnostics_contract());
        assert!(missing.can_accept_runtime_diagnostics_contract());
    }

    #[test]
    fn runtime_diagnostics_hardware_contract_accepts_matching_aliases() {
        let hardware = crate::hardware::HardwareAllocator::new().plan(
            crate::hardware::HardwareLoadSnapshot::new(
                crate::hardware::DeviceClass::DiscreteGpu,
                0.1,
                0.1,
                0.1,
                0.1,
            ),
            TaskProfile::Coding,
            512,
            HierarchyWeights::for_profile(TaskProfile::Coding),
        );
        let diagnostics = RuntimeDiagnostics::empty().with_device_execution(
            "gpu",
            "gpu",
            "cpu",
            "gpu-resident",
            DeviceExecutionSource::RuntimeReported,
        );

        assert!(
            diagnostics
                .hardware_contract_violations(&hardware)
                .is_empty()
        );
        let contract_summary = diagnostics.hardware_contract_summary(&hardware);
        let report = diagnostics.hardware_acceptance_report(&hardware);
        let summary = report.diagnostics_summary();

        assert!(contract_summary.device_profile_reported);
        assert!(contract_summary.device_profile_known);
        assert!(contract_summary.device_profile_matches_hardware);
        assert!(contract_summary.primary_lane_reported);
        assert!(contract_summary.primary_lane_known);
        assert!(contract_summary.primary_lane_matches_hardware);
        assert!(contract_summary.fallback_lane_reported);
        assert!(contract_summary.fallback_lane_known);
        assert!(contract_summary.fallback_lane_matches_hardware);
        assert!(contract_summary.memory_mode_reported);
        assert!(contract_summary.memory_mode_known);
        assert!(contract_summary.memory_mode_matches_hardware);
        assert!(contract_summary.device_profile_contract_is_clean());
        assert!(contract_summary.primary_lane_contract_is_clean());
        assert!(contract_summary.fallback_lane_contract_is_clean());
        assert!(contract_summary.memory_mode_contract_is_clean());
        assert_eq!(
            contract_summary.hardware_contract_signal_component_count(),
            12
        );
        assert!(contract_summary.has_hardware_contract_signals());
        assert_eq!(contract_summary.device_profile_problem_component_count(), 0);
        assert_eq!(contract_summary.lane_problem_component_count(), 0);
        assert_eq!(contract_summary.memory_mode_problem_component_count(), 0);
        assert_eq!(
            contract_summary.hardware_contract_problem_component_count(),
            0
        );
        assert!(!contract_summary.has_hardware_contract_problem_components());
        assert!(contract_summary.hardware_contract_accounting_is_consistent());
        assert!(contract_summary.hardware_contract_shape_is_clean());
        assert!(contract_summary.can_accept_runtime_hardware_contract());
        assert!(report.is_accepted());
        assert!(report.violations().is_empty());
        assert!(report.failure_reports().is_empty());
        let failure_batch = report.failure_batch_summary();
        assert_eq!(failure_batch.total_count, 0);
        assert!(!failure_batch.has_failures());
        assert!(failure_batch.failure_batch_shape_is_clean());
        assert!(!failure_batch.can_format_runtime_failures());
        assert!(report.primary_failure_summary().is_none());
        assert!(summary.accepted);
        assert_eq!(summary.hardware_violation_count, 0);
        assert_eq!(summary.failure_report_count, 0);
        assert!(!summary.has_hardware_violations());
        assert_eq!(summary.hardware_failure_component_count(), 0);
        assert!(!summary.has_failure_reports());
        assert_eq!(summary.mapped_failure_report_component_count(), 0);
        assert_eq!(summary.hardware_acceptance_problem_component_count(), 0);
        assert!(!summary.has_hardware_acceptance_problem_components());
        assert!(summary.failure_report_matches_violations());
        assert!(summary.hardware_acceptance_accounting_is_consistent());
        assert!(summary.is_clean_acceptance());
        assert!(summary.hardware_acceptance_shape_is_clean());
        assert!(summary.can_accept_runtime_hardware_diagnostics());
    }

    #[test]
    fn runtime_diagnostics_hardware_contract_accepts_control_plane_filled_device_execution() {
        let hardware = crate::hardware::HardwareAllocator::new().plan(
            crate::hardware::HardwareLoadSnapshot::new(
                crate::hardware::DeviceClass::DiscreteGpu,
                0.1,
                0.1,
                0.1,
                0.1,
            ),
            TaskProfile::Coding,
            512,
            HierarchyWeights::for_profile(TaskProfile::Coding),
        );
        let diagnostics = RuntimeDiagnostics::empty().with_device_execution(
            "gpu",
            "gpu",
            "cpu",
            "gpu-resident",
            DeviceExecutionSource::ControlPlaneFilled,
        );
        let diagnostics_summary = diagnostics.diagnostics_summary();
        let contract_summary = diagnostics.hardware_contract_summary(&hardware);
        let report = diagnostics.hardware_acceptance_report(&hardware);
        let hardware_summary = report.diagnostics_summary();

        assert!(diagnostics.has_control_plane_filled_device_execution_signal());
        assert!(!diagnostics.has_runtime_reported_device_execution_signal());
        assert!(diagnostics_summary.has_control_plane_filled_device_execution());
        assert!(!diagnostics_summary.has_runtime_reported_device_execution());
        assert_eq!(
            diagnostics_summary.device_execution_source,
            Some(DeviceExecutionSource::ControlPlaneFilled)
        );
        assert_eq!(
            diagnostics_summary.device_execution_signal_component_count(),
            2
        );
        assert!(contract_summary.hardware_contract_shape_is_clean());
        assert!(contract_summary.can_accept_runtime_hardware_contract());
        assert!(report.is_accepted());
        assert!(report.violations().is_empty());
        assert_eq!(hardware_summary.hardware_violation_count, 0);
        assert_eq!(hardware_summary.failure_report_count, 0);
        assert!(hardware_summary.hardware_acceptance_accounting_is_consistent());
        assert!(hardware_summary.can_accept_runtime_hardware_diagnostics());
    }

    #[test]
    fn runtime_diagnostics_control_plane_filled_execution_still_blocks_hardware_drift() {
        let hardware = crate::hardware::HardwareAllocator::new().plan(
            crate::hardware::HardwareLoadSnapshot::new(
                crate::hardware::DeviceClass::DiscreteGpu,
                0.1,
                0.1,
                0.1,
                0.1,
            ),
            TaskProfile::Coding,
            512,
            HierarchyWeights::for_profile(TaskProfile::Coding),
        );
        let diagnostics = RuntimeDiagnostics::empty().with_device_execution(
            "cpu",
            "cpu",
            "gpu",
            "tiered",
            DeviceExecutionSource::ControlPlaneFilled,
        );

        let joined = diagnostics
            .hardware_contract_violations(&hardware)
            .join("\n");
        let diagnostics_summary = diagnostics.diagnostics_summary();
        let contract_summary = diagnostics.hardware_contract_summary(&hardware);
        let report = diagnostics.hardware_acceptance_report(&hardware);
        let hardware_summary = report.diagnostics_summary();
        let failures = report.failure_reports();

        assert!(diagnostics.has_control_plane_filled_device_execution_signal());
        assert!(!diagnostics.has_runtime_reported_device_execution_signal());
        assert!(diagnostics_summary.has_control_plane_filled_device_execution());
        assert_eq!(
            diagnostics_summary.device_execution_source,
            Some(DeviceExecutionSource::ControlPlaneFilled)
        );
        assert!(joined.contains("device_profile cpu differs from request device discrete"));
        assert!(joined.contains("primary_lane cpu-vector differs from request primary"));
        assert!(joined.contains("fallback_lane discrete-gpu differs from request fallback"));
        assert!(joined.contains("memory_mode tiered-disk differs from request memory"));
        assert!(contract_summary.device_profile_reported);
        assert!(contract_summary.device_profile_known);
        assert!(!contract_summary.device_profile_matches_hardware);
        assert!(contract_summary.primary_lane_reported);
        assert!(contract_summary.primary_lane_known);
        assert!(!contract_summary.primary_lane_matches_hardware);
        assert!(contract_summary.fallback_lane_reported);
        assert!(contract_summary.fallback_lane_known);
        assert!(!contract_summary.fallback_lane_matches_hardware);
        assert!(contract_summary.memory_mode_reported);
        assert!(contract_summary.memory_mode_known);
        assert!(!contract_summary.memory_mode_matches_hardware);
        assert_eq!(
            contract_summary.hardware_contract_signal_component_count(),
            8
        );
        assert_eq!(
            contract_summary.hardware_contract_problem_component_count(),
            4
        );
        assert!(contract_summary.hardware_contract_accounting_is_consistent());
        assert!(!contract_summary.can_accept_runtime_hardware_contract());
        assert!(!report.is_accepted());
        assert_eq!(report.violations().len(), 4);
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].kind, RuntimeFailureKind::ContractViolation);
        assert!(!hardware_summary.accepted);
        assert_eq!(hardware_summary.hardware_violation_count, 4);
        assert_eq!(hardware_summary.failure_report_count, 1);
        assert!(hardware_summary.has_hardware_violations());
        assert!(hardware_summary.failure_report_matches_violations());
        assert!(hardware_summary.hardware_acceptance_accounting_is_consistent());
        assert!(!hardware_summary.can_accept_runtime_hardware_diagnostics());
    }

    #[test]
    fn runtime_diagnostics_hardware_contract_reports_unknown_and_mismatched_values() {
        let hardware = crate::hardware::HardwareAllocator::new().plan(
            crate::hardware::HardwareLoadSnapshot::new(
                crate::hardware::DeviceClass::CpuOnly,
                0.1,
                0.1,
                0.1,
                0.1,
            ),
            TaskProfile::General,
            512,
            HierarchyWeights::default(),
        );
        let diagnostics = RuntimeDiagnostics::empty().with_device_execution(
            "discrete-gpu",
            "warp",
            "gpu",
            "hologram",
            DeviceExecutionSource::RuntimeReported,
        );

        let joined = diagnostics
            .hardware_contract_violations(&hardware)
            .join("\n");
        let contract_summary = diagnostics.hardware_contract_summary(&hardware);

        assert!(joined.contains("device_profile discrete differs from request device cpu"));
        assert!(joined.contains("unknown primary_lane warp"));
        assert!(joined.contains("fallback_lane discrete-gpu differs from request fallback"));
        assert!(joined.contains("unknown memory_mode hologram"));
        assert!(contract_summary.device_profile_reported);
        assert!(contract_summary.device_profile_known);
        assert!(!contract_summary.device_profile_matches_hardware);
        assert!(contract_summary.primary_lane_reported);
        assert!(!contract_summary.primary_lane_known);
        assert!(!contract_summary.primary_lane_matches_hardware);
        assert!(contract_summary.fallback_lane_reported);
        assert!(contract_summary.fallback_lane_known);
        assert!(!contract_summary.fallback_lane_matches_hardware);
        assert!(contract_summary.memory_mode_reported);
        assert!(!contract_summary.memory_mode_known);
        assert!(!contract_summary.memory_mode_matches_hardware);
        assert!(!contract_summary.device_profile_contract_is_clean());
        assert!(!contract_summary.primary_lane_contract_is_clean());
        assert!(!contract_summary.fallback_lane_contract_is_clean());
        assert!(!contract_summary.memory_mode_contract_is_clean());
        assert_eq!(
            contract_summary.hardware_contract_signal_component_count(),
            6
        );
        assert!(contract_summary.has_hardware_contract_signals());
        assert_eq!(contract_summary.device_profile_problem_component_count(), 1);
        assert_eq!(contract_summary.lane_problem_component_count(), 2);
        assert_eq!(contract_summary.memory_mode_problem_component_count(), 1);
        assert_eq!(
            contract_summary.hardware_contract_problem_component_count(),
            4
        );
        assert!(contract_summary.has_hardware_contract_problem_components());
        assert!(contract_summary.hardware_contract_accounting_is_consistent());
        assert!(!contract_summary.hardware_contract_shape_is_clean());
        assert!(!contract_summary.can_accept_runtime_hardware_contract());

        let report = diagnostics.hardware_acceptance_report(&hardware);
        let failures = report.failure_reports();
        let failure_batch = report.failure_batch_summary();
        let primary_summary = report.primary_failure_summary().unwrap();
        let summary = report.diagnostics_summary();

        assert!(!report.is_accepted());
        assert!(!summary.accepted);
        assert_eq!(summary.hardware_violation_count, report.violations().len());
        assert_eq!(summary.failure_report_count, failures.len());
        assert!(summary.has_hardware_violations());
        assert_eq!(summary.hardware_failure_component_count(), 1);
        assert!(summary.has_failure_reports());
        assert_eq!(summary.mapped_failure_report_component_count(), 1);
        assert_eq!(summary.hardware_acceptance_problem_component_count(), 2);
        assert!(summary.has_hardware_acceptance_problem_components());
        assert!(summary.failure_report_matches_violations());
        assert!(summary.hardware_acceptance_accounting_is_consistent());
        assert!(!summary.is_clean_acceptance());
        assert!(!summary.hardware_acceptance_shape_is_clean());
        assert!(!summary.can_accept_runtime_hardware_diagnostics());
        assert_eq!(failures.len(), 1);
        assert_eq!(
            failure_batch,
            RuntimeFailureReport::batch_summary(&failures)
        );
        assert_eq!(failure_batch.total_count, 1);
        assert_eq!(failure_batch.contract_violation_count, 1);
        assert_eq!(failure_batch.backend_error_count, 0);
        assert!(failure_batch.has_contract_failures());
        assert!(failure_batch.has_recoverable_failures());
        assert!(failure_batch.failure_batch_shape_is_clean());
        assert!(failure_batch.can_format_runtime_failures());
        assert_eq!(failures[0].kind, RuntimeFailureKind::ContractViolation);
        assert!(
            failures[0]
                .message
                .contains("runtime hardware diagnostics acceptance failed")
        );
        assert_eq!(report.primary_failure_report(), Some(failures[0].clone()));
        assert_eq!(primary_summary.kind, RuntimeFailureKind::ContractViolation);
        assert_eq!(primary_summary.trace_label, "runtime_contract_violation");
        assert!(primary_summary.recoverable);
        assert!(!primary_summary.backend_error);
        assert!(primary_summary.failure_summary_shape_is_clean());
        assert!(primary_summary.can_use_runtime_failure_report());
    }

    #[test]
    fn runtime_hardware_diagnostics_contract_summary_allows_missing_optional_fields() {
        let hardware = crate::hardware::HardwareAllocator::new().plan(
            crate::hardware::HardwareLoadSnapshot::new(
                crate::hardware::DeviceClass::CpuOnly,
                0.1,
                0.1,
                0.1,
                0.1,
            ),
            TaskProfile::General,
            512,
            HierarchyWeights::default(),
        );
        let summary = RuntimeDiagnostics::empty().hardware_contract_summary(&hardware);

        assert!(!summary.device_profile_reported);
        assert!(!summary.device_profile_known);
        assert!(!summary.device_profile_matches_hardware);
        assert!(summary.device_profile_contract_is_clean());
        assert!(summary.primary_lane_contract_is_clean());
        assert!(summary.fallback_lane_contract_is_clean());
        assert!(summary.memory_mode_contract_is_clean());
        assert_eq!(summary.hardware_contract_signal_component_count(), 0);
        assert!(!summary.has_hardware_contract_signals());
        assert_eq!(summary.hardware_contract_problem_component_count(), 0);
        assert!(!summary.has_hardware_contract_problem_components());
        assert!(summary.hardware_contract_accounting_is_consistent());
        assert!(summary.hardware_contract_shape_is_clean());
        assert!(summary.can_accept_runtime_hardware_contract());
        assert_eq!(
            summary.missing_hardware_contract_report_component_count(),
            4
        );
        assert_eq!(
            summary.runtime_hardware_contract_admission_signal_component_count(),
            0
        );
        assert_eq!(
            summary.runtime_hardware_contract_admission_blocker_component_count(),
            4
        );
        assert!(summary.runtime_hardware_contract_admission_accounting_is_consistent());
        assert!(!summary.runtime_hardware_contract_admission_is_clean());
        assert!(!summary.can_admit_runtime_hardware_contract());
    }

    #[test]
    fn runtime_hardware_diagnostics_contract_summary_counts_public_shape_drift() {
        let drift = RuntimeHardwareDiagnosticsContractSummary {
            device_profile_reported: false,
            device_profile_known: true,
            device_profile_matches_hardware: true,
            primary_lane_reported: true,
            primary_lane_known: false,
            primary_lane_matches_hardware: true,
            fallback_lane_reported: false,
            fallback_lane_known: false,
            fallback_lane_matches_hardware: false,
            memory_mode_reported: true,
            memory_mode_known: true,
            memory_mode_matches_hardware: false,
        };

        assert!(!drift.device_profile_contract_is_clean());
        assert!(!drift.primary_lane_contract_is_clean());
        assert!(drift.fallback_lane_contract_is_clean());
        assert!(!drift.memory_mode_contract_is_clean());
        assert_eq!(drift.device_profile_problem_component_count(), 1);
        assert_eq!(drift.lane_problem_component_count(), 1);
        assert_eq!(drift.memory_mode_problem_component_count(), 1);
        assert_eq!(drift.hardware_contract_problem_component_count(), 3);
        assert!(drift.has_hardware_contract_problem_components());
        assert_eq!(drift.hardware_contract_signal_component_count(), 6);
        assert!(drift.has_hardware_contract_signals());
        assert!(drift.hardware_contract_accounting_is_consistent());
        assert!(!drift.hardware_contract_shape_is_clean());
        assert!(!drift.can_accept_runtime_hardware_contract());
    }

    #[test]
    fn runtime_hardware_diagnostics_contract_summary_exposes_admission_boundary() {
        let clean = RuntimeHardwareDiagnosticsContractSummary {
            device_profile_reported: true,
            device_profile_known: true,
            device_profile_matches_hardware: true,
            primary_lane_reported: true,
            primary_lane_known: true,
            primary_lane_matches_hardware: true,
            fallback_lane_reported: true,
            fallback_lane_known: true,
            fallback_lane_matches_hardware: true,
            memory_mode_reported: true,
            memory_mode_known: true,
            memory_mode_matches_hardware: true,
        };
        let drift = RuntimeHardwareDiagnosticsContractSummary {
            device_profile_matches_hardware: false,
            primary_lane_known: false,
            fallback_lane_matches_hardware: false,
            memory_mode_known: false,
            ..clean
        };
        let missing = RuntimeHardwareDiagnosticsContractSummary {
            device_profile_reported: false,
            device_profile_known: false,
            device_profile_matches_hardware: false,
            primary_lane_reported: false,
            primary_lane_known: false,
            primary_lane_matches_hardware: false,
            fallback_lane_reported: false,
            fallback_lane_known: false,
            fallback_lane_matches_hardware: false,
            memory_mode_reported: false,
            memory_mode_known: false,
            memory_mode_matches_hardware: false,
        };

        assert_eq!(
            clean.runtime_hardware_contract_admission_signal_component_count(),
            4
        );
        assert!(clean.has_runtime_hardware_contract_admission_signals());
        assert_eq!(clean.missing_hardware_contract_report_component_count(), 0);
        assert_eq!(
            clean.runtime_hardware_contract_admission_blocker_component_count(),
            0
        );
        assert!(!clean.has_runtime_hardware_contract_admission_blockers());
        assert!(clean.runtime_hardware_contract_admission_accounting_is_consistent());
        assert!(clean.runtime_hardware_contract_admission_is_clean());
        assert!(clean.can_admit_runtime_hardware_contract());
        assert!(clean.can_accept_runtime_hardware_contract());

        assert_eq!(
            drift.runtime_hardware_contract_admission_signal_component_count(),
            0
        );
        assert!(!drift.has_runtime_hardware_contract_admission_signals());
        assert_eq!(drift.missing_hardware_contract_report_component_count(), 0);
        assert_eq!(drift.hardware_contract_problem_component_count(), 4);
        assert_eq!(
            drift.runtime_hardware_contract_admission_blocker_component_count(),
            4
        );
        assert!(drift.has_runtime_hardware_contract_admission_blockers());
        assert!(drift.runtime_hardware_contract_admission_accounting_is_consistent());
        assert!(!drift.runtime_hardware_contract_admission_is_clean());
        assert!(!drift.can_admit_runtime_hardware_contract());
        assert!(!drift.can_accept_runtime_hardware_contract());

        assert_eq!(
            missing.runtime_hardware_contract_admission_signal_component_count(),
            0
        );
        assert!(!missing.has_runtime_hardware_contract_admission_signals());
        assert_eq!(
            missing.missing_hardware_contract_report_component_count(),
            4
        );
        assert_eq!(missing.hardware_contract_problem_component_count(), 0);
        assert_eq!(
            missing.runtime_hardware_contract_admission_blocker_component_count(),
            4
        );
        assert!(missing.has_runtime_hardware_contract_admission_blockers());
        assert!(missing.runtime_hardware_contract_admission_accounting_is_consistent());
        assert!(!missing.runtime_hardware_contract_admission_is_clean());
        assert!(!missing.can_admit_runtime_hardware_contract());
        assert!(missing.can_accept_runtime_hardware_contract());
    }

    #[test]
    fn runtime_hardware_diagnostics_summary_counts_public_shape_drift() {
        let clean = RuntimeHardwareDiagnosticsSummary {
            accepted: true,
            hardware_violation_count: 0,
            failure_report_count: 0,
        };
        let drift = RuntimeHardwareDiagnosticsSummary {
            accepted: true,
            hardware_violation_count: 2,
            failure_report_count: 0,
        };

        assert_eq!(clean.hardware_failure_component_count(), 0);
        assert!(!clean.has_failure_reports());
        assert_eq!(clean.mapped_failure_report_component_count(), 0);
        assert_eq!(clean.hardware_acceptance_problem_component_count(), 0);
        assert!(!clean.has_hardware_acceptance_problem_components());
        assert!(clean.failure_report_matches_violations());
        assert!(clean.hardware_acceptance_accounting_is_consistent());
        assert!(clean.is_clean_acceptance());
        assert!(clean.hardware_acceptance_shape_is_clean());
        assert!(clean.can_accept_runtime_hardware_diagnostics());

        assert!(drift.has_hardware_violations());
        assert_eq!(drift.hardware_failure_component_count(), 1);
        assert!(!drift.has_failure_reports());
        assert_eq!(drift.mapped_failure_report_component_count(), 0);
        assert_eq!(drift.hardware_acceptance_problem_component_count(), 1);
        assert!(drift.has_hardware_acceptance_problem_components());
        assert!(!drift.failure_report_matches_violations());
        assert!(!drift.hardware_acceptance_accounting_is_consistent());
        assert!(!drift.is_clean_acceptance());
        assert!(!drift.hardware_acceptance_shape_is_clean());
        assert!(!drift.can_accept_runtime_hardware_diagnostics());
    }

    #[test]
    fn runtime_hardware_diagnostics_summary_exposes_runtime_admission_boundary() {
        let clean = RuntimeHardwareDiagnosticsSummary {
            accepted: true,
            hardware_violation_count: 0,
            failure_report_count: 0,
        };
        let rejected = RuntimeHardwareDiagnosticsSummary {
            accepted: false,
            hardware_violation_count: 4,
            failure_report_count: 1,
        };
        let drift = RuntimeHardwareDiagnosticsSummary {
            accepted: true,
            hardware_violation_count: 1,
            failure_report_count: 0,
        };

        assert_eq!(clean.runtime_hardware_admission_signal_component_count(), 1);
        assert!(clean.has_runtime_hardware_admission_signals());
        assert_eq!(
            clean.runtime_hardware_admission_blocker_component_count(),
            0
        );
        assert!(!clean.has_runtime_hardware_admission_blockers());
        assert!(clean.runtime_hardware_admission_accounting_is_consistent());
        assert!(clean.runtime_hardware_admission_is_clean());
        assert!(clean.can_admit_runtime_hardware_diagnostics());
        assert_eq!(
            clean.can_admit_runtime_hardware_diagnostics(),
            clean.can_accept_runtime_hardware_diagnostics()
        );

        assert_eq!(
            rejected.runtime_hardware_admission_signal_component_count(),
            0
        );
        assert!(!rejected.has_runtime_hardware_admission_signals());
        assert_eq!(rejected.hardware_acceptance_problem_component_count(), 2);
        assert_eq!(
            rejected.runtime_hardware_admission_blocker_component_count(),
            3
        );
        assert!(rejected.has_runtime_hardware_admission_blockers());
        assert!(rejected.runtime_hardware_admission_accounting_is_consistent());
        assert!(!rejected.runtime_hardware_admission_is_clean());
        assert!(!rejected.can_admit_runtime_hardware_diagnostics());
        assert!(!rejected.can_accept_runtime_hardware_diagnostics());

        assert_eq!(drift.runtime_hardware_admission_signal_component_count(), 1);
        assert!(drift.has_runtime_hardware_admission_signals());
        assert_eq!(drift.hardware_acceptance_problem_component_count(), 1);
        assert_eq!(
            drift.runtime_hardware_admission_blocker_component_count(),
            1
        );
        assert!(drift.has_runtime_hardware_admission_blockers());
        assert!(!drift.runtime_hardware_admission_accounting_is_consistent());
        assert!(!drift.runtime_hardware_admission_is_clean());
        assert!(!drift.can_admit_runtime_hardware_diagnostics());
        assert!(!drift.can_accept_runtime_hardware_diagnostics());
    }

    #[test]
    fn runtime_device_execution_envelope_summary_requires_all_admissions() {
        let runtime_contract = RuntimeDiagnosticsContractSummary {
            model_id_reported: true,
            model_id_matches_metadata: true,
            layer_count_reported: true,
            layer_count_matches_architecture: true,
            hidden_size_reported: true,
            hidden_size_matches_architecture: true,
            local_window_tokens_reported: true,
            local_window_tokens_within_context: true,
            selected_adapter_reported: true,
            selected_adapter_within_execution: true,
            hot_kv_precision_reported: true,
            hot_kv_precision_within_metadata: true,
            cold_kv_precision_reported: true,
            cold_kv_precision_within_metadata: true,
            kv_precision_pair_reported: true,
            kv_precision_pair_valid: true,
        };
        let missing_runtime_contract = RuntimeDiagnosticsContractSummary {
            model_id_reported: false,
            model_id_matches_metadata: false,
            layer_count_reported: false,
            layer_count_matches_architecture: false,
            hidden_size_reported: false,
            hidden_size_matches_architecture: false,
            local_window_tokens_reported: false,
            local_window_tokens_within_context: false,
            selected_adapter_reported: false,
            selected_adapter_within_execution: false,
            hot_kv_precision_reported: false,
            hot_kv_precision_within_metadata: false,
            cold_kv_precision_reported: false,
            cold_kv_precision_within_metadata: false,
            kv_precision_pair_reported: false,
            kv_precision_pair_valid: false,
        };
        let hardware_contract = RuntimeHardwareDiagnosticsContractSummary {
            device_profile_reported: true,
            device_profile_known: true,
            device_profile_matches_hardware: true,
            primary_lane_reported: true,
            primary_lane_known: true,
            primary_lane_matches_hardware: true,
            fallback_lane_reported: true,
            fallback_lane_known: true,
            fallback_lane_matches_hardware: true,
            memory_mode_reported: true,
            memory_mode_known: true,
            memory_mode_matches_hardware: true,
        };
        let hardware_diagnostics = RuntimeHardwareDiagnosticsSummary {
            accepted: true,
            hardware_violation_count: 0,
            failure_report_count: 0,
        };
        let rejected_hardware_diagnostics = RuntimeHardwareDiagnosticsSummary {
            accepted: false,
            hardware_violation_count: 1,
            failure_report_count: 1,
        };

        let clean = RuntimeDeviceExecutionEnvelopeSummary::from_admission_summaries(
            runtime_contract,
            hardware_contract,
            hardware_diagnostics,
        );
        let missing_runtime = RuntimeDeviceExecutionEnvelopeSummary::from_admission_summaries(
            missing_runtime_contract,
            hardware_contract,
            hardware_diagnostics,
        );
        let rejected_hardware = RuntimeDeviceExecutionEnvelopeSummary::from_admission_summaries(
            runtime_contract,
            hardware_contract,
            rejected_hardware_diagnostics,
        );

        assert!(clean.runtime_diagnostics_contract_admitted);
        assert!(clean.hardware_contract_admitted);
        assert!(clean.hardware_diagnostics_admitted);
        assert_eq!(
            clean.runtime_device_execution_envelope_admission_signal_component_count(),
            3
        );
        assert!(clean.has_runtime_device_execution_envelope_admission_signals());
        assert_eq!(
            clean.runtime_device_execution_envelope_admission_blocker_component_count(),
            0
        );
        assert!(!clean.has_runtime_device_execution_envelope_admission_blockers());
        assert!(clean.runtime_device_execution_envelope_admission_accounting_is_consistent());
        assert!(clean.runtime_device_execution_envelope_admission_is_clean());
        assert!(clean.can_submit_runtime_device_execution_envelope());

        assert!(!missing_runtime.runtime_diagnostics_contract_admitted);
        assert!(missing_runtime.hardware_contract_admitted);
        assert!(missing_runtime.hardware_diagnostics_admitted);
        assert_eq!(
            missing_runtime.runtime_device_execution_envelope_admission_signal_component_count(),
            2
        );
        assert_eq!(
            missing_runtime.runtime_device_execution_envelope_admission_blocker_component_count(),
            1
        );
        assert!(missing_runtime.has_runtime_device_execution_envelope_admission_blockers());
        assert!(
            missing_runtime.runtime_device_execution_envelope_admission_accounting_is_consistent()
        );
        assert!(!missing_runtime.runtime_device_execution_envelope_admission_is_clean());
        assert!(!missing_runtime.can_submit_runtime_device_execution_envelope());

        assert!(rejected_hardware.runtime_diagnostics_contract_admitted);
        assert!(rejected_hardware.hardware_contract_admitted);
        assert!(!rejected_hardware.hardware_diagnostics_admitted);
        assert_eq!(
            rejected_hardware.runtime_device_execution_envelope_admission_signal_component_count(),
            2
        );
        assert_eq!(
            rejected_hardware.runtime_device_execution_envelope_admission_blocker_component_count(),
            1
        );
        assert!(rejected_hardware.has_runtime_device_execution_envelope_admission_blockers());
        assert!(
            rejected_hardware
                .runtime_device_execution_envelope_admission_accounting_is_consistent()
        );
        assert!(!rejected_hardware.runtime_device_execution_envelope_admission_is_clean());
        assert!(!rejected_hardware.can_submit_runtime_device_execution_envelope());
    }

    #[test]
    fn runtime_diagnostics_builds_device_execution_envelope_summary() {
        let metadata = RuntimeMetadata::new("model", "tok", 2048, 4096).with_kv_precision(8, 4);
        let architecture = TransformerRuntimeArchitecture::new(24, 4096, 16, 16, 2048);
        let execution =
            AdapterExecutionContext::new([RuntimeAdapter::CpuSimd, RuntimeAdapter::PortableRust]);
        let hardware = crate::hardware::HardwareAllocator::new().plan(
            crate::hardware::HardwareLoadSnapshot::new(
                crate::hardware::DeviceClass::DiscreteGpu,
                0.1,
                0.1,
                0.1,
                0.1,
            ),
            TaskProfile::Coding,
            512,
            HierarchyWeights::for_profile(TaskProfile::Coding),
        );
        let clean_diagnostics = RuntimeDiagnostics::empty()
            .with_model_id("model")
            .with_selected_adapter(RuntimeAdapter::CpuSimd)
            .with_architecture(24, 4096, 2048)
            .with_device_execution(
                "gpu",
                "gpu",
                "cpu",
                "gpu-resident",
                DeviceExecutionSource::RuntimeReported,
            )
            .with_kv_precision(8, 4);
        let control_plane_filled_diagnostics = RuntimeDiagnostics::empty()
            .with_model_id("model")
            .with_selected_adapter(RuntimeAdapter::CpuSimd)
            .with_architecture(24, 4096, 2048)
            .with_device_execution(
                "gpu",
                "gpu",
                "cpu",
                "gpu-resident",
                DeviceExecutionSource::ControlPlaneFilled,
            )
            .with_kv_precision(8, 4);
        let drifted_hardware_diagnostics = RuntimeDiagnostics::empty()
            .with_model_id("model")
            .with_selected_adapter(RuntimeAdapter::CpuSimd)
            .with_architecture(24, 4096, 2048)
            .with_device_execution(
                "cpu",
                "cpu",
                "gpu",
                "tiered",
                DeviceExecutionSource::RuntimeReported,
            )
            .with_kv_precision(8, 4);

        let clean = clean_diagnostics.device_execution_envelope_summary(
            &metadata,
            architecture,
            &execution,
            &hardware,
        );
        let control_plane_filled = control_plane_filled_diagnostics
            .device_execution_envelope_summary(&metadata, architecture, &execution, &hardware);
        let drifted_hardware = drifted_hardware_diagnostics.device_execution_envelope_summary(
            &metadata,
            architecture,
            &execution,
            &hardware,
        );

        assert!(clean.runtime_diagnostics_contract_admitted);
        assert!(clean.hardware_contract_admitted);
        assert!(clean.hardware_diagnostics_admitted);
        assert_eq!(
            clean.runtime_device_execution_envelope_admission_signal_component_count(),
            3
        );
        assert!(clean.runtime_device_execution_envelope_admission_accounting_is_consistent());
        assert!(clean.can_submit_runtime_device_execution_envelope());
        assert!(
            clean_diagnostics.can_submit_runtime_reported_device_execution_envelope(
                &metadata,
                architecture,
                &execution,
                &hardware
            )
        );

        assert!(control_plane_filled.runtime_diagnostics_contract_admitted);
        assert!(control_plane_filled.hardware_contract_admitted);
        assert!(control_plane_filled.hardware_diagnostics_admitted);
        assert!(control_plane_filled.can_submit_runtime_device_execution_envelope());
        assert!(
            !control_plane_filled_diagnostics
                .can_submit_runtime_reported_device_execution_envelope(
                    &metadata,
                    architecture,
                    &execution,
                    &hardware
                )
        );

        assert!(drifted_hardware.runtime_diagnostics_contract_admitted);
        assert!(!drifted_hardware.hardware_contract_admitted);
        assert!(!drifted_hardware.hardware_diagnostics_admitted);
        assert_eq!(
            drifted_hardware.runtime_device_execution_envelope_admission_signal_component_count(),
            1
        );
        assert_eq!(
            drifted_hardware.runtime_device_execution_envelope_admission_blocker_component_count(),
            2
        );
        assert!(
            drifted_hardware.runtime_device_execution_envelope_admission_accounting_is_consistent()
        );
        assert!(!drifted_hardware.can_submit_runtime_device_execution_envelope());
        assert!(
            !drifted_hardware_diagnostics.can_submit_runtime_reported_device_execution_envelope(
                &metadata,
                architecture,
                &execution,
                &hardware
            )
        );
    }

    #[test]
    fn runtime_diagnostics_summary_blocks_device_execution_source_shape_drift() {
        let missing_source = RuntimeDiagnosticsSummary {
            has_model_id: true,
            has_selected_adapter: true,
            has_runtime_architecture: true,
            layer_count: 4,
            layer_mode_count: 3,
            has_all_layer_modes: true,
            has_device_execution: true,
            device_execution_source: None,
            has_forward_energy: true,
            has_kv_influence: false,
            imported_kv_blocks: 0,
            exported_kv_blocks: 0,
            weak_runtime_kv_imports_skipped: 0,
            has_valid_kv_precision: true,
            has_forward_signal: true,
        };
        let detached_source = RuntimeDiagnosticsSummary {
            has_device_execution: false,
            device_execution_source: Some(DeviceExecutionSource::ControlPlaneFilled),
            ..missing_source
        };

        assert!(!missing_source.has_device_execution_source());
        assert!(!missing_source.has_runtime_reported_device_execution());
        assert!(!missing_source.has_control_plane_filled_device_execution());
        assert!(!missing_source.device_execution_source_matches_execution());
        assert_eq!(
            missing_source.device_execution_source_problem_component_count(),
            1
        );
        assert_eq!(missing_source.runtime_identity_problem_component_count(), 0);
        assert_eq!(
            missing_source.runtime_architecture_problem_component_count(),
            0
        );
        assert_eq!(missing_source.runtime_activity_problem_component_count(), 0);
        assert_eq!(
            missing_source.runtime_precision_problem_component_count(),
            0
        );
        assert_eq!(
            missing_source.runtime_diagnostics_problem_component_count(),
            1
        );
        assert!(missing_source.has_runtime_diagnostics_problem_components());
        assert!(missing_source.runtime_diagnostics_accounting_is_consistent());
        assert!(!missing_source.runtime_diagnostics_shape_is_clean());
        assert!(!missing_source.can_use_runtime_diagnostics());

        assert!(detached_source.has_device_execution_source());
        assert!(!detached_source.has_runtime_reported_device_execution());
        assert!(!detached_source.has_control_plane_filled_device_execution());
        assert!(!detached_source.device_execution_source_matches_execution());
        assert_eq!(
            detached_source.device_execution_source_problem_component_count(),
            1
        );
        assert_eq!(
            detached_source.runtime_diagnostics_problem_component_count(),
            1
        );
        assert!(detached_source.runtime_diagnostics_accounting_is_consistent());
        assert!(!detached_source.runtime_diagnostics_shape_is_clean());
        assert!(!detached_source.can_use_runtime_diagnostics());
    }

    #[test]
    fn embedding_diagnostics_counts_runtime_and_fallback_calls() {
        let mut diagnostics = EmbeddingDiagnostics::from_query(EmbeddingCallDiagnostics::new(
            EmbeddingSource::Runtime,
            2048,
        ));
        diagnostics.record_memory_write(EmbeddingCallDiagnostics::new(
            EmbeddingSource::Fallback,
            384,
        ));
        diagnostics.record_gist_write(EmbeddingCallDiagnostics::new(
            EmbeddingSource::Runtime,
            2048,
        ));
        diagnostics.record_gist_write(EmbeddingCallDiagnostics::new(
            EmbeddingSource::Fallback,
            384,
        ));

        let summary = diagnostics.diagnostics_summary();

        assert_eq!(diagnostics.total_calls(), 4);
        assert_eq!(diagnostics.runtime_calls, 2);
        assert_eq!(diagnostics.fallback_calls, 2);
        assert_eq!(diagnostics.gist_write_runtime_calls(), 1);
        assert_eq!(diagnostics.gist_write_fallback_calls(), 1);
        assert!(diagnostics.runtime_embedding_available());
        assert!(diagnostics.fallback_embedding_used());
        assert_eq!(summary.query_source, EmbeddingSource::Runtime);
        assert_eq!(summary.query_dimensions, 2048);
        assert!(summary.has_memory_write);
        assert_eq!(summary.memory_write_source, Some(EmbeddingSource::Fallback));
        assert_eq!(summary.memory_write_dimensions, 384);
        assert_eq!(summary.gist_write_count, 2);
        assert_eq!(summary.gist_runtime_write_count, 1);
        assert_eq!(summary.gist_fallback_write_count, 1);
        assert_eq!(summary.runtime_calls, 2);
        assert_eq!(summary.fallback_calls, 2);
        assert_eq!(summary.total_calls, 4);
        assert!(summary.has_query_dimensions());
        assert!(summary.runtime_embedding_available());
        assert!(summary.fallback_embedding_used());
        assert!(summary.has_gist_writes());
        assert!(summary.has_memory_write_source());
        assert!(summary.uses_mixed_embedding_sources());
        assert!(summary.query_dimensions_shape_is_valid());
        assert!(summary.memory_write_shape_is_valid());
        assert!(summary.gist_writes_match_total());
        assert!(summary.call_counts_match_total());
        assert_eq!(summary.embedding_signal_component_count(), 6);
        assert!(summary.has_embedding_signals());
        assert_eq!(summary.embedding_shape_problem_component_count(), 0);
        assert!(!summary.has_embedding_shape_problem_components());
        assert!(summary.embedding_accounting_is_consistent());
        assert!(summary.embedding_summary_is_clean());
        assert!(summary.can_use_embedding_diagnostics());
    }

    #[test]
    fn embedding_diagnostics_summary_counts_public_shape_drift() {
        let empty = EmbeddingDiagnostics::default().diagnostics_summary();
        let drift = EmbeddingDiagnosticsSummary {
            query_source: EmbeddingSource::Fallback,
            query_dimensions: 0,
            has_memory_write: true,
            memory_write_source: None,
            memory_write_dimensions: 0,
            gist_write_count: 3,
            gist_runtime_write_count: 1,
            gist_fallback_write_count: 1,
            runtime_calls: 1,
            fallback_calls: 1,
            total_calls: 4,
        };

        assert!(!empty.has_embedding_signals());
        assert!(!empty.embedding_summary_is_clean());
        assert!(!empty.can_use_embedding_diagnostics());

        assert!(!drift.has_query_dimensions());
        assert!(drift.has_memory_write);
        assert!(!drift.has_memory_write_source());
        assert!(drift.has_gist_writes());
        assert!(drift.runtime_embedding_available());
        assert!(drift.fallback_embedding_used());
        assert!(drift.uses_mixed_embedding_sources());
        assert!(!drift.query_dimensions_shape_is_valid());
        assert!(!drift.memory_write_shape_is_valid());
        assert!(!drift.gist_writes_match_total());
        assert!(!drift.call_counts_match_total());
        assert_eq!(drift.embedding_signal_component_count(), 5);
        assert!(drift.has_embedding_signals());
        assert_eq!(drift.embedding_shape_problem_component_count(), 4);
        assert!(drift.has_embedding_shape_problem_components());
        assert!(!drift.embedding_accounting_is_consistent());
        assert!(!drift.embedding_summary_is_clean());
        assert!(!drift.can_use_embedding_diagnostics());
    }

    #[test]
    fn inference_diagnostics_summarizes_runtime_execution() {
        let generation_budget =
            RuntimeMetadata::new("model", "tok", 1024, 2048).generation_budget(900, 200);
        let runtime = RuntimeDiagnostics::empty()
            .with_kv_exchange(1, 2)
            .with_forward_signals(Some(0.3), Some(0.7));
        let mut diagnostics = InferenceDiagnostics::new(RouteBudget {
            threshold: 0.5,
            attention_tokens: 3,
            fast_tokens: 1,
            attention_fraction: 0.75,
        })
        .with_runtime(runtime)
        .with_generation_budget(generation_budget)
        .with_hardware(1.2, -0.1, Some(80))
        .with_recursive_runtime_calls(2);
        diagnostics.push_note("runtime_error:timeout=false");

        assert_eq!(diagnostics.kv_exchange_total(), 3);
        assert!(diagnostics.has_runtime_execution_signal());
        assert_eq!(diagnostics.hardware_pressure, 1.0);
        assert_eq!(diagnostics.compute_headroom, 0.0);
        assert_eq!(diagnostics.notes.len(), 1);
        assert!(diagnostics.generation_budget.unwrap().truncated_by_context);

        let summary = diagnostics.diagnostics_summary();

        assert!(summary.has_generation_budget);
        assert!(summary.generation_truncated_by_context);
        assert_eq!(summary.route_attention_tokens, 3);
        assert_eq!(summary.route_fast_tokens, 1);
        assert_eq!(summary.route_token_total(), 4);
        assert!(summary.has_route_activity());
        assert_eq!(summary.runtime_kv_exchange_total, 3);
        assert_eq!(summary.weak_runtime_kv_imports_skipped, 0);
        assert!(summary.has_runtime_kv_exchange());
        assert!(!summary.has_weak_runtime_kv_import_skips());
        assert!(summary.has_runtime_execution_signal);
        assert!(summary.has_runtime_or_embedding_execution());
        assert!(!summary.runtime_embedding_available);
        assert!(!summary.used_any_embedding_fallback());
        assert_eq!(
            summary.hardware_pressure_band,
            DiagnosticsPressureBand::Critical
        );
        assert!(summary.hardware_pressure_band.is_constrained());
        assert!(summary.has_latency_budget);
        assert_eq!(summary.recursive_runtime_calls, 2);
        assert!(summary.has_recursive_runtime());
        assert_eq!(summary.note_count, 1);
        assert!(summary.has_notes());
        assert!(summary.has_complete_diagnostics_signal());
    }

    #[test]
    fn inference_diagnostics_preserves_weak_runtime_kv_skip_without_exchange_count() {
        let generation_budget =
            RuntimeMetadata::new("model", "tok", 1024, 2048).generation_budget(900, 200);
        let runtime = RuntimeDiagnostics::empty().with_weak_runtime_kv_imports_skipped(2);
        let diagnostics = InferenceDiagnostics::new(RouteBudget::default())
            .with_runtime(runtime)
            .with_generation_budget(generation_budget);

        assert_eq!(diagnostics.kv_exchange_total(), 0);
        assert!(diagnostics.has_runtime_execution_signal());

        let summary = diagnostics.diagnostics_summary();

        assert_eq!(summary.runtime_kv_exchange_total, 0);
        assert_eq!(summary.weak_runtime_kv_imports_skipped, 2);
        assert!(!summary.has_runtime_kv_exchange());
        assert!(summary.has_weak_runtime_kv_import_skips());
        assert!(summary.has_runtime_execution_signal);
        assert!(summary.has_runtime_or_embedding_execution());
        assert!(summary.has_complete_diagnostics_signal());
    }

    #[test]
    fn inference_diagnostics_can_seed_from_request_envelope() {
        let runtime = RuntimeMetadata::new("model", "tok", 128, 16)
            .with_kv_exchange(true, true)
            .with_kv_limits(2, 2);
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(96)
            .with_max_tokens(64)
            .with_runtime(runtime);
        let route_budget = RouteBudget {
            threshold: 0.42,
            attention_tokens: 7,
            fast_tokens: 3,
            attention_fraction: 0.70,
        };
        let execution = AdapterExecutionContext::new([RuntimeAdapter::Cuda])
            .with_pressure(0.80, 0.20)
            .with_latency_budget_ms(Some(90))
            .with_kv_prefetch_blocks(4);
        let planning = RuntimePlanningDigest::from_request(
            &request,
            route_budget,
            &execution,
            &[],
            &DeterministicFhtDkeBudgeter::default(),
        );
        let envelope =
            request_envelope(&request, route_budget, &execution, 0).with_planning_digest(planning);

        let diagnostics = InferenceDiagnostics::from_request_envelope(&envelope);

        assert_eq!(diagnostics.route_budget, route_budget);
        assert_eq!(
            diagnostics.generation_budget,
            Some(planning.generation_budget)
        );
        assert_eq!(diagnostics.hardware_pressure, planning.hardware_pressure);
        assert_eq!(diagnostics.compute_headroom, planning.compute_headroom);
        assert_eq!(diagnostics.latency_budget_ms, Some(90));
        assert_eq!(diagnostics.runtime.model_id.as_deref(), Some("model"));
        assert_eq!(
            diagnostics.runtime.selected_adapter,
            Some(RuntimeAdapter::Cuda)
        );
        assert_eq!(
            diagnostics.runtime.layer_count,
            envelope.architecture.layer_count
        );
        assert_eq!(
            diagnostics.runtime.hidden_size,
            envelope.architecture.hidden_size
        );
        assert_eq!(
            diagnostics.runtime.local_window_tokens,
            envelope.architecture.local_window_tokens
        );
        assert_eq!(diagnostics.runtime.imported_kv_blocks, 0);
        assert_eq!(diagnostics.runtime.hot_kv_precision_bits, Some(8));
        assert_eq!(diagnostics.runtime.cold_kv_precision_bits, Some(4));

        let summary = diagnostics.diagnostics_summary();
        let parity = diagnostics.request_parity_summary(&envelope);

        assert!(summary.has_generation_budget);
        assert!(summary.generation_truncated_by_context);
        assert_eq!(summary.route_attention_tokens, 7);
        assert_eq!(summary.route_fast_tokens, 3);
        assert_eq!(summary.runtime_kv_exchange_total, 0);
        assert_eq!(
            summary.hardware_pressure_band,
            DiagnosticsPressureBand::High
        );
        assert!(summary.has_latency_budget);
        assert!(parity.route_budget_matches_request);
        assert!(parity.generation_budget_reported);
        assert!(parity.generation_budget_matches_request);
        assert!(parity.hardware_pressure_matches_request);
        assert!(parity.has_planning_digest);
        assert_eq!(parity.compute_headroom_matches_planning, Some(true));
        assert_eq!(parity.latency_budget_matches_planning, Some(true));
        assert!(parity.routing_parity_ok());
        assert!(parity.generation_parity_ok());
        assert!(parity.hardware_parity_ok());
        assert!(parity.runtime_parity_ok());
        assert!(!parity.routing_drifted());
        assert!(!parity.generation_budget_missing());
        assert!(!parity.generation_budget_drifted());
        assert!(!parity.hardware_pressure_drifted());
        assert!(!parity.compute_headroom_drifted());
        assert!(!parity.latency_budget_drifted());
        assert!(!parity.planning_hardware_drifted());
        assert!(!parity.runtime_drifted());
        assert!(!parity.missing_required_diagnostics_report());
        assert!(!parity.has_request_drift());
        assert_eq!(parity.routing_drift_component_count(), 0);
        assert_eq!(parity.generation_drift_component_count(), 0);
        assert_eq!(parity.hardware_drift_component_count(), 0);
        assert_eq!(parity.runtime.runtime_drift_component_count(), 0);
        assert!(!parity.runtime.has_runtime_drift_components());
        assert!(parity.runtime.runtime_drift_accounting_is_consistent());
        assert!(parity.runtime.runtime_request_parity_shape_is_clean());
        assert!(
            parity
                .runtime
                .can_accept_runtime_diagnostics_request_parity()
        );
        assert_eq!(parity.diagnostics_request_drift_component_count(), 0);
        assert!(!parity.has_diagnostics_request_drift_components());
        assert!(parity.diagnostics_request_accounting_is_consistent());
        assert!(parity.request_parity_is_consistent());
        assert!(parity.diagnostics_request_parity_shape_is_clean());
        assert!(parity.can_accept_inference_diagnostics_request_parity());
    }

    #[test]
    fn request_envelope_seeding_preserves_runtime_diagnostics() {
        let request = InferenceRequest::new("prompt", TaskProfile::General)
            .with_prompt_tokens(8)
            .with_max_tokens(8)
            .with_runtime(RuntimeMetadata::new("model", "tok", 128, 16));
        let execution =
            AdapterExecutionContext::new([RuntimeAdapter::CpuSimd]).with_pressure(0.30, 0.60);
        let envelope = request_envelope(&request, RouteBudget::default(), &execution, 0);
        let runtime = RuntimeDiagnostics::empty()
            .with_model_id("runtime-reported")
            .with_selected_adapter(RuntimeAdapter::Cuda)
            .with_architecture(2, 32, 128)
            .with_kv_exchange(3, 4)
            .with_kv_precision(4, 4);

        let diagnostics = InferenceDiagnostics::new(RouteBudget {
            threshold: 0.99,
            attention_tokens: 0,
            fast_tokens: 1,
            attention_fraction: 0.0,
        })
        .with_runtime(runtime.clone())
        .with_request_envelope(&envelope);

        assert_eq!(diagnostics.runtime, runtime);
        assert_eq!(diagnostics.route_budget, envelope.route_budget);
        assert_eq!(
            diagnostics.generation_budget,
            Some(envelope.generation_budget)
        );
        assert_eq!(diagnostics.hardware_pressure, 0.30);
        assert_eq!(diagnostics.compute_headroom, 0.5);
        assert_eq!(diagnostics.latency_budget_ms, None);

        let summary = diagnostics.diagnostics_summary();

        assert!(summary.has_generation_budget);
        assert!(!summary.generation_truncated_by_context);
        assert_eq!(
            summary.route_attention_tokens,
            envelope.route_budget.attention_tokens
        );
        assert_eq!(summary.route_fast_tokens, envelope.route_budget.fast_tokens);
        assert_eq!(summary.hardware_pressure_band, DiagnosticsPressureBand::Low);
        assert!(!summary.hardware_pressure_band.is_constrained());
        assert!(!summary.has_latency_budget);
    }

    #[test]
    fn inference_diagnostics_request_parity_counts_missing_generation_budget_only() {
        let request = InferenceRequest::new("prompt", TaskProfile::General)
            .with_prompt_tokens(8)
            .with_max_tokens(8)
            .with_runtime(RuntimeMetadata::new("model", "tok", 128, 16));
        let execution = AdapterExecutionContext::new([RuntimeAdapter::CpuSimd]);
        let envelope = request_envelope(&request, RouteBudget::default(), &execution, 0);
        let mut diagnostics = InferenceDiagnostics::from_request_envelope(&envelope);
        diagnostics.generation_budget = None;

        let parity = diagnostics.request_parity_summary(&envelope);

        assert!(parity.generation_budget_missing());
        assert!(!parity.generation_budget_drifted());
        assert!(parity.has_request_drift());
        assert!(!parity.runtime.has_runtime_drift_components());
        assert_eq!(parity.generation_drift_component_count(), 1);
        assert_eq!(parity.diagnostics_request_drift_component_count(), 1);
        assert!(parity.has_diagnostics_request_drift_components());
        assert!(parity.diagnostics_request_accounting_is_consistent());
        assert!(!parity.request_parity_is_consistent());
        assert!(!parity.diagnostics_request_parity_shape_is_clean());
        assert!(!parity.can_accept_inference_diagnostics_request_parity());
    }

    #[test]
    fn inference_diagnostics_request_parity_blocks_stale_hierarchical_route_budget() {
        let runtime = RuntimeMetadata::new("model", "tok", 128, 16);
        let request = InferenceRequest::new("prompt", TaskProfile::General)
            .with_prompt_tokens(8)
            .with_max_tokens(8)
            .with_runtime(runtime);
        let router = DefaultHierarchicalRouter::new();
        let token = TokenFeatures::new("borderline", 0.66, 0);
        let routing_context = RoutingContext {
            hierarchy: HierarchyWeights::new(1.0, 0.0, 0.0),
            ..RoutingContext::default()
        };
        let route_budget = router.budget(&[token], routing_context);
        let stale_fast_only_budget = RouteBudget {
            threshold: route_budget.threshold,
            attention_tokens: 0,
            fast_tokens: route_budget.attention_tokens,
            attention_fraction: 0.0,
        };
        let execution = AdapterExecutionContext::new([RuntimeAdapter::CpuSimd]);
        let planning = RuntimePlanningDigest::from_request(
            &request,
            route_budget,
            &execution,
            &[],
            &DeterministicFhtDkeBudgeter::default(),
        );
        let envelope =
            request_envelope(&request, route_budget, &execution, 0).with_planning_digest(planning);
        let mut diagnostics = InferenceDiagnostics::from_request_envelope(&envelope);
        diagnostics.route_budget = stale_fast_only_budget;

        let summary = diagnostics.diagnostics_summary();
        let parity = diagnostics.request_parity_summary(&envelope);

        assert_eq!(route_budget.attention_tokens, 1);
        assert_eq!(route_budget.fast_tokens, 0);
        assert_eq!(envelope.route_budget, route_budget);
        assert_eq!(summary.route_attention_tokens, 0);
        assert_eq!(summary.route_fast_tokens, 1);
        assert!(parity.generation_parity_ok());
        assert!(parity.hardware_parity_ok());
        assert!(parity.runtime_parity_ok());
        assert!(!parity.route_budget_matches_request);
        assert!(!parity.routing_parity_ok());
        assert!(parity.routing_drifted());
        assert!(!parity.generation_budget_missing());
        assert!(!parity.generation_budget_drifted());
        assert!(!parity.hardware_pressure_drifted());
        assert!(!parity.planning_hardware_drifted());
        assert!(!parity.runtime_drifted());
        assert!(parity.has_request_drift());
        assert_eq!(parity.routing_drift_component_count(), 1);
        assert_eq!(parity.generation_drift_component_count(), 0);
        assert_eq!(parity.hardware_drift_component_count(), 0);
        assert_eq!(parity.runtime.runtime_drift_component_count(), 0);
        assert_eq!(parity.diagnostics_request_drift_component_count(), 1);
        assert!(parity.has_diagnostics_request_drift_components());
        assert!(parity.diagnostics_request_accounting_is_consistent());
        assert!(!parity.request_parity_is_consistent());
        assert!(!parity.diagnostics_request_parity_shape_is_clean());
        assert!(!parity.can_accept_inference_diagnostics_request_parity());
    }

    #[test]
    fn inference_diagnostics_request_parity_blocks_stale_low_pressure_route_after_hardware_demote()
    {
        let runtime = RuntimeMetadata::new("model", "tok", 128, 16);
        let request = InferenceRequest::new("prompt", TaskProfile::General)
            .with_prompt_tokens(8)
            .with_max_tokens(8)
            .with_runtime(runtime);
        let router = DefaultHierarchicalRouter::new();
        let token = TokenFeatures::new("diagnostics-hardware-borderline", 0.80, 0);
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
        let execution =
            AdapterExecutionContext::new([RuntimeAdapter::CpuSimd]).with_pressure(1.0, 0.20);
        let planning = RuntimePlanningDigest::from_request(
            &request,
            high_pressure_budget,
            &execution,
            &[],
            &DeterministicFhtDkeBudgeter::default(),
        );
        let envelope = request_envelope(&request, high_pressure_budget, &execution, 0)
            .with_planning_digest(planning);
        let mut diagnostics = InferenceDiagnostics::from_request_envelope(&envelope);
        diagnostics.route_budget = low_pressure_budget;
        diagnostics.hardware_pressure = 0.0;

        let summary = diagnostics.diagnostics_summary();
        let parity = diagnostics.request_parity_summary(&envelope);

        assert_eq!(
            low_pressure_decision.layer,
            crate::router::RouteLayer::LocalWindow
        );
        assert_eq!(
            high_pressure_decision.layer,
            crate::router::RouteLayer::FastProjection
        );
        assert_eq!(low_pressure_budget.attention_tokens, 1);
        assert_eq!(low_pressure_budget.fast_tokens, 0);
        assert_eq!(high_pressure_budget.attention_tokens, 0);
        assert_eq!(high_pressure_budget.fast_tokens, 1);
        assert_eq!(envelope.route_budget, high_pressure_budget);
        assert_eq!(envelope.hardware_pressure, 1.0);
        assert_eq!(summary.route_attention_tokens, 1);
        assert_eq!(summary.route_fast_tokens, 0);
        assert_eq!(diagnostics.hardware_pressure, 0.0);
        assert_eq!(summary.hardware_pressure_band, DiagnosticsPressureBand::Low);
        assert!(parity.generation_parity_ok());
        assert!(parity.runtime_parity_ok());
        assert!(!parity.route_budget_matches_request);
        assert!(!parity.hardware_pressure_matches_request);
        assert!(!parity.routing_parity_ok());
        assert!(!parity.hardware_parity_ok());
        assert!(parity.routing_drifted());
        assert!(parity.hardware_pressure_drifted());
        assert!(!parity.planning_hardware_drifted());
        assert!(!parity.runtime_drifted());
        assert!(parity.has_request_drift());
        assert_eq!(parity.routing_drift_component_count(), 1);
        assert_eq!(parity.generation_drift_component_count(), 0);
        assert_eq!(parity.hardware_drift_component_count(), 1);
        assert_eq!(parity.runtime.runtime_drift_component_count(), 0);
        assert_eq!(parity.diagnostics_request_drift_component_count(), 2);
        assert!(parity.has_diagnostics_request_drift_components());
        assert!(parity.diagnostics_request_accounting_is_consistent());
        assert!(!parity.request_parity_is_consistent());
        assert!(!parity.diagnostics_request_parity_shape_is_clean());
        assert!(!parity.can_accept_inference_diagnostics_request_parity());
    }

    #[test]
    fn inference_diagnostics_request_parity_summary_reports_request_drift() {
        let runtime = RuntimeMetadata::new("model", "tok", 256, 64)
            .with_kv_exchange(true, true)
            .with_kv_limits(4, 1)
            .with_kv_precision(4, 4);
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(96)
            .with_max_tokens(64)
            .with_runtime(runtime);
        let route_budget = RouteBudget {
            threshold: 0.42,
            attention_tokens: 7,
            fast_tokens: 3,
            attention_fraction: 0.70,
        };
        let execution = AdapterExecutionContext::new([RuntimeAdapter::Metal])
            .with_pressure(0.80, 0.20)
            .with_latency_budget_ms(Some(90));
        let planning = RuntimePlanningDigest::from_request(
            &request,
            route_budget,
            &execution,
            &[],
            &DeterministicFhtDkeBudgeter::default(),
        );
        let envelope =
            request_envelope(&request, route_budget, &execution, 2).with_planning_digest(planning);
        let runtime_diagnostics = RuntimeDiagnostics::empty()
            .with_model_id("other")
            .with_selected_adapter(RuntimeAdapter::Cuda)
            .with_architecture(2, 128, 512)
            .with_kv_exchange(3, 2)
            .with_kv_precision(8, 4);
        let diagnostics = InferenceDiagnostics::new(RouteBudget {
            threshold: 0.10,
            attention_tokens: 1,
            fast_tokens: 9,
            attention_fraction: 0.10,
        })
        .with_runtime(runtime_diagnostics)
        .with_generation_budget(
            RuntimeMetadata::new("other", "tok", 64, 16).generation_budget(8, 1),
        )
        .with_hardware(0.10, 0.10, Some(5));

        let parity = diagnostics.request_parity_summary(&envelope);

        assert!(!parity.route_budget_matches_request);
        assert!(parity.generation_budget_reported);
        assert!(!parity.generation_budget_matches_request);
        assert!(!parity.hardware_pressure_matches_request);
        assert!(parity.has_planning_digest);
        assert_eq!(parity.compute_headroom_matches_planning, Some(false));
        assert_eq!(parity.latency_budget_matches_planning, Some(false));
        assert!(!parity.runtime.model_id_matches_request);
        assert!(!parity.runtime.selected_adapter_matches_request);
        assert!(!parity.runtime.layer_count_matches_request);
        assert!(!parity.runtime.hidden_size_matches_request);
        assert!(!parity.runtime.local_window_tokens_within_request);
        assert!(!parity.runtime.imported_kv_matches_request);
        assert!(!parity.runtime.exported_kv_within_runtime);
        assert!(!parity.runtime.kv_precision_within_request);
        assert!(!parity.routing_parity_ok());
        assert!(!parity.generation_parity_ok());
        assert!(!parity.hardware_parity_ok());
        assert!(!parity.runtime_parity_ok());
        assert!(parity.routing_drifted());
        assert!(!parity.generation_budget_missing());
        assert!(parity.generation_budget_drifted());
        assert!(parity.hardware_pressure_drifted());
        assert!(parity.compute_headroom_drifted());
        assert!(parity.latency_budget_drifted());
        assert!(parity.planning_hardware_drifted());
        assert!(parity.runtime_drifted());
        assert!(!parity.missing_required_diagnostics_report());
        assert!(parity.has_request_drift());
        assert_eq!(parity.runtime.missing_report_component_count(), 0);
        assert_eq!(parity.runtime.identity_drift_component_count(), 2);
        assert_eq!(parity.runtime.architecture_drift_component_count(), 1);
        assert_eq!(parity.runtime.kv_drift_component_count(), 2);
        assert_eq!(parity.runtime.precision_drift_component_count(), 1);
        assert_eq!(parity.runtime.runtime_drift_component_count(), 6);
        assert_eq!(parity.routing_drift_component_count(), 1);
        assert_eq!(parity.generation_drift_component_count(), 1);
        assert_eq!(parity.hardware_drift_component_count(), 3);
        assert_eq!(parity.diagnostics_request_drift_component_count(), 11);
        assert!(parity.runtime.has_runtime_drift_components());
        assert!(parity.runtime.runtime_drift_accounting_is_consistent());
        assert!(parity.has_diagnostics_request_drift_components());
        assert!(parity.diagnostics_request_accounting_is_consistent());
        assert!(!parity.request_parity_is_consistent());
        assert!(!parity.runtime.runtime_request_parity_shape_is_clean());
        assert!(
            !parity
                .runtime
                .can_accept_runtime_diagnostics_request_parity()
        );
        assert!(!parity.diagnostics_request_parity_shape_is_clean());
        assert!(!parity.can_accept_inference_diagnostics_request_parity());
    }

    #[test]
    fn runtime_diagnostics_can_seed_missing_request_fields() {
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(16)
            .with_max_tokens(16)
            .with_runtime(
                RuntimeMetadata::new("seed-model", "tok", 256, 64)
                    .with_kv_exchange(true, true)
                    .with_kv_limits(4, 4)
                    .with_kv_precision(4, 4),
            );
        let execution = AdapterExecutionContext::new([RuntimeAdapter::Metal]);
        let envelope = request_envelope(&request, RouteBudget::default(), &execution, 2);

        let diagnostics = RuntimeDiagnostics::from_request_envelope(&envelope);

        assert_eq!(diagnostics.model_id.as_deref(), Some("seed-model"));
        assert_eq!(diagnostics.selected_adapter, Some(RuntimeAdapter::Metal));
        assert_eq!(diagnostics.layer_count, envelope.architecture.layer_count);
        assert_eq!(diagnostics.hidden_size, envelope.architecture.hidden_size);
        assert_eq!(
            diagnostics.local_window_tokens,
            envelope.architecture.local_window_tokens
        );
        assert_eq!(diagnostics.imported_kv_blocks, 2);
        assert_eq!(diagnostics.exported_kv_blocks, 0);
        assert_eq!(diagnostics.hot_kv_precision_bits, Some(4));
        assert_eq!(diagnostics.cold_kv_precision_bits, Some(4));
        assert!(diagnostics.has_runtime_architecture_signal());
        assert!(diagnostics.has_valid_kv_precision_signal());

        let summary = diagnostics.diagnostics_summary();
        let parity = diagnostics.request_parity_summary(&envelope);

        assert!(summary.has_model_id);
        assert!(summary.has_selected_adapter);
        assert!(summary.has_runtime_architecture);
        assert_eq!(summary.layer_count, envelope.architecture.layer_count);
        assert_eq!(summary.imported_kv_blocks, 2);
        assert_eq!(summary.exported_kv_blocks, 0);
        assert_eq!(summary.kv_exchange_total(), 2);
        assert!(summary.has_runtime_identity());
        assert!(!summary.missing_runtime_identity());
        assert!(summary.has_runtime_kv_exchange());
        assert!(summary.has_valid_kv_precision);
        assert!(summary.has_forward_signal);
        assert!(summary.has_runtime_forward_or_kv_signal());
        assert!(!summary.missing_runtime_architecture());
        assert!(!summary.missing_valid_kv_precision());
        assert!(summary.has_complete_runtime_signal());
        assert_eq!(summary.runtime_identity_signal_component_count(), 2);
        assert_eq!(summary.runtime_architecture_signal_component_count(), 1);
        assert_eq!(summary.device_execution_signal_component_count(), 0);
        assert_eq!(summary.runtime_forward_signal_component_count(), 0);
        assert_eq!(summary.runtime_kv_activity_signal_component_count(), 1);
        assert_eq!(summary.runtime_precision_signal_component_count(), 1);
        assert_eq!(summary.runtime_diagnostics_signal_component_count(), 5);
        assert!(summary.has_runtime_diagnostics_signals());
        assert_eq!(summary.runtime_identity_problem_component_count(), 0);
        assert_eq!(summary.runtime_architecture_problem_component_count(), 0);
        assert_eq!(summary.runtime_activity_problem_component_count(), 0);
        assert_eq!(summary.runtime_precision_problem_component_count(), 0);
        assert_eq!(summary.runtime_diagnostics_problem_component_count(), 0);
        assert!(!summary.has_runtime_diagnostics_problem_components());
        assert!(summary.runtime_diagnostics_accounting_is_consistent());
        assert!(parity.model_parity_ok());
        assert!(parity.adapter_parity_ok());
        assert!(parity.architecture_parity_ok());
        assert!(parity.kv_parity_ok());
        assert!(parity.precision_parity_ok());
        assert!(parity.request_parity_is_consistent());
        assert!(!parity.missing_model_id_report());
        assert!(!parity.missing_selected_adapter_report());
        assert!(!parity.missing_architecture_report());
        assert!(!parity.missing_kv_precision_report());
        assert!(!parity.model_drifted());
        assert!(!parity.adapter_drifted());
        assert!(!parity.architecture_drifted());
        assert!(!parity.kv_count_drifted());
        assert!(!parity.precision_drifted());
        assert!(!parity.missing_required_runtime_report());
        assert_eq!(parity.missing_report_component_count(), 0);
        assert_eq!(parity.identity_drift_component_count(), 0);
        assert_eq!(parity.architecture_drift_component_count(), 0);
        assert_eq!(parity.kv_drift_component_count(), 0);
        assert_eq!(parity.precision_drift_component_count(), 0);
        assert_eq!(parity.runtime_drift_component_count(), 0);
        assert!(!parity.has_runtime_drift_components());
        assert!(parity.runtime_drift_accounting_is_consistent());
        assert!(parity.runtime_request_parity_shape_is_clean());
        assert!(parity.can_accept_runtime_diagnostics_request_parity());
    }

    #[test]
    fn runtime_diagnostics_request_parity_summary_reports_request_drift() {
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(16)
            .with_max_tokens(16)
            .with_runtime(
                RuntimeMetadata::new("seed-model", "tok", 256, 64)
                    .with_kv_exchange(true, true)
                    .with_kv_limits(4, 1)
                    .with_kv_precision(4, 4),
            );
        let execution = AdapterExecutionContext::new([RuntimeAdapter::Metal]);
        let envelope = request_envelope(&request, RouteBudget::default(), &execution, 2);
        let diagnostics = RuntimeDiagnostics::empty()
            .with_model_id("other-model")
            .with_selected_adapter(RuntimeAdapter::Cuda)
            .with_architecture(2, 128, 512)
            .with_kv_exchange(3, 2)
            .with_kv_precision(8, 4);

        let parity = diagnostics.request_parity_summary(&envelope);

        assert!(parity.model_id_reported);
        assert!(!parity.model_id_matches_request);
        assert!(parity.selected_adapter_reported);
        assert!(!parity.selected_adapter_matches_request);
        assert!(parity.architecture_reported);
        assert!(!parity.layer_count_matches_request);
        assert!(!parity.hidden_size_matches_request);
        assert!(!parity.local_window_tokens_within_request);
        assert_eq!(parity.imported_kv_blocks, 3);
        assert_eq!(parity.request_imported_kv_blocks, 2);
        assert!(!parity.imported_kv_matches_request);
        assert_eq!(parity.exported_kv_blocks, 2);
        assert!(parity.runtime_export_enabled);
        assert_eq!(parity.runtime_max_export_blocks, 1);
        assert!(!parity.exported_kv_within_runtime);
        assert!(parity.kv_precision_reported);
        assert!(parity.kv_precision_valid);
        assert!(!parity.kv_precision_within_request);
        assert!(!parity.model_parity_ok());
        assert!(!parity.adapter_parity_ok());
        assert!(!parity.architecture_parity_ok());
        assert!(!parity.kv_parity_ok());
        assert!(!parity.precision_parity_ok());
        assert!(!parity.request_parity_is_consistent());
        assert!(!parity.missing_model_id_report());
        assert!(!parity.missing_selected_adapter_report());
        assert!(!parity.missing_architecture_report());
        assert!(!parity.missing_kv_precision_report());
        assert!(parity.model_drifted());
        assert!(parity.adapter_drifted());
        assert!(parity.architecture_drifted());
        assert!(parity.imported_kv_drifted());
        assert!(parity.exported_kv_exceeds_runtime());
        assert_eq!(parity.exported_kv_block_overflow(), 1);
        assert!(parity.kv_count_drifted());
        assert!(parity.precision_drifted());
        assert!(!parity.missing_required_runtime_report());
        assert_eq!(parity.missing_report_component_count(), 0);
        assert_eq!(parity.identity_drift_component_count(), 2);
        assert_eq!(parity.architecture_drift_component_count(), 1);
        assert_eq!(parity.kv_drift_component_count(), 2);
        assert_eq!(parity.precision_drift_component_count(), 1);
        assert_eq!(parity.runtime_drift_component_count(), 6);
        assert!(parity.has_runtime_drift_components());
        assert!(parity.runtime_drift_accounting_is_consistent());
    }

    #[test]
    fn runtime_diagnostics_request_parity_summary_reports_missing_runtime_fields() {
        let request = InferenceRequest::new("prompt", TaskProfile::General)
            .with_prompt_tokens(8)
            .with_max_tokens(8)
            .with_runtime(RuntimeMetadata::new("seed-model", "tok", 128, 16));
        let execution = AdapterExecutionContext::new([RuntimeAdapter::CpuSimd]);
        let envelope = request_envelope(&request, RouteBudget::default(), &execution, 0);

        let parity = RuntimeDiagnostics::empty().request_parity_summary(&envelope);

        assert!(parity.missing_model_id_report());
        assert!(parity.missing_selected_adapter_report());
        assert!(parity.missing_architecture_report());
        assert!(parity.missing_kv_precision_report());
        assert!(parity.missing_required_runtime_report());
        assert_eq!(parity.missing_report_component_count(), 4);
        assert_eq!(parity.identity_drift_component_count(), 0);
        assert_eq!(parity.architecture_drift_component_count(), 0);
        assert_eq!(parity.kv_drift_component_count(), 0);
        assert_eq!(parity.precision_drift_component_count(), 0);
        assert_eq!(parity.runtime_drift_component_count(), 4);
        assert!(parity.has_runtime_drift_components());
        assert!(parity.runtime_drift_accounting_is_consistent());
        assert!(!parity.model_parity_ok());
        assert!(!parity.adapter_parity_ok());
        assert!(!parity.architecture_parity_ok());
        assert!(!parity.precision_parity_ok());
        assert!(!parity.request_parity_is_consistent());
        assert!(!parity.runtime_request_parity_shape_is_clean());
        assert!(!parity.can_accept_runtime_diagnostics_request_parity());
    }

    #[test]
    fn runtime_diagnostics_request_parity_summary_exposes_admission_boundary() {
        let clean = RuntimeDiagnosticsRequestParitySummary {
            model_id_reported: true,
            model_id_matches_request: true,
            selected_adapter_reported: true,
            selected_adapter_matches_request: true,
            architecture_reported: true,
            layer_count_matches_request: true,
            hidden_size_matches_request: true,
            local_window_tokens_within_request: true,
            imported_kv_blocks: 1,
            request_imported_kv_blocks: 1,
            imported_kv_matches_request: true,
            exported_kv_blocks: 0,
            runtime_export_enabled: true,
            runtime_max_export_blocks: 1,
            exported_kv_within_runtime: true,
            kv_precision_reported: true,
            kv_precision_valid: true,
            kv_precision_within_request: true,
        };
        let drift = RuntimeDiagnosticsRequestParitySummary {
            model_id_matches_request: false,
            selected_adapter_matches_request: false,
            layer_count_matches_request: false,
            hidden_size_matches_request: false,
            local_window_tokens_within_request: false,
            imported_kv_blocks: 2,
            imported_kv_matches_request: false,
            exported_kv_blocks: 2,
            exported_kv_within_runtime: false,
            kv_precision_within_request: false,
            ..clean
        };
        let missing = RuntimeDiagnosticsRequestParitySummary {
            model_id_reported: false,
            model_id_matches_request: false,
            selected_adapter_reported: false,
            selected_adapter_matches_request: false,
            architecture_reported: false,
            layer_count_matches_request: false,
            hidden_size_matches_request: false,
            local_window_tokens_within_request: false,
            imported_kv_blocks: 0,
            request_imported_kv_blocks: 0,
            imported_kv_matches_request: true,
            exported_kv_blocks: 0,
            runtime_export_enabled: false,
            runtime_max_export_blocks: 0,
            exported_kv_within_runtime: true,
            kv_precision_reported: false,
            kv_precision_valid: false,
            kv_precision_within_request: false,
        };

        assert_eq!(
            clean.runtime_request_parity_admission_signal_component_count(),
            5
        );
        assert!(clean.has_runtime_request_parity_admission_signals());
        assert_eq!(
            clean.runtime_request_parity_admission_blocker_component_count(),
            0
        );
        assert!(!clean.has_runtime_request_parity_admission_blockers());
        assert!(clean.runtime_request_parity_admission_accounting_is_consistent());
        assert!(clean.runtime_request_parity_admission_is_clean());
        assert!(clean.can_admit_runtime_diagnostics_request_parity());
        assert!(clean.can_accept_runtime_diagnostics_request_parity());

        assert_eq!(
            drift.runtime_request_parity_admission_signal_component_count(),
            0
        );
        assert!(!drift.has_runtime_request_parity_admission_signals());
        assert_eq!(drift.runtime_drift_component_count(), 6);
        assert_eq!(
            drift.runtime_request_parity_admission_blocker_component_count(),
            7
        );
        assert!(drift.has_runtime_request_parity_admission_blockers());
        assert!(drift.runtime_request_parity_admission_accounting_is_consistent());
        assert!(!drift.runtime_request_parity_admission_is_clean());
        assert!(!drift.can_admit_runtime_diagnostics_request_parity());
        assert!(!drift.can_accept_runtime_diagnostics_request_parity());

        assert_eq!(
            missing.runtime_request_parity_admission_signal_component_count(),
            1
        );
        assert!(missing.has_runtime_request_parity_admission_signals());
        assert_eq!(missing.missing_report_component_count(), 4);
        assert_eq!(missing.runtime_drift_component_count(), 4);
        assert_eq!(
            missing.runtime_request_parity_admission_blocker_component_count(),
            5
        );
        assert!(missing.has_runtime_request_parity_admission_blockers());
        assert!(missing.runtime_request_parity_admission_accounting_is_consistent());
        assert!(!missing.runtime_request_parity_admission_is_clean());
        assert!(!missing.can_admit_runtime_diagnostics_request_parity());
        assert!(!missing.can_accept_runtime_diagnostics_request_parity());
    }

    #[test]
    fn inference_diagnostics_request_parity_summary_reports_missing_diagnostics_fields() {
        let request = InferenceRequest::new("prompt", TaskProfile::General)
            .with_prompt_tokens(8)
            .with_max_tokens(8)
            .with_runtime(RuntimeMetadata::new("seed-model", "tok", 128, 16));
        let execution = AdapterExecutionContext::new([RuntimeAdapter::CpuSimd]);
        let envelope = request_envelope(&request, RouteBudget::default(), &execution, 0);

        let parity = InferenceDiagnostics::default().request_parity_summary(&envelope);

        assert!(parity.generation_budget_missing());
        assert!(!parity.generation_budget_drifted());
        assert!(parity.missing_required_diagnostics_report());
        assert!(parity.runtime.missing_required_runtime_report());
        assert!(parity.runtime_drifted());
        assert!(parity.has_request_drift());
        assert_eq!(parity.routing_drift_component_count(), 0);
        assert_eq!(parity.generation_drift_component_count(), 1);
        assert_eq!(parity.hardware_drift_component_count(), 0);
        assert_eq!(parity.runtime.runtime_drift_component_count(), 4);
        assert_eq!(parity.diagnostics_request_drift_component_count(), 5);
        assert!(parity.runtime.has_runtime_drift_components());
        assert!(parity.runtime.runtime_drift_accounting_is_consistent());
        assert!(parity.has_diagnostics_request_drift_components());
        assert!(parity.diagnostics_request_accounting_is_consistent());
        assert!(!parity.request_parity_is_consistent());
        assert!(!parity.runtime.runtime_request_parity_shape_is_clean());
        assert!(
            !parity
                .runtime
                .can_accept_runtime_diagnostics_request_parity()
        );
        assert!(!parity.diagnostics_request_parity_shape_is_clean());
        assert!(!parity.can_accept_inference_diagnostics_request_parity());
    }

    fn request_envelope(
        request: &InferenceRequest,
        route_budget: RouteBudget,
        execution: &AdapterExecutionContext,
        imported_kv_blocks: usize,
    ) -> RuntimeRequestEnvelope {
        let architecture = TransformerRuntimeArchitecture::new(1, 16, 2, 1, 64);
        let transformer_plan = TransformerPlanDigest::new(
            Some("diagnostics-test"),
            vec![TransformerLayerBudget::new(
                0,
                TransformerAttentionKind::Global,
                0.5,
                64,
            )],
        );

        RuntimeRequestEnvelope::from_parts(
            request,
            architecture,
            route_budget,
            HierarchyWeights::for_profile(request.profile),
            &transformer_plan,
            execution,
            imported_kv_blocks,
        )
    }
}
