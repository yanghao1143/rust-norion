use std::fs;
use std::path::{Path, PathBuf};

use crate::runtime::RuntimeError;
use crate::runtime_manifest::RuntimeManifest;

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
    pub(super) fn from_manifest(manifest: &RuntimeManifest) -> Result<Self, RuntimeError> {
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
