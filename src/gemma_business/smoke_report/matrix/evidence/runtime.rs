use crate::gemma_business::response_json::{response_bool_field, response_optional_string_field};
use crate::gemma_business::smoke_report::types::GemmaBusinessCycleCaseResult;

pub(super) struct MatrixRuntimeEvidence {
    pub(super) model: String,
    pub(super) any_uncertainty: bool,
}

impl MatrixRuntimeEvidence {
    pub(super) fn from_cases(case_results: &[GemmaBusinessCycleCaseResult]) -> Self {
        Self {
            model: case_results
                .iter()
                .find_map(|result| response_optional_string_field(&result.body, "runtime_model"))
                .unwrap_or_default(),
            any_uncertainty: case_results
                .iter()
                .any(|result| response_bool_field(&result.body, "runtime_uncertainty_signal")),
        }
    }
}
