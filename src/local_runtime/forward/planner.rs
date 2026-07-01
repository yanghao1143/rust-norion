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

    plan.layers
        .truncate(runtime_forward_layer_limit(request, layer_count));

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

fn runtime_forward_layer_limit(request: &RuntimeRequest, layer_count: usize) -> usize {
    let attention_fraction = request.route_budget.attention_fraction.clamp(0.0, 1.0);
    if attention_fraction >= 0.50 {
        return layer_count;
    }

    let route_tokens = request.route_budget.attention_tokens + request.route_budget.fast_tokens;
    let attention_ratio = if route_tokens == 0 {
        0.0
    } else {
        request.route_budget.attention_tokens as f32 / route_tokens as f32
    };
    let budget_fraction = attention_fraction.max(attention_ratio.clamp(0.0, 1.0));
    ((layer_count as f32 * (0.35 + budget_fraction)).ceil() as usize).clamp(1, layer_count)
}
