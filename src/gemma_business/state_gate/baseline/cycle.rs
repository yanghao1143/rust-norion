use rust_norion::StateInspectionGate;

use crate::gemma_business::state_gate::minimums::{
    require_min_f32, require_min_u64, require_min_usize,
};

pub(in crate::gemma_business::state_gate) fn require_business_cycle_state_gate(
    gate: &mut StateInspectionGate,
) {
    require_min_u64(&mut gate.min_evolution_external_feedbacks, 2);
    require_min_u64(&mut gate.min_evolution_external_feedback_memory_updates, 2);
    require_min_f32(
        &mut gate.min_evolution_external_feedback_strength_delta,
        0.01,
    );
    require_min_usize(&mut gate.min_rust_check_experiences, 1);
    require_min_usize(&mut gate.min_rust_check_passed, 1);
    gate.max_rust_check_failed = Some(0);
    require_min_u64(&mut gate.min_evolution_replay_rust_check_items, 1);
    require_min_u64(&mut gate.min_evolution_replay_rust_check_passed, 1);
    gate.max_evolution_replay_rust_check_failed = Some(0);
    require_min_u64(
        &mut gate.min_evolution_replay_live_memory_feedback_updates,
        1,
    );
    require_min_u64(
        &mut gate.min_evolution_replay_live_memory_feedback_applied,
        1,
    );
    require_min_f32(
        &mut gate.min_evolution_replay_live_memory_feedback_strength_delta,
        0.01,
    );
    require_min_u64(&mut gate.min_evolution_replay_live_evolution_items, 1);
    require_min_u64(
        &mut gate.min_evolution_replay_live_evolution_online_reward_feedbacks,
        1,
    );
    require_min_u64(
        &mut gate.min_evolution_replay_live_evolution_online_reward_reinforcements,
        1,
    );
}
