use crate::reflection::{DraftToken, InferenceDraft};
use crate::router::{GenerationMetrics, NoironRouter, Route, RoutingDecision};

#[derive(Debug, Clone)]
pub struct TokenObservation {
    pub token: String,
    pub entropy: f32,
    pub route: Route,
    pub loss: f32,
    pub consistency: f32,
}

#[derive(Debug, Clone)]
pub struct TokenWindowReport {
    pub start_token: usize,
    pub end_token: usize,
    pub metrics: GenerationMetrics,
    pub attention_fraction: f32,
    pub threshold_after: f32,
    pub observations: Vec<TokenObservation>,
}

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
        let tokens = tokenize_generated(generated);
        let mut reports = Vec::new();

        for (window_index, chunk) in tokens.chunks(self.window_size).enumerate() {
            let start_token = window_index * self.window_size;
            let observations = chunk
                .iter()
                .map(|token| observe_token(router.route_token(token), semantic_consistency))
                .collect::<Vec<_>>();
            let attention_count = observations
                .iter()
                .filter(|observation| observation.route == Route::Attention)
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

            router.observe(metrics);
            reports.push(TokenWindowReport {
                start_token,
                end_token: start_token + token_count,
                metrics,
                attention_fraction: attention_count as f32 / token_count as f32,
                threshold_after: router.threshold(),
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
        if draft.tokens.is_empty() {
            self.observe_generated(
                router,
                &draft.answer,
                semantic_consistency,
                contradiction_count,
            )
        } else {
            self.observe_tokens(
                router,
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
        let mut reports = Vec::new();

        for (window_index, chunk) in tokens.chunks(self.window_size).enumerate() {
            let start_token = window_index * self.window_size;
            let observations = chunk
                .iter()
                .map(|token| {
                    let entropy = token
                        .entropy
                        .unwrap_or_else(|| router.route_token(&token.text).entropy);
                    let decision = router.route_entropy(&token.text, entropy);
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
                .filter(|observation| observation.route == Route::Attention)
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

            router.observe(metrics);
            reports.push(TokenWindowReport {
                start_token,
                end_token: start_token + token_count,
                metrics,
                attention_fraction: attention_count as f32 / token_count as f32,
                threshold_after: router.threshold(),
                observations,
            });
        }

        reports
    }
}

fn observe_token(decision: RoutingDecision, semantic_consistency: f32) -> TokenObservation {
    let route_mismatch = match decision.route {
        Route::FastProjection if decision.entropy > 0.68 => 2.2,
        Route::Attention if decision.entropy < 0.24 => 0.35,
        _ => 0.0,
    };
    let consistency = semantic_consistency.clamp(0.0, 1.0);
    let loss = 2.0 + decision.entropy * 4.0 + (1.0 - consistency) * 8.0 + route_mismatch;

    TokenObservation {
        token: decision.token,
        entropy: decision.entropy,
        route: decision.route,
        loss,
        consistency,
    }
}

fn tokenize_generated(text: &str) -> Vec<String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_stream_updates_router_per_window() {
        let mut router = NoironRouter::new();
        let monitor = TokenStreamMonitor::new(4);
        let before = router.threshold();
        let reports = monitor.observe_generated(
            &mut router,
            "alpha beta gamma delta epsilon zeta eta theta",
            0.99,
            0,
        );

        assert_eq!(reports.len(), 2);
        assert_eq!(router.observations(), 2);
        assert!(router.threshold() >= before);
    }

    #[test]
    fn weak_stream_lowers_threshold() {
        let mut router = NoironRouter::new();
        let monitor = TokenStreamMonitor::new(4);
        let before = router.threshold();
        monitor.observe_generated(
            &mut router,
            "uncertain contradiction maybe unstable output",
            0.1,
            2,
        );

        assert!(router.threshold() < before);
    }

    #[test]
    fn runtime_tokens_use_supplied_entropy() {
        let mut router = NoironRouter::new();
        let monitor = TokenStreamMonitor::new(2);
        let tokens = vec![
            DraftToken {
                text: "easy".to_owned(),
                logprob: Some(-0.1),
                entropy: Some(0.1),
            },
            DraftToken {
                text: "hard".to_owned(),
                logprob: Some(-1.2),
                entropy: Some(0.9),
            },
        ];

        let reports = monitor.observe_tokens(&mut router, &tokens, 0.95, 0);

        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].observations[0].route, Route::FastProjection);
        assert_eq!(reports[0].observations[1].route, Route::Attention);
    }
}
