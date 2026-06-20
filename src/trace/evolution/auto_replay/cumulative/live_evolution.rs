use super::super::super::shared::{
    OnlineRewardStrength, check_online_reward_strength, require_f32_at_least,
    require_usize_at_least,
};
use super::super::context::AutoReplayTrace;

pub(super) fn require_live_evolution(failures: &mut Vec<String>, trace: &AutoReplayTrace) {
    let current = &trace.live_evolution;
    let cumulative = &trace.cumulative.replay_live_evolution;

    require_usize_at_least(
        failures,
        "cumulative_replay_live_evolution_items",
        cumulative.items,
        "auto_replay live_evolution_items",
        current.items,
    );
    require_usize_at_least(
        failures,
        "cumulative_replay_live_evolution_router_threshold_mutations",
        cumulative.router_threshold_mutations,
        "auto_replay live_evolution_router_threshold_mutations",
        current.router_threshold_mutations,
    );
    require_usize_at_least(
        failures,
        "cumulative_replay_live_evolution_hierarchy_weight_mutations",
        cumulative.hierarchy_weight_mutations,
        "auto_replay live_evolution_hierarchy_weight_mutations",
        current.hierarchy_weight_mutations,
    );
    require_f32_at_least(
        failures,
        "cumulative_replay_live_evolution_router_threshold_delta",
        cumulative.router_threshold_delta,
        "auto_replay live_evolution_router_threshold_delta",
        current.router_threshold_delta,
    );
    require_f32_at_least(
        failures,
        "cumulative_replay_live_evolution_hierarchy_weight_delta",
        cumulative.hierarchy_weight_delta,
        "auto_replay live_evolution_hierarchy_weight_delta",
        current.hierarchy_weight_delta,
    );

    let expected_cumulative_replay_online_reward_feedbacks = cumulative
        .online_reward_reinforcements
        .saturating_add(cumulative.online_reward_penalties);
    if cumulative.online_reward_feedbacks != expected_cumulative_replay_online_reward_feedbacks {
        failures.push(format!(
            "cumulative_replay_live_evolution_online_reward_feedbacks {} does not match cumulative replay live evolution online reward components {expected_cumulative_replay_online_reward_feedbacks}",
            cumulative.online_reward_feedbacks
        ));
    }
    check_online_reward_strength(
        failures,
        "cumulative_replay_live_evolution_online_reward",
        OnlineRewardStrength {
            feedbacks: cumulative.online_reward_feedbacks,
            reinforcements: cumulative.online_reward_reinforcements,
            penalties: cumulative.online_reward_penalties,
            total_strength: cumulative.online_reward_strength,
            reinforcement_strength: cumulative.online_reward_reinforcement_strength,
            penalty_strength: cumulative.online_reward_penalty_strength,
        },
    );

    require_usize_at_least(
        failures,
        "cumulative_replay_live_evolution_online_reward_feedbacks",
        cumulative.online_reward_feedbacks,
        "auto_replay live_evolution_online_reward_feedbacks",
        current.online_reward_feedbacks,
    );
    require_usize_at_least(
        failures,
        "cumulative_replay_live_evolution_online_reward_reinforcements",
        cumulative.online_reward_reinforcements,
        "auto_replay live_evolution_online_reward_reinforcements",
        current.online_reward_reinforcements,
    );
    require_usize_at_least(
        failures,
        "cumulative_replay_live_evolution_online_reward_penalties",
        cumulative.online_reward_penalties,
        "auto_replay live_evolution_online_reward_penalties",
        current.online_reward_penalties,
    );
    require_f32_at_least(
        failures,
        "cumulative_replay_live_evolution_online_reward_strength",
        cumulative.online_reward_strength,
        "auto_replay live_evolution_online_reward_strength",
        current.online_reward_strength,
    );
    require_f32_at_least(
        failures,
        "cumulative_replay_live_evolution_online_reward_reinforcement_strength",
        cumulative.online_reward_reinforcement_strength,
        "auto_replay live_evolution_online_reward_reinforcement_strength",
        current.online_reward_reinforcement_strength,
    );
    require_f32_at_least(
        failures,
        "cumulative_replay_live_evolution_online_reward_penalty_strength",
        cumulative.online_reward_penalty_strength,
        "auto_replay live_evolution_online_reward_penalty_strength",
        current.online_reward_penalty_strength,
    );
    require_usize_at_least(
        failures,
        "cumulative_replay_live_evolution_memory_updates",
        cumulative.memory_updates,
        "auto_replay live_evolution_memory_updates",
        current.memory_updates,
    );
    require_usize_at_least(
        failures,
        "cumulative_replay_live_evolution_stored_memory_updates",
        cumulative.stored_memory_updates,
        "auto_replay live_evolution_stored_memory_updates",
        current.stored_memory_updates,
    );
    require_usize_at_least(
        failures,
        "cumulative_replay_live_evolution_reflection_issues",
        cumulative.reflection_issues,
        "auto_replay live_evolution_reflection_issues",
        current.reflection_issues,
    );
    require_usize_at_least(
        failures,
        "cumulative_replay_live_evolution_critical_reflection_issues",
        cumulative.critical_reflection_issues,
        "auto_replay live_evolution_critical_reflection_issues",
        current.critical_reflection_issues,
    );
    require_usize_at_least(
        failures,
        "cumulative_replay_live_evolution_revision_actions",
        cumulative.revision_actions,
        "auto_replay live_evolution_revision_actions",
        current.revision_actions,
    );
}
