use crate::gemma_business::smoke_report::types::GemmaBusinessCycleCaseResult;

pub(super) struct MatrixReportContract {
    pub(super) runtime_model_experiences: bool,
    pub(super) protocol_leak: bool,
    pub(super) substituted_runtime_model_experiences: bool,
    pub(super) evasive_denial: bool,
    pub(super) handling_signal: bool,
}

impl MatrixReportContract {
    pub(super) fn from_cases(case_results: &[GemmaBusinessCycleCaseResult]) -> Self {
        Self {
            runtime_model_experiences: case_results
                .iter()
                .all(|result| result.answer_audit.has_runtime_model_experiences),
            protocol_leak: case_results
                .iter()
                .any(|result| result.answer_audit.protocol_leak),
            substituted_runtime_model_experiences: case_results
                .iter()
                .any(|result| result.answer_audit.substituted_runtime_model_experiences),
            evasive_denial: case_results
                .iter()
                .any(|result| result.answer_audit.evasive_denial),
            handling_signal: case_results
                .iter()
                .all(|result| result.answer_audit.handling_signal),
        }
    }
}
