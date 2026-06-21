use super::*;
use crate::hierarchy::{HierarchyWeights, TaskProfile};

#[test]
fn poor_quality_lowers_threshold() {
    let mut router = NoironRouter::new();
    let before = router.threshold();

    router.observe(GenerationMetrics {
        perplexity: 30.0,
        semantic_consistency: 0.2,
        contradiction_count: 2,
        token_count: 32,
    });

    assert!(router.threshold() < before);
}

#[test]
fn good_quality_raises_threshold() {
    let mut router = NoironRouter::new();
    let before = router.threshold();

    router.observe(GenerationMetrics {
        perplexity: 4.0,
        semantic_consistency: 0.98,
        contradiction_count: 0,
        token_count: 32,
    });

    assert!(router.threshold() > before);
}

#[test]
fn profile_specific_observations_update_only_that_threshold() {
    let mut router = NoironRouter::new();
    let coding_before = router.threshold_for(TaskProfile::Coding);
    let writing_before = router.threshold_for(TaskProfile::Writing);

    router.observe_with_profile(
        TaskProfile::Writing,
        GenerationMetrics {
            perplexity: 30.0,
            semantic_consistency: 0.2,
            contradiction_count: 2,
            token_count: 32,
        },
    );

    assert_eq!(router.threshold_for(TaskProfile::Coding), coding_before);
    assert!(router.threshold_for(TaskProfile::Writing) < writing_before);
    assert_eq!(
        router
            .state()
            .profile_observations
            .get(TaskProfile::Writing),
        1
    );
}

#[test]
fn router_threshold_adjustment_preview_matches_observe_without_mutating_state() {
    let router = NoironRouter::new();
    let state = router.state();
    let metrics = GenerationMetrics {
        perplexity: 36.0,
        semantic_consistency: 0.20,
        contradiction_count: 2,
        token_count: 64,
    };

    let preview =
        RouterThresholdAdjustmentPreviewPlanner::new().preview(state, TaskProfile::Coding, metrics);

    assert!(preview.read_only);
    assert!(preview.report_only);
    assert!(preview.preview_only);
    assert!(!preview.router_state_write_allowed);
    assert!(!preview.adaptive_state_write_allowed);
    assert!(!preview.ndkv_write_allowed);
    assert!(!preview.router_observation_applied);
    assert!(preview.observation_bump_previewed);
    assert!(preview.adjustment_ready);
    assert!(preview.metrics_finite);
    assert!(preview.thresholds_finite);
    assert!(preview.policy_valid);
    assert!(preview.blocked_reasons.is_empty());
    assert_eq!(preview.observations_before, 0);
    assert_eq!(preview.profile_observations_before, 0);
    assert_eq!(preview.observation_delta_previewed, 1);
    assert!(preview.preview_threshold < preview.threshold_before);
    assert!(preview.threshold_delta < 0.0);
    assert!(
        preview
            .telemetry
            .iter()
            .any(|line| line == "router_threshold_adjustment_preview_ready=true")
    );
    assert!(
        preview
            .summary_line()
            .contains("router_threshold_adjustment_preview")
    );
    assert_threshold_close(
        router.threshold_for(TaskProfile::Coding),
        state.profile_thresholds.get(TaskProfile::Coding),
    );
    assert_eq!(router.observations(), 0);

    let mut observed = NoironRouter::new();
    observed.restore_state(state);
    observed.observe_with_profile(TaskProfile::Coding, metrics);
    let observed_state = observed.state();

    assert_threshold_close(
        preview.preview_threshold,
        observed.threshold_for(TaskProfile::Coding),
    );
    assert_threshold_close(preview.preview_threshold, observed.threshold());
    assert_eq!(observed_state.observations, preview.observations_before + 1);
    assert_eq!(
        observed_state.profile_observations.get(TaskProfile::Coding),
        preview.profile_observations_before + 1
    );
}

