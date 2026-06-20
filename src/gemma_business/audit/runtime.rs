use rust_norion::InferenceOutcome;

pub fn gemma_business_smoke_runtime_failure(outcome: &InferenceOutcome) -> Option<String> {
    gemma_business_smoke_runtime_failure_parts(
        &outcome.answer,
        outcome.runtime_token_metrics.token_count,
    )
}

pub fn gemma_business_smoke_runtime_failure_parts(
    answer: &str,
    runtime_token_count: usize,
) -> Option<String> {
    if let Some(failure) = gemma_business_smoke_runtime_failure_text(answer) {
        return Some(failure);
    }
    if runtime_token_count == 0 {
        return Some("runtime did not report generated token evidence".to_owned());
    }
    None
}

pub fn gemma_business_smoke_runtime_failure_text(answer: &str) -> Option<String> {
    let answer = answer.trim();
    let lower = answer.to_ascii_lowercase();
    if lower.contains("runtime backend error") {
        return Some("runtime backend returned an error draft".to_owned());
    }
    if lower.starts_with("error ") || lower.contains("error 404") {
        return Some("runtime output contains an error response".to_owned());
    }
    if answer.is_empty() {
        return Some("runtime output was empty".to_owned());
    }
    None
}
