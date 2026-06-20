use std::collections::HashSet;

use crate::engine::{RuntimeFailureBatchSummary, RuntimeFailureReport, RuntimeFailureSummary};
use crate::kv::{
    KvBlock, KvNamespaceCounts, RuntimeKvPersistenceFailureReturnReport,
    RuntimeKvPersistenceFailureReturnSource, RuntimeKvPersistenceFailureReturnSummary,
};

#[derive(Debug, Clone, PartialEq)]
pub struct KvFusionPair {
    pub retained_id: u64,
    pub merged_id: u64,
    pub similarity: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KvFusionMerge {
    pub before: usize,
    pub after: usize,
    pub blocks: Vec<KvBlock>,
    pub merged_pairs: Vec<KvFusionPair>,
    pub skipped: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KvFusionMergeSummary {
    pub before: usize,
    pub after: usize,
    pub merged_count: usize,
    pub skipped_count: usize,
    pub merge_fraction: f32,
    pub changed: bool,
    pub skipped_due_to_limit: bool,
    pub runtime_block_count: usize,
    pub non_runtime_block_count: usize,
    pub result_namespace_count: usize,
    pub namespace_counts: KvNamespaceCounts,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KvFusionCommitSummary {
    pub fusion: KvFusionMergeSummary,
    pub action: KvFusionCommitAction,
    pub can_commit: bool,
    pub should_return_failure: bool,
    pub failure_reports: Vec<RuntimeFailureReport>,
    pub primary_failure_report: Option<RuntimeFailureReport>,
    pub primary_failure_summary: Option<RuntimeFailureSummary>,
    pub failure_batch: RuntimeFailureBatchSummary,
    pub failure_report_count: usize,
    pub can_format_runtime_failures: bool,
    pub total_signal_component_count: usize,
    pub total_blocker_component_count: usize,
    pub component_accounting_consistent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KvFusionCommitAction {
    CommitKvFusionPersistence,
    ReturnRuntimeFailure,
}

impl KvFusionCommitAction {
    pub fn can_commit(self) -> bool {
        matches!(self, Self::CommitKvFusionPersistence)
    }

    pub fn should_return_failure(self) -> bool {
        matches!(self, Self::ReturnRuntimeFailure)
    }
}

impl KvFusionMergeSummary {
    pub fn is_noop(self) -> bool {
        !self.changed
            && self.before == self.after
            && self.merged_count == 0
            && self.skipped_count == 0
    }

    pub fn collapsed_blocks(self) -> bool {
        self.after < self.before
    }

    pub fn has_merges(self) -> bool {
        self.merged_count > 0
    }

    pub fn has_skips(self) -> bool {
        self.skipped_count > 0
    }

    pub fn skip_limit_flag_matches_skips(self) -> bool {
        self.skipped_due_to_limit == self.has_skips()
    }

    pub fn skip_limit_flag_drift_component_count(self) -> usize {
        usize::from(!self.skip_limit_flag_matches_skips())
    }

    pub fn changed_due_to_merges(self) -> bool {
        self.changed && self.has_merges()
    }

    pub fn changed_due_to_skips(self) -> bool {
        self.changed && self.has_skips()
    }

    pub fn merge_fraction_shape_is_valid(self) -> bool {
        finite_unit(self.merge_fraction)
            && float_close(
                self.merge_fraction,
                self.merged_count as f32 / self.before.max(1) as f32,
            )
    }

    pub fn merge_fraction_shape_problem_component_count(self) -> usize {
        usize::from(!self.merge_fraction_shape_is_valid())
    }

    pub fn block_accounting_balanced(self) -> bool {
        self.before
            == self
                .after
                .saturating_add(self.merged_count)
                .saturating_add(self.skipped_count)
    }

    pub fn block_accounting_drift_component_count(self) -> usize {
        usize::from(!self.block_accounting_balanced())
    }

    pub fn namespace_counts_match_results(self) -> bool {
        self.namespace_counts.total() == self.after
    }

    pub fn namespace_count_drift_component_count(self) -> usize {
        usize::from(!self.namespace_counts_match_results())
    }

    pub fn result_counts_match_blocks(self) -> bool {
        self.runtime_block_count
            .saturating_add(self.non_runtime_block_count)
            == self.after
    }

    pub fn result_count_drift_component_count(self) -> usize {
        usize::from(!self.result_counts_match_blocks())
    }

    pub fn result_namespace_count_matches_counts(self) -> bool {
        let active_groups = self.namespace_counts.active_namespace_count();
        if self.namespace_counts.custom > 1 {
            self.result_namespace_count >= active_groups
                && self.result_namespace_count <= self.after
        } else {
            self.result_namespace_count == active_groups
        }
    }

    pub fn result_namespace_count_drift_component_count(self) -> usize {
        usize::from(!self.result_namespace_count_matches_counts())
    }

    pub fn all_runtime_blocks(self) -> bool {
        self.after > 0 && self.non_runtime_block_count == 0
    }

    pub fn all_non_runtime_blocks(self) -> bool {
        self.after > 0 && self.runtime_block_count == 0
    }

    pub fn has_runtime_and_non_runtime_blocks(self) -> bool {
        self.runtime_block_count > 0 && self.non_runtime_block_count > 0
    }

    pub fn has_namespace_mix(self) -> bool {
        self.result_namespace_count > 1
    }

    pub fn namespace_mix_signal_component_count(self) -> usize {
        usize::from(self.has_namespace_mix())
    }

    pub fn runtime_namespace_mix_signal_component_count(self) -> usize {
        usize::from(self.has_runtime_and_non_runtime_blocks())
    }

    pub fn merge_signal_component_count(self) -> usize {
        usize::from(self.has_merges())
    }

    pub fn skip_signal_component_count(self) -> usize {
        usize::from(self.has_skips())
    }

    pub fn fusion_boundary_signal_component_count(self) -> usize {
        self.merge_signal_component_count()
            .saturating_add(self.skip_signal_component_count())
            .saturating_add(self.namespace_mix_signal_component_count())
            .saturating_add(self.runtime_namespace_mix_signal_component_count())
    }

    pub fn has_fusion_boundary_signals(self) -> bool {
        self.fusion_boundary_signal_component_count() > 0
    }

    pub fn result_namespace_boundary_signal_component_count(self) -> usize {
        self.namespace_counts
            .namespace_boundary_signal_component_count()
    }

    pub fn has_result_namespace_boundary_signals(self) -> bool {
        self.result_namespace_boundary_signal_component_count() > 0
    }

    pub fn fusion_accounting_drift_component_count(self) -> usize {
        self.block_accounting_drift_component_count()
            .saturating_add(self.namespace_count_drift_component_count())
            .saturating_add(self.result_count_drift_component_count())
            .saturating_add(self.result_namespace_count_drift_component_count())
            .saturating_add(self.skip_limit_flag_drift_component_count())
    }

    pub fn has_fusion_accounting_drift_components(self) -> bool {
        self.fusion_accounting_drift_component_count() > 0
    }

    pub fn fusion_accounting_is_consistent(self) -> bool {
        let expected_drift_count = usize::from(!self.block_accounting_balanced())
            .saturating_add(usize::from(!self.namespace_counts_match_results()))
            .saturating_add(usize::from(!self.result_counts_match_blocks()))
            .saturating_add(usize::from(!self.result_namespace_count_matches_counts()))
            .saturating_add(usize::from(!self.skip_limit_flag_matches_skips()));

        self.fusion_accounting_drift_component_count() == expected_drift_count
            && self.has_fusion_accounting_drift_components() == (expected_drift_count > 0)
            && self.has_clean_accounting() == (expected_drift_count == 0)
    }

    pub fn has_clean_accounting(self) -> bool {
        self.block_accounting_balanced()
            && self.namespace_counts_match_results()
            && self.result_counts_match_blocks()
            && self.result_namespace_count_matches_counts()
            && self.skip_limit_flag_matches_skips()
    }

    pub fn fusion_boundary_problem_component_count(self) -> usize {
        self.fusion_accounting_drift_component_count()
            .saturating_add(self.merge_fraction_shape_problem_component_count())
    }

    pub fn has_fusion_boundary_problem_components(self) -> bool {
        self.fusion_boundary_problem_component_count() > 0
    }

    pub fn fusion_boundary_is_consistent(self) -> bool {
        self.has_clean_accounting()
            && self.fusion_accounting_is_consistent()
            && self.merge_fraction_shape_is_valid()
    }

    pub fn fusion_boundary_shape_is_clean(self) -> bool {
        !self.has_fusion_boundary_problem_components() && self.fusion_boundary_is_consistent()
    }

    pub fn can_use_kv_fusion_merge(self) -> bool {
        self.before > 0 && self.fusion_boundary_shape_is_clean()
    }

    pub fn fusion_commit_signal_component_count(self) -> usize {
        self.fusion_boundary_signal_component_count()
            .saturating_add(self.result_namespace_boundary_signal_component_count())
    }

    pub fn has_fusion_commit_signals(self) -> bool {
        self.fusion_commit_signal_component_count() > 0
    }

    pub fn fusion_commit_blocker_component_count(self) -> usize {
        self.fusion_boundary_problem_component_count()
    }

    pub fn has_fusion_commit_blockers(self) -> bool {
        self.fusion_commit_blocker_component_count() > 0
    }

    pub fn fusion_commit_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .fusion_boundary_signal_component_count()
            .saturating_add(self.result_namespace_boundary_signal_component_count());
        let expected_blocker_count = self.fusion_boundary_problem_component_count();

        self.fusion_commit_signal_component_count() == expected_signal_count
            && self.has_fusion_commit_signals() == (expected_signal_count > 0)
            && self.fusion_commit_blocker_component_count() == expected_blocker_count
            && self.has_fusion_commit_blockers() == (expected_blocker_count > 0)
            && self.fusion_boundary_shape_is_clean() == (expected_blocker_count == 0)
    }

    pub fn fusion_commit_shape_is_clean(self) -> bool {
        !self.has_fusion_commit_blockers() && self.fusion_commit_accounting_is_consistent()
    }

    pub fn can_commit_kv_fusion_persistence(self) -> bool {
        self.before > 0 && self.fusion_commit_shape_is_clean()
    }

    pub fn kv_fusion_admission_signal_component_count(self) -> usize {
        self.fusion_commit_signal_component_count()
    }

    pub fn has_kv_fusion_admission_signals(self) -> bool {
        self.kv_fusion_admission_signal_component_count() > 0
    }

    pub fn empty_persistence_problem_component_count(self) -> usize {
        usize::from(self.before == 0)
    }

    pub fn component_accounting_drift_count(self) -> usize {
        usize::from(!self.fusion_commit_accounting_is_consistent())
    }

    pub fn fusion_persistence_problem_component_count(self) -> usize {
        self.fusion_commit_blocker_component_count()
            .saturating_add(self.empty_persistence_problem_component_count())
            .saturating_add(self.component_accounting_drift_count())
    }

    pub fn has_fusion_persistence_problem_components(self) -> bool {
        self.fusion_persistence_problem_component_count() > 0
    }

    pub fn kv_fusion_admission_blocker_component_count(self) -> usize {
        self.fusion_persistence_problem_component_count()
    }

    pub fn has_kv_fusion_admission_blockers(self) -> bool {
        self.kv_fusion_admission_blocker_component_count() > 0
    }

    pub fn kv_fusion_admission_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self.fusion_commit_signal_component_count();
        let expected_blocker_count = self
            .fusion_commit_blocker_component_count()
            .saturating_add(self.empty_persistence_problem_component_count())
            .saturating_add(self.component_accounting_drift_count());

        self.fusion_commit_accounting_is_consistent()
            && self.kv_fusion_admission_signal_component_count() == expected_signal_count
            && self.has_kv_fusion_admission_signals() == (expected_signal_count > 0)
            && self.kv_fusion_admission_blocker_component_count() == expected_blocker_count
            && self.has_kv_fusion_admission_blockers() == (expected_blocker_count > 0)
    }

    pub fn kv_fusion_admission_is_clean(self) -> bool {
        !self.has_kv_fusion_admission_blockers()
            && self.kv_fusion_admission_accounting_is_consistent()
    }

    pub fn can_admit_kv_fusion_persistence(self) -> bool {
        self.before > 0 && self.kv_fusion_admission_is_clean()
    }

    pub fn failure_report(self) -> Option<RuntimeFailureReport> {
        let component_count = self.fusion_persistence_problem_component_count();
        if component_count == 0 {
            None
        } else {
            Some(RuntimeFailureReport::contract_violation(format!(
                "kv fusion persistence failed: components={component_count}"
            )))
        }
    }

    pub fn failure_reports(self) -> Vec<RuntimeFailureReport> {
        self.failure_report().into_iter().collect()
    }

    pub fn failure_report_count(self) -> usize {
        usize::from(self.failure_report().is_some())
    }

    pub fn has_failure_reports(self) -> bool {
        self.failure_report_count() > 0
    }

    pub fn failure_batch_summary(self) -> RuntimeFailureBatchSummary {
        RuntimeFailureReport::batch_summary(&self.failure_reports())
    }

    pub fn can_format_runtime_failures(self) -> bool {
        self.failure_batch_summary().can_format_runtime_failures()
    }

    pub fn primary_failure_report(self) -> Option<RuntimeFailureReport> {
        self.failure_report()
    }

    pub fn primary_failure_summary(self) -> Option<RuntimeFailureSummary> {
        self.primary_failure_report()
            .map(|failure| failure.failure_summary())
    }

    pub fn commit_summary(self) -> KvFusionCommitSummary {
        KvFusionCommitSummary::new(self)
    }
}

impl KvFusionCommitSummary {
    pub fn new(fusion: KvFusionMergeSummary) -> Self {
        let failure_reports = fusion.failure_reports();
        let primary_failure_report = failure_reports.first().cloned();
        let primary_failure_summary = primary_failure_report
            .as_ref()
            .map(|failure| failure.failure_summary());
        let failure_batch = RuntimeFailureReport::batch_summary(&failure_reports);
        let can_commit = fusion.can_commit_kv_fusion_persistence();
        let failure_report_count = failure_reports.len();
        let can_format_runtime_failures = failure_batch.can_format_runtime_failures();
        let should_return_failure = !can_commit && failure_report_count > 0;
        let action = if can_commit {
            KvFusionCommitAction::CommitKvFusionPersistence
        } else {
            KvFusionCommitAction::ReturnRuntimeFailure
        };

        Self {
            fusion,
            action,
            can_commit,
            should_return_failure,
            failure_reports,
            primary_failure_report,
            primary_failure_summary,
            failure_batch,
            failure_report_count,
            can_format_runtime_failures,
            total_signal_component_count: fusion.fusion_commit_signal_component_count(),
            total_blocker_component_count: fusion.fusion_commit_blocker_component_count(),
            component_accounting_consistent: fusion.fusion_commit_accounting_is_consistent(),
        }
    }

    pub fn action_can_commit(&self) -> bool {
        self.action.can_commit()
    }

    pub fn action_should_return_failure(&self) -> bool {
        self.action.should_return_failure()
    }

    pub fn has_primary_failure_summary(&self) -> bool {
        self.primary_failure_summary.is_some()
    }

    pub fn failure_batch_shape_is_clean(&self) -> bool {
        self.failure_batch.failure_batch_shape_is_clean()
    }

    pub fn failure_return_summary(&self) -> RuntimeKvPersistenceFailureReturnSummary {
        RuntimeKvPersistenceFailureReturnSummary::new(
            RuntimeKvPersistenceFailureReturnSource::KvFusionPersistence,
            self.can_commit,
            self.should_return_failure,
            self.primary_failure_summary,
            self.failure_batch,
            self.failure_report_count,
            self.can_format_runtime_failures,
            self.total_blocker_component_count,
            self.commit_decision_accounting_is_consistent(),
        )
    }

    pub fn runtime_failure_return_report(&self) -> Option<RuntimeKvPersistenceFailureReturnReport> {
        if self.failure_return_summary().can_return_runtime_failure() {
            self.primary_failure_report.clone().map(|failure| {
                RuntimeKvPersistenceFailureReturnReport::new(
                    RuntimeKvPersistenceFailureReturnSource::KvFusionPersistence,
                    failure,
                    self.failure_batch,
                    self.failure_report_count,
                    self.can_format_runtime_failures,
                    self.total_blocker_component_count,
                )
            })
        } else {
            None
        }
    }

    pub fn commit_decision_accounting_is_consistent(&self) -> bool {
        self.can_commit == self.fusion.can_commit_kv_fusion_persistence()
            && self.should_return_failure == (!self.can_commit && self.failure_report_count > 0)
            && self.action_can_commit() == self.can_commit
            && self.action_should_return_failure() == self.should_return_failure
            && self.failure_report_count == self.failure_reports.len()
            && self.failure_report_count == self.fusion.failure_report_count()
            && self.primary_failure_report.as_ref() == self.failure_reports.first()
            && self.primary_failure_summary
                == self
                    .primary_failure_report
                    .as_ref()
                    .map(|failure| failure.failure_summary())
            && self.failure_batch == RuntimeFailureReport::batch_summary(&self.failure_reports)
            && self.can_format_runtime_failures == self.failure_batch.can_format_runtime_failures()
            && self.total_signal_component_count
                == self.fusion.fusion_commit_signal_component_count()
            && self.total_blocker_component_count
                == self.fusion.fusion_commit_blocker_component_count()
            && self.component_accounting_consistent
                == self.fusion.fusion_commit_accounting_is_consistent()
    }

    pub fn can_commit_kv_fusion_persistence(&self) -> bool {
        self.can_commit && self.commit_decision_accounting_is_consistent()
    }

    pub fn should_return_runtime_failure(&self) -> bool {
        self.should_return_failure && self.commit_decision_accounting_is_consistent()
    }

    pub fn kv_fusion_commit_admission_signal_component_count(&self) -> usize {
        self.total_signal_component_count
    }

    pub fn has_kv_fusion_commit_admission_signals(&self) -> bool {
        self.kv_fusion_commit_admission_signal_component_count() > 0
    }

    pub fn missing_commit_component_count(&self) -> usize {
        usize::from(!self.can_commit)
    }

    pub fn commit_decision_drift_component_count(&self) -> usize {
        usize::from(!self.commit_decision_accounting_is_consistent())
    }

    pub fn kv_fusion_commit_admission_blocker_component_count(&self) -> usize {
        self.total_blocker_component_count
            .saturating_add(self.missing_commit_component_count())
            .saturating_add(self.commit_decision_drift_component_count())
    }

    pub fn has_kv_fusion_commit_admission_blockers(&self) -> bool {
        self.kv_fusion_commit_admission_blocker_component_count() > 0
    }

    pub fn kv_fusion_commit_admission_accounting_is_consistent(&self) -> bool {
        let expected_signal_count = self.total_signal_component_count;
        let expected_blocker_count = self
            .total_blocker_component_count
            .saturating_add(usize::from(!self.can_commit))
            .saturating_add(usize::from(
                !self.commit_decision_accounting_is_consistent(),
            ));

        self.kv_fusion_commit_admission_signal_component_count() == expected_signal_count
            && self.has_kv_fusion_commit_admission_signals() == (expected_signal_count > 0)
            && self.missing_commit_component_count() == usize::from(!self.can_commit)
            && self.commit_decision_drift_component_count()
                == usize::from(!self.commit_decision_accounting_is_consistent())
            && self.kv_fusion_commit_admission_blocker_component_count() == expected_blocker_count
            && self.has_kv_fusion_commit_admission_blockers() == (expected_blocker_count > 0)
    }

    pub fn kv_fusion_commit_admission_is_clean(&self) -> bool {
        !self.has_kv_fusion_commit_admission_blockers()
            && self.kv_fusion_commit_admission_accounting_is_consistent()
    }

    pub fn can_admit_kv_fusion_commit(&self) -> bool {
        self.can_commit_kv_fusion_persistence() && self.kv_fusion_commit_admission_is_clean()
    }
}

impl KvFusionMerge {
    pub fn skipped(existing: &[KvBlock], incoming: &[KvBlock]) -> Self {
        let mut blocks = existing.to_vec();
        blocks.extend_from_slice(incoming);
        Self {
            before: existing.len() + incoming.len(),
            after: blocks.len(),
            blocks,
            merged_pairs: Vec::new(),
            skipped: 0,
        }
    }

    pub fn merged_count(&self) -> usize {
        self.merged_pairs.len()
    }

    pub fn changed(&self) -> bool {
        self.merged_count() > 0 || self.skipped > 0 || self.before != self.after
    }

    pub fn skipped_due_to_limit(&self) -> bool {
        self.skipped > 0
    }

    pub fn merge_fraction(&self) -> f32 {
        self.merged_count() as f32 / self.before.max(1) as f32
    }

    pub fn merge_summary(&self) -> KvFusionMergeSummary {
        let namespace_counts = KvNamespaceCounts::from_blocks(&self.blocks);
        let runtime_block_count = self
            .blocks
            .iter()
            .filter(|block| block.namespace.is_runtime_exchange())
            .count();
        let result_namespace_count = self
            .blocks
            .iter()
            .map(|block| &block.namespace)
            .collect::<HashSet<_>>()
            .len();

        KvFusionMergeSummary {
            before: self.before,
            after: self.after,
            merged_count: self.merged_count(),
            skipped_count: self.skipped,
            merge_fraction: self.merge_fraction(),
            changed: self.changed(),
            skipped_due_to_limit: self.skipped_due_to_limit(),
            runtime_block_count,
            non_runtime_block_count: self.blocks.len().saturating_sub(runtime_block_count),
            result_namespace_count,
            namespace_counts,
        }
    }
}

pub trait KvFusionPolicy {
    fn fuse(&self, existing: &[KvBlock], incoming: &[KvBlock]) -> KvFusionMerge;

    fn observe_reward(&mut self, _reward: f32) {}
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReinforcedKvFusionPolicy {
    pub similarity_threshold: f32,
    pub max_candidates: usize,
    pub reinforcement_gain: f32,
}

impl ReinforcedKvFusionPolicy {
    pub fn new(similarity_threshold: f32, max_candidates: usize) -> Self {
        Self {
            similarity_threshold: similarity_threshold.clamp(0.0, 1.0),
            max_candidates: max_candidates.max(1),
            ..Self::default()
        }
    }

    pub fn with_reinforcement_gain(mut self, reinforcement_gain: f32) -> Self {
        self.reinforcement_gain = reinforcement_gain.max(0.0);
        self
    }
}

impl Default for ReinforcedKvFusionPolicy {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.92,
            max_candidates: 64,
            reinforcement_gain: 0.12,
        }
    }
}

impl KvFusionPolicy for ReinforcedKvFusionPolicy {
    fn fuse(&self, existing: &[KvBlock], incoming: &[KvBlock]) -> KvFusionMerge {
        let before = existing.len() + incoming.len();
        let mut blocks = existing.to_vec();
        let mut merged_pairs = Vec::new();
        let mut skipped = incoming.len().saturating_sub(self.max_candidates);

        for incoming_block in incoming.iter().take(self.max_candidates) {
            if let Some((index, similarity)) = matching_block(
                &blocks,
                incoming_block,
                self.similarity_threshold,
                self.max_candidates,
            ) {
                let retained_id = blocks[index].id;
                merge_block(
                    &mut blocks[index],
                    incoming_block,
                    similarity,
                    self.reinforcement_gain,
                );
                merged_pairs.push(KvFusionPair {
                    retained_id,
                    merged_id: incoming_block.id,
                    similarity,
                });
            } else {
                blocks.push(incoming_block.clone());
            }
        }

        if self.max_candidates == 0 {
            skipped = incoming.len();
        }

        KvFusionMerge {
            before,
            after: blocks.len(),
            blocks,
            merged_pairs,
            skipped,
        }
    }

