use crate::drift::DriftSeverity;

use super::BenchmarkSummary;

impl BenchmarkSummary {
    pub fn memory_governance_cases(&self) -> usize {
        self.memory_governance_evidence.cases
    }

    pub fn memory_governance_device_profiles(&self) -> usize {
        self.memory_governance_evidence.device_profiles()
    }

    pub fn memory_admission_cases(&self) -> usize {
        self.memory_governance_evidence.memory_admission_cases
    }

    pub fn memory_admission_device_profiles(&self) -> usize {
        self.memory_governance_evidence
            .memory_admission_device_profiles()
    }

    pub fn total_memory_admission_candidates(&self) -> usize {
        self.memory_governance_evidence.memory_admission_candidates
    }

    pub fn total_memory_admission_ready(&self) -> usize {
        self.memory_governance_evidence.memory_admission_ready
    }

    pub fn total_memory_admission_blocked(&self) -> usize {
        self.memory_governance_evidence.memory_admission_blocked
    }

    pub fn total_memory_admission_admitted(&self) -> usize {
        self.memory_governance_evidence.memory_admission_admitted
    }

    pub fn total_memory_admission_hold(&self) -> usize {
        self.memory_governance_evidence.memory_admission_hold
    }

    pub fn total_memory_admission_reject(&self) -> usize {
        self.memory_governance_evidence.memory_admission_reject
    }

    pub fn total_memory_admission_quarantine(&self) -> usize {
        self.memory_governance_evidence.memory_admission_quarantine
    }

    pub fn total_memory_admission_review_packets(&self) -> usize {
        self.memory_governance_evidence
            .memory_admission_review_packets
    }

    pub fn total_memory_admission_ledger_records(&self) -> usize {
        self.memory_governance_evidence
            .memory_admission_ledger_records
    }

    pub fn total_memory_admission_ledger_authorized(&self) -> usize {
        self.memory_governance_evidence
            .memory_admission_ledger_authorized
    }

    pub fn total_memory_admission_ledger_applied(&self) -> usize {
        self.memory_governance_evidence
            .memory_admission_ledger_applied
    }

    pub fn total_memory_admission_ledger_preview_only(&self) -> usize {
        self.memory_governance_evidence
            .memory_admission_ledger_preview_only
    }

    pub fn total_memory_admission_ledger_held(&self) -> usize {
        self.memory_governance_evidence.memory_admission_ledger_held
    }

    pub fn total_memory_admission_ledger_rejected(&self) -> usize {
        self.memory_governance_evidence
            .memory_admission_ledger_rejected
    }

    pub fn total_memory_admission_ledger_duplicate(&self) -> usize {
        self.memory_governance_evidence
            .memory_admission_ledger_duplicate
    }

    pub fn total_memory_admission_ledger_decayed(&self) -> usize {
        self.memory_governance_evidence
            .memory_admission_ledger_decayed
    }

    pub fn total_memory_admission_ledger_merged(&self) -> usize {
        self.memory_governance_evidence
            .memory_admission_ledger_merged
    }

    pub fn total_memory_admission_ledger_rollback(&self) -> usize {
        self.memory_governance_evidence
            .memory_admission_ledger_rollback
    }

    pub fn kv_fusion_cases(&self) -> usize {
        self.memory_governance_evidence.kv_fusion_cases
    }

    pub fn total_kv_fusion_candidates(&self) -> usize {
        self.memory_governance_evidence.kv_fusion_candidates
    }

    pub fn total_kv_fusion_fused(&self) -> usize {
        self.memory_governance_evidence.kv_fusion_fused
    }

    pub fn total_kv_fusion_compressed(&self) -> usize {
        self.memory_governance_evidence.kv_fusion_compressed
    }

    pub fn total_kv_fusion_skipped(&self) -> usize {
        self.memory_governance_evidence.kv_fusion_skipped
    }

    pub fn total_kv_fusion_held(&self) -> usize {
        self.memory_governance_evidence.kv_fusion_held
    }

    pub fn total_kv_fusion_rejected(&self) -> usize {
        self.memory_governance_evidence.kv_fusion_rejected
    }

    pub fn total_kv_fusion_approval_blocked(&self) -> usize {
        self.memory_governance_evidence.kv_fusion_approval_blocked
    }

    pub fn total_kv_fusion_input_tokens(&self) -> usize {
        self.memory_governance_evidence.kv_fusion_input_tokens
    }

    pub fn total_kv_fusion_retained_tokens(&self) -> usize {
        self.memory_governance_evidence.kv_fusion_retained_tokens
    }

