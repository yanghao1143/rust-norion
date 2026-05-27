use crate::hierarchy::{HierarchyWeights, TaskProfile};
use crate::router::RouteBudget;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttentionKind {
    Global,
    LocalWindow,
    ConvolutionalFusion,
}

#[derive(Debug, Clone)]
pub struct TransformerLayerPlan {
    pub layer_index: usize,
    pub attention: AttentionKind,
    pub compute_fraction: f32,
    pub window_size: usize,
}

#[derive(Debug, Clone, Default)]
pub struct TransformerRefactorPlan {
    pub layers: Vec<TransformerLayerPlan>,
}

impl TransformerRefactorPlan {
    pub fn counts(&self) -> TransformerPlanCounts {
        let mut counts = TransformerPlanCounts::default();

        for layer in &self.layers {
            match layer.attention {
                AttentionKind::Global => counts.global += 1,
                AttentionKind::LocalWindow => counts.local += 1,
                AttentionKind::ConvolutionalFusion => counts.convolution += 1,
            }
        }

        counts
    }

    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TransformerPlanCounts {
    pub global: usize,
    pub local: usize,
    pub convolution: usize,
}

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
        let target = adjusted_weights(profile, hierarchy);
        let mut global_left = quota(self.layer_count, target.global);
        let mut local_left = quota(self.layer_count, target.local);
        let mut convolution_left = self
            .layer_count
            .saturating_sub(global_left)
            .saturating_sub(local_left);

        if convolution_left == 0 && target.convolution > 0.1 && self.layer_count >= 3 {
            convolution_left = 1;
            if local_left >= global_left && local_left > 0 {
                local_left -= 1;
            } else if global_left > 0 {
                global_left -= 1;
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
                window_size: window_size(attention, self.base_window_size, route_budget),
            });
        }

        TransformerRefactorPlan { layers }
    }
}

fn adjusted_weights(profile: TaskProfile, mut hierarchy: HierarchyWeights) -> HierarchyWeights {
    match profile {
        TaskProfile::Coding => hierarchy.local += 0.08,
        TaskProfile::Writing => hierarchy.global += 0.08,
        TaskProfile::LongDocument => hierarchy.convolution += 0.10,
        TaskProfile::General => {}
    }
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

fn window_size(attention: AttentionKind, base: usize, route_budget: RouteBudget) -> usize {
    let multiplier = match attention {
        AttentionKind::Global => 8.0,
        AttentionKind::LocalWindow => 1.0 + route_budget.attention_fraction as f64,
        AttentionKind::ConvolutionalFusion => 0.5,
    };
    ((base as f64 * multiplier).round() as usize).max(16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coding_plan_prefers_local_layers() {
        let planner = TransformerPlanner::new(12, 128);
        let plan = planner.plan(
            TaskProfile::Coding,
            HierarchyWeights::new(0.2, 0.6, 0.2),
            budget(0.5),
        );
        let counts = plan.counts();

        assert!(counts.local >= counts.global);
        assert!(counts.local >= counts.convolution);
    }

    #[test]
    fn long_document_plan_keeps_convolution_layers() {
        let planner = TransformerPlanner::new(12, 128);
        let plan = planner.plan(
            TaskProfile::LongDocument,
            HierarchyWeights::new(0.2, 0.2, 0.6),
            budget(0.3),
        );

        assert!(plan.counts().convolution > 0);
    }

    fn budget(attention_fraction: f32) -> RouteBudget {
        RouteBudget {
            threshold: 0.5,
            attention_tokens: 1,
            fast_tokens: 1,
            attention_fraction,
        }
    }
}
