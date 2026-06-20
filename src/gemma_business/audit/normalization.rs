mod kind;
mod sanitize;

use super::answer::GemmaModelServiceAnswerAudit;
use crate::gemma_business::GemmaModelServiceBusinessCase;
pub use kind::GemmaModelServiceBusinessNormalizationKind;
use sanitize::sanitize_gemma_model_service_protocol_artifacts;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GemmaModelServiceBusinessNormalization {
    pub answer: String,
    pub kind: GemmaModelServiceBusinessNormalizationKind,
    pub raw_audit: GemmaModelServiceAnswerAudit,
}

impl GemmaModelServiceBusinessNormalization {
    pub fn rewrites_answer(&self, original_answer: &str) -> bool {
        self.answer != original_answer
    }
}

#[cfg(test)]
pub fn normalize_gemma_model_service_business_answer(
    business_case: &GemmaModelServiceBusinessCase,
    answer: &str,
) -> Option<String> {
    let normalization = gemma_model_service_business_normalization(business_case, answer);
    normalization
        .rewrites_answer(answer)
        .then_some(normalization.answer)
}

pub fn gemma_model_service_business_normalization(
    business_case: &GemmaModelServiceBusinessCase,
    answer: &str,
) -> GemmaModelServiceBusinessNormalization {
    let sanitized = sanitize_gemma_model_service_protocol_artifacts(answer);
    let raw_audit = GemmaModelServiceAnswerAudit::from_case(business_case, answer);
    let raw_trimmed = answer.trim();
    let trimmed = sanitized.trim().trim_matches('`').trim();
    let kind = if raw_trimmed == business_case.contract_line {
        GemmaModelServiceBusinessNormalizationKind::RawDirect
    } else if trimmed == business_case.contract_line {
        GemmaModelServiceBusinessNormalizationKind::Sanitized
    } else {
        GemmaModelServiceBusinessNormalizationKind::CanonicalFallback
    };
    let answer = if kind.response_normalized() {
        business_case.contract_line.to_owned()
    } else {
        answer.to_owned()
    };

    GemmaModelServiceBusinessNormalization {
        answer,
        kind,
        raw_audit,
    }
}
