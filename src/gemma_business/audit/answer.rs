mod failure;
mod flags;
mod signals;

use crate::gemma_business::GemmaModelServiceBusinessCase;
use failure::business_answer_failure;
use flags::BusinessAnswerFlags;
use signals::BusinessAnswerSignalCoverage;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GemmaModelServiceAnswerAudit {
    pub required_signals: usize,
    pub matched_signals: usize,
    pub missing_signals: Vec<String>,
    pub has_runtime_model_experiences: bool,
    pub protocol_leak: bool,
    pub substituted_runtime_model_experiences: bool,
    pub evasive_denial: bool,
    pub handling_signal: bool,
}

impl GemmaModelServiceAnswerAudit {
    pub fn from_case(business_case: &GemmaModelServiceBusinessCase, answer: &str) -> Self {
        let lower = answer.to_ascii_lowercase();
        let signal_coverage =
            BusinessAnswerSignalCoverage::from_case(business_case, answer, &lower);
        let flags = BusinessAnswerFlags::from_answer(answer, &lower);

        Self {
            required_signals: signal_coverage.required_signals,
            matched_signals: signal_coverage.matched_signals,
            missing_signals: signal_coverage.missing_signals,
            has_runtime_model_experiences: flags.has_runtime_model_experiences,
            protocol_leak: flags.protocol_leak,
            substituted_runtime_model_experiences: flags.substituted_runtime_model_experiences,
            evasive_denial: flags.evasive_denial,
            handling_signal: flags.handling_signal,
        }
    }

    pub fn passed(&self) -> bool {
        self.failure().is_none()
    }

    pub fn failure(&self) -> Option<String> {
        business_answer_failure(
            self.has_runtime_model_experiences,
            self.protocol_leak,
            self.substituted_runtime_model_experiences,
            self.evasive_denial,
            self.handling_signal,
            self.missing_signals.first().map(String::as_str),
        )
    }
}

pub fn gemma_business_smoke_answer_failure(answer: &str) -> Option<String> {
    let lower = answer.to_ascii_lowercase();
    let flags = BusinessAnswerFlags::from_answer(answer, &lower);
    business_answer_failure(
        flags.has_runtime_model_experiences,
        flags.protocol_leak,
        flags.substituted_runtime_model_experiences,
        flags.evasive_denial,
        flags.handling_signal,
        None,
    )
}

#[cfg(test)]
pub fn gemma_model_service_answer_failure(
    business_case: &GemmaModelServiceBusinessCase,
    answer: &str,
) -> Option<String> {
    GemmaModelServiceAnswerAudit::from_case(business_case, answer).failure()
}
