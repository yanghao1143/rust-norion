use crate::gemma_business::GemmaModelServiceBusinessCase;
use crate::gemma_business::audit::signals::business_answer_contains_signal;

pub(super) struct BusinessAnswerSignalCoverage {
    pub(super) required_signals: usize,
    pub(super) matched_signals: usize,
    pub(super) missing_signals: Vec<String>,
}

impl BusinessAnswerSignalCoverage {
    pub(super) fn from_case(
        business_case: &GemmaModelServiceBusinessCase,
        answer: &str,
        lower: &str,
    ) -> Self {
        let missing_signals = business_case
            .required_answer_signals
            .iter()
            .filter(|signal| !business_answer_contains_signal(answer, lower, signal))
            .map(|signal| (*signal).to_owned())
            .collect::<Vec<_>>();
        let required_signals = business_case.required_answer_signals.len();
        let matched_signals = required_signals.saturating_sub(missing_signals.len());
        Self {
            required_signals,
            matched_signals,
            missing_signals,
        }
    }
}
