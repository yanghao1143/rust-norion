use super::*;
use crate::hierarchy::{HierarchyWeights, TaskComputeBudget, TaskProfile};

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

#[test]
fn adaptive_routing_plan_explains_include_compress_defer_and_skip() {
    let planner = AdaptiveRoutingPlanner::new();
    let threshold = 0.58;
    let context = RoutingContext {
        profile: TaskProfile::Coding,
        compute_headroom: 0.8,
        hierarchy: HierarchyWeights::new(0.25, 0.55, 0.20),
        ..RoutingContext::default()
    };

    let plan = planner.plan(
        TaskProfile::Coding,
        threshold,
        context,
        vec![
            route_candidate("include", 48, 0.95, 0.90, 0.95, 0.10, 0.90),
            route_candidate("compress", 96, 0.70, 0.62, 0.68, 0.30, 0.60),
            route_candidate("defer", 32, 0.52, 0.40, 0.48, 0.20, 0.38),
            route_candidate("skip", 320, 0.20, 0.16, 0.18, 0.80, 0.10),
        ],
    );

    assert_eq!(plan.candidates, 4);
    assert_eq!(plan.include, 1);
    assert_eq!(plan.compress, 1);
    assert_eq!(plan.defer, 1);
    assert_eq!(plan.skip, 1);
    assert!(plan.saved_tokens > 0);
    assert!(plan.decision_count_matches());
    assert!(plan.token_accounting_matches());
    assert!(plan.anchors_retained());
    assert!(plan.read_only);
    assert!(!plan.write_allowed);
    assert!(!plan.applied);
    assert!(
        plan.score_summaries(4)
            .iter()
            .all(|summary| summary.contains("action=")
                && summary.contains("candidate_digest=redaction-digest:")
                && summary.contains("route=")
                && summary.contains("score=")
                && summary.contains("threshold=")
                && summary.contains("task=")
                && summary.contains("fitness=")
                && summary.contains("trust=")
                && summary.contains("cost=")
                && summary.contains("reward=")
                && summary.contains("verifier_rule=")
                && summary.contains("verifier_test=")
                && summary.contains("verifier_logic=")
                && summary.contains("verifier_reward=")
                && summary.contains("verifier_cluster=")
                && summary.contains("verifier_evidence_digest=fnv64:"))
    );
    assert!(
        plan.score_summaries(4)
            .iter()
            .any(|summary| summary.contains("verifier_cluster=hold_for_review"))
    );
    assert!(
        plan.score_summaries(4)
            .iter()
            .all(|summary| !summary.contains("id="))
    );
}

#[test]
fn adaptive_routing_score_summary_redacts_polluted_candidate_and_rejects_cluster() {
    let plan = AdaptiveRoutingPlanner::new().plan(
        TaskProfile::Coding,
        0.50,
        RoutingContext {
            profile: TaskProfile::Coding,
            compute_headroom: 0.8,
            ..RoutingContext::default()
        },
        vec![route_candidate(
            "prompt: password=letmein sk-secret",
            64,
            0.88,
            0.80,
            0.82,
            0.20,
            0.74,
        )],
    );

    let summaries = plan.score_summaries(1);
    let summary = summaries.first().expect("score summary");

    assert!(summary.contains("candidate_digest=redaction-digest:"));
    assert!(summary.contains("verifier_rule=reject"));
    assert!(summary.contains("verifier_cluster=reject"));
    assert!(!summary.contains("letmein"));
    assert!(!summary.contains("sk-secret"));
    assert!(!summary.contains("prompt:"));
    assert!(!crate::privacy_redaction::contains_private_or_executable_marker(summary));
}

#[test]
fn adaptive_routing_seeded_inputs_are_deterministic_and_order_stable() {
    let planner = AdaptiveRoutingPlanner::new();
    let threshold = 0.57;
    let context = RoutingContext {
        profile: TaskProfile::Coding,
        hardware_pressure: 0.36,
        compute_headroom: 0.64,
        hierarchy: HierarchyWeights::new(0.25, 0.55, 0.20),
        ..RoutingContext::default()
    };
    let candidates = seeded_route_candidates(0xA17E_D1CE, 8);

    let first = planner.plan(TaskProfile::Coding, threshold, context, candidates.clone());
    let second = planner.plan(TaskProfile::Coding, threshold, context, candidates.clone());
    let mut reversed = candidates;
    reversed.reverse();
    let reversed = planner.plan(TaskProfile::Coding, threshold, context, reversed);

    assert_eq!(first.decisions, second.decisions);
    assert_eq!(first.decisions, reversed.decisions);
    assert!(first.decision_count_matches());
    assert!(first.token_accounting_matches());
    assert!(first.anchors_retained());
}

