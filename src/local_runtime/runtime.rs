use crate::engine::{InferenceRequest, NoironEngine};
use crate::hierarchy::TaskProfile;
use crate::kv_exchange::RuntimeKvBlock;
use crate::runtime::{
    ModelRuntime, RuntimeBackend, RuntimeEmbedding, RuntimeError, RuntimeMetadata, RuntimeRequest,
    RuntimeResponse, RuntimeTokenId,
};
use crate::runtime_manifest::{RuntimeManifest, TransformerRuntimeArchitecture};

use super::config::{local_manifest, manifest_from_metadata, normalize_local_manifest};
use super::forward::{export_forward_kv, run_transformer_forward};
use super::response::build_runtime_response;
use super::session::LocalRuntimeSession;
use super::tokenizer::{embed_tokens, local_tokenize, stable_hash};

#[derive(Debug, Clone)]
pub struct LocalTransformerRuntime {
    manifest: RuntimeManifest,
    session: LocalRuntimeSession,
}

impl Default for LocalTransformerRuntime {
    fn default() -> Self {
        Self::new(8_192, 64)
    }
}

impl LocalTransformerRuntime {
    pub fn new(native_context_window: usize, embedding_dimensions: usize) -> Self {
        Self::with_manifest(local_manifest(native_context_window, embedding_dimensions))
    }

    pub fn with_metadata(metadata: RuntimeMetadata) -> Self {
        Self::with_manifest(manifest_from_metadata(metadata))
    }

    pub fn with_manifest(manifest: RuntimeManifest) -> Self {
        Self {
            manifest: normalize_local_manifest(manifest),
            session: LocalRuntimeSession::default(),
        }
    }

    pub fn manifest(&self) -> &RuntimeManifest {
        &self.manifest
    }

    pub fn imported_kv_blocks(&self) -> &[RuntimeKvBlock] {
        self.session.imported_kv_blocks()
    }

    pub fn run_once(
        prompt: impl Into<String>,
        profile: TaskProfile,
    ) -> crate::engine::InferenceOutcome {
        let mut engine = NoironEngine::new();
        let mut backend = RuntimeBackend::new(Self::default());
        engine.infer(InferenceRequest::new(prompt, profile), &mut backend)
    }
}

impl ModelRuntime for LocalTransformerRuntime {
    fn metadata(&self) -> RuntimeMetadata {
        self.manifest.runtime_metadata()
    }

    fn architecture(&self) -> TransformerRuntimeArchitecture {
        self.manifest.architecture
    }

    fn tokenize(&self, prompt: &str) -> Result<Vec<RuntimeTokenId>, RuntimeError> {
        Ok(local_tokenize(prompt)
            .into_iter()
            .map(|text| {
                let id = stable_hash(&text) % 1_000_000;
                RuntimeTokenId::new(id as u32, text)
            })
            .collect())
    }

    fn embed(&self, tokens: &[RuntimeTokenId]) -> Result<RuntimeEmbedding, RuntimeError> {
        Ok(RuntimeEmbedding::new(embed_tokens(
            tokens,
            self.manifest.metadata.embedding_dimensions,
        )))
    }

    fn import_kv(&mut self, blocks: &[RuntimeKvBlock]) -> Result<usize, RuntimeError> {
        Ok(self
            .session
            .import_kv(blocks, self.manifest.kv_policy.max_import_blocks))
    }

    fn export_kv(&mut self) -> Result<Vec<RuntimeKvBlock>, RuntimeError> {
        Ok(self
            .session
            .export_kv(self.manifest.kv_policy.max_export_blocks))
    }

    fn generate(&mut self, request: RuntimeRequest) -> Result<RuntimeResponse, RuntimeError> {
        let tokens = self.tokenize(&request.prompt)?;
        let embedding = self.embed(&tokens)?.values;
        let forward = run_transformer_forward(
            &embedding,
            self.session.imported_kv_blocks(),
            &request,
            self.manifest.architecture,
        );
        self.session
            .replace_exported_kv(export_forward_kv(&forward, &request));

        Ok(build_runtime_response(
            &request,
            &self.manifest,
            &tokens,
            self.session.imported_kv_blocks(),
            self.session.exported_kv_blocks(),
            &forward,
        ))
    }
}
