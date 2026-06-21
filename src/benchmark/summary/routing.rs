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
}
