use crate::hierarchy::TaskProfile;
use crate::reflection::{DraftToken, InferenceDraft};
use crate::router::{GenerationMetrics, NoironRouter, RoutingContext};

use super::model::TokenWindowReport;
use super::observation::observe_token;
use super::tokenizer::tokenize_generated;

#[derive(Debug, Clone)]
pub struct TokenStreamMonitor {
    window_size: usize,
}

impl Default for TokenStreamMonitor {
    fn default() -> Self {
        Self { window_size: 10 }
    }
}

impl TokenStreamMonitor {
    pub fn new(window_size: usize) -> Self {
        Self {
            window_size: window_size.max(1),
        }
    }

    pub fn window_size(&self) -> usize {
        self.window_size
    }

    pub fn observe_generated(
        &self,
        router: &mut NoironRouter,
        generated: &str,
        semantic_consistency: f32,
        contradiction_count: usize,
    ) -> Vec<TokenWindowReport> {
        self.observe_generated_with_profile(
            router,
            TaskProfile::General,
            generated,
            semantic_consistency,
            contradiction_count,
        )
    }

    pub fn observe_generated_with_profile(
        &self,
        router: &mut NoironRouter,
        profile: TaskProfile,
        generated: &str,
        semantic_consistency: f32,
        contradiction_count: usize,
    ) -> Vec<TokenWindowReport> {
        let tokens = tokenize_generated(generated);
        let mut reports = Vec::new();
        let routing_context = RoutingContext {
            profile,
            ..RoutingContext::default()
        };

        for (window_index, chunk) in tokens.chunks(self.window_size).enumerate() {
            let start_token = window_index * self.window_size;
            let observations = chunk
                .iter()
                .map(|token| {
                    observe_token(
                        router.route_token_with_context(token, routing_context),
                        semantic_consistency,
                    )
                })
                .collect::<Vec<_>>();
            let attention_count = observations
                .iter()
                .filter(|observation| observation.route.uses_attention_budget())
                .count();
            let token_count = observations.len().max(1);
            let average_loss = observations
                .iter()
                .map(|observation| observation.loss)
                .sum::<f32>()
                / token_count as f32;
            let is_last_window = start_token + token_count >= tokens.len();
            let window_contradictions = if is_last_window {
                contradiction_count
            } else {
                0
            };
            let metrics = GenerationMetrics {
                perplexity: average_loss,
                semantic_consistency: semantic_consistency.clamp(0.0, 1.0),
                contradiction_count: window_contradictions,
                token_count,
            };

            router.observe_with_profile(profile, metrics);
            reports.push(TokenWindowReport {
                start_token,
                end_token: start_token + token_count,
                metrics,
                attention_fraction: attention_count as f32 / token_count as f32,
                threshold_after: router.threshold_for(profile),
                observations,
            });
        }

        reports
    }

    pub fn observe_draft(
        &self,
        router: &mut NoironRouter,
        draft: &InferenceDraft,
        semantic_consistency: f32,
        contradiction_count: usize,
    ) -> Vec<TokenWindowReport> {
        self.observe_draft_with_profile(
            router,
            TaskProfile::General,
            draft,
            semantic_consistency,
            contradiction_count,
        )
    }

    pub fn observe_draft_with_profile(
        &self,
        router: &mut NoironRouter,
        profile: TaskProfile,
        draft: &InferenceDraft,
        semantic_consistency: f32,
        contradiction_count: usize,
    ) -> Vec<TokenWindowReport> {
        if draft.tokens.is_empty() {
            self.observe_generated_with_profile(
                router,
                profile,
                &draft.answer,
                semantic_consistency,
                contradiction_count,
            )
        } else {
            self.observe_tokens_with_profile(
                router,
                profile,
                &draft.tokens,
                semantic_consistency,
                contradiction_count,
            )
        }
    }

    pub fn observe_tokens(
        &self,
        router: &mut NoironRouter,
        tokens: &[DraftToken],
        semantic_consistency: f32,
        contradiction_count: usize,
    ) -> Vec<TokenWindowReport> {
        self.observe_tokens_with_profile(
            router,
            TaskProfile::General,
            tokens,
            semantic_consistency,
            contradiction_count,
        )
    }

    pub fn observe_tokens_with_profile(
        &self,
        router: &mut NoironRouter,
        profile: TaskProfile,
        tokens: &[DraftToken],
        semantic_consistency: f32,
        contradiction_count: usize,
    ) -> Vec<TokenWindowReport> {
        let mut reports = Vec::new();
        let routing_context = RoutingContext {
            profile,
            ..RoutingContext::default()
        };

        for (window_index, chunk) in tokens.chunks(self.window_size).enumerate() {
            let start_token = window_index * self.window_size;
            let observations = chunk
                .iter()
                .map(|token| {
                    let entropy = token.entropy.unwrap_or_else(|| {
                        router
                            .route_token_with_context(&token.text, routing_context)
                            .entropy
                    });
                    let decision =
                        router.route_entropy_with_context(&token.text, entropy, routing_context);
                    let mut observation = observe_token(decision, semantic_consistency);
                    if let Some(logprob) = token.logprob {
                        let logprob_loss = (-logprob).max(0.0);
                        observation.loss = (observation.loss + logprob_loss) / 2.0;
                    }
                    observation
                })
                .collect::<Vec<_>>();
            let attention_count = observations
                .iter()
                .filter(|observation| observation.route.uses_attention_budget())
                .count();
            let token_count = observations.len().max(1);
            let average_loss = observations
                .iter()
                .map(|observation| observation.loss)
                .sum::<f32>()
                / token_count as f32;
            let is_last_window = start_token + token_count >= tokens.len();
            let window_contradictions = if is_last_window {
                contradiction_count
            } else {
                0
            };
            let metrics = GenerationMetrics {
                perplexity: average_loss,
                semantic_consistency: semantic_consistency.clamp(0.0, 1.0),
                contradiction_count: window_contradictions,
                token_count,
            };

            router.observe_with_profile(profile, metrics);
            reports.push(TokenWindowReport {
                start_token,
                end_token: start_token + token_count,
                metrics,
                attention_fraction: attention_count as f32 / token_count as f32,
                threshold_after: router.threshold_for(profile),
                observations,
            });
        }

        reports
    }
}
