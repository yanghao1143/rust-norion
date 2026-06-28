use super::BenchmarkSummary;

impl BenchmarkSummary {
    pub fn adaptive_routing_cases(&self) -> usize {
        self.routing_evidence.cases
    }

    pub fn adaptive_routing_device_profiles(&self) -> usize {
        self.routing_evidence.device_profiles()
    }

    pub fn adaptive_routing_saved_token_device_profiles(&self) -> usize {
        self.routing_evidence.saved_token_device_profiles()
    }

    pub fn total_adaptive_routing_candidates(&self) -> usize {
        self.routing_evidence.candidates
    }

    pub fn total_adaptive_routing_included(&self) -> usize {
        self.routing_evidence.included
    }

    pub fn total_adaptive_routing_compressed(&self) -> usize {
        self.routing_evidence.compressed
    }

    pub fn total_adaptive_routing_deferred(&self) -> usize {
        self.routing_evidence.deferred
    }

    pub fn total_adaptive_routing_skipped(&self) -> usize {
        self.routing_evidence.skipped
    }

    pub fn total_adaptive_routing_input_tokens(&self) -> usize {
        self.routing_evidence.input_tokens
    }

    pub fn total_adaptive_routing_retained_tokens(&self) -> usize {
        self.routing_evidence.retained_tokens
    }

    pub fn total_adaptive_routing_saved_tokens(&self) -> usize {
        self.routing_evidence.saved_tokens
    }

    pub fn total_adaptive_routing_failures(&self) -> usize {
        self.routing_evidence.failures.len()
    }

    pub fn task_hierarchy_cases(&self) -> usize {
        self.routing_evidence.task_hierarchy_cases
    }

    pub fn task_hierarchy_mode_count(&self) -> usize {
        self.routing_evidence.task_hierarchy_mode_count()
    }

    pub fn total_task_hierarchy_mutation_records(&self) -> usize {
        self.routing_evidence.task_hierarchy_mutation_records
    }

    pub fn total_task_hierarchy_route_pressure_milli(&self) -> usize {
        self.routing_evidence.task_hierarchy_route_pressure_milli
    }

    pub fn total_task_hierarchy_compute_reduction_milli(&self) -> usize {
        self.routing_evidence.task_hierarchy_compute_reduction_milli
    }

    pub fn compute_budget_cases(&self) -> usize {
        self.routing_evidence.compute_budget_cases
    }

    pub fn total_compute_budget_low_value_skipped(&self) -> usize {
        self.routing_evidence.compute_budget_low_value_skipped
    }

    pub fn total_compute_budget_kv_lookups_skipped(&self) -> usize {
        self.routing_evidence.compute_budget_kv_lookups_skipped
    }

    pub fn total_compute_budget_validation_cost_tokens(&self) -> usize {
        self.routing_evidence.compute_budget_validation_cost_tokens
    }

    pub fn total_compute_budget_saved_tokens(&self) -> usize {
        self.routing_evidence.compute_budget_saved_tokens
    }

    pub fn total_compute_budget_self_evolving_memory_fusion_saved_tokens(&self) -> usize {
        self.routing_evidence
            .compute_budget_self_evolving_memory_fusion_saved_tokens
    }

    pub fn total_compute_budget_avoided_tokens(&self) -> usize {
        self.routing_evidence.compute_budget_avoided_tokens
    }

    pub fn total_compute_budget_fanout_before(&self) -> usize {
        self.routing_evidence.compute_budget_fanout_before
    }

    pub fn total_compute_budget_fanout_after(&self) -> usize {
        self.routing_evidence.compute_budget_fanout_after
    }

    pub fn total_compute_budget_fanout_reduction(&self) -> usize {
        self.routing_evidence.compute_budget_fanout_reduction
    }
}
