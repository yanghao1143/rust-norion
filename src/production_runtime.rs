use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::hardware::{HardwarePlan, RuntimeAdapterHint, RuntimeManifestDeviceGateReport};
use crate::kv_exchange::RuntimeKvBlock;
use crate::reflection::{ReasoningStep, RuntimeDiagnostics};
use crate::runtime::{
    ModelRuntime, RuntimeEmbedding, RuntimeError, RuntimeMetadata, RuntimeRequest, RuntimeResponse,
    RuntimeToken, RuntimeTokenId,
};
use crate::runtime_manifest::{RuntimeManifest, TransformerRuntimeArchitecture};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeAssetSummary {
    pub weights_path: PathBuf,
    pub weights_bytes: u64,
    pub tokenizer_path: PathBuf,
    pub tokenizer_bytes: u64,
    pub config_path: Option<PathBuf>,
    pub config_bytes: Option<u64>,
}

impl RuntimeAssetSummary {
    fn from_manifest(manifest: &RuntimeManifest) -> Result<Self, RuntimeError> {
        let weights_path = manifest.assets.weights.clone().ok_or_else(|| {
            RuntimeError::new("weights asset path is required for production runtimes")
        })?;
        let tokenizer_path = manifest.assets.tokenizer.clone().ok_or_else(|| {
            RuntimeError::new("tokenizer asset path is required for production runtimes")
        })?;
        let config_path = manifest.assets.config.clone();

        Ok(Self {
            weights_bytes: asset_len("weights", &weights_path)?,
            tokenizer_bytes: asset_len("tokenizer", &tokenizer_path)?,
            config_bytes: config_path
                .as_deref()
                .map(|path| asset_len("config", path))
                .transpose()?,
            weights_path,
            tokenizer_path,
            config_path,
        })
    }

    pub fn summary_line(&self) -> String {
        format!(
            "weights={} weights_bytes={} tokenizer={} tokenizer_bytes={} config={} config_bytes={}",
            self.weights_path.display(),
            self.weights_bytes,
            self.tokenizer_path.display(),
            self.tokenizer_bytes,
            self.config_path
                .as_deref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "none".to_owned()),
            self.config_bytes
                .map(|bytes| bytes.to_string())
                .unwrap_or_else(|| "none".to_owned())
        )
    }
}

