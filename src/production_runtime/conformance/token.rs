use crate::runtime::RuntimeToken;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct ProductionTokenUncertainty {
    pub(super) uncertainty_token_count: usize,
    pub(super) average_entropy: Option<f32>,
    pub(super) average_neg_logprob: Option<f32>,
    pub(super) uncertainty_perplexity: Option<f32>,
}

impl ProductionTokenUncertainty {
    pub(super) fn from_tokens(tokens: &[RuntimeToken]) -> Self {
        let mut entropy_total = 0.0;
        let mut entropy_count = 0;
        let mut neg_logprob_total = 0.0;
        let mut logprob_count = 0;
        let mut loss_total = 0.0;
        let mut loss_count = 0;
        let mut uncertainty_token_count = 0;

        for token in tokens {
            let entropy = token.entropy.and_then(production_bounded_entropy);
            let neg_logprob = token.logprob.and_then(production_bounded_neg_logprob);

            if entropy.is_some() || neg_logprob.is_some() {
                uncertainty_token_count += 1;
            }
            if let Some(entropy) = entropy {
                entropy_total += entropy;
                entropy_count += 1;
            }
            if let Some(neg_logprob) = neg_logprob {
                neg_logprob_total += neg_logprob;
                logprob_count += 1;
            }

            match (entropy, neg_logprob) {
                (Some(entropy), Some(neg_logprob)) => {
                    loss_total += 2.0 + entropy * 4.0 + neg_logprob;
                    loss_count += 1;
                }
                (Some(entropy), None) => {
                    loss_total += 2.0 + entropy * 4.0;
                    loss_count += 1;
                }
                (None, Some(neg_logprob)) => {
                    loss_total += 2.0 + neg_logprob;
                    loss_count += 1;
                }
                (None, None) => {}
            }
        }

        Self {
            uncertainty_token_count,
            average_entropy: production_average(entropy_total, entropy_count),
            average_neg_logprob: production_average(neg_logprob_total, logprob_count),
            uncertainty_perplexity: production_average(loss_total, loss_count),
        }
    }
}

fn production_bounded_entropy(value: f32) -> Option<f32> {
    value.is_finite().then(|| value.clamp(0.0, 4.0))
}

fn production_bounded_neg_logprob(value: f32) -> Option<f32> {
    let value = -value;
    value.is_finite().then(|| value.clamp(0.0, 12.0))
}

fn production_average(total: f32, count: usize) -> Option<f32> {
    if count == 0 {
        None
    } else {
        Some(total / count as f32)
    }
}
