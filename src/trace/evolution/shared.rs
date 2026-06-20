use super::super::TRACE_FLOAT_EPSILON;

pub(in crate::trace) fn require_usize_at_least(
    failures: &mut Vec<String>,
    cumulative_name: &str,
    cumulative: usize,
    live_name: &str,
    live: usize,
) {
    if cumulative < live {
        failures.push(format!(
            "{cumulative_name} {cumulative} is below {live_name} {live}"
        ));
    }
}

pub(in crate::trace::evolution) fn require_f32_at_least(
    failures: &mut Vec<String>,
    cumulative_name: &str,
    cumulative: f32,
    live_name: &str,
    live: f32,
) {
    if cumulative + TRACE_FLOAT_EPSILON < live {
        failures.push(format!(
            "{cumulative_name} {cumulative:.6} is below {live_name} {live:.6}"
        ));
    }
}

#[derive(Debug, Clone, Copy)]
pub(in crate::trace::evolution) struct OnlineRewardStrength {
    pub feedbacks: usize,
    pub reinforcements: usize,
    pub penalties: usize,
    pub total_strength: f32,
    pub reinforcement_strength: f32,
    pub penalty_strength: f32,
}

pub(in crate::trace::evolution) fn check_online_reward_strength(
    failures: &mut Vec<String>,
    label: &str,
    reward: OnlineRewardStrength,
) {
    for (name, value) in [
        ("strength", reward.total_strength),
        ("reinforcement_strength", reward.reinforcement_strength),
        ("penalty_strength", reward.penalty_strength),
    ] {
        if value < -TRACE_FLOAT_EPSILON {
            failures.push(format!("{label}_{name} {value:.6} is negative"));
        }
    }

    let component_strength = reward.reinforcement_strength + reward.penalty_strength;
    if (reward.total_strength - component_strength).abs() > TRACE_FLOAT_EPSILON {
        failures.push(format!(
            "{label}_strength {:.6} does not match reinforcement+penalty strength {component_strength:.6}",
            reward.total_strength
        ));
    }
    if reward.total_strength > TRACE_FLOAT_EPSILON && reward.feedbacks == 0 {
        failures.push(format!(
            "{label}_strength {:.6} requires feedbacks > 0",
            reward.total_strength
        ));
    }
    if reward.feedbacks > 0 && reward.total_strength <= TRACE_FLOAT_EPSILON {
        failures.push(format!(
            "{label}_strength {:.6} requires positive strength when feedbacks > 0",
            reward.total_strength
        ));
    }
    if reward.reinforcement_strength > TRACE_FLOAT_EPSILON && reward.reinforcements == 0 {
        failures.push(format!(
            "{label}_reinforcement_strength {:.6} requires reinforcements > 0",
            reward.reinforcement_strength
        ));
    }
    if reward.reinforcements > 0 && reward.reinforcement_strength <= TRACE_FLOAT_EPSILON {
        failures.push(format!(
            "{label}_reinforcement_strength {:.6} requires positive strength when reinforcements > 0",
            reward.reinforcement_strength
        ));
    }
    if reward.penalty_strength > TRACE_FLOAT_EPSILON && reward.penalties == 0 {
        failures.push(format!(
            "{label}_penalty_strength {:.6} requires penalties > 0",
            reward.penalty_strength
        ));
    }
    if reward.penalties > 0 && reward.penalty_strength <= TRACE_FLOAT_EPSILON {
        failures.push(format!(
            "{label}_penalty_strength {:.6} requires positive strength when penalties > 0",
            reward.penalty_strength
        ));
    }
}
