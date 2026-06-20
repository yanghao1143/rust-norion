#[derive(Debug, Clone, PartialEq)]
pub struct MemoryRecord {
    pub id: u64,
    pub namespace: String,
    pub vector: Vec<f32>,
    pub strength: f32,
    pub hits: u64,
    pub failures: u64,
    pub last_score: f32,
    pub created_at: u64,
    pub last_access: u64,
}

impl MemoryRecord {
    pub fn new(id: u64, namespace: impl Into<String>, vector: Vec<f32>) -> Self {
        Self {
            id,
            namespace: namespace.into(),
            vector,
            strength: 1.0,
            hits: 0,
            failures: 0,
            last_score: 0.0,
            created_at: 0,
            last_access: 0,
        }
    }

    pub fn with_strength(mut self, strength: f32) -> Self {
        self.strength = strength.clamp(0.0, 3.0);
        self
    }

    pub fn with_feedback(mut self, hits: u64, failures: u64, last_score: f32) -> Self {
        self.hits = hits;
        self.failures = failures;
        self.last_score = last_score;
        self
    }

    pub fn with_timestamps(mut self, created_at: u64, last_access: u64) -> Self {
        self.created_at = created_at;
        self.last_access = last_access;
        self
    }

    pub fn reliability(&self) -> f32 {
        let attempts = self.hits.saturating_add(self.failures);
        if attempts == 0 {
            0.5
        } else {
            self.hits as f32 / attempts as f32
        }
    }

