mod experience;
mod feedback;
mod replay;

use rust_norion::StateInspectionGate;

use experience::require_case_scaled_experience_state_gate;
use feedback::require_case_scaled_external_feedback_state_gate;
use replay::require_case_scaled_replay_state_gate;

pub(super) fn require_case_scaled_state_gate(
    gate: &mut StateInspectionGate,
    case_count: u64,
    case_count_usize: usize,
) {
    require_case_scaled_experience_state_gate(gate, case_count_usize);
    require_case_scaled_external_feedback_state_gate(gate, case_count);
    require_case_scaled_replay_state_gate(gate, case_count);
}
