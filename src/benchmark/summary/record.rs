use crate::engine::InferenceOutcome;

use super::super::BenchmarkCase;
use super::super::runtime_evidence::runtime_static_architecture_only;
use super::ledger_merge::max_evolution_ledger;
use super::{BenchmarkCaseResult, BenchmarkSummary};

impl BenchmarkSummary {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, case: &BenchmarkCase, elapsed_ms: u128, outcome: &InferenceOutcome) {
        let stored_memories = usize::from(outcome.stored_memory_id.is_some())
            + outcome.stored_gist_memory_ids.len()
            + outcome.stored_runtime_kv_memory_ids.len();
        let auto_replay = outcome.auto_replay_report.as_ref();
        let infini_counts = outcome.infini_memory_plan.counts();
        let selected_adapter = outcome
            .runtime_diagnostics
            .selected_adapter
            .as_deref()
            .filter(|adapter| !adapter.is_empty());
        let runtime_adapter_best_adapter = outcome
            .runtime_adapter_observations
            .first()
            .map(|observation| observation.adapter.as_str())
            .filter(|adapter| !adapter.is_empty());
        let runtime_adapter_selection_mismatches =
            usize::from(match (runtime_adapter_best_adapter, selected_adapter) {
                (Some(best), Some(selected)) => best != selected,
                _ => false,
            });
        let runtime_adapter_contract_ok = selected_adapter
            .map(|adapter| {
                outcome
                    .hardware_plan
                    .execution
                    .adapter_hints
                    .iter()
                    .any(|hint| hint.as_str() == adapter)
            })
            .unwrap_or(false);
        let runtime_has_forward_signal = outcome.runtime_diagnostics.has_forward_signal();
        let runtime_has_static_architecture_only =
            runtime_static_architecture_only(&outcome.runtime_diagnostics);
        let runtime_uncertainty_token_count = outcome
            .runtime_token_metrics
            .entropy_count
            .max(outcome.runtime_token_metrics.logprob_count);

