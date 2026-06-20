mod state;
mod trace;

use crate::gemma_business::model_service_smoke::evidence::InspectEvidence;
use state::push_state_business_contract_failures as push_state_business_contract_failures_impl;
use trace::push_trace_business_contract_failures as push_trace_business_contract_failures_impl;

pub(super) fn push_state_business_contract_failures(
    inspect: &InspectEvidence,
    contract_case_count: u64,
    failures: &mut Vec<String>,
) {
    push_state_business_contract_failures_impl(inspect, contract_case_count, failures);
}

pub(super) fn push_trace_business_contract_failures(
    inspect: &InspectEvidence,
    contract_case_count: u64,
    failures: &mut Vec<String>,
) {
    push_trace_business_contract_failures_impl(inspect, contract_case_count, failures);
}
