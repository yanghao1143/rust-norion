use crate::diagnostics::InferenceDiagnostics;
use crate::experiment::ExperimentSwitches;
use crate::kv::KvBlock;
use crate::profile::TaskProfile;
use crate::router::RouteBudget;
use crate::runtime::{RuntimeGenerationBudget, RuntimeMetadata};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InferenceRequest {
    pub prompt: String,
    pub profile: TaskProfile,
    pub prompt_tokens: usize,
    pub max_tokens: usize,
    pub runtime: RuntimeMetadata,
    pub experiments: ExperimentSwitches,
}

impl InferenceRequest {
    pub fn new(prompt: impl Into<String>, profile: TaskProfile) -> Self {
        Self {
            prompt: prompt.into(),
            profile,
            prompt_tokens: 0,
            max_tokens: 512,
            runtime: RuntimeMetadata::default(),
            experiments: ExperimentSwitches::default(),
        }
    }

    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = max_tokens.max(1);
        self
    }

    pub fn with_prompt_tokens(mut self, prompt_tokens: usize) -> Self {
        self.prompt_tokens = prompt_tokens;
        self
    }

    pub fn with_runtime(mut self, runtime: RuntimeMetadata) -> Self {
        self.runtime = runtime;
        self
    }

    pub fn with_experiments(mut self, experiments: ExperimentSwitches) -> Self {
        self.experiments = experiments;
        self
    }

    pub fn generation_budget(&self) -> RuntimeGenerationBudget {
        self.runtime
            .generation_budget(self.prompt_tokens, self.max_tokens)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GeneratedToken {
    pub text: String,
    pub logprob: Option<f32>,
    pub entropy: Option<f32>,
}

impl GeneratedToken {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            logprob: None,
            entropy: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeneratedTokenMetrics {
    pub token_count: usize,
    pub entropy_count: usize,
    pub logprob_count: usize,
    pub average_entropy: Option<f32>,
    pub average_neg_logprob: Option<f32>,
    pub uncertainty_perplexity: Option<f32>,
}

impl GeneratedTokenMetrics {
    pub fn from_tokens(tokens: &[GeneratedToken]) -> Self {
        let mut entropy_total = 0.0;
        let mut entropy_count = 0;
        let mut neg_logprob_total = 0.0;
        let mut logprob_count = 0;
        let mut loss_total = 0.0;
        let mut loss_count = 0;

        for token in tokens {
            let entropy = token.entropy.and_then(bounded_entropy);
            let neg_logprob = token.logprob.and_then(bounded_neg_logprob);

            if let Some(entropy) = entropy {
                entropy_total += entropy;
                entropy_count += 1;
            }

            if let Some(neg_logprob) = neg_logprob {
                neg_logprob_total += neg_logprob;
                logprob_count += 1;
            }

            match (entropy, neg_logprob) {
                (Some(entropy), Some(neg_logprob)) => {
                    loss_total += 2.0 + entropy * 4.0 + neg_logprob;
                    loss_count += 1;
                }
                (Some(entropy), None) => {
                    loss_total += 2.0 + entropy * 4.0;
                    loss_count += 1;
                }
                (None, Some(neg_logprob)) => {
                    loss_total += 2.0 + neg_logprob;
                    loss_count += 1;
                }
                (None, None) => {}
            }
        }

        Self {
            token_count: tokens.len(),
            entropy_count,
            logprob_count,
            average_entropy: average(entropy_total, entropy_count),
            average_neg_logprob: average(neg_logprob_total, logprob_count),
            uncertainty_perplexity: average(loss_total, loss_count),
        }
    }

    pub fn has_uncertainty_signal(self) -> bool {
        self.uncertainty_perplexity.is_some()
            || self.average_entropy.is_some()
            || self.average_neg_logprob.is_some()
    }

    pub fn has_tokens(self) -> bool {
        self.token_count > 0
    }

    pub fn has_entropy_signal(self) -> bool {
        self.entropy_count > 0
    }

    pub fn has_logprob_signal(self) -> bool {
        self.logprob_count > 0
    }

    pub fn entropy_coverage_is_complete(self) -> bool {
        self.entropy_count == self.token_count
    }

    pub fn logprob_coverage_is_complete(self) -> bool {
        self.logprob_count == self.token_count
    }

    pub fn missing_entropy_signal_component_count(self) -> usize {
        usize::from(self.has_tokens() && !self.has_entropy_signal())
    }

    pub fn partial_entropy_coverage_component_count(self) -> usize {
        usize::from(self.has_entropy_signal() && !self.entropy_coverage_is_complete())
    }

    pub fn missing_logprob_signal_component_count(self) -> usize {
        usize::from(self.has_tokens() && !self.has_logprob_signal())
    }

    pub fn partial_logprob_coverage_component_count(self) -> usize {
        usize::from(self.has_logprob_signal() && !self.logprob_coverage_is_complete())
    }

    pub fn uncertainty_coverage_signal_component_count(self) -> usize {
        self.missing_entropy_signal_component_count()
            .saturating_add(self.partial_entropy_coverage_component_count())
            .saturating_add(self.missing_logprob_signal_component_count())
            .saturating_add(self.partial_logprob_coverage_component_count())
    }

    pub fn has_uncertainty_coverage_signals(self) -> bool {
        self.uncertainty_coverage_signal_component_count() > 0
    }

    pub fn uncertainty_metric_problem_component_count(self) -> usize {
        usize::from(self.entropy_count > self.token_count)
            + usize::from(self.logprob_count > self.token_count)
            + usize::from(self.average_entropy.is_some() != self.has_entropy_signal())
            + usize::from(self.average_neg_logprob.is_some() != self.has_logprob_signal())
            + usize::from(
                self.uncertainty_perplexity.is_some()
                    != (self.has_entropy_signal() || self.has_logprob_signal()),
            )
    }

    pub fn has_uncertainty_metric_problem_components(self) -> bool {
        self.uncertainty_metric_problem_component_count() > 0
    }

    pub fn uncertainty_accounting_is_consistent(self) -> bool {
        self.uncertainty_metric_problem_component_count() == 0
            && !self.has_uncertainty_metric_problem_components()
            && self.has_uncertainty_signal()
                == (self.has_entropy_signal() || self.has_logprob_signal())
    }

    pub fn uncertainty_shape_is_clean(self) -> bool {
        !self.has_uncertainty_metric_problem_components()
            && self.uncertainty_accounting_is_consistent()
    }

    pub fn can_use_token_uncertainty_metrics(self) -> bool {
        self.has_tokens() && self.uncertainty_shape_is_clean()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct InferenceOutcome {
    pub answer: String,
    pub tokens: Vec<GeneratedToken>,
    pub route_budget: RouteBudget,
    pub diagnostics: InferenceDiagnostics,
    pub imported_kv: Vec<KvBlock>,
    pub exported_kv: Vec<KvBlock>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InferenceOutcomeSummary {
    pub answer_chars: usize,
    pub token_count: usize,
    pub has_uncertainty_signal: bool,
    pub route_attention_tokens: usize,
    pub route_fast_tokens: usize,
    pub imported_kv_blocks: usize,
    pub exported_kv_blocks: usize,
    pub diagnostics_kv_exchange_total: usize,
    pub has_runtime_execution_signal: bool,
    pub diagnostic_note_count: usize,
}

impl InferenceOutcomeSummary {
    pub fn has_answer(self) -> bool {
        self.answer_chars > 0
    }

    pub fn has_generated_tokens(self) -> bool {
        self.token_count > 0
    }

    pub fn has_kv_exchange(self) -> bool {
        self.imported_kv_blocks > 0 || self.exported_kv_blocks > 0
    }

    pub fn has_route_activity(self) -> bool {
        self.route_token_total() > 0
    }

    pub fn has_diagnostics_notes(self) -> bool {
        self.diagnostic_note_count > 0
    }

    pub fn text_without_tokens(self) -> bool {
        self.has_answer() && !self.has_generated_tokens()
    }

    pub fn tokens_without_text(self) -> bool {
        self.has_generated_tokens() && !self.has_answer()
    }

    pub fn route_token_total(self) -> usize {
        self.route_attention_tokens
            .saturating_add(self.route_fast_tokens)
    }

    pub fn kv_counts_match_diagnostics(self) -> bool {
        self.imported_kv_blocks
            .saturating_add(self.exported_kv_blocks)
            == self.diagnostics_kv_exchange_total
    }

    pub fn kv_count_drifted_from_diagnostics(self) -> bool {
        !self.kv_counts_match_diagnostics()
    }

    pub fn runtime_execution_missing(self) -> bool {
        !self.has_runtime_execution_signal
    }

    pub fn text_token_shape_problem_component_count(self) -> usize {
        usize::from(self.text_without_tokens()) + usize::from(self.tokens_without_text())
    }

    pub fn kv_diagnostics_drift_component_count(self) -> usize {
        usize::from(self.kv_count_drifted_from_diagnostics())
    }

    pub fn runtime_execution_signal_problem_component_count(self) -> usize {
        usize::from(self.runtime_execution_missing())
    }

    pub fn response_shape_problem_component_count(self) -> usize {
        self.text_token_shape_problem_component_count()
            .saturating_add(self.kv_diagnostics_drift_component_count())
            .saturating_add(self.runtime_execution_signal_problem_component_count())
    }

    pub fn has_response_shape_problem_components(self) -> bool {
        self.response_shape_problem_component_count() > 0
    }

    pub fn response_shape_accounting_is_consistent(self) -> bool {
        let expected_problem_count = self
            .text_token_shape_problem_component_count()
            .saturating_add(self.kv_diagnostics_drift_component_count())
            .saturating_add(self.runtime_execution_signal_problem_component_count());

        self.response_shape_problem_component_count() == expected_problem_count
            && self.has_response_shape_problem_components() == (expected_problem_count > 0)
            && self.has_complete_runtime_response_shape() == (expected_problem_count == 0)
    }

    pub fn has_complete_runtime_response_shape(self) -> bool {
        self.has_answer()
            && self.has_generated_tokens()
            && self.kv_counts_match_diagnostics()
            && self.has_runtime_execution_signal
    }

    pub fn response_shape_is_clean(self) -> bool {
        self.has_complete_runtime_response_shape()
            && !self.has_response_shape_problem_components()
            && self.response_shape_accounting_is_consistent()
    }

    pub fn can_use_runtime_outcome(self) -> bool {
        self.response_shape_is_clean()
    }
}

impl InferenceOutcome {
    pub fn empty() -> Self {
        Self {
            answer: String::new(),
            tokens: Vec::new(),
            route_budget: RouteBudget::default(),
            diagnostics: InferenceDiagnostics::default(),
            imported_kv: Vec::new(),
            exported_kv: Vec::new(),
        }
    }

    pub fn with_diagnostics(mut self, diagnostics: InferenceDiagnostics) -> Self {
        self.route_budget = diagnostics.route_budget;
        self.diagnostics = diagnostics;
        self
    }

    pub fn token_metrics(&self) -> GeneratedTokenMetrics {
        GeneratedTokenMetrics::from_tokens(&self.tokens)
    }

    pub fn outcome_summary(&self) -> InferenceOutcomeSummary {
        let token_metrics = self.token_metrics();

        InferenceOutcomeSummary {
            answer_chars: self.answer.trim().chars().count(),
            token_count: token_metrics.token_count,
            has_uncertainty_signal: token_metrics.has_uncertainty_signal(),
            route_attention_tokens: self.route_budget.attention_tokens,
            route_fast_tokens: self.route_budget.fast_tokens,
            imported_kv_blocks: self.imported_kv.len(),
            exported_kv_blocks: self.exported_kv.len(),
            diagnostics_kv_exchange_total: self.diagnostics.kv_exchange_total(),
            has_runtime_execution_signal: self.diagnostics.has_runtime_execution_signal(),
            diagnostic_note_count: self.diagnostics.notes.len(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InferenceError {
    pub message: String,
}

impl InferenceError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn from_failure(failure: RuntimeFailureReport) -> Self {
        Self::new(failure.backend_message())
    }
}

impl std::fmt::Display for InferenceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for InferenceError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeFailureKind {
    Runtime,
    KvImport,
    KvExport,
    ContractViolation,
    ContextExhausted,
    Unknown,
}

impl RuntimeFailureKind {
    pub fn trace_label(self) -> &'static str {
        match self {
            Self::Runtime => "runtime_error",
            Self::KvImport => "runtime_kv_import_error",
            Self::KvExport => "runtime_kv_export_error",
            Self::ContractViolation => "runtime_contract_violation",
            Self::ContextExhausted => "runtime_context_exhausted",
            Self::Unknown => "runtime_unknown_error",
        }
    }

    pub fn is_recoverable(self) -> bool {
        matches!(self, Self::KvExport | Self::ContractViolation)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeFailureReport {
    pub kind: RuntimeFailureKind,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeFailureSummary {
    pub kind: RuntimeFailureKind,
    pub trace_label: &'static str,
    pub message_len: usize,
    pub recoverable: bool,
    pub backend_error: bool,
    pub diagnostics_note_present: bool,
    pub diagnostics_note_has_trace_label: bool,
    pub trace_confidence: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeFailureBatchSummary {
    pub total_count: usize,
    pub runtime_count: usize,
    pub kv_import_count: usize,
    pub kv_export_count: usize,
    pub contract_violation_count: usize,
    pub context_exhausted_count: usize,
    pub unknown_count: usize,
    pub recoverable_count: usize,
    pub backend_error_count: usize,
    pub diagnostics_note_count: usize,
    pub min_trace_confidence: Option<f32>,
}

impl RuntimeFailureReport {
    pub fn new(kind: RuntimeFailureKind, message: impl Into<String>) -> Self {
        let message = message.into();
        let message = if message.trim().is_empty() {
            "runtime failure".to_owned()
        } else {
            message.trim().to_owned()
        };

        Self { kind, message }
    }

    pub fn runtime(message: impl Into<String>) -> Self {
        Self::new(RuntimeFailureKind::Runtime, message)
    }

    pub fn kv_import(message: impl Into<String>) -> Self {
        Self::new(RuntimeFailureKind::KvImport, message)
    }

    pub fn kv_export(message: impl Into<String>) -> Self {
        Self::new(RuntimeFailureKind::KvExport, message)
    }

    pub fn contract_violation(message: impl Into<String>) -> Self {
        Self::new(RuntimeFailureKind::ContractViolation, message)
    }

    pub fn context_exhausted(budget: RuntimeGenerationBudget) -> Self {
        Self::new(
            RuntimeFailureKind::ContextExhausted,
            format!(
                "prompt_tokens={} leaves no generation room in native_context_window={}",
                budget.prompt_tokens,
                budget
                    .remaining_context_tokens
                    .map(|remaining| budget.prompt_tokens.saturating_add(remaining))
                    .unwrap_or(0)
            ),
        )
    }

    pub fn backend_message(&self) -> String {
        match self.kind {
            RuntimeFailureKind::Runtime
            | RuntimeFailureKind::KvImport
            | RuntimeFailureKind::ContextExhausted
            | RuntimeFailureKind::Unknown => {
                format!("Runtime backend error: {}", self.message)
            }
            RuntimeFailureKind::KvExport | RuntimeFailureKind::ContractViolation => {
                self.message.clone()
            }
        }
    }

    pub fn diagnostics_note(&self) -> String {
        format!("{}:{}", self.kind.trace_label(), self.message)
    }

    pub fn trace_confidence(&self) -> f32 {
        match self.kind {
            RuntimeFailureKind::Runtime
            | RuntimeFailureKind::KvImport
            | RuntimeFailureKind::ContextExhausted
            | RuntimeFailureKind::Unknown => 0.0,
            RuntimeFailureKind::KvExport => 0.22,
            RuntimeFailureKind::ContractViolation => 0.05,
        }
    }

    pub fn failure_summary(&self) -> RuntimeFailureSummary {
        let diagnostics_note = self.diagnostics_note();

        RuntimeFailureSummary {
            kind: self.kind,
            trace_label: self.kind.trace_label(),
            message_len: self.message.trim().chars().count(),
            recoverable: self.is_recoverable(),
            backend_error: self.backend_message().starts_with("Runtime backend error:"),
            diagnostics_note_present: !diagnostics_note.trim().is_empty(),
            diagnostics_note_has_trace_label: diagnostics_note.starts_with(self.kind.trace_label()),
            trace_confidence: self.trace_confidence(),
        }
    }

    pub fn is_recoverable(&self) -> bool {
        self.kind.is_recoverable()
    }

    pub fn batch_summary(reports: &[RuntimeFailureReport]) -> RuntimeFailureBatchSummary {
        RuntimeFailureBatchSummary::from_reports(reports)
    }
}

impl RuntimeFailureSummary {
    pub fn has_message(self) -> bool {
        self.message_len > 0
    }

    pub fn trace_label_matches_kind(self) -> bool {
        self.trace_label == self.kind.trace_label()
    }

    pub fn recoverable_matches_kind(self) -> bool {
        self.recoverable == self.kind.is_recoverable()
    }

    pub fn backend_error_matches_kind(self) -> bool {
        self.backend_error
            == matches!(
                self.kind,
                RuntimeFailureKind::Runtime
                    | RuntimeFailureKind::KvImport
                    | RuntimeFailureKind::ContextExhausted
                    | RuntimeFailureKind::Unknown
            )
    }

    pub fn trace_confidence_is_valid(self) -> bool {
        self.trace_confidence.is_finite() && (0.0..=1.0).contains(&self.trace_confidence)
    }

    pub fn failure_summary_signal_component_count(self) -> usize {
        usize::from(self.has_message())
            .saturating_add(usize::from(self.trace_label_matches_kind()))
            .saturating_add(usize::from(self.recoverable_matches_kind()))
            .saturating_add(usize::from(self.backend_error_matches_kind()))
            .saturating_add(usize::from(self.diagnostics_note_present))
            .saturating_add(usize::from(self.diagnostics_note_has_trace_label))
            .saturating_add(usize::from(self.trace_confidence_is_valid()))
    }

    pub fn has_failure_summary_signals(self) -> bool {
        self.failure_summary_signal_component_count() > 0
    }

    pub fn failure_summary_problem_component_count(self) -> usize {
        usize::from(!self.has_message())
            .saturating_add(usize::from(!self.trace_label_matches_kind()))
            .saturating_add(usize::from(!self.recoverable_matches_kind()))
            .saturating_add(usize::from(!self.backend_error_matches_kind()))
            .saturating_add(usize::from(!self.diagnostics_note_present))
            .saturating_add(usize::from(!self.diagnostics_note_has_trace_label))
            .saturating_add(usize::from(!self.trace_confidence_is_valid()))
    }

    pub fn has_failure_summary_problem_components(self) -> bool {
        self.failure_summary_problem_component_count() > 0
    }

    pub fn failure_summary_accounting_is_consistent(self) -> bool {
        let expected_signal_count = usize::from(self.has_message())
            .saturating_add(usize::from(self.trace_label_matches_kind()))
            .saturating_add(usize::from(self.recoverable_matches_kind()))
            .saturating_add(usize::from(self.backend_error_matches_kind()))
            .saturating_add(usize::from(self.diagnostics_note_present))
            .saturating_add(usize::from(self.diagnostics_note_has_trace_label))
            .saturating_add(usize::from(self.trace_confidence_is_valid()));
        let expected_problem_count = usize::from(!self.has_message())
            .saturating_add(usize::from(!self.trace_label_matches_kind()))
            .saturating_add(usize::from(!self.recoverable_matches_kind()))
            .saturating_add(usize::from(!self.backend_error_matches_kind()))
            .saturating_add(usize::from(!self.diagnostics_note_present))
            .saturating_add(usize::from(!self.diagnostics_note_has_trace_label))
            .saturating_add(usize::from(!self.trace_confidence_is_valid()));

        self.failure_summary_signal_component_count() == expected_signal_count
            && self.has_failure_summary_signals() == (expected_signal_count > 0)
            && self.failure_summary_problem_component_count() == expected_problem_count
            && self.has_failure_summary_problem_components() == (expected_problem_count > 0)
    }

    pub fn failure_summary_shape_is_clean(self) -> bool {
        !self.has_failure_summary_problem_components()
            && self.failure_summary_accounting_is_consistent()
    }

    pub fn can_use_runtime_failure_report(self) -> bool {
        self.failure_summary_shape_is_clean()
    }
}

impl RuntimeFailureBatchSummary {
    pub fn from_reports(reports: &[RuntimeFailureReport]) -> Self {
        let mut summary = Self {
            total_count: reports.len(),
            runtime_count: 0,
            kv_import_count: 0,
            kv_export_count: 0,
            contract_violation_count: 0,
            context_exhausted_count: 0,
            unknown_count: 0,
            recoverable_count: 0,
            backend_error_count: 0,
            diagnostics_note_count: 0,
            min_trace_confidence: None,
        };

        for report in reports {
            match report.kind {
                RuntimeFailureKind::Runtime => summary.runtime_count += 1,
                RuntimeFailureKind::KvImport => summary.kv_import_count += 1,
                RuntimeFailureKind::KvExport => summary.kv_export_count += 1,
                RuntimeFailureKind::ContractViolation => summary.contract_violation_count += 1,
                RuntimeFailureKind::ContextExhausted => summary.context_exhausted_count += 1,
                RuntimeFailureKind::Unknown => summary.unknown_count += 1,
            }
            if report.is_recoverable() {
                summary.recoverable_count += 1;
            }
            if report
                .backend_message()
                .starts_with("Runtime backend error:")
            {
                summary.backend_error_count += 1;
            }
            if !report.diagnostics_note().trim().is_empty() {
                summary.diagnostics_note_count += 1;
            }
            let confidence = report.trace_confidence();
            summary.min_trace_confidence = Some(
                summary
                    .min_trace_confidence
                    .map(|current| current.min(confidence))
                    .unwrap_or(confidence),
            );
        }

        summary
    }

    pub fn has_failures(self) -> bool {
        self.total_count > 0
    }

    pub fn failure_class_total(self) -> usize {
        self.runtime_count
            .saturating_add(self.kv_import_count)
            .saturating_add(self.kv_export_count)
            .saturating_add(self.contract_violation_count)
            .saturating_add(self.context_exhausted_count)
            .saturating_add(self.unknown_count)
    }

    pub fn failure_counts_match_total(self) -> bool {
        self.failure_class_total() == self.total_count
    }

    pub fn non_recoverable_count(self) -> usize {
        self.total_count.saturating_sub(self.recoverable_count)
    }

    pub fn recovery_counts_are_bounded(self) -> bool {
        self.recoverable_count <= self.total_count
    }

    pub fn backend_error_count_is_bounded(self) -> bool {
        self.backend_error_count <= self.total_count
    }

    pub fn trace_confidence_is_reported(self) -> bool {
        self.min_trace_confidence.is_some()
    }

    pub fn trace_confidence_is_valid(self) -> bool {
        self.min_trace_confidence
            .map(|confidence| confidence.is_finite() && (0.0..=1.0).contains(&confidence))
            .unwrap_or(true)
    }

    pub fn has_runtime_failures(self) -> bool {
        self.runtime_count > 0 || self.unknown_count > 0
    }

    pub fn has_kv_failures(self) -> bool {
        self.kv_import_count > 0 || self.kv_export_count > 0
    }

    pub fn has_contract_failures(self) -> bool {
        self.contract_violation_count > 0 || self.context_exhausted_count > 0
    }

    pub fn has_recoverable_failures(self) -> bool {
        self.recoverable_count > 0
    }

    pub fn has_backend_errors(self) -> bool {
        self.backend_error_count > 0
    }

    pub fn all_failures_have_diagnostics_notes(self) -> bool {
        self.diagnostics_note_count == self.total_count
    }

    pub fn diagnostics_notes_match_failures(self) -> bool {
        self.all_failures_have_diagnostics_notes()
    }

    pub fn has_zero_trace_confidence(self) -> bool {
        self.min_trace_confidence == Some(0.0)
    }

    pub fn all_failures_are_recoverable(self) -> bool {
        self.total_count > 0 && self.recoverable_count == self.total_count
    }

    pub fn failure_batch_problem_component_count(self) -> usize {
        usize::from(!self.failure_counts_match_total())
            + usize::from(!self.recovery_counts_are_bounded())
            + usize::from(!self.backend_error_count_is_bounded())
            + usize::from(!self.diagnostics_notes_match_failures())
            + usize::from(self.trace_confidence_is_reported() != self.has_failures())
            + usize::from(!self.trace_confidence_is_valid())
    }

    pub fn has_failure_batch_problem_components(self) -> bool {
        self.failure_batch_problem_component_count() > 0
    }

    pub fn failure_batch_accounting_is_consistent(self) -> bool {
        self.failure_batch_problem_component_count() == 0
    }

    pub fn failure_batch_shape_is_clean(self) -> bool {
        !self.has_failure_batch_problem_components()
            && self.failure_batch_accounting_is_consistent()
    }

    pub fn can_format_runtime_failures(self) -> bool {
        self.has_failures() && self.failure_batch_shape_is_clean()
    }
}

pub trait InferenceEngine {
    fn infer(&mut self, request: InferenceRequest) -> Result<InferenceOutcome, InferenceError>;

    fn infer_stream(
        &mut self,
        request: InferenceRequest,
        on_token: &mut dyn FnMut(&GeneratedToken),
    ) -> Result<InferenceOutcome, InferenceError> {
        let outcome = self.infer(request)?;
        for token in &outcome.tokens {
            on_token(token);
        }
        Ok(outcome)
    }
}

fn bounded_entropy(value: f32) -> Option<f32> {
    value.is_finite().then(|| value.clamp(0.0, 4.0))
}

fn bounded_neg_logprob(value: f32) -> Option<f32> {
    let value = -value;
    value.is_finite().then(|| value.clamp(0.0, 12.0))
}

fn average(total: f32, count: usize) -> Option<f32> {
    if count == 0 {
        None
    } else {
        Some(total / count as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::{InferenceDiagnostics, RuntimeDiagnostics};
    use crate::kv::KvNamespace;

    #[test]
    fn inference_request_clamps_max_tokens() {
        let request = InferenceRequest::new("hello", TaskProfile::General).with_max_tokens(0);

        assert_eq!(request.max_tokens, 1);
    }

    #[test]
    fn inference_request_exposes_runtime_generation_budget() {
        let runtime = RuntimeMetadata::new("ctx", "tok", 64, 128);
        let request = InferenceRequest::new("hello", TaskProfile::General)
            .with_prompt_tokens(60)
            .with_max_tokens(16)
            .with_runtime(runtime);

        let budget = request.generation_budget();

        assert_eq!(budget.max_generated_tokens, 4);
        assert_eq!(budget.planned_context_tokens, 64);
        assert!(budget.truncated_by_context);
    }

    #[test]
    fn inference_outcome_tracks_diagnostics_route_budget() {
        let diagnostics = InferenceDiagnostics::new(RouteBudget {
            threshold: 0.44,
            attention_tokens: 6,
            fast_tokens: 2,
            attention_fraction: 0.75,
        });

        let outcome = InferenceOutcome::empty().with_diagnostics(diagnostics);

        assert_eq!(outcome.route_budget.threshold, 0.44);
        assert_eq!(outcome.diagnostics.route_budget.attention_tokens, 6);
    }

    #[test]
    fn generated_token_metrics_summarize_uncertainty_signals() {
        let tokens = vec![
            GeneratedToken {
                text: "hello".to_owned(),
                logprob: Some(-0.5),
                entropy: Some(0.25),
            },
            GeneratedToken {
                text: "world".to_owned(),
                logprob: Some(-1.5),
                entropy: Some(0.75),
            },
            GeneratedToken::new("!"),
        ];

        let metrics = GeneratedTokenMetrics::from_tokens(&tokens);

        assert_eq!(metrics.token_count, 3);
        assert_eq!(metrics.entropy_count, 2);
        assert_eq!(metrics.logprob_count, 2);
        assert!((metrics.average_entropy.unwrap() - 0.5).abs() < 0.0001);
        assert!((metrics.average_neg_logprob.unwrap() - 1.0).abs() < 0.0001);
        assert!((metrics.uncertainty_perplexity.unwrap() - 5.0).abs() < 0.0001);
        assert!(metrics.has_uncertainty_signal());
        assert!(metrics.has_tokens());
        assert!(metrics.has_entropy_signal());
        assert!(metrics.has_logprob_signal());
        assert!(!metrics.entropy_coverage_is_complete());
        assert!(!metrics.logprob_coverage_is_complete());
        assert_eq!(metrics.missing_entropy_signal_component_count(), 0);
        assert_eq!(metrics.partial_entropy_coverage_component_count(), 1);
        assert_eq!(metrics.missing_logprob_signal_component_count(), 0);
        assert_eq!(metrics.partial_logprob_coverage_component_count(), 1);
        assert_eq!(metrics.uncertainty_coverage_signal_component_count(), 2);
        assert!(metrics.has_uncertainty_coverage_signals());
        assert_eq!(metrics.uncertainty_metric_problem_component_count(), 0);
        assert!(!metrics.has_uncertainty_metric_problem_components());
        assert!(metrics.uncertainty_accounting_is_consistent());
    }

    #[test]
    fn generated_token_metrics_clamp_non_finite_and_out_of_range_values() {
        let tokens = vec![
            GeneratedToken {
                text: "wide".to_owned(),
                logprob: Some(-20.0),
                entropy: Some(99.0),
            },
            GeneratedToken {
                text: "nan".to_owned(),
                logprob: Some(f32::NAN),
                entropy: Some(f32::INFINITY),
            },
        ];

        let metrics = GeneratedTokenMetrics::from_tokens(&tokens);

        assert_eq!(metrics.token_count, 2);
        assert_eq!(metrics.entropy_count, 1);
        assert_eq!(metrics.logprob_count, 1);
        assert_eq!(metrics.average_entropy, Some(4.0));
        assert_eq!(metrics.average_neg_logprob, Some(12.0));
        assert_eq!(metrics.uncertainty_perplexity, Some(30.0));
        assert!(metrics.uncertainty_accounting_is_consistent());
    }

    #[test]
    fn inference_outcome_exposes_generated_token_metrics() {
        let mut outcome = InferenceOutcome::empty();
        outcome.tokens.push(GeneratedToken {
            text: "token".to_owned(),
            logprob: Some(-0.25),
            entropy: None,
        });

        let metrics = outcome.token_metrics();

        assert_eq!(metrics.token_count, 1);
        assert_eq!(metrics.logprob_count, 1);
        assert_eq!(metrics.entropy_count, 0);
        assert!(metrics.has_uncertainty_signal());
        assert!(metrics.has_tokens());
        assert!(!metrics.has_entropy_signal());
        assert!(metrics.has_logprob_signal());
        assert!(!metrics.entropy_coverage_is_complete());
        assert!(metrics.logprob_coverage_is_complete());
        assert_eq!(metrics.missing_entropy_signal_component_count(), 1);
        assert_eq!(metrics.partial_entropy_coverage_component_count(), 0);
        assert_eq!(metrics.missing_logprob_signal_component_count(), 0);
        assert_eq!(metrics.partial_logprob_coverage_component_count(), 0);
        assert_eq!(metrics.uncertainty_coverage_signal_component_count(), 1);
        assert!(metrics.has_uncertainty_coverage_signals());
        assert_eq!(metrics.uncertainty_metric_problem_component_count(), 0);
        assert!(metrics.uncertainty_accounting_is_consistent());
        assert!(metrics.uncertainty_shape_is_clean());
        assert!(metrics.can_use_token_uncertainty_metrics());
    }

    #[test]
    fn generated_token_metrics_reports_uncertainty_accounting_drift() {
        let no_signal = GeneratedTokenMetrics::from_tokens(&[GeneratedToken::new("plain")]);
        let drifted = GeneratedTokenMetrics {
            token_count: 1,
            entropy_count: 2,
            logprob_count: 0,
            average_entropy: None,
            average_neg_logprob: Some(0.5),
            uncertainty_perplexity: None,
        };

        assert!(no_signal.has_tokens());
        assert!(!no_signal.has_uncertainty_signal());
        assert_eq!(no_signal.missing_entropy_signal_component_count(), 1);
        assert_eq!(no_signal.missing_logprob_signal_component_count(), 1);
        assert_eq!(no_signal.uncertainty_coverage_signal_component_count(), 2);
        assert!(no_signal.has_uncertainty_coverage_signals());
        assert_eq!(no_signal.uncertainty_metric_problem_component_count(), 0);
        assert!(no_signal.uncertainty_accounting_is_consistent());
        assert!(no_signal.uncertainty_shape_is_clean());
        assert!(no_signal.can_use_token_uncertainty_metrics());

        assert!(drifted.has_entropy_signal());
        assert!(!drifted.has_logprob_signal());
        assert!(!drifted.entropy_coverage_is_complete());
        assert_eq!(drifted.missing_logprob_signal_component_count(), 1);
        assert_eq!(drifted.uncertainty_coverage_signal_component_count(), 2);
        assert_eq!(drifted.uncertainty_metric_problem_component_count(), 4);
        assert!(drifted.has_uncertainty_metric_problem_components());
        assert!(!drifted.uncertainty_accounting_is_consistent());
        assert!(!drifted.uncertainty_shape_is_clean());
        assert!(!drifted.can_use_token_uncertainty_metrics());
    }

    #[test]
    fn inference_outcome_summary_reports_response_boundary_shape() {
        let runtime = RuntimeDiagnostics::empty().with_kv_exchange(1, 1);
        let mut diagnostics = InferenceDiagnostics::new(RouteBudget {
            threshold: 0.5,
            attention_tokens: 4,
            fast_tokens: 2,
            attention_fraction: 0.66,
        })
        .with_runtime(runtime);
        diagnostics.push_note("trace:ok");
        let mut outcome = InferenceOutcome::empty().with_diagnostics(diagnostics);
        outcome.answer = " answer ".to_owned();
        outcome.tokens.push(GeneratedToken {
            text: "answer".to_owned(),
            logprob: Some(-0.5),
            entropy: Some(0.25),
        });
        outcome.imported_kv.push(KvBlock::new(
            1,
            KvNamespace::Runtime,
            0,
            0,
            0..1,
            vec![0.1],
            vec![0.2],
        ));
        outcome.exported_kv.push(KvBlock::new(
            2,
            KvNamespace::Runtime,
            0,
            0,
            0..1,
            vec![0.3],
            vec![0.4],
        ));

        let summary = outcome.outcome_summary();

        assert_eq!(summary.answer_chars, 6);
        assert_eq!(summary.token_count, 1);
        assert!(summary.has_answer());
        assert!(summary.has_generated_tokens());
        assert!(summary.has_uncertainty_signal);
        assert!(!summary.text_without_tokens());
        assert!(!summary.tokens_without_text());
        assert_eq!(summary.route_attention_tokens, 4);
        assert_eq!(summary.route_fast_tokens, 2);
        assert_eq!(summary.route_token_total(), 6);
        assert!(summary.has_route_activity());
        assert_eq!(summary.imported_kv_blocks, 1);
        assert_eq!(summary.exported_kv_blocks, 1);
        assert!(summary.has_kv_exchange());
        assert_eq!(summary.diagnostics_kv_exchange_total, 2);
        assert!(summary.kv_counts_match_diagnostics());
        assert!(!summary.kv_count_drifted_from_diagnostics());
        assert_eq!(summary.text_token_shape_problem_component_count(), 0);
        assert_eq!(summary.kv_diagnostics_drift_component_count(), 0);
        assert_eq!(
            summary.runtime_execution_signal_problem_component_count(),
            0
        );
        assert_eq!(summary.response_shape_problem_component_count(), 0);
        assert!(!summary.has_response_shape_problem_components());
        assert!(summary.response_shape_accounting_is_consistent());
        assert!(summary.has_runtime_execution_signal);
        assert!(!summary.runtime_execution_missing());
        assert!(summary.has_complete_runtime_response_shape());
        assert!(summary.response_shape_is_clean());
        assert!(summary.can_use_runtime_outcome());
        assert_eq!(summary.diagnostic_note_count, 1);
        assert!(summary.has_diagnostics_notes());
    }

    #[test]
    fn inference_outcome_summary_marks_empty_outputs() {
        let summary = InferenceOutcome::empty().outcome_summary();

        assert_eq!(summary.answer_chars, 0);
        assert_eq!(summary.token_count, 0);
        assert!(!summary.has_answer());
        assert!(!summary.has_generated_tokens());
        assert!(!summary.has_uncertainty_signal);
        assert_eq!(summary.route_token_total(), 0);
        assert!(!summary.has_route_activity());
        assert!(!summary.has_kv_exchange());
        assert!(summary.kv_counts_match_diagnostics());
        assert!(!summary.kv_count_drifted_from_diagnostics());
        assert_eq!(summary.text_token_shape_problem_component_count(), 0);
        assert_eq!(summary.kv_diagnostics_drift_component_count(), 0);
        assert_eq!(
            summary.runtime_execution_signal_problem_component_count(),
            1
        );
        assert_eq!(summary.response_shape_problem_component_count(), 1);
        assert!(summary.has_response_shape_problem_components());
        assert!(summary.response_shape_accounting_is_consistent());
        assert!(!summary.has_runtime_execution_signal);
        assert!(summary.runtime_execution_missing());
        assert!(!summary.has_complete_runtime_response_shape());
        assert!(!summary.response_shape_is_clean());
        assert!(!summary.can_use_runtime_outcome());
        assert_eq!(summary.diagnostic_note_count, 0);
        assert!(!summary.has_diagnostics_notes());
    }

    #[test]
    fn inference_outcome_summary_counts_response_shape_problems() {
        let text_only = InferenceOutcomeSummary {
            answer_chars: 5,
            token_count: 0,
            has_uncertainty_signal: false,
            route_attention_tokens: 0,
            route_fast_tokens: 0,
            imported_kv_blocks: 2,
            exported_kv_blocks: 1,
            diagnostics_kv_exchange_total: 1,
            has_runtime_execution_signal: false,
            diagnostic_note_count: 0,
        };
        let token_only = InferenceOutcomeSummary {
            answer_chars: 0,
            token_count: 2,
            has_uncertainty_signal: false,
            route_attention_tokens: 0,
            route_fast_tokens: 0,
            imported_kv_blocks: 0,
            exported_kv_blocks: 0,
            diagnostics_kv_exchange_total: 0,
            has_runtime_execution_signal: true,
            diagnostic_note_count: 0,
        };

        assert!(text_only.text_without_tokens());
        assert!(!text_only.tokens_without_text());
        assert_eq!(text_only.text_token_shape_problem_component_count(), 1);
        assert_eq!(text_only.kv_diagnostics_drift_component_count(), 1);
        assert_eq!(
            text_only.runtime_execution_signal_problem_component_count(),
            1
        );
        assert_eq!(text_only.response_shape_problem_component_count(), 3);
        assert!(text_only.has_response_shape_problem_components());
        assert!(text_only.response_shape_accounting_is_consistent());
        assert!(!text_only.has_complete_runtime_response_shape());
        assert!(!text_only.response_shape_is_clean());
        assert!(!text_only.can_use_runtime_outcome());

        assert!(!token_only.text_without_tokens());
        assert!(token_only.tokens_without_text());
        assert_eq!(token_only.text_token_shape_problem_component_count(), 1);
        assert_eq!(token_only.kv_diagnostics_drift_component_count(), 0);
        assert_eq!(
            token_only.runtime_execution_signal_problem_component_count(),
            0
        );
        assert_eq!(token_only.response_shape_problem_component_count(), 1);
        assert!(token_only.has_response_shape_problem_components());
        assert!(token_only.response_shape_accounting_is_consistent());
        assert!(!token_only.has_complete_runtime_response_shape());
        assert!(!token_only.response_shape_is_clean());
        assert!(!token_only.can_use_runtime_outcome());
    }

    #[test]
    fn inference_outcome_summary_blocks_public_completion_drift() {
        let summary = InferenceOutcomeSummary {
            answer_chars: 0,
            token_count: 0,
            has_uncertainty_signal: false,
            route_attention_tokens: 0,
            route_fast_tokens: 0,
            imported_kv_blocks: 0,
            exported_kv_blocks: 0,
            diagnostics_kv_exchange_total: 0,
            has_runtime_execution_signal: true,
            diagnostic_note_count: 0,
        };

        assert!(!summary.has_answer());
        assert!(!summary.has_generated_tokens());
        assert!(summary.kv_counts_match_diagnostics());
        assert!(summary.has_runtime_execution_signal);
        assert_eq!(summary.response_shape_problem_component_count(), 0);
        assert!(!summary.has_response_shape_problem_components());
        assert!(!summary.has_complete_runtime_response_shape());
        assert!(!summary.response_shape_accounting_is_consistent());
        assert!(!summary.response_shape_is_clean());
        assert!(!summary.can_use_runtime_outcome());
    }

    #[test]
    fn runtime_failure_report_formats_backend_errors() {
        let failure = RuntimeFailureReport::kv_import("import failed");
        let error = InferenceError::from_failure(failure.clone());
        let summary = failure.failure_summary();

        assert_eq!(failure.kind.trace_label(), "runtime_kv_import_error");
        assert_eq!(
            failure.backend_message(),
            "Runtime backend error: import failed"
        );
        assert_eq!(
            failure.diagnostics_note(),
            "runtime_kv_import_error:import failed"
        );
        assert_eq!(failure.trace_confidence(), 0.0);
        assert!(!failure.is_recoverable());
        assert_eq!(error.message, "Runtime backend error: import failed");
        assert_eq!(summary.kind, RuntimeFailureKind::KvImport);
        assert_eq!(summary.trace_label, "runtime_kv_import_error");
        assert_eq!(summary.message_len, "import failed".len());
        assert!(summary.has_message());
        assert!(!summary.recoverable);
        assert!(summary.recoverable_matches_kind());
        assert!(summary.backend_error);
        assert!(summary.backend_error_matches_kind());
        assert!(summary.diagnostics_note_present);
        assert!(summary.diagnostics_note_has_trace_label);
        assert!(summary.trace_confidence_is_valid());
        assert!(summary.trace_label_matches_kind());
        assert_eq!(summary.failure_summary_signal_component_count(), 7);
        assert!(!summary.has_failure_summary_problem_components());
        assert_eq!(summary.failure_summary_problem_component_count(), 0);
        assert!(summary.failure_summary_accounting_is_consistent());
        assert!(summary.failure_summary_shape_is_clean());
        assert!(summary.can_use_runtime_failure_report());
    }

    #[test]
    fn runtime_failure_report_marks_recoverable_export_and_contract_failures() {
        let export = RuntimeFailureReport::kv_export("export skipped");
        let contract = RuntimeFailureReport::contract_violation("bad adapter");
        let export_summary = export.failure_summary();
        let contract_summary = contract.failure_summary();

        assert_eq!(export.backend_message(), "export skipped");
        assert_eq!(export.trace_confidence(), 0.22);
        assert!(export.is_recoverable());
        assert_eq!(export_summary.kind, RuntimeFailureKind::KvExport);
        assert_eq!(export_summary.trace_label, "runtime_kv_export_error");
        assert!(export_summary.recoverable);
        assert!(!export_summary.backend_error);
        assert!(export_summary.backend_error_matches_kind());
        assert!(export_summary.diagnostics_note_has_trace_label);
        assert!(export_summary.failure_summary_shape_is_clean());
        assert!(export_summary.can_use_runtime_failure_report());
        assert_eq!(
            contract.diagnostics_note(),
            "runtime_contract_violation:bad adapter"
        );
        assert_eq!(contract.trace_confidence(), 0.05);
        assert!(contract.is_recoverable());
        assert_eq!(
            contract_summary.trace_label,
            RuntimeFailureKind::ContractViolation.trace_label()
        );
        assert!(contract_summary.recoverable_matches_kind());
        assert!(!contract_summary.backend_error);
        assert!(contract_summary.trace_confidence_is_valid());
        assert!(contract_summary.failure_summary_shape_is_clean());
    }

    #[test]
    fn runtime_failure_summary_counts_public_shape_drift() {
        let summary = RuntimeFailureSummary {
            kind: RuntimeFailureKind::KvExport,
            trace_label: "runtime_kv_import_error",
            message_len: 0,
            recoverable: false,
            backend_error: true,
            diagnostics_note_present: true,
            diagnostics_note_has_trace_label: false,
            trace_confidence: 1.5,
        };

        assert!(!summary.has_message());
        assert!(!summary.trace_label_matches_kind());
        assert!(!summary.recoverable_matches_kind());
        assert!(!summary.backend_error_matches_kind());
        assert!(!summary.trace_confidence_is_valid());
        assert_eq!(summary.failure_summary_signal_component_count(), 1);
        assert!(summary.has_failure_summary_signals());
        assert_eq!(summary.failure_summary_problem_component_count(), 6);
        assert!(summary.has_failure_summary_problem_components());
        assert!(summary.failure_summary_accounting_is_consistent());
        assert!(!summary.failure_summary_shape_is_clean());
        assert!(!summary.can_use_runtime_failure_report());
    }

    #[test]
    fn runtime_failure_batch_summary_counts_failure_classes() {
        let reports = vec![
            RuntimeFailureReport::runtime("backend timeout"),
            RuntimeFailureReport::kv_import("bad import"),
            RuntimeFailureReport::kv_export("export skipped"),
            RuntimeFailureReport::contract_violation("bad adapter"),
            RuntimeFailureReport::context_exhausted(RuntimeGenerationBudget::new(128, 8, 128)),
            RuntimeFailureReport::new(RuntimeFailureKind::Unknown, "mystery"),
        ];

        let summary = RuntimeFailureReport::batch_summary(&reports);

        assert_eq!(summary, RuntimeFailureBatchSummary::from_reports(&reports));
        assert_eq!(summary.total_count, 6);
        assert_eq!(summary.runtime_count, 1);
        assert_eq!(summary.kv_import_count, 1);
        assert_eq!(summary.kv_export_count, 1);
        assert_eq!(summary.contract_violation_count, 1);
        assert_eq!(summary.context_exhausted_count, 1);
        assert_eq!(summary.unknown_count, 1);
        assert_eq!(summary.recoverable_count, 2);
        assert_eq!(summary.backend_error_count, 4);
        assert_eq!(summary.diagnostics_note_count, 6);
        assert_eq!(summary.min_trace_confidence, Some(0.0));
        assert!(summary.has_failures());
        assert_eq!(summary.failure_class_total(), 6);
        assert!(summary.failure_counts_match_total());
        assert_eq!(summary.non_recoverable_count(), 4);
        assert!(summary.recovery_counts_are_bounded());
        assert!(summary.backend_error_count_is_bounded());
        assert!(summary.trace_confidence_is_reported());
        assert!(summary.trace_confidence_is_valid());
        assert!(summary.has_runtime_failures());
        assert!(summary.has_kv_failures());
        assert!(summary.has_contract_failures());
        assert!(summary.has_recoverable_failures());
        assert!(summary.has_backend_errors());
        assert!(summary.all_failures_have_diagnostics_notes());
        assert!(summary.diagnostics_notes_match_failures());
        assert!(summary.has_zero_trace_confidence());
        assert!(!summary.all_failures_are_recoverable());
        assert_eq!(summary.failure_batch_problem_component_count(), 0);
        assert!(!summary.has_failure_batch_problem_components());
        assert!(summary.failure_batch_accounting_is_consistent());
        assert!(summary.failure_batch_shape_is_clean());
        assert!(summary.can_format_runtime_failures());
    }

    #[test]
    fn runtime_failure_batch_summary_marks_empty_batches() {
        let summary = RuntimeFailureBatchSummary::from_reports(&[]);

        assert_eq!(summary.total_count, 0);
        assert_eq!(summary.recoverable_count, 0);
        assert_eq!(summary.backend_error_count, 0);
        assert_eq!(summary.diagnostics_note_count, 0);
        assert_eq!(summary.min_trace_confidence, None);
        assert!(!summary.has_failures());
        assert_eq!(summary.failure_class_total(), 0);
        assert!(summary.failure_counts_match_total());
        assert_eq!(summary.non_recoverable_count(), 0);
        assert!(summary.recovery_counts_are_bounded());
        assert!(summary.backend_error_count_is_bounded());
        assert!(!summary.trace_confidence_is_reported());
        assert!(summary.trace_confidence_is_valid());
        assert!(!summary.has_runtime_failures());
        assert!(!summary.has_kv_failures());
        assert!(!summary.has_contract_failures());
        assert!(!summary.has_recoverable_failures());
        assert!(!summary.has_backend_errors());
        assert!(summary.all_failures_have_diagnostics_notes());
        assert!(summary.diagnostics_notes_match_failures());
        assert!(!summary.has_zero_trace_confidence());
        assert!(!summary.all_failures_are_recoverable());
        assert_eq!(summary.failure_batch_problem_component_count(), 0);
        assert!(!summary.has_failure_batch_problem_components());
        assert!(summary.failure_batch_accounting_is_consistent());
        assert!(summary.failure_batch_shape_is_clean());
        assert!(!summary.can_format_runtime_failures());
    }

    #[test]
    fn runtime_failure_batch_summary_reports_accounting_drift() {
        let summary = RuntimeFailureBatchSummary {
            total_count: 2,
            runtime_count: 1,
            kv_import_count: 1,
            kv_export_count: 1,
            contract_violation_count: 0,
            context_exhausted_count: 0,
            unknown_count: 0,
            recoverable_count: 3,
            backend_error_count: 3,
            diagnostics_note_count: 1,
            min_trace_confidence: Some(1.5),
        };

        assert_eq!(summary.failure_class_total(), 3);
        assert!(!summary.failure_counts_match_total());
        assert_eq!(summary.non_recoverable_count(), 0);
        assert!(!summary.recovery_counts_are_bounded());
        assert!(!summary.backend_error_count_is_bounded());
        assert!(!summary.diagnostics_notes_match_failures());
        assert!(summary.trace_confidence_is_reported());
        assert!(!summary.trace_confidence_is_valid());
        assert_eq!(summary.failure_batch_problem_component_count(), 5);
        assert!(summary.has_failure_batch_problem_components());
        assert!(!summary.failure_batch_accounting_is_consistent());
        assert!(!summary.failure_batch_shape_is_clean());
        assert!(!summary.can_format_runtime_failures());
    }

    #[test]
    fn runtime_failure_batch_summary_blocks_public_shape_drift() {
        let summary = RuntimeFailureBatchSummary {
            total_count: 1,
            runtime_count: 1,
            kv_import_count: 0,
            kv_export_count: 0,
            contract_violation_count: 0,
            context_exhausted_count: 0,
            unknown_count: 0,
            recoverable_count: 0,
            backend_error_count: 2,
            diagnostics_note_count: 1,
            min_trace_confidence: Some(0.0),
        };

        assert!(summary.has_failures());
        assert!(summary.failure_counts_match_total());
        assert!(!summary.backend_error_count_is_bounded());
        assert_eq!(summary.failure_batch_problem_component_count(), 1);
        assert!(summary.has_failure_batch_problem_components());
        assert!(!summary.failure_batch_accounting_is_consistent());
        assert!(!summary.failure_batch_shape_is_clean());
        assert!(!summary.can_format_runtime_failures());
    }

    #[test]
    fn runtime_failure_report_handles_context_exhaustion() {
        let budget = RuntimeGenerationBudget::new(128, 8, 128);
        let failure = RuntimeFailureReport::context_exhausted(budget);

        assert_eq!(failure.kind, RuntimeFailureKind::ContextExhausted);
        assert!(failure.backend_message().contains("Runtime backend error"));
        assert!(failure.message.contains("leaves no generation room"));
        assert_eq!(failure.trace_confidence(), 0.0);
    }
}
