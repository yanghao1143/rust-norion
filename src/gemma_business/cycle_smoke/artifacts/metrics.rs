use crate::gemma_business::smoke_report::GemmaBusinessCycleCaseResult;

pub(in crate::gemma_business::cycle_smoke) struct BusinessCycleSmokeMetrics {
    pub(in crate::gemma_business::cycle_smoke) runtime_token_count: u64,
    pub(in crate::gemma_business::cycle_smoke) feedback_applied: u64,
    pub(in crate::gemma_business::cycle_smoke) rust_check_feedback_applied: u64,
    pub(in crate::gemma_business::cycle_smoke) checked_trace_lines: u64,
    pub(in crate::gemma_business::cycle_smoke) passed_cases: usize,
}

impl BusinessCycleSmokeMetrics {
    pub(in crate::gemma_business::cycle_smoke) fn from_cases(
        case_results: &[GemmaBusinessCycleCaseResult],
    ) -> Self {
        Self {
            runtime_token_count: case_results
                .iter()
                .map(|result| result.runtime_token_count)
                .sum(),
            feedback_applied: case_results
                .iter()
                .map(|result| result.feedback_applied)
                .sum(),
            rust_check_feedback_applied: case_results
                .iter()
                .map(|result| result.rust_check_feedback_applied)
                .sum(),
            checked_trace_lines: case_results
                .iter()
                .map(|result| result.checked_trace_lines)
                .max()
                .unwrap_or_default(),
            passed_cases: case_results.iter().filter(|result| result.passed).count(),
        }
    }
}
