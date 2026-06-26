use crate::engine::InferenceOutcome;
use crate::experience_replay::ExperienceReplayReport;
use crate::hardware::DeviceClass;
use crate::reflection::RuntimeDiagnostics;

use super::{BenchmarkCase, devices_csv, explicit_device_count, push_unique_device};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BenchmarkRuntimeDeviceExecutionEvidence {
    pub cases: usize,
    pub matched_cases: usize,
    pub runtime_kv_precision_cases: usize,
    pub runtime_kv_segment_cases: usize,
    pub runtime_kv_weak_import_skip_cases: usize,
    pub runtime_kv_budget_import_skip_cases: usize,
    pub runtime_kv_budget_pressure_cases: usize,
    pub runtime_kv_segments_included: usize,
    pub runtime_kv_segments_skipped: usize,
    pub runtime_kv_segments_rejected: usize,
    pub weak_runtime_kv_imports_skipped: usize,
    pub budget_limited_runtime_kv_imports_skipped: usize,
    pub runtime_adapter_cache_mode_cases: usize,
    pub runtime_adapter_stream_trace_cases: usize,
    pub runtime_adapter_stream_gate_summary_cases: usize,
    pub failures: Vec<String>,
    pub(super) adapter_cache_modes: Vec<String>,
    pub(super) matched_devices: Vec<DeviceClass>,
    pub(super) kv_precision_devices: Vec<DeviceClass>,
    pub(super) kv_weak_import_skip_devices: Vec<DeviceClass>,
    pub(super) kv_budget_import_skip_devices: Vec<DeviceClass>,
    pub(super) kv_budget_pressure_devices: Vec<DeviceClass>,
    pub(super) kv_segment_devices: Vec<DeviceClass>,
}

impl BenchmarkRuntimeDeviceExecutionEvidence {
    pub(super) fn record(&mut self, case: &BenchmarkCase, outcome: &InferenceOutcome) {
        let diagnostics = &outcome.runtime_diagnostics;
        self.record_runtime_adapter_cache_mode_evidence(diagnostics);
        self.record_runtime_adapter_stream_evidence(diagnostics);
        self.record_runtime_kv_segment_evidence(diagnostics, outcome.hardware_plan.device);
        self.record_weak_runtime_kv_import_skip_evidence(diagnostics, outcome.hardware_plan.device);
        self.record_budget_limited_runtime_kv_import_skip_evidence(
            diagnostics,
            outcome.hardware_plan.device,
        );
        self.record_runtime_kv_budget_pressure_evidence(diagnostics, outcome.hardware_plan.device);
        let has_forward_signal = diagnostics.has_forward_signal();
        let has_device_execution_signal = diagnostics.has_device_execution_signal();
        let has_runtime_reported_device_execution_signal =
            diagnostics.has_runtime_reported_device_execution_signal();
        if runtime_static_architecture_only(diagnostics) {
            return;
        }

        if !has_forward_signal && !has_device_execution_signal {
            return;
        }

        let device = outcome.hardware_plan.device;
        if !has_device_execution_signal {
            self.failures.push(format!(
                "{}:{} runtime forward signal is missing device execution diagnostics",
                device.as_str(),
                case.name
            ));
            return;
        }
        if !has_runtime_reported_device_execution_signal {
            self.failures.push(format!(
                "{}:{} runtime device execution diagnostics source={} is not runtime-reported",
                device.as_str(),
                case.name,
                diagnostics
                    .device_execution_source
                    .as_deref()
                    .unwrap_or("unknown")
            ));
            return;
        }

        self.cases += 1;
        let execution = &outcome.hardware_plan.execution;
        let mut mismatches = Vec::new();
        record_runtime_device_execution_mismatch(
            &mut mismatches,
            "device_profile",
            diagnostics.device_profile.as_deref(),
            device.as_str(),
        );
        record_runtime_device_execution_mismatch(
            &mut mismatches,
            "primary_lane",
            diagnostics.primary_lane.as_deref(),
            execution.primary_lane.as_str(),
        );
        record_runtime_device_execution_mismatch(
            &mut mismatches,
            "fallback_lane",
            diagnostics.fallback_lane.as_deref(),
            execution.fallback_lane.as_str(),
        );
        record_runtime_device_execution_mismatch(
            &mut mismatches,
            "memory_mode",
            diagnostics.memory_mode.as_deref(),
            execution.memory_mode.as_str(),
        );
        record_runtime_device_execution_usize_mismatch(
            &mut mismatches,
            "hot_kv_precision_bits",
            diagnostics.hot_kv_precision_bits.map(usize::from),
            usize::from(execution.hot_kv_precision_bits),
        );
        record_runtime_device_execution_usize_mismatch(
            &mut mismatches,
            "cold_kv_precision_bits",
            diagnostics.cold_kv_precision_bits.map(usize::from),
            usize::from(execution.cold_kv_precision_bits),
        );

        if diagnostics.has_valid_kv_precision_signal() {
            self.runtime_kv_precision_cases += 1;
            push_unique_device(&mut self.kv_precision_devices, device);
        } else {
            self.failures.push(format!(
                "{}:{} runtime device execution is missing valid KV precision diagnostics",
                device.as_str(),
                case.name
            ));
        }

        if mismatches.is_empty() {
            self.matched_cases += 1;
            push_unique_device(&mut self.matched_devices, device);
        } else {
            self.failures.push(format!(
                "{}:{} runtime device execution mismatch: {}",
                device.as_str(),
                case.name,
                mismatches.join(", ")
            ));
        }
    }

