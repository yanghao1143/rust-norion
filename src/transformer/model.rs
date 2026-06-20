use super::template::TransformerTemplateKind;

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
    pub template: Option<TransformerTemplateKind>,
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

    pub fn template_name(&self) -> &'static str {
        self.template
            .map(TransformerTemplateKind::as_str)
            .unwrap_or("none")
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TransformerPlanCounts {
    pub global: usize,
    pub local: usize,
    pub convolution: usize,
}
