use super::super::super::TRACE_FLOAT_EPSILON;
use super::super::shared::{
    OnlineRewardStrength, check_online_reward_strength, require_f32_at_least,
    require_usize_at_least,
};
use super::context::LiveEvolutionTrace;

pub(super) fn evaluate_cumulative_trace(failures: &mut Vec<String>, trace: &LiveEvolutionTrace) {
    evaluate_inference_and_adaptive(failures, trace);
    evaluate_online_reward(failures, trace);
    evaluate_memory_feedback(failures, trace);
    evaluate_stored_memory(failures, trace);
    evaluate_reflection(failures, trace);
}

fn evaluate_inference_and_adaptive(failures: &mut Vec<String>, trace: &LiveEvolutionTrace) {
    let adaptive = trace.adaptive;
    require_usize_at_least(
        failures,
        "live_inference_runs",
        trace.cumulative.inference_runs,
        "live_inference_recorded",
        usize::from(trace.live_inference_recorded),
    );

    require_f32_at_least(
        failures,
        "cumulative_live_router_threshold_delta",
        adaptive.cumulative_router_threshold_delta,
        "live_router_threshold_delta",
        adaptive.router_threshold_delta,
    );
    require_usize_at_least(
        failures,
        "cumulative_live_router_threshold_mutations",
        adaptive.cumulative_router_threshold_mutations,
        "live_router_threshold_mutation",
        usize::from(adaptive.router_threshold_delta > TRACE_FLOAT_EPSILON),
    );

    require_f32_at_least(
        failures,
        "cumulative_live_hierarchy_weight_delta",
        adaptive.cumulative_hierarchy_weight_delta,
        "live_hierarchy_weight_delta",
        adaptive.hierarchy_weight_delta,
    );
    require_usize_at_least(
        failures,
        "cumulative_live_hierarchy_weight_mutations",
        adaptive.cumulative_hierarchy_weight_mutations,
        "live_hierarchy_weight_mutation",
        usize::from(adaptive.hierarchy_weight_delta > TRACE_FLOAT_EPSILON),
    );
}

fn evaluate_online_reward(failures: &mut Vec<String>, trace: &LiveEvolutionTrace) {
    let live = trace.online_reward;
    let cumulative = trace.cumulative.online_reward;
    let expected_cumulative_live_online_reward_feedbacks = cumulative.expected_feedbacks();
    if cumulative.feedbacks != expected_cumulative_live_online_reward_feedbacks {
        failures.push(format!(
            "cumulative_live_online_reward_feedbacks {} does not match cumulative live online reward components {expected_cumulative_live_online_reward_feedbacks}",
            cumulative.feedbacks
        ));
    }
    check_online_reward_strength(
        failures,
        "cumulative_live_online_reward",
        OnlineRewardStrength {
            feedbacks: cumulative.feedbacks,
            reinforcements: cumulative.reinforcements,
            penalties: cumulative.penalties,
            total_strength: cumulative.strength,
            reinforcement_strength: cumulative.reinforcement_strength,
            penalty_strength: cumulative.penalty_strength,
        },
    );
    require_usize_at_least(
        failures,
        "cumulative_live_online_reward_feedbacks",
        cumulative.feedbacks,
        "live_online_reward_feedbacks",
        live.feedbacks,
    );
    require_usize_at_least(
        failures,
        "cumulative_live_online_reward_reinforcements",
        cumulative.reinforcements,
        "live_online_reward_reinforcements",
        live.reinforcements,
    );
    require_usize_at_least(
        failures,
        "cumulative_live_online_reward_penalties",
        cumulative.penalties,
        "live_online_reward_penalties",
        live.penalties,
    );
    require_f32_at_least(
        failures,
        "cumulative_live_online_reward_strength",
        cumulative.strength,
        "live_online_reward_strength",
        live.strength,
    );
    require_f32_at_least(
        failures,
        "cumulative_live_online_reward_reinforcement_strength",
        cumulative.reinforcement_strength,
        "live_online_reward_reinforcement_strength",
        live.reinforcement_strength,
    );
    require_f32_at_least(
        failures,
        "cumulative_live_online_reward_penalty_strength",
        cumulative.penalty_strength,
        "live_online_reward_penalty_strength",
        live.penalty_strength,
    );
}

fn evaluate_memory_feedback(failures: &mut Vec<String>, trace: &LiveEvolutionTrace) {
    let live = trace.memory;
    let cumulative = trace.cumulative.memory;
    let expected_cumulative_live_memory_updates = cumulative.expected_updates();
    if cumulative.updates != expected_cumulative_live_memory_updates {
        failures.push(format!(
            "cumulative_live_memory_updates {} does not match cumulative_live_memory_reinforcements+cumulative_live_memory_penalties {expected_cumulative_live_memory_updates}",
            cumulative.updates
        ));
    }
    require_usize_at_least(
        failures,
        "cumulative_live_memory_reinforcements",
        cumulative.reinforcements,
        "live_memory_reinforcements",
        live.reinforcements,
    );
    require_usize_at_least(
        failures,
        "cumulative_live_memory_penalties",
        cumulative.penalties,
        "live_memory_penalties",
        live.penalties,
    );
    require_usize_at_least(
        failures,
        "cumulative_live_memory_updates",
        cumulative.updates,
        "live_memory_updates",
        live.updates,
    );
}

fn evaluate_stored_memory(failures: &mut Vec<String>, trace: &LiveEvolutionTrace) {
    let live = trace.stored;
    let cumulative = trace.cumulative.stored;
    let expected_cumulative_live_stored_memory_updates = cumulative.expected_updates();
    if cumulative.updates != expected_cumulative_live_stored_memory_updates {
        failures.push(format!(
            "cumulative_live_stored_memory_updates {} does not match cumulative live stored memory components {expected_cumulative_live_stored_memory_updates}",
            cumulative.updates
        ));
    }
    require_usize_at_least(
        failures,
        "cumulative_live_stored_memories",
        cumulative.memories,
        "live_stored_memory",
        live.memory,
    );
    require_usize_at_least(
        failures,
        "cumulative_live_stored_gist_memories",
        cumulative.gist_memories,
        "live_stored_gist_memories",
        live.gist_memories,
    );
    require_usize_at_least(
        failures,
        "cumulative_live_stored_runtime_kv_memories",
        cumulative.runtime_kv_memories,
        "live_stored_runtime_kv_memories",
        live.runtime_kv_memories,
    );
    require_usize_at_least(
        failures,
        "cumulative_live_stored_memory_updates",
        cumulative.updates,
        "live_stored_memory_updates",
        live.updates,
    );
}

fn evaluate_reflection(failures: &mut Vec<String>, trace: &LiveEvolutionTrace) {
    let live = trace.reflection;
    let cumulative = trace.cumulative.reflection;
    require_usize_at_least(
        failures,
        "cumulative_live_reflection_issues",
        cumulative.issues,
        "live_reflection_issues",
        live.live_issues,
    );
    require_usize_at_least(
        failures,
        "cumulative_live_critical_reflection_issues",
        cumulative.critical_issues,
        "live_critical_reflection_issues",
        live.live_critical_issues,
    );
    require_usize_at_least(
        failures,
        "cumulative_live_revision_actions",
        cumulative.revision_actions,
        "live_revision_actions",
        live.live_revision_actions,
    );
}
