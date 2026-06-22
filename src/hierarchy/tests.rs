use super::*;
use crate::router::GenerationMetrics;

#[test]
fn coding_profile_prefers_local_attention() {
    let target = HierarchyController::target_for_profile(TaskProfile::Coding);

    assert!(target.local > target.global);
    assert!(target.local > target.convolution);
}

#[test]
fn weights_are_normalized() {
    let weights = HierarchyWeights::new(10.0, 5.0, 1.0);
    let sum = weights.global + weights.local + weights.convolution;

    assert!((sum - 1.0).abs() < 0.0001);
}

#[test]
fn observations_update_only_selected_profile_weights() {
    let mut controller = HierarchyController::new();
    let coding_before = controller.state().profile_weights.get(TaskProfile::Coding);
    let writing_before = controller.state().profile_weights.get(TaskProfile::Writing);

    controller.observe(
        TaskProfile::Writing,
        GenerationMetrics {
            perplexity: 30.0,
            semantic_consistency: 0.2,
            contradiction_count: 2,
            token_count: 32,
        },
    );

    let state = controller.state();
    let coding_after = state.profile_weights.get(TaskProfile::Coding);
    let writing_after = state.profile_weights.get(TaskProfile::Writing);
    assert!((coding_after.local - coding_before.local).abs() < 0.0001);
    assert!(writing_after.global > writing_before.global);
    assert_eq!(state.profile_observations.get(TaskProfile::Writing), 1);
    assert_eq!(state.profile_observations.get(TaskProfile::Coding), 0);
}

#[test]
fn adapt_to_profile_uses_profile_specific_learned_weights() {
    let mut controller = HierarchyController::new();
    controller.observe(
        TaskProfile::LongDocument,
        GenerationMetrics {
            perplexity: 32.0,
            semantic_consistency: 0.2,
            contradiction_count: 1,
            token_count: 64,
        },
    );
    let learned_long = controller
        .state()
        .profile_weights
        .get(TaskProfile::LongDocument);

    let adapted_coding = controller.adapt_to_profile(TaskProfile::Coding);
    let adapted_long = controller.adapt_to_profile(TaskProfile::LongDocument);

    assert!(adapted_coding.local > adapted_coding.convolution);
    assert!(learned_long.convolution > adapted_coding.convolution);
    assert!(adapted_long.convolution > adapted_coding.convolution);
}

#[test]
fn hierarchy_adjustment_preview_matches_controller_observe_without_mutating_state() {
    let controller = HierarchyController::new();
    let state = controller.state();
    let metrics = GenerationMetrics {
        perplexity: 36.0,
        semantic_consistency: 0.20,
        contradiction_count: 2,
        token_count: 64,
    };

    let preview =
        HierarchyAdjustmentPreviewPlanner::new().preview(state, TaskProfile::Coding, metrics);

    assert!(preview.read_only);
    assert!(preview.report_only);
    assert!(preview.preview_only);
    assert!(!preview.state_write_allowed);
    assert!(!preview.adaptive_state_write_allowed);
    assert!(!preview.ndkv_write_allowed);
    assert!(!preview.controller_observation_applied);
    assert!(preview.observation_bump_previewed);
    assert!(preview.adjustment_ready);
    assert!(preview.metrics_finite);
    assert!(preview.weights_finite);
    assert!(preview.policy_valid);
    assert!(preview.blocked_reasons.is_empty());
    assert_eq!(preview.observations_before, 0);
    assert_eq!(preview.observation_delta_previewed, 1);
    assert!(preview.preview_profile.local > preview.profile_before.local);
    assert!(preview.delta.abs_total > 0.0);
    assert!(preview.candidate_delta.abs_total > 0.0);
    assert!(
        preview
            .telemetry
            .iter()
            .any(|line| line == "hierarchy_adjustment_preview_ready=true")
    );
    assert!(
        preview
            .summary_line()
            .contains("hierarchy_adjustment_preview")
    );
    assert_weights_close(
        state.profile_weights.get(TaskProfile::Coding),
        controller.state().profile_weights.get(TaskProfile::Coding),
    );

    let mut observed = HierarchyController::new();
    observed.restore_state(state);
    let observed_weights = observed.observe(TaskProfile::Coding, metrics);

    assert_weights_close(preview.preview_current, observed_weights);
    assert_eq!(
        observed
            .state()
            .profile_observations
            .get(TaskProfile::Coding),
        1
    );
}

#[test]
fn hierarchy_adjustment_preview_blocks_non_finite_generation_metrics() {
    let state = HierarchyController::new().state();
    let metrics = GenerationMetrics {
        perplexity: f32::NAN,
        semantic_consistency: 0.90,
        contradiction_count: 0,
        token_count: 32,
    };

    let preview =
        HierarchyAdjustmentPreviewPlanner::new().preview(state, TaskProfile::Writing, metrics);

    assert!(!preview.adjustment_ready);
    assert!(!preview.observation_bump_previewed);
    assert_eq!(preview.observation_delta_previewed, 0);
    assert!(!preview.controller_observation_applied);
    assert_eq!(preview.delta.abs_total, 0.0);
    assert_eq!(preview.candidate_delta.abs_total, 0.0);
    assert!(!preview.metrics_finite);
    assert!(
        preview
            .blocked_reasons
            .contains(&"hierarchy_adjustment_generation_metrics_not_finite".to_owned())
    );
    assert!(
        preview
            .telemetry
            .iter()
            .any(|line| line
                == "hierarchy_adjustment_preview_blocked_reason=hierarchy_adjustment_generation_metrics_not_finite")
    );
}