#[test]
fn adaptive_routing_compute_budget_skips_expensive_non_anchor() {
    let plan = AdaptiveRoutingPlanner::new().plan(
        TaskProfile::LongDocument,
        0.52,
        RoutingContext {
            profile: TaskProfile::LongDocument,
            hardware_pressure: 0.98,
            compute_headroom: 0.02,
            latency_budget_ms: Some(100),
            context_tokens: 24_000,
            ..RoutingContext::default()
        },
        vec![route_candidate(
            "expensive-runtime-kv",
            2048,
            0.72,
            0.58,
            0.50,
            0.95,
            0.42,
        )],
    );

    assert_eq!(plan.skip, 1);
    assert_eq!(plan.retained_tokens, 0);
    assert_eq!(plan.saved_tokens, 2048);
}

#[test]
fn compute_budget_scheduler_prunes_low_value_non_anchor_fanout() {
    let threshold = 0.48;
    let context = RoutingContext {
        profile: TaskProfile::Coding,
        hardware_pressure: 0.92,
        compute_headroom: 0.08,
        latency_budget_ms: Some(120),
        ..RoutingContext::default()
    };
    let budget = ComputeBudgetContext {
        profile: TaskProfile::Coding,
        compute_budget: TaskComputeBudget::Low,
        validation_mode: true,
        prompt_tokens: 320,
        max_tokens: Some(48),
        route_fanout: 4,
        runtime_kv_budget_pressure: 0.0,
    };
    let plan = AdaptiveRoutingPlanner::new().plan_with_compute_budget(
        TaskProfile::Coding,
        threshold,
        context,
        budget,
        vec![
            route_candidate("prompt-anchor", 32, 0.95, 0.90, 0.95, 0.05, 0.90)
                .with_anchor_required(true),
            route_candidate("semantic-memory", 96, 0.88, 0.82, 0.82, 0.30, 0.84),
            route_candidate("runtime-kv", 128, 0.86, 0.76, 0.78, 0.48, 0.72),
            route_candidate("low-value", 256, 0.40, 0.38, 0.36, 0.90, 0.20),
        ],
    );

    assert!(plan.schedule.threshold_after > threshold);
    assert_eq!(plan.schedule.route_fanout_after, 1);
    assert_eq!(plan.schedule.anchor_count, 1);
    assert!(plan.schedule.anchors_preserved());
    assert!(plan.schedule.low_value_skipped >= 1);
    assert!(plan.schedule.kv_lookups_skipped >= 1);
    assert!(plan.schedule.wasted_compute_avoided_tokens > 0);
    assert!(plan.routing_plan.anchors_retained());
    assert!(plan.routing_plan.include + plan.routing_plan.compress <= 1);
    assert!(plan.schedule.summary_line().contains("budget=low"));
}

#[test]
fn compute_budget_scheduler_expanded_budget_retains_more_fanout() {
    let candidates = vec![
        route_candidate("prompt-anchor", 32, 0.95, 0.90, 0.95, 0.05, 0.90)
            .with_anchor_required(true),
        route_candidate("semantic-memory", 96, 0.88, 0.82, 0.82, 0.30, 0.84),
        route_candidate("runtime-kv", 128, 0.86, 0.76, 0.78, 0.48, 0.72),
        route_candidate("gist-memory", 80, 0.80, 0.70, 0.76, 0.35, 0.70),
    ];
    let context = RoutingContext {
        profile: TaskProfile::LongDocument,
        hardware_pressure: 0.30,
        compute_headroom: 0.80,
        ..RoutingContext::default()
    };
    let low = AdaptiveRoutingPlanner::new().plan_with_compute_budget(
        TaskProfile::LongDocument,
        0.50,
        context,
        ComputeBudgetContext {
            profile: TaskProfile::LongDocument,
            compute_budget: TaskComputeBudget::Low,
            validation_mode: false,
            prompt_tokens: 640,
            max_tokens: Some(64),
            route_fanout: 4,
            runtime_kv_budget_pressure: 0.0,
        },
        candidates.clone(),
    );
    let expanded = AdaptiveRoutingPlanner::new().plan_with_compute_budget(
        TaskProfile::LongDocument,
        0.50,
        context,
        ComputeBudgetContext {
            profile: TaskProfile::LongDocument,
            compute_budget: TaskComputeBudget::Expanded,
            validation_mode: false,
            prompt_tokens: 640,
            max_tokens: Some(512),
            route_fanout: 4,
            runtime_kv_budget_pressure: 0.0,
        },
        candidates,
    );

    assert!(expanded.schedule.threshold_after < low.schedule.threshold_after);
    assert!(expanded.schedule.route_fanout_after > low.schedule.route_fanout_after);
    assert!(expanded.schedule.selected_candidates >= low.schedule.selected_candidates);
    assert!(expanded.schedule.kv_lookup_budget > low.schedule.kv_lookup_budget);
}

