mod auto_replay;
mod evidence;
mod evolution;
mod ledger_merge;
mod line;
mod memory;
mod overview;
mod record;
mod recursive;
mod runtime;

use crate::adaptive_state::EvolutionLedger;
use crate::drift::DriftSeverity;
use crate::hardware::DeviceClass;
use crate::hierarchy::TaskProfile;

use super::{
    BenchmarkEmbeddingEvidence, BenchmarkGenomeEvidence, BenchmarkLiveEvolutionEvidence,
    BenchmarkMemoryGovernanceEvidence, BenchmarkReflectionEvidence,
    BenchmarkRuntimeArchitectureEvidence, BenchmarkRuntimeDeviceExecutionEvidence,
};

#[derive(Debug, Clone)]
pub struct BenchmarkCaseResult {
    pub name: String,
    pub profile: TaskProfile,
    pub device: DeviceClass,
    pub elapsed_ms: u128,
    pub quality: f32,
    pub process_reward: f32,
    pub attention_fraction: f32,
    pub requires_recursion: bool,
    pub recursive_chunks: usize,
    pub recursive_waves: usize,
    pub recursive_runtime_calls: usize,
    pub auto_replay_applied: usize,
    pub auto_replay_router_updates: usize,
    pub auto_replay_hierarchy_updates: usize,
    pub auto_replay_router_threshold_mutations: usize,
    pub auto_replay_hierarchy_weight_mutations: usize,
    pub auto_replay_router_threshold_delta: f32,
    pub auto_replay_hierarchy_weight_delta: f32,
    pub auto_replay_memory_reinforcements: usize,
    pub auto_replay_memory_penalties: usize,
    pub auto_replay_live_memory_feedback_items: usize,
    pub auto_replay_live_memory_feedback_updates: usize,
    pub auto_replay_live_memory_feedback_reinforcements: usize,
    pub auto_replay_live_memory_feedback_penalties: usize,
    pub auto_replay_live_memory_feedback_detail_items: usize,
    pub auto_replay_live_memory_feedback_applied: usize,
    pub auto_replay_live_memory_feedback_removed: usize,
    pub auto_replay_live_memory_feedback_missing: usize,
    pub auto_replay_live_memory_feedback_strength_delta: f32,
    pub auto_replay_recursive_runtime_items: usize,
    pub auto_replay_recursive_runtime_calls: usize,
    pub auto_replay_avg_recursive_call_pressure: f32,
    pub auto_replay_max_recursive_call_pressure: f32,
    pub used_memories: usize,
    pub infini_local_window: usize,
    pub infini_global_memory: usize,
    pub sparse_skipped: usize,
    pub sparse_skipped_tokens: usize,
    pub stored_memories: usize,
    pub compacted_memories: usize,
    pub runtime_forward_signal: bool,
    pub runtime_forward_energy_signal: bool,
    pub runtime_kv_influence_signal: bool,
    pub runtime_global_layers: usize,
    pub runtime_local_window_layers: usize,
    pub runtime_convolutional_fusion_layers: usize,
    pub runtime_layer_mode_signal: bool,
    pub runtime_all_layer_modes_signal: bool,
    pub runtime_token_count: usize,
    pub runtime_uncertainty_token_count: usize,
    pub runtime_uncertainty_signal: bool,
    pub runtime_kv_imported: usize,
    pub runtime_kv_exported: usize,
    pub runtime_kv_stored: usize,
    pub runtime_selected_adapter: Option<String>,
    pub runtime_adapter_contract_ok: bool,
    pub runtime_adapter_contract_violations: usize,
    pub runtime_adapter_observations: usize,
    pub runtime_adapter_best_score: Option<f32>,
    pub runtime_adapter_best_adapter: Option<String>,
    pub runtime_adapter_selection_mismatches: usize,
    pub query_embedding_source: String,
    pub query_embedding_dimensions: usize,
    pub runtime_embedding_calls: usize,
    pub fallback_embedding_calls: usize,
    pub embedding_fallback_used: bool,
    pub drift_severity: DriftSeverity,
}

#[derive(Debug, Clone, Default)]
pub struct BenchmarkSummary {
    pub(super) results: Vec<BenchmarkCaseResult>,
    pub(super) evolution_ledger: EvolutionLedger,
    pub(super) reflection_evidence: BenchmarkReflectionEvidence,
    pub(super) live_evolution_evidence: BenchmarkLiveEvolutionEvidence,
    pub(super) genome_evidence: BenchmarkGenomeEvidence,
    pub(super) memory_governance_evidence: BenchmarkMemoryGovernanceEvidence,
    pub(super) embedding_evidence: BenchmarkEmbeddingEvidence,
    pub(super) runtime_architecture_evidence: BenchmarkRuntimeArchitectureEvidence,
    pub(super) runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence,
}