    fn record_runtime_adapter_cache_mode_evidence(&mut self, diagnostics: &RuntimeDiagnostics) {
        let Some(cache_mode) = diagnostics.adapter_cache_mode.as_deref() else {
            return;
        };

        self.runtime_adapter_cache_mode_cases += 1;
        if !self
            .adapter_cache_modes
            .iter()
            .any(|existing| existing == cache_mode)
        {
            self.adapter_cache_modes.push(cache_mode.to_owned());
        }
    }

    pub fn runtime_adapter_cache_modes(&self) -> usize {
        self.adapter_cache_modes.len()
    }

    pub fn runtime_adapter_cache_modes_csv(&self) -> String {
        if self.adapter_cache_modes.is_empty() {
            "none".to_owned()
        } else {
            self.adapter_cache_modes.join("+")
        }
    }

    fn record_runtime_adapter_stream_evidence(&mut self, diagnostics: &RuntimeDiagnostics) {
        if diagnostics.has_adapter_stream_trace_signal() {
            self.runtime_adapter_stream_trace_cases += 1;
        }
        if diagnostics.has_adapter_stream_gate_summary_signal() {
            self.runtime_adapter_stream_gate_summary_cases += 1;
        }
    }

    fn record_runtime_kv_segment_evidence(
        &mut self,
        diagnostics: &RuntimeDiagnostics,
        device: DeviceClass,
    ) {
        if !diagnostics.has_runtime_kv_segment_signal() {
            return;
        }

        self.runtime_kv_segment_cases += 1;
        self.runtime_kv_segments_included += diagnostics.runtime_kv_segments_included;
        self.runtime_kv_segments_skipped += diagnostics.runtime_kv_segments_skipped;
        self.runtime_kv_segments_rejected += diagnostics.runtime_kv_segments_rejected;
        push_unique_device(&mut self.kv_segment_devices, device);
    }

    fn record_weak_runtime_kv_import_skip_evidence(
        &mut self,
        diagnostics: &RuntimeDiagnostics,
        device: DeviceClass,
    ) {
        if diagnostics.weak_runtime_kv_imports_skipped == 0 {
            return;
        }

        self.runtime_kv_weak_import_skip_cases += 1;
        self.weak_runtime_kv_imports_skipped += diagnostics.weak_runtime_kv_imports_skipped;
        push_unique_device(&mut self.kv_weak_import_skip_devices, device);
    }

    fn record_budget_limited_runtime_kv_import_skip_evidence(
        &mut self,
        diagnostics: &RuntimeDiagnostics,
        device: DeviceClass,
    ) {
        if diagnostics.budget_limited_runtime_kv_imports_skipped == 0 {
            return;
        }

        self.runtime_kv_budget_import_skip_cases += 1;
        self.budget_limited_runtime_kv_imports_skipped +=
            diagnostics.budget_limited_runtime_kv_imports_skipped;
        push_unique_device(&mut self.kv_budget_import_skip_devices, device);
    }

    fn record_runtime_kv_budget_pressure_evidence(
        &mut self,
        diagnostics: &RuntimeDiagnostics,
        device: DeviceClass,
    ) {
        if diagnostics.budget_limited_runtime_kv_imports_skipped == 0 {
            return;
        }

        self.runtime_kv_budget_pressure_cases += 1;
        push_unique_device(&mut self.kv_budget_pressure_devices, device);
    }

    pub fn device_profiles(&self) -> usize {
        explicit_device_count(&self.matched_devices)
    }

    pub fn matched_devices_csv(&self) -> String {
        if self.matched_devices.is_empty() {
            "none".to_owned()
        } else {
            self.matched_devices
                .iter()
                .map(|device| device.as_str())
                .collect::<Vec<_>>()
                .join("+")
        }
    }

    pub fn runtime_kv_precision_device_profiles(&self) -> usize {
        explicit_device_count(&self.kv_precision_devices)
    }

    pub fn runtime_kv_precision_devices_csv(&self) -> String {
        devices_csv(self.kv_precision_devices.clone())
    }

    pub fn runtime_kv_weak_import_skip_device_profiles(&self) -> usize {
        explicit_device_count(&self.kv_weak_import_skip_devices)
    }

    pub fn runtime_kv_weak_import_skip_devices_csv(&self) -> String {
        devices_csv(self.kv_weak_import_skip_devices.clone())
    }

    pub fn runtime_kv_budget_import_skip_device_profiles(&self) -> usize {
        explicit_device_count(&self.kv_budget_import_skip_devices)
    }

