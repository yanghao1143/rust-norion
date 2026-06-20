use crate::hardware::HardwarePlan;
use crate::reflection::RuntimeDiagnostics;
use crate::runtime_manifest::TransformerRuntimeArchitecture;

use super::device::{
    parse_runtime_adapter_hint, parse_runtime_compute_lane, parse_runtime_device_class,
    parse_runtime_memory_mode,
};
use super::types::RuntimeMetadata;

pub(super) fn validate_runtime_response_contract(
    diagnostics: &RuntimeDiagnostics,
    metadata: &RuntimeMetadata,
    architecture: TransformerRuntimeArchitecture,
    hardware_plan: &HardwarePlan,
) -> Vec<String> {
    let mut violations = Vec::new();

    if let Some(adapter) = diagnostics
        .selected_adapter
        .as_deref()
        .and_then(parse_runtime_adapter_hint)
    {
        if !hardware_plan.execution.adapter_hints.contains(&adapter) {
            violations.push(format!(
                "runtime selected adapter {} outside device execution adapter hints {}",
                adapter.as_str(),
                hardware_plan
                    .execution
                    .adapter_hints
                    .iter()
                    .map(|hint| hint.as_str())
                    .collect::<Vec<_>>()
                    .join("+")
            ));
        }
    } else if diagnostics.selected_adapter.is_some() {
        violations.push(format!(
            "runtime selected unknown adapter {}",
            diagnostics.selected_adapter.as_deref().unwrap_or_default()
        ));
    }

    if let Some(device) = diagnostics
        .device_profile
        .as_deref()
        .and_then(parse_runtime_device_class)
    {
        if device != hardware_plan.device {
            violations.push(format!(
                "runtime diagnostics device_profile {} differs from request device {}",
                device.as_str(),
                hardware_plan.device.as_str()
            ));
        }
    } else if diagnostics.device_profile.is_some() {
        violations.push(format!(
            "runtime diagnostics unknown device_profile {}",
            diagnostics.device_profile.as_deref().unwrap_or_default()
        ));
    }

    if let Some(primary_lane) = diagnostics
        .primary_lane
        .as_deref()
        .and_then(parse_runtime_compute_lane)
    {
        if primary_lane != hardware_plan.execution.primary_lane {
            violations.push(format!(
                "runtime diagnostics primary_lane {} differs from request primary {}",
                primary_lane.as_str(),
                hardware_plan.execution.primary_lane.as_str()
            ));
        }
    } else if diagnostics.primary_lane.is_some() {
        violations.push(format!(
            "runtime diagnostics unknown primary_lane {}",
            diagnostics.primary_lane.as_deref().unwrap_or_default()
        ));
    }

    if let Some(fallback_lane) = diagnostics
        .fallback_lane
        .as_deref()
        .and_then(parse_runtime_compute_lane)
    {
        if fallback_lane != hardware_plan.execution.fallback_lane {
            violations.push(format!(
                "runtime diagnostics fallback_lane {} differs from request fallback {}",
                fallback_lane.as_str(),
                hardware_plan.execution.fallback_lane.as_str()
            ));
        }
    } else if diagnostics.fallback_lane.is_some() {
        violations.push(format!(
            "runtime diagnostics unknown fallback_lane {}",
            diagnostics.fallback_lane.as_deref().unwrap_or_default()
        ));
    }

    if let Some(memory_mode) = diagnostics
        .memory_mode
        .as_deref()
        .and_then(parse_runtime_memory_mode)
    {
        if memory_mode != hardware_plan.execution.memory_mode {
            violations.push(format!(
                "runtime diagnostics memory_mode {} differs from request memory {}",
                memory_mode.as_str(),
                hardware_plan.execution.memory_mode.as_str()
            ));
        }
    } else if diagnostics.memory_mode.is_some() {
        violations.push(format!(
            "runtime diagnostics unknown memory_mode {}",
            diagnostics.memory_mode.as_deref().unwrap_or_default()
        ));
    }

    if let Some(model_id) = diagnostics.model_id.as_deref()
        && !metadata.model_id.is_empty()
        && model_id != metadata.model_id
    {
        violations.push(format!(
            "runtime diagnostics model_id {model_id} differs from request model_id {}",
            metadata.model_id
        ));
    }
    if diagnostics.layer_count > architecture.layer_count {
        violations.push(format!(
            "runtime diagnostics layer_count {} exceeds request layer_count {}",
            diagnostics.layer_count, architecture.layer_count
        ));
    }
    if diagnostics.hidden_size > 0 && diagnostics.hidden_size != architecture.hidden_size {
        violations.push(format!(
            "runtime diagnostics hidden_size {} differs from request hidden_size {}",
            diagnostics.hidden_size, architecture.hidden_size
        ));
    }
    if diagnostics.local_window_tokens > 0
        && diagnostics.local_window_tokens != architecture.local_window_tokens
    {
        violations.push(format!(
            "runtime diagnostics local_window_tokens {} differs from request local_window_tokens {}",
            diagnostics.local_window_tokens, architecture.local_window_tokens
        ));
    }
    if let Some(hot_bits) = diagnostics.hot_kv_precision_bits
        && hot_bits > metadata.hot_kv_precision_bits
    {
        violations.push(format!(
                "runtime diagnostics hot KV precision {hot_bits} exceeds request runtime hot KV precision {}",
                metadata.hot_kv_precision_bits
            ));
    }
    if let Some(cold_bits) = diagnostics.cold_kv_precision_bits
        && cold_bits > metadata.cold_kv_precision_bits
    {
        violations.push(format!(
                "runtime diagnostics cold KV precision {cold_bits} exceeds request runtime cold KV precision {}",
                metadata.cold_kv_precision_bits
            ));
    }
    if let (Some(hot_bits), Some(cold_bits)) = (
        diagnostics.hot_kv_precision_bits,
        diagnostics.cold_kv_precision_bits,
    ) && cold_bits > hot_bits
    {
        violations.push(format!(
            "runtime diagnostics cold KV precision {cold_bits} exceeds hot KV precision {hot_bits}"
        ));
    }

    violations
}

