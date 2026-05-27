use crate::engine::{InferenceRequest, NoironEngine};
use crate::hierarchy::TaskProfile;
use crate::kv_exchange::RuntimeKvBlock;
use crate::runtime::{
    ModelRuntime, RuntimeBackend, RuntimeEmbedding, RuntimeError, RuntimeMetadata, RuntimeRequest,
    RuntimeResponse, RuntimeToken, RuntimeTokenId,
};

#[derive(Debug, Clone)]
pub struct LocalTransformerRuntime {
    metadata: RuntimeMetadata,
    imported_kv_blocks: Vec<RuntimeKvBlock>,
    exported_kv_blocks: Vec<RuntimeKvBlock>,
}

impl Default for LocalTransformerRuntime {
    fn default() -> Self {
        Self::new(8_192, 64)
    }
}

impl LocalTransformerRuntime {
    pub fn new(native_context_window: usize, embedding_dimensions: usize) -> Self {
        Self::with_metadata(RuntimeMetadata::new(
            "noiron-local-transformer",
            "noiron-local-tokenizer",
            native_context_window,
            embedding_dimensions,
        ))
    }

    pub fn with_metadata(mut metadata: RuntimeMetadata) -> Self {
        if metadata.model_id == RuntimeMetadata::default().model_id {
            metadata.model_id = "noiron-local-transformer".to_owned();
        }
        if metadata.tokenizer == RuntimeMetadata::default().tokenizer {
            metadata.tokenizer = "noiron-local-tokenizer".to_owned();
        }
        if metadata.native_context_window == 0 {
            metadata.native_context_window = 8_192;
        }
        if metadata.embedding_dimensions == 0 {
            metadata.embedding_dimensions = 64;
        }
        metadata.supports_kv_import = true;
        metadata.supports_kv_export = true;

        Self {
            metadata,
            imported_kv_blocks: Vec::new(),
            exported_kv_blocks: Vec::new(),
        }
    }

    pub fn imported_kv_blocks(&self) -> &[RuntimeKvBlock] {
        &self.imported_kv_blocks
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
        self.metadata.clone()
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
            self.metadata.embedding_dimensions,
        )))
    }

    fn import_kv(&mut self, blocks: &[RuntimeKvBlock]) -> Result<usize, RuntimeError> {
        self.imported_kv_blocks = blocks.to_vec();
        Ok(self.imported_kv_blocks.len())
    }

    fn export_kv(&mut self) -> Result<Vec<RuntimeKvBlock>, RuntimeError> {
        Ok(self.exported_kv_blocks.clone())
    }

    fn generate(&mut self, request: RuntimeRequest) -> Result<RuntimeResponse, RuntimeError> {
        let tokens = self.tokenize(&request.prompt)?;
        let embedding = self.embed(&tokens)?.values;
        self.exported_kv_blocks = export_prompt_kv(&embedding, &request);

        let transformer_counts = request.transformer_plan.counts();
        let profile_hint = match request.profile {
            TaskProfile::General => "balanced reasoning",
            TaskProfile::Coding => "local-window syntax and interface tracking",
            TaskProfile::Writing => "global continuity and style preservation",
            TaskProfile::LongDocument => "convolutional compression plus global memory recall",
        };
        let answer = format!(
            "Local Transformer runtime result for '{}'. The self-developed runtime used {} prompt tokens, {} imported KV blocks, {} memory hints, and {} experience hints. It followed a layer plan with {} global, {} local-window, and {} convolutional-fusion layers. Profile policy: {profile_hint}. Noiron keeps model weights fixed here while adapting routing thresholds, reinforced KV memory, hierarchy weights, reflection rewards, and reusable experience around the runtime.",
            compact(&request.prompt, 96),
            tokens.len(),
            self.imported_kv_blocks.len(),
            request.memory_hints.len(),
            request.experience_hints.len(),
            transformer_counts.global,
            transformer_counts.local,
            transformer_counts.convolution,
        );
        let mut response = RuntimeResponse::new(answer.clone());
        response.tokens = answer
            .split_whitespace()
            .map(|text| RuntimeToken {
                text: text.to_owned(),
                logprob: Some(-estimated_entropy(text)),
                entropy: Some(estimated_entropy(text)),
            })
            .collect();
        response.trace = vec![
            crate::reflection::ReasoningStep::new(
                "local_tokenizer",
                format!("tokenized {} prompt tokens", tokens.len()),
                0.84,
            ),
            crate::reflection::ReasoningStep::new(
                "local_transformer_plan",
                format!(
                    "used {} global, {} local, {} convolution layers",
                    transformer_counts.global,
                    transformer_counts.local,
                    transformer_counts.convolution
                ),
                0.82,
            ),
            crate::reflection::ReasoningStep::new(
                "local_kv_exchange",
                format!(
                    "imported {} blocks and prepared {} export blocks",
                    self.imported_kv_blocks.len(),
                    self.exported_kv_blocks.len()
                ),
                0.80,
            ),
        ];

        Ok(response)
    }
}

