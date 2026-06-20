use crate::router::GenerationMetrics;

use super::controller::{self, HierarchyState};
use super::profile::TaskProfile;
use super::weights::HierarchyWeights;

#[derive(Debug, Clone, Copy)]
pub struct HierarchyAdjustmentPreviewPolicy {
    pub learning_rate: f32,
    pub max_component_delta: f32,
    pub minimum_token_count: usize,
}

impl Default for HierarchyAdjustmentPreviewPolicy {
    fn default() -> Self {
        Self {
            learning_rate: 0.22,
            max_component_delta: 0.08,
            minimum_token_count: 1,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HierarchyWeightDelta {
    pub global: f32,
    pub local: f32,
    pub convolution: f32,
    pub abs_total: f32,
    pub max_component_abs: f32,
}

#[derive(Debug, Clone)]
pub struct HierarchyAdjustmentPreviewReport {
    pub profile: TaskProfile,
    pub quality_score: f32,
    pub learning_rate: f32,
    pub max_component_delta: f32,
    pub minimum_token_count: usize,
    pub metrics_finite: bool,
    pub weights_finite: bool,
    pub policy_valid: bool,
    pub observations_before: u64,
    pub observation_delta_previewed: u64,
    pub current_before: HierarchyWeights,
    pub profile_before: HierarchyWeights,
    pub target: HierarchyWeights,
    pub candidate_current: HierarchyWeights,
    pub candidate_profile: HierarchyWeights,
    pub candidate_delta: HierarchyWeightDelta,
    pub preview_current: HierarchyWeights,
    pub preview_profile: HierarchyWeights,
    pub delta: HierarchyWeightDelta,
    pub read_only: bool,
    pub report_only: bool,
    pub preview_only: bool,
    pub state_write_allowed: bool,
    pub adaptive_state_write_allowed: bool,
    pub ndkv_write_allowed: bool,
    pub controller_observation_applied: bool,
    pub observation_bump_previewed: bool,
    pub adjustment_ready: bool,
    pub blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct HierarchyAdjustmentPreviewPlanner {
    pub policy: HierarchyAdjustmentPreviewPolicy,
}

impl Default for HierarchyAdjustmentPreviewPlanner {
    fn default() -> Self {
        Self {
            policy: HierarchyAdjustmentPreviewPolicy::default(),
        }
    }
}

impl HierarchyAdjustmentPreviewPlanner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_policy(mut self, policy: HierarchyAdjustmentPreviewPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn preview(
        &self,
        state: HierarchyState,
        profile: TaskProfile,
        metrics: GenerationMetrics,
    ) -> HierarchyAdjustmentPreviewReport {
        let profile_before = state.profile_weights.get(profile);
        let observations_before = state.profile_observations.get(profile);
        let minimum_token_count = self.policy.minimum_token_count.max(1);
        let mut blocked_reasons = Vec::new();
        let mut policy_valid = true;

        let learning_rate = match normalized_learning_rate(self.policy.learning_rate) {
            Some(rate) => rate,
            None => {
                blocked_reasons.push("hierarchy_adjustment_learning_rate_not_finite".to_owned());
                policy_valid = false;
                0.0
            }
        };
        let max_component_delta =
            match normalized_max_component_delta(self.policy.max_component_delta) {
                Some(delta) => delta,
                None => {
                    blocked_reasons
                        .push("hierarchy_adjustment_max_component_delta_not_finite".to_owned());
                    policy_valid = false;
                    0.0
                }
            };

        let metrics_finite = metrics_are_finite(metrics);
        let weights_finite =
            weights_are_finite(state.current) && weights_are_finite(profile_before);

        if !metrics_finite {
            blocked_reasons.push("hierarchy_adjustment_generation_metrics_not_finite".to_owned());
        }
        if metrics.token_count < minimum_token_count {
            blocked_reasons.push(format!(
                "hierarchy_adjustment_token_count={}<{}",
                metrics.token_count, minimum_token_count
            ));
        }
        if !weights_are_finite(state.current) {
            blocked_reasons.push("hierarchy_adjustment_current_weights_not_finite".to_owned());
        }
        if !weights_are_finite(profile_before) {
            blocked_reasons.push("hierarchy_adjustment_profile_weights_not_finite".to_owned());
        }

        let quality_score = if metrics_finite {
            metrics.quality_score()
        } else {
            0.0
        };
        let pre_candidate_blocked = !blocked_reasons.is_empty();
        let target = if pre_candidate_blocked {
            profile_before
        } else {
            adjusted_target_for_profile(profile, metrics, quality_score)
        };
        let candidate_profile = if pre_candidate_blocked {
            profile_before
        } else {
            profile_before.blend(target, learning_rate)
        };
        let candidate_delta = HierarchyWeightDelta::between(profile_before, candidate_profile);

        if max_component_delta > 0.0 && candidate_delta.max_component_abs > max_component_delta {
            blocked_reasons.push(format!(
                "hierarchy_adjustment_max_component_delta={:.6}>{:.6}",
                candidate_delta.max_component_abs, max_component_delta
            ));
        }

        let adjustment_ready = blocked_reasons.is_empty()
            && candidate_delta.abs_total > f32::EPSILON
            && learning_rate > 0.0;
        let preview_profile = if adjustment_ready {
            candidate_profile
        } else {
            profile_before
        };
        let delta = HierarchyWeightDelta::between(profile_before, preview_profile);
        let observation_delta_previewed = u64::from(adjustment_ready);
        let report = HierarchyAdjustmentPreviewReport {
            profile,
            quality_score,
            learning_rate,
            max_component_delta,
            minimum_token_count,
            metrics_finite,
            weights_finite,
            policy_valid,
            observations_before,
            observation_delta_previewed,
            current_before: state.current,
            profile_before,
            target,
            candidate_current: candidate_profile,
            candidate_profile,
            candidate_delta,
            preview_current: preview_profile,
            preview_profile,
            delta,
            read_only: true,
            report_only: true,
            preview_only: true,
            state_write_allowed: false,
            adaptive_state_write_allowed: false,
            ndkv_write_allowed: false,
            controller_observation_applied: false,
            observation_bump_previewed: adjustment_ready,
            adjustment_ready,
            blocked_reasons,
            telemetry: Vec::new(),
        };

        report.with_telemetry()
    }
}

impl HierarchyAdjustmentPreviewReport {
    pub fn summary_line(&self) -> String {
        format!(
            "hierarchy_adjustment_preview profile={} read_only={} report_only={} preview_only={} adjustment_ready={} quality={:.3} delta_abs={:.6} candidate_delta_abs={:.6} max_component_delta={:.6} blocked_reasons={}",
            profile_label(self.profile),
            self.read_only,
            self.report_only,
            self.preview_only,
            self.adjustment_ready,
            self.quality_score,
            self.delta.abs_total,
            self.candidate_delta.abs_total,
            self.max_component_delta,
            self.blocked_reasons.len(),
        )
    }

    fn with_telemetry(mut self) -> Self {
        self.telemetry = hierarchy_adjustment_preview_telemetry(&self);
        self
    }
}

impl HierarchyWeightDelta {
    pub fn between(before: HierarchyWeights, after: HierarchyWeights) -> Self {
        let global = after.global - before.global;
        let local = after.local - before.local;
        let convolution = after.convolution - before.convolution;
        let abs_total = global.abs() + local.abs() + convolution.abs();
        let max_component_abs = global.abs().max(local.abs()).max(convolution.abs());

        Self {
            global,
            local,
            convolution,
            abs_total,
            max_component_abs,
        }
    }
}

fn adjusted_target_for_profile(
    profile: TaskProfile,
    metrics: GenerationMetrics,
    quality_score: f32,
) -> HierarchyWeights {
    let mut target = controller::target_for_profile(profile);

    if quality_score < 0.55 {
        match profile {
            TaskProfile::Coding => target.local += 0.12,
            TaskProfile::Writing => target.global += 0.12,
            TaskProfile::LongDocument => target.convolution += 0.12,
            TaskProfile::General => target.global += 0.06,
        }
    } else if quality_score > 0.84 {
        target.convolution += 0.05;
    }

    if metrics.token_count == 0 {
        return controller::target_for_profile(profile);
    }

    target.normalize();
    target
}

fn normalized_learning_rate(rate: f32) -> Option<f32> {
    rate.is_finite().then(|| rate.clamp(0.0, 1.0))
}

fn normalized_max_component_delta(delta: f32) -> Option<f32> {
    delta.is_finite().then(|| delta.clamp(0.0001, 1.0))
}

fn metrics_are_finite(metrics: GenerationMetrics) -> bool {
    metrics.perplexity.is_finite() && metrics.semantic_consistency.is_finite()
}

fn weights_are_finite(weights: HierarchyWeights) -> bool {
    weights.global.is_finite() && weights.local.is_finite() && weights.convolution.is_finite()
}

fn hierarchy_adjustment_preview_telemetry(
    report: &HierarchyAdjustmentPreviewReport,
) -> Vec<String> {
    let mut telemetry = vec![
        "hierarchy_adjustment_preview=true".to_owned(),
        format!(
            "hierarchy_adjustment_preview_profile={}",
            profile_label(report.profile)
        ),
        format!(
            "hierarchy_adjustment_preview_read_only={}",
            report.read_only
        ),
        format!(
            "hierarchy_adjustment_preview_report_only={}",
            report.report_only
        ),
        format!("hierarchy_adjustment_preview_only={}", report.preview_only),
        format!(
            "hierarchy_adjustment_preview_state_write_allowed={}",
            report.state_write_allowed
        ),
        format!(
            "hierarchy_adjustment_preview_adaptive_state_write_allowed={}",
            report.adaptive_state_write_allowed
        ),
        format!(
            "hierarchy_adjustment_preview_ndkv_write_allowed={}",
            report.ndkv_write_allowed
        ),
        format!(
            "hierarchy_adjustment_preview_controller_observation_applied={}",
            report.controller_observation_applied
        ),
        format!(
            "hierarchy_adjustment_preview_observation_bump_previewed={}",
            report.observation_bump_previewed
        ),
        format!(
            "hierarchy_adjustment_preview_ready={}",
            report.adjustment_ready
        ),
        format!(
            "hierarchy_adjustment_preview_metrics_finite={}",
            report.metrics_finite
        ),
        format!(
            "hierarchy_adjustment_preview_weights_finite={}",
            report.weights_finite
        ),
        format!(
            "hierarchy_adjustment_preview_policy_valid={}",
            report.policy_valid
        ),
        format!(
            "hierarchy_adjustment_preview_observation_delta={}",
            report.observation_delta_previewed
        ),
        format!(
            "hierarchy_adjustment_preview_minimum_token_count={}",
            report.minimum_token_count
        ),
        format!(
            "hierarchy_adjustment_preview_quality={:.3}",
            report.quality_score
        ),
        format!(
            "hierarchy_adjustment_preview_delta_abs={:.6}",
            report.delta.abs_total
        ),
        format!(
            "hierarchy_adjustment_preview_delta_max_component={:.6}",
            report.delta.max_component_abs
        ),
        format!(
            "hierarchy_adjustment_preview_candidate_delta_abs={:.6}",
            report.candidate_delta.abs_total
        ),
        format!(
            "hierarchy_adjustment_preview_candidate_delta_max_component={:.6}",
            report.candidate_delta.max_component_abs
        ),
        format!(
            "hierarchy_adjustment_preview_blocked_reasons={}",
            report.blocked_reasons.len()
        ),
    ];
    telemetry.extend(
        report
            .blocked_reasons
            .iter()
            .map(|reason| format!("hierarchy_adjustment_preview_blocked_reason={reason}")),
    );
    telemetry
}

fn profile_label(profile: TaskProfile) -> &'static str {
    match profile {
        TaskProfile::General => "general",
        TaskProfile::Coding => "coding",
        TaskProfile::Writing => "writing",
        TaskProfile::LongDocument => "long_document",
    }
}