    pub fn runtime_kv_budget_import_skip_devices_csv(&self) -> String {
        devices_csv(self.kv_budget_import_skip_devices.clone())
    }

    pub fn runtime_kv_budget_pressure_device_profiles(&self) -> usize {
        explicit_device_count(&self.kv_budget_pressure_devices)
    }

    pub fn runtime_kv_budget_pressure_devices_csv(&self) -> String {
        devices_csv(self.kv_budget_pressure_devices.clone())
    }

    pub fn runtime_kv_segment_device_profiles(&self) -> usize {
        explicit_device_count(&self.kv_segment_devices)
    }

    pub fn runtime_kv_segment_devices_csv(&self) -> String {
        devices_csv(self.kv_segment_devices.clone())
    }
}

fn record_runtime_device_execution_mismatch(
    mismatches: &mut Vec<String>,
    field: &str,
    actual: Option<&str>,
    expected: &str,
) {
    match actual {
        Some(actual) if actual == expected => {}
        Some(actual) => mismatches.push(format!("{field} actual={actual} expected={expected}")),
        None => mismatches.push(format!("{field} missing expected={expected}")),
    }
}

fn record_runtime_device_execution_usize_mismatch(
    mismatches: &mut Vec<String>,
    field: &str,
    actual: Option<usize>,
    expected: usize,
) {
    match actual {
        Some(actual) if actual == expected => {}
        Some(actual) => mismatches.push(format!("{field} actual={actual} expected={expected}")),
        None => mismatches.push(format!("{field} missing expected={expected}")),
    }
}

pub(super) fn runtime_static_architecture_only(diagnostics: &RuntimeDiagnostics) -> bool {
    diagnostics.has_runtime_architecture_signal()
        && diagnostics.device_execution_source.as_deref()
            == Some(RuntimeDiagnostics::control_plane_filled_device_execution_source())
        && !diagnostics.has_layer_mode_signal()
        && diagnostics.forward_energy.is_none()
        && diagnostics.kv_influence.is_none()
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BenchmarkRuntimeArchitectureEvidence {
    pub cases: usize,
    pub(super) auto_replay_runtime_kv_budget_pressure_items: usize,
    pub(super) auto_replay_runtime_kv_budget_pressure_weighted_milli_total: usize,
    pub(super) auto_replay_runtime_kv_budget_pressure_weight: usize,
    pub(super) auto_replay_max_runtime_kv_budget_pressure_milli: usize,
    pub(super) devices: Vec<DeviceClass>,
}

impl BenchmarkRuntimeArchitectureEvidence {
    pub(super) fn record(&mut self, outcome: &InferenceOutcome) {
        let diagnostics = &outcome.runtime_diagnostics;
        if diagnostics.has_runtime_architecture_signal()
            && diagnostics.has_valid_kv_precision_signal()
        {
            self.cases += 1;
            push_unique_device(&mut self.devices, outcome.hardware_plan.device);
        }
    }

    pub(super) fn record_auto_replay_runtime_kv_budget_pressure(
        &mut self,
        report: &ExperienceReplayReport,
    ) {
        let pressure_weight = report.planned.max(report.applied);
        self.auto_replay_runtime_kv_budget_pressure_items +=
            report.runtime_kv_budget_pressure_items;
        if pressure_weight > 0 {
            self.auto_replay_runtime_kv_budget_pressure_weighted_milli_total +=
                pressure_milli(report.average_runtime_kv_budget_pressure) * pressure_weight;
            self.auto_replay_runtime_kv_budget_pressure_weight += pressure_weight;
        }
        self.auto_replay_max_runtime_kv_budget_pressure_milli = self
            .auto_replay_max_runtime_kv_budget_pressure_milli
            .max(pressure_milli(report.max_runtime_kv_budget_pressure));
    }

    pub fn auto_replay_runtime_kv_budget_pressure_items(&self) -> usize {
        self.auto_replay_runtime_kv_budget_pressure_items
    }

    pub fn average_auto_replay_runtime_kv_budget_pressure(&self) -> f32 {
        if self.auto_replay_runtime_kv_budget_pressure_weight == 0 {
            0.0
        } else {
            self.auto_replay_runtime_kv_budget_pressure_weighted_milli_total as f32
                / self.auto_replay_runtime_kv_budget_pressure_weight as f32
                / 1000.0
        }
    }

    pub fn max_auto_replay_runtime_kv_budget_pressure(&self) -> f32 {
        self.auto_replay_max_runtime_kv_budget_pressure_milli as f32 / 1000.0
    }

    pub fn device_profiles(&self) -> usize {
        explicit_device_count(&self.devices)
    }

    pub fn devices_csv(&self) -> String {
        if self.devices.is_empty() {
            "none".to_owned()
        } else {
            self.devices
                .iter()
                .map(|device| device.as_str())
                .collect::<Vec<_>>()
                .join("+")
        }
    }
}

fn pressure_milli(pressure: f32) -> usize {
    if pressure.is_finite() {
        (pressure.clamp(0.0, 1.0) * 1000.0).round() as usize
    } else {
        0
    }
}
