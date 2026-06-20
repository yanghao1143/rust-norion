use rust_norion::StateInspectionReport;

pub(super) fn print_gemma_business_smoke_state_summary(inspection: &StateInspectionReport) {
    println!(
        "gemma_business_smoke_state: memories={} experiences={} runtime_model_experiences={} runtime_tokens={} runtime_architecture_experiences={} runtime_kv_precision_experiences={} runtime_device_execution_experiences={} evolution_live_inference_runs={} evolution_replay_runs={} evolution_replay_items={} evolution_replay_business_contract_items={} evolution_replay_business_contract_passed={} evolution_replay_business_contract_failed={}",
        inspection.memory_count,
        inspection.experience_count,
        inspection.runtime_model_experience_count,
        inspection.runtime_token_count,
        inspection.runtime_architecture_experience_count,
        inspection.runtime_kv_precision_experience_count,
        inspection.runtime_device_execution_experience_count,
        inspection.evolution_ledger.live_inference_runs,
        inspection.evolution_ledger.replay_runs,
        inspection.evolution_ledger.replay_items,
        inspection.evolution_ledger.replay_business_contract_items,
        inspection.evolution_ledger.replay_business_contract_passed,
        inspection.evolution_ledger.replay_business_contract_failed
    );
}
