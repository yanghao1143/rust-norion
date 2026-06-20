use rust_norion::StateInspectionGate;

use crate::gemma_business::state_gate::minimums::{
    require_min_f32, require_min_u64, require_min_usize,
};

pub(super) fn require_case_scaled_contract_state_gate(
    gate: &mut StateInspectionGate,
    case_count: usize,
) {
    // Business cases may consolidate into a single stronger memory; coverage is proven by experiences and feedback.
    require_min_usize(&mut gate.min_memories, 1);
    require_min_usize(&mut gate.min_experiences, case_count);
    require_min_usize(&mut gate.min_runtime_model_experiences, case_count);
    require_min_usize(&mut gate.min_runtime_tokens, case_count);
    require_min_usize(&mut gate.min_runtime_architecture_experiences, case_count);
    require_min_usize(&mut gate.min_runtime_kv_precision_experiences, case_count);
    require_min_u64(
        &mut gate.min_evolution_live_inference_runs,
        case_count as u64,
    );
    require_min_u64(
        &mut gate.min_evolution_external_feedbacks,
        case_count as u64,
    );
    require_min_u64(
        &mut gate.min_evolution_external_feedback_memory_updates,
        case_count as u64,
    );
    require_min_f32(
        &mut gate.min_evolution_external_feedback_strength_delta,
        0.01 * case_count as f32,
    );
    require_min_usize(&mut gate.min_business_contract_experiences, case_count);
    require_min_usize(&mut gate.min_business_contract_passed, case_count);
    require_min_u64(&mut gate.min_evolution_replay_runs, 1);
    require_min_u64(&mut gate.min_evolution_replay_items, case_count as u64);
    require_min_u64(
        &mut gate.min_evolution_replay_business_contract_items,
        case_count as u64,
    );
    require_min_u64(
        &mut gate.min_evolution_replay_business_contract_passed,
        case_count as u64,
    );
    require_min_u64(
        &mut gate.min_evolution_replay_business_contract_raw_audits,
        case_count as u64,
    );
    gate.max_business_contract_failed = Some(0);
    gate.max_business_contract_missing_signals = Some(0);
    gate.max_business_contract_protocol_leaks = Some(0);
    gate.max_business_contract_substitutions = Some(0);
    gate.max_business_contract_evasive_denials = Some(0);
    gate.max_business_contract_missing_handling_signals = Some(0);
    gate.max_evolution_replay_business_contract_failed = Some(0);
}
