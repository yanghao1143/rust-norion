use crate::gemma_business::model_service_smoke::evidence::ReplayEvidence;

use super::super::checks::{
    require_at_least_u64, require_business_contract_normalization_match, require_zero_u64,
};

pub(super) fn push_replay_business_contract_failures(
    replay: &ReplayEvidence,
    failures: &mut Vec<String>,
) {
    let contract = replay.business_contract;
    if contract.items == 0 {
        failures.push("replay did not consume business contract item evidence".to_owned());
    }
    require_at_least_u64(
        contract.passed,
        contract.items,
        "replay did not preserve consumed business contract passes",
        failures,
    );
    require_zero_u64(
        contract.failed,
        "replay recorded business_contract_failed",
        failures,
    );
    require_at_least_u64(
        contract.raw_total(),
        contract.items,
        "replay did not preserve consumed raw business contract audits",
        failures,
    );
    require_business_contract_normalization_match(
        contract,
        "replay business contract normalization counters disagree",
        failures,
    );
}
