use std::sync::Arc;

use crate::hardware::{HardwarePlan, RuntimeAdapterHint, RuntimeManifestDeviceGateReport};
use crate::kv_exchange::RuntimeKvBlock;
use crate::runtime::{
    ModelRuntime, RuntimeEmbedding, RuntimeError, RuntimeMetadata, RuntimeRequest, RuntimeResponse,
    RuntimeTokenId,
};
use crate::runtime_manifest::{RuntimeManifest, TransformerRuntimeArchitecture};

use super::assets::RuntimeAssetSummary;
use super::kernel::{ProductionForwardKernel, ProductionKernelContext};
use super::util::{compact, embed_tokens, production_tokenize, stable_hash};
use super::validation::{
    normalize_kernel_diagnostics, validate_exported_kv_blocks, validate_imported_kv_blocks,
    validate_production_runtime_request,
};

#[derive(Debug, Clone)]
pub struct ProductionTransformerRuntime {
    pub(super) manifest: RuntimeManifest,
    pub(super) device_gate: RuntimeManifestDeviceGateReport,
    assets: RuntimeAssetSummary,
    imported_kv_blocks: Vec<RuntimeKvBlock>,
    exported_kv_blocks: Vec<RuntimeKvBlock>,
    kernel: Option<Arc<dyn ProductionForwardKernel>>,
}

pub(super) struct ProductionKernelGeneration {
    pub response: RuntimeResponse,
    pub kernel_reported_runtime_kv_segment_signal: bool,
}

impl ProductionTransformerRuntime {
    pub fn from_manifest_for_plan(
        manifest: RuntimeManifest,
        plan: &HardwarePlan,
    ) -> Result<Self, RuntimeError> {
        let validation = manifest.validate_for_production();
        if !validation.passed() {
            return Err(RuntimeError::new(format!(
                "production runtime manifest rejected for model_id={}: {}",
                manifest.metadata.model_id,
                validation.errors.join("; ")
            )));
        }

        let device_gate = RuntimeManifestDeviceGateReport::evaluate(&manifest, plan);
        if !device_gate.passed() {
            return Err(RuntimeError::new(format!(
                "production runtime device gate rejected model_id={} device={} adapter={}: {}",
                manifest.metadata.model_id,
                device_gate.device.as_str(),
                device_gate.runtime_adapter_name(),
                device_gate.failures.join("; ")
            )));
        }

        let assets = RuntimeAssetSummary::from_manifest(&manifest)?;

        Ok(Self {
            manifest,
            device_gate,
            assets,
            imported_kv_blocks: Vec::new(),
            exported_kv_blocks: Vec::new(),
            kernel: None,
        })
    }

    pub fn manifest(&self) -> &RuntimeManifest {
        &self.manifest
    }

    pub fn device_gate(&self) -> &RuntimeManifestDeviceGateReport {
        &self.device_gate
    }

    pub fn assets(&self) -> &RuntimeAssetSummary {
        &self.assets
    }

    pub fn selected_adapter(&self) -> Option<RuntimeAdapterHint> {
        self.device_gate.runtime_adapter
    }

    pub fn runtime_device_contract(&self) -> &str {
        &self.device_gate.runtime_device_contract
    }

    pub fn imported_kv_blocks(&self) -> &[RuntimeKvBlock] {
        &self.imported_kv_blocks
    }

    pub fn exported_kv_blocks(&self) -> &[RuntimeKvBlock] {
        &self.exported_kv_blocks
    }

    pub fn with_kernel<K>(mut self, kernel: K) -> Self
    where
        K: ProductionForwardKernel + 'static,
    {
        self.kernel = Some(Arc::new(kernel));
        self
    }

    pub fn with_shared_kernel(mut self, kernel: Arc<dyn ProductionForwardKernel>) -> Self {
        self.kernel = Some(kernel);
        self
    }

