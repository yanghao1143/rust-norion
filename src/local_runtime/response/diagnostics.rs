use crate::kv_exchange::RuntimeKvBlock;
use crate::reflection::RuntimeDiagnostics;
use crate::runtime::RuntimeRequest;
use crate::runtime_manifest::RuntimeManifest;

use super::super::forward::LocalForwardState;
use super::ResponseEvidence;

pub(super) fn build_diagnostics(
    request: &RuntimeRequest,
    manifest: &RuntimeManifest,
    imported_kv_blocks: &[RuntimeKvBlock],
    exported_kv_blocks: &[RuntimeKvBlock],
    forward: &LocalForwardState,
    evidence: &ResponseEvidence,
) -> RuntimeDiagnostics {
    RuntimeDiagnostics {
        model_id: Some(manifest.metadata.model_id.clone()),
        selected_adapter: evidence.selected_adapter.clone(),
        device_profile: Some(request.hardware_plan.device.as_str().to_owned()),
        primary_lane: Some(
            request
                .hardware_plan
                .execution
                .primary_lane
                .as_str()
                .to_owned(),
        ),
        fallback_lane: Some(
            request
                .hardware_plan
                .execution
                .fallback_lane
                .as_str()
                .to_owned(),
        ),
        memory_mode: Some(
            request
                .hardware_plan
                .execution
                .memory_mode
                .as_str()
                .to_owned(),
        ),
        device_execution_source: Some(
            RuntimeDiagnostics::runtime_reported_device_execution_source().to_owned(),
        ),
        layer_count: forward.layer_summaries.len(),
        global_layers: evidence.transformer_counts.global,
        local_window_layers: evidence.transformer_counts.local,
        convolutional_fusion_layers: evidence.transformer_counts.convolution,
        hidden_size: manifest.architecture.hidden_size,
        local_window_tokens: manifest.architecture.local_window_tokens,
        forward_energy: Some(forward.energy),
        kv_influence: Some(forward.kv_influence),
        imported_kv_blocks: imported_kv_blocks.len(),
        exported_kv_blocks: exported_kv_blocks.len(),
        hot_kv_precision_bits: Some(request.hardware_plan.execution.hot_kv_precision_bits),
        cold_kv_precision_bits: Some(request.hardware_plan.execution.cold_kv_precision_bits),
        ..RuntimeDiagnostics::default()
    }
}
