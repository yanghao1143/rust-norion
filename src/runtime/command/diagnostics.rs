use crate::reflection::RuntimeDiagnostics;
use crate::runtime_manifest::TransformerRuntimeArchitecture;

use crate::runtime::RuntimeMetadata;

pub(in crate::runtime) fn populate_static_runtime_diagnostics(
    diagnostics: &mut RuntimeDiagnostics,
    metadata: &RuntimeMetadata,
    architecture: TransformerRuntimeArchitecture,
) {
    if diagnostics.model_id.is_none() && !metadata.model_id.trim().is_empty() {
        diagnostics.model_id = Some(metadata.model_id.clone());
    }
    if diagnostics.layer_count == 0 {
        diagnostics.layer_count = architecture.layer_count;
    }
    if diagnostics.hidden_size == 0 {
        diagnostics.hidden_size = architecture.hidden_size;
    }
    if diagnostics.local_window_tokens == 0 {
        diagnostics.local_window_tokens = architecture.local_window_tokens;
    }
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
