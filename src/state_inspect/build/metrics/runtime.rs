use crate::engine::NoironEngine;
use crate::hardware::RuntimeAdapterHint;

use super::super::super::{
    has_runtime_architecture_evidence, has_text, inspection_hardware_plan,
    runtime_adapter_selection_mismatch_count, runtime_kv_held_blocks,
    runtime_kv_precision_mismatch_count, runtime_kv_was_held, FhtDkeBudgetInspectionStats,
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
    pub(super) runtime_imported_kv_blocks: usize,
    pub(super) runtime_kv_weak_import_skip_experience_count: usize,
    pub(super) weak_runtime_kv_imports_skipped: usize,
    pub(super) runtime_kv_weak_import_pressure_experience_count: usize,
    pub(super) runtime_kv_weak_import_pressure_avg: f32,
    pub(super) runtime_kv_weak_import_pressure_max: f32,
    pub(super) runtime_kv_budget_import_skip_experience_count: usize,
    pub(super) budget_limited_runtime_kv_imports_skipped: usize,
    pub(super) runtime_kv_budget_pressure_experience_count: usize,
    pub(super) runtime_kv_budget_pressure_avg: f32,
    pub(super) runtime_kv_budget_pressure_max: f32,
    pub(super) runtime_kv_export_experience_count: usize,
    pub(super) runtime_kv_segment_experience_count: usize,
    pub(super) runtime_kv_segments_included: usize,
    pub(super) runtime_kv_segments_skipped: usize,
    pub(super) runtime_kv_segments_rejected: usize,
    pub(super) runtime_kv_hold_experience_count: usize,
    pub(super) runtime_kv_held_blocks: usize,
    pub(super) fht_dke_budget_experience_count: usize,
    pub(super) fht_dke_enabled_experience_count: usize,
    pub(super) fht_dke_total_tokens: usize,
    pub(super) fht_dke_dense_tokens: usize,
    pub(super) fht_dke_routed_tokens: usize,
    pub(super) fht_dke_kv_exchange_blocks: usize,
    pub(super) fht_dke_token_split_valid_count: usize,
    pub(super) fht_dke_token_split_invalid_count: usize,
    pub(super) fht_dke_attention_threshold_experience_count: usize,
    pub(super) fht_dke_attention_threshold_avg: f32,
    pub(super) fht_dke_attention_threshold_max: f32,
    pub(super) fht_dke_route_pressure_experience_count: usize,
    pub(super) fht_dke_route_pressure_avg: f32,
    pub(super) fht_dke_route_pressure_max: f32,
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
        .filter(|record| {
            record
                .runtime_diagnostics
                .selected_adapter
                .as_deref()
                .and_then(RuntimeAdapterHint::canonical_name)
                .is_some()
        })
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
    let runtime_imported_kv_blocks = engine
        .experience
        .records()
        .iter()
        .map(|record| record.runtime_diagnostics.imported_kv_blocks)
        .sum();
    let runtime_kv_weak_import_skip_experience_count = engine
        .experience
        .records()
        .iter()
        .filter(|record| record.runtime_diagnostics.weak_runtime_kv_imports_skipped > 0)
        .count();
    let weak_runtime_kv_imports_skipped = engine
        .experience
        .records()
        .iter()
        .map(|record| record.runtime_diagnostics.weak_runtime_kv_imports_skipped)
        .sum();
    let runtime_kv_weak_import_pressures =
        engine.experience.records().iter().filter_map(|record| {
            runtime_kv_weak_import_pressure(
                record.runtime_diagnostics.imported_kv_blocks,
                record.runtime_diagnostics.weak_runtime_kv_imports_skipped,
            )
        });
    let mut runtime_kv_weak_import_pressure_experience_count = 0;
    let mut runtime_kv_weak_import_pressure_total = 0.0;
    let mut runtime_kv_weak_import_pressure_max = 0.0;
    for pressure in runtime_kv_weak_import_pressures {
        runtime_kv_weak_import_pressure_experience_count += 1;
        runtime_kv_weak_import_pressure_total += pressure;
        runtime_kv_weak_import_pressure_max =
            f32::max(runtime_kv_weak_import_pressure_max, pressure);
    }
    let runtime_kv_weak_import_pressure_avg =
        if runtime_kv_weak_import_pressure_experience_count == 0 {
            0.0
        } else {
            runtime_kv_weak_import_pressure_total
                / runtime_kv_weak_import_pressure_experience_count as f32
        };
    let runtime_kv_budget_import_skip_experience_count = engine
        .experience
        .records()
        .iter()
        .filter(|record| {
            record
                .runtime_diagnostics
                .budget_limited_runtime_kv_imports_skipped
                > 0
        })
        .count();
    let budget_limited_runtime_kv_imports_skipped = engine
        .experience
        .records()
        .iter()
        .map(|record| {
            record
                .runtime_diagnostics
                .budget_limited_runtime_kv_imports_skipped
        })
        .sum();
    let runtime_kv_budget_pressures = engine.experience.records().iter().filter_map(|record| {
        runtime_kv_budget_pressure(
            record.runtime_diagnostics.exported_kv_blocks,
            record
                .runtime_diagnostics
                .budget_limited_runtime_kv_imports_skipped,
        )
    });
    let mut runtime_kv_budget_pressure_experience_count = 0;
    let mut runtime_kv_budget_pressure_total = 0.0;
    let mut runtime_kv_budget_pressure_max = 0.0;
    for pressure in runtime_kv_budget_pressures {
        runtime_kv_budget_pressure_experience_count += 1;
        runtime_kv_budget_pressure_total += pressure;
        runtime_kv_budget_pressure_max = f32::max(runtime_kv_budget_pressure_max, pressure);
    }
    let runtime_kv_budget_pressure_avg = if runtime_kv_budget_pressure_experience_count == 0 {
        0.0
    } else {
        runtime_kv_budget_pressure_total / runtime_kv_budget_pressure_experience_count as f32
    };
    let runtime_kv_export_experience_count = engine
        .experience
        .records()
        .iter()
        .filter(|record| {
            record.runtime_diagnostics.exported_kv_blocks > 0
                || !record.stored_runtime_kv_memory_ids.is_empty()
        })
        .count();
    let runtime_kv_segment_experience_count = engine
        .experience
        .records()
        .iter()
        .filter(|record| record.runtime_diagnostics.has_runtime_kv_segment_signal())
        .count();
    let runtime_kv_segments_included = engine
        .experience
        .records()
        .iter()
        .map(|record| record.runtime_diagnostics.runtime_kv_segments_included)
        .sum();
    let runtime_kv_segments_skipped = engine
        .experience
        .records()
        .iter()
        .map(|record| record.runtime_diagnostics.runtime_kv_segments_skipped)
        .sum();
    let runtime_kv_segments_rejected = engine
        .experience
        .records()
        .iter()
        .map(|record| record.runtime_diagnostics.runtime_kv_segments_rejected)
        .sum();
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
    let fht_dke_budget_stats = engine
        .experience
        .records()
        .iter()
        .filter_map(|record| FhtDkeBudgetInspectionStats::from_notes(&record.process_reward.notes))
        .collect::<Vec<_>>();
    let fht_dke_budget_experience_count = fht_dke_budget_stats.len();
    let fht_dke_enabled_experience_count = fht_dke_budget_stats
        .iter()
        .map(|stats| stats.enabled)
        .sum::<usize>();
    let fht_dke_total_tokens = fht_dke_budget_stats
        .iter()
        .map(|stats| stats.total_tokens)
        .sum::<usize>();
    let fht_dke_dense_tokens = fht_dke_budget_stats
        .iter()
        .map(|stats| stats.dense_tokens)
        .sum::<usize>();
    let fht_dke_routed_tokens = fht_dke_budget_stats
        .iter()
        .map(|stats| stats.routed_tokens)
        .sum::<usize>();
    let fht_dke_kv_exchange_blocks = fht_dke_budget_stats
        .iter()
        .map(|stats| stats.kv_exchange_blocks)
        .sum::<usize>();
    let fht_dke_token_split_valid_count = fht_dke_budget_stats
        .iter()
        .map(|stats| stats.token_split_valid)
        .sum::<usize>();
    let fht_dke_token_split_invalid_count = fht_dke_budget_stats
        .iter()
        .map(|stats| stats.token_split_invalid)
        .sum::<usize>();
    let fht_dke_attention_threshold_experience_count = fht_dke_budget_stats
        .iter()
        .map(|stats| stats.attention_threshold_count)
        .sum::<usize>();
    let fht_dke_attention_threshold_total = fht_dke_budget_stats
        .iter()
        .map(|stats| stats.attention_threshold_total)
        .sum::<f32>();
    let fht_dke_attention_threshold_avg = if fht_dke_attention_threshold_experience_count == 0 {
        0.0
    } else {
        fht_dke_attention_threshold_total / fht_dke_attention_threshold_experience_count as f32
    };
    let fht_dke_attention_threshold_max = fht_dke_budget_stats
        .iter()
        .map(|stats| stats.attention_threshold_max)
        .fold(0.0, f32::max);
    let fht_dke_route_pressure_experience_count = fht_dke_budget_stats
        .iter()
        .map(|stats| stats.route_pressure_count)
        .sum::<usize>();
    let fht_dke_route_pressure_total = fht_dke_budget_stats
        .iter()
        .map(|stats| stats.route_pressure_total)
        .sum::<f32>();
    let fht_dke_route_pressure_avg = if fht_dke_route_pressure_experience_count == 0 {
        0.0
    } else {
        fht_dke_route_pressure_total / fht_dke_route_pressure_experience_count as f32
    };
    let fht_dke_route_pressure_max = fht_dke_budget_stats
        .iter()
        .map(|stats| stats.route_pressure_max)
        .fold(0.0, f32::max);

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
        runtime_imported_kv_blocks,
        runtime_kv_weak_import_skip_experience_count,
        weak_runtime_kv_imports_skipped,
        runtime_kv_weak_import_pressure_experience_count,
        runtime_kv_weak_import_pressure_avg,
        runtime_kv_weak_import_pressure_max,
        runtime_kv_budget_import_skip_experience_count,
        budget_limited_runtime_kv_imports_skipped,
        runtime_kv_budget_pressure_experience_count,
        runtime_kv_budget_pressure_avg,
        runtime_kv_budget_pressure_max,
        runtime_kv_export_experience_count,
        runtime_kv_segment_experience_count,
        runtime_kv_segments_included,
        runtime_kv_segments_skipped,
        runtime_kv_segments_rejected,
        runtime_kv_hold_experience_count,
        runtime_kv_held_blocks,
        fht_dke_budget_experience_count,
        fht_dke_enabled_experience_count,
        fht_dke_total_tokens,
        fht_dke_dense_tokens,
        fht_dke_routed_tokens,
        fht_dke_kv_exchange_blocks,
        fht_dke_token_split_valid_count,
        fht_dke_token_split_invalid_count,
        fht_dke_attention_threshold_experience_count,
        fht_dke_attention_threshold_avg,
        fht_dke_attention_threshold_max,
        fht_dke_route_pressure_experience_count,
        fht_dke_route_pressure_avg,
        fht_dke_route_pressure_max,
    }
}

fn runtime_kv_weak_import_pressure(imported: usize, weak_skipped: usize) -> Option<f32> {
    if weak_skipped == 0 {
        return None;
    }

    let total = imported.saturating_add(weak_skipped);
    Some((weak_skipped as f32 / total as f32).clamp(0.0, 1.0))
}

fn runtime_kv_budget_pressure(exported: usize, budget_skipped: usize) -> Option<f32> {
    if budget_skipped == 0 {
        return None;
    }

    let total = exported.saturating_add(budget_skipped);
    Some((budget_skipped as f32 / total as f32).clamp(0.0, 1.0))
}
