use crate::runtime::RuntimeRequest;
use crate::runtime_manifest::TransformerRuntimeArchitecture;
use crate::transformer::{
    AttentionKind, TransformerLayerPlan, TransformerPlanCounts, TransformerPlanner,
};

use super::model::LocalLayerSummary;

pub(in crate::local_runtime) fn count_forward_layers(
    layers: &[LocalLayerSummary],
) -> TransformerPlanCounts {
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

pub(super) fn runtime_layers_for_architecture(
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
