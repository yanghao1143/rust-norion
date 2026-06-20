use rust_norion::StateInspectionGate;

use crate::gemma_business::regression::state_gate::minimums::require_min_usize;

pub(super) fn require_case_scaled_experience_state_gate(
    gate: &mut StateInspectionGate,
    case_count_usize: usize,
) {
    require_min_usize(&mut gate.min_experiences, case_count_usize);
    require_min_usize(&mut gate.min_runtime_model_experiences, case_count_usize);
    require_min_usize(&mut gate.min_runtime_tokens, case_count_usize);
    require_min_usize(
        &mut gate.min_runtime_architecture_experiences,
        case_count_usize,
    );
    require_min_usize(
        &mut gate.min_runtime_kv_precision_experiences,
        case_count_usize,
    );
    require_min_usize(
        &mut gate.min_business_contract_experiences,
        case_count_usize,
    );
    require_min_usize(&mut gate.min_business_contract_passed, case_count_usize);
    require_min_usize(&mut gate.min_rust_check_experiences, case_count_usize);
    require_min_usize(&mut gate.min_rust_check_passed, case_count_usize);
}
