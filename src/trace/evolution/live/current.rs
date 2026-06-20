use super::super::shared::{OnlineRewardStrength, check_online_reward_strength};
use super::context::LiveEvolutionTrace;

pub(super) fn evaluate_current_trace(failures: &mut Vec<String>, trace: &LiveEvolutionTrace) {
    if !trace.live_inference_recorded {
        failures.push("live_evolution requires live_inference_recorded=true".to_owned());
    }

    evaluate_memory_feedback(failures, trace);
    evaluate_stored_memory(failures, trace);
    evaluate_reflection(failures, trace);
    evaluate_online_reward(failures, trace);
}

fn evaluate_memory_feedback(failures: &mut Vec<String>, trace: &LiveEvolutionTrace) {
    let memory = trace.memory;
    let expected_memory_updates = memory.expected_updates();
    if memory.updates != expected_memory_updates {
        failures.push(format!(
            "live_memory_updates {} does not match live_memory_reinforcements+live_memory_penalties {expected_memory_updates}",
            memory.updates
        ));
    }

    if memory.reinforcements != memory.feedback_reinforced {
        failures.push(format!(
            "live_memory_reinforcements {} does not match memory feedback_reinforced {}",
            memory.reinforcements, memory.feedback_reinforced
        ));
    }
    if memory.penalties != memory.feedback_penalized {
        failures.push(format!(
            "live_memory_penalties {} does not match memory feedback_penalized {}",
            memory.penalties, memory.feedback_penalized
        ));
    }
}

fn evaluate_stored_memory(failures: &mut Vec<String>, trace: &LiveEvolutionTrace) {
    let stored = trace.stored;
    let expected_stored_memory_updates = stored.expected_updates();
    if stored.updates != expected_stored_memory_updates {
        failures.push(format!(
            "live_stored_memory_updates {} does not match live stored memory components {expected_stored_memory_updates}",
            stored.updates
        ));
    }

    if stored.gist_memories != stored.gist_stored {
        failures.push(format!(
            "live_stored_gist_memories {} does not match memory gist_stored {}",
            stored.gist_memories, stored.gist_stored
        ));
    }
    if stored.runtime_kv_memories != stored.runtime_kv_stored {
        failures.push(format!(
            "live_stored_runtime_kv_memories {} does not match memory runtime_kv_stored {}",
            stored.runtime_kv_memories, stored.runtime_kv_stored
        ));
    }
}

fn evaluate_reflection(failures: &mut Vec<String>, trace: &LiveEvolutionTrace) {
    let reflection = trace.reflection;
    if reflection.live_issues != reflection.issues {
        failures.push(format!(
            "live_reflection_issues {} does not match reflection issues {}",
            reflection.live_issues, reflection.issues
        ));
    }

    if reflection.live_critical_issues != reflection.critical_issues {
        failures.push(format!(
            "live_critical_reflection_issues {} does not match reflection critical_issues {}",
            reflection.live_critical_issues, reflection.critical_issues
        ));
    }

    if reflection.live_revision_actions != reflection.revision_actions {
        failures.push(format!(
            "live_revision_actions {} does not match reflection revision_actions {}",
            reflection.live_revision_actions, reflection.revision_actions
        ));
    }
}

fn evaluate_online_reward(failures: &mut Vec<String>, trace: &LiveEvolutionTrace) {
    let reward = trace.online_reward;
    let expected_live_online_reward_feedbacks = reward.expected_feedbacks();
    if reward.feedbacks != expected_live_online_reward_feedbacks {
        failures.push(format!(
            "live_online_reward_feedbacks {} does not match live_online_reward_reinforcements+live_online_reward_penalties {expected_live_online_reward_feedbacks}",
            reward.feedbacks
        ));
    }
    check_online_reward_strength(
        failures,
        "live_online_reward",
        OnlineRewardStrength {
            feedbacks: reward.feedbacks,
            reinforcements: reward.reinforcements,
            penalties: reward.penalties,
            total_strength: reward.strength,
            reinforcement_strength: reward.reinforcement_strength,
            penalty_strength: reward.penalty_strength,
        },
    );
    if reward.feedbacks > 0 && !trace.has_online_reward_note {
        failures.push(
            "live_online_reward_feedbacks requires an online_reward_feedback note".to_owned(),
        );
    }
}
