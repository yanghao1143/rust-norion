use crate::gemma_business::model_service_smoke::evidence::InspectEvidence;

use super::super::checks::{
    require_at_least_u64, require_business_contract_normalization_match, require_zero_u64,
};

pub(super) fn push_state_business_contract_failures(
    inspect: &InspectEvidence,
    contract_case_count: u64,
    failures: &mut Vec<String>,
) {
    let state = inspect.business_contract_state;
    require_at_least_u64(
        state.items,
        contract_case_count,
        "inspect state did not record every business contract audit",
        failures,
    );
    require_at_least_u64(
        state.passed,
        contract_case_count,
        "inspect state did not record every business contract pass",
        failures,
    );
    require_zero_u64(
        state.failed,
        "inspect state recorded business_contract_failed",
        failures,
    );
    require_zero_u64(
        state.missing_signals,
        "inspect state recorded business_contract_missing_signals",
        failures,
    );
    require_zero_u64(
        state.protocol_leaks,
        "inspect state recorded business_contract_protocol_leaks",
        failures,
    );
    require_zero_u64(
        state.substitutions,
        "inspect state recorded business_contract_substitutions",
        failures,
    );
    require_zero_u64(
        state.evasive_denials,
        "inspect state recorded business_contract_evasive_denials",
        failures,
    );
    require_at_least_u64(
        state.raw_total(),
        contract_case_count,
        "inspect state did not record every raw business contract audit",
        failures,
    );
    require_business_contract_normalization_match(
        state,
        "inspect state business contract normalization counters disagree",
        failures,
    );
}
