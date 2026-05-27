#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    FastProjection,
    Attention,
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
    pub route: Route,
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
            .filter(|decision| decision.route == Route::Attention)
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
    min_threshold: f32,
    max_threshold: f32,
    learning_rate: f32,
    observations: u64,
}

impl Default for NoironRouter {
    fn default() -> Self {
        Self {
            threshold: 0.52,
            min_threshold: 0.18,
            max_threshold: 0.88,
            learning_rate: 0.08,
            observations: 0,
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

    pub fn observations(&self) -> u64 {
        self.observations
    }

    pub fn route_token(&self, token: &str) -> RoutingDecision {
        let entropy = estimate_token_entropy(token);
        self.route_entropy(token, entropy)
    }

    pub fn route_entropy(&self, token: &str, entropy: f32) -> RoutingDecision {
        let entropy = entropy.clamp(0.0, 1.0);
        let route = if entropy >= self.threshold {
            Route::Attention
        } else {
            Route::FastProjection
        };

        RoutingDecision {
            token: token.to_owned(),
            entropy,
            route,
        }
    }

    pub fn route_prompt(&self, prompt: &str) -> Vec<RoutingDecision> {
        tokenize(prompt)
            .into_iter()
            .map(|token| self.route_token(&token))
            .collect()
    }

    pub fn budget_for_prompt(&self, prompt: &str) -> RouteBudget {
        let decisions = self.route_prompt(prompt);
        RouteBudget::from_decisions(self.threshold, &decisions)
    }

    pub fn observe(&mut self, metrics: GenerationMetrics) {
        let quality = metrics.quality_score();
        let contradiction_pressure = (metrics.contradiction_count as f32 * 0.025).min(0.12);

        if quality < 0.58 {
            let delta = self.learning_rate * (0.58 - quality) + contradiction_pressure;
            self.threshold -= delta;
        } else if quality > 0.82 && metrics.perplexity <= 9.0 {
            let delta = self.learning_rate * (quality - 0.82);
            self.threshold += delta;
        }

        self.threshold = self.threshold.clamp(self.min_threshold, self.max_threshold);
        self.observations += 1;
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
}
