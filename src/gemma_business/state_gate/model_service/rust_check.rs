use rust_norion::StateInspectionGate;

use crate::gemma_business::GEMMA_MODEL_SERVICE_BUSINESS_CASES;
use crate::gemma_business::state_gate::minimums::{require_min_u64, require_min_usize};

pub(super) fn require_case_scaled_rust_check_state_gate(gate: &mut StateInspectionGate) {
    let rust_check_case_count = GEMMA_MODEL_SERVICE_BUSINESS_CASES
        .iter()
        .filter(|business_case| business_case.name == "gemma-service-rust-feedback")
        .count();
    require_min_usize(&mut gate.min_rust_check_experiences, rust_check_case_count);
    require_min_usize(&mut gate.min_rust_check_passed, rust_check_case_count);
    gate.max_rust_check_failed = Some(0);
    require_min_u64(
        &mut gate.min_evolution_replay_rust_check_items,
        rust_check_case_count as u64,
    );
    require_min_u64(
        &mut gate.min_evolution_replay_rust_check_passed,
        rust_check_case_count as u64,
    );
    gate.max_evolution_replay_rust_check_failed = Some(0);
    require_min_u64(
        &mut gate.min_evolution_replay_rust_check_live_memory_feedback_updates,
        rust_check_case_count as u64,
    );
    require_min_u64(
        &mut gate.min_evolution_replay_rust_check_live_memory_feedback_applied,
        rust_check_case_count as u64,
    );
}
