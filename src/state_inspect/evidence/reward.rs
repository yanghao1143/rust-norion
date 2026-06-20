use super::super::STATE_INSPECTION_FLOAT_EPSILON;

pub(in crate::state_inspect) fn online_reward_strength_is_consistent(
    feedbacks: u64,
    reinforcements: u64,
    penalties: u64,
    total: f32,
    reinforcement: f32,
    penalty: f32,
) -> bool {
    let has_reinforcement_strength = reinforcement > STATE_INSPECTION_FLOAT_EPSILON;
    let has_penalty_strength = penalty > STATE_INSPECTION_FLOAT_EPSILON;
    total.is_finite()
        && reinforcement.is_finite()
        && penalty.is_finite()
        && feedbacks > 0
        && feedbacks == reinforcements.saturating_add(penalties)
        && total > STATE_INSPECTION_FLOAT_EPSILON
        && reinforcement >= 0.0
        && penalty >= 0.0
        && (!has_reinforcement_strength || reinforcements > 0)
        && (!has_penalty_strength || penalties > 0)
        && (total - (reinforcement + penalty)).abs() <= STATE_INSPECTION_FLOAT_EPSILON
}
