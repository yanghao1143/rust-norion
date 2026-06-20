use rust_norion::InferenceOutcome;

use crate::gemma_business::audit::{
    gemma_business_smoke_answer_failure, gemma_business_smoke_runtime_failure,
};

pub(super) fn run_gemma_business_smoke_outcome_gate(outcome: &InferenceOutcome) -> bool {
    let mut passed = true;
    if let Some(failure) = gemma_business_smoke_runtime_failure(outcome) {
        println!("gemma_business_smoke_runtime: passed=false failure={failure}");
        passed = false;
    } else {
        println!(
            "gemma_business_smoke_runtime: passed=true answer_chars={} runtime_tokens={}",
            outcome.answer.chars().count(),
            outcome.runtime_token_metrics.token_count
        );
    }
    if let Some(failure) = gemma_business_smoke_answer_failure(&outcome.answer) {
        println!("gemma_business_smoke_answer: passed=false failure={failure}");
        passed = false;
    } else {
        println!("gemma_business_smoke_answer: passed=true");
    }
    passed
}
