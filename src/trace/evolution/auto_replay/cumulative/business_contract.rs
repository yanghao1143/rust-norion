use super::super::super::shared::require_usize_at_least;
use super::super::context::AutoReplayTrace;

pub(super) fn require_business_contract(failures: &mut Vec<String>, trace: &AutoReplayTrace) {
    let current = &trace.business_contract;
    let cumulative = &trace.cumulative.replay_business_contract;

    require_usize_at_least(
        failures,
        "cumulative_replay_business_contract_items",
        cumulative.items,
        "auto_replay business_contract_items",
        current.items,
    );
    require_usize_at_least(
        failures,
        "cumulative_replay_business_contract_passed",
        cumulative.passed,
        "auto_replay business_contract_passed",
        current.passed,
    );
    require_usize_at_least(
        failures,
        "cumulative_replay_business_contract_failed",
        cumulative.failed,
        "auto_replay business_contract_failed",
        current.failed,
    );
    require_usize_at_least(
        failures,
        "cumulative_replay_business_contract_raw_passed",
        cumulative.raw_passed,
        "auto_replay business_contract_raw_passed",
        current.raw_passed,
    );
    require_usize_at_least(
        failures,
        "cumulative_replay_business_contract_raw_failed",
        cumulative.raw_failed,
        "auto_replay business_contract_raw_failed",
        current.raw_failed,
    );
    require_usize_at_least(
        failures,
        "cumulative_replay_business_contract_response_normalized",
        cumulative.response_normalized,
        "auto_replay business_contract_response_normalized",
        current.response_normalized,
    );
    require_usize_at_least(
        failures,
        "cumulative_replay_business_contract_sanitized",
        cumulative.sanitized,
        "auto_replay business_contract_sanitized",
        current.sanitized,
    );
    require_usize_at_least(
        failures,
        "cumulative_replay_business_contract_canonical_fallbacks",
        cumulative.canonical_fallbacks,
        "auto_replay business_contract_canonical_fallbacks",
        current.canonical_fallbacks,
    );
}
