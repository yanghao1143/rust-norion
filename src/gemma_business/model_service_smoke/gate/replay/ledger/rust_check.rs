use crate::gemma_business::model_service_smoke::case_flow::ModelServiceCaseRun;
use crate::gemma_business::model_service_smoke::evidence::InspectEvidence;

use super::super::super::checks::require_at_least_u64;

pub(super) fn push_rust_check_ledger_failures(
    case_run: &ModelServiceCaseRun,
    inspect: &InspectEvidence,
    failures: &mut Vec<String>,
) {
    let expected_rust_checks = case_run.rust_check_expected_count as u64;
    require_at_least_u64(
        inspect.evolution_replay_rust_check_items,
        expected_rust_checks,
        "inspect state did not ledger rust-check replay item evidence",
        failures,
    );
    require_at_least_u64(
        inspect.evolution_replay_rust_check_passed,
        expected_rust_checks,
        "inspect state did not ledger rust-check replay pass evidence",
        failures,
    );
}
