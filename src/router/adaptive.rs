use crate::hierarchy::TaskProfile;

use super::budget::{BudgetedAdaptiveRoutingPlan, ComputeBudgetContext, ComputeBudgetScheduler};
use super::scoring::choose_route;
use super::types::{
    AdaptiveRouteAction, AdaptiveRouteCandidate, AdaptiveRouteDecision,
    AdaptiveRouteScoreComponents, AdaptiveRouteSource, AdaptiveRoutingPlan, Route, RoutingContext,
};

#[derive(Debug, Clone, Copy)]
pub struct AdaptiveRoutingPolicy {
    pub include_margin: f32,
    pub compress_margin: f32,
    pub defer_margin: f32,
    pub compression_fraction: f32,
}

impl Default for AdaptiveRoutingPolicy {
    fn default() -> Self {
        Self {
            include_margin: 0.15,
            compress_margin: 0.00,
            defer_margin: 0.26,
            compression_fraction: 0.34,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AdaptiveRoutingPlanner {
    pub policy: AdaptiveRoutingPolicy,
}

impl Default for AdaptiveRoutingPlanner {
    fn default() -> Self {
        Self {
            policy: AdaptiveRoutingPolicy::default(),
        }
    }
}

impl AdaptiveRoutingPlanner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_policy(mut self, policy: AdaptiveRoutingPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn plan(
        &self,
        profile: TaskProfile,
        threshold: f32,
        context: RoutingContext,
        candidates: Vec<AdaptiveRouteCandidate>,
    ) -> AdaptiveRoutingPlan {
        let threshold = finite_unit(threshold).unwrap_or(0.52);
        let mut decisions = candidates
            .into_iter()
            .filter(valid_route_candidate)
            .map(|candidate| self.decide(profile, threshold, context, candidate))
            .collect::<Vec<_>>();
        decisions.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));
        AdaptiveRoutingPlan::from_decisions(profile, threshold, decisions)
    }

    pub fn plan_with_compute_budget(
        &self,
        profile: TaskProfile,
        threshold: f32,
        context: RoutingContext,
        budget: ComputeBudgetContext,
        candidates: Vec<AdaptiveRouteCandidate>,
    ) -> BudgetedAdaptiveRoutingPlan {
        let candidates = candidates
            .into_iter()
            .filter(valid_route_candidate)
            .collect::<Vec<_>>();
        let scheduler = ComputeBudgetScheduler::new();
        let threshold_after = scheduler.threshold_for(threshold, context, budget, &candidates);
        let plan = self.plan(profile, threshold_after, context, candidates.clone());
        scheduler.schedule(threshold, context, budget, &candidates, plan)
    }

    fn decide(
        &self,
        profile: TaskProfile,
        threshold: f32,
        context: RoutingContext,
        candidate: AdaptiveRouteCandidate,
    ) -> AdaptiveRouteDecision {
        let components = candidate.components.clamp();
        let pressure = compute_pressure(context, &candidate);
        let score = adaptive_score(profile, context, components, pressure);
        let action = choose_action(
            &self.policy,
            threshold,
            pressure,
            &candidate,
            components,
            score,
        );
        let route = adaptive_route(profile, context, threshold, candidate.source, action, score);
        let retained_tokens = retained_tokens(
            action,
            candidate.estimated_tokens,
            self.policy.compression_fraction,
        );
        let reason = decision_reason(action, pressure, components, score, threshold);

        AdaptiveRouteDecision {
            candidate_id: candidate.id,
            source: candidate.source,
            estimated_tokens: candidate.estimated_tokens,
            retained_tokens,
            anchor_required: candidate.anchor_required,
            components,
            score,
            threshold,
            route,
            action,
            compute_pressure: pressure,
            reason,
        }
    }
}

fn choose_action(
    policy: &AdaptiveRoutingPolicy,
    threshold: f32,
    pressure: f32,
    candidate: &AdaptiveRouteCandidate,
    components: AdaptiveRouteScoreComponents,
    score: f32,
) -> AdaptiveRouteAction {
    if candidate.anchor_required && score >= (threshold - policy.defer_margin).clamp(0.0, 1.0) {
        return AdaptiveRouteAction::Include;
    }

    if pressure >= 0.82 && components.compute_cost >= 0.70 && !candidate.anchor_required {
        return if score >= (threshold + policy.include_margin + 0.10).clamp(0.0, 1.0) {
            AdaptiveRouteAction::Compress
        } else {
            AdaptiveRouteAction::Skip
        };
    }

    if score >= (threshold + policy.include_margin).clamp(0.0, 1.0) {
        AdaptiveRouteAction::Include
    } else if score >= (threshold - policy.compress_margin).clamp(0.0, 1.0) {
        AdaptiveRouteAction::Compress
    } else if score >= (threshold - policy.defer_margin).clamp(0.0, 1.0) {
        AdaptiveRouteAction::Defer
    } else {
        AdaptiveRouteAction::Skip
    }
}

fn adaptive_route(
    profile: TaskProfile,
    context: RoutingContext,
    threshold: f32,
    source: AdaptiveRouteSource,
    action: AdaptiveRouteAction,
    score: f32,
) -> Route {
    match action {
        AdaptiveRouteAction::Include => choose_route(score, threshold, context),
        AdaptiveRouteAction::Compress => {
            if profile == TaskProfile::LongDocument || source.prefers_fusion() {
                Route::ConvolutionalFusion
            } else {
                Route::LocalWindowAttention
            }
        }
        AdaptiveRouteAction::Defer | AdaptiveRouteAction::Skip => Route::FastProjection,
    }
}

fn retained_tokens(
    action: AdaptiveRouteAction,
    estimated_tokens: usize,
    compression_fraction: f32,
) -> usize {
    match action {
        AdaptiveRouteAction::Include => estimated_tokens,
        AdaptiveRouteAction::Compress => {
            let retained =
                (estimated_tokens as f32 * compression_fraction.clamp(0.10, 0.90)).ceil();
            retained as usize
        }
        AdaptiveRouteAction::Defer | AdaptiveRouteAction::Skip => 0,
    }
    .min(estimated_tokens)
}

fn adaptive_score(
    profile: TaskProfile,
    context: RoutingContext,
    components: AdaptiveRouteScoreComponents,
    compute_pressure: f32,
) -> f32 {
    let reward_penalty = (0.40 - components.reward_history).max(0.0) * 0.50;
    let base = components.task_intent * 0.16
        + components.language_mode * 0.08
        + components.code_mode * 0.10
        + components.memory_fitness * 0.22
        + components.recency * 0.11
        + components.trust * 0.20
        + components.reward_history * 0.13
        - components.compute_cost * 0.18;
    let profile_bonus = match profile {
        TaskProfile::Coding => components.code_mode * 0.08,
        TaskProfile::Writing => components.language_mode * 0.07,
        TaskProfile::LongDocument => {
            components.task_intent * 0.04 + components.memory_fitness * 0.04
        }
        TaskProfile::General => 0.0,
    };
    let hierarchy_bonus = context.hierarchy.global * 0.025
        + context.hierarchy.local * 0.020
        + context.hierarchy.convolution * 0.030;

    (base + profile_bonus + hierarchy_bonus - compute_pressure * 0.15 - reward_penalty)
        .clamp(0.0, 1.0)
}

fn compute_pressure(context: RoutingContext, candidate: &AdaptiveRouteCandidate) -> f32 {
    let hardware = context.hardware_pressure.clamp(0.0, 1.0) * 0.42;
    let headroom = (0.55 - context.compute_headroom.clamp(0.0, 1.0)).max(0.0) * 0.35;
    let token_pressure = (candidate.estimated_tokens as f32 / 2048.0).min(1.0) * 0.16;
    let latency = match context.latency_budget_ms {
        Some(budget) if budget <= 150 => 0.24,
        Some(budget) if budget <= 500 => 0.10,
        _ => 0.0,
    };

    (hardware + headroom + token_pressure + latency).clamp(0.0, 1.0)
}

fn valid_route_candidate(candidate: &AdaptiveRouteCandidate) -> bool {
    !candidate.id.trim().is_empty() && candidate.estimated_tokens > 0
}

fn decision_reason(
    action: AdaptiveRouteAction,
    pressure: f32,
    components: AdaptiveRouteScoreComponents,
    score: f32,
    threshold: f32,
) -> String {
    format!(
        "action={} score={:.3} threshold={:.3} pressure={:.3} task={:.3} fitness={:.3} trust={:.3} cost={:.3}",
        action.as_str(),
        score,
        threshold,
        pressure,
        components.task_intent,
        components.memory_fitness,
        components.trust,
        components.compute_cost
    )
}

fn finite_unit(value: f32) -> Option<f32> {
    value.is_finite().then(|| value.clamp(0.0, 1.0))
}
