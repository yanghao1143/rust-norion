use std::path::PathBuf;

use crate::hardware::{DeviceClass, DeviceExecutionPlan, RuntimeAdapterHint};
use crate::kv_quant::QuantizationBits;
use crate::runtime::{RuntimeAdapterObservation, RuntimeMetadata};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeManifest {
    pub metadata: RuntimeMetadata,
    pub architecture: TransformerRuntimeArchitecture,
    pub assets: RuntimeAssetPaths,
    pub kv_policy: RuntimeKvPolicy,
    pub quantization: RuntimeQuantizationPolicy,
    pub supported_devices: Vec<DeviceClass>,
    pub adapter_hints: Vec<RuntimeAdapterHint>,
}

impl RuntimeManifest {
    pub fn self_developed(
        model_id: impl Into<String>,
        tokenizer: impl Into<String>,
        native_context_window: usize,
        embedding_dimensions: usize,
    ) -> Self {
        let metadata = RuntimeMetadata::new(
            model_id,
            tokenizer,
            native_context_window,
            embedding_dimensions,
        );
        Self::from_metadata(metadata).with_kv_policy(RuntimeKvPolicy::import_export())
    }

    pub fn from_metadata(metadata: RuntimeMetadata) -> Self {
        let native_context_window = metadata.native_context_window.max(1);
        let embedding_dimensions = metadata.embedding_dimensions.max(1);
        let kv_policy = RuntimeKvPolicy::from_capabilities(
            metadata.supports_kv_import,
            metadata.supports_kv_export,
        );
        Self {
            metadata,
            architecture: TransformerRuntimeArchitecture::new(
                24,
                embedding_dimensions,
                choose_head_count(embedding_dimensions),
                choose_head_count(embedding_dimensions),
                native_context_window.min(4_096),
            ),
            assets: RuntimeAssetPaths::default(),
            kv_policy,
            quantization: RuntimeQuantizationPolicy::default(),
            supported_devices: DeviceClass::explicit_profiles().to_vec(),
            adapter_hints: default_adapter_hints(),
        }
    }

    pub fn with_architecture(mut self, architecture: TransformerRuntimeArchitecture) -> Self {
        self.architecture = architecture;
        self
    }

    pub fn with_assets(mut self, assets: RuntimeAssetPaths) -> Self {
        self.assets = assets;
        self
    }

    pub fn with_kv_policy(mut self, kv_policy: RuntimeKvPolicy) -> Self {
        self.kv_policy = kv_policy;
        self.metadata.supports_kv_import = kv_policy.import_enabled;
        self.metadata.supports_kv_export = kv_policy.export_enabled;
        self
    }

    pub fn with_quantization(mut self, quantization: RuntimeQuantizationPolicy) -> Self {
        self.quantization = quantization;
        self
    }

    pub fn with_supported_devices(mut self, supported_devices: Vec<DeviceClass>) -> Self {
        self.supported_devices = supported_devices;
        self
    }

    pub fn with_adapter_hints(mut self, adapter_hints: Vec<RuntimeAdapterHint>) -> Self {
        self.adapter_hints = adapter_hints;
        self
    }

    pub fn runtime_metadata(&self) -> RuntimeMetadata {
        self.metadata
            .clone()
            .with_kv_exchange(self.kv_policy.import_enabled, self.kv_policy.export_enabled)
            .with_kv_limits(
                self.kv_policy.max_import_blocks,
                self.kv_policy.max_export_blocks,
            )
            .with_kv_precision(
                self.quantization.hot_kv.width(),
                self.quantization.cold_kv.width(),
            )
    }

    pub fn supports_device(&self, device: DeviceClass) -> bool {
        if device == DeviceClass::Auto {
            return !self.supported_devices.is_empty();
        }
        self.supported_devices.contains(&device)
    }

    pub fn preferred_adapter_for(
        &self,
        execution: &DeviceExecutionPlan,
    ) -> Option<RuntimeAdapterHint> {
        let device_supported = execution
            .adapter_hints
            .iter()
            .copied()
            .find(|adapter| self.adapter_hints.contains(adapter));

        if execution.adapter_hints.is_empty() {
            device_supported.or_else(|| self.adapter_hints.first().copied())
        } else {
            device_supported
        }
    }

