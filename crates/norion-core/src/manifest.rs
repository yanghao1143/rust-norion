use crate::adapter::{
    AdapterExecutionContext, AdapterObservation, AdapterRuntimeClampSummary, RuntimeAdapter,
};
use crate::engine::{
    InferenceError, RuntimeFailureBatchSummary, RuntimeFailureReport, RuntimeFailureSummary,
};
use crate::hardware::HardwareRuntimeReadinessSummary;
use crate::runtime::RuntimeMetadata;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantizationBits {
    Four,
    Eight,
}

impl QuantizationBits {
    pub fn width(self) -> u8 {
        match self {
            Self::Four => 4,
            Self::Eight => 8,
        }
    }

    pub fn from_width(width: u8) -> Option<Self> {
        match width {
            4 => Some(Self::Four),
            8 => Some(Self::Eight),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransformerRuntimeArchitecture {
    pub layer_count: usize,
    pub hidden_size: usize,
    pub attention_heads: usize,
    pub kv_heads: usize,
    pub local_window_tokens: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransformerRuntimeArchitectureSummary {
    pub layer_count: usize,
    pub hidden_size: usize,
    pub attention_heads: usize,
    pub kv_heads: usize,
    pub local_window_tokens: usize,
    pub attention_head_dim: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformerRuntimeArchitectureCommitAction {
    CommitTransformerRuntimeArchitecture,
    ReturnRuntimeFailure,
}

impl TransformerRuntimeArchitectureCommitAction {
    pub fn can_commit(self) -> bool {
        matches!(self, Self::CommitTransformerRuntimeArchitecture)
    }

    pub fn should_return_failure(self) -> bool {
        matches!(self, Self::ReturnRuntimeFailure)
    }
}

impl TransformerRuntimeArchitectureSummary {
    pub fn has_layers(self) -> bool {
        self.layer_count > 0
    }

    pub fn has_hidden_size(self) -> bool {
        self.hidden_size > 0
    }

    pub fn has_attention_heads(self) -> bool {
        self.attention_heads > 0
    }

    pub fn has_kv_heads(self) -> bool {
        self.kv_heads > 0
    }

    pub fn has_local_window(self) -> bool {
        self.local_window_tokens > 0
    }

    pub fn has_integral_attention_head_dim(self) -> bool {
        self.attention_head_dim.is_some()
    }

    pub fn kv_heads_fit_attention(self) -> bool {
        self.attention_heads == 0 || self.kv_heads <= self.attention_heads
    }

    pub fn local_window_fits_context(self, native_context_window: usize) -> bool {
        native_context_window == 0 || self.local_window_tokens <= native_context_window
    }

    pub fn architecture_dimension_signal_component_count(self) -> usize {
        usize::from(self.has_layers())
            + usize::from(self.has_hidden_size())
            + usize::from(self.has_attention_heads())
            + usize::from(self.has_kv_heads())
            + usize::from(self.has_local_window())
    }

    pub fn attention_geometry_signal_component_count(self) -> usize {
        usize::from(self.has_integral_attention_head_dim())
            + usize::from(
                self.has_attention_heads() && self.has_kv_heads() && self.kv_heads_fit_attention(),
            )
    }

    pub fn architecture_signal_component_count(self) -> usize {
        self.architecture_dimension_signal_component_count()
            .saturating_add(self.attention_geometry_signal_component_count())
    }

    pub fn has_architecture_signal_components(self) -> bool {
        self.architecture_signal_component_count() > 0
    }

    pub fn architecture_dimension_problem_component_count(self) -> usize {
        usize::from(!self.has_layers())
            + usize::from(!self.has_hidden_size())
            + usize::from(!self.has_attention_heads())
            + usize::from(!self.has_kv_heads())
            + usize::from(!self.has_local_window())
    }

    pub fn attention_geometry_problem_component_count(self) -> usize {
        usize::from(!self.has_integral_attention_head_dim())
            + usize::from(!self.kv_heads_fit_attention())
    }

    pub fn local_window_context_problem_component_count(
        self,
        native_context_window: usize,
    ) -> usize {
        usize::from(!self.local_window_fits_context(native_context_window))
    }

    pub fn architecture_problem_component_count(self, native_context_window: usize) -> usize {
        self.architecture_dimension_problem_component_count()
            .saturating_add(self.attention_geometry_problem_component_count())
            .saturating_add(
                self.local_window_context_problem_component_count(native_context_window),
            )
    }

    pub fn has_architecture_problem_components(self, native_context_window: usize) -> bool {
        self.architecture_problem_component_count(native_context_window) > 0
    }

    pub fn architecture_accounting_is_consistent(self, native_context_window: usize) -> bool {
        let expected_signal_count = self
            .architecture_dimension_signal_component_count()
            .saturating_add(self.attention_geometry_signal_component_count());
        let expected_problem_count = self
            .architecture_dimension_problem_component_count()
            .saturating_add(self.attention_geometry_problem_component_count())
            .saturating_add(
                self.local_window_context_problem_component_count(native_context_window),
            );

        self.architecture_signal_component_count() == expected_signal_count
            && self.architecture_problem_component_count(native_context_window)
                == expected_problem_count
    }

    pub fn transformer_runtime_architecture_commit_signal_component_count(self) -> usize {
        self.architecture_signal_component_count()
    }

    pub fn transformer_runtime_architecture_commit_blocker_component_count(
        self,
        native_context_window: usize,
    ) -> usize {
        self.architecture_problem_component_count(native_context_window)
    }

    pub fn transformer_runtime_architecture_commit_accounting_is_consistent(
        self,
        native_context_window: usize,
    ) -> bool {
        self.architecture_accounting_is_consistent(native_context_window)
    }

    pub fn transformer_runtime_architecture_commit_is_clean(
        self,
        native_context_window: usize,
    ) -> bool {
        self.transformer_runtime_architecture_commit_blocker_component_count(native_context_window)
            == 0
            && self.transformer_runtime_architecture_commit_accounting_is_consistent(
                native_context_window,
            )
    }

    pub fn shape_is_valid(self) -> bool {
        self.has_layers()
            && self.has_hidden_size()
            && self.has_attention_heads()
            && self.has_kv_heads()
            && self.has_local_window()
            && self.has_integral_attention_head_dim()
            && self.kv_heads_fit_attention()
    }

    pub fn architecture_shape_is_clean(self, native_context_window: usize) -> bool {
        self.transformer_runtime_architecture_commit_is_clean(native_context_window)
    }

    pub fn can_commit_transformer_runtime_architecture(self, native_context_window: usize) -> bool {
        self.shape_is_valid()
            && self.transformer_runtime_architecture_commit_is_clean(native_context_window)
    }

    pub fn transformer_runtime_architecture_commit_action(
        self,
        native_context_window: usize,
    ) -> TransformerRuntimeArchitectureCommitAction {
        if self.can_commit_transformer_runtime_architecture(native_context_window) {
            TransformerRuntimeArchitectureCommitAction::CommitTransformerRuntimeArchitecture
        } else {
            TransformerRuntimeArchitectureCommitAction::ReturnRuntimeFailure
        }
    }

    pub fn can_use_transformer_runtime_architecture(self, native_context_window: usize) -> bool {
        self.can_commit_transformer_runtime_architecture(native_context_window)
    }
}

impl TransformerRuntimeArchitecture {
    pub fn new(
        layer_count: usize,
        hidden_size: usize,
        attention_heads: usize,
        kv_heads: usize,
        local_window_tokens: usize,
    ) -> Self {
        Self {
            layer_count,
            hidden_size,
            attention_heads,
            kv_heads,
            local_window_tokens,
        }
    }

    pub fn from_metadata(metadata: &RuntimeMetadata) -> Self {
        default_transformer_runtime_architecture(
            metadata.native_context_window,
            metadata.embedding_dimensions,
        )
    }

    pub fn summary(self) -> String {
        format!(
            "layers={} hidden={} attention_heads={} kv_heads={} local_window={}",
            self.layer_count,
            self.hidden_size,
            self.attention_heads,
            self.kv_heads,
            self.local_window_tokens
        )
    }

    pub fn architecture_summary(self) -> TransformerRuntimeArchitectureSummary {
        TransformerRuntimeArchitectureSummary {
            layer_count: self.layer_count,
            hidden_size: self.hidden_size,
            attention_heads: self.attention_heads,
            kv_heads: self.kv_heads,
            local_window_tokens: self.local_window_tokens,
            attention_head_dim: if self.attention_heads > 0
                && self.hidden_size > 0
                && self.hidden_size % self.attention_heads == 0
            {
                Some(self.hidden_size / self.attention_heads)
            } else {
                None
            },
        }
    }

    pub fn validation_errors(self) -> Vec<String> {
        let mut errors = Vec::new();
        if self.layer_count == 0 {
            errors.push("layer_count must be greater than zero".to_owned());
        }
        if self.hidden_size == 0 {
            errors.push("hidden_size must be greater than zero".to_owned());
        }
        if self.attention_heads == 0 {
            errors.push("attention_heads must be greater than zero".to_owned());
        }
        if self.kv_heads == 0 {
            errors.push("kv_heads must be greater than zero".to_owned());
        }
        if self.local_window_tokens == 0 {
            errors.push("local_window_tokens must be greater than zero".to_owned());
        }
        if self.attention_heads > 0 && self.hidden_size % self.attention_heads != 0 {
            errors.push("hidden_size must be divisible by attention_heads".to_owned());
        }
        if self.kv_heads > self.attention_heads && self.attention_heads > 0 {
            errors.push("kv_heads must not exceed attention_heads".to_owned());
        }
        errors
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeKvPolicy {
    pub import_enabled: bool,
    pub export_enabled: bool,
    pub max_import_blocks: usize,
    pub max_export_blocks: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeKvPolicySummary {
    pub import_enabled: bool,
    pub export_enabled: bool,
    pub max_import_blocks: usize,
    pub max_export_blocks: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeKvPolicyCommitAction {
    CommitRuntimeKvPolicy,
    ReturnRuntimeFailure,
}

impl RuntimeKvPolicyCommitAction {
    pub fn can_commit(self) -> bool {
        matches!(self, Self::CommitRuntimeKvPolicy)
    }

    pub fn should_return_failure(self) -> bool {
        matches!(self, Self::ReturnRuntimeFailure)
    }
}

impl RuntimeKvPolicySummary {
    pub fn supports_kv_exchange(self) -> bool {
        self.import_enabled || self.export_enabled
    }

    pub fn has_import_capacity(self) -> bool {
        self.import_enabled && self.max_import_blocks > 0
    }

    pub fn has_export_capacity(self) -> bool {
        self.export_enabled && self.max_export_blocks > 0
    }

    pub fn block_capacity(self) -> usize {
        self.max_import_blocks
            .saturating_add(self.max_export_blocks)
    }

    pub fn limits_match_capabilities(self) -> bool {
        (!self.import_enabled && self.max_import_blocks == 0
            || self.import_enabled && self.max_import_blocks > 0)
            && (!self.export_enabled && self.max_export_blocks == 0
                || self.export_enabled && self.max_export_blocks > 0)
    }

    pub fn kv_capability_signal_component_count(self) -> usize {
        usize::from(self.import_enabled) + usize::from(self.export_enabled)
    }

    pub fn kv_capacity_signal_component_count(self) -> usize {
        usize::from(self.has_import_capacity())
            + usize::from(self.has_export_capacity())
            + usize::from(self.block_capacity() > 0)
    }

    pub fn kv_policy_signal_component_count(self) -> usize {
        self.kv_capability_signal_component_count()
            .saturating_add(self.kv_capacity_signal_component_count())
    }

    pub fn has_kv_policy_signal_components(self) -> bool {
        self.kv_policy_signal_component_count() > 0
    }

    pub fn import_limit_problem_component_count(self) -> usize {
        usize::from(self.import_enabled != (self.max_import_blocks > 0))
    }

    pub fn export_limit_problem_component_count(self) -> usize {
        usize::from(self.export_enabled != (self.max_export_blocks > 0))
    }

    pub fn kv_policy_problem_component_count(self) -> usize {
        self.import_limit_problem_component_count()
            .saturating_add(self.export_limit_problem_component_count())
    }

    pub fn has_kv_policy_problem_components(self) -> bool {
        self.kv_policy_problem_component_count() > 0
    }

    pub fn kv_policy_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .kv_capability_signal_component_count()
            .saturating_add(self.kv_capacity_signal_component_count());
        let expected_problem_count = self
            .import_limit_problem_component_count()
            .saturating_add(self.export_limit_problem_component_count());

        self.kv_policy_signal_component_count() == expected_signal_count
            && self.kv_policy_problem_component_count() == expected_problem_count
            && self.has_kv_policy_problem_components() != self.limits_match_capabilities()
    }

    pub fn runtime_kv_policy_commit_signal_component_count(self) -> usize {
        self.kv_policy_signal_component_count()
    }

    pub fn runtime_kv_policy_commit_blocker_component_count(self) -> usize {
        self.kv_policy_problem_component_count()
    }

    pub fn runtime_kv_policy_commit_accounting_is_consistent(self) -> bool {
        self.kv_policy_accounting_is_consistent()
    }

    pub fn runtime_kv_policy_commit_is_clean(self) -> bool {
        self.runtime_kv_policy_commit_blocker_component_count() == 0
            && self.runtime_kv_policy_commit_accounting_is_consistent()
    }

    pub fn kv_policy_shape_is_clean(self) -> bool {
        self.runtime_kv_policy_commit_is_clean()
    }

    pub fn can_commit_runtime_kv_policy(self) -> bool {
        self.runtime_kv_policy_commit_is_clean()
    }

    pub fn runtime_kv_policy_commit_action(self) -> RuntimeKvPolicyCommitAction {
        if self.can_commit_runtime_kv_policy() {
            RuntimeKvPolicyCommitAction::CommitRuntimeKvPolicy
        } else {
            RuntimeKvPolicyCommitAction::ReturnRuntimeFailure
        }
    }

    pub fn can_use_runtime_kv_policy(self) -> bool {
        self.can_commit_runtime_kv_policy()
    }
}

impl RuntimeKvPolicy {
    pub fn disabled() -> Self {
        Self {
            import_enabled: false,
            export_enabled: false,
            max_import_blocks: 0,
            max_export_blocks: 0,
        }
    }

    pub fn import_export() -> Self {
        Self {
            import_enabled: true,
            export_enabled: true,
            max_import_blocks: 8,
            max_export_blocks: 4,
        }
    }

    pub fn from_capabilities(import_enabled: bool, export_enabled: bool) -> Self {
        Self {
            import_enabled,
            export_enabled,
            max_import_blocks: if import_enabled { 8 } else { 0 },
            max_export_blocks: if export_enabled { 4 } else { 0 },
        }
    }

    pub fn with_limits(mut self, max_import_blocks: usize, max_export_blocks: usize) -> Self {
        self.max_import_blocks = if self.import_enabled {
            max_import_blocks.max(1)
        } else {
            0
        };
        self.max_export_blocks = if self.export_enabled {
            max_export_blocks.max(1)
        } else {
            0
        };
        self
    }

    pub fn kv_policy_summary(self) -> RuntimeKvPolicySummary {
        RuntimeKvPolicySummary {
            import_enabled: self.import_enabled,
            export_enabled: self.export_enabled,
            max_import_blocks: self.max_import_blocks,
            max_export_blocks: self.max_export_blocks,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeQuantizationPolicy {
    pub hot_kv: QuantizationBits,
    pub cold_kv: QuantizationBits,
    pub weights: Option<QuantizationBits>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeQuantizationPolicySummary {
    pub hot_kv_precision_bits: u8,
    pub cold_kv_precision_bits: u8,
    pub weight_precision_bits: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeQuantizationPolicyCommitAction {
    CommitRuntimeQuantizationPolicy,
    ReturnRuntimeFailure,
}

impl RuntimeQuantizationPolicyCommitAction {
    pub fn can_commit(self) -> bool {
        matches!(self, Self::CommitRuntimeQuantizationPolicy)
    }

    pub fn should_return_failure(self) -> bool {
        matches!(self, Self::ReturnRuntimeFailure)
    }
}

impl RuntimeQuantizationPolicySummary {
    pub fn hot_kv_precision_is_supported(self) -> bool {
        QuantizationBits::from_width(self.hot_kv_precision_bits).is_some()
    }

    pub fn cold_kv_precision_is_supported(self) -> bool {
        QuantizationBits::from_width(self.cold_kv_precision_bits).is_some()
    }

    pub fn weight_precision_is_supported(self) -> bool {
        self.weight_precision_bits
            .map_or(true, |bits| QuantizationBits::from_width(bits).is_some())
    }

    pub fn uses_compressed_hot_kv(self) -> bool {
        self.hot_kv_precision_bits < 8
    }

    pub fn uses_compressed_cold_kv(self) -> bool {
        self.cold_kv_precision_bits < 8
    }

    pub fn cold_kv_not_wider_than_hot(self) -> bool {
        self.cold_kv_precision_bits <= self.hot_kv_precision_bits
    }

    pub fn has_weight_quantization(self) -> bool {
        self.weight_precision_bits.is_some()
    }

    pub fn all_kv_is_four_bit(self) -> bool {
        self.hot_kv_precision_bits == 4 && self.cold_kv_precision_bits == 4
    }

    pub fn kv_precision_signal_component_count(self) -> usize {
        usize::from(self.hot_kv_precision_is_supported())
            + usize::from(self.cold_kv_precision_is_supported())
            + usize::from(self.cold_kv_not_wider_than_hot())
    }

    pub fn compression_signal_component_count(self) -> usize {
        usize::from(self.hot_kv_precision_is_supported() && self.uses_compressed_hot_kv())
            + usize::from(self.cold_kv_precision_is_supported() && self.uses_compressed_cold_kv())
            + usize::from(
                self.hot_kv_precision_is_supported()
                    && self.cold_kv_precision_is_supported()
                    && self.all_kv_is_four_bit(),
            )
    }

    pub fn weight_precision_signal_component_count(self) -> usize {
        usize::from(self.has_weight_quantization() && self.weight_precision_is_supported())
    }

    pub fn quantization_signal_component_count(self) -> usize {
        self.kv_precision_signal_component_count()
            .saturating_add(self.compression_signal_component_count())
            .saturating_add(self.weight_precision_signal_component_count())
    }

    pub fn has_quantization_signal_components(self) -> bool {
        self.quantization_signal_component_count() > 0
    }

    pub fn kv_precision_problem_component_count(self) -> usize {
        usize::from(!self.hot_kv_precision_is_supported())
            + usize::from(!self.cold_kv_precision_is_supported())
            + usize::from(!self.cold_kv_not_wider_than_hot())
    }

    pub fn weight_precision_problem_component_count(self) -> usize {
        usize::from(!self.weight_precision_is_supported())
    }

    pub fn quantization_problem_component_count(self) -> usize {
        self.kv_precision_problem_component_count()
            .saturating_add(self.weight_precision_problem_component_count())
    }

    pub fn has_quantization_problem_components(self) -> bool {
        self.quantization_problem_component_count() > 0
    }

    pub fn quantization_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .kv_precision_signal_component_count()
            .saturating_add(self.compression_signal_component_count())
            .saturating_add(self.weight_precision_signal_component_count());
        let expected_problem_count = self
            .kv_precision_problem_component_count()
            .saturating_add(self.weight_precision_problem_component_count());

        self.quantization_signal_component_count() == expected_signal_count
            && self.quantization_problem_component_count() == expected_problem_count
    }

    pub fn runtime_quantization_policy_commit_signal_component_count(self) -> usize {
        self.quantization_signal_component_count()
    }

    pub fn runtime_quantization_policy_commit_blocker_component_count(self) -> usize {
        self.quantization_problem_component_count()
    }

    pub fn runtime_quantization_policy_commit_accounting_is_consistent(self) -> bool {
        self.quantization_accounting_is_consistent()
    }

    pub fn runtime_quantization_policy_commit_is_clean(self) -> bool {
        self.runtime_quantization_policy_commit_blocker_component_count() == 0
            && self.runtime_quantization_policy_commit_accounting_is_consistent()
    }

    pub fn quantization_shape_is_clean(self) -> bool {
        self.runtime_quantization_policy_commit_is_clean()
    }

    pub fn can_commit_runtime_quantization_policy(self) -> bool {
        self.runtime_quantization_policy_commit_is_clean()
    }

    pub fn runtime_quantization_policy_commit_action(
        self,
    ) -> RuntimeQuantizationPolicyCommitAction {
        if self.can_commit_runtime_quantization_policy() {
            RuntimeQuantizationPolicyCommitAction::CommitRuntimeQuantizationPolicy
        } else {
            RuntimeQuantizationPolicyCommitAction::ReturnRuntimeFailure
        }
    }

    pub fn can_use_runtime_quantization_policy(self) -> bool {
        self.can_commit_runtime_quantization_policy()
    }
}

impl RuntimeQuantizationPolicy {
    pub fn from_metadata(metadata: &RuntimeMetadata) -> Self {
        let hot_kv = QuantizationBits::from_width(metadata.hot_kv_precision_bits)
            .unwrap_or(QuantizationBits::Eight);
        let cold_kv = QuantizationBits::from_width(metadata.cold_kv_precision_bits)
            .filter(|bits| bits.width() <= hot_kv.width())
            .unwrap_or_else(|| {
                if metadata.cold_kv_precision_bits > hot_kv.width() {
                    hot_kv
                } else {
                    QuantizationBits::Four
                }
            });

        Self {
            hot_kv,
            cold_kv,
            weights: None,
        }
    }

    pub fn quantization_summary(self) -> RuntimeQuantizationPolicySummary {
        RuntimeQuantizationPolicySummary {
            hot_kv_precision_bits: self.hot_kv.width(),
            cold_kv_precision_bits: self.cold_kv.width(),
            weight_precision_bits: self.weights.map(QuantizationBits::width),
        }
    }
}

impl Default for RuntimeQuantizationPolicy {
    fn default() -> Self {
        Self {
            hot_kv: QuantizationBits::Eight,
            cold_kv: QuantizationBits::Four,
            weights: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeManifestValidation {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeManifestValidationSummary {
    pub passed: bool,
    pub warning_count: usize,
    pub error_count: usize,
    pub warnings_only: bool,
    pub failure_report_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeManifestValidationCommitSummary {
    pub validation: RuntimeManifestValidationSummary,
    pub action: RuntimeManifestValidationCommitAction,
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
pub enum RuntimeManifestValidationCommitAction {
    CommitManifest,
    ReturnRuntimeFailure,
}

impl RuntimeManifestValidationCommitAction {
    pub fn can_commit(self) -> bool {
        matches!(self, Self::CommitManifest)
    }

    pub fn should_return_failure(self) -> bool {
        matches!(self, Self::ReturnRuntimeFailure)
    }
}

impl RuntimeManifestValidationSummary {
    pub fn has_errors(self) -> bool {
        self.error_count > 0
    }

    pub fn has_warnings(self) -> bool {
        self.warning_count > 0
    }

    pub fn has_blocking_failures(self) -> bool {
        self.has_errors()
    }

    pub fn has_failure_reports(self) -> bool {
        self.failure_report_count > 0
    }

    pub fn validation_error_component_count(self) -> usize {
        usize::from(self.has_errors())
    }

    pub fn validation_warning_component_count(self) -> usize {
        usize::from(self.has_warnings())
    }

    pub fn blocking_failure_component_count(self) -> usize {
        usize::from(self.has_blocking_failures())
    }

    pub fn mapped_failure_report_component_count(self) -> usize {
        usize::from(self.has_failure_reports())
    }

    pub fn validation_signal_component_count(self) -> usize {
        self.validation_error_component_count()
            .saturating_add(self.validation_warning_component_count())
            .saturating_add(self.mapped_failure_report_component_count())
    }

    pub fn has_validation_signal_components(self) -> bool {
        self.validation_signal_component_count() > 0
    }

    pub fn validation_signal_accounting_is_consistent(self) -> bool {
        self.validation_signal_component_count()
            == self
                .validation_error_component_count()
                .saturating_add(self.validation_warning_component_count())
                .saturating_add(self.mapped_failure_report_component_count())
    }

    pub fn warnings_only_flag_matches_shape(self) -> bool {
        self.warnings_only == (self.passed && self.has_warnings() && !self.has_errors())
    }

    pub fn failure_reports_match_errors(self) -> bool {
        if self.has_errors() {
            self.failure_report_count == 1
        } else {
            self.failure_report_count == 0
        }
    }

    pub fn validation_activity_signal_component_count(self) -> usize {
        self.validation_warning_component_count()
            .saturating_add(self.mapped_failure_report_component_count())
    }

    pub fn has_validation_activity_signals(self) -> bool {
        self.validation_activity_signal_component_count() > 0
    }

    pub fn validation_problem_component_count(self) -> usize {
        self.blocking_failure_component_count()
            .saturating_add(usize::from(!self.failure_reports_match_errors()))
            .saturating_add(usize::from(!self.warnings_only_flag_matches_shape()))
    }

    pub fn has_validation_problem_components(self) -> bool {
        self.validation_problem_component_count() > 0
    }

    pub fn validation_accounting_is_consistent(self) -> bool {
        let expected_activity_count = self
            .validation_warning_component_count()
            .saturating_add(self.mapped_failure_report_component_count());
        let expected_problem_count = self
            .blocking_failure_component_count()
            .saturating_add(usize::from(!self.failure_reports_match_errors()))
            .saturating_add(usize::from(!self.warnings_only_flag_matches_shape()));

        self.validation_signal_accounting_is_consistent()
            && self.validation_activity_signal_component_count() == expected_activity_count
            && self.validation_problem_component_count() == expected_problem_count
            && self.has_validation_problem_components() == (expected_problem_count > 0)
    }

    pub fn runtime_manifest_validation_commit_signal_component_count(self) -> usize {
        self.validation_signal_component_count()
    }

    pub fn has_runtime_manifest_validation_commit_signals(self) -> bool {
        self.runtime_manifest_validation_commit_signal_component_count() > 0
    }

    pub fn runtime_manifest_validation_commit_blocker_component_count(self) -> usize {
        self.validation_problem_component_count()
    }

    pub fn has_runtime_manifest_validation_commit_blockers(self) -> bool {
        self.runtime_manifest_validation_commit_blocker_component_count() > 0
    }

    pub fn runtime_manifest_validation_commit_accounting_is_consistent(self) -> bool {
        self.validation_accounting_is_consistent()
            && self.runtime_manifest_validation_commit_signal_component_count()
                == self.validation_signal_component_count()
            && self.has_runtime_manifest_validation_commit_signals()
                == (self.runtime_manifest_validation_commit_signal_component_count() > 0)
            && self.runtime_manifest_validation_commit_blocker_component_count()
                == self.validation_problem_component_count()
            && self.has_runtime_manifest_validation_commit_blockers()
                == (self.runtime_manifest_validation_commit_blocker_component_count() > 0)
    }

    pub fn runtime_manifest_validation_commit_is_clean(self) -> bool {
        !self.has_runtime_manifest_validation_commit_blockers()
            && self.runtime_manifest_validation_commit_accounting_is_consistent()
    }

    pub fn is_clean_pass(self) -> bool {
        self.passed
            && !self.has_errors()
            && !self.has_warnings()
            && !self.warnings_only
            && self.failure_reports_match_errors()
            && self.validation_accounting_is_consistent()
    }

    pub fn is_warnings_only_pass(self) -> bool {
        self.passed
            && !self.has_errors()
            && self.has_warnings()
            && self.warnings_only
            && self.failure_reports_match_errors()
            && self.validation_accounting_is_consistent()
    }

    pub fn validation_shape_is_clean(self) -> bool {
        self.runtime_manifest_validation_commit_is_clean()
    }

    pub fn can_commit_runtime_manifest_validation(self) -> bool {
        self.passed && self.runtime_manifest_validation_commit_is_clean()
    }

    pub fn runtime_manifest_validation_commit_action(
        self,
    ) -> RuntimeManifestValidationCommitAction {
        if self.can_commit_runtime_manifest_validation() {
            RuntimeManifestValidationCommitAction::CommitManifest
        } else {
            RuntimeManifestValidationCommitAction::ReturnRuntimeFailure
        }
    }

    pub fn can_accept_runtime_manifest_validation(self) -> bool {
        self.can_commit_runtime_manifest_validation()
    }
}

impl RuntimeManifestValidationCommitSummary {
    pub fn new(validation: &RuntimeManifestValidation) -> Self {
        let summary = validation.validation_summary();
        let failure_reports = validation.failure_reports();
        let primary_failure_report = failure_reports.first().cloned();
        let primary_failure_summary = primary_failure_report
            .as_ref()
            .map(|failure| failure.failure_summary());
        let failure_batch = RuntimeFailureReport::batch_summary(&failure_reports);
        let can_commit = summary.can_commit_runtime_manifest_validation();
        let failure_report_count = failure_reports.len();
        let can_format_runtime_failures = failure_batch.can_format_runtime_failures();
        let should_return_failure = !can_commit && failure_report_count > 0;
        let action = summary.runtime_manifest_validation_commit_action();

        Self {
            validation: summary,
            action,
            can_commit,
            should_return_failure,
            failure_reports,
            primary_failure_report,
            primary_failure_summary,
            failure_batch,
            failure_report_count,
            can_format_runtime_failures,
            total_signal_component_count: summary
                .runtime_manifest_validation_commit_signal_component_count(),
            total_blocker_component_count: summary
                .runtime_manifest_validation_commit_blocker_component_count(),
            component_accounting_consistent: summary
                .runtime_manifest_validation_commit_accounting_is_consistent(),
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

    pub fn failure_return_summary(&self) -> ManifestFailureReturnSummary {
        ManifestFailureReturnSummary::new(
            ManifestFailureReturnSource::ManifestValidation,
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

    pub fn runtime_failure_return_report(&self) -> Option<ManifestFailureReturnReport> {
        if self.failure_return_summary().can_return_runtime_failure() {
            self.primary_failure_report.clone().map(|failure| {
                ManifestFailureReturnReport::new(
                    ManifestFailureReturnSource::ManifestValidation,
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
        self.can_commit == self.validation.can_commit_runtime_manifest_validation()
            && self.should_return_failure == (!self.can_commit && self.failure_report_count > 0)
            && self.action == self.validation.runtime_manifest_validation_commit_action()
            && self.action_can_commit() == self.can_commit
            && self.action_should_return_failure() == self.should_return_failure
            && self.failure_report_count == self.failure_reports.len()
            && self.failure_report_count == self.validation.failure_report_count
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
                    .validation
                    .runtime_manifest_validation_commit_signal_component_count()
            && self.total_blocker_component_count
                == self
                    .validation
                    .runtime_manifest_validation_commit_blocker_component_count()
            && self.component_accounting_consistent
                == self
                    .validation
                    .runtime_manifest_validation_commit_accounting_is_consistent()
    }

    pub fn can_commit_runtime_manifest_validation(&self) -> bool {
        self.can_commit && self.commit_decision_accounting_is_consistent()
    }

    pub fn should_return_runtime_failure(&self) -> bool {
        self.should_return_failure && self.commit_decision_accounting_is_consistent()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeManifestAdapterCompatibilitySummary {
    pub supported_adapter_count: usize,
    pub execution_adapter_count: usize,
    pub compatible_adapter_count: usize,
    pub observation_count: usize,
    pub compatible_observation_count: usize,
    pub preferred_adapter: Option<RuntimeAdapter>,
    pub preferred_observed_adapter: Option<RuntimeAdapter>,
    pub selected_adapter: Option<RuntimeAdapter>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeManifestAdapterCompatibilityCommitAction {
    CommitRuntimeManifestAdapterCompatibility,
    ReturnRuntimeFailure,
}

impl RuntimeManifestAdapterCompatibilitySummary {
    pub fn has_supported_adapters(self) -> bool {
        self.supported_adapter_count > 0
    }

    pub fn has_execution_adapters(self) -> bool {
        self.execution_adapter_count > 0
    }

    pub fn has_compatible_adapter(self) -> bool {
        self.compatible_adapter_count > 0
    }

    pub fn has_adapter_intersection(self) -> bool {
        self.has_compatible_adapter()
    }

    pub fn missing_supported_adapter_catalog(self) -> bool {
        !self.has_supported_adapters()
    }

    pub fn missing_execution_adapter_hints(self) -> bool {
        !self.has_execution_adapters()
    }

    pub fn adapter_sets_are_disjoint(self) -> bool {
        self.has_supported_adapters()
            && self.has_execution_adapters()
            && !self.has_adapter_intersection()
    }

    pub fn has_compatible_observation(self) -> bool {
        self.compatible_observation_count > 0
    }

    pub fn observations_all_rejected(self) -> bool {
        self.observation_count > 0 && !self.has_compatible_observation()
    }

    pub fn rejected_observation_count(self) -> usize {
        self.observation_count
            .saturating_sub(self.compatible_observation_count)
    }

    pub fn has_selected_adapter(self) -> bool {
        self.selected_adapter.is_some()
    }

    pub fn selected_adapter_available(self) -> bool {
        self.has_selected_adapter() && self.has_adapter_intersection()
    }

    pub fn selected_adapter_missing(self) -> bool {
        !self.has_selected_adapter() && self.has_adapter_intersection()
    }

    pub fn selected_adapter_unavailable(self) -> bool {
        self.has_selected_adapter() && !self.selected_adapter_available()
    }

    pub fn can_plan_runtime_adapter(self) -> bool {
        self.has_supported_adapters()
            && self.has_execution_adapters()
            && self.selected_adapter_available()
    }

    pub fn selected_from_observation(self) -> bool {
        self.selected_adapter.is_some() && self.selected_adapter == self.preferred_observed_adapter
    }

    pub fn selected_from_fallback(self) -> bool {
        self.selected_adapter.is_some()
            && self.selected_adapter == self.preferred_adapter
            && self.selected_adapter != self.preferred_observed_adapter
    }

    pub fn selection_requires_fallback(self) -> bool {
        self.has_selected_adapter() && !self.selected_from_observation()
    }

    pub fn compatibility_counts_are_bounded(self) -> bool {
        self.compatible_adapter_count <= self.supported_adapter_count
            && self.compatible_adapter_count <= self.execution_adapter_count
            && self.compatible_observation_count <= self.observation_count
    }

    pub fn compatible_adapter_fraction(self) -> f32 {
        let possible = self
            .supported_adapter_count
            .min(self.execution_adapter_count);
        if possible == 0 {
            0.0
        } else {
            self.compatible_adapter_count as f32 / possible as f32
        }
    }

    pub fn compatible_observation_fraction(self) -> f32 {
        if self.observation_count == 0 {
            0.0
        } else {
            self.compatible_observation_count as f32 / self.observation_count as f32
        }
    }

    pub fn selected_adapter_has_source(self) -> bool {
        if self.selected_adapter.is_none() {
            return !self.has_adapter_intersection();
        }

        self.selected_from_observation() || self.selected_from_fallback()
    }

    pub fn selected_adapter_source_drifted(self) -> bool {
        self.has_selected_adapter() && !self.selected_adapter_has_source()
    }

    pub fn selected_adapter_is_usable(self) -> bool {
        if self.has_selected_adapter() {
            self.selected_adapter_available() && self.selected_adapter_has_source()
        } else {
            !self.has_adapter_intersection()
        }
    }

    pub fn adapter_source_problem_component_count(self) -> usize {
        usize::from(self.missing_supported_adapter_catalog())
            + usize::from(self.missing_execution_adapter_hints())
            + usize::from(self.adapter_sets_are_disjoint())
            + usize::from(!self.compatibility_counts_are_bounded())
            + usize::from(self.selected_adapter_missing())
            + usize::from(self.selected_adapter_unavailable())
            + usize::from(self.selected_adapter_source_drifted())
    }

    pub fn has_adapter_source_problem_components(self) -> bool {
        self.adapter_source_problem_component_count() > 0
    }

    pub fn observation_selection_signal_component_count(self) -> usize {
        usize::from(self.observations_all_rejected())
            + usize::from(self.selection_requires_fallback())
    }

    pub fn has_observation_selection_signals(self) -> bool {
        self.observation_selection_signal_component_count() > 0
    }

    pub fn adapter_catalog_signal_component_count(self) -> usize {
        usize::from(self.has_supported_adapters())
            + usize::from(self.has_execution_adapters())
            + usize::from(self.has_compatible_adapter())
    }

    pub fn adapter_observation_signal_component_count(self) -> usize {
        usize::from(self.observation_count > 0)
            + usize::from(self.has_compatible_observation())
            + self.observation_selection_signal_component_count()
    }

    pub fn adapter_selection_signal_component_count(self) -> usize {
        usize::from(self.has_selected_adapter())
            + usize::from(self.selected_from_observation())
            + usize::from(self.selected_from_fallback())
    }

    pub fn adapter_compatibility_signal_component_count(self) -> usize {
        self.adapter_catalog_signal_component_count()
            .saturating_add(self.adapter_observation_signal_component_count())
            .saturating_add(self.adapter_selection_signal_component_count())
    }

    pub fn has_adapter_compatibility_signals(self) -> bool {
        self.adapter_compatibility_signal_component_count() > 0
    }

    pub fn adapter_catalog_problem_component_count(self) -> usize {
        usize::from(self.missing_supported_adapter_catalog())
            + usize::from(self.missing_execution_adapter_hints())
            + usize::from(self.adapter_sets_are_disjoint())
            + usize::from(!self.compatibility_counts_are_bounded())
    }

    pub fn adapter_selection_problem_component_count(self) -> usize {
        usize::from(self.selected_adapter_missing())
            + usize::from(self.selected_adapter_unavailable())
            + usize::from(self.selected_adapter_source_drifted())
    }

    pub fn adapter_compatibility_problem_component_count(self) -> usize {
        self.adapter_catalog_problem_component_count()
            .saturating_add(self.adapter_selection_problem_component_count())
    }

    pub fn has_adapter_compatibility_problem_components(self) -> bool {
        self.adapter_compatibility_problem_component_count() > 0
    }

    pub fn adapter_compatibility_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .adapter_catalog_signal_component_count()
            .saturating_add(self.adapter_observation_signal_component_count())
            .saturating_add(self.adapter_selection_signal_component_count());
        let expected_problem_count = self
            .adapter_catalog_problem_component_count()
            .saturating_add(self.adapter_selection_problem_component_count());

        self.adapter_compatibility_signal_component_count() == expected_signal_count
            && self.adapter_compatibility_problem_component_count() == expected_problem_count
            && self.adapter_compatibility_problem_component_count()
                == self.adapter_source_problem_component_count()
            && self.has_adapter_compatibility_signals() == (expected_signal_count > 0)
            && self.has_adapter_compatibility_problem_components() == (expected_problem_count > 0)
    }

    pub fn runtime_manifest_adapter_compatibility_commit_signal_component_count(self) -> usize {
        self.adapter_compatibility_signal_component_count()
    }

    pub fn has_runtime_manifest_adapter_compatibility_commit_signals(self) -> bool {
        self.runtime_manifest_adapter_compatibility_commit_signal_component_count() > 0
    }

    pub fn runtime_manifest_adapter_compatibility_commit_blocker_component_count(self) -> usize {
        self.adapter_compatibility_problem_component_count()
    }

    pub fn has_runtime_manifest_adapter_compatibility_commit_blockers(self) -> bool {
        self.runtime_manifest_adapter_compatibility_commit_blocker_component_count() > 0
    }

    pub fn runtime_manifest_adapter_compatibility_commit_accounting_is_consistent(self) -> bool {
        self.adapter_compatibility_accounting_is_consistent()
            && self.runtime_manifest_adapter_compatibility_commit_signal_component_count()
                == self.adapter_compatibility_signal_component_count()
            && self.has_runtime_manifest_adapter_compatibility_commit_signals()
                == (self.runtime_manifest_adapter_compatibility_commit_signal_component_count() > 0)
            && self.runtime_manifest_adapter_compatibility_commit_blocker_component_count()
                == self.adapter_compatibility_problem_component_count()
            && self.has_runtime_manifest_adapter_compatibility_commit_blockers()
                == (self.runtime_manifest_adapter_compatibility_commit_blocker_component_count()
                    > 0)
    }

    pub fn runtime_manifest_adapter_compatibility_commit_is_clean(self) -> bool {
        !self.has_runtime_manifest_adapter_compatibility_commit_blockers()
            && self.runtime_manifest_adapter_compatibility_commit_accounting_is_consistent()
    }

    pub fn adapter_compatibility_shape_is_clean(self) -> bool {
        self.runtime_manifest_adapter_compatibility_commit_is_clean()
    }

    pub fn can_commit_runtime_manifest_adapter_compatibility(self) -> bool {
        self.can_use_runtime_adapter_plan()
            && self.runtime_manifest_adapter_compatibility_commit_is_clean()
    }

    pub fn runtime_manifest_adapter_compatibility_commit_action(
        self,
    ) -> RuntimeManifestAdapterCompatibilityCommitAction {
        if self.can_commit_runtime_manifest_adapter_compatibility() {
            RuntimeManifestAdapterCompatibilityCommitAction::CommitRuntimeManifestAdapterCompatibility
        } else {
            RuntimeManifestAdapterCompatibilityCommitAction::ReturnRuntimeFailure
        }
    }

    pub fn can_use_runtime_adapter_plan(self) -> bool {
        self.can_plan_runtime_adapter()
            && self.selected_adapter_is_usable()
            && self.adapter_compatibility_shape_is_clean()
    }

    pub fn adapter_planning_signal_component_count(self) -> usize {
        self.adapter_source_problem_component_count()
            + self.observation_selection_signal_component_count()
    }

    pub fn has_adapter_planning_signals(self) -> bool {
        self.adapter_planning_signal_component_count() > 0
    }

    pub fn adapter_source_problem(self) -> bool {
        self.has_adapter_source_problem_components()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeManifestExecutionCompatibilitySummary {
    pub kv_import_enabled: bool,
    pub kv_export_enabled: bool,
    pub max_kv_import_blocks: usize,
    pub max_kv_export_blocks: usize,
    pub execution_kv_prefetch_blocks: usize,
    pub manifest_hot_kv_precision_bits: u8,
    pub manifest_cold_kv_precision_bits: u8,
    pub execution_hot_kv_precision_bits: u8,
    pub execution_cold_kv_precision_bits: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeManifestExecutionDeviceCommitAction {
    CommitManifestExecutionDevice,
    ReturnRuntimeFailure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeDeviceHandoffStage {
    HardwareRuntime,
    RuntimeClamp,
    ManifestExecutionDevice,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeDeviceHandoffReadinessSummary {
    pub hardware: HardwareRuntimeReadinessSummary,
    pub runtime_clamp: AdapterRuntimeClampSummary,
    pub manifest_execution: RuntimeManifestExecutionCompatibilitySummary,
    pub hardware_signal_component_count: usize,
    pub runtime_clamp_signal_component_count: usize,
    pub manifest_execution_signal_component_count: usize,
    pub hardware_blocker_component_count: usize,
    pub runtime_clamp_blocker_component_count: usize,
    pub manifest_execution_blocker_component_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeDeviceHandoffCommitSummary {
    pub readiness: RuntimeDeviceHandoffReadinessSummary,
    pub action: RuntimeDeviceHandoffCommitAction,
    pub can_commit: bool,
    pub should_return_failure: bool,
    pub first_unready_stage: Option<RuntimeDeviceHandoffStage>,
    pub first_blocking_stage: Option<RuntimeDeviceHandoffStage>,
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
pub enum RuntimeDeviceHandoffCommitAction {
    CommitDeviceHandoff,
    ReturnRuntimeFailure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestFailureReturnSource {
    ManifestValidation,
    RuntimeDeviceHandoff,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ManifestFailureReturnSummary {
    pub source: ManifestFailureReturnSource,
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
pub struct ManifestFailureReturnReport {
    pub source: ManifestFailureReturnSource,
    pub primary_failure: RuntimeFailureReport,
    pub primary_failure_summary: RuntimeFailureSummary,
    pub failure_batch: RuntimeFailureBatchSummary,
    pub failure_report_count: usize,
    pub can_format_runtime_failures: bool,
    pub total_blocker_component_count: usize,
}

impl RuntimeDeviceHandoffStage {
    pub fn label(self) -> &'static str {
        match self {
            Self::HardwareRuntime => "hardware_runtime",
            Self::RuntimeClamp => "runtime_clamp",
            Self::ManifestExecutionDevice => "manifest_execution_device",
        }
    }
}

impl RuntimeDeviceHandoffCommitAction {
    pub fn can_commit(self) -> bool {
        matches!(self, Self::CommitDeviceHandoff)
    }

    pub fn should_return_failure(self) -> bool {
        matches!(self, Self::ReturnRuntimeFailure)
    }
}

impl RuntimeManifestExecutionDeviceCommitAction {
    pub fn can_commit(self) -> bool {
        matches!(self, Self::CommitManifestExecutionDevice)
    }

    pub fn should_return_failure(self) -> bool {
        matches!(self, Self::ReturnRuntimeFailure)
    }
}

impl RuntimeManifestAdapterCompatibilityCommitAction {
    pub fn can_commit(self) -> bool {
        matches!(self, Self::CommitRuntimeManifestAdapterCompatibility)
    }

    pub fn should_return_failure(self) -> bool {
        matches!(self, Self::ReturnRuntimeFailure)
    }
}

impl ManifestFailureReturnSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::ManifestValidation => "manifest_validation",
            Self::RuntimeDeviceHandoff => "runtime_device_handoff",
        }
    }
}

impl ManifestFailureReturnSummary {
    pub fn new(
        source: ManifestFailureReturnSource,
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

impl ManifestFailureReturnReport {
    pub fn new(
        source: ManifestFailureReturnSource,
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

    pub fn can_use_manifest_failure_return_report(&self) -> bool {
        self.failure_return_report_shape_is_clean()
    }
}

impl RuntimeManifestExecutionCompatibilitySummary {
    pub fn import_enabled_without_capacity(self) -> bool {
        self.kv_import_enabled && self.max_kv_import_blocks == 0
    }

    pub fn export_enabled_without_capacity(self) -> bool {
        self.kv_export_enabled && self.max_kv_export_blocks == 0
    }

    pub fn execution_requests_kv_import(self) -> bool {
        self.execution_kv_prefetch_blocks > 0
    }

    pub fn execution_requests_disabled_kv_import(self) -> bool {
        self.execution_requests_kv_import() && !self.kv_import_enabled
    }

    pub fn kv_prefetch_exceeds_import_limit(self) -> bool {
        self.kv_import_enabled && self.execution_kv_prefetch_blocks > self.max_kv_import_blocks
    }

    pub fn kv_prefetch_overflow_blocks(self) -> usize {
        if self.kv_import_enabled {
            self.execution_kv_prefetch_blocks
                .saturating_sub(self.max_kv_import_blocks)
        } else {
            self.execution_kv_prefetch_blocks
        }
    }

    pub fn kv_prefetch_within_manifest_limit(self) -> bool {
        if self.kv_import_enabled {
            !self.import_enabled_without_capacity() && !self.kv_prefetch_exceeds_import_limit()
        } else {
            !self.execution_requests_kv_import()
        }
    }

    pub fn hot_precision_within_manifest(self) -> bool {
        self.execution_hot_kv_precision_bits <= self.manifest_hot_kv_precision_bits
    }

    pub fn hot_precision_overflow_bits(self) -> u8 {
        self.execution_hot_kv_precision_bits
            .saturating_sub(self.manifest_hot_kv_precision_bits)
    }

    pub fn cold_precision_within_manifest(self) -> bool {
        self.execution_cold_kv_precision_bits <= self.manifest_cold_kv_precision_bits
    }

    pub fn cold_precision_overflow_bits(self) -> u8 {
        self.execution_cold_kv_precision_bits
            .saturating_sub(self.manifest_cold_kv_precision_bits)
    }

    pub fn precision_within_manifest(self) -> bool {
        self.hot_precision_within_manifest() && self.cold_precision_within_manifest()
    }

    pub fn precision_exceeds_manifest(self) -> bool {
        !self.precision_within_manifest()
    }

    pub fn execution_cold_kv_not_wider_than_hot(self) -> bool {
        self.execution_cold_kv_precision_bits <= self.execution_hot_kv_precision_bits
    }

    pub fn manifest_kv_capacity_missing(self) -> bool {
        self.import_enabled_without_capacity() || self.export_enabled_without_capacity()
    }

    pub fn manifest_limits_are_consistent(self) -> bool {
        !self.manifest_kv_capacity_missing()
    }

    pub fn execution_kv_contract_failure(self) -> bool {
        !self.can_use_execution_kv_contract()
    }

    pub fn has_import_capacity(self) -> bool {
        self.max_kv_import_blocks > 0
    }

    pub fn has_export_capacity(self) -> bool {
        self.max_kv_export_blocks > 0
    }

    pub fn manifest_hot_precision_is_valid(self) -> bool {
        valid_quantization_width(self.manifest_hot_kv_precision_bits)
    }

    pub fn manifest_cold_precision_is_valid(self) -> bool {
        valid_quantization_width(self.manifest_cold_kv_precision_bits)
    }

    pub fn execution_hot_precision_is_valid(self) -> bool {
        valid_quantization_width(self.execution_hot_kv_precision_bits)
    }

    pub fn execution_cold_precision_is_valid(self) -> bool {
        valid_quantization_width(self.execution_cold_kv_precision_bits)
    }

    pub fn kv_capacity_signal_component_count(self) -> usize {
        usize::from(self.kv_import_enabled)
            + usize::from(self.kv_export_enabled)
            + usize::from(self.has_import_capacity())
            + usize::from(self.has_export_capacity())
    }

    pub fn kv_prefetch_signal_component_count(self) -> usize {
        usize::from(self.execution_requests_kv_import())
            + usize::from(
                self.execution_requests_kv_import() && self.kv_prefetch_within_manifest_limit(),
            )
    }

    pub fn precision_signal_component_count(self) -> usize {
        usize::from(self.manifest_hot_precision_is_valid())
            + usize::from(self.manifest_cold_precision_is_valid())
            + usize::from(self.execution_hot_precision_is_valid())
            + usize::from(self.execution_cold_precision_is_valid())
            + usize::from(self.precision_within_manifest())
            + usize::from(self.execution_cold_kv_not_wider_than_hot())
    }

    pub fn execution_contract_signal_component_count(self) -> usize {
        self.kv_capacity_signal_component_count()
            .saturating_add(self.kv_prefetch_signal_component_count())
            .saturating_add(self.precision_signal_component_count())
    }

    pub fn has_execution_contract_signals(self) -> bool {
        self.execution_contract_signal_component_count() > 0
    }

    pub fn import_capacity_problem_component_count(self) -> usize {
        usize::from(self.import_enabled_without_capacity())
    }

    pub fn export_capacity_problem_component_count(self) -> usize {
        usize::from(self.export_enabled_without_capacity())
    }

    pub fn kv_capacity_problem_component_count(self) -> usize {
        self.import_capacity_problem_component_count()
            + self.export_capacity_problem_component_count()
    }

    pub fn disabled_import_request_component_count(self) -> usize {
        usize::from(self.execution_requests_disabled_kv_import())
    }

    pub fn kv_prefetch_limit_component_count(self) -> usize {
        usize::from(self.kv_prefetch_exceeds_import_limit())
    }

    pub fn kv_prefetch_problem_component_count(self) -> usize {
        self.disabled_import_request_component_count() + self.kv_prefetch_limit_component_count()
    }

    pub fn hot_precision_problem_component_count(self) -> usize {
        usize::from(!self.hot_precision_within_manifest())
    }

    pub fn cold_precision_problem_component_count(self) -> usize {
        usize::from(!self.cold_precision_within_manifest())
    }

    pub fn cold_precision_inversion_component_count(self) -> usize {
        usize::from(!self.execution_cold_kv_not_wider_than_hot())
    }

    pub fn precision_problem_component_count(self) -> usize {
        self.hot_precision_problem_component_count()
            + self.cold_precision_problem_component_count()
            + self.cold_precision_inversion_component_count()
    }

    pub fn execution_contract_problem_component_count(self) -> usize {
        self.kv_capacity_problem_component_count()
            .saturating_add(self.kv_prefetch_problem_component_count())
            .saturating_add(self.precision_problem_component_count())
    }

    pub fn has_execution_contract_problem_components(self) -> bool {
        self.execution_contract_problem_component_count() > 0
    }

    pub fn execution_device_signal_component_count(self) -> usize {
        self.execution_contract_signal_component_count()
    }

    pub fn has_execution_device_signals(self) -> bool {
        self.execution_device_signal_component_count() > 0
    }

    pub fn execution_device_blocker_component_count(self) -> usize {
        self.execution_contract_problem_component_count()
    }

    pub fn has_execution_device_blockers(self) -> bool {
        self.execution_device_blocker_component_count() > 0
    }

    pub fn runtime_manifest_execution_device_commit_signal_component_count(self) -> usize {
        self.execution_device_signal_component_count()
    }

    pub fn has_runtime_manifest_execution_device_commit_signals(self) -> bool {
        self.runtime_manifest_execution_device_commit_signal_component_count() > 0
    }

    pub fn runtime_manifest_execution_device_commit_blocker_component_count(self) -> usize {
        self.execution_device_blocker_component_count()
    }

    pub fn has_runtime_manifest_execution_device_commit_blockers(self) -> bool {
        self.runtime_manifest_execution_device_commit_blocker_component_count() > 0
    }

    pub fn execution_contract_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .kv_capacity_signal_component_count()
            .saturating_add(self.kv_prefetch_signal_component_count())
            .saturating_add(self.precision_signal_component_count());
        let expected_problem_count = self
            .kv_capacity_problem_component_count()
            .saturating_add(self.kv_prefetch_problem_component_count())
            .saturating_add(self.precision_problem_component_count());

        self.execution_contract_signal_component_count() == expected_signal_count
            && self.has_execution_contract_signals() == (expected_signal_count > 0)
            && self.execution_contract_problem_component_count() == expected_problem_count
            && self.has_execution_contract_problem_components() == (expected_problem_count > 0)
            && self.can_use_execution_kv_contract() == (expected_problem_count == 0)
    }

    pub fn execution_contract_shape_is_clean(self) -> bool {
        !self.has_execution_contract_problem_components()
            && self.execution_contract_accounting_is_consistent()
    }

    pub fn execution_device_accounting_is_consistent(self) -> bool {
        self.execution_contract_accounting_is_consistent()
            && self.execution_device_signal_component_count()
                == self.execution_contract_signal_component_count()
            && self.has_execution_device_signals()
                == (self.execution_device_signal_component_count() > 0)
            && self.execution_device_blocker_component_count()
                == self.execution_contract_problem_component_count()
            && self.has_execution_device_blockers()
                == (self.execution_device_blocker_component_count() > 0)
    }

    pub fn runtime_manifest_execution_device_commit_accounting_is_consistent(self) -> bool {
        self.execution_device_accounting_is_consistent()
            && self.runtime_manifest_execution_device_commit_signal_component_count()
                == self.execution_device_signal_component_count()
            && self.has_runtime_manifest_execution_device_commit_signals()
                == (self.runtime_manifest_execution_device_commit_signal_component_count() > 0)
            && self.runtime_manifest_execution_device_commit_blocker_component_count()
                == self.execution_device_blocker_component_count()
            && self.has_runtime_manifest_execution_device_commit_blockers()
                == (self.runtime_manifest_execution_device_commit_blocker_component_count() > 0)
    }

    pub fn runtime_manifest_execution_device_commit_is_clean(self) -> bool {
        !self.has_runtime_manifest_execution_device_commit_blockers()
            && self.runtime_manifest_execution_device_commit_accounting_is_consistent()
    }

    pub fn execution_device_commit_is_clean(self) -> bool {
        self.runtime_manifest_execution_device_commit_is_clean()
    }

    pub fn can_commit_manifest_execution_device_gate(self) -> bool {
        self.runtime_manifest_execution_device_commit_is_clean()
    }

    pub fn runtime_manifest_execution_device_commit_action(
        self,
    ) -> RuntimeManifestExecutionDeviceCommitAction {
        if self.can_commit_manifest_execution_device_gate() {
            RuntimeManifestExecutionDeviceCommitAction::CommitManifestExecutionDevice
        } else {
            RuntimeManifestExecutionDeviceCommitAction::ReturnRuntimeFailure
        }
    }

    pub fn can_use_execution_kv_contract(self) -> bool {
        self.manifest_limits_are_consistent()
            && self.kv_prefetch_within_manifest_limit()
            && self.precision_within_manifest()
            && self.execution_cold_kv_not_wider_than_hot()
    }
}

impl RuntimeDeviceHandoffReadinessSummary {
    pub fn new(
        hardware: HardwareRuntimeReadinessSummary,
        runtime_clamp: AdapterRuntimeClampSummary,
        manifest_execution: RuntimeManifestExecutionCompatibilitySummary,
    ) -> Self {
        Self {
            hardware,
            runtime_clamp,
            manifest_execution,
            hardware_signal_component_count: hardware.hardware_runtime_signal_component_count(),
            runtime_clamp_signal_component_count: runtime_clamp
                .runtime_clamp_commit_signal_component_count(),
            manifest_execution_signal_component_count: manifest_execution
                .runtime_manifest_execution_device_commit_signal_component_count(),
            hardware_blocker_component_count: hardware.hardware_runtime_blocker_component_count(),
            runtime_clamp_blocker_component_count: runtime_clamp
                .runtime_clamp_commit_blocker_component_count(),
            manifest_execution_blocker_component_count: manifest_execution
                .runtime_manifest_execution_device_commit_blocker_component_count(),
        }
    }

    pub fn stage_order() -> [RuntimeDeviceHandoffStage; 3] {
        [
            RuntimeDeviceHandoffStage::HardwareRuntime,
            RuntimeDeviceHandoffStage::RuntimeClamp,
            RuntimeDeviceHandoffStage::ManifestExecutionDevice,
        ]
    }

    pub fn hardware_ready(self) -> bool {
        self.hardware.can_commit_hardware_runtime()
    }

    pub fn runtime_clamp_ready(self) -> bool {
        self.runtime_clamp.can_commit_runtime_clamp()
    }

    pub fn manifest_execution_ready(self) -> bool {
        self.manifest_execution
            .can_commit_manifest_execution_device_gate()
    }

    pub fn stage_ready(self, stage: RuntimeDeviceHandoffStage) -> bool {
        match stage {
            RuntimeDeviceHandoffStage::HardwareRuntime => self.hardware_ready(),
            RuntimeDeviceHandoffStage::RuntimeClamp => self.runtime_clamp_ready(),
            RuntimeDeviceHandoffStage::ManifestExecutionDevice => self.manifest_execution_ready(),
        }
    }

    pub fn stage_signal_component_count(self, stage: RuntimeDeviceHandoffStage) -> usize {
        match stage {
            RuntimeDeviceHandoffStage::HardwareRuntime => self.hardware_signal_component_count,
            RuntimeDeviceHandoffStage::RuntimeClamp => self.runtime_clamp_signal_component_count,
            RuntimeDeviceHandoffStage::ManifestExecutionDevice => {
                self.manifest_execution_signal_component_count
            }
        }
    }

    pub fn stage_blocker_component_count(self, stage: RuntimeDeviceHandoffStage) -> usize {
        match stage {
            RuntimeDeviceHandoffStage::HardwareRuntime => self.hardware_blocker_component_count,
            RuntimeDeviceHandoffStage::RuntimeClamp => self.runtime_clamp_blocker_component_count,
            RuntimeDeviceHandoffStage::ManifestExecutionDevice => {
                self.manifest_execution_blocker_component_count
            }
        }
    }

    pub fn first_unready_stage(self) -> Option<RuntimeDeviceHandoffStage> {
        Self::stage_order()
            .into_iter()
            .find(|stage| !self.stage_ready(*stage))
    }

    pub fn first_blocking_stage(self) -> Option<RuntimeDeviceHandoffStage> {
        Self::stage_order()
            .into_iter()
            .find(|stage| self.stage_blocker_component_count(*stage) > 0)
    }

    pub fn component_accounting_drift_count(self) -> usize {
        usize::from(!self.runtime_device_handoff_accounting_is_consistent())
    }

    pub fn runtime_device_handoff_signal_component_count(self) -> usize {
        self.hardware_signal_component_count
            .saturating_add(self.runtime_clamp_signal_component_count)
            .saturating_add(self.manifest_execution_signal_component_count)
    }

    pub fn has_runtime_device_handoff_signals(self) -> bool {
        self.runtime_device_handoff_signal_component_count() > 0
    }

    pub fn runtime_device_handoff_blocker_component_count(self) -> usize {
        self.hardware_blocker_component_count
            .saturating_add(self.runtime_clamp_blocker_component_count)
            .saturating_add(self.manifest_execution_blocker_component_count)
    }

    pub fn has_runtime_device_handoff_blockers(self) -> bool {
        self.runtime_device_handoff_blocker_component_count() > 0
    }

    pub fn runtime_device_handoff_problem_component_count(self) -> usize {
        self.runtime_device_handoff_blocker_component_count()
            .saturating_add(self.component_accounting_drift_count())
    }

    pub fn has_runtime_device_handoff_problem_components(self) -> bool {
        self.runtime_device_handoff_problem_component_count() > 0
    }

    pub fn failure_report(self) -> Option<RuntimeFailureReport> {
        let component_count = self.runtime_device_handoff_problem_component_count();
        if component_count == 0 {
            None
        } else {
            let stage = self
                .first_blocking_stage()
                .or_else(|| self.first_unready_stage())
                .map(RuntimeDeviceHandoffStage::label)
                .unwrap_or("accounting");
            Some(RuntimeFailureReport::contract_violation(format!(
                "runtime device handoff failed: stage={stage} components={component_count}"
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

    pub fn runtime_device_handoff_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .hardware_signal_component_count
            .saturating_add(self.runtime_clamp_signal_component_count)
            .saturating_add(self.manifest_execution_signal_component_count);
        let expected_blocker_count = self
            .hardware_blocker_component_count
            .saturating_add(self.runtime_clamp_blocker_component_count)
            .saturating_add(self.manifest_execution_blocker_component_count);

        self.hardware.hardware_runtime_accounting_is_consistent()
            && self
                .runtime_clamp
                .runtime_clamp_commit_accounting_is_consistent()
            && self
                .manifest_execution
                .runtime_manifest_execution_device_commit_accounting_is_consistent()
            && self.hardware_signal_component_count
                == self.hardware.hardware_runtime_signal_component_count()
            && self.runtime_clamp_signal_component_count
                == self
                    .runtime_clamp
                    .runtime_clamp_commit_signal_component_count()
            && self.manifest_execution_signal_component_count
                == self
                    .manifest_execution
                    .runtime_manifest_execution_device_commit_signal_component_count()
            && self.hardware_blocker_component_count
                == self.hardware.hardware_runtime_blocker_component_count()
            && self.runtime_clamp_blocker_component_count
                == self
                    .runtime_clamp
                    .runtime_clamp_commit_blocker_component_count()
            && self.manifest_execution_blocker_component_count
                == self
                    .manifest_execution
                    .runtime_manifest_execution_device_commit_blocker_component_count()
            && self.runtime_device_handoff_signal_component_count() == expected_signal_count
            && self.has_runtime_device_handoff_signals() == (expected_signal_count > 0)
            && self.runtime_device_handoff_blocker_component_count() == expected_blocker_count
            && self.has_runtime_device_handoff_blockers() == (expected_blocker_count > 0)
    }

    pub fn runtime_device_handoff_commit_is_clean(self) -> bool {
        !self.has_runtime_device_handoff_blockers()
            && self.runtime_device_handoff_accounting_is_consistent()
    }

    pub fn can_commit_runtime_device_handoff(self) -> bool {
        self.runtime_device_handoff_commit_is_clean()
            && self.hardware_ready()
            && self.runtime_clamp_ready()
            && self.manifest_execution_ready()
    }

    pub fn runtime_device_handoff_commit_action(self) -> RuntimeDeviceHandoffCommitAction {
        if self.can_commit_runtime_device_handoff() {
            RuntimeDeviceHandoffCommitAction::CommitDeviceHandoff
        } else {
            RuntimeDeviceHandoffCommitAction::ReturnRuntimeFailure
        }
    }

    pub fn commit_summary(self) -> RuntimeDeviceHandoffCommitSummary {
        RuntimeDeviceHandoffCommitSummary::new(self)
    }
}

impl RuntimeDeviceHandoffCommitSummary {
    pub fn new(readiness: RuntimeDeviceHandoffReadinessSummary) -> Self {
        let failure_reports = readiness.failure_reports();
        let primary_failure_report = failure_reports.first().cloned();
        let primary_failure_summary = primary_failure_report
            .as_ref()
            .map(|failure| failure.failure_summary());
        let failure_batch = RuntimeFailureReport::batch_summary(&failure_reports);
        let can_commit = readiness.can_commit_runtime_device_handoff();
        let failure_report_count = failure_reports.len();
        let can_format_runtime_failures = failure_batch.can_format_runtime_failures();
        let should_return_failure = !can_commit && failure_report_count > 0;
        let action = readiness.runtime_device_handoff_commit_action();

        Self {
            readiness,
            action,
            can_commit,
            should_return_failure,
            first_unready_stage: readiness.first_unready_stage(),
            first_blocking_stage: readiness.first_blocking_stage(),
            failure_reports,
            primary_failure_report,
            primary_failure_summary,
            failure_batch,
            failure_report_count,
            can_format_runtime_failures,
            total_signal_component_count: readiness.runtime_device_handoff_signal_component_count(),
            total_blocker_component_count: readiness
                .runtime_device_handoff_blocker_component_count(),
            component_accounting_consistent: readiness
                .runtime_device_handoff_accounting_is_consistent(),
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

    pub fn failure_return_summary(&self) -> ManifestFailureReturnSummary {
        ManifestFailureReturnSummary::new(
            ManifestFailureReturnSource::RuntimeDeviceHandoff,
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

    pub fn runtime_failure_return_report(&self) -> Option<ManifestFailureReturnReport> {
        if self.failure_return_summary().can_return_runtime_failure() {
            self.primary_failure_report.clone().map(|failure| {
                ManifestFailureReturnReport::new(
                    ManifestFailureReturnSource::RuntimeDeviceHandoff,
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
        self.can_commit == self.readiness.can_commit_runtime_device_handoff()
            && self.should_return_failure == (!self.can_commit && self.failure_report_count > 0)
            && self.action == self.readiness.runtime_device_handoff_commit_action()
            && self.action_can_commit() == self.can_commit
            && self.action_should_return_failure() == self.should_return_failure
            && self.first_unready_stage == self.readiness.first_unready_stage()
            && self.first_blocking_stage == self.readiness.first_blocking_stage()
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
                    .runtime_device_handoff_signal_component_count()
            && self.total_blocker_component_count
                == self
                    .readiness
                    .runtime_device_handoff_blocker_component_count()
            && self.component_accounting_consistent
                == self
                    .readiness
                    .runtime_device_handoff_accounting_is_consistent()
    }

    pub fn can_commit_runtime_device_handoff(&self) -> bool {
        self.can_commit && self.commit_decision_accounting_is_consistent()
    }

    pub fn should_return_runtime_failure(&self) -> bool {
        self.should_return_failure && self.commit_decision_accounting_is_consistent()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeManifestAbiSummary {
    pub native_context_window: usize,
    pub embedding_dimensions: usize,
    pub layer_count: usize,
    pub hidden_size: usize,
    pub attention_heads: usize,
    pub kv_heads: usize,
    pub local_window_tokens: usize,
    pub kv_import_enabled: bool,
    pub kv_export_enabled: bool,
    pub max_kv_import_blocks: usize,
    pub max_kv_export_blocks: usize,
    pub hot_kv_precision_bits: u8,
    pub cold_kv_precision_bits: u8,
    pub weight_precision_bits: Option<u8>,
    pub supported_adapter_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeManifestAbiCommitAction {
    CommitRuntimeManifestAdapter,
    ReturnRuntimeFailure,
}

impl RuntimeManifestAbiCommitAction {
    pub fn can_commit(self) -> bool {
        matches!(self, Self::CommitRuntimeManifestAdapter)
    }

    pub fn should_return_failure(self) -> bool {
        matches!(self, Self::ReturnRuntimeFailure)
    }
}

impl RuntimeManifestAbiSummary {
    pub fn has_known_context(self) -> bool {
        self.native_context_window > 0
    }

    pub fn has_embedding_dimensions(self) -> bool {
        self.embedding_dimensions > 0
    }

    pub fn supports_kv_exchange(self) -> bool {
        self.kv_import_enabled || self.kv_export_enabled
    }

    pub fn kv_exchange_block_capacity(self) -> usize {
        self.max_kv_import_blocks
            .saturating_add(self.max_kv_export_blocks)
    }

    pub fn local_window_fits_context(self) -> bool {
        self.native_context_window == 0 || self.local_window_tokens <= self.native_context_window
    }

    pub fn uses_compressed_hot_kv(self) -> bool {
        self.hot_kv_precision_bits < 8
    }

    pub fn cold_kv_not_wider_than_hot(self) -> bool {
        self.cold_kv_precision_bits <= self.hot_kv_precision_bits
    }

    pub fn has_weight_quantization(self) -> bool {
        self.weight_precision_bits.is_some()
    }

    pub fn has_layers(self) -> bool {
        self.layer_count > 0
    }

    pub fn has_hidden_size(self) -> bool {
        self.hidden_size > 0
    }

    pub fn has_attention_heads(self) -> bool {
        self.attention_heads > 0
    }

    pub fn has_kv_heads(self) -> bool {
        self.kv_heads > 0
    }

    pub fn has_local_window(self) -> bool {
        self.local_window_tokens > 0
    }

    pub fn kv_heads_fit_attention(self) -> bool {
        self.attention_heads == 0 || self.kv_heads <= self.attention_heads
    }

    pub fn has_import_capacity(self) -> bool {
        self.max_kv_import_blocks > 0
    }

    pub fn has_export_capacity(self) -> bool {
        self.max_kv_export_blocks > 0
    }

    pub fn import_limits_match_capability(self) -> bool {
        self.kv_import_enabled == self.has_import_capacity()
    }

    pub fn export_limits_match_capability(self) -> bool {
        self.kv_export_enabled == self.has_export_capacity()
    }

    pub fn kv_limits_match_capabilities(self) -> bool {
        self.import_limits_match_capability() && self.export_limits_match_capability()
    }

    pub fn has_valid_hot_kv_precision(self) -> bool {
        valid_quantization_width(self.hot_kv_precision_bits)
    }

    pub fn has_valid_cold_kv_precision(self) -> bool {
        valid_quantization_width(self.cold_kv_precision_bits)
    }

    pub fn has_valid_weight_precision(self) -> bool {
        self.weight_precision_bits
            .is_none_or(valid_quantization_width)
    }

    pub fn has_supported_adapters(self) -> bool {
        self.supported_adapter_count > 0
    }

    pub fn context_abi_signal_component_count(self) -> usize {
        usize::from(self.has_known_context()) + usize::from(self.has_embedding_dimensions())
    }

    pub fn transformer_abi_signal_component_count(self) -> usize {
        usize::from(self.has_layers())
            + usize::from(self.has_hidden_size())
            + usize::from(self.has_attention_heads())
            + usize::from(self.has_kv_heads())
            + usize::from(self.has_local_window())
            + usize::from(self.local_window_fits_context())
    }

    pub fn kv_exchange_abi_signal_component_count(self) -> usize {
        usize::from(self.kv_import_enabled)
            + usize::from(self.kv_export_enabled)
            + usize::from(self.has_import_capacity())
            + usize::from(self.has_export_capacity())
    }

    pub fn quantization_abi_signal_component_count(self) -> usize {
        usize::from(self.has_valid_hot_kv_precision())
            + usize::from(self.has_valid_cold_kv_precision())
            + usize::from(self.has_valid_hot_kv_precision() && self.uses_compressed_hot_kv())
            + usize::from(
                self.has_valid_hot_kv_precision()
                    && self.has_valid_cold_kv_precision()
                    && self.cold_kv_not_wider_than_hot(),
            )
            + usize::from(self.has_weight_quantization())
    }

    pub fn adapter_abi_signal_component_count(self) -> usize {
        usize::from(self.has_supported_adapters())
    }

    pub fn abi_signal_component_count(self) -> usize {
        self.context_abi_signal_component_count()
            .saturating_add(self.transformer_abi_signal_component_count())
            .saturating_add(self.kv_exchange_abi_signal_component_count())
            .saturating_add(self.quantization_abi_signal_component_count())
            .saturating_add(self.adapter_abi_signal_component_count())
    }

    pub fn has_abi_signals(self) -> bool {
        self.abi_signal_component_count() > 0
    }

    pub fn context_abi_problem_component_count(self) -> usize {
        usize::from(!self.has_known_context()) + usize::from(!self.has_embedding_dimensions())
    }

    pub fn transformer_abi_problem_component_count(self) -> usize {
        usize::from(!self.has_layers())
            + usize::from(!self.has_hidden_size())
            + usize::from(!self.has_attention_heads())
            + usize::from(!self.has_kv_heads())
            + usize::from(!self.has_local_window())
            + usize::from(!self.local_window_fits_context())
            + usize::from(!self.kv_heads_fit_attention())
    }

    pub fn kv_exchange_abi_problem_component_count(self) -> usize {
        usize::from(!self.import_limits_match_capability())
            + usize::from(!self.export_limits_match_capability())
    }

    pub fn quantization_abi_problem_component_count(self) -> usize {
        usize::from(!self.has_valid_hot_kv_precision())
            + usize::from(!self.has_valid_cold_kv_precision())
            + usize::from(!self.has_valid_weight_precision())
            + usize::from(!self.cold_kv_not_wider_than_hot())
    }

    pub fn adapter_abi_problem_component_count(self) -> usize {
        usize::from(!self.has_supported_adapters())
    }

    pub fn abi_problem_component_count(self) -> usize {
        self.context_abi_problem_component_count()
            .saturating_add(self.transformer_abi_problem_component_count())
            .saturating_add(self.kv_exchange_abi_problem_component_count())
            .saturating_add(self.quantization_abi_problem_component_count())
            .saturating_add(self.adapter_abi_problem_component_count())
    }

    pub fn has_abi_problem_components(self) -> bool {
        self.abi_problem_component_count() > 0
    }

    pub fn manifest_adapter_signal_component_count(self) -> usize {
        self.abi_signal_component_count()
    }

    pub fn has_manifest_adapter_signals(self) -> bool {
        self.manifest_adapter_signal_component_count() > 0
    }

    pub fn manifest_adapter_blocker_component_count(self) -> usize {
        self.abi_problem_component_count()
    }

    pub fn has_manifest_adapter_blockers(self) -> bool {
        self.manifest_adapter_blocker_component_count() > 0
    }

    pub fn abi_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .context_abi_signal_component_count()
            .saturating_add(self.transformer_abi_signal_component_count())
            .saturating_add(self.kv_exchange_abi_signal_component_count())
            .saturating_add(self.quantization_abi_signal_component_count())
            .saturating_add(self.adapter_abi_signal_component_count());
        let expected_problem_count = self
            .context_abi_problem_component_count()
            .saturating_add(self.transformer_abi_problem_component_count())
            .saturating_add(self.kv_exchange_abi_problem_component_count())
            .saturating_add(self.quantization_abi_problem_component_count())
            .saturating_add(self.adapter_abi_problem_component_count());

        self.abi_signal_component_count() == expected_signal_count
            && self.has_abi_signals() == (expected_signal_count > 0)
            && self.abi_problem_component_count() == expected_problem_count
            && self.has_abi_problem_components() == (expected_problem_count > 0)
    }

    pub fn abi_shape_is_clean(self) -> bool {
        !self.has_abi_problem_components() && self.abi_accounting_is_consistent()
    }

    pub fn manifest_adapter_accounting_is_consistent(self) -> bool {
        self.abi_accounting_is_consistent()
            && self.manifest_adapter_signal_component_count() == self.abi_signal_component_count()
            && self.has_manifest_adapter_signals()
                == (self.manifest_adapter_signal_component_count() > 0)
            && self.manifest_adapter_blocker_component_count() == self.abi_problem_component_count()
            && self.has_manifest_adapter_blockers()
                == (self.manifest_adapter_blocker_component_count() > 0)
    }

    pub fn manifest_adapter_commit_is_clean(self) -> bool {
        !self.has_manifest_adapter_blockers() && self.manifest_adapter_accounting_is_consistent()
    }

    pub fn can_commit_runtime_manifest_adapter(self) -> bool {
        self.manifest_adapter_commit_is_clean()
    }

    pub fn runtime_manifest_adapter_commit_action(self) -> RuntimeManifestAbiCommitAction {
        if self.can_commit_runtime_manifest_adapter() {
            RuntimeManifestAbiCommitAction::CommitRuntimeManifestAdapter
        } else {
            RuntimeManifestAbiCommitAction::ReturnRuntimeFailure
        }
    }

    pub fn can_use_runtime_manifest_abi(self) -> bool {
        self.abi_shape_is_clean()
    }
}

impl RuntimeManifestValidation {
    pub fn passed(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn warnings_only(&self) -> bool {
        self.errors.is_empty() && !self.warnings.is_empty()
    }

    pub fn validation_summary(&self) -> RuntimeManifestValidationSummary {
        RuntimeManifestValidationSummary {
            passed: self.passed(),
            warning_count: self.warnings.len(),
            error_count: self.errors.len(),
            warnings_only: self.warnings_only(),
            failure_report_count: usize::from(!self.errors.is_empty()),
        }
    }

    pub fn failure_reports(&self) -> Vec<RuntimeFailureReport> {
        if self.errors.is_empty() {
            Vec::new()
        } else {
            vec![RuntimeFailureReport::contract_violation(
                acceptance_message("runtime manifest validation failed", &self.errors),
            )]
        }
    }

    pub fn failure_batch_summary(&self) -> RuntimeFailureBatchSummary {
        RuntimeFailureReport::batch_summary(&self.failure_reports())
    }

    pub fn primary_failure_report(&self) -> Option<RuntimeFailureReport> {
        self.failure_reports().into_iter().next()
    }

    pub fn primary_failure_summary(&self) -> Option<RuntimeFailureSummary> {
        self.primary_failure_report()
            .map(|failure| failure.failure_summary())
    }

    pub fn commit_summary(&self) -> RuntimeManifestValidationCommitSummary {
        RuntimeManifestValidationCommitSummary::new(self)
    }
}

fn acceptance_message(prefix: &str, violations: &[String]) -> String {
    if violations.is_empty() {
        prefix.to_owned()
    } else {
        format!("{prefix}: {}", violations.join("; "))
    }
}

fn valid_quantization_width(bits: u8) -> bool {
    matches!(bits, 4 | 8)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeManifestDigest {
    pub metadata: RuntimeMetadata,
    pub architecture: TransformerRuntimeArchitecture,
    pub kv_policy: RuntimeKvPolicy,
    pub quantization: RuntimeQuantizationPolicy,
    pub supported_adapters: Vec<RuntimeAdapter>,
}

impl RuntimeManifestDigest {
    pub fn self_developed(
        model_id: impl Into<String>,
        tokenizer: impl Into<String>,
        native_context_window: usize,
        embedding_dimensions: usize,
    ) -> Self {
        let metadata = RuntimeMetadata::new(
            model_id,
            tokenizer,
            native_context_window,
            embedding_dimensions,
        );
        Self::from_metadata(metadata).with_kv_policy(RuntimeKvPolicy::import_export())
    }

    pub fn from_metadata(metadata: RuntimeMetadata) -> Self {
        let architecture = TransformerRuntimeArchitecture::from_metadata(&metadata);
        let kv_policy = RuntimeKvPolicy::from_capabilities(
            metadata.supports_kv_import,
            metadata.supports_kv_export,
        );
        let quantization = RuntimeQuantizationPolicy::from_metadata(&metadata);

        Self {
            metadata,
            architecture,
            kv_policy,
            quantization,
            supported_adapters: default_runtime_adapters(),
        }
    }

    pub fn with_architecture(mut self, architecture: TransformerRuntimeArchitecture) -> Self {
        self.architecture = architecture;
        self
    }

    pub fn with_kv_policy(mut self, kv_policy: RuntimeKvPolicy) -> Self {
        self.kv_policy = kv_policy;
        self.metadata.supports_kv_import = kv_policy.import_enabled;
        self.metadata.supports_kv_export = kv_policy.export_enabled;
        self
    }

    pub fn with_quantization(mut self, quantization: RuntimeQuantizationPolicy) -> Self {
        self.quantization = quantization;
        self
    }

    pub fn with_supported_adapters(mut self, supported_adapters: Vec<RuntimeAdapter>) -> Self {
        self.supported_adapters = supported_adapters;
        self
    }

    pub fn runtime_metadata(&self) -> RuntimeMetadata {
        self.metadata
            .clone()
            .with_kv_exchange(self.kv_policy.import_enabled, self.kv_policy.export_enabled)
            .with_kv_limits(
                self.kv_policy.max_import_blocks,
                self.kv_policy.max_export_blocks,
            )
            .with_kv_precision(
                self.quantization.hot_kv.width(),
                self.quantization.cold_kv.width(),
            )
    }

    pub fn preferred_adapter_for(
        &self,
        execution: &AdapterExecutionContext,
    ) -> Option<RuntimeAdapter> {
        let supported_by_execution = execution
            .adapters
            .iter()
            .copied()
            .find(|adapter| self.supported_adapters.contains(adapter));

        if execution.adapters.is_empty() {
            supported_by_execution.or_else(|| self.supported_adapters.first().copied())
        } else {
            supported_by_execution
        }
    }

    pub fn preferred_adapter_with_observations(
        &self,
        execution: &AdapterExecutionContext,
        observations: &[AdapterObservation],
    ) -> Option<RuntimeAdapter> {
        let fallback = self.preferred_adapter_for(execution);
        self.preferred_observed_adapter(execution, observations)
            .or(fallback)
    }

    pub fn adapter_compatibility_summary(
        &self,
        execution: &AdapterExecutionContext,
        observations: &[AdapterObservation],
    ) -> RuntimeManifestAdapterCompatibilitySummary {
        let compatible_adapter_count = execution
            .adapters
            .iter()
            .filter(|adapter| self.supported_adapters.contains(adapter))
            .count();
        let compatible_observation_count = observations
            .iter()
            .filter(|observation| observation.score >= 0.50)
            .filter(|observation| execution.adapters.contains(&observation.adapter))
            .filter(|observation| self.supported_adapters.contains(&observation.adapter))
            .count();
        let preferred_adapter = self.preferred_adapter_for(execution);
        let preferred_observed_adapter = self.preferred_observed_adapter(execution, observations);

        RuntimeManifestAdapterCompatibilitySummary {
            supported_adapter_count: self.supported_adapters.len(),
            execution_adapter_count: execution.adapters.len(),
            compatible_adapter_count,
            observation_count: observations.len(),
            compatible_observation_count,
            preferred_adapter,
            preferred_observed_adapter,
            selected_adapter: preferred_observed_adapter.or(preferred_adapter),
        }
    }

    pub fn execution_compatibility_summary(
        &self,
        execution: &AdapterExecutionContext,
    ) -> RuntimeManifestExecutionCompatibilitySummary {
        RuntimeManifestExecutionCompatibilitySummary {
            kv_import_enabled: self.kv_policy.import_enabled,
            kv_export_enabled: self.kv_policy.export_enabled,
            max_kv_import_blocks: self.kv_policy.max_import_blocks,
            max_kv_export_blocks: self.kv_policy.max_export_blocks,
            execution_kv_prefetch_blocks: execution.kv_prefetch_blocks,
            manifest_hot_kv_precision_bits: self.quantization.hot_kv.width(),
            manifest_cold_kv_precision_bits: self.quantization.cold_kv.width(),
            execution_hot_kv_precision_bits: execution.hot_kv_precision_bits,
            execution_cold_kv_precision_bits: execution.cold_kv_precision_bits,
        }
    }

    pub fn abi_summary(&self) -> RuntimeManifestAbiSummary {
        let metadata = self.runtime_metadata();

        RuntimeManifestAbiSummary {
            native_context_window: metadata.native_context_window,
            embedding_dimensions: metadata.embedding_dimensions,
            layer_count: self.architecture.layer_count,
            hidden_size: self.architecture.hidden_size,
            attention_heads: self.architecture.attention_heads,
            kv_heads: self.architecture.kv_heads,
            local_window_tokens: self.architecture.local_window_tokens,
            kv_import_enabled: metadata.supports_kv_import,
            kv_export_enabled: metadata.supports_kv_export,
            max_kv_import_blocks: metadata.max_kv_import_blocks,
            max_kv_export_blocks: metadata.max_kv_export_blocks,
            hot_kv_precision_bits: metadata.hot_kv_precision_bits,
            cold_kv_precision_bits: metadata.cold_kv_precision_bits,
            weight_precision_bits: self.quantization.weights.map(QuantizationBits::width),
            supported_adapter_count: self.supported_adapters.len(),
        }
    }

    pub fn validate(&self) -> RuntimeManifestValidation {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        if self.metadata.model_id.trim().is_empty() {
            errors.push("model_id must not be empty".to_owned());
        }
        if self.metadata.tokenizer.trim().is_empty() {
            errors.push("tokenizer must not be empty".to_owned());
        }
        if self.metadata.native_context_window == 0 {
            errors.push("native_context_window must be greater than zero".to_owned());
        }
        if self.metadata.embedding_dimensions == 0 {
            errors.push("embedding_dimensions must be greater than zero".to_owned());
        }
        errors.extend(self.architecture.validation_errors());
        if self.architecture.local_window_tokens > self.metadata.native_context_window
            && self.metadata.native_context_window > 0
        {
            errors.push("local_window_tokens must not exceed native_context_window".to_owned());
        }
        if self.supported_adapters.is_empty() {
            warnings.push(
                "supported_adapters is empty; runtime will not advertise an execution lane"
                    .to_owned(),
            );
        }
        if self.metadata.supports_kv_import != self.kv_policy.import_enabled {
            warnings.push("metadata supports_kv_import differs from manifest kv_policy".to_owned());
        }
        if self.metadata.supports_kv_export != self.kv_policy.export_enabled {
            warnings.push("metadata supports_kv_export differs from manifest kv_policy".to_owned());
        }
        if self.quantization.cold_kv.width() > self.quantization.hot_kv.width() {
            errors.push("cold_kv quantization width must not exceed hot_kv width".to_owned());
        }

        RuntimeManifestValidation { errors, warnings }
    }

    fn preferred_observed_adapter(
        &self,
        execution: &AdapterExecutionContext,
        observations: &[AdapterObservation],
    ) -> Option<RuntimeAdapter> {
        observations
            .iter()
            .filter(|observation| observation.score >= 0.50)
            .filter(|observation| execution.adapters.contains(&observation.adapter))
            .filter(|observation| self.supported_adapters.contains(&observation.adapter))
            .max_by(|left, right| {
                left.score
                    .total_cmp(&right.score)
                    .then_with(|| right.experience_id.cmp(&left.experience_id))
            })
            .map(|observation| observation.adapter)
    }
}

pub fn default_transformer_runtime_architecture(
    native_context_window: usize,
    embedding_dimensions: usize,
) -> TransformerRuntimeArchitecture {
    let native_context_window = native_context_window.max(1);
    let embedding_dimensions = embedding_dimensions.max(1);
    let heads = choose_head_count(embedding_dimensions);
    TransformerRuntimeArchitecture::new(
        24,
        embedding_dimensions,
        heads,
        heads,
        native_context_window.min(4_096),
    )
}

fn choose_head_count(hidden_size: usize) -> usize {
    [16, 12, 8, 6, 4, 2]
        .into_iter()
        .find(|heads| hidden_size % heads == 0)
        .unwrap_or(1)
}

fn default_runtime_adapters() -> Vec<RuntimeAdapter> {
    vec![
        RuntimeAdapter::PortableRust,
        RuntimeAdapter::CpuSimd,
        RuntimeAdapter::Wgpu,
        RuntimeAdapter::WebGpu,
        RuntimeAdapter::Vulkan,
        RuntimeAdapter::Metal,
        RuntimeAdapter::Cuda,
        RuntimeAdapter::Rocm,
        RuntimeAdapter::OneApi,
        RuntimeAdapter::DirectMl,
        RuntimeAdapter::CoreMl,
        RuntimeAdapter::Nnapi,
        RuntimeAdapter::Qnn,
        RuntimeAdapter::OpenVino,
        RuntimeAdapter::Cann,
        RuntimeAdapter::Mlu,
        RuntimeAdapter::Rknn,
        RuntimeAdapter::MultiDevice,
        RuntimeAdapter::CustomAccelerator,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::RuntimeFailureKind;
    use crate::hardware::{DeviceClass, HardwareAllocator, HardwareLoadSnapshot};
    use crate::profile::{HierarchyWeights, TaskProfile};

    fn assert_clean_manifest_failure_return(
        failure_return: ManifestFailureReturnSummary,
        source: ManifestFailureReturnSource,
    ) {
        assert_eq!(failure_return.source, source);
        assert_eq!(failure_return.source.label(), source.label());
        assert!(failure_return.can_commit);
        assert!(!failure_return.should_return_failure);
        assert!(!failure_return.has_primary_failure_summary);
        assert_eq!(failure_return.primary_failure_summary, None);
        assert_eq!(failure_return.failure_report_count, 0);
        assert!(!failure_return.has_failure_reports());
        assert!(!failure_return.can_format_runtime_failures);
        assert_eq!(failure_return.total_blocker_component_count, 0);
        assert!(!failure_return.has_blocker_components());
        assert!(failure_return.failure_return_accounting_is_consistent());
        assert!(!failure_return.can_return_runtime_failure());
    }

    fn assert_blocked_manifest_failure_return(
        failure_return: ManifestFailureReturnSummary,
        report: ManifestFailureReturnReport,
        source: ManifestFailureReturnSource,
        message_fragment: &str,
    ) {
        assert_eq!(failure_return.source, source);
        assert_eq!(failure_return.source.label(), source.label());
        assert!(!failure_return.can_commit);
        assert!(failure_return.should_return_failure);
        assert!(failure_return.has_primary_failure_summary);
        assert_eq!(failure_return.failure_report_count, 1);
        assert!(failure_return.has_failure_reports());
        assert!(failure_return.can_format_runtime_failures);
        assert!(failure_return.total_blocker_component_count > 0);
        assert!(failure_return.has_blocker_components());
        assert!(failure_return.failure_return_accounting_is_consistent());
        assert!(failure_return.can_return_runtime_failure());

        assert_eq!(report.source, source);
        assert_eq!(
            report.primary_failure_summary,
            failure_return
                .primary_failure_summary
                .expect("primary failure summary is projected")
        );
        assert_eq!(report.failure_batch, failure_return.failure_batch);
        assert_eq!(
            report.failure_report_count,
            failure_return.failure_report_count
        );
        assert_eq!(
            report.can_format_runtime_failures,
            failure_return.can_format_runtime_failures
        );
        assert_eq!(
            report.total_blocker_component_count,
            failure_return.total_blocker_component_count
        );
        assert!(report.backend_message().contains(message_fragment));
        assert!(report.diagnostics_note().contains(message_fragment));
        assert_eq!(report.inference_error().message, report.backend_message());
        assert!(report.can_use_manifest_failure_return_report());
    }

    #[test]
    fn manifest_failure_return_projection_covers_validation_and_handoff() {
        let clean_validation =
            RuntimeManifestDigest::self_developed("model", "tok", 4096, 2048).validate();
        let clean_validation_commit = clean_validation.commit_summary();
        assert_clean_manifest_failure_return(
            clean_validation_commit.failure_return_summary(),
            ManifestFailureReturnSource::ManifestValidation,
        );
        assert_eq!(
            clean_validation_commit.runtime_failure_return_report(),
            None
        );

        let warning_validation = RuntimeManifestDigest::self_developed("model", "tok", 4096, 2048)
            .with_supported_adapters(Vec::new())
            .validate();
        let warning_validation_commit = warning_validation.commit_summary();
        assert_clean_manifest_failure_return(
            warning_validation_commit.failure_return_summary(),
            ManifestFailureReturnSource::ManifestValidation,
        );
        assert_eq!(
            warning_validation_commit.runtime_failure_return_report(),
            None
        );

        let blocked_validation =
            RuntimeManifestDigest::from_metadata(RuntimeMetadata::new("", "", 0, 0))
                .with_architecture(TransformerRuntimeArchitecture::new(0, 10, 3, 4, 0))
                .with_quantization(RuntimeQuantizationPolicy {
                    hot_kv: QuantizationBits::Four,
                    cold_kv: QuantizationBits::Eight,
                    weights: None,
                })
                .validate();
        let blocked_validation_commit = blocked_validation.commit_summary();
        assert_blocked_manifest_failure_return(
            blocked_validation_commit.failure_return_summary(),
            blocked_validation_commit
                .runtime_failure_return_report()
                .expect("manifest validation failure return report"),
            ManifestFailureReturnSource::ManifestValidation,
            "runtime manifest validation failed",
        );

        let snapshot = HardwareLoadSnapshot::new(DeviceClass::Server, 0.20, 0.20, 0.20, 0.20);
        let plan = HardwareAllocator::new().plan(
            snapshot,
            TaskProfile::Coding,
            4096,
            HierarchyWeights::for_profile(TaskProfile::Coding),
        );
        let manifest = RuntimeManifestDigest::self_developed("model", "tok", 4096, 2048)
            .with_kv_policy(RuntimeKvPolicy::import_export().with_limits(4, 2))
            .with_quantization(RuntimeQuantizationPolicy {
                hot_kv: QuantizationBits::Eight,
                cold_kv: QuantizationBits::Four,
                weights: None,
            });
        let hardware = plan.runtime_readiness_summary(snapshot.snapshot_summary());
        let context = plan.adapter_execution_context();
        let runtime = manifest.runtime_metadata();
        let runtime_clamp = context.runtime_clamp_summary(&runtime);
        let clamped_context = context.clone().clamp_for_runtime(&runtime);
        let clean_manifest_execution = manifest.execution_compatibility_summary(&clamped_context);
        let clean_handoff_commit = RuntimeDeviceHandoffReadinessSummary::new(
            hardware,
            runtime_clamp,
            clean_manifest_execution,
        )
        .commit_summary();
        assert_clean_manifest_failure_return(
            clean_handoff_commit.failure_return_summary(),
            ManifestFailureReturnSource::RuntimeDeviceHandoff,
        );
        assert_eq!(clean_handoff_commit.runtime_failure_return_report(), None);

        let blocked_manifest =
            manifest.with_kv_policy(RuntimeKvPolicy::import_export().with_limits(0, 0));
        let blocked_manifest_execution = blocked_manifest.execution_compatibility_summary(&context);
        let blocked_handoff_commit = RuntimeDeviceHandoffReadinessSummary::new(
            hardware,
            runtime_clamp,
            blocked_manifest_execution,
        )
        .commit_summary();
        assert_blocked_manifest_failure_return(
            blocked_handoff_commit.failure_return_summary(),
            blocked_handoff_commit
                .runtime_failure_return_report()
                .expect("runtime device handoff failure return report"),
            ManifestFailureReturnSource::RuntimeDeviceHandoff,
            "runtime device handoff failed",
        );
    }

    #[test]
    fn manifest_digest_round_trips_runtime_metadata_capabilities() {
        let manifest = RuntimeManifestDigest::self_developed("noiron-dev", "tok", 65_536, 4096)
            .with_kv_policy(RuntimeKvPolicy::import_export().with_limits(12, 6))
            .with_quantization(RuntimeQuantizationPolicy {
                hot_kv: QuantizationBits::Four,
                cold_kv: QuantizationBits::Four,
                weights: Some(QuantizationBits::Eight),
            });

        let metadata = manifest.runtime_metadata();

        assert!(metadata.supports_kv_import);
        assert!(metadata.supports_kv_export);
        assert_eq!(metadata.max_kv_import_blocks, 12);
        assert_eq!(metadata.max_kv_export_blocks, 6);
        assert_eq!(metadata.hot_kv_precision_bits, 4);
        assert_eq!(metadata.cold_kv_precision_bits, 4);
        assert_eq!(manifest.architecture.local_window_tokens, 4096);
        assert!(manifest.validate().passed());
    }

    #[test]
    fn architecture_summary_reports_transformer_shape() {
        let architecture = TransformerRuntimeArchitecture::new(24, 4096, 16, 8, 4096);

        let summary = architecture.architecture_summary();

        assert_eq!(summary.layer_count, 24);
        assert_eq!(summary.hidden_size, 4096);
        assert_eq!(summary.attention_heads, 16);
        assert_eq!(summary.kv_heads, 8);
        assert_eq!(summary.local_window_tokens, 4096);
        assert_eq!(summary.attention_head_dim, Some(256));
        assert!(summary.has_layers());
        assert!(summary.has_hidden_size());
        assert!(summary.has_attention_heads());
        assert!(summary.has_kv_heads());
        assert!(summary.has_local_window());
        assert!(summary.has_integral_attention_head_dim());
        assert!(summary.kv_heads_fit_attention());
        assert!(summary.local_window_fits_context(8192));
        assert_eq!(summary.architecture_dimension_signal_component_count(), 5);
        assert_eq!(summary.attention_geometry_signal_component_count(), 2);
        assert_eq!(summary.architecture_signal_component_count(), 7);
        assert!(summary.has_architecture_signal_components());
        assert_eq!(summary.architecture_dimension_problem_component_count(), 0);
        assert_eq!(summary.attention_geometry_problem_component_count(), 0);
        assert_eq!(
            summary.local_window_context_problem_component_count(8192),
            0
        );
        assert_eq!(summary.architecture_problem_component_count(8192), 0);
        assert!(!summary.has_architecture_problem_components(8192));
        assert!(summary.architecture_accounting_is_consistent(8192));
        assert_eq!(
            summary.transformer_runtime_architecture_commit_signal_component_count(),
            7
        );
        assert_eq!(
            summary.transformer_runtime_architecture_commit_blocker_component_count(8192),
            0
        );
        assert!(summary.transformer_runtime_architecture_commit_accounting_is_consistent(8192));
        assert!(summary.transformer_runtime_architecture_commit_is_clean(8192));
        assert!(summary.shape_is_valid());
        assert!(summary.architecture_shape_is_clean(8192));
        assert!(summary.can_commit_transformer_runtime_architecture(8192));
        assert_eq!(
            summary.transformer_runtime_architecture_commit_action(8192),
            TransformerRuntimeArchitectureCommitAction::CommitTransformerRuntimeArchitecture
        );
        assert!(
            summary
                .transformer_runtime_architecture_commit_action(8192)
                .can_commit()
        );
        assert!(
            !summary
                .transformer_runtime_architecture_commit_action(8192)
                .should_return_failure()
        );
        assert!(summary.can_use_transformer_runtime_architecture(8192));
    }

    #[test]
    fn architecture_summary_reports_invalid_shape_boundaries() {
        let architecture = TransformerRuntimeArchitecture::new(0, 10, 3, 4, 8192);

        let summary = architecture.architecture_summary();

        assert_eq!(summary.attention_head_dim, None);
        assert!(!summary.has_layers());
        assert!(summary.has_hidden_size());
        assert!(summary.has_attention_heads());
        assert!(summary.has_kv_heads());
        assert!(summary.has_local_window());
        assert!(!summary.has_integral_attention_head_dim());
        assert!(!summary.kv_heads_fit_attention());
        assert!(!summary.local_window_fits_context(4096));
        assert_eq!(summary.architecture_dimension_signal_component_count(), 4);
        assert_eq!(summary.attention_geometry_signal_component_count(), 0);
        assert_eq!(summary.architecture_signal_component_count(), 4);
        assert_eq!(summary.architecture_dimension_problem_component_count(), 1);
        assert_eq!(summary.attention_geometry_problem_component_count(), 2);
        assert_eq!(
            summary.local_window_context_problem_component_count(4096),
            1
        );
        assert_eq!(summary.architecture_problem_component_count(4096), 4);
        assert!(summary.has_architecture_problem_components(4096));
        assert!(summary.architecture_accounting_is_consistent(4096));
        assert_eq!(
            summary.transformer_runtime_architecture_commit_signal_component_count(),
            4
        );
        assert_eq!(
            summary.transformer_runtime_architecture_commit_blocker_component_count(4096),
            4
        );
        assert!(summary.transformer_runtime_architecture_commit_accounting_is_consistent(4096));
        assert!(!summary.transformer_runtime_architecture_commit_is_clean(4096));
        assert!(!summary.shape_is_valid());
        assert!(!summary.architecture_shape_is_clean(4096));
        assert!(!summary.can_commit_transformer_runtime_architecture(4096));
        assert_eq!(
            summary.transformer_runtime_architecture_commit_action(4096),
            TransformerRuntimeArchitectureCommitAction::ReturnRuntimeFailure
        );
        assert!(
            !summary
                .transformer_runtime_architecture_commit_action(4096)
                .can_commit()
        );
        assert!(
            summary
                .transformer_runtime_architecture_commit_action(4096)
                .should_return_failure()
        );
        assert!(!summary.can_use_transformer_runtime_architecture(4096));
    }

    #[test]
    fn runtime_kv_policy_summary_reports_exchange_capacity() {
        let policy = RuntimeKvPolicy::import_export().with_limits(12, 6);

        let summary = policy.kv_policy_summary();

        assert!(summary.import_enabled);
        assert!(summary.export_enabled);
        assert_eq!(summary.max_import_blocks, 12);
        assert_eq!(summary.max_export_blocks, 6);
        assert!(summary.supports_kv_exchange());
        assert!(summary.has_import_capacity());
        assert!(summary.has_export_capacity());
        assert_eq!(summary.block_capacity(), 18);
        assert!(summary.limits_match_capabilities());
        assert_eq!(summary.kv_capability_signal_component_count(), 2);
        assert_eq!(summary.kv_capacity_signal_component_count(), 3);
        assert_eq!(summary.kv_policy_signal_component_count(), 5);
        assert!(summary.has_kv_policy_signal_components());
        assert_eq!(summary.import_limit_problem_component_count(), 0);
        assert_eq!(summary.export_limit_problem_component_count(), 0);
        assert_eq!(summary.kv_policy_problem_component_count(), 0);
        assert!(!summary.has_kv_policy_problem_components());
        assert!(summary.kv_policy_accounting_is_consistent());
        assert_eq!(summary.runtime_kv_policy_commit_signal_component_count(), 5);
        assert_eq!(
            summary.runtime_kv_policy_commit_blocker_component_count(),
            0
        );
        assert!(summary.runtime_kv_policy_commit_accounting_is_consistent());
        assert!(summary.runtime_kv_policy_commit_is_clean());
        assert!(summary.kv_policy_shape_is_clean());
        assert!(summary.can_commit_runtime_kv_policy());
        assert_eq!(
            summary.runtime_kv_policy_commit_action(),
            RuntimeKvPolicyCommitAction::CommitRuntimeKvPolicy
        );
        assert!(summary.runtime_kv_policy_commit_action().can_commit());
        assert!(
            !summary
                .runtime_kv_policy_commit_action()
                .should_return_failure()
        );
        assert!(summary.can_use_runtime_kv_policy());
    }

    #[test]
    fn runtime_kv_policy_summary_reports_disabled_exchange() {
        let policy = RuntimeKvPolicy::disabled();

        let summary = policy.kv_policy_summary();

        assert!(!summary.import_enabled);
        assert!(!summary.export_enabled);
        assert_eq!(summary.max_import_blocks, 0);
        assert_eq!(summary.max_export_blocks, 0);
        assert!(!summary.supports_kv_exchange());
        assert!(!summary.has_import_capacity());
        assert!(!summary.has_export_capacity());
        assert_eq!(summary.block_capacity(), 0);
        assert!(summary.limits_match_capabilities());
        assert_eq!(summary.kv_capability_signal_component_count(), 0);
        assert_eq!(summary.kv_capacity_signal_component_count(), 0);
        assert_eq!(summary.kv_policy_signal_component_count(), 0);
        assert!(!summary.has_kv_policy_signal_components());
        assert_eq!(summary.import_limit_problem_component_count(), 0);
        assert_eq!(summary.export_limit_problem_component_count(), 0);
        assert_eq!(summary.kv_policy_problem_component_count(), 0);
        assert!(!summary.has_kv_policy_problem_components());
        assert!(summary.kv_policy_accounting_is_consistent());
        assert_eq!(summary.runtime_kv_policy_commit_signal_component_count(), 0);
        assert_eq!(
            summary.runtime_kv_policy_commit_blocker_component_count(),
            0
        );
        assert!(summary.runtime_kv_policy_commit_accounting_is_consistent());
        assert!(summary.runtime_kv_policy_commit_is_clean());
        assert!(summary.kv_policy_shape_is_clean());
        assert!(summary.can_commit_runtime_kv_policy());
        assert_eq!(
            summary.runtime_kv_policy_commit_action(),
            RuntimeKvPolicyCommitAction::CommitRuntimeKvPolicy
        );
        assert!(summary.runtime_kv_policy_commit_action().can_commit());
        assert!(
            !summary
                .runtime_kv_policy_commit_action()
                .should_return_failure()
        );
        assert!(summary.can_use_runtime_kv_policy());
    }

    #[test]
    fn runtime_kv_policy_summary_counts_limit_capability_drift() {
        let summary = RuntimeKvPolicySummary {
            import_enabled: false,
            export_enabled: true,
            max_import_blocks: 3,
            max_export_blocks: 0,
        };

        assert!(!summary.limits_match_capabilities());
        assert_eq!(summary.kv_capability_signal_component_count(), 1);
        assert_eq!(summary.kv_capacity_signal_component_count(), 1);
        assert_eq!(summary.kv_policy_signal_component_count(), 2);
        assert_eq!(summary.import_limit_problem_component_count(), 1);
        assert_eq!(summary.export_limit_problem_component_count(), 1);
        assert_eq!(summary.kv_policy_problem_component_count(), 2);
        assert!(summary.has_kv_policy_problem_components());
        assert!(summary.kv_policy_accounting_is_consistent());
        assert_eq!(summary.runtime_kv_policy_commit_signal_component_count(), 2);
        assert_eq!(
            summary.runtime_kv_policy_commit_blocker_component_count(),
            2
        );
        assert!(summary.runtime_kv_policy_commit_accounting_is_consistent());
        assert!(!summary.runtime_kv_policy_commit_is_clean());
        assert!(!summary.kv_policy_shape_is_clean());
        assert!(!summary.can_commit_runtime_kv_policy());
        assert_eq!(
            summary.runtime_kv_policy_commit_action(),
            RuntimeKvPolicyCommitAction::ReturnRuntimeFailure
        );
        assert!(!summary.runtime_kv_policy_commit_action().can_commit());
        assert!(
            summary
                .runtime_kv_policy_commit_action()
                .should_return_failure()
        );
        assert!(!summary.can_use_runtime_kv_policy());
    }

    #[test]
    fn quantization_policy_summary_reports_kv_and_weight_widths() {
        let policy = RuntimeQuantizationPolicy {
            hot_kv: QuantizationBits::Four,
            cold_kv: QuantizationBits::Four,
            weights: Some(QuantizationBits::Eight),
        };

        let summary = policy.quantization_summary();

        assert_eq!(summary.hot_kv_precision_bits, 4);
        assert_eq!(summary.cold_kv_precision_bits, 4);
        assert_eq!(summary.weight_precision_bits, Some(8));
        assert!(summary.uses_compressed_hot_kv());
        assert!(summary.uses_compressed_cold_kv());
        assert!(summary.cold_kv_not_wider_than_hot());
        assert!(summary.has_weight_quantization());
        assert!(summary.all_kv_is_four_bit());
        assert!(summary.hot_kv_precision_is_supported());
        assert!(summary.cold_kv_precision_is_supported());
        assert!(summary.weight_precision_is_supported());
        assert_eq!(summary.kv_precision_signal_component_count(), 3);
        assert_eq!(summary.compression_signal_component_count(), 3);
        assert_eq!(summary.weight_precision_signal_component_count(), 1);
        assert_eq!(summary.quantization_signal_component_count(), 7);
        assert!(summary.has_quantization_signal_components());
        assert_eq!(summary.kv_precision_problem_component_count(), 0);
        assert_eq!(summary.weight_precision_problem_component_count(), 0);
        assert_eq!(summary.quantization_problem_component_count(), 0);
        assert!(!summary.has_quantization_problem_components());
        assert!(summary.quantization_accounting_is_consistent());
        assert_eq!(
            summary.runtime_quantization_policy_commit_signal_component_count(),
            7
        );
        assert_eq!(
            summary.runtime_quantization_policy_commit_blocker_component_count(),
            0
        );
        assert!(summary.runtime_quantization_policy_commit_accounting_is_consistent());
        assert!(summary.runtime_quantization_policy_commit_is_clean());
        assert!(summary.quantization_shape_is_clean());
        assert!(summary.can_commit_runtime_quantization_policy());
        assert_eq!(
            summary.runtime_quantization_policy_commit_action(),
            RuntimeQuantizationPolicyCommitAction::CommitRuntimeQuantizationPolicy
        );
        assert!(
            summary
                .runtime_quantization_policy_commit_action()
                .can_commit()
        );
        assert!(
            !summary
                .runtime_quantization_policy_commit_action()
                .should_return_failure()
        );
        assert!(summary.can_use_runtime_quantization_policy());
    }

    #[test]
    fn quantization_policy_summary_reflects_metadata_clamps() {
        let metadata = RuntimeMetadata::new("model", "tok", 4096, 2048).with_kv_precision(4, 8);
        let policy = RuntimeQuantizationPolicy::from_metadata(&metadata);

        let summary = policy.quantization_summary();

        assert_eq!(summary.hot_kv_precision_bits, 4);
        assert_eq!(summary.cold_kv_precision_bits, 4);
        assert_eq!(summary.weight_precision_bits, None);
        assert!(summary.uses_compressed_hot_kv());
        assert!(summary.uses_compressed_cold_kv());
        assert!(summary.cold_kv_not_wider_than_hot());
        assert!(!summary.has_weight_quantization());
        assert!(summary.all_kv_is_four_bit());
        assert_eq!(summary.kv_precision_signal_component_count(), 3);
        assert_eq!(summary.compression_signal_component_count(), 3);
        assert_eq!(summary.weight_precision_signal_component_count(), 0);
        assert_eq!(summary.quantization_signal_component_count(), 6);
        assert_eq!(summary.quantization_problem_component_count(), 0);
        assert!(summary.quantization_accounting_is_consistent());
        assert_eq!(
            summary.runtime_quantization_policy_commit_signal_component_count(),
            6
        );
        assert_eq!(
            summary.runtime_quantization_policy_commit_blocker_component_count(),
            0
        );
        assert!(summary.runtime_quantization_policy_commit_accounting_is_consistent());
        assert!(summary.runtime_quantization_policy_commit_is_clean());
        assert!(summary.quantization_shape_is_clean());
        assert!(summary.can_commit_runtime_quantization_policy());
        assert_eq!(
            summary.runtime_quantization_policy_commit_action(),
            RuntimeQuantizationPolicyCommitAction::CommitRuntimeQuantizationPolicy
        );
        assert!(
            summary
                .runtime_quantization_policy_commit_action()
                .can_commit()
        );
        assert!(
            !summary
                .runtime_quantization_policy_commit_action()
                .should_return_failure()
        );
        assert!(summary.can_use_runtime_quantization_policy());
    }

    #[test]
    fn quantization_policy_summary_counts_invalid_public_shape() {
        let summary = RuntimeQuantizationPolicySummary {
            hot_kv_precision_bits: 6,
            cold_kv_precision_bits: 8,
            weight_precision_bits: Some(7),
        };

        assert!(!summary.hot_kv_precision_is_supported());
        assert!(summary.cold_kv_precision_is_supported());
        assert!(!summary.weight_precision_is_supported());
        assert!(!summary.cold_kv_not_wider_than_hot());
        assert_eq!(summary.kv_precision_signal_component_count(), 1);
        assert_eq!(summary.compression_signal_component_count(), 0);
        assert_eq!(summary.weight_precision_signal_component_count(), 0);
        assert_eq!(summary.quantization_signal_component_count(), 1);
        assert_eq!(summary.kv_precision_problem_component_count(), 2);
        assert_eq!(summary.weight_precision_problem_component_count(), 1);
        assert_eq!(summary.quantization_problem_component_count(), 3);
        assert!(summary.has_quantization_problem_components());
        assert!(summary.quantization_accounting_is_consistent());
        assert_eq!(
            summary.runtime_quantization_policy_commit_signal_component_count(),
            1
        );
        assert_eq!(
            summary.runtime_quantization_policy_commit_blocker_component_count(),
            3
        );
        assert!(summary.runtime_quantization_policy_commit_accounting_is_consistent());
        assert!(!summary.runtime_quantization_policy_commit_is_clean());
        assert!(!summary.quantization_shape_is_clean());
        assert!(!summary.can_commit_runtime_quantization_policy());
        assert_eq!(
            summary.runtime_quantization_policy_commit_action(),
            RuntimeQuantizationPolicyCommitAction::ReturnRuntimeFailure
        );
        assert!(
            !summary
                .runtime_quantization_policy_commit_action()
                .can_commit()
        );
        assert!(
            summary
                .runtime_quantization_policy_commit_action()
                .should_return_failure()
        );
        assert!(!summary.can_use_runtime_quantization_policy());
    }

    #[test]
    fn manifest_validation_rejects_invalid_architecture() {
        let manifest = RuntimeManifestDigest::from_metadata(RuntimeMetadata::new("", "", 0, 0))
            .with_architecture(TransformerRuntimeArchitecture::new(0, 10, 3, 4, 0))
            .with_quantization(RuntimeQuantizationPolicy {
                hot_kv: QuantizationBits::Four,
                cold_kv: QuantizationBits::Eight,
                weights: None,
            });

        let validation = manifest.validate();
        let summary = validation.validation_summary();

        assert!(!validation.passed());
        assert!(!summary.passed);
        assert!(summary.has_errors());
        assert!(!summary.has_warnings());
        assert!(summary.has_blocking_failures());
        assert!(summary.has_failure_reports());
        assert_eq!(summary.validation_error_component_count(), 1);
        assert_eq!(summary.validation_warning_component_count(), 0);
        assert_eq!(summary.blocking_failure_component_count(), 1);
        assert_eq!(summary.mapped_failure_report_component_count(), 1);
        assert_eq!(summary.validation_signal_component_count(), 2);
        assert!(summary.has_validation_signal_components());
        assert!(summary.validation_signal_accounting_is_consistent());
        assert_eq!(summary.validation_activity_signal_component_count(), 1);
        assert!(summary.has_validation_activity_signals());
        assert!(summary.warnings_only_flag_matches_shape());
        assert_eq!(summary.validation_problem_component_count(), 1);
        assert!(summary.has_validation_problem_components());
        assert!(summary.validation_accounting_is_consistent());
        assert_eq!(
            summary.runtime_manifest_validation_commit_signal_component_count(),
            2
        );
        assert!(summary.has_runtime_manifest_validation_commit_signals());
        assert_eq!(
            summary.runtime_manifest_validation_commit_blocker_component_count(),
            1
        );
        assert!(summary.has_runtime_manifest_validation_commit_blockers());
        assert!(summary.runtime_manifest_validation_commit_accounting_is_consistent());
        assert!(!summary.runtime_manifest_validation_commit_is_clean());
        assert!(summary.failure_reports_match_errors());
        assert!(!summary.is_clean_pass());
        assert!(!summary.is_warnings_only_pass());
        assert!(!summary.validation_shape_is_clean());
        assert!(!summary.can_commit_runtime_manifest_validation());
        assert!(!summary.can_accept_runtime_manifest_validation());
        assert_eq!(summary.warning_count, 0);
        assert_eq!(summary.error_count, validation.errors.len());
        assert!(!summary.warnings_only);
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.contains("model_id"))
        );
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.contains("hidden_size"))
        );
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.contains("kv_heads"))
        );
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.contains("cold_kv"))
        );

        let failures = validation.failure_reports();
        let failure_batch = validation.failure_batch_summary();
        let primary_summary = validation.primary_failure_summary().unwrap();

        assert_eq!(failures.len(), 1);
        assert_eq!(summary.failure_report_count, failures.len());
        assert_eq!(
            failure_batch,
            RuntimeFailureReport::batch_summary(&failures)
        );
        assert_eq!(failure_batch.total_count, 1);
        assert_eq!(failure_batch.contract_violation_count, 1);
        assert_eq!(failure_batch.backend_error_count, 0);
        assert!(failure_batch.has_contract_failures());
        assert!(failure_batch.has_recoverable_failures());
        assert!(failure_batch.failure_batch_shape_is_clean());
        assert!(failure_batch.can_format_runtime_failures());
        assert_eq!(failures[0].kind, RuntimeFailureKind::ContractViolation);
        assert!(
            failures[0]
                .message
                .contains("runtime manifest validation failed")
        );
        assert_eq!(
            validation.primary_failure_report(),
            Some(failures[0].clone())
        );
        assert_eq!(primary_summary.kind, RuntimeFailureKind::ContractViolation);
        assert_eq!(primary_summary.trace_label, "runtime_contract_violation");
        assert!(primary_summary.recoverable);
        assert!(!primary_summary.backend_error);
        assert!(primary_summary.failure_summary_shape_is_clean());
        assert!(primary_summary.can_use_runtime_failure_report());

        assert_eq!(
            summary.runtime_manifest_validation_commit_action(),
            RuntimeManifestValidationCommitAction::ReturnRuntimeFailure
        );
        let commit = validation.commit_summary();
        assert_eq!(
            commit.action,
            RuntimeManifestValidationCommitAction::ReturnRuntimeFailure
        );
        assert_eq!(
            commit.action,
            summary.runtime_manifest_validation_commit_action()
        );
        assert!(!commit.action_can_commit());
        assert!(commit.action_should_return_failure());
        assert!(!commit.can_commit_runtime_manifest_validation());
        assert!(commit.should_return_runtime_failure());
        assert_eq!(commit.validation, summary);
        assert_eq!(commit.failure_reports, failures.clone());
        assert_eq!(commit.primary_failure_report, Some(failures[0].clone()));
        assert_eq!(commit.primary_failure_summary, Some(primary_summary));
        assert_eq!(commit.failure_batch, failure_batch);
        assert_eq!(commit.failure_report_count, 1);
        assert!(commit.can_format_runtime_failures);
        assert_eq!(commit.total_signal_component_count, 2);
        assert_eq!(commit.total_blocker_component_count, 1);
        assert!(commit.component_accounting_consistent);
        assert!(commit.has_primary_failure_summary());
        assert!(commit.failure_batch_shape_is_clean());
        assert!(commit.commit_decision_accounting_is_consistent());
    }

    #[test]
    fn manifest_validation_keeps_warnings_non_blocking() {
        let manifest = RuntimeManifestDigest::self_developed("model", "tok", 4096, 2048)
            .with_supported_adapters(Vec::new());

        let validation = manifest.validate();
        let summary = validation.validation_summary();

        assert!(validation.passed());
        assert!(validation.warnings_only());
        assert!(summary.passed);
        assert!(!summary.has_errors());
        assert!(summary.has_warnings());
        assert!(!summary.has_blocking_failures());
        assert!(!summary.has_failure_reports());
        assert_eq!(summary.validation_error_component_count(), 0);
        assert_eq!(summary.validation_warning_component_count(), 1);
        assert_eq!(summary.blocking_failure_component_count(), 0);
        assert_eq!(summary.mapped_failure_report_component_count(), 0);
        assert_eq!(summary.validation_signal_component_count(), 1);
        assert!(summary.has_validation_signal_components());
        assert!(summary.validation_signal_accounting_is_consistent());
        assert_eq!(summary.validation_activity_signal_component_count(), 1);
        assert!(summary.has_validation_activity_signals());
        assert!(summary.warnings_only_flag_matches_shape());
        assert_eq!(summary.validation_problem_component_count(), 0);
        assert!(!summary.has_validation_problem_components());
        assert!(summary.validation_accounting_is_consistent());
        assert_eq!(
            summary.runtime_manifest_validation_commit_signal_component_count(),
            1
        );
        assert!(summary.has_runtime_manifest_validation_commit_signals());
        assert_eq!(
            summary.runtime_manifest_validation_commit_blocker_component_count(),
            0
        );
        assert!(!summary.has_runtime_manifest_validation_commit_blockers());
        assert!(summary.runtime_manifest_validation_commit_accounting_is_consistent());
        assert!(summary.runtime_manifest_validation_commit_is_clean());
        assert!(summary.failure_reports_match_errors());
        assert!(!summary.is_clean_pass());
        assert!(summary.is_warnings_only_pass());
        assert!(summary.validation_shape_is_clean());
        assert!(summary.can_commit_runtime_manifest_validation());
        assert!(summary.can_accept_runtime_manifest_validation());
        assert_eq!(summary.warning_count, validation.warnings.len());
        assert_eq!(summary.error_count, 0);
        assert!(summary.warnings_only);
        assert_eq!(summary.failure_report_count, 0);
        let failure_batch = validation.failure_batch_summary();
        assert_eq!(failure_batch.total_count, 0);
        assert!(!failure_batch.has_failures());
        assert!(failure_batch.failure_batch_shape_is_clean());
        assert!(!failure_batch.can_format_runtime_failures());
        assert!(
            validation
                .warnings
                .iter()
                .any(|warning| warning.contains("supported_adapters"))
        );
        assert!(validation.failure_reports().is_empty());
        assert!(validation.primary_failure_summary().is_none());
        assert_eq!(
            summary.runtime_manifest_validation_commit_action(),
            RuntimeManifestValidationCommitAction::CommitManifest
        );
        let commit = validation.commit_summary();
        assert_eq!(
            commit.action,
            RuntimeManifestValidationCommitAction::CommitManifest
        );
        assert_eq!(
            commit.action,
            summary.runtime_manifest_validation_commit_action()
        );
        assert!(commit.action_can_commit());
        assert!(!commit.action_should_return_failure());
        assert!(commit.can_commit_runtime_manifest_validation());
        assert!(!commit.should_return_runtime_failure());
        assert_eq!(commit.validation, summary);
        assert!(commit.failure_reports.is_empty());
        assert_eq!(commit.primary_failure_report, None);
        assert_eq!(commit.primary_failure_summary, None);
        assert_eq!(commit.failure_batch, failure_batch);
        assert_eq!(commit.failure_report_count, 0);
        assert!(!commit.can_format_runtime_failures);
        assert_eq!(commit.total_signal_component_count, 1);
        assert_eq!(commit.total_blocker_component_count, 0);
        assert!(commit.component_accounting_consistent);
        assert!(!commit.has_primary_failure_summary());
        assert!(commit.failure_batch_shape_is_clean());
        assert!(commit.commit_decision_accounting_is_consistent());
    }

    #[test]
    fn manifest_validation_summary_reports_clean_passes() {
        let validation =
            RuntimeManifestDigest::self_developed("model", "tok", 4096, 2048).validate();

        let summary = validation.validation_summary();

        assert!(summary.passed);
        assert!(!summary.has_errors());
        assert!(!summary.has_warnings());
        assert!(!summary.has_blocking_failures());
        assert!(!summary.has_failure_reports());
        assert_eq!(summary.validation_error_component_count(), 0);
        assert_eq!(summary.validation_warning_component_count(), 0);
        assert_eq!(summary.blocking_failure_component_count(), 0);
        assert_eq!(summary.mapped_failure_report_component_count(), 0);
        assert_eq!(summary.validation_signal_component_count(), 0);
        assert!(!summary.has_validation_signal_components());
        assert!(summary.validation_signal_accounting_is_consistent());
        assert_eq!(summary.validation_activity_signal_component_count(), 0);
        assert!(!summary.has_validation_activity_signals());
        assert!(summary.warnings_only_flag_matches_shape());
        assert_eq!(summary.validation_problem_component_count(), 0);
        assert!(!summary.has_validation_problem_components());
        assert!(summary.validation_accounting_is_consistent());
        assert_eq!(
            summary.runtime_manifest_validation_commit_signal_component_count(),
            0
        );
        assert!(!summary.has_runtime_manifest_validation_commit_signals());
        assert_eq!(
            summary.runtime_manifest_validation_commit_blocker_component_count(),
            0
        );
        assert!(!summary.has_runtime_manifest_validation_commit_blockers());
        assert!(summary.runtime_manifest_validation_commit_accounting_is_consistent());
        assert!(summary.runtime_manifest_validation_commit_is_clean());
        assert!(summary.failure_reports_match_errors());
        assert!(summary.is_clean_pass());
        assert!(!summary.is_warnings_only_pass());
        assert!(summary.validation_shape_is_clean());
        assert!(summary.can_commit_runtime_manifest_validation());
        assert!(summary.can_accept_runtime_manifest_validation());
        assert_eq!(summary.error_count, 0);
        assert_eq!(summary.warning_count, 0);
        assert!(!summary.warnings_only);
        assert_eq!(summary.failure_report_count, 0);
        let failure_batch = validation.failure_batch_summary();
        assert_eq!(failure_batch.total_count, 0);
        assert!(!failure_batch.has_failures());
        assert!(failure_batch.failure_batch_shape_is_clean());
        assert!(!failure_batch.can_format_runtime_failures());
        assert!(validation.failure_reports().is_empty());
        assert!(validation.primary_failure_summary().is_none());
        assert_eq!(
            summary.runtime_manifest_validation_commit_action(),
            RuntimeManifestValidationCommitAction::CommitManifest
        );
        let commit = validation.commit_summary();
        assert_eq!(
            commit.action,
            RuntimeManifestValidationCommitAction::CommitManifest
        );
        assert_eq!(
            commit.action,
            summary.runtime_manifest_validation_commit_action()
        );
        assert!(commit.action_can_commit());
        assert!(!commit.action_should_return_failure());
        assert!(commit.can_commit_runtime_manifest_validation());
        assert!(!commit.should_return_runtime_failure());
        assert_eq!(commit.validation, summary);
        assert!(commit.failure_reports.is_empty());
        assert_eq!(commit.primary_failure_report, None);
        assert_eq!(commit.primary_failure_summary, None);
        assert_eq!(commit.failure_batch, failure_batch);
        assert_eq!(commit.failure_report_count, 0);
        assert!(!commit.can_format_runtime_failures);
        assert_eq!(commit.total_signal_component_count, 0);
        assert_eq!(commit.total_blocker_component_count, 0);
        assert!(commit.component_accounting_consistent);
        assert!(!commit.has_primary_failure_summary());
        assert!(commit.failure_batch_shape_is_clean());
        assert!(commit.commit_decision_accounting_is_consistent());
    }

    #[test]
    fn manifest_validation_summary_counts_public_shape_drift() {
        let drift = RuntimeManifestValidationSummary {
            passed: true,
            warning_count: 0,
            error_count: 2,
            warnings_only: true,
            failure_report_count: 0,
        };

        assert!(drift.passed);
        assert!(drift.has_errors());
        assert!(!drift.has_warnings());
        assert!(drift.has_blocking_failures());
        assert!(!drift.has_failure_reports());
        assert_eq!(drift.validation_error_component_count(), 1);
        assert_eq!(drift.validation_warning_component_count(), 0);
        assert_eq!(drift.blocking_failure_component_count(), 1);
        assert_eq!(drift.mapped_failure_report_component_count(), 0);
        assert_eq!(drift.validation_signal_component_count(), 1);
        assert!(drift.has_validation_signal_components());
        assert!(drift.validation_signal_accounting_is_consistent());
        assert_eq!(drift.validation_activity_signal_component_count(), 0);
        assert!(!drift.has_validation_activity_signals());
        assert!(!drift.failure_reports_match_errors());
        assert!(!drift.warnings_only_flag_matches_shape());
        assert_eq!(drift.validation_problem_component_count(), 3);
        assert!(drift.has_validation_problem_components());
        assert!(drift.validation_accounting_is_consistent());
        assert_eq!(
            drift.runtime_manifest_validation_commit_signal_component_count(),
            1
        );
        assert!(drift.has_runtime_manifest_validation_commit_signals());
        assert_eq!(
            drift.runtime_manifest_validation_commit_blocker_component_count(),
            3
        );
        assert!(drift.has_runtime_manifest_validation_commit_blockers());
        assert!(drift.runtime_manifest_validation_commit_accounting_is_consistent());
        assert!(!drift.runtime_manifest_validation_commit_is_clean());
        assert!(!drift.is_clean_pass());
        assert!(!drift.is_warnings_only_pass());
        assert!(!drift.validation_shape_is_clean());
        assert!(!drift.can_commit_runtime_manifest_validation());
        assert!(!drift.can_accept_runtime_manifest_validation());
    }

    #[test]
    fn manifest_prefers_observed_adapter_inside_execution_context() {
        let manifest = RuntimeManifestDigest::self_developed("model", "tok", 4096, 2048)
            .with_supported_adapters(vec![RuntimeAdapter::CpuSimd, RuntimeAdapter::Cuda]);
        let execution = AdapterExecutionContext::new([
            RuntimeAdapter::CpuSimd,
            RuntimeAdapter::Cuda,
            RuntimeAdapter::Metal,
        ]);
        let observations = [
            AdapterObservation::new(RuntimeAdapter::Metal, 0.99, 1.0, 1.0, None, None, 1),
            AdapterObservation::new(RuntimeAdapter::CpuSimd, 0.51, 1.0, 1.0, None, None, 2),
            AdapterObservation::new(RuntimeAdapter::Cuda, 0.80, 1.0, 1.0, None, None, 3),
        ];

        assert_eq!(
            manifest.preferred_adapter_for(&execution),
            Some(RuntimeAdapter::CpuSimd)
        );
        assert_eq!(
            manifest.preferred_adapter_with_observations(&execution, &observations),
            Some(RuntimeAdapter::Cuda)
        );

        let summary = manifest.adapter_compatibility_summary(&execution, &observations);

        assert_eq!(summary.supported_adapter_count, 2);
        assert_eq!(summary.execution_adapter_count, 3);
        assert_eq!(summary.compatible_adapter_count, 2);
        assert_eq!(summary.observation_count, 3);
        assert_eq!(summary.compatible_observation_count, 2);
        assert_eq!(summary.preferred_adapter, Some(RuntimeAdapter::CpuSimd));
        assert_eq!(
            summary.preferred_observed_adapter,
            Some(RuntimeAdapter::Cuda)
        );
        assert_eq!(summary.selected_adapter, Some(RuntimeAdapter::Cuda));
        assert!(summary.has_supported_adapters());
        assert!(summary.has_execution_adapters());
        assert!(summary.has_compatible_adapter());
        assert!(summary.has_adapter_intersection());
        assert!(!summary.missing_supported_adapter_catalog());
        assert!(!summary.missing_execution_adapter_hints());
        assert!(!summary.adapter_sets_are_disjoint());
        assert!(summary.has_compatible_observation());
        assert!(!summary.observations_all_rejected());
        assert_eq!(summary.rejected_observation_count(), 1);
        assert!(summary.has_selected_adapter());
        assert!(summary.selected_adapter_available());
        assert!(!summary.selected_adapter_missing());
        assert!(!summary.selected_adapter_unavailable());
        assert!(summary.can_plan_runtime_adapter());
        assert!(summary.selected_from_observation());
        assert!(!summary.selected_from_fallback());
        assert!(!summary.selection_requires_fallback());
        assert!(summary.compatibility_counts_are_bounded());
        assert_eq!(summary.compatible_adapter_fraction(), 1.0);
        assert!((summary.compatible_observation_fraction() - (2.0 / 3.0)).abs() < 0.0001);
        assert!(summary.selected_adapter_has_source());
        assert!(!summary.selected_adapter_source_drifted());
        assert!(summary.selected_adapter_is_usable());
        assert_eq!(summary.adapter_source_problem_component_count(), 0);
        assert!(!summary.has_adapter_source_problem_components());
        assert_eq!(summary.observation_selection_signal_component_count(), 0);
        assert!(!summary.has_observation_selection_signals());
        assert_eq!(summary.adapter_catalog_signal_component_count(), 3);
        assert_eq!(summary.adapter_observation_signal_component_count(), 2);
        assert_eq!(summary.adapter_selection_signal_component_count(), 2);
        assert_eq!(summary.adapter_compatibility_signal_component_count(), 7);
        assert!(summary.has_adapter_compatibility_signals());
        assert_eq!(summary.adapter_catalog_problem_component_count(), 0);
        assert_eq!(summary.adapter_selection_problem_component_count(), 0);
        assert_eq!(summary.adapter_compatibility_problem_component_count(), 0);
        assert!(!summary.has_adapter_compatibility_problem_components());
        assert!(summary.adapter_compatibility_accounting_is_consistent());
        assert_eq!(
            summary.runtime_manifest_adapter_compatibility_commit_signal_component_count(),
            7
        );
        assert!(summary.has_runtime_manifest_adapter_compatibility_commit_signals());
        assert_eq!(
            summary.runtime_manifest_adapter_compatibility_commit_blocker_component_count(),
            0
        );
        assert!(!summary.has_runtime_manifest_adapter_compatibility_commit_blockers());
        assert!(summary.runtime_manifest_adapter_compatibility_commit_accounting_is_consistent());
        assert!(summary.runtime_manifest_adapter_compatibility_commit_is_clean());
        assert!(summary.adapter_compatibility_shape_is_clean());
        assert!(summary.can_use_runtime_adapter_plan());
        assert!(summary.can_commit_runtime_manifest_adapter_compatibility());
        assert_eq!(summary.adapter_planning_signal_component_count(), 0);
        assert!(!summary.has_adapter_planning_signals());
        assert!(!summary.adapter_source_problem());
    }

    #[test]
    fn manifest_adapter_compatibility_reports_manifest_fallback_selection() {
        let manifest = RuntimeManifestDigest::self_developed("model", "tok", 4096, 2048)
            .with_supported_adapters(vec![RuntimeAdapter::CpuSimd, RuntimeAdapter::Cuda]);
        let execution = AdapterExecutionContext::new([
            RuntimeAdapter::CpuSimd,
            RuntimeAdapter::Cuda,
            RuntimeAdapter::Metal,
        ]);
        let observations = [
            AdapterObservation::new(RuntimeAdapter::Metal, 0.99, 1.0, 1.0, None, None, 1),
            AdapterObservation::new(RuntimeAdapter::Cuda, 0.49, 1.0, 1.0, None, None, 2),
        ];

        let summary = manifest.adapter_compatibility_summary(&execution, &observations);

        assert_eq!(summary.supported_adapter_count, 2);
        assert_eq!(summary.execution_adapter_count, 3);
        assert_eq!(summary.compatible_adapter_count, 2);
        assert_eq!(summary.observation_count, 2);
        assert_eq!(summary.compatible_observation_count, 0);
        assert_eq!(summary.preferred_adapter, Some(RuntimeAdapter::CpuSimd));
        assert_eq!(summary.preferred_observed_adapter, None);
        assert_eq!(summary.selected_adapter, Some(RuntimeAdapter::CpuSimd));
        assert!(summary.has_adapter_intersection());
        assert!(!summary.missing_supported_adapter_catalog());
        assert!(!summary.missing_execution_adapter_hints());
        assert!(!summary.adapter_sets_are_disjoint());
        assert!(!summary.has_compatible_observation());
        assert!(summary.observations_all_rejected());
        assert_eq!(summary.rejected_observation_count(), 2);
        assert!(summary.has_selected_adapter());
        assert!(summary.selected_adapter_available());
        assert!(!summary.selected_adapter_missing());
        assert!(!summary.selected_adapter_unavailable());
        assert!(summary.can_plan_runtime_adapter());
        assert!(!summary.selected_from_observation());
        assert!(summary.selected_from_fallback());
        assert!(summary.selection_requires_fallback());
        assert!(summary.compatibility_counts_are_bounded());
        assert_eq!(summary.compatible_adapter_fraction(), 1.0);
        assert_eq!(summary.compatible_observation_fraction(), 0.0);
        assert!(summary.selected_adapter_has_source());
        assert!(!summary.selected_adapter_source_drifted());
        assert!(summary.selected_adapter_is_usable());
        assert_eq!(summary.adapter_source_problem_component_count(), 0);
        assert!(!summary.has_adapter_source_problem_components());
        assert_eq!(summary.observation_selection_signal_component_count(), 2);
        assert!(summary.has_observation_selection_signals());
        assert_eq!(summary.adapter_catalog_signal_component_count(), 3);
        assert_eq!(summary.adapter_observation_signal_component_count(), 3);
        assert_eq!(summary.adapter_selection_signal_component_count(), 2);
        assert_eq!(summary.adapter_compatibility_signal_component_count(), 8);
        assert!(summary.has_adapter_compatibility_signals());
        assert_eq!(summary.adapter_catalog_problem_component_count(), 0);
        assert_eq!(summary.adapter_selection_problem_component_count(), 0);
        assert_eq!(summary.adapter_compatibility_problem_component_count(), 0);
        assert!(!summary.has_adapter_compatibility_problem_components());
        assert!(summary.adapter_compatibility_accounting_is_consistent());
        assert_eq!(
            summary.runtime_manifest_adapter_compatibility_commit_signal_component_count(),
            8
        );
        assert!(summary.has_runtime_manifest_adapter_compatibility_commit_signals());
        assert_eq!(
            summary.runtime_manifest_adapter_compatibility_commit_blocker_component_count(),
            0
        );
        assert!(!summary.has_runtime_manifest_adapter_compatibility_commit_blockers());
        assert!(summary.runtime_manifest_adapter_compatibility_commit_accounting_is_consistent());
        assert!(summary.runtime_manifest_adapter_compatibility_commit_is_clean());
        assert!(summary.adapter_compatibility_shape_is_clean());
        assert!(summary.can_use_runtime_adapter_plan());
        assert!(summary.can_commit_runtime_manifest_adapter_compatibility());
        assert_eq!(
            summary.runtime_manifest_adapter_compatibility_commit_action(),
            RuntimeManifestAdapterCompatibilityCommitAction::CommitRuntimeManifestAdapterCompatibility
        );
        assert!(
            summary
                .runtime_manifest_adapter_compatibility_commit_action()
                .can_commit()
        );
        assert!(
            !summary
                .runtime_manifest_adapter_compatibility_commit_action()
                .should_return_failure()
        );
        assert_eq!(summary.adapter_planning_signal_component_count(), 2);
        assert!(summary.has_adapter_planning_signals());
        assert!(!summary.adapter_source_problem());
    }

    #[test]
    fn manifest_adapter_compatibility_reports_empty_execution_intersections() {
        let manifest = RuntimeManifestDigest::self_developed("model", "tok", 4096, 2048)
            .with_supported_adapters(vec![RuntimeAdapter::Cuda]);
        let execution = AdapterExecutionContext::new([RuntimeAdapter::CpuSimd]);
        let observations = [AdapterObservation::new(
            RuntimeAdapter::Cuda,
            0.95,
            1.0,
            1.0,
            None,
            None,
            9,
        )];

        let summary = manifest.adapter_compatibility_summary(&execution, &observations);

        assert_eq!(summary.supported_adapter_count, 1);
        assert_eq!(summary.execution_adapter_count, 1);
        assert_eq!(summary.compatible_adapter_count, 0);
        assert_eq!(summary.observation_count, 1);
        assert_eq!(summary.compatible_observation_count, 0);
        assert_eq!(summary.preferred_adapter, None);
        assert_eq!(summary.preferred_observed_adapter, None);
        assert_eq!(summary.selected_adapter, None);
        assert!(summary.has_supported_adapters());
        assert!(summary.has_execution_adapters());
        assert!(!summary.has_compatible_adapter());
        assert!(!summary.has_adapter_intersection());
        assert!(!summary.missing_supported_adapter_catalog());
        assert!(!summary.missing_execution_adapter_hints());
        assert!(summary.adapter_sets_are_disjoint());
        assert!(!summary.has_compatible_observation());
        assert!(summary.observations_all_rejected());
        assert_eq!(summary.rejected_observation_count(), 1);
        assert!(!summary.has_selected_adapter());
        assert!(!summary.selected_adapter_available());
        assert!(!summary.selected_adapter_missing());
        assert!(!summary.selected_adapter_unavailable());
        assert!(!summary.can_plan_runtime_adapter());
        assert!(!summary.selected_from_observation());
        assert!(!summary.selected_from_fallback());
        assert!(!summary.selection_requires_fallback());
        assert!(summary.compatibility_counts_are_bounded());
        assert_eq!(summary.compatible_adapter_fraction(), 0.0);
        assert_eq!(summary.compatible_observation_fraction(), 0.0);
        assert!(summary.selected_adapter_has_source());
        assert!(!summary.selected_adapter_source_drifted());
        assert!(summary.selected_adapter_is_usable());
        assert_eq!(summary.adapter_source_problem_component_count(), 1);
        assert!(summary.has_adapter_source_problem_components());
        assert_eq!(summary.observation_selection_signal_component_count(), 1);
        assert!(summary.has_observation_selection_signals());
        assert_eq!(summary.adapter_catalog_signal_component_count(), 2);
        assert_eq!(summary.adapter_observation_signal_component_count(), 2);
        assert_eq!(summary.adapter_selection_signal_component_count(), 0);
        assert_eq!(summary.adapter_compatibility_signal_component_count(), 4);
        assert!(summary.has_adapter_compatibility_signals());
        assert_eq!(summary.adapter_catalog_problem_component_count(), 1);
        assert_eq!(summary.adapter_selection_problem_component_count(), 0);
        assert_eq!(summary.adapter_compatibility_problem_component_count(), 1);
        assert!(summary.has_adapter_compatibility_problem_components());
        assert!(summary.adapter_compatibility_accounting_is_consistent());
        assert_eq!(
            summary.runtime_manifest_adapter_compatibility_commit_signal_component_count(),
            4
        );
        assert!(summary.has_runtime_manifest_adapter_compatibility_commit_signals());
        assert_eq!(
            summary.runtime_manifest_adapter_compatibility_commit_blocker_component_count(),
            1
        );
        assert!(summary.has_runtime_manifest_adapter_compatibility_commit_blockers());
        assert!(summary.runtime_manifest_adapter_compatibility_commit_accounting_is_consistent());
        assert!(!summary.runtime_manifest_adapter_compatibility_commit_is_clean());
        assert!(!summary.adapter_compatibility_shape_is_clean());
        assert!(!summary.can_use_runtime_adapter_plan());
        assert!(!summary.can_commit_runtime_manifest_adapter_compatibility());
        assert_eq!(
            summary.runtime_manifest_adapter_compatibility_commit_action(),
            RuntimeManifestAdapterCompatibilityCommitAction::ReturnRuntimeFailure
        );
        assert!(
            !summary
                .runtime_manifest_adapter_compatibility_commit_action()
                .can_commit()
        );
        assert!(
            summary
                .runtime_manifest_adapter_compatibility_commit_action()
                .should_return_failure()
        );
        assert_eq!(summary.adapter_planning_signal_component_count(), 2);
        assert!(summary.has_adapter_planning_signals());
        assert!(summary.adapter_source_problem());
    }

    #[test]
    fn manifest_adapter_compatibility_reports_missing_adapter_sources() {
        let execution = AdapterExecutionContext::new(vec![RuntimeAdapter::CpuSimd]);
        let missing_manifest_catalog =
            RuntimeManifestDigest::self_developed("model", "tok", 4096, 2048)
                .with_supported_adapters(Vec::new())
                .adapter_compatibility_summary(&execution, &[]);

        assert_eq!(missing_manifest_catalog.supported_adapter_count, 0);
        assert_eq!(missing_manifest_catalog.execution_adapter_count, 1);
        assert!(missing_manifest_catalog.missing_supported_adapter_catalog());
        assert!(!missing_manifest_catalog.missing_execution_adapter_hints());
        assert!(!missing_manifest_catalog.adapter_sets_are_disjoint());
        assert!(!missing_manifest_catalog.can_plan_runtime_adapter());
        assert!(missing_manifest_catalog.compatibility_counts_are_bounded());
        assert_eq!(missing_manifest_catalog.compatible_adapter_fraction(), 0.0);
        assert!(!missing_manifest_catalog.selected_adapter_missing());
        assert!(!missing_manifest_catalog.selected_adapter_unavailable());
        assert!(missing_manifest_catalog.selected_adapter_has_source());
        assert!(!missing_manifest_catalog.selected_adapter_source_drifted());
        assert!(missing_manifest_catalog.selected_adapter_is_usable());
        assert_eq!(
            missing_manifest_catalog.adapter_source_problem_component_count(),
            1
        );
        assert!(missing_manifest_catalog.has_adapter_source_problem_components());
        assert_eq!(
            missing_manifest_catalog.observation_selection_signal_component_count(),
            0
        );
        assert_eq!(
            missing_manifest_catalog.adapter_catalog_signal_component_count(),
            1
        );
        assert_eq!(
            missing_manifest_catalog.adapter_observation_signal_component_count(),
            0
        );
        assert_eq!(
            missing_manifest_catalog.adapter_selection_signal_component_count(),
            0
        );
        assert_eq!(
            missing_manifest_catalog.adapter_compatibility_signal_component_count(),
            1
        );
        assert_eq!(
            missing_manifest_catalog.adapter_catalog_problem_component_count(),
            1
        );
        assert_eq!(
            missing_manifest_catalog.adapter_selection_problem_component_count(),
            0
        );
        assert_eq!(
            missing_manifest_catalog.adapter_compatibility_problem_component_count(),
            1
        );
        assert!(missing_manifest_catalog.has_adapter_compatibility_problem_components());
        assert!(missing_manifest_catalog.adapter_compatibility_accounting_is_consistent());
        assert_eq!(
            missing_manifest_catalog
                .runtime_manifest_adapter_compatibility_commit_signal_component_count(),
            1
        );
        assert!(
            missing_manifest_catalog.has_runtime_manifest_adapter_compatibility_commit_signals()
        );
        assert_eq!(
            missing_manifest_catalog
                .runtime_manifest_adapter_compatibility_commit_blocker_component_count(),
            1
        );
        assert!(
            missing_manifest_catalog.has_runtime_manifest_adapter_compatibility_commit_blockers()
        );
        assert!(
            missing_manifest_catalog
                .runtime_manifest_adapter_compatibility_commit_accounting_is_consistent()
        );
        assert!(!missing_manifest_catalog.runtime_manifest_adapter_compatibility_commit_is_clean());
        assert!(!missing_manifest_catalog.adapter_compatibility_shape_is_clean());
        assert!(!missing_manifest_catalog.can_use_runtime_adapter_plan());
        assert!(!missing_manifest_catalog.can_commit_runtime_manifest_adapter_compatibility());
        assert_eq!(
            missing_manifest_catalog.runtime_manifest_adapter_compatibility_commit_action(),
            RuntimeManifestAdapterCompatibilityCommitAction::ReturnRuntimeFailure
        );
        assert_eq!(
            missing_manifest_catalog.adapter_planning_signal_component_count(),
            1
        );
        assert!(missing_manifest_catalog.has_adapter_planning_signals());
        assert!(missing_manifest_catalog.adapter_source_problem());

        let missing_execution_hints =
            RuntimeManifestDigest::self_developed("model", "tok", 4096, 2048)
                .with_supported_adapters(vec![RuntimeAdapter::CpuSimd])
                .adapter_compatibility_summary(
                    &AdapterExecutionContext::new(Vec::<RuntimeAdapter>::new()),
                    &[],
                );

        assert_eq!(missing_execution_hints.supported_adapter_count, 1);
        assert_eq!(missing_execution_hints.execution_adapter_count, 0);
        assert!(!missing_execution_hints.missing_supported_adapter_catalog());
        assert!(missing_execution_hints.missing_execution_adapter_hints());
        assert!(!missing_execution_hints.adapter_sets_are_disjoint());
        assert!(!missing_execution_hints.can_plan_runtime_adapter());
        assert!(missing_execution_hints.compatibility_counts_are_bounded());
        assert_eq!(missing_execution_hints.compatible_adapter_fraction(), 0.0);
        assert!(!missing_execution_hints.selected_adapter_missing());
        assert!(missing_execution_hints.selected_adapter_unavailable());
        assert!(missing_execution_hints.selected_adapter_has_source());
        assert!(!missing_execution_hints.selected_adapter_source_drifted());
        assert!(!missing_execution_hints.selected_adapter_is_usable());
        assert_eq!(
            missing_execution_hints.adapter_source_problem_component_count(),
            2
        );
        assert!(missing_execution_hints.has_adapter_source_problem_components());
        assert_eq!(
            missing_execution_hints.observation_selection_signal_component_count(),
            1
        );
        assert_eq!(
            missing_execution_hints.adapter_catalog_signal_component_count(),
            1
        );
        assert_eq!(
            missing_execution_hints.adapter_observation_signal_component_count(),
            1
        );
        assert_eq!(
            missing_execution_hints.adapter_selection_signal_component_count(),
            2
        );
        assert_eq!(
            missing_execution_hints.adapter_compatibility_signal_component_count(),
            4
        );
        assert_eq!(
            missing_execution_hints.adapter_catalog_problem_component_count(),
            1
        );
        assert_eq!(
            missing_execution_hints.adapter_selection_problem_component_count(),
            1
        );
        assert_eq!(
            missing_execution_hints.adapter_compatibility_problem_component_count(),
            2
        );
        assert!(missing_execution_hints.has_adapter_compatibility_problem_components());
        assert!(missing_execution_hints.adapter_compatibility_accounting_is_consistent());
        assert_eq!(
            missing_execution_hints
                .runtime_manifest_adapter_compatibility_commit_signal_component_count(),
            4
        );
        assert!(
            missing_execution_hints.has_runtime_manifest_adapter_compatibility_commit_signals()
        );
        assert_eq!(
            missing_execution_hints
                .runtime_manifest_adapter_compatibility_commit_blocker_component_count(),
            2
        );
        assert!(
            missing_execution_hints.has_runtime_manifest_adapter_compatibility_commit_blockers()
        );
        assert!(
            missing_execution_hints
                .runtime_manifest_adapter_compatibility_commit_accounting_is_consistent()
        );
        assert!(!missing_execution_hints.runtime_manifest_adapter_compatibility_commit_is_clean());
        assert!(!missing_execution_hints.adapter_compatibility_shape_is_clean());
        assert!(!missing_execution_hints.can_use_runtime_adapter_plan());
        assert!(!missing_execution_hints.can_commit_runtime_manifest_adapter_compatibility());
        assert_eq!(
            missing_execution_hints.runtime_manifest_adapter_compatibility_commit_action(),
            RuntimeManifestAdapterCompatibilityCommitAction::ReturnRuntimeFailure
        );
        assert_eq!(
            missing_execution_hints.adapter_planning_signal_component_count(),
            3
        );
        assert!(missing_execution_hints.has_adapter_planning_signals());
        assert!(missing_execution_hints.adapter_source_problem());
    }

    #[test]
    fn manifest_adapter_compatibility_counts_invalid_shape_and_source_drift() {
        let summary = RuntimeManifestAdapterCompatibilitySummary {
            supported_adapter_count: 1,
            execution_adapter_count: 1,
            compatible_adapter_count: 2,
            observation_count: 1,
            compatible_observation_count: 2,
            preferred_adapter: None,
            preferred_observed_adapter: None,
            selected_adapter: Some(RuntimeAdapter::Cuda),
        };

        assert!(!summary.compatibility_counts_are_bounded());
        assert!(summary.selected_adapter_source_drifted());
        assert!(!summary.selected_adapter_is_usable());
        assert_eq!(summary.adapter_catalog_signal_component_count(), 3);
        assert_eq!(summary.adapter_observation_signal_component_count(), 3);
        assert_eq!(summary.adapter_selection_signal_component_count(), 1);
        assert_eq!(summary.adapter_compatibility_signal_component_count(), 7);
        assert_eq!(summary.adapter_catalog_problem_component_count(), 1);
        assert_eq!(summary.adapter_selection_problem_component_count(), 1);
        assert_eq!(summary.adapter_compatibility_problem_component_count(), 2);
        assert_eq!(summary.adapter_source_problem_component_count(), 2);
        assert!(summary.has_adapter_compatibility_problem_components());
        assert!(summary.adapter_compatibility_accounting_is_consistent());
        assert_eq!(
            summary.runtime_manifest_adapter_compatibility_commit_signal_component_count(),
            7
        );
        assert!(summary.has_runtime_manifest_adapter_compatibility_commit_signals());
        assert_eq!(
            summary.runtime_manifest_adapter_compatibility_commit_blocker_component_count(),
            2
        );
        assert!(summary.has_runtime_manifest_adapter_compatibility_commit_blockers());
        assert!(summary.runtime_manifest_adapter_compatibility_commit_accounting_is_consistent());
        assert!(!summary.runtime_manifest_adapter_compatibility_commit_is_clean());
        assert!(!summary.adapter_compatibility_shape_is_clean());
        assert!(!summary.can_use_runtime_adapter_plan());
        assert!(!summary.can_commit_runtime_manifest_adapter_compatibility());
    }

    #[test]
    fn manifest_execution_compatibility_accepts_execution_inside_kv_contract() {
        let manifest = RuntimeManifestDigest::self_developed("model", "tok", 4096, 2048)
            .with_kv_policy(RuntimeKvPolicy::import_export().with_limits(4, 2))
            .with_quantization(RuntimeQuantizationPolicy {
                hot_kv: QuantizationBits::Eight,
                cold_kv: QuantizationBits::Four,
                weights: None,
            });
        let execution = AdapterExecutionContext::new([RuntimeAdapter::CpuSimd])
            .with_kv_prefetch_blocks(3)
            .with_kv_precision(8, 4);

        let summary = manifest.execution_compatibility_summary(&execution);

        assert!(summary.kv_import_enabled);
        assert!(summary.kv_export_enabled);
        assert_eq!(summary.max_kv_import_blocks, 4);
        assert_eq!(summary.max_kv_export_blocks, 2);
        assert_eq!(summary.execution_kv_prefetch_blocks, 3);
        assert_eq!(summary.manifest_hot_kv_precision_bits, 8);
        assert_eq!(summary.manifest_cold_kv_precision_bits, 4);
        assert_eq!(summary.execution_hot_kv_precision_bits, 8);
        assert_eq!(summary.execution_cold_kv_precision_bits, 4);
        assert!(!summary.import_enabled_without_capacity());
        assert!(!summary.export_enabled_without_capacity());
        assert!(summary.execution_requests_kv_import());
        assert!(!summary.execution_requests_disabled_kv_import());
        assert!(!summary.kv_prefetch_exceeds_import_limit());
        assert_eq!(summary.kv_prefetch_overflow_blocks(), 0);
        assert!(summary.kv_prefetch_within_manifest_limit());
        assert!(summary.hot_precision_within_manifest());
        assert_eq!(summary.hot_precision_overflow_bits(), 0);
        assert!(summary.cold_precision_within_manifest());
        assert_eq!(summary.cold_precision_overflow_bits(), 0);
        assert!(summary.precision_within_manifest());
        assert!(!summary.precision_exceeds_manifest());
        assert!(summary.execution_cold_kv_not_wider_than_hot());
        assert!(!summary.manifest_kv_capacity_missing());
        assert!(summary.manifest_limits_are_consistent());
        assert!(!summary.execution_kv_contract_failure());
        assert!(summary.has_import_capacity());
        assert!(summary.has_export_capacity());
        assert!(summary.manifest_hot_precision_is_valid());
        assert!(summary.manifest_cold_precision_is_valid());
        assert!(summary.execution_hot_precision_is_valid());
        assert!(summary.execution_cold_precision_is_valid());
        assert_eq!(summary.kv_capacity_signal_component_count(), 4);
        assert_eq!(summary.kv_prefetch_signal_component_count(), 2);
        assert_eq!(summary.precision_signal_component_count(), 6);
        assert_eq!(summary.execution_contract_signal_component_count(), 12);
        assert!(summary.has_execution_contract_signals());
        assert_eq!(summary.import_capacity_problem_component_count(), 0);
        assert_eq!(summary.export_capacity_problem_component_count(), 0);
        assert_eq!(summary.kv_capacity_problem_component_count(), 0);
        assert_eq!(summary.disabled_import_request_component_count(), 0);
        assert_eq!(summary.kv_prefetch_limit_component_count(), 0);
        assert_eq!(summary.kv_prefetch_problem_component_count(), 0);
        assert_eq!(summary.hot_precision_problem_component_count(), 0);
        assert_eq!(summary.cold_precision_problem_component_count(), 0);
        assert_eq!(summary.cold_precision_inversion_component_count(), 0);
        assert_eq!(summary.precision_problem_component_count(), 0);
        assert_eq!(summary.execution_contract_problem_component_count(), 0);
        assert!(!summary.has_execution_contract_problem_components());
        assert_eq!(summary.execution_device_signal_component_count(), 12);
        assert!(summary.has_execution_device_signals());
        assert_eq!(summary.execution_device_blocker_component_count(), 0);
        assert!(!summary.has_execution_device_blockers());
        assert!(summary.execution_contract_accounting_is_consistent());
        assert!(summary.execution_contract_shape_is_clean());
        assert!(summary.execution_device_accounting_is_consistent());
        assert_eq!(
            summary.runtime_manifest_execution_device_commit_signal_component_count(),
            12
        );
        assert!(summary.has_runtime_manifest_execution_device_commit_signals());
        assert_eq!(
            summary.runtime_manifest_execution_device_commit_blocker_component_count(),
            0
        );
        assert!(!summary.has_runtime_manifest_execution_device_commit_blockers());
        assert!(summary.runtime_manifest_execution_device_commit_accounting_is_consistent());
        assert!(summary.runtime_manifest_execution_device_commit_is_clean());
        assert!(summary.execution_device_commit_is_clean());
        assert!(summary.can_commit_manifest_execution_device_gate());
        assert_eq!(
            summary.runtime_manifest_execution_device_commit_action(),
            RuntimeManifestExecutionDeviceCommitAction::CommitManifestExecutionDevice
        );
        assert!(
            summary
                .runtime_manifest_execution_device_commit_action()
                .can_commit()
        );
        assert!(
            !summary
                .runtime_manifest_execution_device_commit_action()
                .should_return_failure()
        );
        assert!(summary.can_use_execution_kv_contract());
    }

    #[test]
    fn runtime_device_handoff_readiness_accepts_clean_hardware_clamp_and_manifest() {
        let snapshot = HardwareLoadSnapshot::new(DeviceClass::Server, 0.20, 0.20, 0.20, 0.20);
        let plan = HardwareAllocator::new().plan(
            snapshot,
            TaskProfile::Coding,
            4096,
            HierarchyWeights::for_profile(TaskProfile::Coding),
        );
        let manifest = RuntimeManifestDigest::self_developed("model", "tok", 4096, 2048)
            .with_kv_policy(RuntimeKvPolicy::import_export().with_limits(4, 2))
            .with_quantization(RuntimeQuantizationPolicy {
                hot_kv: QuantizationBits::Eight,
                cold_kv: QuantizationBits::Four,
                weights: None,
            });
        let hardware = plan.runtime_readiness_summary(snapshot.snapshot_summary());
        let context = plan.adapter_execution_context();
        let runtime = manifest.runtime_metadata();
        let runtime_clamp = context.runtime_clamp_summary(&runtime);
        let clamped_context = context.clamp_for_runtime(&runtime);
        let manifest_execution = manifest.execution_compatibility_summary(&clamped_context);
        let readiness =
            RuntimeDeviceHandoffReadinessSummary::new(hardware, runtime_clamp, manifest_execution);

        assert_eq!(
            RuntimeDeviceHandoffReadinessSummary::stage_order(),
            [
                RuntimeDeviceHandoffStage::HardwareRuntime,
                RuntimeDeviceHandoffStage::RuntimeClamp,
                RuntimeDeviceHandoffStage::ManifestExecutionDevice,
            ]
        );
        assert!(readiness.hardware_ready());
        assert!(readiness.runtime_clamp_ready());
        assert!(readiness.manifest_execution_ready());
        assert_eq!(readiness.first_unready_stage(), None);
        assert_eq!(readiness.first_blocking_stage(), None);
        assert_eq!(
            readiness.stage_signal_component_count(RuntimeDeviceHandoffStage::HardwareRuntime),
            readiness.hardware_signal_component_count
        );
        assert_eq!(
            readiness
                .stage_blocker_component_count(RuntimeDeviceHandoffStage::ManifestExecutionDevice),
            readiness.manifest_execution_blocker_component_count
        );
        assert_eq!(readiness.hardware_signal_component_count, 25);
        assert_eq!(readiness.runtime_clamp_signal_component_count, 1);
        assert_eq!(readiness.manifest_execution_signal_component_count, 12);
        assert_eq!(
            readiness.runtime_device_handoff_signal_component_count(),
            38
        );
        assert!(readiness.has_runtime_device_handoff_signals());
        assert_eq!(
            readiness.runtime_device_handoff_blocker_component_count(),
            0
        );
        assert!(!readiness.has_runtime_device_handoff_blockers());
        assert_eq!(readiness.component_accounting_drift_count(), 0);
        assert_eq!(
            readiness.runtime_device_handoff_problem_component_count(),
            0
        );
        assert!(!readiness.has_runtime_device_handoff_problem_components());
        assert_eq!(readiness.failure_report(), None);
        assert_eq!(readiness.failure_reports(), Vec::new());
        assert_eq!(readiness.failure_report_count(), 0);
        assert!(!readiness.has_failure_reports());
        assert_eq!(readiness.failure_batch_summary().total_count, 0);
        assert!(!readiness.can_format_runtime_failures());
        assert_eq!(readiness.primary_failure_report(), None);
        assert_eq!(readiness.primary_failure_summary(), None);
        assert!(readiness.runtime_device_handoff_accounting_is_consistent());
        assert!(readiness.runtime_device_handoff_commit_is_clean());
        assert!(readiness.can_commit_runtime_device_handoff());
        assert_eq!(
            readiness.runtime_device_handoff_commit_action(),
            RuntimeDeviceHandoffCommitAction::CommitDeviceHandoff
        );
        let commit = readiness.commit_summary();
        assert_eq!(
            commit.action,
            RuntimeDeviceHandoffCommitAction::CommitDeviceHandoff
        );
        assert_eq!(
            commit.action,
            readiness.runtime_device_handoff_commit_action()
        );
        assert!(commit.action_can_commit());
        assert!(!commit.action_should_return_failure());
        assert!(commit.can_commit_runtime_device_handoff());
        assert!(!commit.should_return_runtime_failure());
        assert_eq!(commit.first_unready_stage, None);
        assert_eq!(commit.first_blocking_stage, None);
        assert!(commit.failure_reports.is_empty());
        assert_eq!(commit.primary_failure_report, None);
        assert_eq!(commit.primary_failure_summary, None);
        assert_eq!(commit.failure_report_count, 0);
        assert!(!commit.can_format_runtime_failures);
        assert_eq!(commit.total_signal_component_count, 38);
        assert_eq!(commit.total_blocker_component_count, 0);
        assert!(commit.component_accounting_consistent);
        assert!(!commit.has_primary_failure_summary());
        assert!(commit.failure_batch_shape_is_clean());
        assert!(commit.commit_decision_accounting_is_consistent());
    }

    #[test]
    fn runtime_device_handoff_readiness_routes_manifest_execution_blockers() {
        let snapshot = HardwareLoadSnapshot::new(DeviceClass::Server, 0.20, 0.20, 0.20, 0.20);
        let plan = HardwareAllocator::new().plan(
            snapshot,
            TaskProfile::Coding,
            4096,
            HierarchyWeights::for_profile(TaskProfile::Coding),
        );
        let manifest = RuntimeManifestDigest::self_developed("model", "tok", 4096, 2048)
            .with_kv_policy(RuntimeKvPolicy::import_export().with_limits(0, 0));
        let hardware = plan.runtime_readiness_summary(snapshot.snapshot_summary());
        let context = plan.adapter_execution_context();
        let runtime = manifest.runtime_metadata();
        let runtime_clamp = context.runtime_clamp_summary(&runtime);
        let manifest_execution = manifest.execution_compatibility_summary(&context);
        let readiness =
            RuntimeDeviceHandoffReadinessSummary::new(hardware, runtime_clamp, manifest_execution);

        assert!(readiness.hardware_ready());
        assert!(readiness.runtime_clamp_ready());
        assert!(!readiness.manifest_execution_ready());
        assert_eq!(
            readiness.first_unready_stage(),
            Some(RuntimeDeviceHandoffStage::ManifestExecutionDevice)
        );
        assert_eq!(
            readiness.first_blocking_stage(),
            Some(RuntimeDeviceHandoffStage::ManifestExecutionDevice)
        );
        assert_eq!(readiness.hardware_blocker_component_count, 0);
        assert_eq!(readiness.runtime_clamp_blocker_component_count, 0);
        assert_eq!(readiness.manifest_execution_blocker_component_count, 1);
        assert_eq!(
            readiness.runtime_device_handoff_blocker_component_count(),
            1
        );
        assert!(readiness.has_runtime_device_handoff_blockers());
        assert_eq!(
            readiness.runtime_device_handoff_problem_component_count(),
            1
        );
        assert!(readiness.has_runtime_device_handoff_problem_components());
        assert!(readiness.runtime_device_handoff_accounting_is_consistent());
        assert!(!readiness.runtime_device_handoff_commit_is_clean());
        assert!(!readiness.can_commit_runtime_device_handoff());
        let failures = readiness.failure_reports();
        let primary_summary = readiness
            .primary_failure_summary()
            .expect("manifest execution failure summary is reported");
        assert_eq!(failures.len(), 1);
        assert_eq!(readiness.failure_report_count(), 1);
        assert!(readiness.has_failure_reports());
        assert!(failures[0].message.contains("manifest_execution_device"));
        assert_eq!(failures[0].kind, RuntimeFailureKind::ContractViolation);
        assert_eq!(
            readiness.primary_failure_report(),
            Some(failures[0].clone())
        );
        assert_eq!(primary_summary.kind, RuntimeFailureKind::ContractViolation);
        assert!(readiness.can_format_runtime_failures());
        assert_eq!(
            readiness.runtime_device_handoff_commit_action(),
            RuntimeDeviceHandoffCommitAction::ReturnRuntimeFailure
        );
        let commit = readiness.commit_summary();
        assert_eq!(
            commit.action,
            RuntimeDeviceHandoffCommitAction::ReturnRuntimeFailure
        );
        assert_eq!(
            commit.action,
            readiness.runtime_device_handoff_commit_action()
        );
        assert!(!commit.action_can_commit());
        assert!(commit.action_should_return_failure());
        assert!(!commit.can_commit_runtime_device_handoff());
        assert!(commit.should_return_runtime_failure());
        assert_eq!(
            commit.first_unready_stage,
            Some(RuntimeDeviceHandoffStage::ManifestExecutionDevice)
        );
        assert_eq!(
            commit.first_blocking_stage,
            Some(RuntimeDeviceHandoffStage::ManifestExecutionDevice)
        );
        assert_eq!(commit.failure_reports, failures.clone());
        assert_eq!(commit.primary_failure_report, Some(failures[0].clone()));
        assert_eq!(commit.primary_failure_summary, Some(primary_summary));
        assert_eq!(commit.failure_batch.contract_violation_count, 1);
        assert_eq!(commit.failure_report_count, 1);
        assert!(commit.can_format_runtime_failures);
        assert_eq!(commit.total_blocker_component_count, 1);
        assert!(commit.component_accounting_consistent);
        assert!(commit.has_primary_failure_summary());
        assert!(commit.failure_batch_shape_is_clean());
        assert!(commit.commit_decision_accounting_is_consistent());
    }

    #[test]
    fn runtime_device_handoff_routes_manifest_blockers_after_clean_runtime_clamp() {
        let snapshot = HardwareLoadSnapshot::new(DeviceClass::Server, 0.20, 0.20, 0.20, 0.20);
        let plan = HardwareAllocator::new().plan(
            snapshot,
            TaskProfile::Coding,
            4096,
            HierarchyWeights::for_profile(TaskProfile::Coding),
        );
        let runtime_manifest = RuntimeManifestDigest::self_developed("model", "tok", 4096, 2048)
            .with_kv_policy(RuntimeKvPolicy::import_export().with_limits(4, 2))
            .with_quantization(RuntimeQuantizationPolicy {
                hot_kv: QuantizationBits::Eight,
                cold_kv: QuantizationBits::Four,
                weights: None,
            });
        let stricter_manifest = RuntimeManifestDigest::self_developed("model", "tok", 4096, 2048)
            .with_kv_policy(RuntimeKvPolicy::import_export().with_limits(1, 2))
            .with_quantization(RuntimeQuantizationPolicy {
                hot_kv: QuantizationBits::Four,
                cold_kv: QuantizationBits::Four,
                weights: None,
            });
        let hardware = plan.runtime_readiness_summary(snapshot.snapshot_summary());
        let context = plan.adapter_execution_context();
        let runtime = runtime_manifest.runtime_metadata();
        let runtime_clamp = context.runtime_clamp_summary(&runtime);
        let clamped_context = context.clone().clamp_for_runtime(&runtime);
        let manifest_execution =
            stricter_manifest.execution_compatibility_summary(&clamped_context);
        let readiness =
            RuntimeDeviceHandoffReadinessSummary::new(hardware, runtime_clamp, manifest_execution);

        assert!(readiness.hardware_ready());
        assert!(readiness.runtime_clamp_ready());
        assert!(!readiness.manifest_execution_ready());
        assert_eq!(runtime_clamp.after.kv_prefetch_blocks, 4);
        assert_eq!(clamped_context.kv_prefetch_blocks, 4);
        assert_eq!(clamped_context.hot_kv_precision_bits, 8);
        assert_eq!(manifest_execution.max_kv_import_blocks, 1);
        assert_eq!(manifest_execution.execution_kv_prefetch_blocks, 4);
        assert!(manifest_execution.kv_prefetch_exceeds_import_limit());
        assert_eq!(manifest_execution.kv_prefetch_overflow_blocks(), 3);
        assert!(!manifest_execution.hot_precision_within_manifest());
        assert_eq!(manifest_execution.hot_precision_overflow_bits(), 4);
        assert!(manifest_execution.cold_precision_within_manifest());
        assert!(manifest_execution.execution_kv_contract_failure());
        assert!(
            manifest_execution.runtime_manifest_execution_device_commit_accounting_is_consistent()
        );
        assert!(!manifest_execution.runtime_manifest_execution_device_commit_is_clean());
        assert_eq!(readiness.hardware_blocker_component_count, 0);
        assert_eq!(readiness.runtime_clamp_blocker_component_count, 0);
        assert!(readiness.manifest_execution_blocker_component_count > 0);
        assert_eq!(
            readiness.first_unready_stage(),
            Some(RuntimeDeviceHandoffStage::ManifestExecutionDevice)
        );
        assert_eq!(
            readiness.first_blocking_stage(),
            Some(RuntimeDeviceHandoffStage::ManifestExecutionDevice)
        );
        assert!(readiness.runtime_device_handoff_accounting_is_consistent());
        assert!(!readiness.runtime_device_handoff_commit_is_clean());
        assert!(!readiness.can_commit_runtime_device_handoff());
        let failure = readiness
            .primary_failure_report()
            .expect("manifest execution drift is reported after clean clamp");
        assert!(failure.message.contains("manifest_execution_device"));
        assert_eq!(failure.kind, RuntimeFailureKind::ContractViolation);
        let commit = readiness.commit_summary();
        assert_eq!(
            commit.action,
            RuntimeDeviceHandoffCommitAction::ReturnRuntimeFailure
        );
        assert_eq!(
            commit.first_blocking_stage,
            Some(RuntimeDeviceHandoffStage::ManifestExecutionDevice)
        );
        assert_eq!(commit.primary_failure_report, Some(failure));
        assert!(commit.should_return_runtime_failure());
        assert!(!commit.can_commit_runtime_device_handoff());
        assert!(commit.commit_decision_accounting_is_consistent());
    }

    #[test]
    fn runtime_device_handoff_failure_return_preserves_manifest_execution_stage() {
        let snapshot = HardwareLoadSnapshot::new(DeviceClass::Server, 0.20, 0.20, 0.20, 0.20);
        let plan = HardwareAllocator::new().plan(
            snapshot,
            TaskProfile::Coding,
            4096,
            HierarchyWeights::for_profile(TaskProfile::Coding),
        );
        let runtime_manifest = RuntimeManifestDigest::self_developed("model", "tok", 4096, 2048)
            .with_kv_policy(RuntimeKvPolicy::import_export().with_limits(4, 2))
            .with_quantization(RuntimeQuantizationPolicy {
                hot_kv: QuantizationBits::Eight,
                cold_kv: QuantizationBits::Four,
                weights: None,
            });
        let stricter_manifest = RuntimeManifestDigest::self_developed("model", "tok", 4096, 2048)
            .with_kv_policy(RuntimeKvPolicy::import_export().with_limits(1, 2))
            .with_quantization(RuntimeQuantizationPolicy {
                hot_kv: QuantizationBits::Four,
                cold_kv: QuantizationBits::Four,
                weights: None,
            });
        let hardware = plan.runtime_readiness_summary(snapshot.snapshot_summary());
        let context = plan.adapter_execution_context();
        let runtime = runtime_manifest.runtime_metadata();
        let runtime_clamp = context.runtime_clamp_summary(&runtime);
        let clamped_context = context.clamp_for_runtime(&runtime);
        let manifest_execution =
            stricter_manifest.execution_compatibility_summary(&clamped_context);
        let readiness =
            RuntimeDeviceHandoffReadinessSummary::new(hardware, runtime_clamp, manifest_execution);
        let commit = readiness.commit_summary();
        let failure_return = commit.failure_return_summary();
        let report = commit
            .runtime_failure_return_report()
            .expect("manifest execution handoff failure return report");

        assert!(readiness.hardware_ready());
        assert!(readiness.runtime_clamp_ready());
        assert!(!readiness.manifest_execution_ready());
        assert_eq!(
            readiness.first_blocking_stage(),
            Some(RuntimeDeviceHandoffStage::ManifestExecutionDevice)
        );
        assert_eq!(
            failure_return.source,
            ManifestFailureReturnSource::RuntimeDeviceHandoff
        );
        assert!(failure_return.should_return_failure);
        assert!(failure_return.has_failure_reports());
        assert!(failure_return.has_blocker_components());
        assert!(failure_return.can_return_runtime_failure());
        assert_eq!(
            report.source,
            ManifestFailureReturnSource::RuntimeDeviceHandoff
        );
        assert_eq!(
            report.primary_failure.kind,
            RuntimeFailureKind::ContractViolation
        );
        assert!(
            report
                .primary_failure
                .message
                .contains("manifest_execution_device")
        );
        assert!(
            report
                .backend_message()
                .contains("manifest_execution_device")
        );
        assert!(
            report
                .diagnostics_note()
                .contains("manifest_execution_device")
        );
        assert_eq!(report.failure_batch.contract_violation_count, 1);
        assert_eq!(report.failure_report_count, 1);
        assert!(report.total_blocker_component_count > 0);
        assert!(report.failure_return_report_shape_is_clean());
        assert!(report.can_use_manifest_failure_return_report());
        assert!(commit.should_return_runtime_failure());
        assert!(commit.commit_decision_accounting_is_consistent());
    }

    #[test]
    fn runtime_device_handoff_readiness_routes_runtime_clamp_blockers() {
        let snapshot = HardwareLoadSnapshot::new(DeviceClass::Server, 0.20, 0.20, 0.20, 0.20);
        let plan = HardwareAllocator::new().plan(
            snapshot,
            TaskProfile::Coding,
            4096,
            HierarchyWeights::for_profile(TaskProfile::Coding),
        );
        let manifest = RuntimeManifestDigest::self_developed("model", "tok", 4096, 2048)
            .with_kv_policy(RuntimeKvPolicy::import_export().with_limits(4, 2))
            .with_quantization(RuntimeQuantizationPolicy {
                hot_kv: QuantizationBits::Eight,
                cold_kv: QuantizationBits::Four,
                weights: None,
            });
        let hardware = plan.runtime_readiness_summary(snapshot.snapshot_summary());
        let context = plan.adapter_execution_context();
        let runtime = manifest.runtime_metadata();
        let manifest_execution =
            manifest.execution_compatibility_summary(&context.clone().clamp_for_runtime(&runtime));
        let before = context.context_summary();
        let mut after = before;
        after.adapter_count = 0;
        let runtime_clamp = AdapterRuntimeClampSummary {
            before,
            after,
            runtime: runtime.shape_summary(),
            kv_prefetch_reduction: 0,
            hot_kv_precision_reduced: false,
            cold_kv_precision_reduced: false,
        };
        let readiness =
            RuntimeDeviceHandoffReadinessSummary::new(hardware, runtime_clamp, manifest_execution);

        assert!(readiness.hardware_ready());
        assert!(!readiness.runtime_clamp_ready());
        assert!(readiness.manifest_execution_ready());
        assert_eq!(
            readiness.first_unready_stage(),
            Some(RuntimeDeviceHandoffStage::RuntimeClamp)
        );
        assert_eq!(
            readiness.first_blocking_stage(),
            Some(RuntimeDeviceHandoffStage::RuntimeClamp)
        );
        assert_eq!(readiness.hardware_blocker_component_count, 0);
        assert_eq!(readiness.runtime_clamp_blocker_component_count, 2);
        assert_eq!(readiness.manifest_execution_blocker_component_count, 0);
        assert_eq!(
            readiness.runtime_device_handoff_blocker_component_count(),
            2
        );
        assert!(readiness.has_runtime_device_handoff_blockers());
        assert_eq!(
            readiness.runtime_device_handoff_problem_component_count(),
            2
        );
        assert!(readiness.has_runtime_device_handoff_problem_components());
        assert!(readiness.runtime_device_handoff_accounting_is_consistent());
        assert!(!readiness.runtime_device_handoff_commit_is_clean());
        assert!(!readiness.can_commit_runtime_device_handoff());
        let failures = readiness.failure_reports();
        let primary_summary = readiness
            .primary_failure_summary()
            .expect("runtime clamp failure summary is reported");
        assert_eq!(failures.len(), 1);
        assert_eq!(readiness.failure_report_count(), 1);
        assert!(readiness.has_failure_reports());
        assert!(failures[0].message.contains("runtime_clamp"));
        assert_eq!(failures[0].kind, RuntimeFailureKind::ContractViolation);
        assert_eq!(primary_summary.kind, RuntimeFailureKind::ContractViolation);
        assert!(readiness.can_format_runtime_failures());
        assert_eq!(
            readiness.runtime_device_handoff_commit_action(),
            RuntimeDeviceHandoffCommitAction::ReturnRuntimeFailure
        );
        let commit = readiness.commit_summary();
        assert_eq!(
            commit.action,
            RuntimeDeviceHandoffCommitAction::ReturnRuntimeFailure
        );
        assert_eq!(
            commit.action,
            readiness.runtime_device_handoff_commit_action()
        );
        assert!(!commit.action_can_commit());
        assert!(commit.action_should_return_failure());
        assert!(!commit.can_commit_runtime_device_handoff());
        assert!(commit.should_return_runtime_failure());
        assert_eq!(
            commit.first_unready_stage,
            Some(RuntimeDeviceHandoffStage::RuntimeClamp)
        );
        assert_eq!(
            commit.first_blocking_stage,
            Some(RuntimeDeviceHandoffStage::RuntimeClamp)
        );
        assert_eq!(commit.failure_reports, failures);
        assert_eq!(commit.primary_failure_report, Some(failures[0].clone()));
        assert_eq!(commit.primary_failure_summary, Some(primary_summary));
        assert_eq!(commit.failure_batch.contract_violation_count, 1);
        assert_eq!(commit.failure_report_count, 1);
        assert!(commit.can_format_runtime_failures);
        assert_eq!(commit.total_blocker_component_count, 2);
        assert!(commit.component_accounting_consistent);
        assert!(commit.has_primary_failure_summary());
        assert!(commit.failure_batch_shape_is_clean());
        assert!(commit.commit_decision_accounting_is_consistent());
    }

    #[test]
    fn manifest_execution_compatibility_reports_kv_prefetch_limit_drift() {
        let manifest = RuntimeManifestDigest::self_developed("model", "tok", 4096, 2048)
            .with_kv_policy(RuntimeKvPolicy::from_capabilities(true, false).with_limits(2, 0))
            .with_quantization(RuntimeQuantizationPolicy {
                hot_kv: QuantizationBits::Four,
                cold_kv: QuantizationBits::Four,
                weights: None,
            });
        let execution = AdapterExecutionContext::new([RuntimeAdapter::Cuda])
            .with_kv_prefetch_blocks(5)
            .with_kv_precision(8, 4);

        let summary = manifest.execution_compatibility_summary(&execution);

        assert!(summary.kv_import_enabled);
        assert!(!summary.kv_export_enabled);
        assert_eq!(summary.max_kv_import_blocks, 2);
        assert_eq!(summary.max_kv_export_blocks, 0);
        assert_eq!(summary.execution_kv_prefetch_blocks, 5);
        assert!(summary.execution_requests_kv_import());
        assert!(!summary.execution_requests_disabled_kv_import());
        assert!(summary.kv_prefetch_exceeds_import_limit());
        assert_eq!(summary.kv_prefetch_overflow_blocks(), 3);
        assert!(!summary.kv_prefetch_within_manifest_limit());
        assert!(!summary.hot_precision_within_manifest());
        assert_eq!(summary.hot_precision_overflow_bits(), 4);
        assert!(summary.cold_precision_within_manifest());
        assert_eq!(summary.cold_precision_overflow_bits(), 0);
        assert!(!summary.precision_within_manifest());
        assert!(summary.precision_exceeds_manifest());
        assert!(summary.manifest_limits_are_consistent());
        assert!(!summary.manifest_kv_capacity_missing());
        assert!(summary.execution_kv_contract_failure());
        assert!(summary.has_import_capacity());
        assert!(!summary.has_export_capacity());
        assert!(summary.manifest_hot_precision_is_valid());
        assert!(summary.manifest_cold_precision_is_valid());
        assert!(summary.execution_hot_precision_is_valid());
        assert!(summary.execution_cold_precision_is_valid());
        assert_eq!(summary.kv_capacity_signal_component_count(), 2);
        assert_eq!(summary.kv_prefetch_signal_component_count(), 1);
        assert_eq!(summary.precision_signal_component_count(), 5);
        assert_eq!(summary.execution_contract_signal_component_count(), 8);
        assert!(summary.has_execution_contract_signals());
        assert_eq!(summary.import_capacity_problem_component_count(), 0);
        assert_eq!(summary.export_capacity_problem_component_count(), 0);
        assert_eq!(summary.kv_capacity_problem_component_count(), 0);
        assert_eq!(summary.disabled_import_request_component_count(), 0);
        assert_eq!(summary.kv_prefetch_limit_component_count(), 1);
        assert_eq!(summary.kv_prefetch_problem_component_count(), 1);
        assert_eq!(summary.hot_precision_problem_component_count(), 1);
        assert_eq!(summary.cold_precision_problem_component_count(), 0);
        assert_eq!(summary.cold_precision_inversion_component_count(), 0);
        assert_eq!(summary.precision_problem_component_count(), 1);
        assert_eq!(summary.execution_contract_problem_component_count(), 2);
        assert!(summary.has_execution_contract_problem_components());
        assert_eq!(summary.execution_device_signal_component_count(), 8);
        assert!(summary.has_execution_device_signals());
        assert_eq!(summary.execution_device_blocker_component_count(), 2);
        assert!(summary.has_execution_device_blockers());
        assert!(summary.execution_contract_accounting_is_consistent());
        assert!(!summary.execution_contract_shape_is_clean());
        assert!(summary.execution_device_accounting_is_consistent());
        assert_eq!(
            summary.runtime_manifest_execution_device_commit_signal_component_count(),
            8
        );
        assert!(summary.has_runtime_manifest_execution_device_commit_signals());
        assert_eq!(
            summary.runtime_manifest_execution_device_commit_blocker_component_count(),
            2
        );
        assert!(summary.has_runtime_manifest_execution_device_commit_blockers());
        assert!(summary.runtime_manifest_execution_device_commit_accounting_is_consistent());
        assert!(!summary.runtime_manifest_execution_device_commit_is_clean());
        assert!(!summary.execution_device_commit_is_clean());
        assert!(!summary.can_commit_manifest_execution_device_gate());
        assert_eq!(
            summary.runtime_manifest_execution_device_commit_action(),
            RuntimeManifestExecutionDeviceCommitAction::ReturnRuntimeFailure
        );
        assert!(
            !summary
                .runtime_manifest_execution_device_commit_action()
                .can_commit()
        );
        assert!(
            summary
                .runtime_manifest_execution_device_commit_action()
                .should_return_failure()
        );
        assert!(!summary.can_use_execution_kv_contract());
    }

    #[test]
    fn manifest_execution_compatibility_reports_missing_kv_capacities() {
        let manifest = RuntimeManifestDigest::self_developed("model", "tok", 4096, 2048)
            .with_kv_policy(RuntimeKvPolicy {
                import_enabled: true,
                export_enabled: true,
                max_import_blocks: 0,
                max_export_blocks: 0,
            });
        let execution = AdapterExecutionContext::new([RuntimeAdapter::CpuSimd])
            .with_kv_prefetch_blocks(1)
            .with_kv_precision(8, 4);

        let summary = manifest.execution_compatibility_summary(&execution);

        assert!(summary.import_enabled_without_capacity());
        assert!(summary.export_enabled_without_capacity());
        assert!(summary.execution_requests_kv_import());
        assert!(summary.kv_prefetch_exceeds_import_limit());
        assert_eq!(summary.kv_prefetch_overflow_blocks(), 1);
        assert!(!summary.kv_prefetch_within_manifest_limit());
        assert!(summary.precision_within_manifest());
        assert!(summary.manifest_kv_capacity_missing());
        assert!(!summary.manifest_limits_are_consistent());
        assert!(summary.execution_kv_contract_failure());
        assert_eq!(summary.import_capacity_problem_component_count(), 1);
        assert_eq!(summary.export_capacity_problem_component_count(), 1);
        assert_eq!(summary.kv_capacity_problem_component_count(), 2);
        assert_eq!(summary.disabled_import_request_component_count(), 0);
        assert_eq!(summary.kv_prefetch_limit_component_count(), 1);
        assert_eq!(summary.kv_prefetch_problem_component_count(), 1);
        assert_eq!(summary.hot_precision_problem_component_count(), 0);
        assert_eq!(summary.cold_precision_problem_component_count(), 0);
        assert_eq!(summary.cold_precision_inversion_component_count(), 0);
        assert_eq!(summary.precision_problem_component_count(), 0);
        assert_eq!(summary.execution_contract_problem_component_count(), 3);
        assert!(summary.has_execution_contract_problem_components());
        assert!(summary.execution_contract_accounting_is_consistent());
        assert_eq!(
            summary.runtime_manifest_execution_device_commit_blocker_component_count(),
            3
        );
        assert!(summary.has_runtime_manifest_execution_device_commit_blockers());
        assert!(summary.runtime_manifest_execution_device_commit_accounting_is_consistent());
        assert!(!summary.runtime_manifest_execution_device_commit_is_clean());
        assert!(!summary.execution_contract_shape_is_clean());
        assert!(!summary.can_commit_manifest_execution_device_gate());
        assert_eq!(
            summary.runtime_manifest_execution_device_commit_action(),
            RuntimeManifestExecutionDeviceCommitAction::ReturnRuntimeFailure
        );
        assert!(!summary.can_use_execution_kv_contract());
    }

    #[test]
    fn manifest_execution_compatibility_reports_disabled_import_requests() {
        let manifest = RuntimeManifestDigest::self_developed("model", "tok", 4096, 2048)
            .with_kv_policy(RuntimeKvPolicy::disabled());
        let execution = AdapterExecutionContext::new([RuntimeAdapter::CpuSimd])
            .with_kv_prefetch_blocks(1)
            .with_kv_precision(8, 4);

        let summary = manifest.execution_compatibility_summary(&execution);

        assert!(!summary.kv_import_enabled);
        assert!(!summary.kv_export_enabled);
        assert_eq!(summary.max_kv_import_blocks, 0);
        assert_eq!(summary.max_kv_export_blocks, 0);
        assert!(summary.execution_requests_kv_import());
        assert!(summary.execution_requests_disabled_kv_import());
        assert_eq!(summary.kv_prefetch_overflow_blocks(), 1);
        assert!(!summary.kv_prefetch_exceeds_import_limit());
        assert!(!summary.kv_prefetch_within_manifest_limit());
        assert!(summary.precision_within_manifest());
        assert!(!summary.manifest_kv_capacity_missing());
        assert!(summary.manifest_limits_are_consistent());
        assert!(summary.execution_kv_contract_failure());
        assert!(!summary.has_import_capacity());
        assert!(!summary.has_export_capacity());
        assert!(summary.manifest_hot_precision_is_valid());
        assert!(summary.manifest_cold_precision_is_valid());
        assert!(summary.execution_hot_precision_is_valid());
        assert!(summary.execution_cold_precision_is_valid());
        assert_eq!(summary.kv_capacity_signal_component_count(), 0);
        assert_eq!(summary.kv_prefetch_signal_component_count(), 1);
        assert_eq!(summary.precision_signal_component_count(), 6);
        assert_eq!(summary.execution_contract_signal_component_count(), 7);
        assert!(summary.has_execution_contract_signals());
        assert_eq!(summary.import_capacity_problem_component_count(), 0);
        assert_eq!(summary.export_capacity_problem_component_count(), 0);
        assert_eq!(summary.kv_capacity_problem_component_count(), 0);
        assert_eq!(summary.disabled_import_request_component_count(), 1);
        assert_eq!(summary.kv_prefetch_limit_component_count(), 0);
        assert_eq!(summary.kv_prefetch_problem_component_count(), 1);
        assert_eq!(summary.hot_precision_problem_component_count(), 0);
        assert_eq!(summary.cold_precision_problem_component_count(), 0);
        assert_eq!(summary.cold_precision_inversion_component_count(), 0);
        assert_eq!(summary.precision_problem_component_count(), 0);
        assert_eq!(summary.execution_contract_problem_component_count(), 1);
        assert!(summary.has_execution_contract_problem_components());
        assert!(summary.execution_contract_accounting_is_consistent());
        assert_eq!(
            summary.runtime_manifest_execution_device_commit_blocker_component_count(),
            1
        );
        assert!(summary.has_runtime_manifest_execution_device_commit_blockers());
        assert!(summary.runtime_manifest_execution_device_commit_accounting_is_consistent());
        assert!(!summary.runtime_manifest_execution_device_commit_is_clean());
        assert!(!summary.execution_contract_shape_is_clean());
        assert!(!summary.can_commit_manifest_execution_device_gate());
        assert_eq!(
            summary.runtime_manifest_execution_device_commit_action(),
            RuntimeManifestExecutionDeviceCommitAction::ReturnRuntimeFailure
        );
        assert!(!summary.can_use_execution_kv_contract());
    }

    #[test]
    fn manifest_execution_compatibility_counts_precision_inversion() {
        let summary = RuntimeManifestExecutionCompatibilitySummary {
            kv_import_enabled: true,
            kv_export_enabled: true,
            max_kv_import_blocks: 2,
            max_kv_export_blocks: 1,
            execution_kv_prefetch_blocks: 1,
            manifest_hot_kv_precision_bits: 4,
            manifest_cold_kv_precision_bits: 4,
            execution_hot_kv_precision_bits: 4,
            execution_cold_kv_precision_bits: 8,
        };

        assert!(summary.kv_prefetch_within_manifest_limit());
        assert!(summary.hot_precision_within_manifest());
        assert!(!summary.cold_precision_within_manifest());
        assert_eq!(summary.cold_precision_overflow_bits(), 4);
        assert!(!summary.execution_cold_kv_not_wider_than_hot());
        assert_eq!(summary.kv_capacity_signal_component_count(), 4);
        assert_eq!(summary.kv_prefetch_signal_component_count(), 2);
        assert_eq!(summary.precision_signal_component_count(), 4);
        assert_eq!(summary.execution_contract_signal_component_count(), 10);
        assert!(summary.has_execution_contract_signals());
        assert_eq!(summary.import_capacity_problem_component_count(), 0);
        assert_eq!(summary.export_capacity_problem_component_count(), 0);
        assert_eq!(summary.kv_capacity_problem_component_count(), 0);
        assert_eq!(summary.disabled_import_request_component_count(), 0);
        assert_eq!(summary.kv_prefetch_limit_component_count(), 0);
        assert_eq!(summary.kv_prefetch_problem_component_count(), 0);
        assert_eq!(summary.hot_precision_problem_component_count(), 0);
        assert_eq!(summary.cold_precision_problem_component_count(), 1);
        assert_eq!(summary.cold_precision_inversion_component_count(), 1);
        assert_eq!(summary.precision_problem_component_count(), 2);
        assert_eq!(summary.execution_contract_problem_component_count(), 2);
        assert!(summary.has_execution_contract_problem_components());
        assert!(summary.execution_contract_accounting_is_consistent());
        assert!(summary.execution_kv_contract_failure());
        assert_eq!(
            summary.runtime_manifest_execution_device_commit_blocker_component_count(),
            2
        );
        assert!(summary.has_runtime_manifest_execution_device_commit_blockers());
        assert!(summary.runtime_manifest_execution_device_commit_accounting_is_consistent());
        assert!(!summary.runtime_manifest_execution_device_commit_is_clean());
        assert!(!summary.execution_contract_shape_is_clean());
        assert!(!summary.can_commit_manifest_execution_device_gate());
        assert!(!summary.can_use_execution_kv_contract());
    }

    #[test]
    fn manifest_abi_summary_exposes_effective_runtime_boundary() {
        let manifest = RuntimeManifestDigest::self_developed("noiron-dev", "tok", 65_536, 4096)
            .with_architecture(TransformerRuntimeArchitecture::new(32, 4096, 32, 8, 8192))
            .with_kv_policy(RuntimeKvPolicy::import_export().with_limits(10, 5))
            .with_quantization(RuntimeQuantizationPolicy {
                hot_kv: QuantizationBits::Four,
                cold_kv: QuantizationBits::Four,
                weights: Some(QuantizationBits::Eight),
            })
            .with_supported_adapters(vec![
                RuntimeAdapter::CpuSimd,
                RuntimeAdapter::Cuda,
                RuntimeAdapter::Vulkan,
            ]);

        let summary = manifest.abi_summary();

        assert_eq!(summary.native_context_window, 65_536);
        assert_eq!(summary.embedding_dimensions, 4096);
        assert_eq!(summary.layer_count, 32);
        assert_eq!(summary.hidden_size, 4096);
        assert_eq!(summary.attention_heads, 32);
        assert_eq!(summary.kv_heads, 8);
        assert_eq!(summary.local_window_tokens, 8192);
        assert!(summary.kv_import_enabled);
        assert!(summary.kv_export_enabled);
        assert_eq!(summary.max_kv_import_blocks, 10);
        assert_eq!(summary.max_kv_export_blocks, 5);
        assert_eq!(summary.hot_kv_precision_bits, 4);
        assert_eq!(summary.cold_kv_precision_bits, 4);
        assert_eq!(summary.weight_precision_bits, Some(8));
        assert_eq!(summary.supported_adapter_count, 3);
        assert!(summary.has_known_context());
        assert!(summary.has_embedding_dimensions());
        assert!(summary.supports_kv_exchange());
        assert_eq!(summary.kv_exchange_block_capacity(), 15);
        assert!(summary.local_window_fits_context());
        assert!(summary.uses_compressed_hot_kv());
        assert!(summary.cold_kv_not_wider_than_hot());
        assert!(summary.has_weight_quantization());
        assert!(summary.has_layers());
        assert!(summary.has_hidden_size());
        assert!(summary.has_attention_heads());
        assert!(summary.has_kv_heads());
        assert!(summary.has_local_window());
        assert!(summary.kv_heads_fit_attention());
        assert!(summary.has_import_capacity());
        assert!(summary.has_export_capacity());
        assert!(summary.import_limits_match_capability());
        assert!(summary.export_limits_match_capability());
        assert!(summary.kv_limits_match_capabilities());
        assert!(summary.has_valid_hot_kv_precision());
        assert!(summary.has_valid_cold_kv_precision());
        assert!(summary.has_valid_weight_precision());
        assert!(summary.has_supported_adapters());
        assert_eq!(summary.context_abi_signal_component_count(), 2);
        assert_eq!(summary.transformer_abi_signal_component_count(), 6);
        assert_eq!(summary.kv_exchange_abi_signal_component_count(), 4);
        assert_eq!(summary.quantization_abi_signal_component_count(), 5);
        assert_eq!(summary.adapter_abi_signal_component_count(), 1);
        assert_eq!(summary.abi_signal_component_count(), 18);
        assert!(summary.has_abi_signals());
        assert_eq!(summary.context_abi_problem_component_count(), 0);
        assert_eq!(summary.transformer_abi_problem_component_count(), 0);
        assert_eq!(summary.kv_exchange_abi_problem_component_count(), 0);
        assert_eq!(summary.quantization_abi_problem_component_count(), 0);
        assert_eq!(summary.adapter_abi_problem_component_count(), 0);
        assert_eq!(summary.abi_problem_component_count(), 0);
        assert!(!summary.has_abi_problem_components());
        assert_eq!(summary.manifest_adapter_signal_component_count(), 18);
        assert!(summary.has_manifest_adapter_signals());
        assert_eq!(summary.manifest_adapter_blocker_component_count(), 0);
        assert!(!summary.has_manifest_adapter_blockers());
        assert!(summary.abi_accounting_is_consistent());
        assert!(summary.abi_shape_is_clean());
        assert!(summary.manifest_adapter_accounting_is_consistent());
        assert!(summary.manifest_adapter_commit_is_clean());
        assert!(summary.can_commit_runtime_manifest_adapter());
        assert_eq!(
            summary.runtime_manifest_adapter_commit_action(),
            RuntimeManifestAbiCommitAction::CommitRuntimeManifestAdapter
        );
        assert!(
            summary
                .runtime_manifest_adapter_commit_action()
                .can_commit()
        );
        assert!(
            !summary
                .runtime_manifest_adapter_commit_action()
                .should_return_failure()
        );
        assert!(summary.can_use_runtime_manifest_abi());
    }

    #[test]
    fn manifest_abi_summary_uses_effective_kv_policy() {
        let metadata =
            RuntimeMetadata::new("mismatch", "tok", 4096, 2048).with_kv_exchange(false, false);
        let manifest = RuntimeManifestDigest::from_metadata(metadata)
            .with_kv_policy(RuntimeKvPolicy::from_capabilities(true, false).with_limits(2, 9));

        let summary = manifest.abi_summary();

        assert!(summary.kv_import_enabled);
        assert!(!summary.kv_export_enabled);
        assert_eq!(summary.max_kv_import_blocks, 2);
        assert_eq!(summary.max_kv_export_blocks, 0);
        assert_eq!(summary.kv_exchange_block_capacity(), 2);
        assert!(summary.supports_kv_exchange());
        assert!(summary.local_window_fits_context());
        assert!(!summary.has_weight_quantization());
        assert!(summary.import_limits_match_capability());
        assert!(summary.export_limits_match_capability());
        assert!(summary.kv_limits_match_capabilities());
        assert_eq!(summary.kv_exchange_abi_signal_component_count(), 2);
        assert_eq!(summary.kv_exchange_abi_problem_component_count(), 0);
        assert_eq!(summary.quantization_abi_signal_component_count(), 3);
        assert_eq!(summary.quantization_abi_problem_component_count(), 0);
        assert!(summary.abi_accounting_is_consistent());
        assert!(summary.abi_shape_is_clean());
        assert!(summary.can_use_runtime_manifest_abi());
    }

    #[test]
    fn manifest_abi_summary_counts_shape_problem_components() {
        let summary = RuntimeManifestAbiSummary {
            native_context_window: 1024,
            embedding_dimensions: 0,
            layer_count: 0,
            hidden_size: 4096,
            attention_heads: 0,
            kv_heads: 2,
            local_window_tokens: 2048,
            kv_import_enabled: true,
            kv_export_enabled: false,
            max_kv_import_blocks: 0,
            max_kv_export_blocks: 4,
            hot_kv_precision_bits: 6,
            cold_kv_precision_bits: 8,
            weight_precision_bits: Some(3),
            supported_adapter_count: 0,
        };

        assert!(summary.has_known_context());
        assert!(!summary.has_embedding_dimensions());
        assert!(!summary.has_layers());
        assert!(summary.has_hidden_size());
        assert!(!summary.has_attention_heads());
        assert!(summary.has_kv_heads());
        assert!(summary.has_local_window());
        assert!(!summary.local_window_fits_context());
        assert!(summary.kv_heads_fit_attention());
        assert!(summary.kv_import_enabled);
        assert!(!summary.has_import_capacity());
        assert!(!summary.import_limits_match_capability());
        assert!(!summary.export_limits_match_capability());
        assert!(!summary.kv_limits_match_capabilities());
        assert!(!summary.has_valid_hot_kv_precision());
        assert!(summary.has_valid_cold_kv_precision());
        assert!(!summary.has_valid_weight_precision());
        assert!(!summary.cold_kv_not_wider_than_hot());
        assert!(!summary.has_supported_adapters());
        assert_eq!(summary.context_abi_signal_component_count(), 1);
        assert_eq!(summary.transformer_abi_signal_component_count(), 3);
        assert_eq!(summary.kv_exchange_abi_signal_component_count(), 2);
        assert_eq!(summary.quantization_abi_signal_component_count(), 2);
        assert_eq!(summary.adapter_abi_signal_component_count(), 0);
        assert_eq!(summary.abi_signal_component_count(), 8);
        assert!(summary.has_abi_signals());
        assert_eq!(summary.context_abi_problem_component_count(), 1);
        assert_eq!(summary.transformer_abi_problem_component_count(), 3);
        assert_eq!(summary.kv_exchange_abi_problem_component_count(), 2);
        assert_eq!(summary.quantization_abi_problem_component_count(), 3);
        assert_eq!(summary.adapter_abi_problem_component_count(), 1);
        assert_eq!(summary.abi_problem_component_count(), 10);
        assert!(summary.has_abi_problem_components());
        assert_eq!(summary.manifest_adapter_signal_component_count(), 8);
        assert!(summary.has_manifest_adapter_signals());
        assert_eq!(summary.manifest_adapter_blocker_component_count(), 10);
        assert!(summary.has_manifest_adapter_blockers());
        assert!(summary.abi_accounting_is_consistent());
        assert!(!summary.abi_shape_is_clean());
        assert!(summary.manifest_adapter_accounting_is_consistent());
        assert!(!summary.manifest_adapter_commit_is_clean());
        assert!(!summary.can_commit_runtime_manifest_adapter());
        assert_eq!(
            summary.runtime_manifest_adapter_commit_action(),
            RuntimeManifestAbiCommitAction::ReturnRuntimeFailure
        );
        assert!(
            !summary
                .runtime_manifest_adapter_commit_action()
                .can_commit()
        );
        assert!(
            summary
                .runtime_manifest_adapter_commit_action()
                .should_return_failure()
        );
        assert!(!summary.can_use_runtime_manifest_abi());
    }

    #[test]
    fn quantization_policy_clamps_cold_kv_to_hot_metadata_width() {
        let metadata = RuntimeMetadata::new("compact", "tok", 4096, 2048).with_kv_precision(4, 8);

        let policy = RuntimeQuantizationPolicy::from_metadata(&metadata);

        assert_eq!(policy.hot_kv, QuantizationBits::Four);
        assert_eq!(policy.cold_kv, QuantizationBits::Four);
    }
}
