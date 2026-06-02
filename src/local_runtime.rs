use crate::engine::{InferenceRequest, NoironEngine};
use crate::hierarchy::TaskProfile;
use crate::kv_exchange::RuntimeKvBlock;
use crate::reflection::RuntimeDiagnostics;
use crate::runtime::{
    ModelRuntime, RuntimeBackend, RuntimeEmbedding, RuntimeError, RuntimeMetadata, RuntimeRequest,
    RuntimeResponse, RuntimeToken, RuntimeTokenId,
};
use crate::runtime_manifest::{RuntimeManifest, TransformerRuntimeArchitecture};
use crate::transformer::{
    AttentionKind, TransformerLayerPlan, TransformerPlanCounts, TransformerPlanner,
};

#[derive(Debug, Clone)]
pub struct LocalTransformerRuntime {
    manifest: RuntimeManifest,
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
        Self::with_manifest(RuntimeManifest::self_developed(
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
        Self::with_manifest(
            RuntimeManifest::from_metadata(metadata)
                .with_kv_policy(crate::runtime_manifest::RuntimeKvPolicy::import_export()),
        )
    }

    pub fn with_manifest(mut manifest: RuntimeManifest) -> Self {
        if manifest.metadata.model_id == RuntimeMetadata::default().model_id {
            manifest.metadata.model_id = "noiron-local-transformer".to_owned();
        }
        if manifest.metadata.tokenizer == RuntimeMetadata::default().tokenizer {
            manifest.metadata.tokenizer = "noiron-local-tokenizer".to_owned();
        }
        if manifest.metadata.native_context_window == 0 {
            manifest.metadata.native_context_window = 8_192;
        }
        if manifest.metadata.embedding_dimensions == 0 {
            manifest.metadata.embedding_dimensions = 64;
        }
        manifest.kv_policy.import_enabled = true;
        manifest.kv_policy.export_enabled = true;
        manifest.kv_policy.max_import_blocks = manifest.kv_policy.max_import_blocks.max(1);
        manifest.kv_policy.max_export_blocks = manifest.kv_policy.max_export_blocks.max(1);
        manifest.metadata.supports_kv_import = manifest.kv_policy.import_enabled;
        manifest.metadata.supports_kv_export = manifest.kv_policy.export_enabled;
        manifest.architecture = normalize_manifest_architecture(&manifest);
        Self {
            manifest,
            imported_kv_blocks: Vec::new(),
            exported_kv_blocks: Vec::new(),
        }
    }

    pub fn manifest(&self) -> &RuntimeManifest {
        &self.manifest
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
        self.imported_kv_blocks = blocks
            .iter()
            .take(self.manifest.kv_policy.max_import_blocks)
            .cloned()
            .collect();
        Ok(self.imported_kv_blocks.len())
    }

    fn export_kv(&mut self) -> Result<Vec<RuntimeKvBlock>, RuntimeError> {
        Ok(self
            .exported_kv_blocks
            .iter()
            .take(self.manifest.kv_policy.max_export_blocks)
            .cloned()
            .collect())
    }

    fn generate(&mut self, request: RuntimeRequest) -> Result<RuntimeResponse, RuntimeError> {
        let tokens = self.tokenize(&request.prompt)?;
        let embedding = self.embed(&tokens)?.values;
        let forward = run_transformer_forward(
            &embedding,
            &self.imported_kv_blocks,
            &request,
            self.manifest.architecture,
        );
        self.exported_kv_blocks = export_forward_kv(&forward, &request);

        let transformer_counts = count_forward_layers(&forward.layer_summaries);
        let profile_hint = match request.profile {
            TaskProfile::General => "balanced reasoning",
            TaskProfile::Coding => "local-window syntax and interface tracking",
            TaskProfile::Writing => "global continuity and style preservation",
            TaskProfile::LongDocument => "convolutional compression plus global memory recall",
        };
        let selected_adapter = self
            .manifest
            .preferred_adapter_with_observations(
                &request.hardware_plan.execution,
                &request.runtime_adapter_observations,
            )
            .map(|adapter| adapter.as_str().to_owned());
        let answer = format!(
            "Local Transformer runtime result for '{}'. The self-developed runtime used manifest {}, {} prompt tokens, {} imported KV blocks, {} memory hints, and {} experience hints. It executed {} deterministic Transformer layers with state energy {:.3} and KV influence {:.3}: {} global, {} local-window, and {} convolutional-fusion layers. Hardware execution targeted {} with {} memory and {} fallback. Profile policy: {profile_hint}. Noiron keeps model weights fixed here while adapting routing thresholds, reinforced KV memory, hierarchy weights, reflection rewards, and reusable experience around the runtime.",
            compact(&request.prompt, 96),
            self.manifest.metadata.model_id,
            tokens.len(),
            self.imported_kv_blocks.len(),
            request.memory_hints.len(),
            request.experience_hints.len(),
            forward.layer_summaries.len(),
            forward.energy,
            forward.kv_influence,
            transformer_counts.global,
            transformer_counts.local,
            transformer_counts.convolution,
            request.hardware_plan.execution.primary_lane.as_str(),
            request.hardware_plan.execution.memory_mode.as_str(),
            request.hardware_plan.execution.fallback_lane.as_str(),
        );
        let mut response =
            RuntimeResponse::new(answer.clone()).with_diagnostics(RuntimeDiagnostics {
                model_id: Some(self.manifest.metadata.model_id.clone()),
                selected_adapter: selected_adapter.clone(),
                device_profile: Some(request.hardware_plan.device.as_str().to_owned()),
                primary_lane: Some(
                    request
                        .hardware_plan
                        .execution
                        .primary_lane
                        .as_str()
                        .to_owned(),
                ),
                fallback_lane: Some(
                    request
                        .hardware_plan
                        .execution
                        .fallback_lane
                        .as_str()
                        .to_owned(),
                ),
                memory_mode: Some(
                    request
                        .hardware_plan
                        .execution
                        .memory_mode
                        .as_str()
                        .to_owned(),
                ),
                device_execution_source: Some(
                    RuntimeDiagnostics::runtime_reported_device_execution_source().to_owned(),
                ),
                layer_count: forward.layer_summaries.len(),
                global_layers: transformer_counts.global,
                local_window_layers: transformer_counts.local,
                convolutional_fusion_layers: transformer_counts.convolution,
                hidden_size: self.manifest.architecture.hidden_size,
                local_window_tokens: self.manifest.architecture.local_window_tokens,
                forward_energy: Some(forward.energy),
                kv_influence: Some(forward.kv_influence),
                imported_kv_blocks: self.imported_kv_blocks.len(),
                exported_kv_blocks: self.exported_kv_blocks.len(),
                hot_kv_precision_bits: Some(request.hardware_plan.execution.hot_kv_precision_bits),
                cold_kv_precision_bits: Some(
                    request.hardware_plan.execution.cold_kv_precision_bits,
                ),
            });
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
                "local_transformer_forward",
                format!(
                    "executed {} layers with hidden size {} and energy {:.3} and KV influence {:.3}",
                    forward.layer_summaries.len(),
                    self.manifest.architecture.hidden_size,
                    forward.energy,
                    forward.kv_influence
                ),
                0.83,
            ),
            crate::reflection::ReasoningStep::new(
                "local_transformer_plan",
                format!(
                    "planned {} global, {} local, {} convolution layers",
                    transformer_counts.global,
                    transformer_counts.local,
                    transformer_counts.convolution
                ),
                0.82,
            ),
            crate::reflection::ReasoningStep::new(
                "local_device_execution",
                format!(
                    "primary {} fallback {} memory {} adapter {:?}",
                    request.hardware_plan.execution.primary_lane.as_str(),
                    request.hardware_plan.execution.fallback_lane.as_str(),
                    request.hardware_plan.execution.memory_mode.as_str(),
                    selected_adapter
                ),
                0.80,
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

fn normalize_manifest_architecture(manifest: &RuntimeManifest) -> TransformerRuntimeArchitecture {
    let embedding_dimensions = manifest.metadata.embedding_dimensions.max(1);
    let native_window = manifest.metadata.native_context_window.max(1);
    let mut architecture = manifest.architecture;
    if architecture.layer_count == 0 {
        architecture.layer_count = 24;
    }
    if architecture.hidden_size == 0 {
        architecture.hidden_size = embedding_dimensions;
    }
    if architecture.attention_heads == 0 {
        architecture.attention_heads = choose_head_count(architecture.hidden_size);
    }
    if architecture.kv_heads == 0 {
        architecture.kv_heads = architecture.attention_heads;
    }
    architecture.kv_heads = architecture.kv_heads.min(architecture.attention_heads);
    if architecture.local_window_tokens == 0 {
        architecture.local_window_tokens = native_window.min(4_096);
    }
    architecture.local_window_tokens = architecture.local_window_tokens.min(native_window);
    architecture
}

fn choose_head_count(hidden_size: usize) -> usize {
    [16, 12, 8, 6, 4, 2]
        .into_iter()
        .find(|heads| hidden_size % heads == 0)
        .unwrap_or(1)
}

#[derive(Debug, Clone)]
struct LocalForwardState {
    vector: Vec<f32>,
    layer_summaries: Vec<LocalLayerSummary>,
    energy: f32,
    kv_influence: f32,
}

#[derive(Debug, Clone)]
struct LocalLayerSummary {
    layer_index: usize,
    attention: AttentionKind,
    window_size: usize,
    compute_fraction: f32,
    activation: f32,
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

fn run_transformer_forward(
    embedding: &[f32],
    imported_kv_blocks: &[RuntimeKvBlock],
    request: &RuntimeRequest,
    architecture: TransformerRuntimeArchitecture,
) -> LocalForwardState {
    let mut vector = embedding.to_vec();
    if vector.is_empty() {
        vector.push(0.0);
    }

    let layers = runtime_layers_for_architecture(request, architecture);
    let mut layer_summaries = Vec::with_capacity(layers.len());
    let mut kv_influence = 0.0;

    for layer in &layers {
        let influence = apply_imported_kv(&mut vector, imported_kv_blocks, layer);
        kv_influence += influence;
        apply_transformer_layer(&mut vector, layer, request.route_budget.attention_fraction);
        layer_summaries.push(LocalLayerSummary {
            layer_index: layer.layer_index,
            attention: layer.attention,
            window_size: layer.window_size,
            compute_fraction: layer.compute_fraction,
            activation: mean_abs(&vector),
        });
    }

    if layer_summaries.is_empty() {
        normalize(&mut vector);
    }

    LocalForwardState {
        energy: mean_abs(&vector),
        vector,
        layer_summaries,
        kv_influence,
    }
}

fn runtime_layers_for_architecture(
    request: &RuntimeRequest,
    architecture: TransformerRuntimeArchitecture,
) -> Vec<TransformerLayerPlan> {
    let layer_count = architecture.layer_count.max(1);
    let local_window = architecture.local_window_tokens.max(16);
    let native_window = request
        .runtime_metadata
        .native_context_window
        .max(local_window)
        .max(16);

    let mut plan = if request.transformer_plan.layers.len() == layer_count {
        request.transformer_plan.clone()
    } else {
        TransformerPlanner::new(layer_count, local_window).plan(
            request.profile,
            request.hierarchy,
            request.route_budget,
        )
    };

    for (index, layer) in plan.layers.iter_mut().enumerate() {
        layer.layer_index = index;
        layer.window_size = match layer.attention {
            AttentionKind::Global => layer.window_size.clamp(local_window, native_window),
            AttentionKind::LocalWindow | AttentionKind::ConvolutionalFusion => {
                layer.window_size.clamp(16, local_window)
            }
        };
    }

    plan.layers
}

fn count_forward_layers(layers: &[LocalLayerSummary]) -> TransformerPlanCounts {
    let mut counts = TransformerPlanCounts::default();
    for layer in layers {
        match layer.attention {
            AttentionKind::Global => counts.global += 1,
            AttentionKind::LocalWindow => counts.local += 1,
            AttentionKind::ConvolutionalFusion => counts.convolution += 1,
        }
    }
    counts
}

fn apply_imported_kv(
    vector: &mut [f32],
    imported_kv_blocks: &[RuntimeKvBlock],
    layer: &TransformerLayerPlan,
) -> f32 {
    if vector.is_empty() || imported_kv_blocks.is_empty() {
        return 0.0;
    }

    let mut applied = 0.0;
    let selected = imported_kv_blocks
        .iter()
        .filter(|block| {
            block.layer == layer.layer_index || block.layer % 4 == layer.layer_index % 4
        })
        .take(4);

    for block in selected {
        let scale = match layer.attention {
            AttentionKind::Global => 0.030,
            AttentionKind::LocalWindow => 0.018,
            AttentionKind::ConvolutionalFusion => 0.012,
        } * layer.compute_fraction.clamp(0.1, 1.0);
        for (offset, value) in block.vector().iter().enumerate() {
            let index = offset % vector.len();
            vector[index] += value * scale;
            applied += value.abs() * scale;
        }
    }

    applied
}

fn apply_transformer_layer(
    vector: &mut [f32],
    layer: &TransformerLayerPlan,
    attention_fraction: f32,
) {
    if vector.is_empty() {
        return;
    }

    match layer.attention {
        AttentionKind::Global => apply_global_layer(vector, layer, attention_fraction),
        AttentionKind::LocalWindow => apply_local_layer(vector, layer),
        AttentionKind::ConvolutionalFusion => apply_convolution_layer(vector, layer),
    }
    normalize(vector);
}

fn apply_global_layer(vector: &mut [f32], layer: &TransformerLayerPlan, attention_fraction: f32) {
    let mean = vector.iter().sum::<f32>() / vector.len() as f32;
    let gain = 0.04 + layer.compute_fraction * 0.05 + attention_fraction.clamp(0.0, 1.0) * 0.02;
    for (index, value) in vector.iter_mut().enumerate() {
        let positional = ((index + layer.layer_index + 1) as f32).sin() * 0.003;
        *value = *value * (1.0 - gain) + mean * gain + positional;
    }
}

fn apply_local_layer(vector: &mut [f32], layer: &TransformerLayerPlan) {
    let previous = vector.to_vec();
    let radius = ((layer.window_size / 64).max(1)).min(previous.len().saturating_sub(1).max(1));
    let gain = 0.10 + layer.compute_fraction * 0.08;
    for index in 0..vector.len() {
        let left = previous[index.saturating_sub(radius)];
        let right = previous[(index + radius).min(previous.len() - 1)];
        let local = (left + previous[index] + right) / 3.0;
        vector[index] = previous[index] * (1.0 - gain) + local * gain;
    }
}

fn apply_convolution_layer(vector: &mut [f32], layer: &TransformerLayerPlan) {
    let previous = vector.to_vec();
    let gain = 0.12 + layer.compute_fraction * 0.12;
    for index in 0..vector.len() {
        let prev = previous[index.saturating_sub(1)];
        let center = previous[index];
        let next = previous[(index + 1).min(previous.len() - 1)];
        let fused = prev * 0.25 + center * 0.50 + next * 0.25;
        let phase = ((layer.layer_index + index + 1) as f32).cos() * 0.002;
        vector[index] = center * (1.0 - gain) + fused * gain + phase;
    }
}

fn export_forward_kv(forward: &LocalForwardState, request: &RuntimeRequest) -> Vec<RuntimeKvBlock> {
    if forward.vector.is_empty() {
        return Vec::new();
    }

    let layer_count = request.runtime_architecture.layer_count.max(1);
    let kv_heads = request.runtime_architecture.kv_heads.max(1);
    let block_count = forward
        .layer_summaries
        .len()
        .clamp(1, 4)
        .min(request.recursive_schedule.chunk_count().max(1) + 2);
    let midpoint = (forward.vector.len() / 2).max(1);
    let key = forward
        .vector
        .iter()
        .copied()
        .take(midpoint)
        .collect::<Vec<_>>();
    let value = forward
        .vector
        .iter()
        .copied()
        .skip(midpoint)
        .collect::<Vec<_>>();
    let value = if value.is_empty() { key.clone() } else { value };
    let paired_len = key.len().min(value.len()).max(1);
    let key = key.into_iter().take(paired_len).collect::<Vec<_>>();
    let value = value.into_iter().take(paired_len).collect::<Vec<_>>();

    (0..block_count)
        .map(|index| {
            let summary = forward.layer_summaries.get(index);
            let layer = summary.map(|summary| summary.layer_index).unwrap_or(index) % layer_count;
            let head = summary
                .map(|summary| {
                    let attention_offset = match summary.attention {
                        AttentionKind::Global => 0,
                        AttentionKind::LocalWindow => 2,
                        AttentionKind::ConvolutionalFusion => 4,
                    };
                    (attention_offset + summary.window_size + index) % kv_heads
                })
                .unwrap_or(index % kv_heads);
            let compute_scale = summary
                .map(|summary| summary.compute_fraction + summary.activation)
                .unwrap_or(1.0)
                .clamp(0.25, 1.50);
            RuntimeKvBlock::new(
                layer,
                head,
                index,
                index + 1,
                scaled(&key, compute_scale + index as f32 * 0.02),
                scaled(&value, compute_scale - index as f32 * 0.015),
            )
        })
        .collect()
}

fn mean_abs(values: &[f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().map(|value| value.abs()).sum::<f32>() / values.len() as f32
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
        let manifest = runtime.manifest();
        let tokens = runtime.tokenize("Rust Noiron local runtime").unwrap();
        let embedding = runtime.embed(&tokens).unwrap();

        assert_eq!(metadata.model_id, "noiron-local-transformer");
        assert_eq!(metadata.native_context_window, 16_384);
        assert_eq!(metadata.embedding_dimensions, 32);
        assert!(metadata.supports_kv_import);
        assert!(metadata.supports_kv_export);
        assert!(manifest.validate().passed());
        assert!(manifest.supports_device(crate::hardware::DeviceClass::CpuOnly));
        assert_eq!(tokens.len(), 4);
        assert_eq!(embedding.dimensions, 32);
        assert!(embedding.values.iter().any(|value| *value > 0.0));
    }

    #[test]
    fn local_runtime_can_be_configured_from_manifest() {
        let manifest = RuntimeManifest::self_developed(
            "noiron-v2-transformer",
            "noiron-v2-tokenizer",
            131_072,
            48,
        )
        .with_architecture(TransformerRuntimeArchitecture::new(12, 48, 6, 3, 16_384))
        .with_kv_policy(crate::runtime_manifest::RuntimeKvPolicy {
            import_enabled: true,
            export_enabled: true,
            max_import_blocks: 1,
            max_export_blocks: 2,
        })
        .with_supported_devices(vec![
            crate::hardware::DeviceClass::CpuOnly,
            crate::hardware::DeviceClass::UnifiedMemory,
            crate::hardware::DeviceClass::Mobile,
        ]);
        let mut runtime = LocalTransformerRuntime::with_manifest(manifest);

        let imported = runtime
            .import_kv(&[
                RuntimeKvBlock::new(0, 0, 0, 1, vec![0.1], vec![0.2]),
                RuntimeKvBlock::new(1, 0, 1, 2, vec![0.3], vec![0.4]),
            ])
            .unwrap();
        let metadata = runtime.metadata();

        assert_eq!(imported, 1);
        assert_eq!(metadata.model_id, "noiron-v2-transformer");
        assert_eq!(metadata.native_context_window, 131_072);
        assert_eq!(runtime.manifest().architecture.layer_count, 12);
        assert_eq!(runtime.manifest().architecture.local_window_tokens, 16_384);
        assert!(
            runtime
                .manifest()
                .supports_device(crate::hardware::DeviceClass::Mobile)
        );
        assert!(
            !runtime
                .manifest()
                .supports_device(crate::hardware::DeviceClass::Server)
        );

        let request = RuntimeRequest {
            prompt: "manifest configured runtime".to_owned(),
            profile: TaskProfile::Coding,
            runtime_metadata: metadata,
            runtime_architecture: runtime.architecture(),
            memory_hints: Vec::new(),
            infini_memory_hints: Vec::new(),
            experience_hints: Vec::new(),
            runtime_adapter_observations: Vec::new(),
            toolsmith_plan: crate::toolsmith::ToolsmithPlan::default(),
            agent_team_plan: crate::agent_team::AgentTeamPlan::default(),
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
            imported_kv_blocks: Vec::new(),
            max_tokens: 64,
        };
        let response = runtime.generate(request).unwrap();
        let exported = runtime.export_kv().unwrap();

        assert!(response.answer.contains("manifest noiron-v2-transformer"));
        assert_eq!(
            response.diagnostics.model_id.as_deref(),
            Some("noiron-v2-transformer")
        );
        assert_eq!(response.diagnostics.layer_count, 12);
        assert_eq!(response.diagnostics.hidden_size, 48);
        assert_eq!(response.diagnostics.local_window_tokens, 16_384);
        assert!(response.diagnostics.forward_energy.unwrap() > 0.0);
        assert_eq!(response.diagnostics.hot_kv_precision_bits, Some(8));
        assert_eq!(response.diagnostics.cold_kv_precision_bits, Some(4));
        assert!(response.diagnostics.has_valid_kv_precision_signal());
        assert!(response.diagnostics.has_forward_signal());
        assert!(exported.len() <= 2);
    }

    #[test]
    fn local_runtime_generates_tokens_and_exports_kv() {
        let mut runtime = LocalTransformerRuntime::default();
        let request = RuntimeRequest {
            prompt: "Build local Noiron runtime".to_owned(),
            profile: TaskProfile::Coding,
            runtime_metadata: runtime.metadata(),
            runtime_architecture: runtime.architecture(),
            memory_hints: vec!["hot memory".to_owned()],
            infini_memory_hints: Vec::new(),
            experience_hints: vec!["reuse prior route".to_owned()],
            runtime_adapter_observations: Vec::new(),
            toolsmith_plan: crate::toolsmith::ToolsmithPlan::default(),
            agent_team_plan: crate::agent_team::AgentTeamPlan::default(),
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
            imported_kv_blocks: Vec::new(),
            max_tokens: 64,
        };

        let response = runtime.generate(request).unwrap();
        let exported = runtime.export_kv().unwrap();

        assert!(response.answer.contains("Local Transformer runtime"));
        assert!(response.answer.contains("deterministic Transformer layers"));
        assert!(!response.tokens.is_empty());
        assert!(!exported.is_empty());
        assert!(
            response
                .trace
                .iter()
                .any(|step| step.label == "local_transformer_forward")
        );
    }

    #[test]
    fn local_runtime_uses_observed_adapter_when_device_allows_it() {
        let mut runtime = LocalTransformerRuntime::default();
        let mut hardware_plan = crate::hardware::HardwarePlan::default();
        hardware_plan.execution.adapter_hints = vec![
            crate::hardware::RuntimeAdapterHint::CpuSimd,
            crate::hardware::RuntimeAdapterHint::PortableRust,
        ];
        let request = RuntimeRequest {
            prompt: "Select observed local adapter".to_owned(),
            profile: TaskProfile::Coding,
            runtime_metadata: runtime.metadata(),
            runtime_architecture: runtime.architecture(),
            memory_hints: Vec::new(),
            infini_memory_hints: Vec::new(),
            experience_hints: Vec::new(),
            runtime_adapter_observations: vec![crate::runtime::RuntimeAdapterObservation::new(
                crate::hardware::RuntimeAdapterHint::CpuSimd,
                0.89,
                0.88,
                0.91,
                Some(0.18),
                Some(0.42),
                42,
            )],
            toolsmith_plan: crate::toolsmith::ToolsmithPlan::default(),
            agent_team_plan: crate::agent_team::AgentTeamPlan::default(),
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
            hardware_plan,
            imported_kv_blocks: Vec::new(),
            max_tokens: 64,
        };

        let response = runtime.generate(request).unwrap();

        assert_eq!(
            response.diagnostics.selected_adapter.as_deref(),
            Some("cpu-simd")
        );
    }

    #[test]
    fn imported_kv_changes_local_forward_state() {
        let request = RuntimeRequest {
            prompt: "Build local Noiron runtime".to_owned(),
            profile: TaskProfile::Coding,
            runtime_metadata: RuntimeMetadata::default(),
            runtime_architecture: TransformerRuntimeArchitecture::new(6, 16, 4, 2, 128),
            memory_hints: Vec::new(),
            infini_memory_hints: Vec::new(),
            experience_hints: Vec::new(),
            runtime_adapter_observations: Vec::new(),
            toolsmith_plan: crate::toolsmith::ToolsmithPlan::default(),
            agent_team_plan: crate::agent_team::AgentTeamPlan::default(),
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
            imported_kv_blocks: Vec::new(),
            max_tokens: 64,
        };
        let tokens = local_tokenize(&request.prompt)
            .into_iter()
            .map(|text| RuntimeTokenId::new((stable_hash(&text) % 1_000_000) as u32, text))
            .collect::<Vec<_>>();
        let embedding = embed_tokens(&tokens, 16);
        let no_kv =
            run_transformer_forward(&embedding, &[], &request, request.runtime_architecture);
        let with_kv = run_transformer_forward(
            &embedding,
            &[RuntimeKvBlock::new(
                0,
                0,
                0,
                1,
                vec![1.0, 0.5, 0.25, 0.125],
                vec![0.75, 0.375, 0.1875, 0.09375],
            )],
            &request,
            request.runtime_architecture,
        );

        assert!(with_kv.kv_influence > no_kv.kv_influence);
        assert_ne!(with_kv.vector, no_kv.vector);
    }

    #[test]
    fn engine_can_use_local_runtime_end_to_end() {
        let outcome = LocalTransformerRuntime::run_once(
            "Build a Rust Noiron local Transformer runtime with KV exchange",
            TaskProfile::Coding,
        );

        assert!(outcome.answer.contains("Local Transformer runtime"));
        assert!(outcome.exported_runtime_kv_blocks > 0);
        assert!(outcome.runtime_diagnostics.has_forward_signal());
        assert_eq!(
            outcome.runtime_diagnostics.model_id.as_deref(),
            Some("noiron-local-transformer")
        );
        assert!(!outcome.stored_runtime_kv_memory_ids.is_empty());
    }

    #[test]
    fn engine_uses_local_runtime_embeddings_for_memory_vectors() {
        let mut engine = NoironEngine::new();
        let runtime = LocalTransformerRuntime::new(128, 24);
        let mut backend = RuntimeBackend::new(runtime);

        let outcome = engine.infer(
            InferenceRequest::new(
                "Store a reusable Noiron memory using model-side embeddings",
                TaskProfile::Coding,
            ),
            &mut backend,
        );

        assert!(outcome.stored_memory_id.is_some());
        assert_eq!(
            outcome.embedding_diagnostics.query.source,
            crate::engine::EmbeddingSource::Runtime
        );
        assert_eq!(outcome.embedding_diagnostics.query.dimensions, 24);
        assert!(outcome.embedding_diagnostics.runtime_embedding_available());
        assert!(!outcome.embedding_diagnostics.fallback_embedding_used());
        assert!(
            engine
                .cache
                .entries()
                .iter()
                .any(|entry| entry.vector.len() == 24)
        );
    }
}
