use std::fs;
use std::path::{Path, PathBuf};

use crate::hardware::{HardwarePlan, RuntimeAdapterHint, RuntimeManifestDeviceGateReport};
use crate::kv_exchange::RuntimeKvBlock;
use crate::runtime::{
    ModelRuntime, RuntimeEmbedding, RuntimeError, RuntimeMetadata, RuntimeRequest, RuntimeResponse,
    RuntimeTokenId,
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
}

#[derive(Debug, Clone)]
pub struct ProductionTransformerRuntime {
    manifest: RuntimeManifest,
    device_gate: RuntimeManifestDeviceGateReport,
    assets: RuntimeAssetSummary,
    imported_kv_blocks: Vec<RuntimeKvBlock>,
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

    pub fn summary_line(&self) -> String {
        format!(
            "production_runtime: model_id={} device={} adapter={} weights_bytes={} tokenizer_bytes={} kernel=not-connected",
            self.manifest.metadata.model_id,
            self.device_gate.device.as_str(),
            self.device_gate.runtime_adapter_name(),
            self.assets.weights_bytes,
            self.assets.tokenizer_bytes
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
        Ok(Vec::new())
    }

    fn generate(&mut self, request: RuntimeRequest) -> Result<RuntimeResponse, RuntimeError> {
        Err(RuntimeError::new(format!(
            "production Transformer kernel is not connected for model_id={} adapter={} device={}; manifest assets and device contract passed, but a self-developed forward kernel must implement generate for prompt '{}'",
            self.manifest.metadata.model_id,
            self.device_gate.runtime_adapter_name(),
            self.device_gate.device.as_str(),
            compact(&request.prompt, 96)
        )))
    }
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
}