    fn observe_reward(&mut self, reward: f32) {
        let reward = reward.clamp(-1.0, 1.0);
        self.similarity_threshold = (self.similarity_threshold - reward * 0.02).clamp(0.75, 0.99);
    }
}

fn matching_block(
    blocks: &[KvBlock],
    incoming: &KvBlock,
    threshold: f32,
    max_candidates: usize,
) -> Option<(usize, f32)> {
    blocks
        .iter()
        .enumerate()
        .filter(|(_, block)| block.same_slot(incoming))
        .take(max_candidates)
        .filter_map(|(index, block)| {
            let similarity = if block.content_signature_eq(incoming) {
                1.0
            } else {
                kv_similarity(block, incoming)
            };

            if similarity >= threshold {
                Some((index, similarity))
            } else {
                None
            }
        })
        .max_by(|left, right| {
            left.1
                .total_cmp(&right.1)
                .then_with(|| right.0.cmp(&left.0))
        })
}

fn merge_block(primary: &mut KvBlock, duplicate: &KvBlock, similarity: f32, gain: f32) {
    let primary_weight = primary.merge_weight();
    let duplicate_weight = duplicate.merge_weight();
    fuse_vector(
        &mut primary.key,
        &duplicate.key,
        primary_weight,
        duplicate_weight,
    );
    fuse_vector(
        &mut primary.value,
        &duplicate.value,
        primary_weight,
        duplicate_weight,
    );

    let total = (primary_weight + duplicate_weight).max(0.001);
    primary.score = ((primary.score * primary_weight + duplicate.score * duplicate_weight) / total)
        .clamp(0.0, 1.0);
    primary.reinforcement =
        (primary.reinforcement + duplicate.reinforcement + gain * similarity.clamp(0.0, 1.0))
            .clamp(0.0, 4.0);
}

fn fuse_vector(
    existing: &mut Vec<f32>,
    incoming: &[f32],
    existing_weight: f32,
    incoming_weight: f32,
) {
    let len = existing.len().max(incoming.len());
    existing.resize(len, 0.0);
    let total = (existing_weight + incoming_weight).max(0.001);

    for (index, value) in existing.iter_mut().enumerate() {
        let current = *value * existing_weight;
        let next = incoming.get(index).copied().unwrap_or(0.0) * incoming_weight;
        *value = (current + next) / total;
    }
}

fn kv_similarity(left: &KvBlock, right: &KvBlock) -> f32 {
    let key_similarity = cosine_similarity(&left.key, &right.key);
    let value_similarity = cosine_similarity(&left.value, &right.value);

    if key_similarity <= 0.0 && value_similarity <= 0.0 {
        0.0
    } else {
        ((key_similarity.max(0.0) + value_similarity.max(0.0)) / 2.0).clamp(0.0, 1.0)
    }
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
        let raw = dot / (left_norm.sqrt() * right_norm.sqrt());
        (raw * dimension_compatibility(left, right)).clamp(-1.0, 1.0)
    }
}

