use rust_norion::StateInspectionReport;

use crate::gemma_business::response_json::response_u64_field;

use super::minimums::{require_state_min_u64, require_state_min_usize};

pub(super) fn require_state_report_minimums(
    inspection: &StateInspectionReport,
    report_body: &str,
    expected_case_count: u64,
    failures: &mut Vec<String>,
) {
    require_state_min_usize(
        failures,
        "runtime_model_experience_count",
        inspection.runtime_model_experience_count,
        expected_case_count,
    );
    require_state_min_usize(
        failures,
        "runtime_token_count",
        inspection.runtime_token_count,
        response_u64_field(report_body, "runtime_token_count"),
    );
    require_state_min_usize(
        failures,
        "rust_check_passed_count",
        inspection.rust_check_passed_count,
        expected_case_count,
    );
    require_state_min_usize(
        failures,
        "business_contract_experience_count",
        inspection.business_contract_experience_count,
        expected_case_count,
    );
    require_state_min_usize(
        failures,
        "business_contract_passed_count",
        inspection.business_contract_passed_count,
        expected_case_count,
    );
    require_state_min_u64(
        failures,
        "external_feedbacks",
        inspection.evolution_ledger.external_feedbacks,
        response_u64_field(report_body, "external_feedbacks"),
    );
    require_state_min_u64(
        failures,
        "feedback_memory_updates",
        inspection.evolution_ledger.external_feedback_memory_updates,
        response_u64_field(report_body, "feedback_memory_updates"),
    );
    require_state_min_u64(
        failures,
        "replay_rust_check_passed",
        inspection.evolution_ledger.replay_rust_check_passed,
        response_u64_field(report_body, "replay_rust_check_passed"),
    );
    require_state_min_u64(
        failures,
        "replay_business_contract_passed",
        inspection.evolution_ledger.replay_business_contract_passed,
        expected_case_count,
    );
}
