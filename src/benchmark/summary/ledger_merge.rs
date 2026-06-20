use crate::adaptive_state::EvolutionLedger;

pub(super) fn max_evolution_ledger(
    left: EvolutionLedger,
    right: EvolutionLedger,
) -> EvolutionLedger {
    EvolutionLedger {
        live_inference_runs: left.live_inference_runs.max(right.live_inference_runs),
        live_router_threshold_mutations: left
            .live_router_threshold_mutations
            .max(right.live_router_threshold_mutations),
        live_hierarchy_weight_mutations: left
            .live_hierarchy_weight_mutations
            .max(right.live_hierarchy_weight_mutations),
        live_router_threshold_delta: left
            .live_router_threshold_delta
            .max(right.live_router_threshold_delta),
        live_hierarchy_weight_delta: left
            .live_hierarchy_weight_delta
            .max(right.live_hierarchy_weight_delta),
        live_online_reward_feedbacks: left
            .live_online_reward_feedbacks
            .max(right.live_online_reward_feedbacks),
        live_online_reward_reinforcements: left
            .live_online_reward_reinforcements
            .max(right.live_online_reward_reinforcements),
        live_online_reward_penalties: left
            .live_online_reward_penalties
            .max(right.live_online_reward_penalties),
        live_online_reward_strength: left
            .live_online_reward_strength
            .max(right.live_online_reward_strength),
        live_online_reward_reinforcement_strength: left
            .live_online_reward_reinforcement_strength
            .max(right.live_online_reward_reinforcement_strength),
        live_online_reward_penalty_strength: left
            .live_online_reward_penalty_strength
            .max(right.live_online_reward_penalty_strength),
        live_memory_reinforcements: left
            .live_memory_reinforcements
            .max(right.live_memory_reinforcements),
        live_memory_penalties: left.live_memory_penalties.max(right.live_memory_penalties),
        live_stored_memories: left.live_stored_memories.max(right.live_stored_memories),
        live_stored_gist_memories: left
            .live_stored_gist_memories
            .max(right.live_stored_gist_memories),
        live_stored_runtime_kv_memories: left
            .live_stored_runtime_kv_memories
            .max(right.live_stored_runtime_kv_memories),
        live_reflection_issues: left
            .live_reflection_issues
            .max(right.live_reflection_issues),
        live_critical_reflection_issues: left
            .live_critical_reflection_issues
            .max(right.live_critical_reflection_issues),
        live_revision_actions: left.live_revision_actions.max(right.live_revision_actions),
        replay_runs: left.replay_runs.max(right.replay_runs),
        replay_items: left.replay_items.max(right.replay_items),
        router_threshold_mutations: left
            .router_threshold_mutations
            .max(right.router_threshold_mutations),
        hierarchy_weight_mutations: left
            .hierarchy_weight_mutations
            .max(right.hierarchy_weight_mutations),
        router_threshold_delta: left
            .router_threshold_delta
            .max(right.router_threshold_delta),
        hierarchy_weight_delta: left
            .hierarchy_weight_delta
            .max(right.hierarchy_weight_delta),
        memory_reinforcements: left.memory_reinforcements.max(right.memory_reinforcements),
        memory_penalties: left.memory_penalties.max(right.memory_penalties),
        replay_live_memory_feedback_items: left
            .replay_live_memory_feedback_items
            .max(right.replay_live_memory_feedback_items),
        replay_live_memory_feedback_reinforcements: left
            .replay_live_memory_feedback_reinforcements
            .max(right.replay_live_memory_feedback_reinforcements),
        replay_live_memory_feedback_penalties: left
            .replay_live_memory_feedback_penalties
            .max(right.replay_live_memory_feedback_penalties),
        replay_live_memory_feedback_detail_items: left
            .replay_live_memory_feedback_detail_items
            .max(right.replay_live_memory_feedback_detail_items),
        replay_live_memory_feedback_applied: left
            .replay_live_memory_feedback_applied
            .max(right.replay_live_memory_feedback_applied),
        replay_live_memory_feedback_removed: left
            .replay_live_memory_feedback_removed
            .max(right.replay_live_memory_feedback_removed),
        replay_live_memory_feedback_missing: left
            .replay_live_memory_feedback_missing
            .max(right.replay_live_memory_feedback_missing),
        replay_live_memory_feedback_strength_delta: left
            .replay_live_memory_feedback_strength_delta
            .max(right.replay_live_memory_feedback_strength_delta),
        replay_rust_check_items: left
            .replay_rust_check_items
            .max(right.replay_rust_check_items),
        replay_rust_check_passed: left
            .replay_rust_check_passed
            .max(right.replay_rust_check_passed),
        replay_rust_check_failed: left
            .replay_rust_check_failed
            .max(right.replay_rust_check_failed),
        replay_rust_check_diagnostic_chars: left
            .replay_rust_check_diagnostic_chars
            .max(right.replay_rust_check_diagnostic_chars),
        replay_rust_check_live_memory_feedback_items: left
            .replay_rust_check_live_memory_feedback_items
            .max(right.replay_rust_check_live_memory_feedback_items),
        replay_rust_check_live_memory_feedback_updates: left
            .replay_rust_check_live_memory_feedback_updates
            .max(right.replay_rust_check_live_memory_feedback_updates),
        replay_rust_check_live_memory_feedback_applied: left
            .replay_rust_check_live_memory_feedback_applied
            .max(right.replay_rust_check_live_memory_feedback_applied),
        replay_rust_check_live_memory_feedback_strength_delta: left
            .replay_rust_check_live_memory_feedback_strength_delta
            .max(right.replay_rust_check_live_memory_feedback_strength_delta),
        replay_business_contract_items: left
            .replay_business_contract_items
            .max(right.replay_business_contract_items),
        replay_business_contract_passed: left
            .replay_business_contract_passed
            .max(right.replay_business_contract_passed),
        replay_business_contract_failed: left
            .replay_business_contract_failed
            .max(right.replay_business_contract_failed),
        replay_business_contract_raw_passed: left
            .replay_business_contract_raw_passed
            .max(right.replay_business_contract_raw_passed),
        replay_business_contract_raw_failed: left
            .replay_business_contract_raw_failed
            .max(right.replay_business_contract_raw_failed),
        replay_business_contract_response_normalized: left
            .replay_business_contract_response_normalized
            .max(right.replay_business_contract_response_normalized),
        replay_business_contract_sanitized: left
            .replay_business_contract_sanitized
            .max(right.replay_business_contract_sanitized),
        replay_business_contract_canonical_fallbacks: left
            .replay_business_contract_canonical_fallbacks
            .max(right.replay_business_contract_canonical_fallbacks),
        replay_live_evolution_items: left
            .replay_live_evolution_items
            .max(right.replay_live_evolution_items),
        replay_live_evolution_router_threshold_mutations: left
            .replay_live_evolution_router_threshold_mutations
            .max(right.replay_live_evolution_router_threshold_mutations),
        replay_live_evolution_hierarchy_weight_mutations: left
            .replay_live_evolution_hierarchy_weight_mutations
            .max(right.replay_live_evolution_hierarchy_weight_mutations),
        replay_live_evolution_router_threshold_delta: left
            .replay_live_evolution_router_threshold_delta
            .max(right.replay_live_evolution_router_threshold_delta),
        replay_live_evolution_hierarchy_weight_delta: left
            .replay_live_evolution_hierarchy_weight_delta
            .max(right.replay_live_evolution_hierarchy_weight_delta),
        replay_live_evolution_online_reward_feedbacks: left
            .replay_live_evolution_online_reward_feedbacks
            .max(right.replay_live_evolution_online_reward_feedbacks),
        replay_live_evolution_online_reward_reinforcements: left
            .replay_live_evolution_online_reward_reinforcements
            .max(right.replay_live_evolution_online_reward_reinforcements),
        replay_live_evolution_online_reward_penalties: left
            .replay_live_evolution_online_reward_penalties
            .max(right.replay_live_evolution_online_reward_penalties),
        replay_live_evolution_online_reward_strength: left
            .replay_live_evolution_online_reward_strength
            .max(right.replay_live_evolution_online_reward_strength),
        replay_live_evolution_online_reward_reinforcement_strength: left
            .replay_live_evolution_online_reward_reinforcement_strength
            .max(right.replay_live_evolution_online_reward_reinforcement_strength),
        replay_live_evolution_online_reward_penalty_strength: left
            .replay_live_evolution_online_reward_penalty_strength
            .max(right.replay_live_evolution_online_reward_penalty_strength),
        replay_live_evolution_memory_updates: left
            .replay_live_evolution_memory_updates
            .max(right.replay_live_evolution_memory_updates),
        replay_live_evolution_stored_memory_updates: left
            .replay_live_evolution_stored_memory_updates
            .max(right.replay_live_evolution_stored_memory_updates),
        replay_live_evolution_reflection_issues: left
            .replay_live_evolution_reflection_issues
            .max(right.replay_live_evolution_reflection_issues),
        replay_live_evolution_critical_reflection_issues: left
            .replay_live_evolution_critical_reflection_issues
            .max(right.replay_live_evolution_critical_reflection_issues),
        replay_live_evolution_revision_actions: left
            .replay_live_evolution_revision_actions
            .max(right.replay_live_evolution_revision_actions),
        recursive_replay_items: left
            .recursive_replay_items
            .max(right.recursive_replay_items),
        recursive_runtime_calls: left
            .recursive_runtime_calls
            .max(right.recursive_runtime_calls),
        drift_rollbacks: left.drift_rollbacks.max(right.drift_rollbacks),
        rollback_router_threshold_delta: left
            .rollback_router_threshold_delta
            .max(right.rollback_router_threshold_delta),
        rollback_hierarchy_weight_delta: left
            .rollback_hierarchy_weight_delta
            .max(right.rollback_hierarchy_weight_delta),
        external_feedbacks: left.external_feedbacks.max(right.external_feedbacks),
        external_feedback_reinforcements: left
            .external_feedback_reinforcements
            .max(right.external_feedback_reinforcements),
        external_feedback_penalties: left
            .external_feedback_penalties
            .max(right.external_feedback_penalties),
        external_feedback_memory_updates: left
            .external_feedback_memory_updates
            .max(right.external_feedback_memory_updates),
        external_feedback_removed: left
            .external_feedback_removed
            .max(right.external_feedback_removed),
        external_feedback_missing: left
            .external_feedback_missing
            .max(right.external_feedback_missing),
        external_feedback_strength_delta: left
            .external_feedback_strength_delta
            .max(right.external_feedback_strength_delta),
    }
}
