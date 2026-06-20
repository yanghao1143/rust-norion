use std::str::FromStr;

use crate::engine::{
    InferenceError, RuntimeFailureBatchSummary, RuntimeFailureReport, RuntimeFailureSummary,
};
use crate::profile::{HierarchyWeights, TaskProfile};
use crate::router::RoutingContext;
use crate::runtime::RuntimeMetadata;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimeAdapter {
    PortableRust,
    CpuSimd,
    Wgpu,
    WebGpu,
    Vulkan,
    Metal,
    Cuda,
    Rocm,
    OneApi,
    DirectMl,
    CoreMl,
    Nnapi,
    Qnn,
    OpenVino,
    Cann,
    Mlu,
    Rknn,
    MultiDevice,
    CustomAccelerator,
}

impl RuntimeAdapter {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PortableRust => "portable-rust",
            Self::CpuSimd => "cpu-simd",
            Self::Wgpu => "wgpu",
            Self::WebGpu => "webgpu",
            Self::Vulkan => "vulkan",
            Self::Metal => "metal",
            Self::Cuda => "cuda",
            Self::Rocm => "rocm",
            Self::OneApi => "oneapi",
            Self::DirectMl => "directml",
            Self::CoreMl => "coreml",
            Self::Nnapi => "nnapi",
            Self::Qnn => "qnn",
            Self::OpenVino => "openvino",
            Self::Cann => "cann",
            Self::Mlu => "mlu",
            Self::Rknn => "rknn",
            Self::MultiDevice => "multi-device",
            Self::CustomAccelerator => "custom-accelerator",
        }
    }
}

