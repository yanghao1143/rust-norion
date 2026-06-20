use crate::hardware::{DeviceClass, HardwarePlan, RuntimeManifestDeviceGateReport};
use crate::hierarchy::{HierarchyWeights, TaskProfile};
use crate::kv_exchange::RuntimeKvBlock;
use crate::reflection::RuntimeDiagnostics;
use crate::router::RouteBudget;
use crate::runtime::{RuntimeError, RuntimeRequest};
use crate::runtime_manifest::RuntimeManifest;
use crate::transformer::TransformerPlanner;

pub(super) fn validate_imported_kv_blocks(
    blocks: &[RuntimeKvBlock],
    max_blocks: usize,
    manifest: &RuntimeManifest,
) -> Result<Vec<RuntimeKvBlock>, RuntimeError> {
    let mut accepted = Vec::new();
    for (index, block) in blocks.iter().take(max_blocks).enumerate() {
        validate_kv_block(
            index,
            block,
            manifest,
            manifest.metadata.native_context_window,
            "imported",
            "control plane supplied invalid imported KV block",
        )?;
        accepted.push(block.clone());
    }

    Ok(accepted)
}

pub(super) fn validate_exported_kv_blocks(
    blocks: Vec<RuntimeKvBlock>,
    manifest: &RuntimeManifest,
    request: &RuntimeRequest,
) -> Result<Vec<RuntimeKvBlock>, RuntimeError> {
    if !manifest.kv_policy.export_enabled {
        if blocks.is_empty() {
            return Ok(Vec::new());
        }

        return Err(RuntimeError::new(format!(
            "production kernel exported {} KV blocks but runtime KV export is disabled for model_id={}",
            blocks.len(),
            manifest.metadata.model_id
        )));
    }

    let token_upper_bound = manifest
        .metadata
        .native_context_window
        .max(request.runtime_metadata.native_context_window)
        .max(request.recursive_schedule.prompt_tokens)
        .saturating_add(request.max_tokens.max(1));
    let mut accepted = Vec::new();
    for (index, block) in blocks
        .into_iter()
        .take(manifest.kv_policy.max_export_blocks)
        .enumerate()
    {
        validate_kv_block(
            index,
            &block,
            manifest,
            token_upper_bound,
            "exported",
            "production kernel returned invalid exported KV block",
        )?;
        accepted.push(block);
    }

    Ok(accepted)
}

fn validate_kv_block(
    index: usize,
    block: &RuntimeKvBlock,
    manifest: &RuntimeManifest,
    token_upper_bound: usize,
    direction: &str,
    prefix: &str,
) -> Result<(), RuntimeError> {
    let architecture = manifest.architecture;
    let token_span = block.token_end.saturating_sub(block.token_start).max(1);
    let per_token_vector_bound = architecture
        .hidden_size
        .max(manifest.metadata.embedding_dimensions)
        .max(1);
    let vector_bound = per_token_vector_bound.saturating_mul(token_span);

    let error = |reason: String| {
        RuntimeError::new(format!(
            "{prefix} {index} for model_id={}: {reason}",
            manifest.metadata.model_id
        ))
    };

    if block.layer >= architecture.layer_count {
        return Err(error(format!(
            "layer {} exceeds manifest layer_count {}",
            block.layer, architecture.layer_count
        )));
    }
    if block.head >= architecture.kv_heads {
        return Err(error(format!(
            "head {} exceeds manifest kv_heads {}",
            block.head, architecture.kv_heads
        )));
    }
    if block.token_start >= block.token_end {
        return Err(error(format!(
            "token range {}..{} is empty or reversed",
            block.token_start, block.token_end
        )));
    }
    if block.token_end > token_upper_bound {
        return Err(error(format!(
            "token_end {} exceeds KV token bound {}",
            block.token_end, token_upper_bound
        )));
    }
    if block.key.is_empty() || block.value.is_empty() {
        return Err(error(
            "key and value vectors must both be non-empty".to_owned(),
        ));
    }
    if block.key.len() != block.value.len() {
        return Err(error(format!(
            "key/value dimensions differ: key={} value={}",
            block.key.len(),
            block.value.len()
        )));
    }
    if block.key.len() > vector_bound {
        return Err(error(format!(
            "key/value dimensions {} exceed per-block bound {}",
            block.key.len(),
            vector_bound
        )));
    }
    if !block.key.iter().all(|value| value.is_finite()) {
        return Err(error(format!("{direction} key contains non-finite value")));
    }
    if !block.value.iter().all(|value| value.is_finite()) {
        return Err(error(format!(
            "{direction} value contains non-finite value"
        )));
    }

    Ok(())
}

