mod contract;
mod rust_check;

use rust_norion::StateInspectionGate;

use crate::gemma_business::GEMMA_MODEL_SERVICE_BUSINESS_CASES;
use contract::require_case_scaled_contract_state_gate;
use rust_check::require_case_scaled_rust_check_state_gate;

pub(super) fn require_gemma_model_service_smoke_state_gate(gate: &mut StateInspectionGate) {
    let case_count = GEMMA_MODEL_SERVICE_BUSINESS_CASES.len();
    require_case_scaled_contract_state_gate(gate, case_count);
    require_case_scaled_rust_check_state_gate(gate);
}
