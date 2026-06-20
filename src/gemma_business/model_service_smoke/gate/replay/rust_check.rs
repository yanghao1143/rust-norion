use crate::gemma_business::model_service_smoke::case_flow::ModelServiceCaseRun;
use crate::gemma_business::model_service_smoke::evidence::ReplayEvidence;

use super::super::checks::require_at_least_u64;

pub(super) fn push_replay_rust_check_failures(
    case_run: &ModelServiceCaseRun,
    replay: &ReplayEvidence,
    failures: &mut Vec<String>,
) {
    let expected = case_run.rust_check_expected_count as u64;
    require_at_least_u64(
        replay.rust_check.items,
        expected,
        "replay did not consume rust-check item evidence",
        failures,
    );
    require_at_least_u64(
        replay.rust_check.passed,
        expected,
        "replay did not consume rust-check pass evidence",
        failures,
    );
    require_at_least_u64(
        replay.rust_check.feedback_updates,
        expected,
        "replay did not consume rust-check memory feedback evidence",
        failures,
    );
    require_at_least_u64(
        replay.rust_check.feedback_applied,
        expected,
        "replay did not apply rust-check memory feedback evidence",
        failures,
    );
}
