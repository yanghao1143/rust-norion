use crate::kv_exchange::RuntimeKvBlock;

use super::diagnostics::RuntimeDiagnostics;

#[derive(Debug, Clone)]
pub struct ReasoningStep {
    pub label: String,
    pub content: String,
    pub confidence: f32,
}

impl ReasoningStep {
    pub fn new(label: impl Into<String>, content: impl Into<String>, confidence: f32) -> Self {
        Self {
            label: label.into(),
            content: content.into(),
            confidence: confidence.clamp(0.0, 1.0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DraftToken {
    pub text: String,
    pub logprob: Option<f32>,
    pub entropy: Option<f32>,
}

impl DraftToken {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            logprob: None,
            entropy: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct InferenceDraft {
    pub answer: String,
    pub trace: Vec<ReasoningStep>,
    pub tokens: Vec<DraftToken>,
    pub exported_kv_blocks: Vec<RuntimeKvBlock>,
    pub runtime_diagnostics: RuntimeDiagnostics,
}

impl InferenceDraft {
    pub fn new(answer: impl Into<String>, trace: Vec<ReasoningStep>) -> Self {
        Self {
            answer: answer.into(),
            trace,
            tokens: Vec::new(),
            exported_kv_blocks: Vec::new(),
            runtime_diagnostics: RuntimeDiagnostics::default(),
        }
    }

    pub fn with_tokens(mut self, tokens: Vec<DraftToken>) -> Self {
        self.tokens = tokens;
        self
    }

    pub fn with_exported_kv_blocks(mut self, blocks: Vec<RuntimeKvBlock>) -> Self {
        self.exported_kv_blocks = blocks;
        self.runtime_diagnostics.exported_kv_blocks = self.exported_kv_blocks.len();
        self
    }

    pub fn with_runtime_diagnostics(mut self, diagnostics: RuntimeDiagnostics) -> Self {
        self.runtime_diagnostics = diagnostics;
        self.runtime_diagnostics.exported_kv_blocks = self.exported_kv_blocks.len();
        self
    }
}
