#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransformerRuntimeArchitecture {
    pub layer_count: usize,
    pub hidden_size: usize,
    pub attention_heads: usize,
    pub kv_heads: usize,
    pub local_window_tokens: usize,
}

impl TransformerRuntimeArchitecture {
    pub fn new(
        layer_count: usize,
        hidden_size: usize,
        attention_heads: usize,
        kv_heads: usize,
        local_window_tokens: usize,
    ) -> Self {
        Self {
            layer_count,
            hidden_size,
            attention_heads,
            kv_heads,
            local_window_tokens,
        }
    }

    pub fn summary(self) -> String {
        format!(
            "layers={} hidden={} attention_heads={} kv_heads={} local_window={}",
            self.layer_count,
            self.hidden_size,
            self.attention_heads,
            self.kv_heads,
            self.local_window_tokens
        )
    }

    pub(super) fn validation_errors(self) -> Vec<String> {
        let mut errors = Vec::new();
        if self.layer_count == 0 {
            errors.push("layer_count must be greater than zero".to_owned());
        }
        if self.hidden_size == 0 {
            errors.push("hidden_size must be greater than zero".to_owned());
        }
        if self.attention_heads == 0 {
            errors.push("attention_heads must be greater than zero".to_owned());
        }
        if self.kv_heads == 0 {
            errors.push("kv_heads must be greater than zero".to_owned());
        }
        if self.local_window_tokens == 0 {
            errors.push("local_window_tokens must be greater than zero".to_owned());
        }
        if self.attention_heads > 0 && self.hidden_size % self.attention_heads != 0 {
            errors.push("hidden_size must be divisible by attention_heads".to_owned());
        }
        if self.kv_heads > self.attention_heads && self.attention_heads > 0 {
            errors.push("kv_heads must not exceed attention_heads".to_owned());
        }
        errors
    }
}

pub fn default_transformer_runtime_architecture(
    native_context_window: usize,
    embedding_dimensions: usize,
) -> TransformerRuntimeArchitecture {
    let native_context_window = native_context_window.max(1);
    let embedding_dimensions = embedding_dimensions.max(1);
    let heads = choose_head_count(embedding_dimensions);
    TransformerRuntimeArchitecture::new(
        24,
        embedding_dimensions,
        heads,
        heads,
        native_context_window.min(4_096),
    )
}

fn choose_head_count(hidden_size: usize) -> usize {
    [16, 12, 8, 6, 4, 2]
        .into_iter()
        .find(|heads| hidden_size % heads == 0)
        .unwrap_or(1)
}
