use crate::kv_exchange::RuntimeKvBlock;
use crate::reflection::{ReasoningStep, RuntimeDiagnostics};
use crate::runtime::{RuntimeError, RuntimeToken, RuntimeTokenId};
use crate::transformer::{
    AttentionKind, TransformerLayerPlan, TransformerPlanCounts, TransformerPlanner,
};

use super::kernel::{ProductionForwardKernel, ProductionKernelContext, ProductionKernelOutput};
use super::util::{compact, embed_tokens, normalize, production_tokenize, stable_hash};

#[derive(Debug, Clone, Default)]
pub struct ReferenceProductionForwardKernel;

impl ReferenceProductionForwardKernel {
    pub fn new() -> Self {
        Self
    }
}

impl ProductionForwardKernel for ReferenceProductionForwardKernel {
    fn generate(
        &self,
        context: ProductionKernelContext<'_>,
    ) -> Result<ProductionKernelOutput, RuntimeError> {
        let token_ids = production_token_ids(&context.request.prompt);
        let forward = run_reference_forward(&token_ids, context);
        let counts = count_reference_layers(&forward.layer_summaries);
        let exported_kv_blocks = export_reference_kv(&forward, context);
        let answer = reference_answer(context, token_ids.len(), &forward, counts);
        let tokens = answer
            .split_whitespace()
            .map(|text| {
                let entropy = estimated_entropy(text, forward.energy, forward.kv_influence);
                RuntimeToken {
                    text: text.to_owned(),
                    logprob: Some(-entropy),
                    entropy: Some(entropy),
                }
            })
            .collect::<Vec<_>>();
        let trace = vec![
            ReasoningStep::new(
                "reference_production_kernel",
                format!(
                    "executed manifest-backed Rust reference kernel for model_id={} adapter={}",
                    context.manifest.metadata.model_id,
                    context.device_gate.runtime_adapter_name()
                ),
                0.84,
            ),
            ReasoningStep::new(
                "reference_transformer_forward",
                format!(
                    "ran {} layers with hidden size {} energy {:.3} kv_influence {:.3}",
                    forward.layer_summaries.len(),
                    context.manifest.architecture.hidden_size,
                    forward.energy,
                    forward.kv_influence
                ),
                0.82,
            ),
            ReasoningStep::new(
                "reference_kv_exchange",
                format!(
                    "received {} imported KV blocks and prepared {} exported KV blocks",
                    context.imported_kv_blocks.len(),
                    exported_kv_blocks.len()
                ),
                0.80,
            ),
        ];
        let diagnostics = RuntimeDiagnostics {
            model_id: Some(context.manifest.metadata.model_id.clone()),
            selected_adapter: context
                .device_gate
                .runtime_adapter
                .map(|adapter| adapter.as_str().to_owned()),
            device_profile: Some(context.device_gate.device.as_str().to_owned()),
            primary_lane: Some(
                context
                    .request
                    .hardware_plan
                    .execution
                    .primary_lane
                    .as_str()
                    .to_owned(),
            ),
            fallback_lane: Some(
                context
                    .request
                    .hardware_plan
                    .execution
                    .fallback_lane
                    .as_str()
                    .to_owned(),
            ),
            memory_mode: Some(
                context
                    .request
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
            global_layers: counts.global,
            local_window_layers: counts.local,
            convolutional_fusion_layers: counts.convolution,
            hidden_size: context.manifest.architecture.hidden_size,
            local_window_tokens: context.manifest.architecture.local_window_tokens,
            forward_energy: Some(forward.energy),
            kv_influence: Some(forward.kv_influence),
            imported_kv_blocks: context.imported_kv_blocks.len(),
            exported_kv_blocks: exported_kv_blocks.len(),
            runtime_kv_segments_included: context.imported_kv_blocks.len(),
            runtime_kv_segments_skipped: 0,
            runtime_kv_segments_rejected: 0,
            hot_kv_precision_bits: Some(context.device_gate.hot_kv_precision_bits),
            cold_kv_precision_bits: Some(context.device_gate.cold_kv_precision_bits),
            ..RuntimeDiagnostics::default()
        };

        Ok(ProductionKernelOutput::new(answer)
            .with_tokens(tokens)
            .with_trace(trace)
            .with_diagnostics(diagnostics)
            .with_exported_kv_blocks(exported_kv_blocks))
    }
}

#[derive(Debug, Clone)]
struct ReferenceForwardState {
    vector: Vec<f32>,
    layer_summaries: Vec<ReferenceLayerSummary>,
    energy: f32,
    kv_influence: f32,
}

#[derive(Debug, Clone)]
struct ReferenceLayerSummary {
    layer_index: usize,
    attention: AttentionKind,
    window_size: usize,
    compute_fraction: f32,
    activation: f32,
}

fn production_token_ids(prompt: &str) -> Vec<RuntimeTokenId> {
    production_tokenize(prompt)
        .into_iter()
        .map(|text| RuntimeTokenId::new((stable_hash(&text) % 1_000_000) as u32, text))
        .collect()
}

fn run_reference_forward(
    token_ids: &[RuntimeTokenId],
    context: ProductionKernelContext<'_>,
) -> ReferenceForwardState {
    let mut vector = embed_tokens(
        token_ids,
        context.manifest.metadata.embedding_dimensions.max(1),
    );
    if vector.is_empty() {
        vector.push(0.0);
    }

    let layers = reference_layers(context);
    let mut layer_summaries = Vec::with_capacity(layers.len());
    let mut kv_influence = 0.0;

    for layer in &layers {
        kv_influence += apply_reference_imported_kv(&mut vector, context.imported_kv_blocks, layer);
        apply_reference_layer(
            &mut vector,
            layer,
            context.request.route_budget.attention_fraction,
            context.assets.weights_bytes,
        );
        layer_summaries.push(ReferenceLayerSummary {
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

    ReferenceForwardState {
        energy: mean_abs(&vector),
        vector,
        layer_summaries,
        kv_influence,
    }
}

fn reference_layers(context: ProductionKernelContext<'_>) -> Vec<TransformerLayerPlan> {
    let architecture = context.manifest.architecture;
    let layer_count = architecture.layer_count.max(1);
    let local_window = architecture.local_window_tokens.max(16);
    let native_window = context
        .manifest
        .metadata
        .native_context_window
        .max(local_window)
        .max(16);

    let mut plan = if context.request.transformer_plan.layers.len() == layer_count {
        context.request.transformer_plan.clone()
    } else {
        TransformerPlanner::new(layer_count, local_window).plan(
            context.request.profile,
            context.request.hierarchy,
            context.request.route_budget,
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

fn apply_reference_imported_kv(
    vector: &mut [f32],
    imported_kv_blocks: &[RuntimeKvBlock],
    layer: &TransformerLayerPlan,
) -> f32 {
    if vector.is_empty() || imported_kv_blocks.is_empty() {
        return 0.0;
    }

    let mut applied = 0.0;
    for block in imported_kv_blocks
        .iter()
        .filter(|block| block.layer == layer.layer_index)
        .take(8)
    {
        let scale = match layer.attention {
            AttentionKind::Global => 0.034,
            AttentionKind::LocalWindow => 0.021,
            AttentionKind::ConvolutionalFusion => 0.014,
        } * layer.compute_fraction.clamp(0.1, 1.0);
        for (offset, value) in block.vector().iter().enumerate() {
            let index = (offset + block.head) % vector.len();
            vector[index] += value * scale;
            applied += value.abs() * scale;
        }
    }

    applied
}

fn apply_reference_layer(
    vector: &mut [f32],
    layer: &TransformerLayerPlan,
    attention_fraction: f32,
    weights_bytes: u64,
) {
    if vector.is_empty() {
        return;
    }

    let asset_phase = ((weights_bytes as usize % 97) + layer.layer_index + 1) as f32;
    match layer.attention {
        AttentionKind::Global => {
            let mean = vector.iter().sum::<f32>() / vector.len() as f32;
            let gain =
                0.042 + layer.compute_fraction * 0.052 + attention_fraction.clamp(0.0, 1.0) * 0.018;
            for (index, value) in vector.iter_mut().enumerate() {
                let positional = ((index + 1) as f32 * asset_phase).sin() * 0.0025;
                *value = *value * (1.0 - gain) + mean * gain + positional;
            }
        }
        AttentionKind::LocalWindow => {
            let previous = vector.to_vec();
            let radius =
                ((layer.window_size / 64).max(1)).min(previous.len().saturating_sub(1).max(1));
            let gain = 0.096 + layer.compute_fraction * 0.075;
            for index in 0..vector.len() {
                let left = previous[index.saturating_sub(radius)];
                let right = previous[(index + radius).min(previous.len() - 1)];
                let local = (left + previous[index] + right) / 3.0;
                vector[index] = previous[index] * (1.0 - gain) + local * gain;
            }
        }
        AttentionKind::ConvolutionalFusion => {
            let previous = vector.to_vec();
            let gain = 0.118 + layer.compute_fraction * 0.118;
            for index in 0..vector.len() {
                let prev = previous[index.saturating_sub(1)];
                let center = previous[index];
                let next = previous[(index + 1).min(previous.len() - 1)];
                let fused = prev * 0.25 + center * 0.50 + next * 0.25;
                let phase = ((index + 1) as f32 + asset_phase).cos() * 0.0018;
                vector[index] = center * (1.0 - gain) + fused * gain + phase;
            }
        }
    }
    normalize(vector);
}

fn count_reference_layers(layers: &[ReferenceLayerSummary]) -> TransformerPlanCounts {
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

fn export_reference_kv(
    forward: &ReferenceForwardState,
    context: ProductionKernelContext<'_>,
) -> Vec<RuntimeKvBlock> {
    if forward.vector.is_empty() || !context.manifest.kv_policy.export_enabled {
        return Vec::new();
    }

    let architecture = context.manifest.architecture;
    let max_blocks = context.manifest.kv_policy.max_export_blocks.max(1);
    let block_count = forward
        .layer_summaries
        .len()
        .clamp(1, 4)
        .min(context.request.recursive_schedule.chunk_count().max(1) + 2)
        .min(max_blocks);
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
        .take(midpoint)
        .collect::<Vec<_>>();
    let value = if value.is_empty() { key.clone() } else { value };

    (0..block_count)
        .map(|index| {
            let summary = forward.layer_summaries.get(index);
            let layer = summary.map(|summary| summary.layer_index).unwrap_or(index)
                % architecture.layer_count.max(1);
            let head = summary
                .map(|summary| {
                    let attention_offset = match summary.attention {
                        AttentionKind::Global => 0,
                        AttentionKind::LocalWindow => 1,
                        AttentionKind::ConvolutionalFusion => 2,
                    };
                    attention_offset + summary.window_size + index
                })
                .unwrap_or(index)
                % architecture.kv_heads.max(1);
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

fn reference_answer(
    context: ProductionKernelContext<'_>,
    prompt_tokens: usize,
    forward: &ReferenceForwardState,
    counts: TransformerPlanCounts,
) -> String {
    format!(
        "Reference production Transformer kernel result for '{}'. The self-developed production boundary loaded manifest {}, tokenizer {}, local weights bytes {}, and selected adapter {} on device {}. It processed {} prompt tokens, {} imported KV blocks, and executed {} deterministic Rust Transformer-style layers with {:.3} state energy and {:.3} KV influence: {} global, {} local-window, and {} convolutional-fusion layers. Noiron keeps this reference kernel replaceable while the control plane exercises production manifest gates, device contracts, runtime KV exchange, reflection, process rewards, and durable local memory.",
        compact(&context.request.prompt, 96),
        context.manifest.metadata.model_id,
        context.manifest.metadata.tokenizer,
        context.assets.weights_bytes,
        context.device_gate.runtime_adapter_name(),
        context.device_gate.device.as_str(),
        prompt_tokens,
        context.imported_kv_blocks.len(),
        forward.layer_summaries.len(),
        forward.energy,
        forward.kv_influence,
        counts.global,
        counts.local,
        counts.convolution,
    )
}

fn mean_abs(values: &[f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().map(|value| value.abs()).sum::<f32>() / values.len() as f32
}

fn estimated_entropy(token: &str, energy: f32, kv_influence: f32) -> f32 {
    let unique_chars = token
        .chars()
        .collect::<std::collections::HashSet<_>>()
        .len();
    (0.10 + unique_chars as f32 / 36.0 + energy * 0.08 + kv_influence.min(1.0) * 0.03)
        .clamp(0.05, 1.35)
}

fn scaled(values: &[f32], scale: f32) -> Vec<f32> {
    values.iter().map(|value| value * scale).collect()
}