impl FromStr for RuntimeAdapter {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "portable-rust" | "portable_rust" | "rust" => Ok(Self::PortableRust),
            "cpu-simd" | "cpu_simd" | "simd" => Ok(Self::CpuSimd),
            "wgpu" => Ok(Self::Wgpu),
            "webgpu" | "web-gpu" => Ok(Self::WebGpu),
            "vulkan" => Ok(Self::Vulkan),
            "metal" => Ok(Self::Metal),
            "cuda" => Ok(Self::Cuda),
            "rocm" => Ok(Self::Rocm),
            "oneapi" | "one-api" => Ok(Self::OneApi),
            "directml" | "direct-ml" => Ok(Self::DirectMl),
            "coreml" | "core-ml" => Ok(Self::CoreMl),
            "nnapi" => Ok(Self::Nnapi),
            "qnn" => Ok(Self::Qnn),
            "openvino" | "open-vino" => Ok(Self::OpenVino),
            "cann" => Ok(Self::Cann),
            "mlu" => Ok(Self::Mlu),
            "rknn" => Ok(Self::Rknn),
            "multi-device" | "multidevice" => Ok(Self::MultiDevice),
            "custom-accelerator" | "custom" => Ok(Self::CustomAccelerator),
            other => Err(format!("unknown runtime adapter: {other}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdapterObservation {
    pub adapter: RuntimeAdapter,
    pub score: f32,
    pub reward: f32,
    pub quality: f32,
    pub forward_energy: Option<f32>,
    pub kv_influence: Option<f32>,
    pub experience_id: u64,
}

impl AdapterObservation {
    pub fn new(
        adapter: RuntimeAdapter,
        score: f32,
        reward: f32,
        quality: f32,
        forward_energy: Option<f32>,
        kv_influence: Option<f32>,
        experience_id: u64,
    ) -> Self {
        Self {
            adapter,
            score: score.clamp(0.0, 1.0),
            reward: reward.clamp(0.0, 1.0),
            quality: quality.clamp(0.0, 1.0),
            forward_energy: forward_energy.filter(|value| value.is_finite()),
            kv_influence: kv_influence.filter(|value| value.is_finite()),
            experience_id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdapterSelection {
    pub adapter: RuntimeAdapter,
    pub score: f32,
    pub experience_id: Option<u64>,
    pub used_fallback: bool,
}

impl AdapterSelection {
    pub fn fallback(adapter: RuntimeAdapter) -> Self {
        Self {
            adapter,
            score: 0.0,
            experience_id: None,
            used_fallback: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterFallbackReason {
    NoFallback,
    NoAllowedAdapter,
    NoMatchingObservation,
}

impl AdapterFallbackReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoFallback => "none",
            Self::NoAllowedAdapter => "no-allowed-adapter",
            Self::NoMatchingObservation => "no-matching-observation",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdapterSelectionReport {
    pub selection: AdapterSelection,
    pub allowed_adapter_count: usize,
    pub observation_count: usize,
    pub matching_observation_count: usize,
    pub fallback_reason: AdapterFallbackReason,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdapterSelectionRuntimeSummary {
    pub selection: AdapterSelection,
    pub fallback_reason: AdapterFallbackReason,
    pub allowed_adapter_count: usize,
    pub matching_observation_count: usize,
    pub runtime_selected_adapter: Option<RuntimeAdapter>,
    pub runtime_adapter_reported: bool,
    pub runtime_adapter_matches_selection: bool,
    pub runtime_adapter_allowed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdapterSelectionCommitSummary {
    pub report: AdapterSelectionReport,
    pub action: AdapterSelectionCommitAction,
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
pub enum AdapterSelectionCommitAction {
    CommitAdapterSelection,
    ReturnRuntimeFailure,
}

impl AdapterSelectionCommitAction {
    pub fn can_commit(self) -> bool {
        matches!(self, Self::CommitAdapterSelection)
    }

    pub fn should_return_failure(self) -> bool {
        matches!(self, Self::ReturnRuntimeFailure)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdapterSelectionRuntimeCommitSummary {
    pub runtime: AdapterSelectionRuntimeSummary,
    pub action: AdapterSelectionRuntimeCommitAction,
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
pub enum AdapterFailureReturnSource {
    AdapterSelection,
    RuntimeAdapterExecution,
    AdapterExecutionContext,
    RuntimeClamp,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdapterFailureReturnSummary {
    pub source: AdapterFailureReturnSource,
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
pub struct AdapterFailureReturnReport {
    pub source: AdapterFailureReturnSource,
    pub primary_failure: RuntimeFailureReport,
    pub primary_failure_summary: RuntimeFailureSummary,
    pub failure_batch: RuntimeFailureBatchSummary,
    pub failure_report_count: usize,
    pub can_format_runtime_failures: bool,
    pub total_blocker_component_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterSelectionRuntimeCommitAction {
    CommitRuntimeAdapterExecution,
    ReturnRuntimeFailure,
}

impl AdapterSelectionRuntimeCommitAction {
    pub fn can_commit(self) -> bool {
        matches!(self, Self::CommitRuntimeAdapterExecution)
    }

    pub fn should_return_failure(self) -> bool {
        matches!(self, Self::ReturnRuntimeFailure)
    }
}

impl AdapterFailureReturnSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::AdapterSelection => "adapter_selection",
            Self::RuntimeAdapterExecution => "runtime_adapter_execution",
            Self::AdapterExecutionContext => "adapter_execution_context",
            Self::RuntimeClamp => "runtime_clamp",
        }
    }
}

impl AdapterFailureReturnSummary {
    pub fn new(
        source: AdapterFailureReturnSource,
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

impl AdapterFailureReturnReport {
    pub fn new(
        source: AdapterFailureReturnSource,
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

    pub fn can_use_adapter_failure_return_report(&self) -> bool {
        self.failure_return_report_shape_is_clean()
    }
}

impl AdapterSelectionReport {
    pub fn used_fallback(self) -> bool {
        self.selection.used_fallback
    }

    pub fn selection_from_observation(self) -> bool {
        !self.used_fallback() && self.selection.experience_id.is_some()
    }

    pub fn rejected_observation_count(self) -> usize {
        self.observation_count
            .saturating_sub(self.matching_observation_count)
    }

    pub fn has_allowed_adapters(self) -> bool {
        self.allowed_adapter_count > 0
    }

    pub fn has_observations(self) -> bool {
        self.observation_count > 0
    }

    pub fn has_matching_observations(self) -> bool {
        self.matching_observation_count > 0
    }

    pub fn observations_all_rejected(self) -> bool {
        self.observation_count > 0 && self.matching_observation_count == 0
    }

    pub fn matching_observations_within_observation_count(self) -> bool {
        self.matching_observation_count <= self.observation_count
    }

    pub fn fallback_due_to_no_allowed_adapter(self) -> bool {
        self.used_fallback() && self.fallback_reason == AdapterFallbackReason::NoAllowedAdapter
    }

    pub fn fallback_due_to_no_matching_observation(self) -> bool {
        self.used_fallback() && self.fallback_reason == AdapterFallbackReason::NoMatchingObservation
    }

    pub fn fallback_reason_matches_selection(self) -> bool {
        if !self.used_fallback() {
            return self.fallback_reason == AdapterFallbackReason::NoFallback;
        }

        if !self.has_allowed_adapters() {
            return self.fallback_reason == AdapterFallbackReason::NoAllowedAdapter;
        }

        self.matching_observation_count == 0
            && self.fallback_reason == AdapterFallbackReason::NoMatchingObservation
    }

    pub fn matched_observation_fraction(self) -> f32 {
        if self.observation_count == 0 {
            0.0
        } else {
            self.matching_observation_count as f32 / self.observation_count as f32
        }
    }

    pub fn adapter_catalog_signal_component_count(self) -> usize {
        usize::from(self.has_allowed_adapters())
    }

    pub fn observation_signal_component_count(self) -> usize {
        usize::from(self.has_observations())
            + usize::from(self.has_matching_observations())
            + usize::from(self.selection_from_observation())
    }

    pub fn fallback_signal_component_count(self) -> usize {
        usize::from(self.used_fallback())
            + usize::from(self.observations_all_rejected())
            + usize::from(self.fallback_due_to_no_allowed_adapter())
            + usize::from(self.fallback_due_to_no_matching_observation())
    }

    pub fn selection_report_signal_component_count(self) -> usize {
        self.adapter_catalog_signal_component_count()
            .saturating_add(self.observation_signal_component_count())
            .saturating_add(self.fallback_signal_component_count())
    }

    pub fn has_selection_report_signals(self) -> bool {
        self.selection_report_signal_component_count() > 0
    }

    pub fn observation_shape_problem_component_count(self) -> usize {
        usize::from(!self.matching_observations_within_observation_count())
    }

    pub fn fallback_reason_problem_component_count(self) -> usize {
        usize::from(!self.fallback_reason_matches_selection())
    }

    pub fn selection_report_problem_component_count(self) -> usize {
        self.observation_shape_problem_component_count()
            .saturating_add(self.fallback_reason_problem_component_count())
    }

    pub fn has_selection_report_problem_components(self) -> bool {
        self.selection_report_problem_component_count() > 0
    }

    pub fn selection_report_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .adapter_catalog_signal_component_count()
            .saturating_add(self.observation_signal_component_count())
            .saturating_add(self.fallback_signal_component_count());
        let expected_problem_count = self
            .observation_shape_problem_component_count()
            .saturating_add(self.fallback_reason_problem_component_count());

        self.selection_report_signal_component_count() == expected_signal_count
            && self.has_selection_report_signals() == (expected_signal_count > 0)
            && self.selection_report_problem_component_count() == expected_problem_count
            && self.has_selection_report_problem_components() == (expected_problem_count > 0)
    }

    pub fn selection_report_shape_is_clean(self) -> bool {
        !self.has_selection_report_problem_components()
            && self.selection_report_accounting_is_consistent()
    }

    pub fn adapter_selection_commit_signal_component_count(self) -> usize {
        self.selection_report_signal_component_count()
    }

    pub fn adapter_selection_commit_blocker_component_count(self) -> usize {
        self.selection_report_problem_component_count()
            .saturating_add(usize::from(!self.has_allowed_adapters()))
    }

    pub fn has_adapter_selection_commit_blockers(self) -> bool {
        self.adapter_selection_commit_blocker_component_count() > 0
    }

    pub fn component_accounting_drift_count(self) -> usize {
        usize::from(!self.adapter_selection_commit_accounting_is_consistent())
    }

    pub fn adapter_selection_commit_problem_component_count(self) -> usize {
        self.adapter_selection_commit_blocker_component_count()
            .saturating_add(self.component_accounting_drift_count())
    }

    pub fn has_adapter_selection_commit_problem_components(self) -> bool {
        self.adapter_selection_commit_problem_component_count() > 0
    }

    pub fn adapter_selection_commit_accounting_is_consistent(self) -> bool {
        let expected_blocker_count = self
            .selection_report_problem_component_count()
            .saturating_add(usize::from(!self.has_allowed_adapters()));

        self.selection_report_accounting_is_consistent()
            && self.adapter_selection_commit_signal_component_count()
                == self.selection_report_signal_component_count()
            && self.adapter_selection_commit_blocker_component_count() == expected_blocker_count
            && self.has_adapter_selection_commit_blockers()
                == (self.adapter_selection_commit_blocker_component_count() > 0)
    }

    pub fn adapter_selection_commit_is_clean(self) -> bool {
        self.adapter_selection_commit_blocker_component_count() == 0
            && self.adapter_selection_commit_accounting_is_consistent()
    }

    pub fn can_commit_adapter_selection(self) -> bool {
        self.adapter_selection_commit_is_clean()
    }

    pub fn adapter_selection_commit_action(self) -> AdapterSelectionCommitAction {
        if self.can_commit_adapter_selection() {
            AdapterSelectionCommitAction::CommitAdapterSelection
        } else {
            AdapterSelectionCommitAction::ReturnRuntimeFailure
        }
    }

    pub fn can_use_adapter_selection(self) -> bool {
        self.can_commit_adapter_selection()
    }

    pub fn failure_report(self) -> Option<RuntimeFailureReport> {
        let component_count = self.adapter_selection_commit_problem_component_count();
        if component_count == 0 {
            None
        } else {
            Some(RuntimeFailureReport::contract_violation(format!(
                "adapter selection failed: components={component_count}"
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

    pub fn commit_summary(self) -> AdapterSelectionCommitSummary {
        AdapterSelectionCommitSummary::new(self)
    }
}

impl AdapterSelectionCommitSummary {
    pub fn new(report: AdapterSelectionReport) -> Self {
        let failure_reports = report.failure_reports();
        let primary_failure_report = failure_reports.first().cloned();
        let primary_failure_summary = primary_failure_report
            .as_ref()
            .map(|failure| failure.failure_summary());
        let failure_batch = RuntimeFailureReport::batch_summary(&failure_reports);
        let can_commit = report.can_commit_adapter_selection();
        let failure_report_count = failure_reports.len();
        let can_format_runtime_failures = failure_batch.can_format_runtime_failures();
        let should_return_failure = !can_commit && failure_report_count > 0;
        let action = report.adapter_selection_commit_action();

        Self {
            report,
            action,
            can_commit,
            should_return_failure,
            failure_reports,
            primary_failure_report,
            primary_failure_summary,
            failure_batch,
            failure_report_count,
            can_format_runtime_failures,
            total_signal_component_count: report.adapter_selection_commit_signal_component_count(),
            total_blocker_component_count: report
                .adapter_selection_commit_blocker_component_count(),
            component_accounting_consistent: report
                .adapter_selection_commit_accounting_is_consistent(),
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

    pub fn failure_return_summary(&self) -> AdapterFailureReturnSummary {
        AdapterFailureReturnSummary::new(
            AdapterFailureReturnSource::AdapterSelection,
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

    pub fn runtime_failure_return_report(&self) -> Option<AdapterFailureReturnReport> {
        if self.failure_return_summary().can_return_runtime_failure() {
            self.primary_failure_report.clone().map(|failure| {
                AdapterFailureReturnReport::new(
                    AdapterFailureReturnSource::AdapterSelection,
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
        self.can_commit == self.report.can_commit_adapter_selection()
            && self.should_return_failure == (!self.can_commit && self.failure_report_count > 0)
            && self.action == self.report.adapter_selection_commit_action()
            && self.action_can_commit() == self.can_commit
            && self.action_should_return_failure() == self.should_return_failure
            && self.failure_report_count == self.failure_reports.len()
            && self.failure_report_count == self.report.failure_report_count()
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
                    .report
                    .adapter_selection_commit_signal_component_count()
            && self.total_blocker_component_count
                == self
                    .report
                    .adapter_selection_commit_blocker_component_count()
            && self.component_accounting_consistent
                == self
                    .report
                    .adapter_selection_commit_accounting_is_consistent()
    }

    pub fn can_commit_adapter_selection(&self) -> bool {
        self.can_commit && self.commit_decision_accounting_is_consistent()
    }

    pub fn should_return_runtime_failure(&self) -> bool {
        self.should_return_failure && self.commit_decision_accounting_is_consistent()
    }
}

impl AdapterSelectionRuntimeSummary {
    pub fn has_allowed_adapters(self) -> bool {
        self.allowed_adapter_count > 0
    }

    pub fn has_matching_observations(self) -> bool {
        self.matching_observation_count > 0
    }

    pub fn runtime_adapter_missing(self) -> bool {
        !self.runtime_adapter_reported
    }

    pub fn runtime_selection_confirmed(self) -> bool {
        self.runtime_adapter_reported
            && self.runtime_adapter_matches_selection
            && self.runtime_adapter_allowed
    }

    pub fn runtime_selection_drifted(self) -> bool {
        self.runtime_adapter_reported && !self.runtime_adapter_matches_selection
    }

    pub fn runtime_adapter_outside_execution_context(self) -> bool {
        self.runtime_adapter_reported && !self.runtime_adapter_allowed
    }

    pub fn fallback_has_reason(self) -> bool {
        if self.selection.used_fallback {
            self.fallback_reason != AdapterFallbackReason::NoFallback
        } else {
            self.fallback_reason == AdapterFallbackReason::NoFallback
        }
    }

    pub fn fallback_without_reason(self) -> bool {
        self.selection.used_fallback && self.fallback_reason == AdapterFallbackReason::NoFallback
    }

    pub fn runtime_adapter_problem(self) -> bool {
        self.runtime_adapter_missing()
            || self.runtime_selection_drifted()
            || self.runtime_adapter_outside_execution_context()
            || !self.fallback_has_reason()
    }

    pub fn adapter_source_signal_component_count(self) -> usize {
        usize::from(self.has_allowed_adapters()) + usize::from(self.has_matching_observations())
    }

    pub fn runtime_report_signal_component_count(self) -> usize {
        usize::from(self.runtime_adapter_reported) + usize::from(self.runtime_selection_confirmed())
    }

    pub fn fallback_reason_signal_component_count(self) -> usize {
        usize::from(self.fallback_has_reason())
    }

    pub fn runtime_adapter_signal_component_count(self) -> usize {
        self.adapter_source_signal_component_count()
            .saturating_add(self.runtime_report_signal_component_count())
            .saturating_add(self.fallback_reason_signal_component_count())
    }

    pub fn has_runtime_adapter_signals(self) -> bool {
        self.runtime_adapter_signal_component_count() > 0
    }

    pub fn fallback_reason_problem_component_count(self) -> usize {
        usize::from(!self.fallback_has_reason())
    }

    pub fn runtime_adapter_problem_component_count(self) -> usize {
        usize::from(self.runtime_adapter_missing())
            + usize::from(self.runtime_selection_drifted())
            + usize::from(self.runtime_adapter_outside_execution_context())
            + self.fallback_reason_problem_component_count()
    }

    pub fn has_runtime_adapter_problem_components(self) -> bool {
        self.runtime_adapter_problem_component_count() > 0
    }

    pub fn runtime_adapter_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .adapter_source_signal_component_count()
            .saturating_add(self.runtime_report_signal_component_count())
            .saturating_add(self.fallback_reason_signal_component_count());
        let expected_problem_count = usize::from(self.runtime_adapter_missing())
            .saturating_add(usize::from(self.runtime_selection_drifted()))
            .saturating_add(usize::from(
                self.runtime_adapter_outside_execution_context(),
            ))
            .saturating_add(self.fallback_reason_problem_component_count());

        self.runtime_adapter_signal_component_count() == expected_signal_count
            && self.has_runtime_adapter_signals() == (expected_signal_count > 0)
            && self.runtime_adapter_problem_component_count() == expected_problem_count
            && self.has_runtime_adapter_problem_components() == (expected_problem_count > 0)
            && self.runtime_adapter_problem() == (expected_problem_count > 0)
    }

    pub fn runtime_adapter_shape_is_clean(self) -> bool {
        !self.has_runtime_adapter_problem_components()
            && self.runtime_adapter_accounting_is_consistent()
    }

    pub fn runtime_adapter_execution_commit_signal_component_count(self) -> usize {
        self.runtime_adapter_signal_component_count()
    }

    pub fn runtime_adapter_execution_commit_blocker_component_count(self) -> usize {
        self.runtime_adapter_problem_component_count()
    }

    pub fn has_runtime_adapter_execution_commit_blockers(self) -> bool {
        self.runtime_adapter_execution_commit_blocker_component_count() > 0
    }

    pub fn component_accounting_drift_count(self) -> usize {
        usize::from(!self.runtime_adapter_execution_commit_accounting_is_consistent())
    }

    pub fn runtime_adapter_execution_commit_problem_component_count(self) -> usize {
        self.runtime_adapter_execution_commit_blocker_component_count()
            .saturating_add(self.component_accounting_drift_count())
    }

    pub fn has_runtime_adapter_execution_commit_problem_components(self) -> bool {
        self.runtime_adapter_execution_commit_problem_component_count() > 0
    }

    pub fn runtime_adapter_execution_commit_accounting_is_consistent(self) -> bool {
        self.runtime_adapter_accounting_is_consistent()
            && self.runtime_adapter_execution_commit_signal_component_count()
                == self.runtime_adapter_signal_component_count()
            && self.runtime_adapter_execution_commit_blocker_component_count()
                == self.runtime_adapter_problem_component_count()
            && self.has_runtime_adapter_execution_commit_blockers()
                == (self.runtime_adapter_execution_commit_blocker_component_count() > 0)
    }

    pub fn runtime_adapter_execution_commit_is_clean(self) -> bool {
        self.runtime_adapter_execution_commit_blocker_component_count() == 0
            && self.runtime_adapter_execution_commit_accounting_is_consistent()
    }

    pub fn can_commit_runtime_adapter_execution(self) -> bool {
        self.runtime_selection_confirmed() && self.runtime_adapter_execution_commit_is_clean()
    }

    pub fn runtime_adapter_execution_commit_action(self) -> AdapterSelectionRuntimeCommitAction {
        if self.can_commit_runtime_adapter_execution() {
            AdapterSelectionRuntimeCommitAction::CommitRuntimeAdapterExecution
        } else {
            AdapterSelectionRuntimeCommitAction::ReturnRuntimeFailure
        }
    }

    pub fn can_use_runtime_adapter_execution(self) -> bool {
        self.can_commit_runtime_adapter_execution()
    }

    pub fn is_clean_runtime_adapter_execution(self) -> bool {
        self.can_commit_runtime_adapter_execution()
    }

    pub fn failure_report(self) -> Option<RuntimeFailureReport> {
        let component_count = self.runtime_adapter_execution_commit_problem_component_count();
        if component_count == 0 {
            None
        } else {
            Some(RuntimeFailureReport::contract_violation(format!(
                "adapter runtime selection failed: components={component_count}"
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

    pub fn commit_summary(self) -> AdapterSelectionRuntimeCommitSummary {
        AdapterSelectionRuntimeCommitSummary::new(self)
    }
}

impl AdapterSelectionRuntimeCommitSummary {
    pub fn new(runtime: AdapterSelectionRuntimeSummary) -> Self {
        let failure_reports = runtime.failure_reports();
        let primary_failure_report = failure_reports.first().cloned();
        let primary_failure_summary = primary_failure_report
            .as_ref()
            .map(|failure| failure.failure_summary());
        let failure_batch = RuntimeFailureReport::batch_summary(&failure_reports);
        let can_commit = runtime.can_commit_runtime_adapter_execution();
        let failure_report_count = failure_reports.len();
        let can_format_runtime_failures = failure_batch.can_format_runtime_failures();
        let should_return_failure = !can_commit && failure_report_count > 0;
        let action = runtime.runtime_adapter_execution_commit_action();

        Self {
            runtime,
            action,
            can_commit,
            should_return_failure,
            failure_reports,
            primary_failure_report,
            primary_failure_summary,
            failure_batch,
            failure_report_count,
            can_format_runtime_failures,
            total_signal_component_count: runtime
                .runtime_adapter_execution_commit_signal_component_count(),
            total_blocker_component_count: runtime
                .runtime_adapter_execution_commit_blocker_component_count(),
            component_accounting_consistent: runtime
                .runtime_adapter_execution_commit_accounting_is_consistent(),
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

    pub fn failure_return_summary(&self) -> AdapterFailureReturnSummary {
        AdapterFailureReturnSummary::new(
            AdapterFailureReturnSource::RuntimeAdapterExecution,
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

    pub fn runtime_failure_return_report(&self) -> Option<AdapterFailureReturnReport> {
        if self.failure_return_summary().can_return_runtime_failure() {
            self.primary_failure_report.clone().map(|failure| {
                AdapterFailureReturnReport::new(
                    AdapterFailureReturnSource::RuntimeAdapterExecution,
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
        self.can_commit == self.runtime.can_commit_runtime_adapter_execution()
            && self.should_return_failure == (!self.can_commit && self.failure_report_count > 0)
            && self.action == self.runtime.runtime_adapter_execution_commit_action()
            && self.action_can_commit() == self.can_commit
            && self.action_should_return_failure() == self.should_return_failure
            && self.failure_report_count == self.failure_reports.len()
            && self.failure_report_count == self.runtime.failure_report_count()
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
                    .runtime
                    .runtime_adapter_execution_commit_signal_component_count()
            && self.total_blocker_component_count
                == self
                    .runtime
                    .runtime_adapter_execution_commit_blocker_component_count()
            && self.component_accounting_consistent
                == self
                    .runtime
                    .runtime_adapter_execution_commit_accounting_is_consistent()
    }

    pub fn can_commit_runtime_adapter_execution(&self) -> bool {
        self.can_commit && self.commit_decision_accounting_is_consistent()
    }

    pub fn should_return_runtime_failure(&self) -> bool {
        self.should_return_failure && self.commit_decision_accounting_is_consistent()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdapterExecutionContext {
    pub adapters: Vec<RuntimeAdapter>,
    pub hardware_pressure: f32,
    pub compute_headroom: f32,
    pub latency_budget_ms: Option<u64>,
    pub max_parallel_chunks: usize,
    pub kv_prefetch_blocks: usize,
    pub hot_kv_precision_bits: u8,
    pub cold_kv_precision_bits: u8,
    pub local_kv_token_budget: usize,
    pub global_kv_token_budget: usize,
    pub allow_disk_spill: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdapterExecutionContextSummary {
    pub adapter_count: usize,
    pub hardware_pressure: f32,
    pub compute_headroom: f32,
    pub latency_budget_ms: Option<u64>,
    pub max_parallel_chunks: usize,
    pub kv_prefetch_blocks: usize,
    pub hot_kv_precision_bits: u8,
    pub cold_kv_precision_bits: u8,
    pub local_kv_token_budget: usize,
    pub global_kv_token_budget: usize,
    pub allow_disk_spill: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdapterExecutionContextCommitSummary {
    pub context: AdapterExecutionContextSummary,
    pub action: AdapterExecutionContextCommitAction,
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
pub enum AdapterExecutionContextCommitAction {
    CommitAdapterExecutionContext,
    ReturnRuntimeFailure,
}

impl AdapterExecutionContextCommitAction {
    pub fn can_commit(self) -> bool {
        matches!(self, Self::CommitAdapterExecutionContext)
    }

    pub fn should_return_failure(self) -> bool {
        matches!(self, Self::ReturnRuntimeFailure)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdapterRuntimeClampSummary {
    pub before: AdapterExecutionContextSummary,
    pub after: AdapterExecutionContextSummary,
    pub runtime: crate::runtime::RuntimeMetadataShapeSummary,
    pub kv_prefetch_reduction: usize,
    pub hot_kv_precision_reduced: bool,
    pub cold_kv_precision_reduced: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdapterRuntimeClampCommitSummary {
    pub clamp: AdapterRuntimeClampSummary,
    pub action: AdapterRuntimeClampCommitAction,
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
pub enum AdapterRuntimeClampCommitAction {
    CommitRuntimeClamp,
    ReturnRuntimeFailure,
}

impl AdapterRuntimeClampCommitAction {
    pub fn can_commit(self) -> bool {
        matches!(self, Self::CommitRuntimeClamp)
    }

    pub fn should_return_failure(self) -> bool {
        matches!(self, Self::ReturnRuntimeFailure)
    }
}

impl AdapterExecutionContextSummary {
    pub fn has_adapters(self) -> bool {
        self.adapter_count > 0
    }

    pub fn pressure_limited(self) -> bool {
        self.hardware_pressure >= 0.72
    }

    pub fn has_latency_budget(self) -> bool {
        self.latency_budget_ms.is_some()
    }

    pub fn has_parallel_capacity(self) -> bool {
        self.max_parallel_chunks > 0
    }

    pub fn has_kv_prefetch(self) -> bool {
        self.kv_prefetch_blocks > 0
    }

    pub fn has_kv_token_budget(self) -> bool {
        self.total_kv_token_budget() > 0
    }

    pub fn uses_compressed_hot_kv(self) -> bool {
        self.hot_kv_precision_bits <= 4
    }

    pub fn has_valid_hot_kv_precision(self) -> bool {
        valid_kv_bits(self.hot_kv_precision_bits)
    }

    pub fn has_valid_cold_kv_precision(self) -> bool {
        valid_kv_bits(self.cold_kv_precision_bits)
    }

    pub fn cold_kv_not_wider_than_hot(self) -> bool {
        self.cold_kv_precision_bits <= self.hot_kv_precision_bits
    }

    pub fn total_kv_token_budget(self) -> usize {
        self.local_kv_token_budget
            .saturating_add(self.global_kv_token_budget)
    }

    pub fn pressure_values_are_bounded(self) -> bool {
        bounded_unit_float(self.hardware_pressure) && bounded_unit_float(self.compute_headroom)
    }

    pub fn adapter_candidate_signal_component_count(self) -> usize {
        usize::from(self.has_adapters())
    }

    pub fn pressure_signal_component_count(self) -> usize {
        usize::from(self.pressure_limited()) + usize::from(self.compute_headroom < 0.5)
    }

    pub fn execution_capacity_signal_component_count(self) -> usize {
        usize::from(self.has_latency_budget())
            + usize::from(self.max_parallel_chunks > 1)
            + usize::from(self.allow_disk_spill)
    }

    pub fn kv_budget_signal_component_count(self) -> usize {
        usize::from(self.has_kv_prefetch()) + usize::from(self.has_kv_token_budget())
    }

    pub fn kv_precision_signal_component_count(self) -> usize {
        usize::from(self.has_valid_hot_kv_precision())
            + usize::from(self.has_valid_cold_kv_precision())
            + usize::from(
                self.has_valid_hot_kv_precision()
                    && self.has_valid_cold_kv_precision()
                    && self.uses_compressed_hot_kv(),
            )
    }

    pub fn adapter_context_signal_component_count(self) -> usize {
        self.adapter_candidate_signal_component_count()
            .saturating_add(self.pressure_signal_component_count())
            .saturating_add(self.execution_capacity_signal_component_count())
            .saturating_add(self.kv_budget_signal_component_count())
            .saturating_add(self.kv_precision_signal_component_count())
    }

    pub fn has_adapter_context_signals(self) -> bool {
        self.adapter_context_signal_component_count() > 0
    }

    pub fn adapter_candidate_problem_component_count(self) -> usize {
        usize::from(!self.has_adapters())
    }

    pub fn pressure_shape_problem_component_count(self) -> usize {
        usize::from(!bounded_unit_float(self.hardware_pressure))
            + usize::from(!bounded_unit_float(self.compute_headroom))
    }

    pub fn execution_shape_problem_component_count(self) -> usize {
        usize::from(!self.has_parallel_capacity())
    }

    pub fn kv_precision_problem_component_count(self) -> usize {
        usize::from(!self.has_valid_hot_kv_precision())
            + usize::from(!self.has_valid_cold_kv_precision())
            + usize::from(!self.cold_kv_not_wider_than_hot())
    }

    pub fn adapter_context_problem_component_count(self) -> usize {
        self.adapter_candidate_problem_component_count()
            .saturating_add(self.pressure_shape_problem_component_count())
            .saturating_add(self.execution_shape_problem_component_count())
            .saturating_add(self.kv_precision_problem_component_count())
    }

    pub fn has_adapter_context_problem_components(self) -> bool {
        self.adapter_context_problem_component_count() > 0
    }

    pub fn adapter_context_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .adapter_candidate_signal_component_count()
            .saturating_add(self.pressure_signal_component_count())
            .saturating_add(self.execution_capacity_signal_component_count())
            .saturating_add(self.kv_budget_signal_component_count())
            .saturating_add(self.kv_precision_signal_component_count());
        let expected_problem_count = self
            .adapter_candidate_problem_component_count()
            .saturating_add(self.pressure_shape_problem_component_count())
            .saturating_add(self.execution_shape_problem_component_count())
            .saturating_add(self.kv_precision_problem_component_count());

        self.adapter_context_signal_component_count() == expected_signal_count
            && self.has_adapter_context_signals() == (expected_signal_count > 0)
            && self.adapter_context_problem_component_count() == expected_problem_count
            && self.has_adapter_context_problem_components() == (expected_problem_count > 0)
    }

    pub fn adapter_context_shape_is_clean(self) -> bool {
        !self.has_adapter_context_problem_components()
            && self.adapter_context_accounting_is_consistent()
    }

    pub fn adapter_execution_context_commit_signal_component_count(self) -> usize {
        self.adapter_context_signal_component_count()
    }

    pub fn adapter_execution_context_commit_blocker_component_count(self) -> usize {
        self.adapter_context_problem_component_count()
    }

    pub fn has_adapter_execution_context_commit_blockers(self) -> bool {
        self.adapter_execution_context_commit_blocker_component_count() > 0
    }

    pub fn component_accounting_drift_count(self) -> usize {
        usize::from(!self.adapter_execution_context_commit_accounting_is_consistent())
    }

    pub fn adapter_execution_context_commit_problem_component_count(self) -> usize {
        self.adapter_execution_context_commit_blocker_component_count()
            .saturating_add(self.component_accounting_drift_count())
    }

    pub fn has_adapter_execution_context_commit_problem_components(self) -> bool {
        self.adapter_execution_context_commit_problem_component_count() > 0
    }

    pub fn adapter_execution_context_commit_accounting_is_consistent(self) -> bool {
        self.adapter_context_accounting_is_consistent()
            && self.adapter_execution_context_commit_signal_component_count()
                == self.adapter_context_signal_component_count()
            && self.adapter_execution_context_commit_blocker_component_count()
                == self.adapter_context_problem_component_count()
            && self.has_adapter_execution_context_commit_blockers()
                == (self.adapter_execution_context_commit_blocker_component_count() > 0)
    }

    pub fn adapter_execution_context_commit_is_clean(self) -> bool {
        self.adapter_execution_context_commit_blocker_component_count() == 0
            && self.adapter_execution_context_commit_accounting_is_consistent()
    }

    pub fn can_commit_adapter_execution_context(self) -> bool {
        self.adapter_execution_context_commit_is_clean()
    }

    pub fn adapter_execution_context_commit_action(self) -> AdapterExecutionContextCommitAction {
        if self.can_commit_adapter_execution_context() {
            AdapterExecutionContextCommitAction::CommitAdapterExecutionContext
        } else {
            AdapterExecutionContextCommitAction::ReturnRuntimeFailure
        }
    }

    pub fn can_use_adapter_execution_context(self) -> bool {
        self.can_commit_adapter_execution_context()
    }

    pub fn failure_report(self) -> Option<RuntimeFailureReport> {
        let component_count = self.adapter_execution_context_commit_problem_component_count();
        if component_count == 0 {
            None
        } else {
            Some(RuntimeFailureReport::contract_violation(format!(
                "adapter execution context failed: components={component_count}"
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

    pub fn commit_summary(self) -> AdapterExecutionContextCommitSummary {
        AdapterExecutionContextCommitSummary::new(self)
    }
}

impl AdapterExecutionContextCommitSummary {
    pub fn new(context: AdapterExecutionContextSummary) -> Self {
        let failure_reports = context.failure_reports();
        let primary_failure_report = failure_reports.first().cloned();
        let primary_failure_summary = primary_failure_report
            .as_ref()
            .map(|failure| failure.failure_summary());
        let failure_batch = RuntimeFailureReport::batch_summary(&failure_reports);
        let can_commit = context.can_commit_adapter_execution_context();
        let failure_report_count = failure_reports.len();
        let can_format_runtime_failures = failure_batch.can_format_runtime_failures();
        let should_return_failure = !can_commit && failure_report_count > 0;
        let action = context.adapter_execution_context_commit_action();

        Self {
            context,
            action,
            can_commit,
            should_return_failure,
            failure_reports,
            primary_failure_report,
            primary_failure_summary,
            failure_batch,
            failure_report_count,
            can_format_runtime_failures,
            total_signal_component_count: context
                .adapter_execution_context_commit_signal_component_count(),
            total_blocker_component_count: context
                .adapter_execution_context_commit_blocker_component_count(),
            component_accounting_consistent: context
                .adapter_execution_context_commit_accounting_is_consistent(),
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

    pub fn failure_return_summary(&self) -> AdapterFailureReturnSummary {
        AdapterFailureReturnSummary::new(
            AdapterFailureReturnSource::AdapterExecutionContext,
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

    pub fn runtime_failure_return_report(&self) -> Option<AdapterFailureReturnReport> {
        if self.failure_return_summary().can_return_runtime_failure() {
            self.primary_failure_report.clone().map(|failure| {
                AdapterFailureReturnReport::new(
                    AdapterFailureReturnSource::AdapterExecutionContext,
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
        self.can_commit == self.context.can_commit_adapter_execution_context()
            && self.should_return_failure == (!self.can_commit && self.failure_report_count > 0)
            && self.action == self.context.adapter_execution_context_commit_action()
            && self.action_can_commit() == self.can_commit
            && self.action_should_return_failure() == self.should_return_failure
            && self.failure_report_count == self.failure_reports.len()
            && self.failure_report_count == self.context.failure_report_count()
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
                    .context
                    .adapter_execution_context_commit_signal_component_count()
            && self.total_blocker_component_count
                == self
                    .context
                    .adapter_execution_context_commit_blocker_component_count()
            && self.component_accounting_consistent
                == self
                    .context
                    .adapter_execution_context_commit_accounting_is_consistent()
    }

    pub fn can_commit_adapter_execution_context(&self) -> bool {
        self.can_commit && self.commit_decision_accounting_is_consistent()
    }

    pub fn should_return_runtime_failure(&self) -> bool {
        self.should_return_failure && self.commit_decision_accounting_is_consistent()
    }
}

impl AdapterRuntimeClampSummary {
    pub fn kv_prefetch_was_clamped(self) -> bool {
        self.kv_prefetch_reduction > 0
    }

    pub fn precision_was_clamped(self) -> bool {
        self.hot_kv_precision_reduced || self.cold_kv_precision_reduced
    }

    pub fn adapter_count_preserved(self) -> bool {
        self.before.adapter_count == self.after.adapter_count
    }

    pub fn pressure_preserved(self) -> bool {
        float_close(self.before.hardware_pressure, self.after.hardware_pressure)
            && float_close(self.before.compute_headroom, self.after.compute_headroom)
    }

    pub fn execution_shape_preserved(self) -> bool {
        self.before.latency_budget_ms == self.after.latency_budget_ms
            && self.before.max_parallel_chunks == self.after.max_parallel_chunks
            && self.before.allow_disk_spill == self.after.allow_disk_spill
    }

    pub fn token_budgets_preserved(self) -> bool {
        self.before.local_kv_token_budget == self.after.local_kv_token_budget
            && self.before.global_kv_token_budget == self.after.global_kv_token_budget
    }

    pub fn runtime_limits_are_monotonic(self) -> bool {
        self.after.kv_prefetch_blocks <= self.before.kv_prefetch_blocks
            && self.after.hot_kv_precision_bits <= self.before.hot_kv_precision_bits
            && self.after.cold_kv_precision_bits <= self.before.cold_kv_precision_bits
    }

    pub fn only_runtime_limits_changed(self) -> bool {
        self.adapter_count_preserved()
            && self.pressure_preserved()
            && self.execution_shape_preserved()
            && self.token_budgets_preserved()
            && self.runtime_limits_are_monotonic()
    }

    pub fn kv_prefetch_clamp_signal_component_count(self) -> usize {
        usize::from(self.kv_prefetch_was_clamped())
    }

    pub fn precision_clamp_signal_component_count(self) -> usize {
        usize::from(self.hot_kv_precision_reduced) + usize::from(self.cold_kv_precision_reduced)
    }

    pub fn runtime_clamp_signal_component_count(self) -> usize {
        self.kv_prefetch_clamp_signal_component_count()
            .saturating_add(self.precision_clamp_signal_component_count())
    }

    pub fn has_runtime_clamp_signals(self) -> bool {
        self.runtime_clamp_signal_component_count() > 0
    }

    pub fn preservation_problem_component_count(self) -> usize {
        usize::from(!self.adapter_count_preserved())
            + usize::from(!self.pressure_preserved())
            + usize::from(!self.execution_shape_preserved())
            + usize::from(!self.token_budgets_preserved())
    }

    pub fn runtime_limit_problem_component_count(self) -> usize {
        usize::from(!self.runtime_limits_are_monotonic())
    }

    pub fn after_context_problem_component_count(self) -> usize {
        self.after.adapter_context_problem_component_count()
    }

    pub fn runtime_clamp_problem_component_count(self) -> usize {
        self.preservation_problem_component_count()
            .saturating_add(self.runtime_limit_problem_component_count())
            .saturating_add(self.after_context_problem_component_count())
    }

    pub fn has_runtime_clamp_problem_components(self) -> bool {
        self.runtime_clamp_problem_component_count() > 0
    }

    pub fn runtime_clamp_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .kv_prefetch_clamp_signal_component_count()
            .saturating_add(self.precision_clamp_signal_component_count());
        let expected_problem_count = self
            .preservation_problem_component_count()
            .saturating_add(self.runtime_limit_problem_component_count())
            .saturating_add(self.after_context_problem_component_count());

        self.runtime_clamp_signal_component_count() == expected_signal_count
            && self.has_runtime_clamp_signals() == (expected_signal_count > 0)
            && self.runtime_clamp_problem_component_count() == expected_problem_count
            && self.has_runtime_clamp_problem_components() == (expected_problem_count > 0)
            && self.only_runtime_limits_changed()
                == (self.preservation_problem_component_count() == 0
                    && self.runtime_limit_problem_component_count() == 0)
    }

    pub fn runtime_clamp_shape_is_clean(self) -> bool {
        !self.has_runtime_clamp_problem_components()
            && self.runtime_clamp_accounting_is_consistent()
    }

    pub fn runtime_clamp_commit_signal_component_count(self) -> usize {
        self.runtime_clamp_signal_component_count()
    }

    pub fn has_runtime_clamp_commit_signals(self) -> bool {
        self.runtime_clamp_commit_signal_component_count() > 0
    }

    pub fn runtime_clamp_commit_blocker_component_count(self) -> usize {
        self.runtime_clamp_problem_component_count()
    }

    pub fn component_accounting_drift_count(self) -> usize {
        usize::from(!self.runtime_clamp_commit_accounting_is_consistent())
    }

    pub fn runtime_clamp_commit_problem_component_count(self) -> usize {
        self.runtime_clamp_commit_blocker_component_count()
            .saturating_add(self.component_accounting_drift_count())
    }

    pub fn has_runtime_clamp_commit_problem_components(self) -> bool {
        self.runtime_clamp_commit_problem_component_count() > 0
    }

    pub fn has_runtime_clamp_commit_blockers(self) -> bool {
        self.runtime_clamp_commit_blocker_component_count() > 0
    }

    pub fn runtime_clamp_commit_accounting_is_consistent(self) -> bool {
        self.runtime_clamp_accounting_is_consistent()
            && self.runtime_clamp_commit_signal_component_count()
                == self.runtime_clamp_signal_component_count()
            && self.has_runtime_clamp_commit_signals()
                == (self.runtime_clamp_commit_signal_component_count() > 0)
            && self.runtime_clamp_commit_blocker_component_count()
                == self.runtime_clamp_problem_component_count()
            && self.has_runtime_clamp_commit_blockers()
                == (self.runtime_clamp_commit_blocker_component_count() > 0)
    }

    pub fn runtime_clamp_commit_is_clean(self) -> bool {
        !self.has_runtime_clamp_commit_blockers()
            && self.runtime_clamp_commit_accounting_is_consistent()
    }

    pub fn can_commit_runtime_clamp(self) -> bool {
        self.runtime_clamp_commit_is_clean()
    }

    pub fn runtime_clamp_commit_action(self) -> AdapterRuntimeClampCommitAction {
        if self.can_commit_runtime_clamp() {
            AdapterRuntimeClampCommitAction::CommitRuntimeClamp
        } else {
            AdapterRuntimeClampCommitAction::ReturnRuntimeFailure
        }
    }

    pub fn can_use_runtime_clamp(self) -> bool {
        self.runtime_clamp_shape_is_clean()
    }

    pub fn failure_report(self) -> Option<RuntimeFailureReport> {
        let component_count = self.runtime_clamp_commit_problem_component_count();
        if component_count == 0 {
            None
        } else {
            Some(RuntimeFailureReport::contract_violation(format!(
                "adapter runtime clamp failed: components={component_count}"
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

    pub fn commit_summary(self) -> AdapterRuntimeClampCommitSummary {
        AdapterRuntimeClampCommitSummary::new(self)
    }
}

impl AdapterRuntimeClampCommitSummary {
    pub fn new(clamp: AdapterRuntimeClampSummary) -> Self {
        let failure_reports = clamp.failure_reports();
        let primary_failure_report = failure_reports.first().cloned();
        let primary_failure_summary = primary_failure_report
            .as_ref()
            .map(|failure| failure.failure_summary());
        let failure_batch = RuntimeFailureReport::batch_summary(&failure_reports);
        let can_commit = clamp.can_commit_runtime_clamp();
        let failure_report_count = failure_reports.len();
        let can_format_runtime_failures = failure_batch.can_format_runtime_failures();
        let should_return_failure = !can_commit && failure_report_count > 0;
        let action = clamp.runtime_clamp_commit_action();

        Self {
            clamp,
            action,
            can_commit,
            should_return_failure,
            failure_reports,
            primary_failure_report,
            primary_failure_summary,
            failure_batch,
            failure_report_count,
            can_format_runtime_failures,
            total_signal_component_count: clamp.runtime_clamp_commit_signal_component_count(),
            total_blocker_component_count: clamp.runtime_clamp_commit_blocker_component_count(),
            component_accounting_consistent: clamp.runtime_clamp_commit_accounting_is_consistent(),
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

    pub fn failure_return_summary(&self) -> AdapterFailureReturnSummary {
        AdapterFailureReturnSummary::new(
            AdapterFailureReturnSource::RuntimeClamp,
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

    pub fn runtime_failure_return_report(&self) -> Option<AdapterFailureReturnReport> {
        if self.failure_return_summary().can_return_runtime_failure() {
            self.primary_failure_report.clone().map(|failure| {
                AdapterFailureReturnReport::new(
                    AdapterFailureReturnSource::RuntimeClamp,
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
        self.can_commit == self.clamp.can_commit_runtime_clamp()
            && self.should_return_failure == (!self.can_commit && self.failure_report_count > 0)
            && self.action == self.clamp.runtime_clamp_commit_action()
            && self.action_can_commit() == self.can_commit
            && self.action_should_return_failure() == self.should_return_failure
            && self.failure_report_count == self.failure_reports.len()
            && self.failure_report_count == self.clamp.failure_report_count()
            && self.primary_failure_report.as_ref() == self.failure_reports.first()
            && self.primary_failure_summary
                == self
                    .primary_failure_report
                    .as_ref()
                    .map(|failure| failure.failure_summary())
            && self.failure_batch == RuntimeFailureReport::batch_summary(&self.failure_reports)
            && self.can_format_runtime_failures == self.failure_batch.can_format_runtime_failures()
            && self.total_signal_component_count
                == self.clamp.runtime_clamp_commit_signal_component_count()
            && self.total_blocker_component_count
                == self.clamp.runtime_clamp_commit_blocker_component_count()
            && self.component_accounting_consistent
                == self.clamp.runtime_clamp_commit_accounting_is_consistent()
    }

    pub fn can_commit_runtime_clamp(&self) -> bool {
        self.can_commit && self.commit_decision_accounting_is_consistent()
    }

    pub fn should_return_runtime_failure(&self) -> bool {
        self.should_return_failure && self.commit_decision_accounting_is_consistent()
    }
}

impl AdapterExecutionContext {
    pub fn new(adapters: impl Into<Vec<RuntimeAdapter>>) -> Self {
        Self {
            adapters: adapters.into(),
            hardware_pressure: 0.0,
            compute_headroom: 0.5,
            latency_budget_ms: None,
            max_parallel_chunks: 1,
            kv_prefetch_blocks: 0,
            hot_kv_precision_bits: 8,
            cold_kv_precision_bits: 4,
            local_kv_token_budget: 0,
            global_kv_token_budget: 0,
            allow_disk_spill: false,
        }
    }

    pub fn with_pressure(mut self, hardware_pressure: f32, compute_headroom: f32) -> Self {
        self.hardware_pressure = hardware_pressure.clamp(0.0, 1.0);
        self.compute_headroom = compute_headroom.clamp(0.0, 1.0);
        self
    }

    pub fn with_latency_budget_ms(mut self, latency_budget_ms: Option<u64>) -> Self {
        self.latency_budget_ms = latency_budget_ms;
        self
    }

    pub fn with_parallel_chunks(mut self, max_parallel_chunks: usize) -> Self {
        self.max_parallel_chunks = max_parallel_chunks.max(1);
        self
    }

    pub fn with_kv_prefetch_blocks(mut self, kv_prefetch_blocks: usize) -> Self {
        self.kv_prefetch_blocks = kv_prefetch_blocks;
        self
    }

    pub fn with_kv_precision(mut self, hot_bits: u8, cold_bits: u8) -> Self {
        self.hot_kv_precision_bits = normalize_kv_bits(hot_bits, 8);
        self.cold_kv_precision_bits =
            normalize_kv_bits(cold_bits, 4).min(self.hot_kv_precision_bits);
        self
    }

    pub fn with_kv_token_budgets(mut self, local: usize, global: usize) -> Self {
        self.local_kv_token_budget = local;
        self.global_kv_token_budget = global;
        self
    }

    pub fn with_disk_spill(mut self, allow_disk_spill: bool) -> Self {
        self.allow_disk_spill = allow_disk_spill;
        self
    }

    pub fn select_adapter(&self, observations: &[AdapterObservation]) -> AdapterSelection {
        self.select_adapter_report(observations).selection
    }

    pub fn context_summary(&self) -> AdapterExecutionContextSummary {
        AdapterExecutionContextSummary {
            adapter_count: self.adapters.len(),
            hardware_pressure: self.hardware_pressure,
            compute_headroom: self.compute_headroom,
            latency_budget_ms: self.latency_budget_ms,
            max_parallel_chunks: self.max_parallel_chunks,
            kv_prefetch_blocks: self.kv_prefetch_blocks,
            hot_kv_precision_bits: self.hot_kv_precision_bits,
            cold_kv_precision_bits: self.cold_kv_precision_bits,
            local_kv_token_budget: self.local_kv_token_budget,
            global_kv_token_budget: self.global_kv_token_budget,
            allow_disk_spill: self.allow_disk_spill,
        }
    }

    pub fn runtime_clamp_summary(&self, runtime: &RuntimeMetadata) -> AdapterRuntimeClampSummary {
        let before = self.context_summary();
        let after = self.clone().clamp_for_runtime(runtime).context_summary();

        AdapterRuntimeClampSummary {
            before,
            after,
            runtime: runtime.shape_summary(),
            kv_prefetch_reduction: before
                .kv_prefetch_blocks
                .saturating_sub(after.kv_prefetch_blocks),
            hot_kv_precision_reduced: after.hot_kv_precision_bits < before.hot_kv_precision_bits,
            cold_kv_precision_reduced: after.cold_kv_precision_bits < before.cold_kv_precision_bits,
        }
    }

    pub fn select_adapter_report(
        &self,
        observations: &[AdapterObservation],
    ) -> AdapterSelectionReport {
        let matching_observation_count = observations
            .iter()
            .filter(|observation| self.adapters.contains(&observation.adapter))
            .count();

        observations
            .iter()
            .filter(|observation| self.adapters.contains(&observation.adapter))
            .max_by(|left, right| {
                left.score
                    .partial_cmp(&right.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| right.experience_id.cmp(&left.experience_id))
            })
            .map(|observation| AdapterSelection {
                adapter: observation.adapter,
                score: observation.score,
                experience_id: Some(observation.experience_id),
                used_fallback: false,
            })
            .map(|selection| AdapterSelectionReport {
                selection,
                allowed_adapter_count: self.adapters.len(),
                observation_count: observations.len(),
                matching_observation_count,
                fallback_reason: AdapterFallbackReason::NoFallback,
            })
            .unwrap_or_else(|| {
                let fallback_adapter = self
                    .adapters
                    .first()
                    .copied()
                    .unwrap_or(RuntimeAdapter::PortableRust);
                let fallback_reason = if self.adapters.is_empty() {
                    AdapterFallbackReason::NoAllowedAdapter
                } else {
                    AdapterFallbackReason::NoMatchingObservation
                };

                AdapterSelectionReport {
                    selection: AdapterSelection::fallback(fallback_adapter),
                    allowed_adapter_count: self.adapters.len(),
                    observation_count: observations.len(),
                    matching_observation_count,
                    fallback_reason,
                }
            })
    }

    pub fn selection_runtime_summary(
        &self,
        report: AdapterSelectionReport,
        runtime_selected_adapter: Option<RuntimeAdapter>,
    ) -> AdapterSelectionRuntimeSummary {
        let runtime_adapter_reported = runtime_selected_adapter.is_some();
        let runtime_adapter_matches_selection =
            runtime_selected_adapter == Some(report.selection.adapter);
        let runtime_adapter_allowed = runtime_selected_adapter
            .map(|adapter| self.adapters.contains(&adapter))
            .unwrap_or(false);

        AdapterSelectionRuntimeSummary {
            selection: report.selection,
            fallback_reason: report.fallback_reason,
            allowed_adapter_count: report.allowed_adapter_count,
            matching_observation_count: report.matching_observation_count,
            runtime_selected_adapter,
            runtime_adapter_reported,
            runtime_adapter_matches_selection,
            runtime_adapter_allowed,
        }
    }

    pub fn clamp_for_runtime(mut self, runtime: &RuntimeMetadata) -> Self {
        self.hot_kv_precision_bits = self
            .hot_kv_precision_bits
            .min(runtime.hot_kv_precision_bits);
        self.cold_kv_precision_bits = self
            .cold_kv_precision_bits
            .min(runtime.cold_kv_precision_bits)
            .min(self.hot_kv_precision_bits);

        if runtime.supports_kv_import {
            self.kv_prefetch_blocks = self.kv_prefetch_blocks.min(runtime.max_kv_import_blocks);
        } else {
            self.kv_prefetch_blocks = 0;
        }

        self
    }

    pub fn routing_context(
        &self,
        profile: TaskProfile,
        context_tokens: usize,
        cache_hit_rate: f32,
        hierarchy: HierarchyWeights,
    ) -> RoutingContext {
        RoutingContext {
            profile,
            context_tokens,
            cache_hit_rate,
            hardware_pressure: self.hardware_pressure,
            compute_headroom: self.compute_headroom,
            hierarchy,
        }
    }
}

impl Default for AdapterExecutionContext {
    fn default() -> Self {
        Self::new([RuntimeAdapter::PortableRust])
    }
}

fn normalize_kv_bits(bits: u8, fallback: u8) -> u8 {
    if matches!(bits, 4 | 8) {
        bits
    } else {
        fallback
    }
}

fn valid_kv_bits(bits: u8) -> bool {
    matches!(bits, 4 | 8)
}

fn bounded_unit_float(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn float_close(left: f32, right: f32) -> bool {
    (left - right).abs() <= 0.0001
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::RuntimeFailureKind;

    #[test]
    fn adapter_names_round_trip_from_root_style_strings() {
        let adapter = RuntimeAdapter::from_str("direct-ml").expect("known adapter");

        assert_eq!(adapter, RuntimeAdapter::DirectMl);
        assert_eq!(adapter.as_str(), "directml");
        assert!(RuntimeAdapter::from_str("missing").is_err());
    }

    #[test]
    fn execution_context_clamps_to_runtime_contract() {
        let runtime = RuntimeMetadata::new("adapter-model", "tok", 2048, 1024)
            .with_kv_exchange(true, true)
            .with_kv_limits(2, 4)
            .with_kv_precision(4, 4);
        let requested = AdapterExecutionContext::new([RuntimeAdapter::Cuda])
            .with_pressure(1.2, -0.1)
            .with_kv_prefetch_blocks(8)
            .with_kv_precision(8, 8);
        let clamp = requested.runtime_clamp_summary(&runtime);
        let context = requested.clone().clamp_for_runtime(&runtime);

        assert_eq!(context.hardware_pressure, 1.0);
        assert_eq!(context.compute_headroom, 0.0);
        assert_eq!(context.kv_prefetch_blocks, 2);
        assert_eq!(context.hot_kv_precision_bits, 4);
        assert_eq!(context.cold_kv_precision_bits, 4);

        let summary = context.context_summary();

        assert_eq!(summary.adapter_count, 1);
        assert_eq!(summary.hardware_pressure, 1.0);
        assert_eq!(summary.compute_headroom, 0.0);
        assert_eq!(summary.kv_prefetch_blocks, 2);
        assert_eq!(summary.hot_kv_precision_bits, 4);
        assert_eq!(summary.cold_kv_precision_bits, 4);
        assert!(summary.has_adapters());
        assert!(summary.pressure_limited());
        assert!(summary.has_kv_prefetch());
        assert!(summary.uses_compressed_hot_kv());
        assert!(summary.has_valid_hot_kv_precision());
        assert!(summary.has_valid_cold_kv_precision());
        assert!(summary.cold_kv_not_wider_than_hot());
        assert!(summary.pressure_values_are_bounded());
        assert_eq!(summary.adapter_candidate_signal_component_count(), 1);
        assert_eq!(summary.pressure_signal_component_count(), 2);
        assert_eq!(summary.execution_capacity_signal_component_count(), 0);
        assert_eq!(summary.kv_budget_signal_component_count(), 1);
        assert_eq!(summary.kv_precision_signal_component_count(), 3);
        assert_eq!(summary.adapter_context_signal_component_count(), 7);
        assert!(summary.has_adapter_context_signals());
        assert_eq!(summary.adapter_candidate_problem_component_count(), 0);
        assert_eq!(summary.pressure_shape_problem_component_count(), 0);
        assert_eq!(summary.execution_shape_problem_component_count(), 0);
        assert_eq!(summary.kv_precision_problem_component_count(), 0);
        assert_eq!(summary.adapter_context_problem_component_count(), 0);
        assert!(!summary.has_adapter_context_problem_components());
        assert!(summary.adapter_context_accounting_is_consistent());
        assert_eq!(
            summary.adapter_execution_context_commit_signal_component_count(),
            7
        );
        assert_eq!(
            summary.adapter_execution_context_commit_blocker_component_count(),
            0
        );
        assert!(!summary.has_adapter_execution_context_commit_blockers());
        assert_eq!(summary.component_accounting_drift_count(), 0);
        assert_eq!(
            summary.adapter_execution_context_commit_problem_component_count(),
            0
        );
        assert!(!summary.has_adapter_execution_context_commit_problem_components());
        assert!(summary.adapter_execution_context_commit_accounting_is_consistent());
        assert!(summary.adapter_execution_context_commit_is_clean());
        assert!(summary.adapter_context_shape_is_clean());
        assert!(summary.can_commit_adapter_execution_context());
        assert_eq!(
            summary.adapter_execution_context_commit_action(),
            AdapterExecutionContextCommitAction::CommitAdapterExecutionContext
        );
        assert!(summary.can_use_adapter_execution_context());
        assert_eq!(summary.failure_report(), None);
        assert_eq!(summary.failure_reports(), Vec::new());
        assert_eq!(summary.failure_report_count(), 0);
        assert!(!summary.has_failure_reports());
        assert_eq!(summary.failure_batch_summary().total_count, 0);
        assert!(!summary.can_format_runtime_failures());
        assert_eq!(summary.primary_failure_report(), None);
        assert_eq!(summary.primary_failure_summary(), None);
        let context_commit = summary.commit_summary();
        assert_eq!(
            context_commit.action,
            AdapterExecutionContextCommitAction::CommitAdapterExecutionContext
        );
        assert_eq!(
            context_commit.action,
            summary.adapter_execution_context_commit_action()
        );
        assert!(context_commit.action_can_commit());
        assert!(!context_commit.action_should_return_failure());
        assert!(context_commit.can_commit_adapter_execution_context());
        assert!(!context_commit.should_return_runtime_failure());
        assert!(context_commit.failure_reports.is_empty());
        assert_eq!(context_commit.primary_failure_report, None);
        assert_eq!(context_commit.primary_failure_summary, None);
        assert_eq!(context_commit.failure_report_count, 0);
        assert!(!context_commit.can_format_runtime_failures);
        assert_eq!(context_commit.total_signal_component_count, 7);
        assert_eq!(context_commit.total_blocker_component_count, 0);
        assert!(context_commit.component_accounting_consistent);
        assert!(!context_commit.has_primary_failure_summary());
        assert!(context_commit.failure_batch_shape_is_clean());
        assert!(context_commit.commit_decision_accounting_is_consistent());
        assert!(
            !context_commit
                .failure_return_summary()
                .can_return_runtime_failure()
        );
        assert_eq!(context_commit.runtime_failure_return_report(), None);
        assert_eq!(clamp.before.kv_prefetch_blocks, 8);
        assert_eq!(clamp.after, summary);
        assert_eq!(clamp.runtime, runtime.shape_summary());
        assert_eq!(clamp.kv_prefetch_reduction, 6);
        assert!(clamp.hot_kv_precision_reduced);
        assert!(clamp.cold_kv_precision_reduced);
        assert!(clamp.kv_prefetch_was_clamped());
        assert!(clamp.precision_was_clamped());
        assert!(clamp.adapter_count_preserved());
        assert!(clamp.pressure_preserved());
        assert!(clamp.execution_shape_preserved());
        assert!(clamp.token_budgets_preserved());
        assert!(clamp.runtime_limits_are_monotonic());
        assert!(clamp.only_runtime_limits_changed());
        assert_eq!(clamp.kv_prefetch_clamp_signal_component_count(), 1);
        assert_eq!(clamp.precision_clamp_signal_component_count(), 2);
        assert_eq!(clamp.runtime_clamp_signal_component_count(), 3);
        assert!(clamp.has_runtime_clamp_signals());
        assert_eq!(clamp.preservation_problem_component_count(), 0);
        assert_eq!(clamp.runtime_limit_problem_component_count(), 0);
        assert_eq!(clamp.after_context_problem_component_count(), 0);
        assert_eq!(clamp.runtime_clamp_problem_component_count(), 0);
        assert!(!clamp.has_runtime_clamp_problem_components());
        assert!(clamp.runtime_clamp_accounting_is_consistent());
        assert!(clamp.runtime_clamp_shape_is_clean());
        assert_eq!(clamp.runtime_clamp_commit_signal_component_count(), 3);
        assert!(clamp.has_runtime_clamp_commit_signals());
        assert_eq!(clamp.runtime_clamp_commit_blocker_component_count(), 0);
        assert!(!clamp.has_runtime_clamp_commit_blockers());
        assert_eq!(clamp.component_accounting_drift_count(), 0);
        assert_eq!(clamp.runtime_clamp_commit_problem_component_count(), 0);
        assert!(!clamp.has_runtime_clamp_commit_problem_components());
        assert_eq!(clamp.failure_report(), None);
        assert_eq!(clamp.failure_reports(), Vec::new());
        assert_eq!(clamp.failure_report_count(), 0);
        assert!(!clamp.has_failure_reports());
        assert_eq!(clamp.failure_batch_summary().total_count, 0);
        assert!(!clamp.can_format_runtime_failures());
        assert_eq!(clamp.primary_failure_report(), None);
        assert_eq!(clamp.primary_failure_summary(), None);
        assert!(clamp.runtime_clamp_commit_accounting_is_consistent());
        assert!(clamp.runtime_clamp_commit_is_clean());
        assert!(clamp.can_commit_runtime_clamp());
        assert_eq!(
            clamp.runtime_clamp_commit_action(),
            AdapterRuntimeClampCommitAction::CommitRuntimeClamp
        );
        assert!(clamp.can_use_runtime_clamp());
        let clamp_commit = clamp.commit_summary();
        assert_eq!(
            clamp_commit.action,
            AdapterRuntimeClampCommitAction::CommitRuntimeClamp
        );
        assert_eq!(clamp_commit.action, clamp.runtime_clamp_commit_action());
        assert!(clamp_commit.action_can_commit());
        assert!(!clamp_commit.action_should_return_failure());
        assert!(clamp_commit.can_commit_runtime_clamp());
        assert!(!clamp_commit.should_return_runtime_failure());
        assert!(clamp_commit.failure_reports.is_empty());
        assert_eq!(clamp_commit.primary_failure_report, None);
        assert_eq!(clamp_commit.primary_failure_summary, None);
        assert_eq!(clamp_commit.failure_report_count, 0);
        assert!(!clamp_commit.can_format_runtime_failures);
        assert_eq!(clamp_commit.total_signal_component_count, 3);
        assert_eq!(clamp_commit.total_blocker_component_count, 0);
        assert!(clamp_commit.component_accounting_consistent);
        assert!(!clamp_commit.has_primary_failure_summary());
        assert!(clamp_commit.failure_batch_shape_is_clean());
        assert!(clamp_commit.commit_decision_accounting_is_consistent());
        assert!(
            !clamp_commit
                .failure_return_summary()
                .can_return_runtime_failure()
        );
        assert_eq!(clamp_commit.runtime_failure_return_report(), None);
    }

    #[test]
    fn execution_context_builds_router_context() {
        let execution = AdapterExecutionContext::new([RuntimeAdapter::CpuSimd])
            .with_pressure(0.75, 0.25)
            .with_latency_budget_ms(Some(120))
            .with_parallel_chunks(3)
            .with_kv_prefetch_blocks(2)
            .with_kv_token_budgets(64, 512)
            .with_disk_spill(true);

        let routing = execution.routing_context(
            TaskProfile::Coding,
            4096,
            0.4,
            HierarchyWeights::for_profile(TaskProfile::Coding),
        );
        let summary = execution.context_summary();

        assert_eq!(routing.profile, TaskProfile::Coding);
        assert_eq!(routing.context_tokens, 4096);
        assert_eq!(routing.hardware_pressure, 0.75);
        assert_eq!(routing.compute_headroom, 0.25);
        assert_eq!(summary.latency_budget_ms, Some(120));
        assert_eq!(summary.max_parallel_chunks, 3);
        assert_eq!(summary.total_kv_token_budget(), 576);
        assert!(summary.has_latency_budget());
        assert!(summary.allow_disk_spill);
        assert_eq!(summary.adapter_candidate_signal_component_count(), 1);
        assert_eq!(summary.pressure_signal_component_count(), 2);
        assert_eq!(summary.execution_capacity_signal_component_count(), 3);
        assert_eq!(summary.kv_budget_signal_component_count(), 2);
        assert_eq!(summary.kv_precision_signal_component_count(), 2);
        assert_eq!(summary.adapter_context_signal_component_count(), 10);
        assert!(summary.has_adapter_context_signals());
        assert_eq!(summary.adapter_context_problem_component_count(), 0);
        assert!(!summary.has_adapter_context_problem_components());
        assert!(summary.adapter_context_accounting_is_consistent());
        assert_eq!(
            summary.adapter_execution_context_commit_signal_component_count(),
            10
        );
        assert_eq!(
            summary.adapter_execution_context_commit_blocker_component_count(),
            0
        );
        assert!(!summary.has_adapter_execution_context_commit_blockers());
        assert_eq!(summary.component_accounting_drift_count(), 0);
        assert_eq!(
            summary.adapter_execution_context_commit_problem_component_count(),
            0
        );
        assert!(!summary.has_adapter_execution_context_commit_problem_components());
        assert!(summary.adapter_execution_context_commit_accounting_is_consistent());
        assert!(summary.adapter_execution_context_commit_is_clean());
        assert!(summary.adapter_context_shape_is_clean());
        assert!(summary.can_commit_adapter_execution_context());
        assert!(summary.can_use_adapter_execution_context());
        assert!(
            summary
                .commit_summary()
                .can_commit_adapter_execution_context()
        );
    }

    #[test]
    fn adapter_execution_context_summary_counts_shape_problems() {
        let summary = AdapterExecutionContextSummary {
            adapter_count: 0,
            hardware_pressure: f32::NAN,
            compute_headroom: 1.2,
            latency_budget_ms: None,
            max_parallel_chunks: 0,
            kv_prefetch_blocks: 0,
            hot_kv_precision_bits: 6,
            cold_kv_precision_bits: 8,
            local_kv_token_budget: 0,
            global_kv_token_budget: 0,
            allow_disk_spill: false,
        };

        assert!(!summary.has_adapters());
        assert!(!summary.pressure_values_are_bounded());
        assert!(!summary.has_parallel_capacity());
        assert!(!summary.has_valid_hot_kv_precision());
        assert!(summary.has_valid_cold_kv_precision());
        assert!(!summary.cold_kv_not_wider_than_hot());
        assert_eq!(summary.adapter_candidate_signal_component_count(), 0);
        assert_eq!(summary.pressure_signal_component_count(), 0);
        assert_eq!(summary.execution_capacity_signal_component_count(), 0);
        assert_eq!(summary.kv_budget_signal_component_count(), 0);
        assert_eq!(summary.kv_precision_signal_component_count(), 1);
        assert_eq!(summary.adapter_context_signal_component_count(), 1);
        assert!(summary.has_adapter_context_signals());
        assert_eq!(summary.adapter_candidate_problem_component_count(), 1);
        assert_eq!(summary.pressure_shape_problem_component_count(), 2);
        assert_eq!(summary.execution_shape_problem_component_count(), 1);
        assert_eq!(summary.kv_precision_problem_component_count(), 2);
        assert_eq!(summary.adapter_context_problem_component_count(), 6);
        assert!(summary.has_adapter_context_problem_components());
        assert!(summary.adapter_context_accounting_is_consistent());
        assert_eq!(
            summary.adapter_execution_context_commit_signal_component_count(),
            1
        );
        assert_eq!(
            summary.adapter_execution_context_commit_blocker_component_count(),
            6
        );
        assert!(summary.has_adapter_execution_context_commit_blockers());
        assert_eq!(summary.component_accounting_drift_count(), 0);
        assert_eq!(
            summary.adapter_execution_context_commit_problem_component_count(),
            6
        );
        assert!(summary.has_adapter_execution_context_commit_problem_components());
        assert!(summary.adapter_execution_context_commit_accounting_is_consistent());
        assert!(!summary.adapter_execution_context_commit_is_clean());
        assert!(!summary.adapter_context_shape_is_clean());
        assert!(!summary.can_commit_adapter_execution_context());
        assert_eq!(
            summary.adapter_execution_context_commit_action(),
            AdapterExecutionContextCommitAction::ReturnRuntimeFailure
        );
        assert!(!summary.can_use_adapter_execution_context());
        let failures = summary.failure_reports();
        let primary_summary = summary
            .primary_failure_summary()
            .expect("adapter execution context failure summary is reported");
        assert_eq!(failures.len(), 1);
        assert_eq!(summary.failure_report_count(), 1);
        assert!(summary.has_failure_reports());
        assert_eq!(failures[0].kind, RuntimeFailureKind::ContractViolation);
        assert!(
            failures[0]
                .message
                .contains("adapter execution context failed")
        );
        assert_eq!(summary.primary_failure_report(), Some(failures[0].clone()));
        assert_eq!(primary_summary.kind, RuntimeFailureKind::ContractViolation);
        assert!(summary.can_format_runtime_failures());
        let commit = summary.commit_summary();
        assert_eq!(
            commit.action,
            AdapterExecutionContextCommitAction::ReturnRuntimeFailure
        );
        assert_eq!(
            commit.action,
            summary.adapter_execution_context_commit_action()
        );
        assert!(!commit.action_can_commit());
        assert!(commit.action_should_return_failure());
        assert!(!commit.can_commit_adapter_execution_context());
        assert!(commit.should_return_runtime_failure());
        assert_eq!(commit.failure_reports, failures.clone());
        assert_eq!(commit.primary_failure_report, Some(failures[0].clone()));
        assert_eq!(commit.primary_failure_summary, Some(primary_summary));
        assert_eq!(commit.failure_batch.contract_violation_count, 1);
        assert_eq!(commit.failure_report_count, 1);
        assert!(commit.can_format_runtime_failures);
        assert_eq!(commit.total_signal_component_count, 1);
        assert_eq!(commit.total_blocker_component_count, 6);
        assert!(commit.component_accounting_consistent);
        assert!(commit.has_primary_failure_summary());
        assert!(commit.failure_batch_shape_is_clean());
        assert!(commit.commit_decision_accounting_is_consistent());
        let failure_return = commit.failure_return_summary();
        assert_eq!(
            failure_return.source,
            AdapterFailureReturnSource::AdapterExecutionContext
        );
        assert_eq!(failure_return.source.label(), "adapter_execution_context");
        assert!(failure_return.can_return_runtime_failure());
        assert!(failure_return.failure_return_accounting_is_consistent());
        let return_report = commit
            .runtime_failure_return_report()
            .expect("adapter execution context failure return report");
        assert_eq!(
            return_report.source,
            AdapterFailureReturnSource::AdapterExecutionContext
        );
        assert_eq!(return_report.primary_failure, failures[0]);
        assert_eq!(
            return_report.inference_error().message,
            return_report.backend_message()
        );
        assert!(return_report.can_use_adapter_failure_return_report());
    }

    #[test]
    fn adapter_runtime_clamp_summary_counts_preservation_problems() {
        let before = AdapterExecutionContext::new([RuntimeAdapter::Cuda])
            .with_pressure(0.2, 0.8)
            .with_latency_budget_ms(Some(50))
            .with_parallel_chunks(2)
            .with_kv_prefetch_blocks(1)
            .with_kv_precision(4, 4)
            .with_kv_token_budgets(64, 128)
            .context_summary();
        let after = AdapterExecutionContextSummary {
            adapter_count: 0,
            hardware_pressure: 0.9,
            compute_headroom: 0.8,
            latency_budget_ms: None,
            max_parallel_chunks: 0,
            kv_prefetch_blocks: 2,
            hot_kv_precision_bits: 8,
            cold_kv_precision_bits: 8,
            local_kv_token_budget: 32,
            global_kv_token_budget: 128,
            allow_disk_spill: true,
        };
        let clamp = AdapterRuntimeClampSummary {
            before,
            after,
            runtime: RuntimeMetadata::new("model", "tok", 4096, 2048).shape_summary(),
            kv_prefetch_reduction: 0,
            hot_kv_precision_reduced: false,
            cold_kv_precision_reduced: false,
        };

        assert!(!clamp.kv_prefetch_was_clamped());
        assert!(!clamp.precision_was_clamped());
        assert_eq!(clamp.runtime_clamp_signal_component_count(), 0);
        assert!(!clamp.has_runtime_clamp_signals());
        assert!(!clamp.adapter_count_preserved());
        assert!(!clamp.pressure_preserved());
        assert!(!clamp.execution_shape_preserved());
        assert!(!clamp.token_budgets_preserved());
        assert!(!clamp.runtime_limits_are_monotonic());
        assert!(!clamp.only_runtime_limits_changed());
        assert_eq!(clamp.preservation_problem_component_count(), 4);
        assert_eq!(clamp.runtime_limit_problem_component_count(), 1);
        assert_eq!(clamp.after_context_problem_component_count(), 2);
        assert_eq!(clamp.runtime_clamp_problem_component_count(), 7);
        assert!(clamp.has_runtime_clamp_problem_components());
        assert!(clamp.runtime_clamp_accounting_is_consistent());
        assert!(!clamp.runtime_clamp_shape_is_clean());
        assert_eq!(clamp.runtime_clamp_commit_signal_component_count(), 0);
        assert!(!clamp.has_runtime_clamp_commit_signals());
        assert_eq!(clamp.runtime_clamp_commit_blocker_component_count(), 7);
        assert!(clamp.has_runtime_clamp_commit_blockers());
        assert_eq!(clamp.component_accounting_drift_count(), 0);
        assert_eq!(clamp.runtime_clamp_commit_problem_component_count(), 7);
        assert!(clamp.has_runtime_clamp_commit_problem_components());
        assert!(clamp.runtime_clamp_commit_accounting_is_consistent());
        assert!(!clamp.runtime_clamp_commit_is_clean());
        assert!(!clamp.can_commit_runtime_clamp());
        assert_eq!(
            clamp.runtime_clamp_commit_action(),
            AdapterRuntimeClampCommitAction::ReturnRuntimeFailure
        );
        assert!(!clamp.can_use_runtime_clamp());
        let failures = clamp.failure_reports();
        let primary_summary = clamp
            .primary_failure_summary()
            .expect("runtime clamp failure summary is reported");
        assert_eq!(failures.len(), 1);
        assert_eq!(clamp.failure_report_count(), 1);
        assert!(clamp.has_failure_reports());
        assert_eq!(failures[0].kind, RuntimeFailureKind::ContractViolation);
        assert!(failures[0].message.contains("adapter runtime clamp failed"));
        assert_eq!(clamp.primary_failure_report(), Some(failures[0].clone()));
        assert_eq!(primary_summary.kind, RuntimeFailureKind::ContractViolation);
        assert!(clamp.can_format_runtime_failures());
        let commit = clamp.commit_summary();
        assert_eq!(
            commit.action,
            AdapterRuntimeClampCommitAction::ReturnRuntimeFailure
        );
        assert_eq!(commit.action, clamp.runtime_clamp_commit_action());
        assert!(!commit.action_can_commit());
        assert!(commit.action_should_return_failure());
        assert!(!commit.can_commit_runtime_clamp());
        assert!(commit.should_return_runtime_failure());
        assert_eq!(commit.failure_reports, failures.clone());
        assert_eq!(commit.primary_failure_report, Some(failures[0].clone()));
        assert_eq!(commit.primary_failure_summary, Some(primary_summary));
        assert_eq!(commit.failure_batch.contract_violation_count, 1);
        assert_eq!(commit.failure_report_count, 1);
        assert!(commit.can_format_runtime_failures);
        assert_eq!(commit.total_signal_component_count, 0);
        assert_eq!(commit.total_blocker_component_count, 7);
        assert!(commit.component_accounting_consistent);
        assert!(commit.has_primary_failure_summary());
        assert!(commit.failure_batch_shape_is_clean());
        assert!(commit.commit_decision_accounting_is_consistent());
        let failure_return = commit.failure_return_summary();
        assert_eq!(
            failure_return.source,
            AdapterFailureReturnSource::RuntimeClamp
        );
        assert!(failure_return.can_return_runtime_failure());
        assert!(failure_return.failure_return_accounting_is_consistent());
        let return_report = commit
            .runtime_failure_return_report()
            .expect("runtime clamp failure return report");
        assert_eq!(
            return_report.source,
            AdapterFailureReturnSource::RuntimeClamp
        );
        assert_eq!(return_report.primary_failure, failures[0]);
        assert_eq!(
            return_report.diagnostics_note(),
            failures[0].diagnostics_note()
        );
        assert!(return_report.can_use_adapter_failure_return_report());
    }

    #[test]
    fn adapter_selection_prefers_best_allowed_observation() {
        let context = AdapterExecutionContext::new([RuntimeAdapter::CpuSimd, RuntimeAdapter::Cuda]);
        let observations = [
            AdapterObservation::new(RuntimeAdapter::Metal, 0.99, 0.9, 0.9, None, None, 1),
            AdapterObservation::new(RuntimeAdapter::CpuSimd, 0.52, 0.5, 0.5, None, None, 2),
            AdapterObservation::new(RuntimeAdapter::Cuda, 0.78, 0.7, 0.8, None, None, 3),
        ];

        let selection = context.select_adapter(&observations);

        assert_eq!(selection.adapter, RuntimeAdapter::Cuda);
        assert_eq!(selection.score, 0.78);
        assert_eq!(selection.experience_id, Some(3));
        assert!(!selection.used_fallback);
    }

    #[test]
    fn adapter_selection_report_counts_matching_observations() {
        let context = AdapterExecutionContext::new([RuntimeAdapter::CpuSimd, RuntimeAdapter::Cuda]);
        let observations = [
            AdapterObservation::new(RuntimeAdapter::Metal, 0.99, 0.9, 0.9, None, None, 1),
            AdapterObservation::new(RuntimeAdapter::CpuSimd, 0.52, 0.5, 0.5, None, None, 2),
            AdapterObservation::new(RuntimeAdapter::Cuda, 0.78, 0.7, 0.8, None, None, 3),
        ];

        let report = context.select_adapter_report(&observations);

        assert_eq!(report.selection.adapter, RuntimeAdapter::Cuda);
        assert_eq!(report.allowed_adapter_count, 2);
        assert_eq!(report.observation_count, 3);
        assert_eq!(report.matching_observation_count, 2);
        assert_eq!(report.rejected_observation_count(), 1);
        assert_eq!(report.fallback_reason, AdapterFallbackReason::NoFallback);
        assert_eq!(report.fallback_reason.as_str(), "none");
        assert!(!report.used_fallback());
        assert!(report.selection_from_observation());
        assert!(report.has_allowed_adapters());
        assert!(report.has_observations());
        assert!(report.has_matching_observations());
        assert!(report.matching_observations_within_observation_count());
        assert!(!report.observations_all_rejected());
        assert!(!report.fallback_due_to_no_allowed_adapter());
        assert!(!report.fallback_due_to_no_matching_observation());
        assert!(report.fallback_reason_matches_selection());
        assert!((report.matched_observation_fraction() - (2.0 / 3.0)).abs() < f32::EPSILON);
        assert_eq!(report.adapter_catalog_signal_component_count(), 1);
        assert_eq!(report.observation_signal_component_count(), 3);
        assert_eq!(report.fallback_signal_component_count(), 0);
        assert_eq!(report.selection_report_signal_component_count(), 4);
        assert!(report.has_selection_report_signals());
        assert_eq!(report.observation_shape_problem_component_count(), 0);
        assert_eq!(report.fallback_reason_problem_component_count(), 0);
        assert_eq!(report.selection_report_problem_component_count(), 0);
        assert!(!report.has_selection_report_problem_components());
        assert!(report.selection_report_accounting_is_consistent());
        assert_eq!(report.adapter_selection_commit_signal_component_count(), 4);
        assert_eq!(report.adapter_selection_commit_blocker_component_count(), 0);
        assert!(!report.has_adapter_selection_commit_blockers());
        assert_eq!(report.component_accounting_drift_count(), 0);
        assert_eq!(report.adapter_selection_commit_problem_component_count(), 0);
        assert!(!report.has_adapter_selection_commit_problem_components());
        assert!(report.adapter_selection_commit_accounting_is_consistent());
        assert!(report.adapter_selection_commit_is_clean());
        assert!(report.selection_report_shape_is_clean());
        assert!(report.can_commit_adapter_selection());
        assert_eq!(
            report.adapter_selection_commit_action(),
            AdapterSelectionCommitAction::CommitAdapterSelection
        );
        assert!(report.can_use_adapter_selection());
        assert_eq!(report.failure_report(), None);
        assert_eq!(report.failure_reports(), Vec::new());
        assert_eq!(report.failure_report_count(), 0);
        assert!(!report.has_failure_reports());
        assert_eq!(report.failure_batch_summary().total_count, 0);
        assert!(!report.can_format_runtime_failures());
        assert_eq!(report.primary_failure_report(), None);
        assert_eq!(report.primary_failure_summary(), None);
        let commit = report.commit_summary();
        assert_eq!(
            commit.action,
            AdapterSelectionCommitAction::CommitAdapterSelection
        );
        assert_eq!(commit.action, report.adapter_selection_commit_action());
        assert!(commit.action_can_commit());
        assert!(!commit.action_should_return_failure());
        assert!(commit.can_commit_adapter_selection());
        assert!(!commit.should_return_runtime_failure());
        assert!(commit.failure_reports.is_empty());
        assert_eq!(commit.primary_failure_report, None);
        assert_eq!(commit.primary_failure_summary, None);
        assert_eq!(commit.failure_report_count, 0);
        assert!(!commit.can_format_runtime_failures);
        assert_eq!(commit.total_signal_component_count, 4);
        assert_eq!(commit.total_blocker_component_count, 0);
        assert!(commit.component_accounting_consistent);
        assert!(!commit.has_primary_failure_summary());
        assert!(commit.failure_batch_shape_is_clean());
        assert!(commit.commit_decision_accounting_is_consistent());
        assert!(!commit.failure_return_summary().can_return_runtime_failure());
        assert_eq!(commit.runtime_failure_return_report(), None);

        let runtime = context.selection_runtime_summary(report, Some(RuntimeAdapter::Cuda));

        assert_eq!(runtime.selection.adapter, RuntimeAdapter::Cuda);
        assert_eq!(runtime.allowed_adapter_count, 2);
        assert_eq!(runtime.matching_observation_count, 2);
        assert_eq!(runtime.runtime_selected_adapter, Some(RuntimeAdapter::Cuda));
        assert!(runtime.runtime_selection_confirmed());
        assert!(!runtime.runtime_adapter_missing());
        assert!(!runtime.runtime_selection_drifted());
        assert!(!runtime.runtime_adapter_outside_execution_context());
        assert!(runtime.fallback_has_reason());
        assert!(!runtime.fallback_without_reason());
        assert!(!runtime.runtime_adapter_problem());
        assert_eq!(runtime.adapter_source_signal_component_count(), 2);
        assert_eq!(runtime.runtime_report_signal_component_count(), 2);
        assert_eq!(runtime.fallback_reason_signal_component_count(), 1);
        assert_eq!(runtime.runtime_adapter_signal_component_count(), 5);
        assert!(runtime.has_runtime_adapter_signals());
        assert_eq!(runtime.fallback_reason_problem_component_count(), 0);
        assert_eq!(runtime.runtime_adapter_problem_component_count(), 0);
        assert!(!runtime.has_runtime_adapter_problem_components());
        assert!(runtime.runtime_adapter_accounting_is_consistent());
        assert_eq!(
            runtime.runtime_adapter_execution_commit_signal_component_count(),
            5
        );
        assert_eq!(
            runtime.runtime_adapter_execution_commit_blocker_component_count(),
            0
        );
        assert!(!runtime.has_runtime_adapter_execution_commit_blockers());
        assert_eq!(runtime.component_accounting_drift_count(), 0);
        assert_eq!(
            runtime.runtime_adapter_execution_commit_problem_component_count(),
            0
        );
        assert!(!runtime.has_runtime_adapter_execution_commit_problem_components());
        assert!(runtime.runtime_adapter_execution_commit_accounting_is_consistent());
        assert!(runtime.runtime_adapter_execution_commit_is_clean());
        assert!(runtime.runtime_adapter_shape_is_clean());
        assert!(runtime.can_commit_runtime_adapter_execution());
        assert_eq!(
            runtime.runtime_adapter_execution_commit_action(),
            AdapterSelectionRuntimeCommitAction::CommitRuntimeAdapterExecution
        );
        assert!(runtime.can_use_runtime_adapter_execution());
        assert!(runtime.is_clean_runtime_adapter_execution());
        assert_eq!(runtime.failure_report(), None);
        assert_eq!(runtime.failure_reports(), Vec::new());
        assert_eq!(runtime.failure_report_count(), 0);
        assert!(!runtime.has_failure_reports());
        assert_eq!(runtime.failure_batch_summary().total_count, 0);
        assert!(!runtime.can_format_runtime_failures());
        assert_eq!(runtime.primary_failure_report(), None);
        assert_eq!(runtime.primary_failure_summary(), None);
        let runtime_commit = runtime.commit_summary();
        assert_eq!(
            runtime_commit.action,
            AdapterSelectionRuntimeCommitAction::CommitRuntimeAdapterExecution
        );
        assert_eq!(
            runtime_commit.action,
            runtime.runtime_adapter_execution_commit_action()
        );
        assert!(runtime_commit.action_can_commit());
        assert!(!runtime_commit.action_should_return_failure());
        assert!(runtime_commit.can_commit_runtime_adapter_execution());
        assert!(!runtime_commit.should_return_runtime_failure());
        assert!(runtime_commit.failure_reports.is_empty());
        assert_eq!(runtime_commit.primary_failure_report, None);
        assert_eq!(runtime_commit.primary_failure_summary, None);
        assert_eq!(runtime_commit.failure_report_count, 0);
        assert!(!runtime_commit.can_format_runtime_failures);
        assert_eq!(runtime_commit.total_signal_component_count, 5);
        assert_eq!(runtime_commit.total_blocker_component_count, 0);
        assert!(runtime_commit.component_accounting_consistent);
        assert!(!runtime_commit.has_primary_failure_summary());
        assert!(runtime_commit.failure_batch_shape_is_clean());
        assert!(runtime_commit.commit_decision_accounting_is_consistent());
        assert!(
            !runtime_commit
                .failure_return_summary()
                .can_return_runtime_failure()
        );
        assert_eq!(runtime_commit.runtime_failure_return_report(), None);
    }

    #[test]
    fn adapter_selection_uses_stable_experience_tie_break() {
        let context = AdapterExecutionContext::new([RuntimeAdapter::Cuda, RuntimeAdapter::CpuSimd]);
        let observations = [
            AdapterObservation::new(RuntimeAdapter::Cuda, 0.80, 0.9, 0.8, None, None, 42),
            AdapterObservation::new(RuntimeAdapter::CpuSimd, 0.80, 0.9, 0.8, None, None, 7),
        ];

        let selection = context.select_adapter(&observations);

        assert_eq!(selection.adapter, RuntimeAdapter::CpuSimd);
        assert_eq!(selection.experience_id, Some(7));
    }

    #[test]
    fn adapter_selection_falls_back_to_first_allowed_adapter() {
        let context = AdapterExecutionContext::new([RuntimeAdapter::PortableRust]);
        let observations = [AdapterObservation::new(
            RuntimeAdapter::Cuda,
            0.95,
            0.9,
            0.8,
            Some(f32::INFINITY),
            Some(f32::NAN),
            10,
        )];

        let selection = context.select_adapter(&observations);

        assert_eq!(selection.adapter, RuntimeAdapter::PortableRust);
        assert_eq!(selection.experience_id, None);
        assert!(selection.used_fallback);
        assert_eq!(observations[0].forward_energy, None);
        assert_eq!(observations[0].kv_influence, None);

        let report = context.select_adapter_report(&observations);

        assert!(report.used_fallback());
        assert!(!report.selection_from_observation());
        assert!(report.observations_all_rejected());
        assert!(!report.fallback_due_to_no_allowed_adapter());
        assert!(report.fallback_due_to_no_matching_observation());
        assert!(report.fallback_reason_matches_selection());
        assert_eq!(report.adapter_catalog_signal_component_count(), 1);
        assert_eq!(report.observation_signal_component_count(), 1);
        assert_eq!(report.fallback_signal_component_count(), 3);
        assert_eq!(report.selection_report_signal_component_count(), 5);
        assert!(report.has_selection_report_signals());
        assert_eq!(report.selection_report_problem_component_count(), 0);
        assert!(report.selection_report_accounting_is_consistent());
        assert_eq!(report.adapter_selection_commit_signal_component_count(), 5);
        assert_eq!(report.adapter_selection_commit_blocker_component_count(), 0);
        assert!(!report.has_adapter_selection_commit_blockers());
        assert_eq!(report.component_accounting_drift_count(), 0);
        assert_eq!(report.adapter_selection_commit_problem_component_count(), 0);
        assert!(!report.has_adapter_selection_commit_problem_components());
        assert!(report.adapter_selection_commit_accounting_is_consistent());
        assert!(report.adapter_selection_commit_is_clean());
        assert!(report.selection_report_shape_is_clean());
        assert!(report.can_commit_adapter_selection());
        assert!(report.can_use_adapter_selection());
        assert!(report.commit_summary().can_commit_adapter_selection());
    }

    #[test]
    fn adapter_selection_report_marks_empty_allowed_adapter_fallback() {
        let context = AdapterExecutionContext::new(Vec::new());
        let observations = [AdapterObservation::new(
            RuntimeAdapter::Cuda,
            0.95,
            0.9,
            0.8,
            None,
            None,
            10,
        )];

        let report = context.select_adapter_report(&observations);

        assert_eq!(report.selection.adapter, RuntimeAdapter::PortableRust);
        assert!(report.used_fallback());
        assert!(!report.selection_from_observation());
        assert!(!report.has_allowed_adapters());
        assert!(report.has_observations());
        assert!(!report.has_matching_observations());
        assert_eq!(report.allowed_adapter_count, 0);
        assert_eq!(report.matching_observation_count, 0);
        assert_eq!(report.rejected_observation_count(), 1);
        assert!(report.observations_all_rejected());
        assert!(report.fallback_due_to_no_allowed_adapter());
        assert!(!report.fallback_due_to_no_matching_observation());
        assert!(report.matching_observations_within_observation_count());
        assert!(report.fallback_reason_matches_selection());
        assert_eq!(
            report.fallback_reason,
            AdapterFallbackReason::NoAllowedAdapter
        );
        assert_eq!(report.fallback_reason.as_str(), "no-allowed-adapter");
        assert_eq!(report.adapter_catalog_signal_component_count(), 0);
        assert_eq!(report.observation_signal_component_count(), 1);
        assert_eq!(report.fallback_signal_component_count(), 3);
        assert_eq!(report.selection_report_signal_component_count(), 4);
        assert!(report.has_selection_report_signals());
        assert_eq!(report.observation_shape_problem_component_count(), 0);
        assert_eq!(report.fallback_reason_problem_component_count(), 0);
        assert_eq!(report.selection_report_problem_component_count(), 0);
        assert!(report.selection_report_accounting_is_consistent());
        assert_eq!(report.adapter_selection_commit_signal_component_count(), 4);
        assert_eq!(report.adapter_selection_commit_blocker_component_count(), 1);
        assert!(report.has_adapter_selection_commit_blockers());
        assert_eq!(report.component_accounting_drift_count(), 0);
        assert_eq!(report.adapter_selection_commit_problem_component_count(), 1);
        assert!(report.has_adapter_selection_commit_problem_components());
        assert!(report.adapter_selection_commit_accounting_is_consistent());
        assert!(!report.adapter_selection_commit_is_clean());
        assert!(report.selection_report_shape_is_clean());
        assert!(!report.can_commit_adapter_selection());
        assert_eq!(
            report.adapter_selection_commit_action(),
            AdapterSelectionCommitAction::ReturnRuntimeFailure
        );
        assert!(!report.can_use_adapter_selection());
        let failures = report.failure_reports();
        let primary_summary = report
            .primary_failure_summary()
            .expect("adapter selection failure summary is reported");
        assert_eq!(failures.len(), 1);
        assert_eq!(report.failure_report_count(), 1);
        assert!(report.has_failure_reports());
        assert_eq!(failures[0].kind, RuntimeFailureKind::ContractViolation);
        assert!(failures[0].message.contains("adapter selection failed"));
        assert_eq!(report.primary_failure_report(), Some(failures[0].clone()));
        assert_eq!(primary_summary.kind, RuntimeFailureKind::ContractViolation);
        assert!(report.can_format_runtime_failures());
        let commit = report.commit_summary();
        assert_eq!(
            commit.action,
            AdapterSelectionCommitAction::ReturnRuntimeFailure
        );
        assert_eq!(commit.action, report.adapter_selection_commit_action());
        assert!(!commit.action_can_commit());
        assert!(commit.action_should_return_failure());
        assert!(!commit.can_commit_adapter_selection());
        assert!(commit.should_return_runtime_failure());
        assert_eq!(commit.failure_reports, failures.clone());
        assert_eq!(commit.primary_failure_report, Some(failures[0].clone()));
        assert_eq!(commit.primary_failure_summary, Some(primary_summary));
        assert_eq!(commit.failure_batch.contract_violation_count, 1);
        assert_eq!(commit.failure_report_count, 1);
        assert!(commit.can_format_runtime_failures);
        assert_eq!(commit.total_signal_component_count, 4);
        assert_eq!(commit.total_blocker_component_count, 1);
        assert!(commit.component_accounting_consistent);
        assert!(commit.has_primary_failure_summary());
        assert!(commit.failure_batch_shape_is_clean());
        assert!(commit.commit_decision_accounting_is_consistent());
        let failure_return = commit.failure_return_summary();
        assert_eq!(
            failure_return.source,
            AdapterFailureReturnSource::AdapterSelection
        );
        assert!(failure_return.can_return_runtime_failure());
        assert!(failure_return.failure_return_accounting_is_consistent());
        let return_report = commit
            .runtime_failure_return_report()
            .expect("adapter selection failure return report");
        assert_eq!(
            return_report.source,
            AdapterFailureReturnSource::AdapterSelection
        );
        assert_eq!(return_report.primary_failure, failures[0]);
        assert_eq!(
            return_report.backend_message(),
            failures[0].backend_message()
        );
        assert!(return_report.can_use_adapter_failure_return_report());

        let summary = context.context_summary();

        assert_eq!(summary.adapter_count, 0);
        assert!(!summary.has_adapters());
        assert!(!summary.has_kv_prefetch());
        assert_eq!(summary.total_kv_token_budget(), 0);
    }

    #[test]
    fn adapter_selection_report_counts_public_shape_drift() {
        let report = AdapterSelectionReport {
            selection: AdapterSelection {
                adapter: RuntimeAdapter::Cuda,
                score: 0.72,
                experience_id: Some(11),
                used_fallback: false,
            },
            allowed_adapter_count: 2,
            observation_count: 1,
            matching_observation_count: 3,
            fallback_reason: AdapterFallbackReason::NoMatchingObservation,
        };

        assert!(!report.used_fallback());
        assert!(report.selection_from_observation());
        assert!(report.has_allowed_adapters());
        assert!(report.has_observations());
        assert!(report.has_matching_observations());
        assert!(!report.matching_observations_within_observation_count());
        assert!(!report.fallback_reason_matches_selection());
        assert_eq!(report.adapter_catalog_signal_component_count(), 1);
        assert_eq!(report.observation_signal_component_count(), 3);
        assert_eq!(report.fallback_signal_component_count(), 0);
        assert_eq!(report.selection_report_signal_component_count(), 4);
        assert_eq!(report.observation_shape_problem_component_count(), 1);
        assert_eq!(report.fallback_reason_problem_component_count(), 1);
        assert_eq!(report.selection_report_problem_component_count(), 2);
        assert!(report.has_selection_report_problem_components());
        assert!(report.selection_report_accounting_is_consistent());
        assert_eq!(report.adapter_selection_commit_signal_component_count(), 4);
        assert_eq!(report.adapter_selection_commit_blocker_component_count(), 2);
        assert!(report.has_adapter_selection_commit_blockers());
        assert_eq!(report.component_accounting_drift_count(), 0);
        assert_eq!(report.adapter_selection_commit_problem_component_count(), 2);
        assert!(report.has_adapter_selection_commit_problem_components());
        assert!(report.adapter_selection_commit_accounting_is_consistent());
        assert!(!report.adapter_selection_commit_is_clean());
        assert!(!report.selection_report_shape_is_clean());
        assert!(!report.can_commit_adapter_selection());
        assert_eq!(
            report.adapter_selection_commit_action(),
            AdapterSelectionCommitAction::ReturnRuntimeFailure
        );
        assert!(!report.can_use_adapter_selection());
        let commit = report.commit_summary();
        assert_eq!(
            commit.action,
            AdapterSelectionCommitAction::ReturnRuntimeFailure
        );
        assert_eq!(commit.action, report.adapter_selection_commit_action());
        assert_eq!(commit.failure_report_count, 1);
        assert!(commit.should_return_runtime_failure());
        assert_eq!(commit.total_signal_component_count, 4);
        assert_eq!(commit.total_blocker_component_count, 2);
        assert!(commit.commit_decision_accounting_is_consistent());
    }

    #[test]
    fn selection_runtime_summary_reports_missing_and_drifted_runtime_adapters() {
        let context = AdapterExecutionContext::new([RuntimeAdapter::CpuSimd, RuntimeAdapter::Cuda]);
        let observations = [AdapterObservation::new(
            RuntimeAdapter::Cuda,
            0.78,
            0.7,
            0.8,
            None,
            None,
            3,
        )];
        let report = context.select_adapter_report(&observations);

        let missing = context.selection_runtime_summary(report, None);

        assert!(missing.runtime_adapter_missing());
        assert!(!missing.runtime_selection_confirmed());
        assert!(!missing.runtime_selection_drifted());
        assert!(!missing.runtime_adapter_outside_execution_context());
        assert!(missing.runtime_adapter_problem());
        assert_eq!(missing.adapter_source_signal_component_count(), 2);
        assert_eq!(missing.runtime_report_signal_component_count(), 0);
        assert_eq!(missing.fallback_reason_signal_component_count(), 1);
        assert_eq!(missing.runtime_adapter_signal_component_count(), 3);
        assert!(missing.has_runtime_adapter_signals());
        assert_eq!(missing.fallback_reason_problem_component_count(), 0);
        assert_eq!(missing.runtime_adapter_problem_component_count(), 1);
        assert!(missing.has_runtime_adapter_problem_components());
        assert!(missing.runtime_adapter_accounting_is_consistent());
        assert_eq!(
            missing.runtime_adapter_execution_commit_signal_component_count(),
            3
        );
        assert_eq!(
            missing.runtime_adapter_execution_commit_blocker_component_count(),
            1
        );
        assert!(missing.has_runtime_adapter_execution_commit_blockers());
        assert_eq!(missing.component_accounting_drift_count(), 0);
        assert_eq!(
            missing.runtime_adapter_execution_commit_problem_component_count(),
            1
        );
        assert!(missing.has_runtime_adapter_execution_commit_problem_components());
        assert!(missing.runtime_adapter_execution_commit_accounting_is_consistent());
        assert!(!missing.runtime_adapter_execution_commit_is_clean());
        assert!(!missing.runtime_adapter_shape_is_clean());
        assert!(!missing.can_commit_runtime_adapter_execution());
        assert_eq!(
            missing.runtime_adapter_execution_commit_action(),
            AdapterSelectionRuntimeCommitAction::ReturnRuntimeFailure
        );
        assert!(!missing.can_use_runtime_adapter_execution());
        assert!(!missing.is_clean_runtime_adapter_execution());
        let missing_failures = missing.failure_reports();
        let missing_primary_summary = missing
            .primary_failure_summary()
            .expect("runtime adapter missing failure summary is reported");
        assert_eq!(missing_failures.len(), 1);
        assert_eq!(missing.failure_report_count(), 1);
        assert!(missing.has_failure_reports());
        assert_eq!(
            missing_failures[0].kind,
            RuntimeFailureKind::ContractViolation
        );
        assert!(
            missing_failures[0]
                .message
                .contains("adapter runtime selection failed")
        );
        assert_eq!(
            missing.primary_failure_report(),
            Some(missing_failures[0].clone())
        );
        assert_eq!(
            missing_primary_summary.kind,
            RuntimeFailureKind::ContractViolation
        );
        assert!(missing.can_format_runtime_failures());
        let missing_commit = missing.commit_summary();
        assert_eq!(
            missing_commit.action,
            AdapterSelectionRuntimeCommitAction::ReturnRuntimeFailure
        );
        assert_eq!(
            missing_commit.action,
            missing.runtime_adapter_execution_commit_action()
        );
        assert!(!missing_commit.action_can_commit());
        assert!(missing_commit.action_should_return_failure());
        assert!(!missing_commit.can_commit_runtime_adapter_execution());
        assert!(missing_commit.should_return_runtime_failure());
        assert_eq!(missing_commit.failure_reports, missing_failures.clone());
        assert_eq!(
            missing_commit.primary_failure_report,
            Some(missing_failures[0].clone())
        );
        assert_eq!(
            missing_commit.primary_failure_summary,
            Some(missing_primary_summary)
        );
        assert_eq!(missing_commit.failure_batch.contract_violation_count, 1);
        assert_eq!(missing_commit.failure_report_count, 1);
        assert!(missing_commit.can_format_runtime_failures);
        assert_eq!(missing_commit.total_signal_component_count, 3);
        assert_eq!(missing_commit.total_blocker_component_count, 1);
        assert!(missing_commit.component_accounting_consistent);
        assert!(missing_commit.has_primary_failure_summary());
        assert!(missing_commit.failure_batch_shape_is_clean());
        assert!(missing_commit.commit_decision_accounting_is_consistent());
        let missing_failure_return = missing_commit.failure_return_summary();
        assert_eq!(
            missing_failure_return.source,
            AdapterFailureReturnSource::RuntimeAdapterExecution
        );
        assert!(missing_failure_return.can_return_runtime_failure());
        assert!(missing_failure_return.failure_return_accounting_is_consistent());
        let missing_return_report = missing_commit
            .runtime_failure_return_report()
            .expect("runtime adapter missing failure return report");
        assert_eq!(
            missing_return_report.source,
            AdapterFailureReturnSource::RuntimeAdapterExecution
        );
        assert_eq!(missing_return_report.primary_failure, missing_failures[0]);
        assert_eq!(
            missing_return_report.inference_error().message,
            missing_return_report.backend_message()
        );
        assert!(missing_return_report.can_use_adapter_failure_return_report());

        let drifted = context.selection_runtime_summary(report, Some(RuntimeAdapter::Metal));

        assert_eq!(
            drifted.runtime_selected_adapter,
            Some(RuntimeAdapter::Metal)
        );
        assert!(drifted.runtime_selection_drifted());
        assert!(drifted.runtime_adapter_outside_execution_context());
        assert!(!drifted.runtime_selection_confirmed());
        assert!(drifted.fallback_has_reason());
        assert!(!drifted.fallback_without_reason());
        assert!(drifted.runtime_adapter_problem());
        assert_eq!(drifted.adapter_source_signal_component_count(), 2);
        assert_eq!(drifted.runtime_report_signal_component_count(), 1);
        assert_eq!(drifted.fallback_reason_signal_component_count(), 1);
        assert_eq!(drifted.runtime_adapter_signal_component_count(), 4);
        assert!(drifted.has_runtime_adapter_signals());
        assert_eq!(drifted.fallback_reason_problem_component_count(), 0);
        assert_eq!(drifted.runtime_adapter_problem_component_count(), 2);
        assert!(drifted.has_runtime_adapter_problem_components());
        assert!(drifted.runtime_adapter_accounting_is_consistent());
        assert_eq!(
            drifted.runtime_adapter_execution_commit_signal_component_count(),
            4
        );
        assert_eq!(
            drifted.runtime_adapter_execution_commit_blocker_component_count(),
            2
        );
        assert!(drifted.has_runtime_adapter_execution_commit_blockers());
        assert_eq!(drifted.component_accounting_drift_count(), 0);
        assert_eq!(
            drifted.runtime_adapter_execution_commit_problem_component_count(),
            2
        );
        assert!(drifted.has_runtime_adapter_execution_commit_problem_components());
        assert!(drifted.runtime_adapter_execution_commit_accounting_is_consistent());
        assert!(!drifted.runtime_adapter_execution_commit_is_clean());
        assert!(!drifted.runtime_adapter_shape_is_clean());
        assert!(!drifted.can_commit_runtime_adapter_execution());
        assert_eq!(
            drifted.runtime_adapter_execution_commit_action(),
            AdapterSelectionRuntimeCommitAction::ReturnRuntimeFailure
        );
        assert!(!drifted.can_use_runtime_adapter_execution());
        assert!(!drifted.is_clean_runtime_adapter_execution());
        let drifted_commit = drifted.commit_summary();
        assert_eq!(
            drifted_commit.action,
            AdapterSelectionRuntimeCommitAction::ReturnRuntimeFailure
        );
        assert_eq!(
            drifted_commit.action,
            drifted.runtime_adapter_execution_commit_action()
        );
        assert_eq!(drifted_commit.failure_report_count, 1);
        assert!(drifted_commit.should_return_runtime_failure());
        assert_eq!(drifted_commit.total_signal_component_count, 4);
        assert_eq!(drifted_commit.total_blocker_component_count, 2);
        assert!(drifted_commit.commit_decision_accounting_is_consistent());
        assert!(
            drifted_commit
                .failure_return_summary()
                .can_return_runtime_failure()
        );
        assert!(
            drifted_commit
                .runtime_failure_return_report()
                .expect("runtime adapter drift failure return report")
                .can_use_adapter_failure_return_report()
        );
    }

    #[test]
    fn selection_runtime_summary_counts_public_fallback_reason_shape_drift() {
        let summary = AdapterSelectionRuntimeSummary {
            selection: AdapterSelection {
                adapter: RuntimeAdapter::Cuda,
                score: 0.80,
                experience_id: Some(7),
                used_fallback: false,
            },
            fallback_reason: AdapterFallbackReason::NoMatchingObservation,
            allowed_adapter_count: 2,
            matching_observation_count: 1,
            runtime_selected_adapter: Some(RuntimeAdapter::Cuda),
            runtime_adapter_reported: true,
            runtime_adapter_matches_selection: true,
            runtime_adapter_allowed: true,
        };

        assert!(summary.has_allowed_adapters());
        assert!(summary.has_matching_observations());
        assert!(summary.runtime_selection_confirmed());
        assert!(!summary.fallback_has_reason());
        assert!(!summary.fallback_without_reason());
        assert_eq!(summary.adapter_source_signal_component_count(), 2);
        assert_eq!(summary.runtime_report_signal_component_count(), 2);
        assert_eq!(summary.fallback_reason_signal_component_count(), 0);
        assert_eq!(summary.runtime_adapter_signal_component_count(), 4);
        assert_eq!(summary.fallback_reason_problem_component_count(), 1);
        assert_eq!(summary.runtime_adapter_problem_component_count(), 1);
        assert!(summary.has_runtime_adapter_problem_components());
        assert!(summary.runtime_adapter_problem());
        assert!(summary.runtime_adapter_accounting_is_consistent());
        assert_eq!(
            summary.runtime_adapter_execution_commit_signal_component_count(),
            4
        );
        assert_eq!(
            summary.runtime_adapter_execution_commit_blocker_component_count(),
            1
        );
        assert!(summary.has_runtime_adapter_execution_commit_blockers());
        assert_eq!(summary.component_accounting_drift_count(), 0);
        assert_eq!(
            summary.runtime_adapter_execution_commit_problem_component_count(),
            1
        );
        assert!(summary.has_runtime_adapter_execution_commit_problem_components());
        assert!(summary.runtime_adapter_execution_commit_accounting_is_consistent());
        assert!(!summary.runtime_adapter_execution_commit_is_clean());
        assert!(!summary.runtime_adapter_shape_is_clean());
        assert!(!summary.can_commit_runtime_adapter_execution());
        assert_eq!(
            summary.runtime_adapter_execution_commit_action(),
            AdapterSelectionRuntimeCommitAction::ReturnRuntimeFailure
        );
        assert!(!summary.can_use_runtime_adapter_execution());
        assert!(!summary.is_clean_runtime_adapter_execution());
        let commit = summary.commit_summary();
        assert_eq!(
            commit.action,
            AdapterSelectionRuntimeCommitAction::ReturnRuntimeFailure
        );
        assert_eq!(
            commit.action,
            summary.runtime_adapter_execution_commit_action()
        );
        assert_eq!(commit.failure_report_count, 1);
        assert!(commit.should_return_runtime_failure());
        assert_eq!(commit.total_signal_component_count, 4);
        assert_eq!(commit.total_blocker_component_count, 1);
        assert!(commit.commit_decision_accounting_is_consistent());
    }
}