#[test]
fn router_threshold_adjustment_preview_updates_only_selected_profile() {
    let router = NoironRouter::new();
    let state = router.state();
    let metrics = GenerationMetrics {
        perplexity: 36.0,
        semantic_consistency: 0.20,
        contradiction_count: 2,
        token_count: 64,
    };

    let preview = RouterThresholdAdjustmentPreviewPlanner::new().preview(
        state,
        TaskProfile::Writing,
        metrics,
    );

    assert!(preview.adjustment_ready);
    assert_threshold_close(
        preview.preview_profile_thresholds.get(TaskProfile::Writing),
        preview.preview_threshold,
    );
    assert_threshold_close(
        preview.preview_profile_thresholds.get(TaskProfile::General),
        state.profile_thresholds.get(TaskProfile::General),
    );
    assert_threshold_close(
        preview.preview_profile_thresholds.get(TaskProfile::Coding),
        state.profile_thresholds.get(TaskProfile::Coding),
    );
    assert_threshold_close(
        preview
            .preview_profile_thresholds
            .get(TaskProfile::LongDocument),
        state.profile_thresholds.get(TaskProfile::LongDocument),
    );
}

#[test]
fn router_threshold_adjustment_preview_raises_high_quality_threshold() {
    let state = NoironRouter::new().state();
    let metrics = GenerationMetrics {
        perplexity: 4.0,
        semantic_consistency: 0.98,
        contradiction_count: 0,
        token_count: 64,
    };

    let preview = RouterThresholdAdjustmentPreviewPlanner::new().preview(
        state,
        TaskProfile::General,
        metrics,
    );

    assert!(preview.adjustment_ready);
    assert!(preview.preview_threshold > preview.threshold_before);
    assert!(preview.threshold_delta > 0.0);

    let mut observed = NoironRouter::new();
    observed.restore_state(state);
    observed.observe_with_profile(TaskProfile::General, metrics);

    assert_threshold_close(
        preview.preview_threshold,
        observed.threshold_for(TaskProfile::General),
    );
}

#[test]
fn router_threshold_adjustment_preview_blocks_non_finite_generation_metrics() {
    let state = NoironRouter::new().state();
    let metrics = GenerationMetrics {
        perplexity: f32::NAN,
        semantic_consistency: 0.90,
        contradiction_count: 0,
        token_count: 32,
    };

    let preview = RouterThresholdAdjustmentPreviewPlanner::new().preview(
        state,
        TaskProfile::Writing,
        metrics,
    );

    assert!(!preview.adjustment_ready);
    assert!(!preview.observation_bump_previewed);
    assert_eq!(preview.observation_delta_previewed, 0);
    assert!(!preview.router_observation_applied);
    assert_eq!(preview.threshold_delta, 0.0);
    assert_eq!(preview.preview_threshold, preview.threshold_before);
    assert!(!preview.metrics_finite);
    assert!(
        preview
            .blocked_reasons
            .contains(&"router_threshold_adjustment_generation_metrics_not_finite".to_owned())
    );
    assert!(preview.telemetry.iter().any(|line| line
        == "router_threshold_adjustment_preview_blocked_reason=router_threshold_adjustment_generation_metrics_not_finite"));
}

#[test]
fn router_threshold_adjustment_preview_blocks_empty_metric_samples() {
    let state = NoironRouter::new().state();
    let metrics = GenerationMetrics {
        perplexity: 12.0,
        semantic_consistency: 0.90,
        contradiction_count: 0,
        token_count: 0,
    };

    let preview = RouterThresholdAdjustmentPreviewPlanner::new().preview(
        state,
        TaskProfile::General,
        metrics,
    );

    assert!(!preview.adjustment_ready);
    assert!(!preview.observation_bump_previewed);
    assert_eq!(preview.minimum_token_count, 1);
    assert_eq!(preview.threshold_delta, 0.0);
    assert_eq!(
        preview.blocked_reasons,
        vec!["router_threshold_adjustment_token_count=0<1".to_owned()]
    );

    let zero_policy_preview = RouterThresholdAdjustmentPreviewPlanner::new()
        .with_policy(RouterThresholdAdjustmentPreviewPolicy {
            learning_rate: 0.08,
            min_threshold: 0.18,
            max_threshold: 0.88,
            minimum_token_count: 0,
        })
        .preview(state, TaskProfile::General, metrics);

    assert!(!zero_policy_preview.adjustment_ready);
    assert_eq!(zero_policy_preview.minimum_token_count, 1);
    assert_eq!(
        zero_policy_preview.blocked_reasons,
        vec!["router_threshold_adjustment_token_count=0<1".to_owned()]
    );
}

