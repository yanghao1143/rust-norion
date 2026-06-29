use std::path::Path;

use crate::hardware::RuntimeAdapterHint;
use crate::runtime::RuntimeMetadata;

use super::architecture::TransformerRuntimeArchitecture;
use super::assets::{RuntimeAssetProvenance, sha256_text_digest};
use super::kv_policy::RuntimeKvPolicy;
use super::quantization::RuntimeQuantizationPolicy;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeManifestValidation {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl RuntimeManifestValidation {
    pub fn passed(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn digest_only_summary(&self, probe: &RuntimeManifestConformanceProbe) -> String {
        let seed = format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}",
            self.passed(),
            self.errors.join("|"),
            self.warnings.join("|"),
            probe
                .asset_provenance
                .weights_sha256
                .as_deref()
                .unwrap_or("missing"),
            probe
                .asset_provenance
                .tokenizer_sha256
                .as_deref()
                .unwrap_or("missing"),
            probe
                .asset_provenance
                .config_sha256
                .as_deref()
                .unwrap_or("none"),
            probe.adapter_metadata_sha256,
            probe.quantization_metadata_sha256,
            probe.device_contract_sha256,
        );
        format!(
            "runtime_manifest_pre_weight_load passed={} errors={} warnings={} evidence_digest={} asset_digests={} adapter_metadata={} quantization_metadata={} device_contract={}",
            self.passed(),
            self.errors.len(),
            self.warnings.len(),
            sha256_text_digest(&seed),
            probe.asset_digest_count(),
            probe.adapter_metadata_sha256,
            probe.quantization_metadata_sha256,
            probe.device_contract_sha256
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeManifestConformanceProbe {
    pub metadata: RuntimeMetadata,
    pub architecture: TransformerRuntimeArchitecture,
    pub kv_policy: RuntimeKvPolicy,
    pub quantization: RuntimeQuantizationPolicy,
    pub runtime_adapter: Option<RuntimeAdapterHint>,
    pub asset_provenance: RuntimeAssetProvenance,
    pub adapter_metadata_sha256: String,
    pub quantization_metadata_sha256: String,
    pub device_contract_sha256: String,
}

impl RuntimeManifestConformanceProbe {
    pub fn asset_digest_count(&self) -> usize {
        [
            self.asset_provenance.weights_sha256.as_ref(),
            self.asset_provenance.tokenizer_sha256.as_ref(),
            self.asset_provenance.config_sha256.as_ref(),
        ]
        .into_iter()
        .filter(|digest| digest.is_some())
        .count()
    }
}

pub(super) fn validate_required_asset_file(
    label: &str,
    path: Option<&Path>,
    errors: &mut Vec<String>,
) {
    let Some(path) = path else {
        errors.push(format!(
            "{label} asset path is required for production runtimes"
        ));
        return;
    };

    validate_optional_asset_file(label, path, errors);
}

pub(super) fn validate_optional_asset_file(label: &str, path: &Path, errors: &mut Vec<String>) {
    if path.as_os_str().is_empty() {
        errors.push(format!("{label} asset path must not be empty"));
        return;
    }
    if !path.exists() {
        errors.push(format!(
            "{} asset path does not exist: {}",
            label,
            path.display()
        ));
        return;
    }
    if !path.is_file() {
        errors.push(format!(
            "{} asset path is not a file: {}",
            label,
            path.display()
        ));
    }
}