#[derive(Debug, Clone)]
pub struct ProductionTransformerRuntime {
    manifest: RuntimeManifest,
    device_gate: RuntimeManifestDeviceGateReport,
    assets: RuntimeAssetSummary,
    imported_kv_blocks: Vec<RuntimeKvBlock>,
    exported_kv_blocks: Vec<RuntimeKvBlock>,
    kernel: Option<Arc<dyn ProductionForwardKernel>>,
}

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
        self.imported_kv_blocks = blocks.iter().take(max_blocks).cloned().collect();

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
        if let Some(kernel) = &self.kernel {
            self.exported_kv_blocks.clear();
            let output = kernel.generate(ProductionKernelContext {
                manifest: &self.manifest,
                device_gate: &self.device_gate,
                assets: &self.assets,
                imported_kv_blocks: &self.imported_kv_blocks,
                request: &request,
            })?;
            self.exported_kv_blocks =
                validate_exported_kv_blocks(output.exported_kv_blocks, &self.manifest, &request)?;

            let mut response =
                RuntimeResponse::new(output.answer).with_diagnostics(normalize_kernel_diagnostics(
                    output.diagnostics,
                    &self.manifest,
                    &self.device_gate,
                    self.imported_kv_blocks.len(),
                    self.exported_kv_blocks.len(),
                ));
            response.tokens = output.tokens;
            response.trace = output.trace;

            return Ok(response);
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

fn validate_exported_kv_blocks(
    blocks: Vec<RuntimeKvBlock>,
    manifest: &RuntimeManifest,
    request: &RuntimeRequest,
) -> Result<Vec<RuntimeKvBlock>, RuntimeError> {
    if !manifest.kv_policy.export_enabled {
        if blocks.is_empty() {
            return Ok(Vec::new());
        }

        return Err(RuntimeError::new(format!(
            "production kernel exported {} KV blocks but runtime KV export is disabled for model_id={}",
            blocks.len(),
            manifest.metadata.model_id
        )));
    }

    let mut accepted = Vec::new();
    for (index, block) in blocks
        .into_iter()
        .take(manifest.kv_policy.max_export_blocks)
        .enumerate()
    {
        validate_exported_kv_block(index, &block, manifest, request)?;
        accepted.push(block);
    }

    Ok(accepted)
}

fn validate_exported_kv_block(
    index: usize,
    block: &RuntimeKvBlock,
    manifest: &RuntimeManifest,
    request: &RuntimeRequest,
) -> Result<(), RuntimeError> {
    let architecture = manifest.architecture;
    let token_upper_bound = manifest
        .metadata
        .native_context_window
        .max(request.runtime_metadata.native_context_window)
        .max(request.recursive_schedule.prompt_tokens)
        .saturating_add(request.max_tokens.max(1));

    let token_span = block.token_end.saturating_sub(block.token_start).max(1);
    let per_token_vector_bound = architecture
        .hidden_size
        .max(manifest.metadata.embedding_dimensions)
        .max(request.runtime_metadata.embedding_dimensions)
        .max(1);
    let vector_bound = per_token_vector_bound.saturating_mul(token_span);

    let error = |reason: String| {
        RuntimeError::new(format!(
            "production kernel returned invalid exported KV block {index} for model_id={}: {reason}",
            manifest.metadata.model_id
        ))
    };

    if block.layer >= architecture.layer_count {
        return Err(error(format!(
            "layer {} exceeds manifest layer_count {}",
            block.layer, architecture.layer_count
        )));
    }
    if block.head >= architecture.kv_heads {
        return Err(error(format!(
            "head {} exceeds manifest kv_heads {}",
            block.head, architecture.kv_heads
        )));
    }
    if block.token_start >= block.token_end {
        return Err(error(format!(
            "token range {}..{} is empty or reversed",
            block.token_start, block.token_end
        )));
    }
    if block.token_end > token_upper_bound {
        return Err(error(format!(
            "token_end {} exceeds request-local KV bound {}",
            block.token_end, token_upper_bound
        )));
    }
    if block.key.is_empty() || block.value.is_empty() {
        return Err(error(
            "key and value vectors must both be non-empty".to_owned(),
        ));
    }
    if block.key.len() != block.value.len() {
        return Err(error(format!(
            "key/value dimensions differ: key={} value={}",
            block.key.len(),
            block.value.len()
        )));
    }
    if block.key.len() > vector_bound {
        return Err(error(format!(
            "key/value dimensions {} exceed per-block bound {}",
            block.key.len(),
            vector_bound
        )));
    }
    if !block.key.iter().all(|value| value.is_finite()) {
        return Err(error("key contains non-finite value".to_owned()));
    }
    if !block.value.iter().all(|value| value.is_finite()) {
        return Err(error("value contains non-finite value".to_owned()));
    }

    Ok(())
}

fn normalize_kernel_diagnostics(
    mut diagnostics: RuntimeDiagnostics,
    manifest: &RuntimeManifest,
    device_gate: &RuntimeManifestDeviceGateReport,
    imported_kv_blocks: usize,
    exported_kv_blocks: usize,
) -> RuntimeDiagnostics {
    if diagnostics.model_id.is_none() {
        diagnostics.model_id = Some(manifest.metadata.model_id.clone());
    }
    if diagnostics.selected_adapter.is_none() {
        diagnostics.selected_adapter = device_gate
            .runtime_adapter
            .map(|adapter| adapter.as_str().to_owned());
    }
    if diagnostics.layer_count == 0 {
        diagnostics.layer_count = manifest.architecture.layer_count;
    }
    if diagnostics.hidden_size == 0 {
        diagnostics.hidden_size = manifest.architecture.hidden_size;
    }
    if diagnostics.local_window_tokens == 0 {
        diagnostics.local_window_tokens = manifest.architecture.local_window_tokens;
    }
    diagnostics.imported_kv_blocks = imported_kv_blocks;
    diagnostics.exported_kv_blocks = exported_kv_blocks;
    diagnostics
}

fn asset_len(label: &str, path: &Path) -> Result<u64, RuntimeError> {
    fs::metadata(path)
        .map(|metadata| metadata.len())
        .map_err(|error| {
            RuntimeError::new(format!(
                "failed to read {label} asset metadata at {}: {error}",
                path.display()
            ))
        })
}

fn production_tokenize(text: &str) -> Vec<String> {
    let tokens = text
        .split_whitespace()
        .map(|token| {
            token
                .trim_matches(|ch: char| ch.is_ascii_punctuation())
                .to_owned()
        })
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();

    if !tokens.is_empty() {
        return tokens;
    }

    text.chars()
        .filter(|ch| !ch.is_whitespace())
        .map(|ch| ch.to_string())
        .collect()
}

fn embed_tokens(tokens: &[RuntimeTokenId], dimensions: usize) -> Vec<f32> {
    let dimensions = dimensions.max(1);
    let mut vector = vec![0.0; dimensions];

    for (position, token) in tokens.iter().enumerate() {
        let hash = stable_hash(&format!("{}:{}", token.id, token.text));
        for offset in 0..4 {
            let index = ((hash >> (offset * 11)) as usize) % dimensions;
            vector[index] += 1.0 / (position as f32 + offset as f32 + 1.0);
        }
    }

    normalize(&mut vector);
    vector
}

fn normalize(vector: &mut [f32]) {
    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in vector {
            *value /= norm;
        }
    }
}