    pub fn kernel_connected(&self) -> bool {
        self.kernel.is_some()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "production_runtime: model_id={} device={} adapter={} weights_bytes={} tokenizer_bytes={} kernel={}",
            self.manifest.metadata.model_id,
            self.device_gate.device.as_str(),
            self.device_gate.runtime_adapter_name(),
            self.assets.weights_bytes,
            self.assets.tokenizer_bytes,
            if self.kernel_connected() {
                "connected"
            } else {
                "not-connected"
            }
        )
    }

    pub(super) fn generate_with_kernel_report(
        &mut self,
        request: RuntimeRequest,
    ) -> Result<ProductionKernelGeneration, RuntimeError> {
        if let Some(kernel) = &self.kernel {
            validate_production_runtime_request(&self.manifest, &self.device_gate, &request)?;
            self.exported_kv_blocks.clear();
            let output = kernel.generate(ProductionKernelContext {
                manifest: &self.manifest,
                device_gate: &self.device_gate,
                assets: &self.assets,
                imported_kv_blocks: &self.imported_kv_blocks,
                request: &request,
            })?;
            let kernel_reported_runtime_kv_segment_signal =
                output.diagnostics.has_runtime_kv_segment_signal();
            self.exported_kv_blocks =
                validate_exported_kv_blocks(output.exported_kv_blocks, &self.manifest, &request)?;

            let mut response =
                RuntimeResponse::new(output.answer).with_diagnostics(normalize_kernel_diagnostics(
                    output.diagnostics,
                    &self.manifest,
                    &self.device_gate,
                    &request.hardware_plan,
                    self.imported_kv_blocks.len(),
                    self.exported_kv_blocks.len(),
                ));
            response.tokens = output.tokens;
            response.trace = output.trace;

            return Ok(ProductionKernelGeneration {
                response,
                kernel_reported_runtime_kv_segment_signal,
            });
        }

        Err(RuntimeError::new(format!(
            "production Transformer kernel is not connected for model_id={} adapter={} device={}; manifest assets and device contract passed, but a self-developed forward kernel must implement generate for prompt '{}'",
            self.manifest.metadata.model_id,
            self.device_gate.runtime_adapter_name(),
            self.device_gate.device.as_str(),
            compact(&request.prompt, 96)
        )))
    }
}

impl ModelRuntime for ProductionTransformerRuntime {
    fn metadata(&self) -> RuntimeMetadata {
        self.manifest.runtime_metadata()
    }

    fn architecture(&self) -> TransformerRuntimeArchitecture {
        self.manifest.architecture
    }

    fn tokenize(&self, prompt: &str) -> Result<Vec<RuntimeTokenId>, RuntimeError> {
        Ok(production_tokenize(prompt)
            .into_iter()
            .map(|text| RuntimeTokenId::new((stable_hash(&text) % 1_000_000) as u32, text))
            .collect())
    }

    fn embed(&self, tokens: &[RuntimeTokenId]) -> Result<RuntimeEmbedding, RuntimeError> {
        Ok(RuntimeEmbedding::new(embed_tokens(
            tokens,
            self.manifest.metadata.embedding_dimensions,
        )))
    }

    fn import_kv(&mut self, blocks: &[RuntimeKvBlock]) -> Result<usize, RuntimeError> {
        if !self.manifest.kv_policy.import_enabled {
            self.imported_kv_blocks.clear();
            return Ok(0);
        }

        let max_blocks = self
            .manifest
            .kv_policy
            .max_import_blocks
            .min(self.device_gate.kv_prefetch_blocks);
        self.imported_kv_blocks.clear();
        self.imported_kv_blocks = validate_imported_kv_blocks(blocks, max_blocks, &self.manifest)?;

        Ok(self.imported_kv_blocks.len())
    }

    fn export_kv(&mut self) -> Result<Vec<RuntimeKvBlock>, RuntimeError> {
        if !self.manifest.kv_policy.export_enabled {
            self.exported_kv_blocks.clear();
            return Ok(Vec::new());
        }

        Ok(self
            .exported_kv_blocks
            .iter()
            .take(self.manifest.kv_policy.max_export_blocks)
            .cloned()
            .collect())
    }

    fn generate(&mut self, request: RuntimeRequest) -> Result<RuntimeResponse, RuntimeError> {
        self.generate_with_kernel_report(request)
            .map(|generation| generation.response)
    }
}
