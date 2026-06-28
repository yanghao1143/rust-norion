use rust_norion::ExperienceReplayReport;

use super::super::super::json::{
    service_json_string, service_json_string_array, service_memory_update_array,
};

pub(crate) fn model_service_replay_json(report: &ExperienceReplayReport) -> String {
    format!(
        "{{\"summary\":{},\"planned\":{},\"applied\":{},\"router_updates\":{},\"hierarchy_updates\":{},\"router_threshold_mutations\":{},\"hierarchy_weight_mutations\":{},\"router_threshold_delta\":{:.6},\"hierarchy_weight_delta\":{:.6},\"reinforced\":{},\"penalized\":{},\"touched_memories\":{},\"memory_reinforcements\":{},\"memory_penalties\":{},\"memory_update_reports\":{},\"average_reward\":{:.6},\"memory_updates\":{},\"removed_memory_updates\":{},\"missing_memory_updates\":{},\"memory_strength_delta\":{:.6},\"recursive_runtime_items\":{},\"recursive_runtime_calls\":{},\"average_recursive_call_pressure\":{:.6},\"max_recursive_call_pressure\":{:.6},\"runtime_kv_budget_pressure_items\":{},\"average_runtime_kv_budget_pressure\":{:.6},\"max_runtime_kv_budget_pressure\":{:.6},\"runtime_kv_weak_import_pressure_items\":{},\"average_runtime_kv_weak_import_pressure\":{:.6},\"max_runtime_kv_weak_import_pressure\":{:.6},\"external_semantic_context_items\":{},\"external_semantic_contexts\":{},\"live_memory_feedback_items\":{},\"live_memory_feedback_updates\":{},\"live_memory_feedback_reinforcements\":{},\"live_memory_feedback_penalties\":{},\"live_memory_feedback_detail_items\":{},\"live_memory_feedback_applied\":{},\"live_memory_feedback_removed\":{},\"live_memory_feedback_missing\":{},\"live_memory_feedback_strength_delta\":{:.6},\"rust_check_items\":{},\"rust_check_passed\":{},\"rust_check_failed\":{},\"rust_check_diagnostic_chars\":{},\"rust_check_live_memory_feedback_items\":{},\"rust_check_live_memory_feedback_updates\":{},\"rust_check_live_memory_feedback_applied\":{},\"rust_check_live_memory_feedback_missing\":{},\"rust_check_live_memory_feedback_strength_delta\":{:.6},\"business_contract_items\":{},\"business_contract_passed\":{},\"business_contract_failed\":{},\"business_contract_raw_passed\":{},\"business_contract_raw_failed\":{},\"business_contract_response_normalized\":{},\"business_contract_sanitized\":{},\"business_contract_canonical_fallbacks\":{},\"pool_dispatch_items\":{},\"pool_dispatch_forwarded\":{},\"pool_dispatch_clamped\":{},\"pool_dispatch_low_priority\":{},\"live_evolution_items\":{},\"live_evolution_router_threshold_mutations\":{},\"live_evolution_hierarchy_weight_mutations\":{},\"live_evolution_router_threshold_delta\":{:.6},\"live_evolution_hierarchy_weight_delta\":{:.6},\"live_evolution_online_reward_feedbacks\":{},\"live_evolution_online_reward_reinforcements\":{},\"live_evolution_online_reward_penalties\":{},\"live_evolution_online_reward_strength\":{:.6},\"live_evolution_online_reward_reinforcement_strength\":{:.6},\"live_evolution_online_reward_penalty_strength\":{:.6},\"live_evolution_memory_updates\":{},\"live_evolution_stored_memory_updates\":{},\"live_evolution_reflection_issues\":{},\"live_evolution_critical_reflection_issues\":{},\"live_evolution_revision_actions\":{},\"notes\":{}}}",
        service_json_string(&report.summary()),
        report.planned,
        report.applied,
        report.router_updates,
        report.hierarchy_updates,
        report.router_threshold_mutations,
        report.hierarchy_weight_mutations,
        report.router_threshold_delta,
        report.hierarchy_weight_delta,
        report.reinforced,
        report.penalized,
        report.touched_memories,
        report.memory_reinforcements,
        report.memory_penalties,
        service_memory_update_array(&report.memory_update_reports),
        report.average_reward,
        report.applied_memory_updates,
        report.removed_memory_updates,
        report.missing_memory_updates,
        report.memory_strength_delta,
        report.recursive_runtime_items,
        report.recursive_runtime_calls,
        report.average_recursive_call_pressure,
        report.max_recursive_call_pressure,
        report.runtime_kv_budget_pressure_items,
        report.average_runtime_kv_budget_pressure,
        report.max_runtime_kv_budget_pressure,
        report.runtime_kv_weak_import_pressure_items,
        report.average_runtime_kv_weak_import_pressure,
        report.max_runtime_kv_weak_import_pressure,
        report.external_semantic_context_items,
        report.external_semantic_contexts,
        report.live_memory_feedback_items,
        report.live_memory_feedback_updates,
        report.live_memory_feedback_reinforcements,
        report.live_memory_feedback_penalties,
        report.live_memory_feedback_detail_items,
        report.live_memory_feedback_applied,
        report.live_memory_feedback_removed,
        report.live_memory_feedback_missing,
        report.live_memory_feedback_strength_delta,
        report.rust_check_items,
        report.rust_check_passed,
        report.rust_check_failed,
        report.rust_check_diagnostic_chars,
        report.rust_check_live_memory_feedback_items,
        report.rust_check_live_memory_feedback_updates,
        report.rust_check_live_memory_feedback_applied,
        report.rust_check_live_memory_feedback_missing,
        report.rust_check_live_memory_feedback_strength_delta,
        report.business_contract_items,
        report.business_contract_passed,
        report.business_contract_failed,
        report.business_contract_raw_passed,
        report.business_contract_raw_failed,
        report.business_contract_response_normalized,
        report.business_contract_sanitized,
        report.business_contract_canonical_fallbacks,
        report.pool_dispatch_items,
        report.pool_dispatch_forwarded,
        report.pool_dispatch_clamped,
        report.pool_dispatch_low_priority,
        report.live_evolution_items,
        report.live_evolution_router_threshold_mutations,
        report.live_evolution_hierarchy_weight_mutations,
        report.live_evolution_router_threshold_delta,
        report.live_evolution_hierarchy_weight_delta,
        report.live_evolution_online_reward_feedbacks,
        report.live_evolution_online_reward_reinforcements,
        report.live_evolution_online_reward_penalties,
        report.live_evolution_online_reward_strength,
        report.live_evolution_online_reward_reinforcement_strength,
        report.live_evolution_online_reward_penalty_strength,
        report.live_evolution_memory_updates,
        report.live_evolution_stored_memory_updates,
        report.live_evolution_reflection_issues,
        report.live_evolution_critical_reflection_issues,
        report.live_evolution_revision_actions,
        service_json_string_array(&report.notes)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_norion::{MemoryUpdateAction, MemoryUpdateReport};

    #[test]
    fn replay_json_renders_core_and_business_evidence() {
        let report = ExperienceReplayReport {
            planned: 3,
            applied: 2,
            router_updates: 1,
            hierarchy_updates: 2,
            router_threshold_mutations: 3,
            hierarchy_weight_mutations: 4,
            router_threshold_delta: 0.12,
            hierarchy_weight_delta: 0.34,
            reinforced: 5,
            penalized: 6,
            touched_memories: 7,
            memory_reinforcements: 8,
            memory_penalties: 9,
            memory_update_reports: vec![MemoryUpdateReport::applied(
                7,
                MemoryUpdateAction::Reinforce,
                0.25,
                0.50,
                0.75,
                false,
            )],
            average_reward: 0.91,
            applied_memory_updates: 4,
            removed_memory_updates: 1,
            missing_memory_updates: 2,
            memory_strength_delta: 0.56,
            recursive_runtime_items: 2,
            recursive_runtime_calls: 5,
            average_recursive_call_pressure: 0.22,
            max_recursive_call_pressure: 0.44,
            runtime_kv_budget_pressure_items: 3,
            average_runtime_kv_budget_pressure: 0.11,
            max_runtime_kv_budget_pressure: 0.33,
            runtime_kv_weak_import_pressure_items: 4,
            average_runtime_kv_weak_import_pressure: 0.15,
            max_runtime_kv_weak_import_pressure: 0.45,
            external_semantic_context_items: 1,
            external_semantic_contexts: 4,
            live_memory_feedback_items: 5,
            live_memory_feedback_updates: 6,
            live_memory_feedback_reinforcements: 7,
            live_memory_feedback_penalties: 8,
            live_memory_feedback_detail_items: 9,
            live_memory_feedback_applied: 10,
            live_memory_feedback_removed: 11,
            live_memory_feedback_missing: 12,
            live_memory_feedback_strength_delta: 0.78,
            business_contract_items: 6,
            business_contract_response_normalized: 7,
            pool_dispatch_items: 8,
            pool_dispatch_forwarded: 9,
            pool_dispatch_clamped: 10,
            pool_dispatch_low_priority: 11,
            live_evolution_items: 12,
            live_evolution_router_threshold_mutations: 13,
            live_evolution_hierarchy_weight_mutations: 14,
            live_evolution_router_threshold_delta: 0.21,
            live_evolution_hierarchy_weight_delta: 0.43,
            live_evolution_online_reward_feedbacks: 15,
            live_evolution_online_reward_reinforcements: 16,
            live_evolution_online_reward_penalties: 17,
            live_evolution_online_reward_strength: 0.64,
            live_evolution_online_reward_reinforcement_strength: 0.54,
            live_evolution_online_reward_penalty_strength: 0.10,
            live_evolution_memory_updates: 8,
            live_evolution_stored_memory_updates: 9,
            live_evolution_reflection_issues: 10,
            live_evolution_critical_reflection_issues: 1,
            live_evolution_revision_actions: 11,
            notes: vec!["experience:7:reinforce reward=0.910".to_owned()],
            ..ExperienceReplayReport::default()
        };

        let json = model_service_replay_json(&report);

        assert!(json.contains("\"planned\":3"));
        assert!(json.contains("\"applied\":2"));
        assert!(json.contains("\"router_updates\":1"));
        assert!(json.contains("\"hierarchy_updates\":2"));
        assert!(json.contains("\"router_threshold_mutations\":3"));
        assert!(json.contains("\"hierarchy_weight_mutations\":4"));
        assert!(json.contains("\"router_threshold_delta\":0.120000"));
        assert!(json.contains("\"hierarchy_weight_delta\":0.340000"));
        assert!(json.contains("\"reinforced\":5"));
        assert!(json.contains("\"penalized\":6"));
        assert!(json.contains("\"touched_memories\":7"));
        assert!(json.contains("\"memory_reinforcements\":8"));
        assert!(json.contains("\"memory_penalties\":9"));
        assert!(json.contains("\"memory_update_reports\":[{"));
        assert!(json.contains("\"id\":7"));
        assert!(json.contains("\"action\":\"reinforce\""));
        assert!(json.contains("\"requested_amount\":0.250000"));
        assert!(json.contains("\"strength_before\":0.500000"));
        assert!(json.contains("\"strength_after\":0.750000"));
        assert!(json.contains("\"strength_delta\":0.250000"));
        assert!(json.contains("\"average_reward\":0.910000"));
        assert!(json.contains("\"memory_updates\":4"));
        assert!(json.contains("\"removed_memory_updates\":1"));
        assert!(json.contains("\"missing_memory_updates\":2"));
        assert!(json.contains("\"memory_strength_delta\":0.560000"));
        assert!(json.contains("\"recursive_runtime_items\":2"));
        assert!(json.contains("\"recursive_runtime_calls\":5"));
        assert!(json.contains("\"average_recursive_call_pressure\":0.220000"));
        assert!(json.contains("\"max_recursive_call_pressure\":0.440000"));
        assert!(json.contains("\"runtime_kv_budget_pressure_items\":3"));
        assert!(json.contains("\"average_runtime_kv_budget_pressure\":0.110000"));
        assert!(json.contains("\"max_runtime_kv_budget_pressure\":0.330000"));
        assert!(json.contains("\"runtime_kv_weak_import_pressure_items\":4"));
        assert!(json.contains("\"average_runtime_kv_weak_import_pressure\":0.150000"));
        assert!(json.contains("\"max_runtime_kv_weak_import_pressure\":0.450000"));
        assert!(json.contains("\"external_semantic_context_items\":1"));
        assert!(json.contains("\"external_semantic_contexts\":4"));
        assert!(json.contains("\"live_memory_feedback_reinforcements\":7"));
        assert!(json.contains("\"live_memory_feedback_penalties\":8"));
        assert!(json.contains("\"live_memory_feedback_detail_items\":9"));
        assert!(json.contains("\"live_memory_feedback_removed\":11"));
        assert!(json.contains("\"live_memory_feedback_strength_delta\":0.780000"));
        assert!(json.contains("\"business_contract_items\":6"));
        assert!(json.contains("\"business_contract_response_normalized\":7"));
        assert!(json.contains("\"pool_dispatch_items\":8"));
        assert!(json.contains("\"pool_dispatch_forwarded\":9"));
        assert!(json.contains("\"pool_dispatch_clamped\":10"));
        assert!(json.contains("\"pool_dispatch_low_priority\":11"));
        assert!(json.contains("\"live_evolution_router_threshold_mutations\":13"));
        assert!(json.contains("\"live_evolution_hierarchy_weight_mutations\":14"));
        assert!(json.contains("\"live_evolution_router_threshold_delta\":0.210000"));
        assert!(json.contains("\"live_evolution_hierarchy_weight_delta\":0.430000"));
        assert!(json.contains("\"live_evolution_online_reward_feedbacks\":15"));
        assert!(json.contains("\"live_evolution_online_reward_reinforcements\":16"));
        assert!(json.contains("\"live_evolution_online_reward_penalties\":17"));
        assert!(json.contains("\"live_evolution_online_reward_strength\":0.640000"));
        assert!(json.contains("\"live_evolution_online_reward_reinforcement_strength\":0.540000"));
        assert!(json.contains("\"live_evolution_online_reward_penalty_strength\":0.100000"));
        assert!(json.contains("\"live_evolution_memory_updates\":8"));
        assert!(json.contains("\"live_evolution_stored_memory_updates\":9"));
        assert!(json.contains("\"live_evolution_reflection_issues\":10"));
        assert!(json.contains("\"live_evolution_critical_reflection_issues\":1"));
        assert!(json.contains("\"live_evolution_revision_actions\":11"));
        assert!(json.contains("\"notes\":[\"experience:7:reinforce reward=0.910\"]"));
    }
}