fn stable_hash(value: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn compact(text: &str, max_chars: usize) -> String {
    let mut out = text.chars().take(max_chars).collect::<String>();
    if text.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hardware::{DeviceClass, HardwareAllocator, HardwareSnapshot, RuntimeAdapterHint};
    use crate::hierarchy::HierarchyWeights;
    use crate::runtime::ModelRuntime;
    use crate::runtime_manifest::{
        RuntimeAssetPaths, RuntimeKvPolicy, TransformerRuntimeArchitecture,
    };
    use std::fs::{self, File};
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn production_runtime_accepts_manifest_assets_and_device_contract() {
        let (asset_dir, weights, tokenizer, config) = create_assets("production-runtime-assets");
        let manifest = production_manifest(&weights, &tokenizer)
            .with_assets(
                RuntimeAssetPaths::new()
                    .with_weights(&weights)
                    .with_tokenizer(&tokenizer)
                    .with_config(&config),
            )
            .with_adapter_hints(vec![
                RuntimeAdapterHint::PortableRust,
                RuntimeAdapterHint::CpuSimd,
            ]);
        let plan = cpu_plan();

        let runtime =
            ProductionTransformerRuntime::from_manifest_for_plan(manifest, &plan).unwrap();

        assert_eq!(runtime.metadata().model_id, "noiron-production-transformer");
        assert_eq!(runtime.architecture().layer_count, 6);
        assert!(runtime.device_gate().passed());
        assert_eq!(runtime.assets().weights_bytes, 7);
        assert_eq!(runtime.assets().tokenizer_bytes, 9);
        assert_eq!(runtime.assets().config_bytes, Some(6));
        assert!(runtime.runtime_device_contract().contains("device=cpu"));
        assert!(runtime.selected_adapter().is_some());
        assert!(runtime.summary_line().contains("kernel=not-connected"));
        assert!(!runtime.kernel_connected());

        fs::remove_dir_all(asset_dir).unwrap();
    }

    #[test]
    fn production_runtime_exposes_bootstrap_tokens_and_embeddings() {
        let (asset_dir, weights, tokenizer, _config) = create_assets("production-runtime-embed");
        let manifest = production_manifest(&weights, &tokenizer);
        let runtime =
            ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan()).unwrap();

        let tokens = runtime.tokenize("Rust Noiron runtime").unwrap();
        let embedding = runtime.embed(&tokens).unwrap();

        assert_eq!(tokens.len(), 3);
        assert_eq!(embedding.dimensions, 64);
        assert!(embedding.values.iter().any(|value| *value > 0.0));

        fs::remove_dir_all(asset_dir).unwrap();
    }

    #[test]
    fn production_runtime_rejects_missing_assets() {
        let manifest = RuntimeManifest::self_developed("missing-production", "tokenizer", 4096, 64)
            .with_architecture(TransformerRuntimeArchitecture::new(6, 64, 4, 2, 1024));

        let error = ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan())
            .unwrap_err();

        assert!(error.message().contains("weights asset path is required"));
        assert!(error.message().contains("tokenizer asset path is required"));
    }

    #[test]
    fn production_runtime_rejects_device_adapter_mismatch() {
        let (asset_dir, weights, tokenizer, _config) = create_assets("production-runtime-mismatch");
        let manifest = production_manifest(&weights, &tokenizer)
            .with_adapter_hints(vec![RuntimeAdapterHint::Cuda]);

        let error = ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan())
            .unwrap_err();

        assert!(error.message().contains("device gate rejected"));
        assert!(error.message().contains("no adapter intersection"));

        fs::remove_dir_all(asset_dir).unwrap();
    }

    #[test]
    fn production_runtime_imports_kv_with_manifest_and_device_limits() {
        let (asset_dir, weights, tokenizer, _config) =
            create_assets("production-runtime-import-kv");
        let manifest = production_manifest(&weights, &tokenizer).with_kv_policy(RuntimeKvPolicy {
            import_enabled: true,
            export_enabled: true,
            max_import_blocks: 1,
            max_export_blocks: 2,
        });
        let mut plan = cpu_plan();
        plan.execution.kv_prefetch_blocks = 1;
        let mut runtime =
            ProductionTransformerRuntime::from_manifest_for_plan(manifest, &plan).unwrap();
        let blocks = vec![
            RuntimeKvBlock::new(0, 0, 0, 1, vec![0.1], vec![0.2]),
            RuntimeKvBlock::new(0, 1, 1, 2, vec![0.3], vec![0.4]),
        ];

        let imported = runtime.import_kv(&blocks).unwrap();
        let exported = runtime.export_kv().unwrap();

        assert_eq!(imported, 1);
        assert_eq!(runtime.imported_kv_blocks().len(), 1);
        assert!(exported.is_empty());

        fs::remove_dir_all(asset_dir).unwrap();
    }

    #[test]
    fn production_runtime_generate_errors_until_forward_kernel_is_connected() {
        let (asset_dir, weights, tokenizer, _config) = create_assets("production-runtime-generate");
        let manifest = production_manifest(&weights, &tokenizer);
        let mut runtime =
            ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan()).unwrap();

        let error = runtime.generate(sample_request()).unwrap_err();

        assert!(error.message().contains("kernel is not connected"));
        assert!(error.message().contains("noiron-production-transformer"));

        fs::remove_dir_all(asset_dir).unwrap();
    }

    #[test]
    fn production_runtime_can_generate_through_attached_forward_kernel() {
        let (asset_dir, weights, tokenizer, _config) =
            create_assets("production-runtime-attached-kernel");
        let manifest = production_manifest(&weights, &tokenizer);
        let mut runtime =
            ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan())
                .unwrap()
                .with_kernel(MockForwardKernel);
        let blocks = vec![RuntimeKvBlock::new(0, 0, 0, 1, vec![0.1], vec![0.2])];
        runtime.import_kv(&blocks).unwrap();

        let response = runtime.generate(sample_request()).unwrap();
        let exported = runtime.export_kv().unwrap();

        assert!(runtime.kernel_connected());
        assert!(runtime.summary_line().contains("kernel=connected"));
        assert!(response.answer.contains("kernel answer"));
        assert_eq!(response.tokens.len(), 1);
        assert_eq!(response.trace[0].label, "production_kernel");
        assert_eq!(
            response.diagnostics.model_id.as_deref(),
            Some("noiron-production-transformer")
        );
        assert_eq!(
            response.diagnostics.selected_adapter.as_deref(),
            Some("portable-rust")
        );
        assert_eq!(response.diagnostics.imported_kv_blocks, 1);
        assert_eq!(response.diagnostics.exported_kv_blocks, 1);
        assert_eq!(exported.len(), 1);
        assert_eq!(runtime.exported_kv_blocks().len(), 1);

        fs::remove_dir_all(asset_dir).unwrap();
    }

    #[test]
    fn production_runtime_rejects_invalid_kernel_exported_kv() {
        let (asset_dir, weights, tokenizer, _config) =
            create_assets("production-runtime-invalid-kernel-kv");
        let manifest = production_manifest(&weights, &tokenizer);
        let runtime = ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan())
            .unwrap()
            .with_kernel(MockForwardKernel);
        let mut runtime = runtime;

        runtime.generate(sample_request()).unwrap();
        assert_eq!(runtime.exported_kv_blocks().len(), 1);

        let cases = vec![
            (
                "layer",
                RuntimeKvBlock::new(6, 0, 0, 1, vec![0.1], vec![0.2]),
                "layer 6 exceeds manifest layer_count 6",
            ),
            (
                "head",
                RuntimeKvBlock::new(0, 2, 0, 1, vec![0.1], vec![0.2]),
                "head 2 exceeds manifest kv_heads 2",
            ),
            (
                "range",
                RuntimeKvBlock::new(0, 0, 2, 2, vec![0.1], vec![0.2]),
                "token range 2..2 is empty or reversed",
            ),
            (
                "dimension",
                RuntimeKvBlock::new(0, 0, 0, 1, vec![0.1], vec![0.2, 0.3]),
                "key/value dimensions differ",
            ),
            (
                "finite",
                RuntimeKvBlock::new(0, 0, 0, 1, vec![f32::NAN], vec![0.2]),
                "key contains non-finite value",
            ),
        ];

        for (label, block, expected) in cases {
            let mut invalid_runtime = runtime.clone().with_kernel(InvalidExportKernel { block });

            let error = invalid_runtime.generate(sample_request()).unwrap_err();

            assert!(
                error.message().contains("invalid exported KV block 0"),
                "{label}: {}",
                error.message()
            );
            assert!(
                error.message().contains(expected),
                "{label}: {}",
                error.message()
            );
            assert!(invalid_runtime.exported_kv_blocks().is_empty(), "{label}");
            assert!(invalid_runtime.export_kv().unwrap().is_empty(), "{label}");
        }

        fs::remove_dir_all(asset_dir).unwrap();
    }

    fn production_manifest(weights: &Path, tokenizer: &Path) -> RuntimeManifest {
        RuntimeManifest::self_developed(
            "noiron-production-transformer",
            "noiron-production-tokenizer",
            4096,
            64,
        )
        .with_architecture(TransformerRuntimeArchitecture::new(6, 64, 4, 2, 1024))
        .with_supported_devices(vec![DeviceClass::CpuOnly])
        .with_adapter_hints(vec![RuntimeAdapterHint::PortableRust])
        .with_assets(
            RuntimeAssetPaths::new()
                .with_weights(weights)
                .with_tokenizer(tokenizer),
        )
    }

    fn cpu_plan() -> HardwarePlan {
        HardwareAllocator::new().plan(
            HardwareSnapshot::new(DeviceClass::CpuOnly, 0.20, 0.10, 0.30, 0.10),
            crate::hierarchy::TaskProfile::General,
            512,
            HierarchyWeights::default(),
        )
    }

    fn sample_request() -> RuntimeRequest {
        RuntimeRequest {
            prompt: "connect production runtime".to_owned(),
            profile: crate::hierarchy::TaskProfile::General,
            runtime_metadata: RuntimeMetadata::new(
                "noiron-production-transformer",
                "noiron-production-tokenizer",
                4096,
                64,
            ),
            runtime_architecture: TransformerRuntimeArchitecture::new(6, 64, 4, 2, 1024),
            memory_hints: Vec::new(),
            infini_memory_hints: Vec::new(),
            experience_hints: Vec::new(),
            runtime_adapter_observations: Vec::new(),
            route_budget: crate::router::RouteBudget {
                threshold: 0.5,
                attention_tokens: 1,
                fast_tokens: 1,
                attention_fraction: 0.5,
            },
            hierarchy: HierarchyWeights::default(),
            transformer_plan: crate::transformer::TransformerRefactorPlan::default(),
            recursive_schedule: crate::recursive_scheduler::RecursiveSchedule::default(),
            hardware_plan: cpu_plan(),
            max_tokens: 32,
        }
    }

    fn create_assets(name: &str) -> (PathBuf, PathBuf, PathBuf, PathBuf) {
        let asset_dir = temp_asset_dir(name);
        fs::create_dir_all(&asset_dir).unwrap();
        let weights = asset_dir.join("weights.noiron");
        let tokenizer = asset_dir.join("tokenizer.noiron");
        let config = asset_dir.join("config.noiron");
        write_asset(&weights, b"weights");
        write_asset(&tokenizer, b"tokenizer");
        write_asset(&config, b"config");
        (asset_dir, weights, tokenizer, config)
    }

    fn write_asset(path: &Path, bytes: &[u8]) {
        let mut file = File::create(path).unwrap();
        file.write_all(bytes).unwrap();
    }

    fn temp_asset_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{name}-{unique}"))
    }

    #[derive(Debug, Clone)]
    struct MockForwardKernel;

    impl ProductionForwardKernel for MockForwardKernel {
        fn generate(
            &self,
            context: ProductionKernelContext<'_>,
        ) -> Result<ProductionKernelOutput, RuntimeError> {
            Ok(ProductionKernelOutput::new(format!(
                "kernel answer for {} with {} imported KV blocks",
                context.manifest.metadata.model_id,
                context.imported_kv_blocks.len()
            ))
            .with_tokens(vec![RuntimeToken {
                text: "kernel".to_owned(),
                logprob: Some(-0.2),
                entropy: Some(0.3),
            }])
            .with_trace(vec![ReasoningStep::new(
                "production_kernel",
                context.device_gate.runtime_device_contract.clone(),
                0.88,
            )])
            .with_diagnostics(RuntimeDiagnostics {
                forward_energy: Some(0.42),
                kv_influence: Some(0.25),
                ..RuntimeDiagnostics::default()
            })
            .with_exported_kv_blocks(vec![RuntimeKvBlock::new(1, 0, 0, 1, vec![0.3], vec![0.4])]))
        }
    }

    #[derive(Debug, Clone)]
    struct InvalidExportKernel {
        block: RuntimeKvBlock,
    }

    impl ProductionForwardKernel for InvalidExportKernel {
        fn generate(
            &self,
            _context: ProductionKernelContext<'_>,
        ) -> Result<ProductionKernelOutput, RuntimeError> {
            Ok(ProductionKernelOutput::new("invalid kernel export")
                .with_exported_kv_blocks(vec![self.block.clone()]))
        }
    }
}
