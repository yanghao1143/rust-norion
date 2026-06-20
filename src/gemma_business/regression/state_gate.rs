mod baseline;
mod minimums;
mod scaled;

use rust_norion::StateInspectionGate;

use baseline::{
    require_business_contract_state_gate, require_business_cycle_state_gate,
    require_gemma_business_smoke_state_gate,
};
use scaled::require_case_scaled_state_gate;

pub(super) fn business_cycle_report_state_gate_from_base(
    mut gate: StateInspectionGate,
    case_count: u64,
) -> StateInspectionGate {
    let case_count = case_count.max(1);
    let case_count_usize = case_count as usize;
    require_business_contract_state_gate(&mut gate);
    require_gemma_business_smoke_state_gate(&mut gate);
    require_business_cycle_state_gate(&mut gate);
    require_case_scaled_state_gate(&mut gate, case_count, case_count_usize);
    gate
}
