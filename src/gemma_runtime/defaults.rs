use crate::runtime::RuntimeMetadata;
use crate::runtime_manifest::TransformerRuntimeArchitecture;

use super::config::GemmaRuntimeConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GemmaRuntimeDefaults {
    pub metadata: RuntimeMetadata,
    pub architecture: TransformerRuntimeArchitecture,
}

struct GemmaRuntimeDefaultTargets<'a> {
    enabled: &'a mut bool,
    runtime_metadata: &'a mut RuntimeMetadata,
    runtime_layer_count: &'a mut Option<usize>,
    runtime_hidden_size: &'a mut Option<usize>,
    runtime_attention_heads: &'a mut Option<usize>,
    runtime_kv_heads: &'a mut Option<usize>,
    runtime_local_window_tokens: &'a mut Option<usize>,
}

impl GemmaRuntimeDefaults {
    pub fn gemma4_12b() -> Self {
        let config = GemmaRuntimeConfig::default();
        Self {
            metadata: config.metadata(),
            architecture: config.architecture(),
        }
    }

    fn apply_if_disabled(&self, targets: GemmaRuntimeDefaultTargets<'_>) {
        if *targets.enabled {
            return;
        }

        *targets.enabled = true;
        *targets.runtime_metadata = self.metadata.clone();
        *targets.runtime_layer_count = Some(self.architecture.layer_count);
        *targets.runtime_hidden_size = Some(self.architecture.hidden_size);
        *targets.runtime_attention_heads = Some(self.architecture.attention_heads);
        *targets.runtime_kv_heads = Some(self.architecture.kv_heads);
        *targets.runtime_local_window_tokens = Some(self.architecture.local_window_tokens);
    }
}

pub fn ensure_gemma4_12b_runtime_defaults(
    enabled: &mut bool,
    runtime_metadata: &mut RuntimeMetadata,
    runtime_layer_count: &mut Option<usize>,
    runtime_hidden_size: &mut Option<usize>,
    runtime_attention_heads: &mut Option<usize>,
    runtime_kv_heads: &mut Option<usize>,
    runtime_local_window_tokens: &mut Option<usize>,
) {
    GemmaRuntimeDefaults::gemma4_12b().apply_if_disabled(GemmaRuntimeDefaultTargets {
        enabled,
        runtime_metadata,
        runtime_layer_count,
        runtime_hidden_size,
        runtime_attention_heads,
        runtime_kv_heads,
        runtime_local_window_tokens,
    });
}
