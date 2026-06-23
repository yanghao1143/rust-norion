use super::super::{
    BenchmarkEmbeddingEvidence, BenchmarkGenomeEvidence, BenchmarkImprovementCorpusEvidence,
    BenchmarkLiveEvolutionEvidence, BenchmarkMemoryGovernanceEvidence, BenchmarkReflectionEvidence,
    BenchmarkRuntimeDeviceExecutionEvidence,
};
use super::BenchmarkSummary;
use crate::improvement_corpus::ImprovementCorpusReport;

impl BenchmarkSummary {
    pub fn reflection_evidence(&self) -> BenchmarkReflectionEvidence {
        self.reflection_evidence.clone()
    }

    pub fn live_evolution_evidence(&self) -> BenchmarkLiveEvolutionEvidence {
        self.live_evolution_evidence.clone()
    }

    pub fn genome_evidence(&self) -> BenchmarkGenomeEvidence {
        self.genome_evidence.clone()
    }

    pub fn record_improvement_corpus_report(&mut self, report: &ImprovementCorpusReport) {
        self.improvement_corpus_evidence.record_report(report);
    }

    pub fn improvement_corpus_evidence(&self) -> BenchmarkImprovementCorpusEvidence {
        self.improvement_corpus_evidence.clone()
    }

    pub fn improvement_corpus_reports(&self) -> usize {
        self.improvement_corpus_evidence.reports
    }

    pub fn improvement_corpus_episodes(&self) -> usize {
        self.improvement_corpus_evidence.episodes
    }

    pub fn improvement_corpus_active_adaptation(&self) -> usize {
        self.improvement_corpus_evidence.active_adaptation
    }

    pub fn improvement_corpus_compiler_passed(&self) -> u64 {
        self.improvement_corpus_evidence.compiler_passed
    }

    pub fn improvement_corpus_test_passed(&self) -> u64 {
        self.improvement_corpus_evidence.test_passed
    }

    pub fn improvement_corpus_benchmark_passed(&self) -> u64 {
        self.improvement_corpus_evidence.benchmark_passed
    }

    pub fn reasoning_genome_expression_cases(&self) -> usize {
        self.genome_evidence.expression_cases
    }

    pub fn reasoning_genome_expression_device_profiles(&self) -> usize {
        self.genome_evidence.expression_device_profiles()
    }

    pub fn reasoning_genome_splice_cases(&self) -> usize {
        self.genome_evidence.splice_cases
    }

    pub fn reasoning_genome_splice_device_profiles(&self) -> usize {
        self.genome_evidence.splice_device_profiles()
    }

    pub fn gene_scissors_proposal_cases(&self) -> usize {
        self.genome_evidence.gene_scissors_proposal_cases
    }

    pub fn gene_scissors_proposal_device_profiles(&self) -> usize {
        self.genome_evidence
            .gene_scissors_proposal_device_profiles()
    }

    pub fn total_reasoning_genome_repair_payloads(&self) -> usize {
        self.genome_evidence.total_repair_payloads
    }

    pub fn total_reasoning_genome_regeneration_payloads(&self) -> usize {
        self.genome_evidence.total_regeneration_payloads
    }

    pub fn mutation_repair_fixtures(&self) -> usize {
        self.genome_evidence.mutation_repair_fixtures
    }

    pub fn mutation_repair_fixture_kinds(&self) -> usize {
        self.genome_evidence.mutation_repair_fixture_kinds
    }

    pub fn mutation_repair_candidates(&self) -> usize {
        self.genome_evidence.mutation_repair_candidates
    }

    pub fn mutation_repair_review_packets(&self) -> usize {
        self.genome_evidence.mutation_repair_review_packets
    }

    pub fn malignant_gene_recovery_drills(&self) -> usize {
        self.genome_evidence.malignant_gene_recovery_drills
    }

    pub fn malignant_gene_quarantines(&self) -> usize {
        self.genome_evidence.malignant_gene_quarantines
    }

    pub fn malignant_gene_cut_candidates(&self) -> usize {
        self.genome_evidence.malignant_gene_cut_candidates
    }

    pub fn malignant_gene_regeneration_candidates(&self) -> usize {
        self.genome_evidence.malignant_gene_regeneration_candidates
    }

    pub fn malignant_gene_failed_replay(&self) -> usize {
        self.genome_evidence.malignant_gene_failed_replay
    }

    pub fn dna_evolution_reports(&self) -> usize {
        self.genome_evidence.dna_evolution_reports
    }

    pub fn dna_evolution_candidates(&self) -> usize {
        self.genome_evidence.dna_evolution_candidates
    }

    pub fn dna_evolution_candidate_previews(&self) -> usize {
        self.genome_evidence.dna_evolution_candidate_previews
    }

    pub fn dna_evolution_holds(&self) -> usize {
        self.genome_evidence.dna_evolution_holds
    }

    pub fn dna_evolution_rejects(&self) -> usize {
        self.genome_evidence.dna_evolution_rejects
    }

    pub fn dna_evolution_rollbacks(&self) -> usize {
        self.genome_evidence.dna_evolution_rollbacks
    }

    pub fn dna_evolution_activation_eligible(&self) -> usize {
        self.genome_evidence.dna_evolution_activation_eligible
    }

    pub fn dna_evolution_transaction_replays(&self) -> usize {
        self.genome_evidence.dna_evolution_transaction_replays
    }

    pub fn dna_evolution_replay_passed(&self) -> usize {
        self.genome_evidence.dna_evolution_replay_passed
    }

    pub fn dna_evolution_validation_passed(&self) -> usize {
        self.genome_evidence.dna_evolution_validation_passed
    }

    pub fn dna_evolution_fitness_delta_milli(&self) -> i64 {
        self.genome_evidence.dna_evolution_fitness_delta_milli
    }

    pub fn total_reasoning_genome_lifecycle_records(&self) -> usize {
        self.genome_evidence.total_lifecycle_records
    }

    pub fn total_reasoning_genome_tombstone_candidates(&self) -> usize {
        self.genome_evidence.total_lifecycle_tombstone_candidates
    }

    pub fn total_reasoning_genome_failures(&self) -> usize {
        self.genome_evidence.failures.len()
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
