use crate::kv_exchange::RuntimeKvBlock;
use crate::runtime_manifest::{
    TransformerRuntimeArchitecture, default_transformer_runtime_architecture,
};

use super::{
    RuntimeEmbedding, RuntimeError, RuntimeMetadata, RuntimeRequest, RuntimeResponse, RuntimeToken,
    RuntimeTokenId,
};

pub trait ModelRuntime {
    fn metadata(&self) -> RuntimeMetadata {
        RuntimeMetadata::default()
    }

    fn architecture(&self) -> TransformerRuntimeArchitecture {
        let metadata = self.metadata();
        default_transformer_runtime_architecture(
            metadata.native_context_window,
            metadata.embedding_dimensions,
        )
    }

    fn tokenize(&self, prompt: &str) -> Result<Vec<RuntimeTokenId>, RuntimeError> {
        Ok(prompt
            .split_whitespace()
            .enumerate()
            .map(|(index, text)| RuntimeTokenId::new(index as u32, text))
            .collect())
    }

    fn embed(&self, _tokens: &[RuntimeTokenId]) -> Result<RuntimeEmbedding, RuntimeError> {
        Ok(RuntimeEmbedding::empty())
    }

    fn embed_text(&self, text: &str) -> Result<RuntimeEmbedding, RuntimeError> {
        let tokens = self.tokenize(text)?;
        self.embed(&tokens)
    }

    fn import_kv(&mut self, _blocks: &[RuntimeKvBlock]) -> Result<usize, RuntimeError> {
        Ok(0)
    }

    fn export_kv(&mut self) -> Result<Vec<RuntimeKvBlock>, RuntimeError> {
        Ok(Vec::new())
    }

    fn supports_endpoint_override(&self) -> bool {
        false
    }

    fn clone_for_endpoint_override(&self, _base_url: &str) -> Result<Option<Self>, RuntimeError>
    where
        Self: Sized,
    {
        Ok(None)
    }

    fn generate(&mut self, request: RuntimeRequest) -> Result<RuntimeResponse, RuntimeError>;

    fn generate_stream(
        &mut self,
        request: RuntimeRequest,
        on_token: &mut dyn FnMut(&RuntimeToken) -> Result<(), RuntimeError>,
    ) -> Result<RuntimeResponse, RuntimeError> {
        let response = self.generate(request)?;
        for token in &response.tokens {
            on_token(token)?;
        }
        Ok(response)
    }
}
