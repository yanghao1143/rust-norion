use crate::runtime::RuntimeMetadata;

pub fn normalize_runtime_metadata(metadata: RuntimeMetadata) -> RuntimeMetadata {
    let hot_bits = metadata.hot_kv_precision_bits;
    let cold_bits = metadata.cold_kv_precision_bits;
    let max_import_blocks = if metadata.supports_kv_import {
        metadata.max_kv_import_blocks.max(8)
    } else {
        0
    };
    let max_export_blocks = if metadata.supports_kv_export {
        metadata.max_kv_export_blocks.max(4)
    } else {
        0
    };

    metadata
        .with_kv_limits(max_import_blocks, max_export_blocks)
        .with_kv_precision(hot_bits, cold_bits)
}
