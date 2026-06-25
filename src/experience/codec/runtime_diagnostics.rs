use crate::reflection::RuntimeDiagnostics;

use super::fields::{
    field_to_finite_f32, non_empty_string, option_f32_to_field, sanitize_control_part,
};

pub(in crate::experience) fn serialize_runtime_diagnostics(
    diagnostics: &RuntimeDiagnostics,
) -> String {
    [
        diagnostics
            .model_id
            .as_deref()
            .map(sanitize_control_part)
            .unwrap_or_default(),
        diagnostics
            .selected_adapter
            .as_deref()
            .map(sanitize_control_part)
            .unwrap_or_default(),
        diagnostics
            .adapter_cache_mode
            .as_deref()
            .map(sanitize_control_part)
            .unwrap_or_default(),
        diagnostics
            .adapter_stream_trace_id
            .as_deref()
            .map(sanitize_control_part)
            .unwrap_or_default(),
        diagnostics
            .adapter_stream_gate_summary_digest
            .as_deref()
            .map(sanitize_control_part)
            .unwrap_or_default(),
        diagnostics
            .device_profile
            .as_deref()
            .map(sanitize_control_part)
            .unwrap_or_default(),
        diagnostics
            .primary_lane
            .as_deref()
            .map(sanitize_control_part)
            .unwrap_or_default(),
        diagnostics
            .fallback_lane
            .as_deref()
            .map(sanitize_control_part)
            .unwrap_or_default(),
        diagnostics
            .memory_mode
            .as_deref()
            .map(sanitize_control_part)
            .unwrap_or_default(),
        diagnostics.layer_count.to_string(),
        diagnostics.global_layers.to_string(),
        diagnostics.local_window_layers.to_string(),
        diagnostics.convolutional_fusion_layers.to_string(),
        diagnostics.hidden_size.to_string(),
        diagnostics.local_window_tokens.to_string(),
        option_f32_to_field(diagnostics.forward_energy),
        option_f32_to_field(diagnostics.kv_influence),
        diagnostics.imported_kv_blocks.to_string(),
        diagnostics.exported_kv_blocks.to_string(),
        option_u8_to_field(diagnostics.hot_kv_precision_bits),
        option_u8_to_field(diagnostics.cold_kv_precision_bits),
        diagnostics
            .device_execution_source
            .as_deref()
            .map(sanitize_control_part)
            .unwrap_or_default(),
        diagnostics.runtime_kv_segments_included.to_string(),
        diagnostics.runtime_kv_segments_skipped.to_string(),
        diagnostics.runtime_kv_segments_rejected.to_string(),
        diagnostics.weak_runtime_kv_imports_skipped.to_string(),
    ]
    .join("\u{1f}")
}

