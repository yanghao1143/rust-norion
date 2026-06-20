use super::*;
use crate::hierarchy::TaskProfile;

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
fn accelerator_headroom_spends_attention_on_borderline_tokens() {
    let router = NoironRouter::new();
    let normal = router.route_entropy("token", 0.68);
    let accelerated = router.route_entropy_with_context(
        "token",
        0.68,
        RoutingContext {
            compute_headroom: 1.0,
            ..RoutingContext::default()
        },
    );

    assert_eq!(normal.route, Route::FastProjection);
    assert!(accelerated.route.uses_attention_budget());
    assert!(accelerated.score > normal.score);
}
