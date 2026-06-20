use crate::hardware::RuntimeManifestDeviceGateReport;
use crate::kv_exchange::RuntimeKvBlock;
use crate::reflection::{ReasoningStep, RuntimeDiagnostics};
use crate::runtime::{RuntimeError, RuntimeRequest, RuntimeToken};
use crate::runtime_manifest::RuntimeManifest;

use super::assets::RuntimeAssetSummary;

pub trait ProductionForwardKernel: std::fmt::Debug + Send + Sync {
    fn generate(
        &self,
        context: ProductionKernelContext<'_>,
    ) -> Result<ProductionKernelOutput, RuntimeError>;
}

#[derive(Debug, Clone, Copy)]
pub struct ProductionKernelContext<'a> {
    pub manifest: &'a RuntimeManifest,
    pub device_gate: &'a RuntimeManifestDeviceGateReport,
    pub assets: &'a RuntimeAssetSummary,
    pub imported_kv_blocks: &'a [RuntimeKvBlock],
    pub request: &'a RuntimeRequest,
}

#[derive(Debug, Clone)]
pub struct ProductionKernelOutput {
    pub answer: String,
    pub tokens: Vec<RuntimeToken>,
    pub trace: Vec<ReasoningStep>,
    pub diagnostics: RuntimeDiagnostics,
    pub exported_kv_blocks: Vec<RuntimeKvBlock>,
}

impl ProductionKernelOutput {
    pub fn new(answer: impl Into<String>) -> Self {
        Self {
            answer: answer.into(),
            tokens: Vec::new(),
            trace: Vec::new(),
            diagnostics: RuntimeDiagnostics::default(),
            exported_kv_blocks: Vec::new(),
        }
    }

    pub fn with_tokens(mut self, tokens: Vec<RuntimeToken>) -> Self {
        self.tokens = tokens;
        self
    }

    pub fn with_trace(mut self, trace: Vec<ReasoningStep>) -> Self {
        self.trace = trace;
        self
    }

    pub fn with_diagnostics(mut self, diagnostics: RuntimeDiagnostics) -> Self {
        self.diagnostics = diagnostics;
        self
    }

    pub fn with_exported_kv_blocks(mut self, blocks: Vec<RuntimeKvBlock>) -> Self {
        self.exported_kv_blocks = blocks;
        self
    }
}
