use crate::engine::NoironEngine;

use super::super::super::{
    has_runtime_architecture_evidence, has_text, inspection_hardware_plan,
    runtime_adapter_selection_mismatch_count, runtime_kv_held_blocks,
    runtime_kv_precision_mismatch_count, runtime_kv_was_held,
};

#[derive(Debug, Clone, Copy)]
pub(super) struct RuntimeSignalCounts {
    pub(super) runtime_model_experience_count: usize,
    pub(super) runtime_adapter_experience_count: usize,
    pub(super) runtime_adapter_selection_mismatch_count: usize,
    pub(super) runtime_forward_energy_experience_count: usize,
    pub(super) runtime_kv_influence_experience_count: usize,
    pub(super) runtime_token_count: usize,
    pub(super) runtime_uncertainty_experience_count: usize,
    pub(super) runtime_uncertainty_token_count: usize,
    pub(super) runtime_architecture_experience_count: usize,
    pub(super) runtime_kv_precision_experience_count: usize,
    pub(super) runtime_kv_precision_mismatch_count: usize,
    pub(super) runtime_device_execution_experience_count: usize,
    pub(super) runtime_layer_mode_experience_count: usize,
    pub(super) runtime_all_layer_mode_experience_count: usize,
    pub(super) runtime_global_layers: usize,
    pub(super) runtime_local_window_layers: usize,
    pub(super) runtime_convolutional_fusion_layers: usize,
    pub(super) runtime_kv_import_experience_count: usize,
    pub(super) runtime_kv_export_experience_count: usize,
    pub(super) runtime_kv_hold_experience_count: usize,
    pub(super) runtime_kv_held_blocks: usize,
}

pub(super) fn runtime_signal_counts(engine: &NoironEngine) -> RuntimeSignalCounts {
    let hardware_plan = inspection_hardware_plan(engine);
    let runtime_model_experience_count = engine
        .experience
        .records()
        .iter()
        .filter(|record| has_text(record.runtime_diagnostics.model_id.as_deref()))
        .count();
    let runtime_adapter_experience_count = engine
        .experience
        .records()
        .iter()
        .filter(|record| has_text(record.runtime_diagnostics.selected_adapter.as_deref()))
        .count();
    let runtime_adapter_selection_mismatch_count =
        runtime_adapter_selection_mismatch_count(engine, &hardware_plan);
    let runtime_forward_energy_experience_count = engine
        .experience
        .records()
        .iter()
        .filter(|record| record.runtime_diagnostics.forward_energy.is_some())
        .count();
    let runtime_kv_influence_experience_count = engine
        .experience
        .records()
        .iter()
        .filter(|record| record.runtime_diagnostics.kv_influence.is_some())
        .count();
    let runtime_token_count = engine
        .experience
        .records()
        .iter()
        .map(|record| record.runtime_token_metrics.token_count)
        .sum();
    let runtime_uncertainty_experience_count = engine
        .experience
        .records()
        .iter()
        .filter(|record| record.runtime_token_metrics.has_uncertainty_signal())
        .count();
    let runtime_uncertainty_token_count = engine
        .experience
        .records()
        .iter()
        .map(|record| {
            record
                .runtime_token_metrics
                .entropy_count
                .saturating_add(record.runtime_token_metrics.logprob_count)
        })
        .sum();
    let runtime_architecture_experience_count = engine
        .experience
        .records()
        .iter()
        .filter(|record| has_runtime_architecture_evidence(record))
        .count();
    let runtime_kv_precision_experience_count = engine
        .experience
        .records()
        .iter()
        .filter(|record| record.runtime_diagnostics.has_valid_kv_precision_signal())
        .count();
    let runtime_kv_precision_mismatch_count =
        runtime_kv_precision_mismatch_count(engine, &hardware_plan);
    let runtime_device_execution_experience_count = engine
        .experience
        .records()
        .iter()
        .filter(|record| {
            record
                .runtime_diagnostics
                .has_runtime_reported_device_execution_signal()
        })
        .count();
    let runtime_layer_mode_experience_count = engine
        .experience
        .records()
        .iter()
        .filter(|record| record.runtime_diagnostics.has_layer_mode_signal())
        .count();
    let runtime_all_layer_mode_experience_count = engine
        .experience
        .records()
        .iter()
        .filter(|record| record.runtime_diagnostics.has_all_layer_modes())
        .count();
    let runtime_global_layers = engine
        .experience
        .records()
        .iter()
        .map(|record| record.runtime_diagnostics.global_layers)
        .sum();
    let runtime_local_window_layers = engine
        .experience
        .records()
        .iter()
        .map(|record| record.runtime_diagnostics.local_window_layers)
        .sum();
    let runtime_convolutional_fusion_layers = engine
        .experience
        .records()
        .iter()
        .map(|record| record.runtime_diagnostics.convolutional_fusion_layers)
        .sum();
    let runtime_kv_import_experience_count = engine
        .experience
        .records()
        .iter()
        .filter(|record| record.runtime_diagnostics.imported_kv_blocks > 0)
        .count();
    let runtime_kv_export_experience_count = engine
        .experience
        .records()
        .iter()
        .filter(|record| {
            record.runtime_diagnostics.exported_kv_blocks > 0
                || !record.stored_runtime_kv_memory_ids.is_empty()
        })
        .count();
    let runtime_kv_hold_experience_count = engine
        .experience
        .records()
        .iter()
        .filter(|record| runtime_kv_was_held(record))
        .count();
    let runtime_kv_held_blocks = engine
        .experience
        .records()
        .iter()
        .map(runtime_kv_held_blocks)
        .sum::<usize>();

    RuntimeSignalCounts {
        runtime_model_experience_count,
        runtime_adapter_experience_count,
        runtime_adapter_selection_mismatch_count,
        runtime_forward_energy_experience_count,
        runtime_kv_influence_experience_count,
        runtime_token_count,
        runtime_uncertainty_experience_count,
        runtime_uncertainty_token_count,
        runtime_architecture_experience_count,
        runtime_kv_precision_experience_count,
        runtime_kv_precision_mismatch_count,
        runtime_device_execution_experience_count,
        runtime_layer_mode_experience_count,
        runtime_all_layer_mode_experience_count,
        runtime_global_layers,
        runtime_local_window_layers,
        runtime_convolutional_fusion_layers,
        runtime_kv_import_experience_count,
        runtime_kv_export_experience_count,
        runtime_kv_hold_experience_count,
        runtime_kv_held_blocks,
    }
}