#[test]
fn hierarchy_adjustment_preview_blocks_empty_metric_samples() {
    let state = HierarchyController::new().state();
    let metrics = GenerationMetrics {
        perplexity: 12.0,
        semantic_consistency: 0.90,
        contradiction_count: 0,
        token_count: 0,
    };

    let preview =
        HierarchyAdjustmentPreviewPlanner::new().preview(state, TaskProfile::General, metrics);

    assert!(!preview.adjustment_ready);
    assert!(!preview.observation_bump_previewed);
    assert_eq!(preview.minimum_token_count, 1);
    assert_eq!(
        preview.blocked_reasons,
        vec!["hierarchy_adjustment_token_count=0<1".to_owned()]
    );

    let zero_policy_preview = HierarchyAdjustmentPreviewPlanner::new()
        .with_policy(HierarchyAdjustmentPreviewPolicy {
            learning_rate: 0.22,
            max_component_delta: 0.08,
            minimum_token_count: 0,
        })
        .preview(state, TaskProfile::General, metrics);

    assert!(!zero_policy_preview.adjustment_ready);
    assert_eq!(zero_policy_preview.minimum_token_count, 1);
    assert_eq!(
        zero_policy_preview.blocked_reasons,
        vec!["hierarchy_adjustment_token_count=0<1".to_owned()]
    );
}

#[test]
fn hierarchy_adjustment_preview_blocks_candidate_delta_over_policy_limit() {
    let state = HierarchyController::new().state();
    let metrics = GenerationMetrics {
        perplexity: 36.0,
        semantic_consistency: 0.20,
        contradiction_count: 2,
        token_count: 64,
    };

    let preview = HierarchyAdjustmentPreviewPlanner::new()
        .with_policy(HierarchyAdjustmentPreviewPolicy {
            learning_rate: 0.22,
            max_component_delta: 0.0001,
            minimum_token_count: 1,
        })
        .preview(state, TaskProfile::Coding, metrics);

    assert!(!preview.adjustment_ready);
    assert!(!preview.observation_bump_previewed);
    assert_eq!(preview.delta.abs_total, 0.0);
    assert!(preview.candidate_delta.abs_total > 0.0);
    assert_weights_close(preview.preview_profile, preview.profile_before);
    assert!(
        preview
            .blocked_reasons
            .iter()
            .any(|reason| { reason.starts_with("hierarchy_adjustment_max_component_delta=") })
    );
    assert!(preview.telemetry.iter().any(|line| line.starts_with(
        "hierarchy_adjustment_preview_blocked_reason=hierarchy_adjustment_max_component_delta="
    )));
}

#[test]
fn hierarchy_adjustment_preview_blocks_invalid_policy_and_non_finite_state() {
    let mut state = HierarchyController::new().state();
    state.current.global = f32::NAN;
    let metrics = GenerationMetrics {
        perplexity: 12.0,
        semantic_consistency: 0.90,
        contradiction_count: 0,
        token_count: 32,
    };

    let preview = HierarchyAdjustmentPreviewPlanner::new()
        .with_policy(HierarchyAdjustmentPreviewPolicy {
            learning_rate: f32::NAN,
            max_component_delta: f32::INFINITY,
            minimum_token_count: 1,
        })
        .preview(state, TaskProfile::General, metrics);

    assert!(!preview.adjustment_ready);
    assert!(!preview.policy_valid);
    assert!(!preview.weights_finite);
    assert_eq!(preview.delta.abs_total, 0.0);
    assert_eq!(preview.candidate_delta.abs_total, 0.0);
    assert!(
        preview
            .blocked_reasons
            .contains(&"hierarchy_adjustment_learning_rate_not_finite".to_owned())
    );
    assert!(
        preview
            .blocked_reasons
            .contains(&"hierarchy_adjustment_max_component_delta_not_finite".to_owned())
    );
    assert!(
        preview
            .blocked_reasons
            .contains(&"hierarchy_adjustment_current_weights_not_finite".to_owned())
    );
}

#[test]
fn hierarchy_weights_normalize_non_finite_values_without_nan() {
    let weights = HierarchyWeights::new(f32::INFINITY, f32::NAN, 1.0);
    let sum = weights.global + weights.local + weights.convolution;

    assert!(weights.global.is_finite());
    assert!(weights.local.is_finite());
    assert!(weights.convolution.is_finite());
    assert!((sum - 1.0).abs() < 0.0001);
    assert_eq!(weights.global, 0.0);
    assert_eq!(weights.local, 0.0);
    assert_eq!(weights.convolution, 1.0);

    let huge = HierarchyWeights::new(f32::MAX, f32::MAX, f32::MAX);
    let huge_sum = huge.global + huge.local + huge.convolution;
    assert!(huge.global.is_finite());
    assert!(huge.local.is_finite());
    assert!(huge.convolution.is_finite());
    assert!((huge_sum - 1.0).abs() < 0.0001);
}

#[test]
fn hierarchy_weights_blend_ignores_non_finite_rate() {
    let before = HierarchyWeights::new(0.2, 0.7, 0.1);
    let target = HierarchyWeights::new(0.7, 0.2, 0.1);

    let blended = before.blend(target, f32::NAN);

    assert_weights_close(blended, before);
}

fn assert_weights_close(left: HierarchyWeights, right: HierarchyWeights) {
    assert!((left.global - right.global).abs() < 0.0001);
    assert!((left.local - right.local).abs() < 0.0001);
    assert!((left.convolution - right.convolution).abs() < 0.0001);
}
