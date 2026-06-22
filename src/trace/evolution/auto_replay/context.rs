use super::super::super::fields::{
    extract_json_f32_field, extract_json_usize_field, json_object_after_field,
};

pub(super) struct AutoReplayTrace {
    pub(super) applied: usize,
    pub(super) router_updates: usize,
    pub(super) hierarchy_updates: usize,
    pub(super) router_threshold_mutations: usize,
    pub(super) hierarchy_weight_mutations: usize,
    pub(super) router_threshold_delta: f32,
    pub(super) hierarchy_weight_delta: f32,
    pub(super) reinforced: usize,
    pub(super) penalized: usize,
    pub(super) touched_memories: usize,
    pub(super) memory_reinforcements: usize,
    pub(super) memory_penalties: usize,
    pub(super) live_memory_feedback: LiveMemoryFeedbackTrace,
    pub(super) business_contract: BusinessContractTrace,
    pub(super) live_evolution: LiveEvolutionTrace,
    pub(super) recursive_runtime: RecursiveRuntimeTrace,
    pub(super) replay_runs: usize,
    pub(super) replay_items: usize,
    pub(super) cumulative: CumulativeAutoReplayTrace,
}

impl AutoReplayTrace {
    pub(super) fn from_line(line: &str) -> Self {
        let auto_replay = json_object_after_field(line, "auto_replay").unwrap_or("");
        let evolution_ledger = json_object_after_field(line, "evolution_ledger").unwrap_or("");

        Self {
            applied: json_usize(auto_replay, "applied"),
            router_updates: json_usize(auto_replay, "router_updates"),
            hierarchy_updates: json_usize(auto_replay, "hierarchy_updates"),
            router_threshold_mutations: json_usize(auto_replay, "router_threshold_mutations"),
            hierarchy_weight_mutations: json_usize(auto_replay, "hierarchy_weight_mutations"),
            router_threshold_delta: json_f32(auto_replay, "router_threshold_delta"),
            hierarchy_weight_delta: json_f32(auto_replay, "hierarchy_weight_delta"),
            reinforced: json_usize(auto_replay, "reinforced"),
            penalized: json_usize(auto_replay, "penalized"),
            touched_memories: json_usize(auto_replay, "touched_memories"),
            memory_reinforcements: json_usize(auto_replay, "memory_reinforcements"),
            memory_penalties: json_usize(auto_replay, "memory_penalties"),
            live_memory_feedback: LiveMemoryFeedbackTrace::from_scope(
                auto_replay,
                "live_memory_feedback",
            ),
            business_contract: BusinessContractTrace::from_scope(auto_replay, "business_contract"),
            live_evolution: LiveEvolutionTrace::from_scope(auto_replay, "live_evolution"),
            recursive_runtime: RecursiveRuntimeTrace::from_scope(auto_replay),
            replay_runs: json_usize(evolution_ledger, "replay_runs"),
            replay_items: json_usize(evolution_ledger, "replay_items"),
            cumulative: CumulativeAutoReplayTrace::from_scope(evolution_ledger),
        }
    }
}

pub(super) struct LiveMemoryFeedbackTrace {
    pub(super) items: usize,
    pub(super) updates: usize,
    pub(super) reinforcements: usize,
    pub(super) penalties: usize,
    pub(super) detail_items: usize,
    pub(super) applied: usize,
    pub(super) removed: usize,
    pub(super) missing: usize,
    pub(super) strength_delta: f32,
}

impl LiveMemoryFeedbackTrace {
    fn from_scope(scope: &str, prefix: &str) -> Self {
        Self {
            items: json_usize(scope, &format!("{prefix}_items")),
            updates: json_usize(scope, &format!("{prefix}_updates")),
            reinforcements: json_usize(scope, &format!("{prefix}_reinforcements")),
            penalties: json_usize(scope, &format!("{prefix}_penalties")),
            detail_items: json_usize(scope, &format!("{prefix}_detail_items")),
            applied: json_usize(scope, &format!("{prefix}_applied")),
            removed: json_usize(scope, &format!("{prefix}_removed")),
            missing: json_usize(scope, &format!("{prefix}_missing")),
            strength_delta: json_f32(scope, &format!("{prefix}_strength_delta")),
        }
    }
}

pub(super) struct BusinessContractTrace {
    pub(super) items: usize,
    pub(super) passed: usize,
    pub(super) failed: usize,
    pub(super) raw_passed: usize,
    pub(super) raw_failed: usize,
    pub(super) response_normalized: usize,
    pub(super) sanitized: usize,
    pub(super) canonical_fallbacks: usize,
}

impl BusinessContractTrace {
    fn from_scope(scope: &str, prefix: &str) -> Self {
        Self {
            items: json_usize(scope, &format!("{prefix}_items")),
            passed: json_usize(scope, &format!("{prefix}_passed")),
            failed: json_usize(scope, &format!("{prefix}_failed")),
            raw_passed: json_usize(scope, &format!("{prefix}_raw_passed")),
            raw_failed: json_usize(scope, &format!("{prefix}_raw_failed")),
            response_normalized: json_usize(scope, &format!("{prefix}_response_normalized")),
            sanitized: json_usize(scope, &format!("{prefix}_sanitized")),
            canonical_fallbacks: json_usize(scope, &format!("{prefix}_canonical_fallbacks")),
        }
    }
}

