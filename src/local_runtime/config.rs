use crate::runtime::RuntimeMetadata;
use crate::runtime_manifest::{RuntimeKvPolicy, RuntimeManifest};

use super::architecture::normalize_manifest_architecture;

const DEFAULT_MODEL_ID: &str = "noiron-local-transformer";
const DEFAULT_TOKENIZER: &str = "noiron-local-tokenizer";
const DEFAULT_CONTEXT_WINDOW: usize = 8_192;
const DEFAULT_EMBEDDING_DIMENSIONS: usize = 64;

pub(super) fn local_manifest(
    native_context_window: usize,
    embedding_dimensions: usize,
) -> RuntimeManifest {
    normalize_local_manifest(RuntimeManifest::self_developed(
        DEFAULT_MODEL_ID,
        DEFAULT_TOKENIZER,
        native_context_window,
        embedding_dimensions,
    ))
}

pub(super) fn manifest_from_metadata(metadata: RuntimeMetadata) -> RuntimeManifest {
    normalize_local_manifest(
        RuntimeManifest::from_metadata(defaulted_metadata(metadata))
            .with_kv_policy(RuntimeKvPolicy::import_export()),
    )
}

pub(super) fn normalize_local_manifest(mut manifest: RuntimeManifest) -> RuntimeManifest {
    manifest.metadata = defaulted_metadata(manifest.metadata);
    manifest.kv_policy.import_enabled = true;
    manifest.kv_policy.export_enabled = true;
    manifest.kv_policy.max_import_blocks = manifest.kv_policy.max_import_blocks.max(1);
    manifest.kv_policy.max_export_blocks = manifest.kv_policy.max_export_blocks.max(1);
    manifest.metadata.supports_kv_import = manifest.kv_policy.import_enabled;
    manifest.metadata.supports_kv_export = manifest.kv_policy.export_enabled;
    manifest.architecture = normalize_manifest_architecture(&manifest);
    manifest
}

fn defaulted_metadata(mut metadata: RuntimeMetadata) -> RuntimeMetadata {
    let default_metadata = RuntimeMetadata::default();
    if metadata.model_id == default_metadata.model_id {
        metadata.model_id = DEFAULT_MODEL_ID.to_owned();
    }
    if metadata.tokenizer == default_metadata.tokenizer {
        metadata.tokenizer = DEFAULT_TOKENIZER.to_owned();
    }
    if metadata.native_context_window == 0 {
        metadata.native_context_window = DEFAULT_CONTEXT_WINDOW;
    }
    if metadata.embedding_dimensions == 0 {
        metadata.embedding_dimensions = DEFAULT_EMBEDDING_DIMENSIONS;
    }
    metadata
}