    pub fn summary(&self) -> MemoryRecordSummary {
        MemoryRecordSummary {
            id: self.id,
            namespace: self.namespace.clone(),
            vector_len: self.vector.len(),
            strength: self.strength,
            reliability: self.reliability(),
            attempts: self.hits.saturating_add(self.failures),
            has_failures: self.failures > 0,
            has_non_finite_values: !self.vector.iter().all(|value| value.is_finite()),
            age_span: self.last_access.saturating_sub(self.created_at),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryRecordSummary {
    pub id: u64,
    pub namespace: String,
    pub vector_len: usize,
    pub strength: f32,
    pub reliability: f32,
    pub attempts: u64,
    pub has_failures: bool,
    pub has_non_finite_values: bool,
    pub age_span: u64,
}

impl MemoryRecordSummary {
    pub fn is_empty_vector(&self) -> bool {
        self.vector_len == 0
    }

    pub fn has_feedback(&self) -> bool {
        self.attempts > 0
    }

    pub fn has_finite_values(&self) -> bool {
        !self.has_non_finite_values
    }

    pub fn is_reliable(&self, threshold: f32) -> bool {
        self.reliability >= threshold.clamp(0.0, 1.0)
    }

    pub fn is_failure_heavy(&self) -> bool {
        self.has_feedback() && self.has_failures && self.reliability <= 0.5
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MemoryRetentionPolicy {
    pub stale_after: u64,
    pub decay_rate: f32,
    pub remove_below_strength: f32,
    pub remove_after_failures: u64,
}

impl Default for MemoryRetentionPolicy {
    fn default() -> Self {
        Self {
            stale_after: 64,
            decay_rate: 0.04,
            remove_below_strength: 0.04,
            remove_after_failures: 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MemoryRetentionDecision {
    pub id: u64,
    pub strength_before: f32,
    pub strength_after: f32,
    pub removed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RetentionReport {
    pub before: usize,
    pub after: usize,
    pub decayed: usize,
    pub removed: Vec<u64>,
    pub decisions: Vec<MemoryRetentionDecision>,
}

impl RetentionReport {
    pub fn skipped(current_len: usize) -> Self {
        Self {
            before: current_len,
            after: current_len,
            decayed: 0,
            removed: Vec::new(),
            decisions: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MemoryCompactionPolicy {
    pub similarity_threshold: f32,
    pub max_candidates: usize,
    pub max_merges: usize,
}

impl Default for MemoryCompactionPolicy {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.92,
            max_candidates: 512,
            max_merges: 32,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MemoryCompactionMerge {
    pub primary_id: u64,
    pub removed_id: u64,
    pub similarity: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryCompactionReport {
    pub before: usize,
    pub after: usize,
    pub merged: Vec<MemoryCompactionMerge>,
    pub removed: Vec<u64>,
}

impl MemoryCompactionReport {
    pub fn skipped(current_len: usize) -> Self {
        Self {
            before: current_len,
            after: current_len,
            merged: Vec::new(),
            removed: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryUpdateAction {
    Reinforce,
    Penalize,
}

impl MemoryUpdateAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Reinforce => "reinforce",
            Self::Penalize => "penalize",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MemoryUpdateReport {
    pub id: u64,
    pub action: MemoryUpdateAction,
    pub requested_amount: f32,
    pub strength_before: Option<f32>,
    pub strength_after: Option<f32>,
    pub strength_delta: f32,
    pub removed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MemoryUpdateSummary {
    pub id: u64,
    pub action: MemoryUpdateAction,
    pub requested_amount: f32,
    pub applied: bool,
    pub removed: bool,
    pub strength_delta: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryUpdateCommitAction {
    CommitMemoryUpdate,
    CommitMissingMemoryUpdate,
    RepairMemoryUpdate,
}

impl MemoryUpdateCommitAction {
    pub fn can_commit(self) -> bool {
        matches!(
            self,
            Self::CommitMemoryUpdate | Self::CommitMissingMemoryUpdate
        )
    }

    pub fn is_missing(self) -> bool {
        matches!(self, Self::CommitMissingMemoryUpdate)
    }

    pub fn should_repair(self) -> bool {
        matches!(self, Self::RepairMemoryUpdate)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MemoryUpdateBatchSummary {
    pub report_count: usize,
    pub applied_count: usize,
    pub missing_count: usize,
    pub reinforced_count: usize,
    pub penalized_count: usize,
    pub removed_count: usize,
    pub positive_delta_count: usize,
    pub negative_delta_count: usize,
    pub requested_amount_total: f32,
    pub net_strength_delta: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryUpdateBatchCommitAction {
    CommitMemoryUpdateBatch,
    CommitMemoryUpdateBatchNoop,
    RepairMemoryUpdateBatch,
}

impl MemoryUpdateBatchCommitAction {
    pub fn can_commit(self) -> bool {
        matches!(
            self,
            Self::CommitMemoryUpdateBatch | Self::CommitMemoryUpdateBatchNoop
        )
    }

    pub fn is_noop(self) -> bool {
        matches!(self, Self::CommitMemoryUpdateBatchNoop)
    }

    pub fn should_repair(self) -> bool {
        matches!(self, Self::RepairMemoryUpdateBatch)
    }
}

impl MemoryUpdateReport {
    pub fn missing(id: u64, action: MemoryUpdateAction, requested_amount: f32) -> Self {
        Self {
            id,
            action,
            requested_amount,
            strength_before: None,
            strength_after: None,
            strength_delta: 0.0,
            removed: false,
        }
    }

    pub fn applied(
        id: u64,
        action: MemoryUpdateAction,
        requested_amount: f32,
        strength_before: f32,
        strength_after: f32,
        removed: bool,
    ) -> Self {
        Self {
            id,
            action,
            requested_amount,
            strength_before: Some(strength_before),
            strength_after: Some(strength_after),
            strength_delta: strength_after - strength_before,
            removed,
        }
    }

    pub fn was_applied(self) -> bool {
        self.strength_before.is_some()
    }

    pub fn update_summary(self) -> MemoryUpdateSummary {
        MemoryUpdateSummary {
            id: self.id,
            action: self.action,
            requested_amount: self.requested_amount,
            applied: self.was_applied(),
            removed: self.removed,
            strength_delta: self.strength_delta,
        }
    }

    pub fn batch_summary(reports: &[MemoryUpdateReport]) -> MemoryUpdateBatchSummary {
        MemoryUpdateBatchSummary::from_reports(reports)
    }
}

impl MemoryUpdateSummary {
    pub fn is_missing(self) -> bool {
        !self.applied
    }

    pub fn is_reinforce(self) -> bool {
        self.action == MemoryUpdateAction::Reinforce
    }

    pub fn is_penalize(self) -> bool {
        self.action == MemoryUpdateAction::Penalize
    }

    pub fn changed_strength(self) -> bool {
        self.strength_delta.abs() > f32::EPSILON
    }

    pub fn increased_strength(self) -> bool {
        self.strength_delta > f32::EPSILON
    }

    pub fn decreased_strength(self) -> bool {
        self.strength_delta < -f32::EPSILON
    }

    pub fn applied_removal(self) -> bool {
        self.applied && self.removed
    }

    pub fn requested_amount_shape_is_valid(self) -> bool {
        self.requested_amount.is_finite() && self.requested_amount >= 0.0
    }

    pub fn strength_delta_shape_is_valid(self) -> bool {
        self.strength_delta.is_finite()
    }

    pub fn removal_shape_is_valid(self) -> bool {
        !self.removed || self.applied
    }

    pub fn update_signal_component_count(self) -> usize {
        usize::from(self.applied)
            .saturating_add(usize::from(self.is_missing()))
            .saturating_add(usize::from(self.is_reinforce()))
            .saturating_add(usize::from(self.is_penalize()))
            .saturating_add(usize::from(self.changed_strength()))
            .saturating_add(usize::from(self.applied_removal()))
    }

    pub fn has_update_signals(self) -> bool {
        self.update_signal_component_count() > 0
    }

    pub fn update_problem_component_count(self) -> usize {
        usize::from(!self.requested_amount_shape_is_valid())
            .saturating_add(usize::from(!self.strength_delta_shape_is_valid()))
            .saturating_add(usize::from(!self.removal_shape_is_valid()))
    }

    pub fn has_update_problem_components(self) -> bool {
        self.update_problem_component_count() > 0
    }

    pub fn update_accounting_is_consistent(self) -> bool {
        let expected_signal_count = usize::from(self.applied)
            .saturating_add(usize::from(self.is_missing()))
            .saturating_add(usize::from(self.is_reinforce()))
            .saturating_add(usize::from(self.is_penalize()))
            .saturating_add(usize::from(self.changed_strength()))
            .saturating_add(usize::from(self.applied_removal()));
        let expected_problem_count = usize::from(!self.requested_amount_shape_is_valid())
            .saturating_add(usize::from(!self.strength_delta_shape_is_valid()))
            .saturating_add(usize::from(!self.removal_shape_is_valid()));

        self.update_signal_component_count() == expected_signal_count
            && self.update_problem_component_count() == expected_problem_count
            && self.has_update_problem_components() == (expected_problem_count > 0)
    }

    pub fn update_shape_is_clean(self) -> bool {
        !self.has_update_problem_components() && self.update_accounting_is_consistent()
    }

    pub fn memory_update_commit_signal_component_count(self) -> usize {
        self.update_signal_component_count()
    }

    pub fn has_memory_update_commit_signals(self) -> bool {
        self.memory_update_commit_signal_component_count() > 0
    }

    pub fn memory_update_commit_blocker_component_count(self) -> usize {
        self.update_problem_component_count()
    }

    pub fn has_memory_update_commit_blockers(self) -> bool {
        self.memory_update_commit_blocker_component_count() > 0
    }

    pub fn memory_update_commit_accounting_is_consistent(self) -> bool {
        self.update_accounting_is_consistent()
            && self.memory_update_commit_signal_component_count()
                == self.update_signal_component_count()
            && self.has_memory_update_commit_signals()
                == (self.memory_update_commit_signal_component_count() > 0)
            && self.memory_update_commit_blocker_component_count()
                == self.update_problem_component_count()
            && self.has_memory_update_commit_blockers()
                == (self.memory_update_commit_blocker_component_count() > 0)
    }

    pub fn memory_update_commit_is_clean(self) -> bool {
        !self.has_memory_update_commit_blockers()
            && self.memory_update_commit_accounting_is_consistent()
    }

    pub fn can_commit_memory_update(self) -> bool {
        self.memory_update_commit_is_clean()
    }

    pub fn memory_update_commit_action(self) -> MemoryUpdateCommitAction {
        if !self.can_commit_memory_update() {
            MemoryUpdateCommitAction::RepairMemoryUpdate
        } else if self.is_missing() {
            MemoryUpdateCommitAction::CommitMissingMemoryUpdate
        } else {
            MemoryUpdateCommitAction::CommitMemoryUpdate
        }
    }
}

impl MemoryUpdateBatchSummary {
    pub fn from_reports(reports: &[MemoryUpdateReport]) -> Self {
        let mut summary = Self {
            report_count: reports.len(),
            applied_count: 0,
            missing_count: 0,
            reinforced_count: 0,
            penalized_count: 0,
            removed_count: 0,
            positive_delta_count: 0,
            negative_delta_count: 0,
            requested_amount_total: 0.0,
            net_strength_delta: 0.0,
        };

        for report in reports {
            if report.was_applied() {
                summary.applied_count += 1;
            } else {
                summary.missing_count += 1;
            }

            match report.action {
                MemoryUpdateAction::Reinforce => summary.reinforced_count += 1,
                MemoryUpdateAction::Penalize => summary.penalized_count += 1,
            }

            if report.removed {
                summary.removed_count += 1;
            }
            if report.strength_delta > f32::EPSILON {
                summary.positive_delta_count += 1;
            } else if report.strength_delta < -f32::EPSILON {
                summary.negative_delta_count += 1;
            }

            summary.requested_amount_total += report.requested_amount;
            summary.net_strength_delta += report.strength_delta;
        }

        summary
    }

    pub fn is_empty(self) -> bool {
        self.report_count == 0
    }

    pub fn all_applied(self) -> bool {
        self.report_count > 0 && self.applied_count == self.report_count
    }

    pub fn has_missing(self) -> bool {
        self.missing_count > 0
    }

    pub fn has_removals(self) -> bool {
        self.removed_count > 0
    }

    pub fn has_strength_changes(self) -> bool {
        self.positive_delta_count > 0 || self.negative_delta_count > 0
    }

    pub fn counts_match_reports(self) -> bool {
        self.applied_count.saturating_add(self.missing_count) == self.report_count
            && self.reinforced_count.saturating_add(self.penalized_count) == self.report_count
            && self.removed_count <= self.applied_count
    }

    pub fn applied_missing_counts_match(self) -> bool {
        self.applied_count.saturating_add(self.missing_count) == self.report_count
    }

    pub fn action_counts_match(self) -> bool {
        self.reinforced_count.saturating_add(self.penalized_count) == self.report_count
    }

    pub fn removed_count_within_applied(self) -> bool {
        self.removed_count <= self.applied_count
    }

    pub fn delta_counts_within_applied(self) -> bool {
        self.positive_delta_count
            .saturating_add(self.negative_delta_count)
            <= self.applied_count
    }

    pub fn has_reinforcements(self) -> bool {
        self.reinforced_count > 0
    }

    pub fn has_penalties(self) -> bool {
        self.penalized_count > 0
    }

    pub fn has_mixed_actions(self) -> bool {
        self.has_reinforcements() && self.has_penalties()
    }

    pub fn net_positive(self) -> bool {
        self.net_strength_delta > f32::EPSILON
    }

    pub fn net_negative(self) -> bool {
        self.net_strength_delta < -f32::EPSILON
    }

    pub fn requested_amount_shape_is_valid(self) -> bool {
        self.requested_amount_total.is_finite() && self.requested_amount_total >= 0.0
    }

    pub fn net_strength_delta_shape_is_valid(self) -> bool {
        self.net_strength_delta.is_finite()
    }

    pub fn update_batch_signal_component_count(self) -> usize {
        usize::from(!self.is_empty())
            .saturating_add(usize::from(self.all_applied()))
            .saturating_add(usize::from(self.has_missing()))
            .saturating_add(usize::from(self.has_removals()))
            .saturating_add(usize::from(self.has_reinforcements()))
            .saturating_add(usize::from(self.has_penalties()))
            .saturating_add(usize::from(self.has_mixed_actions()))
            .saturating_add(usize::from(self.has_strength_changes()))
            .saturating_add(usize::from(self.net_positive()))
            .saturating_add(usize::from(self.net_negative()))
    }

    pub fn has_update_batch_signals(self) -> bool {
        self.update_batch_signal_component_count() > 0
    }

    pub fn update_batch_count_problem_component_count(self) -> usize {
        usize::from(!self.applied_missing_counts_match())
            .saturating_add(usize::from(!self.action_counts_match()))
            .saturating_add(usize::from(!self.removed_count_within_applied()))
            .saturating_add(usize::from(!self.delta_counts_within_applied()))
    }

    pub fn update_batch_shape_problem_component_count(self) -> usize {
        usize::from(!self.requested_amount_shape_is_valid())
            .saturating_add(usize::from(!self.net_strength_delta_shape_is_valid()))
    }

    pub fn update_batch_problem_component_count(self) -> usize {
        self.update_batch_count_problem_component_count()
            .saturating_add(self.update_batch_shape_problem_component_count())
    }

    pub fn has_update_batch_problem_components(self) -> bool {
        self.update_batch_problem_component_count() > 0
    }

    pub fn update_batch_accounting_is_consistent(self) -> bool {
        let expected_signal_count = usize::from(!self.is_empty())
            .saturating_add(usize::from(self.all_applied()))
            .saturating_add(usize::from(self.has_missing()))
            .saturating_add(usize::from(self.has_removals()))
            .saturating_add(usize::from(self.has_reinforcements()))
            .saturating_add(usize::from(self.has_penalties()))
            .saturating_add(usize::from(self.has_mixed_actions()))
            .saturating_add(usize::from(self.has_strength_changes()))
            .saturating_add(usize::from(self.net_positive()))
            .saturating_add(usize::from(self.net_negative()));
        let expected_problem_count = usize::from(!self.applied_missing_counts_match())
            .saturating_add(usize::from(!self.action_counts_match()))
            .saturating_add(usize::from(!self.removed_count_within_applied()))
            .saturating_add(usize::from(!self.delta_counts_within_applied()))
            .saturating_add(usize::from(!self.requested_amount_shape_is_valid()))
            .saturating_add(usize::from(!self.net_strength_delta_shape_is_valid()));

        self.update_batch_signal_component_count() == expected_signal_count
            && self.update_batch_problem_component_count() == expected_problem_count
            && self.has_update_batch_problem_components() == (expected_problem_count > 0)
    }

    pub fn update_batch_commit_is_clean(self) -> bool {
        self.counts_match_reports()
            && self.delta_counts_within_applied()
            && !self.has_update_batch_problem_components()
            && self.update_batch_accounting_is_consistent()
    }

    pub fn memory_update_batch_commit_signal_component_count(self) -> usize {
        self.update_batch_signal_component_count()
    }

    pub fn has_memory_update_batch_commit_signals(self) -> bool {
        self.memory_update_batch_commit_signal_component_count() > 0
    }

    pub fn memory_update_batch_commit_blocker_component_count(self) -> usize {
        self.update_batch_problem_component_count()
    }

    pub fn has_memory_update_batch_commit_blockers(self) -> bool {
        self.memory_update_batch_commit_blocker_component_count() > 0
    }

    pub fn memory_update_batch_commit_accounting_is_consistent(self) -> bool {
        self.update_batch_accounting_is_consistent()
            && self.memory_update_batch_commit_signal_component_count()
                == self.update_batch_signal_component_count()
            && self.has_memory_update_batch_commit_signals()
                == (self.memory_update_batch_commit_signal_component_count() > 0)
            && self.memory_update_batch_commit_blocker_component_count()
                == self.update_batch_problem_component_count()
            && self.has_memory_update_batch_commit_blockers()
                == (self.memory_update_batch_commit_blocker_component_count() > 0)
    }

    pub fn memory_update_batch_commit_is_clean(self) -> bool {
        self.counts_match_reports()
            && self.delta_counts_within_applied()
            && !self.has_memory_update_batch_commit_blockers()
            && self.memory_update_batch_commit_accounting_is_consistent()
    }

    pub fn can_commit_memory_update_batch(self) -> bool {
        self.memory_update_batch_commit_is_clean()
    }

    pub fn memory_update_batch_commit_action(self) -> MemoryUpdateBatchCommitAction {
        if !self.can_commit_memory_update_batch() {
            MemoryUpdateBatchCommitAction::RepairMemoryUpdateBatch
        } else if self.is_empty() {
            MemoryUpdateBatchCommitAction::CommitMemoryUpdateBatchNoop
        } else {
            MemoryUpdateBatchCommitAction::CommitMemoryUpdateBatch
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MemoryGovernancePolicy {
    pub retention: MemoryRetentionPolicy,
    pub compaction: MemoryCompactionPolicy,
}

impl Default for MemoryGovernancePolicy {
    fn default() -> Self {
        Self {
            retention: MemoryRetentionPolicy::default(),
            compaction: MemoryCompactionPolicy::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryGovernanceReport {
    pub retention: RetentionReport,
    pub compaction: MemoryCompactionReport,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryGovernanceSummary {
    pub retention_before: usize,
    pub retention_after: usize,
    pub retention_decayed_count: usize,
    pub retention_removed_count: usize,
    pub compaction_before: usize,
    pub compaction_after: usize,
    pub compaction_merged_count: usize,
    pub compaction_removed_count: usize,
    pub total_removed_count: usize,
    pub note_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryGovernanceCommitAction {
    CommitMemoryGovernanceChanges,
    CommitMemoryGovernanceNoop,
    RepairMemoryGovernance,
}

impl MemoryGovernanceCommitAction {
    pub fn can_commit(self) -> bool {
        matches!(
            self,
            Self::CommitMemoryGovernanceChanges | Self::CommitMemoryGovernanceNoop
        )
    }

    pub fn is_noop(self) -> bool {
        matches!(self, Self::CommitMemoryGovernanceNoop)
    }

    pub fn should_repair(self) -> bool {
        matches!(self, Self::RepairMemoryGovernance)
    }
}

impl MemoryGovernanceReport {
    pub fn removed_ids(&self) -> Vec<u64> {
        let mut removed = self.retention.removed.clone();
        removed.extend(self.compaction.removed.iter().copied());
        removed.sort_unstable();
        removed.dedup();
        removed
    }

    pub fn total_removed(&self) -> usize {
        self.removed_ids().len()
    }

    pub fn is_noop(&self) -> bool {
        self.retention.removed.is_empty()
            && self.retention.decayed == 0
            && self.compaction.merged.is_empty()
    }

    pub fn governance_summary(&self) -> MemoryGovernanceSummary {
        MemoryGovernanceSummary {
            retention_before: self.retention.before,
            retention_after: self.retention.after,
            retention_decayed_count: self.retention.decayed,
            retention_removed_count: self.retention.removed.len(),
            compaction_before: self.compaction.before,
            compaction_after: self.compaction.after,
            compaction_merged_count: self.compaction.merged.len(),
            compaction_removed_count: self.compaction.removed.len(),
            total_removed_count: self.total_removed(),
            note_count: self.notes.len(),
        }
    }
}

impl MemoryGovernanceSummary {
    pub fn has_retention_changes(self) -> bool {
        self.retention_decayed_count > 0 || self.retention_removed_count > 0
    }

    pub fn has_compaction_changes(self) -> bool {
        self.compaction_merged_count > 0 || self.compaction_removed_count > 0
    }

    pub fn has_any_changes(self) -> bool {
        self.has_retention_changes() || self.has_compaction_changes()
    }

    pub fn is_noop(self) -> bool {
        !self.has_any_changes()
    }

    pub fn final_record_count(self) -> usize {
        self.compaction_after
    }

    pub fn retention_count_balanced(self) -> bool {
        self.retention_after
            .saturating_add(self.retention_removed_count)
            == self.retention_before
    }

    pub fn compaction_count_balanced(self) -> bool {
        self.compaction_after
            .saturating_add(self.compaction_removed_count)
            == self.compaction_before
    }

    pub fn pipeline_count_balanced(self) -> bool {
        self.retention_after == self.compaction_before
    }

    pub fn total_removed_matches_phases(self) -> bool {
        self.total_removed_count
            == self
                .retention_removed_count
                .saturating_add(self.compaction_removed_count)
    }

    pub fn has_notes(self) -> bool {
        self.note_count > 0
    }

    pub fn notes_match_reportable_changes(self) -> bool {
        let expected = usize::from(self.retention_removed_count > 0)
            + usize::from(self.compaction_merged_count > 0);
        self.note_count == expected
    }

    pub fn retention_change_signal_component_count(self) -> usize {
        usize::from(self.has_retention_changes())
    }

    pub fn compaction_change_signal_component_count(self) -> usize {
        usize::from(self.has_compaction_changes())
    }

    pub fn governance_note_signal_component_count(self) -> usize {
        usize::from(self.has_notes())
    }

    pub fn governance_signal_component_count(self) -> usize {
        self.retention_change_signal_component_count()
            .saturating_add(self.compaction_change_signal_component_count())
    }

    pub fn has_governance_signals(self) -> bool {
        self.governance_signal_component_count() > 0
    }

    pub fn governance_problem_component_count(self) -> usize {
        usize::from(!self.retention_count_balanced())
            .saturating_add(usize::from(!self.compaction_count_balanced()))
            .saturating_add(usize::from(!self.pipeline_count_balanced()))
            .saturating_add(usize::from(!self.total_removed_matches_phases()))
            .saturating_add(usize::from(!self.notes_match_reportable_changes()))
    }

    pub fn has_governance_problem_components(self) -> bool {
        self.governance_problem_component_count() > 0
    }

    pub fn governance_accounting_is_consistent(self) -> bool {
        self.governance_problem_component_count() == 0 && !self.has_governance_problem_components()
    }

    pub fn governance_commit_signal_component_count(self) -> usize {
        self.governance_signal_component_count()
            .saturating_add(self.governance_note_signal_component_count())
    }

    pub fn has_governance_commit_signals(self) -> bool {
        self.governance_commit_signal_component_count() > 0
    }

    pub fn governance_commit_blocker_component_count(self) -> usize {
        self.governance_problem_component_count()
    }

    pub fn has_governance_commit_blockers(self) -> bool {
        self.governance_commit_blocker_component_count() > 0
    }

    pub fn governance_commit_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .governance_signal_component_count()
            .saturating_add(self.governance_note_signal_component_count());
        let expected_blocker_count = self.governance_problem_component_count();

        self.governance_commit_signal_component_count() == expected_signal_count
            && self.has_governance_commit_signals() == (expected_signal_count > 0)
            && self.governance_commit_blocker_component_count() == expected_blocker_count
            && self.has_governance_commit_blockers() == (expected_blocker_count > 0)
    }

    pub fn governance_commit_is_clean(self) -> bool {
        self.governance_accounting_is_consistent()
            && !self.has_governance_commit_blockers()
            && self.governance_commit_accounting_is_consistent()
    }

    pub fn can_commit_memory_governance(self) -> bool {
        self.governance_commit_is_clean()
    }

    pub fn memory_governance_commit_action(self) -> MemoryGovernanceCommitAction {
        if !self.can_commit_memory_governance() {
            MemoryGovernanceCommitAction::RepairMemoryGovernance
        } else if self.is_clean_noop() {
            MemoryGovernanceCommitAction::CommitMemoryGovernanceNoop
        } else {
            MemoryGovernanceCommitAction::CommitMemoryGovernanceChanges
        }
    }

    pub fn is_clean_noop(self) -> bool {
        self.is_noop()
            && !self.has_notes()
            && self.retention_count_balanced()
            && self.compaction_count_balanced()
            && self.pipeline_count_balanced()
            && self.total_removed_matches_phases()
    }
}

pub fn preview_retention(
    records: &[MemoryRecord],
    policy: MemoryRetentionPolicy,
    now: u64,
) -> RetentionReport {
    if records.is_empty() {
        return RetentionReport::skipped(0);
    }

    let stale_after = policy.stale_after.max(1);
    let decay_rate = policy.decay_rate.clamp(0.0, 0.95);
    let remove_below_strength = policy.remove_below_strength.clamp(0.0, 3.0);
    let remove_after_failures = policy.remove_after_failures.max(1);
    let mut decisions = Vec::with_capacity(records.len());
    let mut decayed = 0;
    let mut removed = Vec::new();

    for record in records {
        let idle = now.saturating_sub(record.last_access);
        let mut strength_after = record.strength.clamp(0.0, 3.0);
        if idle > stale_after {
            let periods = (idle - stale_after) as f32 / stale_after as f32;
            let decay = (decay_rate * periods.max(1.0)).clamp(0.0, 0.95);
            strength_after = (strength_after * (1.0 - decay)).clamp(0.0, 3.0);
        }
        if strength_after < record.strength {
            decayed += 1;
        }

        let weak_and_stale = strength_after <= remove_below_strength
            && idle > stale_after
            && record.failures >= record.hits;
        let repeatedly_failed = record.failures >= remove_after_failures && record.hits == 0;
        let remove = weak_and_stale || repeatedly_failed;
        if remove {
            removed.push(record.id);
        }

        decisions.push(MemoryRetentionDecision {
            id: record.id,
            strength_before: record.strength,
            strength_after,
            removed: remove,
        });
    }

    RetentionReport {
        before: records.len(),
        after: records.len().saturating_sub(removed.len()),
        decayed,
        removed,
        decisions,
    }
}

pub fn plan_compaction(
    records: &[MemoryRecord],
    policy: MemoryCompactionPolicy,
    protected_ids: &[u64],
    now: u64,
) -> MemoryCompactionReport {
    let before = records.len();
    if before < 2 || policy.max_merges == 0 || policy.max_candidates < 2 {
        return MemoryCompactionReport::skipped(before);
    }

    let threshold = policy.similarity_threshold.clamp(0.10, 0.999);
    let mut candidates = records
        .iter()
        .map(|record| (record.id, memory_value_score(record, now)))
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .1
            .total_cmp(&left.1)
            .then_with(|| left.0.cmp(&right.0))
    });
    candidates.truncate(policy.max_candidates.min(candidates.len()));

    let candidate_ids = candidates.into_iter().map(|(id, _)| id).collect::<Vec<_>>();
    let mut removed = Vec::<u64>::new();
    let mut merges = Vec::new();

    'outer: for left_pos in 0..candidate_ids.len() {
        for right_pos in (left_pos + 1)..candidate_ids.len() {
            if merges.len() >= policy.max_merges {
                break 'outer;
            }

            let left_id = candidate_ids[left_pos];
            let right_id = candidate_ids[right_pos];
            if removed.contains(&left_id) || removed.contains(&right_id) {
                continue;
            }

            let Some(left) = records.iter().find(|record| record.id == left_id) else {
                continue;
            };
            let Some(right) = records.iter().find(|record| record.id == right_id) else {
                continue;
            };
            if left.namespace != right.namespace {
                continue;
            }

            let similarity = cosine_similarity(&left.vector, &right.vector);
            if similarity < threshold {
                continue;
            }

            let Some((primary_id, removed_id)) =
                choose_compaction_pair(left, right, protected_ids, now)
            else {
                continue;
            };

            removed.push(removed_id);
            merges.push(MemoryCompactionMerge {
                primary_id,
                removed_id,
                similarity,
            });
        }
    }

    removed.sort_unstable();
    removed.dedup();

    MemoryCompactionReport {
        before,
        after: before.saturating_sub(removed.len()),
        merged: merges,
        removed,
    }
}

pub fn plan_memory_governance(
    records: &[MemoryRecord],
    policy: MemoryGovernancePolicy,
    protected_ids: &[u64],
    now: u64,
) -> MemoryGovernanceReport {
    let retention = preview_retention(records, policy.retention, now);
    let retained = records
        .iter()
        .filter(|record| !retention.removed.contains(&record.id))
        .cloned()
        .collect::<Vec<_>>();
    let compaction = plan_compaction(&retained, policy.compaction, protected_ids, now);
    let mut notes = Vec::new();
    if !retention.removed.is_empty() {
        notes.push(format!("retention:removed={}", retention.removed.len()));
    }
    if !compaction.merged.is_empty() {
        notes.push(format!("compaction:merged={}", compaction.merged.len()));
    }

    MemoryGovernanceReport {
        retention,
        compaction,
        notes,
    }
}

fn choose_compaction_pair(
    left: &MemoryRecord,
    right: &MemoryRecord,
    protected_ids: &[u64],
    now: u64,
) -> Option<(u64, u64)> {
    let left_protected = protected_ids.contains(&left.id);
    let right_protected = protected_ids.contains(&right.id);
    if left_protected && right_protected {
        return None;
    }
    if left_protected {
        return Some((left.id, right.id));
    }
    if right_protected {
        return Some((right.id, left.id));
    }

    let left_score = memory_value_score(left, now);
    let right_score = memory_value_score(right, now);
    if left_score > right_score {
        Some((left.id, right.id))
    } else if right_score > left_score {
        Some((right.id, left.id))
    } else if left.id <= right.id {
        Some((left.id, right.id))
    } else {
        Some((right.id, left.id))
    }
}

fn memory_value_score(record: &MemoryRecord, now: u64) -> f32 {
    let idle = now.saturating_sub(record.last_access) as f32;
    let recency = 1.0 / (1.0 + idle / 64.0);
    (record.strength * 0.42
        + record.last_score.max(0.0) * 0.22
        + record.reliability() * 0.24
        + recency * 0.12)
        .clamp(0.0, 3.0)
}

fn cosine_similarity(left: &[f32], right: &[f32]) -> f32 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }

    let len = left.len().max(right.len());
    let mut dot = 0.0;
    let mut left_norm = 0.0;
    let mut right_norm = 0.0;

    for index in 0..len {
        let left_value = left.get(index).copied().unwrap_or(0.0);
        let right_value = right.get(index).copied().unwrap_or(0.0);
        dot += left_value * right_value;
        left_norm += left_value * left_value;
        right_norm += right_value * right_value;
    }

    if left_norm <= f32::EPSILON || right_norm <= f32::EPSILON {
        0.0
    } else {
        (dot / (left_norm.sqrt() * right_norm.sqrt())).clamp(-1.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retention_preview_decays_and_removes_stale_failed_records() {
        let records = vec![
            MemoryRecord::new(1, "semantic", vec![1.0, 0.0])
                .with_strength(0.50)
                .with_feedback(2, 0, 0.8)
                .with_timestamps(0, 90),
            MemoryRecord::new(2, "semantic", vec![0.0, 1.0])
                .with_strength(0.03)
                .with_feedback(0, 4, 0.1)
                .with_timestamps(0, 0),
        ];
        let policy = MemoryRetentionPolicy {
            stale_after: 32,
            decay_rate: 0.10,
            remove_below_strength: 0.05,
            remove_after_failures: 4,
        };

        let report = preview_retention(&records, policy, 128);

        assert_eq!(report.before, 2);
        assert_eq!(report.after, 1);
        assert_eq!(report.decayed, 2);
        assert_eq!(report.removed, vec![2]);
        assert!(report.decisions[0].strength_after < records[0].strength);
    }

    #[test]
    fn memory_record_summary_reports_shape_and_feedback() {
        let record = MemoryRecord::new(42, "agent:planner", vec![1.0, f32::NAN, 3.0])
            .with_strength(1.4)
            .with_feedback(3, 1, 0.8)
            .with_timestamps(10, 74);

        let summary = record.summary();

        assert_eq!(summary.id, 42);
        assert_eq!(summary.namespace, "agent:planner");
        assert_eq!(summary.vector_len, 3);
        assert_eq!(summary.strength, 1.4);
        assert_eq!(summary.reliability, 0.75);
        assert_eq!(summary.attempts, 4);
        assert!(summary.has_failures);
        assert!(summary.has_non_finite_values);
        assert_eq!(summary.age_span, 64);
        assert!(!summary.is_empty_vector());
        assert!(summary.has_feedback());
        assert!(!summary.has_finite_values());
        assert!(summary.is_reliable(0.70));
        assert!(!summary.is_reliable(0.90));
        assert!(!summary.is_failure_heavy());
    }

    #[test]
    fn memory_update_summaries_report_applied_missing_and_delta_state() {
        let reports = vec![
            MemoryUpdateReport::applied(1, MemoryUpdateAction::Reinforce, 0.5, 0.80, 0.89, false),
            MemoryUpdateReport::applied(2, MemoryUpdateAction::Penalize, 1.0, 0.12, 0.0, true),
            MemoryUpdateReport::missing(99, MemoryUpdateAction::Reinforce, 0.4),
        ];

        let applied = reports[0].update_summary();
        let removed = reports[1].update_summary();
        let missing = reports[2].update_summary();
        let batch = MemoryUpdateReport::batch_summary(&reports);

        assert_eq!(applied.id, 1);
        assert_eq!(applied.action, MemoryUpdateAction::Reinforce);
        assert!(applied.applied);
        assert!(!applied.is_missing());
        assert!(applied.is_reinforce());
        assert!(!applied.is_penalize());
        assert!(applied.changed_strength());
        assert!(applied.increased_strength());
        assert!(!applied.decreased_strength());
        assert!(!applied.removed);
        assert!(applied.requested_amount_shape_is_valid());
        assert!(applied.strength_delta_shape_is_valid());
        assert!(applied.removal_shape_is_valid());
        assert_eq!(applied.update_signal_component_count(), 3);
        assert!(applied.has_update_signals());
        assert_eq!(applied.update_problem_component_count(), 0);
        assert!(!applied.has_update_problem_components());
        assert!(applied.update_accounting_is_consistent());
        assert!(applied.update_shape_is_clean());
        assert_eq!(applied.memory_update_commit_signal_component_count(), 3);
        assert!(applied.has_memory_update_commit_signals());
        assert_eq!(applied.memory_update_commit_blocker_component_count(), 0);
        assert!(!applied.has_memory_update_commit_blockers());
        assert!(applied.memory_update_commit_accounting_is_consistent());
        assert!(applied.memory_update_commit_is_clean());
        assert!(applied.can_commit_memory_update());
        assert_eq!(
            applied.memory_update_commit_action(),
            MemoryUpdateCommitAction::CommitMemoryUpdate
        );
        assert!(applied.memory_update_commit_action().can_commit());
        assert!(!applied.memory_update_commit_action().is_missing());
        assert!(!applied.memory_update_commit_action().should_repair());
        assert_eq!(removed.action, MemoryUpdateAction::Penalize);
        assert!(removed.applied);
        assert!(removed.removed);
        assert!(removed.applied_removal());
        assert!(!removed.is_reinforce());
        assert!(removed.is_penalize());
        assert!(removed.strength_delta < 0.0);
        assert!(removed.decreased_strength());
        assert_eq!(removed.update_signal_component_count(), 4);
        assert!(removed.update_accounting_is_consistent());
        assert!(removed.update_shape_is_clean());
        assert_eq!(removed.memory_update_commit_signal_component_count(), 4);
        assert!(removed.has_memory_update_commit_signals());
        assert_eq!(removed.memory_update_commit_blocker_component_count(), 0);
        assert!(!removed.has_memory_update_commit_blockers());
        assert!(removed.memory_update_commit_accounting_is_consistent());
        assert!(removed.memory_update_commit_is_clean());
        assert!(removed.can_commit_memory_update());
        assert_eq!(
            removed.memory_update_commit_action(),
            MemoryUpdateCommitAction::CommitMemoryUpdate
        );
        assert!(removed.memory_update_commit_action().can_commit());
        assert!(!removed.memory_update_commit_action().is_missing());
        assert!(!removed.memory_update_commit_action().should_repair());
        assert!(!missing.applied);
        assert!(missing.is_missing());
        assert!(!missing.changed_strength());
        assert!(!missing.applied_removal());
        assert_eq!(missing.update_signal_component_count(), 2);
        assert!(missing.update_accounting_is_consistent());
        assert!(missing.update_shape_is_clean());
        assert_eq!(missing.memory_update_commit_signal_component_count(), 2);
        assert!(missing.has_memory_update_commit_signals());
        assert_eq!(missing.memory_update_commit_blocker_component_count(), 0);
        assert!(!missing.has_memory_update_commit_blockers());
        assert!(missing.memory_update_commit_accounting_is_consistent());
        assert!(missing.memory_update_commit_is_clean());
        assert!(missing.can_commit_memory_update());
        assert_eq!(
            missing.memory_update_commit_action(),
            MemoryUpdateCommitAction::CommitMissingMemoryUpdate
        );
        assert!(missing.memory_update_commit_action().can_commit());
        assert!(missing.memory_update_commit_action().is_missing());
        assert!(!missing.memory_update_commit_action().should_repair());
        assert_eq!(batch.report_count, 3);
        assert_eq!(batch.applied_count, 2);
        assert_eq!(batch.missing_count, 1);
        assert_eq!(batch.reinforced_count, 2);
        assert_eq!(batch.penalized_count, 1);
        assert_eq!(batch.removed_count, 1);
        assert_eq!(batch.positive_delta_count, 1);
        assert_eq!(batch.negative_delta_count, 1);
        assert!((batch.requested_amount_total - 1.9).abs() < 1e-6);
        assert!((batch.net_strength_delta + 0.03).abs() < 1e-6);
        assert!(!batch.is_empty());
        assert!(!batch.all_applied());
        assert!(batch.has_missing());
        assert!(batch.has_removals());
        assert!(batch.has_strength_changes());
        assert!(batch.counts_match_reports());
        assert!(batch.has_reinforcements());
        assert!(batch.has_penalties());
        assert!(batch.has_mixed_actions());
        assert!(!batch.net_positive());
        assert!(batch.net_negative());
        assert!(batch.applied_missing_counts_match());
        assert!(batch.action_counts_match());
        assert!(batch.removed_count_within_applied());
        assert!(batch.delta_counts_within_applied());
        assert!(batch.requested_amount_shape_is_valid());
        assert!(batch.net_strength_delta_shape_is_valid());
        assert_eq!(batch.update_batch_signal_component_count(), 8);
        assert!(batch.has_update_batch_signals());
        assert_eq!(batch.update_batch_count_problem_component_count(), 0);
        assert_eq!(batch.update_batch_shape_problem_component_count(), 0);
        assert_eq!(batch.update_batch_problem_component_count(), 0);
        assert!(!batch.has_update_batch_problem_components());
        assert!(batch.update_batch_accounting_is_consistent());
        assert!(batch.update_batch_commit_is_clean());
        assert_eq!(batch.memory_update_batch_commit_signal_component_count(), 8);
        assert!(batch.has_memory_update_batch_commit_signals());
        assert_eq!(
            batch.memory_update_batch_commit_blocker_component_count(),
            0
        );
        assert!(!batch.has_memory_update_batch_commit_blockers());
        assert!(batch.memory_update_batch_commit_accounting_is_consistent());
        assert!(batch.memory_update_batch_commit_is_clean());
        assert!(batch.can_commit_memory_update_batch());
        assert_eq!(
            batch.memory_update_batch_commit_action(),
            MemoryUpdateBatchCommitAction::CommitMemoryUpdateBatch
        );
        assert!(batch.memory_update_batch_commit_action().can_commit());
        assert!(!batch.memory_update_batch_commit_action().is_noop());
        assert!(!batch.memory_update_batch_commit_action().should_repair());
    }

    #[test]
    fn empty_memory_update_batch_summary_is_noop() {
        let batch = MemoryUpdateBatchSummary::from_reports(&[]);

        assert_eq!(batch.report_count, 0);
        assert_eq!(batch.applied_count, 0);
        assert_eq!(batch.missing_count, 0);
        assert!(batch.is_empty());
        assert!(!batch.all_applied());
        assert!(!batch.has_missing());
        assert!(!batch.has_removals());
        assert!(!batch.has_strength_changes());
        assert!(batch.counts_match_reports());
        assert!(!batch.has_reinforcements());
        assert!(!batch.has_penalties());
        assert!(!batch.has_mixed_actions());
        assert!(!batch.net_positive());
        assert!(!batch.net_negative());
        assert!(batch.applied_missing_counts_match());
        assert!(batch.action_counts_match());
        assert!(batch.removed_count_within_applied());
        assert!(batch.delta_counts_within_applied());
        assert!(batch.requested_amount_shape_is_valid());
        assert!(batch.net_strength_delta_shape_is_valid());
        assert_eq!(batch.update_batch_signal_component_count(), 0);
        assert!(!batch.has_update_batch_signals());
        assert_eq!(batch.update_batch_problem_component_count(), 0);
        assert!(!batch.has_update_batch_problem_components());
        assert!(batch.update_batch_accounting_is_consistent());
        assert!(batch.update_batch_commit_is_clean());
        assert_eq!(batch.memory_update_batch_commit_signal_component_count(), 0);
        assert!(!batch.has_memory_update_batch_commit_signals());
        assert_eq!(
            batch.memory_update_batch_commit_blocker_component_count(),
            0
        );
        assert!(!batch.has_memory_update_batch_commit_blockers());
        assert!(batch.memory_update_batch_commit_accounting_is_consistent());
        assert!(batch.memory_update_batch_commit_is_clean());
        assert!(batch.can_commit_memory_update_batch());
        assert_eq!(
            batch.memory_update_batch_commit_action(),
            MemoryUpdateBatchCommitAction::CommitMemoryUpdateBatchNoop
        );
        assert!(batch.memory_update_batch_commit_action().can_commit());
        assert!(batch.memory_update_batch_commit_action().is_noop());
        assert!(!batch.memory_update_batch_commit_action().should_repair());
    }

    #[test]
    fn memory_update_summaries_count_public_shape_drift() {
        let update = MemoryUpdateSummary {
            id: 9,
            action: MemoryUpdateAction::Penalize,
            requested_amount: -1.0,
            applied: false,
            removed: true,
            strength_delta: f32::NAN,
        };
        let batch = MemoryUpdateBatchSummary {
            report_count: 2,
            applied_count: 3,
            missing_count: 0,
            reinforced_count: 1,
            penalized_count: 0,
            removed_count: 4,
            positive_delta_count: 2,
            negative_delta_count: 2,
            requested_amount_total: f32::NAN,
            net_strength_delta: f32::INFINITY,
        };

        assert!(!update.requested_amount_shape_is_valid());
        assert!(!update.strength_delta_shape_is_valid());
        assert!(!update.removal_shape_is_valid());
        assert_eq!(update.update_signal_component_count(), 2);
        assert!(update.has_update_signals());
        assert_eq!(update.update_problem_component_count(), 3);
        assert!(update.has_update_problem_components());
        assert!(update.update_accounting_is_consistent());
        assert!(!update.update_shape_is_clean());
        assert_eq!(update.memory_update_commit_signal_component_count(), 2);
        assert!(update.has_memory_update_commit_signals());
        assert_eq!(update.memory_update_commit_blocker_component_count(), 3);
        assert!(update.has_memory_update_commit_blockers());
        assert!(update.memory_update_commit_accounting_is_consistent());
        assert!(!update.memory_update_commit_is_clean());
        assert!(!update.can_commit_memory_update());
        assert_eq!(
            update.memory_update_commit_action(),
            MemoryUpdateCommitAction::RepairMemoryUpdate
        );
        assert!(!update.memory_update_commit_action().can_commit());
        assert!(!update.memory_update_commit_action().is_missing());
        assert!(update.memory_update_commit_action().should_repair());

        assert!(!batch.counts_match_reports());
        assert!(!batch.applied_missing_counts_match());
        assert!(!batch.action_counts_match());
        assert!(!batch.removed_count_within_applied());
        assert!(!batch.delta_counts_within_applied());
        assert!(!batch.requested_amount_shape_is_valid());
        assert!(!batch.net_strength_delta_shape_is_valid());
        assert_eq!(batch.update_batch_signal_component_count(), 5);
        assert!(batch.has_update_batch_signals());
        assert_eq!(batch.update_batch_count_problem_component_count(), 4);
        assert_eq!(batch.update_batch_shape_problem_component_count(), 2);
        assert_eq!(batch.update_batch_problem_component_count(), 6);
        assert!(batch.has_update_batch_problem_components());
        assert!(batch.update_batch_accounting_is_consistent());
        assert!(!batch.update_batch_commit_is_clean());
        assert_eq!(batch.memory_update_batch_commit_signal_component_count(), 5);
        assert!(batch.has_memory_update_batch_commit_signals());
        assert_eq!(
            batch.memory_update_batch_commit_blocker_component_count(),
            6
        );
        assert!(batch.has_memory_update_batch_commit_blockers());
        assert!(batch.memory_update_batch_commit_accounting_is_consistent());
        assert!(!batch.memory_update_batch_commit_is_clean());
        assert!(!batch.can_commit_memory_update_batch());
        assert_eq!(
            batch.memory_update_batch_commit_action(),
            MemoryUpdateBatchCommitAction::RepairMemoryUpdateBatch
        );
        assert!(!batch.memory_update_batch_commit_action().can_commit());
        assert!(!batch.memory_update_batch_commit_action().is_noop());
        assert!(batch.memory_update_batch_commit_action().should_repair());
    }

    #[test]
    fn compaction_respects_namespace_and_protected_ids() {
        let records = vec![
            MemoryRecord::new(1, "semantic", vec![1.0, 0.0])
                .with_strength(1.0)
                .with_feedback(4, 0, 0.9)
                .with_timestamps(0, 10),
            MemoryRecord::new(2, "semantic", vec![0.99, 0.01])
                .with_strength(0.9)
                .with_feedback(1, 0, 0.8)
                .with_timestamps(0, 10),
            MemoryRecord::new(3, "agent:planner", vec![0.99, 0.01])
                .with_strength(0.9)
                .with_feedback(1, 0, 0.8)
                .with_timestamps(0, 10),
        ];

        let report = plan_compaction(
            &records,
            MemoryCompactionPolicy {
                similarity_threshold: 0.95,
                max_candidates: 8,
                max_merges: 4,
            },
            &[2],
            16,
        );

        assert_eq!(report.before, 3);
        assert_eq!(report.after, 2);
        assert_eq!(report.merged.len(), 1);
        assert_eq!(report.merged[0].primary_id, 2);
        assert_eq!(report.merged[0].removed_id, 1);
        assert_eq!(report.removed, vec![1]);
    }

    #[test]
    fn compaction_is_skipped_when_policy_disables_merges() {
        let records = vec![
            MemoryRecord::new(1, "semantic", vec![1.0]),
            MemoryRecord::new(2, "semantic", vec![1.0]),
        ];

        let report = plan_compaction(
            &records,
            MemoryCompactionPolicy {
                max_merges: 0,
                ..MemoryCompactionPolicy::default()
            },
            &[],
            0,
        );

        assert_eq!(report, MemoryCompactionReport::skipped(2));
    }

    #[test]
    fn governance_runs_retention_before_compaction() {
        let records = vec![
            MemoryRecord::new(1, "semantic", vec![1.0, 0.0])
                .with_strength(1.0)
                .with_feedback(3, 0, 0.8)
                .with_timestamps(0, 90),
            MemoryRecord::new(2, "semantic", vec![0.99, 0.01])
                .with_strength(0.9)
                .with_feedback(2, 0, 0.7)
                .with_timestamps(0, 90),
            MemoryRecord::new(3, "semantic", vec![0.0, 1.0])
                .with_strength(0.01)
                .with_feedback(0, 8, 0.0)
                .with_timestamps(0, 0),
        ];

        let report = plan_memory_governance(
            &records,
            MemoryGovernancePolicy {
                retention: MemoryRetentionPolicy {
                    stale_after: 16,
                    decay_rate: 0.05,
                    remove_below_strength: 0.05,
                    remove_after_failures: 4,
                },
                compaction: MemoryCompactionPolicy {
                    similarity_threshold: 0.95,
                    max_candidates: 8,
                    max_merges: 1,
                },
            },
            &[],
            128,
        );

        let summary = report.governance_summary();

        assert_eq!(report.retention.removed, vec![3]);
        assert_eq!(report.compaction.merged.len(), 1);
        assert_eq!(report.compaction.after, 1);
        assert_eq!(report.notes.len(), 2);
        assert_eq!(report.removed_ids(), vec![2, 3]);
        assert_eq!(report.total_removed(), 2);
        assert!(!report.is_noop());
        assert_eq!(summary.retention_before, 3);
        assert_eq!(summary.retention_after, 2);
        assert_eq!(summary.retention_decayed_count, report.retention.decayed);
        assert_eq!(summary.retention_decayed_count, 3);
        assert_eq!(summary.retention_removed_count, 1);
        assert_eq!(summary.compaction_before, 2);
        assert_eq!(summary.compaction_after, 1);
        assert_eq!(summary.compaction_merged_count, 1);
        assert_eq!(summary.compaction_removed_count, 1);
        assert_eq!(summary.total_removed_count, 2);
        assert_eq!(summary.note_count, 2);
        assert!(summary.has_retention_changes());
        assert!(summary.has_compaction_changes());
        assert!(summary.has_any_changes());
        assert!(!summary.is_noop());
        assert_eq!(summary.final_record_count(), 1);
        assert!(summary.retention_count_balanced());
        assert!(summary.compaction_count_balanced());
        assert!(summary.pipeline_count_balanced());
        assert!(summary.total_removed_matches_phases());
        assert!(summary.has_notes());
        assert!(summary.notes_match_reportable_changes());
        assert_eq!(summary.retention_change_signal_component_count(), 1);
        assert_eq!(summary.compaction_change_signal_component_count(), 1);
        assert_eq!(summary.governance_note_signal_component_count(), 1);
        assert_eq!(summary.governance_signal_component_count(), 2);
        assert!(summary.has_governance_signals());
        assert_eq!(summary.governance_problem_component_count(), 0);
        assert!(!summary.has_governance_problem_components());
        assert!(summary.governance_accounting_is_consistent());
        assert_eq!(summary.governance_commit_signal_component_count(), 3);
        assert!(summary.has_governance_commit_signals());
        assert_eq!(summary.governance_commit_blocker_component_count(), 0);
        assert!(!summary.has_governance_commit_blockers());
        assert!(summary.governance_commit_accounting_is_consistent());
        assert!(summary.governance_commit_is_clean());
        assert!(summary.can_commit_memory_governance());
        assert!(!summary.is_clean_noop());
        assert_eq!(
            summary.memory_governance_commit_action(),
            MemoryGovernanceCommitAction::CommitMemoryGovernanceChanges
        );
        assert!(summary.memory_governance_commit_action().can_commit());
        assert!(!summary.memory_governance_commit_action().is_noop());
        assert!(!summary.memory_governance_commit_action().should_repair());
    }

    #[test]
    fn governance_report_marks_noop_plans() {
        let records = vec![
            MemoryRecord::new(1, "semantic", vec![1.0, 0.0])
                .with_strength(1.0)
                .with_feedback(1, 0, 0.9)
                .with_timestamps(0, 15),
        ];

        let report = plan_memory_governance(
            &records,
            MemoryGovernancePolicy {
                retention: MemoryRetentionPolicy {
                    stale_after: 64,
                    decay_rate: 0.05,
                    remove_below_strength: 0.05,
                    remove_after_failures: 4,
                },
                compaction: MemoryCompactionPolicy {
                    similarity_threshold: 0.95,
                    max_candidates: 8,
                    max_merges: 1,
                },
            },
            &[],
            16,
        );

        let summary = report.governance_summary();

        assert!(report.is_noop());
        assert!(report.removed_ids().is_empty());
        assert_eq!(report.total_removed(), 0);
        assert_eq!(summary.retention_before, 1);
        assert_eq!(summary.retention_after, 1);
        assert_eq!(summary.retention_decayed_count, 0);
        assert_eq!(summary.retention_removed_count, 0);
        assert_eq!(summary.compaction_before, 1);
        assert_eq!(summary.compaction_after, 1);
        assert_eq!(summary.compaction_merged_count, 0);
        assert_eq!(summary.compaction_removed_count, 0);
        assert_eq!(summary.total_removed_count, 0);
        assert_eq!(summary.note_count, 0);
        assert!(!summary.has_retention_changes());
        assert!(!summary.has_compaction_changes());
        assert!(!summary.has_any_changes());
        assert!(summary.is_noop());
        assert_eq!(summary.final_record_count(), 1);
        assert!(summary.retention_count_balanced());
        assert!(summary.compaction_count_balanced());
        assert!(summary.pipeline_count_balanced());
        assert!(summary.total_removed_matches_phases());
        assert!(!summary.has_notes());
        assert!(summary.notes_match_reportable_changes());
        assert_eq!(summary.retention_change_signal_component_count(), 0);
        assert_eq!(summary.compaction_change_signal_component_count(), 0);
        assert_eq!(summary.governance_note_signal_component_count(), 0);
        assert_eq!(summary.governance_signal_component_count(), 0);
        assert!(!summary.has_governance_signals());
        assert_eq!(summary.governance_problem_component_count(), 0);
        assert!(!summary.has_governance_problem_components());
        assert!(summary.governance_accounting_is_consistent());
        assert_eq!(summary.governance_commit_signal_component_count(), 0);
        assert!(!summary.has_governance_commit_signals());
        assert_eq!(summary.governance_commit_blocker_component_count(), 0);
        assert!(!summary.has_governance_commit_blockers());
        assert!(summary.governance_commit_accounting_is_consistent());
        assert!(summary.governance_commit_is_clean());
        assert!(summary.can_commit_memory_governance());
        assert!(summary.is_clean_noop());
        assert_eq!(
            summary.memory_governance_commit_action(),
            MemoryGovernanceCommitAction::CommitMemoryGovernanceNoop
        );
        assert!(summary.memory_governance_commit_action().can_commit());
        assert!(summary.memory_governance_commit_action().is_noop());
        assert!(!summary.memory_governance_commit_action().should_repair());
    }

    #[test]
    fn governance_summary_reports_accounting_drift() {
        let summary = MemoryGovernanceSummary {
            retention_before: 3,
            retention_after: 3,
            retention_decayed_count: 1,
            retention_removed_count: 1,
            compaction_before: 1,
            compaction_after: 3,
            compaction_merged_count: 1,
            compaction_removed_count: 0,
            total_removed_count: 3,
            note_count: 0,
        };

        assert!(summary.has_retention_changes());
        assert!(summary.has_compaction_changes());
        assert_eq!(summary.governance_note_signal_component_count(), 0);
        assert_eq!(summary.governance_signal_component_count(), 2);
        assert!(summary.has_governance_signals());
        assert!(!summary.retention_count_balanced());
        assert!(!summary.compaction_count_balanced());
        assert!(!summary.pipeline_count_balanced());
        assert!(!summary.total_removed_matches_phases());
        assert!(!summary.notes_match_reportable_changes());
        assert_eq!(summary.governance_problem_component_count(), 5);
        assert!(summary.has_governance_problem_components());
        assert!(!summary.governance_accounting_is_consistent());
        assert_eq!(summary.governance_commit_signal_component_count(), 2);
        assert!(summary.has_governance_commit_signals());
        assert_eq!(summary.governance_commit_blocker_component_count(), 5);
        assert!(summary.has_governance_commit_blockers());
        assert!(summary.governance_commit_accounting_is_consistent());
        assert!(!summary.governance_commit_is_clean());
        assert!(!summary.can_commit_memory_governance());
        assert_eq!(
            summary.memory_governance_commit_action(),
            MemoryGovernanceCommitAction::RepairMemoryGovernance
        );
        assert!(!summary.memory_governance_commit_action().can_commit());
        assert!(!summary.memory_governance_commit_action().is_noop());
        assert!(summary.memory_governance_commit_action().should_repair());
    }
}