fn dimension_compatibility(left: &[f32], right: &[f32]) -> f32 {
    if left.len() == right.len() {
        return 1.0;
    }

    let shorter = left.len().min(right.len()) as f32;
    let longer = left.len().max(right.len()) as f32;
    if shorter <= f32::EPSILON {
        0.0
    } else {
        (shorter / longer).powi(2)
    }
}

fn finite_unit(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn float_close(left: f32, right: f32) -> bool {
    (left - right).abs() <= 0.0001
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::experiment::ExperimentSwitches;
    use crate::kv::KvNamespace;

    #[test]
    fn reinforced_fusion_deduplicates_same_runtime_slot() {
        let existing = vec![
            KvBlock::new(
                1,
                KvNamespace::Runtime,
                2,
                3,
                8..16,
                vec![1.0, 0.0],
                vec![0.5, 0.0],
            )
            .with_score(0.80)
            .with_reinforcement(0.20),
        ];
        let incoming = vec![
            KvBlock::new(
                2,
                KvNamespace::Runtime,
                2,
                3,
                8..16,
                vec![0.99, 0.01],
                vec![0.49, 0.01],
            )
            .with_score(0.70),
            KvBlock::new(
                3,
                KvNamespace::Semantic,
                2,
                3,
                8..16,
                vec![0.99, 0.01],
                vec![0.49, 0.01],
            ),
        ];

        let report = ReinforcedKvFusionPolicy::default().fuse(&existing, &incoming);

        assert_eq!(report.before, 3);
        assert_eq!(report.after, 2);
        assert_eq!(report.merged_pairs.len(), 1);
        assert_eq!(report.merged_count(), 1);
        assert!(report.changed());
        assert!(!report.skipped_due_to_limit());
        assert!((report.merge_fraction() - (1.0 / 3.0)).abs() < 0.0001);
        assert_eq!(report.merged_pairs[0].retained_id, 1);
        assert_eq!(report.merged_pairs[0].merged_id, 2);
        assert!(report.blocks[0].reinforcement > existing[0].reinforcement);
        assert!(
            report
                .blocks
                .iter()
                .any(|block| block.namespace == KvNamespace::Semantic)
        );

        let summary = report.merge_summary();

        assert_eq!(summary.before, 3);
        assert_eq!(summary.after, 2);
        assert_eq!(summary.merged_count, 1);
        assert_eq!(summary.skipped_count, 0);
        assert!((summary.merge_fraction - (1.0 / 3.0)).abs() < 0.0001);
        assert!(summary.changed);
        assert!(!summary.skipped_due_to_limit);
        assert_eq!(summary.runtime_block_count, 1);
        assert_eq!(summary.non_runtime_block_count, 1);
        assert_eq!(summary.result_namespace_count, 2);
        assert_eq!(summary.namespace_counts.runtime, 1);
        assert_eq!(summary.namespace_counts.semantic, 1);
        assert_eq!(summary.namespace_counts.non_runtime_total(), 1);
        assert_eq!(summary.namespace_counts.active_namespace_count(), 2);
        assert!(!summary.is_noop());
        assert!(summary.has_merges());
        assert!(!summary.has_skips());
        assert!(summary.changed_due_to_merges());
        assert!(!summary.changed_due_to_skips());
        assert!(summary.collapsed_blocks());
        assert!(summary.merge_fraction_shape_is_valid());
        assert_eq!(summary.merge_fraction_shape_problem_component_count(), 0);
        assert!(summary.block_accounting_balanced());
        assert_eq!(summary.block_accounting_drift_component_count(), 0);
        assert!(summary.namespace_counts_match_results());
        assert_eq!(summary.namespace_count_drift_component_count(), 0);
        assert!(summary.result_counts_match_blocks());
        assert_eq!(summary.result_count_drift_component_count(), 0);
        assert!(summary.result_namespace_count_matches_counts());
        assert_eq!(summary.result_namespace_count_drift_component_count(), 0);
        assert!(summary.has_clean_accounting());
        assert_eq!(summary.merge_signal_component_count(), 1);
        assert_eq!(summary.skip_signal_component_count(), 0);
        assert_eq!(summary.namespace_mix_signal_component_count(), 1);
        assert_eq!(summary.runtime_namespace_mix_signal_component_count(), 1);
        assert_eq!(summary.fusion_boundary_signal_component_count(), 3);
        assert!(summary.has_fusion_boundary_signals());
        assert_eq!(
            summary.result_namespace_boundary_signal_component_count(),
            4
        );
        assert!(summary.has_result_namespace_boundary_signals());
        assert_eq!(summary.fusion_accounting_drift_component_count(), 0);
        assert!(!summary.has_fusion_accounting_drift_components());
        assert!(summary.fusion_accounting_is_consistent());
        assert_eq!(summary.fusion_boundary_problem_component_count(), 0);
        assert!(!summary.has_fusion_boundary_problem_components());
        assert!(summary.fusion_boundary_is_consistent());
        assert!(summary.fusion_boundary_shape_is_clean());
        assert!(summary.can_use_kv_fusion_merge());
        assert_eq!(summary.fusion_commit_signal_component_count(), 7);
        assert!(summary.has_fusion_commit_signals());
        assert_eq!(summary.fusion_commit_blocker_component_count(), 0);
        assert!(!summary.has_fusion_commit_blockers());
        assert!(summary.fusion_commit_accounting_is_consistent());
        assert!(summary.fusion_commit_shape_is_clean());
        assert!(summary.can_commit_kv_fusion_persistence());
        assert_eq!(summary.empty_persistence_problem_component_count(), 0);
        assert_eq!(summary.component_accounting_drift_count(), 0);
        assert_eq!(summary.fusion_persistence_problem_component_count(), 0);
        assert!(!summary.has_fusion_persistence_problem_components());
        assert_eq!(summary.failure_report(), None);
        assert_eq!(summary.failure_reports(), Vec::new());
        assert_eq!(summary.failure_report_count(), 0);
        assert!(!summary.has_failure_reports());
        assert_eq!(summary.failure_batch_summary().total_count, 0);
        assert!(!summary.can_format_runtime_failures());
        assert_eq!(summary.primary_failure_report(), None);
        assert_eq!(summary.primary_failure_summary(), None);
        let commit = summary.commit_summary();
        assert_eq!(
            commit.action,
            KvFusionCommitAction::CommitKvFusionPersistence
        );
        assert!(commit.action_can_commit());
        assert!(!commit.action_should_return_failure());
        assert!(commit.can_commit_kv_fusion_persistence());
        assert!(!commit.should_return_runtime_failure());
        assert!(commit.failure_reports.is_empty());
        assert_eq!(commit.failure_report_count, 0);
        assert_eq!(commit.total_signal_component_count, 7);
        assert_eq!(commit.total_blocker_component_count, 0);
        assert!(commit.component_accounting_consistent);
        assert!(!commit.has_primary_failure_summary());
        assert!(commit.failure_batch_shape_is_clean());
        assert!(commit.commit_decision_accounting_is_consistent());
        let failure_return = commit.failure_return_summary();
        assert_eq!(
            failure_return.source,
            RuntimeKvPersistenceFailureReturnSource::KvFusionPersistence
        );
        assert_eq!(failure_return.source.label(), "kv_fusion_persistence");
        assert!(!failure_return.has_failure_reports());
        assert!(!failure_return.has_blocker_components());
        assert!(failure_return.failure_return_accounting_is_consistent());
        assert!(!failure_return.can_return_runtime_failure());
        assert_eq!(commit.runtime_failure_return_report(), None);
        assert!(!summary.all_runtime_blocks());
        assert!(!summary.all_non_runtime_blocks());
        assert!(summary.has_runtime_and_non_runtime_blocks());
        assert!(summary.has_namespace_mix());
    }

    #[test]
    fn kv_fusion_merge_path_admits_commit_summary() {
        let existing = vec![
            KvBlock::new(
                1,
                KvNamespace::Runtime,
                2,
                3,
                8..16,
                vec![1.0, 0.0],
                vec![0.5, 0.0],
            )
            .with_score(0.80)
            .with_reinforcement(0.20),
        ];
        let incoming = vec![
            KvBlock::new(
                2,
                KvNamespace::Runtime,
                2,
                3,
                8..16,
                vec![0.99, 0.01],
                vec![0.49, 0.01],
            )
            .with_score(0.70),
            KvBlock::new(
                3,
                KvNamespace::Semantic,
                2,
                3,
                8..16,
                vec![0.99, 0.01],
                vec![0.49, 0.01],
            ),
        ];

        let merge = ReinforcedKvFusionPolicy::default().fuse(&existing, &incoming);
        let summary = merge.merge_summary();
        let commit = summary.commit_summary();

        assert_eq!(merge.before, 3);
        assert_eq!(merge.after, 2);
        assert_eq!(summary.fusion_commit_signal_component_count(), 7);
        assert!(summary.can_admit_kv_fusion_persistence());
        assert_eq!(
            commit.action,
            KvFusionCommitAction::CommitKvFusionPersistence
        );
        assert_eq!(
            commit.kv_fusion_commit_admission_signal_component_count(),
            7
        );
        assert!(commit.has_kv_fusion_commit_admission_signals());
        assert_eq!(
            commit.kv_fusion_commit_admission_blocker_component_count(),
            0
        );
        assert!(!commit.has_kv_fusion_commit_admission_blockers());
        assert!(commit.kv_fusion_commit_admission_accounting_is_consistent());
        assert!(commit.kv_fusion_commit_admission_is_clean());
        assert!(commit.can_admit_kv_fusion_commit());
        assert!(!commit.should_return_runtime_failure());
    }

    #[test]
    fn fusion_does_not_merge_dissimilar_same_slot_blocks() {
        let existing = vec![KvBlock::new(
            1,
            KvNamespace::Runtime,
            0,
            0,
            0..4,
            vec![1.0, 0.0],
            vec![1.0, 0.0],
        )];
        let incoming = vec![KvBlock::new(
            2,
            KvNamespace::Runtime,
            0,
            0,
            0..4,
            vec![0.0, 1.0],
            vec![0.0, 1.0],
        )];

        let report = ReinforcedKvFusionPolicy::default().fuse(&existing, &incoming);

        assert_eq!(report.after, 2);
        assert!(report.merged_pairs.is_empty());
        assert_eq!(report.merged_count(), 0);
        assert!(!report.skipped_due_to_limit());
        assert!(!report.changed());

        let summary = report.merge_summary();

        assert_eq!(summary.before, 2);
        assert_eq!(summary.after, 2);
        assert_eq!(summary.merged_count, 0);
        assert!(!summary.changed);
        assert!(!summary.collapsed_blocks());
        assert_eq!(summary.namespace_counts.runtime, 2);
        assert_eq!(summary.namespace_counts.non_runtime_total(), 0);
        assert!(summary.is_noop());
        assert!(!summary.has_merges());
        assert!(!summary.has_skips());
        assert!(!summary.changed_due_to_merges());
        assert!(!summary.changed_due_to_skips());
        assert!(summary.merge_fraction_shape_is_valid());
        assert_eq!(summary.merge_fraction_shape_problem_component_count(), 0);
        assert!(summary.block_accounting_balanced());
        assert_eq!(summary.block_accounting_drift_component_count(), 0);
        assert!(summary.namespace_counts_match_results());
        assert_eq!(summary.namespace_count_drift_component_count(), 0);
        assert!(summary.result_counts_match_blocks());
        assert_eq!(summary.result_count_drift_component_count(), 0);
        assert!(summary.result_namespace_count_matches_counts());
        assert_eq!(summary.result_namespace_count_drift_component_count(), 0);
        assert!(summary.has_clean_accounting());
        assert_eq!(summary.merge_signal_component_count(), 0);
        assert_eq!(summary.skip_signal_component_count(), 0);
        assert_eq!(summary.namespace_mix_signal_component_count(), 0);
        assert_eq!(summary.runtime_namespace_mix_signal_component_count(), 0);
        assert_eq!(summary.fusion_boundary_signal_component_count(), 0);
        assert!(!summary.has_fusion_boundary_signals());
        assert_eq!(
            summary.result_namespace_boundary_signal_component_count(),
            1
        );
        assert!(summary.has_result_namespace_boundary_signals());
        assert_eq!(summary.fusion_accounting_drift_component_count(), 0);
        assert!(!summary.has_fusion_accounting_drift_components());
        assert!(summary.fusion_accounting_is_consistent());
        assert_eq!(summary.fusion_boundary_problem_component_count(), 0);
        assert!(!summary.has_fusion_boundary_problem_components());
        assert!(summary.fusion_boundary_is_consistent());
        assert!(summary.fusion_boundary_shape_is_clean());
        assert!(summary.can_use_kv_fusion_merge());
        assert_eq!(summary.fusion_commit_signal_component_count(), 1);
        assert!(summary.has_fusion_commit_signals());
        assert_eq!(summary.fusion_commit_blocker_component_count(), 0);
        assert!(!summary.has_fusion_commit_blockers());
        assert!(summary.fusion_commit_accounting_is_consistent());
        assert!(summary.fusion_commit_shape_is_clean());
        assert!(summary.can_commit_kv_fusion_persistence());
        assert!(summary.all_runtime_blocks());
        assert!(!summary.all_non_runtime_blocks());
        assert!(!summary.has_runtime_and_non_runtime_blocks());
        assert!(!summary.has_namespace_mix());
    }

    #[test]
    fn fusion_keeps_namespace_boundaries_even_for_identical_vectors() {
        let existing = vec![KvBlock::new(
            1,
            KvNamespace::Custom("runtime-a".to_owned()),
            1,
            2,
            16..32,
            vec![0.25, 0.75],
            vec![0.5, 0.5],
        )];
        let incoming = vec![KvBlock::new(
            2,
            KvNamespace::Custom("runtime-b".to_owned()),
            1,
            2,
            16..32,
            vec![0.25, 0.75],
            vec![0.5, 0.5],
        )];

        let report = ReinforcedKvFusionPolicy::default().fuse(&existing, &incoming);

        assert_eq!(report.before, 2);
        assert_eq!(report.after, 2);
        assert!(report.merged_pairs.is_empty());
        assert_eq!(report.merge_fraction(), 0.0);

        let summary = report.merge_summary();

        assert!(summary.is_noop());
        assert!(summary.block_accounting_balanced());
        assert!(summary.namespace_counts_match_results());
        assert_eq!(summary.namespace_counts.custom, 2);
        assert_eq!(summary.result_namespace_count, 2);
        assert!(summary.result_counts_match_blocks());
        assert!(summary.result_namespace_count_matches_counts());
        assert!(summary.has_clean_accounting());
        assert_eq!(summary.namespace_mix_signal_component_count(), 1);
        assert_eq!(summary.runtime_namespace_mix_signal_component_count(), 0);
        assert_eq!(
            summary.result_namespace_boundary_signal_component_count(),
            1
        );
        assert!(summary.has_result_namespace_boundary_signals());
        assert_eq!(summary.fusion_accounting_drift_component_count(), 0);
        assert!(!summary.has_fusion_accounting_drift_components());
        assert!(summary.fusion_accounting_is_consistent());
        assert!(summary.merge_fraction_shape_is_valid());
        assert_eq!(summary.merge_fraction_shape_problem_component_count(), 0);
        assert!(summary.fusion_boundary_shape_is_clean());
        assert!(summary.can_use_kv_fusion_merge());
        assert_eq!(summary.fusion_commit_signal_component_count(), 2);
        assert!(summary.has_fusion_commit_signals());
        assert!(summary.fusion_commit_accounting_is_consistent());
        assert!(summary.fusion_commit_shape_is_clean());
        assert!(summary.can_commit_kv_fusion_persistence());
        assert!(!summary.all_runtime_blocks());
        assert!(summary.all_non_runtime_blocks());
        assert!(summary.has_namespace_mix());
    }

    #[test]
    fn fusion_report_marks_candidate_limit_skips() {
        let existing = Vec::new();
        let incoming = vec![
            KvBlock::new(1, KvNamespace::Runtime, 0, 0, 0..1, vec![1.0], vec![1.0]),
            KvBlock::new(2, KvNamespace::Runtime, 0, 1, 0..1, vec![1.0], vec![1.0]),
            KvBlock::new(3, KvNamespace::Runtime, 0, 2, 0..1, vec![1.0], vec![1.0]),
        ];

        let report = ReinforcedKvFusionPolicy::new(0.92, 2).fuse(&existing, &incoming);

        assert_eq!(report.before, 3);
        assert_eq!(report.after, 2);
        assert_eq!(report.skipped, 1);
        assert!(report.changed());
        assert!(report.skipped_due_to_limit());

        let summary = report.merge_summary();

        assert_eq!(summary.skipped_count, 1);
        assert_eq!(summary.runtime_block_count, 2);
        assert_eq!(summary.non_runtime_block_count, 0);
        assert_eq!(summary.namespace_counts.runtime, 2);
        assert_eq!(summary.namespace_counts.total(), 2);
        assert!(summary.skipped_due_to_limit);
        assert!(!summary.is_noop());
        assert!(!summary.has_merges());
        assert!(summary.has_skips());
        assert!(!summary.changed_due_to_merges());
        assert!(summary.changed_due_to_skips());
        assert!(summary.block_accounting_balanced());
        assert!(summary.namespace_counts_match_results());
        assert!(summary.result_counts_match_blocks());
        assert!(summary.result_namespace_count_matches_counts());
        assert!(summary.has_clean_accounting());
        assert_eq!(summary.merge_signal_component_count(), 0);
        assert_eq!(summary.skip_signal_component_count(), 1);
        assert_eq!(summary.namespace_mix_signal_component_count(), 0);
        assert_eq!(summary.runtime_namespace_mix_signal_component_count(), 0);
        assert_eq!(summary.fusion_boundary_signal_component_count(), 1);
        assert!(summary.has_fusion_boundary_signals());
        assert_eq!(
            summary.result_namespace_boundary_signal_component_count(),
            1
        );
        assert!(summary.has_result_namespace_boundary_signals());
        assert_eq!(summary.fusion_accounting_drift_component_count(), 0);
        assert!(!summary.has_fusion_accounting_drift_components());
        assert!(summary.fusion_accounting_is_consistent());
        assert_eq!(summary.fusion_boundary_problem_component_count(), 0);
        assert!(!summary.has_fusion_boundary_problem_components());
        assert!(summary.fusion_boundary_is_consistent());
        assert!(summary.fusion_boundary_shape_is_clean());
        assert!(summary.can_use_kv_fusion_merge());
        assert_eq!(summary.fusion_commit_signal_component_count(), 2);
        assert!(summary.has_fusion_commit_signals());
        assert_eq!(summary.fusion_commit_blocker_component_count(), 0);
        assert!(!summary.has_fusion_commit_blockers());
        assert!(summary.fusion_commit_accounting_is_consistent());
        assert!(summary.fusion_commit_shape_is_clean());
        assert!(summary.can_commit_kv_fusion_persistence());
        assert!(summary.all_runtime_blocks());
        assert!(!summary.all_non_runtime_blocks());
        assert!(!summary.has_runtime_and_non_runtime_blocks());
    }

    #[test]
    fn fusion_candidate_budget_from_experiment_switches_is_visible_but_committable() {
        let switches = ExperimentSwitches {
            enable_reinforced_kv_fusion: true,
            max_kv_fusion_candidates: 2,
            ..ExperimentSwitches::default()
        };
        let switch_summary = switches.switches_summary();
        let existing = vec![KvBlock::new(
            1,
            KvNamespace::Runtime,
            0,
            0,
            0..4,
            vec![1.0, 0.0],
            vec![1.0, 0.0],
        )];
        let incoming = vec![
            KvBlock::new(
                2,
                KvNamespace::Runtime,
                0,
                0,
                0..4,
                vec![1.0, 0.0],
                vec![1.0, 0.0],
            ),
            KvBlock::new(
                3,
                KvNamespace::Runtime,
                0,
                1,
                4..8,
                vec![0.0, 1.0],
                vec![0.0, 1.0],
            ),
            KvBlock::new(
                4,
                KvNamespace::Runtime,
                0,
                2,
                8..12,
                vec![0.5, 0.5],
                vec![0.5, 0.5],
            ),
        ];

        let report = ReinforcedKvFusionPolicy::new(0.92, switches.max_kv_fusion_candidates)
            .fuse(&existing, &incoming);
        let summary = report.merge_summary();
        let commit = summary.commit_summary();

        assert!(switch_summary.reinforced_kv_fusion_enabled);
        assert!(switch_summary.kv_fusion_budget_is_conservative());
        assert_eq!(switch_summary.max_kv_fusion_candidates, 2);
        assert_eq!(report.before, 4);
        assert_eq!(report.after, 2);
        assert_eq!(report.merged_count(), 1);
        assert_eq!(report.skipped, 1);
        assert!(report.skipped_due_to_limit());
        assert_eq!(report.merged_pairs[0].retained_id, 1);
        assert_eq!(report.merged_pairs[0].merged_id, 2);
        assert_eq!(summary.skipped_count, 1);
        assert!(summary.skipped_due_to_limit);
        assert!(summary.has_merges());
        assert!(summary.has_skips());
        assert!(summary.changed_due_to_merges());
        assert!(summary.changed_due_to_skips());
        assert!(summary.block_accounting_balanced());
        assert!(summary.namespace_counts_match_results());
        assert_eq!(summary.runtime_block_count, 2);
        assert_eq!(summary.non_runtime_block_count, 0);
        assert_eq!(summary.fusion_boundary_problem_component_count(), 0);
        assert!(!summary.has_fusion_commit_blockers());
        assert!(summary.fusion_commit_accounting_is_consistent());
        assert!(summary.can_commit_kv_fusion_persistence());
        assert_eq!(
            commit.action,
            KvFusionCommitAction::CommitKvFusionPersistence
        );
        assert!(commit.can_commit_kv_fusion_persistence());
        assert!(!commit.should_return_runtime_failure());
        assert_eq!(commit.failure_report_count, 0);
    }

    #[test]
    fn fusion_candidate_budget_full_scan_commits_without_skip_pressure() {
        let switches = ExperimentSwitches {
            enable_reinforced_kv_fusion: true,
            max_kv_fusion_candidates: 4,
            ..ExperimentSwitches::default()
        };
        let switch_summary = switches.switches_summary();
        let existing = vec![KvBlock::new(
            1,
            KvNamespace::Runtime,
            0,
            0,
            0..4,
            vec![1.0, 0.0],
            vec![1.0, 0.0],
        )];
        let incoming = vec![
            KvBlock::new(
                2,
                KvNamespace::Runtime,
                0,
                0,
                0..4,
                vec![1.0, 0.0],
                vec![1.0, 0.0],
            ),
            KvBlock::new(
                3,
                KvNamespace::Runtime,
                0,
                1,
                4..8,
                vec![0.0, 1.0],
                vec![0.0, 1.0],
            ),
            KvBlock::new(
                4,
                KvNamespace::Runtime,
                0,
                2,
                8..12,
                vec![0.5, 0.5],
                vec![0.5, 0.5],
            ),
        ];

        let report = ReinforcedKvFusionPolicy::new(0.92, switches.max_kv_fusion_candidates)
            .fuse(&existing, &incoming);
        let summary = report.merge_summary();
        let commit = summary.commit_summary();

        assert!(switch_summary.reinforced_kv_fusion_enabled);
        assert!(switch_summary.kv_fusion_budget_is_conservative());
        assert_eq!(switch_summary.max_kv_fusion_candidates, 4);
        assert_eq!(report.before, 4);
        assert_eq!(report.after, 3);
        assert_eq!(report.merged_count(), 1);
        assert_eq!(report.skipped, 0);
        assert!(!report.skipped_due_to_limit());
        assert_eq!(report.merged_pairs[0].retained_id, 1);
        assert_eq!(report.merged_pairs[0].merged_id, 2);
        assert_eq!(summary.skipped_count, 0);
        assert!(!summary.skipped_due_to_limit);
        assert!(summary.has_merges());
        assert!(!summary.has_skips());
        assert!(summary.changed_due_to_merges());
        assert!(!summary.changed_due_to_skips());
        assert_eq!(summary.merge_signal_component_count(), 1);
        assert_eq!(summary.skip_signal_component_count(), 0);
        assert!(summary.block_accounting_balanced());
        assert!(summary.namespace_counts_match_results());
        assert!(summary.result_counts_match_blocks());
        assert!(summary.result_namespace_count_matches_counts());
        assert_eq!(summary.runtime_block_count, 3);
        assert_eq!(summary.non_runtime_block_count, 0);
        assert_eq!(summary.namespace_counts.runtime, 3);
        assert_eq!(summary.fusion_boundary_problem_component_count(), 0);
        assert!(summary.fusion_boundary_shape_is_clean());
        assert!(!summary.has_fusion_commit_blockers());
        assert!(summary.fusion_commit_accounting_is_consistent());
        assert!(summary.can_commit_kv_fusion_persistence());
        assert_eq!(
            commit.action,
            KvFusionCommitAction::CommitKvFusionPersistence
        );
        assert!(commit.can_commit_kv_fusion_persistence());
        assert!(!commit.should_return_runtime_failure());
        assert_eq!(commit.failure_report_count, 0);
    }

    #[test]
    fn fusion_public_zero_candidate_budget_clamps_to_one_before_persistence() {
        let switches = ExperimentSwitches {
            enable_reinforced_kv_fusion: true,
            max_kv_fusion_candidates: 0,
            ..ExperimentSwitches::default()
        };
        let switch_summary = switches.switches_summary();
        let incoming = vec![
            KvBlock::new(1, KvNamespace::Runtime, 0, 0, 0..4, vec![1.0], vec![1.0]),
            KvBlock::new(2, KvNamespace::Semantic, 0, 1, 4..8, vec![0.0], vec![0.0]),
        ];

        let policy = ReinforcedKvFusionPolicy::new(0.92, switches.max_kv_fusion_candidates);
        let report = policy.fuse(&[], &incoming);
        let summary = report.merge_summary();
        let commit = summary.commit_summary();

        assert!(switch_summary.reinforced_kv_fusion_enabled);
        assert!(switch_summary.kv_fusion_budget_is_conservative());
        assert_eq!(switch_summary.max_kv_fusion_candidates, 0);
        assert_eq!(policy.max_candidates, 1);
        assert_eq!(report.before, 2);
        assert_eq!(report.after, 1);
        assert_eq!(report.skipped, 1);
        assert!(report.skipped_due_to_limit());
        assert_eq!(summary.skipped_count, 1);
        assert!(summary.skipped_due_to_limit);
        assert_eq!(summary.runtime_block_count, 1);
        assert_eq!(summary.non_runtime_block_count, 0);
        assert_eq!(summary.namespace_counts.runtime, 1);
        assert_eq!(summary.namespace_counts.semantic, 0);
        assert!(summary.block_accounting_balanced());
        assert!(summary.namespace_counts_match_results());
        assert!(summary.result_counts_match_blocks());
        assert!(summary.result_namespace_count_matches_counts());
        assert!(summary.fusion_boundary_shape_is_clean());
        assert!(summary.can_commit_kv_fusion_persistence());
        assert_eq!(
            commit.action,
            KvFusionCommitAction::CommitKvFusionPersistence
        );
        assert!(commit.can_commit_kv_fusion_persistence());
        assert!(!commit.should_return_runtime_failure());
        assert_eq!(commit.failure_report_count, 0);
    }

    #[test]
    fn fusion_candidate_budget_skip_does_not_pollute_result_namespaces() {
        let existing = vec![KvBlock::new(
            1,
            KvNamespace::Runtime,
            0,
            0,
            0..4,
            vec![1.0, 0.0],
            vec![1.0, 0.0],
        )];
        let incoming = vec![
            KvBlock::new(
                2,
                KvNamespace::Runtime,
                0,
                0,
                0..4,
                vec![1.0, 0.0],
                vec![1.0, 0.0],
            ),
            KvBlock::new(
                3,
                KvNamespace::Semantic,
                0,
                1,
                4..8,
                vec![0.0, 1.0],
                vec![0.0, 1.0],
            ),
        ];

        let report = ReinforcedKvFusionPolicy::new(0.92, 1).fuse(&existing, &incoming);
        let summary = report.merge_summary();
        let commit = summary.commit_summary();

        assert_eq!(report.before, 3);
        assert_eq!(report.after, 1);
        assert_eq!(report.merged_count(), 1);
        assert_eq!(report.skipped, 1);
        assert!(report.skipped_due_to_limit());
        assert_eq!(report.merged_pairs[0].retained_id, 1);
        assert_eq!(report.merged_pairs[0].merged_id, 2);
        assert_eq!(summary.skipped_count, 1);
        assert!(summary.skipped_due_to_limit);
        assert!(summary.has_merges());
        assert!(summary.has_skips());
        assert!(summary.changed_due_to_merges());
        assert!(summary.changed_due_to_skips());
        assert_eq!(summary.runtime_block_count, 1);
        assert_eq!(summary.non_runtime_block_count, 0);
        assert_eq!(summary.result_namespace_count, 1);
        assert_eq!(summary.namespace_counts.runtime, 1);
        assert_eq!(summary.namespace_counts.semantic, 0);
        assert!(summary.all_runtime_blocks());
        assert!(!summary.has_namespace_mix());
        assert!(summary.block_accounting_balanced());
        assert!(summary.namespace_counts_match_results());
        assert!(summary.result_counts_match_blocks());
        assert!(summary.result_namespace_count_matches_counts());
        assert_eq!(summary.fusion_accounting_drift_component_count(), 0);
        assert!(summary.fusion_boundary_shape_is_clean());
        assert!(summary.can_commit_kv_fusion_persistence());
        assert_eq!(
            commit.action,
            KvFusionCommitAction::CommitKvFusionPersistence
        );
        assert!(commit.can_commit_kv_fusion_persistence());
        assert_eq!(commit.failure_report_count, 0);
    }

    #[test]
    fn fusion_zero_candidate_budget_skips_all_incoming_without_namespace_pollution() {
        let existing = vec![KvBlock::new(
            1,
            KvNamespace::Runtime,
            0,
            0,
            0..4,
            vec![1.0, 0.0],
            vec![1.0, 0.0],
        )];
        let incoming = vec![
            KvBlock::new(
                2,
                KvNamespace::Runtime,
                0,
                0,
                0..4,
                vec![1.0, 0.0],
                vec![1.0, 0.0],
            ),
            KvBlock::new(
                3,
                KvNamespace::Semantic,
                0,
                1,
                4..8,
                vec![0.0, 1.0],
                vec![0.0, 1.0],
            ),
        ];
        let policy = ReinforcedKvFusionPolicy {
            max_candidates: 0,
            ..ReinforcedKvFusionPolicy::default()
        };

        let report = policy.fuse(&existing, &incoming);
        let summary = report.merge_summary();
        let commit = summary.commit_summary();

        assert_eq!(report.before, 3);
        assert_eq!(report.after, 1);
        assert_eq!(report.merged_count(), 0);
        assert_eq!(report.skipped, 2);
        assert!(report.skipped_due_to_limit());
        assert_eq!(summary.skipped_count, 2);
        assert!(summary.skipped_due_to_limit);
        assert!(!summary.has_merges());
        assert!(summary.has_skips());
        assert!(!summary.changed_due_to_merges());
        assert!(summary.changed_due_to_skips());
        assert_eq!(summary.runtime_block_count, 1);
        assert_eq!(summary.non_runtime_block_count, 0);
        assert_eq!(summary.result_namespace_count, 1);
        assert_eq!(summary.namespace_counts.runtime, 1);
        assert_eq!(summary.namespace_counts.semantic, 0);
        assert!(summary.all_runtime_blocks());
        assert!(!summary.has_namespace_mix());
        assert!(summary.block_accounting_balanced());
        assert!(summary.namespace_counts_match_results());
        assert!(summary.result_counts_match_blocks());
        assert!(summary.result_namespace_count_matches_counts());
        assert_eq!(summary.fusion_accounting_drift_component_count(), 0);
        assert_eq!(summary.skip_signal_component_count(), 1);
        assert!(summary.fusion_boundary_shape_is_clean());
        assert!(summary.can_commit_kv_fusion_persistence());
        assert_eq!(
            commit.action,
            KvFusionCommitAction::CommitKvFusionPersistence
        );
        assert!(commit.can_commit_kv_fusion_persistence());
        assert_eq!(commit.failure_report_count, 0);
    }

    #[test]
    fn fusion_merge_summary_counts_accounting_drift_components() {
        let summary = KvFusionMergeSummary {
            before: 4,
            after: 3,
            merged_count: 0,
            skipped_count: 0,
            merge_fraction: 0.0,
            changed: true,
            skipped_due_to_limit: false,
            runtime_block_count: 1,
            non_runtime_block_count: 1,
            result_namespace_count: 1,
            namespace_counts: KvNamespaceCounts {
                runtime: 1,
                semantic: 1,
                gist: 1,
                agent: 0,
                custom: 0,
            },
        };

        assert!(!summary.block_accounting_balanced());
        assert_eq!(summary.block_accounting_drift_component_count(), 1);
        assert!(summary.namespace_counts_match_results());
        assert_eq!(summary.namespace_count_drift_component_count(), 0);
        assert!(!summary.result_counts_match_blocks());
        assert_eq!(summary.result_count_drift_component_count(), 1);
        assert!(!summary.result_namespace_count_matches_counts());
        assert_eq!(summary.result_namespace_count_drift_component_count(), 1);
        assert_eq!(summary.namespace_mix_signal_component_count(), 0);
        assert_eq!(summary.runtime_namespace_mix_signal_component_count(), 1);
        assert_eq!(summary.fusion_boundary_signal_component_count(), 1);
        assert!(summary.has_fusion_boundary_signals());
        assert_eq!(
            summary.result_namespace_boundary_signal_component_count(),
            4
        );
        assert!(summary.has_result_namespace_boundary_signals());
        assert_eq!(summary.fusion_accounting_drift_component_count(), 3);
        assert!(summary.has_fusion_accounting_drift_components());
        assert!(!summary.has_clean_accounting());
        assert!(summary.fusion_accounting_is_consistent());
        assert!(summary.merge_fraction_shape_is_valid());
        assert_eq!(summary.merge_fraction_shape_problem_component_count(), 0);
        assert_eq!(summary.fusion_boundary_problem_component_count(), 3);
        assert!(summary.has_fusion_boundary_problem_components());
        assert!(!summary.fusion_boundary_is_consistent());
        assert!(!summary.fusion_boundary_shape_is_clean());
        assert!(!summary.can_use_kv_fusion_merge());
        assert_eq!(summary.fusion_commit_signal_component_count(), 5);
        assert!(summary.has_fusion_commit_signals());
        assert_eq!(summary.fusion_commit_blocker_component_count(), 3);
        assert!(summary.has_fusion_commit_blockers());
        assert!(summary.fusion_commit_accounting_is_consistent());
        assert!(!summary.fusion_commit_shape_is_clean());
        assert!(!summary.can_commit_kv_fusion_persistence());
        assert_eq!(summary.empty_persistence_problem_component_count(), 0);
        assert_eq!(summary.component_accounting_drift_count(), 0);
        assert_eq!(summary.fusion_persistence_problem_component_count(), 3);
        assert!(summary.has_fusion_persistence_problem_components());
        let report = summary.failure_report().expect("fusion failure report");
        assert_eq!(
            report.kind,
            crate::engine::RuntimeFailureKind::ContractViolation
        );
        assert!(report.message.contains("components=3"));
        let report_summary = report.failure_summary();
        assert_eq!(summary.failure_reports(), vec![report.clone()]);
        assert_eq!(summary.failure_report_count(), 1);
        assert!(summary.has_failure_reports());
        assert_eq!(summary.failure_batch_summary().contract_violation_count, 1);
        assert!(summary.can_format_runtime_failures());
        assert_eq!(summary.primary_failure_report(), Some(report.clone()));
        assert_eq!(
            summary
                .primary_failure_summary()
                .map(|failure| failure.kind),
            Some(crate::engine::RuntimeFailureKind::ContractViolation)
        );
        let commit = summary.commit_summary();
        assert_eq!(commit.action, KvFusionCommitAction::ReturnRuntimeFailure);
        assert!(!commit.action_can_commit());
        assert!(commit.action_should_return_failure());
        assert!(!commit.can_commit_kv_fusion_persistence());
        assert!(commit.should_return_runtime_failure());
        assert_eq!(commit.failure_reports, vec![report]);
        assert_eq!(commit.failure_report_count, 1);
        assert_eq!(commit.failure_batch.contract_violation_count, 1);
        assert!(commit.can_format_runtime_failures);
        assert_eq!(commit.total_signal_component_count, 5);
        assert_eq!(commit.total_blocker_component_count, 3);
        assert!(commit.component_accounting_consistent);
        assert!(commit.has_primary_failure_summary());
        assert!(commit.failure_batch_shape_is_clean());
        assert!(commit.commit_decision_accounting_is_consistent());
        let failure_return = commit.failure_return_summary();
        assert_eq!(
            failure_return.source,
            RuntimeKvPersistenceFailureReturnSource::KvFusionPersistence
        );
        assert!(failure_return.has_failure_reports());
        assert!(failure_return.has_blocker_components());
        assert!(failure_return.failure_return_accounting_is_consistent());
        assert!(failure_return.can_return_runtime_failure());
        let return_report = commit
            .runtime_failure_return_report()
            .expect("fusion persistence return report");
        assert_eq!(
            return_report.source,
            RuntimeKvPersistenceFailureReturnSource::KvFusionPersistence
        );
        assert_eq!(return_report.primary_failure_summary, report_summary);
        assert_eq!(return_report.failure_batch.contract_violation_count, 1);
        assert!(return_report.failure_return_report_shape_is_clean());
        assert!(return_report.can_use_runtime_kv_persistence_failure_return_report());
        assert!(
            return_report
                .backend_message()
                .contains("kv fusion persistence failed")
        );
        assert!(
            return_report
                .diagnostics_note()
                .starts_with("runtime_contract_violation")
        );
        assert_eq!(
            return_report.inference_error().message,
            return_report.backend_message()
        );
    }

    #[test]
    fn fusion_merge_summary_blocks_namespace_count_drift_before_persistence() {
        let summary = KvFusionMergeSummary {
            before: 3,
            after: 2,
            merged_count: 1,
            skipped_count: 0,
            merge_fraction: 1.0 / 3.0,
            changed: true,
            skipped_due_to_limit: false,
            runtime_block_count: 2,
            non_runtime_block_count: 0,
            result_namespace_count: 1,
            namespace_counts: KvNamespaceCounts {
                runtime: 3,
                semantic: 0,
                gist: 0,
                agent: 0,
                custom: 0,
            },
        };

        assert!(summary.block_accounting_balanced());
        assert!(!summary.namespace_counts_match_results());
        assert_eq!(summary.namespace_count_drift_component_count(), 1);
        assert!(summary.result_counts_match_blocks());
        assert_eq!(summary.result_count_drift_component_count(), 0);
        assert!(summary.result_namespace_count_matches_counts());
        assert_eq!(summary.result_namespace_count_drift_component_count(), 0);
        assert_eq!(summary.fusion_accounting_drift_component_count(), 1);
        assert!(summary.has_fusion_accounting_drift_components());
        assert!(!summary.has_clean_accounting());
        assert!(summary.fusion_accounting_is_consistent());
        assert!(summary.merge_fraction_shape_is_valid());
        assert_eq!(summary.fusion_boundary_problem_component_count(), 1);
        assert!(summary.has_fusion_boundary_problem_components());
        assert!(!summary.fusion_boundary_is_consistent());
        assert!(!summary.fusion_boundary_shape_is_clean());
        assert!(!summary.can_use_kv_fusion_merge());
        assert_eq!(summary.fusion_commit_blocker_component_count(), 1);
        assert!(summary.has_fusion_commit_blockers());
        assert!(summary.fusion_commit_accounting_is_consistent());
        assert!(!summary.fusion_commit_shape_is_clean());
        assert!(!summary.can_commit_kv_fusion_persistence());
        assert_eq!(summary.fusion_persistence_problem_component_count(), 1);

        let report = summary
            .failure_report()
            .expect("namespace count drift blocks fusion persistence");
        assert_eq!(
            report.kind,
            crate::engine::RuntimeFailureKind::ContractViolation
        );
        assert!(report.message.contains("components=1"));
        assert_eq!(summary.failure_reports(), vec![report.clone()]);
        assert_eq!(summary.failure_batch_summary().contract_violation_count, 1);
        let commit = summary.commit_summary();
        assert_eq!(commit.action, KvFusionCommitAction::ReturnRuntimeFailure);
        assert!(commit.action_should_return_failure());
        assert!(!commit.action_can_commit());
        assert!(commit.should_return_runtime_failure());
        assert!(!commit.can_commit_kv_fusion_persistence());
        assert_eq!(commit.failure_reports, vec![report.clone()]);
        assert_eq!(commit.failure_report_count, 1);
        assert_eq!(commit.failure_batch.contract_violation_count, 1);
        assert_eq!(commit.total_blocker_component_count, 1);
        assert!(commit.component_accounting_consistent);
        assert!(commit.failure_batch_shape_is_clean());
        assert!(commit.commit_decision_accounting_is_consistent());
        let return_report = commit
            .runtime_failure_return_report()
            .expect("namespace count drift return report");
        assert_eq!(
            return_report.source,
            RuntimeKvPersistenceFailureReturnSource::KvFusionPersistence
        );
        assert_eq!(return_report.primary_failure, report);
        assert!(return_report.can_use_runtime_kv_persistence_failure_return_report());
    }

    #[test]
    fn fusion_merge_summary_blocks_public_fraction_drift() {
        let summary = KvFusionMergeSummary {
            before: 4,
            after: 3,
            merged_count: 1,
            skipped_count: 0,
            merge_fraction: 0.75,
            changed: true,
            skipped_due_to_limit: false,
            runtime_block_count: 3,
            non_runtime_block_count: 0,
            result_namespace_count: 1,
            namespace_counts: KvNamespaceCounts {
                runtime: 3,
                semantic: 0,
                gist: 0,
                agent: 0,
                custom: 0,
            },
        };

        assert!(summary.has_clean_accounting());
        assert!(summary.fusion_accounting_is_consistent());
        assert!(!summary.merge_fraction_shape_is_valid());
        assert_eq!(summary.merge_fraction_shape_problem_component_count(), 1);
        assert_eq!(summary.fusion_boundary_problem_component_count(), 1);
        assert!(summary.has_fusion_boundary_problem_components());
        assert!(!summary.fusion_boundary_is_consistent());
        assert!(!summary.fusion_boundary_shape_is_clean());
        assert!(!summary.can_use_kv_fusion_merge());
        assert_eq!(
            summary.result_namespace_boundary_signal_component_count(),
            1
        );
        assert_eq!(summary.fusion_commit_signal_component_count(), 2);
        assert!(summary.has_fusion_commit_signals());
        assert_eq!(summary.fusion_commit_blocker_component_count(), 1);
        assert!(summary.has_fusion_commit_blockers());
        assert!(summary.fusion_commit_accounting_is_consistent());
        assert!(!summary.fusion_commit_shape_is_clean());
        assert!(!summary.can_commit_kv_fusion_persistence());
    }

    #[test]
    fn fusion_merge_summary_blocks_skip_limit_flag_drift() {
        let summary = KvFusionMergeSummary {
            before: 3,
            after: 2,
            merged_count: 1,
            skipped_count: 0,
            merge_fraction: 1.0 / 3.0,
            changed: true,
            skipped_due_to_limit: true,
            runtime_block_count: 2,
            non_runtime_block_count: 0,
            result_namespace_count: 1,
            namespace_counts: KvNamespaceCounts {
                runtime: 2,
                semantic: 0,
                gist: 0,
                agent: 0,
                custom: 0,
            },
        };

        assert!(!summary.has_skips());
        assert!(!summary.changed_due_to_skips());
        assert!(!summary.skip_limit_flag_matches_skips());
        assert_eq!(summary.skip_limit_flag_drift_component_count(), 1);
        assert!(summary.block_accounting_balanced());
        assert!(summary.namespace_counts_match_results());
        assert!(summary.result_counts_match_blocks());
        assert!(summary.result_namespace_count_matches_counts());
        assert_eq!(summary.fusion_accounting_drift_component_count(), 1);
        assert!(summary.has_fusion_accounting_drift_components());
        assert!(!summary.has_clean_accounting());
        assert!(summary.fusion_accounting_is_consistent());
        assert!(summary.merge_fraction_shape_is_valid());
        assert_eq!(summary.fusion_boundary_problem_component_count(), 1);
        assert!(summary.has_fusion_boundary_problem_components());
        assert!(!summary.fusion_boundary_is_consistent());
        assert!(!summary.fusion_boundary_shape_is_clean());
        assert!(!summary.can_use_kv_fusion_merge());
        assert_eq!(summary.fusion_commit_blocker_component_count(), 1);
        assert!(summary.has_fusion_commit_blockers());
        assert!(summary.fusion_commit_accounting_is_consistent());
        assert!(!summary.fusion_commit_shape_is_clean());
        assert!(!summary.can_commit_kv_fusion_persistence());

        let commit = summary.commit_summary();
        assert_eq!(commit.action, KvFusionCommitAction::ReturnRuntimeFailure);
        assert!(commit.should_return_runtime_failure());
        assert_eq!(commit.total_blocker_component_count, 1);
        assert!(commit.commit_decision_accounting_is_consistent());
    }

    #[test]
    fn fusion_merge_summary_exposes_persistence_admission_boundary() {
        let admitted = KvFusionMergeSummary {
            before: 3,
            after: 2,
            merged_count: 1,
            skipped_count: 0,
            merge_fraction: 1.0 / 3.0,
            changed: true,
            skipped_due_to_limit: false,
            runtime_block_count: 1,
            non_runtime_block_count: 1,
            result_namespace_count: 2,
            namespace_counts: KvNamespaceCounts {
                runtime: 1,
                semantic: 1,
                gist: 0,
                agent: 0,
                custom: 0,
            },
        };
        let empty = KvFusionMergeSummary {
            before: 0,
            after: 0,
            merged_count: 0,
            skipped_count: 0,
            merge_fraction: 0.0,
            changed: false,
            skipped_due_to_limit: false,
            runtime_block_count: 0,
            non_runtime_block_count: 0,
            result_namespace_count: 0,
            namespace_counts: KvNamespaceCounts::default(),
        };
        let drifted = KvFusionMergeSummary {
            before: 4,
            after: 3,
            merged_count: 1,
            skipped_count: 0,
            merge_fraction: 0.75,
            changed: true,
            skipped_due_to_limit: false,
            runtime_block_count: 3,
            non_runtime_block_count: 0,
            result_namespace_count: 1,
            namespace_counts: KvNamespaceCounts {
                runtime: 3,
                semantic: 0,
                gist: 0,
                agent: 0,
                custom: 0,
            },
        };

        assert_eq!(admitted.kv_fusion_admission_signal_component_count(), 7);
        assert!(admitted.has_kv_fusion_admission_signals());
        assert_eq!(admitted.kv_fusion_admission_blocker_component_count(), 0);
        assert!(!admitted.has_kv_fusion_admission_blockers());
        assert!(admitted.kv_fusion_admission_accounting_is_consistent());
        assert!(admitted.kv_fusion_admission_is_clean());
        assert!(admitted.can_admit_kv_fusion_persistence());
        assert_eq!(
            admitted.can_admit_kv_fusion_persistence(),
            admitted.can_commit_kv_fusion_persistence()
        );

        assert_eq!(empty.kv_fusion_admission_signal_component_count(), 0);
        assert!(!empty.has_kv_fusion_admission_signals());
        assert_eq!(empty.kv_fusion_admission_blocker_component_count(), 1);
        assert!(empty.has_kv_fusion_admission_blockers());
        assert!(empty.kv_fusion_admission_accounting_is_consistent());
        assert!(!empty.kv_fusion_admission_is_clean());
        assert!(!empty.can_admit_kv_fusion_persistence());
        assert_eq!(
            empty.kv_fusion_admission_blocker_component_count(),
            empty.fusion_persistence_problem_component_count()
        );
        assert_eq!(
            empty.can_admit_kv_fusion_persistence(),
            empty.can_commit_kv_fusion_persistence()
        );

        assert_eq!(drifted.kv_fusion_admission_signal_component_count(), 2);
        assert!(drifted.has_kv_fusion_admission_signals());
        assert_eq!(drifted.kv_fusion_admission_blocker_component_count(), 1);
        assert!(drifted.has_kv_fusion_admission_blockers());
        assert!(drifted.kv_fusion_admission_accounting_is_consistent());
        assert!(!drifted.kv_fusion_admission_is_clean());
        assert!(!drifted.can_admit_kv_fusion_persistence());
        assert_eq!(
            drifted.kv_fusion_admission_blocker_component_count(),
            drifted.fusion_persistence_problem_component_count()
        );
        assert_eq!(
            drifted.can_admit_kv_fusion_persistence(),
            drifted.can_commit_kv_fusion_persistence()
        );
    }

    #[test]
    fn kv_fusion_commit_summary_exposes_admission_boundary() {
        let admitted = KvFusionMergeSummary {
            before: 3,
            after: 2,
            merged_count: 1,
            skipped_count: 0,
            merge_fraction: 1.0 / 3.0,
            changed: true,
            skipped_due_to_limit: false,
            runtime_block_count: 1,
            non_runtime_block_count: 1,
            result_namespace_count: 2,
            namespace_counts: KvNamespaceCounts {
                runtime: 1,
                semantic: 1,
                gist: 0,
                agent: 0,
                custom: 0,
            },
        }
        .commit_summary();
        let empty = KvFusionMergeSummary {
            before: 0,
            after: 0,
            merged_count: 0,
            skipped_count: 0,
            merge_fraction: 0.0,
            changed: false,
            skipped_due_to_limit: false,
            runtime_block_count: 0,
            non_runtime_block_count: 0,
            result_namespace_count: 0,
            namespace_counts: KvNamespaceCounts::default(),
        }
        .commit_summary();
        let drifted = KvFusionMergeSummary {
            before: 4,
            after: 3,
            merged_count: 1,
            skipped_count: 0,
            merge_fraction: 0.75,
            changed: true,
            skipped_due_to_limit: false,
            runtime_block_count: 1,
            non_runtime_block_count: 1,
            result_namespace_count: 1,
            namespace_counts: KvNamespaceCounts {
                runtime: 1,
                semantic: 1,
                gist: 1,
                agent: 0,
                custom: 0,
            },
        }
        .commit_summary();

        assert_eq!(
            admitted.action,
            KvFusionCommitAction::CommitKvFusionPersistence
        );
        assert_eq!(
            admitted.kv_fusion_commit_admission_signal_component_count(),
            7
        );
        assert!(admitted.has_kv_fusion_commit_admission_signals());
        assert_eq!(admitted.missing_commit_component_count(), 0);
        assert_eq!(admitted.commit_decision_drift_component_count(), 0);
        assert_eq!(
            admitted.kv_fusion_commit_admission_blocker_component_count(),
            0
        );
        assert!(!admitted.has_kv_fusion_commit_admission_blockers());
        assert!(admitted.kv_fusion_commit_admission_accounting_is_consistent());
        assert!(admitted.kv_fusion_commit_admission_is_clean());
        assert!(admitted.can_admit_kv_fusion_commit());
        assert!(admitted.can_commit_kv_fusion_persistence());
        assert!(!admitted.should_return_runtime_failure());

        assert_eq!(empty.action, KvFusionCommitAction::ReturnRuntimeFailure);
        assert_eq!(empty.kv_fusion_commit_admission_signal_component_count(), 0);
        assert!(!empty.has_kv_fusion_commit_admission_signals());
        assert_eq!(empty.missing_commit_component_count(), 1);
        assert_eq!(empty.commit_decision_drift_component_count(), 0);
        assert_eq!(
            empty.kv_fusion_commit_admission_blocker_component_count(),
            1
        );
        assert!(empty.has_kv_fusion_commit_admission_blockers());
        assert!(empty.kv_fusion_commit_admission_accounting_is_consistent());
        assert!(!empty.kv_fusion_commit_admission_is_clean());
        assert!(!empty.can_admit_kv_fusion_commit());
        assert!(!empty.can_commit_kv_fusion_persistence());
        assert!(empty.should_return_runtime_failure());

        assert_eq!(drifted.action, KvFusionCommitAction::ReturnRuntimeFailure);
        assert_eq!(
            drifted.kv_fusion_commit_admission_signal_component_count(),
            6
        );
        assert!(drifted.has_kv_fusion_commit_admission_signals());
        assert_eq!(drifted.missing_commit_component_count(), 1);
        assert_eq!(drifted.commit_decision_drift_component_count(), 0);
        assert_eq!(
            drifted.kv_fusion_commit_admission_blocker_component_count(),
            4
        );
        assert!(drifted.has_kv_fusion_commit_admission_blockers());
        assert!(drifted.kv_fusion_commit_admission_accounting_is_consistent());
        assert!(!drifted.kv_fusion_commit_admission_is_clean());
        assert!(!drifted.can_admit_kv_fusion_commit());
        assert!(!drifted.can_commit_kv_fusion_persistence());
        assert!(drifted.should_return_runtime_failure());
    }

    #[test]
    fn empty_fusion_summary_is_clean_but_not_usable() {
        let summary = KvFusionMergeSummary {
            before: 0,
            after: 0,
            merged_count: 0,
            skipped_count: 0,
            merge_fraction: 0.0,
            changed: false,
            skipped_due_to_limit: false,
            runtime_block_count: 0,
            non_runtime_block_count: 0,
            result_namespace_count: 0,
            namespace_counts: KvNamespaceCounts::default(),
        };

        assert!(summary.is_noop());
        assert!(summary.has_clean_accounting());
        assert!(summary.fusion_accounting_is_consistent());
        assert!(summary.merge_fraction_shape_is_valid());
        assert_eq!(summary.fusion_boundary_problem_component_count(), 0);
        assert!(!summary.has_fusion_boundary_problem_components());
        assert!(summary.fusion_boundary_is_consistent());
        assert!(summary.fusion_boundary_shape_is_clean());
        assert!(!summary.can_use_kv_fusion_merge());
        assert_eq!(
            summary.result_namespace_boundary_signal_component_count(),
            0
        );
        assert!(!summary.has_result_namespace_boundary_signals());
        assert_eq!(summary.fusion_commit_signal_component_count(), 0);
        assert!(!summary.has_fusion_commit_signals());
        assert_eq!(summary.fusion_commit_blocker_component_count(), 0);
        assert!(!summary.has_fusion_commit_blockers());
        assert!(summary.fusion_commit_accounting_is_consistent());
        assert!(summary.fusion_commit_shape_is_clean());
        assert!(!summary.can_commit_kv_fusion_persistence());
        assert_eq!(summary.empty_persistence_problem_component_count(), 1);
        assert_eq!(summary.component_accounting_drift_count(), 0);
        assert_eq!(summary.fusion_persistence_problem_component_count(), 1);
        assert!(summary.has_fusion_persistence_problem_components());
        let report = summary
            .failure_report()
            .expect("empty fusion persistence failure report");
        assert_eq!(
            report.kind,
            crate::engine::RuntimeFailureKind::ContractViolation
        );
        assert!(report.message.contains("components=1"));
        assert_eq!(summary.failure_reports(), vec![report.clone()]);
        assert_eq!(summary.failure_report_count(), 1);
        assert!(summary.has_failure_reports());
        assert_eq!(summary.failure_batch_summary().contract_violation_count, 1);
        assert!(summary.can_format_runtime_failures());
        assert_eq!(summary.primary_failure_report(), Some(report.clone()));
        let commit = summary.commit_summary();
        assert_eq!(commit.action, KvFusionCommitAction::ReturnRuntimeFailure);
        assert!(!commit.action_can_commit());
        assert!(commit.action_should_return_failure());
        assert!(!commit.can_commit_kv_fusion_persistence());
        assert!(commit.should_return_runtime_failure());
        assert_eq!(commit.failure_reports, vec![report]);
        assert_eq!(commit.failure_report_count, 1);
        assert_eq!(commit.failure_batch.contract_violation_count, 1);
        assert!(commit.can_format_runtime_failures);
        assert_eq!(commit.total_signal_component_count, 0);
        assert_eq!(commit.total_blocker_component_count, 0);
        assert!(commit.component_accounting_consistent);
        assert!(commit.has_primary_failure_summary());
        assert!(commit.failure_batch_shape_is_clean());
        assert!(commit.commit_decision_accounting_is_consistent());
    }
}