        self.results.push(BenchmarkCaseResult {
            name: case.name.clone(),
            profile: case.profile,
            device: outcome.hardware_plan.device,
            elapsed_ms,
            quality: outcome.report.quality,
            process_reward: outcome.process_reward.total,
            attention_fraction: outcome.route_budget.attention_fraction,
            requires_recursion: outcome.recursive_schedule.requires_recursion,
            recursive_chunks: outcome.recursive_schedule.chunk_count(),
            recursive_waves: outcome.recursive_schedule.execution_wave_count(),
            recursive_runtime_calls: outcome.recursive_runtime_calls,
            auto_replay_applied: auto_replay.map(|report| report.applied).unwrap_or(0),
            auto_replay_router_updates: auto_replay
                .map(|report| report.router_updates)
                .unwrap_or(0),
            auto_replay_hierarchy_updates: auto_replay
                .map(|report| report.hierarchy_updates)
                .unwrap_or(0),
            auto_replay_router_threshold_mutations: auto_replay
                .map(|report| report.router_threshold_mutations)
                .unwrap_or(0),
            auto_replay_hierarchy_weight_mutations: auto_replay
                .map(|report| report.hierarchy_weight_mutations)
                .unwrap_or(0),
            auto_replay_router_threshold_delta: auto_replay
                .map(|report| report.router_threshold_delta)
                .unwrap_or(0.0),
            auto_replay_hierarchy_weight_delta: auto_replay
                .map(|report| report.hierarchy_weight_delta)
                .unwrap_or(0.0),
            auto_replay_memory_reinforcements: auto_replay
                .map(|report| report.memory_reinforcements)
                .unwrap_or(0),
            auto_replay_memory_penalties: auto_replay
                .map(|report| report.memory_penalties)
                .unwrap_or(0),
            auto_replay_live_memory_feedback_items: auto_replay
                .map(|report| report.live_memory_feedback_items)
                .unwrap_or(0),
            auto_replay_live_memory_feedback_updates: auto_replay
                .map(|report| report.live_memory_feedback_updates)
                .unwrap_or(0),
            auto_replay_live_memory_feedback_reinforcements: auto_replay
                .map(|report| report.live_memory_feedback_reinforcements)
                .unwrap_or(0),
            auto_replay_live_memory_feedback_penalties: auto_replay
                .map(|report| report.live_memory_feedback_penalties)
                .unwrap_or(0),
            auto_replay_live_memory_feedback_detail_items: auto_replay
                .map(|report| report.live_memory_feedback_detail_items)
                .unwrap_or(0),
            auto_replay_live_memory_feedback_applied: auto_replay
                .map(|report| report.live_memory_feedback_applied)
                .unwrap_or(0),
            auto_replay_live_memory_feedback_removed: auto_replay
                .map(|report| report.live_memory_feedback_removed)
                .unwrap_or(0),
            auto_replay_live_memory_feedback_missing: auto_replay
                .map(|report| report.live_memory_feedback_missing)
                .unwrap_or(0),
            auto_replay_live_memory_feedback_strength_delta: auto_replay
                .map(|report| report.live_memory_feedback_strength_delta)
                .unwrap_or(0.0),
            auto_replay_recursive_runtime_items: auto_replay
                .map(|report| report.recursive_runtime_items)
                .unwrap_or(0),
            auto_replay_recursive_runtime_calls: auto_replay
                .map(|report| report.recursive_runtime_calls)
                .unwrap_or(0),
            auto_replay_avg_recursive_call_pressure: auto_replay
                .map(|report| report.average_recursive_call_pressure)
                .unwrap_or(0.0),
            auto_replay_max_recursive_call_pressure: auto_replay
                .map(|report| report.max_recursive_call_pressure)
                .unwrap_or(0.0),
            used_memories: outcome.used_memories.len(),
            infini_local_window: infini_counts.local_window,
            infini_global_memory: infini_counts.global_memory,
            sparse_skipped: infini_counts.skipped,
            sparse_skipped_tokens: infini_counts.skipped_tokens,
            stored_memories,
            compacted_memories: outcome.memory_compaction_report.merged.len(),
            runtime_forward_signal: runtime_has_forward_signal,
            runtime_forward_energy_signal: outcome.runtime_diagnostics.forward_energy.is_some(),
            runtime_kv_influence_signal: outcome.runtime_diagnostics.kv_influence.is_some(),
            runtime_global_layers: outcome.runtime_diagnostics.global_layers,
            runtime_local_window_layers: outcome.runtime_diagnostics.local_window_layers,
            runtime_convolutional_fusion_layers: outcome
                .runtime_diagnostics
                .convolutional_fusion_layers,
            runtime_layer_mode_signal: outcome.runtime_diagnostics.has_layer_mode_signal(),
            runtime_all_layer_modes_signal: outcome.runtime_diagnostics.has_all_layer_modes(),
            runtime_token_count: outcome.runtime_token_metrics.token_count,
            runtime_uncertainty_token_count,
            runtime_uncertainty_signal: outcome.runtime_token_metrics.has_uncertainty_signal(),
            runtime_kv_imported: outcome.runtime_diagnostics.imported_kv_blocks,
            runtime_kv_exported: outcome.exported_runtime_kv_blocks,
            runtime_kv_stored: outcome.stored_runtime_kv_memory_ids.len(),
            runtime_selected_adapter: selected_adapter.map(str::to_owned),
            runtime_adapter_contract_ok,
            runtime_adapter_contract_violations: usize::from(
                !runtime_has_static_architecture_only
                    && !runtime_adapter_contract_ok
                    && (runtime_has_forward_signal || selected_adapter.is_some()),
            ),
            runtime_adapter_observations: outcome.runtime_adapter_observations.len(),
            runtime_adapter_best_score: outcome
                .runtime_adapter_observations
                .first()
                .map(|observation| observation.score),
            runtime_adapter_best_adapter: runtime_adapter_best_adapter.map(str::to_owned),
            runtime_adapter_selection_mismatches,
            query_embedding_source: outcome
                .embedding_diagnostics
                .query
                .source
                .as_str()
                .to_owned(),
            query_embedding_dimensions: outcome.embedding_diagnostics.query.dimensions,
            runtime_embedding_calls: outcome.embedding_diagnostics.runtime_calls,
            fallback_embedding_calls: outcome.embedding_diagnostics.fallback_calls,
            embedding_fallback_used: outcome.embedding_diagnostics.fallback_embedding_used(),
            drift_severity: outcome.drift_report.severity,
        });
        self.evolution_ledger =
            max_evolution_ledger(self.evolution_ledger, outcome.evolution_ledger);
        self.reflection_evidence.record(outcome);
        self.live_evolution_evidence.record(outcome);
        self.routing_evidence.record(case, outcome);
        self.genome_evidence.record(case, outcome);
        self.memory_governance_evidence.record(case, outcome);
        self.embedding_evidence.record(case, outcome);
        self.runtime_architecture_evidence.record(outcome);
        self.runtime_device_execution_evidence.record(case, outcome);
    }
}
