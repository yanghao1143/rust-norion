use crate::hierarchy::TaskProfile;

use super::adaptive::AdaptiveRoutingPlanner;
use super::adjustment::threshold_after_observation;
use super::scoring::{choose_route, estimate_token_entropy, routing_score, tokenize};
use super::types::{
    AdaptiveRouteCandidate, AdaptiveRoutingPlan, GenerationMetrics, ProfileObservations,
    ProfileThresholds, Route, RouteBudget, RouterState, RoutingContext, RoutingDecision,
};

#[derive(Debug, Clone)]
pub struct NoironRouter {
    threshold: f32,
    profile_thresholds: ProfileThresholds,
    min_threshold: f32,
    max_threshold: f32,
    learning_rate: f32,
    observations: u64,
    profile_observations: ProfileObservations,
}

impl Default for NoironRouter {
    fn default() -> Self {
        let threshold = 0.52;
        Self {
            threshold,
            profile_thresholds: ProfileThresholds::from_single(threshold),
            min_threshold: 0.18,
            max_threshold: 0.88,
            learning_rate: 0.08,
            observations: 0,
            profile_observations: ProfileObservations::default(),
        }
    }
}

impl NoironRouter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn threshold(&self) -> f32 {
        self.threshold
    }

    pub fn threshold_for(&self, profile: TaskProfile) -> f32 {
        self.profile_thresholds
            .get(profile)
            .clamp(self.min_threshold, self.max_threshold)
    }

    pub fn observations(&self) -> u64 {
        self.observations
    }

    pub fn state(&self) -> RouterState {
        RouterState {
            threshold: self.threshold,
            observations: self.observations,
            profile_thresholds: self.profile_thresholds,
            profile_observations: self.profile_observations,
        }
    }

    pub fn restore_state(&mut self, state: RouterState) {
        self.threshold = state
            .threshold
            .clamp(self.min_threshold, self.max_threshold);
        self.profile_thresholds = state
            .profile_thresholds
            .clamp(self.min_threshold, self.max_threshold);
        self.observations = state.observations;
        self.profile_observations = state.profile_observations;
    }

    pub fn route_token(&self, token: &str) -> RoutingDecision {
        let entropy = estimate_token_entropy(token);
        self.route_entropy_with_context(token, entropy, RoutingContext::default())
    }

    pub fn route_entropy(&self, token: &str, entropy: f32) -> RoutingDecision {
        self.route_entropy_with_context(token, entropy, RoutingContext::default())
    }

    pub fn route_token_with_context(
        &self,
        token: &str,
        context: RoutingContext,
    ) -> RoutingDecision {
        let entropy = estimate_token_entropy(token);
        self.route_entropy_with_context(token, entropy, context)
    }

    pub fn route_entropy_with_context(
        &self,
        token: &str,
        entropy: f32,
        context: RoutingContext,
    ) -> RoutingDecision {
        self.route_entropy_with_context_threshold(
            token,
            entropy,
            context,
            self.threshold_for(context.profile),
        )
    }

    pub fn route_entropy_with_context_threshold(
        &self,
        token: &str,
        entropy: f32,
        context: RoutingContext,
        threshold: f32,
    ) -> RoutingDecision {
        let entropy = entropy.clamp(0.0, 1.0);
        let score = routing_score(entropy, context);
        let threshold = threshold.clamp(self.min_threshold, self.max_threshold);
        let route = if score < threshold {
            Route::FastProjection
        } else {
            choose_route(score, threshold, context)
        };

        RoutingDecision {
            token: token.to_owned(),
            entropy,
            score,
            route,
        }
    }

    pub fn route_prompt(&self, prompt: &str) -> Vec<RoutingDecision> {
        self.route_prompt_with_context(prompt, RoutingContext::default())
    }

    pub fn route_prompt_with_context(
        &self,
        prompt: &str,
        context: RoutingContext,
    ) -> Vec<RoutingDecision> {
        self.route_prompt_with_context_threshold(
            prompt,
            context,
            self.threshold_for(context.profile),
        )
    }

    pub fn route_prompt_with_context_threshold(
        &self,
        prompt: &str,
        context: RoutingContext,
        threshold: f32,
    ) -> Vec<RoutingDecision> {
        tokenize(prompt)
            .into_iter()
            .map(|token| {
                self.route_entropy_with_context_threshold(
                    &token,
                    estimate_token_entropy(&token),
                    context,
                    threshold,
                )
            })
            .collect()
    }

    pub fn budget_for_prompt(&self, prompt: &str) -> RouteBudget {
        self.budget_for_prompt_with_context(prompt, RoutingContext::default())
    }

    pub fn budget_for_prompt_with_context(
        &self,
        prompt: &str,
        context: RoutingContext,
    ) -> RouteBudget {
        self.budget_for_prompt_with_context_threshold(
            prompt,
            context,
            self.threshold_for(context.profile),
        )
    }

    pub fn budget_for_prompt_with_context_threshold(
        &self,
        prompt: &str,
        context: RoutingContext,
        threshold: f32,
    ) -> RouteBudget {
        let threshold = threshold.clamp(self.min_threshold, self.max_threshold);
        let decisions = self.route_prompt_with_context_threshold(prompt, context, threshold);
        RouteBudget::from_decisions(threshold, &decisions)
    }

    pub fn adaptive_plan_with_context(
        &self,
        context: RoutingContext,
        candidates: Vec<AdaptiveRouteCandidate>,
    ) -> AdaptiveRoutingPlan {
        AdaptiveRoutingPlanner::new().plan(
            context.profile,
            self.threshold_for(context.profile),
            context,
            candidates,
        )
    }

    pub fn observe(&mut self, metrics: GenerationMetrics) {
        self.observe_with_profile(TaskProfile::General, metrics);
    }

    pub fn observe_with_profile(&mut self, profile: TaskProfile, metrics: GenerationMetrics) {
        let quality = metrics.quality_score();
        let threshold = threshold_after_observation(
            self.threshold_for(profile),
            self.learning_rate,
            self.min_threshold,
            self.max_threshold,
            metrics,
            quality,
        );
        self.profile_thresholds.set(profile, threshold);
        self.threshold = threshold;
        self.observations += 1;
        self.profile_observations.bump(profile);
    }
}