pub(super) fn populate_runtime_device_execution(
    diagnostics: &mut RuntimeDiagnostics,
    hardware_plan: &HardwarePlan,
) {
    let runtime_reported_complete = diagnostics.has_device_execution_signal()
        && diagnostics
            .device_execution_source
            .as_deref()
            .map(|source| {
                source != RuntimeDiagnostics::control_plane_filled_device_execution_source()
            })
            .unwrap_or(true);

    if diagnostics.device_profile.is_none() {
        diagnostics.device_profile = Some(hardware_plan.device.as_str().to_owned());
    }
    if diagnostics.primary_lane.is_none() {
        diagnostics.primary_lane = Some(hardware_plan.execution.primary_lane.as_str().to_owned());
    }
    if diagnostics.fallback_lane.is_none() {
        diagnostics.fallback_lane = Some(hardware_plan.execution.fallback_lane.as_str().to_owned());
    }
    if diagnostics.memory_mode.is_none() {
        diagnostics.memory_mode = Some(hardware_plan.execution.memory_mode.as_str().to_owned());
    }

    if diagnostics.has_device_execution_signal() {
        diagnostics.device_execution_source = Some(
            if runtime_reported_complete {
                RuntimeDiagnostics::runtime_reported_device_execution_source()
            } else {
                RuntimeDiagnostics::control_plane_filled_device_execution_source()
            }
            .to_owned(),
        );
    }
}

pub(super) fn populate_runtime_kv_precision(
    diagnostics: &mut RuntimeDiagnostics,
    metadata: &RuntimeMetadata,
) {
    if diagnostics.hot_kv_precision_bits.is_none() {
        diagnostics.hot_kv_precision_bits = Some(metadata.hot_kv_precision_bits);
    }
    if diagnostics.cold_kv_precision_bits.is_none() {
        diagnostics.cold_kv_precision_bits = Some(
            metadata
                .cold_kv_precision_bits
                .min(metadata.hot_kv_precision_bits),
        );
    }
}
