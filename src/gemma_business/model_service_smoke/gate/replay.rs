mod business_contract;
mod ledger;
mod rust_check;

use crate::gemma_business::model_service_smoke::case_flow::ModelServiceCaseRun;
use crate::gemma_business::model_service_smoke::evidence::{InspectEvidence, ReplayEvidence};

pub(super) fn push_replay_rust_check_failures(
    case_run: &ModelServiceCaseRun,
    replay: &ReplayEvidence,
    failures: &mut Vec<String>,
) {
    rust_check::push_replay_rust_check_failures(case_run, replay, failures);
}

pub(super) fn push_replay_business_contract_failures(
    replay: &ReplayEvidence,
    failures: &mut Vec<String>,
) {
    business_contract::push_replay_business_contract_failures(replay, failures);
}

pub(super) fn push_replay_ledger_failures(
    case_run: &ModelServiceCaseRun,
    inspect: &InspectEvidence,
    replay: &ReplayEvidence,
    contract_case_count: u64,
    failures: &mut Vec<String>,
) {
    ledger::push_replay_ledger_failures(case_run, inspect, replay, contract_case_count, failures);
}
