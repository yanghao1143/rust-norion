use crate::hardware::{DeviceClass, DeviceExecutionPlan, RuntimeAdapterHint};
use crate::runtime::{RuntimeAdapterObservation, RuntimeMetadata};

use super::architecture::{
    TransformerRuntimeArchitecture, default_transformer_runtime_architecture,
};
use super::assets::RuntimeAssetPaths;
use super::kv_policy::RuntimeKvPolicy;
use super::quantization::RuntimeQuantizationPolicy;
use super::validation::{
    RuntimeManifestValidation, validate_optional_asset_file, validate_required_asset_file,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeManifest {
    pub metadata: RuntimeMetadata,
    pub architecture: TransformerRuntimeArchitecture,
    pub assets: RuntimeAssetPaths,
    pub kv_policy: RuntimeKvPolicy,
    pub quantization: RuntimeQuantizationPolicy,
    pub supported_devices: Vec<DeviceClass>,
    pub adapter_hints: Vec<RuntimeAdapterHint>,
    pub retired_adapter_hints: Vec<RuntimeAdapterHint>,
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
        let quantization = RuntimeQuantizationPolicy::from_metadata(&metadata);
        let kv_policy = RuntimeKvPolicy::from_capabilities(
            metadata.supports_kv_import,
            metadata.supports_kv_export,
        );
        Self {
            metadata,
            architecture: default_transformer_runtime_architecture(
                native_context_window,
                embedding_dimensions,
            ),
            assets: RuntimeAssetPaths::default(),
            kv_policy,
            quantization,
            supported_devices: DeviceClass::explicit_profiles().to_vec(),
            adapter_hints: default_adapter_hints(),
            retired_adapter_hints: Vec::new(),
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

    pub fn with_retired_adapter_hints(
        mut self,
        retired_adapter_hints: Vec<RuntimeAdapterHint>,
    ) -> Self {
        self.retired_adapter_hints = retired_adapter_hints;
        self
    }

    pub fn is_adapter_retired(&self, adapter: RuntimeAdapterHint) -> bool {
        self.retired_adapter_hints.contains(&adapter)
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
            .filter(|adapter| !self.is_adapter_retired(*adapter))
            .find(|adapter| self.adapter_hints.contains(adapter));

        if execution.adapter_hints.is_empty() {
            device_supported.or_else(|| {
                self.adapter_hints
                    .iter()
                    .copied()
                    .find(|adapter| !self.is_adapter_retired(*adapter))
            })
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
            .filter(|observation| !self.is_adapter_retired(observation.adapter))
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
        if self.quantization.cold_kv.width() > self.quantization.hot_kv.width() {
            errors.push("cold_kv quantization width must not exceed hot_kv width".to_owned());
        }

        RuntimeManifestValidation { errors, warnings }
    }

    pub fn validate_for_production(&self) -> RuntimeManifestValidation {
        let mut validation = self.validate();

        validate_required_asset_file(
            "weights",
            self.assets.weights.as_deref(),
            &mut validation.errors,
        );
        validate_required_asset_file(
            "tokenizer",
            self.assets.tokenizer.as_deref(),
            &mut validation.errors,
        );
        if let Some(config_path) = self.assets.config.as_deref() {
            validate_optional_asset_file("config", config_path, &mut validation.errors);
        }

        validation
    }
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