pub(super) struct LiveEvolutionTrace {
    pub(super) items: usize,
    pub(super) router_threshold_mutations: usize,
    pub(super) hierarchy_weight_mutations: usize,
    pub(super) router_threshold_delta: f32,
    pub(super) hierarchy_weight_delta: f32,
    pub(super) online_reward_feedbacks: usize,
    pub(super) online_reward_reinforcements: usize,
    pub(super) online_reward_penalties: usize,
    pub(super) online_reward_strength: f32,
    pub(super) online_reward_reinforcement_strength: f32,
    pub(super) online_reward_penalty_strength: f32,
    pub(super) memory_updates: usize,
    pub(super) stored_memory_updates: usize,
    pub(super) reflection_issues: usize,
    pub(super) critical_reflection_issues: usize,
    pub(super) revision_actions: usize,
}

impl LiveEvolutionTrace {
    fn from_scope(scope: &str, prefix: &str) -> Self {
        Self {
            items: json_usize(scope, &format!("{prefix}_items")),
            router_threshold_mutations: json_usize(
                scope,
                &format!("{prefix}_router_threshold_mutations"),
            ),
            hierarchy_weight_mutations: json_usize(
                scope,
                &format!("{prefix}_hierarchy_weight_mutations"),
            ),
            router_threshold_delta: json_f32(scope, &format!("{prefix}_router_threshold_delta")),
            hierarchy_weight_delta: json_f32(scope, &format!("{prefix}_hierarchy_weight_delta")),
            online_reward_feedbacks: json_usize(
                scope,
                &format!("{prefix}_online_reward_feedbacks"),
            ),
            online_reward_reinforcements: json_usize(
                scope,
                &format!("{prefix}_online_reward_reinforcements"),
            ),
            online_reward_penalties: json_usize(
                scope,
                &format!("{prefix}_online_reward_penalties"),
            ),
            online_reward_strength: json_f32(scope, &format!("{prefix}_online_reward_strength")),
            online_reward_reinforcement_strength: json_f32(
                scope,
                &format!("{prefix}_online_reward_reinforcement_strength"),
            ),
            online_reward_penalty_strength: json_f32(
                scope,
                &format!("{prefix}_online_reward_penalty_strength"),
            ),
            memory_updates: json_usize(scope, &format!("{prefix}_memory_updates")),
            stored_memory_updates: json_usize(scope, &format!("{prefix}_stored_memory_updates")),
            reflection_issues: json_usize(scope, &format!("{prefix}_reflection_issues")),
            critical_reflection_issues: json_usize(
                scope,
                &format!("{prefix}_critical_reflection_issues"),
            ),
            revision_actions: json_usize(scope, &format!("{prefix}_revision_actions")),
        }
    }
}

pub(super) struct RecursiveRuntimeTrace {
    pub(super) items: usize,
    pub(super) calls: usize,
    pub(super) average_call_pressure: f32,
    pub(super) max_call_pressure: f32,
}

impl RecursiveRuntimeTrace {
    fn from_scope(scope: &str) -> Self {
        Self {
            items: json_usize(scope, "recursive_runtime_items"),
            calls: json_usize(scope, "recursive_runtime_calls"),
            average_call_pressure: json_f32(scope, "avg_recursive_call_pressure"),
            max_call_pressure: json_f32(scope, "max_recursive_call_pressure"),
        }
    }
}

pub(super) struct CumulativeAutoReplayTrace {
    pub(super) router_threshold_mutations: usize,
    pub(super) hierarchy_weight_mutations: usize,
    pub(super) router_threshold_delta: f32,
    pub(super) hierarchy_weight_delta: f32,
    pub(super) memory_reinforcements: usize,
    pub(super) memory_penalties: usize,
    pub(super) memory_updates: usize,
    pub(super) replay_live_memory_feedback: LiveMemoryFeedbackTrace,
    pub(super) replay_business_contract: BusinessContractTrace,
    pub(super) replay_live_evolution: LiveEvolutionTrace,
    pub(super) recursive_replay_items: usize,
    pub(super) recursive_runtime_calls: usize,
}

impl CumulativeAutoReplayTrace {
    fn from_scope(scope: &str) -> Self {
        Self {
            router_threshold_mutations: json_usize(scope, "cumulative_router_threshold_mutations"),
            hierarchy_weight_mutations: json_usize(scope, "cumulative_hierarchy_weight_mutations"),
            router_threshold_delta: json_f32(scope, "cumulative_router_threshold_delta"),
            hierarchy_weight_delta: json_f32(scope, "cumulative_hierarchy_weight_delta"),
            memory_reinforcements: json_usize(scope, "cumulative_memory_reinforcements"),
            memory_penalties: json_usize(scope, "cumulative_memory_penalties"),
            memory_updates: json_usize(scope, "cumulative_memory_updates"),
            replay_live_memory_feedback: LiveMemoryFeedbackTrace::from_scope(
                scope,
                "cumulative_replay_live_memory_feedback",
            ),
            replay_business_contract: BusinessContractTrace::from_scope(
                scope,
                "cumulative_replay_business_contract",
            ),
            replay_live_evolution: LiveEvolutionTrace::from_scope(
                scope,
                "cumulative_replay_live_evolution",
            ),
            recursive_replay_items: json_usize(scope, "cumulative_recursive_replay_items"),
            recursive_runtime_calls: json_usize(scope, "cumulative_recursive_runtime_calls"),
        }
    }
}

fn json_usize(line: &str, name: &str) -> usize {
    extract_json_usize_field(line, name).unwrap_or(0)
}

fn json_f32(line: &str, name: &str) -> f32 {
    extract_json_f32_field(line, name).unwrap_or(0.0)
}
