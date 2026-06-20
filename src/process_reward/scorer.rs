use super::components::{
    action_for_total, coordination_adjusted_total, score_components, toolsmith_adjusted_total,
    weighted_total,
};
use super::notes::reward_notes;
use super::types::{ProcessRewardInput, ProcessRewardReport};

#[derive(Debug, Clone, Default)]
pub struct ProcessRewarder;

impl ProcessRewarder {
    pub fn new() -> Self {
        Self
    }

    pub fn score(&self, input: ProcessRewardInput) -> ProcessRewardReport {
        let quality = input.quality.clamp(0.0, 1.0);
        let quality_score = input.metrics.quality_score();
        let components = score_components(&input, quality, quality_score);
        let total = coordination_adjusted_total(
            toolsmith_adjusted_total(weighted_total(components), &input.toolsmith_plan),
            &input.agent_team_plan,
        );
        let action = action_for_total(total);
        let notes = reward_notes(&input, components, total);

        ProcessRewardReport {
            total,
            components,
            action,
            notes,
        }
    }
}
