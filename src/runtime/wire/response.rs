use crate::kv_exchange::RuntimeKvBlock;
use crate::reflection::{ReasoningStep, RuntimeDiagnostics};

use super::super::{RuntimeError, RuntimeResponse, RuntimeToken};
use super::json::{
    extract_json_array_field, extract_json_array_field_by_value_kind, extract_json_f32_array_field,
    extract_json_finite_number_field, extract_json_kv_precision_bits, extract_json_number_field,
    extract_json_object_field, extract_json_string_field, extract_json_usize_field,
    split_json_objects,
};

pub fn parse_runtime_response_json(payload: &str) -> Result<RuntimeResponse, RuntimeError> {
    let schema = extract_json_string_field(payload, "schema")
        .ok_or_else(|| RuntimeError::new("runtime response JSON must include a schema string"))?;
    if schema != "rust-norion-runtime-response-v1" {
        return Err(RuntimeError::new(
            "runtime response schema must be rust-norion-runtime-response-v1",
        ));
    }
    let answer = extract_json_string_field(payload, "answer")
        .ok_or_else(|| RuntimeError::new("runtime response JSON must include an answer string"))?;
    if answer.trim().is_empty() {
        return Err(RuntimeError::new(
            "runtime response JSON must include a non-empty answer",
        ));
    }

    let mut response = RuntimeResponse::new(answer);
    response.tokens = extract_json_array_field(payload, "tokens")
        .map(split_json_objects)
        .unwrap_or_default()
        .iter()
        .map(|token| RuntimeToken {
            text: extract_json_string_field(token, "text").unwrap_or_default(),
            logprob: extract_json_number_field(token, "logprob"),
            entropy: extract_json_number_field(token, "entropy"),
        })
        .filter(|token| !token.text.is_empty())
        .collect();
    response.trace = extract_json_array_field(payload, "trace")
        .map(split_json_objects)
        .unwrap_or_default()
        .iter()
        .map(|step| {
            ReasoningStep::new(
                extract_json_string_field(step, "label").unwrap_or_else(|| "runtime".to_owned()),
                extract_json_string_field(step, "content").unwrap_or_default(),
                extract_json_number_field(step, "confidence").unwrap_or(0.5),
            )
        })
        .collect();
    response.diagnostics = extract_json_object_field(payload, "diagnostics")
        .map(parse_runtime_diagnostics)
        .unwrap_or_default();
    response.exported_kv_blocks = parse_runtime_kv_blocks(payload, "exported_kv_blocks")?;
    Ok(response)
}

fn parse_runtime_kv_blocks(
    payload: &str,
    field: &str,
) -> Result<Vec<RuntimeKvBlock>, RuntimeError> {
    let Some(blocks) = extract_json_array_field_by_value_kind(payload, field) else {
        return Ok(Vec::new());
    };

    split_json_objects(blocks)
        .into_iter()
        .enumerate()
        .map(|(index, block)| parse_runtime_kv_block(block, index, field))
        .collect()
}

fn parse_runtime_kv_block(
    payload: &str,
    index: usize,
    field: &str,
) -> Result<RuntimeKvBlock, RuntimeError> {
    let required_usize = |name: &str| {
        extract_json_usize_field(payload, name).ok_or_else(|| {
            RuntimeError::new(format!(
                "runtime response {field}[{index}] must include usize field {name}"
            ))
        })
    };
    let layer = required_usize("layer")?;
    let head = required_usize("head")?;
    let token_start = required_usize("token_start")?;
    let token_end = required_usize("token_end")?;
    let key = extract_json_f32_array_field(payload, "key").ok_or_else(|| {
        RuntimeError::new(format!(
            "runtime response {field}[{index}] must include finite f32 array key"
        ))
    })?;
    let value = extract_json_f32_array_field(payload, "value").ok_or_else(|| {
        RuntimeError::new(format!(
            "runtime response {field}[{index}] must include finite f32 array value"
        ))
    })?;

    Ok(RuntimeKvBlock::new(
        layer,
        head,
        token_start,
        token_end,
        key,
        value,
    ))
}

fn parse_runtime_diagnostics(payload: &str) -> RuntimeDiagnostics {
    RuntimeDiagnostics {
        model_id: extract_json_string_field(payload, "model_id"),
        selected_adapter: extract_json_string_field(payload, "selected_adapter"),
        adapter_cache_mode: extract_json_string_field(payload, "adapter_cache_mode")
            .and_then(RuntimeDiagnostics::normalize_adapter_cache_mode),
        adapter_stream_trace_id: extract_json_string_field(payload, "adapter_stream_trace_id")
            .and_then(RuntimeDiagnostics::normalize_adapter_stream_trace_id),
        adapter_stream_gate_summary_digest: extract_json_string_field(
            payload,
            "adapter_stream_gate_summary_digest",
        )
        .and_then(RuntimeDiagnostics::normalize_adapter_stream_gate_summary_digest),
        device_profile: extract_json_string_field(payload, "device_profile"),
        primary_lane: extract_json_string_field(payload, "primary_lane"),
        fallback_lane: extract_json_string_field(payload, "fallback_lane"),
        memory_mode: extract_json_string_field(payload, "memory_mode"),
        device_execution_source: extract_json_string_field(payload, "device_execution_source")
            .and_then(RuntimeDiagnostics::normalize_device_execution_source),
        layer_count: extract_json_usize_field(payload, "layer_count").unwrap_or(0),
        global_layers: extract_json_usize_field(payload, "global_layers").unwrap_or(0),
        local_window_layers: extract_json_usize_field(payload, "local_window_layers").unwrap_or(0),
        convolutional_fusion_layers: extract_json_usize_field(
            payload,
            "convolutional_fusion_layers",
        )
        .unwrap_or(0),
        hidden_size: extract_json_usize_field(payload, "hidden_size").unwrap_or(0),
        local_window_tokens: extract_json_usize_field(payload, "local_window_tokens").unwrap_or(0),
        forward_energy: extract_json_finite_number_field(payload, "forward_energy"),
        kv_influence: extract_json_finite_number_field(payload, "kv_influence"),
        imported_kv_blocks: extract_json_usize_field(payload, "imported_kv_blocks").unwrap_or(0),
        weak_runtime_kv_imports_skipped: extract_json_usize_field(
            payload,
            "weak_runtime_kv_imports_skipped",
        )
        .unwrap_or(0),
        exported_kv_blocks: extract_json_usize_field(payload, "exported_kv_blocks").unwrap_or(0),
        runtime_kv_segments_included: extract_json_usize_field(
            payload,
            "runtime_kv_segments_included",
        )
        .unwrap_or(0),
        runtime_kv_segments_skipped: extract_json_usize_field(
            payload,
            "runtime_kv_segments_skipped",
        )
        .unwrap_or(0),
        runtime_kv_segments_rejected: extract_json_usize_field(
            payload,
            "runtime_kv_segments_rejected",
        )
        .unwrap_or(0),
        hot_kv_precision_bits: extract_json_kv_precision_bits(payload, "hot_kv_precision_bits"),
        cold_kv_precision_bits: extract_json_kv_precision_bits(payload, "cold_kv_precision_bits"),
    }
}
