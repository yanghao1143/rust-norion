use crate::hierarchy::TaskProfile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    FastProjection,
    LocalWindowAttention,
    GlobalAttention,
    ConvolutionalFusion,
}

impl Route {
    pub fn uses_attention_budget(self) -> bool {
        self != Self::FastProjection
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GenerationMetrics {
    pub perplexity: f32,
    pub semantic_consistency: f32,
    pub contradiction_count: usize,
    pub token_count: usize,
}

impl GenerationMetrics {
    pub fn quality_score(self) -> f32 {
        let perplexity_score = (1.0 / (1.0 + self.perplexity / 12.0)).clamp(0.0, 1.0);
        let consistency_score = self.semantic_consistency.clamp(0.0, 1.0);
        let contradiction_penalty = (self.contradiction_count as f32 * 0.18).min(0.72);
        ((perplexity_score * 0.35) + (consistency_score * 0.65) - contradiction_penalty)
            .clamp(0.0, 1.0)
    }
}

#[derive(Debug, Clone)]
pub struct RoutingDecision {
    pub token: String,
    pub entropy: f32,
    pub score: f32,
    pub route: Route,
}

#[derive(Debug, Clone, Copy)]
pub struct RoutingContext {
    pub profile: TaskProfile,
    pub context_tokens: usize,
    pub cache_hit_rate: f32,
    pub latency_budget_ms: Option<u64>,
    pub hardware_pressure: f32,
    pub compute_headroom: f32,
}

impl Default for RoutingContext {
    fn default() -> Self {
        Self {
            profile: TaskProfile::General,
            context_tokens: 0,
            cache_hit_rate: 0.0,
            latency_budget_ms: None,
            hardware_pressure: 0.0,
            compute_headroom: 0.5,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RouteBudget {
    pub threshold: f32,
    pub attention_tokens: usize,
    pub fast_tokens: usize,
    pub attention_fraction: f32,
}

impl RouteBudget {
    pub fn from_decisions(threshold: f32, decisions: &[RoutingDecision]) -> Self {
        let attention_tokens = decisions
            .iter()
            .filter(|decision| decision.route.uses_attention_budget())
            .count();
        let fast_tokens = decisions.len().saturating_sub(attention_tokens);
        let total = decisions.len().max(1) as f32;

        Self {
            threshold,
            attention_tokens,
            fast_tokens,
            attention_fraction: attention_tokens as f32 / total,
        }
    }
}

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

#[derive(Debug, Clone, Copy)]
pub struct RouterState {
    pub threshold: f32,
    pub observations: u64,
    pub profile_thresholds: ProfileThresholds,
    pub profile_observations: ProfileObservations,
}

#[derive(Debug, Clone, Copy)]
pub struct ProfileThresholds {
    pub general: f32,
    pub coding: f32,
    pub writing: f32,
    pub long_document: f32,
}

impl ProfileThresholds {
    pub fn from_single(threshold: f32) -> Self {
        Self {
            general: threshold,
            coding: threshold,
            writing: threshold,
            long_document: threshold,
        }
    }

    pub fn get(self, profile: TaskProfile) -> f32 {
        match profile {
            TaskProfile::General => self.general,
            TaskProfile::Coding => self.coding,
            TaskProfile::Writing => self.writing,
            TaskProfile::LongDocument => self.long_document,
        }
    }

    pub fn set(&mut self, profile: TaskProfile, threshold: f32) {
        match profile {
            TaskProfile::General => self.general = threshold,
            TaskProfile::Coding => self.coding = threshold,
            TaskProfile::Writing => self.writing = threshold,
            TaskProfile::LongDocument => self.long_document = threshold,
        }
    }

    pub fn clamp(self, min_threshold: f32, max_threshold: f32) -> Self {
        Self {
            general: self.general.clamp(min_threshold, max_threshold),
            coding: self.coding.clamp(min_threshold, max_threshold),
            writing: self.writing.clamp(min_threshold, max_threshold),
            long_document: self.long_document.clamp(min_threshold, max_threshold),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ProfileObservations {
    pub general: u64,
    pub coding: u64,
    pub writing: u64,
    pub long_document: u64,
}

impl ProfileObservations {
    pub fn from_single(observations: u64) -> Self {
        Self {
            general: observations,
            coding: 0,
            writing: 0,
            long_document: 0,
        }
    }

    pub fn get(self, profile: TaskProfile) -> u64 {
        match profile {
            TaskProfile::General => self.general,
            TaskProfile::Coding => self.coding,
            TaskProfile::Writing => self.writing,
            TaskProfile::LongDocument => self.long_document,
        }
    }

    pub fn bump(&mut self, profile: TaskProfile) {
        match profile {
            TaskProfile::General => self.general = self.general.saturating_add(1),
            TaskProfile::Coding => self.coding = self.coding.saturating_add(1),
            TaskProfile::Writing => self.writing = self.writing.saturating_add(1),
            TaskProfile::LongDocument => {
                self.long_document = self.long_document.saturating_add(1);
            }
        }
    }
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
        let entropy = entropy.clamp(0.0, 1.0);
        let score = routing_score(entropy, context);
        let threshold = self.threshold_for(context.profile);
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
        tokenize(prompt)
            .into_iter()
            .map(|token| self.route_token_with_context(&token, context))
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
        let decisions = self.route_prompt_with_context(prompt, context);
        RouteBudget::from_decisions(self.threshold_for(context.profile), &decisions)
    }

    pub fn observe(&mut self, metrics: GenerationMetrics) {
        self.observe_with_profile(TaskProfile::General, metrics);
    }

    pub fn observe_with_profile(&mut self, profile: TaskProfile, metrics: GenerationMetrics) {
        let quality = metrics.quality_score();
        let contradiction_pressure = (metrics.contradiction_count as f32 * 0.025).min(0.12);
        let mut threshold = self.threshold_for(profile);

        if quality < 0.58 {
            let delta = self.learning_rate * (0.58 - quality) + contradiction_pressure;
            threshold -= delta;
        } else if quality > 0.82 && metrics.perplexity <= 9.0 {
            let delta = self.learning_rate * (quality - 0.82);
            threshold += delta;
        }

        threshold = threshold.clamp(self.min_threshold, self.max_threshold);
        self.profile_thresholds.set(profile, threshold);
        self.threshold = threshold;
        self.observations += 1;
        self.profile_observations.bump(profile);
    }
}

fn tokenize(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();

    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || (!ch.is_ascii() && !ch.is_whitespace()) {
            current.push(ch);
        } else if !current.is_empty() {
            out.push(std::mem::take(&mut current));
        }
    }

    if !current.is_empty() {
        out.push(current);
    }

    out
}

fn estimate_token_entropy(token: &str) -> f32 {
    if token.is_empty() {
        return 0.0;
    }

    let len = token.chars().count() as f32;
    let unique = token
        .chars()
        .collect::<std::collections::HashSet<_>>()
        .len() as f32;
    let unique_ratio = unique / len.max(1.0);
    let symbol_ratio = token
        .chars()
        .filter(|ch| !ch.is_alphanumeric() && *ch != '_')
        .count() as f32
        / len.max(1.0);
    let digit_ratio = token.chars().filter(|ch| ch.is_ascii_digit()).count() as f32 / len.max(1.0);
    let case_mix = if token.chars().any(|ch| ch.is_ascii_uppercase())
        && token.chars().any(|ch| ch.is_ascii_lowercase())
    {
        0.08
    } else {
        0.0
    };
    let length_pressure = (len / 24.0).min(0.22);

    (unique_ratio * 0.52 + symbol_ratio * 0.16 + digit_ratio * 0.12 + case_mix + length_pressure)
        .clamp(0.0, 1.0)
}

fn routing_score(entropy: f32, context: RoutingContext) -> f32 {
    let task_pressure = match context.profile {
        TaskProfile::General => 0.0,
        TaskProfile::Coding => 0.05,
        TaskProfile::Writing => 0.08,
        TaskProfile::LongDocument => 0.10,
    };
    let context_pressure = (context.context_tokens as f32 / 32_000.0).min(0.18);
    let cache_discount = context.cache_hit_rate.clamp(0.0, 1.0) * 0.10;
    let latency_discount = match context.latency_budget_ms {
        Some(budget) if budget <= 150 => 0.10,
        Some(budget) if budget <= 500 => 0.04,
        _ => 0.0,
    };
    let compute_headroom = context.compute_headroom.clamp(0.0, 1.0);
    let hardware_pressure_discount = context.hardware_pressure.clamp(0.0, 1.0) * 0.16;
    let constrained_device_discount = (0.5 - compute_headroom).max(0.0) * 0.10;
    let accelerator_bonus = (compute_headroom - 0.5).max(0.0) * 0.12;

    (entropy * 0.72 + task_pressure + context_pressure + accelerator_bonus
        - cache_discount
        - latency_discount
        - hardware_pressure_discount
        - constrained_device_discount)
        .clamp(0.0, 1.0)
}

fn choose_route(score: f32, threshold: f32, context: RoutingContext) -> Route {
    match context.profile {
        TaskProfile::LongDocument if context.context_tokens >= 8_192 => Route::ConvolutionalFusion,
        TaskProfile::LongDocument if score < threshold + 0.18 => Route::ConvolutionalFusion,
        TaskProfile::Coding if score < threshold + 0.24 => Route::LocalWindowAttention,
        TaskProfile::Writing => Route::GlobalAttention,
        _ if score >= threshold + 0.24 => Route::GlobalAttention,
        _ => Route::LocalWindowAttention,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