    pub fn preferred_adapter_with_observations(
        &self,
        execution: &DeviceExecutionPlan,
        observations: &[RuntimeAdapterObservation],
    ) -> Option<RuntimeAdapterHint> {
        let fallback = self.preferred_adapter_for(execution);
        observations
            .iter()
            .filter(|observation| observation.score >= 0.50)
            .filter(|observation| execution.adapter_hints.contains(&observation.adapter))
            .filter(|observation| self.adapter_hints.contains(&observation.adapter))
            .max_by(|left, right| {
                left.score
                    .partial_cmp(&right.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| right.experience_id.cmp(&left.experience_id))
            })
            .map(|observation| observation.adapter)
            .or(fallback)
    }

    pub fn validate(&self) -> RuntimeManifestValidation {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        if self.metadata.model_id.trim().is_empty() {
            errors.push("model_id must not be empty".to_owned());
        }
        if self.metadata.tokenizer.trim().is_empty() {
            errors.push("tokenizer must not be empty".to_owned());
        }
        if self.metadata.native_context_window == 0 {
            errors.push("native_context_window must be greater than zero".to_owned());
        }
        if self.metadata.embedding_dimensions == 0 {
            errors.push("embedding_dimensions must be greater than zero".to_owned());
        }
        errors.extend(self.architecture.validation_errors());
        if self.architecture.local_window_tokens > self.metadata.native_context_window
            && self.metadata.native_context_window > 0
        {
            errors.push("local_window_tokens must not exceed native_context_window".to_owned());
        }
        if self.supported_devices.is_empty() {
            errors.push("supported_devices must not be empty".to_owned());
        }
        if self.supported_devices.contains(&DeviceClass::Auto) {
            warnings
                .push("supported_devices should list explicit device classes, not auto".to_owned());
        }
        if self.adapter_hints.is_empty() {
            warnings.push(
                "adapter_hints is empty; runtime will not advertise an execution lane".to_owned(),
            );
        }
        if self.assets.weights.is_none() {
            warnings.push(
                "weights path is not set; this is only valid for embedded or prototype runtimes"
                    .to_owned(),
            );
        }
        if self.metadata.supports_kv_import != self.kv_policy.import_enabled {
            warnings.push("metadata supports_kv_import differs from manifest kv_policy".to_owned());
        }
        if self.metadata.supports_kv_export != self.kv_policy.export_enabled {
            warnings.push("metadata supports_kv_export differs from manifest kv_policy".to_owned());
        }

        RuntimeManifestValidation { errors, warnings }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RuntimeAssetPaths {
    pub weights: Option<PathBuf>,
    pub tokenizer: Option<PathBuf>,
    pub config: Option<PathBuf>,
}

impl RuntimeAssetPaths {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_weights(mut self, path: impl Into<PathBuf>) -> Self {
        self.weights = Some(path.into());
        self
    }

    pub fn with_tokenizer(mut self, path: impl Into<PathBuf>) -> Self {
        self.tokenizer = Some(path.into());
        self
    }

    pub fn with_config(mut self, path: impl Into<PathBuf>) -> Self {
        self.config = Some(path.into());
        self
    }
}

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

    fn validation_errors(self) -> Vec<String> {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeKvPolicy {
    pub import_enabled: bool,
    pub export_enabled: bool,
    pub max_import_blocks: usize,
    pub max_export_blocks: usize,
}

impl RuntimeKvPolicy {
    pub fn disabled() -> Self {
        Self {
            import_enabled: false,
            export_enabled: false,
            max_import_blocks: 0,
            max_export_blocks: 0,
        }
    }

    pub fn import_export() -> Self {
        Self {
            import_enabled: true,
            export_enabled: true,
            max_import_blocks: 8,
            max_export_blocks: 4,
        }
    }

    pub fn from_capabilities(import_enabled: bool, export_enabled: bool) -> Self {
        Self {
            import_enabled,
            export_enabled,
            max_import_blocks: if import_enabled { 8 } else { 0 },
            max_export_blocks: if export_enabled { 4 } else { 0 },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeQuantizationPolicy {
    pub hot_kv: QuantizationBits,
    pub cold_kv: QuantizationBits,
    pub weights: Option<QuantizationBits>,
}

impl Default for RuntimeQuantizationPolicy {
    fn default() -> Self {
        Self {
            hot_kv: QuantizationBits::Eight,
            cold_kv: QuantizationBits::Four,
            weights: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeManifestValidation {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl RuntimeManifestValidation {
    pub fn passed(&self) -> bool {
        self.errors.is_empty()
    }
}

fn choose_head_count(hidden_size: usize) -> usize {
    [16, 12, 8, 6, 4, 2]
        .into_iter()
        .find(|heads| hidden_size % heads == 0)
        .unwrap_or(1)
}

fn default_adapter_hints() -> Vec<RuntimeAdapterHint> {
    vec![
        RuntimeAdapterHint::PortableRust,
        RuntimeAdapterHint::CpuSimd,
        RuntimeAdapterHint::Wgpu,
        RuntimeAdapterHint::WebGpu,
        RuntimeAdapterHint::Vulkan,
        RuntimeAdapterHint::Metal,
        RuntimeAdapterHint::Cuda,
        RuntimeAdapterHint::Rocm,
        RuntimeAdapterHint::OneApi,
        RuntimeAdapterHint::DirectMl,
        RuntimeAdapterHint::CoreMl,
        RuntimeAdapterHint::Nnapi,
        RuntimeAdapterHint::Qnn,
        RuntimeAdapterHint::OpenVino,
        RuntimeAdapterHint::Cann,
        RuntimeAdapterHint::Mlu,
        RuntimeAdapterHint::Rknn,
        RuntimeAdapterHint::MultiDevice,
        RuntimeAdapterHint::CustomAccelerator,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hardware::{ComputeLane, DeviceMemoryMode};

    #[test]
    fn self_developed_manifest_validates_and_covers_all_devices() {
        let manifest = RuntimeManifest::self_developed(
            "noiron-dev-transformer",
            "noiron-tokenizer",
            65_536,
            256,
        )
        .with_assets(
            RuntimeAssetPaths::new()
                .with_weights("weights.noiron")
                .with_tokenizer("tokenizer.noiron"),
        );

        let validation = manifest.validate();

        assert!(validation.passed(), "{validation:?}");
        for device in DeviceClass::explicit_profiles() {
            assert!(manifest.supports_device(*device), "{device:?}");
        }
        let metadata = manifest.runtime_metadata();
        assert!(metadata.supports_kv_import);
        assert!(metadata.supports_kv_export);
        assert_eq!(metadata.max_kv_import_blocks, 8);
        assert_eq!(metadata.max_kv_export_blocks, 4);
        assert_eq!(metadata.hot_kv_precision_bits, 8);
        assert_eq!(metadata.cold_kv_precision_bits, 4);
        assert_eq!(metadata.native_context_window, 65_536);
        assert_eq!(metadata.embedding_dimensions, 256);
    }

    #[test]
    fn manifest_selects_first_supported_device_adapter() {
        let manifest =
            RuntimeManifest::self_developed("model", "tokenizer", 8_192, 128).with_adapter_hints(
                vec![RuntimeAdapterHint::Wgpu, RuntimeAdapterHint::PortableRust],
            );
        let execution = DeviceExecutionPlan {
            primary_lane: ComputeLane::DiscreteGpu,
            fallback_lane: ComputeLane::CpuVector,
            memory_mode: DeviceMemoryMode::GpuResident,
            adapter_hints: vec![RuntimeAdapterHint::Cuda, RuntimeAdapterHint::Wgpu],
            max_parallel_chunks: 4,
            kv_prefetch_blocks: 4,
            hot_kv_precision_bits: 8,
            cold_kv_precision_bits: 4,
            allow_disk_spill: true,
        };

        assert_eq!(
            manifest.preferred_adapter_for(&execution),
            Some(RuntimeAdapterHint::Wgpu)
        );
    }

    #[test]
    fn manifest_does_not_pick_adapter_outside_device_plan() {
        let manifest = RuntimeManifest::self_developed("model", "tokenizer", 8_192, 128)
            .with_adapter_hints(vec![RuntimeAdapterHint::PortableRust]);
        let execution = DeviceExecutionPlan {
            primary_lane: ComputeLane::DiscreteGpu,
            fallback_lane: ComputeLane::CpuVector,
            memory_mode: DeviceMemoryMode::GpuResident,
            adapter_hints: vec![RuntimeAdapterHint::Cuda, RuntimeAdapterHint::Wgpu],
            max_parallel_chunks: 4,
            kv_prefetch_blocks: 4,
            hot_kv_precision_bits: 8,
            cold_kv_precision_bits: 4,
            allow_disk_spill: true,
        };

        assert_eq!(manifest.preferred_adapter_for(&execution), None);
    }

    #[test]
    fn manifest_uses_runtime_observations_within_device_adapter_bounds() {
        let manifest = RuntimeManifest::self_developed("model", "tokenizer", 8_192, 128)
            .with_adapter_hints(vec![
                RuntimeAdapterHint::Wgpu,
                RuntimeAdapterHint::Vulkan,
                RuntimeAdapterHint::PortableRust,
            ]);
        let execution = DeviceExecutionPlan {
            primary_lane: ComputeLane::DiscreteGpu,
            fallback_lane: ComputeLane::CpuVector,
            memory_mode: DeviceMemoryMode::GpuResident,
            adapter_hints: vec![
                RuntimeAdapterHint::Cuda,
                RuntimeAdapterHint::Vulkan,
                RuntimeAdapterHint::Wgpu,
            ],
            max_parallel_chunks: 4,
            kv_prefetch_blocks: 4,
            hot_kv_precision_bits: 8,
            cold_kv_precision_bits: 4,
            allow_disk_spill: true,
        };
        let observations = vec![
            RuntimeAdapterObservation::new(
                RuntimeAdapterHint::Wgpu,
                0.62,
                0.60,
                0.70,
                Some(0.20),
                Some(0.10),
                1,
            ),
            RuntimeAdapterObservation::new(
                RuntimeAdapterHint::Vulkan,
                0.91,
                0.90,
                0.92,
                Some(0.16),
                Some(0.40),
                2,
            ),
            RuntimeAdapterObservation::new(
                RuntimeAdapterHint::PortableRust,
                0.99,
                0.99,
                0.99,
                Some(0.10),
                Some(0.60),
                3,
            ),
        ];

        assert_eq!(
            manifest.preferred_adapter_with_observations(&execution, &observations),
            Some(RuntimeAdapterHint::Vulkan)
        );
    }

    #[test]
    fn kv_policy_updates_runtime_metadata_capabilities() {
        let manifest =
            RuntimeManifest::from_metadata(RuntimeMetadata::new("model", "tok", 4096, 64))
                .with_kv_policy(RuntimeKvPolicy::import_export());

        assert!(manifest.metadata.supports_kv_import);
        assert!(manifest.metadata.supports_kv_export);
        assert!(manifest.runtime_metadata().supports_kv_import);
        assert!(manifest.runtime_metadata().supports_kv_export);
        assert!(manifest.validate().passed());
    }

    #[test]
    fn invalid_manifest_reports_blocking_errors() {
        let manifest = RuntimeManifest {
            metadata: RuntimeMetadata::new("", "", 0, 0),
            architecture: TransformerRuntimeArchitecture::new(0, 130, 8, 16, 0),
            assets: RuntimeAssetPaths::default(),
            kv_policy: RuntimeKvPolicy::import_export(),
            quantization: RuntimeQuantizationPolicy::default(),
            supported_devices: Vec::new(),
            adapter_hints: Vec::new(),
        };

        let validation = manifest.validate();

        assert!(!validation.passed());
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.contains("model_id"))
        );
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.contains("native_context_window"))
        );
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.contains("hidden_size"))
        );
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.contains("supported_devices"))
        );
    }
}
