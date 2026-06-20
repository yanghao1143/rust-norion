use super::{BusinessContractEvidence, field};

pub(super) fn from_body(body: &str) -> BusinessContractEvidence {
    BusinessContractEvidence {
        items: field(body, "business_contract_events"),
        passed: field(body, "business_contract_event_passed"),
        failed: field(body, "business_contract_event_failed"),
        missing_signals: field(body, "business_contract_event_missing_signals"),
        raw_passed: field(body, "business_contract_event_raw_passed"),
        raw_failed: field(body, "business_contract_event_raw_failed"),
        response_normalized: field(body, "business_contract_event_response_normalized"),
        sanitized: field(body, "business_contract_event_sanitized"),
        canonical_fallbacks: field(body, "business_contract_event_canonical_fallbacks"),
        ..BusinessContractEvidence::default()
    }
}
