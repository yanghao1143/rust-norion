use rust_norion::StateInspectionGate;

use crate::gemma_business::state_gate::minimums::{require_min_u64, require_min_usize};

pub(in crate::gemma_business::state_gate) fn require_business_contract_state_gate(
    gate: &mut StateInspectionGate,
) {
    require_min_usize(&mut gate.min_memories, 1);
    require_min_usize(&mut gate.min_experiences, 1);
    require_min_usize(&mut gate.min_business_contract_experiences, 1);
    require_min_usize(&mut gate.min_business_contract_passed, 1);
    require_min_u64(&mut gate.min_evolution_live_inference_runs, 1);
    require_min_u64(&mut gate.min_evolution_replay_runs, 1);
    require_min_u64(&mut gate.min_evolution_replay_items, 1);
    require_min_u64(&mut gate.min_evolution_replay_business_contract_items, 1);
    require_min_u64(&mut gate.min_evolution_replay_business_contract_passed, 1);
    require_min_u64(
        &mut gate.min_evolution_replay_business_contract_raw_audits,
        1,
    );
    gate.max_business_contract_failed = Some(0);
    gate.max_business_contract_missing_signals = Some(0);
    gate.max_business_contract_protocol_leaks = Some(0);
    gate.max_business_contract_substitutions = Some(0);
    gate.max_business_contract_evasive_denials = Some(0);
    gate.max_business_contract_missing_handling_signals = Some(0);
    gate.max_evolution_replay_business_contract_failed = Some(0);
}
