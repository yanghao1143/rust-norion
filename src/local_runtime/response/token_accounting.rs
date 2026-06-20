use crate::runtime::RuntimeToken;

use super::super::tokenizer::estimated_entropy;

pub(super) fn response_tokens(answer: &str) -> Vec<RuntimeToken> {
    answer
        .split_whitespace()
        .map(|text| RuntimeToken {
            text: text.to_owned(),
            logprob: Some(-estimated_entropy(text)),
            entropy: Some(estimated_entropy(text)),
        })
        .collect()
}
