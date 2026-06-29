use crate::danger_signal::{DangerSignalInput, DangerSignalReview, review_danger_signals};
use crate::hardware::{DeviceClass, DeviceExecutionPlan, RuntimeAdapterHint};
use crate::runtime::{RuntimeAdapterObservation, RuntimeMetadata};

use super::architecture::{
    TransformerRuntimeArchitecture, default_transformer_runtime_architecture,
};
use super::assets::{RuntimeAssetPaths, RuntimeAssetProvenance, sha256_text_digest};
use super::kv_policy::RuntimeKvPolicy;
use super::quantization::RuntimeQuantizationPolicy;
use super::validation::{
    RuntimeManifestConformanceProbe, RuntimeManifestValidation, validate_optional_asset_file,
    validate_required_asset_file,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeAdapterLifecycleState {
    Active,
    Suspect,
    Quarantined,
    RetiredBlocked,
    TombstonePreview,
    RecycleCandidate,
    RepairedCandidate,
    RejectedFinal,
}

impl RuntimeAdapterLifecycleState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Suspect => "suspect",
            Self::Quarantined => "quarantined",
            Self::RetiredBlocked => "retired_blocked",
            Self::TombstonePreview => "tombstone_preview",
            Self::RecycleCandidate => "recycle_candidate",
            Self::RepairedCandidate => "repaired_candidate",
            Self::RejectedFinal => "rejected_final",
        }
    }

    pub fn blocks_runtime_worker(self) -> bool {
        !matches!(self, Self::Active)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeAdapterLifecycleRecord {
    pub adapter: RuntimeAdapterHint,
    pub state: RuntimeAdapterLifecycleState,
    pub reason_code: String,
    pub source_digest: String,
    pub parent_lineage: String,
    pub rollback_anchor: String,
    pub affected_scope: String,
    pub readmission_gate: String,
    pub operator_approval_required: bool,
}

impl RuntimeAdapterLifecycleRecord {
    pub fn new(
        adapter: RuntimeAdapterHint,
        state: RuntimeAdapterLifecycleState,
        reason_code: impl Into<String>,
        source_digest: impl Into<String>,
        parent_lineage: impl Into<String>,
        rollback_anchor: impl Into<String>,
        affected_scope: impl Into<String>,
    ) -> Self {
        Self {
            adapter,
            state,
            reason_code: reason_code.into(),
            source_digest: source_digest.into(),
            parent_lineage: parent_lineage.into(),
            rollback_anchor: rollback_anchor.into(),
            affected_scope: affected_scope.into(),
            readmission_gate: "hold_until_verifier_and_operator_approval".to_owned(),
            operator_approval_required: true,
        }
    }

    pub fn summary_line(&self) -> String {
        format!(
            "runtime_adapter_lifecycle adapter={} state={} reason_code={} source_digest={} parent_lineage={} rollback_anchor={} affected_scope={} readmission_gate={} operator_approval_required={}",
            self.adapter.as_str(),
            self.state.as_str(),
            self.reason_code,
            self.source_digest,
            self.parent_lineage,
            self.rollback_anchor,
            self.affected_scope,
            self.readmission_gate,
            self.operator_approval_required
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeManifest {
    pub metadata: RuntimeMetadata,
    pub architecture: TransformerRuntimeArchitecture,
    pub assets: RuntimeAssetPaths,
    pub asset_provenance: RuntimeAssetProvenance,
    pub kv_policy: RuntimeKvPolicy,
    pub quantization: RuntimeQuantizationPolicy,
    pub supported_devices: Vec<DeviceClass>,
    pub adapter_hints: Vec<RuntimeAdapterHint>,
    pub retired_adapter_hints: Vec<RuntimeAdapterHint>,
    pub adapter_lifecycle_records: Vec<RuntimeAdapterLifecycleRecord>,
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
            asset_provenance: RuntimeAssetProvenance::default(),
            kv_policy,
            quantization,
            supported_devices: DeviceClass::explicit_profiles().to_vec(),
            adapter_hints: default_adapter_hints(),
            retired_adapter_hints: Vec::new(),
            adapter_lifecycle_records: Vec::new(),
        }
    }

    pub fn with_architecture(mut self, architecture: TransformerRuntimeArchitecture) -> Self {
        self.architecture = architecture;
        self
    }

    pub fn with_assets(mut self, assets: RuntimeAssetPaths) -> Self {
        self.assets = assets;
        self.asset_provenance = RuntimeAssetProvenance::from_existing_assets(&self.assets);
        self
    }

    pub fn with_asset_provenance(mut self, provenance: RuntimeAssetProvenance) -> Self {
        self.asset_provenance = provenance;
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
            || self.adapter_lifecycle_records.iter().any(|record| {
                record.adapter == adapter
                    && record.state == RuntimeAdapterLifecycleState::RetiredBlocked
            })
    }

    pub fn with_adapter_lifecycle_records(
        mut self,
        adapter_lifecycle_records: Vec<RuntimeAdapterLifecycleRecord>,
    ) -> Self {
        self.adapter_lifecycle_records = adapter_lifecycle_records;
        self
    }

    pub fn adapter_lifecycle_block(
        &self,
        adapter: RuntimeAdapterHint,
    ) -> Option<&RuntimeAdapterLifecycleRecord> {
        self.adapter_lifecycle_records
            .iter()
            .find(|record| record.adapter == adapter && record.state.blocks_runtime_worker())
    }

    pub fn runtime_adapter_lifecycle_block_summary(
        &self,
        adapter: RuntimeAdapterHint,
    ) -> Option<String> {
        self.adapter_lifecycle_block(adapter)
            .map(RuntimeAdapterLifecycleRecord::summary_line)
            .or_else(|| {
                self.retired_adapter_hints
                    .contains(&adapter)
                    .then(|| format!("runtime_adapter_lifecycle adapter={} state=retired_blocked reason_code=retired_adapter_hint source_digest=missing parent_lineage=missing rollback_anchor=missing affected_scope=manifest readmission_gate=operator_approval_required operator_approval_required=true", adapter.as_str()))
            })
    }

    pub fn runtime_adapter_danger_signal_review(
        &self,
        adapter: RuntimeAdapterHint,
    ) -> DangerSignalReview {
        if let Some(record) = self
            .adapter_lifecycle_records
            .iter()
            .find(|record| record.adapter == adapter)
        {
            return review_danger_signals(
                DangerSignalInput::new("runtime_asset")
                    .trusted_self_provenance(record.state == RuntimeAdapterLifecycleState::Active)
                    .source_digest(record.source_digest.clone())
                    .lifecycle_state(record.state.as_str())
                    .affected_scope(record.affected_scope.clone())
                    .marker_text(format!(
                        "{} {} {}",
                        record.reason_code, record.parent_lineage, record.rollback_anchor
                    )),
            );
        }

        if self.retired_adapter_hints.contains(&adapter) {
            return review_danger_signals(
                DangerSignalInput::new("runtime_asset")
                    .source_digest("missing")
                    .lifecycle_state("retired_blocked")
                    .affected_scope("manifest"),
            );
        }

        review_danger_signals(
            DangerSignalInput::new("runtime_asset")
                .trusted_self_provenance(true)
                .source_digest("sha256:self-developed-runtime-manifest")
                .lifecycle_state("active")
                .affected_scope("manifest"),
        )
    }

    pub fn runtime_adapter_danger_signal_block_summary(
        &self,
        adapter: RuntimeAdapterHint,
    ) -> Option<String> {
        let review = self.runtime_adapter_danger_signal_review(adapter);
        (!review.activation_allowed).then(|| review.summary_line())
    }

    pub fn blocks_runtime_adapter(&self, adapter: RuntimeAdapterHint) -> bool {
        self.adapter_lifecycle_block(adapter).is_some()
            || self.is_adapter_retired(adapter)
            || self
                .runtime_adapter_danger_signal_block_summary(adapter)
                .is_some()
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

    pub fn pre_weight_load_probe(
        &self,
        device_contract: &str,
        runtime_adapter: Option<RuntimeAdapterHint>,
    ) -> RuntimeManifestConformanceProbe {
        RuntimeManifestConformanceProbe {
            metadata: self.runtime_metadata(),
            architecture: self.architecture,
            kv_policy: self.kv_policy,
            quantization: self.quantization,
            runtime_adapter,
            asset_provenance: RuntimeAssetProvenance::from_existing_assets(&self.assets),
            adapter_metadata_sha256: self.adapter_metadata_sha256(),
            quantization_metadata_sha256: self.quantization_metadata_sha256(),
            device_contract_sha256: sha256_text_digest(device_contract),
        }
    }

    pub fn adapter_metadata_sha256(&self) -> String {
        let adapters = self
            .adapter_hints
            .iter()
            .map(|adapter| adapter.as_str())
            .collect::<Vec<_>>()
            .join(",");
        let retired = self
            .retired_adapter_hints
            .iter()
            .map(|adapter| adapter.as_str())
            .collect::<Vec<_>>()
            .join(",");
        let lifecycle = self
            .adapter_lifecycle_records
            .iter()
            .map(RuntimeAdapterLifecycleRecord::summary_line)
            .collect::<Vec<_>>()
            .join("|");

        sha256_text_digest(&format!(
            "adapters={adapters};retired={retired};lifecycle={lifecycle}"
        ))
    }

    pub fn quantization_metadata_sha256(&self) -> String {
        sha256_text_digest(&format!(
            "hot_kv={};cold_kv={};weights={}",
            self.quantization.hot_kv.width(),
            self.quantization.cold_kv.width(),
            self.quantization
                .weights
                .map(|bits| bits.width().to_string())
                .unwrap_or_else(|| "none".to_owned())
        ))
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
            .filter(|adapter| !self.blocks_runtime_adapter(*adapter))
            .find(|adapter| self.adapter_hints.contains(adapter));

        if execution.adapter_hints.is_empty() {
            device_supported.or_else(|| {
                self.adapter_hints
                    .iter()
                    .copied()
                    .find(|adapter| !self.blocks_runtime_adapter(*adapter))
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
            .filter(|observation| !self.blocks_runtime_adapter(observation.adapter))
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
        for record in &self.adapter_lifecycle_records {
            if !self.adapter_hints.contains(&record.adapter) {
                errors.push(format!(
                    "runtime adapter lifecycle record {} is outside manifest adapter hints",
                    record.adapter.as_str()
                ));
            }
            if record.reason_code.trim().is_empty() {
                errors.push(format!(
                    "runtime adapter lifecycle record {} missing reason_code",
                    record.adapter.as_str()
                ));
            }
            if record.source_digest.trim().is_empty() {
                errors.push(format!(
                    "runtime adapter lifecycle record {} missing source_digest",
                    record.adapter.as_str()
                ));
            }
            if record.parent_lineage.trim().is_empty()
                || record.rollback_anchor.trim().is_empty()
                || record.affected_scope.trim().is_empty()
                || record.readmission_gate.trim().is_empty()
            {
                errors.push(format!(
                    "runtime adapter lifecycle record {} missing lineage or gate evidence",
                    record.adapter.as_str()
                ));
            }
            if record.state.blocks_runtime_worker() && !record.operator_approval_required {
                errors.push(format!(
                    "runtime adapter lifecycle record {} must require operator approval before re-admission",
                    record.adapter.as_str()
                ));
            }
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

    pub fn validate_pre_weight_load(
        &self,
        probe: &RuntimeManifestConformanceProbe,
    ) -> RuntimeManifestValidation {
        let mut validation = self.validate_for_production();

        validate_asset_digest(
            "weights",
            self.assets.weights.is_some(),
            self.asset_provenance.weights_sha256.as_deref(),
            probe.asset_provenance.weights_sha256.as_deref(),
            &mut validation.errors,
        );
        validate_asset_digest(
            "tokenizer",
            self.assets.tokenizer.is_some(),
            self.asset_provenance.tokenizer_sha256.as_deref(),
            probe.asset_provenance.tokenizer_sha256.as_deref(),
            &mut validation.errors,
        );
        validate_asset_digest(
            "config",
            self.assets.config.is_some(),
            self.asset_provenance.config_sha256.as_deref(),
            probe.asset_provenance.config_sha256.as_deref(),
            &mut validation.errors,
        );

        let expected_metadata = self.runtime_metadata();
        if probe.metadata.model_id != expected_metadata.model_id {
            validation.errors.push(format!(
                "pre-weight-load ABI model_id mismatch: probe={} manifest={}",
                probe.metadata.model_id, expected_metadata.model_id
            ));
        }
        if probe.metadata.tokenizer != expected_metadata.tokenizer {
            validation.errors.push(format!(
                "pre-weight-load ABI tokenizer mismatch: probe={} manifest={}",
                probe.metadata.tokenizer, expected_metadata.tokenizer
            ));
        }
        if probe.metadata.native_context_window != expected_metadata.native_context_window {
            validation.errors.push(format!(
                "pre-weight-load ABI native_context_window mismatch: probe={} manifest={}",
                probe.metadata.native_context_window, expected_metadata.native_context_window
            ));
        }
        if probe.metadata.embedding_dimensions != expected_metadata.embedding_dimensions {
            validation.errors.push(format!(
                "pre-weight-load ABI embedding_dimensions mismatch: probe={} manifest={}",
                probe.metadata.embedding_dimensions, expected_metadata.embedding_dimensions
            ));
        }
        if probe.architecture != self.architecture {
            validation.errors.push(format!(
                "pre-weight-load ABI architecture mismatch: probe={} manifest={}",
                probe.architecture.summary(),
                self.architecture.summary()
            ));
        }
        if probe.kv_policy != self.kv_policy {
            validation
                .errors
                .push("pre-weight-load KV ABI mismatch between probe and manifest".to_owned());
        }
        if probe.quantization != self.quantization {
            validation.errors.push(
                "pre-weight-load quantization mismatch between probe and manifest".to_owned(),
            );
        }
        if probe.adapter_metadata_sha256 != self.adapter_metadata_sha256() {
            validation
                .errors
                .push("pre-weight-load adapter metadata SHA mismatch".to_owned());
        }
        if probe.quantization_metadata_sha256 != self.quantization_metadata_sha256() {
            validation
                .errors
                .push("pre-weight-load quantization metadata SHA mismatch".to_owned());
        }
        match probe.runtime_adapter {
            Some(adapter)
                if self.adapter_hints.contains(&adapter)
                    && !self.blocks_runtime_adapter(adapter) => {}
            Some(adapter) => validation.errors.push(format!(
                "pre-weight-load device adapter {} is not active in manifest",
                adapter.as_str()
            )),
            None => validation
                .errors
                .push("pre-weight-load device adapter is missing".to_owned()),
        }
        if !probe.device_contract_sha256.starts_with("sha256:") {
            validation
                .errors
                .push("pre-weight-load device contract SHA is missing".to_owned());
        }

        validation
    }
}

fn validate_asset_digest(
    label: &str,
    required: bool,
    expected: Option<&str>,
    observed: Option<&str>,
    errors: &mut Vec<String>,
) {
    if !required {
        return;
    }
    match (expected, observed) {
        (Some(expected), Some(observed))
            if expected.starts_with("sha256:") && observed.starts_with("sha256:") =>
        {
            if expected != observed {
                errors.push(format!("{label} asset SHA mismatch before weight load"));
            }
        }
        (None, _) => errors.push(format!(
            "{label} asset SHA provenance is required before weight load"
        )),
        (_, None) => errors.push(format!("{label} asset SHA probe failed before weight load")),
        _ => errors.push(format!(
            "{label} asset SHA provenance must use sha256 before weight load"
        )),
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
