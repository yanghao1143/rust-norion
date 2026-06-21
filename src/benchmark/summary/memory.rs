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
