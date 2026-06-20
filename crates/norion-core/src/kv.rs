use std::ops::Range;

use crate::engine::{
    InferenceError, RuntimeFailureBatchSummary, RuntimeFailureReport, RuntimeFailureSummary,
};
use crate::manifest::{RuntimeManifestDigest, TransformerRuntimeArchitecture};
use crate::request::RuntimeRequestEnvelope;
use crate::runtime::RuntimeMetadata;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum KvNamespace {
    Runtime,
    Semantic,
    Gist,
    Agent(String),
    Custom(String),
}

impl KvNamespace {
    pub fn from_key(key: &str) -> Self {
        if key.starts_with("runtime_kv:") {
            Self::Runtime
        } else if key.starts_with("gist:") {
            Self::Gist
        } else if let Some(agent) = key.strip_prefix("agent:") {
            Self::Agent(namespace_segment(agent))
        } else if let Some(custom) = key.strip_prefix("custom:") {
            Self::Custom(namespace_segment(custom))
        } else {
            Self::Semantic
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Self::Runtime => "runtime",
            Self::Semantic => "semantic",
            Self::Gist => "gist",
            Self::Agent(_) => "agent",
            Self::Custom(_) => "custom",
        }
    }

    pub fn is_runtime_exchange(&self) -> bool {
        matches!(self, Self::Runtime)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KvNamespaceCounts {
    pub runtime: usize,
    pub semantic: usize,
    pub gist: usize,
    pub agent: usize,
    pub custom: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KvNamespaceCountDriftSummary {
    pub expected: KvNamespaceCounts,
    pub actual: KvNamespaceCounts,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KvNamespaceCountDriftCommitSummary {
    pub drift: KvNamespaceCountDriftSummary,
    pub action: KvNamespaceCountDriftCommitAction,
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
pub enum KvNamespaceCountDriftCommitAction {
    CommitKvNamespaceDistribution,
    ReturnRuntimeFailure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeKvPersistenceFailureReturnSource {
    NamespaceDistribution,
    KvFusionPersistence,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeKvPersistenceFailureReturnSummary {
    pub source: RuntimeKvPersistenceFailureReturnSource,
    pub can_commit: bool,
    pub should_return_failure: bool,
    pub has_primary_failure_summary: bool,
    pub primary_failure_summary: Option<RuntimeFailureSummary>,
    pub failure_batch: RuntimeFailureBatchSummary,
    pub failure_report_count: usize,
    pub can_format_runtime_failures: bool,
    pub total_blocker_component_count: usize,
    pub commit_decision_accounting_consistent: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeKvPersistenceFailureReturnReport {
    pub source: RuntimeKvPersistenceFailureReturnSource,
    pub primary_failure: RuntimeFailureReport,
    pub primary_failure_summary: RuntimeFailureSummary,
    pub failure_batch: RuntimeFailureBatchSummary,
    pub failure_report_count: usize,
    pub can_format_runtime_failures: bool,
    pub total_blocker_component_count: usize,
}

impl KvNamespaceCountDriftCommitAction {
    pub fn can_commit(self) -> bool {
        matches!(self, Self::CommitKvNamespaceDistribution)
    }

    pub fn should_return_failure(self) -> bool {
        matches!(self, Self::ReturnRuntimeFailure)
    }
}

impl RuntimeKvPersistenceFailureReturnSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::NamespaceDistribution => "kv_namespace_distribution",
            Self::KvFusionPersistence => "kv_fusion_persistence",
        }
    }
}

impl RuntimeKvPersistenceFailureReturnSummary {
    pub fn new(
        source: RuntimeKvPersistenceFailureReturnSource,
        can_commit: bool,
        should_return_failure: bool,
        primary_failure_summary: Option<RuntimeFailureSummary>,
        failure_batch: RuntimeFailureBatchSummary,
        failure_report_count: usize,
        can_format_runtime_failures: bool,
        total_blocker_component_count: usize,
        commit_decision_accounting_consistent: bool,
    ) -> Self {
        Self {
            source,
            can_commit,
            should_return_failure,
            has_primary_failure_summary: primary_failure_summary.is_some(),
            primary_failure_summary,
            failure_batch,
            failure_report_count,
            can_format_runtime_failures,
            total_blocker_component_count,
            commit_decision_accounting_consistent,
        }
    }

    pub fn has_failure_reports(self) -> bool {
        self.failure_report_count > 0
    }

    pub fn has_blocker_components(self) -> bool {
        self.total_blocker_component_count > 0
    }

    pub fn failure_return_accounting_is_consistent(self) -> bool {
        self.commit_decision_accounting_consistent
            && self.should_return_failure == (!self.can_commit && self.has_failure_reports())
            && self.has_primary_failure_summary == self.primary_failure_summary.is_some()
            && self.has_primary_failure_summary == self.has_failure_reports()
            && self.failure_batch.total_count == self.failure_report_count
            && self.can_format_runtime_failures == self.failure_batch.can_format_runtime_failures()
            && (!self.has_failure_reports() || self.has_blocker_components())
    }

    pub fn can_return_runtime_failure(self) -> bool {
        self.should_return_failure
            && self.has_primary_failure_summary
            && self.can_format_runtime_failures
            && self.failure_return_accounting_is_consistent()
    }
}

impl RuntimeKvPersistenceFailureReturnReport {
    pub fn new(
        source: RuntimeKvPersistenceFailureReturnSource,
        primary_failure: RuntimeFailureReport,
        failure_batch: RuntimeFailureBatchSummary,
        failure_report_count: usize,
        can_format_runtime_failures: bool,
        total_blocker_component_count: usize,
    ) -> Self {
        let primary_failure_summary = primary_failure.failure_summary();
        Self {
            source,
            primary_failure,
            primary_failure_summary,
            failure_batch,
            failure_report_count,
            can_format_runtime_failures,
            total_blocker_component_count,
        }
    }

    pub fn backend_message(&self) -> String {
        self.primary_failure.backend_message()
    }

    pub fn diagnostics_note(&self) -> String {
        self.primary_failure.diagnostics_note()
    }

    pub fn inference_error(&self) -> InferenceError {
        InferenceError::from_failure(self.primary_failure.clone())
    }

    pub fn failure_return_report_shape_is_clean(&self) -> bool {
        self.primary_failure_summary == self.primary_failure.failure_summary()
            && self.failure_report_count > 0
            && self.failure_batch.total_count == self.failure_report_count
            && self.can_format_runtime_failures == self.failure_batch.can_format_runtime_failures()
            && self.can_format_runtime_failures
            && self.total_blocker_component_count > 0
    }

    pub fn can_use_runtime_kv_persistence_failure_return_report(&self) -> bool {
        self.failure_return_report_shape_is_clean()
    }
}

impl KvNamespaceCounts {
    pub fn total(self) -> usize {
        self.runtime
            .saturating_add(self.semantic)
            .saturating_add(self.gist)
            .saturating_add(self.agent)
            .saturating_add(self.custom)
    }

    pub fn non_runtime_total(self) -> usize {
        self.total().saturating_sub(self.runtime)
    }

    pub fn active_namespace_count(self) -> usize {
        [
            self.runtime,
            self.semantic,
            self.gist,
            self.agent,
            self.custom,
        ]
        .into_iter()
        .filter(|count| *count > 0)
        .count()
    }

    pub fn has_runtime_exchange(self) -> bool {
        self.runtime > 0
    }

    pub fn has_namespace_mix(self) -> bool {
        self.active_namespace_count() > 1
    }

    pub fn has_runtime_and_non_runtime_blocks(self) -> bool {
        self.runtime > 0 && self.non_runtime_total() > 0
    }

    pub fn only_runtime_exchange(self) -> bool {
        self.runtime > 0 && self.non_runtime_total() == 0
    }

    pub fn only_non_runtime_blocks(self) -> bool {
        self.runtime == 0 && self.non_runtime_total() > 0
    }

    pub fn runtime_fraction(self) -> f32 {
        let total = self.total();
        if total == 0 {
            0.0
        } else {
            self.runtime as f32 / total as f32
        }
    }

    pub fn runtime_exchange_signal_component_count(self) -> usize {
        usize::from(self.has_runtime_exchange())
    }

    pub fn non_runtime_payload_signal_component_count(self) -> usize {
        usize::from(self.non_runtime_total() > 0)
    }

    pub fn namespace_mix_signal_component_count(self) -> usize {
        usize::from(self.has_namespace_mix())
    }

    pub fn runtime_non_runtime_mix_signal_component_count(self) -> usize {
        usize::from(self.has_runtime_and_non_runtime_blocks())
    }

    pub fn namespace_boundary_signal_component_count(self) -> usize {
        self.runtime_exchange_signal_component_count()
            .saturating_add(self.non_runtime_payload_signal_component_count())
            .saturating_add(self.namespace_mix_signal_component_count())
            .saturating_add(self.runtime_non_runtime_mix_signal_component_count())
    }

    pub fn has_namespace_boundary_signals(self) -> bool {
        self.namespace_boundary_signal_component_count() > 0
    }

    pub fn drift_summary(self, actual: KvNamespaceCounts) -> KvNamespaceCountDriftSummary {
        KvNamespaceCountDriftSummary {
            expected: self,
            actual,
        }
    }
}

impl KvNamespaceCountDriftSummary {
    pub fn exact_match(self) -> bool {
        self.expected == self.actual
    }

    pub fn runtime_count_drift(self) -> usize {
        self.expected.runtime.abs_diff(self.actual.runtime)
    }

    pub fn semantic_count_drift(self) -> usize {
        self.expected.semantic.abs_diff(self.actual.semantic)
    }

    pub fn gist_count_drift(self) -> usize {
        self.expected.gist.abs_diff(self.actual.gist)
    }

    pub fn agent_count_drift(self) -> usize {
        self.expected.agent.abs_diff(self.actual.agent)
    }

    pub fn custom_count_drift(self) -> usize {
        self.expected.custom.abs_diff(self.actual.custom)
    }

    pub fn total_count_drift(self) -> usize {
        self.expected.total().abs_diff(self.actual.total())
    }

    pub fn non_runtime_count_drift(self) -> usize {
        self.expected
            .non_runtime_total()
            .abs_diff(self.actual.non_runtime_total())
    }

    pub fn active_namespace_count_drift(self) -> usize {
        self.expected
            .active_namespace_count()
            .abs_diff(self.actual.active_namespace_count())
    }

    pub fn runtime_count_drift_component_count(self) -> usize {
        usize::from(self.runtime_count_drift() > 0)
    }

    pub fn semantic_count_drift_component_count(self) -> usize {
        usize::from(self.semantic_count_drift() > 0)
    }

    pub fn gist_count_drift_component_count(self) -> usize {
        usize::from(self.gist_count_drift() > 0)
    }

    pub fn agent_count_drift_component_count(self) -> usize {
        usize::from(self.agent_count_drift() > 0)
    }

    pub fn custom_count_drift_component_count(self) -> usize {
        usize::from(self.custom_count_drift() > 0)
    }

    pub fn namespace_distribution_drift_component_count(self) -> usize {
        self.runtime_count_drift_component_count()
            .saturating_add(self.semantic_count_drift_component_count())
            .saturating_add(self.gist_count_drift_component_count())
            .saturating_add(self.agent_count_drift_component_count())
            .saturating_add(self.custom_count_drift_component_count())
    }

    pub fn total_count_drift_signal_component_count(self) -> usize {
        usize::from(self.total_count_drift() > 0)
    }

    pub fn non_runtime_count_drift_signal_component_count(self) -> usize {
        usize::from(self.non_runtime_count_drift() > 0)
    }

    pub fn active_namespace_count_drift_signal_component_count(self) -> usize {
        usize::from(self.active_namespace_count_drift() > 0)
    }

    pub fn namespace_shape_signal_component_count(self) -> usize {
        self.total_count_drift_signal_component_count()
            .saturating_add(self.non_runtime_count_drift_signal_component_count())
            .saturating_add(self.active_namespace_count_drift_signal_component_count())
    }

    pub fn has_namespace_shape_signals(self) -> bool {
        self.namespace_shape_signal_component_count() > 0
    }

    pub fn namespace_boundary_signal_component_count(self) -> usize {
        self.namespace_shape_signal_component_count()
    }

    pub fn has_namespace_boundary_signals(self) -> bool {
        self.namespace_boundary_signal_component_count() > 0
    }

    pub fn has_namespace_distribution_drift_components(self) -> bool {
        self.namespace_distribution_drift_component_count() > 0
    }

    pub fn namespace_distribution_accounting_is_consistent(self) -> bool {
        let expected_component_count = usize::from(self.runtime_count_drift() > 0)
            .saturating_add(usize::from(self.semantic_count_drift() > 0))
            .saturating_add(usize::from(self.gist_count_drift() > 0))
            .saturating_add(usize::from(self.agent_count_drift() > 0))
            .saturating_add(usize::from(self.custom_count_drift() > 0));

        self.namespace_distribution_drift_component_count() == expected_component_count
            && self.has_namespace_distribution_drift_components() == (expected_component_count > 0)
            && self.exact_match() == (expected_component_count == 0)
    }

    pub fn namespace_boundary_problem_component_count(self) -> usize {
        self.namespace_distribution_drift_component_count()
    }

    pub fn has_namespace_boundary_problem_components(self) -> bool {
        self.namespace_boundary_problem_component_count() > 0
    }

    pub fn namespace_boundary_is_clean(self) -> bool {
        !self.has_namespace_boundary_problem_components()
            && self.namespace_distribution_accounting_is_consistent()
    }

    pub fn namespace_distribution_shape_is_clean(self) -> bool {
        self.namespace_boundary_is_clean()
    }

    pub fn can_use_namespace_distribution(self) -> bool {
        self.namespace_distribution_shape_is_clean()
    }

    pub fn namespace_distribution_commit_signal_component_count(self) -> usize {
        self.namespace_boundary_signal_component_count()
    }

    pub fn has_namespace_distribution_commit_signals(self) -> bool {
        self.namespace_distribution_commit_signal_component_count() > 0
    }

    pub fn namespace_distribution_commit_blocker_component_count(self) -> usize {
        self.namespace_boundary_problem_component_count()
    }

    pub fn has_namespace_distribution_commit_blockers(self) -> bool {
        self.namespace_distribution_commit_blocker_component_count() > 0
    }

    pub fn namespace_distribution_commit_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self.namespace_boundary_signal_component_count();
        let expected_blocker_count = self.namespace_boundary_problem_component_count();

        self.namespace_distribution_commit_signal_component_count() == expected_signal_count
            && self.has_namespace_distribution_commit_signals() == (expected_signal_count > 0)
            && self.namespace_distribution_commit_blocker_component_count()
                == expected_blocker_count
            && self.has_namespace_distribution_commit_blockers() == (expected_blocker_count > 0)
            && self.can_use_namespace_distribution()
                == (expected_blocker_count == 0
                    && self.namespace_distribution_accounting_is_consistent())
    }

    pub fn namespace_distribution_commit_shape_is_clean(self) -> bool {
        !self.has_namespace_distribution_commit_blockers()
            && self.namespace_distribution_commit_accounting_is_consistent()
    }

    pub fn can_commit_namespace_distribution(self) -> bool {
        self.namespace_distribution_commit_shape_is_clean()
    }

    pub fn component_accounting_drift_count(self) -> usize {
        usize::from(!self.namespace_distribution_commit_accounting_is_consistent())
    }

    pub fn namespace_distribution_commit_problem_component_count(self) -> usize {
        self.namespace_distribution_commit_blocker_component_count()
            .saturating_add(self.component_accounting_drift_count())
    }

    pub fn has_namespace_distribution_commit_problem_components(self) -> bool {
        self.namespace_distribution_commit_problem_component_count() > 0
    }

    pub fn failure_report(self) -> Option<RuntimeFailureReport> {
        let component_count = self.namespace_distribution_commit_problem_component_count();
        if component_count == 0 {
            None
        } else {
            Some(RuntimeFailureReport::contract_violation(format!(
                "kv namespace distribution failed: components={component_count}"
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

    pub fn commit_summary(self) -> KvNamespaceCountDriftCommitSummary {
        KvNamespaceCountDriftCommitSummary::new(self)
    }
}

impl KvNamespaceCountDriftCommitSummary {
    pub fn new(drift: KvNamespaceCountDriftSummary) -> Self {
        let failure_reports = drift.failure_reports();
        let primary_failure_report = failure_reports.first().cloned();
        let primary_failure_summary = primary_failure_report
            .as_ref()
            .map(|failure| failure.failure_summary());
        let failure_batch = RuntimeFailureReport::batch_summary(&failure_reports);
        let can_commit = drift.can_commit_namespace_distribution();
        let failure_report_count = failure_reports.len();
        let can_format_runtime_failures = failure_batch.can_format_runtime_failures();
        let should_return_failure = !can_commit && failure_report_count > 0;
        let action = if can_commit {
            KvNamespaceCountDriftCommitAction::CommitKvNamespaceDistribution
        } else {
            KvNamespaceCountDriftCommitAction::ReturnRuntimeFailure
        };

        Self {
            drift,
            action,
            can_commit,
            should_return_failure,
            failure_reports,
            primary_failure_report,
            primary_failure_summary,
            failure_batch,
            failure_report_count,
            can_format_runtime_failures,
            total_signal_component_count: drift
                .namespace_distribution_commit_signal_component_count(),
            total_blocker_component_count: drift
                .namespace_distribution_commit_blocker_component_count(),
            component_accounting_consistent: drift
                .namespace_distribution_commit_accounting_is_consistent(),
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
            RuntimeKvPersistenceFailureReturnSource::NamespaceDistribution,
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
                    RuntimeKvPersistenceFailureReturnSource::NamespaceDistribution,
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
        self.can_commit == self.drift.can_commit_namespace_distribution()
            && self.should_return_failure == (!self.can_commit && self.failure_report_count > 0)
            && self.action_can_commit() == self.can_commit
            && self.action_should_return_failure() == self.should_return_failure
            && self.failure_report_count == self.failure_reports.len()
            && self.failure_report_count == self.drift.failure_report_count()
            && self.primary_failure_report.as_ref() == self.failure_reports.first()
            && self.primary_failure_summary
                == self
                    .primary_failure_report
                    .as_ref()
                    .map(|failure| failure.failure_summary())
            && self.failure_batch == RuntimeFailureReport::batch_summary(&self.failure_reports)
            && self.can_format_runtime_failures == self.failure_batch.can_format_runtime_failures()
            && self.total_signal_component_count
                == self
                    .drift
                    .namespace_distribution_commit_signal_component_count()
            && self.total_blocker_component_count
                == self
                    .drift
                    .namespace_distribution_commit_blocker_component_count()
            && self.component_accounting_consistent
                == self
                    .drift
                    .namespace_distribution_commit_accounting_is_consistent()
    }

    pub fn can_commit_namespace_distribution(&self) -> bool {
        self.can_commit && self.commit_decision_accounting_is_consistent()
    }

    pub fn should_return_runtime_failure(&self) -> bool {
        self.should_return_failure && self.commit_decision_accounting_is_consistent()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeKvCandidate {
    pub id: u64,
    pub vector: Vec<f32>,
    pub weight: f32,
}

impl RuntimeKvCandidate {
    pub fn new(id: u64, vector: impl Into<Vec<f32>>, weight: f32) -> Self {
        Self {
            id,
            vector: vector.into(),
            weight: if weight.is_finite() {
                weight.max(0.05)
            } else {
                0.05
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeKvImportPlan {
    pub max_blocks: usize,
    pub embedding_dimensions: Option<usize>,
    pub layer_count: usize,
    pub kv_heads: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeKvImportManifestPlanSummary {
    pub manifest_import_enabled: bool,
    pub manifest_max_import_blocks: usize,
    pub runtime_import_enabled: bool,
    pub runtime_max_import_blocks: usize,
    pub requested_prefetch_blocks: usize,
    pub import_plan_max_blocks: usize,
    pub embedding_dimensions: Option<usize>,
    pub architecture_layer_count: usize,
    pub architecture_kv_heads: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeKvImportSummary {
    pub enabled: bool,
    pub max_blocks: usize,
    pub candidate_count: usize,
    pub non_empty_candidate_count: usize,
    pub planned_blocks: usize,
    pub hit_import_limit: bool,
    pub embedding_dimensions: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RuntimeKvImportBlockSummary {
    pub planned_blocks: usize,
    pub materialized_blocks: usize,
    pub runtime_namespace_blocks: usize,
    pub block_shape_signal_component_count: usize,
    pub block_shape_problem_component_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeKvImportReadinessStage {
    ImportPlan,
    ImportBlocks,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeKvImportReadinessSummary {
    pub import: RuntimeKvImportSummary,
    pub blocks: RuntimeKvImportBlockSummary,
    pub import_signal_component_count: usize,
    pub block_signal_component_count: usize,
    pub import_blocker_component_count: usize,
    pub block_blocker_component_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeKvImportReadinessCommitSummary {
    pub readiness: RuntimeKvImportReadinessSummary,
    pub action: RuntimeKvImportReadinessCommitAction,
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
pub enum RuntimeKvImportReadinessCommitAction {
    CommitRuntimeKvImport,
    ReturnRuntimeFailure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeKvExchangeFailureReturnSource {
    RuntimeKvImportReadiness,
    RuntimeKvExportReadiness,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeKvExchangeFailureReturnSummary {
    pub source: RuntimeKvExchangeFailureReturnSource,
    pub can_commit: bool,
    pub should_return_failure: bool,
    pub has_primary_failure_summary: bool,
    pub primary_failure_summary: Option<RuntimeFailureSummary>,
    pub failure_batch: RuntimeFailureBatchSummary,
    pub failure_report_count: usize,
    pub can_format_runtime_failures: bool,
    pub total_blocker_component_count: usize,
    pub commit_decision_accounting_consistent: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeKvExchangeFailureReturnReport {
    pub source: RuntimeKvExchangeFailureReturnSource,
    pub primary_failure: RuntimeFailureReport,
    pub primary_failure_summary: RuntimeFailureSummary,
    pub failure_batch: RuntimeFailureBatchSummary,
    pub failure_report_count: usize,
    pub can_format_runtime_failures: bool,
    pub total_blocker_component_count: usize,
}

impl RuntimeKvImportReadinessCommitAction {
    pub fn can_commit(self) -> bool {
        matches!(self, Self::CommitRuntimeKvImport)
    }

    pub fn should_return_failure(self) -> bool {
        matches!(self, Self::ReturnRuntimeFailure)
    }
}

impl RuntimeKvExchangeFailureReturnSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::RuntimeKvImportReadiness => "runtime_kv_import_readiness",
            Self::RuntimeKvExportReadiness => "runtime_kv_export_readiness",
        }
    }
}

impl RuntimeKvExchangeFailureReturnSummary {
    pub fn new(
        source: RuntimeKvExchangeFailureReturnSource,
        can_commit: bool,
        should_return_failure: bool,
        primary_failure_summary: Option<RuntimeFailureSummary>,
        failure_batch: RuntimeFailureBatchSummary,
        failure_report_count: usize,
        can_format_runtime_failures: bool,
        total_blocker_component_count: usize,
        commit_decision_accounting_consistent: bool,
    ) -> Self {
        Self {
            source,
            can_commit,
            should_return_failure,
            has_primary_failure_summary: primary_failure_summary.is_some(),
            primary_failure_summary,
            failure_batch,
            failure_report_count,
            can_format_runtime_failures,
            total_blocker_component_count,
            commit_decision_accounting_consistent,
        }
    }

    pub fn has_failure_reports(self) -> bool {
        self.failure_report_count > 0
    }

    pub fn has_blocker_components(self) -> bool {
        self.total_blocker_component_count > 0
    }

    pub fn failure_return_accounting_is_consistent(self) -> bool {
        self.commit_decision_accounting_consistent
            && self.should_return_failure == (!self.can_commit && self.has_failure_reports())
            && self.has_primary_failure_summary == self.primary_failure_summary.is_some()
            && self.has_primary_failure_summary == self.has_failure_reports()
            && self.failure_batch.total_count == self.failure_report_count
            && self.can_format_runtime_failures == self.failure_batch.can_format_runtime_failures()
            && (!self.has_failure_reports() || self.has_blocker_components())
    }

    pub fn can_return_runtime_failure(self) -> bool {
        self.should_return_failure
            && self.has_primary_failure_summary
            && self.can_format_runtime_failures
            && self.failure_return_accounting_is_consistent()
    }
}

impl RuntimeKvExchangeFailureReturnReport {
    pub fn new(
        source: RuntimeKvExchangeFailureReturnSource,
        primary_failure: RuntimeFailureReport,
        failure_batch: RuntimeFailureBatchSummary,
        failure_report_count: usize,
        can_format_runtime_failures: bool,
        total_blocker_component_count: usize,
    ) -> Self {
        let primary_failure_summary = primary_failure.failure_summary();
        Self {
            source,
            primary_failure,
            primary_failure_summary,
            failure_batch,
            failure_report_count,
            can_format_runtime_failures,
            total_blocker_component_count,
        }
    }

    pub fn backend_message(&self) -> String {
        self.primary_failure.backend_message()
    }

    pub fn diagnostics_note(&self) -> String {
        self.primary_failure.diagnostics_note()
    }

    pub fn inference_error(&self) -> InferenceError {
        InferenceError::from_failure(self.primary_failure.clone())
    }

    pub fn failure_return_report_shape_is_clean(&self) -> bool {
        self.primary_failure_summary == self.primary_failure.failure_summary()
            && self.failure_report_count > 0
            && self.failure_batch.total_count == self.failure_report_count
            && self.can_format_runtime_failures == self.failure_batch.can_format_runtime_failures()
            && self.can_format_runtime_failures
            && self.total_blocker_component_count > 0
    }

    pub fn can_use_runtime_kv_exchange_failure_return_report(&self) -> bool {
        self.failure_return_report_shape_is_clean()
    }
}

impl RuntimeKvImportSummary {
    pub fn will_import(self) -> bool {
        self.enabled && self.planned_blocks > 0
    }

    pub fn skipped_due_to_empty_candidates(self) -> bool {
        self.enabled && self.candidate_count > 0 && self.non_empty_candidate_count == 0
    }

    pub fn has_candidates(self) -> bool {
        self.candidate_count > 0
    }

    pub fn has_non_empty_candidates(self) -> bool {
        self.non_empty_candidate_count > 0
    }

    pub fn empty_candidate_count(self) -> usize {
        self.candidate_count
            .saturating_sub(self.non_empty_candidate_count)
    }

    pub fn has_embedding_dimensions(self) -> bool {
        self.embedding_dimensions.is_some()
    }

    pub fn enabled_matches_capacity(self) -> bool {
        self.enabled == (self.max_blocks > 0)
    }

    pub fn candidate_counts_are_valid(self) -> bool {
        self.non_empty_candidate_count <= self.candidate_count
    }

    pub fn planned_blocks_within_limit(self) -> bool {
        self.planned_blocks <= self.max_blocks
    }

    pub fn planned_blocks_within_candidates(self) -> bool {
        self.planned_blocks <= self.non_empty_candidate_count
    }

    pub fn import_limit_flag_matches_shape(self) -> bool {
        self.hit_import_limit
            == (self.planned_blocks > 0
                && self.planned_blocks == self.max_blocks
                && self.non_empty_candidate_count >= self.max_blocks)
    }

    pub fn disabled_import_is_empty(self) -> bool {
        self.enabled || (self.planned_blocks == 0 && !self.hit_import_limit)
    }

    pub fn embedding_dimensions_shape_is_valid(self) -> bool {
        match self.embedding_dimensions {
            Some(dimensions) => dimensions > 0,
            None => true,
        }
    }

    pub fn import_signal_component_count(self) -> usize {
        usize::from(self.enabled)
            .saturating_add(usize::from(self.has_candidates()))
            .saturating_add(usize::from(self.has_non_empty_candidates()))
            .saturating_add(usize::from(self.will_import()))
            .saturating_add(usize::from(self.skipped_due_to_empty_candidates()))
            .saturating_add(usize::from(self.hit_import_limit))
            .saturating_add(usize::from(self.has_embedding_dimensions()))
    }

    pub fn has_import_signals(self) -> bool {
        self.import_signal_component_count() > 0
    }

    pub fn import_shape_problem_component_count(self) -> usize {
        usize::from(!self.enabled_matches_capacity())
            .saturating_add(usize::from(!self.candidate_counts_are_valid()))
            .saturating_add(usize::from(!self.planned_blocks_within_limit()))
            .saturating_add(usize::from(!self.planned_blocks_within_candidates()))
            .saturating_add(usize::from(!self.import_limit_flag_matches_shape()))
            .saturating_add(usize::from(!self.disabled_import_is_empty()))
            .saturating_add(usize::from(!self.embedding_dimensions_shape_is_valid()))
    }

    pub fn has_import_shape_problem_components(self) -> bool {
        self.import_shape_problem_component_count() > 0
    }

    pub fn import_accounting_is_consistent(self) -> bool {
        let expected_signal_count = usize::from(self.enabled)
            .saturating_add(usize::from(self.has_candidates()))
            .saturating_add(usize::from(self.has_non_empty_candidates()))
            .saturating_add(usize::from(self.will_import()))
            .saturating_add(usize::from(self.skipped_due_to_empty_candidates()))
            .saturating_add(usize::from(self.hit_import_limit))
            .saturating_add(usize::from(self.has_embedding_dimensions()));
        let expected_problem_count = usize::from(!self.enabled_matches_capacity())
            .saturating_add(usize::from(!self.candidate_counts_are_valid()))
            .saturating_add(usize::from(!self.planned_blocks_within_limit()))
            .saturating_add(usize::from(!self.planned_blocks_within_candidates()))
            .saturating_add(usize::from(!self.import_limit_flag_matches_shape()))
            .saturating_add(usize::from(!self.disabled_import_is_empty()))
            .saturating_add(usize::from(!self.embedding_dimensions_shape_is_valid()));

        self.import_signal_component_count() == expected_signal_count
            && self.import_shape_problem_component_count() == expected_problem_count
            && self.has_import_shape_problem_components() == (expected_problem_count > 0)
    }

    pub fn runtime_kv_import_commit_signal_component_count(self) -> usize {
        self.import_signal_component_count()
    }

    pub fn has_runtime_kv_import_commit_signals(self) -> bool {
        self.runtime_kv_import_commit_signal_component_count() > 0
    }

    pub fn runtime_kv_import_commit_blocker_component_count(self) -> usize {
        self.import_shape_problem_component_count()
    }

    pub fn has_runtime_kv_import_commit_blockers(self) -> bool {
        self.runtime_kv_import_commit_blocker_component_count() > 0
    }

    pub fn runtime_kv_import_commit_accounting_is_consistent(self) -> bool {
        self.import_accounting_is_consistent()
            && self.runtime_kv_import_commit_signal_component_count()
                == self.import_signal_component_count()
            && self.has_runtime_kv_import_commit_signals()
                == (self.runtime_kv_import_commit_signal_component_count() > 0)
            && self.runtime_kv_import_commit_blocker_component_count()
                == self.import_shape_problem_component_count()
            && self.has_runtime_kv_import_commit_blockers()
                == (self.runtime_kv_import_commit_blocker_component_count() > 0)
    }

    pub fn runtime_kv_import_commit_is_clean(self) -> bool {
        !self.has_runtime_kv_import_commit_blockers()
            && self.runtime_kv_import_commit_accounting_is_consistent()
    }

    pub fn import_commit_is_clean(self) -> bool {
        self.runtime_kv_import_commit_is_clean()
    }

    pub fn import_shape_is_clean(self) -> bool {
        self.runtime_kv_import_commit_is_clean()
    }

    pub fn can_commit_runtime_kv_import(self) -> bool {
        self.runtime_kv_import_commit_is_clean()
    }
}

impl RuntimeKvImportReadinessSummary {
    pub fn new(import: RuntimeKvImportSummary, blocks: RuntimeKvImportBlockSummary) -> Self {
        Self {
            import,
            blocks,
            import_signal_component_count: import.runtime_kv_import_commit_signal_component_count(),
            block_signal_component_count: blocks
                .runtime_kv_import_block_commit_signal_component_count(),
            import_blocker_component_count: import
                .runtime_kv_import_commit_blocker_component_count(),
            block_blocker_component_count: blocks
                .runtime_kv_import_block_commit_blocker_component_count()
                .saturating_add(usize::from(blocks.planned_blocks != import.planned_blocks)),
        }
    }

    pub fn stage_order() -> [RuntimeKvImportReadinessStage; 2] {
        [
            RuntimeKvImportReadinessStage::ImportPlan,
            RuntimeKvImportReadinessStage::ImportBlocks,
        ]
    }

    pub fn import_ready(self) -> bool {
        self.import.can_commit_runtime_kv_import()
    }

    pub fn import_block_plan_matches(self) -> bool {
        self.blocks.planned_blocks == self.import.planned_blocks
    }

    pub fn import_block_plan_drift_component_count(self) -> usize {
        usize::from(!self.import_block_plan_matches())
    }

    pub fn blocks_ready(self) -> bool {
        if self.import.will_import() {
            self.blocks.can_commit_runtime_kv_import_blocks() && self.import_block_plan_matches()
        } else {
            self.blocks.materialized_blocks == 0
                && self.import_block_plan_matches()
                && !self.blocks.has_runtime_kv_import_block_commit_blockers()
                && self
                    .blocks
                    .runtime_kv_import_block_commit_accounting_is_consistent()
        }
    }

    pub fn stage_ready(self, stage: RuntimeKvImportReadinessStage) -> bool {
        match stage {
            RuntimeKvImportReadinessStage::ImportPlan => self.import_ready(),
            RuntimeKvImportReadinessStage::ImportBlocks => self.blocks_ready(),
        }
    }

    pub fn stage_signal_component_count(self, stage: RuntimeKvImportReadinessStage) -> usize {
        match stage {
            RuntimeKvImportReadinessStage::ImportPlan => self.import_signal_component_count,
            RuntimeKvImportReadinessStage::ImportBlocks => self.block_signal_component_count,
        }
    }

    pub fn stage_blocker_component_count(self, stage: RuntimeKvImportReadinessStage) -> usize {
        match stage {
            RuntimeKvImportReadinessStage::ImportPlan => self.import_blocker_component_count,
            RuntimeKvImportReadinessStage::ImportBlocks => self.block_blocker_component_count,
        }
    }

    pub fn first_unready_stage(self) -> Option<RuntimeKvImportReadinessStage> {
        Self::stage_order()
            .into_iter()
            .find(|stage| !self.stage_ready(*stage))
    }

    pub fn first_blocking_stage(self) -> Option<RuntimeKvImportReadinessStage> {
        Self::stage_order()
            .into_iter()
            .find(|stage| self.stage_blocker_component_count(*stage) > 0)
    }

    pub fn runtime_kv_import_readiness_signal_component_count(self) -> usize {
        self.import_signal_component_count
            .saturating_add(self.block_signal_component_count)
    }

    pub fn has_runtime_kv_import_readiness_signals(self) -> bool {
        self.runtime_kv_import_readiness_signal_component_count() > 0
    }

    pub fn runtime_kv_import_readiness_blocker_component_count(self) -> usize {
        self.import_blocker_component_count
            .saturating_add(self.block_blocker_component_count)
    }

    pub fn has_runtime_kv_import_readiness_blockers(self) -> bool {
        self.runtime_kv_import_readiness_blocker_component_count() > 0
    }

    pub fn runtime_kv_import_readiness_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .import_signal_component_count
            .saturating_add(self.block_signal_component_count);
        let expected_blocker_count = self
            .import_blocker_component_count
            .saturating_add(self.block_blocker_component_count);

        self.import
            .runtime_kv_import_commit_accounting_is_consistent()
            && self
                .blocks
                .runtime_kv_import_block_commit_accounting_is_consistent()
            && self.import_signal_component_count
                == self
                    .import
                    .runtime_kv_import_commit_signal_component_count()
            && self.block_signal_component_count
                == self
                    .blocks
                    .runtime_kv_import_block_commit_signal_component_count()
            && self.import_blocker_component_count
                == self
                    .import
                    .runtime_kv_import_commit_blocker_component_count()
            && self.block_blocker_component_count
                == self
                    .blocks
                    .runtime_kv_import_block_commit_blocker_component_count()
                    .saturating_add(self.import_block_plan_drift_component_count())
            && self.runtime_kv_import_readiness_signal_component_count() == expected_signal_count
            && self.has_runtime_kv_import_readiness_signals() == (expected_signal_count > 0)
            && self.runtime_kv_import_readiness_blocker_component_count() == expected_blocker_count
            && self.has_runtime_kv_import_readiness_blockers() == (expected_blocker_count > 0)
    }

    pub fn runtime_kv_import_readiness_commit_is_clean(self) -> bool {
        !self.has_runtime_kv_import_readiness_blockers()
            && self.runtime_kv_import_readiness_accounting_is_consistent()
    }

    pub fn can_commit_runtime_kv_import_readiness(self) -> bool {
        self.runtime_kv_import_readiness_commit_is_clean()
            && self.import_ready()
            && self.blocks_ready()
    }

    pub fn runtime_kv_import_readiness_commit_action(self) -> RuntimeKvImportReadinessCommitAction {
        if self.can_commit_runtime_kv_import_readiness() {
            RuntimeKvImportReadinessCommitAction::CommitRuntimeKvImport
        } else {
            RuntimeKvImportReadinessCommitAction::ReturnRuntimeFailure
        }
    }

    pub fn component_accounting_drift_count(self) -> usize {
        usize::from(!self.runtime_kv_import_readiness_accounting_is_consistent())
    }

    pub fn runtime_kv_import_readiness_commit_problem_component_count(self) -> usize {
        self.runtime_kv_import_readiness_blocker_component_count()
            .saturating_add(self.component_accounting_drift_count())
    }

    pub fn has_runtime_kv_import_readiness_commit_problem_components(self) -> bool {
        self.runtime_kv_import_readiness_commit_problem_component_count() > 0
    }

    pub fn failure_report(self) -> Option<RuntimeFailureReport> {
        let component_count = self.runtime_kv_import_readiness_commit_problem_component_count();
        if component_count == 0 {
            None
        } else {
            Some(RuntimeFailureReport::kv_import(format!(
                "runtime kv import readiness failed: components={component_count}, first_blocking_stage={:?}",
                self.first_blocking_stage()
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

    pub fn commit_summary(self) -> RuntimeKvImportReadinessCommitSummary {
        RuntimeKvImportReadinessCommitSummary::new(self)
    }
}

impl RuntimeKvImportReadinessCommitSummary {
    pub fn new(readiness: RuntimeKvImportReadinessSummary) -> Self {
        let failure_reports = readiness.failure_reports();
        let primary_failure_report = failure_reports.first().cloned();
        let primary_failure_summary = primary_failure_report
            .as_ref()
            .map(|failure| failure.failure_summary());
        let failure_batch = RuntimeFailureReport::batch_summary(&failure_reports);
        let can_commit = readiness.can_commit_runtime_kv_import_readiness();
        let failure_report_count = failure_reports.len();
        let can_format_runtime_failures = failure_batch.can_format_runtime_failures();
        let should_return_failure = !can_commit && failure_report_count > 0;
        let action = readiness.runtime_kv_import_readiness_commit_action();

        Self {
            readiness,
            action,
            can_commit,
            should_return_failure,
            failure_reports,
            primary_failure_report,
            primary_failure_summary,
            failure_batch,
            failure_report_count,
            can_format_runtime_failures,
            total_signal_component_count: readiness
                .runtime_kv_import_readiness_signal_component_count(),
            total_blocker_component_count: readiness
                .runtime_kv_import_readiness_blocker_component_count(),
            component_accounting_consistent: readiness
                .runtime_kv_import_readiness_accounting_is_consistent(),
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

    pub fn failure_return_summary(&self) -> RuntimeKvExchangeFailureReturnSummary {
        RuntimeKvExchangeFailureReturnSummary::new(
            RuntimeKvExchangeFailureReturnSource::RuntimeKvImportReadiness,
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

    pub fn runtime_failure_return_report(&self) -> Option<RuntimeKvExchangeFailureReturnReport> {
        if self.failure_return_summary().can_return_runtime_failure() {
            self.primary_failure_report.clone().map(|failure| {
                RuntimeKvExchangeFailureReturnReport::new(
                    RuntimeKvExchangeFailureReturnSource::RuntimeKvImportReadiness,
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
        self.can_commit == self.readiness.can_commit_runtime_kv_import_readiness()
            && self.should_return_failure == (!self.can_commit && self.failure_report_count > 0)
            && self.action == self.readiness.runtime_kv_import_readiness_commit_action()
            && self.action_can_commit() == self.can_commit
            && self.action_should_return_failure() == self.should_return_failure
            && self.failure_report_count == self.failure_reports.len()
            && self.failure_report_count == self.readiness.failure_report_count()
            && self.primary_failure_report.as_ref() == self.failure_reports.first()
            && self.primary_failure_summary
                == self
                    .primary_failure_report
                    .as_ref()
                    .map(|failure| failure.failure_summary())
            && self.failure_batch == RuntimeFailureReport::batch_summary(&self.failure_reports)
            && self.can_format_runtime_failures == self.failure_batch.can_format_runtime_failures()
            && self.total_signal_component_count
                == self
                    .readiness
                    .runtime_kv_import_readiness_signal_component_count()
            && self.total_blocker_component_count
                == self
                    .readiness
                    .runtime_kv_import_readiness_blocker_component_count()
            && self.component_accounting_consistent
                == self
                    .readiness
                    .runtime_kv_import_readiness_accounting_is_consistent()
    }

    pub fn can_commit_runtime_kv_import(&self) -> bool {
        self.can_commit && self.commit_decision_accounting_is_consistent()
    }

    pub fn should_return_runtime_failure(&self) -> bool {
        self.should_return_failure && self.commit_decision_accounting_is_consistent()
    }
}

impl RuntimeKvImportBlockSummary {
    pub fn from_blocks(planned_blocks: usize, blocks: &[KvBlock]) -> Self {
        let block_summaries = blocks
            .iter()
            .map(KvBlock::shape_summary)
            .collect::<Vec<_>>();
        Self::from_block_summaries(planned_blocks, &block_summaries)
    }

    pub fn from_block_summaries(
        planned_blocks: usize,
        block_summaries: &[KvBlockShapeSummary],
    ) -> Self {
        let mut summary = Self {
            planned_blocks,
            materialized_blocks: block_summaries.len(),
            ..Self::default()
        };

        for block_summary in block_summaries {
            summary.runtime_namespace_blocks += usize::from(block_summary.is_runtime_namespace);
            summary.block_shape_signal_component_count = summary
                .block_shape_signal_component_count
                .saturating_add(block_summary.block_shape_signal_component_count());
            summary.block_shape_problem_component_count = summary
                .block_shape_problem_component_count
                .saturating_add(block_summary.runtime_exchange_shape_problem_component_count());
        }

        summary
    }

    pub fn is_empty(self) -> bool {
        self.materialized_blocks == 0
    }

    pub fn block_count_matches_plan(self) -> bool {
        self.materialized_blocks == self.planned_blocks
    }

    pub fn block_count_drift(self) -> usize {
        self.materialized_blocks.abs_diff(self.planned_blocks)
    }

    pub fn all_blocks_are_runtime_namespace(self) -> bool {
        self.materialized_blocks > 0 && self.runtime_namespace_blocks == self.materialized_blocks
    }

    pub fn runtime_namespace_drift_component_count(self) -> usize {
        usize::from(!self.is_empty() && !self.all_blocks_are_runtime_namespace())
    }

    pub fn block_count_drift_component_count(self) -> usize {
        usize::from(!self.block_count_matches_plan())
    }

    pub fn import_block_problem_component_count(self) -> usize {
        self.block_count_drift_component_count()
            .saturating_add(self.block_shape_problem_component_count)
    }

    pub fn has_import_block_problem_components(self) -> bool {
        self.import_block_problem_component_count() > 0
    }

    pub fn has_import_block_signals(self) -> bool {
        self.block_shape_signal_component_count > 0
    }

    pub fn import_block_accounting_is_consistent(self) -> bool {
        let expected_problem_count = usize::from(!self.block_count_matches_plan())
            .saturating_add(self.block_shape_problem_component_count);

        self.import_block_problem_component_count() == expected_problem_count
            && self.has_import_block_problem_components() == (expected_problem_count > 0)
            && self.has_import_block_signals() == (self.block_shape_signal_component_count > 0)
    }

    pub fn runtime_kv_import_block_commit_signal_component_count(self) -> usize {
        self.block_shape_signal_component_count
    }

    pub fn has_runtime_kv_import_block_commit_signals(self) -> bool {
        self.runtime_kv_import_block_commit_signal_component_count() > 0
    }

    pub fn runtime_kv_import_block_commit_blocker_component_count(self) -> usize {
        self.import_block_problem_component_count()
    }

    pub fn has_runtime_kv_import_block_commit_blockers(self) -> bool {
        self.runtime_kv_import_block_commit_blocker_component_count() > 0
    }

    pub fn runtime_kv_import_block_commit_accounting_is_consistent(self) -> bool {
        self.import_block_accounting_is_consistent()
            && self.runtime_kv_import_block_commit_signal_component_count()
                == self.block_shape_signal_component_count
            && self.has_runtime_kv_import_block_commit_signals()
                == (self.runtime_kv_import_block_commit_signal_component_count() > 0)
            && self.runtime_kv_import_block_commit_blocker_component_count()
                == self.import_block_problem_component_count()
            && self.has_runtime_kv_import_block_commit_blockers()
                == (self.runtime_kv_import_block_commit_blocker_component_count() > 0)
    }

    pub fn runtime_kv_import_block_commit_is_clean(self) -> bool {
        self.block_count_matches_plan()
            && !self.has_runtime_kv_import_block_commit_blockers()
            && self.runtime_kv_import_block_commit_accounting_is_consistent()
    }

    pub fn import_block_shape_is_clean(self) -> bool {
        self.runtime_kv_import_block_commit_is_clean()
    }

    pub fn can_commit_runtime_kv_import_blocks(self) -> bool {
        !self.is_empty() && self.runtime_kv_import_block_commit_is_clean()
    }
}

impl RuntimeKvImportManifestPlanSummary {
    pub fn manifest_allows_import(self) -> bool {
        self.manifest_import_enabled && self.manifest_max_import_blocks > 0
    }

    pub fn runtime_allows_import(self) -> bool {
        self.runtime_import_enabled && self.runtime_max_import_blocks > 0
    }

    pub fn requested_prefetch(self) -> bool {
        self.requested_prefetch_blocks > 0
    }

    pub fn plan_will_import(self) -> bool {
        self.import_plan_max_blocks > 0
    }

    pub fn has_embedding_dimensions(self) -> bool {
        self.embedding_dimensions.is_some()
    }

    pub fn architecture_has_import_shape(self) -> bool {
        self.architecture_layer_count > 0 && self.architecture_kv_heads > 0
    }

    pub fn manifest_import_capability_is_consistent(self) -> bool {
        self.manifest_import_enabled == (self.manifest_max_import_blocks > 0)
    }

    pub fn runtime_import_capability_is_consistent(self) -> bool {
        self.runtime_import_enabled == (self.runtime_max_import_blocks > 0)
    }

    pub fn import_plan_within_manifest_limit(self) -> bool {
        self.import_plan_max_blocks <= self.manifest_max_import_blocks
    }

    pub fn import_plan_within_runtime_limit(self) -> bool {
        self.import_plan_max_blocks <= self.runtime_max_import_blocks
    }

    pub fn import_plan_within_requested_limit(self) -> bool {
        self.import_plan_max_blocks <= self.requested_prefetch_blocks
    }

    pub fn requested_prefetch_without_manifest_capacity(self) -> bool {
        self.requested_prefetch() && !self.manifest_allows_import()
    }

    pub fn requested_prefetch_without_runtime_capacity(self) -> bool {
        self.requested_prefetch() && !self.runtime_allows_import()
    }

    pub fn manifest_import_signal_component_count(self) -> usize {
        usize::from(self.manifest_import_enabled) + usize::from(self.manifest_max_import_blocks > 0)
    }

    pub fn runtime_import_signal_component_count(self) -> usize {
        usize::from(self.runtime_import_enabled) + usize::from(self.runtime_max_import_blocks > 0)
    }

    pub fn requested_prefetch_signal_component_count(self) -> usize {
        usize::from(self.requested_prefetch())
    }

    pub fn import_plan_signal_component_count(self) -> usize {
        usize::from(self.plan_will_import())
    }

    pub fn embedding_signal_component_count(self) -> usize {
        usize::from(self.has_embedding_dimensions())
    }

    pub fn architecture_import_signal_component_count(self) -> usize {
        usize::from(self.architecture_layer_count > 0) + usize::from(self.architecture_kv_heads > 0)
    }

    pub fn manifest_bridge_signal_component_count(self) -> usize {
        self.manifest_import_signal_component_count()
            .saturating_add(self.runtime_import_signal_component_count())
            .saturating_add(self.requested_prefetch_signal_component_count())
            .saturating_add(self.import_plan_signal_component_count())
            .saturating_add(self.embedding_signal_component_count())
            .saturating_add(self.architecture_import_signal_component_count())
    }

    pub fn has_manifest_bridge_signals(self) -> bool {
        self.manifest_bridge_signal_component_count() > 0
    }

    pub fn manifest_import_capability_problem_component_count(self) -> usize {
        usize::from(!self.manifest_import_capability_is_consistent())
    }

    pub fn runtime_import_capability_problem_component_count(self) -> usize {
        usize::from(!self.runtime_import_capability_is_consistent())
    }

    pub fn requested_prefetch_capacity_problem_component_count(self) -> usize {
        usize::from(self.requested_prefetch_without_manifest_capacity()).saturating_add(
            usize::from(self.requested_prefetch_without_runtime_capacity()),
        )
    }

    pub fn import_plan_limit_problem_component_count(self) -> usize {
        usize::from(!self.import_plan_within_manifest_limit())
            .saturating_add(usize::from(!self.import_plan_within_runtime_limit()))
            .saturating_add(usize::from(!self.import_plan_within_requested_limit()))
    }

    pub fn embedding_shape_problem_component_count(self) -> usize {
        0
    }

    pub fn architecture_import_shape_problem_component_count(self) -> usize {
        usize::from(self.architecture_layer_count == 0)
            .saturating_add(usize::from(self.architecture_kv_heads == 0))
    }

    pub fn manifest_bridge_problem_component_count(self) -> usize {
        self.manifest_import_capability_problem_component_count()
            .saturating_add(self.runtime_import_capability_problem_component_count())
            .saturating_add(self.requested_prefetch_capacity_problem_component_count())
            .saturating_add(self.import_plan_limit_problem_component_count())
            .saturating_add(self.embedding_shape_problem_component_count())
            .saturating_add(self.architecture_import_shape_problem_component_count())
    }

    pub fn has_manifest_bridge_problem_components(self) -> bool {
        self.manifest_bridge_problem_component_count() > 0
    }

    pub fn manifest_bridge_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .manifest_import_signal_component_count()
            .saturating_add(self.runtime_import_signal_component_count())
            .saturating_add(self.requested_prefetch_signal_component_count())
            .saturating_add(self.import_plan_signal_component_count())
            .saturating_add(self.embedding_signal_component_count())
            .saturating_add(self.architecture_import_signal_component_count());
        let expected_problem_count = self
            .manifest_import_capability_problem_component_count()
            .saturating_add(self.runtime_import_capability_problem_component_count())
            .saturating_add(self.requested_prefetch_capacity_problem_component_count())
            .saturating_add(self.import_plan_limit_problem_component_count())
            .saturating_add(self.embedding_shape_problem_component_count())
            .saturating_add(self.architecture_import_shape_problem_component_count());

        self.manifest_bridge_signal_component_count() == expected_signal_count
            && self.has_manifest_bridge_signals() == (expected_signal_count > 0)
            && self.manifest_bridge_problem_component_count() == expected_problem_count
            && self.has_manifest_bridge_problem_components() == (expected_problem_count > 0)
    }

    pub fn manifest_bridge_shape_is_clean(self) -> bool {
        !self.has_manifest_bridge_problem_components()
            && self.manifest_bridge_accounting_is_consistent()
    }

    pub fn can_use_manifest_runtime_kv_import_plan(self) -> bool {
        self.manifest_bridge_shape_is_clean()
    }
}

impl RuntimeKvImportPlan {
    pub fn new(
        runtime: &RuntimeMetadata,
        architecture: TransformerRuntimeArchitecture,
        requested_prefetch_blocks: usize,
    ) -> Self {
        let manifest_limit = if !runtime.supports_kv_import {
            0
        } else if runtime.max_kv_import_blocks > 0 {
            runtime.max_kv_import_blocks
        } else {
            requested_prefetch_blocks
        };
        let max_blocks = requested_prefetch_blocks.min(manifest_limit);
        let embedding_dimensions =
            (runtime.embedding_dimensions > 0).then_some(runtime.embedding_dimensions);

        Self {
            max_blocks,
            embedding_dimensions,
            layer_count: architecture.layer_count.max(1),
            kv_heads: architecture.kv_heads.max(1),
        }
    }

    pub fn from_manifest(
        manifest: &RuntimeManifestDigest,
        requested_prefetch_blocks: usize,
    ) -> Self {
        let metadata = manifest.runtime_metadata();
        let mut plan = Self::new(&metadata, manifest.architecture, requested_prefetch_blocks);

        plan.max_blocks = if manifest.kv_policy.import_enabled {
            plan.max_blocks.min(manifest.kv_policy.max_import_blocks)
        } else {
            0
        };
        plan
    }

    pub fn manifest_plan_summary(
        manifest: &RuntimeManifestDigest,
        requested_prefetch_blocks: usize,
    ) -> RuntimeKvImportManifestPlanSummary {
        let metadata = manifest.runtime_metadata();
        let plan = Self::from_manifest(manifest, requested_prefetch_blocks);

        RuntimeKvImportManifestPlanSummary {
            manifest_import_enabled: manifest.kv_policy.import_enabled,
            manifest_max_import_blocks: manifest.kv_policy.max_import_blocks,
            runtime_import_enabled: metadata.supports_kv_import,
            runtime_max_import_blocks: metadata.max_kv_import_blocks,
            requested_prefetch_blocks,
            import_plan_max_blocks: plan.max_blocks,
            embedding_dimensions: plan.embedding_dimensions,
            architecture_layer_count: manifest.architecture.layer_count,
            architecture_kv_heads: manifest.architecture.kv_heads,
        }
    }

    pub fn is_enabled(self) -> bool {
        self.max_blocks > 0
    }

    pub fn planned_block_count(self, candidates: &[RuntimeKvCandidate]) -> usize {
        if !self.is_enabled() {
            0
        } else {
            candidates
                .iter()
                .filter(|candidate| !candidate.vector.is_empty())
                .count()
                .min(self.max_blocks)
        }
    }

    pub fn import_summary(self, candidates: &[RuntimeKvCandidate]) -> RuntimeKvImportSummary {
        let non_empty_candidate_count = candidates
            .iter()
            .filter(|candidate| !candidate.vector.is_empty())
            .count();
        let planned_blocks = self.planned_block_count(candidates);

        RuntimeKvImportSummary {
            enabled: self.is_enabled(),
            max_blocks: self.max_blocks,
            candidate_count: candidates.len(),
            non_empty_candidate_count,
            planned_blocks,
            hit_import_limit: planned_blocks > 0
                && planned_blocks == self.max_blocks
                && non_empty_candidate_count >= self.max_blocks,
            embedding_dimensions: self.embedding_dimensions,
        }
    }

    pub fn build_blocks(self, candidates: &[RuntimeKvCandidate]) -> Vec<KvBlock> {
        if !self.is_enabled() {
            return Vec::new();
        }

        candidates
            .iter()
            .filter(|candidate| !candidate.vector.is_empty())
            .take(self.max_blocks)
            .enumerate()
            .map(|(index, candidate)| {
                let key = fit_runtime_vector(&candidate.vector, self.embedding_dimensions);
                let weighted = candidate
                    .vector
                    .iter()
                    .map(|value| {
                        if value.is_finite() {
                            value * candidate.weight
                        } else {
                            0.0
                        }
                    })
                    .collect::<Vec<_>>();
                let value = fit_runtime_vector(&weighted, self.embedding_dimensions);

                KvBlock::new(
                    candidate.id,
                    KvNamespace::Runtime,
                    (index / self.kv_heads) % self.layer_count,
                    index % self.kv_heads,
                    index..index + 1,
                    key,
                    value,
                )
                .with_score(candidate.weight.min(1.0))
                .with_reinforcement((candidate.weight - 1.0).max(0.0))
            })
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeKvDirection {
    Imported,
    Exported,
}

impl RuntimeKvDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Imported => "imported",
            Self::Exported => "exported",
        }
    }

    pub fn failure_trace_label(self) -> &'static str {
        match self {
            Self::Imported => "runtime_kv_import_error",
            Self::Exported => "runtime_kv_export_error",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeKvBlockContract {
    pub max_blocks: usize,
    pub token_upper_bound: usize,
    pub direction: RuntimeKvDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeKvBlockContractSummary {
    pub max_blocks: usize,
    pub token_upper_bound: usize,
    pub direction: RuntimeKvDirection,
    pub direction_label: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeKvBlockContractCheckSummary {
    pub block_index: usize,
    pub direction: RuntimeKvDirection,
    pub layer_count: usize,
    pub kv_heads: usize,
    pub token_upper_bound: usize,
    pub vector_bound: usize,
    pub namespace_is_runtime: bool,
    pub layer_within_bounds: bool,
    pub head_within_bounds: bool,
    pub token_range_is_valid: bool,
    pub token_end_within_bound: bool,
    pub vectors_are_present: bool,
    pub key_value_len_matches: bool,
    pub vector_len_within_bound: bool,
    pub key_values_are_finite: bool,
    pub value_values_are_finite: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeKvValidationBoundarySummary {
    pub direction: RuntimeKvDirection,
    pub direction_label: &'static str,
    pub failure_trace_label: &'static str,
    pub max_blocks: usize,
    pub token_upper_bound: usize,
    pub accepted_count: usize,
    pub violation_count: usize,
    pub valid: bool,
}

impl RuntimeKvBlockContract {
    pub fn new(max_blocks: usize, token_upper_bound: usize, direction: RuntimeKvDirection) -> Self {
        Self {
            max_blocks,
            token_upper_bound,
            direction,
        }
    }

    pub fn contract_summary(self) -> RuntimeKvBlockContractSummary {
        RuntimeKvBlockContractSummary {
            max_blocks: self.max_blocks,
            token_upper_bound: self.token_upper_bound,
            direction: self.direction,
            direction_label: self.direction.as_str(),
        }
    }

    pub fn validation_boundary_summary(
        self,
        report: &RuntimeKvValidationReport,
    ) -> RuntimeKvValidationBoundarySummary {
        RuntimeKvValidationBoundarySummary {
            direction: self.direction,
            direction_label: self.direction.as_str(),
            failure_trace_label: self.direction.failure_trace_label(),
            max_blocks: self.max_blocks,
            token_upper_bound: self.token_upper_bound,
            accepted_count: report.accepted.len(),
            violation_count: report.violations.len(),
            valid: report.is_valid(),
        }
    }

    pub fn for_request_imports(request: &RuntimeRequestEnvelope) -> Self {
        Self::new(
            request.imported_kv_blocks,
            request.generation_budget.planned_context_tokens,
            RuntimeKvDirection::Imported,
        )
    }

    pub fn for_request_exports(request: &RuntimeRequestEnvelope) -> Self {
        let max_blocks = if !request.runtime.supports_kv_export {
            0
        } else if let Some(planning) = request.planning {
            planning.planned_kv_exchange().export_blocks
        } else {
            request.runtime.max_kv_export_blocks
        };

        Self::new(
            max_blocks,
            request.generation_budget.planned_context_tokens,
            RuntimeKvDirection::Exported,
        )
    }

    pub fn validate_blocks(
        self,
        blocks: &[KvBlock],
        runtime: &RuntimeMetadata,
        architecture: TransformerRuntimeArchitecture,
    ) -> RuntimeKvValidationReport {
        let mut accepted = Vec::new();
        let mut violations = Vec::new();

        if blocks.len() > self.max_blocks {
            violations.push(format!(
                "{} KV block count {} exceeds contract max_blocks {}",
                self.direction.as_str(),
                blocks.len(),
                self.max_blocks
            ));
        }

        for (index, block) in blocks.iter().take(self.max_blocks).enumerate() {
            let block_violations = self.validate_block(index, block, runtime, architecture);
            if block_violations.is_empty() {
                accepted.push(block.clone());
            } else {
                violations.extend(block_violations);
            }
        }

        RuntimeKvValidationReport {
            accepted,
            violations,
        }
    }

    pub fn validate_block(
        self,
        index: usize,
        block: &KvBlock,
        runtime: &RuntimeMetadata,
        architecture: TransformerRuntimeArchitecture,
    ) -> Vec<String> {
        let mut violations = Vec::new();
        let layer_count = architecture.layer_count;
        let kv_heads = architecture.kv_heads;
        let token_span = block.token_len().max(1);
        let per_token_vector_bound = architecture
            .hidden_size
            .max(runtime.embedding_dimensions)
            .max(1);
        let vector_bound = per_token_vector_bound.saturating_mul(token_span);
        let prefix = format!(
            "{} KV block {} for model_id={}",
            self.direction.as_str(),
            index,
            runtime.model_id
        );

        if block.namespace != KvNamespace::Runtime {
            violations.push(format!(
                "{prefix}: namespace {} is not runtime",
                block.namespace.label()
            ));
        }
        if layer_count == 0 || block.layer >= layer_count {
            violations.push(format!(
                "{prefix}: layer {} exceeds manifest layer_count {}",
                block.layer, layer_count
            ));
        }
        if kv_heads == 0 || block.head >= kv_heads {
            violations.push(format!(
                "{prefix}: head {} exceeds manifest kv_heads {}",
                block.head, kv_heads
            ));
        }
        if block.token_start >= block.token_end {
            violations.push(format!(
                "{prefix}: token range {}..{} is empty or reversed",
                block.token_start, block.token_end
            ));
        }
        if block.token_end > self.token_upper_bound {
            violations.push(format!(
                "{prefix}: token_end {} exceeds KV token bound {}",
                block.token_end, self.token_upper_bound
            ));
        }
        if block.key.is_empty() || block.value.is_empty() {
            violations.push(format!(
                "{prefix}: key and value vectors must both be non-empty"
            ));
        }
        if block.key.len() != block.value.len() {
            violations.push(format!(
                "{prefix}: key/value dimensions differ: key={} value={}",
                block.key.len(),
                block.value.len()
            ));
        }
        if block.key.len() > vector_bound {
            violations.push(format!(
                "{prefix}: key/value dimensions {} exceed per-block bound {}",
                block.key.len(),
                vector_bound
            ));
        }
        if !block.key.iter().all(|value| value.is_finite()) {
            violations.push(format!(
                "{prefix}: {} key contains non-finite value",
                self.direction.as_str()
            ));
        }
        if !block.value.iter().all(|value| value.is_finite()) {
            violations.push(format!(
                "{prefix}: {} value contains non-finite value",
                self.direction.as_str()
            ));
        }

        violations
    }

    pub fn block_check_summary(
        self,
        index: usize,
        block: &KvBlock,
        runtime: &RuntimeMetadata,
        architecture: TransformerRuntimeArchitecture,
    ) -> RuntimeKvBlockContractCheckSummary {
        let layer_count = architecture.layer_count;
        let kv_heads = architecture.kv_heads;
        let token_span = block.token_len().max(1);
        let per_token_vector_bound = architecture
            .hidden_size
            .max(runtime.embedding_dimensions)
            .max(1);
        let vector_bound = per_token_vector_bound.saturating_mul(token_span);

        RuntimeKvBlockContractCheckSummary {
            block_index: index,
            direction: self.direction,
            layer_count,
            kv_heads,
            token_upper_bound: self.token_upper_bound,
            vector_bound,
            namespace_is_runtime: block.namespace == KvNamespace::Runtime,
            layer_within_bounds: layer_count > 0 && block.layer < layer_count,
            head_within_bounds: kv_heads > 0 && block.head < kv_heads,
            token_range_is_valid: block.token_start < block.token_end,
            token_end_within_bound: block.token_end <= self.token_upper_bound,
            vectors_are_present: !block.key.is_empty() && !block.value.is_empty(),
            key_value_len_matches: block.key.len() == block.value.len(),
            vector_len_within_bound: block.key.len() <= vector_bound,
            key_values_are_finite: block.key.iter().all(|value| value.is_finite()),
            value_values_are_finite: block.value.iter().all(|value| value.is_finite()),
        }
    }
}

impl RuntimeKvBlockContractSummary {
    pub fn has_block_capacity(self) -> bool {
        self.max_blocks > 0
    }

    pub fn has_token_bound(self) -> bool {
        self.token_upper_bound > 0
    }

    pub fn direction_label_matches_kind(self) -> bool {
        self.direction_label == self.direction.as_str()
    }

    pub fn contract_signal_component_count(self) -> usize {
        usize::from(self.has_block_capacity())
            .saturating_add(usize::from(self.has_token_bound()))
            .saturating_add(usize::from(self.direction == RuntimeKvDirection::Imported))
            .saturating_add(usize::from(self.direction == RuntimeKvDirection::Exported))
    }

    pub fn has_contract_signals(self) -> bool {
        self.contract_signal_component_count() > 0
    }

    pub fn contract_problem_component_count(self) -> usize {
        usize::from(!self.has_token_bound())
            .saturating_add(usize::from(!self.direction_label_matches_kind()))
    }

    pub fn has_contract_problem_components(self) -> bool {
        self.contract_problem_component_count() > 0
    }

    pub fn contract_accounting_is_consistent(self) -> bool {
        let expected_signal_count = usize::from(self.has_block_capacity())
            .saturating_add(usize::from(self.has_token_bound()))
            .saturating_add(usize::from(self.direction == RuntimeKvDirection::Imported))
            .saturating_add(usize::from(self.direction == RuntimeKvDirection::Exported));
        let expected_problem_count = usize::from(!self.has_token_bound())
            .saturating_add(usize::from(!self.direction_label_matches_kind()));

        self.contract_signal_component_count() == expected_signal_count
            && self.has_contract_signals() == (expected_signal_count > 0)
            && self.contract_problem_component_count() == expected_problem_count
            && self.has_contract_problem_components() == (expected_problem_count > 0)
    }

    pub fn runtime_kv_block_contract_commit_signal_component_count(self) -> usize {
        self.contract_signal_component_count()
    }

    pub fn has_runtime_kv_block_contract_commit_signals(self) -> bool {
        self.runtime_kv_block_contract_commit_signal_component_count() > 0
    }

    pub fn runtime_kv_block_contract_commit_blocker_component_count(self) -> usize {
        self.contract_problem_component_count()
    }

    pub fn has_runtime_kv_block_contract_commit_blockers(self) -> bool {
        self.runtime_kv_block_contract_commit_blocker_component_count() > 0
    }

    pub fn runtime_kv_block_contract_commit_accounting_is_consistent(self) -> bool {
        self.contract_accounting_is_consistent()
            && self.runtime_kv_block_contract_commit_signal_component_count()
                == self.contract_signal_component_count()
            && self.has_runtime_kv_block_contract_commit_signals()
                == (self.runtime_kv_block_contract_commit_signal_component_count() > 0)
            && self.runtime_kv_block_contract_commit_blocker_component_count()
                == self.contract_problem_component_count()
            && self.has_runtime_kv_block_contract_commit_blockers()
                == (self.runtime_kv_block_contract_commit_blocker_component_count() > 0)
    }

    pub fn runtime_kv_block_contract_commit_is_clean(self) -> bool {
        !self.has_runtime_kv_block_contract_commit_blockers()
            && self.runtime_kv_block_contract_commit_accounting_is_consistent()
    }

    pub fn contract_shape_is_clean(self) -> bool {
        self.runtime_kv_block_contract_commit_is_clean()
    }

    pub fn can_commit_runtime_kv_block_contract(self) -> bool {
        self.has_block_capacity() && self.runtime_kv_block_contract_commit_is_clean()
    }

    pub fn can_use_runtime_kv_block_contract(self) -> bool {
        self.can_commit_runtime_kv_block_contract()
    }
}

impl RuntimeKvBlockContractCheckSummary {
    pub fn namespace_problem_component_count(self) -> usize {
        usize::from(!self.namespace_is_runtime)
    }

    pub fn layer_head_problem_component_count(self) -> usize {
        usize::from(!self.layer_within_bounds).saturating_add(usize::from(!self.head_within_bounds))
    }

    pub fn token_problem_component_count(self) -> usize {
        usize::from(!self.token_range_is_valid)
            .saturating_add(usize::from(!self.token_end_within_bound))
    }

    pub fn vector_problem_component_count(self) -> usize {
        usize::from(!self.vectors_are_present)
            .saturating_add(usize::from(!self.key_value_len_matches))
            .saturating_add(usize::from(!self.vector_len_within_bound))
            .saturating_add(usize::from(!self.key_values_are_finite))
            .saturating_add(usize::from(!self.value_values_are_finite))
    }

    pub fn contract_check_problem_component_count(self) -> usize {
        self.namespace_problem_component_count()
            .saturating_add(self.layer_head_problem_component_count())
            .saturating_add(self.token_problem_component_count())
            .saturating_add(self.vector_problem_component_count())
    }

    pub fn has_contract_check_problem_components(self) -> bool {
        self.contract_check_problem_component_count() > 0
    }

    pub fn contract_check_signal_component_count(self) -> usize {
        usize::from(self.namespace_is_runtime)
            .saturating_add(usize::from(self.layer_within_bounds))
            .saturating_add(usize::from(self.head_within_bounds))
            .saturating_add(usize::from(self.token_range_is_valid))
            .saturating_add(usize::from(self.token_end_within_bound))
            .saturating_add(usize::from(self.vectors_are_present))
            .saturating_add(usize::from(self.key_value_len_matches))
            .saturating_add(usize::from(self.vector_len_within_bound))
            .saturating_add(usize::from(self.key_values_are_finite))
            .saturating_add(usize::from(self.value_values_are_finite))
    }

    pub fn has_contract_check_signals(self) -> bool {
        self.contract_check_signal_component_count() > 0
    }

    pub fn contract_check_accounting_is_consistent(self) -> bool {
        let expected_signal_count = usize::from(self.namespace_is_runtime)
            .saturating_add(usize::from(self.layer_within_bounds))
            .saturating_add(usize::from(self.head_within_bounds))
            .saturating_add(usize::from(self.token_range_is_valid))
            .saturating_add(usize::from(self.token_end_within_bound))
            .saturating_add(usize::from(self.vectors_are_present))
            .saturating_add(usize::from(self.key_value_len_matches))
            .saturating_add(usize::from(self.vector_len_within_bound))
            .saturating_add(usize::from(self.key_values_are_finite))
            .saturating_add(usize::from(self.value_values_are_finite));
        let expected_problem_count = usize::from(!self.namespace_is_runtime)
            .saturating_add(usize::from(!self.layer_within_bounds))
            .saturating_add(usize::from(!self.head_within_bounds))
            .saturating_add(usize::from(!self.token_range_is_valid))
            .saturating_add(usize::from(!self.token_end_within_bound))
            .saturating_add(usize::from(!self.vectors_are_present))
            .saturating_add(usize::from(!self.key_value_len_matches))
            .saturating_add(usize::from(!self.vector_len_within_bound))
            .saturating_add(usize::from(!self.key_values_are_finite))
            .saturating_add(usize::from(!self.value_values_are_finite));

        self.contract_check_signal_component_count() == expected_signal_count
            && self.has_contract_check_signals() == (expected_signal_count > 0)
            && self.contract_check_problem_component_count() == expected_problem_count
            && self.has_contract_check_problem_components() == (expected_problem_count > 0)
    }

    pub fn runtime_kv_block_contract_check_commit_signal_component_count(self) -> usize {
        self.contract_check_signal_component_count()
    }

    pub fn has_runtime_kv_block_contract_check_commit_signals(self) -> bool {
        self.runtime_kv_block_contract_check_commit_signal_component_count() > 0
    }

    pub fn runtime_kv_block_contract_check_commit_blocker_component_count(self) -> usize {
        self.contract_check_problem_component_count()
    }

    pub fn has_runtime_kv_block_contract_check_commit_blockers(self) -> bool {
        self.runtime_kv_block_contract_check_commit_blocker_component_count() > 0
    }

    pub fn runtime_kv_block_contract_check_commit_accounting_is_consistent(self) -> bool {
        self.contract_check_accounting_is_consistent()
            && self.runtime_kv_block_contract_check_commit_signal_component_count()
                == self.contract_check_signal_component_count()
            && self.has_runtime_kv_block_contract_check_commit_signals()
                == (self.runtime_kv_block_contract_check_commit_signal_component_count() > 0)
            && self.runtime_kv_block_contract_check_commit_blocker_component_count()
                == self.contract_check_problem_component_count()
            && self.has_runtime_kv_block_contract_check_commit_blockers()
                == (self.runtime_kv_block_contract_check_commit_blocker_component_count() > 0)
    }

    pub fn runtime_kv_block_contract_check_commit_is_clean(self) -> bool {
        !self.has_runtime_kv_block_contract_check_commit_blockers()
            && self.runtime_kv_block_contract_check_commit_accounting_is_consistent()
    }

    pub fn contract_check_shape_is_clean(self) -> bool {
        self.runtime_kv_block_contract_check_commit_is_clean()
    }

    pub fn can_commit_runtime_kv_block_contract_check(self) -> bool {
        self.runtime_kv_block_contract_check_commit_is_clean()
    }

    pub fn can_accept_runtime_kv_block(self) -> bool {
        self.can_commit_runtime_kv_block_contract_check()
    }
}

impl RuntimeKvValidationBoundarySummary {
    pub fn direction_label_matches_kind(self) -> bool {
        self.direction_label == self.direction.as_str()
    }

    pub fn failure_trace_label_matches_direction(self) -> bool {
        self.failure_trace_label == self.direction.failure_trace_label()
    }

    pub fn has_block_capacity(self) -> bool {
        self.max_blocks > 0
    }

    pub fn has_token_bound(self) -> bool {
        self.token_upper_bound > 0
    }

    pub fn accepted_any(self) -> bool {
        self.accepted_count > 0
    }

    pub fn has_violations(self) -> bool {
        self.violation_count > 0
    }

    pub fn rejected_all(self) -> bool {
        self.has_violations() && self.accepted_count == 0
    }

    pub fn partially_accepted(self) -> bool {
        self.accepted_any() && self.has_violations()
    }

    pub fn accepted_within_contract_limit(self) -> bool {
        self.accepted_count <= self.max_blocks
    }

    pub fn valid_flag_matches_violations(self) -> bool {
        self.valid == !self.has_violations()
    }

    pub fn maps_to_runtime_kv_failure(self) -> bool {
        self.has_violations() && self.failure_trace_label_matches_direction()
    }

    pub fn boundary_signal_component_count(self) -> usize {
        usize::from(self.has_block_capacity())
            .saturating_add(usize::from(self.has_token_bound()))
            .saturating_add(usize::from(self.accepted_any()))
            .saturating_add(usize::from(self.rejected_all()))
            .saturating_add(usize::from(self.partially_accepted()))
            .saturating_add(usize::from(self.maps_to_runtime_kv_failure()))
    }

    pub fn has_boundary_signals(self) -> bool {
        self.boundary_signal_component_count() > 0
    }

    pub fn boundary_problem_component_count(self) -> usize {
        usize::from(!self.direction_label_matches_kind())
            .saturating_add(usize::from(!self.failure_trace_label_matches_direction()))
            .saturating_add(usize::from(!self.accepted_within_contract_limit()))
            .saturating_add(usize::from(!self.valid_flag_matches_violations()))
            .saturating_add(usize::from(self.has_violations()))
    }

    pub fn has_boundary_problem_components(self) -> bool {
        self.boundary_problem_component_count() > 0
    }

    pub fn boundary_accounting_is_consistent(self) -> bool {
        let expected_signal_count = usize::from(self.has_block_capacity())
            .saturating_add(usize::from(self.has_token_bound()))
            .saturating_add(usize::from(self.accepted_any()))
            .saturating_add(usize::from(self.rejected_all()))
            .saturating_add(usize::from(self.partially_accepted()))
            .saturating_add(usize::from(self.maps_to_runtime_kv_failure()));
        let expected_problem_count = usize::from(!self.direction_label_matches_kind())
            .saturating_add(usize::from(!self.failure_trace_label_matches_direction()))
            .saturating_add(usize::from(!self.accepted_within_contract_limit()))
            .saturating_add(usize::from(!self.valid_flag_matches_violations()))
            .saturating_add(usize::from(self.has_violations()));

        self.boundary_signal_component_count() == expected_signal_count
            && self.has_boundary_signals() == (expected_signal_count > 0)
            && self.boundary_problem_component_count() == expected_problem_count
            && self.has_boundary_problem_components() == (expected_problem_count > 0)
    }

    pub fn runtime_kv_boundary_commit_signal_component_count(self) -> usize {
        self.boundary_signal_component_count()
    }

    pub fn has_runtime_kv_boundary_commit_signals(self) -> bool {
        self.runtime_kv_boundary_commit_signal_component_count() > 0
    }

    pub fn runtime_kv_boundary_commit_blocker_component_count(self) -> usize {
        self.boundary_problem_component_count()
    }

    pub fn has_runtime_kv_boundary_commit_blockers(self) -> bool {
        self.runtime_kv_boundary_commit_blocker_component_count() > 0
    }

    pub fn runtime_kv_boundary_commit_accounting_is_consistent(self) -> bool {
        self.boundary_accounting_is_consistent()
            && self.runtime_kv_boundary_commit_signal_component_count()
                == self.boundary_signal_component_count()
            && self.has_runtime_kv_boundary_commit_signals()
                == (self.runtime_kv_boundary_commit_signal_component_count() > 0)
            && self.runtime_kv_boundary_commit_blocker_component_count()
                == self.boundary_problem_component_count()
            && self.has_runtime_kv_boundary_commit_blockers()
                == (self.runtime_kv_boundary_commit_blocker_component_count() > 0)
    }

    pub fn runtime_kv_boundary_commit_is_clean(self) -> bool {
        !self.has_runtime_kv_boundary_commit_blockers()
            && self.runtime_kv_boundary_commit_accounting_is_consistent()
    }

    pub fn boundary_shape_is_clean(self) -> bool {
        self.runtime_kv_boundary_commit_is_clean()
    }

    pub fn can_commit_runtime_kv_boundary(self) -> bool {
        self.valid && self.boundary_shape_is_clean()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeKvValidationReport {
    pub accepted: Vec<KvBlock>,
    pub violations: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeKvValidationSummary {
    pub accepted_count: usize,
    pub violation_count: usize,
    pub valid: bool,
}

impl RuntimeKvValidationSummary {
    pub fn has_violations(self) -> bool {
        self.violation_count > 0
    }

    pub fn accepted_any(self) -> bool {
        self.accepted_count > 0
    }

    pub fn rejected_all(self) -> bool {
        self.violation_count > 0 && self.accepted_count == 0
    }

    pub fn partially_accepted(self) -> bool {
        self.accepted_count > 0 && self.violation_count > 0
    }

    pub fn valid_flag_matches_violations(self) -> bool {
        self.valid == (self.violation_count == 0)
    }

    pub fn accepted_signal_component_count(self) -> usize {
        usize::from(self.accepted_any())
    }

    pub fn partial_acceptance_signal_component_count(self) -> usize {
        usize::from(self.partially_accepted())
    }

    pub fn rejected_all_signal_component_count(self) -> usize {
        usize::from(self.rejected_all())
    }

    pub fn validation_signal_component_count(self) -> usize {
        self.accepted_signal_component_count()
            .saturating_add(self.partial_acceptance_signal_component_count())
            .saturating_add(self.rejected_all_signal_component_count())
    }

    pub fn has_validation_signals(self) -> bool {
        self.validation_signal_component_count() > 0
    }

    pub fn violation_problem_component_count(self) -> usize {
        usize::from(self.has_violations())
    }

    pub fn valid_flag_drift_component_count(self) -> usize {
        usize::from(!self.valid_flag_matches_violations())
    }

    pub fn validation_problem_component_count(self) -> usize {
        self.violation_problem_component_count()
            .saturating_add(self.valid_flag_drift_component_count())
    }

    pub fn has_validation_problem_components(self) -> bool {
        self.validation_problem_component_count() > 0
    }

    pub fn validation_accounting_is_consistent(self) -> bool {
        let expected_signal_count = usize::from(self.accepted_any())
            .saturating_add(usize::from(self.partially_accepted()))
            .saturating_add(usize::from(self.rejected_all()));
        let expected_problem_count = usize::from(self.violation_count > 0)
            .saturating_add(usize::from(!self.valid_flag_matches_violations()));

        self.validation_signal_component_count() == expected_signal_count
            && self.has_validation_signals() == (expected_signal_count > 0)
            && self.validation_problem_component_count() == expected_problem_count
            && self.has_validation_problem_components() == (expected_problem_count > 0)
    }

    pub fn runtime_kv_validation_commit_signal_component_count(self) -> usize {
        self.validation_signal_component_count()
    }

    pub fn has_runtime_kv_validation_commit_signals(self) -> bool {
        self.runtime_kv_validation_commit_signal_component_count() > 0
    }

    pub fn runtime_kv_validation_commit_blocker_component_count(self) -> usize {
        self.validation_problem_component_count()
    }

    pub fn has_runtime_kv_validation_commit_blockers(self) -> bool {
        self.runtime_kv_validation_commit_blocker_component_count() > 0
    }

    pub fn runtime_kv_validation_commit_accounting_is_consistent(self) -> bool {
        self.validation_accounting_is_consistent()
            && self.runtime_kv_validation_commit_signal_component_count()
                == self.validation_signal_component_count()
            && self.has_runtime_kv_validation_commit_signals()
                == (self.runtime_kv_validation_commit_signal_component_count() > 0)
            && self.runtime_kv_validation_commit_blocker_component_count()
                == self.validation_problem_component_count()
            && self.has_runtime_kv_validation_commit_blockers()
                == (self.runtime_kv_validation_commit_blocker_component_count() > 0)
    }

    pub fn runtime_kv_validation_commit_is_clean(self) -> bool {
        !self.has_runtime_kv_validation_commit_blockers()
            && self.runtime_kv_validation_commit_accounting_is_consistent()
    }

    pub fn validation_commit_is_clean(self) -> bool {
        self.runtime_kv_validation_commit_is_clean()
    }

    pub fn validation_shape_is_clean(self) -> bool {
        self.validation_commit_is_clean()
    }

    pub fn can_commit_runtime_kv_validation(self) -> bool {
        self.validation_commit_is_clean()
    }
}

impl RuntimeKvValidationReport {
    pub fn is_valid(&self) -> bool {
        self.violations.is_empty()
    }

    pub fn validation_summary(&self) -> RuntimeKvValidationSummary {
        RuntimeKvValidationSummary {
            accepted_count: self.accepted.len(),
            violation_count: self.violations.len(),
            valid: self.is_valid(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KvBlock {
    pub id: u64,
    pub namespace: KvNamespace,
    pub layer: usize,
    pub head: usize,
    pub token_start: usize,
    pub token_end: usize,
    pub key: Vec<f32>,
    pub value: Vec<f32>,
    pub score: f32,
    pub reinforcement: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KvBlockShapeSummary {
    pub namespace_label: String,
    pub is_runtime_namespace: bool,
    pub layer: usize,
    pub head: usize,
    pub token_start: usize,
    pub token_end: usize,
    pub token_len: usize,
    pub key_len: usize,
    pub value_len: usize,
    pub key_value_len_match: bool,
    pub has_empty_vectors: bool,
    pub has_non_finite_values: bool,
}

impl KvBlockShapeSummary {
    pub fn has_runtime_exchange_shape(&self) -> bool {
        self.is_runtime_namespace
            && self.token_len > 0
            && self.key_value_len_match
            && !self.has_empty_vectors
            && !self.has_non_finite_values
    }

    pub fn token_range_is_empty(&self) -> bool {
        self.token_len == 0
    }

    pub fn vector_len_delta(&self) -> usize {
        self.key_len.abs_diff(self.value_len)
    }

    pub fn vectors_are_paired_and_finite(&self) -> bool {
        self.key_value_len_match && !self.has_empty_vectors && !self.has_non_finite_values
    }

    pub fn runtime_namespace_signal_component_count(&self) -> usize {
        usize::from(self.is_runtime_namespace)
    }

    pub fn token_span_signal_component_count(&self) -> usize {
        usize::from(self.token_len > 0)
    }

    pub fn vector_payload_signal_component_count(&self) -> usize {
        usize::from(!self.has_empty_vectors)
    }

    pub fn block_shape_signal_component_count(&self) -> usize {
        self.runtime_namespace_signal_component_count()
            .saturating_add(self.token_span_signal_component_count())
            .saturating_add(self.vector_payload_signal_component_count())
    }

    pub fn has_block_shape_signals(&self) -> bool {
        self.block_shape_signal_component_count() > 0
    }

    pub fn runtime_namespace_drift_component_count(&self) -> usize {
        usize::from(!self.is_runtime_namespace)
    }

    pub fn token_range_problem_component_count(&self) -> usize {
        usize::from(self.token_range_is_empty())
    }

    pub fn vector_length_drift_component_count(&self) -> usize {
        usize::from(!self.key_value_len_match)
    }

    pub fn empty_vector_problem_component_count(&self) -> usize {
        usize::from(self.has_empty_vectors)
    }

    pub fn non_finite_value_problem_component_count(&self) -> usize {
        usize::from(self.has_non_finite_values)
    }

    pub fn runtime_exchange_shape_problem_component_count(&self) -> usize {
        self.runtime_namespace_drift_component_count()
            .saturating_add(self.token_range_problem_component_count())
            .saturating_add(self.vector_length_drift_component_count())
            .saturating_add(self.empty_vector_problem_component_count())
            .saturating_add(self.non_finite_value_problem_component_count())
    }

    pub fn has_runtime_exchange_shape_problem_components(&self) -> bool {
        self.runtime_exchange_shape_problem_component_count() > 0
    }

    pub fn runtime_exchange_shape_accounting_is_consistent(&self) -> bool {
        let expected_problem_count = usize::from(!self.is_runtime_namespace)
            .saturating_add(usize::from(self.token_range_is_empty()))
            .saturating_add(usize::from(!self.key_value_len_match))
            .saturating_add(usize::from(self.has_empty_vectors))
            .saturating_add(usize::from(self.has_non_finite_values));

        self.runtime_exchange_shape_problem_component_count() == expected_problem_count
            && self.has_runtime_exchange_shape_problem_components() == (expected_problem_count > 0)
            && self.has_runtime_exchange_shape() == (expected_problem_count == 0)
    }

    pub fn runtime_exchange_shape_is_clean(&self) -> bool {
        !self.has_runtime_exchange_shape_problem_components()
            && self.runtime_exchange_shape_accounting_is_consistent()
    }

    pub fn can_use_runtime_exchange_block(&self) -> bool {
        self.runtime_exchange_shape_is_clean()
    }
}

impl KvBlock {
    pub fn new(
        id: u64,
        namespace: KvNamespace,
        layer: usize,
        head: usize,
        token_range: Range<usize>,
        key: Vec<f32>,
        value: Vec<f32>,
    ) -> Self {
        Self {
            id,
            namespace,
            layer,
            head,
            token_start: token_range.start,
            token_end: token_range.end.max(token_range.start),
            key,
            value,
            score: 1.0,
            reinforcement: 0.0,
        }
    }

    pub fn with_score(mut self, score: f32) -> Self {
        self.score = score.clamp(0.0, 1.0);
        self
    }

    pub fn with_reinforcement(mut self, reinforcement: f32) -> Self {
        self.reinforcement = reinforcement.max(0.0);
        self
    }

    pub fn token_len(&self) -> usize {
        self.token_end.saturating_sub(self.token_start)
    }

    pub fn shape_summary(&self) -> KvBlockShapeSummary {
        KvBlockShapeSummary {
            namespace_label: self.namespace.label().to_owned(),
            is_runtime_namespace: self.namespace.is_runtime_exchange(),
            layer: self.layer,
            head: self.head,
            token_start: self.token_start,
            token_end: self.token_end,
            token_len: self.token_len(),
            key_len: self.key.len(),
            value_len: self.value.len(),
            key_value_len_match: self.key.len() == self.value.len(),
            has_empty_vectors: self.key.is_empty() || self.value.is_empty(),
            has_non_finite_values: !self.key.iter().all(|value| value.is_finite())
                || !self.value.iter().all(|value| value.is_finite()),
        }
    }

    pub fn same_slot(&self, other: &Self) -> bool {
        self.namespace == other.namespace
            && self.layer == other.layer
            && self.head == other.head
            && self.token_start == other.token_start
            && self.token_end == other.token_end
    }

    pub fn content_signature_eq(&self, other: &Self) -> bool {
        float_slices_equal(&self.key, &other.key) && float_slices_equal(&self.value, &other.value)
    }

    pub fn merge_weight(&self) -> f32 {
        (self.score + self.reinforcement).max(0.05)
    }
}

impl KvNamespaceCounts {
    pub fn from_blocks(blocks: &[KvBlock]) -> Self {
        let mut counts = Self::default();

        for block in blocks {
            match block.namespace {
                KvNamespace::Runtime => counts.runtime += 1,
                KvNamespace::Semantic => counts.semantic += 1,
                KvNamespace::Gist => counts.gist += 1,
                KvNamespace::Agent(_) => counts.agent += 1,
                KvNamespace::Custom(_) => counts.custom += 1,
            }
        }

        counts
    }
}

pub trait KvCachePort {
    fn store(&mut self, block: KvBlock) -> Option<KvBlock>;

    fn lookup(
        &self,
        namespace: &KvNamespace,
        layer: usize,
        head: usize,
        token_start: usize,
        token_end: usize,
    ) -> Option<&KvBlock>;

    fn list(&self, namespace: Option<&KvNamespace>) -> Vec<KvBlock>;

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn import_blocks(&mut self, blocks: &[KvBlock]) -> usize {
        let mut imported = 0;
        for block in blocks {
            self.store(block.clone());
            imported += 1;
        }
        imported
    }

    fn export_blocks(&self, namespace: Option<&KvNamespace>, limit: usize) -> Vec<KvBlock> {
        let mut blocks = self.list(namespace);
        blocks.sort_by(|left, right| {
            right
                .merge_weight()
                .total_cmp(&left.merge_weight())
                .then_with(|| left.id.cmp(&right.id))
        });
        blocks.truncate(limit);
        blocks
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct InMemoryKvCache {
    blocks: Vec<KvBlock>,
    max_blocks: usize,
}

impl InMemoryKvCache {
    pub fn new(max_blocks: usize) -> Self {
        Self {
            blocks: Vec::new(),
            max_blocks: max_blocks.max(1),
        }
    }

    pub fn blocks(&self) -> &[KvBlock] {
        &self.blocks
    }
}

impl Default for InMemoryKvCache {
    fn default() -> Self {
        Self::new(1024)
    }
}

impl KvCachePort for InMemoryKvCache {
    fn store(&mut self, block: KvBlock) -> Option<KvBlock> {
        if let Some(index) = self
            .blocks
            .iter()
            .position(|stored| stored.same_slot(&block))
        {
            return Some(std::mem::replace(&mut self.blocks[index], block));
        }

        let evicted = if self.blocks.len() >= self.max_blocks {
            Some(self.blocks.remove(0))
        } else {
            None
        };
        self.blocks.push(block);
        evicted
    }

    fn lookup(
        &self,
        namespace: &KvNamespace,
        layer: usize,
        head: usize,
        token_start: usize,
        token_end: usize,
    ) -> Option<&KvBlock> {
        self.blocks.iter().find(|block| {
            &block.namespace == namespace
                && block.layer == layer
                && block.head == head
                && block.token_start == token_start
                && block.token_end == token_end
        })
    }

    fn list(&self, namespace: Option<&KvNamespace>) -> Vec<KvBlock> {
        self.blocks
            .iter()
            .filter(|block| namespace.is_none_or(|namespace| &block.namespace == namespace))
            .cloned()
            .collect()
    }

    fn len(&self) -> usize {
        self.blocks.len()
    }
}

fn float_slices_equal(left: &[f32], right: &[f32]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| left.to_bits() == right.to_bits())
}

fn namespace_segment(value: &str) -> String {
    value
        .split(':')
        .next()
        .filter(|segment| !segment.is_empty())
        .unwrap_or("default")
        .to_owned()
}

fn fit_runtime_vector(vector: &[f32], dimensions: Option<usize>) -> Vec<f32> {
    let sanitize = |value: f32| if value.is_finite() { value } else { 0.0 };
    let Some(dimensions) = dimensions else {
        return vector.iter().copied().map(sanitize).collect();
    };

    let mut out = vector
        .iter()
        .copied()
        .map(sanitize)
        .take(dimensions)
        .collect::<Vec<_>>();
    out.resize(dimensions, 0.0);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::RuntimeAdapter;
    use crate::engine::InferenceRequest;
    use crate::fht_dke::DeterministicFhtDkeBudgeter;
    use crate::planning::RuntimePlanningDigest;
    use crate::profile::{HierarchyWeights, TaskProfile};
    use crate::request::RuntimeRequestEnvelope;
    use crate::router::RouteBudget;
    use crate::transformer::{
        TransformerAttentionKind, TransformerLayerBudget, TransformerPlanDigest,
    };

    #[test]
    fn in_memory_cache_imports_exports_by_namespace() {
        let mut cache = InMemoryKvCache::new(8);
        let runtime = KvBlock::new(1, KvNamespace::Runtime, 0, 0, 0..4, vec![1.0], vec![0.5])
            .with_reinforcement(0.3);
        let semantic = KvBlock::new(2, KvNamespace::Semantic, 0, 0, 0..4, vec![0.1], vec![0.2]);

        assert_eq!(cache.import_blocks(&[runtime.clone(), semantic]), 2);

        let exported = cache.export_blocks(Some(&KvNamespace::Runtime), 4);
        assert_eq!(exported, vec![runtime]);
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn store_replaces_same_slot() {
        let mut cache = InMemoryKvCache::new(8);
        let first = KvBlock::new(1, KvNamespace::Runtime, 0, 0, 0..4, vec![1.0], vec![1.0]);
        let second = KvBlock::new(2, KvNamespace::Runtime, 0, 0, 0..4, vec![2.0], vec![2.0]);

        assert!(cache.store(first).is_none());
        let replaced = cache.store(second.clone()).expect("same slot replaced");

        assert_eq!(replaced.id, 1);
        assert_eq!(cache.len(), 1);
        assert_eq!(
            cache.lookup(&KvNamespace::Runtime, 0, 0, 0, 4),
            Some(&second)
        );
    }

    #[test]
    fn namespace_parser_separates_adapter_sources() {
        assert_eq!(
            KvNamespace::from_key("runtime_kv:layer0"),
            KvNamespace::Runtime
        );
        assert_eq!(KvNamespace::from_key("gist:summary"), KvNamespace::Gist);
        assert_eq!(
            KvNamespace::from_key("agent:planner:layer0"),
            KvNamespace::Agent("planner".to_owned())
        );
        assert_eq!(
            KvNamespace::from_key("custom:adapter-a:block"),
            KvNamespace::Custom("adapter-a".to_owned())
        );
        assert_eq!(KvNamespace::from_key("memory:item"), KvNamespace::Semantic);
        assert!(KvNamespace::from_key("runtime_kv:layer0").is_runtime_exchange());
    }

    #[test]
    fn cache_keeps_custom_namespaces_isolated() {
        let mut cache = InMemoryKvCache::new(8);
        let alpha = KvBlock::new(
            1,
            KvNamespace::Custom("alpha".to_owned()),
            0,
            0,
            0..4,
            vec![1.0],
            vec![1.0],
        );
        let beta = KvBlock::new(
            2,
            KvNamespace::Custom("beta".to_owned()),
            0,
            0,
            0..4,
            vec![2.0],
            vec![2.0],
        );

        cache.import_blocks(&[alpha.clone(), beta.clone()]);

        assert_eq!(
            cache.export_blocks(Some(&KvNamespace::Custom("alpha".to_owned())), 8),
            vec![alpha]
        );
        assert_eq!(
            cache.export_blocks(Some(&KvNamespace::Custom("beta".to_owned())), 8),
            vec![beta]
        );
    }

    #[test]
    fn namespace_counts_group_adapter_boundaries() {
        let blocks = [
            KvBlock::new(1, KvNamespace::Runtime, 0, 0, 0..1, vec![1.0], vec![1.0]),
            KvBlock::new(2, KvNamespace::Semantic, 0, 0, 0..1, vec![1.0], vec![1.0]),
            KvBlock::new(3, KvNamespace::Gist, 0, 0, 0..1, vec![1.0], vec![1.0]),
            KvBlock::new(
                4,
                KvNamespace::Agent("planner".to_owned()),
                0,
                0,
                0..1,
                vec![1.0],
                vec![1.0],
            ),
            KvBlock::new(
                5,
                KvNamespace::Agent("critic".to_owned()),
                0,
                0,
                0..1,
                vec![1.0],
                vec![1.0],
            ),
            KvBlock::new(
                6,
                KvNamespace::Custom("adapter-a".to_owned()),
                0,
                0,
                0..1,
                vec![1.0],
                vec![1.0],
            ),
        ];

        let counts = KvNamespaceCounts::from_blocks(&blocks);

        assert_eq!(counts.runtime, 1);
        assert_eq!(counts.semantic, 1);
        assert_eq!(counts.gist, 1);
        assert_eq!(counts.agent, 2);
        assert_eq!(counts.custom, 1);
        assert_eq!(counts.total(), 6);
        assert_eq!(counts.non_runtime_total(), 5);
        assert_eq!(counts.active_namespace_count(), 5);
        assert!(counts.has_runtime_exchange());
        assert!(counts.has_namespace_mix());
        assert!(counts.has_runtime_and_non_runtime_blocks());
        assert!(!counts.only_runtime_exchange());
        assert!(!counts.only_non_runtime_blocks());
        assert!((counts.runtime_fraction() - (1.0 / 6.0)).abs() < 0.0001);
        assert_eq!(counts.runtime_exchange_signal_component_count(), 1);
        assert_eq!(counts.non_runtime_payload_signal_component_count(), 1);
        assert_eq!(counts.namespace_mix_signal_component_count(), 1);
        assert_eq!(counts.runtime_non_runtime_mix_signal_component_count(), 1);
        assert_eq!(counts.namespace_boundary_signal_component_count(), 4);
        assert!(counts.has_namespace_boundary_signals());

        let runtime_only = KvNamespaceCounts::from_blocks(&blocks[..1]);
        let non_runtime_only = KvNamespaceCounts::from_blocks(&blocks[1..]);

        assert!(runtime_only.only_runtime_exchange());
        assert!(!runtime_only.has_namespace_mix());
        assert_eq!(runtime_only.runtime_fraction(), 1.0);
        assert_eq!(runtime_only.namespace_boundary_signal_component_count(), 1);
        assert!(runtime_only.has_namespace_boundary_signals());
        assert!(non_runtime_only.only_non_runtime_blocks());
        assert!(!non_runtime_only.has_runtime_exchange());
        assert_eq!(non_runtime_only.runtime_fraction(), 0.0);
        assert_eq!(
            non_runtime_only.namespace_boundary_signal_component_count(),
            2
        );
        assert!(non_runtime_only.has_namespace_boundary_signals());

        let clean_drift = counts.drift_summary(counts);

        assert!(clean_drift.exact_match());
        assert_eq!(clean_drift.total_count_drift(), 0);
        assert_eq!(clean_drift.non_runtime_count_drift(), 0);
        assert_eq!(clean_drift.active_namespace_count_drift(), 0);
        assert_eq!(
            clean_drift.namespace_distribution_drift_component_count(),
            0
        );
        assert_eq!(clean_drift.namespace_shape_signal_component_count(), 0);
        assert!(!clean_drift.has_namespace_shape_signals());
        assert_eq!(clean_drift.namespace_boundary_signal_component_count(), 0);
        assert!(!clean_drift.has_namespace_boundary_signals());
        assert!(!clean_drift.has_namespace_distribution_drift_components());
        assert!(clean_drift.namespace_distribution_accounting_is_consistent());
        assert_eq!(clean_drift.namespace_boundary_problem_component_count(), 0);
        assert!(!clean_drift.has_namespace_boundary_problem_components());
        assert!(clean_drift.namespace_boundary_is_clean());
        assert!(clean_drift.namespace_distribution_shape_is_clean());
        assert!(clean_drift.can_use_namespace_distribution());
        assert_eq!(
            clean_drift.namespace_distribution_commit_signal_component_count(),
            0
        );
        assert!(!clean_drift.has_namespace_distribution_commit_signals());
        assert_eq!(
            clean_drift.namespace_distribution_commit_blocker_component_count(),
            0
        );
        assert!(!clean_drift.has_namespace_distribution_commit_blockers());
        assert!(clean_drift.namespace_distribution_commit_accounting_is_consistent());
        assert!(clean_drift.namespace_distribution_commit_shape_is_clean());
        assert!(clean_drift.can_commit_namespace_distribution());
        assert_eq!(clean_drift.component_accounting_drift_count(), 0);
        assert_eq!(
            clean_drift.namespace_distribution_commit_problem_component_count(),
            0
        );
        assert!(!clean_drift.has_namespace_distribution_commit_problem_components());
        assert_eq!(clean_drift.failure_report(), None);
        assert_eq!(clean_drift.failure_reports(), Vec::new());
        assert_eq!(clean_drift.failure_report_count(), 0);
        assert!(!clean_drift.has_failure_reports());
        assert_eq!(clean_drift.failure_batch_summary().total_count, 0);
        assert!(!clean_drift.can_format_runtime_failures());
        assert_eq!(clean_drift.primary_failure_report(), None);
        assert_eq!(clean_drift.primary_failure_summary(), None);
        let clean_commit = clean_drift.commit_summary();
        assert_eq!(
            clean_commit.action,
            KvNamespaceCountDriftCommitAction::CommitKvNamespaceDistribution
        );
        assert!(clean_commit.action_can_commit());
        assert!(!clean_commit.action_should_return_failure());
        assert!(clean_commit.can_commit_namespace_distribution());
        assert!(!clean_commit.should_return_runtime_failure());
        assert!(clean_commit.failure_reports.is_empty());
        assert_eq!(clean_commit.primary_failure_report, None);
        assert_eq!(clean_commit.primary_failure_summary, None);
        assert_eq!(clean_commit.failure_report_count, 0);
        assert!(!clean_commit.can_format_runtime_failures);
        assert_eq!(clean_commit.total_signal_component_count, 0);
        assert_eq!(clean_commit.total_blocker_component_count, 0);
        assert!(clean_commit.component_accounting_consistent);
        assert!(!clean_commit.has_primary_failure_summary());
        assert!(clean_commit.failure_batch_shape_is_clean());
        assert!(clean_commit.commit_decision_accounting_is_consistent());
        let clean_failure_return = clean_commit.failure_return_summary();
        assert_eq!(
            clean_failure_return.source,
            RuntimeKvPersistenceFailureReturnSource::NamespaceDistribution
        );
        assert_eq!(
            clean_failure_return.source.label(),
            "kv_namespace_distribution"
        );
        assert!(!clean_failure_return.has_failure_reports());
        assert!(!clean_failure_return.has_blocker_components());
        assert!(clean_failure_return.failure_return_accounting_is_consistent());
        assert!(!clean_failure_return.can_return_runtime_failure());
        assert_eq!(clean_commit.runtime_failure_return_report(), None);

        let actual = KvNamespaceCounts {
            runtime: 2,
            semantic: 0,
            gist: 1,
            agent: 1,
            custom: 0,
        };
        let drift = counts.drift_summary(actual);

        assert!(!drift.exact_match());
        assert_eq!(drift.runtime_count_drift(), 1);
        assert_eq!(drift.semantic_count_drift(), 1);
        assert_eq!(drift.gist_count_drift(), 0);
        assert_eq!(drift.agent_count_drift(), 1);
        assert_eq!(drift.custom_count_drift(), 1);
        assert_eq!(drift.total_count_drift(), 2);
        assert_eq!(drift.non_runtime_count_drift(), 3);
        assert_eq!(drift.active_namespace_count_drift(), 2);
        assert_eq!(drift.runtime_count_drift_component_count(), 1);
        assert_eq!(drift.semantic_count_drift_component_count(), 1);
        assert_eq!(drift.gist_count_drift_component_count(), 0);
        assert_eq!(drift.agent_count_drift_component_count(), 1);
        assert_eq!(drift.custom_count_drift_component_count(), 1);
        assert_eq!(drift.namespace_distribution_drift_component_count(), 4);
        assert_eq!(drift.total_count_drift_signal_component_count(), 1);
        assert_eq!(drift.non_runtime_count_drift_signal_component_count(), 1);
        assert_eq!(
            drift.active_namespace_count_drift_signal_component_count(),
            1
        );
        assert_eq!(drift.namespace_shape_signal_component_count(), 3);
        assert!(drift.has_namespace_shape_signals());
        assert_eq!(drift.namespace_boundary_signal_component_count(), 3);
        assert!(drift.has_namespace_boundary_signals());
        assert!(drift.has_namespace_distribution_drift_components());
        assert!(drift.namespace_distribution_accounting_is_consistent());
        assert_eq!(drift.namespace_boundary_problem_component_count(), 4);
        assert!(drift.has_namespace_boundary_problem_components());
        assert!(!drift.namespace_boundary_is_clean());
        assert!(!drift.namespace_distribution_shape_is_clean());
        assert!(!drift.can_use_namespace_distribution());
        assert_eq!(
            drift.namespace_distribution_commit_signal_component_count(),
            3
        );
        assert!(drift.has_namespace_distribution_commit_signals());
        assert_eq!(
            drift.namespace_distribution_commit_blocker_component_count(),
            4
        );
        assert!(drift.has_namespace_distribution_commit_blockers());
        assert!(drift.namespace_distribution_commit_accounting_is_consistent());
        assert!(!drift.namespace_distribution_commit_shape_is_clean());
        assert!(!drift.can_commit_namespace_distribution());
        assert_eq!(drift.component_accounting_drift_count(), 0);
        assert_eq!(
            drift.namespace_distribution_commit_problem_component_count(),
            4
        );
        assert!(drift.has_namespace_distribution_commit_problem_components());
        assert_eq!(drift.failure_report_count(), 1);
        assert!(drift.has_failure_reports());
        assert_eq!(drift.failure_reports().len(), 1);
        let failure = drift.failure_report().expect("drift failure report");
        let failure_summary = failure.failure_summary();
        assert_eq!(failure_summary.trace_label, "runtime_contract_violation");
        assert!(failure_summary.message_len > 0);
        assert!(failure_summary.trace_label_matches_kind());
        assert!(failure_summary.can_use_runtime_failure_report());
        let failure_batch = drift.failure_batch_summary();
        assert_eq!(failure_batch.total_count, 1);
        assert_eq!(failure_batch.contract_violation_count, 1);
        assert!(failure_batch.can_format_runtime_failures());
        assert!(drift.can_format_runtime_failures());
        assert_eq!(drift.primary_failure_report(), Some(failure.clone()));
        assert_eq!(drift.primary_failure_summary(), Some(failure_summary));
        let drift_commit = drift.commit_summary();
        assert_eq!(
            drift_commit.action,
            KvNamespaceCountDriftCommitAction::ReturnRuntimeFailure
        );
        assert!(!drift_commit.action_can_commit());
        assert!(drift_commit.action_should_return_failure());
        assert!(!drift_commit.can_commit_namespace_distribution());
        assert!(drift_commit.should_return_runtime_failure());
        assert_eq!(drift_commit.failure_report_count, 1);
        assert_eq!(drift_commit.failure_reports.len(), 1);
        assert_eq!(drift_commit.primary_failure_report, Some(failure));
        assert_eq!(drift_commit.primary_failure_summary, Some(failure_summary));
        assert!(drift_commit.can_format_runtime_failures);
        assert_eq!(drift_commit.total_signal_component_count, 3);
        assert_eq!(drift_commit.total_blocker_component_count, 4);
        assert!(drift_commit.component_accounting_consistent);
        assert!(drift_commit.has_primary_failure_summary());
        assert!(drift_commit.failure_batch_shape_is_clean());
        assert!(drift_commit.commit_decision_accounting_is_consistent());
        let drift_failure_return = drift_commit.failure_return_summary();
        assert_eq!(
            drift_failure_return.source,
            RuntimeKvPersistenceFailureReturnSource::NamespaceDistribution
        );
        assert!(drift_failure_return.has_failure_reports());
        assert!(drift_failure_return.has_blocker_components());
        assert!(drift_failure_return.failure_return_accounting_is_consistent());
        assert!(drift_failure_return.can_return_runtime_failure());
        let drift_return_report = drift_commit
            .runtime_failure_return_report()
            .expect("namespace drift return report");
        assert_eq!(
            drift_return_report.source,
            RuntimeKvPersistenceFailureReturnSource::NamespaceDistribution
        );
        assert_eq!(drift_return_report.primary_failure_summary, failure_summary);
        assert_eq!(
            drift_return_report.failure_batch.contract_violation_count,
            1
        );
        assert!(drift_return_report.failure_return_report_shape_is_clean());
        assert!(drift_return_report.can_use_runtime_kv_persistence_failure_return_report());
        assert!(
            drift_return_report
                .backend_message()
                .contains("kv namespace distribution failed")
        );
        assert!(
            drift_return_report
                .diagnostics_note()
                .starts_with("runtime_contract_violation")
        );
        assert_eq!(
            drift_return_report.inference_error().message,
            drift_return_report.backend_message()
        );
    }

    #[test]
    fn runtime_kv_import_plan_builds_runtime_namespace_blocks() {
        let runtime = RuntimeMetadata::new("model", "tok", 4096, 3).with_kv_exchange(true, false);
        let architecture = TransformerRuntimeArchitecture::new(2, 6, 2, 2, 128);
        let plan = RuntimeKvImportPlan::new(&runtime, architecture, 3);
        let candidates = [
            RuntimeKvCandidate::new(10, vec![1.0, 2.0], 0.50),
            RuntimeKvCandidate::new(11, vec![3.0, f32::NAN, 5.0, 7.0], 1.25),
            RuntimeKvCandidate::new(12, Vec::new(), 0.90),
            RuntimeKvCandidate::new(13, vec![9.0], 0.20),
        ];

        let blocks = plan.build_blocks(&candidates);
        let summary = plan.import_summary(&candidates);

        assert_eq!(plan.planned_block_count(&candidates), 3);
        assert_eq!(
            summary,
            RuntimeKvImportSummary {
                enabled: true,
                max_blocks: 3,
                candidate_count: 4,
                non_empty_candidate_count: 3,
                planned_blocks: 3,
                hit_import_limit: true,
                embedding_dimensions: Some(3),
            }
        );
        assert!(summary.will_import());
        assert!(!summary.skipped_due_to_empty_candidates());
        assert!(summary.has_candidates());
        assert!(summary.has_non_empty_candidates());
        assert_eq!(summary.empty_candidate_count(), 1);
        assert!(summary.has_embedding_dimensions());
        assert!(summary.enabled_matches_capacity());
        assert!(summary.candidate_counts_are_valid());
        assert!(summary.planned_blocks_within_limit());
        assert!(summary.planned_blocks_within_candidates());
        assert!(summary.import_limit_flag_matches_shape());
        assert!(summary.disabled_import_is_empty());
        assert!(summary.embedding_dimensions_shape_is_valid());
        assert_eq!(summary.import_signal_component_count(), 6);
        assert!(summary.has_import_signals());
        assert_eq!(summary.import_shape_problem_component_count(), 0);
        assert!(!summary.has_import_shape_problem_components());
        assert!(summary.import_accounting_is_consistent());
        assert_eq!(summary.runtime_kv_import_commit_signal_component_count(), 6);
        assert!(summary.has_runtime_kv_import_commit_signals());
        assert_eq!(
            summary.runtime_kv_import_commit_blocker_component_count(),
            0
        );
        assert!(!summary.has_runtime_kv_import_commit_blockers());
        assert!(summary.runtime_kv_import_commit_accounting_is_consistent());
        assert!(summary.runtime_kv_import_commit_is_clean());
        assert!(summary.import_commit_is_clean());
        assert!(summary.import_shape_is_clean());
        assert!(summary.can_commit_runtime_kv_import());
        assert_eq!(blocks.len(), 3);
        assert!(
            blocks
                .iter()
                .all(|block| block.namespace == KvNamespace::Runtime)
        );
        assert_eq!(blocks[0].id, 10);
        assert_eq!(blocks[0].layer, 0);
        assert_eq!(blocks[0].head, 0);
        assert_eq!(blocks[0].key, vec![1.0, 2.0, 0.0]);
        assert_eq!(blocks[0].value, vec![0.5, 1.0, 0.0]);
        assert_eq!(blocks[1].layer, 0);
        assert_eq!(blocks[1].head, 1);
        assert_eq!(blocks[1].key, vec![3.0, 0.0, 5.0]);
        assert_eq!(blocks[1].score, 1.0);
        assert_eq!(blocks[1].reinforcement, 0.25);
        assert_eq!(blocks[2].id, 13);
        assert_eq!(blocks[2].layer, 1);
        assert_eq!(blocks[2].head, 0);

        let block_summary =
            RuntimeKvImportBlockSummary::from_blocks(summary.planned_blocks, &blocks);
        let block_shape_summaries = blocks
            .iter()
            .map(KvBlock::shape_summary)
            .collect::<Vec<_>>();

        assert_eq!(
            block_summary,
            RuntimeKvImportBlockSummary::from_block_summaries(
                summary.planned_blocks,
                &block_shape_summaries
            )
        );
        assert_eq!(block_summary.planned_blocks, 3);
        assert_eq!(block_summary.materialized_blocks, 3);
        assert_eq!(block_summary.runtime_namespace_blocks, 3);
        assert_eq!(block_summary.block_shape_signal_component_count, 9);
        assert_eq!(block_summary.block_shape_problem_component_count, 0);
        assert!(!block_summary.is_empty());
        assert!(block_summary.block_count_matches_plan());
        assert_eq!(block_summary.block_count_drift(), 0);
        assert!(block_summary.all_blocks_are_runtime_namespace());
        assert_eq!(block_summary.runtime_namespace_drift_component_count(), 0);
        assert_eq!(block_summary.block_count_drift_component_count(), 0);
        assert_eq!(block_summary.import_block_problem_component_count(), 0);
        assert!(!block_summary.has_import_block_problem_components());
        assert!(block_summary.has_import_block_signals());
        assert!(block_summary.import_block_accounting_is_consistent());
        assert_eq!(
            block_summary.runtime_kv_import_block_commit_signal_component_count(),
            9
        );
        assert!(block_summary.has_runtime_kv_import_block_commit_signals());
        assert_eq!(
            block_summary.runtime_kv_import_block_commit_blocker_component_count(),
            0
        );
        assert!(!block_summary.has_runtime_kv_import_block_commit_blockers());
        assert!(block_summary.runtime_kv_import_block_commit_accounting_is_consistent());
        assert!(block_summary.runtime_kv_import_block_commit_is_clean());
        assert!(block_summary.import_block_shape_is_clean());
        assert!(block_summary.can_commit_runtime_kv_import_blocks());

        let readiness = RuntimeKvImportReadinessSummary::new(summary, block_summary);

        assert_eq!(
            RuntimeKvImportReadinessSummary::stage_order(),
            [
                RuntimeKvImportReadinessStage::ImportPlan,
                RuntimeKvImportReadinessStage::ImportBlocks,
            ]
        );
        assert!(readiness.import_ready());
        assert!(readiness.blocks_ready());
        assert!(readiness.import_block_plan_matches());
        assert_eq!(readiness.import_block_plan_drift_component_count(), 0);
        assert!(readiness.stage_ready(RuntimeKvImportReadinessStage::ImportPlan));
        assert!(readiness.stage_ready(RuntimeKvImportReadinessStage::ImportBlocks));
        assert_eq!(
            readiness.stage_signal_component_count(RuntimeKvImportReadinessStage::ImportPlan),
            readiness.import_signal_component_count
        );
        assert_eq!(
            readiness.stage_blocker_component_count(RuntimeKvImportReadinessStage::ImportBlocks),
            readiness.block_blocker_component_count
        );
        assert_eq!(readiness.first_unready_stage(), None);
        assert_eq!(readiness.first_blocking_stage(), None);
        assert_eq!(readiness.import_signal_component_count, 6);
        assert_eq!(readiness.block_signal_component_count, 9);
        assert_eq!(readiness.import_blocker_component_count, 0);
        assert_eq!(readiness.block_blocker_component_count, 0);
        assert_eq!(
            readiness.runtime_kv_import_readiness_signal_component_count(),
            15
        );
        assert!(readiness.has_runtime_kv_import_readiness_signals());
        assert_eq!(
            readiness.runtime_kv_import_readiness_blocker_component_count(),
            0
        );
        assert!(!readiness.has_runtime_kv_import_readiness_blockers());
        assert!(readiness.runtime_kv_import_readiness_accounting_is_consistent());
        assert!(readiness.runtime_kv_import_readiness_commit_is_clean());
        assert!(readiness.can_commit_runtime_kv_import_readiness());
        assert_eq!(readiness.component_accounting_drift_count(), 0);
        assert_eq!(
            readiness.runtime_kv_import_readiness_commit_problem_component_count(),
            0
        );
        assert!(!readiness.has_runtime_kv_import_readiness_commit_problem_components());
        assert_eq!(readiness.failure_report(), None);
        assert_eq!(readiness.failure_reports(), Vec::new());
        assert_eq!(readiness.failure_report_count(), 0);
        assert!(!readiness.has_failure_reports());
        assert_eq!(readiness.failure_batch_summary().total_count, 0);
        assert!(!readiness.can_format_runtime_failures());
        assert_eq!(readiness.primary_failure_report(), None);
        assert_eq!(readiness.primary_failure_summary(), None);
        let readiness_commit = readiness.commit_summary();
        assert_eq!(
            readiness_commit.action,
            RuntimeKvImportReadinessCommitAction::CommitRuntimeKvImport
        );
        assert!(readiness_commit.action_can_commit());
        assert!(!readiness_commit.action_should_return_failure());
        assert!(readiness_commit.can_commit_runtime_kv_import());
        assert!(!readiness_commit.should_return_runtime_failure());
        assert!(readiness_commit.failure_reports.is_empty());
        assert_eq!(readiness_commit.primary_failure_report, None);
        assert_eq!(readiness_commit.primary_failure_summary, None);
        assert_eq!(readiness_commit.failure_report_count, 0);
        assert!(!readiness_commit.can_format_runtime_failures);
        assert_eq!(readiness_commit.total_signal_component_count, 15);
        assert_eq!(readiness_commit.total_blocker_component_count, 0);
        assert!(readiness_commit.component_accounting_consistent);
        assert!(!readiness_commit.has_primary_failure_summary());
        assert!(readiness_commit.failure_batch_shape_is_clean());
        assert!(readiness_commit.commit_decision_accounting_is_consistent());
        let failure_return = readiness_commit.failure_return_summary();
        assert_eq!(
            failure_return.source,
            RuntimeKvExchangeFailureReturnSource::RuntimeKvImportReadiness
        );
        assert_eq!(failure_return.source.label(), "runtime_kv_import_readiness");
        assert!(!failure_return.has_failure_reports());
        assert!(!failure_return.has_blocker_components());
        assert!(failure_return.failure_return_accounting_is_consistent());
        assert!(!failure_return.can_return_runtime_failure());
        assert_eq!(readiness_commit.runtime_failure_return_report(), None);
    }

    #[test]
    fn runtime_kv_import_plan_from_manifest_uses_manifest_policy_limit() {
        let metadata = RuntimeMetadata::new("manifest", "tok", 4096, 2048)
            .with_kv_exchange(true, false)
            .with_kv_limits(8, 0);
        let manifest = crate::manifest::RuntimeManifestDigest::from_metadata(metadata)
            .with_architecture(TransformerRuntimeArchitecture::new(6, 2048, 8, 2, 256))
            .with_kv_policy(
                crate::manifest::RuntimeKvPolicy::from_capabilities(true, false).with_limits(2, 0),
            );

        let plan = RuntimeKvImportPlan::from_manifest(&manifest, 5);
        let summary = RuntimeKvImportPlan::manifest_plan_summary(&manifest, 5);

        assert_eq!(plan.max_blocks, 2);
        assert_eq!(plan.embedding_dimensions, Some(2048));
        assert_eq!(plan.layer_count, 6);
        assert_eq!(plan.kv_heads, 2);
        assert!(summary.manifest_allows_import());
        assert!(summary.runtime_allows_import());
        assert!(summary.requested_prefetch());
        assert!(summary.plan_will_import());
        assert!(summary.has_embedding_dimensions());
        assert!(summary.architecture_has_import_shape());
        assert!(summary.manifest_import_capability_is_consistent());
        assert!(summary.runtime_import_capability_is_consistent());
        assert!(summary.import_plan_within_manifest_limit());
        assert!(summary.import_plan_within_runtime_limit());
        assert!(summary.import_plan_within_requested_limit());
        assert!(!summary.requested_prefetch_without_manifest_capacity());
        assert!(!summary.requested_prefetch_without_runtime_capacity());
        assert_eq!(summary.manifest_bridge_signal_component_count(), 9);
        assert!(summary.has_manifest_bridge_signals());
        assert_eq!(summary.manifest_bridge_problem_component_count(), 0);
        assert!(!summary.has_manifest_bridge_problem_components());
        assert!(summary.manifest_bridge_accounting_is_consistent());
        assert!(summary.manifest_bridge_shape_is_clean());
        assert!(summary.can_use_manifest_runtime_kv_import_plan());
    }

    #[test]
    fn runtime_kv_import_manifest_plan_summary_reports_import_capacity_drift() {
        let metadata = RuntimeMetadata::new("manifest", "tok", 4096, 2048)
            .with_kv_exchange(true, false)
            .with_kv_limits(8, 0);
        let manifest = crate::manifest::RuntimeManifestDigest::from_metadata(metadata)
            .with_architecture(TransformerRuntimeArchitecture::new(6, 2048, 8, 2, 256))
            .with_kv_policy(crate::manifest::RuntimeKvPolicy {
                import_enabled: true,
                export_enabled: false,
                max_import_blocks: 0,
                max_export_blocks: 0,
            });

        let plan = RuntimeKvImportPlan::from_manifest(&manifest, 2);
        let summary = RuntimeKvImportPlan::manifest_plan_summary(&manifest, 2);

        assert_eq!(plan.max_blocks, 0);
        assert!(!summary.manifest_allows_import());
        assert!(summary.runtime_allows_import());
        assert!(summary.requested_prefetch());
        assert!(!summary.plan_will_import());
        assert!(!summary.manifest_import_capability_is_consistent());
        assert!(summary.runtime_import_capability_is_consistent());
        assert!(summary.requested_prefetch_without_manifest_capacity());
        assert!(!summary.requested_prefetch_without_runtime_capacity());
        assert_eq!(
            summary.manifest_import_capability_problem_component_count(),
            1
        );
        assert_eq!(
            summary.runtime_import_capability_problem_component_count(),
            0
        );
        assert_eq!(
            summary.requested_prefetch_capacity_problem_component_count(),
            1
        );
        assert_eq!(summary.import_plan_limit_problem_component_count(), 0);
        assert_eq!(summary.embedding_shape_problem_component_count(), 0);
        assert_eq!(
            summary.architecture_import_shape_problem_component_count(),
            0
        );
        assert_eq!(summary.manifest_bridge_problem_component_count(), 2);
        assert!(summary.has_manifest_bridge_problem_components());
        assert!(summary.manifest_bridge_accounting_is_consistent());
        assert!(!summary.manifest_bridge_shape_is_clean());
        assert!(!summary.can_use_manifest_runtime_kv_import_plan());
    }

    #[test]
    fn runtime_kv_import_plan_from_manifest_feeds_readiness_gate() {
        let manifest =
            crate::manifest::RuntimeManifestDigest::self_developed("manifest", "tok", 4096, 3)
                .with_architecture(TransformerRuntimeArchitecture::new(2, 6, 2, 2, 128))
                .with_kv_policy(
                    crate::manifest::RuntimeKvPolicy::from_capabilities(true, false)
                        .with_limits(2, 0),
                );
        let candidates = [
            RuntimeKvCandidate::new(10, vec![1.0, 2.0], 0.50),
            RuntimeKvCandidate::new(11, vec![3.0, 4.0, 5.0], 1.25),
            RuntimeKvCandidate::new(12, Vec::new(), 0.90),
        ];

        let bridge = RuntimeKvImportPlan::manifest_plan_summary(&manifest, 2);
        let plan = RuntimeKvImportPlan::from_manifest(&manifest, 2);
        let summary = plan.import_summary(&candidates);
        let blocks = plan.build_blocks(&candidates);
        let block_summary =
            RuntimeKvImportBlockSummary::from_blocks(summary.planned_blocks, &blocks);
        let readiness = RuntimeKvImportReadinessSummary::new(summary, block_summary);

        assert!(bridge.can_use_manifest_runtime_kv_import_plan());
        assert_eq!(plan.max_blocks, 2);
        assert_eq!(summary.planned_blocks, 2);
        assert_eq!(blocks.len(), 2);
        assert!(
            blocks
                .iter()
                .all(|block| block.namespace == KvNamespace::Runtime)
        );
        assert!(readiness.import_ready());
        assert!(readiness.blocks_ready());
        assert!(readiness.runtime_kv_import_readiness_commit_is_clean());
        assert!(readiness.can_commit_runtime_kv_import_readiness());
    }

    #[test]
    fn runtime_kv_import_block_summary_counts_materialized_shape_drift() {
        let malformed_block =
            KvBlock::new(7, KvNamespace::Semantic, 0, 0, 0..0, Vec::new(), Vec::new());
        let malformed = RuntimeKvImportBlockSummary::from_blocks(1, &[malformed_block]);

        assert_eq!(malformed.planned_blocks, 1);
        assert_eq!(malformed.materialized_blocks, 1);
        assert_eq!(malformed.runtime_namespace_blocks, 0);
        assert_eq!(malformed.block_shape_signal_component_count, 0);
        assert_eq!(malformed.block_shape_problem_component_count, 3);
        assert!(!malformed.is_empty());
        assert!(malformed.block_count_matches_plan());
        assert_eq!(malformed.block_count_drift(), 0);
        assert!(!malformed.all_blocks_are_runtime_namespace());
        assert_eq!(malformed.runtime_namespace_drift_component_count(), 1);
        assert_eq!(malformed.block_count_drift_component_count(), 0);
        assert_eq!(malformed.import_block_problem_component_count(), 3);
        assert!(malformed.has_import_block_problem_components());
        assert!(!malformed.has_import_block_signals());
        assert!(malformed.import_block_accounting_is_consistent());
        assert_eq!(
            malformed.runtime_kv_import_block_commit_signal_component_count(),
            0
        );
        assert!(!malformed.has_runtime_kv_import_block_commit_signals());
        assert_eq!(
            malformed.runtime_kv_import_block_commit_blocker_component_count(),
            3
        );
        assert!(malformed.has_runtime_kv_import_block_commit_blockers());
        assert!(malformed.runtime_kv_import_block_commit_accounting_is_consistent());
        assert!(!malformed.runtime_kv_import_block_commit_is_clean());
        assert!(!malformed.import_block_shape_is_clean());
        assert!(!malformed.can_commit_runtime_kv_import_blocks());

        let import = RuntimeKvImportSummary {
            enabled: true,
            max_blocks: 1,
            candidate_count: 1,
            non_empty_candidate_count: 1,
            planned_blocks: 1,
            hit_import_limit: true,
            embedding_dimensions: Some(1),
        };
        let malformed_readiness = RuntimeKvImportReadinessSummary::new(import, malformed);

        assert!(malformed_readiness.import_ready());
        assert!(!malformed_readiness.blocks_ready());
        assert_eq!(
            malformed_readiness.first_unready_stage(),
            Some(RuntimeKvImportReadinessStage::ImportBlocks)
        );
        assert_eq!(
            malformed_readiness.first_blocking_stage(),
            Some(RuntimeKvImportReadinessStage::ImportBlocks)
        );
        assert_eq!(malformed_readiness.block_blocker_component_count, 3);
        assert_eq!(
            malformed_readiness.runtime_kv_import_readiness_blocker_component_count(),
            3
        );
        assert!(malformed_readiness.runtime_kv_import_readiness_accounting_is_consistent());
        assert!(!malformed_readiness.can_commit_runtime_kv_import_readiness());
        assert_eq!(malformed_readiness.component_accounting_drift_count(), 0);
        assert_eq!(
            malformed_readiness.runtime_kv_import_readiness_commit_problem_component_count(),
            3
        );
        assert!(malformed_readiness.has_runtime_kv_import_readiness_commit_problem_components());
        assert_eq!(malformed_readiness.failure_report_count(), 1);
        assert!(malformed_readiness.has_failure_reports());
        let malformed_failure = malformed_readiness
            .failure_report()
            .expect("malformed import readiness failure");
        let malformed_failure_summary = malformed_failure.failure_summary();
        assert_eq!(
            malformed_failure_summary.trace_label,
            "runtime_kv_import_error"
        );
        assert!(malformed_failure_summary.message_len > 0);
        assert!(malformed_failure_summary.trace_label_matches_kind());
        assert!(malformed_failure_summary.can_use_runtime_failure_report());
        let malformed_failure_batch = malformed_readiness.failure_batch_summary();
        assert_eq!(malformed_failure_batch.total_count, 1);
        assert_eq!(malformed_failure_batch.kv_import_count, 1);
        assert!(malformed_failure_batch.can_format_runtime_failures());
        assert!(malformed_readiness.can_format_runtime_failures());
        assert_eq!(
            malformed_readiness.primary_failure_report(),
            Some(malformed_failure.clone())
        );
        assert_eq!(
            malformed_readiness.primary_failure_summary(),
            Some(malformed_failure_summary)
        );
        let malformed_commit = malformed_readiness.commit_summary();
        assert_eq!(
            malformed_commit.action,
            RuntimeKvImportReadinessCommitAction::ReturnRuntimeFailure
        );
        assert!(!malformed_commit.action_can_commit());
        assert!(malformed_commit.action_should_return_failure());
        assert!(!malformed_commit.can_commit_runtime_kv_import());
        assert!(malformed_commit.should_return_runtime_failure());
        assert_eq!(malformed_commit.failure_report_count, 1);
        assert_eq!(malformed_commit.failure_reports.len(), 1);
        assert_eq!(
            malformed_commit.primary_failure_report,
            Some(malformed_failure)
        );
        assert_eq!(
            malformed_commit.primary_failure_summary,
            Some(malformed_failure_summary)
        );
        assert!(malformed_commit.can_format_runtime_failures);
        assert_eq!(malformed_commit.total_signal_component_count, 6);
        assert_eq!(malformed_commit.total_blocker_component_count, 3);
        assert!(malformed_commit.component_accounting_consistent);
        assert!(malformed_commit.has_primary_failure_summary());
        assert!(malformed_commit.failure_batch_shape_is_clean());
        assert!(malformed_commit.commit_decision_accounting_is_consistent());
        let malformed_failure_return = malformed_commit.failure_return_summary();
        assert_eq!(
            malformed_failure_return.source,
            RuntimeKvExchangeFailureReturnSource::RuntimeKvImportReadiness
        );
        assert!(malformed_failure_return.has_failure_reports());
        assert!(malformed_failure_return.has_blocker_components());
        assert!(malformed_failure_return.failure_return_accounting_is_consistent());
        assert!(malformed_failure_return.can_return_runtime_failure());
        let malformed_return_report = malformed_commit
            .runtime_failure_return_report()
            .expect("malformed import readiness return report");
        assert_eq!(
            malformed_return_report.source,
            RuntimeKvExchangeFailureReturnSource::RuntimeKvImportReadiness
        );
        assert_eq!(
            malformed_return_report.primary_failure_summary,
            malformed_failure_summary
        );
        assert_eq!(malformed_return_report.failure_batch.kv_import_count, 1);
        assert!(malformed_return_report.failure_return_report_shape_is_clean());
        assert!(malformed_return_report.can_use_runtime_kv_exchange_failure_return_report());
        assert!(
            malformed_return_report
                .backend_message()
                .contains("runtime kv import readiness failed")
        );
        assert!(
            malformed_return_report
                .diagnostics_note()
                .starts_with("runtime_kv_import_error")
        );
        assert_eq!(
            malformed_return_report.inference_error().message,
            malformed_return_report.backend_message()
        );

        let missing = RuntimeKvImportBlockSummary::from_blocks(1, &[]);

        assert!(missing.is_empty());
        assert_eq!(missing.planned_blocks, 1);
        assert_eq!(missing.materialized_blocks, 0);
        assert_eq!(missing.runtime_namespace_blocks, 0);
        assert!(!missing.block_count_matches_plan());
        assert_eq!(missing.block_count_drift(), 1);
        assert!(!missing.all_blocks_are_runtime_namespace());
        assert_eq!(missing.runtime_namespace_drift_component_count(), 0);
        assert_eq!(missing.block_count_drift_component_count(), 1);
        assert_eq!(missing.import_block_problem_component_count(), 1);
        assert!(missing.has_import_block_problem_components());
        assert!(!missing.has_import_block_signals());
        assert!(missing.import_block_accounting_is_consistent());
        assert_eq!(
            missing.runtime_kv_import_block_commit_signal_component_count(),
            0
        );
        assert!(!missing.has_runtime_kv_import_block_commit_signals());
        assert_eq!(
            missing.runtime_kv_import_block_commit_blocker_component_count(),
            1
        );
        assert!(missing.has_runtime_kv_import_block_commit_blockers());
        assert!(missing.runtime_kv_import_block_commit_accounting_is_consistent());
        assert!(!missing.runtime_kv_import_block_commit_is_clean());
        assert!(!missing.import_block_shape_is_clean());
        assert!(!missing.can_commit_runtime_kv_import_blocks());

        let missing_readiness = RuntimeKvImportReadinessSummary::new(import, missing);

        assert!(missing_readiness.import_ready());
        assert!(!missing_readiness.blocks_ready());
        assert_eq!(
            missing_readiness.first_unready_stage(),
            Some(RuntimeKvImportReadinessStage::ImportBlocks)
        );
        assert_eq!(
            missing_readiness.first_blocking_stage(),
            Some(RuntimeKvImportReadinessStage::ImportBlocks)
        );
        assert_eq!(missing_readiness.block_blocker_component_count, 1);
        assert!(missing_readiness.runtime_kv_import_readiness_accounting_is_consistent());
        assert!(!missing_readiness.can_commit_runtime_kv_import_readiness());
        assert_eq!(missing_readiness.component_accounting_drift_count(), 0);
        assert_eq!(
            missing_readiness.runtime_kv_import_readiness_commit_problem_component_count(),
            1
        );
        assert_eq!(missing_readiness.failure_report_count(), 1);
        assert!(missing_readiness.has_failure_reports());
        assert_eq!(
            missing_readiness
                .primary_failure_summary()
                .expect("missing import readiness failure")
                .trace_label,
            "runtime_kv_import_error"
        );
        assert!(
            missing_readiness
                .commit_summary()
                .should_return_runtime_failure()
        );
    }

    #[test]
    fn runtime_kv_import_plan_respects_runtime_support_and_limits() {
        let architecture = TransformerRuntimeArchitecture::new(1, 4, 1, 1, 64);
        let disabled = RuntimeMetadata::new("model", "tok", 4096, 4);
        let limited = RuntimeMetadata::new("model", "tok", 4096, 4)
            .with_kv_exchange(true, false)
            .with_kv_limits(1, 0);
        let candidates = [
            RuntimeKvCandidate::new(1, vec![1.0], 0.5),
            RuntimeKvCandidate::new(2, vec![2.0], 0.5),
        ];

        assert!(!RuntimeKvImportPlan::new(&disabled, architecture, 2).is_enabled());
        assert_eq!(
            RuntimeKvImportPlan::new(&disabled, architecture, 2)
                .build_blocks(&candidates)
                .len(),
            0
        );
        assert_eq!(
            RuntimeKvImportPlan::new(&disabled, architecture, 2)
                .import_summary(&candidates)
                .planned_blocks,
            0
        );
        assert_eq!(
            RuntimeKvImportPlan::new(&limited, architecture, 2)
                .build_blocks(&candidates)
                .len(),
            1
        );
    }

    #[test]
    fn runtime_kv_import_plan_allows_unknown_embedding_dimensions() {
        let runtime = RuntimeMetadata::new("model", "tok", 4096, 0).with_kv_exchange(true, false);
        let architecture = TransformerRuntimeArchitecture::new(1, 4, 1, 1, 64);
        let plan = RuntimeKvImportPlan::new(&runtime, architecture, 1);
        let blocks = plan.build_blocks(&[RuntimeKvCandidate::new(
            7,
            vec![1.0, f32::INFINITY, 3.0],
            f32::NAN,
        )]);

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].key, vec![1.0, 0.0, 3.0]);
        assert_eq!(blocks[0].value, vec![0.05, 0.0, 0.15]);
        assert_eq!(blocks[0].score, 0.05);
    }

    #[test]
    fn runtime_kv_import_summary_marks_empty_candidates() {
        let runtime = RuntimeMetadata::new("model", "tok", 4096, 4).with_kv_exchange(true, false);
        let architecture = TransformerRuntimeArchitecture::new(1, 4, 1, 1, 64);
        let plan = RuntimeKvImportPlan::new(&runtime, architecture, 2);
        let candidates = [
            RuntimeKvCandidate::new(1, Vec::new(), 0.5),
            RuntimeKvCandidate::new(2, Vec::new(), 0.6),
        ];

        let summary = plan.import_summary(&candidates);

        assert!(summary.enabled);
        assert!(!summary.will_import());
        assert!(summary.skipped_due_to_empty_candidates());
        assert_eq!(summary.candidate_count, 2);
        assert_eq!(summary.non_empty_candidate_count, 0);
        assert_eq!(summary.planned_blocks, 0);
        assert!(summary.has_candidates());
        assert!(!summary.has_non_empty_candidates());
        assert_eq!(summary.empty_candidate_count(), 2);
        assert!(summary.has_embedding_dimensions());
        assert!(summary.enabled_matches_capacity());
        assert!(summary.candidate_counts_are_valid());
        assert!(summary.planned_blocks_within_limit());
        assert!(summary.planned_blocks_within_candidates());
        assert!(summary.import_limit_flag_matches_shape());
        assert!(summary.disabled_import_is_empty());
        assert!(summary.embedding_dimensions_shape_is_valid());
        assert_eq!(summary.import_signal_component_count(), 4);
        assert!(summary.has_import_signals());
        assert_eq!(summary.import_shape_problem_component_count(), 0);
        assert!(!summary.has_import_shape_problem_components());
        assert!(summary.import_accounting_is_consistent());
        assert_eq!(summary.runtime_kv_import_commit_signal_component_count(), 4);
        assert!(summary.has_runtime_kv_import_commit_signals());
        assert_eq!(
            summary.runtime_kv_import_commit_blocker_component_count(),
            0
        );
        assert!(!summary.has_runtime_kv_import_commit_blockers());
        assert!(summary.runtime_kv_import_commit_accounting_is_consistent());
        assert!(summary.runtime_kv_import_commit_is_clean());
        assert!(summary.import_commit_is_clean());
        assert!(summary.import_shape_is_clean());
        assert!(summary.can_commit_runtime_kv_import());

        let block_summary = RuntimeKvImportBlockSummary::from_blocks(summary.planned_blocks, &[]);
        let readiness = RuntimeKvImportReadinessSummary::new(summary, block_summary);

        assert!(block_summary.is_empty());
        assert!(block_summary.runtime_kv_import_block_commit_is_clean());
        assert!(!block_summary.can_commit_runtime_kv_import_blocks());
        assert!(readiness.import_ready());
        assert!(readiness.blocks_ready());
        assert_eq!(readiness.first_unready_stage(), None);
        assert_eq!(readiness.first_blocking_stage(), None);
        assert_eq!(readiness.import_signal_component_count, 4);
        assert_eq!(readiness.block_signal_component_count, 0);
        assert_eq!(readiness.import_blocker_component_count, 0);
        assert_eq!(readiness.block_blocker_component_count, 0);
        assert_eq!(
            readiness.runtime_kv_import_readiness_signal_component_count(),
            4
        );
        assert!(readiness.has_runtime_kv_import_readiness_signals());
        assert_eq!(
            readiness.runtime_kv_import_readiness_blocker_component_count(),
            0
        );
        assert!(!readiness.has_runtime_kv_import_readiness_blockers());
        assert!(readiness.runtime_kv_import_readiness_accounting_is_consistent());
        assert!(readiness.runtime_kv_import_readiness_commit_is_clean());
        assert!(readiness.can_commit_runtime_kv_import_readiness());
    }

    #[test]
    fn runtime_kv_import_readiness_exposes_commit_action_boundary() {
        let clean_import = RuntimeKvImportSummary {
            enabled: false,
            max_blocks: 0,
            candidate_count: 0,
            non_empty_candidate_count: 0,
            planned_blocks: 0,
            hit_import_limit: false,
            embedding_dimensions: None,
        };
        let clean_readiness = RuntimeKvImportReadinessSummary::new(
            clean_import,
            RuntimeKvImportBlockSummary::from_blocks(clean_import.planned_blocks, &[]),
        );

        assert_eq!(
            clean_readiness.runtime_kv_import_readiness_commit_action(),
            RuntimeKvImportReadinessCommitAction::CommitRuntimeKvImport
        );
        assert_eq!(
            clean_readiness.commit_summary().action,
            clean_readiness.runtime_kv_import_readiness_commit_action()
        );

        let blocked_import = RuntimeKvImportSummary {
            enabled: false,
            max_blocks: 0,
            candidate_count: 1,
            non_empty_candidate_count: 1,
            planned_blocks: 1,
            hit_import_limit: true,
            embedding_dimensions: Some(0),
        };
        let blocked_readiness = RuntimeKvImportReadinessSummary::new(
            blocked_import,
            RuntimeKvImportBlockSummary::from_blocks(blocked_import.planned_blocks, &[]),
        );

        assert_eq!(
            blocked_readiness.runtime_kv_import_readiness_commit_action(),
            RuntimeKvImportReadinessCommitAction::ReturnRuntimeFailure
        );
        assert_eq!(
            blocked_readiness.commit_summary().action,
            blocked_readiness.runtime_kv_import_readiness_commit_action()
        );
    }

    #[test]
    fn runtime_kv_import_summary_counts_public_shape_drift() {
        let summary = RuntimeKvImportSummary {
            enabled: false,
            max_blocks: 1,
            candidate_count: 1,
            non_empty_candidate_count: 2,
            planned_blocks: 3,
            hit_import_limit: true,
            embedding_dimensions: Some(0),
        };

        assert!(!summary.will_import());
        assert!(!summary.skipped_due_to_empty_candidates());
        assert!(summary.has_candidates());
        assert!(summary.has_non_empty_candidates());
        assert_eq!(summary.empty_candidate_count(), 0);
        assert!(summary.has_embedding_dimensions());
        assert!(!summary.enabled_matches_capacity());
        assert!(!summary.candidate_counts_are_valid());
        assert!(!summary.planned_blocks_within_limit());
        assert!(!summary.planned_blocks_within_candidates());
        assert!(!summary.import_limit_flag_matches_shape());
        assert!(!summary.disabled_import_is_empty());
        assert!(!summary.embedding_dimensions_shape_is_valid());
        assert_eq!(summary.import_signal_component_count(), 4);
        assert!(summary.has_import_signals());
        assert_eq!(summary.import_shape_problem_component_count(), 7);
        assert!(summary.has_import_shape_problem_components());
        assert!(summary.import_accounting_is_consistent());
        assert_eq!(summary.runtime_kv_import_commit_signal_component_count(), 4);
        assert!(summary.has_runtime_kv_import_commit_signals());
        assert_eq!(
            summary.runtime_kv_import_commit_blocker_component_count(),
            7
        );
        assert!(summary.has_runtime_kv_import_commit_blockers());
        assert!(summary.runtime_kv_import_commit_accounting_is_consistent());
        assert!(!summary.runtime_kv_import_commit_is_clean());
        assert!(!summary.import_commit_is_clean());
        assert!(!summary.import_shape_is_clean());
        assert!(!summary.can_commit_runtime_kv_import());

        let readiness = RuntimeKvImportReadinessSummary::new(
            summary,
            RuntimeKvImportBlockSummary::from_blocks(summary.planned_blocks, &[]),
        );

        assert!(!readiness.import_ready());
        assert!(!readiness.blocks_ready());
        assert_eq!(
            readiness.first_unready_stage(),
            Some(RuntimeKvImportReadinessStage::ImportPlan)
        );
        assert_eq!(
            readiness.first_blocking_stage(),
            Some(RuntimeKvImportReadinessStage::ImportPlan)
        );
        assert_eq!(readiness.import_blocker_component_count, 7);
        assert!(readiness.block_blocker_component_count > 0);
        assert!(readiness.runtime_kv_import_readiness_accounting_is_consistent());
        assert!(!readiness.can_commit_runtime_kv_import_readiness());
    }

    #[test]
    fn kv_block_shape_summary_reports_runtime_exchange_boundaries() {
        let runtime = KvBlock::new(
            1,
            KvNamespace::Runtime,
            2,
            3,
            8..10,
            vec![0.1, 0.2],
            vec![0.3, 0.4],
        );
        let invalid = KvBlock::new(2, KvNamespace::Gist, 0, 0, 4..4, vec![f32::NAN], Vec::new());

        let runtime_summary = runtime.shape_summary();
        let invalid_summary = invalid.shape_summary();

        assert_eq!(runtime_summary.namespace_label, "runtime");
        assert!(runtime_summary.is_runtime_namespace);
        assert_eq!(runtime_summary.layer, 2);
        assert_eq!(runtime_summary.head, 3);
        assert_eq!(runtime_summary.token_len, 2);
        assert_eq!(runtime_summary.key_len, 2);
        assert_eq!(runtime_summary.value_len, 2);
        assert!(runtime_summary.key_value_len_match);
        assert!(!runtime_summary.has_empty_vectors);
        assert!(!runtime_summary.has_non_finite_values);
        assert!(runtime_summary.has_runtime_exchange_shape());
        assert!(!runtime_summary.token_range_is_empty());
        assert_eq!(runtime_summary.vector_len_delta(), 0);
        assert!(runtime_summary.vectors_are_paired_and_finite());
        assert_eq!(
            runtime_summary.runtime_namespace_signal_component_count(),
            1
        );
        assert_eq!(runtime_summary.token_span_signal_component_count(), 1);
        assert_eq!(runtime_summary.vector_payload_signal_component_count(), 1);
        assert_eq!(runtime_summary.block_shape_signal_component_count(), 3);
        assert!(runtime_summary.has_block_shape_signals());
        assert_eq!(runtime_summary.runtime_namespace_drift_component_count(), 0);
        assert_eq!(runtime_summary.token_range_problem_component_count(), 0);
        assert_eq!(runtime_summary.vector_length_drift_component_count(), 0);
        assert_eq!(runtime_summary.empty_vector_problem_component_count(), 0);
        assert_eq!(
            runtime_summary.non_finite_value_problem_component_count(),
            0
        );
        assert_eq!(
            runtime_summary.runtime_exchange_shape_problem_component_count(),
            0
        );
        assert!(!runtime_summary.has_runtime_exchange_shape_problem_components());
        assert!(runtime_summary.runtime_exchange_shape_accounting_is_consistent());
        assert!(runtime_summary.runtime_exchange_shape_is_clean());
        assert!(runtime_summary.can_use_runtime_exchange_block());

        assert_eq!(invalid_summary.namespace_label, "gist");
        assert!(!invalid_summary.is_runtime_namespace);
        assert_eq!(invalid_summary.token_len, 0);
        assert!(!invalid_summary.key_value_len_match);
        assert!(invalid_summary.has_empty_vectors);
        assert!(invalid_summary.has_non_finite_values);
        assert!(!invalid_summary.has_runtime_exchange_shape());
        assert!(invalid_summary.token_range_is_empty());
        assert_eq!(invalid_summary.vector_len_delta(), 1);
        assert!(!invalid_summary.vectors_are_paired_and_finite());
        assert_eq!(
            invalid_summary.runtime_namespace_signal_component_count(),
            0
        );
        assert_eq!(invalid_summary.token_span_signal_component_count(), 0);
        assert_eq!(invalid_summary.vector_payload_signal_component_count(), 0);
        assert_eq!(invalid_summary.block_shape_signal_component_count(), 0);
        assert!(!invalid_summary.has_block_shape_signals());
        assert_eq!(invalid_summary.runtime_namespace_drift_component_count(), 1);
        assert_eq!(invalid_summary.token_range_problem_component_count(), 1);
        assert_eq!(invalid_summary.vector_length_drift_component_count(), 1);
        assert_eq!(invalid_summary.empty_vector_problem_component_count(), 1);
        assert_eq!(
            invalid_summary.non_finite_value_problem_component_count(),
            1
        );
        assert_eq!(
            invalid_summary.runtime_exchange_shape_problem_component_count(),
            5
        );
        assert!(invalid_summary.has_runtime_exchange_shape_problem_components());
        assert!(invalid_summary.runtime_exchange_shape_accounting_is_consistent());
        assert!(!invalid_summary.runtime_exchange_shape_is_clean());
        assert!(!invalid_summary.can_use_runtime_exchange_block());
    }

    #[test]
    fn runtime_kv_contract_reports_blocks_above_limit() {
        let runtime = RuntimeMetadata::new("model", "tok", 8, 4);
        let architecture = TransformerRuntimeArchitecture::new(2, 4, 2, 2, 4);
        let contract = RuntimeKvBlockContract::new(1, 8, RuntimeKvDirection::Imported);
        let blocks = [
            KvBlock::new(
                1,
                KvNamespace::Runtime,
                1,
                1,
                2..4,
                vec![0.1, 0.2, 0.3, 0.4],
                vec![0.4, 0.3, 0.2, 0.1],
            ),
            KvBlock::new(2, KvNamespace::Runtime, 99, 0, 0..1, vec![0.1], vec![0.2]),
        ];

        let report = contract.validate_blocks(&blocks, &runtime, architecture);
        let joined = report.violations.join("\n");
        let summary = report.validation_summary();
        let boundary = contract.validation_boundary_summary(&report);

        assert!(!report.is_valid());
        assert!(!summary.valid);
        assert!(summary.has_violations());
        assert!(summary.accepted_any());
        assert!(!summary.rejected_all());
        assert!(summary.partially_accepted());
        assert_eq!(summary.accepted_count, 1);
        assert_eq!(summary.violation_count, 1);
        assert!(summary.valid_flag_matches_violations());
        assert_eq!(summary.accepted_signal_component_count(), 1);
        assert_eq!(summary.partial_acceptance_signal_component_count(), 1);
        assert_eq!(summary.rejected_all_signal_component_count(), 0);
        assert_eq!(summary.validation_signal_component_count(), 2);
        assert!(summary.has_validation_signals());
        assert_eq!(summary.violation_problem_component_count(), 1);
        assert_eq!(summary.valid_flag_drift_component_count(), 0);
        assert_eq!(summary.validation_problem_component_count(), 1);
        assert!(summary.has_validation_problem_components());
        assert!(summary.validation_accounting_is_consistent());
        assert_eq!(
            summary.runtime_kv_validation_commit_signal_component_count(),
            2
        );
        assert!(summary.has_runtime_kv_validation_commit_signals());
        assert_eq!(
            summary.runtime_kv_validation_commit_blocker_component_count(),
            1
        );
        assert!(summary.has_runtime_kv_validation_commit_blockers());
        assert!(summary.runtime_kv_validation_commit_accounting_is_consistent());
        assert!(!summary.runtime_kv_validation_commit_is_clean());
        assert!(!summary.validation_commit_is_clean());
        assert!(!summary.validation_shape_is_clean());
        assert!(!summary.can_commit_runtime_kv_validation());
        assert_eq!(boundary.direction, RuntimeKvDirection::Imported);
        assert_eq!(boundary.direction_label, "imported");
        assert_eq!(boundary.failure_trace_label, "runtime_kv_import_error");
        assert_eq!(boundary.max_blocks, 1);
        assert_eq!(boundary.token_upper_bound, 8);
        assert_eq!(boundary.accepted_count, 1);
        assert_eq!(boundary.violation_count, 1);
        assert!(!boundary.valid);
        assert!(boundary.direction_label_matches_kind());
        assert!(boundary.failure_trace_label_matches_direction());
        assert!(boundary.has_block_capacity());
        assert!(boundary.has_token_bound());
        assert!(boundary.accepted_any());
        assert!(boundary.has_violations());
        assert!(!boundary.rejected_all());
        assert!(boundary.partially_accepted());
        assert!(boundary.accepted_within_contract_limit());
        assert!(boundary.valid_flag_matches_violations());
        assert!(boundary.maps_to_runtime_kv_failure());
        assert_eq!(boundary.boundary_signal_component_count(), 5);
        assert!(boundary.has_boundary_signals());
        assert_eq!(boundary.boundary_problem_component_count(), 1);
        assert!(boundary.has_boundary_problem_components());
        assert!(boundary.boundary_accounting_is_consistent());
        assert_eq!(
            boundary.runtime_kv_boundary_commit_signal_component_count(),
            5
        );
        assert!(boundary.has_runtime_kv_boundary_commit_signals());
        assert_eq!(
            boundary.runtime_kv_boundary_commit_blocker_component_count(),
            1
        );
        assert!(boundary.has_runtime_kv_boundary_commit_blockers());
        assert!(boundary.runtime_kv_boundary_commit_accounting_is_consistent());
        assert!(!boundary.runtime_kv_boundary_commit_is_clean());
        assert!(!boundary.boundary_shape_is_clean());
        assert!(!boundary.can_commit_runtime_kv_boundary());
        assert_eq!(report.accepted, vec![blocks[0].clone()]);
        assert!(joined.contains("imported KV block count 2 exceeds contract max_blocks 1"));
    }

    #[test]
    fn runtime_kv_contract_reports_root_style_block_violations() {
        let runtime = RuntimeMetadata::new("model", "tok", 8, 2);
        let architecture = TransformerRuntimeArchitecture::new(2, 2, 2, 2, 4);
        let contract = RuntimeKvBlockContract::new(16, 4, RuntimeKvDirection::Exported);
        let invalid = [
            KvBlock::new(1, KvNamespace::Semantic, 0, 0, 0..1, vec![0.1], vec![0.2]),
            KvBlock::new(2, KvNamespace::Runtime, 2, 0, 0..1, vec![0.1], vec![0.2]),
            KvBlock::new(3, KvNamespace::Runtime, 0, 2, 0..1, vec![0.1], vec![0.2]),
            KvBlock::new(4, KvNamespace::Runtime, 0, 0, 3..3, vec![0.1], vec![0.2]),
            KvBlock::new(5, KvNamespace::Runtime, 0, 0, 3..5, vec![0.1], vec![0.2]),
            KvBlock::new(6, KvNamespace::Runtime, 0, 0, 0..1, Vec::new(), vec![0.2]),
            KvBlock::new(
                7,
                KvNamespace::Runtime,
                0,
                0,
                0..1,
                vec![0.1],
                vec![0.2, 0.3],
            ),
            KvBlock::new(
                8,
                KvNamespace::Runtime,
                0,
                0,
                0..1,
                vec![0.0, 1.0, 2.0],
                vec![0.0, 1.0, 2.0],
            ),
            KvBlock::new(
                9,
                KvNamespace::Runtime,
                0,
                0,
                0..1,
                vec![f32::NAN],
                vec![f32::INFINITY],
            ),
        ];

        let report = contract.validate_blocks(&invalid, &runtime, architecture);
        let joined = report.violations.join("\n");
        let summary = report.validation_summary();
        let boundary = contract.validation_boundary_summary(&report);

        assert!(report.accepted.is_empty());
        assert!(!summary.valid);
        assert!(summary.has_violations());
        assert!(!summary.accepted_any());
        assert!(summary.rejected_all());
        assert!(!summary.partially_accepted());
        assert_eq!(summary.accepted_count, 0);
        assert_eq!(summary.violation_count, report.violations.len());
        assert!(summary.valid_flag_matches_violations());
        assert_eq!(summary.accepted_signal_component_count(), 0);
        assert_eq!(summary.partial_acceptance_signal_component_count(), 0);
        assert_eq!(summary.rejected_all_signal_component_count(), 1);
        assert_eq!(summary.validation_signal_component_count(), 1);
        assert!(summary.has_validation_signals());
        assert_eq!(summary.violation_problem_component_count(), 1);
        assert_eq!(summary.valid_flag_drift_component_count(), 0);
        assert_eq!(summary.validation_problem_component_count(), 1);
        assert!(summary.has_validation_problem_components());
        assert!(summary.validation_accounting_is_consistent());
        assert_eq!(
            summary.runtime_kv_validation_commit_signal_component_count(),
            1
        );
        assert!(summary.has_runtime_kv_validation_commit_signals());
        assert_eq!(
            summary.runtime_kv_validation_commit_blocker_component_count(),
            1
        );
        assert!(summary.has_runtime_kv_validation_commit_blockers());
        assert!(summary.runtime_kv_validation_commit_accounting_is_consistent());
        assert!(!summary.runtime_kv_validation_commit_is_clean());
        assert!(!summary.validation_commit_is_clean());
        assert!(!summary.validation_shape_is_clean());
        assert!(!summary.can_commit_runtime_kv_validation());
        assert_eq!(boundary.direction, RuntimeKvDirection::Exported);
        assert_eq!(boundary.direction_label, "exported");
        assert_eq!(boundary.failure_trace_label, "runtime_kv_export_error");
        assert_eq!(boundary.accepted_count, 0);
        assert_eq!(boundary.violation_count, report.violations.len());
        assert!(!boundary.valid);
        assert!(!boundary.accepted_any());
        assert!(boundary.has_violations());
        assert!(boundary.rejected_all());
        assert!(!boundary.partially_accepted());
        assert!(boundary.accepted_within_contract_limit());
        assert!(boundary.valid_flag_matches_violations());
        assert!(boundary.maps_to_runtime_kv_failure());
        assert_eq!(boundary.boundary_signal_component_count(), 4);
        assert!(boundary.has_boundary_signals());
        assert_eq!(boundary.boundary_problem_component_count(), 1);
        assert!(boundary.has_boundary_problem_components());
        assert!(boundary.boundary_accounting_is_consistent());
        assert_eq!(
            boundary.runtime_kv_boundary_commit_signal_component_count(),
            4
        );
        assert!(boundary.has_runtime_kv_boundary_commit_signals());
        assert_eq!(
            boundary.runtime_kv_boundary_commit_blocker_component_count(),
            1
        );
        assert!(boundary.has_runtime_kv_boundary_commit_blockers());
        assert!(boundary.runtime_kv_boundary_commit_accounting_is_consistent());
        assert!(!boundary.runtime_kv_boundary_commit_is_clean());
        assert!(!boundary.boundary_shape_is_clean());
        assert!(!boundary.can_commit_runtime_kv_boundary());
        assert!(joined.contains("namespace semantic is not runtime"));
        assert!(joined.contains("layer 2 exceeds manifest layer_count 2"));
        assert!(joined.contains("head 2 exceeds manifest kv_heads 2"));
        assert!(joined.contains("token range 3..3 is empty or reversed"));
        assert!(joined.contains("token_end 5 exceeds KV token bound 4"));
        assert!(joined.contains("key and value vectors must both be non-empty"));
        assert!(joined.contains("key/value dimensions differ: key=1 value=2"));
        assert!(joined.contains("key/value dimensions 3 exceed per-block bound 2"));
        assert!(joined.contains("exported key contains non-finite value"));
        assert!(joined.contains("exported value contains non-finite value"));
    }

    #[test]
    fn runtime_kv_block_contract_check_accepts_clean_runtime_block() {
        let runtime = RuntimeMetadata::new("model", "tok", 8, 2);
        let architecture = TransformerRuntimeArchitecture::new(2, 2, 2, 2, 4);
        let contract = RuntimeKvBlockContract::new(16, 4, RuntimeKvDirection::Imported);
        let block = KvBlock::new(
            1,
            KvNamespace::Runtime,
            1,
            1,
            1..2,
            vec![0.1, 0.2],
            vec![0.3, 0.4],
        );

        let summary = contract.block_check_summary(3, &block, &runtime, architecture);

        assert_eq!(summary.block_index, 3);
        assert_eq!(summary.direction, RuntimeKvDirection::Imported);
        assert_eq!(summary.layer_count, 2);
        assert_eq!(summary.kv_heads, 2);
        assert_eq!(summary.token_upper_bound, 4);
        assert_eq!(summary.vector_bound, 2);
        assert!(summary.namespace_is_runtime);
        assert!(summary.layer_within_bounds);
        assert!(summary.head_within_bounds);
        assert!(summary.token_range_is_valid);
        assert!(summary.token_end_within_bound);
        assert!(summary.vectors_are_present);
        assert!(summary.key_value_len_matches);
        assert!(summary.vector_len_within_bound);
        assert!(summary.key_values_are_finite);
        assert!(summary.value_values_are_finite);
        assert_eq!(summary.namespace_problem_component_count(), 0);
        assert_eq!(summary.layer_head_problem_component_count(), 0);
        assert_eq!(summary.token_problem_component_count(), 0);
        assert_eq!(summary.vector_problem_component_count(), 0);
        assert_eq!(summary.contract_check_problem_component_count(), 0);
        assert!(!summary.has_contract_check_problem_components());
        assert_eq!(summary.contract_check_signal_component_count(), 10);
        assert!(summary.has_contract_check_signals());
        assert!(summary.contract_check_accounting_is_consistent());
        assert_eq!(
            summary.runtime_kv_block_contract_check_commit_signal_component_count(),
            10
        );
        assert!(summary.has_runtime_kv_block_contract_check_commit_signals());
        assert_eq!(
            summary.runtime_kv_block_contract_check_commit_blocker_component_count(),
            0
        );
        assert!(!summary.has_runtime_kv_block_contract_check_commit_blockers());
        assert!(summary.runtime_kv_block_contract_check_commit_accounting_is_consistent());
        assert!(summary.runtime_kv_block_contract_check_commit_is_clean());
        assert!(summary.contract_check_shape_is_clean());
        assert!(summary.can_commit_runtime_kv_block_contract_check());
        assert!(summary.can_accept_runtime_kv_block());
    }

    #[test]
    fn runtime_kv_block_contract_check_splits_payload_problem_components() {
        let runtime = RuntimeMetadata::new("model", "tok", 8, 2);
        let architecture = TransformerRuntimeArchitecture::new(2, 2, 2, 2, 4);
        let contract = RuntimeKvBlockContract::new(16, 4, RuntimeKvDirection::Exported);

        let semantic_namespace = contract.block_check_summary(
            0,
            &KvBlock::new(1, KvNamespace::Semantic, 0, 0, 0..1, vec![0.1], vec![0.2]),
            &runtime,
            architecture,
        );
        assert!(!semantic_namespace.namespace_is_runtime);
        assert_eq!(semantic_namespace.namespace_problem_component_count(), 1);
        assert_eq!(
            semantic_namespace.contract_check_problem_component_count(),
            1
        );
        assert_eq!(
            semantic_namespace.contract_check_signal_component_count(),
            9
        );
        assert_eq!(
            semantic_namespace.runtime_kv_block_contract_check_commit_signal_component_count(),
            9
        );
        assert!(semantic_namespace.has_runtime_kv_block_contract_check_commit_signals());
        assert_eq!(
            semantic_namespace.runtime_kv_block_contract_check_commit_blocker_component_count(),
            1
        );
        assert!(semantic_namespace.has_runtime_kv_block_contract_check_commit_blockers());
        assert!(
            semantic_namespace.runtime_kv_block_contract_check_commit_accounting_is_consistent()
        );
        assert!(!semantic_namespace.runtime_kv_block_contract_check_commit_is_clean());
        assert!(!semantic_namespace.can_commit_runtime_kv_block_contract_check());
        assert!(!semantic_namespace.can_accept_runtime_kv_block());

        let layer_overflow = contract.block_check_summary(
            1,
            &KvBlock::new(2, KvNamespace::Runtime, 2, 0, 0..1, vec![0.1], vec![0.2]),
            &runtime,
            architecture,
        );
        assert!(!layer_overflow.layer_within_bounds);
        assert_eq!(layer_overflow.layer_head_problem_component_count(), 1);
        assert_eq!(layer_overflow.contract_check_problem_component_count(), 1);

        let head_overflow = contract.block_check_summary(
            2,
            &KvBlock::new(3, KvNamespace::Runtime, 0, 2, 0..1, vec![0.1], vec![0.2]),
            &runtime,
            architecture,
        );
        assert!(!head_overflow.head_within_bounds);
        assert_eq!(head_overflow.layer_head_problem_component_count(), 1);
        assert_eq!(head_overflow.contract_check_problem_component_count(), 1);

        let empty_range = contract.block_check_summary(
            3,
            &KvBlock::new(4, KvNamespace::Runtime, 0, 0, 3..3, vec![0.1], vec![0.2]),
            &runtime,
            architecture,
        );
        assert!(!empty_range.token_range_is_valid);
        assert!(empty_range.token_end_within_bound);
        assert_eq!(empty_range.token_problem_component_count(), 1);

        let token_overflow = contract.block_check_summary(
            4,
            &KvBlock::new(5, KvNamespace::Runtime, 0, 0, 3..5, vec![0.1], vec![0.2]),
            &runtime,
            architecture,
        );
        assert!(token_overflow.token_range_is_valid);
        assert!(!token_overflow.token_end_within_bound);
        assert_eq!(token_overflow.token_problem_component_count(), 1);

        let missing_vector = contract.block_check_summary(
            5,
            &KvBlock::new(6, KvNamespace::Runtime, 0, 0, 0..1, Vec::new(), Vec::new()),
            &runtime,
            architecture,
        );
        assert!(!missing_vector.vectors_are_present);
        assert!(missing_vector.key_value_len_matches);
        assert_eq!(missing_vector.vector_problem_component_count(), 1);

        let mismatched_vector = contract.block_check_summary(
            6,
            &KvBlock::new(
                7,
                KvNamespace::Runtime,
                0,
                0,
                0..1,
                vec![0.1],
                vec![0.2, 0.3],
            ),
            &runtime,
            architecture,
        );
        assert!(mismatched_vector.vectors_are_present);
        assert!(!mismatched_vector.key_value_len_matches);
        assert_eq!(mismatched_vector.vector_problem_component_count(), 1);

        let oversized_vector = contract.block_check_summary(
            7,
            &KvBlock::new(
                8,
                KvNamespace::Runtime,
                0,
                0,
                0..1,
                vec![0.0, 1.0, 2.0],
                vec![0.0, 1.0, 2.0],
            ),
            &runtime,
            architecture,
        );
        assert!(!oversized_vector.vector_len_within_bound);
        assert_eq!(oversized_vector.vector_problem_component_count(), 1);

        let non_finite = contract.block_check_summary(
            8,
            &KvBlock::new(
                9,
                KvNamespace::Runtime,
                0,
                0,
                0..1,
                vec![f32::NAN],
                vec![f32::INFINITY],
            ),
            &runtime,
            architecture,
        );
        assert!(!non_finite.key_values_are_finite);
        assert!(!non_finite.value_values_are_finite);
        assert_eq!(non_finite.vector_problem_component_count(), 2);
        assert_eq!(non_finite.contract_check_problem_component_count(), 2);
        assert!(non_finite.contract_check_accounting_is_consistent());
        assert_eq!(
            non_finite.runtime_kv_block_contract_check_commit_blocker_component_count(),
            2
        );
        assert!(non_finite.has_runtime_kv_block_contract_check_commit_blockers());
        assert!(non_finite.runtime_kv_block_contract_check_commit_accounting_is_consistent());
        assert!(!non_finite.runtime_kv_block_contract_check_commit_is_clean());
        assert!(!non_finite.contract_check_shape_is_clean());
        assert!(!non_finite.can_commit_runtime_kv_block_contract_check());
        assert!(!non_finite.can_accept_runtime_kv_block());
    }

    #[test]
    fn runtime_kv_contract_reports_invalid_zero_architecture() {
        let runtime = RuntimeMetadata::new("model", "tok", 8, 2);
        let architecture = TransformerRuntimeArchitecture::new(0, 2, 2, 0, 4);
        let contract = RuntimeKvBlockContract::new(1, 8, RuntimeKvDirection::Imported);
        let block = KvBlock::new(1, KvNamespace::Runtime, 0, 0, 0..1, vec![0.1], vec![0.2]);

        let violations = contract.validate_block(0, &block, &runtime, architecture);
        let joined = violations.join("\n");
        let summary = contract.block_check_summary(0, &block, &runtime, architecture);

        assert!(joined.contains("layer 0 exceeds manifest layer_count 0"));
        assert!(joined.contains("head 0 exceeds manifest kv_heads 0"));
        assert!(!summary.layer_within_bounds);
        assert!(!summary.head_within_bounds);
        assert_eq!(summary.layer_head_problem_component_count(), 2);
        assert_eq!(summary.contract_check_problem_component_count(), 2);
        assert!(summary.contract_check_accounting_is_consistent());
        assert_eq!(
            summary.runtime_kv_block_contract_check_commit_blocker_component_count(),
            2
        );
        assert!(summary.has_runtime_kv_block_contract_check_commit_blockers());
        assert!(summary.runtime_kv_block_contract_check_commit_accounting_is_consistent());
        assert!(!summary.runtime_kv_block_contract_check_commit_is_clean());
        assert!(!summary.contract_check_shape_is_clean());
        assert!(!summary.can_commit_runtime_kv_block_contract_check());
        assert!(!summary.can_accept_runtime_kv_block());
    }

    #[test]
    fn runtime_kv_contract_derives_import_and_export_limits_from_request() {
        let runtime = RuntimeMetadata::new("model", "tok", 128, 4)
            .with_kv_exchange(true, true)
            .with_kv_limits(2, 3);
        let request = InferenceRequest::new("prompt", TaskProfile::Coding)
            .with_prompt_tokens(120)
            .with_max_tokens(16)
            .with_runtime(runtime);
        let architecture = TransformerRuntimeArchitecture::new(2, 4, 2, 2, 64);
        let transformer_plan = TransformerPlanDigest::new(
            Some("kv-request"),
            vec![
                TransformerLayerBudget::new(0, TransformerAttentionKind::Global, 0.5, 64),
                TransformerLayerBudget::new(1, TransformerAttentionKind::LocalWindow, 0.5, 64),
            ],
        );
        let execution = crate::adapter::AdapterExecutionContext::new([RuntimeAdapter::Cuda])
            .with_pressure(0.20, 0.80)
            .with_kv_prefetch_blocks(2);
        let planning = RuntimePlanningDigest::from_request(
            &request,
            RouteBudget {
                threshold: 0.4,
                attention_tokens: 8,
                fast_tokens: 2,
                attention_fraction: 0.8,
            },
            &execution,
            &[],
            &DeterministicFhtDkeBudgeter::new(0.10, 0.60, 4),
        );
        let envelope = RuntimeRequestEnvelope::from_parts(
            &request,
            architecture,
            RouteBudget::default(),
            HierarchyWeights::for_profile(TaskProfile::Coding),
            &transformer_plan,
            &execution,
            2,
        )
        .with_planning_digest(planning);

        let imports = RuntimeKvBlockContract::for_request_imports(&envelope);
        let exports = RuntimeKvBlockContract::for_request_exports(&envelope);
        let import_summary = imports.contract_summary();
        let export_summary = exports.contract_summary();

        assert_eq!(imports.max_blocks, 2);
        assert_eq!(
            imports.token_upper_bound,
            envelope.generation_budget.planned_context_tokens
        );
        assert_eq!(imports.direction, RuntimeKvDirection::Imported);
        assert_eq!(
            import_summary,
            RuntimeKvBlockContractSummary {
                max_blocks: 2,
                token_upper_bound: envelope.generation_budget.planned_context_tokens,
                direction: RuntimeKvDirection::Imported,
                direction_label: "imported",
            }
        );
        assert!(import_summary.has_block_capacity());
        assert!(import_summary.has_token_bound());
        assert!(import_summary.direction_label_matches_kind());
        assert_eq!(import_summary.contract_signal_component_count(), 3);
        assert!(import_summary.has_contract_signals());
        assert_eq!(import_summary.contract_problem_component_count(), 0);
        assert!(!import_summary.has_contract_problem_components());
        assert!(import_summary.contract_accounting_is_consistent());
        assert_eq!(
            import_summary.runtime_kv_block_contract_commit_signal_component_count(),
            3
        );
        assert!(import_summary.has_runtime_kv_block_contract_commit_signals());
        assert_eq!(
            import_summary.runtime_kv_block_contract_commit_blocker_component_count(),
            0
        );
        assert!(!import_summary.has_runtime_kv_block_contract_commit_blockers());
        assert!(import_summary.runtime_kv_block_contract_commit_accounting_is_consistent());
        assert!(import_summary.runtime_kv_block_contract_commit_is_clean());
        assert!(import_summary.contract_shape_is_clean());
        assert!(import_summary.can_commit_runtime_kv_block_contract());
        assert!(import_summary.can_use_runtime_kv_block_contract());
        assert_eq!(
            exports.max_blocks,
            planning.planned_kv_exchange().export_blocks
        );
        assert_eq!(
            exports.token_upper_bound,
            envelope.generation_budget.planned_context_tokens
        );
        assert_eq!(exports.direction, RuntimeKvDirection::Exported);
        assert_eq!(export_summary.direction_label, "exported");
        assert!(export_summary.has_token_bound());
        assert_eq!(export_summary.contract_problem_component_count(), 0);
        assert!(export_summary.contract_accounting_is_consistent());
        assert_eq!(
            export_summary.runtime_kv_block_contract_commit_blocker_component_count(),
            0
        );
        assert!(!export_summary.has_runtime_kv_block_contract_commit_blockers());
        assert!(export_summary.runtime_kv_block_contract_commit_accounting_is_consistent());
        assert_eq!(
            export_summary.can_commit_runtime_kv_block_contract(),
            export_summary.has_block_capacity()
        );
        assert!(export_summary.contract_shape_is_clean());
        assert_eq!(
            export_summary.can_use_runtime_kv_block_contract(),
            export_summary.has_block_capacity()
        );

        let direct_export =
            RuntimeKvBlockContract::new(3, 16, RuntimeKvDirection::Exported).contract_summary();

        assert!(direct_export.has_block_capacity());
        assert!(direct_export.has_token_bound());
        assert_eq!(direct_export.direction_label, "exported");
        assert_eq!(direct_export.contract_signal_component_count(), 3);
        assert_eq!(direct_export.contract_problem_component_count(), 0);
        assert!(direct_export.contract_accounting_is_consistent());
        assert_eq!(
            direct_export.runtime_kv_block_contract_commit_signal_component_count(),
            3
        );
        assert!(direct_export.has_runtime_kv_block_contract_commit_signals());
        assert_eq!(
            direct_export.runtime_kv_block_contract_commit_blocker_component_count(),
            0
        );
        assert!(direct_export.runtime_kv_block_contract_commit_accounting_is_consistent());
        assert!(direct_export.runtime_kv_block_contract_commit_is_clean());
        assert!(direct_export.contract_shape_is_clean());
        assert!(direct_export.can_commit_runtime_kv_block_contract());
        assert!(direct_export.can_use_runtime_kv_block_contract());
    }

    #[test]
    fn request_export_contract_rejects_runtime_exports_when_disabled() {
        let runtime = RuntimeMetadata::new("model", "tok", 128, 4).with_kv_exchange(true, false);
        let request = InferenceRequest::new("prompt", TaskProfile::General)
            .with_prompt_tokens(8)
            .with_max_tokens(8)
            .with_runtime(runtime);
        let architecture = TransformerRuntimeArchitecture::new(1, 4, 1, 1, 64);
        let transformer_plan = TransformerPlanDigest::new(
            Some("kv-disabled"),
            vec![TransformerLayerBudget::new(
                0,
                TransformerAttentionKind::Global,
                0.5,
                64,
            )],
        );
        let execution =
            crate::adapter::AdapterExecutionContext::new([RuntimeAdapter::PortableRust]);
        let envelope = RuntimeRequestEnvelope::from_parts(
            &request,
            architecture,
            RouteBudget::default(),
            HierarchyWeights::default(),
            &transformer_plan,
            &execution,
            0,
        );
        let block = KvBlock::new(1, KvNamespace::Runtime, 0, 0, 0..1, vec![0.1], vec![0.2]);
        let contract = RuntimeKvBlockContract::for_request_exports(&envelope);
        let contract_summary = contract.contract_summary();

        assert_eq!(contract_summary.max_blocks, 0);
        assert!(contract_summary.has_token_bound());
        assert!(!contract_summary.has_block_capacity());
        assert_eq!(contract_summary.contract_problem_component_count(), 0);
        assert!(contract_summary.contract_accounting_is_consistent());
        assert_eq!(
            contract_summary.runtime_kv_block_contract_commit_signal_component_count(),
            2
        );
        assert!(contract_summary.has_runtime_kv_block_contract_commit_signals());
        assert_eq!(
            contract_summary.runtime_kv_block_contract_commit_blocker_component_count(),
            0
        );
        assert!(!contract_summary.has_runtime_kv_block_contract_commit_blockers());
        assert!(contract_summary.runtime_kv_block_contract_commit_accounting_is_consistent());
        assert!(contract_summary.runtime_kv_block_contract_commit_is_clean());
        assert!(contract_summary.contract_shape_is_clean());
        assert!(!contract_summary.can_commit_runtime_kv_block_contract());
        assert!(!contract_summary.can_use_runtime_kv_block_contract());

        let report = contract.validate_blocks(&[block], &envelope.runtime, envelope.architecture);
        let summary = report.validation_summary();
        let boundary = contract.validation_boundary_summary(&report);

        assert!(report.accepted.is_empty());
        assert!(!summary.valid);
        assert_eq!(summary.accepted_count, 0);
        assert_eq!(summary.violation_count, 1);
        assert!(summary.valid_flag_matches_violations());
        assert_eq!(summary.validation_signal_component_count(), 1);
        assert!(summary.has_validation_signals());
        assert_eq!(summary.validation_problem_component_count(), 1);
        assert!(summary.has_validation_problem_components());
        assert!(summary.validation_accounting_is_consistent());
        assert_eq!(
            summary.runtime_kv_validation_commit_signal_component_count(),
            1
        );
        assert!(summary.has_runtime_kv_validation_commit_signals());
        assert_eq!(
            summary.runtime_kv_validation_commit_blocker_component_count(),
            1
        );
        assert!(summary.has_runtime_kv_validation_commit_blockers());
        assert!(summary.runtime_kv_validation_commit_accounting_is_consistent());
        assert!(!summary.runtime_kv_validation_commit_is_clean());
        assert!(!summary.validation_commit_is_clean());
        assert!(!summary.validation_shape_is_clean());
        assert!(!summary.can_commit_runtime_kv_validation());
        assert_eq!(boundary.direction, RuntimeKvDirection::Exported);
        assert_eq!(boundary.failure_trace_label, "runtime_kv_export_error");
        assert!(!boundary.has_block_capacity());
        assert!(boundary.has_token_bound());
        assert!(boundary.rejected_all());
        assert!(boundary.maps_to_runtime_kv_failure());
        assert_eq!(boundary.boundary_signal_component_count(), 3);
        assert_eq!(boundary.boundary_problem_component_count(), 1);
        assert!(boundary.boundary_accounting_is_consistent());
        assert_eq!(
            boundary.runtime_kv_boundary_commit_signal_component_count(),
            3
        );
        assert!(boundary.has_runtime_kv_boundary_commit_signals());
        assert_eq!(
            boundary.runtime_kv_boundary_commit_blocker_component_count(),
            1
        );
        assert!(boundary.has_runtime_kv_boundary_commit_blockers());
        assert!(boundary.runtime_kv_boundary_commit_accounting_is_consistent());
        assert!(!boundary.runtime_kv_boundary_commit_is_clean());
        assert!(!boundary.boundary_shape_is_clean());
        assert!(!boundary.can_commit_runtime_kv_boundary());
        assert!(
            report
                .violations
                .join("\n")
                .contains("exported KV block count 1 exceeds contract max_blocks 0")
        );
    }

    #[test]
    fn runtime_kv_validation_summary_reports_valid_flag_drift() {
        let clean = RuntimeKvValidationSummary {
            accepted_count: 1,
            violation_count: 0,
            valid: true,
        };
        let summary = RuntimeKvValidationSummary {
            accepted_count: 1,
            violation_count: 1,
            valid: true,
        };
        let boundary = RuntimeKvValidationBoundarySummary {
            direction: RuntimeKvDirection::Imported,
            direction_label: "exported",
            failure_trace_label: "runtime_kv_export_error",
            max_blocks: 0,
            token_upper_bound: 0,
            accepted_count: 1,
            violation_count: 1,
            valid: true,
        };
        let clean_boundary = RuntimeKvValidationBoundarySummary {
            direction: RuntimeKvDirection::Imported,
            direction_label: "imported",
            failure_trace_label: "runtime_kv_import_error",
            max_blocks: 2,
            token_upper_bound: 8,
            accepted_count: 1,
            violation_count: 0,
            valid: true,
        };

        assert!(clean.accepted_any());
        assert!(!clean.has_violations());
        assert!(clean.valid_flag_matches_violations());
        assert_eq!(clean.validation_signal_component_count(), 1);
        assert!(clean.has_validation_signals());
        assert_eq!(clean.validation_problem_component_count(), 0);
        assert!(!clean.has_validation_problem_components());
        assert!(clean.validation_accounting_is_consistent());
        assert_eq!(
            clean.runtime_kv_validation_commit_signal_component_count(),
            1
        );
        assert!(clean.has_runtime_kv_validation_commit_signals());
        assert_eq!(
            clean.runtime_kv_validation_commit_blocker_component_count(),
            0
        );
        assert!(!clean.has_runtime_kv_validation_commit_blockers());
        assert!(clean.runtime_kv_validation_commit_accounting_is_consistent());
        assert!(clean.runtime_kv_validation_commit_is_clean());
        assert!(clean.validation_commit_is_clean());
        assert!(clean.validation_shape_is_clean());
        assert!(clean.can_commit_runtime_kv_validation());

        assert!(summary.has_violations());
        assert!(summary.accepted_any());
        assert!(!summary.rejected_all());
        assert!(summary.partially_accepted());
        assert!(!summary.valid_flag_matches_violations());
        assert_eq!(summary.accepted_signal_component_count(), 1);
        assert_eq!(summary.partial_acceptance_signal_component_count(), 1);
        assert_eq!(summary.rejected_all_signal_component_count(), 0);
        assert_eq!(summary.validation_signal_component_count(), 2);
        assert!(summary.has_validation_signals());
        assert_eq!(summary.violation_problem_component_count(), 1);
        assert_eq!(summary.valid_flag_drift_component_count(), 1);
        assert_eq!(summary.validation_problem_component_count(), 2);
        assert!(summary.has_validation_problem_components());
        assert!(summary.validation_accounting_is_consistent());
        assert_eq!(
            summary.runtime_kv_validation_commit_signal_component_count(),
            2
        );
        assert!(summary.has_runtime_kv_validation_commit_signals());
        assert_eq!(
            summary.runtime_kv_validation_commit_blocker_component_count(),
            2
        );
        assert!(summary.has_runtime_kv_validation_commit_blockers());
        assert!(summary.runtime_kv_validation_commit_accounting_is_consistent());
        assert!(!summary.runtime_kv_validation_commit_is_clean());
        assert!(!summary.validation_commit_is_clean());
        assert!(!summary.validation_shape_is_clean());
        assert!(!summary.can_commit_runtime_kv_validation());

        assert!(clean_boundary.direction_label_matches_kind());
        assert!(clean_boundary.failure_trace_label_matches_direction());
        assert!(clean_boundary.has_block_capacity());
        assert!(clean_boundary.has_token_bound());
        assert!(clean_boundary.accepted_any());
        assert!(!clean_boundary.has_violations());
        assert!(!clean_boundary.rejected_all());
        assert!(!clean_boundary.partially_accepted());
        assert!(clean_boundary.accepted_within_contract_limit());
        assert!(clean_boundary.valid_flag_matches_violations());
        assert!(!clean_boundary.maps_to_runtime_kv_failure());
        assert_eq!(clean_boundary.boundary_signal_component_count(), 3);
        assert!(clean_boundary.has_boundary_signals());
        assert_eq!(clean_boundary.boundary_problem_component_count(), 0);
        assert!(!clean_boundary.has_boundary_problem_components());
        assert!(clean_boundary.boundary_accounting_is_consistent());
        assert_eq!(
            clean_boundary.runtime_kv_boundary_commit_signal_component_count(),
            3
        );
        assert!(clean_boundary.has_runtime_kv_boundary_commit_signals());
        assert_eq!(
            clean_boundary.runtime_kv_boundary_commit_blocker_component_count(),
            0
        );
        assert!(!clean_boundary.has_runtime_kv_boundary_commit_blockers());
        assert!(clean_boundary.runtime_kv_boundary_commit_accounting_is_consistent());
        assert!(clean_boundary.runtime_kv_boundary_commit_is_clean());
        assert!(clean_boundary.boundary_shape_is_clean());
        assert!(clean_boundary.can_commit_runtime_kv_boundary());

        assert!(!boundary.direction_label_matches_kind());
        assert!(!boundary.failure_trace_label_matches_direction());
        assert!(!boundary.has_block_capacity());
        assert!(!boundary.has_token_bound());
        assert!(boundary.accepted_any());
        assert!(boundary.has_violations());
        assert!(!boundary.rejected_all());
        assert!(boundary.partially_accepted());
        assert!(!boundary.accepted_within_contract_limit());
        assert!(!boundary.valid_flag_matches_violations());
        assert!(!boundary.maps_to_runtime_kv_failure());
        assert_eq!(boundary.boundary_signal_component_count(), 2);
        assert!(boundary.has_boundary_signals());
        assert_eq!(boundary.boundary_problem_component_count(), 5);
        assert!(boundary.has_boundary_problem_components());
        assert!(boundary.boundary_accounting_is_consistent());
        assert_eq!(
            boundary.runtime_kv_boundary_commit_signal_component_count(),
            2
        );
        assert!(boundary.has_runtime_kv_boundary_commit_signals());
        assert_eq!(
            boundary.runtime_kv_boundary_commit_blocker_component_count(),
            5
        );
        assert!(boundary.has_runtime_kv_boundary_commit_blockers());
        assert!(boundary.runtime_kv_boundary_commit_accounting_is_consistent());
        assert!(!boundary.runtime_kv_boundary_commit_is_clean());
        assert!(!boundary.boundary_shape_is_clean());
        assert!(!boundary.can_commit_runtime_kv_boundary());
    }

    #[test]
    fn runtime_kv_contract_summary_counts_public_shape_drift() {
        let clean_no_capacity = RuntimeKvBlockContractSummary {
            max_blocks: 0,
            token_upper_bound: 8,
            direction: RuntimeKvDirection::Exported,
            direction_label: "exported",
        };

        assert!(!clean_no_capacity.has_block_capacity());
        assert!(clean_no_capacity.has_token_bound());
        assert!(clean_no_capacity.direction_label_matches_kind());
        assert_eq!(clean_no_capacity.contract_signal_component_count(), 2);
        assert!(clean_no_capacity.has_contract_signals());
        assert_eq!(clean_no_capacity.contract_problem_component_count(), 0);
        assert!(!clean_no_capacity.has_contract_problem_components());
        assert!(clean_no_capacity.contract_accounting_is_consistent());
        assert_eq!(
            clean_no_capacity.runtime_kv_block_contract_commit_signal_component_count(),
            2
        );
        assert!(clean_no_capacity.has_runtime_kv_block_contract_commit_signals());
        assert_eq!(
            clean_no_capacity.runtime_kv_block_contract_commit_blocker_component_count(),
            0
        );
        assert!(!clean_no_capacity.has_runtime_kv_block_contract_commit_blockers());
        assert!(clean_no_capacity.runtime_kv_block_contract_commit_accounting_is_consistent());
        assert!(clean_no_capacity.runtime_kv_block_contract_commit_is_clean());
        assert!(clean_no_capacity.contract_shape_is_clean());
        assert!(!clean_no_capacity.can_commit_runtime_kv_block_contract());
        assert!(!clean_no_capacity.can_use_runtime_kv_block_contract());

        let drifted = RuntimeKvBlockContractSummary {
            max_blocks: 2,
            token_upper_bound: 0,
            direction: RuntimeKvDirection::Imported,
            direction_label: "exported",
        };

        assert!(drifted.has_block_capacity());
        assert!(!drifted.has_token_bound());
        assert!(!drifted.direction_label_matches_kind());
        assert_eq!(drifted.contract_signal_component_count(), 2);
        assert!(drifted.has_contract_signals());
        assert_eq!(drifted.contract_problem_component_count(), 2);
        assert!(drifted.has_contract_problem_components());
        assert!(drifted.contract_accounting_is_consistent());
        assert_eq!(
            drifted.runtime_kv_block_contract_commit_signal_component_count(),
            2
        );
        assert!(drifted.has_runtime_kv_block_contract_commit_signals());
        assert_eq!(
            drifted.runtime_kv_block_contract_commit_blocker_component_count(),
            2
        );
        assert!(drifted.has_runtime_kv_block_contract_commit_blockers());
        assert!(drifted.runtime_kv_block_contract_commit_accounting_is_consistent());
        assert!(!drifted.runtime_kv_block_contract_commit_is_clean());
        assert!(!drifted.contract_shape_is_clean());
        assert!(!drifted.can_commit_runtime_kv_block_contract());
        assert!(!drifted.can_use_runtime_kv_block_contract());
    }
}
