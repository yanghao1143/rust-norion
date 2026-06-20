use crate::runtime_manifest::{RuntimeManifest, TransformerRuntimeArchitecture};

pub(super) fn normalize_manifest_architecture(
    manifest: &RuntimeManifest,
) -> TransformerRuntimeArchitecture {
    let embedding_dimensions = manifest.metadata.embedding_dimensions.max(1);
    let native_window = manifest.metadata.native_context_window.max(1);
    let mut architecture = manifest.architecture;
    if architecture.layer_count == 0 {
        architecture.layer_count = 24;
    }
    if architecture.hidden_size == 0 {
        architecture.hidden_size = embedding_dimensions;
    }
    if architecture.attention_heads == 0 {
        architecture.attention_heads = choose_head_count(architecture.hidden_size);
    }
    if architecture.kv_heads == 0 {
        architecture.kv_heads = architecture.attention_heads;
    }
    architecture.kv_heads = architecture.kv_heads.min(architecture.attention_heads);
    if architecture.local_window_tokens == 0 {
        architecture.local_window_tokens = native_window.min(4_096);
    }
    architecture.local_window_tokens = architecture.local_window_tokens.min(native_window);
    architecture
}

fn choose_head_count(hidden_size: usize) -> usize {
    [16, 12, 8, 6, 4, 2]
        .into_iter()
        .find(|heads| hidden_size % heads == 0)
        .unwrap_or(1)
}