#[test]
fn route_budget_uses_profile_specific_threshold() {
    let mut router = NoironRouter::new();
    router.observe_with_profile(
        TaskProfile::LongDocument,
        GenerationMetrics {
            perplexity: 30.0,
            semantic_consistency: 0.2,
            contradiction_count: 2,
            token_count: 64,
        },
    );

    let budget = router.budget_for_prompt_with_context(
        "long document memory routing",
        RoutingContext {
            profile: TaskProfile::LongDocument,
            ..RoutingContext::default()
        },
    );

    assert_eq!(
        budget.threshold,
        router.threshold_for(TaskProfile::LongDocument)
    );
}

#[test]
fn routing_context_selects_long_document_convolution() {
    let router = NoironRouter::new();
    let decision = router.route_entropy_with_context(
        "context",
        0.9,
        RoutingContext {
            profile: TaskProfile::LongDocument,
            context_tokens: 16_384,
            ..RoutingContext::default()
        },
    );

    assert_eq!(decision.route, Route::ConvolutionalFusion);
}

#[test]
fn latency_budget_can_keep_token_on_fast_path() {
    let router = NoironRouter::new();
    let normal = router.route_entropy("token", 0.78);
    let constrained = router.route_entropy_with_context(
        "token",
        0.78,
        RoutingContext {
            latency_budget_ms: Some(100),
            ..RoutingContext::default()
        },
    );

    assert!(normal.route.uses_attention_budget());
    assert_eq!(constrained.route, Route::FastProjection);
}

#[test]
fn cache_hits_conserve_attention_for_reusable_context() {
    let router = NoironRouter::new();
    let uncached = router.route_entropy_with_context(
        "token",
        0.78,
        RoutingContext {
            cache_hit_rate: 0.0,
            ..RoutingContext::default()
        },
    );
    let cached = router.route_entropy_with_context(
        "token",
        0.78,
        RoutingContext {
            cache_hit_rate: 1.0,
            ..RoutingContext::default()
        },
    );

    assert!(uncached.route.uses_attention_budget());
    assert_eq!(cached.route, Route::FastProjection);
    assert!(cached.score < uncached.score);
}

#[test]
fn hardware_pressure_conserves_attention_budget() {
    let router = NoironRouter::new();
    let normal = router.route_entropy("token", 0.76);
    let constrained = router.route_entropy_with_context(
        "token",
        0.76,
        RoutingContext {
            hardware_pressure: 0.95,
            compute_headroom: 0.08,
            ..RoutingContext::default()
        },
    );

    assert!(normal.route.uses_attention_budget());
    assert_eq!(constrained.route, Route::FastProjection);
    assert!(constrained.score < normal.score);
}

#[test]
fn generation_metrics_quality_score_stays_finite_for_bad_inputs() {
    let score = GenerationMetrics {
        perplexity: f32::NAN,
        semantic_consistency: f32::INFINITY,
        contradiction_count: usize::MAX,
        token_count: 32,
    }
    .quality_score();

    assert!(score.is_finite());
    assert_eq!(score, 0.0);
}

#[test]
fn accelerator_headroom_spends_attention_on_borderline_tokens() {
    let router = NoironRouter::new();
    let normal = router.route_entropy("token", 0.62);
    let accelerated = router.route_entropy_with_context(
        "token",
        0.62,
        RoutingContext {
            compute_headroom: 1.0,
            ..RoutingContext::default()
        },
    );

    assert_eq!(normal.route, Route::FastProjection);
    assert!(accelerated.route.uses_attention_budget());
    assert!(accelerated.score > normal.score);
}

#[test]
fn hierarchy_bias_spends_attention_on_borderline_tokens() {
    let router = NoironRouter::new();
    let local_heavy = router.route_entropy_with_context(
        "token",
        0.665,
        RoutingContext {
            hierarchy: HierarchyWeights::new(0.0, 1.0, 0.0),
            ..RoutingContext::default()
        },
    );
    let global_heavy = router.route_entropy_with_context(
        "token",
        0.665,
        RoutingContext {
            hierarchy: HierarchyWeights::new(1.0, 0.0, 0.0),
            ..RoutingContext::default()
        },
    );

    assert_eq!(local_heavy.route, Route::FastProjection);
    assert!(global_heavy.route.uses_attention_budget());
    assert!(global_heavy.score > local_heavy.score);
}

fn assert_threshold_close(left: f32, right: f32) {
    assert!((left - right).abs() < 0.0001);
}
