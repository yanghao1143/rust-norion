mod endpoint;
mod evidence;

use super::ModelServiceSmokeGateInputs;

pub(super) fn push_endpoint_gate_failures(
    input: &ModelServiceSmokeGateInputs<'_>,
    failures: &mut Vec<String>,
) {
    endpoint::push_endpoint_gate_failures(input, failures);
}

pub(super) fn push_inspect_evidence_failures(
    input: &ModelServiceSmokeGateInputs<'_>,
    contract_case_count: u64,
    failures: &mut Vec<String>,
) {
    evidence::push_inspect_evidence_failures(input, contract_case_count, failures);
}
