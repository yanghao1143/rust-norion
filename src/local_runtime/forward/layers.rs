use crate::kv_exchange::RuntimeKvBlock;
use crate::transformer::{AttentionKind, TransformerLayerPlan};

use super::super::tokenizer::normalize;

pub(super) fn apply_imported_kv(
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

pub(super) fn apply_transformer_layer(
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
