mod config;
mod defaults;
mod fit;
mod metadata;
mod paths;
mod quantization;
mod spec;

pub use config::GemmaRuntimeConfig;
pub use defaults::{GemmaRuntimeDefaults, ensure_gemma4_12b_runtime_defaults};
pub use fit::GemmaRuntimeFitSummary;
pub use metadata::normalize_runtime_metadata;
pub use paths::infer_hf_cache_from_local_snapshot;
pub use quantization::GemmaRuntimeQuantizationMode;
pub use spec::{
    GEMMA4_12B_ATTENTION_HEADS, GEMMA4_12B_DEFAULT_LOCAL_ISQ, GEMMA4_12B_DEFAULT_PAGED_ATTN,
    GEMMA4_12B_DEFAULT_QUANT, GEMMA4_12B_DEFAULT_RUNTIME_WINDOW, GEMMA4_12B_DEFAULT_THINKING,
    GEMMA4_12B_HIDDEN_SIZE, GEMMA4_12B_KV_HEADS, GEMMA4_12B_LAYER_COUNT,
    GEMMA4_12B_LOCAL_WINDOW_TOKENS, GEMMA4_12B_MODEL_ID, GEMMA4_12B_NATIVE_CONTEXT_WINDOW,
    GEMMA4_12B_TOKENIZER,
};

#[cfg(test)]
mod tests;
