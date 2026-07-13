#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeMetadata {
    pub model_id: String,
    pub tokenizer: String,
    pub native_context_window: usize,
    pub embedding_dimensions: usize,
    pub supports_kv_import: bool,
    pub supports_kv_export: bool,
    pub max_kv_import_blocks: usize,
    pub max_kv_export_blocks: usize,
    pub hot_kv_precision_bits: u8,
    pub cold_kv_precision_bits: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeMetadataShapeSummary {
    pub native_context_window: usize,
    pub embedding_dimensions: usize,
    pub supports_kv_import: bool,
    pub supports_kv_export: bool,
    pub max_kv_import_blocks: usize,
    pub max_kv_export_blocks: usize,
    pub hot_kv_precision_bits: u8,
    pub cold_kv_precision_bits: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeBackendMaxTokensCommitAction {
    CommitBackendMaxTokens,
    ReturnContextExhausted,
    RepairContextBudget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeBackendMaxTokensCommitSummary {
    pub budget: RuntimeGenerationBudget,
    pub action: RuntimeBackendMaxTokensCommitAction,
    pub backend_max_tokens: Option<usize>,
    pub can_commit: bool,
    pub should_return_context_exhausted: bool,
    pub should_repair_context_budget: bool,
    pub total_signal_component_count: usize,
    pub total_blocker_component_count: usize,
    pub context_budget_shape_problem_component_count: usize,
    pub context_exhaustion_blocker_component_count: usize,
    pub component_accounting_consistent: bool,
}

impl RuntimeMetadataShapeSummary {
    pub fn has_known_context_window(self) -> bool {
        self.native_context_window > 0
    }

    pub fn has_embedding_dimensions(self) -> bool {
        self.embedding_dimensions > 0
    }

    pub fn supports_any_kv_exchange(self) -> bool {
        self.supports_kv_import || self.supports_kv_export
    }

    pub fn kv_exchange_block_capacity(self) -> usize {
        self.max_kv_import_blocks
            .saturating_add(self.max_kv_export_blocks)
    }

    pub fn kv_precision_is_compressed(self) -> bool {
        self.hot_kv_precision_bits < 8 || self.cold_kv_precision_bits < 8
    }

    pub fn cold_kv_not_wider_than_hot(self) -> bool {
        self.cold_kv_precision_bits <= self.hot_kv_precision_bits
    }

    pub fn has_valid_hot_kv_precision(self) -> bool {
        valid_kv_precision_bits(self.hot_kv_precision_bits)
    }

    pub fn has_valid_cold_kv_precision(self) -> bool {
        valid_kv_precision_bits(self.cold_kv_precision_bits)
    }

    pub fn context_shape_signal_component_count(self) -> usize {
        usize::from(self.has_known_context_window()) + usize::from(self.has_embedding_dimensions())
    }

    pub fn kv_import_capability_signal_component_count(self) -> usize {
        usize::from(self.supports_kv_import) + usize::from(self.max_kv_import_blocks > 0)
    }

    pub fn kv_export_capability_signal_component_count(self) -> usize {
        usize::from(self.supports_kv_export) + usize::from(self.max_kv_export_blocks > 0)
    }

    pub fn kv_exchange_capability_signal_component_count(self) -> usize {
        self.kv_import_capability_signal_component_count()
            .saturating_add(self.kv_export_capability_signal_component_count())
    }

    pub fn kv_precision_signal_component_count(self) -> usize {
        usize::from(self.has_valid_hot_kv_precision())
            + usize::from(self.has_valid_cold_kv_precision())
            + usize::from(
                self.has_valid_hot_kv_precision()
                    && self.has_valid_cold_kv_precision()
                    && self.kv_precision_is_compressed(),
            )
    }

    pub fn metadata_shape_signal_component_count(self) -> usize {
        self.context_shape_signal_component_count()
            .saturating_add(self.kv_exchange_capability_signal_component_count())
            .saturating_add(self.kv_precision_signal_component_count())
    }

    pub fn has_metadata_shape_signals(self) -> bool {
        self.metadata_shape_signal_component_count() > 0
    }

    pub fn kv_import_contract_problem_component_count(self) -> usize {
        usize::from(self.supports_kv_import && self.max_kv_import_blocks == 0)
            + usize::from(!self.supports_kv_import && self.max_kv_import_blocks > 0)
    }

    pub fn kv_export_contract_problem_component_count(self) -> usize {
        usize::from(self.supports_kv_export && self.max_kv_export_blocks == 0)
            + usize::from(!self.supports_kv_export && self.max_kv_export_blocks > 0)
    }

    pub fn kv_exchange_contract_problem_component_count(self) -> usize {
        self.kv_import_contract_problem_component_count()
            .saturating_add(self.kv_export_contract_problem_component_count())
    }

    pub fn kv_precision_problem_component_count(self) -> usize {
        usize::from(!self.has_valid_hot_kv_precision())
            + usize::from(!self.has_valid_cold_kv_precision())
            + usize::from(!self.cold_kv_not_wider_than_hot())
    }

    pub fn metadata_shape_problem_component_count(self) -> usize {
        self.kv_exchange_contract_problem_component_count()
            .saturating_add(self.kv_precision_problem_component_count())
    }

    pub fn has_metadata_shape_problem_components(self) -> bool {
        self.metadata_shape_problem_component_count() > 0
    }

    pub fn runtime_metadata_adapter_signal_component_count(self) -> usize {
        self.metadata_shape_signal_component_count()
    }

    pub fn has_runtime_metadata_adapter_signals(self) -> bool {
        self.runtime_metadata_adapter_signal_component_count() > 0
    }

    pub fn runtime_metadata_adapter_missing_component_count(self) -> usize {
        usize::from(!self.has_known_context_window())
            .saturating_add(usize::from(!self.has_embedding_dimensions()))
    }

    pub fn runtime_metadata_adapter_blocker_component_count(self) -> usize {
        self.metadata_shape_problem_component_count()
            .saturating_add(self.runtime_metadata_adapter_missing_component_count())
    }

    pub fn has_runtime_metadata_adapter_blockers(self) -> bool {
        self.runtime_metadata_adapter_blocker_component_count() > 0
    }

    pub fn metadata_shape_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .context_shape_signal_component_count()
            .saturating_add(self.kv_exchange_capability_signal_component_count())
            .saturating_add(self.kv_precision_signal_component_count());
        let expected_problem_count = self
            .kv_exchange_contract_problem_component_count()
            .saturating_add(self.kv_precision_problem_component_count());

        self.metadata_shape_signal_component_count() == expected_signal_count
            && self.has_metadata_shape_signals() == (expected_signal_count > 0)
            && self.metadata_shape_problem_component_count() == expected_problem_count
            && self.has_metadata_shape_problem_components() == (expected_problem_count > 0)
    }

    pub fn metadata_shape_is_clean(self) -> bool {
        !self.has_metadata_shape_problem_components()
            && self.metadata_shape_accounting_is_consistent()
    }

    pub fn runtime_metadata_adapter_accounting_is_consistent(self) -> bool {
        let expected_missing_count = usize::from(!self.has_known_context_window())
            .saturating_add(usize::from(!self.has_embedding_dimensions()));
        let expected_blocker_count = self
            .metadata_shape_problem_component_count()
            .saturating_add(expected_missing_count);

        self.metadata_shape_accounting_is_consistent()
            && self.runtime_metadata_adapter_signal_component_count()
                == self.metadata_shape_signal_component_count()
            && self.has_runtime_metadata_adapter_signals()
                == (self.runtime_metadata_adapter_signal_component_count() > 0)
            && self.runtime_metadata_adapter_missing_component_count() == expected_missing_count
            && self.runtime_metadata_adapter_blocker_component_count() == expected_blocker_count
            && self.has_runtime_metadata_adapter_blockers() == (expected_blocker_count > 0)
    }

    pub fn runtime_metadata_adapter_commit_is_clean(self) -> bool {
        !self.has_runtime_metadata_adapter_blockers()
            && self.runtime_metadata_adapter_accounting_is_consistent()
    }

    pub fn can_commit_runtime_metadata_adapter(self) -> bool {
        self.runtime_metadata_adapter_commit_is_clean()
    }

    pub fn can_use_runtime_metadata_contract(self) -> bool {
        self.metadata_shape_is_clean()
    }
}

impl RuntimeBackendMaxTokensCommitAction {
    pub fn can_commit(self) -> bool {
        matches!(self, Self::CommitBackendMaxTokens)
    }

    pub fn should_return_context_exhausted(self) -> bool {
        matches!(self, Self::ReturnContextExhausted)
    }

    pub fn should_repair_context_budget(self) -> bool {
        matches!(self, Self::RepairContextBudget)
    }
}

impl RuntimeBackendMaxTokensCommitSummary {
    pub fn new(budget: RuntimeGenerationBudget) -> Self {
        let component_accounting_consistent = budget.backend_request_accounting_is_consistent();
        let action = if budget.can_commit_backend_max_tokens() {
            RuntimeBackendMaxTokensCommitAction::CommitBackendMaxTokens
        } else if component_accounting_consistent
            && budget.context_budget_shape_is_clean()
            && budget.context_exhausted()
        {
            RuntimeBackendMaxTokensCommitAction::ReturnContextExhausted
        } else {
            RuntimeBackendMaxTokensCommitAction::RepairContextBudget
        };
        let backend_max_tokens = action.can_commit().then_some(budget.max_generated_tokens);

        Self {
            budget,
            action,
            backend_max_tokens,
            can_commit: action.can_commit(),
            should_return_context_exhausted: action.should_return_context_exhausted(),
            should_repair_context_budget: action.should_repair_context_budget(),
            total_signal_component_count: budget.backend_request_signal_component_count(),
            total_blocker_component_count: budget.backend_request_blocker_component_count(),
            context_budget_shape_problem_component_count: budget
                .context_budget_shape_problem_component_count(),
            context_exhaustion_blocker_component_count: usize::from(!budget.can_generate()),
            component_accounting_consistent,
        }
    }

    pub fn action_can_commit(self) -> bool {
        self.action.can_commit()
    }

    pub fn action_should_return_context_exhausted(self) -> bool {
        self.action.should_return_context_exhausted()
    }

    pub fn action_should_repair_context_budget(self) -> bool {
        self.action.should_repair_context_budget()
    }

    pub fn can_commit_backend_max_tokens(self) -> bool {
        self.can_commit
    }

    pub fn should_return_runtime_context_exhausted(self) -> bool {
        self.should_return_context_exhausted
    }

    pub fn should_repair_context_budget(self) -> bool {
        self.should_repair_context_budget
    }

    pub fn can_use_backend_max_tokens(self) -> bool {
        self.can_commit && self.backend_max_tokens.is_some()
    }

    pub fn commit_decision_accounting_is_consistent(self) -> bool {
        let expected_action = if self.budget.can_commit_backend_max_tokens() {
            RuntimeBackendMaxTokensCommitAction::CommitBackendMaxTokens
        } else if self.component_accounting_consistent
            && self.budget.context_budget_shape_is_clean()
            && self.budget.context_exhausted()
        {
            RuntimeBackendMaxTokensCommitAction::ReturnContextExhausted
        } else {
            RuntimeBackendMaxTokensCommitAction::RepairContextBudget
        };
        let expected_backend_max_tokens = expected_action
            .can_commit()
            .then_some(self.budget.max_generated_tokens);

        self.action == expected_action
            && self.backend_max_tokens == expected_backend_max_tokens
            && self.can_commit == self.action.can_commit()
            && self.should_return_context_exhausted == self.action.should_return_context_exhausted()
            && self.should_repair_context_budget == self.action.should_repair_context_budget()
            && self.total_signal_component_count
                == self.budget.backend_request_signal_component_count()
            && self.total_blocker_component_count
                == self.budget.backend_request_blocker_component_count()
            && self.context_budget_shape_problem_component_count
                == self.budget.context_budget_shape_problem_component_count()
            && self.context_exhaustion_blocker_component_count
                == usize::from(!self.budget.can_generate())
            && self.component_accounting_consistent
                == self.budget.backend_request_accounting_is_consistent()
    }
}

impl RuntimeMetadata {
    pub fn new(
        model_id: impl Into<String>,
        tokenizer: impl Into<String>,
        native_context_window: usize,
        embedding_dimensions: usize,
    ) -> Self {
        Self {
            model_id: model_id.into(),
            tokenizer: tokenizer.into(),
            native_context_window,
            embedding_dimensions,
            supports_kv_import: false,
            supports_kv_export: false,
            max_kv_import_blocks: 0,
            max_kv_export_blocks: 0,
            hot_kv_precision_bits: 8,
            cold_kv_precision_bits: 4,
        }
    }

    pub fn with_kv_exchange(mut self, import: bool, export: bool) -> Self {
        self.supports_kv_import = import;
        self.supports_kv_export = export;
        self.max_kv_import_blocks = if import {
            self.max_kv_import_blocks.max(8)
        } else {
            0
        };
        self.max_kv_export_blocks = if export {
            self.max_kv_export_blocks.max(4)
        } else {
            0
        };
        self
    }

    pub fn with_kv_limits(mut self, max_import_blocks: usize, max_export_blocks: usize) -> Self {
        self.max_kv_import_blocks = if self.supports_kv_import {
            max_import_blocks.max(1)
        } else {
            0
        };
        self.max_kv_export_blocks = if self.supports_kv_export {
            max_export_blocks.max(1)
        } else {
            0
        };
        self
    }

    pub fn with_kv_precision(mut self, hot_bits: u8, cold_bits: u8) -> Self {
        let hot_bits = if matches!(hot_bits, 4 | 8) {
            hot_bits
        } else {
            8
        };
        let cold_bits = if matches!(cold_bits, 4 | 8) {
            cold_bits
        } else {
            4
        };
        self.hot_kv_precision_bits = hot_bits;
        self.cold_kv_precision_bits = cold_bits.min(hot_bits);
        self
    }

    pub fn generation_budget(
        &self,
        prompt_tokens: usize,
        requested_max_tokens: usize,
    ) -> RuntimeGenerationBudget {
        RuntimeGenerationBudget::new(
            prompt_tokens,
            requested_max_tokens,
            self.native_context_window,
        )
    }

    pub fn shape_summary(&self) -> RuntimeMetadataShapeSummary {
        RuntimeMetadataShapeSummary {
            native_context_window: self.native_context_window,
            embedding_dimensions: self.embedding_dimensions,
            supports_kv_import: self.supports_kv_import,
            supports_kv_export: self.supports_kv_export,
            max_kv_import_blocks: self.max_kv_import_blocks,
            max_kv_export_blocks: self.max_kv_export_blocks,
            hot_kv_precision_bits: self.hot_kv_precision_bits,
            cold_kv_precision_bits: self.cold_kv_precision_bits,
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "model_id={} tokenizer={} native_context_window={} embedding_dimensions={} kv_import={} kv_export={} max_kv_import_blocks={} max_kv_export_blocks={} kv_bits={}/{}",
            self.model_id,
            self.tokenizer,
            self.native_context_window,
            self.embedding_dimensions,
            self.supports_kv_import,
            self.supports_kv_export,
            self.max_kv_import_blocks,
            self.max_kv_export_blocks,
            self.hot_kv_precision_bits,
            self.cold_kv_precision_bits
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeGenerationBudget {
    pub prompt_tokens: usize,
    pub requested_max_tokens: usize,
    pub max_generated_tokens: usize,
    pub requested_context_tokens: usize,
    pub planned_context_tokens: usize,
    pub remaining_context_tokens: Option<usize>,
    pub truncated_by_context: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeToolResultProjectionBudget {
    pub source_characters: usize,
    pub projected_characters: usize,
    pub tokens_before: usize,
    pub tokens_after: usize,
    pub tokens_saved: usize,
    pub handle_present: bool,
    pub digest_only: bool,
}

impl RuntimeToolResultProjectionBudget {
    pub fn new(
        source_characters: usize,
        projected_characters: usize,
        handle_present: bool,
        digest_only: bool,
    ) -> Self {
        let tokens_before = estimate_text_tokens(source_characters);
        let tokens_after = estimate_text_tokens(projected_characters).min(tokens_before);
        Self {
            source_characters,
            projected_characters,
            tokens_before,
            tokens_after,
            tokens_saved: tokens_before.saturating_sub(tokens_after),
            handle_present,
            digest_only,
        }
    }

    pub fn accounting_is_consistent(self) -> bool {
        self.tokens_before == estimate_text_tokens(self.source_characters)
            && self.tokens_after
                == estimate_text_tokens(self.projected_characters).min(self.tokens_before)
            && self.tokens_saved == self.tokens_before.saturating_sub(self.tokens_after)
            && self.tokens_after <= self.tokens_before
    }
}

fn estimate_text_tokens(characters: usize) -> usize {
    characters.saturating_add(3) / 4
}

impl RuntimeGenerationBudget {
    pub fn new(
        prompt_tokens: usize,
        requested_max_tokens: usize,
        native_context_window: usize,
    ) -> Self {
        let requested_max_tokens = requested_max_tokens.max(1);
        let requested_context_tokens = prompt_tokens.saturating_add(requested_max_tokens);

        if native_context_window == 0 {
            return Self {
                prompt_tokens,
                requested_max_tokens,
                max_generated_tokens: requested_max_tokens,
                requested_context_tokens,
                planned_context_tokens: requested_context_tokens,
                remaining_context_tokens: None,
                truncated_by_context: false,
            };
        }

        let remaining_context_tokens = native_context_window.saturating_sub(prompt_tokens);
        let max_generated_tokens = requested_max_tokens.min(remaining_context_tokens);
        let planned_context_tokens = prompt_tokens
            .saturating_add(max_generated_tokens)
            .min(native_context_window);

        Self {
            prompt_tokens,
            requested_max_tokens,
            max_generated_tokens,
            requested_context_tokens,
            planned_context_tokens,
            remaining_context_tokens: Some(remaining_context_tokens),
            truncated_by_context: requested_context_tokens > native_context_window,
        }
    }

    pub fn can_generate(self) -> bool {
        self.max_generated_tokens > 0
    }

    pub fn has_known_context_window(self) -> bool {
        self.remaining_context_tokens.is_some()
    }

    pub fn context_window_tokens(self) -> Option<usize> {
        self.remaining_context_tokens.map(|remaining| {
            if remaining == 0 && self.prompt_tokens > self.planned_context_tokens {
                self.planned_context_tokens
            } else {
                self.prompt_tokens.saturating_add(remaining)
            }
        })
    }

    pub fn requested_context_overflow_tokens(self) -> usize {
        self.requested_context_tokens
            .saturating_sub(self.planned_context_tokens)
    }

    pub fn requested_context_overflow_component_count(self) -> usize {
        usize::from(self.requested_context_overflow_tokens() > 0)
    }

    pub fn requested_generation_deficit_tokens(self) -> usize {
        self.requested_max_tokens
            .saturating_sub(self.max_generated_tokens)
    }

    pub fn requested_generation_deficit_component_count(self) -> usize {
        usize::from(self.requested_generation_deficit_tokens() > 0)
    }

    pub fn context_exhausted(self) -> bool {
        self.has_known_context_window()
            && self.remaining_context_tokens == Some(0)
            && !self.can_generate()
    }

    pub fn context_exhaustion_component_count(self) -> usize {
        usize::from(self.context_exhausted())
    }

    pub fn truncated_but_can_generate(self) -> bool {
        self.truncated_by_context && self.can_generate()
    }

    pub fn context_soft_limit_component_count(self) -> usize {
        usize::from(self.truncated_but_can_generate())
    }

    pub fn requested_generation_was_clamped(self) -> bool {
        self.max_generated_tokens < self.requested_max_tokens
    }

    pub fn context_budget_signal_component_count(self) -> usize {
        self.requested_context_overflow_component_count()
            .saturating_add(self.requested_generation_deficit_component_count())
            .saturating_add(self.context_exhaustion_component_count())
            .saturating_add(self.context_soft_limit_component_count())
    }

    pub fn has_context_budget_signal_components(self) -> bool {
        self.context_budget_signal_component_count() > 0
    }

    pub fn requested_context_matches_parts(self) -> bool {
        self.requested_context_tokens
            == self.prompt_tokens.saturating_add(self.requested_max_tokens)
    }

    pub fn max_generated_not_above_requested(self) -> bool {
        self.max_generated_tokens <= self.requested_max_tokens
    }

    pub fn planned_context_matches_generation(self) -> bool {
        self.planned_context_tokens
            .saturating_sub(self.prompt_tokens)
            == self.max_generated_tokens
    }

    pub fn planned_context_not_above_requested(self) -> bool {
        self.planned_context_tokens <= self.requested_context_tokens
    }

    pub fn known_context_window_bounds_are_consistent(self) -> bool {
        match self.context_window_tokens() {
            Some(window) => {
                self.planned_context_tokens <= window
                    && self.remaining_context_tokens.unwrap_or(0) <= window
            }
            None => true,
        }
    }

    pub fn context_budget_shape_problem_component_count(self) -> usize {
        usize::from(!self.requested_context_matches_parts())
            + usize::from(!self.max_generated_not_above_requested())
            + usize::from(!self.planned_context_matches_generation())
            + usize::from(!self.planned_context_not_above_requested())
            + usize::from(!self.known_context_window_bounds_are_consistent())
    }

    pub fn has_context_budget_shape_problem_components(self) -> bool {
        self.context_budget_shape_problem_component_count() > 0
    }

    pub fn backend_request_signal_component_count(self) -> usize {
        self.context_budget_signal_component_count()
    }

    pub fn has_backend_request_signals(self) -> bool {
        self.backend_request_signal_component_count() > 0
    }

    pub fn backend_request_blocker_component_count(self) -> usize {
        self.context_budget_shape_problem_component_count()
            .saturating_add(usize::from(!self.can_generate()))
    }

    pub fn has_backend_request_blockers(self) -> bool {
        self.backend_request_blocker_component_count() > 0
    }

    pub fn context_budget_shape_accounting_is_consistent(self) -> bool {
        let expected_problem_count = usize::from(!self.requested_context_matches_parts())
            .saturating_add(usize::from(!self.max_generated_not_above_requested()))
            .saturating_add(usize::from(!self.planned_context_matches_generation()))
            .saturating_add(usize::from(!self.planned_context_not_above_requested()))
            .saturating_add(usize::from(
                !self.known_context_window_bounds_are_consistent(),
            ));

        self.context_budget_shape_problem_component_count() == expected_problem_count
            && self.has_context_budget_shape_problem_components() == (expected_problem_count > 0)
    }

    pub fn context_budget_shape_is_clean(self) -> bool {
        !self.has_context_budget_shape_problem_components()
            && self.context_budget_accounting_is_consistent()
            && self.context_budget_shape_accounting_is_consistent()
    }

    pub fn can_use_backend_max_tokens(self) -> bool {
        self.can_generate() && self.context_budget_shape_is_clean()
    }

    pub fn backend_request_accounting_is_consistent(self) -> bool {
        let expected_blocker_count = self
            .context_budget_shape_problem_component_count()
            .saturating_add(usize::from(!self.can_generate()));

        self.context_budget_accounting_is_consistent()
            && self.context_budget_shape_accounting_is_consistent()
            && self.backend_request_signal_component_count()
                == self.context_budget_signal_component_count()
            && self.has_backend_request_signals()
                == (self.backend_request_signal_component_count() > 0)
            && self.backend_request_blocker_component_count() == expected_blocker_count
            && self.has_backend_request_blockers() == (expected_blocker_count > 0)
    }

    pub fn backend_request_commit_is_clean(self) -> bool {
        !self.has_backend_request_blockers() && self.backend_request_accounting_is_consistent()
    }

    pub fn can_commit_backend_max_tokens(self) -> bool {
        self.backend_request_commit_is_clean()
    }

    pub fn context_budget_accounting_is_consistent(self) -> bool {
        let expected_signal_count = usize::from(self.requested_context_overflow_tokens() > 0)
            .saturating_add(usize::from(self.requested_generation_deficit_tokens() > 0))
            .saturating_add(usize::from(self.context_exhausted()))
            .saturating_add(usize::from(self.truncated_but_can_generate()));

        self.context_budget_signal_component_count() == expected_signal_count
            && self.has_context_budget_signal_components() == (expected_signal_count > 0)
            && self.requested_generation_was_clamped()
                == (self.requested_generation_deficit_tokens() > 0)
            && self.truncated_by_context == (self.requested_context_overflow_tokens() > 0)
            && !(self.context_exhausted() && self.truncated_but_can_generate())
    }

    pub fn planned_context_fraction(self) -> Option<f32> {
        fraction_of_window(self.planned_context_tokens, self.context_window_tokens()?)
    }

    pub fn remaining_context_fraction(self) -> Option<f32> {
        fraction_of_window(
            self.remaining_context_tokens?,
            self.context_window_tokens()?,
        )
    }

    pub fn requested_generation_fraction(self) -> f32 {
        if self.requested_max_tokens == 0 {
            0.0
        } else {
            self.max_generated_tokens as f32 / self.requested_max_tokens as f32
        }
    }

    pub fn backend_max_tokens_commit_summary(self) -> RuntimeBackendMaxTokensCommitSummary {
        RuntimeBackendMaxTokensCommitSummary::new(self)
    }
}

fn fraction_of_window(value: usize, window: usize) -> Option<f32> {
    (window > 0).then_some(value as f32 / window as f32)
}

fn valid_kv_precision_bits(bits: u8) -> bool {
    matches!(bits, 4 | 8)
}

impl Default for RuntimeMetadata {
    fn default() -> Self {
        Self {
            model_id: "unknown-runtime".to_owned(),
            tokenizer: "unknown".to_owned(),
            native_context_window: 0,
            embedding_dimensions: 0,
            supports_kv_import: false,
            supports_kv_export: false,
            max_kv_import_blocks: 0,
            max_kv_export_blocks: 0,
            hot_kv_precision_bits: 8,
            cold_kv_precision_bits: 4,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kv_precision_is_clamped_to_supported_bits() {
        let metadata = RuntimeMetadata::new("model", "tok", 4096, 2048).with_kv_precision(4, 8);

        assert_eq!(metadata.hot_kv_precision_bits, 4);
        assert_eq!(metadata.cold_kv_precision_bits, 4);
        assert!(metadata.summary().contains("kv_bits=4/4"));
    }

    #[test]
    fn runtime_metadata_shape_summary_reports_adapter_capabilities() {
        let metadata = RuntimeMetadata::new("model", "tok", 4096, 2048)
            .with_kv_exchange(true, true)
            .with_kv_limits(3, 2)
            .with_kv_precision(8, 4);

        let summary = metadata.shape_summary();

        assert_eq!(summary.native_context_window, 4096);
        assert_eq!(summary.embedding_dimensions, 2048);
        assert!(summary.supports_kv_import);
        assert!(summary.supports_kv_export);
        assert_eq!(summary.max_kv_import_blocks, 3);
        assert_eq!(summary.max_kv_export_blocks, 2);
        assert_eq!(summary.hot_kv_precision_bits, 8);
        assert_eq!(summary.cold_kv_precision_bits, 4);
        assert!(summary.has_known_context_window());
        assert!(summary.has_embedding_dimensions());
        assert!(summary.supports_any_kv_exchange());
        assert_eq!(summary.kv_exchange_block_capacity(), 5);
        assert!(summary.kv_precision_is_compressed());
        assert!(summary.cold_kv_not_wider_than_hot());
        assert!(summary.has_valid_hot_kv_precision());
        assert!(summary.has_valid_cold_kv_precision());
        assert_eq!(summary.context_shape_signal_component_count(), 2);
        assert_eq!(summary.kv_import_capability_signal_component_count(), 2);
        assert_eq!(summary.kv_export_capability_signal_component_count(), 2);
        assert_eq!(summary.kv_exchange_capability_signal_component_count(), 4);
        assert_eq!(summary.kv_precision_signal_component_count(), 3);
        assert_eq!(summary.metadata_shape_signal_component_count(), 9);
        assert!(summary.has_metadata_shape_signals());
        assert_eq!(summary.kv_import_contract_problem_component_count(), 0);
        assert_eq!(summary.kv_export_contract_problem_component_count(), 0);
        assert_eq!(summary.kv_exchange_contract_problem_component_count(), 0);
        assert_eq!(summary.kv_precision_problem_component_count(), 0);
        assert_eq!(summary.metadata_shape_problem_component_count(), 0);
        assert!(!summary.has_metadata_shape_problem_components());
        assert_eq!(summary.runtime_metadata_adapter_signal_component_count(), 9);
        assert!(summary.has_runtime_metadata_adapter_signals());
        assert_eq!(
            summary.runtime_metadata_adapter_missing_component_count(),
            0
        );
        assert_eq!(
            summary.runtime_metadata_adapter_blocker_component_count(),
            0
        );
        assert!(!summary.has_runtime_metadata_adapter_blockers());
        assert!(summary.metadata_shape_accounting_is_consistent());
        assert!(summary.metadata_shape_is_clean());
        assert!(summary.runtime_metadata_adapter_accounting_is_consistent());
        assert!(summary.runtime_metadata_adapter_commit_is_clean());
        assert!(summary.can_commit_runtime_metadata_adapter());
        assert!(summary.can_use_runtime_metadata_contract());

        let budget = metadata.generation_budget(4000, 256);

        assert_eq!(budget.max_generated_tokens, 96);
        assert_eq!(budget.requested_context_tokens, 4256);
        assert_eq!(budget.planned_context_tokens, 4096);
        assert!(budget.truncated_by_context);
    }

    #[test]
    fn runtime_metadata_disabled_kv_limits_normalize_to_clean_zero_capacity() {
        let metadata = RuntimeMetadata::new("model", "tok", 4096, 2048)
            .with_kv_exchange(false, false)
            .with_kv_limits(8, 8);
        let summary = metadata.shape_summary();

        assert!(!metadata.supports_kv_import);
        assert!(!metadata.supports_kv_export);
        assert_eq!(metadata.max_kv_import_blocks, 0);
        assert_eq!(metadata.max_kv_export_blocks, 0);
        assert!(!summary.supports_any_kv_exchange());
        assert_eq!(summary.kv_exchange_block_capacity(), 0);
        assert_eq!(summary.kv_import_capability_signal_component_count(), 0);
        assert_eq!(summary.kv_export_capability_signal_component_count(), 0);
        assert_eq!(summary.kv_exchange_contract_problem_component_count(), 0);
        assert!(!summary.has_metadata_shape_problem_components());
        assert!(summary.metadata_shape_accounting_is_consistent());
        assert!(summary.metadata_shape_is_clean());
        assert_eq!(
            summary.runtime_metadata_adapter_missing_component_count(),
            0
        );
        assert_eq!(
            summary.runtime_metadata_adapter_blocker_component_count(),
            0
        );
        assert!(summary.runtime_metadata_adapter_accounting_is_consistent());
        assert!(summary.runtime_metadata_adapter_commit_is_clean());
        assert!(summary.can_commit_runtime_metadata_adapter());
        assert!(summary.can_use_runtime_metadata_contract());
    }

    #[test]
    fn runtime_metadata_shape_summary_preserves_unknown_shapes() {
        let summary = RuntimeMetadata::default().shape_summary();

        assert_eq!(summary.native_context_window, 0);
        assert_eq!(summary.embedding_dimensions, 0);
        assert!(!summary.has_known_context_window());
        assert!(!summary.has_embedding_dimensions());
        assert!(!summary.supports_any_kv_exchange());
        assert_eq!(summary.kv_exchange_block_capacity(), 0);
        assert!(summary.kv_precision_is_compressed());
        assert!(summary.cold_kv_not_wider_than_hot());
        assert!(summary.has_valid_hot_kv_precision());
        assert!(summary.has_valid_cold_kv_precision());
        assert_eq!(summary.context_shape_signal_component_count(), 0);
        assert_eq!(summary.kv_exchange_capability_signal_component_count(), 0);
        assert_eq!(summary.kv_precision_signal_component_count(), 3);
        assert_eq!(summary.metadata_shape_signal_component_count(), 3);
        assert!(summary.has_metadata_shape_signals());
        assert_eq!(summary.metadata_shape_problem_component_count(), 0);
        assert!(!summary.has_metadata_shape_problem_components());
        assert_eq!(summary.runtime_metadata_adapter_signal_component_count(), 3);
        assert!(summary.has_runtime_metadata_adapter_signals());
        assert_eq!(
            summary.runtime_metadata_adapter_missing_component_count(),
            2
        );
        assert_eq!(
            summary.runtime_metadata_adapter_blocker_component_count(),
            2
        );
        assert!(summary.has_runtime_metadata_adapter_blockers());
        assert!(summary.metadata_shape_accounting_is_consistent());
        assert!(summary.metadata_shape_is_clean());
        assert!(summary.runtime_metadata_adapter_accounting_is_consistent());
        assert!(!summary.runtime_metadata_adapter_commit_is_clean());
        assert!(!summary.can_commit_runtime_metadata_adapter());
        assert!(summary.can_use_runtime_metadata_contract());
    }

    #[test]
    fn runtime_metadata_readiness_degrades_without_committing_missing_embeddings() {
        let metadata = RuntimeMetadata::new("model", "tok", 4096, 0)
            .with_kv_exchange(true, false)
            .with_kv_limits(2, 9);
        let summary = metadata.shape_summary();
        let budget = metadata.generation_budget(4000, 256);

        assert!(summary.has_known_context_window());
        assert!(!summary.has_embedding_dimensions());
        assert!(summary.supports_kv_import);
        assert!(!summary.supports_kv_export);
        assert_eq!(summary.max_kv_import_blocks, 2);
        assert_eq!(summary.max_kv_export_blocks, 0);
        assert_eq!(
            summary.runtime_metadata_adapter_missing_component_count(),
            1
        );
        assert_eq!(
            summary.runtime_metadata_adapter_blocker_component_count(),
            1
        );
        assert!(summary.has_runtime_metadata_adapter_blockers());
        assert!(summary.metadata_shape_is_clean());
        assert!(summary.can_use_runtime_metadata_contract());
        assert!(summary.runtime_metadata_adapter_accounting_is_consistent());
        assert!(!summary.runtime_metadata_adapter_commit_is_clean());
        assert!(!summary.can_commit_runtime_metadata_adapter());

        assert_eq!(budget.max_generated_tokens, 96);
        assert_eq!(budget.requested_generation_deficit_tokens(), 160);
        assert!(budget.truncated_but_can_generate());
        assert!(budget.can_use_backend_max_tokens());
        assert!(budget.can_commit_backend_max_tokens());
    }

    #[test]
    fn runtime_metadata_shape_summary_counts_contract_problems() {
        let summary = RuntimeMetadataShapeSummary {
            native_context_window: 8192,
            embedding_dimensions: 0,
            supports_kv_import: true,
            supports_kv_export: false,
            max_kv_import_blocks: 0,
            max_kv_export_blocks: 3,
            hot_kv_precision_bits: 6,
            cold_kv_precision_bits: 8,
        };

        assert!(summary.has_known_context_window());
        assert!(!summary.has_embedding_dimensions());
        assert!(!summary.has_valid_hot_kv_precision());
        assert!(summary.has_valid_cold_kv_precision());
        assert!(!summary.cold_kv_not_wider_than_hot());
        assert_eq!(summary.context_shape_signal_component_count(), 1);
        assert_eq!(summary.kv_import_capability_signal_component_count(), 1);
        assert_eq!(summary.kv_export_capability_signal_component_count(), 1);
        assert_eq!(summary.kv_exchange_capability_signal_component_count(), 2);
        assert_eq!(summary.kv_precision_signal_component_count(), 1);
        assert_eq!(summary.metadata_shape_signal_component_count(), 4);
        assert!(summary.has_metadata_shape_signals());
        assert_eq!(summary.kv_import_contract_problem_component_count(), 1);
        assert_eq!(summary.kv_export_contract_problem_component_count(), 1);
        assert_eq!(summary.kv_exchange_contract_problem_component_count(), 2);
        assert_eq!(summary.kv_precision_problem_component_count(), 2);
        assert_eq!(summary.metadata_shape_problem_component_count(), 4);
        assert!(summary.has_metadata_shape_problem_components());
        assert_eq!(summary.runtime_metadata_adapter_signal_component_count(), 4);
        assert!(summary.has_runtime_metadata_adapter_signals());
        assert_eq!(
            summary.runtime_metadata_adapter_missing_component_count(),
            1
        );
        assert_eq!(
            summary.runtime_metadata_adapter_blocker_component_count(),
            5
        );
        assert!(summary.has_runtime_metadata_adapter_blockers());
        assert!(summary.metadata_shape_accounting_is_consistent());
        assert!(!summary.metadata_shape_is_clean());
        assert!(summary.runtime_metadata_adapter_accounting_is_consistent());
        assert!(!summary.runtime_metadata_adapter_commit_is_clean());
        assert!(!summary.can_commit_runtime_metadata_adapter());
        assert!(!summary.can_use_runtime_metadata_contract());
    }

    #[test]
    fn runtime_generation_budget_preserves_unknown_context_window() {
        let metadata = RuntimeMetadata::new("model", "tok", 0, 2048);

        let budget = metadata.generation_budget(900, 0);

        assert_eq!(budget.max_generated_tokens, 1);
        assert_eq!(budget.requested_context_tokens, 901);
        assert_eq!(budget.planned_context_tokens, 901);
        assert_eq!(budget.remaining_context_tokens, None);
        assert!(!budget.truncated_by_context);
        assert!(budget.can_generate());
        assert!(!budget.has_known_context_window());
        assert_eq!(budget.context_window_tokens(), None);
        assert_eq!(budget.requested_context_overflow_tokens(), 0);
        assert_eq!(budget.requested_context_overflow_component_count(), 0);
        assert_eq!(budget.requested_generation_deficit_tokens(), 0);
        assert_eq!(budget.requested_generation_deficit_component_count(), 0);
        assert!(!budget.context_exhausted());
        assert_eq!(budget.context_exhaustion_component_count(), 0);
        assert!(!budget.truncated_but_can_generate());
        assert_eq!(budget.context_soft_limit_component_count(), 0);
        assert!(!budget.requested_generation_was_clamped());
        assert_eq!(budget.context_budget_signal_component_count(), 0);
        assert!(!budget.has_context_budget_signal_components());
        assert!(budget.context_budget_accounting_is_consistent());
        assert!(budget.requested_context_matches_parts());
        assert!(budget.max_generated_not_above_requested());
        assert!(budget.planned_context_matches_generation());
        assert!(budget.planned_context_not_above_requested());
        assert!(budget.known_context_window_bounds_are_consistent());
        assert_eq!(budget.context_budget_shape_problem_component_count(), 0);
        assert!(!budget.has_context_budget_shape_problem_components());
        assert_eq!(budget.backend_request_signal_component_count(), 0);
        assert!(!budget.has_backend_request_signals());
        assert_eq!(budget.backend_request_blocker_component_count(), 0);
        assert!(!budget.has_backend_request_blockers());
        assert!(budget.context_budget_shape_accounting_is_consistent());
        assert!(budget.context_budget_shape_is_clean());
        assert!(budget.backend_request_accounting_is_consistent());
        assert!(budget.backend_request_commit_is_clean());
        assert!(budget.can_commit_backend_max_tokens());
        assert!(budget.can_use_backend_max_tokens());
        let commit = budget.backend_max_tokens_commit_summary();
        assert_eq!(
            commit.action,
            RuntimeBackendMaxTokensCommitAction::CommitBackendMaxTokens
        );
        assert!(commit.action_can_commit());
        assert!(!commit.action_should_return_context_exhausted());
        assert!(!commit.action_should_repair_context_budget());
        assert_eq!(commit.backend_max_tokens, Some(1));
        assert!(commit.can_commit_backend_max_tokens());
        assert!(!commit.should_return_runtime_context_exhausted());
        assert!(!commit.should_repair_context_budget());
        assert!(commit.can_use_backend_max_tokens());
        assert_eq!(commit.total_signal_component_count, 0);
        assert_eq!(commit.total_blocker_component_count, 0);
        assert_eq!(commit.context_budget_shape_problem_component_count, 0);
        assert_eq!(commit.context_exhaustion_blocker_component_count, 0);
        assert!(commit.component_accounting_consistent);
        assert!(commit.commit_decision_accounting_is_consistent());
        assert_eq!(budget.planned_context_fraction(), None);
        assert_eq!(budget.remaining_context_fraction(), None);
        assert_eq!(budget.requested_generation_fraction(), 1.0);
    }

    #[test]
    fn runtime_generation_budget_clamps_to_context_window() {
        let metadata = RuntimeMetadata::new("model", "tok", 1024, 2048);

        let budget = metadata.generation_budget(1000, 128);

        assert_eq!(budget.max_generated_tokens, 24);
        assert_eq!(budget.requested_context_tokens, 1128);
        assert_eq!(budget.planned_context_tokens, 1024);
        assert_eq!(budget.remaining_context_tokens, Some(24));
        assert!(budget.truncated_by_context);
        assert!(budget.can_generate());
        assert!(budget.has_known_context_window());
        assert_eq!(budget.context_window_tokens(), Some(1024));
        assert_eq!(budget.requested_context_overflow_tokens(), 104);
        assert_eq!(budget.requested_context_overflow_component_count(), 1);
        assert_eq!(budget.requested_generation_deficit_tokens(), 104);
        assert_eq!(budget.requested_generation_deficit_component_count(), 1);
        assert!(!budget.context_exhausted());
        assert_eq!(budget.context_exhaustion_component_count(), 0);
        assert!(budget.truncated_but_can_generate());
        assert_eq!(budget.context_soft_limit_component_count(), 1);
        assert!(budget.requested_generation_was_clamped());
        assert_eq!(budget.context_budget_signal_component_count(), 3);
        assert!(budget.has_context_budget_signal_components());
        assert!(budget.context_budget_accounting_is_consistent());
        assert!(budget.requested_context_matches_parts());
        assert!(budget.max_generated_not_above_requested());
        assert!(budget.planned_context_matches_generation());
        assert!(budget.planned_context_not_above_requested());
        assert!(budget.known_context_window_bounds_are_consistent());
        assert_eq!(budget.context_budget_shape_problem_component_count(), 0);
        assert!(!budget.has_context_budget_shape_problem_components());
        assert_eq!(budget.backend_request_signal_component_count(), 3);
        assert!(budget.has_backend_request_signals());
        assert_eq!(budget.backend_request_blocker_component_count(), 0);
        assert!(!budget.has_backend_request_blockers());
        assert!(budget.context_budget_shape_accounting_is_consistent());
        assert!(budget.context_budget_shape_is_clean());
        assert!(budget.backend_request_accounting_is_consistent());
        assert!(budget.backend_request_commit_is_clean());
        assert!(budget.can_commit_backend_max_tokens());
        assert!(budget.can_use_backend_max_tokens());
        let commit = budget.backend_max_tokens_commit_summary();
        assert_eq!(
            commit.action,
            RuntimeBackendMaxTokensCommitAction::CommitBackendMaxTokens
        );
        assert_eq!(commit.backend_max_tokens, Some(24));
        assert!(commit.can_commit_backend_max_tokens());
        assert!(!commit.should_return_runtime_context_exhausted());
        assert!(!commit.should_repair_context_budget());
        assert_eq!(commit.total_signal_component_count, 3);
        assert_eq!(commit.total_blocker_component_count, 0);
        assert_eq!(commit.context_budget_shape_problem_component_count, 0);
        assert_eq!(commit.context_exhaustion_blocker_component_count, 0);
        assert!(commit.commit_decision_accounting_is_consistent());
        assert_eq!(budget.planned_context_fraction(), Some(1.0));
        assert!((budget.remaining_context_fraction().unwrap() - (24.0 / 1024.0)).abs() < 0.0001);
        assert!((budget.requested_generation_fraction() - (24.0 / 128.0)).abs() < 0.0001);
    }

    #[test]
    fn runtime_generation_budget_reports_exhausted_context() {
        let metadata = RuntimeMetadata::new("model", "tok", 1024, 2048);

        let budget = metadata.generation_budget(2048, 128);

        assert_eq!(budget.max_generated_tokens, 0);
        assert_eq!(budget.planned_context_tokens, 1024);
        assert_eq!(budget.remaining_context_tokens, Some(0));
        assert!(budget.truncated_by_context);
        assert!(!budget.can_generate());
        assert!(budget.has_known_context_window());
        assert_eq!(budget.context_window_tokens(), Some(1024));
        assert_eq!(budget.requested_context_overflow_tokens(), 1152);
        assert_eq!(budget.requested_context_overflow_component_count(), 1);
        assert_eq!(budget.requested_generation_deficit_tokens(), 128);
        assert_eq!(budget.requested_generation_deficit_component_count(), 1);
        assert!(budget.context_exhausted());
        assert_eq!(budget.context_exhaustion_component_count(), 1);
        assert!(!budget.truncated_but_can_generate());
        assert_eq!(budget.context_soft_limit_component_count(), 0);
        assert!(budget.requested_generation_was_clamped());
        assert_eq!(budget.context_budget_signal_component_count(), 3);
        assert!(budget.has_context_budget_signal_components());
        assert!(budget.context_budget_accounting_is_consistent());
        assert_eq!(budget.backend_request_signal_component_count(), 3);
        assert!(budget.has_backend_request_signals());
        assert_eq!(budget.backend_request_blocker_component_count(), 1);
        assert!(budget.has_backend_request_blockers());
        assert!(budget.context_budget_shape_is_clean());
        assert!(budget.backend_request_accounting_is_consistent());
        assert!(!budget.backend_request_commit_is_clean());
        assert!(!budget.can_commit_backend_max_tokens());
        assert!(!budget.can_use_backend_max_tokens());
        let commit = budget.backend_max_tokens_commit_summary();
        assert_eq!(
            commit.action,
            RuntimeBackendMaxTokensCommitAction::ReturnContextExhausted
        );
        assert!(!commit.action_can_commit());
        assert!(commit.action_should_return_context_exhausted());
        assert!(!commit.action_should_repair_context_budget());
        assert_eq!(commit.backend_max_tokens, None);
        assert!(!commit.can_commit_backend_max_tokens());
        assert!(commit.should_return_runtime_context_exhausted());
        assert!(!commit.should_repair_context_budget());
        assert!(!commit.can_use_backend_max_tokens());
        assert_eq!(commit.total_signal_component_count, 3);
        assert_eq!(commit.total_blocker_component_count, 1);
        assert_eq!(commit.context_budget_shape_problem_component_count, 0);
        assert_eq!(commit.context_exhaustion_blocker_component_count, 1);
        assert!(commit.component_accounting_consistent);
        assert!(commit.commit_decision_accounting_is_consistent());
        assert_eq!(budget.planned_context_fraction(), Some(1.0));
        assert_eq!(budget.remaining_context_fraction(), Some(0.0));
        assert_eq!(budget.requested_generation_fraction(), 0.0);
    }

    #[test]
    fn runtime_generation_budget_reports_exhausted_context_for_zero_requested_tokens() {
        let metadata = RuntimeMetadata::new("model", "tok", 1024, 2048);

        let budget = metadata.generation_budget(1024, 0);

        assert_eq!(budget.requested_max_tokens, 1);
        assert_eq!(budget.max_generated_tokens, 0);
        assert_eq!(budget.requested_context_tokens, 1025);
        assert_eq!(budget.planned_context_tokens, 1024);
        assert_eq!(budget.remaining_context_tokens, Some(0));
        assert!(budget.truncated_by_context);
        assert!(budget.context_exhausted());
        assert_eq!(budget.requested_context_overflow_tokens(), 1);
        assert_eq!(budget.requested_generation_deficit_tokens(), 1);
        assert_eq!(budget.context_budget_signal_component_count(), 3);
        assert_eq!(budget.backend_request_blocker_component_count(), 1);
        assert!(budget.context_budget_shape_is_clean());
        assert!(budget.backend_request_accounting_is_consistent());
        assert!(!budget.can_commit_backend_max_tokens());

        let commit = budget.backend_max_tokens_commit_summary();
        assert_eq!(
            commit.action,
            RuntimeBackendMaxTokensCommitAction::ReturnContextExhausted
        );
        assert_eq!(commit.backend_max_tokens, None);
        assert!(!commit.can_commit_backend_max_tokens());
        assert!(commit.should_return_runtime_context_exhausted());
        assert!(!commit.should_repair_context_budget());
        assert_eq!(commit.context_exhaustion_blocker_component_count, 1);
        assert!(commit.commit_decision_accounting_is_consistent());
    }

    #[test]
    fn runtime_generation_budget_reports_unclamped_known_context_pressure() {
        let metadata = RuntimeMetadata::new("model", "tok", 2048, 2048);

        let budget = metadata.generation_budget(512, 256);

        assert_eq!(budget.max_generated_tokens, 256);
        assert_eq!(budget.requested_context_tokens, 768);
        assert_eq!(budget.planned_context_tokens, 768);
        assert_eq!(budget.remaining_context_tokens, Some(1536));
        assert!(!budget.truncated_by_context);
        assert!(budget.can_generate());
        assert!(budget.has_known_context_window());
        assert_eq!(budget.context_window_tokens(), Some(2048));
        assert_eq!(budget.requested_context_overflow_tokens(), 0);
        assert_eq!(budget.requested_context_overflow_component_count(), 0);
        assert_eq!(budget.requested_generation_deficit_tokens(), 0);
        assert_eq!(budget.requested_generation_deficit_component_count(), 0);
        assert!(!budget.context_exhausted());
        assert_eq!(budget.context_exhaustion_component_count(), 0);
        assert!(!budget.truncated_but_can_generate());
        assert_eq!(budget.context_soft_limit_component_count(), 0);
        assert!(!budget.requested_generation_was_clamped());
        assert_eq!(budget.context_budget_signal_component_count(), 0);
        assert!(!budget.has_context_budget_signal_components());
        assert!(budget.context_budget_accounting_is_consistent());
        assert_eq!(budget.backend_request_signal_component_count(), 0);
        assert!(!budget.has_backend_request_signals());
        assert_eq!(budget.backend_request_blocker_component_count(), 0);
        assert!(!budget.has_backend_request_blockers());
        assert!(budget.context_budget_shape_is_clean());
        assert!(budget.backend_request_accounting_is_consistent());
        assert!(budget.backend_request_commit_is_clean());
        assert!(budget.can_commit_backend_max_tokens());
        assert!(budget.can_use_backend_max_tokens());
        assert!((budget.planned_context_fraction().unwrap() - (768.0 / 2048.0)).abs() < 0.0001);
        assert!((budget.remaining_context_fraction().unwrap() - (1536.0 / 2048.0)).abs() < 0.0001);
        assert_eq!(budget.requested_generation_fraction(), 1.0);
    }

    #[test]
    fn runtime_generation_budget_counts_public_shape_drift() {
        let budget = RuntimeGenerationBudget {
            prompt_tokens: 100,
            requested_max_tokens: 50,
            max_generated_tokens: 80,
            requested_context_tokens: 120,
            planned_context_tokens: 210,
            remaining_context_tokens: Some(20),
            truncated_by_context: false,
        };

        assert!(!budget.requested_context_matches_parts());
        assert!(!budget.max_generated_not_above_requested());
        assert!(!budget.planned_context_matches_generation());
        assert!(!budget.planned_context_not_above_requested());
        assert!(!budget.known_context_window_bounds_are_consistent());
        assert_eq!(budget.context_budget_shape_problem_component_count(), 5);
        assert!(budget.has_context_budget_shape_problem_components());
        assert_eq!(budget.backend_request_signal_component_count(), 0);
        assert!(!budget.has_backend_request_signals());
        assert_eq!(budget.backend_request_blocker_component_count(), 5);
        assert!(budget.has_backend_request_blockers());
        assert!(budget.context_budget_shape_accounting_is_consistent());
        assert!(budget.context_budget_accounting_is_consistent());
        assert!(budget.backend_request_accounting_is_consistent());
        assert!(!budget.context_budget_shape_is_clean());
        assert!(!budget.backend_request_commit_is_clean());
        assert!(budget.can_generate());
        assert!(!budget.can_commit_backend_max_tokens());
        assert!(!budget.can_use_backend_max_tokens());
        let commit = budget.backend_max_tokens_commit_summary();
        assert_eq!(
            commit.action,
            RuntimeBackendMaxTokensCommitAction::RepairContextBudget
        );
        assert!(!commit.action_can_commit());
        assert!(!commit.action_should_return_context_exhausted());
        assert!(commit.action_should_repair_context_budget());
        assert_eq!(commit.backend_max_tokens, None);
        assert!(!commit.can_commit_backend_max_tokens());
        assert!(!commit.should_return_runtime_context_exhausted());
        assert!(commit.should_repair_context_budget());
        assert!(!commit.can_use_backend_max_tokens());
        assert_eq!(commit.total_signal_component_count, 0);
        assert_eq!(commit.total_blocker_component_count, 5);
        assert_eq!(commit.context_budget_shape_problem_component_count, 5);
        assert_eq!(commit.context_exhaustion_blocker_component_count, 0);
        assert!(commit.component_accounting_consistent);
        assert!(commit.commit_decision_accounting_is_consistent());
    }

    #[test]
    fn tool_result_projection_budget_reports_bounded_token_savings() {
        let budget = RuntimeToolResultProjectionBudget::new(8_000, 800, true, false);

        assert_eq!(budget.tokens_before, 2_000);
        assert_eq!(budget.tokens_after, 200);
        assert_eq!(budget.tokens_saved, 1_800);
        assert!(budget.handle_present);
        assert!(!budget.digest_only);
        assert!(budget.accounting_is_consistent());
    }

    #[test]
    fn digest_only_tool_result_projection_never_reports_negative_savings() {
        let budget = RuntimeToolResultProjectionBudget::new(8, 64, true, true);

        assert_eq!(budget.tokens_before, 2);
        assert_eq!(budget.tokens_after, 2);
        assert_eq!(budget.tokens_saved, 0);
        assert!(budget.digest_only);
        assert!(budget.accounting_is_consistent());
    }
}
