#![allow(clippy::field_reassign_with_default)]

use super::*;
#[path = "tests/auto_replay.rs"]
mod auto_replay;
#[path = "tests/core.rs"]
mod core;
#[path = "tests/coverage.rs"]
mod coverage;
#[path = "tests/embedding.rs"]
mod embedding;
#[path = "tests/evolution_ledger.rs"]
mod evolution_ledger;
#[path = "tests/live_evolution.rs"]
mod live_evolution;
#[path = "tests/live_memory_feedback.rs"]
mod live_memory_feedback;
#[path = "tests/memory_governance.rs"]
mod memory_governance;
#[path = "tests/recursive_reflection.rs"]
mod recursive_reflection;
#[path = "tests/roundtrip_quant.rs"]
mod roundtrip_quant;
#[path = "tests/runtime_adapter.rs"]
mod runtime_adapter;
#[path = "tests/runtime_device.rs"]
mod runtime_device;
#[path = "tests/runtime_forward.rs"]
mod runtime_forward;
#[path = "tests/runtime_kv.rs"]
mod runtime_kv;

fn baseline_benchmark_result(
    name: impl Into<String>,
    profile: TaskProfile,
    device: DeviceClass,
) -> BenchmarkCaseResult {
    BenchmarkCaseResult {
        name: name.into(),
        profile,
        device,
        elapsed_ms: 1,
        quality: 0.9,
        process_reward: 0.9,
        attention_fraction: 0.5,
        requires_recursion: false,
        recursive_chunks: 1,
        recursive_waves: 1,
        recursive_runtime_calls: 1,
        auto_replay_applied: 0,
        auto_replay_router_updates: 0,
        auto_replay_hierarchy_updates: 0,
        auto_replay_router_threshold_mutations: 0,
        auto_replay_hierarchy_weight_mutations: 0,
        auto_replay_router_threshold_delta: 0.0,
        auto_replay_hierarchy_weight_delta: 0.0,
        auto_replay_memory_reinforcements: 0,
        auto_replay_memory_penalties: 0,
        auto_replay_live_memory_feedback_items: 0,
        auto_replay_live_memory_feedback_updates: 0,
        auto_replay_live_memory_feedback_reinforcements: 0,
        auto_replay_live_memory_feedback_penalties: 0,
        auto_replay_live_memory_feedback_detail_items: 0,
        auto_replay_live_memory_feedback_applied: 0,
        auto_replay_live_memory_feedback_removed: 0,
        auto_replay_live_memory_feedback_missing: 0,
        auto_replay_live_memory_feedback_strength_delta: 0.0,
        auto_replay_recursive_runtime_items: 0,
        auto_replay_recursive_runtime_calls: 0,
        auto_replay_avg_recursive_call_pressure: 0.0,
        auto_replay_max_recursive_call_pressure: 0.0,
        used_memories: 0,
        infini_local_window: 0,
        infini_global_memory: 0,
        sparse_skipped: 0,
        sparse_skipped_tokens: 0,
        stored_memories: 0,
        compacted_memories: 0,
        runtime_forward_signal: false,
        runtime_forward_energy_signal: false,
        runtime_kv_influence_signal: false,
        runtime_global_layers: 0,
        runtime_local_window_layers: 0,
        runtime_convolutional_fusion_layers: 0,
        runtime_layer_mode_signal: false,
        runtime_all_layer_modes_signal: false,
        runtime_token_count: 0,
        runtime_uncertainty_token_count: 0,
        runtime_uncertainty_signal: false,
        runtime_kv_imported: 0,
        runtime_kv_exported: 0,
        runtime_kv_stored: 0,
        runtime_selected_adapter: None,
        runtime_adapter_contract_ok: false,
        runtime_adapter_contract_violations: 0,
        runtime_adapter_observations: 0,
        runtime_adapter_best_score: None,
        runtime_adapter_best_adapter: None,
        runtime_adapter_selection_mismatches: 0,
        query_embedding_source: "fallback".to_owned(),
        query_embedding_dimensions: 64,
        runtime_embedding_calls: 0,
        fallback_embedding_calls: 1,
        embedding_fallback_used: true,
        drift_severity: DriftSeverity::Stable,
    }
}

fn runtime_uncertainty_result(device: DeviceClass) -> BenchmarkCaseResult {
    BenchmarkCaseResult {
        name: "runtime_uncertainty".to_owned(),
        profile: TaskProfile::Coding,
        device,
        elapsed_ms: 1,
        quality: 0.9,
        process_reward: 0.9,
        attention_fraction: 0.5,
        requires_recursion: false,
        recursive_chunks: 1,
        recursive_waves: 1,
        recursive_runtime_calls: 1,
        auto_replay_applied: 0,
        auto_replay_router_updates: 0,
        auto_replay_hierarchy_updates: 0,
        auto_replay_router_threshold_mutations: 0,
        auto_replay_hierarchy_weight_mutations: 0,
        auto_replay_router_threshold_delta: 0.0,
        auto_replay_hierarchy_weight_delta: 0.0,
        auto_replay_memory_reinforcements: 0,
        auto_replay_memory_penalties: 0,
        auto_replay_live_memory_feedback_items: 0,
        auto_replay_live_memory_feedback_updates: 0,
        auto_replay_live_memory_feedback_reinforcements: 0,
        auto_replay_live_memory_feedback_penalties: 0,
        auto_replay_live_memory_feedback_detail_items: 0,
        auto_replay_live_memory_feedback_applied: 0,
        auto_replay_live_memory_feedback_removed: 0,
        auto_replay_live_memory_feedback_missing: 0,
        auto_replay_live_memory_feedback_strength_delta: 0.0,
        auto_replay_recursive_runtime_items: 0,
        auto_replay_recursive_runtime_calls: 0,
        auto_replay_avg_recursive_call_pressure: 0.0,
        auto_replay_max_recursive_call_pressure: 0.0,
        used_memories: 0,
        infini_local_window: 0,
        infini_global_memory: 0,
        sparse_skipped: 0,
        sparse_skipped_tokens: 0,
        stored_memories: 0,
        compacted_memories: 0,
        runtime_forward_signal: true,
        runtime_forward_energy_signal: false,
        runtime_kv_influence_signal: false,
        runtime_global_layers: 0,
        runtime_local_window_layers: 0,
        runtime_convolutional_fusion_layers: 0,
        runtime_layer_mode_signal: false,
        runtime_all_layer_modes_signal: false,
        runtime_token_count: 4,
        runtime_uncertainty_token_count: 4,
        runtime_uncertainty_signal: true,
        runtime_kv_imported: 1,
        runtime_kv_exported: 1,
        runtime_kv_stored: 1,
        runtime_selected_adapter: Some("portable-rust".to_owned()),
        runtime_adapter_contract_ok: true,
        runtime_adapter_contract_violations: 0,
        runtime_adapter_observations: 0,
        runtime_adapter_best_score: None,
        runtime_adapter_best_adapter: None,
        runtime_adapter_selection_mismatches: 0,
        query_embedding_source: "runtime".to_owned(),
        query_embedding_dimensions: 64,
        runtime_embedding_calls: 1,
        fallback_embedding_calls: 0,
        embedding_fallback_used: false,
        drift_severity: DriftSeverity::Stable,
    }
}
