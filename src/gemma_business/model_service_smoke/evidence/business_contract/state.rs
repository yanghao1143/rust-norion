use super::{BusinessContractEvidence, field};

pub(super) fn from_body(body: &str) -> BusinessContractEvidence {
    BusinessContractEvidence {
        items: field(body, "business_contract_experiences"),
        passed: field(body, "business_contract_passed"),
        failed: field(body, "business_contract_failed"),
        missing_signals: field(body, "business_contract_missing_signals"),
        protocol_leaks: field(body, "business_contract_protocol_leaks"),
        substitutions: field(body, "business_contract_substitutions"),
        evasive_denials: field(body, "business_contract_evasive_denials"),
        raw_passed: field(body, "business_contract_raw_passed"),
        raw_failed: field(body, "business_contract_raw_failed"),
        response_normalized: field(body, "business_contract_response_normalized"),
        sanitized: field(body, "business_contract_sanitized"),
        canonical_fallbacks: field(body, "business_contract_canonical_fallbacks"),
    }
}
