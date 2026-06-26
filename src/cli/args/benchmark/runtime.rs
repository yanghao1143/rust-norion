use super::BenchmarkFlagParse;
use crate::cli::args::values::{parse_f32, parse_usize};

pub(crate) fn parse(
    parser: &mut BenchmarkFlagParse<'_>,
    raw: &[String],
    index: usize,
) -> Option<usize> {
    match raw.get(index)?.as_str() {
        "--benchmark-min-sparse-skipped-cases" if index + 1 < raw.len() => {
            *parser.benchmark_min_sparse_skipped_cases = Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-sparse-skipped-tokens" if index + 1 < raw.len() => {
            *parser.benchmark_min_sparse_skipped_tokens = Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-runtime-forward-cases" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_forward_cases = Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-runtime-forward-energy-cases" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_forward_energy_cases =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-runtime-kv-influence-cases" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_kv_influence_cases =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-runtime-architecture-cases" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_architecture_cases =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-runtime-architecture-device-profiles" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_architecture_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-runtime-kv-precision-cases" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_kv_precision_cases =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-runtime-layer-mode-cases" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_layer_mode_cases = Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-runtime-all-layer-mode-cases" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_all_layer_mode_cases =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-runtime-global-layers" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_global_layers = Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-runtime-local-window-layers" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_local_window_layers =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-runtime-convolutional-fusion-layers" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_convolutional_fusion_layers =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-runtime-uncertainty-cases" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_uncertainty_cases = Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-runtime-uncertainty-tokens" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_uncertainty_tokens =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-runtime-uncertainty-device-profiles" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_uncertainty_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--benchmark-min-runtime-uncertainty-token-device-profiles" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_uncertainty_token_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--benchmark-min-runtime-kv-import-cases" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_kv_import_cases = Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-runtime-kv-weak-import-skip-cases" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_kv_weak_import_skip_cases =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-weak-runtime-kv-imports-skipped" if index + 1 < raw.len() => {
            *parser.benchmark_min_weak_runtime_kv_imports_skipped =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-runtime-kv-weak-import-skip-device-profiles" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_kv_weak_import_skip_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--benchmark-min-runtime-kv-budget-import-skip-cases" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_kv_budget_import_skip_cases =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-budget-limited-runtime-kv-imports-skipped" if index + 1 < raw.len() => {
            *parser.benchmark_min_budget_limited_runtime_kv_imports_skipped =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-runtime-kv-budget-import-skip-device-profiles"
            if index + 1 < raw.len() =>
        {
            *parser.benchmark_min_runtime_kv_budget_import_skip_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--benchmark-min-runtime-kv-budget-pressure-cases" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_kv_budget_pressure_cases =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-runtime-kv-budget-pressure-device-profiles" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_kv_budget_pressure_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--benchmark-min-runtime-kv-segment-cases" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_kv_segment_cases = Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-runtime-kv-segments-included" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_kv_segments_included =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-max-runtime-kv-segments-rejected" if index + 1 < raw.len() => {
            *parser.benchmark_max_runtime_kv_segments_rejected =
                Some(parse_usize(&raw[index + 1], usize::MAX));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-runtime-kv-segment-device-profiles" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_kv_segment_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--benchmark-min-runtime-kv-imported" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_kv_imported = Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-runtime-kv-import-device-profiles" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_kv_import_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--benchmark-min-runtime-kv-exported" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_kv_exported = Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-runtime-kv-export-device-profiles" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_kv_export_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--benchmark-min-runtime-kv-stored" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_kv_stored = Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-runtime-kv-stored-device-profiles" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_kv_stored_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--benchmark-min-runtime-kv-hold-cases" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_kv_hold_cases = Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-runtime-kv-held" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_kv_held = Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-runtime-kv-hold-device-profiles" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_kv_hold_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--benchmark-min-runtime-adapter-contract-cases" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_adapter_contract_cases =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-runtime-adapter-kinds" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_adapter_kinds = Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-runtime-adapter-cache-modes" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_adapter_cache_modes =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-runtime-adapter-stream-trace-cases" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_adapter_stream_trace_cases =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-runtime-adapter-stream-gate-summary-cases" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_adapter_stream_gate_summary_cases =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-runtime-adapter-observations" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_adapter_observations =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-runtime-adapter-current-signals" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_adapter_current_signals =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-runtime-adapter-best-score" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_adapter_best_score =
                Some(parse_f32(&raw[index + 1], 0.0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-max-runtime-adapter-contract-violations" if index + 1 < raw.len() => {
            *parser.benchmark_max_runtime_adapter_contract_violations =
                Some(parse_usize(&raw[index + 1], usize::MAX));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-max-runtime-adapter-selection-mismatches" if index + 1 < raw.len() => {
            *parser.benchmark_max_runtime_adapter_selection_mismatches =
                Some(parse_usize(&raw[index + 1], usize::MAX));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-runtime-embedding-cases" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_embedding_cases = Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-runtime-embedding-device-profiles" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_embedding_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--benchmark-max-embedding-fallback-cases" if index + 1 < raw.len() => {
            *parser.benchmark_max_embedding_fallback_cases =
                Some(parse_usize(&raw[index + 1], usize::MAX));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-max-embedding-evidence-failures" if index + 1 < raw.len() => {
            *parser.benchmark_max_embedding_evidence_failures =
                Some(parse_usize(&raw[index + 1], usize::MAX));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-runtime-device-execution-cases" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_device_execution_cases =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-runtime-device-execution-device-profiles" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_device_execution_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--benchmark-min-runtime-kv-precision-device-profiles" if index + 1 < raw.len() => {
            *parser.benchmark_min_runtime_kv_precision_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--benchmark-max-runtime-device-execution-violations" if index + 1 < raw.len() => {
            *parser.benchmark_max_runtime_device_execution_violations =
                Some(parse_usize(&raw[index + 1], usize::MAX));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-max-memory-governance-failures" if index + 1 < raw.len() => {
            *parser.benchmark_max_memory_governance_failures =
                Some(parse_usize(&raw[index + 1], usize::MAX));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-memory-governance-cases" if index + 1 < raw.len() => {
            *parser.benchmark_min_memory_governance_cases = Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-memory-governance-device-profiles" if index + 1 < raw.len() => {
            *parser.benchmark_min_memory_governance_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--benchmark-min-memory-retention-activity-cases" if index + 1 < raw.len() => {
            *parser.benchmark_min_memory_retention_activity_cases =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-memory-compaction-activity-cases" if index + 1 < raw.len() => {
            *parser.benchmark_min_memory_compaction_activity_cases =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        _ => None,
    }
}
