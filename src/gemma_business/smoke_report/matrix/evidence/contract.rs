use crate::gemma_business::smoke_report::types::GemmaBusinessCycleCaseResult;

pub(super) struct MatrixContractSignals {
    pub(super) required_signals: usize,
    pub(super) matched_signals: usize,
    pub(super) missing_signals: Vec<String>,
}

impl MatrixContractSignals {
    pub(super) fn from_cases(case_results: &[GemmaBusinessCycleCaseResult]) -> Self {
        Self {
            required_signals: case_results
                .iter()
                .map(|result| result.answer_audit.required_signals)
                .sum(),
            matched_signals: case_results
                .iter()
                .map(|result| result.answer_audit.matched_signals)
                .sum(),
            missing_signals: case_results
                .iter()
                .flat_map(|result| result.answer_audit.missing_signals.iter().cloned())
                .collect(),
        }
    }
}
