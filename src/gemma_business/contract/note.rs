use crate::gemma_business::audit::{
    GemmaModelServiceAnswerAudit, GemmaModelServiceBusinessNormalization,
};

pub(super) fn model_service_business_contract_note(
    case_name: &str,
    audit: &GemmaModelServiceAnswerAudit,
    normalization: &GemmaModelServiceBusinessNormalization,
) -> String {
    format!(
        "business_contract:case={}:passed={}:required={}:matched={}:missing={}:has_runtime_model_experiences={}:protocol_leak={}:substituted_runtime_model_experiences={}:evasive_denial={}:handling_signal={}:raw_passed={}:normalization={}:response_normalized={}:canonical_fallback={}",
        compact_note_value(case_name, 64),
        audit.passed(),
        audit.required_signals,
        audit.matched_signals,
        audit.missing_signals.len(),
        audit.has_runtime_model_experiences,
        audit.protocol_leak,
        audit.substituted_runtime_model_experiences,
        audit.evasive_denial,
        audit.handling_signal,
        normalization.raw_audit.passed(),
        normalization.kind.as_str(),
        normalization.kind.response_normalized(),
        normalization.kind.canonical_fallback()
    )
}

fn compact_note_value(value: &str, max_chars: usize) -> String {
    value
        .chars()
        .take(max_chars)
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}
