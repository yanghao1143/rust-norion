use rust_norion::StateInspectionGate;

use crate::gemma_business::regression::state_gate::minimums::require_min_usize;

pub(in crate::gemma_business::regression::state_gate) fn require_gemma_business_smoke_state_gate(
    gate: &mut StateInspectionGate,
) {
    require_min_usize(&mut gate.min_runtime_model_experiences, 1);
    require_min_usize(&mut gate.min_runtime_tokens, 1);
    require_min_usize(&mut gate.min_runtime_architecture_experiences, 1);
    require_min_usize(&mut gate.min_runtime_kv_precision_experiences, 1);
    gate.max_runtime_kv_precision_mismatches = Some(0);
    gate.max_runtime_errors = Some(0);
    gate.max_runtime_timeouts = Some(0);
}
