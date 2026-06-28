use super::InspectFlagParse;
use crate::cli::args::values::{parse_f32, parse_usize};

pub(crate) fn parse(
    parser: &mut InspectFlagParse<'_>,
    raw: &[String],
    index: usize,
) -> Option<usize> {
    match raw.get(index)?.as_str() {
        "--inspect-min-runtime-model-experiences" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_model_experiences = Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-runtime-adapter-experiences" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_adapter_experiences = Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-max-runtime-adapter-selection-mismatches" if index + 1 < raw.len() => {
            *parser.inspect_max_runtime_adapter_selection_mismatches =
                Some(parse_usize(&raw[index + 1], usize::MAX));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-runtime-forward-energy-experiences" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_forward_energy_experiences =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-runtime-kv-influence-experiences" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_kv_influence_experiences =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-runtime-tokens" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_tokens = Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-runtime-uncertainty-experiences" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_uncertainty_experiences =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-runtime-uncertainty-tokens" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_uncertainty_tokens = Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-runtime-architecture-experiences" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_architecture_experiences =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-runtime-kv-precision-experiences" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_kv_precision_experiences =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-max-runtime-kv-precision-mismatches" if index + 1 < raw.len() => {
            *parser.inspect_max_runtime_kv_precision_mismatches =
                Some(parse_usize(&raw[index + 1], usize::MAX));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-max-runtime-errors" if index + 1 < raw.len() => {
            *parser.inspect_max_runtime_errors = Some(parse_usize(&raw[index + 1], usize::MAX));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-max-runtime-timeouts" if index + 1 < raw.len() => {
            *parser.inspect_max_runtime_timeouts = Some(parse_usize(&raw[index + 1], usize::MAX));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-max-runtime-error-message-chars" if index + 1 < raw.len() => {
            *parser.inspect_max_runtime_error_message_chars =
                Some(parse_usize(&raw[index + 1], usize::MAX));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-runtime-device-execution-experiences" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_device_execution_experiences =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-runtime-layer-mode-experiences" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_layer_mode_experiences =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-runtime-all-layer-mode-experiences" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_all_layer_mode_experiences =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-runtime-global-layers" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_global_layers = Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-runtime-local-window-layers" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_local_window_layers = Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-runtime-convolutional-fusion-layers" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_convolutional_fusion_layers =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-runtime-kv-import-experiences" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_kv_import_experiences =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-runtime-imported-kv-blocks" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_imported_kv_blocks = Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-self-evolving-memory-writeback-experiences" if index + 1 < raw.len() => {
            *parser.inspect_min_self_evolving_memory_writeback_experiences =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-self-evolving-memory-writeback-attempted-records"
            if index + 1 < raw.len() =>
        {
            *parser.inspect_min_self_evolving_memory_writeback_attempted_records =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-self-evolving-memory-writeback-accepted-records"
            if index + 1 < raw.len() =>
        {
            *parser.inspect_min_self_evolving_memory_writeback_accepted_records =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-max-self-evolving-memory-writeback-rejected-records"
            if index + 1 < raw.len() =>
        {
            *parser.inspect_max_self_evolving_memory_writeback_rejected_records =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-self-evolving-memory-writeback-write-allowed" if index + 1 < raw.len() => {
            *parser.inspect_min_self_evolving_memory_writeback_write_allowed =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-self-evolving-memory-writeback-durable-write-allowed"
            if index + 1 < raw.len() =>
        {
            *parser.inspect_min_self_evolving_memory_writeback_durable_write_allowed =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-self-evolving-memory-writeback-applied" if index + 1 < raw.len() => {
            *parser.inspect_min_self_evolving_memory_writeback_applied =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-self-evolving-memory-writeback-applied-to-disk" if index + 1 < raw.len() => {
            *parser.inspect_min_self_evolving_memory_writeback_applied_to_disk =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-self-evolving-memory-writeback-snapshot-changes"
            if index + 1 < raw.len() =>
        {
            *parser.inspect_min_self_evolving_memory_writeback_snapshot_changes =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-runtime-kv-weak-import-skip-experiences" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_kv_weak_import_skip_experiences =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-weak-runtime-kv-imports-skipped" if index + 1 < raw.len() => {
            *parser.inspect_min_weak_runtime_kv_imports_skipped =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-runtime-kv-weak-import-pressure-experiences" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_kv_weak_import_pressure_experiences =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-runtime-kv-weak-import-pressure" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_kv_weak_import_pressure =
                Some(parse_f32(&raw[index + 1], 0.0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-max-runtime-kv-weak-import-pressure" if index + 1 < raw.len() => {
            *parser.inspect_max_runtime_kv_weak_import_pressure =
                Some(parse_f32(&raw[index + 1], 1.0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-runtime-kv-budget-import-skip-experiences" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_kv_budget_import_skip_experiences =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-budget-limited-runtime-kv-imports-skipped" if index + 1 < raw.len() => {
            *parser.inspect_min_budget_limited_runtime_kv_imports_skipped =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-runtime-kv-budget-pressure-experiences" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_kv_budget_pressure_experiences =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-runtime-kv-budget-pressure" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_kv_budget_pressure = Some(parse_f32(&raw[index + 1], 0.0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-max-runtime-kv-budget-pressure" if index + 1 < raw.len() => {
            *parser.inspect_max_runtime_kv_budget_pressure = Some(parse_f32(&raw[index + 1], 1.0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-runtime-kv-export-experiences" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_kv_export_experiences =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-runtime-kv-segment-experiences" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_kv_segment_experiences =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-runtime-kv-segments-included" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_kv_segments_included =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-max-runtime-kv-segments-skipped" if index + 1 < raw.len() => {
            *parser.inspect_max_runtime_kv_segments_skipped =
                Some(parse_usize(&raw[index + 1], usize::MAX));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-max-runtime-kv-segments-rejected" if index + 1 < raw.len() => {
            *parser.inspect_max_runtime_kv_segments_rejected =
                Some(parse_usize(&raw[index + 1], usize::MAX));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-runtime-kv-hold-experiences" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_kv_hold_experiences = Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-runtime-kv-held-blocks" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_kv_held_blocks = Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-fht-dke-budget-experiences" if index + 1 < raw.len() => {
            *parser.inspect_min_fht_dke_budget_experiences = Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-fht-dke-enabled-experiences" if index + 1 < raw.len() => {
            *parser.inspect_min_fht_dke_enabled_experiences = Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-fht-dke-routed-tokens" if index + 1 < raw.len() => {
            *parser.inspect_min_fht_dke_routed_tokens = Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-max-fht-dke-token-split-invalid" if index + 1 < raw.len() => {
            *parser.inspect_max_fht_dke_token_split_invalid =
                Some(parse_usize(&raw[index + 1], usize::MAX));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-fht-dke-attention-threshold" if index + 1 < raw.len() => {
            *parser.inspect_min_fht_dke_attention_threshold = Some(parse_f32(&raw[index + 1], 0.0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-max-fht-dke-attention-threshold" if index + 1 < raw.len() => {
            *parser.inspect_max_fht_dke_attention_threshold = Some(parse_f32(&raw[index + 1], 1.0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-fht-dke-route-pressure" if index + 1 < raw.len() => {
            *parser.inspect_min_fht_dke_route_pressure = Some(parse_f32(&raw[index + 1], 0.0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-max-fht-dke-route-pressure" if index + 1 < raw.len() => {
            *parser.inspect_max_fht_dke_route_pressure = Some(parse_f32(&raw[index + 1], 1.0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-require-runtime-kv-dimensions" => {
            *parser.inspect_require_runtime_kv_dimensions = true;
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(1)
        }
        _ => None,
    }
}
