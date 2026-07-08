use crate::hierarchy::TaskProfile;
use crate::privacy_redaction::stable_redaction_digest;

use super::types::{GenerationMetrics, ProfileThresholds, RouterState};

#[derive(Debug, Clone, Copy)]
pub struct RouterThresholdAdjustmentPreviewPolicy {
    pub learning_rate: f32,
    pub min_threshold: f32,
    pub max_threshold: f32,
    pub minimum_token_count: usize,
}

impl Default for RouterThresholdAdjustmentPreviewPolicy {
    fn default() -> Self {
        Self {
            learning_rate: 0.08,
            min_threshold: 0.18,
            max_threshold: 0.88,
            minimum_token_count: 1,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RouterThresholdAdjustmentPreviewReport {
    pub profile: TaskProfile,
    pub quality_score: f32,
    pub learning_rate: f32,
    pub min_threshold: f32,
    pub max_threshold: f32,
    pub minimum_token_count: usize,
    pub metrics_finite: bool,
    pub thresholds_finite: bool,
    pub policy_valid: bool,
    pub observations_before: u64,
    pub profile_observations_before: u64,
    pub observation_delta_previewed: u64,
    pub threshold_before: f32,
    pub profile_thresholds_before: ProfileThresholds,
    pub candidate_threshold: f32,
    pub candidate_profile_thresholds: ProfileThresholds,
    pub preview_threshold: f32,
    pub preview_profile_thresholds: ProfileThresholds,
    pub threshold_delta: f32,
    pub rollback_anchor_id: String,
    pub read_only: bool,
    pub report_only: bool,
    pub preview_only: bool,
    pub router_state_write_allowed: bool,
    pub adaptive_state_write_allowed: bool,
    pub ndkv_write_allowed: bool,
    pub router_observation_applied: bool,
    pub observation_bump_previewed: bool,
    pub adjustment_ready: bool,
    pub blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RouterThresholdAdjustmentPreviewPlanner {
    pub policy: RouterThresholdAdjustmentPreviewPolicy,
}

impl Default for RouterThresholdAdjustmentPreviewPlanner {
    fn default() -> Self {
        Self {
            policy: RouterThresholdAdjustmentPreviewPolicy::default(),
        }
    }
}

impl RouterThresholdAdjustmentPreviewPlanner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_policy(mut self, policy: RouterThresholdAdjustmentPreviewPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn preview(
        &self,
        state: RouterState,
        profile: TaskProfile,
        metrics: GenerationMetrics,
    ) -> RouterThresholdAdjustmentPreviewReport {
        let mut blocked_reasons = Vec::new();
        let mut policy_valid = true;

        let (min_threshold, max_threshold) =
            match normalized_threshold_bounds(self.policy.min_threshold, self.policy.max_threshold)
            {
                Some(bounds) => bounds,
                None => {
                    blocked_reasons
                        .push("router_threshold_adjustment_threshold_bounds_invalid".to_owned());
                    policy_valid = false;
                    (
                        RouterThresholdAdjustmentPreviewPolicy::default().min_threshold,
                        RouterThresholdAdjustmentPreviewPolicy::default().max_threshold,
                    )
                }
            };
        let learning_rate = match normalized_learning_rate(self.policy.learning_rate) {
            Some(rate) => rate,
            None => {
                blocked_reasons
                    .push("router_threshold_adjustment_learning_rate_not_finite".to_owned());
                policy_valid = false;
                0.0
            }
        };
        let minimum_token_count = self.policy.minimum_token_count.max(1);

        let raw_profile_threshold = state.profile_thresholds.get(profile);
        let metrics_finite = metrics_are_finite(metrics);
        let thresholds_finite = state.threshold.is_finite() && raw_profile_threshold.is_finite();

        if !metrics_finite {
            blocked_reasons
                .push("router_threshold_adjustment_generation_metrics_not_finite".to_owned());
        }
        if metrics.token_count < minimum_token_count {
            blocked_reasons.push(format!(
                "router_threshold_adjustment_token_count={}<{}",
                metrics.token_count, minimum_token_count
            ));
        }
        if !state.threshold.is_finite() {
            blocked_reasons
                .push("router_threshold_adjustment_current_threshold_not_finite".to_owned());
        }
        if !raw_profile_threshold.is_finite() {
            blocked_reasons
                .push("router_threshold_adjustment_profile_threshold_not_finite".to_owned());
        }

        let threshold_before = safe_threshold(
            raw_profile_threshold,
            state.threshold,
            min_threshold,
            max_threshold,
        );
        let quality_score = if metrics_finite {
            metrics.quality_score()
        } else {
            0.0
        };
        let pre_candidate_blocked = !blocked_reasons.is_empty();
        let candidate_threshold = if pre_candidate_blocked {
            threshold_before
        } else {
            threshold_after_observation(
                threshold_before,
                learning_rate,
                min_threshold,
                max_threshold,
                metrics,
                quality_score,
            )
        };
        let mut candidate_profile_thresholds = state.profile_thresholds;
        if !pre_candidate_blocked {
            candidate_profile_thresholds.set(profile, candidate_threshold);
        }

        let candidate_delta = candidate_threshold - threshold_before;
        let adjustment_ready = blocked_reasons.is_empty() && candidate_delta.abs() > f32::EPSILON;
        let preview_threshold = if adjustment_ready {
            candidate_threshold
        } else {
            threshold_before
        };
        let mut preview_profile_thresholds = state.profile_thresholds;
        if adjustment_ready {
            preview_profile_thresholds.set(profile, preview_threshold);
        }
        let threshold_delta = preview_threshold - threshold_before;
        let observation_delta_previewed = u64::from(adjustment_ready);
        let rollback_anchor_id = router_threshold_adjustment_rollback_anchor_id(
            profile,
            state.observations,
            state.profile_observations.get(profile),
            threshold_before,
            preview_threshold,
            threshold_delta,
            observation_delta_previewed,
        );

        let report = RouterThresholdAdjustmentPreviewReport {
            profile,
            quality_score,
            learning_rate,
            min_threshold,
            max_threshold,
            minimum_token_count,
            metrics_finite,
            thresholds_finite,
            policy_valid,
            observations_before: state.observations,
            profile_observations_before: state.profile_observations.get(profile),
            observation_delta_previewed,
            threshold_before,
            profile_thresholds_before: state.profile_thresholds,
            candidate_threshold,
            candidate_profile_thresholds,
            preview_threshold,
            preview_profile_thresholds,
            threshold_delta,
            rollback_anchor_id,
            read_only: true,
            report_only: true,
            preview_only: true,
            router_state_write_allowed: false,
            adaptive_state_write_allowed: false,
            ndkv_write_allowed: false,
            router_observation_applied: false,
            observation_bump_previewed: adjustment_ready,
            adjustment_ready,
            blocked_reasons,
            telemetry: Vec::new(),
        };

        report.with_telemetry()
    }
}

impl RouterThresholdAdjustmentPreviewReport {
    pub fn summary_line(&self) -> String {
        format!(
            "router_threshold_adjustment_preview profile={} read_only={} report_only={} preview_only={} adjustment_ready={} quality={:.3} threshold_delta={:.6} rollback_anchor={} candidate_delta={:.6} blocked_reasons={}",
            profile_label(self.profile),
            self.read_only,
            self.report_only,
            self.preview_only,
            self.adjustment_ready,
            self.quality_score,
            self.threshold_delta,
            self.rollback_anchor_id,
            self.candidate_threshold - self.threshold_before,
            self.blocked_reasons.len(),
        )
    }

    fn with_telemetry(mut self) -> Self {
        self.telemetry = router_threshold_adjustment_preview_telemetry(&self);
        self
    }
}

pub(super) fn threshold_after_observation(
    threshold_before: f32,
    learning_rate: f32,
    min_threshold: f32,
    max_threshold: f32,
    metrics: GenerationMetrics,
    quality_score: f32,
) -> f32 {
    let contradiction_pressure = (metrics.contradiction_count as f32 * 0.025).min(0.12);
    let mut threshold = threshold_before;

    if quality_score < 0.58 {
        let delta = learning_rate * (0.58 - quality_score) + contradiction_pressure;
        threshold -= delta;
    } else if quality_score > 0.82 && metrics.perplexity <= 9.0 && metrics.contradiction_count == 0
    {
        let delta = learning_rate * (quality_score - 0.82);
        threshold += delta;
    }

    threshold.clamp(min_threshold, max_threshold)
}

fn normalized_learning_rate(rate: f32) -> Option<f32> {
    rate.is_finite().then(|| rate.clamp(0.0, 1.0))
}

fn normalized_threshold_bounds(min_threshold: f32, max_threshold: f32) -> Option<(f32, f32)> {
    (min_threshold.is_finite() && max_threshold.is_finite() && min_threshold <= max_threshold)
        .then_some((min_threshold.clamp(0.0, 1.0), max_threshold.clamp(0.0, 1.0)))
}

fn metrics_are_finite(metrics: GenerationMetrics) -> bool {
    metrics.perplexity.is_finite() && metrics.semantic_consistency.is_finite()
}

fn safe_threshold(
    profile_threshold: f32,
    current_threshold: f32,
    min_threshold: f32,
    max_threshold: f32,
) -> f32 {
    if profile_threshold.is_finite() {
        profile_threshold.clamp(min_threshold, max_threshold)
    } else if current_threshold.is_finite() {
        current_threshold.clamp(min_threshold, max_threshold)
    } else {
        min_threshold
    }
}

fn router_threshold_adjustment_preview_telemetry(
    report: &RouterThresholdAdjustmentPreviewReport,
) -> Vec<String> {
    let mut telemetry = vec![
        "router_threshold_adjustment_preview=true".to_owned(),
        format!(
            "router_threshold_adjustment_preview_profile={}",
            profile_label(report.profile)
        ),
        format!(
            "router_threshold_adjustment_preview_read_only={}",
            report.read_only
        ),
        format!(
            "router_threshold_adjustment_preview_report_only={}",
            report.report_only
        ),
        format!(
            "router_threshold_adjustment_preview_only={}",
            report.preview_only
        ),
        format!(
            "router_threshold_adjustment_preview_router_state_write_allowed={}",
            report.router_state_write_allowed
        ),
        format!(
            "router_threshold_adjustment_preview_adaptive_state_write_allowed={}",
            report.adaptive_state_write_allowed
        ),
        format!(
            "router_threshold_adjustment_preview_ndkv_write_allowed={}",
            report.ndkv_write_allowed
        ),
        format!(
            "router_threshold_adjustment_preview_router_observation_applied={}",
            report.router_observation_applied
        ),
        format!(
            "router_threshold_adjustment_preview_observation_bump_previewed={}",
            report.observation_bump_previewed
        ),
        format!(
            "router_threshold_adjustment_preview_ready={}",
            report.adjustment_ready
        ),
        format!(
            "router_threshold_adjustment_preview_metrics_finite={}",
            report.metrics_finite
        ),
        format!(
            "router_threshold_adjustment_preview_thresholds_finite={}",
            report.thresholds_finite
        ),
        format!(
            "router_threshold_adjustment_preview_policy_valid={}",
            report.policy_valid
        ),
        format!(
            "router_threshold_adjustment_preview_observation_delta={}",
            report.observation_delta_previewed
        ),
        format!(
            "router_threshold_adjustment_preview_minimum_token_count={}",
            report.minimum_token_count
        ),
        format!(
            "router_threshold_adjustment_preview_quality={:.3}",
            report.quality_score
        ),
        format!(
            "router_threshold_adjustment_preview_threshold_before={:.6}",
            report.threshold_before
        ),
        format!(
            "router_threshold_adjustment_preview_threshold_delta={:.6}",
            report.threshold_delta
        ),
        format!(
            "router_threshold_adjustment_preview_rollback_anchor_id={}",
            report.rollback_anchor_id
        ),
        format!(
            "router_threshold_adjustment_preview_candidate_delta={:.6}",
            report.candidate_threshold - report.threshold_before
        ),
        format!(
            "router_threshold_adjustment_preview_blocked_reasons={}",
            report.blocked_reasons.len()
        ),
    ];
    telemetry.extend(
        report
            .blocked_reasons
            .iter()
            .map(|reason| format!("router_threshold_adjustment_preview_blocked_reason={reason}")),
    );
    telemetry
}

fn router_threshold_adjustment_rollback_anchor_id(
    profile: TaskProfile,
    observations_before: u64,
    profile_observations_before: u64,
    threshold_before: f32,
    preview_threshold: f32,
    threshold_delta: f32,
    observation_delta_previewed: u64,
) -> String {
    let observations_before = observations_before.to_string();
    let profile_observations_before = profile_observations_before.to_string();
    let threshold_before = format!("{threshold_before:.6}");
    let preview_threshold = format!("{preview_threshold:.6}");
    let threshold_delta = format!("{threshold_delta:.6}");
    let observation_delta_previewed = observation_delta_previewed.to_string();
    format!(
        "router_threshold_adjustment:{}",
        stable_redaction_digest([
            "router-threshold-adjustment-preview",
            profile_label(profile),
            observations_before.as_str(),
            profile_observations_before.as_str(),
            threshold_before.as_str(),
            preview_threshold.as_str(),
            threshold_delta.as_str(),
            observation_delta_previewed.as_str(),
        ])
    )
}

fn profile_label(profile: TaskProfile) -> &'static str {
    match profile {
        TaskProfile::General => "general",
        TaskProfile::Coding => "coding",
        TaskProfile::Writing => "writing",
        TaskProfile::LongDocument => "long_document",
    }
}
