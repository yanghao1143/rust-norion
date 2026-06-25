use crate::drift::DriftSeverity;
use crate::hardware::DeviceClass;

use super::super::{devices_csv, explicit_device_count, push_unique_device};
use super::{BenchmarkCaseResult, BenchmarkSummary};

impl BenchmarkSummary {
    pub fn total_runtime_kv_stored(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.runtime_kv_stored)
            .sum()
    }

    pub fn runtime_kv_stored_device_profiles(&self) -> usize {
        explicit_device_count(&self.runtime_kv_stored_devices())
    }

    pub fn runtime_kv_stored_devices_csv(&self) -> String {
        devices_csv(self.runtime_kv_stored_devices())
    }

    fn runtime_kv_stored_devices(&self) -> Vec<DeviceClass> {
        let mut devices = Vec::new();

        for result in &self.results {
            if result.runtime_kv_stored > 0 {
                push_unique_device(&mut devices, result.device);
            }
        }

        devices
    }

    pub fn runtime_kv_hold_cases(&self) -> usize {
        self.results
            .iter()
            .filter(|result| runtime_kv_was_held(result))
            .count()
    }

    pub fn total_runtime_kv_held(&self) -> usize {
        self.results
            .iter()
            .filter(|result| runtime_kv_was_held(result))
            .map(|result| {
                result
                    .runtime_kv_exported
                    .saturating_sub(result.runtime_kv_stored)
            })
            .sum()
    }

    pub fn runtime_kv_hold_device_profiles(&self) -> usize {
        explicit_device_count(&self.runtime_kv_hold_devices())
    }

    pub fn runtime_kv_hold_devices_csv(&self) -> String {
        let devices = self
            .runtime_kv_hold_devices()
            .into_iter()
            .map(DeviceClass::as_str)
            .collect::<Vec<_>>();

        if devices.is_empty() {
            "none".to_owned()
        } else {
            devices.join("+")
        }
    }

    fn runtime_kv_hold_devices(&self) -> Vec<DeviceClass> {
        let mut devices = Vec::new();

        for result in &self.results {
            if runtime_kv_was_held(result)
                && result.device != DeviceClass::Auto
                && !devices.contains(&result.device)
            {
                devices.push(result.device);
            }
        }

        devices
    }

    pub fn runtime_token_cases(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.runtime_token_count > 0)
            .count()
    }

    pub fn total_runtime_tokens(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.runtime_token_count)
            .sum()
    }

    pub fn runtime_uncertainty_cases(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.runtime_uncertainty_signal)
            .count()
    }

    pub fn total_runtime_uncertainty_tokens(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.runtime_uncertainty_token_count)
            .sum()
    }

    pub fn runtime_uncertainty_device_profiles(&self) -> usize {
        explicit_device_count(&self.runtime_uncertainty_devices())
    }

    pub fn runtime_uncertainty_devices_csv(&self) -> String {
        devices_csv(self.runtime_uncertainty_devices())
    }

    pub fn runtime_uncertainty_token_device_profiles(&self) -> usize {
        explicit_device_count(&self.runtime_uncertainty_token_devices())
    }

    pub fn runtime_uncertainty_token_devices_csv(&self) -> String {
        devices_csv(self.runtime_uncertainty_token_devices())
    }

    fn runtime_uncertainty_devices(&self) -> Vec<DeviceClass> {
        let mut devices = Vec::new();

        for result in &self.results {
            if result.runtime_uncertainty_signal {
                push_unique_device(&mut devices, result.device);
            }
        }

        devices
    }

    fn runtime_uncertainty_token_devices(&self) -> Vec<DeviceClass> {
        let mut devices = Vec::new();

        for result in &self.results {
            if result.runtime_uncertainty_token_count > 0 {
                push_unique_device(&mut devices, result.device);
            }
        }

        devices
    }

    pub fn runtime_kv_import_cases(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.runtime_kv_imported > 0)
            .count()
    }

    pub fn runtime_kv_weak_import_skip_cases(&self) -> usize {
        self.runtime_device_execution_evidence
            .runtime_kv_weak_import_skip_cases
    }

    pub fn total_weak_runtime_kv_imports_skipped(&self) -> usize {
        self.runtime_device_execution_evidence
            .weak_runtime_kv_imports_skipped
    }

    pub fn runtime_kv_weak_import_skip_device_profiles(&self) -> usize {
        self.runtime_device_execution_evidence
            .runtime_kv_weak_import_skip_device_profiles()
    }

    pub fn runtime_kv_weak_import_skip_devices_csv(&self) -> String {
        self.runtime_device_execution_evidence
            .runtime_kv_weak_import_skip_devices_csv()
    }

    pub fn runtime_kv_segment_cases(&self) -> usize {
        self.runtime_device_execution_evidence
            .runtime_kv_segment_cases
    }

    pub fn total_runtime_kv_segments_included(&self) -> usize {
        self.runtime_device_execution_evidence
            .runtime_kv_segments_included
    }

    pub fn total_runtime_kv_segments_skipped(&self) -> usize {
        self.runtime_device_execution_evidence
            .runtime_kv_segments_skipped
    }

    pub fn total_runtime_kv_segments_rejected(&self) -> usize {
        self.runtime_device_execution_evidence
            .runtime_kv_segments_rejected
    }

    pub fn runtime_kv_segment_device_profiles(&self) -> usize {
        self.runtime_device_execution_evidence
            .runtime_kv_segment_device_profiles()
    }

    pub fn runtime_kv_segment_devices_csv(&self) -> String {
        self.runtime_device_execution_evidence
            .runtime_kv_segment_devices_csv()
    }

    pub fn runtime_kv_import_device_profiles(&self) -> usize {
        explicit_device_count(&self.runtime_kv_import_devices())
    }

    pub fn runtime_kv_import_devices_csv(&self) -> String {
        devices_csv(self.runtime_kv_import_devices())
    }

    fn runtime_kv_import_devices(&self) -> Vec<DeviceClass> {
        let mut devices = Vec::new();

        for result in &self.results {
            if result.runtime_kv_imported > 0 {
                push_unique_device(&mut devices, result.device);
            }
        }

        devices
    }

    pub fn total_runtime_kv_imported(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.runtime_kv_imported)
            .sum()
    }

    pub fn runtime_kv_export_device_profiles(&self) -> usize {
        explicit_device_count(&self.runtime_kv_export_devices())
    }

    pub fn runtime_kv_export_devices_csv(&self) -> String {
        devices_csv(self.runtime_kv_export_devices())
    }

    fn runtime_kv_export_devices(&self) -> Vec<DeviceClass> {
        let mut devices = Vec::new();

        for result in &self.results {
            if result.runtime_kv_exported > 0 {
                push_unique_device(&mut devices, result.device);
            }
        }

        devices
    }

    pub fn total_runtime_kv_exported(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.runtime_kv_exported)
            .sum()
    }

    pub fn runtime_adapter_contract_cases(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.runtime_forward_signal && result.runtime_adapter_contract_ok)
            .count()
    }

    pub fn runtime_adapter_kinds(&self) -> usize {
        let mut adapters = Vec::new();

        for result in &self.results {
            if result.runtime_forward_signal
                && result.runtime_adapter_contract_ok
                && let Some(adapter) = result.runtime_selected_adapter.as_deref()
                && !adapters.contains(&adapter)
            {
                adapters.push(adapter);
            }
        }

        adapters.len()
    }

    pub fn runtime_adapter_cache_mode_cases(&self) -> usize {
        self.runtime_device_execution_evidence
            .runtime_adapter_cache_mode_cases
    }

    pub fn runtime_adapter_cache_modes(&self) -> usize {
        self.runtime_device_execution_evidence
            .runtime_adapter_cache_modes()
    }

    pub fn runtime_adapter_cache_modes_csv(&self) -> String {
        self.runtime_device_execution_evidence
            .runtime_adapter_cache_modes_csv()
    }

    pub fn runtime_adapter_stream_trace_cases(&self) -> usize {
        self.runtime_device_execution_evidence
            .runtime_adapter_stream_trace_cases
    }

    pub fn runtime_adapter_stream_gate_summary_cases(&self) -> usize {
        self.runtime_device_execution_evidence
            .runtime_adapter_stream_gate_summary_cases
    }

    pub fn total_runtime_adapter_contract_violations(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.runtime_adapter_contract_violations)
            .sum()
    }

    pub fn total_runtime_adapter_selection_mismatches(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.runtime_adapter_selection_mismatches)
            .sum()
    }

    pub fn runtime_forward_cases(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.runtime_forward_signal)
            .count()
    }

    pub fn runtime_forward_energy_cases(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.runtime_forward_energy_signal)
            .count()
    }

    pub fn runtime_kv_influence_cases(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.runtime_kv_influence_signal)
            .count()
    }

    pub fn runtime_layer_mode_cases(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.runtime_layer_mode_signal)
            .count()
    }

    pub fn runtime_all_layer_mode_cases(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.runtime_all_layer_modes_signal)
            .count()
    }

    pub fn total_runtime_global_layers(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.runtime_global_layers)
            .sum()
    }

    pub fn total_runtime_local_window_layers(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.runtime_local_window_layers)
            .sum()
    }

    pub fn total_runtime_convolutional_fusion_layers(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.runtime_convolutional_fusion_layers)
            .sum()
    }

    pub fn total_runtime_adapter_observations(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.runtime_adapter_observations)
            .sum()
    }

    pub fn max_runtime_adapter_score(&self) -> Option<f32> {
        self.results
            .iter()
            .filter_map(|result| result.runtime_adapter_best_score)
            .reduce(f32::max)
    }
}

fn runtime_kv_was_held(result: &BenchmarkCaseResult) -> bool {
    result.drift_severity == DriftSeverity::Watch
        && result.runtime_kv_exported > result.runtime_kv_stored
}
