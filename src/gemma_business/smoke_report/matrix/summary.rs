use super::evidence::MatrixReportEvidence;
use crate::gemma_business::GEMMA_MODEL_SERVICE_BUSINESS_CASES;

pub(super) struct MatrixReportSummary {
    pub(super) expected_case_count: usize,
    pub(super) all_expected_cases_passed: bool,
}

impl MatrixReportSummary {
    pub(super) fn from_evidence(evidence: &MatrixReportEvidence) -> Self {
        let expected_case_count = GEMMA_MODEL_SERVICE_BUSINESS_CASES.len();
        Self {
            expected_case_count,
            all_expected_cases_passed: evidence.passed_cases == evidence.case_count
                && evidence.case_count == expected_case_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{MatrixReportEvidence, MatrixReportSummary};
    use crate::gemma_business::GEMMA_MODEL_SERVICE_BUSINESS_CASES;

    #[test]
    fn matrix_summary_passes_only_when_every_expected_case_passes() {
        let expected = GEMMA_MODEL_SERVICE_BUSINESS_CASES.len();

        assert!(summary_for(expected, expected).all_expected_cases_passed);
        assert!(
            !summary_for(expected.saturating_sub(1), expected.saturating_sub(1))
                .all_expected_cases_passed
        );
        assert!(!summary_for(expected, expected.saturating_sub(1)).all_expected_cases_passed);
    }

    #[test]
    fn matrix_summary_records_expected_case_count() {
        assert_eq!(
            summary_for(0, 0).expected_case_count,
            GEMMA_MODEL_SERVICE_BUSINESS_CASES.len()
        );
    }

    fn summary_for(case_count: usize, passed_cases: usize) -> MatrixReportSummary {
        MatrixReportSummary::from_evidence(&MatrixReportEvidence {
            business_cases: Vec::new(),
            case_count,
            passed_cases,
            contract_required_signals: 0,
            contract_matched_signals: 0,
            missing_signals: Vec::new(),
            runtime_model: String::new(),
            any_runtime_uncertainty: false,
            answer_preview: String::new(),
        })
    }
}
