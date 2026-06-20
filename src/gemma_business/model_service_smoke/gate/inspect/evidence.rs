use crate::gemma_business::model_service_smoke::gate::ModelServiceSmokeGateInputs;

use super::super::checks::require_at_least_u64;

pub(super) fn push_inspect_evidence_failures(
    input: &ModelServiceSmokeGateInputs<'_>,
    contract_case_count: u64,
    failures: &mut Vec<String>,
) {
    input.inspect.runtime_audit.push_failures(failures);
    require_at_least_u64(
        input.inspect.runtime_tokens,
        input.case_run.total_runtime_token_count,
        "inspect state did not preserve generated runtime token evidence",
        failures,
    );
    require_at_least_u64(
        input.inspect.evolution_external_feedbacks,
        contract_case_count,
        "inspect state did not record external feedback evidence",
        failures,
    );
    require_at_least_u64(
        input.inspect.evolution_external_feedback_memory_updates,
        input.case_run.total_feedback_memory_ids,
        "inspect state did not record feedback memory update evidence",
        failures,
    );
    let expected_rust_checks = input.case_run.rust_check_expected_count as u64;
    require_at_least_u64(
        input.inspect.rust_check_passed,
        expected_rust_checks,
        "inspect state did not record rust-check pass evidence",
        failures,
    );
    require_at_least_u64(
        input.inspect.rust_check_experiences,
        expected_rust_checks,
        "inspect state did not record rust-check experience evidence",
        failures,
    );
}
