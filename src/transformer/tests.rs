use crate::hierarchy::{HierarchyWeights, TaskProfile};
use crate::router::RouteBudget;

use super::{AttentionKind, TransformerPlanner, TransformerTemplateKind};

#[test]
fn coding_plan_prefers_local_layers() {
    let planner = TransformerPlanner::new(12, 128);
    let plan = planner.plan(
        TaskProfile::Coding,
        HierarchyWeights::new(0.2, 0.6, 0.2),
        budget(0.5),
    );
    let counts = plan.counts();

    assert_eq!(plan.template, Some(TransformerTemplateKind::CodingLocal));
    assert!(counts.local >= counts.global);
    assert!(counts.local >= counts.convolution);
    assert!(
        plan.layers
            .iter()
            .filter(|layer| layer.attention == AttentionKind::LocalWindow)
            .all(|layer| layer.window_size <= 192)
    );
}

#[test]
fn long_document_plan_keeps_convolution_layers() {
    let planner = TransformerPlanner::new(12, 128);
    let plan = planner.plan(
        TaskProfile::LongDocument,
        HierarchyWeights::new(0.2, 0.2, 0.6),
        budget(0.3),
    );

    assert_eq!(
        plan.template,
        Some(TransformerTemplateKind::LongContextConvolution)
    );
    assert!(plan.counts().convolution > 0);
}

#[test]
fn writing_plan_uses_global_template() {
    let planner = TransformerPlanner::new(12, 128);
    let plan = planner.plan(
        TaskProfile::Writing,
        HierarchyWeights::new(0.3, 0.4, 0.3),
        budget(0.4),
    );
    let counts = plan.counts();

    assert_eq!(
        plan.template,
        Some(TransformerTemplateKind::CreativeWritingGlobal)
    );
    assert!(counts.global >= counts.convolution);
    assert_eq!(plan.template_name(), "creative_writing_global");
}

fn budget(attention_fraction: f32) -> RouteBudget {
    RouteBudget {
        threshold: 0.5,
        attention_tokens: 1,
        fast_tokens: 1,
        attention_fraction,
    }
}