fn local_tokenize(text: &str) -> Vec<String> {
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

    for token in tokens {
        let hash = stable_hash(&token.text);
        for offset in 0..4 {
            let index = ((hash >> (offset * 8)) as usize) % dimensions;
            vector[index] += 1.0 / (offset as f32 + 1.0);
        }
    }

    normalize(&mut vector);
    vector
}

fn export_prompt_kv(embedding: &[f32], request: &RuntimeRequest) -> Vec<RuntimeKvBlock> {
    if embedding.is_empty() {
        return Vec::new();
    }

    let block_count = request
        .transformer_plan
        .layers
        .len()
        .clamp(1, 3)
        .min(request.recursive_schedule.chunk_count().max(1) + 1);
    let midpoint = (embedding.len() / 2).max(1);
    let key = embedding.iter().copied().take(midpoint).collect::<Vec<_>>();
    let value = embedding.iter().copied().skip(midpoint).collect::<Vec<_>>();
    let value = if value.is_empty() { key.clone() } else { value };

    (0..block_count)
        .map(|index| {
            RuntimeKvBlock::new(
                index,
                index % 8,
                index,
                index + 1,
                scaled(&key, 1.0 + index as f32 * 0.03),
                scaled(&value, 1.0 - index as f32 * 0.02),
            )
        })
        .collect()
}

fn estimated_entropy(token: &str) -> f32 {
    let unique_chars = token
        .chars()
        .collect::<std::collections::HashSet<_>>()
        .len();
    (0.12 + unique_chars as f32 / 32.0).clamp(0.05, 1.25)
}

fn scaled(values: &[f32], scale: f32) -> Vec<f32> {
    values.iter().map(|value| value * scale).collect()
}

fn normalize(values: &mut [f32]) {
    let norm = values.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in values {
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

    #[test]
    fn local_runtime_exposes_tokens_embeddings_and_metadata() {
        let runtime = LocalTransformerRuntime::new(16_384, 32);

        let metadata = runtime.metadata();
        let tokens = runtime.tokenize("Rust Noiron local runtime").unwrap();
        let embedding = runtime.embed(&tokens).unwrap();

        assert_eq!(metadata.model_id, "noiron-local-transformer");
        assert_eq!(metadata.native_context_window, 16_384);
        assert_eq!(metadata.embedding_dimensions, 32);
        assert!(metadata.supports_kv_import);
        assert!(metadata.supports_kv_export);
        assert_eq!(tokens.len(), 4);
        assert_eq!(embedding.dimensions, 32);
        assert!(embedding.values.iter().any(|value| *value > 0.0));
    }

    #[test]
    fn local_runtime_generates_tokens_and_exports_kv() {
        let mut runtime = LocalTransformerRuntime::default();
        let request = RuntimeRequest {
            prompt: "Build local Noiron runtime".to_owned(),
            profile: TaskProfile::Coding,
            runtime_metadata: runtime.metadata(),
            memory_hints: vec!["hot memory".to_owned()],
            infini_memory_hints: Vec::new(),
            experience_hints: vec!["reuse prior route".to_owned()],
            route_budget: crate::router::RouteBudget {
                threshold: 0.5,
                attention_tokens: 2,
                fast_tokens: 1,
                attention_fraction: 0.66,
            },
            hierarchy: crate::hierarchy::HierarchyWeights::new(0.2, 0.6, 0.2),
            transformer_plan: crate::transformer::TransformerPlanner::new(6, 128).plan(
                TaskProfile::Coding,
                crate::hierarchy::HierarchyWeights::new(0.2, 0.6, 0.2),
                crate::router::RouteBudget {
                    threshold: 0.5,
                    attention_tokens: 2,
                    fast_tokens: 1,
                    attention_fraction: 0.66,
                },
            ),
            recursive_schedule: crate::recursive_scheduler::RecursiveSchedule::default(),
            hardware_plan: crate::hardware::HardwarePlan::default(),
            max_tokens: 64,
        };

        let response = runtime.generate(request).unwrap();
        let exported = runtime.export_kv().unwrap();

        assert!(response.answer.contains("Local Transformer runtime"));
        assert!(!response.tokens.is_empty());
        assert!(!exported.is_empty());
    }

    #[test]
    fn engine_can_use_local_runtime_end_to_end() {
        let outcome = LocalTransformerRuntime::run_once(
            "Build a Rust Noiron local Transformer runtime with KV exchange",
            TaskProfile::Coding,
        );

        assert!(outcome.answer.contains("Local Transformer runtime"));
        assert!(outcome.exported_runtime_kv_blocks > 0);
        assert!(!outcome.stored_runtime_kv_memory_ids.is_empty());
    }
}