pub(super) fn normalize_kernel_diagnostics(
    mut diagnostics: RuntimeDiagnostics,
    manifest: &RuntimeManifest,
    device_gate: &RuntimeManifestDeviceGateReport,
    hardware_plan: &HardwarePlan,
    imported_kv_blocks: usize,
    exported_kv_blocks: usize,
) -> RuntimeDiagnostics {
    let kernel_has_forward_signal = diagnostics.has_forward_signal();
    let runtime_reported_complete = diagnostics.has_device_execution_signal()
        && diagnostics
            .device_execution_source
            .as_deref()
            .map(|source| {
                source != RuntimeDiagnostics::control_plane_filled_device_execution_source()
            })
            .unwrap_or(true);

    if diagnostics.model_id.is_none() {
        diagnostics.model_id = Some(manifest.metadata.model_id.clone());
    }
    if diagnostics.selected_adapter.is_none() {
        diagnostics.selected_adapter = device_gate
            .runtime_adapter
            .map(|adapter| adapter.as_str().to_owned());
    }
    if diagnostics.device_profile.is_none() {
        diagnostics.device_profile = Some(device_gate.device.as_str().to_owned());
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
    if diagnostics.layer_count == 0 {
        diagnostics.layer_count = manifest.architecture.layer_count;
    }
    if diagnostics.layer_mode_count() == 0 && kernel_has_forward_signal {
        let fallback_counts = TransformerPlanner::new(
            manifest.architecture.layer_count,
            manifest.architecture.local_window_tokens.max(16),
        )
        .plan(
            TaskProfile::Coding,
            HierarchyWeights::default(),
            RouteBudget {
                threshold: 0.5,
                attention_tokens: 2,
                fast_tokens: 1,
                attention_fraction: 2.0 / 3.0,
            },
        )
        .counts();
        diagnostics.global_layers = fallback_counts.global;
        diagnostics.local_window_layers = fallback_counts.local;
        diagnostics.convolutional_fusion_layers = fallback_counts.convolution;
    }
    if diagnostics.hidden_size == 0 {
        diagnostics.hidden_size = manifest.architecture.hidden_size;
    }
    if diagnostics.local_window_tokens == 0 {
        diagnostics.local_window_tokens = manifest.architecture.local_window_tokens;
    }
    if diagnostics.hot_kv_precision_bits.is_none() {
        diagnostics.hot_kv_precision_bits = Some(device_gate.hot_kv_precision_bits);
    }
    if diagnostics.cold_kv_precision_bits.is_none() {
        diagnostics.cold_kv_precision_bits = Some(device_gate.cold_kv_precision_bits);
    }
    diagnostics.imported_kv_blocks = imported_kv_blocks;
    diagnostics.exported_kv_blocks = exported_kv_blocks;
    diagnostics
}

pub(super) fn validate_production_runtime_request(
    manifest: &RuntimeManifest,
    device_gate: &RuntimeManifestDeviceGateReport,
    request: &RuntimeRequest,
) -> Result<(), RuntimeError> {
    let expected_metadata = manifest.runtime_metadata();
    let mut failures = Vec::new();

    if request.runtime_metadata.model_id != expected_metadata.model_id {
        failures.push(format!(
            "request model_id {} does not match manifest model_id {}",
            request.runtime_metadata.model_id, expected_metadata.model_id
        ));
    }
    if request.runtime_metadata.tokenizer != expected_metadata.tokenizer {
        failures.push(format!(
            "request tokenizer {} does not match manifest tokenizer {}",
            request.runtime_metadata.tokenizer, expected_metadata.tokenizer
        ));
    }
    if request.runtime_metadata.hot_kv_precision_bits > expected_metadata.hot_kv_precision_bits {
        failures.push(format!(
            "request runtime hot KV precision {} exceeds manifest hot KV precision {}",
            request.runtime_metadata.hot_kv_precision_bits, expected_metadata.hot_kv_precision_bits
        ));
    }
    if request.runtime_metadata.cold_kv_precision_bits > expected_metadata.cold_kv_precision_bits {
        failures.push(format!(
            "request runtime cold KV precision {} exceeds manifest cold KV precision {}",
            request.runtime_metadata.cold_kv_precision_bits,
            expected_metadata.cold_kv_precision_bits
        ));
    }
    if request.runtime_metadata.cold_kv_precision_bits
        > request.runtime_metadata.hot_kv_precision_bits
    {
        failures.push(format!(
            "request runtime cold KV precision {} must not exceed hot KV precision {}",
            request.runtime_metadata.cold_kv_precision_bits,
            request.runtime_metadata.hot_kv_precision_bits
        ));
    }
    if request.hardware_plan.device != DeviceClass::Auto
        && request.hardware_plan.device != device_gate.device
    {
        failures.push(format!(
            "request hardware device {} does not match production device gate {}",
            request.hardware_plan.device.as_str(),
            device_gate.device.as_str()
        ));
    }
    if request.hardware_plan.execution.hot_kv_precision_bits > device_gate.hot_kv_precision_bits {
        failures.push(format!(
            "request device hot KV precision {} exceeds production device gate hot KV precision {}",
            request.hardware_plan.execution.hot_kv_precision_bits,
            device_gate.hot_kv_precision_bits
        ));
    }
    if request.hardware_plan.execution.cold_kv_precision_bits > device_gate.cold_kv_precision_bits {
        failures.push(format!(
            "request device cold KV precision {} exceeds production device gate cold KV precision {}",
            request.hardware_plan.execution.cold_kv_precision_bits,
            device_gate.cold_kv_precision_bits
        ));
    }
    if request.hardware_plan.execution.cold_kv_precision_bits
        > request.hardware_plan.execution.hot_kv_precision_bits
    {
        failures.push(format!(
            "request device cold KV precision {} must not exceed hot KV precision {}",
            request.hardware_plan.execution.cold_kv_precision_bits,
            request.hardware_plan.execution.hot_kv_precision_bits
        ));
    }
    if request.runtime_architecture != manifest.architecture {
        failures.push(format!(
            "request architecture {} does not match manifest architecture {}",
            request.runtime_architecture.summary(),
            manifest.architecture.summary()
        ));
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(RuntimeError::new(format!(
            "production runtime request rejected for model_id={}: {}",
            manifest.metadata.model_id,
            failures.join("; ")
        )))
    }
}
