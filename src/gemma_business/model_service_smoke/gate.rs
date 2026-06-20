mod business_contract;
mod checks;
mod inputs;
mod inspect;
mod replay;

use crate::gemma_business::GEMMA_MODEL_SERVICE_BUSINESS_CASES;
use business_contract::{
    push_state_business_contract_failures, push_trace_business_contract_failures,
};
use inspect::{push_endpoint_gate_failures, push_inspect_evidence_failures};
use replay::{
    push_replay_business_contract_failures, push_replay_ledger_failures,
    push_replay_rust_check_failures,
};

pub(super) use inputs::ModelServiceSmokeGateInputs;

pub(super) fn push_model_service_smoke_failures(
    input: ModelServiceSmokeGateInputs<'_>,
    failures: &mut Vec<String>,
) {
    let contract_case_count = GEMMA_MODEL_SERVICE_BUSINESS_CASES.len() as u64;
    push_endpoint_gate_failures(&input, failures);
    push_inspect_evidence_failures(&input, contract_case_count, failures);

    push_state_business_contract_failures(input.inspect, contract_case_count, failures);
    push_trace_business_contract_failures(input.inspect, contract_case_count, failures);
    push_replay_rust_check_failures(input.case_run, input.replay, failures);
    push_replay_business_contract_failures(input.replay, failures);
    push_replay_ledger_failures(
        input.case_run,
        input.inspect,
        input.replay,
        contract_case_count,
        failures,
    );
}