    pub fn total_kv_fusion_saved_tokens(&self) -> usize {
        self.memory_governance_evidence.kv_fusion_saved_tokens
    }

    pub fn total_memory_retention_decayed(&self) -> usize {
        self.memory_governance_evidence.total_retention_decayed
    }

    pub fn total_memory_retention_removed(&self) -> usize {
        self.memory_governance_evidence.total_retention_removed
    }

    pub fn total_memory_compaction_merged(&self) -> usize {
        self.memory_governance_evidence.total_compaction_merged
    }

    pub fn total_memory_compaction_removed(&self) -> usize {
        self.memory_governance_evidence.total_compaction_removed
    }

    pub fn total_memory_compaction_pair_evidence(&self) -> usize {
        self.memory_governance_evidence
            .total_compaction_pair_evidence
    }

    pub fn memory_storage_benchmark_samples(&self) -> usize {
        self.memory_governance_evidence.memory_storage_samples
    }

    pub fn total_memory_storage_entries_before(&self) -> usize {
        self.memory_governance_evidence
            .memory_storage_entries_before
    }

    pub fn total_memory_storage_entries_after(&self) -> usize {
        self.memory_governance_evidence.memory_storage_entries_after
    }

    pub fn total_memory_storage_entries_removed(&self) -> usize {
        self.memory_governance_evidence
            .memory_storage_entries_removed
    }

    pub fn total_memory_storage_reduction_entries(&self) -> usize {
        self.total_memory_storage_entries_before()
            .saturating_sub(self.total_memory_storage_entries_after())
    }

    pub fn memory_retrieval_latency_samples(&self) -> usize {
        self.memory_governance_evidence
            .memory_retrieval_latency_samples
    }

    pub fn total_memory_retrieval_latency_ms(&self) -> u128 {
        self.memory_governance_evidence
            .total_memory_retrieval_latency_ms
    }

    pub fn max_memory_retrieval_latency_ms(&self) -> u128 {
        self.memory_governance_evidence
            .max_memory_retrieval_latency_ms
    }

    pub fn average_memory_retrieval_latency_ms(&self) -> u128 {
        let samples = self.memory_retrieval_latency_samples() as u128;
        if samples == 0 {
            0
        } else {
            self.total_memory_retrieval_latency_ms() / samples
        }
    }

    pub fn memory_retained_usefulness_delta_milli(&self) -> i64 {
        self.memory_governance_evidence
            .memory_retained_usefulness_delta_milli
    }

    pub fn memory_retained_usefulness_abs_delta_milli(&self) -> usize {
        self.memory_governance_evidence
            .memory_retained_usefulness_abs_delta_milli
    }

    pub fn total_live_memory_feedback_reinforcements(&self) -> usize {
        self.reflection_evidence.live_memory_feedback_reinforcements
    }

    pub fn total_live_memory_feedback_penalties(&self) -> usize {
        self.reflection_evidence.live_memory_feedback_penalties
    }

    pub fn total_live_memory_feedback_updates(&self) -> usize {
        self.reflection_evidence.live_memory_feedback_updates()
    }

    pub fn total_live_memory_feedback_applied(&self) -> usize {
        self.reflection_evidence.live_memory_feedback_applied
    }

    pub fn total_live_memory_feedback_removed(&self) -> usize {
        self.reflection_evidence.live_memory_feedback_removed
    }

    pub fn total_live_memory_feedback_missing(&self) -> usize {
        self.reflection_evidence.live_memory_feedback_missing
    }

    pub fn total_live_memory_feedback_strength_delta(&self) -> f32 {
        self.reflection_evidence.live_memory_feedback_strength_delta
    }

    pub fn total_memory_feedback_evidence_failures(&self) -> usize {
        self.reflection_evidence.memory_feedback_evidence_failures()
    }

    pub fn total_stored_memories(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.stored_memories)
            .sum()
    }

    pub fn total_compacted_memories(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.compacted_memories)
            .sum()
    }

    pub fn sparse_skipped_cases(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.sparse_skipped > 0)
            .count()
    }

    pub fn total_sparse_skipped(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.sparse_skipped)
            .sum()
    }

    pub fn total_sparse_skipped_tokens(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.sparse_skipped_tokens)
            .sum()
    }

    pub fn drift_watches(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.drift_severity == DriftSeverity::Watch)
            .count()
    }

    pub fn drift_blocks(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.drift_severity == DriftSeverity::Block)
            .count()
    }

    pub fn drift_rollbacks(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.drift_severity == DriftSeverity::Rollback)
            .count()
    }
}
