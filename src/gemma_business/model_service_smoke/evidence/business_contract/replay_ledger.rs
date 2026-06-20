use super::{BusinessContractEvidence, field};

pub(super) fn from_body(body: &str) -> BusinessContractEvidence {
    BusinessContractEvidence {
        items: field(body, "evolution_replay_business_contract_items"),
        passed: field(body, "evolution_replay_business_contract_passed"),
        failed: field(body, "evolution_replay_business_contract_failed"),
        raw_passed: field(body, "evolution_replay_business_contract_raw_passed"),
        raw_failed: field(body, "evolution_replay_business_contract_raw_failed"),
        response_normalized: field(
            body,
            "evolution_replay_business_contract_response_normalized",
        ),
        sanitized: field(body, "evolution_replay_business_contract_sanitized"),
        canonical_fallbacks: field(
            body,
            "evolution_replay_business_contract_canonical_fallbacks",
        ),
        ..BusinessContractEvidence::default()
    }
}
