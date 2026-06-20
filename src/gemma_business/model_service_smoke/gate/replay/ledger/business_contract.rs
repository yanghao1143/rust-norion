use crate::gemma_business::model_service_smoke::evidence::{InspectEvidence, ReplayEvidence};

use super::super::super::checks::{
    require_at_least_u64, require_business_contract_normalization_match, require_zero_u64,
};

pub(super) fn push_business_contract_ledger_failures(
    inspect: &InspectEvidence,
    replay: &ReplayEvidence,
    contract_case_count: u64,
    failures: &mut Vec<String>,
) {
    let ledger = inspect.business_contract_replay_ledger;
    require_at_least_u64(
        ledger.items,
        contract_case_count,
        "inspect state did not ledger business contract replay item evidence",
        failures,
    );
    require_at_least_u64(
        ledger.passed,
        contract_case_count,
        "inspect state did not ledger business contract replay pass evidence",
        failures,
    );
    require_zero_u64(
        ledger.failed,
        "inspect state ledger recorded evolution_replay_business_contract_failed",
        failures,
    );
    require_at_least_u64(
        ledger.raw_total(),
        contract_case_count,
        "inspect state did not ledger every raw business contract replay audit",
        failures,
    );
    require_business_contract_normalization_match(
        ledger,
        "inspect state ledger business contract replay normalization counters disagree",
        failures,
    );
    require_at_least_u64(
        ledger.raw_failed,
        replay.business_contract.raw_failed,
        "inspect state ledger lost raw-failed business replay evidence",
        failures,
    );
    require_at_least_u64(
        ledger.canonical_fallbacks,
        replay.business_contract.canonical_fallbacks,
        "inspect state ledger lost canonical-fallback business replay evidence",
        failures,
    );
}
