use rust_norion::StateInspectionGate;

use crate::gemma_business::regression::state_gate::minimums::{require_min_f32, require_min_u64};

pub(super) fn require_case_scaled_external_feedback_state_gate(
    gate: &mut StateInspectionGate,
    case_count: u64,
) {
    require_min_u64(&mut gate.min_evolution_live_inference_runs, case_count);
    require_min_u64(
        &mut gate.min_evolution_external_feedbacks,
        case_count.saturating_mul(2),
    );
    require_min_u64(
        &mut gate.min_evolution_external_feedback_memory_updates,
        case_count.saturating_mul(2),
    );
    require_min_f32(
        &mut gate.min_evolution_external_feedback_strength_delta,
        0.01 * case_count as f32,
    );
}