pub(in crate::experience) fn deserialize_runtime_diagnostics(
    value: &str,
) -> Option<RuntimeDiagnostics> {
    if value.is_empty() {
        return Some(RuntimeDiagnostics::default());
    }

    let fields = value.split('\u{1f}').collect::<Vec<_>>();
    match fields.len() {
        9 => Some(RuntimeDiagnostics {
            model_id: non_empty_string(fields[0]),
            selected_adapter: non_empty_string(fields[1]),
            adapter_cache_mode: None,
            adapter_stream_trace_id: None,
            adapter_stream_gate_summary_digest: None,
            device_profile: None,
            primary_lane: None,
            fallback_lane: None,
            memory_mode: None,
            device_execution_source: None,
            layer_count: fields[2].parse::<usize>().ok()?,
            global_layers: 0,
            local_window_layers: 0,
            convolutional_fusion_layers: 0,
            hidden_size: fields[3].parse::<usize>().ok()?,
            local_window_tokens: fields[4].parse::<usize>().ok()?,
            forward_energy: field_to_finite_f32(fields[5]),
            kv_influence: field_to_finite_f32(fields[6]),
            imported_kv_blocks: fields[7].parse::<usize>().ok()?,
            weak_runtime_kv_imports_skipped: 0,
            exported_kv_blocks: fields[8].parse::<usize>().ok()?,
            runtime_kv_segments_included: 0,
            runtime_kv_segments_skipped: 0,
            runtime_kv_segments_rejected: 0,
            hot_kv_precision_bits: None,
            cold_kv_precision_bits: None,
        }),
        12 => Some(RuntimeDiagnostics {
            model_id: non_empty_string(fields[0]),
            selected_adapter: non_empty_string(fields[1]),
            adapter_cache_mode: None,
            adapter_stream_trace_id: None,
            adapter_stream_gate_summary_digest: None,
            device_profile: None,
            primary_lane: None,
            fallback_lane: None,
            memory_mode: None,
            device_execution_source: None,
            layer_count: fields[2].parse::<usize>().ok()?,
            global_layers: fields[3].parse::<usize>().ok()?,
            local_window_layers: fields[4].parse::<usize>().ok()?,
            convolutional_fusion_layers: fields[5].parse::<usize>().ok()?,
            hidden_size: fields[6].parse::<usize>().ok()?,
            local_window_tokens: fields[7].parse::<usize>().ok()?,
            forward_energy: field_to_finite_f32(fields[8]),
            kv_influence: field_to_finite_f32(fields[9]),
            imported_kv_blocks: fields[10].parse::<usize>().ok()?,
            weak_runtime_kv_imports_skipped: 0,
            exported_kv_blocks: fields[11].parse::<usize>().ok()?,
            runtime_kv_segments_included: 0,
            runtime_kv_segments_skipped: 0,
            runtime_kv_segments_rejected: 0,
            hot_kv_precision_bits: None,
            cold_kv_precision_bits: None,
        }),
        16 | 18 | 19 | 22 | 23 => Some(RuntimeDiagnostics {
            model_id: non_empty_string(fields[0]),
            selected_adapter: non_empty_string(fields[1]),
            adapter_cache_mode: None,
            adapter_stream_trace_id: None,
            adapter_stream_gate_summary_digest: None,
            device_profile: non_empty_string(fields[2]),
            primary_lane: non_empty_string(fields[3]),
            fallback_lane: non_empty_string(fields[4]),
            memory_mode: non_empty_string(fields[5]),
            device_execution_source: fields
                .get(18)
                .and_then(RuntimeDiagnostics::normalize_device_execution_source),
            layer_count: fields[6].parse::<usize>().ok()?,
            global_layers: fields[7].parse::<usize>().ok()?,
            local_window_layers: fields[8].parse::<usize>().ok()?,
            convolutional_fusion_layers: fields[9].parse::<usize>().ok()?,
            hidden_size: fields[10].parse::<usize>().ok()?,
            local_window_tokens: fields[11].parse::<usize>().ok()?,
            forward_energy: field_to_finite_f32(fields[12]),
            kv_influence: field_to_finite_f32(fields[13]),
            imported_kv_blocks: fields[14].parse::<usize>().ok()?,
            exported_kv_blocks: fields[15].parse::<usize>().ok()?,
            runtime_kv_segments_included: optional_usize_field(&fields, 19)?,
            runtime_kv_segments_skipped: optional_usize_field(&fields, 20)?,
            runtime_kv_segments_rejected: optional_usize_field(&fields, 21)?,
            weak_runtime_kv_imports_skipped: optional_usize_field(&fields, 22)?,
            hot_kv_precision_bits: fields
                .get(16)
                .and_then(|value| field_to_kv_precision_bits(value)),
            cold_kv_precision_bits: fields
                .get(17)
                .and_then(|value| field_to_kv_precision_bits(value)),
        }),
        24 => Some(RuntimeDiagnostics {
            model_id: non_empty_string(fields[0]),
            selected_adapter: non_empty_string(fields[1]),
            adapter_cache_mode: fields
                .get(2)
                .and_then(RuntimeDiagnostics::normalize_adapter_cache_mode),
            adapter_stream_trace_id: None,
            adapter_stream_gate_summary_digest: None,
            device_profile: non_empty_string(fields[3]),
            primary_lane: non_empty_string(fields[4]),
            fallback_lane: non_empty_string(fields[5]),
            memory_mode: non_empty_string(fields[6]),
            device_execution_source: fields
                .get(19)
                .and_then(RuntimeDiagnostics::normalize_device_execution_source),
            layer_count: fields[7].parse::<usize>().ok()?,
            global_layers: fields[8].parse::<usize>().ok()?,
            local_window_layers: fields[9].parse::<usize>().ok()?,
            convolutional_fusion_layers: fields[10].parse::<usize>().ok()?,
            hidden_size: fields[11].parse::<usize>().ok()?,
            local_window_tokens: fields[12].parse::<usize>().ok()?,
            forward_energy: field_to_finite_f32(fields[13]),
            kv_influence: field_to_finite_f32(fields[14]),
            imported_kv_blocks: fields[15].parse::<usize>().ok()?,
            exported_kv_blocks: fields[16].parse::<usize>().ok()?,
            runtime_kv_segments_included: optional_usize_field(&fields, 20)?,
            runtime_kv_segments_skipped: optional_usize_field(&fields, 21)?,
            runtime_kv_segments_rejected: optional_usize_field(&fields, 22)?,
            weak_runtime_kv_imports_skipped: optional_usize_field(&fields, 23)?,
            hot_kv_precision_bits: fields
                .get(17)
                .and_then(|value| field_to_kv_precision_bits(value)),
            cold_kv_precision_bits: fields
                .get(18)
                .and_then(|value| field_to_kv_precision_bits(value)),
        }),
        26 => Some(RuntimeDiagnostics {
            model_id: non_empty_string(fields[0]),
            selected_adapter: non_empty_string(fields[1]),
            adapter_cache_mode: fields
                .get(2)
                .and_then(RuntimeDiagnostics::normalize_adapter_cache_mode),
            adapter_stream_trace_id: fields
                .get(3)
                .and_then(RuntimeDiagnostics::normalize_adapter_stream_trace_id),
            adapter_stream_gate_summary_digest: fields
                .get(4)
                .and_then(RuntimeDiagnostics::normalize_adapter_stream_gate_summary_digest),
            device_profile: non_empty_string(fields[5]),
            primary_lane: non_empty_string(fields[6]),
            fallback_lane: non_empty_string(fields[7]),
            memory_mode: non_empty_string(fields[8]),
            device_execution_source: fields
                .get(21)
                .and_then(RuntimeDiagnostics::normalize_device_execution_source),
            layer_count: fields[9].parse::<usize>().ok()?,
            global_layers: fields[10].parse::<usize>().ok()?,
            local_window_layers: fields[11].parse::<usize>().ok()?,
            convolutional_fusion_layers: fields[12].parse::<usize>().ok()?,
            hidden_size: fields[13].parse::<usize>().ok()?,
            local_window_tokens: fields[14].parse::<usize>().ok()?,
            forward_energy: field_to_finite_f32(fields[15]),
            kv_influence: field_to_finite_f32(fields[16]),
            imported_kv_blocks: fields[17].parse::<usize>().ok()?,
            exported_kv_blocks: fields[18].parse::<usize>().ok()?,
            runtime_kv_segments_included: optional_usize_field(&fields, 22)?,
            runtime_kv_segments_skipped: optional_usize_field(&fields, 23)?,
            runtime_kv_segments_rejected: optional_usize_field(&fields, 24)?,
            weak_runtime_kv_imports_skipped: optional_usize_field(&fields, 25)?,
            hot_kv_precision_bits: fields
                .get(19)
                .and_then(|value| field_to_kv_precision_bits(value)),
            cold_kv_precision_bits: fields
                .get(20)
                .and_then(|value| field_to_kv_precision_bits(value)),
        }),
        _ => None,
    }
}

fn optional_usize_field(fields: &[&str], index: usize) -> Option<usize> {
    fields
        .get(index)
        .map_or(Some(0), |value| value.parse::<usize>().ok())
}

fn option_u8_to_field(value: Option<u8>) -> String {
    value
        .filter(|value| matches!(value, 4 | 8))
        .map(|value| value.to_string())
        .unwrap_or_default()
}

fn field_to_kv_precision_bits(value: &str) -> Option<u8> {
    if value.is_empty() {
        return None;
    }
    value
        .parse::<u8>()
        .ok()
        .filter(|value| matches!(value, 4 | 8))
}
