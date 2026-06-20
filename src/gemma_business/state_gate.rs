mod baseline;
mod minimums;
mod model_service;

use rust_norion::StateInspectionGate;

use crate::Args;

use baseline::{
    require_business_contract_state_gate, require_business_cycle_state_gate,
    require_gemma_business_smoke_state_gate,
};
use model_service::require_gemma_model_service_smoke_state_gate;

pub(crate) fn business_contract_state_gate(args: &Args) -> StateInspectionGate {
    let mut gate = args.state_inspection_gate();
    require_business_contract_state_gate(&mut gate);
    gate
}

pub(crate) fn business_cycle_state_gate(args: &Args) -> StateInspectionGate {
    let mut gate = business_contract_state_gate(args);
    require_business_cycle_state_gate(&mut gate);
    gate
}

fn gemma_business_smoke_state_gate_from_base(mut gate: StateInspectionGate) -> StateInspectionGate {
    require_business_contract_state_gate(&mut gate);
    require_gemma_business_smoke_state_gate(&mut gate);
    gate
}

pub(crate) fn gemma_business_smoke_state_gate(args: &Args) -> StateInspectionGate {
    gemma_business_smoke_state_gate_from_base(args.state_inspection_gate())
}

fn gemma_business_cycle_state_gate_from_base(gate: StateInspectionGate) -> StateInspectionGate {
    let mut gate = gemma_business_smoke_state_gate_from_base(gate);
    require_business_cycle_state_gate(&mut gate);
    gate
}

pub(crate) fn gemma_business_cycle_state_gate(args: &Args) -> StateInspectionGate {
    gemma_business_cycle_state_gate_from_base(args.state_inspection_gate())
}

pub(crate) fn gemma_model_service_smoke_state_gate(args: &Args) -> StateInspectionGate {
    let mut gate = gemma_business_smoke_state_gate(args);
    require_gemma_model_service_smoke_state_gate(&mut gate);
    gate
}
