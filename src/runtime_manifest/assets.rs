use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RuntimeAssetProvenance {
    pub weights_sha256: Option<String>,
    pub tokenizer_sha256: Option<String>,
    pub config_sha256: Option<String>,
}

impl RuntimeAssetProvenance {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_existing_assets(assets: &RuntimeAssetPaths) -> Self {
        Self {
            weights_sha256: digest_existing_asset(assets.weights.as_deref()),
            tokenizer_sha256: digest_existing_asset(assets.tokenizer.as_deref()),
            config_sha256: digest_existing_asset(assets.config.as_deref()),
        }
    }

    pub fn with_weights_sha256(mut self, digest: impl Into<String>) -> Self {
        self.weights_sha256 = Some(digest.into());
        self
    }

    pub fn with_tokenizer_sha256(mut self, digest: impl Into<String>) -> Self {
        self.tokenizer_sha256 = Some(digest.into());
        self
    }

    pub fn with_config_sha256(mut self, digest: impl Into<String>) -> Self {
        self.config_sha256 = Some(digest.into());
        self
    }
}

pub fn sha256_file_digest(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8 * 1024];

    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    let digest = hasher.finalize();
    Ok(format_sha256(&digest))
}

pub fn sha256_text_digest(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    let digest = hasher.finalize();
    format_sha256(&digest)
}

fn digest_existing_asset(path: Option<&Path>) -> Option<String> {
    path.and_then(|path| {
        if path.is_file() {
            sha256_file_digest(path).ok()
        } else {
            None
        }
    })
}

fn format_sha256(bytes: &[u8]) -> String {
    let mut out = String::with_capacity("sha256:".len() + bytes.len() * 2);
    out.push_str("sha256:");
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}
