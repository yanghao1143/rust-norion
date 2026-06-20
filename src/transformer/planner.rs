use crate::hierarchy::{HierarchyWeights, TaskProfile};
use crate::router::RouteBudget;

use super::model::{AttentionKind, TransformerLayerPlan, TransformerRefactorPlan};
use super::template::TransformerTemplate;

#[derive(Debug, Clone)]
pub struct TransformerPlanner {
    layer_count: usize,
    base_window_size: usize,
}

impl Default for TransformerPlanner {
    fn default() -> Self {
        Self {
            layer_count: 24,
            base_window_size: 256,
        }
    }
}

impl TransformerPlanner {
    pub fn new(layer_count: usize, base_window_size: usize) -> Self {
        Self {
            layer_count: layer_count.max(1),
            base_window_size: base_window_size.max(16),
        }
    }

    pub fn plan(
        &self,
        profile: TaskProfile,
        hierarchy: HierarchyWeights,
        route_budget: RouteBudget,
    ) -> TransformerRefactorPlan {
        let template = TransformerTemplate::for_profile(profile);
        let target = adjusted_weights(template, hierarchy);
        let mut global_left = quota(self.layer_count, target.global);
        let mut local_left = quota(self.layer_count, target.local);
        let mut convolution_left = self
            .layer_count
            .saturating_sub(global_left)
            .saturating_sub(local_left);

        if convolution_left == 0 && target.convolution > 0.1 && self.layer_count >= 3 {
            convolution_left = 1;
            if local_left >= global_left && local_left > 0 {
                local_left = local_left.saturating_sub(1);
            } else if global_left > 0 {
                global_left = global_left.saturating_sub(1);
            }
        }

        let mut layers = Vec::with_capacity(self.layer_count);
        for layer_index in 0..self.layer_count {
            let attention = choose_attention(
                layer_index,
                self.layer_count,
                &mut global_left,
                &mut local_left,
                &mut convolution_left,
            );
            layers.push(TransformerLayerPlan {
                layer_index,
                attention,
                compute_fraction: compute_fraction(attention, route_budget),
                window_size: window_size(attention, self.base_window_size, route_budget, template),
            });
        }

        TransformerRefactorPlan {
            template: Some(template.kind),
            layers,
        }
    }
}

fn adjusted_weights(
    template: TransformerTemplate,
    mut hierarchy: HierarchyWeights,
) -> HierarchyWeights {
    hierarchy.global += template.global_bias;
    hierarchy.local += template.local_bias;
    hierarchy.convolution += template.convolution_bias;
    hierarchy.normalize();
    hierarchy
}

fn quota(total: usize, fraction: f32) -> usize {
    ((total as f32 * fraction).round() as usize).min(total)
}

fn choose_attention(
    layer_index: usize,
    layer_count: usize,
    global_left: &mut usize,
    local_left: &mut usize,
    convolution_left: &mut usize,
) -> AttentionKind {
    let early_or_late = layer_index == 0 || layer_index + 1 == layer_count;

    if early_or_late && *global_left > 0 {
        *global_left -= 1;
        return AttentionKind::Global;
    }
    if layer_index % 4 == 3 && *convolution_left > 0 {
        *convolution_left -= 1;
        return AttentionKind::ConvolutionalFusion;
    }
    if *local_left > 0 {
        *local_left -= 1;
        return AttentionKind::LocalWindow;
    }
    if *global_left > 0 {
        *global_left -= 1;
        return AttentionKind::Global;
    }
    if *convolution_left > 0 {
        *convolution_left -= 1;
        return AttentionKind::ConvolutionalFusion;
    }
    AttentionKind::LocalWindow
}

fn compute_fraction(attention: AttentionKind, route_budget: RouteBudget) -> f32 {
    let attention_pressure = route_budget.attention_fraction.clamp(0.0, 1.0);
    match attention {
        AttentionKind::Global => 0.65 + attention_pressure * 0.35,
        AttentionKind::LocalWindow => 0.45 + attention_pressure * 0.25,
        AttentionKind::ConvolutionalFusion => 0.30 + attention_pressure * 0.20,
    }
}

fn window_size(
    attention: AttentionKind,
    base: usize,
    route_budget: RouteBudget,
    template: TransformerTemplate,
) -> usize {
    let multiplier = match attention {
        AttentionKind::Global => template.global_window_scale as f64,
        AttentionKind::LocalWindow => {
            template.local_window_scale as f64 * (1.0 + route_budget.attention_fraction as f64)
        }
        AttentionKind::ConvolutionalFusion => template.convolution_window_scale as f64,
    };
    ((base as f64 * multiplier).round() as usize).max(16)
}
