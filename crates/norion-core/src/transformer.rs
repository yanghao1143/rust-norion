use crate::attention::{
    AttentionDecisionSummary, AttentionSelectionReadinessCommitSummary,
    AttentionSelectionReadinessSummary,
};
use crate::engine::{RuntimeFailureBatchSummary, RuntimeFailureReport, RuntimeFailureSummary};
use crate::kv::{
    KvBlock, KvBlockShapeSummary, KvNamespace, RuntimeKvExchangeFailureReturnReport,
    RuntimeKvExchangeFailureReturnSource, RuntimeKvExchangeFailureReturnSummary,
};
use crate::manifest::{RuntimeManifestDigest, TransformerRuntimeArchitecture};
use crate::planning::RuntimePlanningDigest;
use crate::profile::{HierarchyWeights, TaskProfile};
use crate::router::{RouteBudget, RouteBudgetReadinessSummary};
use crate::runtime::RuntimeMetadata;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformerAttentionKind {
    Global,
    LocalWindow,
    Fusion,
}

impl TransformerAttentionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::LocalWindow => "local_window",
            Self::Fusion => "fusion",
        }
    }

    pub fn is_fusion(self) -> bool {
        self == Self::Fusion
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformerTemplateKind {
    GeneralBalanced,
    CodingLocal,
    CreativeWritingGlobal,
    LongContextFusion,
}

impl TransformerTemplateKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::GeneralBalanced => "general_balanced",
            Self::CodingLocal => "coding_local",
            Self::CreativeWritingGlobal => "creative_writing_global",
            Self::LongContextFusion => "long_context_fusion",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformerTemplate {
    pub kind: TransformerTemplateKind,
    pub global_bias: f32,
    pub local_bias: f32,
    pub fusion_bias: f32,
    pub global_window_scale: f32,
    pub local_window_scale: f32,
    pub fusion_window_scale: f32,
}

impl TransformerTemplate {
    pub fn for_profile(profile: TaskProfile) -> Self {
        match profile {
            TaskProfile::General => Self {
                kind: TransformerTemplateKind::GeneralBalanced,
                global_bias: 0.0,
                local_bias: 0.0,
                fusion_bias: 0.0,
                global_window_scale: 8.0,
                local_window_scale: 1.0,
                fusion_window_scale: 0.5,
            },
            TaskProfile::Coding => Self {
                kind: TransformerTemplateKind::CodingLocal,
                global_bias: -0.02,
                local_bias: 0.12,
                fusion_bias: 0.02,
                global_window_scale: 6.0,
                local_window_scale: 0.75,
                fusion_window_scale: 0.5,
            },
            TaskProfile::Writing => Self {
                kind: TransformerTemplateKind::CreativeWritingGlobal,
                global_bias: 0.12,
                local_bias: -0.02,
                fusion_bias: 0.02,
                global_window_scale: 10.0,
                local_window_scale: 1.25,
                fusion_window_scale: 0.6,
            },
            TaskProfile::LongDocument => Self {
                kind: TransformerTemplateKind::LongContextFusion,
                global_bias: 0.02,
                local_bias: -0.04,
                fusion_bias: 0.16,
                global_window_scale: 12.0,
                local_window_scale: 1.5,
                fusion_window_scale: 0.75,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformerPlanningInput {
    pub profile: TaskProfile,
    pub hierarchy: HierarchyWeights,
    pub route_budget: RouteBudget,
    pub layer_count: usize,
    pub base_window_size: usize,
}

impl TransformerPlanningInput {
    pub fn new(
        profile: TaskProfile,
        hierarchy: HierarchyWeights,
        route_budget: RouteBudget,
    ) -> Self {
        Self {
            profile,
            hierarchy,
            route_budget,
            layer_count: 24,
            base_window_size: 256,
        }
    }

    pub fn with_shape(mut self, layer_count: usize, base_window_size: usize) -> Self {
        self.layer_count = layer_count.max(1);
        self.base_window_size = base_window_size.max(16);
        self
    }
}

pub trait TransformerPlannerContract {
    fn plan(&self, input: TransformerPlanningInput) -> TransformerPlanDigest;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefaultTransformerPlanner;

impl TransformerPlannerContract for DefaultTransformerPlanner {
    fn plan(&self, input: TransformerPlanningInput) -> TransformerPlanDigest {
        let template = TransformerTemplate::for_profile(input.profile);
        let target = adjusted_weights(template, input.hierarchy);
        let layer_count = input.layer_count.max(1);
        let mut global_left = quota(layer_count, target.global);
        let mut local_left = quota(layer_count, target.local);
        let mut fusion_left = layer_count
            .saturating_sub(global_left)
            .saturating_sub(local_left);

        if fusion_left == 0 && target.fusion > 0.1 && layer_count >= 3 {
            fusion_left = 1;
            if local_left >= global_left && local_left > 0 {
                local_left = local_left.saturating_sub(1);
            } else if global_left > 0 {
                global_left = global_left.saturating_sub(1);
            }
        }

        let mut layers = Vec::with_capacity(layer_count);
        for layer_index in 0..layer_count {
            let attention = choose_attention(
                layer_index,
                layer_count,
                &mut global_left,
                &mut local_left,
                &mut fusion_left,
            );
            layers.push(TransformerLayerBudget::new(
                layer_index,
                attention,
                planned_compute_fraction(attention, input.route_budget),
                planned_window_size(
                    attention,
                    input.base_window_size,
                    input.route_budget,
                    template,
                ),
            ));
        }

        TransformerPlanDigest::new(Some(template.kind.as_str()), layers)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransformerLayerBudget {
    pub layer_index: usize,
    pub attention: TransformerAttentionKind,
    pub compute_fraction: f32,
    pub window_size: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformerLayerBudgetSummary {
    pub layer_index: usize,
    pub attention: TransformerAttentionKind,
    pub attention_label: &'static str,
    pub compute_fraction: f32,
    pub window_size: usize,
    pub uses_fusion: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TransformerLayerBudgetBatchSummary {
    pub layer_count: usize,
    pub usable_layer_count: usize,
    pub signal_component_count: usize,
    pub problem_component_count: usize,
}

impl TransformerLayerBudget {
    pub fn new(
        layer_index: usize,
        attention: TransformerAttentionKind,
        compute_fraction: f32,
        window_size: usize,
    ) -> Self {
        Self {
            layer_index,
            attention,
            compute_fraction: compute_fraction.clamp(0.0, 1.0),
            window_size,
        }
    }

    pub fn layer_summary(&self) -> TransformerLayerBudgetSummary {
        TransformerLayerBudgetSummary {
            layer_index: self.layer_index,
            attention: self.attention,
            attention_label: self.attention.as_str(),
            compute_fraction: self.compute_fraction,
            window_size: self.window_size,
            uses_fusion: self.attention.is_fusion(),
        }
    }
}

impl TransformerLayerBudgetSummary {
    pub fn compute_reaches(self, threshold: f32) -> bool {
        self.compute_fraction >= threshold.clamp(0.0, 1.0)
    }

    pub fn window_at_least(self, min_window_size: usize) -> bool {
        self.window_size >= min_window_size
    }

    pub fn attention_label_matches_kind(self) -> bool {
        self.attention_label == self.attention.as_str()
    }

    pub fn fusion_flag_matches_kind(self) -> bool {
        self.uses_fusion == self.attention.is_fusion()
    }

    pub fn compute_shape_is_valid(self) -> bool {
        finite_unit(self.compute_fraction)
    }

    pub fn window_shape_is_valid(self) -> bool {
        self.window_size > 0
    }

    pub fn layer_budget_signal_component_count(self) -> usize {
        usize::from(self.window_shape_is_valid())
            + usize::from(self.compute_fraction > 0.0 && self.compute_shape_is_valid())
            + usize::from(self.uses_fusion)
            + usize::from(self.attention != TransformerAttentionKind::LocalWindow)
    }

    pub fn has_layer_budget_signal_components(self) -> bool {
        self.layer_budget_signal_component_count() > 0
    }

    pub fn layer_budget_problem_component_count(self) -> usize {
        usize::from(!self.attention_label_matches_kind())
            + usize::from(!self.fusion_flag_matches_kind())
            + usize::from(!self.compute_shape_is_valid())
            + usize::from(!self.window_shape_is_valid())
    }

    pub fn has_layer_budget_problem_components(self) -> bool {
        self.layer_budget_problem_component_count() > 0
    }

    pub fn layer_budget_accounting_is_consistent(self) -> bool {
        let expected_signal_count = usize::from(self.window_shape_is_valid())
            .saturating_add(usize::from(
                self.compute_fraction > 0.0 && self.compute_shape_is_valid(),
            ))
            .saturating_add(usize::from(self.uses_fusion))
            .saturating_add(usize::from(
                self.attention != TransformerAttentionKind::LocalWindow,
            ));
        let expected_problem_count = usize::from(!self.attention_label_matches_kind())
            .saturating_add(usize::from(!self.fusion_flag_matches_kind()))
            .saturating_add(usize::from(!self.compute_shape_is_valid()))
            .saturating_add(usize::from(!self.window_shape_is_valid()));

        self.layer_budget_signal_component_count() == expected_signal_count
            && self.layer_budget_problem_component_count() == expected_problem_count
    }

    pub fn layer_budget_shape_is_clean(self) -> bool {
        !self.has_layer_budget_problem_components() && self.layer_budget_accounting_is_consistent()
    }

    pub fn can_use_transformer_layer_budget(self) -> bool {
        self.has_layer_budget_signal_components() && self.layer_budget_shape_is_clean()
    }
}

impl TransformerLayerBudgetBatchSummary {
    pub fn from_summaries(summaries: &[TransformerLayerBudgetSummary]) -> Self {
        let mut summary = Self {
            layer_count: summaries.len(),
            ..Self::default()
        };

        for layer in summaries {
            summary.usable_layer_count += usize::from(layer.can_use_transformer_layer_budget());
            summary.signal_component_count = summary
                .signal_component_count
                .saturating_add(layer.layer_budget_signal_component_count());
            summary.problem_component_count = summary
                .problem_component_count
                .saturating_add(layer.layer_budget_problem_component_count());
        }

        summary
    }

    pub fn is_empty(self) -> bool {
        self.layer_count == 0
    }

    pub fn all_layers_usable(self) -> bool {
        !self.is_empty() && self.usable_layer_count == self.layer_count
    }

    pub fn has_layer_budget_signals(self) -> bool {
        self.signal_component_count > 0
    }

    pub fn has_layer_budget_problem_components(self) -> bool {
        self.problem_component_count > 0
    }

    pub fn unusable_layer_count(self) -> usize {
        self.layer_count.saturating_sub(self.usable_layer_count)
    }

    pub fn layer_batch_accounting_is_consistent(self) -> bool {
        self.usable_layer_count <= self.layer_count
            && self.all_layers_usable() == (!self.is_empty() && self.unusable_layer_count() == 0)
            && self.has_layer_budget_signals() == (self.signal_component_count > 0)
            && self.has_layer_budget_problem_components() == (self.problem_component_count > 0)
    }

    pub fn layer_batch_shape_is_clean(self) -> bool {
        self.all_layers_usable()
            && !self.has_layer_budget_problem_components()
            && self.layer_batch_accounting_is_consistent()
    }

    pub fn can_use_transformer_layer_budget_batch(self) -> bool {
        self.layer_batch_shape_is_clean()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformerForwardSummary {
    pub layer_index: usize,
    pub attention: TransformerAttentionKind,
    pub window_size: usize,
    pub compute_fraction: f32,
    pub activation: f32,
}

impl TransformerForwardSummary {
    pub fn new(
        layer_index: usize,
        attention: TransformerAttentionKind,
        window_size: usize,
        compute_fraction: f32,
        activation: f32,
    ) -> Self {
        Self {
            layer_index,
            attention,
            window_size,
            compute_fraction: compute_fraction.clamp(0.0, 1.0),
            activation: if activation.is_finite() {
                activation.max(0.0)
            } else {
                0.0
            },
        }
    }

    pub fn from_layer_budget(layer: &TransformerLayerBudget, activation: f32) -> Self {
        Self::new(
            layer.layer_index,
            layer.attention,
            layer.window_size,
            layer.compute_fraction,
            activation,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformerForwardBatchSummary {
    pub summary_count: usize,
    pub counts: TransformerPlanCounts,
    pub average_compute_fraction: f32,
    pub min_window_size: usize,
    pub max_window_size: usize,
    pub average_activation: f32,
    pub max_activation: f32,
    pub active_layer_count: usize,
    pub has_non_finite_values: bool,
}

impl TransformerForwardBatchSummary {
    pub fn from_summaries(summaries: &[TransformerForwardSummary]) -> Self {
        let mut counts = TransformerPlanCounts::default();
        let mut total_compute = 0.0;
        let mut total_activation = 0.0;
        let mut max_activation = 0.0_f32;
        let mut active_layer_count = 0;
        let mut has_non_finite_values = false;

        for summary in summaries {
            match summary.attention {
                TransformerAttentionKind::Global => counts.global += 1,
                TransformerAttentionKind::LocalWindow => counts.local += 1,
                TransformerAttentionKind::Fusion => counts.fusion += 1,
            }

            if summary.compute_fraction.is_finite() {
                total_compute += summary.compute_fraction;
            } else {
                has_non_finite_values = true;
            }

            if summary.activation.is_finite() {
                total_activation += summary.activation;
                max_activation = max_activation.max(summary.activation);
                if summary.activation > 0.0 {
                    active_layer_count += 1;
                }
            } else {
                has_non_finite_values = true;
            }
        }

        let summary_count = summaries.len();
        let min_window_size = summaries
            .iter()
            .map(|summary| summary.window_size)
            .min()
            .unwrap_or(0);
        let max_window_size = summaries
            .iter()
            .map(|summary| summary.window_size)
            .max()
            .unwrap_or(0);

        Self {
            summary_count,
            counts,
            average_compute_fraction: average(total_compute, summary_count),
            min_window_size,
            max_window_size,
            average_activation: average(total_activation, summary_count),
            max_activation,
            active_layer_count,
            has_non_finite_values,
        }
    }

    pub fn has_forward_activity(self) -> bool {
        self.active_layer_count > 0
    }

    pub fn all_layers_active(self) -> bool {
        self.summary_count > 0 && self.active_layer_count == self.summary_count
    }

    pub fn attention_count_matches_summary_count(self) -> bool {
        self.counts.total() == self.summary_count
    }

    pub fn attention_count_drift(self) -> usize {
        self.counts.total().abs_diff(self.summary_count)
    }

    pub fn active_layer_count_within_summary_count(self) -> bool {
        self.active_layer_count <= self.summary_count
    }

    pub fn active_layer_fraction(self) -> f32 {
        average(self.active_layer_count as f32, self.summary_count)
    }

    pub fn non_local_layer_count(self) -> usize {
        self.counts.global.saturating_add(self.counts.fusion)
    }

    pub fn non_local_fraction(self) -> f32 {
        average(self.non_local_layer_count() as f32, self.summary_count)
    }

    pub fn fusion_fraction(self) -> f32 {
        average(self.counts.fusion as f32, self.summary_count)
    }

    pub fn window_span(self) -> usize {
        self.max_window_size.saturating_sub(self.min_window_size)
    }

    pub fn compute_fraction_shape_is_valid(self) -> bool {
        finite_unit(self.average_compute_fraction)
    }

    pub fn activation_shape_is_valid(self) -> bool {
        self.average_activation.is_finite()
            && self.max_activation.is_finite()
            && self.average_activation >= 0.0
            && self.max_activation >= 0.0
            && self.average_activation <= self.max_activation.max(0.0)
    }

    pub fn window_bounds_shape_is_valid(self) -> bool {
        if self.summary_count == 0 {
            self.min_window_size == 0 && self.max_window_size == 0
        } else {
            self.min_window_size > 0 && self.max_window_size >= self.min_window_size
        }
    }

    pub fn forward_batch_signal_component_count(self) -> usize {
        usize::from(self.summary_count > 0)
            .saturating_add(usize::from(self.has_forward_activity()))
            .saturating_add(usize::from(self.all_layers_active()))
            .saturating_add(usize::from(self.non_local_layer_count() > 0))
            .saturating_add(usize::from(self.counts.fusion > 0))
            .saturating_add(usize::from(
                self.average_compute_fraction > 0.0 && self.compute_fraction_shape_is_valid(),
            ))
            .saturating_add(usize::from(
                self.average_activation > 0.0 && self.activation_shape_is_valid(),
            ))
            .saturating_add(usize::from(
                self.window_span() > 0 && self.window_bounds_shape_is_valid(),
            ))
    }

    pub fn has_forward_batch_signals(self) -> bool {
        self.forward_batch_signal_component_count() > 0
    }

    pub fn forward_count_problem_component_count(self) -> usize {
        usize::from(!self.attention_count_matches_summary_count())
            .saturating_add(usize::from(!self.active_layer_count_within_summary_count()))
    }

    pub fn forward_shape_problem_component_count(self) -> usize {
        usize::from(!self.compute_fraction_shape_is_valid())
            .saturating_add(usize::from(!self.activation_shape_is_valid()))
            .saturating_add(usize::from(!self.window_bounds_shape_is_valid()))
            .saturating_add(usize::from(self.has_non_finite_values))
    }

    pub fn forward_batch_problem_component_count(self) -> usize {
        self.forward_count_problem_component_count()
            .saturating_add(self.forward_shape_problem_component_count())
    }

    pub fn has_forward_batch_problem_components(self) -> bool {
        self.forward_batch_problem_component_count() > 0
    }

    pub fn forward_batch_accounting_is_consistent(self) -> bool {
        let expected_signal_count = usize::from(self.summary_count > 0)
            .saturating_add(usize::from(self.has_forward_activity()))
            .saturating_add(usize::from(self.all_layers_active()))
            .saturating_add(usize::from(self.non_local_layer_count() > 0))
            .saturating_add(usize::from(self.counts.fusion > 0))
            .saturating_add(usize::from(
                self.average_compute_fraction > 0.0 && self.compute_fraction_shape_is_valid(),
            ))
            .saturating_add(usize::from(
                self.average_activation > 0.0 && self.activation_shape_is_valid(),
            ))
            .saturating_add(usize::from(
                self.window_span() > 0 && self.window_bounds_shape_is_valid(),
            ));
        let expected_problem_count = usize::from(!self.attention_count_matches_summary_count())
            .saturating_add(usize::from(!self.active_layer_count_within_summary_count()))
            .saturating_add(usize::from(!self.compute_fraction_shape_is_valid()))
            .saturating_add(usize::from(!self.activation_shape_is_valid()))
            .saturating_add(usize::from(!self.window_bounds_shape_is_valid()))
            .saturating_add(usize::from(self.has_non_finite_values));

        self.forward_batch_signal_component_count() == expected_signal_count
            && self.forward_batch_problem_component_count() == expected_problem_count
            && self.has_forward_batch_problem_components() == (expected_problem_count > 0)
    }

    pub fn forward_batch_shape_is_clean(self) -> bool {
        !self.has_forward_batch_problem_components()
            && self.forward_batch_accounting_is_consistent()
    }

    pub fn can_use_forward_batch(self) -> bool {
        self.summary_count > 0 && self.forward_batch_shape_is_clean()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeKvExportPlan {
    pub max_blocks: usize,
    pub layer_count: usize,
    pub kv_heads: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeKvExportManifestPlanSummary {
    pub manifest_export_enabled: bool,
    pub manifest_max_export_blocks: usize,
    pub runtime_export_enabled: bool,
    pub runtime_max_export_blocks: usize,
    pub requested_export_blocks: usize,
    pub export_plan_max_blocks: usize,
    pub architecture_layer_count: usize,
    pub architecture_kv_heads: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeKvExportSummary {
    pub enabled: bool,
    pub max_blocks: usize,
    pub planned_blocks: usize,
    pub forward_value_len: usize,
    pub forward_summary_count: usize,
    pub forward_batch: TransformerForwardBatchSummary,
    pub hit_export_limit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeKvExportPlanningSummary {
    pub planning_export_blocks: usize,
    pub export_plan_max_blocks: usize,
    pub export_summary: RuntimeKvExportSummary,
    pub export_plan_matches_planning_limit: bool,
    pub planned_export_within_planning: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RuntimeKvExportBlockSummary {
    pub planned_blocks: usize,
    pub materialized_blocks: usize,
    pub runtime_namespace_blocks: usize,
    pub block_shape_signal_component_count: usize,
    pub block_shape_problem_component_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeKvExportReadinessStage {
    ForwardBatch,
    ExportPayload,
    ExportPlanning,
    ExportBlocks,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeKvExportReadinessSummary {
    pub planning_summary: RuntimeKvExportPlanningSummary,
    pub block_summary: RuntimeKvExportBlockSummary,
    pub forward_batch_signal_component_count: usize,
    pub export_payload_signal_component_count: usize,
    pub export_planning_signal_component_count: usize,
    pub export_block_signal_component_count: usize,
    pub forward_batch_blocker_component_count: usize,
    pub export_payload_blocker_component_count: usize,
    pub export_planning_blocker_component_count: usize,
    pub export_block_blocker_component_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeKvExportReadinessCommitSummary {
    pub readiness: RuntimeKvExportReadinessSummary,
    pub action: RuntimeKvExportReadinessCommitAction,
    pub can_commit: bool,
    pub should_return_failure: bool,
    pub failure_reports: Vec<RuntimeFailureReport>,
    pub primary_failure_report: Option<RuntimeFailureReport>,
    pub primary_failure_summary: Option<RuntimeFailureSummary>,
    pub failure_batch: RuntimeFailureBatchSummary,
    pub failure_report_count: usize,
    pub can_format_runtime_failures: bool,
    pub total_signal_component_count: usize,
    pub total_blocker_component_count: usize,
    pub component_accounting_consistent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeKvExportReadinessCommitAction {
    CommitRuntimeKvExport,
    ReturnRuntimeFailure,
}

impl RuntimeKvExportReadinessCommitAction {
    pub fn can_commit(self) -> bool {
        matches!(self, Self::CommitRuntimeKvExport)
    }

    pub fn should_return_failure(self) -> bool {
        matches!(self, Self::ReturnRuntimeFailure)
    }
}

impl RuntimeKvExportSummary {
    pub fn will_export(self) -> bool {
        self.enabled && self.planned_blocks > 0
    }

    pub fn skipped_due_to_empty_forward(self) -> bool {
        self.enabled && self.forward_value_len == 0 && self.planned_blocks == 0
    }

    pub fn has_forward_values(self) -> bool {
        self.forward_value_len > 0
    }

    pub fn has_forward_summaries(self) -> bool {
        self.forward_summary_count > 0
    }

    pub fn forward_batch_matches_summary_count(self) -> bool {
        self.forward_batch.summary_count == self.forward_summary_count
    }

    pub fn forward_summary_count_drift(self) -> usize {
        self.forward_batch
            .summary_count
            .abs_diff(self.forward_summary_count)
    }

    pub fn forward_summary_count_drift_component_count(self) -> usize {
        usize::from(!self.forward_batch_matches_summary_count())
    }

    pub fn planned_blocks_within_limit(self) -> bool {
        self.planned_blocks <= self.max_blocks
    }

    pub fn planned_blocks_over_limit(self) -> usize {
        self.planned_blocks.saturating_sub(self.max_blocks)
    }

    pub fn planned_block_limit_overflow_component_count(self) -> usize {
        usize::from(!self.planned_blocks_within_limit())
    }

    pub fn non_finite_forward_summary_component_count(self) -> usize {
        usize::from(self.forward_batch.has_non_finite_values)
    }

    pub fn forward_input_signal_component_count(self) -> usize {
        usize::from(self.has_forward_values() || self.has_forward_summaries())
    }

    pub fn forward_activity_signal_component_count(self) -> usize {
        usize::from(self.forward_batch.has_forward_activity())
    }

    pub fn export_emit_signal_component_count(self) -> usize {
        usize::from(self.will_export())
    }

    pub fn empty_forward_skip_signal_component_count(self) -> usize {
        usize::from(self.skipped_due_to_empty_forward())
    }

    pub fn export_limit_signal_component_count(self) -> usize {
        usize::from(self.hit_plan_limit())
    }

    pub fn export_payload_signal_component_count(self) -> usize {
        self.forward_input_signal_component_count()
            .saturating_add(self.forward_activity_signal_component_count())
            .saturating_add(self.export_emit_signal_component_count())
            .saturating_add(self.empty_forward_skip_signal_component_count())
            .saturating_add(self.export_limit_signal_component_count())
    }

    pub fn has_export_payload_signals(self) -> bool {
        self.export_payload_signal_component_count() > 0
    }

    pub fn export_payload_problem_component_count(self) -> usize {
        self.forward_summary_count_drift_component_count()
            .saturating_add(self.planned_block_limit_overflow_component_count())
            .saturating_add(self.non_finite_forward_summary_component_count())
    }

    pub fn has_export_payload_problem_components(self) -> bool {
        self.export_payload_problem_component_count() > 0
    }

    pub fn export_payload_accounting_is_consistent(self) -> bool {
        let expected_problem_count = usize::from(self.forward_summary_count_drift() > 0)
            .saturating_add(usize::from(self.planned_blocks_over_limit() > 0))
            .saturating_add(usize::from(self.forward_batch.has_non_finite_values));

        self.export_payload_problem_component_count() == expected_problem_count
            && self.has_export_payload_problem_components() == (expected_problem_count > 0)
    }

    pub fn export_payload_is_clean(self) -> bool {
        !self.has_export_payload_problem_components()
            && self.export_payload_accounting_is_consistent()
    }

    pub fn export_payload_shape_is_clean(self) -> bool {
        self.export_payload_is_clean()
    }

    pub fn can_use_runtime_kv_export_payload(self) -> bool {
        self.export_payload_shape_is_clean() && self.will_export()
    }

    pub fn hit_plan_limit(self) -> bool {
        self.hit_export_limit
    }
}

impl RuntimeKvExportBlockSummary {
    pub fn from_blocks(planned_blocks: usize, blocks: &[KvBlock]) -> Self {
        let block_summaries = blocks
            .iter()
            .map(KvBlock::shape_summary)
            .collect::<Vec<_>>();
        Self::from_block_summaries(planned_blocks, &block_summaries)
    }

    pub fn from_block_summaries(
        planned_blocks: usize,
        block_summaries: &[KvBlockShapeSummary],
    ) -> Self {
        let mut summary = Self {
            planned_blocks,
            materialized_blocks: block_summaries.len(),
            ..Self::default()
        };

        for block_summary in block_summaries {
            summary.runtime_namespace_blocks += usize::from(block_summary.is_runtime_namespace);
            summary.block_shape_signal_component_count = summary
                .block_shape_signal_component_count
                .saturating_add(block_summary.block_shape_signal_component_count());
            summary.block_shape_problem_component_count = summary
                .block_shape_problem_component_count
                .saturating_add(block_summary.runtime_exchange_shape_problem_component_count());
        }

        summary
    }

    pub fn is_empty(self) -> bool {
        self.materialized_blocks == 0
    }

    pub fn block_count_matches_plan(self) -> bool {
        self.materialized_blocks == self.planned_blocks
    }

    pub fn block_count_drift(self) -> usize {
        self.materialized_blocks.abs_diff(self.planned_blocks)
    }

    pub fn all_blocks_are_runtime_namespace(self) -> bool {
        self.materialized_blocks > 0 && self.runtime_namespace_blocks == self.materialized_blocks
    }

    pub fn runtime_namespace_drift_component_count(self) -> usize {
        usize::from(!self.is_empty() && !self.all_blocks_are_runtime_namespace())
    }

    pub fn block_count_drift_component_count(self) -> usize {
        usize::from(!self.block_count_matches_plan())
    }

    pub fn export_block_problem_component_count(self) -> usize {
        self.block_count_drift_component_count()
            .saturating_add(self.block_shape_problem_component_count)
    }

    pub fn has_export_block_problem_components(self) -> bool {
        self.export_block_problem_component_count() > 0
    }

    pub fn has_export_block_signals(self) -> bool {
        self.block_shape_signal_component_count > 0
    }

    pub fn export_block_accounting_is_consistent(self) -> bool {
        let expected_problem_count = usize::from(!self.block_count_matches_plan())
            .saturating_add(self.block_shape_problem_component_count);

        self.export_block_problem_component_count() == expected_problem_count
            && self.has_export_block_problem_components() == (expected_problem_count > 0)
            && self.has_export_block_signals() == (self.block_shape_signal_component_count > 0)
    }

    pub fn runtime_kv_export_block_commit_signal_component_count(self) -> usize {
        self.block_shape_signal_component_count
    }

    pub fn has_runtime_kv_export_block_commit_signals(self) -> bool {
        self.runtime_kv_export_block_commit_signal_component_count() > 0
    }

    pub fn runtime_kv_export_block_commit_blocker_component_count(self) -> usize {
        self.export_block_problem_component_count()
    }

    pub fn has_runtime_kv_export_block_commit_blockers(self) -> bool {
        self.runtime_kv_export_block_commit_blocker_component_count() > 0
    }

    pub fn runtime_kv_export_block_commit_accounting_is_consistent(self) -> bool {
        self.export_block_accounting_is_consistent()
            && self.runtime_kv_export_block_commit_signal_component_count()
                == self.block_shape_signal_component_count
            && self.has_runtime_kv_export_block_commit_signals()
                == (self.runtime_kv_export_block_commit_signal_component_count() > 0)
            && self.runtime_kv_export_block_commit_blocker_component_count()
                == self.export_block_problem_component_count()
            && self.has_runtime_kv_export_block_commit_blockers()
                == (self.runtime_kv_export_block_commit_blocker_component_count() > 0)
    }

    pub fn runtime_kv_export_block_commit_is_clean(self) -> bool {
        self.block_count_matches_plan()
            && !self.has_runtime_kv_export_block_commit_blockers()
            && self.runtime_kv_export_block_commit_accounting_is_consistent()
    }

    pub fn export_block_shape_is_clean(self) -> bool {
        self.runtime_kv_export_block_commit_is_clean()
    }

    pub fn can_commit_runtime_kv_export_blocks(self) -> bool {
        !self.is_empty() && self.runtime_kv_export_block_commit_is_clean()
    }
}

impl RuntimeKvExportPlanningSummary {
    pub fn plan_allows_export(self) -> bool {
        self.planning_export_blocks > 0
    }

    pub fn forward_has_activity(self) -> bool {
        self.export_summary.forward_batch.has_forward_activity()
    }

    pub fn export_will_emit(self) -> bool {
        self.export_summary.will_export()
    }

    pub fn export_is_blocked_by_planning(self) -> bool {
        self.export_will_emit() && !self.planned_export_within_planning
    }

    pub fn export_boundary_is_consistent(self) -> bool {
        self.export_plan_matches_planning_limit && self.planned_export_within_planning
    }

    pub fn export_plan_limit_drift_component_count(self) -> usize {
        usize::from(!self.export_plan_matches_planning_limit)
    }

    pub fn export_plan_limit_drift_blocks(self) -> usize {
        self.export_plan_max_blocks
            .abs_diff(self.planning_export_blocks)
    }

    pub fn export_plan_exceeds_planning_limit(self) -> bool {
        self.export_plan_max_blocks > self.planning_export_blocks
    }

    pub fn export_plan_below_planning_limit(self) -> bool {
        self.export_plan_max_blocks < self.planning_export_blocks
    }

    pub fn planned_export_overflow_blocks(self) -> usize {
        self.export_summary
            .planned_blocks
            .saturating_sub(self.planning_export_blocks)
    }

    pub fn planned_export_overflow_component_count(self) -> usize {
        usize::from(!self.planned_export_within_planning)
    }

    pub fn planning_export_signal_component_count(self) -> usize {
        usize::from(self.plan_allows_export())
    }

    pub fn planning_forward_activity_signal_component_count(self) -> usize {
        usize::from(self.forward_has_activity())
    }

    pub fn planning_export_emit_signal_component_count(self) -> usize {
        usize::from(self.export_will_emit())
    }

    pub fn planning_export_limit_signal_component_count(self) -> usize {
        usize::from(self.export_hit_plan_limit())
    }

    pub fn export_boundary_signal_component_count(self) -> usize {
        self.planning_export_signal_component_count()
            .saturating_add(self.planning_forward_activity_signal_component_count())
            .saturating_add(self.planning_export_emit_signal_component_count())
            .saturating_add(self.planning_export_limit_signal_component_count())
    }

    pub fn has_export_boundary_signals(self) -> bool {
        self.export_boundary_signal_component_count() > 0
    }

    pub fn export_boundary_problem_component_count(self) -> usize {
        self.export_plan_limit_drift_component_count()
            .saturating_add(self.planned_export_overflow_component_count())
    }

    pub fn has_export_boundary_problem_components(self) -> bool {
        self.export_boundary_problem_component_count() > 0
    }

    pub fn export_boundary_accounting_is_consistent(self) -> bool {
        let expected_problem_count = usize::from(self.export_plan_limit_drift_blocks() > 0)
            .saturating_add(usize::from(self.planned_export_overflow_blocks() > 0));

        self.export_boundary_problem_component_count() == expected_problem_count
            && self.has_export_boundary_problem_components() == (expected_problem_count > 0)
            && self.export_boundary_is_consistent() == (expected_problem_count == 0)
    }

    pub fn export_boundary_shape_is_clean(self) -> bool {
        !self.has_export_boundary_problem_components()
            && self.export_boundary_accounting_is_consistent()
    }

    pub fn export_hit_plan_limit(self) -> bool {
        self.export_summary.hit_plan_limit()
    }

    pub fn export_commit_problem_component_count(self) -> usize {
        self.export_boundary_problem_component_count()
            .saturating_add(self.export_summary.export_payload_problem_component_count())
    }

    pub fn has_export_commit_problem_components(self) -> bool {
        self.export_commit_problem_component_count() > 0
    }

    pub fn runtime_kv_export_commit_signal_component_count(self) -> usize {
        self.export_boundary_signal_component_count()
            .saturating_add(self.export_summary.export_payload_signal_component_count())
    }

    pub fn has_runtime_kv_export_commit_signals(self) -> bool {
        self.runtime_kv_export_commit_signal_component_count() > 0
    }

    pub fn runtime_kv_export_commit_blocker_component_count(self) -> usize {
        self.export_commit_problem_component_count()
    }

    pub fn has_runtime_kv_export_commit_blockers(self) -> bool {
        self.runtime_kv_export_commit_blocker_component_count() > 0
    }

    pub fn runtime_kv_export_commit_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .export_boundary_signal_component_count()
            .saturating_add(self.export_summary.export_payload_signal_component_count());
        let expected_blocker_count = self
            .export_boundary_problem_component_count()
            .saturating_add(self.export_summary.export_payload_problem_component_count());

        self.export_boundary_accounting_is_consistent()
            && self
                .export_summary
                .export_payload_accounting_is_consistent()
            && self.runtime_kv_export_commit_signal_component_count() == expected_signal_count
            && self.has_runtime_kv_export_commit_signals() == (expected_signal_count > 0)
            && self.runtime_kv_export_commit_blocker_component_count() == expected_blocker_count
            && self.has_runtime_kv_export_commit_blockers() == (expected_blocker_count > 0)
            && self.has_export_commit_problem_components() == (expected_blocker_count > 0)
    }

    pub fn runtime_kv_export_commit_is_clean(self) -> bool {
        !self.has_runtime_kv_export_commit_blockers()
            && self.runtime_kv_export_commit_accounting_is_consistent()
    }

    pub fn export_commit_is_clean(self) -> bool {
        self.runtime_kv_export_commit_is_clean()
    }

    pub fn can_commit_runtime_kv_export(self) -> bool {
        self.runtime_kv_export_commit_is_clean()
    }
}

impl RuntimeKvExportReadinessSummary {
    pub fn new(
        planning_summary: RuntimeKvExportPlanningSummary,
        block_summary: RuntimeKvExportBlockSummary,
    ) -> Self {
        let export_summary = planning_summary.export_summary;
        let forward_batch = export_summary.forward_batch;

        Self {
            planning_summary,
            block_summary,
            forward_batch_signal_component_count: forward_batch
                .forward_batch_signal_component_count(),
            export_payload_signal_component_count: export_summary
                .export_payload_signal_component_count(),
            export_planning_signal_component_count: planning_summary
                .export_boundary_signal_component_count(),
            export_block_signal_component_count: block_summary
                .runtime_kv_export_block_commit_signal_component_count(),
            forward_batch_blocker_component_count: forward_batch
                .forward_batch_problem_component_count(),
            export_payload_blocker_component_count: export_summary
                .export_payload_problem_component_count(),
            export_planning_blocker_component_count: planning_summary
                .export_boundary_problem_component_count(),
            export_block_blocker_component_count: block_summary
                .runtime_kv_export_block_commit_blocker_component_count(),
        }
    }

    pub fn from_blocks(
        planning_summary: RuntimeKvExportPlanningSummary,
        blocks: &[KvBlock],
    ) -> Self {
        Self::new(
            planning_summary,
            RuntimeKvExportBlockSummary::from_blocks(
                planning_summary.export_summary.planned_blocks,
                blocks,
            ),
        )
    }

    pub fn from_block_summaries(
        planning_summary: RuntimeKvExportPlanningSummary,
        block_summaries: &[KvBlockShapeSummary],
    ) -> Self {
        Self::new(
            planning_summary,
            RuntimeKvExportBlockSummary::from_block_summaries(
                planning_summary.export_summary.planned_blocks,
                block_summaries,
            ),
        )
    }

    pub fn stage_order() -> [RuntimeKvExportReadinessStage; 4] {
        [
            RuntimeKvExportReadinessStage::ForwardBatch,
            RuntimeKvExportReadinessStage::ExportPayload,
            RuntimeKvExportReadinessStage::ExportPlanning,
            RuntimeKvExportReadinessStage::ExportBlocks,
        ]
    }

    pub fn export_will_emit(self) -> bool {
        self.planning_summary.export_will_emit()
    }

    pub fn forward_batch_ready(self) -> bool {
        let forward_batch = self.planning_summary.export_summary.forward_batch;
        if self.export_will_emit() {
            forward_batch.can_use_forward_batch()
        } else {
            forward_batch.forward_batch_shape_is_clean()
        }
    }

    pub fn export_payload_ready(self) -> bool {
        let export_summary = self.planning_summary.export_summary;
        if self.export_will_emit() {
            export_summary.can_use_runtime_kv_export_payload()
        } else {
            export_summary.export_payload_shape_is_clean()
        }
    }

    pub fn export_planning_ready(self) -> bool {
        self.planning_summary.export_boundary_shape_is_clean()
    }

    pub fn export_blocks_ready(self) -> bool {
        if self.export_will_emit() {
            self.block_summary.can_commit_runtime_kv_export_blocks()
        } else {
            self.block_summary.is_empty()
                && self.block_summary.runtime_kv_export_block_commit_is_clean()
        }
    }

    pub fn stage_ready(self, stage: RuntimeKvExportReadinessStage) -> bool {
        match stage {
            RuntimeKvExportReadinessStage::ForwardBatch => self.forward_batch_ready(),
            RuntimeKvExportReadinessStage::ExportPayload => self.export_payload_ready(),
            RuntimeKvExportReadinessStage::ExportPlanning => self.export_planning_ready(),
            RuntimeKvExportReadinessStage::ExportBlocks => self.export_blocks_ready(),
        }
    }

    pub fn stage_signal_component_count(self, stage: RuntimeKvExportReadinessStage) -> usize {
        match stage {
            RuntimeKvExportReadinessStage::ForwardBatch => {
                self.forward_batch_signal_component_count
            }
            RuntimeKvExportReadinessStage::ExportPayload => {
                self.export_payload_signal_component_count
            }
            RuntimeKvExportReadinessStage::ExportPlanning => {
                self.export_planning_signal_component_count
            }
            RuntimeKvExportReadinessStage::ExportBlocks => self.export_block_signal_component_count,
        }
    }

    pub fn stage_blocker_component_count(self, stage: RuntimeKvExportReadinessStage) -> usize {
        match stage {
            RuntimeKvExportReadinessStage::ForwardBatch => {
                self.forward_batch_blocker_component_count
            }
            RuntimeKvExportReadinessStage::ExportPayload => {
                self.export_payload_blocker_component_count
            }
            RuntimeKvExportReadinessStage::ExportPlanning => {
                self.export_planning_blocker_component_count
            }
            RuntimeKvExportReadinessStage::ExportBlocks => {
                self.export_block_blocker_component_count
            }
        }
    }

    pub fn first_unready_stage(self) -> Option<RuntimeKvExportReadinessStage> {
        Self::stage_order()
            .into_iter()
            .find(|stage| !self.stage_ready(*stage))
    }

    pub fn first_blocking_stage(self) -> Option<RuntimeKvExportReadinessStage> {
        Self::stage_order()
            .into_iter()
            .find(|stage| self.stage_blocker_component_count(*stage) > 0)
    }

    pub fn runtime_kv_export_readiness_signal_component_count(self) -> usize {
        self.forward_batch_signal_component_count
            .saturating_add(self.export_payload_signal_component_count)
            .saturating_add(self.export_planning_signal_component_count)
            .saturating_add(self.export_block_signal_component_count)
    }

    pub fn has_runtime_kv_export_readiness_signals(self) -> bool {
        self.runtime_kv_export_readiness_signal_component_count() > 0
    }

    pub fn runtime_kv_export_readiness_blocker_component_count(self) -> usize {
        self.forward_batch_blocker_component_count
            .saturating_add(self.export_payload_blocker_component_count)
            .saturating_add(self.export_planning_blocker_component_count)
            .saturating_add(self.export_block_blocker_component_count)
    }

    pub fn has_runtime_kv_export_readiness_blockers(self) -> bool {
        self.runtime_kv_export_readiness_blocker_component_count() > 0
    }

    pub fn runtime_kv_export_readiness_accounting_is_consistent(self) -> bool {
        let forward_batch = self.planning_summary.export_summary.forward_batch;
        let export_summary = self.planning_summary.export_summary;
        let expected_signal_count = self
            .forward_batch_signal_component_count
            .saturating_add(self.export_payload_signal_component_count)
            .saturating_add(self.export_planning_signal_component_count)
            .saturating_add(self.export_block_signal_component_count);
        let expected_blocker_count = self
            .forward_batch_blocker_component_count
            .saturating_add(self.export_payload_blocker_component_count)
            .saturating_add(self.export_planning_blocker_component_count)
            .saturating_add(self.export_block_blocker_component_count);

        forward_batch.forward_batch_accounting_is_consistent()
            && export_summary.export_payload_accounting_is_consistent()
            && self
                .planning_summary
                .export_boundary_accounting_is_consistent()
            && self
                .block_summary
                .runtime_kv_export_block_commit_accounting_is_consistent()
            && self.forward_batch_signal_component_count
                == forward_batch.forward_batch_signal_component_count()
            && self.forward_batch_blocker_component_count
                == forward_batch.forward_batch_problem_component_count()
            && self.export_payload_signal_component_count
                == export_summary.export_payload_signal_component_count()
            && self.export_payload_blocker_component_count
                == export_summary.export_payload_problem_component_count()
            && self.export_planning_signal_component_count
                == self
                    .planning_summary
                    .export_boundary_signal_component_count()
            && self.export_planning_blocker_component_count
                == self
                    .planning_summary
                    .export_boundary_problem_component_count()
            && self.export_block_signal_component_count
                == self
                    .block_summary
                    .runtime_kv_export_block_commit_signal_component_count()
            && self.export_block_blocker_component_count
                == self
                    .block_summary
                    .runtime_kv_export_block_commit_blocker_component_count()
            && self.runtime_kv_export_readiness_signal_component_count() == expected_signal_count
            && self.has_runtime_kv_export_readiness_signals() == (expected_signal_count > 0)
            && self.runtime_kv_export_readiness_blocker_component_count() == expected_blocker_count
            && self.has_runtime_kv_export_readiness_blockers() == (expected_blocker_count > 0)
    }

    pub fn runtime_kv_export_readiness_is_clean(self) -> bool {
        !self.has_runtime_kv_export_readiness_blockers()
            && self.runtime_kv_export_readiness_accounting_is_consistent()
    }

    pub fn can_commit_runtime_kv_export_readiness(self) -> bool {
        self.runtime_kv_export_readiness_is_clean()
            && self.forward_batch_ready()
            && self.export_payload_ready()
            && self.export_planning_ready()
            && self.export_blocks_ready()
    }

    pub fn runtime_kv_export_readiness_commit_action(self) -> RuntimeKvExportReadinessCommitAction {
        if self.can_commit_runtime_kv_export_readiness() {
            RuntimeKvExportReadinessCommitAction::CommitRuntimeKvExport
        } else {
            RuntimeKvExportReadinessCommitAction::ReturnRuntimeFailure
        }
    }

    pub fn component_accounting_drift_count(self) -> usize {
        usize::from(!self.runtime_kv_export_readiness_accounting_is_consistent())
    }

    pub fn runtime_kv_export_readiness_commit_problem_component_count(self) -> usize {
        self.runtime_kv_export_readiness_blocker_component_count()
            .saturating_add(self.component_accounting_drift_count())
    }

    pub fn has_runtime_kv_export_readiness_commit_problem_components(self) -> bool {
        self.runtime_kv_export_readiness_commit_problem_component_count() > 0
    }

    pub fn failure_report(self) -> Option<RuntimeFailureReport> {
        let component_count = self.runtime_kv_export_readiness_commit_problem_component_count();
        if component_count == 0 {
            None
        } else {
            Some(RuntimeFailureReport::kv_export(format!(
                "runtime kv export readiness failed: components={component_count}, first_blocking_stage={:?}",
                self.first_blocking_stage()
            )))
        }
    }

    pub fn failure_reports(self) -> Vec<RuntimeFailureReport> {
        self.failure_report().into_iter().collect()
    }

    pub fn failure_report_count(self) -> usize {
        usize::from(self.failure_report().is_some())
    }

    pub fn has_failure_reports(self) -> bool {
        self.failure_report_count() > 0
    }

    pub fn failure_batch_summary(self) -> RuntimeFailureBatchSummary {
        RuntimeFailureReport::batch_summary(&self.failure_reports())
    }

    pub fn can_format_runtime_failures(self) -> bool {
        self.failure_batch_summary().can_format_runtime_failures()
    }

    pub fn primary_failure_report(self) -> Option<RuntimeFailureReport> {
        self.failure_report()
    }

    pub fn primary_failure_summary(self) -> Option<RuntimeFailureSummary> {
        self.primary_failure_report()
            .map(|failure| failure.failure_summary())
    }

    pub fn commit_summary(self) -> RuntimeKvExportReadinessCommitSummary {
        RuntimeKvExportReadinessCommitSummary::new(self)
    }
}

impl RuntimeKvExportReadinessCommitSummary {
    pub fn new(readiness: RuntimeKvExportReadinessSummary) -> Self {
        let failure_reports = readiness.failure_reports();
        let primary_failure_report = failure_reports.first().cloned();
        let primary_failure_summary = primary_failure_report
            .as_ref()
            .map(|failure| failure.failure_summary());
        let failure_batch = RuntimeFailureReport::batch_summary(&failure_reports);
        let can_commit = readiness.can_commit_runtime_kv_export_readiness();
        let failure_report_count = failure_reports.len();
        let can_format_runtime_failures = failure_batch.can_format_runtime_failures();
        let should_return_failure = !can_commit && failure_report_count > 0;
        let action = readiness.runtime_kv_export_readiness_commit_action();

        Self {
            readiness,
            action,
            can_commit,
            should_return_failure,
            failure_reports,
            primary_failure_report,
            primary_failure_summary,
            failure_batch,
            failure_report_count,
            can_format_runtime_failures,
            total_signal_component_count: readiness
                .runtime_kv_export_readiness_signal_component_count(),
            total_blocker_component_count: readiness
                .runtime_kv_export_readiness_blocker_component_count(),
            component_accounting_consistent: readiness
                .runtime_kv_export_readiness_accounting_is_consistent(),
        }
    }

    pub fn action_can_commit(&self) -> bool {
        self.action.can_commit()
    }

    pub fn action_should_return_failure(&self) -> bool {
        self.action.should_return_failure()
    }

    pub fn has_primary_failure_summary(&self) -> bool {
        self.primary_failure_summary.is_some()
    }

    pub fn failure_batch_shape_is_clean(&self) -> bool {
        self.failure_batch.failure_batch_shape_is_clean()
    }

    pub fn failure_return_summary(&self) -> RuntimeKvExchangeFailureReturnSummary {
        RuntimeKvExchangeFailureReturnSummary::new(
            RuntimeKvExchangeFailureReturnSource::RuntimeKvExportReadiness,
            self.can_commit,
            self.should_return_failure,
            self.primary_failure_summary,
            self.failure_batch,
            self.failure_report_count,
            self.can_format_runtime_failures,
            self.total_blocker_component_count,
            self.commit_decision_accounting_is_consistent(),
        )
    }

    pub fn runtime_failure_return_report(&self) -> Option<RuntimeKvExchangeFailureReturnReport> {
        if self.failure_return_summary().can_return_runtime_failure() {
            self.primary_failure_report.clone().map(|failure| {
                RuntimeKvExchangeFailureReturnReport::new(
                    RuntimeKvExchangeFailureReturnSource::RuntimeKvExportReadiness,
                    failure,
                    self.failure_batch,
                    self.failure_report_count,
                    self.can_format_runtime_failures,
                    self.total_blocker_component_count,
                )
            })
        } else {
            None
        }
    }

    pub fn commit_decision_accounting_is_consistent(&self) -> bool {
        self.can_commit == self.readiness.can_commit_runtime_kv_export_readiness()
            && self.should_return_failure == (!self.can_commit && self.failure_report_count > 0)
            && self.action == self.readiness.runtime_kv_export_readiness_commit_action()
            && self.action_can_commit() == self.can_commit
            && self.action_should_return_failure() == self.should_return_failure
            && self.failure_report_count == self.failure_reports.len()
            && self.failure_report_count == self.readiness.failure_report_count()
            && self.primary_failure_report.as_ref() == self.failure_reports.first()
            && self.primary_failure_summary
                == self
                    .primary_failure_report
                    .as_ref()
                    .map(|failure| failure.failure_summary())
            && self.failure_batch == RuntimeFailureReport::batch_summary(&self.failure_reports)
            && self.can_format_runtime_failures == self.failure_batch.can_format_runtime_failures()
            && self.total_signal_component_count
                == self
                    .readiness
                    .runtime_kv_export_readiness_signal_component_count()
            && self.total_blocker_component_count
                == self
                    .readiness
                    .runtime_kv_export_readiness_blocker_component_count()
            && self.component_accounting_consistent
                == self
                    .readiness
                    .runtime_kv_export_readiness_accounting_is_consistent()
    }

    pub fn can_commit_runtime_kv_export_readiness(&self) -> bool {
        self.can_commit && self.commit_decision_accounting_is_consistent()
    }

    pub fn should_return_runtime_failure(&self) -> bool {
        self.should_return_failure && self.commit_decision_accounting_is_consistent()
    }
}

impl RuntimeKvExportManifestPlanSummary {
    pub fn manifest_allows_export(self) -> bool {
        self.manifest_export_enabled && self.manifest_max_export_blocks > 0
    }

    pub fn runtime_allows_export(self) -> bool {
        self.runtime_export_enabled && self.runtime_max_export_blocks > 0
    }

    pub fn requested_export(self) -> bool {
        self.requested_export_blocks > 0
    }

    pub fn plan_will_export(self) -> bool {
        self.export_plan_max_blocks > 0
    }

    pub fn architecture_has_export_shape(self) -> bool {
        self.architecture_layer_count > 0 && self.architecture_kv_heads > 0
    }

    pub fn manifest_export_capability_is_consistent(self) -> bool {
        self.manifest_export_enabled == (self.manifest_max_export_blocks > 0)
    }

    pub fn runtime_export_capability_is_consistent(self) -> bool {
        self.runtime_export_enabled == (self.runtime_max_export_blocks > 0)
    }

    pub fn export_plan_within_manifest_limit(self) -> bool {
        self.export_plan_max_blocks <= self.manifest_max_export_blocks
    }

    pub fn export_plan_within_runtime_limit(self) -> bool {
        self.export_plan_max_blocks <= self.runtime_max_export_blocks
    }

    pub fn export_plan_within_requested_limit(self) -> bool {
        self.export_plan_max_blocks <= self.requested_export_blocks
    }

    pub fn requested_export_without_manifest_capacity(self) -> bool {
        self.requested_export() && !self.manifest_allows_export()
    }

    pub fn requested_export_without_runtime_capacity(self) -> bool {
        self.requested_export() && !self.runtime_allows_export()
    }

    pub fn manifest_export_signal_component_count(self) -> usize {
        usize::from(self.manifest_export_enabled) + usize::from(self.manifest_max_export_blocks > 0)
    }

    pub fn runtime_export_signal_component_count(self) -> usize {
        usize::from(self.runtime_export_enabled) + usize::from(self.runtime_max_export_blocks > 0)
    }

    pub fn requested_export_signal_component_count(self) -> usize {
        usize::from(self.requested_export())
    }

    pub fn export_plan_signal_component_count(self) -> usize {
        usize::from(self.plan_will_export())
    }

    pub fn architecture_export_signal_component_count(self) -> usize {
        usize::from(self.architecture_layer_count > 0) + usize::from(self.architecture_kv_heads > 0)
    }

    pub fn manifest_bridge_signal_component_count(self) -> usize {
        self.manifest_export_signal_component_count()
            .saturating_add(self.runtime_export_signal_component_count())
            .saturating_add(self.requested_export_signal_component_count())
            .saturating_add(self.export_plan_signal_component_count())
            .saturating_add(self.architecture_export_signal_component_count())
    }

    pub fn has_manifest_bridge_signals(self) -> bool {
        self.manifest_bridge_signal_component_count() > 0
    }

    pub fn manifest_export_capability_problem_component_count(self) -> usize {
        usize::from(!self.manifest_export_capability_is_consistent())
    }

    pub fn runtime_export_capability_problem_component_count(self) -> usize {
        usize::from(!self.runtime_export_capability_is_consistent())
    }

    pub fn requested_export_capacity_problem_component_count(self) -> usize {
        usize::from(self.requested_export_without_manifest_capacity()).saturating_add(usize::from(
            self.requested_export_without_runtime_capacity(),
        ))
    }

    pub fn export_plan_limit_problem_component_count(self) -> usize {
        usize::from(!self.export_plan_within_manifest_limit())
            .saturating_add(usize::from(!self.export_plan_within_runtime_limit()))
            .saturating_add(usize::from(!self.export_plan_within_requested_limit()))
    }

    pub fn architecture_export_shape_problem_component_count(self) -> usize {
        usize::from(self.architecture_layer_count == 0)
            .saturating_add(usize::from(self.architecture_kv_heads == 0))
    }

    pub fn manifest_bridge_problem_component_count(self) -> usize {
        self.manifest_export_capability_problem_component_count()
            .saturating_add(self.runtime_export_capability_problem_component_count())
            .saturating_add(self.requested_export_capacity_problem_component_count())
            .saturating_add(self.export_plan_limit_problem_component_count())
            .saturating_add(self.architecture_export_shape_problem_component_count())
    }

    pub fn has_manifest_bridge_problem_components(self) -> bool {
        self.manifest_bridge_problem_component_count() > 0
    }

    pub fn manifest_bridge_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .manifest_export_signal_component_count()
            .saturating_add(self.runtime_export_signal_component_count())
            .saturating_add(self.requested_export_signal_component_count())
            .saturating_add(self.export_plan_signal_component_count())
            .saturating_add(self.architecture_export_signal_component_count());
        let expected_problem_count = self
            .manifest_export_capability_problem_component_count()
            .saturating_add(self.runtime_export_capability_problem_component_count())
            .saturating_add(self.requested_export_capacity_problem_component_count())
            .saturating_add(self.export_plan_limit_problem_component_count())
            .saturating_add(self.architecture_export_shape_problem_component_count());

        self.manifest_bridge_signal_component_count() == expected_signal_count
            && self.has_manifest_bridge_signals() == (expected_signal_count > 0)
            && self.manifest_bridge_problem_component_count() == expected_problem_count
            && self.has_manifest_bridge_problem_components() == (expected_problem_count > 0)
    }

    pub fn manifest_bridge_shape_is_clean(self) -> bool {
        !self.has_manifest_bridge_problem_components()
            && self.manifest_bridge_accounting_is_consistent()
    }

    pub fn can_use_manifest_runtime_kv_export_plan(self) -> bool {
        self.manifest_bridge_shape_is_clean()
    }
}

impl RuntimeKvExportPlan {
    pub fn new(
        runtime: &RuntimeMetadata,
        architecture: TransformerRuntimeArchitecture,
        requested_export_blocks: usize,
    ) -> Self {
        let manifest_limit = if !runtime.supports_kv_export {
            0
        } else if runtime.max_kv_export_blocks > 0 {
            runtime.max_kv_export_blocks
        } else {
            requested_export_blocks
        };

        Self {
            max_blocks: requested_export_blocks.min(manifest_limit),
            layer_count: architecture.layer_count.max(1),
            kv_heads: architecture.kv_heads.max(1),
        }
    }

    pub fn from_manifest(manifest: &RuntimeManifestDigest, requested_export_blocks: usize) -> Self {
        let metadata = manifest.runtime_metadata();
        let mut plan = Self::new(&metadata, manifest.architecture, requested_export_blocks);

        plan.max_blocks = if manifest.kv_policy.export_enabled {
            plan.max_blocks.min(manifest.kv_policy.max_export_blocks)
        } else {
            0
        };
        plan
    }

    pub fn manifest_plan_summary(
        manifest: &RuntimeManifestDigest,
        requested_export_blocks: usize,
    ) -> RuntimeKvExportManifestPlanSummary {
        let metadata = manifest.runtime_metadata();
        let plan = Self::from_manifest(manifest, requested_export_blocks);

        RuntimeKvExportManifestPlanSummary {
            manifest_export_enabled: manifest.kv_policy.export_enabled,
            manifest_max_export_blocks: manifest.kv_policy.max_export_blocks,
            runtime_export_enabled: metadata.supports_kv_export,
            runtime_max_export_blocks: metadata.max_kv_export_blocks,
            requested_export_blocks,
            export_plan_max_blocks: plan.max_blocks,
            architecture_layer_count: manifest.architecture.layer_count,
            architecture_kv_heads: manifest.architecture.kv_heads,
        }
    }

    pub fn is_enabled(self) -> bool {
        self.max_blocks > 0
    }

    pub fn planned_block_count(
        self,
        forward_vector: &[f32],
        summaries: &[TransformerForwardSummary],
    ) -> usize {
        if !self.is_enabled() || forward_vector.is_empty() {
            0
        } else {
            summaries.len().clamp(1, 4).min(self.max_blocks)
        }
    }

    pub fn export_summary(
        self,
        forward_vector: &[f32],
        summaries: &[TransformerForwardSummary],
    ) -> RuntimeKvExportSummary {
        let planned_blocks = self.planned_block_count(forward_vector, summaries);

        RuntimeKvExportSummary {
            enabled: self.is_enabled(),
            max_blocks: self.max_blocks,
            planned_blocks,
            forward_value_len: forward_vector.len(),
            forward_summary_count: summaries.len(),
            forward_batch: TransformerForwardBatchSummary::from_summaries(summaries),
            hit_export_limit: planned_blocks > 0
                && planned_blocks == self.max_blocks
                && summaries.len().clamp(1, 4) >= self.max_blocks,
        }
    }

    pub fn planning_summary(
        self,
        planning: RuntimePlanningDigest,
        forward_vector: &[f32],
        summaries: &[TransformerForwardSummary],
    ) -> RuntimeKvExportPlanningSummary {
        let export_summary = self.export_summary(forward_vector, summaries);
        let planning_export_blocks = planning.planned_kv_exchange().export_blocks;

        RuntimeKvExportPlanningSummary {
            planning_export_blocks,
            export_plan_max_blocks: self.max_blocks,
            export_summary,
            export_plan_matches_planning_limit: self.max_blocks == planning_export_blocks,
            planned_export_within_planning: export_summary.planned_blocks <= planning_export_blocks,
        }
    }

    pub fn readiness_summary(
        self,
        planning: RuntimePlanningDigest,
        forward_vector: &[f32],
        summaries: &[TransformerForwardSummary],
    ) -> RuntimeKvExportReadinessSummary {
        let planning_summary = self.planning_summary(planning, forward_vector, summaries);
        let blocks = self.build_blocks(forward_vector, summaries);

        RuntimeKvExportReadinessSummary::from_blocks(planning_summary, &blocks)
    }

    pub fn readiness_summary_for_blocks(
        self,
        planning: RuntimePlanningDigest,
        forward_vector: &[f32],
        summaries: &[TransformerForwardSummary],
        blocks: &[KvBlock],
    ) -> RuntimeKvExportReadinessSummary {
        RuntimeKvExportReadinessSummary::from_blocks(
            self.planning_summary(planning, forward_vector, summaries),
            blocks,
        )
    }

    pub fn build_blocks(
        self,
        forward_vector: &[f32],
        summaries: &[TransformerForwardSummary],
    ) -> Vec<KvBlock> {
        let block_count = self.planned_block_count(forward_vector, summaries);
        if block_count == 0 {
            return Vec::new();
        }

        let (key, value) = split_forward_vector(forward_vector);

        (0..block_count)
            .map(|index| {
                let summary = summaries.get(index);
                let layer =
                    summary.map(|summary| summary.layer_index).unwrap_or(index) % self.layer_count;
                let head = summary
                    .map(|summary| {
                        let attention_offset = match summary.attention {
                            TransformerAttentionKind::Global => 0,
                            TransformerAttentionKind::LocalWindow => 2,
                            TransformerAttentionKind::Fusion => 4,
                        };
                        (attention_offset + summary.window_size + index) % self.kv_heads
                    })
                    .unwrap_or(index % self.kv_heads);
                let compute_scale = summary
                    .map(|summary| summary.compute_fraction + summary.activation)
                    .unwrap_or(1.0)
                    .clamp(0.25, 1.50);

                KvBlock::new(
                    index as u64,
                    KvNamespace::Runtime,
                    layer,
                    head,
                    index..index + 1,
                    scaled(&key, compute_scale + index as f32 * 0.02),
                    scaled(&value, compute_scale - index as f32 * 0.015),
                )
                .with_score((compute_scale / 1.50).clamp(0.0, 1.0))
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TransformerPlanDigest {
    pub template: Option<String>,
    pub layers: Vec<TransformerLayerBudget>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformerPlanSummary {
    pub layer_count: usize,
    pub counts: TransformerPlanCounts,
    pub average_compute_fraction: f32,
    pub min_window_size: usize,
    pub max_window_size: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformerPlanReadinessStage {
    RouteBudget,
    PlanSummary,
    LayerBudgets,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformerPlanReadinessSummary {
    pub route_budget: RouteBudget,
    pub plan_summary: TransformerPlanSummary,
    pub layer_batch_summary: TransformerLayerBudgetBatchSummary,
    pub route_budget_signal_component_count: usize,
    pub plan_summary_signal_component_count: usize,
    pub layer_budget_signal_component_count: usize,
    pub route_budget_blocker_component_count: usize,
    pub plan_summary_blocker_component_count: usize,
    pub layer_budget_blocker_component_count: usize,
}

impl TransformerPlanSummary {
    pub fn is_empty(self) -> bool {
        self.layer_count == 0
    }

    pub fn attention_layer_count(self) -> usize {
        self.counts.global.saturating_add(self.counts.fusion)
    }

    pub fn local_fraction(self) -> f32 {
        if self.layer_count == 0 {
            0.0
        } else {
            self.counts.local as f32 / self.layer_count as f32
        }
    }

    pub fn global_fraction(self) -> f32 {
        if self.layer_count == 0 {
            0.0
        } else {
            self.counts.global as f32 / self.layer_count as f32
        }
    }

    pub fn fusion_fraction(self) -> f32 {
        if self.layer_count == 0 {
            0.0
        } else {
            self.counts.fusion as f32 / self.layer_count as f32
        }
    }

    pub fn non_local_fraction(self) -> f32 {
        if self.layer_count == 0 {
            0.0
        } else {
            self.attention_layer_count() as f32 / self.layer_count as f32
        }
    }

    pub fn counts_match_layer_count(self) -> bool {
        self.counts.total() == self.layer_count
    }

    pub fn compute_shape_is_valid(self) -> bool {
        finite_unit(self.average_compute_fraction)
    }

    pub fn window_shape_is_valid(self) -> bool {
        if self.is_empty() {
            self.min_window_size == 0 && self.max_window_size == 0
        } else {
            self.min_window_size > 0 && self.min_window_size <= self.max_window_size
        }
    }

    pub fn fractions_are_valid(self) -> bool {
        finite_unit(self.local_fraction())
            && finite_unit(self.global_fraction())
            && finite_unit(self.fusion_fraction())
            && finite_unit(self.non_local_fraction())
    }

    pub fn plan_mix_signal_component_count(self) -> usize {
        usize::from(!self.is_empty())
            + usize::from(self.counts.local > 0)
            + usize::from(self.counts.global > 0)
            + usize::from(self.counts.fusion > 0)
    }

    pub fn plan_pressure_signal_component_count(self) -> usize {
        usize::from(self.attention_layer_count() > 0)
            + usize::from(self.non_local_fraction() > 0.0 && finite_unit(self.non_local_fraction()))
            + usize::from(self.fusion_fraction() > 0.0 && finite_unit(self.fusion_fraction()))
            + usize::from(
                self.average_compute_fraction > 0.0 && finite_unit(self.average_compute_fraction),
            )
    }

    pub fn plan_summary_signal_component_count(self) -> usize {
        self.plan_mix_signal_component_count()
            .saturating_add(self.plan_pressure_signal_component_count())
    }

    pub fn has_plan_summary_signal_components(self) -> bool {
        self.plan_summary_signal_component_count() > 0
    }

    pub fn plan_count_problem_component_count(self) -> usize {
        usize::from(!self.counts_match_layer_count())
    }

    pub fn plan_shape_problem_component_count(self) -> usize {
        usize::from(!self.compute_shape_is_valid())
            + usize::from(!self.window_shape_is_valid())
            + usize::from(!self.fractions_are_valid())
    }

    pub fn plan_summary_problem_component_count(self) -> usize {
        self.plan_count_problem_component_count()
            .saturating_add(self.plan_shape_problem_component_count())
    }

    pub fn has_plan_summary_problem_components(self) -> bool {
        self.plan_summary_problem_component_count() > 0
    }

    pub fn plan_summary_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .plan_mix_signal_component_count()
            .saturating_add(self.plan_pressure_signal_component_count());
        let expected_problem_count = self
            .plan_count_problem_component_count()
            .saturating_add(self.plan_shape_problem_component_count());

        self.plan_summary_signal_component_count() == expected_signal_count
            && self.plan_summary_problem_component_count() == expected_problem_count
    }

    pub fn plan_summary_shape_is_clean(self) -> bool {
        !self.has_plan_summary_problem_components() && self.plan_summary_accounting_is_consistent()
    }

    pub fn can_use_transformer_plan(self) -> bool {
        !self.is_empty() && self.plan_summary_shape_is_clean()
    }
}

impl TransformerPlanReadinessSummary {
    pub fn new(
        route_budget: RouteBudget,
        plan_summary: TransformerPlanSummary,
        layer_batch_summary: TransformerLayerBudgetBatchSummary,
    ) -> Self {
        Self {
            route_budget,
            plan_summary,
            layer_batch_summary,
            route_budget_signal_component_count: route_budget.route_budget_signal_component_count(),
            plan_summary_signal_component_count: plan_summary.plan_summary_signal_component_count(),
            layer_budget_signal_component_count: layer_batch_summary.signal_component_count,
            route_budget_blocker_component_count: route_budget
                .route_budget_problem_component_count(),
            plan_summary_blocker_component_count: plan_summary
                .plan_summary_problem_component_count(),
            layer_budget_blocker_component_count: layer_batch_summary.problem_component_count,
        }
    }

    pub fn from_digest(route_budget: RouteBudget, digest: &TransformerPlanDigest) -> Self {
        Self::new(
            route_budget,
            digest.plan_summary(),
            digest.layer_batch_summary(),
        )
    }

    pub fn stage_order() -> [TransformerPlanReadinessStage; 3] {
        [
            TransformerPlanReadinessStage::RouteBudget,
            TransformerPlanReadinessStage::PlanSummary,
            TransformerPlanReadinessStage::LayerBudgets,
        ]
    }

    pub fn route_budget_ready(self) -> bool {
        self.route_budget.can_use_route_budget()
    }

    pub fn plan_summary_ready(self) -> bool {
        self.plan_summary.can_use_transformer_plan()
    }

    pub fn layer_budgets_ready(self) -> bool {
        self.layer_batch_summary
            .can_use_transformer_layer_budget_batch()
    }

    pub fn stage_ready(self, stage: TransformerPlanReadinessStage) -> bool {
        match stage {
            TransformerPlanReadinessStage::RouteBudget => self.route_budget_ready(),
            TransformerPlanReadinessStage::PlanSummary => self.plan_summary_ready(),
            TransformerPlanReadinessStage::LayerBudgets => self.layer_budgets_ready(),
        }
    }

    pub fn stage_signal_component_count(self, stage: TransformerPlanReadinessStage) -> usize {
        match stage {
            TransformerPlanReadinessStage::RouteBudget => self.route_budget_signal_component_count,
            TransformerPlanReadinessStage::PlanSummary => self.plan_summary_signal_component_count,
            TransformerPlanReadinessStage::LayerBudgets => self.layer_budget_signal_component_count,
        }
    }

    pub fn stage_blocker_component_count(self, stage: TransformerPlanReadinessStage) -> usize {
        match stage {
            TransformerPlanReadinessStage::RouteBudget => self.route_budget_blocker_component_count,
            TransformerPlanReadinessStage::PlanSummary => self.plan_summary_blocker_component_count,
            TransformerPlanReadinessStage::LayerBudgets => {
                self.layer_budget_blocker_component_count
            }
        }
    }

    pub fn first_unready_stage(self) -> Option<TransformerPlanReadinessStage> {
        Self::stage_order()
            .into_iter()
            .find(|stage| !self.stage_ready(*stage))
    }

    pub fn first_blocking_stage(self) -> Option<TransformerPlanReadinessStage> {
        Self::stage_order()
            .into_iter()
            .find(|stage| self.stage_blocker_component_count(*stage) > 0)
    }

    pub fn transformer_plan_readiness_signal_component_count(self) -> usize {
        self.route_budget_signal_component_count
            .saturating_add(self.plan_summary_signal_component_count)
            .saturating_add(self.layer_budget_signal_component_count)
    }

    pub fn has_transformer_plan_readiness_signals(self) -> bool {
        self.transformer_plan_readiness_signal_component_count() > 0
    }

    pub fn transformer_plan_readiness_blocker_component_count(self) -> usize {
        self.route_budget_blocker_component_count
            .saturating_add(self.plan_summary_blocker_component_count)
            .saturating_add(self.layer_budget_blocker_component_count)
    }

    pub fn has_transformer_plan_readiness_blockers(self) -> bool {
        self.transformer_plan_readiness_blocker_component_count() > 0
    }

    pub fn transformer_plan_readiness_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .route_budget_signal_component_count
            .saturating_add(self.plan_summary_signal_component_count)
            .saturating_add(self.layer_budget_signal_component_count);
        let expected_blocker_count = self
            .route_budget_blocker_component_count
            .saturating_add(self.plan_summary_blocker_component_count)
            .saturating_add(self.layer_budget_blocker_component_count);

        self.route_budget.route_budget_accounting_is_consistent()
            && self.plan_summary.plan_summary_accounting_is_consistent()
            && self
                .layer_batch_summary
                .layer_batch_accounting_is_consistent()
            && self.route_budget_signal_component_count
                == self.route_budget.route_budget_signal_component_count()
            && self.plan_summary_signal_component_count
                == self.plan_summary.plan_summary_signal_component_count()
            && self.layer_budget_signal_component_count
                == self.layer_batch_summary.signal_component_count
            && self.route_budget_blocker_component_count
                == self.route_budget.route_budget_problem_component_count()
            && self.plan_summary_blocker_component_count
                == self.plan_summary.plan_summary_problem_component_count()
            && self.layer_budget_blocker_component_count
                == self.layer_batch_summary.problem_component_count
            && self.transformer_plan_readiness_signal_component_count() == expected_signal_count
            && self.has_transformer_plan_readiness_signals() == (expected_signal_count > 0)
            && self.transformer_plan_readiness_blocker_component_count() == expected_blocker_count
            && self.has_transformer_plan_readiness_blockers() == (expected_blocker_count > 0)
    }

    pub fn transformer_plan_readiness_is_clean(self) -> bool {
        !self.has_transformer_plan_readiness_blockers()
            && self.transformer_plan_readiness_accounting_is_consistent()
    }

    pub fn can_commit_transformer_plan_readiness(self) -> bool {
        self.transformer_plan_readiness_is_clean()
            && self.route_budget_ready()
            && self.plan_summary_ready()
            && self.layer_budgets_ready()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformerPlanningPressureSummary {
    pub route_attention_fraction: f32,
    pub route_attention_tokens: usize,
    pub route_fast_tokens: usize,
    pub attention_selection_fraction: f32,
    pub selected_attention_tokens: usize,
    pub rejected_attention_tokens: usize,
    pub hit_attention_selection_cap: bool,
    pub transformer_layer_count: usize,
    pub transformer_non_local_fraction: f32,
    pub transformer_fusion_fraction: f32,
    pub transformer_average_compute_fraction: f32,
    pub route_to_selection_delta: f32,
    pub route_to_non_local_delta: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformerPlanningReadinessStage {
    RouteBudget,
    AttentionSelection,
    PlanningPressure,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformerPlanningReadinessSummary {
    pub route_budget: RouteBudgetReadinessSummary,
    pub attention_selection: AttentionSelectionReadinessSummary,
    pub planning_pressure: TransformerPlanningPressureSummary,
    pub route_budget_signal_component_count: usize,
    pub attention_selection_signal_component_count: usize,
    pub planning_pressure_signal_component_count: usize,
    pub route_budget_blocker_component_count: usize,
    pub attention_selection_blocker_component_count: usize,
    pub planning_pressure_blocker_component_count: usize,
}

impl TransformerPlanningPressureSummary {
    pub fn from_parts(
        route_budget: RouteBudget,
        attention_summary: AttentionDecisionSummary,
        transformer_summary: TransformerPlanSummary,
    ) -> Self {
        let route_attention_fraction = route_budget.attention_fraction.clamp(0.0, 1.0);
        let attention_selection_fraction = attention_summary.selection_fraction.clamp(0.0, 1.0);
        let transformer_non_local_fraction = transformer_summary.non_local_fraction();
        let transformer_fusion_fraction = transformer_summary.fusion_fraction();

        Self {
            route_attention_fraction,
            route_attention_tokens: route_budget.attention_tokens,
            route_fast_tokens: route_budget.fast_tokens,
            attention_selection_fraction,
            selected_attention_tokens: attention_summary.selected_attention_tokens(),
            rejected_attention_tokens: attention_summary.rejected_attention_tokens(),
            hit_attention_selection_cap: attention_summary.hit_selection_cap,
            transformer_layer_count: transformer_summary.layer_count,
            transformer_non_local_fraction,
            transformer_fusion_fraction,
            transformer_average_compute_fraction: transformer_summary.average_compute_fraction,
            route_to_selection_delta: route_attention_fraction - attention_selection_fraction,
            route_to_non_local_delta: route_attention_fraction - transformer_non_local_fraction,
        }
    }

    pub fn attention_selection_is_clamped(self) -> bool {
        self.hit_attention_selection_cap
    }

    pub fn has_attention_rejections(self) -> bool {
        self.rejected_attention_tokens > 0
    }

    pub fn transformer_uses_fusion(self) -> bool {
        self.transformer_fusion_fraction > 0.0
    }

    pub fn route_and_transformer_diverge(self, tolerance: f32) -> bool {
        self.route_to_non_local_delta.abs() > tolerance.max(0.0)
    }

    pub fn pressure_values_are_finite(self) -> bool {
        self.route_attention_fraction.is_finite()
            && self.attention_selection_fraction.is_finite()
            && self.transformer_non_local_fraction.is_finite()
            && self.transformer_fusion_fraction.is_finite()
            && self.transformer_average_compute_fraction.is_finite()
            && self.route_to_selection_delta.is_finite()
            && self.route_to_non_local_delta.is_finite()
    }

    pub fn pressure_fractions_are_unit(self) -> bool {
        finite_unit(self.route_attention_fraction)
            && finite_unit(self.attention_selection_fraction)
            && finite_unit(self.transformer_non_local_fraction)
            && finite_unit(self.transformer_fusion_fraction)
            && finite_unit(self.transformer_average_compute_fraction)
    }

    pub fn route_token_pressure_signal_component_count(self) -> usize {
        usize::from(self.route_attention_tokens > 0)
            + usize::from(self.route_fast_tokens > 0)
            + usize::from(
                self.route_attention_fraction > 0.0 && finite_unit(self.route_attention_fraction),
            )
    }

    pub fn attention_selection_pressure_signal_component_count(self) -> usize {
        usize::from(self.selected_attention_tokens > 0)
            + usize::from(self.has_attention_rejections())
            + usize::from(self.attention_selection_is_clamped())
            + usize::from(
                self.attention_selection_fraction > 0.0
                    && finite_unit(self.attention_selection_fraction),
            )
    }

    pub fn transformer_mix_pressure_signal_component_count(self) -> usize {
        usize::from(self.transformer_layer_count > 0)
            + usize::from(
                self.transformer_non_local_fraction > 0.0
                    && finite_unit(self.transformer_non_local_fraction),
            )
            + usize::from(self.transformer_uses_fusion())
            + usize::from(
                self.transformer_average_compute_fraction > 0.0
                    && finite_unit(self.transformer_average_compute_fraction),
            )
    }

    pub fn planning_pressure_signal_component_count(self) -> usize {
        self.route_token_pressure_signal_component_count()
            .saturating_add(self.attention_selection_pressure_signal_component_count())
            .saturating_add(self.transformer_mix_pressure_signal_component_count())
    }

    pub fn has_planning_pressure_signals(self) -> bool {
        self.planning_pressure_signal_component_count() > 0
    }

    pub fn planning_pressure_shape_problem_component_count(self) -> usize {
        usize::from(!self.pressure_values_are_finite())
            + usize::from(!self.pressure_fractions_are_unit())
    }

    pub fn planning_pressure_delta_problem_component_count(self) -> usize {
        let expected_selection_delta =
            self.route_attention_fraction - self.attention_selection_fraction;
        let expected_non_local_delta =
            self.route_attention_fraction - self.transformer_non_local_fraction;

        usize::from(
            self.pressure_values_are_finite()
                && !float_close(self.route_to_selection_delta, expected_selection_delta),
        ) + usize::from(
            self.pressure_values_are_finite()
                && !float_close(self.route_to_non_local_delta, expected_non_local_delta),
        )
    }

    pub fn planning_pressure_problem_component_count(self) -> usize {
        self.planning_pressure_shape_problem_component_count()
            .saturating_add(self.planning_pressure_delta_problem_component_count())
    }

    pub fn has_planning_pressure_problem_components(self) -> bool {
        self.planning_pressure_problem_component_count() > 0
    }

    pub fn planning_pressure_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .route_token_pressure_signal_component_count()
            .saturating_add(self.attention_selection_pressure_signal_component_count())
            .saturating_add(self.transformer_mix_pressure_signal_component_count());
        let expected_problem_count = self
            .planning_pressure_shape_problem_component_count()
            .saturating_add(self.planning_pressure_delta_problem_component_count());

        self.planning_pressure_signal_component_count() == expected_signal_count
            && self.planning_pressure_problem_component_count() == expected_problem_count
    }

    pub fn planning_pressure_shape_is_clean(self) -> bool {
        !self.has_planning_pressure_problem_components()
            && self.planning_pressure_accounting_is_consistent()
    }

    pub fn can_use_planning_pressure(self) -> bool {
        self.planning_pressure_shape_is_clean()
    }
}

impl TransformerPlanningReadinessSummary {
    pub fn new(
        route_budget: RouteBudgetReadinessSummary,
        attention_selection: AttentionSelectionReadinessSummary,
        planning_pressure: TransformerPlanningPressureSummary,
    ) -> Self {
        Self {
            route_budget,
            attention_selection,
            planning_pressure,
            route_budget_signal_component_count: route_budget
                .route_budget_readiness_signal_component_count(),
            attention_selection_signal_component_count: attention_selection
                .attention_selection_readiness_signal_component_count(),
            planning_pressure_signal_component_count: planning_pressure
                .planning_pressure_signal_component_count()
                .saturating_add(usize::from(Self::planning_pressure_matches_parts(
                    route_budget,
                    attention_selection,
                    planning_pressure,
                ))),
            route_budget_blocker_component_count: route_budget
                .route_budget_readiness_blocker_component_count(),
            attention_selection_blocker_component_count: attention_selection
                .attention_selection_readiness_blocker_component_count(),
            planning_pressure_blocker_component_count: planning_pressure
                .planning_pressure_problem_component_count()
                .saturating_add(
                    Self::planning_pressure_boundary_drift_component_count_parts(
                        route_budget,
                        attention_selection,
                        planning_pressure,
                    ),
                ),
        }
    }

    pub fn stage_order() -> [TransformerPlanningReadinessStage; 3] {
        [
            TransformerPlanningReadinessStage::RouteBudget,
            TransformerPlanningReadinessStage::AttentionSelection,
            TransformerPlanningReadinessStage::PlanningPressure,
        ]
    }

    pub fn planning_pressure_matches_route_budget(self) -> bool {
        self.planning_pressure.route_attention_tokens
            == self.route_budget.route_budget.attention_tokens
            && self.planning_pressure.route_fast_tokens
                == self.route_budget.route_budget.fast_tokens
            && float_close(
                self.planning_pressure.route_attention_fraction,
                self.route_budget.route_budget.attention_fraction,
            )
    }

    pub fn planning_pressure_matches_attention_selection(self) -> bool {
        self.planning_pressure.selected_attention_tokens
            == self
                .attention_selection
                .decision
                .selected_attention_tokens()
            && self.planning_pressure.rejected_attention_tokens
                == self
                    .attention_selection
                    .decision
                    .rejected_attention_tokens()
            && self.planning_pressure.hit_attention_selection_cap
                == self.attention_selection.decision.hit_selection_cap
            && float_close(
                self.planning_pressure.attention_selection_fraction,
                self.attention_selection.decision.selection_fraction,
            )
    }

    pub fn planning_pressure_boundary_matches(self) -> bool {
        Self::planning_pressure_matches_parts(
            self.route_budget,
            self.attention_selection,
            self.planning_pressure,
        )
    }

    pub fn planning_pressure_boundary_drift_component_count(self) -> usize {
        Self::planning_pressure_boundary_drift_component_count_parts(
            self.route_budget,
            self.attention_selection,
            self.planning_pressure,
        )
    }

    pub fn route_budget_ready(self) -> bool {
        self.route_budget.can_commit_route_budget_readiness()
    }

    pub fn attention_selection_ready(self) -> bool {
        self.attention_selection
            .can_commit_attention_selection_readiness()
    }

    pub fn attention_selection_commit_summary(self) -> AttentionSelectionReadinessCommitSummary {
        self.attention_selection.commit_summary()
    }

    pub fn can_use_committed_attention_selection_for_planning(self) -> bool {
        self.attention_selection_commit_summary()
            .can_use_committed_attention_decision()
    }

    pub fn planning_pressure_ready(self) -> bool {
        self.planning_pressure.can_use_planning_pressure()
            && self.planning_pressure_boundary_matches()
    }

    pub fn stage_ready(self, stage: TransformerPlanningReadinessStage) -> bool {
        match stage {
            TransformerPlanningReadinessStage::RouteBudget => self.route_budget_ready(),
            TransformerPlanningReadinessStage::AttentionSelection => {
                self.attention_selection_ready()
            }
            TransformerPlanningReadinessStage::PlanningPressure => self.planning_pressure_ready(),
        }
    }

    pub fn stage_signal_component_count(self, stage: TransformerPlanningReadinessStage) -> usize {
        match stage {
            TransformerPlanningReadinessStage::RouteBudget => {
                self.route_budget_signal_component_count
            }
            TransformerPlanningReadinessStage::AttentionSelection => {
                self.attention_selection_signal_component_count
            }
            TransformerPlanningReadinessStage::PlanningPressure => {
                self.planning_pressure_signal_component_count
            }
        }
    }

    pub fn stage_blocker_component_count(self, stage: TransformerPlanningReadinessStage) -> usize {
        match stage {
            TransformerPlanningReadinessStage::RouteBudget => {
                self.route_budget_blocker_component_count
            }
            TransformerPlanningReadinessStage::AttentionSelection => {
                self.attention_selection_blocker_component_count
            }
            TransformerPlanningReadinessStage::PlanningPressure => {
                self.planning_pressure_blocker_component_count
            }
        }
    }

    pub fn first_unready_stage(self) -> Option<TransformerPlanningReadinessStage> {
        Self::stage_order()
            .into_iter()
            .find(|stage| !self.stage_ready(*stage))
    }

    pub fn first_blocking_stage(self) -> Option<TransformerPlanningReadinessStage> {
        Self::stage_order()
            .into_iter()
            .find(|stage| self.stage_blocker_component_count(*stage) > 0)
    }

    pub fn transformer_planning_readiness_signal_component_count(self) -> usize {
        self.route_budget_signal_component_count
            .saturating_add(self.attention_selection_signal_component_count)
            .saturating_add(self.planning_pressure_signal_component_count)
    }

    pub fn has_transformer_planning_readiness_signals(self) -> bool {
        self.transformer_planning_readiness_signal_component_count() > 0
    }

    pub fn transformer_planning_readiness_blocker_component_count(self) -> usize {
        self.route_budget_blocker_component_count
            .saturating_add(self.attention_selection_blocker_component_count)
            .saturating_add(self.planning_pressure_blocker_component_count)
    }

    pub fn has_transformer_planning_readiness_blockers(self) -> bool {
        self.transformer_planning_readiness_blocker_component_count() > 0
    }

    pub fn transformer_planning_readiness_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .route_budget_signal_component_count
            .saturating_add(self.attention_selection_signal_component_count)
            .saturating_add(self.planning_pressure_signal_component_count);
        let expected_blocker_count = self
            .route_budget_blocker_component_count
            .saturating_add(self.attention_selection_blocker_component_count)
            .saturating_add(self.planning_pressure_blocker_component_count);
        let expected_pressure_signal_count = self
            .planning_pressure
            .planning_pressure_signal_component_count()
            .saturating_add(usize::from(self.planning_pressure_boundary_matches()));
        let expected_pressure_blocker_count = self
            .planning_pressure
            .planning_pressure_problem_component_count()
            .saturating_add(self.planning_pressure_boundary_drift_component_count());

        self.route_budget
            .route_budget_readiness_accounting_is_consistent()
            && self
                .attention_selection
                .attention_selection_readiness_accounting_is_consistent()
            && self
                .planning_pressure
                .planning_pressure_accounting_is_consistent()
            && self.route_budget_signal_component_count
                == self
                    .route_budget
                    .route_budget_readiness_signal_component_count()
            && self.attention_selection_signal_component_count
                == self
                    .attention_selection
                    .attention_selection_readiness_signal_component_count()
            && self.planning_pressure_signal_component_count == expected_pressure_signal_count
            && self.route_budget_blocker_component_count
                == self
                    .route_budget
                    .route_budget_readiness_blocker_component_count()
            && self.attention_selection_blocker_component_count
                == self
                    .attention_selection
                    .attention_selection_readiness_blocker_component_count()
            && self.planning_pressure_blocker_component_count == expected_pressure_blocker_count
            && self.transformer_planning_readiness_signal_component_count() == expected_signal_count
            && self.has_transformer_planning_readiness_signals() == (expected_signal_count > 0)
            && self.transformer_planning_readiness_blocker_component_count()
                == expected_blocker_count
            && self.has_transformer_planning_readiness_blockers() == (expected_blocker_count > 0)
    }

    pub fn transformer_planning_readiness_is_clean(self) -> bool {
        !self.has_transformer_planning_readiness_blockers()
            && self.transformer_planning_readiness_accounting_is_consistent()
    }

    pub fn can_commit_transformer_planning_readiness(self) -> bool {
        self.transformer_planning_readiness_is_clean()
            && self.route_budget_ready()
            && self.attention_selection_ready()
            && self.planning_pressure_ready()
    }

    fn planning_pressure_matches_parts(
        route_budget: RouteBudgetReadinessSummary,
        attention_selection: AttentionSelectionReadinessSummary,
        planning_pressure: TransformerPlanningPressureSummary,
    ) -> bool {
        planning_pressure.route_attention_tokens == route_budget.route_budget.attention_tokens
            && planning_pressure.route_fast_tokens == route_budget.route_budget.fast_tokens
            && float_close(
                planning_pressure.route_attention_fraction,
                route_budget.route_budget.attention_fraction,
            )
            && planning_pressure.selected_attention_tokens
                == attention_selection.decision.selected_attention_tokens()
            && planning_pressure.rejected_attention_tokens
                == attention_selection.decision.rejected_attention_tokens()
            && planning_pressure.hit_attention_selection_cap
                == attention_selection.decision.hit_selection_cap
            && float_close(
                planning_pressure.attention_selection_fraction,
                attention_selection.decision.selection_fraction,
            )
    }

    fn planning_pressure_boundary_drift_component_count_parts(
        route_budget: RouteBudgetReadinessSummary,
        attention_selection: AttentionSelectionReadinessSummary,
        planning_pressure: TransformerPlanningPressureSummary,
    ) -> usize {
        usize::from(
            planning_pressure.route_attention_tokens != route_budget.route_budget.attention_tokens,
        )
        .saturating_add(usize::from(
            planning_pressure.route_fast_tokens != route_budget.route_budget.fast_tokens,
        ))
        .saturating_add(usize::from(!float_close(
            planning_pressure.route_attention_fraction,
            route_budget.route_budget.attention_fraction,
        )))
        .saturating_add(usize::from(
            planning_pressure.selected_attention_tokens
                != attention_selection.decision.selected_attention_tokens(),
        ))
        .saturating_add(usize::from(
            planning_pressure.rejected_attention_tokens
                != attention_selection.decision.rejected_attention_tokens(),
        ))
        .saturating_add(usize::from(
            planning_pressure.hit_attention_selection_cap
                != attention_selection.decision.hit_selection_cap,
        ))
        .saturating_add(usize::from(!float_close(
            planning_pressure.attention_selection_fraction,
            attention_selection.decision.selection_fraction,
        )))
    }
}

impl TransformerPlanDigest {
    pub fn new(template: Option<impl Into<String>>, layers: Vec<TransformerLayerBudget>) -> Self {
        Self {
            template: template.map(Into::into),
            layers,
        }
    }

    pub fn layer_summaries(&self) -> Vec<TransformerLayerBudgetSummary> {
        self.layers
            .iter()
            .map(TransformerLayerBudget::layer_summary)
            .collect()
    }

    pub fn layer_batch_summary(&self) -> TransformerLayerBudgetBatchSummary {
        TransformerLayerBudgetBatchSummary::from_summaries(&self.layer_summaries())
    }

    pub fn from_route_budget(route_budget: RouteBudget, layer_count: usize) -> Self {
        let layer_count = layer_count.max(1);
        let attention_fraction = route_budget.attention_fraction.clamp(0.0, 1.0);
        let fusion_stride = if attention_fraction >= 0.70 { 3 } else { 4 };
        let mut layers = Vec::with_capacity(layer_count);

        for layer_index in 0..layer_count {
            let attention = if attention_fraction >= 0.85 {
                TransformerAttentionKind::Global
            } else if layer_index % fusion_stride == fusion_stride - 1 {
                TransformerAttentionKind::Fusion
            } else {
                TransformerAttentionKind::LocalWindow
            };
            let compute_fraction = compute_fraction(attention, attention_fraction);
            let window_size = window_size(attention, attention_fraction);

            layers.push(TransformerLayerBudget::new(
                layer_index,
                attention,
                compute_fraction,
                window_size,
            ));
        }

        Self::new(Some("route-budget-digest"), layers)
    }

    pub fn counts(&self) -> TransformerPlanCounts {
        let mut counts = TransformerPlanCounts::default();

        for layer in &self.layers {
            match layer.attention {
                TransformerAttentionKind::Global => counts.global += 1,
                TransformerAttentionKind::LocalWindow => counts.local += 1,
                TransformerAttentionKind::Fusion => counts.fusion += 1,
            }
        }

        counts
    }

    pub fn plan_summary(&self) -> TransformerPlanSummary {
        let counts = self.counts();
        let layer_count = self.layers.len();
        let total_compute = self
            .layers
            .iter()
            .map(|layer| layer.compute_fraction)
            .sum::<f32>();
        let average_compute_fraction = if layer_count == 0 {
            0.0
        } else {
            total_compute / layer_count as f32
        };
        let min_window_size = self
            .layers
            .iter()
            .map(|layer| layer.window_size)
            .min()
            .unwrap_or(0);
        let max_window_size = self
            .layers
            .iter()
            .map(|layer| layer.window_size)
            .max()
            .unwrap_or(0);

        TransformerPlanSummary {
            layer_count,
            counts,
            average_compute_fraction,
            min_window_size,
            max_window_size,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TransformerPlanCounts {
    pub global: usize,
    pub local: usize,
    pub fusion: usize,
}

impl TransformerPlanCounts {
    pub fn total(self) -> usize {
        self.global
            .saturating_add(self.local)
            .saturating_add(self.fusion)
    }
}

fn adjusted_weights(
    template: TransformerTemplate,
    mut hierarchy: HierarchyWeights,
) -> HierarchyWeights {
    hierarchy.global += template.global_bias;
    hierarchy.local += template.local_bias;
    hierarchy.fusion += template.fusion_bias;
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
    fusion_left: &mut usize,
) -> TransformerAttentionKind {
    let early_or_late = layer_index == 0 || layer_index + 1 == layer_count;

    if early_or_late && *global_left > 0 {
        *global_left -= 1;
        return TransformerAttentionKind::Global;
    }
    if layer_index % 4 == 3 && *fusion_left > 0 {
        *fusion_left -= 1;
        return TransformerAttentionKind::Fusion;
    }
    if *local_left > 0 {
        *local_left -= 1;
        return TransformerAttentionKind::LocalWindow;
    }
    if *global_left > 0 {
        *global_left -= 1;
        return TransformerAttentionKind::Global;
    }
    if *fusion_left > 0 {
        *fusion_left -= 1;
        return TransformerAttentionKind::Fusion;
    }
    TransformerAttentionKind::LocalWindow
}

fn planned_compute_fraction(attention: TransformerAttentionKind, route_budget: RouteBudget) -> f32 {
    let attention_pressure = route_budget.attention_fraction.clamp(0.0, 1.0);
    match attention {
        TransformerAttentionKind::Global => 0.65 + attention_pressure * 0.35,
        TransformerAttentionKind::LocalWindow => 0.45 + attention_pressure * 0.25,
        TransformerAttentionKind::Fusion => 0.30 + attention_pressure * 0.20,
    }
    .clamp(0.0, 1.0)
}

fn planned_window_size(
    attention: TransformerAttentionKind,
    base: usize,
    route_budget: RouteBudget,
    template: TransformerTemplate,
) -> usize {
    let multiplier = match attention {
        TransformerAttentionKind::Global => template.global_window_scale as f64,
        TransformerAttentionKind::LocalWindow => {
            template.local_window_scale as f64 * (1.0 + route_budget.attention_fraction as f64)
        }
        TransformerAttentionKind::Fusion => template.fusion_window_scale as f64,
    };
    ((base.max(16) as f64 * multiplier).round() as usize).max(16)
}

fn compute_fraction(attention: TransformerAttentionKind, attention_fraction: f32) -> f32 {
    match attention {
        TransformerAttentionKind::Global => 0.65 + attention_fraction * 0.25,
        TransformerAttentionKind::LocalWindow => 0.45 + attention_fraction * 0.20,
        TransformerAttentionKind::Fusion => 0.30 + attention_fraction * 0.18,
    }
    .clamp(0.0, 1.0)
}

fn window_size(attention: TransformerAttentionKind, attention_fraction: f32) -> usize {
    match attention {
        TransformerAttentionKind::Global => 8192,
        TransformerAttentionKind::LocalWindow => {
            if attention_fraction >= 0.70 {
                4096
            } else {
                2048
            }
        }
        TransformerAttentionKind::Fusion => 1024,
    }
}

fn average(total: f32, count: usize) -> f32 {
    if count == 0 {
        0.0
    } else {
        total / count as f32
    }
}

fn finite_unit(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn float_close(left: f32, right: f32) -> bool {
    (left - right).abs() <= 0.0001
}

fn split_forward_vector(vector: &[f32]) -> (Vec<f32>, Vec<f32>) {
    let midpoint = (vector.len() / 2).max(1);
    let key = vector
        .iter()
        .copied()
        .map(sanitize_forward_value)
        .take(midpoint)
        .collect::<Vec<_>>();
    let value = vector
        .iter()
        .copied()
        .map(sanitize_forward_value)
        .skip(midpoint)
        .collect::<Vec<_>>();
    let value = if value.is_empty() { key.clone() } else { value };
    let paired_len = key.len().min(value.len()).max(1);

    (
        key.into_iter().take(paired_len).collect(),
        value.into_iter().take(paired_len).collect(),
    )
}

fn sanitize_forward_value(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

fn scaled(values: &[f32], scale: f32) -> Vec<f32> {
    values.iter().map(|value| value * scale).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::{AdapterExecutionContext, RuntimeAdapter};
    use crate::attention::{
        AttentionCandidate, AttentionDecision, AttentionPolicy, AttentionSelectionReadinessSummary,
        ThresholdAttentionPolicy,
    };
    use crate::engine::InferenceRequest;
    use crate::experiment::ExperimentSwitches;
    use crate::fht_dke::DeterministicFhtDkeBudgeter;
    use crate::router::{
        RouteBudgetReadinessSummary, RouteLayer, RouteLayerCounts, RoutingContext,
        RoutingDecisionSummary,
    };

    fn runtime_kv_export_fixture(
        max_export_blocks: usize,
    ) -> (
        RuntimeMetadata,
        TransformerRuntimeArchitecture,
        RuntimePlanningDigest,
    ) {
        let runtime = RuntimeMetadata::new("model", "tok", 4096, 4)
            .with_kv_exchange(false, true)
            .with_kv_limits(0, max_export_blocks);
        let architecture = TransformerRuntimeArchitecture::new(4, 4, 4, 2, 128);
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(64)
            .with_max_tokens(32)
            .with_runtime(runtime.clone())
            .with_experiments(ExperimentSwitches::default().with_fht_dke(true));
        let planning = crate::planning::RuntimePlanningDigest::from_request(
            &request,
            RouteBudget {
                threshold: 0.50,
                attention_tokens: 8,
                fast_tokens: 2,
                attention_fraction: 0.80,
            },
            &AdapterExecutionContext::new([RuntimeAdapter::Cuda]),
            &[],
            &DeterministicFhtDkeBudgeter::new(0.12, 0.58, 1),
        );

        (runtime, architecture, planning)
    }

    #[test]
    fn runtime_kv_export_plan_from_manifest_uses_manifest_policy_limit() {
        let metadata = RuntimeMetadata::new("manifest", "tok", 4096, 2048)
            .with_kv_exchange(false, true)
            .with_kv_limits(0, 8);
        let manifest = crate::manifest::RuntimeManifestDigest::from_metadata(metadata)
            .with_architecture(TransformerRuntimeArchitecture::new(6, 2048, 8, 2, 256))
            .with_kv_policy(
                crate::manifest::RuntimeKvPolicy::from_capabilities(false, true).with_limits(0, 2),
            );

        let plan = RuntimeKvExportPlan::from_manifest(&manifest, 5);
        let summary = RuntimeKvExportPlan::manifest_plan_summary(&manifest, 5);

        assert_eq!(plan.max_blocks, 2);
        assert_eq!(plan.layer_count, 6);
        assert_eq!(plan.kv_heads, 2);
        assert!(summary.manifest_allows_export());
        assert!(summary.runtime_allows_export());
        assert!(summary.requested_export());
        assert!(summary.plan_will_export());
        assert!(summary.architecture_has_export_shape());
        assert!(summary.manifest_export_capability_is_consistent());
        assert!(summary.runtime_export_capability_is_consistent());
        assert!(summary.export_plan_within_manifest_limit());
        assert!(summary.export_plan_within_runtime_limit());
        assert!(summary.export_plan_within_requested_limit());
        assert_eq!(summary.manifest_bridge_signal_component_count(), 8);
        assert!(summary.has_manifest_bridge_signals());
        assert_eq!(summary.manifest_bridge_problem_component_count(), 0);
        assert!(!summary.has_manifest_bridge_problem_components());
        assert!(summary.manifest_bridge_accounting_is_consistent());
        assert!(summary.manifest_bridge_shape_is_clean());
        assert!(summary.can_use_manifest_runtime_kv_export_plan());
    }

    #[test]
    fn runtime_kv_export_manifest_plan_summary_reports_export_capacity_drift() {
        let metadata = RuntimeMetadata::new("manifest", "tok", 4096, 2048)
            .with_kv_exchange(false, true)
            .with_kv_limits(0, 8);
        let manifest = crate::manifest::RuntimeManifestDigest::from_metadata(metadata)
            .with_architecture(TransformerRuntimeArchitecture::new(6, 2048, 8, 2, 256))
            .with_kv_policy(crate::manifest::RuntimeKvPolicy {
                import_enabled: false,
                export_enabled: true,
                max_import_blocks: 0,
                max_export_blocks: 0,
            });

        let plan = RuntimeKvExportPlan::from_manifest(&manifest, 2);
        let summary = RuntimeKvExportPlan::manifest_plan_summary(&manifest, 2);

        assert_eq!(plan.max_blocks, 0);
        assert!(!summary.manifest_allows_export());
        assert!(summary.runtime_allows_export());
        assert!(summary.requested_export());
        assert!(!summary.plan_will_export());
        assert!(!summary.manifest_export_capability_is_consistent());
        assert!(summary.runtime_export_capability_is_consistent());
        assert!(summary.requested_export_without_manifest_capacity());
        assert!(!summary.requested_export_without_runtime_capacity());
        assert_eq!(
            summary.manifest_export_capability_problem_component_count(),
            1
        );
        assert_eq!(
            summary.runtime_export_capability_problem_component_count(),
            0
        );
        assert_eq!(
            summary.requested_export_capacity_problem_component_count(),
            1
        );
        assert_eq!(summary.export_plan_limit_problem_component_count(), 0);
        assert_eq!(
            summary.architecture_export_shape_problem_component_count(),
            0
        );
        assert_eq!(summary.manifest_bridge_problem_component_count(), 2);
        assert!(summary.has_manifest_bridge_problem_components());
        assert!(summary.manifest_bridge_accounting_is_consistent());
        assert!(!summary.manifest_bridge_shape_is_clean());
        assert!(!summary.can_use_manifest_runtime_kv_export_plan());
    }

    #[test]
    fn runtime_kv_export_plan_from_manifest_feeds_readiness_gate() {
        let manifest =
            crate::manifest::RuntimeManifestDigest::self_developed("manifest", "tok", 4096, 2048)
                .with_architecture(TransformerRuntimeArchitecture::new(6, 2048, 8, 2, 256))
                .with_kv_policy(
                    crate::manifest::RuntimeKvPolicy::from_capabilities(false, true)
                        .with_limits(0, 2),
                );
        let runtime = manifest.runtime_metadata();
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(64)
            .with_max_tokens(32)
            .with_runtime(runtime)
            .with_experiments(ExperimentSwitches::default().with_fht_dke(true));
        let planning = crate::planning::RuntimePlanningDigest::from_request(
            &request,
            RouteBudget {
                threshold: 0.50,
                attention_tokens: 8,
                fast_tokens: 2,
                attention_fraction: 0.80,
            },
            &AdapterExecutionContext::new([RuntimeAdapter::Cuda]),
            &[],
            &DeterministicFhtDkeBudgeter::new(0.12, 0.58, 1),
        );
        let export_plan =
            RuntimeKvExportPlan::from_manifest(&manifest, planning.planned_kv_export_blocks);
        let summaries = [
            TransformerForwardSummary::new(0, TransformerAttentionKind::Global, 8, 0.50, 0.25),
            TransformerForwardSummary::new(1, TransformerAttentionKind::Fusion, 16, 0.50, 0.25),
        ];
        let forward = [1.0, 2.0, 3.0, 4.0];

        let bridge = RuntimeKvExportPlan::manifest_plan_summary(
            &manifest,
            planning.planned_kv_export_blocks,
        );
        let readiness = export_plan.readiness_summary(planning, &forward, &summaries);

        assert!(bridge.can_use_manifest_runtime_kv_export_plan());
        assert_eq!(export_plan.max_blocks, 2);
        assert_eq!(readiness.planning_summary.export_plan_max_blocks, 2);
        assert_eq!(readiness.planning_summary.export_summary.planned_blocks, 2);
        assert!(readiness.export_will_emit());
        assert_eq!(readiness.first_blocking_stage(), None);
        assert!(readiness.can_commit_runtime_kv_export_readiness());
    }

    #[test]
    fn route_budget_digest_counts_attention_kinds() {
        let digest = TransformerPlanDigest::from_route_budget(
            RouteBudget {
                threshold: 0.50,
                attention_tokens: 7,
                fast_tokens: 3,
                attention_fraction: 0.70,
            },
            6,
        );

        let counts = digest.counts();
        let summary = digest.plan_summary();

        assert_eq!(digest.layers.len(), 6);
        assert_eq!(counts.fusion, 2);
        assert_eq!(counts.local, 4);
        assert_eq!(counts.global, 0);
        assert_eq!(counts.total(), 6);
        assert_eq!(summary.layer_count, 6);
        assert_eq!(summary.counts, counts);
        assert_eq!(summary.attention_layer_count(), 2);
        assert!((summary.local_fraction() - (4.0 / 6.0)).abs() < f32::EPSILON);
        assert_eq!(summary.global_fraction(), 0.0);
        assert!((summary.fusion_fraction() - (2.0 / 6.0)).abs() < f32::EPSILON);
        assert!(summary.average_compute_fraction > 0.0);
        assert_eq!(summary.min_window_size, 1024);
        assert_eq!(summary.max_window_size, 4096);
        assert_eq!(digest.template.as_deref(), Some("route-budget-digest"));

        let fusion_layer = digest
            .layers
            .iter()
            .find(|layer| layer.attention == TransformerAttentionKind::Fusion)
            .expect("route pressure should create a fusion layer");
        let layer_summary = fusion_layer.layer_summary();

        assert_eq!(TransformerAttentionKind::Fusion.as_str(), "fusion");
        assert!(TransformerAttentionKind::Fusion.is_fusion());
        assert_eq!(layer_summary.layer_index, fusion_layer.layer_index);
        assert_eq!(layer_summary.attention, TransformerAttentionKind::Fusion);
        assert_eq!(layer_summary.attention_label, "fusion");
        assert_eq!(
            layer_summary.compute_fraction,
            fusion_layer.compute_fraction
        );
        assert_eq!(layer_summary.window_size, fusion_layer.window_size);
        assert!(layer_summary.uses_fusion);
        assert!(layer_summary.compute_reaches(0.40));
        assert!(layer_summary.window_at_least(1024));
        assert!(layer_summary.attention_label_matches_kind());
        assert!(layer_summary.fusion_flag_matches_kind());
        assert!(layer_summary.compute_shape_is_valid());
        assert!(layer_summary.window_shape_is_valid());
        assert_eq!(layer_summary.layer_budget_signal_component_count(), 4);
        assert!(layer_summary.has_layer_budget_signal_components());
        assert_eq!(layer_summary.layer_budget_problem_component_count(), 0);
        assert!(!layer_summary.has_layer_budget_problem_components());
        assert!(layer_summary.layer_budget_accounting_is_consistent());
        assert!(layer_summary.layer_budget_shape_is_clean());
        assert!(layer_summary.can_use_transformer_layer_budget());

        let layer_summaries = digest.layer_summaries();
        let layer_batch = digest.layer_batch_summary();
        assert_eq!(layer_summaries.len(), 6);
        assert_eq!(layer_batch.layer_count, 6);
        assert_eq!(layer_batch.usable_layer_count, 6);
        assert_eq!(layer_batch.unusable_layer_count(), 0);
        assert_eq!(layer_batch.signal_component_count, 16);
        assert_eq!(layer_batch.problem_component_count, 0);
        assert!(!layer_batch.is_empty());
        assert!(layer_batch.all_layers_usable());
        assert!(layer_batch.has_layer_budget_signals());
        assert!(!layer_batch.has_layer_budget_problem_components());
        assert!(layer_batch.layer_batch_accounting_is_consistent());
        assert!(layer_batch.layer_batch_shape_is_clean());
        assert!(layer_batch.can_use_transformer_layer_budget_batch());

        assert!(!summary.is_empty());
        assert!(summary.counts_match_layer_count());
        assert!(summary.compute_shape_is_valid());
        assert!(summary.window_shape_is_valid());
        assert!(summary.fractions_are_valid());
        assert_eq!(summary.plan_mix_signal_component_count(), 3);
        assert_eq!(summary.plan_pressure_signal_component_count(), 4);
        assert_eq!(summary.plan_summary_signal_component_count(), 7);
        assert!(summary.has_plan_summary_signal_components());
        assert_eq!(summary.plan_count_problem_component_count(), 0);
        assert_eq!(summary.plan_shape_problem_component_count(), 0);
        assert_eq!(summary.plan_summary_problem_component_count(), 0);
        assert!(!summary.has_plan_summary_problem_components());
        assert!(summary.plan_summary_accounting_is_consistent());
        assert!(summary.plan_summary_shape_is_clean());
        assert!(summary.can_use_transformer_plan());

        let readiness = TransformerPlanReadinessSummary::from_digest(
            RouteBudget {
                threshold: 0.50,
                attention_tokens: 7,
                fast_tokens: 3,
                attention_fraction: 0.70,
            },
            &digest,
        );
        assert_eq!(
            TransformerPlanReadinessSummary::stage_order(),
            [
                TransformerPlanReadinessStage::RouteBudget,
                TransformerPlanReadinessStage::PlanSummary,
                TransformerPlanReadinessStage::LayerBudgets,
            ]
        );
        assert!(readiness.route_budget_ready());
        assert!(readiness.plan_summary_ready());
        assert!(readiness.layer_budgets_ready());
        assert_eq!(readiness.first_unready_stage(), None);
        assert_eq!(readiness.first_blocking_stage(), None);
        assert_eq!(readiness.route_budget_signal_component_count, 4);
        assert_eq!(readiness.plan_summary_signal_component_count, 7);
        assert_eq!(readiness.layer_budget_signal_component_count, 16);
        assert_eq!(
            readiness.stage_signal_component_count(TransformerPlanReadinessStage::PlanSummary),
            readiness.plan_summary_signal_component_count
        );
        assert_eq!(
            readiness.stage_blocker_component_count(TransformerPlanReadinessStage::LayerBudgets),
            readiness.layer_budget_blocker_component_count
        );
        assert_eq!(
            readiness.transformer_plan_readiness_signal_component_count(),
            27
        );
        assert!(readiness.has_transformer_plan_readiness_signals());
        assert_eq!(
            readiness.transformer_plan_readiness_blocker_component_count(),
            0
        );
        assert!(!readiness.has_transformer_plan_readiness_blockers());
        assert!(readiness.transformer_plan_readiness_accounting_is_consistent());
        assert!(readiness.transformer_plan_readiness_is_clean());
        assert!(readiness.can_commit_transformer_plan_readiness());
    }

    #[test]
    fn route_budget_digest_uses_global_layers_at_extreme_pressure() {
        let digest = TransformerPlanDigest::from_route_budget(
            RouteBudget {
                threshold: 0.40,
                attention_tokens: 10,
                fast_tokens: 0,
                attention_fraction: 1.0,
            },
            3,
        );

        let counts = digest.counts();

        assert_eq!(counts.global, 3);
        assert_eq!(counts.local, 0);
        assert_eq!(counts.fusion, 0);
        assert!(digest.layers.iter().all(|layer| layer.window_size == 8192));

        let summary = digest.plan_summary();
        assert_eq!(summary.plan_mix_signal_component_count(), 2);
        assert_eq!(summary.plan_pressure_signal_component_count(), 3);
        assert_eq!(summary.plan_summary_signal_component_count(), 5);
        assert_eq!(summary.plan_summary_problem_component_count(), 0);
        assert!(summary.plan_summary_accounting_is_consistent());
        assert!(summary.plan_summary_shape_is_clean());
        assert!(summary.can_use_transformer_plan());
    }

    #[test]
    fn transformer_plan_summaries_count_public_shape_drift() {
        let layer = TransformerLayerBudgetSummary {
            layer_index: 0,
            attention: TransformerAttentionKind::Fusion,
            attention_label: "global",
            compute_fraction: f32::NAN,
            window_size: 0,
            uses_fusion: false,
        };

        assert!(!layer.attention_label_matches_kind());
        assert!(!layer.fusion_flag_matches_kind());
        assert!(!layer.compute_shape_is_valid());
        assert!(!layer.window_shape_is_valid());
        assert_eq!(layer.layer_budget_signal_component_count(), 1);
        assert_eq!(layer.layer_budget_problem_component_count(), 4);
        assert!(layer.has_layer_budget_problem_components());
        assert!(layer.layer_budget_accounting_is_consistent());
        assert!(!layer.layer_budget_shape_is_clean());
        assert!(!layer.can_use_transformer_layer_budget());

        let batch = TransformerLayerBudgetBatchSummary::from_summaries(&[layer]);
        assert_eq!(batch.layer_count, 1);
        assert_eq!(batch.usable_layer_count, 0);
        assert_eq!(batch.unusable_layer_count(), 1);
        assert_eq!(batch.signal_component_count, 1);
        assert_eq!(batch.problem_component_count, 4);
        assert!(!batch.all_layers_usable());
        assert!(batch.has_layer_budget_signals());
        assert!(batch.has_layer_budget_problem_components());
        assert!(batch.layer_batch_accounting_is_consistent());
        assert!(!batch.layer_batch_shape_is_clean());
        assert!(!batch.can_use_transformer_layer_budget_batch());

        let summary = TransformerPlanSummary {
            layer_count: 2,
            counts: TransformerPlanCounts {
                global: 2,
                local: 1,
                fusion: 1,
            },
            average_compute_fraction: 1.2,
            min_window_size: 4096,
            max_window_size: 1024,
        };

        assert!(!summary.counts_match_layer_count());
        assert!(!summary.compute_shape_is_valid());
        assert!(!summary.window_shape_is_valid());
        assert!(!summary.fractions_are_valid());
        assert_eq!(summary.plan_mix_signal_component_count(), 4);
        assert_eq!(summary.plan_pressure_signal_component_count(), 2);
        assert_eq!(summary.plan_summary_signal_component_count(), 6);
        assert_eq!(summary.plan_count_problem_component_count(), 1);
        assert_eq!(summary.plan_shape_problem_component_count(), 3);
        assert_eq!(summary.plan_summary_problem_component_count(), 4);
        assert!(summary.has_plan_summary_problem_components());
        assert!(summary.plan_summary_accounting_is_consistent());
        assert!(!summary.plan_summary_shape_is_clean());
        assert!(!summary.can_use_transformer_plan());
    }

    #[test]
    fn empty_transformer_plan_summary_is_clean_but_not_usable() {
        let summary = TransformerPlanSummary {
            layer_count: 0,
            counts: TransformerPlanCounts::default(),
            average_compute_fraction: 0.0,
            min_window_size: 0,
            max_window_size: 0,
        };

        assert!(summary.is_empty());
        assert!(summary.counts_match_layer_count());
        assert!(summary.compute_shape_is_valid());
        assert!(summary.window_shape_is_valid());
        assert!(summary.fractions_are_valid());
        assert_eq!(summary.plan_summary_signal_component_count(), 0);
        assert!(!summary.has_plan_summary_signal_components());
        assert_eq!(summary.plan_summary_problem_component_count(), 0);
        assert!(!summary.has_plan_summary_problem_components());
        assert!(summary.plan_summary_accounting_is_consistent());
        assert!(summary.plan_summary_shape_is_clean());
        assert!(!summary.can_use_transformer_plan());

        let batch = TransformerLayerBudgetBatchSummary::from_summaries(&[]);
        assert!(batch.is_empty());
        assert_eq!(batch.layer_count, 0);
        assert_eq!(batch.usable_layer_count, 0);
        assert_eq!(batch.unusable_layer_count(), 0);
        assert_eq!(batch.signal_component_count, 0);
        assert_eq!(batch.problem_component_count, 0);
        assert!(!batch.all_layers_usable());
        assert!(!batch.has_layer_budget_signals());
        assert!(!batch.has_layer_budget_problem_components());
        assert!(batch.layer_batch_accounting_is_consistent());
        assert!(!batch.layer_batch_shape_is_clean());
        assert!(!batch.can_use_transformer_layer_budget_batch());

        let digest = TransformerPlanDigest::new(
            Some("empty-route"),
            vec![TransformerLayerBudget::new(
                0,
                TransformerAttentionKind::LocalWindow,
                0.25,
                128,
            )],
        );
        let readiness =
            TransformerPlanReadinessSummary::from_digest(RouteBudget::default(), &digest);

        assert!(!readiness.route_budget_ready());
        assert!(readiness.plan_summary_ready());
        assert!(readiness.layer_budgets_ready());
        assert_eq!(
            readiness.first_unready_stage(),
            Some(TransformerPlanReadinessStage::RouteBudget)
        );
        assert_eq!(readiness.first_blocking_stage(), None);
        assert_eq!(readiness.route_budget_signal_component_count, 0);
        assert_eq!(readiness.route_budget_blocker_component_count, 0);
        assert_eq!(
            readiness.transformer_plan_readiness_blocker_component_count(),
            0
        );
        assert!(readiness.transformer_plan_readiness_accounting_is_consistent());
        assert!(readiness.transformer_plan_readiness_is_clean());
        assert!(!readiness.can_commit_transformer_plan_readiness());
    }

    #[test]
    fn transformer_plan_readiness_blocks_malformed_layer_budget() {
        let route_budget = RouteBudget {
            threshold: 0.50,
            attention_tokens: 1,
            fast_tokens: 1,
            attention_fraction: 0.50,
        };
        let digest = TransformerPlanDigest::new(
            Some("bad-layer"),
            vec![TransformerLayerBudget {
                layer_index: 0,
                attention: TransformerAttentionKind::Fusion,
                compute_fraction: f32::NAN,
                window_size: 0,
            }],
        );
        let readiness = TransformerPlanReadinessSummary::from_digest(route_budget, &digest);

        assert!(readiness.route_budget_ready());
        assert!(!readiness.plan_summary_ready());
        assert!(!readiness.layer_budgets_ready());
        assert_eq!(
            readiness.first_unready_stage(),
            Some(TransformerPlanReadinessStage::PlanSummary)
        );
        assert_eq!(
            readiness.first_blocking_stage(),
            Some(TransformerPlanReadinessStage::PlanSummary)
        );
        assert_eq!(readiness.route_budget_signal_component_count, 3);
        assert_eq!(readiness.plan_summary_signal_component_count, 5);
        assert_eq!(readiness.layer_budget_signal_component_count, 2);
        assert_eq!(readiness.route_budget_blocker_component_count, 0);
        assert_eq!(readiness.plan_summary_blocker_component_count, 2);
        assert_eq!(readiness.layer_budget_blocker_component_count, 2);
        assert_eq!(
            readiness.transformer_plan_readiness_signal_component_count(),
            10
        );
        assert_eq!(
            readiness.transformer_plan_readiness_blocker_component_count(),
            4
        );
        assert!(readiness.has_transformer_plan_readiness_blockers());
        assert!(readiness.transformer_plan_readiness_accounting_is_consistent());
        assert!(!readiness.transformer_plan_readiness_is_clean());
        assert!(!readiness.can_commit_transformer_plan_readiness());
    }

    #[test]
    fn local_transformer_layer_budget_is_clean_at_zero_compute() {
        let layer = TransformerLayerBudget {
            layer_index: 0,
            attention: TransformerAttentionKind::LocalWindow,
            compute_fraction: 0.0,
            window_size: 1,
        };
        let summary = layer.layer_summary();

        assert_eq!(summary.layer_budget_signal_component_count(), 1);
        assert!(summary.has_layer_budget_signal_components());
        assert_eq!(summary.layer_budget_problem_component_count(), 0);
        assert!(!summary.has_layer_budget_problem_components());
        assert!(summary.layer_budget_accounting_is_consistent());
        assert!(summary.layer_budget_shape_is_clean());
        assert!(summary.can_use_transformer_layer_budget());
    }

    #[test]
    fn default_planner_prefers_local_layers_for_coding() {
        let planner = DefaultTransformerPlanner;
        let digest = planner.plan(
            TransformerPlanningInput::new(
                TaskProfile::Coding,
                HierarchyWeights::new(0.2, 0.6, 0.2),
                budget(0.5),
            )
            .with_shape(12, 128),
        );
        let counts = digest.counts();

        assert_eq!(digest.template.as_deref(), Some("coding_local"));
        assert!(counts.local >= counts.global);
        assert!(counts.local >= counts.fusion);
        assert!(
            digest
                .layers
                .iter()
                .filter(|layer| layer.attention == TransformerAttentionKind::LocalWindow)
                .all(|layer| layer.window_size <= 192)
        );
    }

    #[test]
    fn default_planner_keeps_fusion_for_long_documents() {
        let planner = DefaultTransformerPlanner;
        let digest = planner.plan(
            TransformerPlanningInput::new(
                TaskProfile::LongDocument,
                HierarchyWeights::new(0.2, 0.2, 0.6),
                budget(0.3),
            )
            .with_shape(12, 128),
        );

        assert_eq!(digest.template.as_deref(), Some("long_context_fusion"));
        assert!(digest.counts().fusion > 0);
    }

    #[test]
    fn default_planner_uses_global_template_for_writing() {
        let planner = DefaultTransformerPlanner;
        let digest = planner.plan(
            TransformerPlanningInput::new(
                TaskProfile::Writing,
                HierarchyWeights::new(0.3, 0.4, 0.3),
                budget(0.4),
            )
            .with_shape(12, 128),
        );
        let counts = digest.counts();

        assert_eq!(digest.template.as_deref(), Some("creative_writing_global"));
        assert!(counts.global >= counts.fusion);
    }

    #[test]
    fn planning_pressure_summary_bridges_route_attention_and_transformer_mix() {
        let route_budget = RouteBudget {
            threshold: 0.50,
            attention_tokens: 6,
            fast_tokens: 2,
            attention_fraction: 0.75,
        };
        let attention_summary = AttentionDecision {
            threshold: 0.50,
            max_selected: 2,
            selected: vec![
                AttentionCandidate::new("local", 0, 0.70, 0.30, RouteLayer::LocalWindow),
                AttentionCandidate::new("global", 1, 0.90, 0.60, RouteLayer::Global),
            ],
            rejected: vec![
                AttentionCandidate::new("fast", 2, 0.95, 0.10, RouteLayer::FastProjection),
                AttentionCandidate::new("fusion", 3, 0.80, 0.50, RouteLayer::Fusion),
            ],
        }
        .decision_summary();
        let transformer_summary = TransformerPlanDigest::new(
            Some("adapter-pressure-test"),
            vec![
                TransformerLayerBudget::new(0, TransformerAttentionKind::Global, 0.90, 4096),
                TransformerLayerBudget::new(1, TransformerAttentionKind::LocalWindow, 0.60, 1024),
                TransformerLayerBudget::new(2, TransformerAttentionKind::Fusion, 0.40, 512),
                TransformerLayerBudget::new(3, TransformerAttentionKind::LocalWindow, 0.50, 1024),
            ],
        )
        .plan_summary();

        let summary = TransformerPlanningPressureSummary::from_parts(
            route_budget,
            attention_summary,
            transformer_summary,
        );

        assert_eq!(summary.route_attention_tokens, 6);
        assert_eq!(summary.route_fast_tokens, 2);
        assert_eq!(summary.selected_attention_tokens, 2);
        assert_eq!(summary.rejected_attention_tokens, 1);
        assert!(summary.attention_selection_is_clamped());
        assert!(summary.has_attention_rejections());
        assert_eq!(summary.transformer_layer_count, 4);
        assert!(summary.transformer_uses_fusion());
        assert!((summary.route_attention_fraction - 0.75).abs() < 0.0001);
        assert!((summary.attention_selection_fraction - 0.50).abs() < 0.0001);
        assert!((summary.transformer_non_local_fraction - 0.50).abs() < 0.0001);
        assert!((summary.transformer_fusion_fraction - 0.25).abs() < 0.0001);
        assert!((summary.transformer_average_compute_fraction - 0.60).abs() < 0.0001);
        assert!((summary.route_to_selection_delta - 0.25).abs() < 0.0001);
        assert!((summary.route_to_non_local_delta - 0.25).abs() < 0.0001);
        assert!(summary.route_and_transformer_diverge(0.20));
        assert!(!summary.route_and_transformer_diverge(0.30));
        assert!(summary.pressure_values_are_finite());
        assert!(summary.pressure_fractions_are_unit());
        assert_eq!(summary.route_token_pressure_signal_component_count(), 3);
        assert_eq!(
            summary.attention_selection_pressure_signal_component_count(),
            4
        );
        assert_eq!(summary.transformer_mix_pressure_signal_component_count(), 4);
        assert_eq!(summary.planning_pressure_signal_component_count(), 11);
        assert!(summary.has_planning_pressure_signals());
        assert_eq!(summary.planning_pressure_shape_problem_component_count(), 0);
        assert_eq!(summary.planning_pressure_delta_problem_component_count(), 0);
        assert_eq!(summary.planning_pressure_problem_component_count(), 0);
        assert!(!summary.has_planning_pressure_problem_components());
        assert!(summary.planning_pressure_accounting_is_consistent());
        assert!(summary.planning_pressure_shape_is_clean());
        assert!(summary.can_use_planning_pressure());
    }

    #[test]
    fn transformer_planning_readiness_confirms_route_attention_pressure_chain() {
        let route_summary = RoutingDecisionSummary {
            threshold: 0.50,
            token_count: 8,
            layer_counts: RouteLayerCounts {
                fast_projection: 2,
                local_window: 2,
                global: 2,
                fusion: 2,
            },
            attention_fraction: 0.75,
            average_score: 0.60,
            min_score: 0.10,
            max_score: 0.90,
            above_threshold_tokens: 6,
            below_threshold_tokens: 2,
        };
        let route_budget = route_summary.route_budget();
        let route_readiness = RouteBudgetReadinessSummary::new(route_summary, route_budget);
        let candidates = [
            AttentionCandidate::new("local", 0, 0.70, 0.30, RouteLayer::LocalWindow),
            AttentionCandidate::new("global", 1, 0.90, 0.60, RouteLayer::Global),
            AttentionCandidate::new("fast", 2, 0.95, 0.10, RouteLayer::FastProjection),
            AttentionCandidate::new("fusion", 3, 0.80, 0.50, RouteLayer::Fusion),
        ];
        let attention_decision = AttentionDecision {
            threshold: 0.50,
            max_selected: 2,
            selected: candidates[..2].to_vec(),
            rejected: candidates[2..].to_vec(),
        };
        let attention_readiness = AttentionSelectionReadinessSummary::from_decision(
            AttentionCandidate::batch_summary(&candidates),
            &attention_decision,
        );
        let transformer_summary = TransformerPlanDigest::new(
            Some("adapter-pressure-test"),
            vec![
                TransformerLayerBudget::new(0, TransformerAttentionKind::Global, 0.90, 4096),
                TransformerLayerBudget::new(1, TransformerAttentionKind::LocalWindow, 0.60, 1024),
                TransformerLayerBudget::new(2, TransformerAttentionKind::Fusion, 0.40, 512),
                TransformerLayerBudget::new(3, TransformerAttentionKind::LocalWindow, 0.50, 1024),
            ],
        )
        .plan_summary();
        let pressure = TransformerPlanningPressureSummary::from_parts(
            route_budget,
            attention_decision.decision_summary(),
            transformer_summary,
        );
        let readiness = TransformerPlanningReadinessSummary::new(
            route_readiness,
            attention_readiness,
            pressure,
        );

        assert_eq!(
            TransformerPlanningReadinessSummary::stage_order(),
            [
                TransformerPlanningReadinessStage::RouteBudget,
                TransformerPlanningReadinessStage::AttentionSelection,
                TransformerPlanningReadinessStage::PlanningPressure,
            ]
        );
        assert!(readiness.route_budget_ready());
        assert!(readiness.attention_selection_ready());
        assert!(readiness.planning_pressure_ready());
        assert!(readiness.planning_pressure_matches_route_budget());
        assert!(readiness.planning_pressure_matches_attention_selection());
        assert!(readiness.planning_pressure_boundary_matches());
        assert_eq!(
            readiness.planning_pressure_boundary_drift_component_count(),
            0
        );
        assert_eq!(readiness.first_unready_stage(), None);
        assert_eq!(readiness.first_blocking_stage(), None);
        assert_eq!(readiness.route_budget_signal_component_count, 15);
        assert_eq!(readiness.attention_selection_signal_component_count, 18);
        assert_eq!(readiness.planning_pressure_signal_component_count, 12);
        assert_eq!(
            readiness
                .stage_signal_component_count(TransformerPlanningReadinessStage::PlanningPressure),
            readiness.planning_pressure_signal_component_count
        );
        assert_eq!(
            readiness.stage_blocker_component_count(
                TransformerPlanningReadinessStage::AttentionSelection
            ),
            readiness.attention_selection_blocker_component_count
        );
        assert_eq!(
            readiness.transformer_planning_readiness_signal_component_count(),
            45
        );
        assert!(readiness.has_transformer_planning_readiness_signals());
        assert_eq!(
            readiness.transformer_planning_readiness_blocker_component_count(),
            0
        );
        assert!(!readiness.has_transformer_planning_readiness_blockers());
        assert!(readiness.transformer_planning_readiness_accounting_is_consistent());
        assert!(readiness.transformer_planning_readiness_is_clean());
        assert!(readiness.can_commit_transformer_planning_readiness());
    }

    #[test]
    fn transformer_planning_readiness_exposes_attention_selection_commit_boundary() {
        let route_summary = RoutingDecisionSummary {
            threshold: 0.50,
            token_count: 5,
            layer_counts: RouteLayerCounts {
                fast_projection: 1,
                local_window: 1,
                global: 2,
                fusion: 1,
            },
            attention_fraction: 0.80,
            average_score: 0.74,
            min_score: 0.20,
            max_score: 0.96,
            above_threshold_tokens: 4,
            below_threshold_tokens: 1,
        };
        let route_budget = route_summary.route_budget();
        let route_readiness = RouteBudgetReadinessSummary::new(route_summary, route_budget);
        let candidates = [
            AttentionCandidate::new("fast", 0, 0.99, 0.10, RouteLayer::FastProjection),
            AttentionCandidate::new("keep-local", 1, 0.74, 0.40, RouteLayer::LocalWindow),
            AttentionCandidate::new("drop-low", 2, 0.49, 0.90, RouteLayer::Global),
            AttentionCandidate::new("keep-global", 3, 0.96, 0.80, RouteLayer::Global),
            AttentionCandidate::new("over-budget", 4, 0.60, 0.50, RouteLayer::Fusion),
        ];
        let switches = ExperimentSwitches {
            max_attention_tokens: 2,
            ..ExperimentSwitches::default()
        };
        let decision = ThresholdAttentionPolicy::new(0.50).select(
            &candidates,
            RoutingContext::default(),
            switches,
        );
        let attention_readiness = AttentionSelectionReadinessSummary::from_decision(
            AttentionCandidate::batch_summary(&candidates),
            &decision,
        );
        let transformer_summary = TransformerPlanDigest::new(
            Some("attention-selection-commit-boundary"),
            vec![
                TransformerLayerBudget::new(0, TransformerAttentionKind::Global, 0.88, 4096),
                TransformerLayerBudget::new(1, TransformerAttentionKind::LocalWindow, 0.60, 2048),
                TransformerLayerBudget::new(2, TransformerAttentionKind::Fusion, 0.42, 1024),
                TransformerLayerBudget::new(3, TransformerAttentionKind::LocalWindow, 0.55, 2048),
            ],
        )
        .plan_summary();
        let pressure = TransformerPlanningPressureSummary::from_parts(
            route_budget,
            decision.decision_summary(),
            transformer_summary,
        );
        let ready = TransformerPlanningReadinessSummary::new(
            route_readiness,
            attention_readiness,
            pressure,
        );
        let commit = ready.attention_selection_commit_summary();

        assert!(ready.attention_selection_ready());
        assert!(ready.can_use_committed_attention_selection_for_planning());
        assert!(commit.can_commit_attention_selection());
        assert!(commit.can_use_committed_attention_decision());
        assert_eq!(
            commit.committed_attention_decision,
            Some(decision.decision_summary())
        );
        assert!(commit.commit_decision_accounting_is_consistent());

        let stale_attention_readiness = AttentionSelectionReadinessSummary::from_decision(
            AttentionCandidate::batch_summary(&candidates[..4]),
            &decision,
        );
        let repair = TransformerPlanningReadinessSummary::new(
            route_readiness,
            stale_attention_readiness,
            pressure,
        );
        let repair_commit = repair.attention_selection_commit_summary();

        assert!(!repair.attention_selection_ready());
        assert!(!repair.can_use_committed_attention_selection_for_planning());
        assert!(!repair_commit.can_commit_attention_selection());
        assert!(!repair_commit.can_use_committed_attention_decision());
        assert_eq!(repair_commit.committed_attention_decision, None);
        assert!(repair_commit.should_repair_attention_selection());
        assert!(repair_commit.commit_decision_accounting_is_consistent());
    }

    #[test]
    fn transformer_planning_readiness_preserves_real_attention_cap_pressure() {
        let route_summary = RoutingDecisionSummary {
            threshold: 0.50,
            token_count: 5,
            layer_counts: RouteLayerCounts {
                fast_projection: 1,
                local_window: 1,
                global: 2,
                fusion: 1,
            },
            attention_fraction: 0.80,
            average_score: 0.74,
            min_score: 0.20,
            max_score: 0.96,
            above_threshold_tokens: 4,
            below_threshold_tokens: 1,
        };
        let route_budget = route_summary.route_budget();
        let route_readiness = RouteBudgetReadinessSummary::new(route_summary, route_budget);
        let candidates = [
            AttentionCandidate::new("fast", 0, 0.99, 0.10, RouteLayer::FastProjection),
            AttentionCandidate::new("keep-local", 1, 0.74, 0.40, RouteLayer::LocalWindow),
            AttentionCandidate::new("drop-low", 2, 0.49, 0.90, RouteLayer::Global),
            AttentionCandidate::new("keep-global", 3, 0.96, 0.80, RouteLayer::Global),
            AttentionCandidate::new("over-budget", 4, 0.60, 0.50, RouteLayer::Fusion),
        ];
        let switches = ExperimentSwitches {
            max_attention_tokens: 2,
            ..ExperimentSwitches::default()
        };
        let decision = ThresholdAttentionPolicy::new(0.50).select(
            &candidates,
            RoutingContext::default(),
            switches,
        );
        let attention_readiness = AttentionSelectionReadinessSummary::from_decision(
            AttentionCandidate::batch_summary(&candidates),
            &decision,
        );
        let transformer_summary = TransformerPlanDigest::new(
            Some("attention-cap-pressure"),
            vec![
                TransformerLayerBudget::new(0, TransformerAttentionKind::Global, 0.88, 4096),
                TransformerLayerBudget::new(1, TransformerAttentionKind::LocalWindow, 0.60, 2048),
                TransformerLayerBudget::new(2, TransformerAttentionKind::Fusion, 0.42, 1024),
                TransformerLayerBudget::new(3, TransformerAttentionKind::LocalWindow, 0.55, 2048),
            ],
        )
        .plan_summary();
        let pressure = TransformerPlanningPressureSummary::from_parts(
            route_budget,
            decision.decision_summary(),
            transformer_summary,
        );
        let readiness = TransformerPlanningReadinessSummary::new(
            route_readiness,
            attention_readiness,
            pressure,
        );

        assert_eq!(
            decision.selected_tokens(),
            vec!["keep-local", "keep-global"]
        );
        assert!(decision.hit_selection_cap());
        assert!(attention_readiness.decision.hit_selection_cap);
        assert!(attention_readiness.decision.has_rejected_attention());
        assert!(attention_readiness.can_commit_attention_selection_readiness());
        assert_eq!(
            pressure.route_attention_tokens,
            route_budget.attention_tokens
        );
        assert_eq!(pressure.route_fast_tokens, route_budget.fast_tokens);
        assert_eq!(
            pressure.selected_attention_tokens,
            attention_readiness.decision.selected_attention_tokens()
        );
        assert_eq!(
            pressure.rejected_attention_tokens,
            attention_readiness.decision.rejected_attention_tokens()
        );
        assert!(pressure.attention_selection_is_clamped());
        assert!(pressure.has_attention_rejections());
        assert!(pressure.transformer_uses_fusion());
        assert!(pressure.route_and_transformer_diverge(0.20));
        assert!(pressure.can_use_planning_pressure());
        assert!(readiness.route_budget_ready());
        assert!(readiness.attention_selection_ready());
        assert!(readiness.planning_pressure_ready());
        assert!(readiness.planning_pressure_matches_route_budget());
        assert!(readiness.planning_pressure_matches_attention_selection());
        assert!(readiness.planning_pressure_boundary_matches());
        assert_eq!(
            readiness.planning_pressure_boundary_drift_component_count(),
            0
        );
        assert_eq!(readiness.first_unready_stage(), None);
        assert_eq!(readiness.first_blocking_stage(), None);
        assert!(readiness.transformer_planning_readiness_accounting_is_consistent());
        assert!(readiness.transformer_planning_readiness_is_clean());
        assert!(readiness.can_commit_transformer_planning_readiness());
    }

    #[test]
    fn transformer_planning_readiness_blocks_stale_pressure_summary() {
        let route_summary = RoutingDecisionSummary {
            threshold: 0.50,
            token_count: 8,
            layer_counts: RouteLayerCounts {
                fast_projection: 2,
                local_window: 2,
                global: 2,
                fusion: 2,
            },
            attention_fraction: 0.75,
            average_score: 0.60,
            min_score: 0.10,
            max_score: 0.90,
            above_threshold_tokens: 6,
            below_threshold_tokens: 2,
        };
        let route_budget = route_summary.route_budget();
        let route_readiness = RouteBudgetReadinessSummary::new(route_summary, route_budget);
        let candidates = [
            AttentionCandidate::new("local", 0, 0.70, 0.30, RouteLayer::LocalWindow),
            AttentionCandidate::new("global", 1, 0.90, 0.60, RouteLayer::Global),
            AttentionCandidate::new("fast", 2, 0.95, 0.10, RouteLayer::FastProjection),
            AttentionCandidate::new("fusion", 3, 0.80, 0.50, RouteLayer::Fusion),
        ];
        let attention_decision = AttentionDecision {
            threshold: 0.50,
            max_selected: 2,
            selected: candidates[..2].to_vec(),
            rejected: candidates[2..].to_vec(),
        };
        let attention_readiness = AttentionSelectionReadinessSummary::from_decision(
            AttentionCandidate::batch_summary(&candidates),
            &attention_decision,
        );
        let transformer_summary = TransformerPlanDigest::new(
            Some("adapter-pressure-test"),
            vec![
                TransformerLayerBudget::new(0, TransformerAttentionKind::Global, 0.90, 4096),
                TransformerLayerBudget::new(1, TransformerAttentionKind::LocalWindow, 0.60, 1024),
                TransformerLayerBudget::new(2, TransformerAttentionKind::Fusion, 0.40, 512),
                TransformerLayerBudget::new(3, TransformerAttentionKind::LocalWindow, 0.50, 1024),
            ],
        )
        .plan_summary();
        let stale_pressure = TransformerPlanningPressureSummary {
            selected_attention_tokens: 1,
            ..TransformerPlanningPressureSummary::from_parts(
                route_budget,
                attention_decision.decision_summary(),
                transformer_summary,
            )
        };
        let readiness = TransformerPlanningReadinessSummary::new(
            route_readiness,
            attention_readiness,
            stale_pressure,
        );

        assert!(readiness.route_budget_ready());
        assert!(readiness.attention_selection_ready());
        assert!(!readiness.planning_pressure_ready());
        assert!(readiness.planning_pressure_matches_route_budget());
        assert!(!readiness.planning_pressure_matches_attention_selection());
        assert!(!readiness.planning_pressure_boundary_matches());
        assert_eq!(
            readiness.planning_pressure_boundary_drift_component_count(),
            1
        );
        assert_eq!(
            readiness.first_unready_stage(),
            Some(TransformerPlanningReadinessStage::PlanningPressure)
        );
        assert_eq!(
            readiness.first_blocking_stage(),
            Some(TransformerPlanningReadinessStage::PlanningPressure)
        );
        assert_eq!(readiness.route_budget_signal_component_count, 15);
        assert_eq!(readiness.attention_selection_signal_component_count, 18);
        assert_eq!(readiness.planning_pressure_signal_component_count, 11);
        assert_eq!(readiness.route_budget_blocker_component_count, 0);
        assert_eq!(readiness.attention_selection_blocker_component_count, 0);
        assert_eq!(readiness.planning_pressure_blocker_component_count, 1);
        assert_eq!(
            readiness.transformer_planning_readiness_signal_component_count(),
            44
        );
        assert_eq!(
            readiness.transformer_planning_readiness_blocker_component_count(),
            1
        );
        assert!(readiness.has_transformer_planning_readiness_blockers());
        assert!(readiness.transformer_planning_readiness_accounting_is_consistent());
        assert!(!readiness.transformer_planning_readiness_is_clean());
        assert!(!readiness.can_commit_transformer_planning_readiness());
    }

    #[test]
    fn planning_pressure_summary_counts_public_shape_drift() {
        let summary = TransformerPlanningPressureSummary {
            route_attention_fraction: 1.2,
            route_attention_tokens: 2,
            route_fast_tokens: 1,
            attention_selection_fraction: 0.4,
            selected_attention_tokens: 1,
            rejected_attention_tokens: 0,
            hit_attention_selection_cap: false,
            transformer_layer_count: 2,
            transformer_non_local_fraction: 0.6,
            transformer_fusion_fraction: f32::NAN,
            transformer_average_compute_fraction: 0.7,
            route_to_selection_delta: 0.1,
            route_to_non_local_delta: 0.2,
        };

        assert!(!summary.pressure_values_are_finite());
        assert!(!summary.pressure_fractions_are_unit());
        assert_eq!(summary.route_token_pressure_signal_component_count(), 2);
        assert_eq!(
            summary.attention_selection_pressure_signal_component_count(),
            2
        );
        assert_eq!(summary.transformer_mix_pressure_signal_component_count(), 3);
        assert_eq!(summary.planning_pressure_signal_component_count(), 7);
        assert_eq!(summary.planning_pressure_shape_problem_component_count(), 2);
        assert_eq!(summary.planning_pressure_delta_problem_component_count(), 0);
        assert_eq!(summary.planning_pressure_problem_component_count(), 2);
        assert!(summary.has_planning_pressure_problem_components());
        assert!(summary.planning_pressure_accounting_is_consistent());
        assert!(!summary.planning_pressure_shape_is_clean());
        assert!(!summary.can_use_planning_pressure());

        let drift = TransformerPlanningPressureSummary {
            route_attention_fraction: 0.8,
            attention_selection_fraction: 0.4,
            transformer_non_local_fraction: 0.5,
            transformer_fusion_fraction: 0.25,
            route_to_selection_delta: 0.0,
            route_to_non_local_delta: 0.0,
            ..summary
        };

        assert!(drift.pressure_values_are_finite());
        assert!(drift.pressure_fractions_are_unit());
        assert_eq!(drift.planning_pressure_shape_problem_component_count(), 0);
        assert_eq!(drift.planning_pressure_delta_problem_component_count(), 2);
        assert_eq!(drift.planning_pressure_problem_component_count(), 2);
        assert!(drift.planning_pressure_accounting_is_consistent());
        assert!(!drift.planning_pressure_shape_is_clean());
        assert!(!drift.can_use_planning_pressure());
    }

    #[test]
    fn transformer_forward_batch_summary_reports_export_boundary_shape() {
        let summaries = [
            TransformerForwardSummary::new(0, TransformerAttentionKind::Global, 4096, 0.90, 0.80),
            TransformerForwardSummary::new(
                1,
                TransformerAttentionKind::LocalWindow,
                1024,
                0.50,
                0.0,
            ),
            TransformerForwardSummary::new(2, TransformerAttentionKind::Fusion, 512, 0.30, 0.40),
            TransformerForwardSummary {
                layer_index: 3,
                attention: TransformerAttentionKind::Fusion,
                window_size: 256,
                compute_fraction: f32::NAN,
                activation: f32::INFINITY,
            },
        ];

        let summary = TransformerForwardBatchSummary::from_summaries(&summaries);

        assert_eq!(summary.summary_count, 4);
        assert_eq!(summary.counts.global, 1);
        assert_eq!(summary.counts.local, 1);
        assert_eq!(summary.counts.fusion, 2);
        assert_eq!(summary.counts.total(), 4);
        assert!((summary.average_compute_fraction - 0.425).abs() < 0.0001);
        assert_eq!(summary.min_window_size, 256);
        assert_eq!(summary.max_window_size, 4096);
        assert!((summary.average_activation - 0.30).abs() < 0.0001);
        assert_eq!(summary.max_activation, 0.80);
        assert_eq!(summary.active_layer_count, 2);
        assert!(summary.has_non_finite_values);
        assert!(summary.has_forward_activity());
        assert!(!summary.all_layers_active());
        assert!(summary.attention_count_matches_summary_count());
        assert_eq!(summary.attention_count_drift(), 0);
        assert!(summary.active_layer_count_within_summary_count());
        assert!((summary.active_layer_fraction() - 0.50).abs() < 0.0001);
        assert_eq!(summary.non_local_layer_count(), 3);
        assert!((summary.non_local_fraction() - 0.75).abs() < 0.0001);
        assert!((summary.fusion_fraction() - 0.50).abs() < 0.0001);
        assert_eq!(summary.window_span(), 3840);
        assert!(summary.compute_fraction_shape_is_valid());
        assert!(summary.activation_shape_is_valid());
        assert!(summary.window_bounds_shape_is_valid());
        assert_eq!(summary.forward_batch_signal_component_count(), 7);
        assert!(summary.has_forward_batch_signals());
        assert_eq!(summary.forward_count_problem_component_count(), 0);
        assert_eq!(summary.forward_shape_problem_component_count(), 1);
        assert_eq!(summary.forward_batch_problem_component_count(), 1);
        assert!(summary.has_forward_batch_problem_components());
        assert!(summary.forward_batch_accounting_is_consistent());
        assert!(!summary.forward_batch_shape_is_clean());
        assert!(!summary.can_use_forward_batch());
    }

    #[test]
    fn transformer_forward_batch_summary_counts_public_shape_drift() {
        let summary = TransformerForwardBatchSummary {
            summary_count: 3,
            counts: TransformerPlanCounts {
                global: 1,
                local: 0,
                fusion: 1,
            },
            average_compute_fraction: 1.2,
            min_window_size: 10,
            max_window_size: 5,
            average_activation: f32::NAN,
            max_activation: 0.2,
            active_layer_count: 4,
            has_non_finite_values: false,
        };

        assert!(!summary.attention_count_matches_summary_count());
        assert_eq!(summary.attention_count_drift(), 1);
        assert!(!summary.active_layer_count_within_summary_count());
        assert!(!summary.compute_fraction_shape_is_valid());
        assert!(!summary.activation_shape_is_valid());
        assert!(!summary.window_bounds_shape_is_valid());
        assert_eq!(summary.forward_batch_signal_component_count(), 4);
        assert!(summary.has_forward_batch_signals());
        assert_eq!(summary.forward_count_problem_component_count(), 2);
        assert_eq!(summary.forward_shape_problem_component_count(), 3);
        assert_eq!(summary.forward_batch_problem_component_count(), 5);
        assert!(summary.has_forward_batch_problem_components());
        assert!(summary.forward_batch_accounting_is_consistent());
        assert!(!summary.forward_batch_shape_is_clean());
        assert!(!summary.can_use_forward_batch());
    }

    #[test]
    fn transformer_forward_batch_summary_gates_clean_and_empty_batches() {
        let clean_summary = TransformerForwardBatchSummary::from_summaries(&[
            TransformerForwardSummary::new(0, TransformerAttentionKind::Global, 512, 0.75, 0.50),
            TransformerForwardSummary::new(1, TransformerAttentionKind::Fusion, 256, 0.25, 0.25),
        ]);

        assert!(!clean_summary.has_forward_batch_problem_components());
        assert!(clean_summary.forward_batch_accounting_is_consistent());
        assert!(clean_summary.forward_batch_shape_is_clean());
        assert!(clean_summary.can_use_forward_batch());

        let empty_summary = TransformerForwardBatchSummary::from_summaries(&[]);

        assert_eq!(empty_summary.summary_count, 0);
        assert!(!empty_summary.has_forward_batch_problem_components());
        assert!(empty_summary.forward_batch_accounting_is_consistent());
        assert!(empty_summary.forward_batch_shape_is_clean());
        assert!(!empty_summary.can_use_forward_batch());
    }

    #[test]
    fn runtime_kv_export_plan_builds_blocks_from_forward_summaries() {
        let runtime = RuntimeMetadata::new("model", "tok", 4096, 4)
            .with_kv_exchange(false, true)
            .with_kv_limits(0, 3);
        let architecture = TransformerRuntimeArchitecture::new(4, 4, 4, 2, 128);
        let plan = RuntimeKvExportPlan::new(&runtime, architecture, 4);
        let summaries = [
            TransformerForwardSummary::new(3, TransformerAttentionKind::Global, 8, 0.50, 0.25),
            TransformerForwardSummary::new(4, TransformerAttentionKind::LocalWindow, 9, 0.75, 0.50),
            TransformerForwardSummary::new(5, TransformerAttentionKind::Fusion, 10, 0.20, 0.10),
            TransformerForwardSummary::new(6, TransformerAttentionKind::Global, 11, 0.10, 0.10),
        ];

        assert_eq!(
            plan.planned_block_count(&[1.0, 2.0, 3.0, 4.0], &summaries),
            3
        );
        assert_eq!(
            plan.export_summary(&[1.0, 2.0, 3.0, 4.0], &summaries),
            RuntimeKvExportSummary {
                enabled: true,
                max_blocks: 3,
                planned_blocks: 3,
                forward_value_len: 4,
                forward_summary_count: 4,
                forward_batch: TransformerForwardBatchSummary::from_summaries(&summaries),
                hit_export_limit: true,
            }
        );
        let export_summary = plan.export_summary(&[1.0, 2.0, 3.0, 4.0], &summaries);

        assert_eq!(export_summary.forward_summary_count_drift(), 0);
        assert_eq!(
            export_summary.forward_summary_count_drift_component_count(),
            0
        );
        assert_eq!(
            export_summary.planned_block_limit_overflow_component_count(),
            0
        );
        assert_eq!(
            export_summary.non_finite_forward_summary_component_count(),
            0
        );
        assert_eq!(export_summary.forward_input_signal_component_count(), 1);
        assert_eq!(export_summary.forward_activity_signal_component_count(), 1);
        assert_eq!(export_summary.export_emit_signal_component_count(), 1);
        assert_eq!(
            export_summary.empty_forward_skip_signal_component_count(),
            0
        );
        assert_eq!(export_summary.export_limit_signal_component_count(), 1);
        assert_eq!(export_summary.export_payload_signal_component_count(), 4);
        assert!(export_summary.has_export_payload_signals());
        assert_eq!(export_summary.export_payload_problem_component_count(), 0);
        assert!(!export_summary.has_export_payload_problem_components());
        assert!(export_summary.export_payload_accounting_is_consistent());
        assert!(export_summary.export_payload_is_clean());
        assert!(export_summary.export_payload_shape_is_clean());
        assert!(export_summary.can_use_runtime_kv_export_payload());

        let blocks = plan.build_blocks(&[1.0, 2.0, 3.0, 4.0], &summaries);

        assert_eq!(blocks.len(), 3);
        assert!(
            blocks
                .iter()
                .all(|block| block.namespace == KvNamespace::Runtime)
        );
        assert_eq!(blocks[0].layer, 3);
        assert_eq!(blocks[0].head, 0);
        assert_eq!(blocks[0].token_start, 0);
        assert_eq!(blocks[0].token_end, 1);
        assert_eq!(blocks[0].key, vec![0.75, 1.50]);
        assert_eq!(blocks[0].value, vec![2.25, 3.00]);
        assert_eq!(blocks[1].layer, 0);
        assert_eq!(blocks[1].head, 0);
        assert_eq!(blocks[2].layer, 1);
        assert_eq!(blocks[2].head, 0);

        let block_summary =
            RuntimeKvExportBlockSummary::from_blocks(export_summary.planned_blocks, &blocks);
        let block_shape_summaries = blocks
            .iter()
            .map(KvBlock::shape_summary)
            .collect::<Vec<_>>();
        assert_eq!(
            block_summary,
            RuntimeKvExportBlockSummary::from_block_summaries(
                export_summary.planned_blocks,
                &block_shape_summaries
            )
        );
        assert_eq!(block_summary.planned_blocks, 3);
        assert_eq!(block_summary.materialized_blocks, 3);
        assert_eq!(block_summary.runtime_namespace_blocks, 3);
        assert_eq!(block_summary.block_shape_signal_component_count, 9);
        assert_eq!(block_summary.block_shape_problem_component_count, 0);
        assert!(!block_summary.is_empty());
        assert!(block_summary.block_count_matches_plan());
        assert_eq!(block_summary.block_count_drift(), 0);
        assert!(block_summary.all_blocks_are_runtime_namespace());
        assert_eq!(block_summary.runtime_namespace_drift_component_count(), 0);
        assert_eq!(block_summary.block_count_drift_component_count(), 0);
        assert_eq!(block_summary.export_block_problem_component_count(), 0);
        assert!(!block_summary.has_export_block_problem_components());
        assert!(block_summary.has_export_block_signals());
        assert!(block_summary.export_block_accounting_is_consistent());
        assert_eq!(
            block_summary.runtime_kv_export_block_commit_signal_component_count(),
            9
        );
        assert!(block_summary.has_runtime_kv_export_block_commit_signals());
        assert_eq!(
            block_summary.runtime_kv_export_block_commit_blocker_component_count(),
            0
        );
        assert!(!block_summary.has_runtime_kv_export_block_commit_blockers());
        assert!(block_summary.runtime_kv_export_block_commit_accounting_is_consistent());
        assert!(block_summary.runtime_kv_export_block_commit_is_clean());
        assert!(block_summary.export_block_shape_is_clean());
        assert!(block_summary.can_commit_runtime_kv_export_blocks());
    }

    #[test]
    fn runtime_kv_export_summary_reports_payload_boundary_drift() {
        let forward_batch =
            TransformerForwardBatchSummary::from_summaries(&[TransformerForwardSummary {
                layer_index: 0,
                attention: TransformerAttentionKind::Fusion,
                window_size: 8,
                compute_fraction: f32::NAN,
                activation: f32::INFINITY,
            }]);
        let summary = RuntimeKvExportSummary {
            enabled: true,
            max_blocks: 1,
            planned_blocks: 3,
            forward_value_len: 4,
            forward_summary_count: 3,
            forward_batch,
            hit_export_limit: false,
        };

        assert!(!summary.forward_batch_matches_summary_count());
        assert_eq!(summary.forward_summary_count_drift(), 2);
        assert_eq!(summary.forward_summary_count_drift_component_count(), 1);
        assert!(!summary.planned_blocks_within_limit());
        assert_eq!(summary.planned_blocks_over_limit(), 2);
        assert_eq!(summary.planned_block_limit_overflow_component_count(), 1);
        assert_eq!(summary.non_finite_forward_summary_component_count(), 1);
        assert_eq!(summary.forward_input_signal_component_count(), 1);
        assert_eq!(summary.forward_activity_signal_component_count(), 0);
        assert_eq!(summary.export_emit_signal_component_count(), 1);
        assert_eq!(summary.empty_forward_skip_signal_component_count(), 0);
        assert_eq!(summary.export_limit_signal_component_count(), 0);
        assert_eq!(summary.export_payload_signal_component_count(), 2);
        assert!(summary.has_export_payload_signals());
        assert_eq!(summary.export_payload_problem_component_count(), 3);
        assert!(summary.has_export_payload_problem_components());
        assert!(summary.export_payload_accounting_is_consistent());
        assert!(!summary.export_payload_is_clean());
        assert!(!summary.export_payload_shape_is_clean());
        assert!(!summary.can_use_runtime_kv_export_payload());
    }

    #[test]
    fn runtime_kv_export_block_summary_counts_materialized_shape_drift() {
        let malformed_block =
            KvBlock::new(7, KvNamespace::Semantic, 0, 0, 0..0, Vec::new(), Vec::new());
        let malformed = RuntimeKvExportBlockSummary::from_blocks(1, &[malformed_block]);

        assert_eq!(malformed.planned_blocks, 1);
        assert_eq!(malformed.materialized_blocks, 1);
        assert_eq!(malformed.runtime_namespace_blocks, 0);
        assert_eq!(malformed.block_shape_signal_component_count, 0);
        assert_eq!(malformed.block_shape_problem_component_count, 3);
        assert!(!malformed.is_empty());
        assert!(malformed.block_count_matches_plan());
        assert_eq!(malformed.block_count_drift(), 0);
        assert!(!malformed.all_blocks_are_runtime_namespace());
        assert_eq!(malformed.runtime_namespace_drift_component_count(), 1);
        assert_eq!(malformed.block_count_drift_component_count(), 0);
        assert_eq!(malformed.export_block_problem_component_count(), 3);
        assert!(malformed.has_export_block_problem_components());
        assert!(!malformed.has_export_block_signals());
        assert!(malformed.export_block_accounting_is_consistent());
        assert_eq!(
            malformed.runtime_kv_export_block_commit_signal_component_count(),
            0
        );
        assert!(!malformed.has_runtime_kv_export_block_commit_signals());
        assert_eq!(
            malformed.runtime_kv_export_block_commit_blocker_component_count(),
            3
        );
        assert!(malformed.has_runtime_kv_export_block_commit_blockers());
        assert!(malformed.runtime_kv_export_block_commit_accounting_is_consistent());
        assert!(!malformed.runtime_kv_export_block_commit_is_clean());
        assert!(!malformed.export_block_shape_is_clean());
        assert!(!malformed.can_commit_runtime_kv_export_blocks());

        let missing = RuntimeKvExportBlockSummary::from_blocks(1, &[]);

        assert!(missing.is_empty());
        assert_eq!(missing.planned_blocks, 1);
        assert_eq!(missing.materialized_blocks, 0);
        assert_eq!(missing.runtime_namespace_blocks, 0);
        assert!(!missing.block_count_matches_plan());
        assert_eq!(missing.block_count_drift(), 1);
        assert!(!missing.all_blocks_are_runtime_namespace());
        assert_eq!(missing.runtime_namespace_drift_component_count(), 0);
        assert_eq!(missing.block_count_drift_component_count(), 1);
        assert_eq!(missing.export_block_problem_component_count(), 1);
        assert!(missing.has_export_block_problem_components());
        assert!(!missing.has_export_block_signals());
        assert!(missing.export_block_accounting_is_consistent());
        assert_eq!(
            missing.runtime_kv_export_block_commit_signal_component_count(),
            0
        );
        assert!(!missing.has_runtime_kv_export_block_commit_signals());
        assert_eq!(
            missing.runtime_kv_export_block_commit_blocker_component_count(),
            1
        );
        assert!(missing.has_runtime_kv_export_block_commit_blockers());
        assert!(missing.runtime_kv_export_block_commit_accounting_is_consistent());
        assert!(!missing.runtime_kv_export_block_commit_is_clean());
        assert!(!missing.export_block_shape_is_clean());
        assert!(!missing.can_commit_runtime_kv_export_blocks());
    }

    #[test]
    fn runtime_kv_export_planning_summary_confirms_planning_limit() {
        let runtime = RuntimeMetadata::new("model", "tok", 4096, 4)
            .with_kv_exchange(false, true)
            .with_kv_limits(0, 2);
        let architecture = TransformerRuntimeArchitecture::new(4, 4, 4, 2, 128);
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(64)
            .with_max_tokens(32)
            .with_runtime(runtime.clone())
            .with_experiments(ExperimentSwitches::default().with_fht_dke(true));
        let planning = crate::planning::RuntimePlanningDigest::from_request(
            &request,
            RouteBudget {
                threshold: 0.50,
                attention_tokens: 8,
                fast_tokens: 2,
                attention_fraction: 0.80,
            },
            &AdapterExecutionContext::new([RuntimeAdapter::Cuda]),
            &[],
            &DeterministicFhtDkeBudgeter::new(0.12, 0.58, 1),
        );
        let export_plan =
            RuntimeKvExportPlan::new(&runtime, architecture, planning.planned_kv_export_blocks);
        let summaries = [
            TransformerForwardSummary::new(0, TransformerAttentionKind::Global, 8, 0.50, 0.25),
            TransformerForwardSummary::new(1, TransformerAttentionKind::Fusion, 8, 0.50, 0.25),
        ];

        let summary = export_plan.planning_summary(planning, &[1.0, 2.0, 3.0, 4.0], &summaries);

        assert_eq!(summary.planning_export_blocks, 2);
        assert_eq!(summary.export_plan_max_blocks, 2);
        assert_eq!(summary.export_summary.planned_blocks, 2);
        assert!(summary.plan_allows_export());
        assert!(summary.forward_has_activity());
        assert!(summary.export_will_emit());
        assert!(!summary.export_is_blocked_by_planning());
        assert!(summary.export_plan_matches_planning_limit);
        assert!(summary.planned_export_within_planning);
        assert!(summary.export_boundary_is_consistent());
        assert_eq!(summary.export_plan_limit_drift_component_count(), 0);
        assert_eq!(summary.export_plan_limit_drift_blocks(), 0);
        assert!(!summary.export_plan_exceeds_planning_limit());
        assert!(!summary.export_plan_below_planning_limit());
        assert_eq!(summary.planned_export_overflow_component_count(), 0);
        assert_eq!(summary.planned_export_overflow_blocks(), 0);
        assert_eq!(summary.planning_export_signal_component_count(), 1);
        assert_eq!(
            summary.planning_forward_activity_signal_component_count(),
            1
        );
        assert_eq!(summary.planning_export_emit_signal_component_count(), 1);
        assert_eq!(summary.planning_export_limit_signal_component_count(), 1);
        assert_eq!(summary.export_boundary_signal_component_count(), 4);
        assert!(summary.has_export_boundary_signals());
        assert_eq!(summary.export_boundary_problem_component_count(), 0);
        assert!(!summary.has_export_boundary_problem_components());
        assert!(summary.export_boundary_accounting_is_consistent());
        assert!(summary.export_boundary_shape_is_clean());
        assert!(summary.export_hit_plan_limit());
        assert_eq!(summary.export_commit_problem_component_count(), 0);
        assert!(!summary.has_export_commit_problem_components());
        assert_eq!(summary.runtime_kv_export_commit_signal_component_count(), 8);
        assert!(summary.has_runtime_kv_export_commit_signals());
        assert_eq!(
            summary.runtime_kv_export_commit_blocker_component_count(),
            0
        );
        assert!(!summary.has_runtime_kv_export_commit_blockers());
        assert!(summary.runtime_kv_export_commit_accounting_is_consistent());
        assert!(summary.runtime_kv_export_commit_is_clean());
        assert!(summary.export_commit_is_clean());
        assert!(summary.can_commit_runtime_kv_export());
        assert!(summary.export_summary.has_forward_values());
        assert!(summary.export_summary.has_forward_summaries());
        assert!(summary.export_summary.forward_batch_matches_summary_count());
        assert!(summary.export_summary.planned_blocks_within_limit());
        assert_eq!(summary.export_summary.planned_blocks_over_limit(), 0);
    }

    #[test]
    fn runtime_kv_export_readiness_summary_confirms_stage_order_and_counts() {
        let runtime = RuntimeMetadata::new("model", "tok", 4096, 4)
            .with_kv_exchange(false, true)
            .with_kv_limits(0, 2);
        let architecture = TransformerRuntimeArchitecture::new(4, 4, 4, 2, 128);
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(64)
            .with_max_tokens(32)
            .with_runtime(runtime.clone())
            .with_experiments(ExperimentSwitches::default().with_fht_dke(true));
        let planning = crate::planning::RuntimePlanningDigest::from_request(
            &request,
            RouteBudget {
                threshold: 0.50,
                attention_tokens: 8,
                fast_tokens: 2,
                attention_fraction: 0.80,
            },
            &AdapterExecutionContext::new([RuntimeAdapter::Cuda]),
            &[],
            &DeterministicFhtDkeBudgeter::new(0.12, 0.58, 1),
        );
        let export_plan =
            RuntimeKvExportPlan::new(&runtime, architecture, planning.planned_kv_export_blocks);
        let summaries = [
            TransformerForwardSummary::new(0, TransformerAttentionKind::Global, 8, 0.50, 0.25),
            TransformerForwardSummary::new(1, TransformerAttentionKind::Fusion, 16, 0.50, 0.25),
        ];
        let forward = [1.0, 2.0, 3.0, 4.0];
        let planning_summary = export_plan.planning_summary(planning, &forward, &summaries);
        let blocks = export_plan.build_blocks(&forward, &summaries);
        let readiness = RuntimeKvExportReadinessSummary::from_blocks(planning_summary, &blocks);

        assert_eq!(
            RuntimeKvExportReadinessSummary::stage_order(),
            [
                RuntimeKvExportReadinessStage::ForwardBatch,
                RuntimeKvExportReadinessStage::ExportPayload,
                RuntimeKvExportReadinessStage::ExportPlanning,
                RuntimeKvExportReadinessStage::ExportBlocks,
            ]
        );
        assert!(readiness.export_will_emit());
        assert!(readiness.forward_batch_ready());
        assert!(readiness.export_payload_ready());
        assert!(readiness.export_planning_ready());
        assert!(readiness.export_blocks_ready());
        assert_eq!(readiness.first_unready_stage(), None);
        assert_eq!(readiness.first_blocking_stage(), None);
        assert_eq!(
            readiness.stage_signal_component_count(RuntimeKvExportReadinessStage::ForwardBatch),
            readiness.forward_batch_signal_component_count
        );
        assert_eq!(
            readiness.stage_blocker_component_count(RuntimeKvExportReadinessStage::ExportBlocks),
            readiness.export_block_blocker_component_count
        );
        assert_eq!(readiness.forward_batch_signal_component_count, 8);
        assert_eq!(readiness.export_payload_signal_component_count, 4);
        assert_eq!(readiness.export_planning_signal_component_count, 4);
        assert_eq!(readiness.export_block_signal_component_count, 6);
        assert_eq!(
            readiness.runtime_kv_export_readiness_signal_component_count(),
            22
        );
        assert!(readiness.has_runtime_kv_export_readiness_signals());
        assert_eq!(
            readiness.runtime_kv_export_readiness_blocker_component_count(),
            0
        );
        assert!(!readiness.has_runtime_kv_export_readiness_blockers());
        assert!(readiness.runtime_kv_export_readiness_accounting_is_consistent());
        assert!(readiness.runtime_kv_export_readiness_is_clean());
        assert!(readiness.can_commit_runtime_kv_export_readiness());
        assert_eq!(readiness.component_accounting_drift_count(), 0);
        assert_eq!(
            readiness.runtime_kv_export_readiness_commit_problem_component_count(),
            0
        );
        assert!(!readiness.has_runtime_kv_export_readiness_commit_problem_components());
        assert_eq!(readiness.failure_report(), None);
        assert_eq!(readiness.failure_reports(), Vec::new());
        assert_eq!(readiness.failure_report_count(), 0);
        assert!(!readiness.has_failure_reports());
        assert_eq!(readiness.failure_batch_summary().total_count, 0);
        assert!(!readiness.can_format_runtime_failures());
        assert_eq!(readiness.primary_failure_report(), None);
        assert_eq!(readiness.primary_failure_summary(), None);
        let readiness_commit = readiness.commit_summary();
        assert_eq!(
            readiness_commit.action,
            RuntimeKvExportReadinessCommitAction::CommitRuntimeKvExport
        );
        assert!(readiness_commit.action_can_commit());
        assert!(!readiness_commit.action_should_return_failure());
        assert!(readiness_commit.can_commit_runtime_kv_export_readiness());
        assert!(!readiness_commit.should_return_runtime_failure());
        assert!(readiness_commit.failure_reports.is_empty());
        assert_eq!(readiness_commit.primary_failure_report, None);
        assert_eq!(readiness_commit.primary_failure_summary, None);
        assert_eq!(readiness_commit.failure_report_count, 0);
        assert!(!readiness_commit.can_format_runtime_failures);
        assert_eq!(readiness_commit.total_signal_component_count, 22);
        assert_eq!(readiness_commit.total_blocker_component_count, 0);
        assert!(readiness_commit.component_accounting_consistent);
        assert!(!readiness_commit.has_primary_failure_summary());
        assert!(readiness_commit.failure_batch_shape_is_clean());
        assert!(readiness_commit.commit_decision_accounting_is_consistent());
        let failure_return = readiness_commit.failure_return_summary();
        assert_eq!(
            failure_return.source,
            RuntimeKvExchangeFailureReturnSource::RuntimeKvExportReadiness
        );
        assert_eq!(failure_return.source.label(), "runtime_kv_export_readiness");
        assert!(!failure_return.has_failure_reports());
        assert!(!failure_return.has_blocker_components());
        assert!(failure_return.failure_return_accounting_is_consistent());
        assert!(!failure_return.can_return_runtime_failure());
        assert_eq!(readiness_commit.runtime_failure_return_report(), None);
    }

    #[test]
    fn runtime_kv_export_plan_readiness_summary_matches_manual_composition() {
        let (runtime, architecture, planning) = runtime_kv_export_fixture(2);
        let export_plan =
            RuntimeKvExportPlan::new(&runtime, architecture, planning.planned_kv_export_blocks);
        let summaries = [
            TransformerForwardSummary::new(0, TransformerAttentionKind::Global, 8, 0.50, 0.25),
            TransformerForwardSummary::new(1, TransformerAttentionKind::Fusion, 16, 0.50, 0.25),
        ];
        let forward = [1.0, 2.0, 3.0, 4.0];

        let planning_summary = export_plan.planning_summary(planning, &forward, &summaries);
        let blocks = export_plan.build_blocks(&forward, &summaries);
        let manual = RuntimeKvExportReadinessSummary::from_blocks(planning_summary, &blocks);
        let helper = export_plan.readiness_summary(planning, &forward, &summaries);

        assert_eq!(helper, manual);
        assert_eq!(helper.block_summary.materialized_blocks, 2);
        assert_eq!(helper.first_unready_stage(), None);
        assert_eq!(helper.first_blocking_stage(), None);
        assert!(helper.export_will_emit());
        assert!(helper.runtime_kv_export_readiness_accounting_is_consistent());
        assert!(helper.can_commit_runtime_kv_export_readiness());
    }

    #[test]
    fn runtime_kv_export_readiness_exposes_commit_action_boundary() {
        let (runtime, architecture, planning) = runtime_kv_export_fixture(2);
        let export_plan =
            RuntimeKvExportPlan::new(&runtime, architecture, planning.planned_kv_export_blocks);
        let summaries = [
            TransformerForwardSummary::new(0, TransformerAttentionKind::Global, 8, 0.50, 0.25),
            TransformerForwardSummary::new(1, TransformerAttentionKind::Fusion, 16, 0.50, 0.25),
        ];
        let forward = [1.0, 2.0, 3.0, 4.0];
        let ready = export_plan.readiness_summary(planning, &forward, &summaries);

        assert_eq!(
            ready.runtime_kv_export_readiness_commit_action(),
            RuntimeKvExportReadinessCommitAction::CommitRuntimeKvExport
        );
        assert_eq!(
            ready.commit_summary().action,
            ready.runtime_kv_export_readiness_commit_action()
        );

        let mut missing_blocks = export_plan.build_blocks(&forward, &summaries);
        missing_blocks.pop();
        let blocked = export_plan.readiness_summary_for_blocks(
            planning,
            &forward,
            &summaries,
            &missing_blocks,
        );

        assert_eq!(
            blocked.runtime_kv_export_readiness_commit_action(),
            RuntimeKvExportReadinessCommitAction::ReturnRuntimeFailure
        );
        assert_eq!(
            blocked.commit_summary().action,
            blocked.runtime_kv_export_readiness_commit_action()
        );
    }

    #[test]
    fn runtime_kv_export_plan_readiness_for_blocks_catches_materialized_block_drift() {
        let (runtime, architecture, planning) = runtime_kv_export_fixture(2);
        let export_plan =
            RuntimeKvExportPlan::new(&runtime, architecture, planning.planned_kv_export_blocks);
        let summaries = [
            TransformerForwardSummary::new(0, TransformerAttentionKind::Global, 8, 0.50, 0.25),
            TransformerForwardSummary::new(1, TransformerAttentionKind::Fusion, 16, 0.50, 0.25),
        ];
        let forward = [1.0, 2.0, 3.0, 4.0];
        let mut blocks = export_plan.build_blocks(&forward, &summaries);
        blocks.pop();

        let missing_block =
            export_plan.readiness_summary_for_blocks(planning, &forward, &summaries, &blocks);

        assert!(missing_block.export_will_emit());
        assert!(missing_block.forward_batch_ready());
        assert!(missing_block.export_payload_ready());
        assert!(missing_block.export_planning_ready());
        assert!(!missing_block.export_blocks_ready());
        assert_eq!(
            missing_block.first_unready_stage(),
            Some(RuntimeKvExportReadinessStage::ExportBlocks)
        );
        assert_eq!(
            missing_block.first_blocking_stage(),
            Some(RuntimeKvExportReadinessStage::ExportBlocks)
        );
        assert_eq!(missing_block.block_summary.block_count_drift(), 1);
        assert_eq!(missing_block.export_block_blocker_component_count, 1);
        assert!(missing_block.runtime_kv_export_readiness_accounting_is_consistent());
        assert!(!missing_block.can_commit_runtime_kv_export_readiness());
        assert_eq!(missing_block.component_accounting_drift_count(), 0);
        assert_eq!(
            missing_block.runtime_kv_export_readiness_commit_problem_component_count(),
            1
        );
        assert!(missing_block.has_runtime_kv_export_readiness_commit_problem_components());
        assert_eq!(missing_block.failure_report_count(), 1);
        assert!(missing_block.has_failure_reports());
        let missing_failure = missing_block
            .failure_report()
            .expect("missing export readiness failure");
        let missing_failure_summary = missing_failure.failure_summary();
        assert_eq!(
            missing_failure_summary.trace_label,
            "runtime_kv_export_error"
        );
        assert!(missing_failure_summary.message_len > 0);
        assert!(missing_failure_summary.trace_label_matches_kind());
        assert!(missing_failure_summary.can_use_runtime_failure_report());
        let missing_failure_batch = missing_block.failure_batch_summary();
        assert_eq!(missing_failure_batch.total_count, 1);
        assert_eq!(missing_failure_batch.kv_export_count, 1);
        assert!(missing_failure_batch.can_format_runtime_failures());
        assert!(missing_block.can_format_runtime_failures());
        assert_eq!(
            missing_block.primary_failure_report(),
            Some(missing_failure.clone())
        );
        assert_eq!(
            missing_block.primary_failure_summary(),
            Some(missing_failure_summary)
        );
        let missing_commit = missing_block.commit_summary();
        assert_eq!(
            missing_commit.action,
            RuntimeKvExportReadinessCommitAction::ReturnRuntimeFailure
        );
        assert!(!missing_commit.action_can_commit());
        assert!(missing_commit.action_should_return_failure());
        assert!(!missing_commit.can_commit_runtime_kv_export_readiness());
        assert!(missing_commit.should_return_runtime_failure());
        assert_eq!(missing_commit.failure_report_count, 1);
        assert_eq!(missing_commit.failure_reports.len(), 1);
        assert_eq!(missing_commit.primary_failure_report, Some(missing_failure));
        assert_eq!(
            missing_commit.primary_failure_summary,
            Some(missing_failure_summary)
        );
        assert!(missing_commit.can_format_runtime_failures);
        assert_eq!(
            missing_commit.total_signal_component_count,
            missing_block.runtime_kv_export_readiness_signal_component_count()
        );
        assert_eq!(missing_commit.total_blocker_component_count, 1);
        assert!(missing_commit.component_accounting_consistent);
        assert!(missing_commit.has_primary_failure_summary());
        assert!(missing_commit.failure_batch_shape_is_clean());
        assert!(missing_commit.commit_decision_accounting_is_consistent());
        let missing_failure_return = missing_commit.failure_return_summary();
        assert_eq!(
            missing_failure_return.source,
            RuntimeKvExchangeFailureReturnSource::RuntimeKvExportReadiness
        );
        assert!(missing_failure_return.has_failure_reports());
        assert!(missing_failure_return.has_blocker_components());
        assert!(missing_failure_return.failure_return_accounting_is_consistent());
        assert!(missing_failure_return.can_return_runtime_failure());
        let missing_return_report = missing_commit
            .runtime_failure_return_report()
            .expect("missing export readiness return report");
        assert_eq!(
            missing_return_report.source,
            RuntimeKvExchangeFailureReturnSource::RuntimeKvExportReadiness
        );
        assert_eq!(
            missing_return_report.primary_failure_summary,
            missing_failure_summary
        );
        assert_eq!(missing_return_report.failure_batch.kv_export_count, 1);
        assert!(missing_return_report.failure_return_report_shape_is_clean());
        assert!(missing_return_report.can_use_runtime_kv_exchange_failure_return_report());
        assert!(
            missing_return_report
                .backend_message()
                .contains("runtime kv export readiness failed")
        );
        assert!(
            missing_return_report
                .diagnostics_note()
                .starts_with("runtime_kv_export_error")
        );
        assert_eq!(
            missing_return_report.inference_error().message,
            missing_return_report.backend_message()
        );

        let mut wrong_namespace = export_plan.build_blocks(&forward, &summaries);
        wrong_namespace[0].namespace = KvNamespace::Semantic;
        let namespace_drift = export_plan.readiness_summary_for_blocks(
            planning,
            &forward,
            &summaries,
            &wrong_namespace,
        );

        assert!(namespace_drift.block_summary.block_count_matches_plan());
        assert!(
            !namespace_drift
                .block_summary
                .all_blocks_are_runtime_namespace()
        );
        assert_eq!(
            namespace_drift
                .block_summary
                .runtime_namespace_drift_component_count(),
            1
        );
        assert_eq!(
            namespace_drift.first_blocking_stage(),
            Some(RuntimeKvExportReadinessStage::ExportBlocks)
        );
        assert!(!namespace_drift.can_commit_runtime_kv_export_readiness());
        assert_eq!(
            namespace_drift.commit_summary().action,
            RuntimeKvExportReadinessCommitAction::ReturnRuntimeFailure
        );
        assert_eq!(namespace_drift.failure_batch_summary().kv_export_count, 1);
    }

    #[test]
    fn runtime_kv_export_plan_readiness_summary_allows_planned_zero_export_noop() {
        let runtime = RuntimeMetadata::new("model", "tok", 4096, 4).with_kv_exchange(false, false);
        let architecture = TransformerRuntimeArchitecture::new(4, 4, 4, 2, 128);
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(64)
            .with_max_tokens(32)
            .with_runtime(runtime.clone())
            .with_experiments(ExperimentSwitches::default().with_fht_dke(true));
        let planning = crate::planning::RuntimePlanningDigest::from_request(
            &request,
            RouteBudget {
                threshold: 0.50,
                attention_tokens: 8,
                fast_tokens: 2,
                attention_fraction: 0.80,
            },
            &AdapterExecutionContext::new([RuntimeAdapter::Cuda]),
            &[],
            &DeterministicFhtDkeBudgeter::new(0.12, 0.58, 1),
        );
        let export_plan =
            RuntimeKvExportPlan::new(&runtime, architecture, planning.planned_kv_export_blocks);
        let summaries = [TransformerForwardSummary::new(
            0,
            TransformerAttentionKind::Global,
            8,
            0.50,
            0.25,
        )];
        let forward = [1.0, 2.0, 3.0, 4.0];

        let readiness = export_plan.readiness_summary(planning, &forward, &summaries);

        assert_eq!(planning.planned_kv_export_blocks, 0);
        assert_eq!(readiness.planning_summary.planning_export_blocks, 0);
        assert_eq!(readiness.planning_summary.export_summary.planned_blocks, 0);
        assert_eq!(readiness.block_summary.materialized_blocks, 0);
        assert!(!readiness.export_will_emit());
        assert!(readiness.forward_batch_ready());
        assert!(readiness.export_payload_ready());
        assert!(readiness.export_planning_ready());
        assert!(readiness.export_blocks_ready());
        assert_eq!(readiness.first_unready_stage(), None);
        assert_eq!(readiness.first_blocking_stage(), None);
        assert!(readiness.runtime_kv_export_readiness_accounting_is_consistent());
        assert!(readiness.can_commit_runtime_kv_export_readiness());
    }

    #[test]
    fn runtime_kv_export_planning_summary_reports_plan_drift() {
        let runtime = RuntimeMetadata::new("model", "tok", 4096, 4)
            .with_kv_exchange(false, true)
            .with_kv_limits(0, 3);
        let architecture = TransformerRuntimeArchitecture::new(4, 4, 4, 2, 128);
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(64)
            .with_max_tokens(32)
            .with_runtime(runtime.clone())
            .with_experiments(ExperimentSwitches::default().with_fht_dke(true));
        let planning = crate::planning::RuntimePlanningDigest::from_request(
            &request,
            RouteBudget {
                threshold: 0.50,
                attention_tokens: 3,
                fast_tokens: 7,
                attention_fraction: 0.30,
            },
            &AdapterExecutionContext::new([RuntimeAdapter::Cuda]),
            &[],
            &DeterministicFhtDkeBudgeter::default(),
        );
        let export_plan = RuntimeKvExportPlan::new(&runtime, architecture, 3);
        let summaries = [
            TransformerForwardSummary::new(0, TransformerAttentionKind::Global, 8, 0.50, 0.25),
            TransformerForwardSummary::new(1, TransformerAttentionKind::Fusion, 8, 0.50, 0.25),
            TransformerForwardSummary::new(2, TransformerAttentionKind::LocalWindow, 8, 0.50, 0.25),
        ];

        let summary = export_plan.planning_summary(planning, &[1.0, 2.0, 3.0, 4.0], &summaries);

        assert_eq!(summary.planning_export_blocks, 1);
        assert_eq!(summary.export_plan_max_blocks, 3);
        assert_eq!(summary.export_summary.planned_blocks, 3);
        assert!(summary.plan_allows_export());
        assert!(summary.forward_has_activity());
        assert!(summary.export_will_emit());
        assert!(summary.export_is_blocked_by_planning());
        assert!(!summary.export_plan_matches_planning_limit);
        assert!(!summary.planned_export_within_planning);
        assert!(!summary.export_boundary_is_consistent());
        assert_eq!(summary.export_plan_limit_drift_component_count(), 1);
        assert_eq!(summary.export_plan_limit_drift_blocks(), 2);
        assert!(summary.export_plan_exceeds_planning_limit());
        assert!(!summary.export_plan_below_planning_limit());
        assert_eq!(summary.planned_export_overflow_component_count(), 1);
        assert_eq!(summary.planned_export_overflow_blocks(), 2);
        assert_eq!(summary.planning_export_signal_component_count(), 1);
        assert_eq!(
            summary.planning_forward_activity_signal_component_count(),
            1
        );
        assert_eq!(summary.planning_export_emit_signal_component_count(), 1);
        assert_eq!(summary.planning_export_limit_signal_component_count(), 1);
        assert_eq!(summary.export_boundary_signal_component_count(), 4);
        assert!(summary.has_export_boundary_signals());
        assert_eq!(summary.export_boundary_problem_component_count(), 2);
        assert!(summary.has_export_boundary_problem_components());
        assert!(summary.export_boundary_accounting_is_consistent());
        assert!(!summary.export_boundary_shape_is_clean());
        assert!(summary.export_hit_plan_limit());
        assert_eq!(summary.export_commit_problem_component_count(), 2);
        assert!(summary.has_export_commit_problem_components());
        assert_eq!(summary.runtime_kv_export_commit_signal_component_count(), 8);
        assert!(summary.has_runtime_kv_export_commit_signals());
        assert_eq!(
            summary.runtime_kv_export_commit_blocker_component_count(),
            2
        );
        assert!(summary.has_runtime_kv_export_commit_blockers());
        assert!(summary.runtime_kv_export_commit_accounting_is_consistent());
        assert!(!summary.runtime_kv_export_commit_is_clean());
        assert!(!summary.export_commit_is_clean());
        assert!(!summary.can_commit_runtime_kv_export());
        assert!(summary.export_summary.has_forward_values());
        assert!(summary.export_summary.has_forward_summaries());
        assert!(summary.export_summary.forward_batch_matches_summary_count());
        assert!(summary.export_summary.planned_blocks_within_limit());
        assert_eq!(summary.export_summary.planned_blocks_over_limit(), 0);

        let blocks = export_plan.build_blocks(&[1.0, 2.0, 3.0, 4.0], &summaries);
        let readiness = RuntimeKvExportReadinessSummary::from_blocks(summary, &blocks);

        assert!(readiness.forward_batch_ready());
        assert!(readiness.export_payload_ready());
        assert!(!readiness.export_planning_ready());
        assert!(readiness.export_blocks_ready());
        assert_eq!(
            readiness.first_unready_stage(),
            Some(RuntimeKvExportReadinessStage::ExportPlanning)
        );
        assert_eq!(
            readiness.first_blocking_stage(),
            Some(RuntimeKvExportReadinessStage::ExportPlanning)
        );
        assert_eq!(readiness.export_planning_blocker_component_count, 2);
        assert_eq!(
            readiness.runtime_kv_export_readiness_blocker_component_count(),
            2
        );
        assert!(readiness.has_runtime_kv_export_readiness_blockers());
        assert!(readiness.runtime_kv_export_readiness_accounting_is_consistent());
        assert!(!readiness.runtime_kv_export_readiness_is_clean());
        assert!(!readiness.can_commit_runtime_kv_export_readiness());
    }

    #[test]
    fn runtime_kv_export_plan_respects_export_support_and_empty_vectors() {
        let disabled = RuntimeMetadata::new("model", "tok", 4096, 4);
        let enabled = RuntimeMetadata::new("model", "tok", 4096, 4).with_kv_exchange(false, true);
        let architecture = TransformerRuntimeArchitecture::new(1, 4, 1, 1, 128);
        let summary = [TransformerForwardSummary::new(
            0,
            TransformerAttentionKind::Global,
            8,
            1.0,
            0.0,
        )];

        assert_eq!(
            RuntimeKvExportPlan::new(&disabled, architecture, 2)
                .planned_block_count(&[1.0, 2.0], &summary),
            0
        );
        assert!(
            RuntimeKvExportPlan::new(&disabled, architecture, 2)
                .build_blocks(&[1.0, 2.0], &summary)
                .is_empty()
        );
        assert_eq!(
            RuntimeKvExportPlan::new(&enabled, architecture, 2).planned_block_count(&[], &summary),
            0
        );
        let empty_summary =
            RuntimeKvExportPlan::new(&enabled, architecture, 2).export_summary(&[], &summary);
        assert!(empty_summary.enabled);
        assert!(!empty_summary.will_export());
        assert!(empty_summary.skipped_due_to_empty_forward());
        assert!(!empty_summary.has_forward_values());
        assert!(empty_summary.has_forward_summaries());
        assert!(empty_summary.forward_batch_matches_summary_count());
        assert!(empty_summary.planned_blocks_within_limit());
        assert_eq!(empty_summary.planned_blocks_over_limit(), 0);
        assert_eq!(empty_summary.forward_input_signal_component_count(), 1);
        assert_eq!(empty_summary.forward_activity_signal_component_count(), 0);
        assert_eq!(empty_summary.export_emit_signal_component_count(), 0);
        assert_eq!(empty_summary.empty_forward_skip_signal_component_count(), 1);
        assert_eq!(empty_summary.export_limit_signal_component_count(), 0);
        assert_eq!(empty_summary.export_payload_signal_component_count(), 2);
        assert!(empty_summary.has_export_payload_signals());
        assert_eq!(empty_summary.export_payload_problem_component_count(), 0);
        assert!(!empty_summary.has_export_payload_problem_components());
        assert!(empty_summary.export_payload_accounting_is_consistent());
        assert!(empty_summary.export_payload_is_clean());
        assert!(empty_summary.export_payload_shape_is_clean());
        assert!(!empty_summary.can_use_runtime_kv_export_payload());
        assert!(!empty_summary.hit_plan_limit());
        assert!(
            RuntimeKvExportPlan::new(&enabled, architecture, 2)
                .build_blocks(&[], &summary)
                .is_empty()
        );

        let no_op_planning = RuntimeKvExportPlanningSummary {
            planning_export_blocks: 2,
            export_plan_max_blocks: 2,
            export_summary: empty_summary,
            export_plan_matches_planning_limit: true,
            planned_export_within_planning: true,
        };
        let no_op_readiness = RuntimeKvExportReadinessSummary::from_blocks(no_op_planning, &[]);

        assert!(!no_op_readiness.export_will_emit());
        assert!(no_op_readiness.forward_batch_ready());
        assert!(no_op_readiness.export_payload_ready());
        assert!(no_op_readiness.export_planning_ready());
        assert!(no_op_readiness.export_blocks_ready());
        assert_eq!(no_op_readiness.first_unready_stage(), None);
        assert_eq!(no_op_readiness.first_blocking_stage(), None);
        assert_eq!(
            no_op_readiness.runtime_kv_export_readiness_blocker_component_count(),
            0
        );
        assert!(no_op_readiness.runtime_kv_export_readiness_accounting_is_consistent());
        assert!(no_op_readiness.can_commit_runtime_kv_export_readiness());
    }

    #[test]
    fn runtime_kv_export_planning_summary_counts_payload_commit_blockers() {
        let forward_batch =
            TransformerForwardBatchSummary::from_summaries(&[TransformerForwardSummary {
                layer_index: 0,
                attention: TransformerAttentionKind::Fusion,
                window_size: 8,
                compute_fraction: f32::NAN,
                activation: f32::INFINITY,
            }]);
        let export_summary = RuntimeKvExportSummary {
            enabled: true,
            max_blocks: 4,
            planned_blocks: 1,
            forward_value_len: 4,
            forward_summary_count: 2,
            forward_batch,
            hit_export_limit: false,
        };
        let summary = RuntimeKvExportPlanningSummary {
            planning_export_blocks: 4,
            export_plan_max_blocks: 4,
            export_summary,
            export_plan_matches_planning_limit: true,
            planned_export_within_planning: true,
        };

        assert!(summary.plan_allows_export());
        assert!(!summary.forward_has_activity());
        assert!(summary.export_will_emit());
        assert!(!summary.export_is_blocked_by_planning());
        assert!(summary.export_boundary_is_consistent());
        assert_eq!(summary.export_boundary_problem_component_count(), 0);
        assert!(!summary.has_export_boundary_problem_components());
        assert!(summary.export_boundary_accounting_is_consistent());
        assert_eq!(
            summary
                .export_summary
                .export_payload_problem_component_count(),
            2
        );
        assert!(
            summary
                .export_summary
                .has_export_payload_problem_components()
        );
        assert!(!summary.export_summary.export_payload_is_clean());
        assert!(!summary.export_summary.export_payload_shape_is_clean());
        assert!(!summary.export_summary.can_use_runtime_kv_export_payload());
        assert_eq!(summary.export_commit_problem_component_count(), 2);
        assert!(summary.has_export_commit_problem_components());
        assert_eq!(summary.runtime_kv_export_commit_signal_component_count(), 4);
        assert!(summary.has_runtime_kv_export_commit_signals());
        assert_eq!(
            summary.runtime_kv_export_commit_blocker_component_count(),
            2
        );
        assert!(summary.has_runtime_kv_export_commit_blockers());
        assert!(summary.runtime_kv_export_commit_accounting_is_consistent());
        assert!(!summary.runtime_kv_export_commit_is_clean());
        assert!(!summary.export_commit_is_clean());
        assert!(!summary.can_commit_runtime_kv_export());
    }

    #[test]
    fn runtime_kv_export_plan_handles_missing_summaries_and_non_finite_values() {
        let runtime = RuntimeMetadata::new("model", "tok", 4096, 0).with_kv_exchange(false, true);
        let architecture = TransformerRuntimeArchitecture::new(2, 4, 1, 2, 128);
        let plan = RuntimeKvExportPlan::new(&runtime, architecture, 2);

        assert_eq!(plan.planned_block_count(&[1.0, f32::NAN, 3.0], &[]), 1);

        let blocks = plan.build_blocks(&[1.0, f32::NAN, 3.0], &[]);

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].layer, 0);
        assert_eq!(blocks[0].head, 0);
        assert_eq!(blocks[0].key, vec![1.0]);
        assert_eq!(blocks[0].value, vec![0.0]);
        assert_eq!(blocks[0].score, 1.0 / 1.5);
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
