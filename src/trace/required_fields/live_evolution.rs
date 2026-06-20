use super::{TraceRequiredField, required_field};

pub(super) const LIVE_EVOLUTION_TRACE_REQUIRED_FIELDS: &[TraceRequiredField] = &[
    required_field("live_evolution", "\"live_evolution\":{"),
    required_field("live_inference_recorded", "\"live_inference_recorded\":"),
    required_field(
        "live_router_threshold_delta",
        "\"live_router_threshold_delta\":",
    ),
    required_field(
        "live_hierarchy_weight_delta",
        "\"live_hierarchy_weight_delta\":",
    ),
    required_field(
        "live_online_reward_feedbacks",
        "\"live_online_reward_feedbacks\":",
    ),
    required_field(
        "live_online_reward_reinforcements",
        "\"live_online_reward_reinforcements\":",
    ),
    required_field(
        "live_online_reward_penalties",
        "\"live_online_reward_penalties\":",
    ),
    required_field(
        "live_online_reward_strength",
        "\"live_online_reward_strength\":",
    ),
    required_field(
        "live_online_reward_reinforcement_strength",
        "\"live_online_reward_reinforcement_strength\":",
    ),
    required_field(
        "live_online_reward_penalty_strength",
        "\"live_online_reward_penalty_strength\":",
    ),
    required_field("live_memory_updates", "\"live_memory_updates\":"),
    required_field(
        "live_memory_reinforcements",
        "\"live_memory_reinforcements\":",
    ),
    required_field("live_memory_penalties", "\"live_memory_penalties\":"),
    required_field(
        "live_stored_memory_updates",
        "\"live_stored_memory_updates\":",
    ),
    required_field("live_stored_memory", "\"live_stored_memory\":"),
    required_field(
        "live_stored_gist_memories",
        "\"live_stored_gist_memories\":",
    ),
    required_field(
        "live_stored_runtime_kv_memories",
        "\"live_stored_runtime_kv_memories\":",
    ),
    required_field("live_reflection_issues", "\"live_reflection_issues\":"),
    required_field(
        "live_critical_reflection_issues",
        "\"live_critical_reflection_issues\":",
    ),
    required_field("live_revision_actions", "\"live_revision_actions\":"),
];
