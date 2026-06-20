#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeMetadata {
    pub model_id: String,
    pub tokenizer: String,
    pub native_context_window: usize,
    pub embedding_dimensions: usize,
    pub supports_kv_import: bool,
    pub supports_kv_export: bool,
    pub max_kv_import_blocks: usize,
    pub max_kv_export_blocks: usize,
    pub hot_kv_precision_bits: u8,
    pub cold_kv_precision_bits: u8,
}

impl RuntimeMetadata {
    pub fn new(
        model_id: impl Into<String>,
        tokenizer: impl Into<String>,
        native_context_window: usize,
        embedding_dimensions: usize,
    ) -> Self {
        Self {
            model_id: model_id.into(),
            tokenizer: tokenizer.into(),
            native_context_window,
            embedding_dimensions,
            supports_kv_import: false,
            supports_kv_export: false,
            max_kv_import_blocks: 0,
            max_kv_export_blocks: 0,
            hot_kv_precision_bits: 8,
            cold_kv_precision_bits: 4,
        }
    }

    pub fn with_kv_exchange(mut self, import: bool, export: bool) -> Self {
        self.supports_kv_import = import;
        self.supports_kv_export = export;
        self.max_kv_import_blocks = if import {
            self.max_kv_import_blocks.max(8)
        } else {
            0
        };
        self.max_kv_export_blocks = if export {
            self.max_kv_export_blocks.max(4)
        } else {
            0
        };
        self
    }

    pub fn with_kv_limits(mut self, max_import_blocks: usize, max_export_blocks: usize) -> Self {
        self.max_kv_import_blocks = if self.supports_kv_import {
            max_import_blocks.max(1)
        } else {
            0
        };
        self.max_kv_export_blocks = if self.supports_kv_export {
            max_export_blocks.max(1)
        } else {
            0
        };
        self
    }

    pub fn with_kv_precision(mut self, hot_bits: u8, cold_bits: u8) -> Self {
        let hot_bits = if matches!(hot_bits, 4 | 8) {
            hot_bits
        } else {
            8
        };
        let cold_bits = if matches!(cold_bits, 4 | 8) {
            cold_bits
        } else {
            4
        };
        self.hot_kv_precision_bits = hot_bits;
        self.cold_kv_precision_bits = cold_bits.min(hot_bits);
        self
    }

    pub fn summary(&self) -> String {
        format!(
            "model_id={} tokenizer={} native_context_window={} embedding_dimensions={} kv_import={} kv_export={} max_kv_import_blocks={} max_kv_export_blocks={} kv_bits={}/{}",
            self.model_id,
            self.tokenizer,
            self.native_context_window,
            self.embedding_dimensions,
            self.supports_kv_import,
            self.supports_kv_export,
            self.max_kv_import_blocks,
            self.max_kv_export_blocks,
            self.hot_kv_precision_bits,
            self.cold_kv_precision_bits
        )
    }
}

impl Default for RuntimeMetadata {
    fn default() -> Self {
        Self {
            model_id: "unknown-self-developed-runtime".to_owned(),
            tokenizer: "unknown".to_owned(),
            native_context_window: 0,
            embedding_dimensions: 0,
            supports_kv_import: false,
            supports_kv_export: false,
            max_kv_import_blocks: 0,
            max_kv_export_blocks: 0,
            hot_kv_precision_bits: 8,
            cold_kv_precision_bits: 4,
        }
    }
}
