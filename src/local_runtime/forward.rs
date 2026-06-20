mod kv_export;
mod layers;
mod math;
mod model;
mod planner;

pub(super) use kv_export::export_forward_kv;
use layers::{apply_imported_kv, apply_transformer_layer};
use math::mean_abs;
pub(super) use model::{LocalForwardState, LocalLayerSummary};
pub(super) use planner::count_forward_layers;
use planner::runtime_layers_for_architecture;

use crate::kv_exchange::RuntimeKvBlock;
use crate::runtime::RuntimeRequest;
use crate::runtime_manifest::TransformerRuntimeArchitecture;

use super::tokenizer::normalize;

pub(super) fn run_transformer_forward(
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
