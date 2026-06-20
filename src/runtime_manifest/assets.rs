use std::path::PathBuf;

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
