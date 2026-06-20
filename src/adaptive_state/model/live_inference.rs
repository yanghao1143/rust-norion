use super::nonnegative_f32;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LiveInferenceEvolution {
    pub router_threshold_delta: f32,
    pub hierarchy_weight_delta: f32,
    pub online_reward_feedbacks: usize,
    pub online_reward_reinforcements: usize,
    pub online_reward_penalties: usize,
    pub online_reward_strength: f32,
    pub online_reward_reinforcement_strength: f32,
    pub online_reward_penalty_strength: f32,
    pub memory_reinforcements: usize,
    pub memory_penalties: usize,
    pub stored_memory: bool,
    pub stored_gist_memories: usize,
    pub stored_runtime_kv_memories: usize,
    pub reflection_issues: usize,
    pub critical_reflection_issues: usize,
    pub revision_actions: usize,
}

impl LiveInferenceEvolution {
    pub fn memory_updates(self) -> usize {
        self.memory_reinforcements
            .saturating_add(self.memory_penalties)
    }

    pub fn stored_memory_updates(self) -> usize {
        usize::from(self.stored_memory)
            .saturating_add(self.stored_gist_memories)
            .saturating_add(self.stored_runtime_kv_memories)
    }

    pub fn has_evidence(self) -> bool {
        self.router_threshold_delta > 0.000001
            || self.hierarchy_weight_delta > 0.000001
            || self.online_reward_feedbacks > 0
            || nonnegative_f32(self.online_reward_strength) > 0.000001
            || self.memory_updates() > 0
            || self.stored_memory_updates() > 0
            || self.reflection_issues > 0
            || self.critical_reflection_issues > 0
            || self.revision_actions > 0
    }
}
