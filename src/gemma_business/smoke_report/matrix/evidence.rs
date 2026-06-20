mod contract;
mod preview;
mod runtime;

use crate::gemma_business::GEMMA_MODEL_SERVICE_BUSINESS_CASES;
use crate::gemma_business::smoke_report::types::GemmaBusinessCycleCaseResult;
use contract::MatrixContractSignals;
use preview::matrix_answer_preview;
use runtime::MatrixRuntimeEvidence;

pub(super) struct MatrixReportEvidence {
    pub(super) business_cases: Vec<String>,
    pub(super) case_count: usize,
    pub(super) passed_cases: usize,
    pub(super) contract_required_signals: usize,
    pub(super) contract_matched_signals: usize,
    pub(super) missing_signals: Vec<String>,
    pub(super) runtime_model: String,
    pub(super) any_runtime_uncertainty: bool,
    pub(super) answer_preview: String,
}

impl MatrixReportEvidence {
    pub(super) fn from_cases(case_results: &[GemmaBusinessCycleCaseResult]) -> Self {
        let contract = MatrixContractSignals::from_cases(case_results);
        let runtime = MatrixRuntimeEvidence::from_cases(case_results);
        Self {
            business_cases: GEMMA_MODEL_SERVICE_BUSINESS_CASES
                .iter()
                .map(|business_case| business_case.name.to_owned())
                .collect(),
            case_count: case_results.len(),
            passed_cases: case_results.iter().filter(|result| result.passed).count(),
            contract_required_signals: contract.required_signals,
            contract_matched_signals: contract.matched_signals,
            missing_signals: contract.missing_signals,
            runtime_model: runtime.model,
            any_runtime_uncertainty: runtime.any_uncertainty,
            answer_preview: matrix_answer_preview(case_results),
        }
    }

    pub(super) fn contract_passed(&self) -> bool {
        self.missing_signals.is_empty() && self.passed_cases == self.case_count
    }
}