#[test]
fn compute_budget_runtime_kv_pressure_lifts_threshold() {
    let context = RoutingContext {
        profile: TaskProfile::Coding,
        hardware_pressure: 0.20,
        compute_headroom: 0.80,
        ..RoutingContext::default()
    };
    let candidates = vec![
        route_candidate("prompt-anchor", 32, 0.95, 0.90, 0.95, 0.05, 0.90)
            .with_anchor_required(true),
        route_candidate("runtime-kv", 128, 0.86, 0.76, 0.78, 0.48, 0.72),
    ];
    let normal = AdaptiveRoutingPlanner::new().plan_with_compute_budget(
        TaskProfile::Coding,
        0.50,
        context,
        ComputeBudgetContext {
            profile: TaskProfile::Coding,
            compute_budget: TaskComputeBudget::Normal,
            validation_mode: false,
            prompt_tokens: 256,
            max_tokens: Some(256),
            route_fanout: 3,
            runtime_kv_budget_pressure: 0.0,
        },
        candidates.clone(),
    );
    let pressured = AdaptiveRoutingPlanner::new().plan_with_compute_budget(
        TaskProfile::Coding,
        0.50,
        context,
        ComputeBudgetContext {
            profile: TaskProfile::Coding,
            compute_budget: TaskComputeBudget::Normal,
            validation_mode: false,
            prompt_tokens: 256,
            max_tokens: Some(256),
            route_fanout: 3,
            runtime_kv_budget_pressure: 0.8,
        },
        candidates,
    );

    assert!(pressured.schedule.threshold_after > normal.schedule.threshold_after);
    assert_eq!(pressured.schedule.runtime_kv_budget_pressure, 0.8);
    assert!(
        pressured
            .schedule
            .summary_line()
            .contains("runtime_kv_budget_pressure=0.800")
    );
    assert!(
        pressured
            .schedule
            .notes
            .iter()
            .any(|note| { note == "runtime_kv_budget_pressure=0.800" })
    );
}

#[test]
fn adaptive_routing_task_profile_changes_hierarchy_action() {
    let candidate = route_candidate("rust-anchor", 32, 0.58, 0.55, 0.68, 0.18, 0.60);
    let general = AdaptiveRoutingPlanner::new().plan(
        TaskProfile::General,
        0.55,
        RoutingContext::default(),
        vec![candidate.clone()],
    );
    let coding = AdaptiveRoutingPlanner::new().plan(
        TaskProfile::Coding,
        0.55,
        RoutingContext {
            profile: TaskProfile::Coding,
            hierarchy: HierarchyWeights::new(0.20, 0.65, 0.15),
            ..RoutingContext::default()
        },
        vec![candidate],
    );

    assert!(coding.average_score > general.average_score);
    assert!(coding.include >= general.include);
}

#[test]
fn adaptive_routing_reward_history_reinforces_or_penalizes_candidates() {
    let rewarded = route_candidate("rewarded-memory", 64, 0.58, 0.62, 0.70, 0.18, 0.95);
    let penalized = route_candidate("penalized-memory", 64, 0.58, 0.62, 0.70, 0.18, 0.05);
    let context = RoutingContext {
        profile: TaskProfile::Coding,
        ..RoutingContext::default()
    };

    let plan = AdaptiveRoutingPlanner::new().plan(
        TaskProfile::Coding,
        0.58,
        context,
        vec![rewarded, penalized],
    );
    let rewarded = plan
        .decisions
        .iter()
        .find(|decision| decision.candidate_id == "rewarded-memory")
        .unwrap();
    let penalized = plan
        .decisions
        .iter()
        .find(|decision| decision.candidate_id == "penalized-memory")
        .unwrap();

    assert!(rewarded.score > penalized.score);
    assert_ne!(rewarded.action, AdaptiveRouteAction::Skip);
    assert!(matches!(
        penalized.action,
        AdaptiveRouteAction::Defer | AdaptiveRouteAction::Skip
    ));
}

fn assert_threshold_close(left: f32, right: f32) {
    assert!((left - right).abs() < 0.0001);
}

fn route_candidate(
    id: &str,
    estimated_tokens: usize,
    task_intent: f32,
    memory_fitness: f32,
    trust: f32,
    compute_cost: f32,
    reward_history: f32,
) -> AdaptiveRouteCandidate {
    AdaptiveRouteCandidate::new(
        id,
        AdaptiveRouteSource::SemanticMemory,
        estimated_tokens,
        AdaptiveRouteScoreComponents::new(
            task_intent,
            0.55,
            0.90,
            memory_fitness,
            0.70,
            trust,
            compute_cost,
            reward_history,
        ),
    )
}

fn seeded_route_candidates(seed: u32, count: usize) -> Vec<AdaptiveRouteCandidate> {
    let mut state = seed;
    (0..count)
        .map(|index| {
            let task_intent = seeded_unit(&mut state);
            let memory_fitness = seeded_unit(&mut state);
            let trust = seeded_unit(&mut state);
            let compute_cost = seeded_unit(&mut state);
            let reward_history = seeded_unit(&mut state);
            let estimated_tokens = 32 + (seeded_u32(&mut state) as usize % 256);

            route_candidate(
                &format!("seeded-{index:02}"),
                estimated_tokens,
                task_intent,
                memory_fitness,
                trust,
                compute_cost,
                reward_history,
            )
            .with_anchor_required(index == 0)
        })
        .collect()
}

fn seeded_unit(state: &mut u32) -> f32 {
    (seeded_u32(state) % 1000) as f32 / 1000.0
}

fn seeded_u32(state: &mut u32) -> u32 {
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *state
}
