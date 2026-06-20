use super::{BusinessContractEvidence, field};

pub(super) fn from_body(body: &str) -> BusinessContractEvidence {
    BusinessContractEvidence {
        items: field(body, "business_contract_items"),
        passed: field(body, "business_contract_passed"),
        failed: field(body, "business_contract_failed"),
        raw_passed: field(body, "business_contract_raw_passed"),
        raw_failed: field(body, "business_contract_raw_failed"),
        response_normalized: field(body, "business_contract_response_normalized"),
        sanitized: field(body, "business_contract_sanitized"),
        canonical_fallbacks: field(body, "business_contract_canonical_fallbacks"),
        ..BusinessContractEvidence::default()
    }
}
