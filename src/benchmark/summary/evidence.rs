use super::super::{
    BenchmarkEmbeddingEvidence, BenchmarkLiveEvolutionEvidence, BenchmarkMemoryGovernanceEvidence,
    BenchmarkReflectionEvidence, BenchmarkRuntimeDeviceExecutionEvidence,
};
use super::BenchmarkSummary;

impl BenchmarkSummary {
    pub fn reflection_evidence(&self) -> BenchmarkReflectionEvidence {
        self.reflection_evidence.clone()
    }

    pub fn live_evolution_evidence(&self) -> BenchmarkLiveEvolutionEvidence {
        self.live_evolution_evidence.clone()
    }

    pub fn memory_governance_evidence(&self) -> BenchmarkMemoryGovernanceEvidence {
        self.memory_governance_evidence.clone()
    }

    pub fn embedding_evidence(&self) -> BenchmarkEmbeddingEvidence {
        self.embedding_evidence.clone()
    }

    pub fn runtime_embedding_cases(&self) -> usize {
        self.embedding_evidence.runtime_cases
    }

    pub fn embedding_fallback_cases(&self) -> usize {
        self.embedding_evidence.fallback_cases
    }

    pub fn runtime_embedding_device_profiles(&self) -> usize {
        self.embedding_evidence.runtime_device_profiles()
    }

    pub fn total_runtime_embedding_calls(&self) -> usize {
        self.embedding_evidence.runtime_calls
    }

    pub fn total_fallback_embedding_calls(&self) -> usize {
        self.embedding_evidence.fallback_calls
    }

    pub fn total_embedding_evidence_failures(&self) -> usize {
        self.embedding_evidence.failures.len()
    }

    pub fn runtime_architecture_cases(&self) -> usize {
        self.runtime_architecture_evidence.cases
    }

    pub fn runtime_architecture_device_profiles(&self) -> usize {
        self.runtime_architecture_evidence.device_profiles()
    }

    pub fn runtime_device_execution_evidence(&self) -> BenchmarkRuntimeDeviceExecutionEvidence {
        self.runtime_device_execution_evidence.clone()
    }

    pub fn runtime_device_execution_cases(&self) -> usize {
        self.runtime_device_execution_evidence.cases
    }

    pub fn runtime_device_execution_matched_cases(&self) -> usize {
        self.runtime_device_execution_evidence.matched_cases
    }

    pub fn runtime_device_execution_device_profiles(&self) -> usize {
        self.runtime_device_execution_evidence.device_profiles()
    }

    pub fn runtime_kv_precision_cases(&self) -> usize {
        self.runtime_device_execution_evidence
            .runtime_kv_precision_cases
    }

    pub fn runtime_kv_precision_device_profiles(&self) -> usize {
        self.runtime_device_execution_evidence
            .runtime_kv_precision_device_profiles()
    }

    pub fn total_runtime_device_execution_violations(&self) -> usize {
        self.runtime_device_execution_evidence.failures.len()
    }
}
