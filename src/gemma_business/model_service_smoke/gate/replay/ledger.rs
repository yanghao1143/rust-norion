mod business_contract;
mod rust_check;

use crate::gemma_business::model_service_smoke::case_flow::ModelServiceCaseRun;
use crate::gemma_business::model_service_smoke::evidence::{InspectEvidence, ReplayEvidence};
use business_contract::push_business_contract_ledger_failures;
use rust_check::push_rust_check_ledger_failures;

pub(super) fn push_replay_ledger_failures(
    case_run: &ModelServiceCaseRun,
    inspect: &InspectEvidence,
    replay: &ReplayEvidence,
    contract_case_count: u64,
    failures: &mut Vec<String>,
) {
    push_rust_check_ledger_failures(case_run, inspect, failures);
    push_business_contract_ledger_failures(inspect, replay, contract_case_count, failures);
}
