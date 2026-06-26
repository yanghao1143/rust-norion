use super::BenchmarkSummary;

impl BenchmarkSummary {
    pub fn total_auto_replay_applied(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.auto_replay_applied)
            .sum()
    }

    pub fn total_auto_replay_router_updates(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.auto_replay_router_updates)
            .sum()
    }

    pub fn total_auto_replay_hierarchy_updates(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.auto_replay_hierarchy_updates)
            .sum()
    }

    pub fn total_auto_replay_router_threshold_mutations(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.auto_replay_router_threshold_mutations)
            .sum()
    }

    pub fn total_auto_replay_hierarchy_weight_mutations(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.auto_replay_hierarchy_weight_mutations)
            .sum()
    }

    pub fn total_auto_replay_router_threshold_delta(&self) -> f32 {
        self.results
            .iter()
            .map(|result| result.auto_replay_router_threshold_delta)
            .sum()
    }

    pub fn total_auto_replay_hierarchy_weight_delta(&self) -> f32 {
        self.results
            .iter()
            .map(|result| result.auto_replay_hierarchy_weight_delta)
            .sum()
    }

    pub fn total_auto_replay_memory_reinforcements(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.auto_replay_memory_reinforcements)
            .sum()
    }

    pub fn total_auto_replay_memory_penalties(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.auto_replay_memory_penalties)
            .sum()
    }

    pub fn total_auto_replay_memory_updates(&self) -> usize {
        self.total_auto_replay_memory_reinforcements() + self.total_auto_replay_memory_penalties()
    }

    pub fn total_auto_replay_live_memory_feedback_items(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.auto_replay_live_memory_feedback_items)
            .sum()
    }

    pub fn total_auto_replay_live_memory_feedback_reinforcements(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.auto_replay_live_memory_feedback_reinforcements)
            .sum()
    }

    pub fn total_auto_replay_live_memory_feedback_penalties(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.auto_replay_live_memory_feedback_penalties)
            .sum()
    }

    pub fn total_auto_replay_live_memory_feedback_updates(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.auto_replay_live_memory_feedback_updates)
            .sum()
    }

    pub fn total_auto_replay_live_memory_feedback_detail_items(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.auto_replay_live_memory_feedback_detail_items)
            .sum()
    }

    pub fn total_auto_replay_live_memory_feedback_applied(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.auto_replay_live_memory_feedback_applied)
            .sum()
    }

    pub fn total_auto_replay_live_memory_feedback_removed(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.auto_replay_live_memory_feedback_removed)
            .sum()
    }

    pub fn total_auto_replay_live_memory_feedback_missing(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.auto_replay_live_memory_feedback_missing)
            .sum()
    }

    pub fn total_auto_replay_live_memory_feedback_strength_delta(&self) -> f32 {
        self.results
            .iter()
            .map(|result| result.auto_replay_live_memory_feedback_strength_delta)
            .sum()
    }

    pub fn total_auto_replay_recursive_items(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.auto_replay_recursive_runtime_items)
            .sum()
    }

    pub fn total_auto_replay_recursive_runtime_calls(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.auto_replay_recursive_runtime_calls)
            .sum()
    }

    pub fn max_auto_replay_recursive_call_pressure(&self) -> f32 {
        self.results
            .iter()
            .map(|result| result.auto_replay_max_recursive_call_pressure)
            .fold(0.0, f32::max)
    }

    pub fn total_auto_replay_runtime_kv_budget_pressure_items(&self) -> usize {
        self.runtime_architecture_evidence
            .auto_replay_runtime_kv_budget_pressure_items()
    }

    pub fn average_auto_replay_runtime_kv_budget_pressure(&self) -> f32 {
        self.runtime_architecture_evidence
            .average_auto_replay_runtime_kv_budget_pressure()
    }

    pub fn max_auto_replay_runtime_kv_budget_pressure(&self) -> f32 {
        self.runtime_architecture_evidence
            .max_auto_replay_runtime_kv_budget_pressure()
    }
}
