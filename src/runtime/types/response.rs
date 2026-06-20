use crate::kv_exchange::RuntimeKvBlock;
use crate::reflection::{ReasoningStep, RuntimeDiagnostics};

use super::RuntimeToken;

#[derive(Debug, Clone)]
pub struct RuntimeResponse {
    pub answer: String,
    pub tokens: Vec<RuntimeToken>,
    pub trace: Vec<ReasoningStep>,
    pub diagnostics: RuntimeDiagnostics,
    pub exported_kv_blocks: Vec<RuntimeKvBlock>,
}

impl RuntimeResponse {
    pub fn new(answer: impl Into<String>) -> Self {
        Self {
            answer: answer.into(),
            tokens: Vec::new(),
            trace: Vec::new(),
            diagnostics: RuntimeDiagnostics::default(),
            exported_kv_blocks: Vec::new(),
        }
    }

    pub fn with_diagnostics(mut self, diagnostics: RuntimeDiagnostics) -> Self {
        self.diagnostics = diagnostics;
        self
    }
}
