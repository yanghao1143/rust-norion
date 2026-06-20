use rust_norion::StateInspectionGate;

use crate::gemma_business::regression::state_gate::minimums::{require_min_f32, require_min_u64};

pub(super) fn require_case_scaled_replay_state_gate(
    gate: &mut StateInspectionGate,
    case_count: u64,
) {
    require_min_u64(&mut gate.min_evolution_replay_runs, case_count);
    require_min_u64(&mut gate.min_evolution_replay_items, case_count);
    require_min_u64(&mut gate.min_evolution_replay_rust_check_items, case_count);
    require_min_u64(&mut gate.min_evolution_replay_rust_check_passed, case_count);
    require_min_u64(
        &mut gate.min_evolution_replay_business_contract_items,
        case_count,
    );
    require_min_u64(
        &mut gate.min_evolution_replay_business_contract_passed,
        case_count,
    );
    require_min_u64(
        &mut gate.min_evolution_replay_live_memory_feedback_updates,
        case_count,
    );
    require_min_u64(
        &mut gate.min_evolution_replay_live_memory_feedback_applied,
        case_count,
    );
    require_min_f32(
        &mut gate.min_evolution_replay_live_memory_feedback_strength_delta,
        0.01 * case_count as f32,
    );
    require_min_u64(
        &mut gate.min_evolution_replay_live_evolution_items,
        case_count,
    );
    require_min_u64(
        &mut gate.min_evolution_replay_live_evolution_online_reward_feedbacks,
        case_count,
    );
    require_min_u64(
        &mut gate.min_evolution_replay_live_evolution_online_reward_reinforcements,
        case_count,
    );
}
