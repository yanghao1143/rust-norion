use crate::adaptive_state::LiveInferenceEvolution;

use super::fields::{bool_to_field, field_to_bool, field_to_finite_f32, finite_f32_to_field};

const EXPERIENCE_FLOAT_EPSILON: f32 = 0.000_001;

pub(super) fn serialize_live_evolution(report: LiveInferenceEvolution) -> String {
    [
        finite_f32_to_field(report.router_threshold_delta),
        finite_f32_to_field(report.hierarchy_weight_delta),
        report.online_reward_feedbacks.to_string(),
        report.online_reward_reinforcements.to_string(),
        report.online_reward_penalties.to_string(),
        finite_f32_to_field(report.online_reward_strength.max(0.0)),
        finite_f32_to_field(report.online_reward_reinforcement_strength.max(0.0)),
        finite_f32_to_field(report.online_reward_penalty_strength.max(0.0)),
        report.memory_reinforcements.to_string(),
        report.memory_penalties.to_string(),
        bool_to_field(report.stored_memory).to_owned(),
        report.stored_gist_memories.to_string(),
        report.stored_runtime_kv_memories.to_string(),
        report.reflection_issues.to_string(),
        report.critical_reflection_issues.to_string(),
        report.revision_actions.to_string(),
    ]
    .join(",")
}

pub(in crate::experience) fn deserialize_live_evolution(
    value: &str,
) -> Option<LiveInferenceEvolution> {
    if value.is_empty() {
        return Some(LiveInferenceEvolution::default());
    }

    let fields = value.split(',').collect::<Vec<_>>();
    if fields.len() != 10 && fields.len() != 13 && fields.len() != 16 {
        return None;
    }
    let has_online_reward_feedback = fields.len() >= 13;
    let has_online_reward_strength = fields.len() == 16;
    let memory_index = if has_online_reward_strength {
        8
    } else if has_online_reward_feedback {
        5
    } else {
        2
    };

    let online_reward_feedbacks = if has_online_reward_feedback {
        fields[2].parse::<usize>().ok()?
    } else {
        0
    };
    let online_reward_reinforcements = if has_online_reward_feedback {
        fields[3].parse::<usize>().ok()?
    } else {
        0
    };
    let online_reward_penalties = if has_online_reward_feedback {
        fields[4].parse::<usize>().ok()?
    } else {
        0
    };
    if has_online_reward_feedback
        && online_reward_feedbacks
            != online_reward_reinforcements.saturating_add(online_reward_penalties)
    {
        return None;
    }

    let online_reward_strength = if has_online_reward_strength {
        nonnegative_finite_f32_field(fields[5])?
    } else {
        0.0
    };
    let online_reward_reinforcement_strength = if has_online_reward_strength {
        nonnegative_finite_f32_field(fields[6])?
    } else {
        0.0
    };
    let online_reward_penalty_strength = if has_online_reward_strength {
        nonnegative_finite_f32_field(fields[7])?
    } else {
        0.0
    };

    let report = LiveInferenceEvolution {
        router_threshold_delta: field_to_finite_f32(fields[0])?.max(0.0),
        hierarchy_weight_delta: field_to_finite_f32(fields[1])?.max(0.0),
        online_reward_feedbacks,
        online_reward_reinforcements,
        online_reward_penalties,
        online_reward_strength,
        online_reward_reinforcement_strength,
        online_reward_penalty_strength,
        memory_reinforcements: fields[memory_index].parse::<usize>().ok()?,
        memory_penalties: fields[memory_index + 1].parse::<usize>().ok()?,
        stored_memory: field_to_bool(fields[memory_index + 2])?,
        stored_gist_memories: fields[memory_index + 3].parse::<usize>().ok()?,
        stored_runtime_kv_memories: fields[memory_index + 4].parse::<usize>().ok()?,
        reflection_issues: fields[memory_index + 5].parse::<usize>().ok()?,
        critical_reflection_issues: fields[memory_index + 6].parse::<usize>().ok()?,
        revision_actions: fields[memory_index + 7].parse::<usize>().ok()?,
    };

    if has_online_reward_strength && !live_online_reward_strength_is_consistent(&report) {
        return None;
    }

    Some(report)
}

fn nonnegative_finite_f32_field(value: &str) -> Option<f32> {
    field_to_finite_f32(value).filter(|value| *value >= 0.0)
}

fn live_online_reward_strength_is_consistent(report: &LiveInferenceEvolution) -> bool {
    let has_reinforcement_strength =
        report.online_reward_reinforcement_strength > EXPERIENCE_FLOAT_EPSILON;
    let has_penalty_strength = report.online_reward_penalty_strength > EXPERIENCE_FLOAT_EPSILON;
    report.online_reward_strength.is_finite()
        && report.online_reward_reinforcement_strength.is_finite()
        && report.online_reward_penalty_strength.is_finite()
        && report.online_reward_feedbacks
            == report
                .online_reward_reinforcements
                .saturating_add(report.online_reward_penalties)
        && report.online_reward_strength >= 0.0
        && report.online_reward_reinforcement_strength >= 0.0
        && report.online_reward_penalty_strength >= 0.0
        && !(report.online_reward_strength > EXPERIENCE_FLOAT_EPSILON
            && report.online_reward_feedbacks == 0)
        && !(report.online_reward_feedbacks > 0
            && report.online_reward_strength <= EXPERIENCE_FLOAT_EPSILON)
        && !(has_reinforcement_strength && report.online_reward_reinforcements == 0)
        && !(report.online_reward_reinforcements > 0
            && report.online_reward_reinforcement_strength <= EXPERIENCE_FLOAT_EPSILON)
        && !(has_penalty_strength && report.online_reward_penalties == 0)
        && !(report.online_reward_penalties > 0
            && report.online_reward_penalty_strength <= EXPERIENCE_FLOAT_EPSILON)
        && (report.online_reward_strength
            - (report.online_reward_reinforcement_strength + report.online_reward_penalty_strength))
            .abs()
            <= EXPERIENCE_FLOAT_EPSILON
}
