use crate::adaptive_state::EvolutionLedger;
use crate::drift::DriftSeverity;
use crate::engine::InferenceOutcome;
use crate::hardware::DeviceClass;
use crate::hierarchy::TaskProfile;
use crate::kv_quant::{QuantizationBits, QuantizedVector};
use std::time::Instant;

const BENCHMARK_FLOAT_EPSILON: f32 = 0.000_001;

#[derive(Debug, Clone)]
pub struct BenchmarkCase {
    pub name: String,
    pub profile: TaskProfile,
    pub prompt: String,
}

impl BenchmarkCase {
    pub fn new(name: impl Into<String>, profile: TaskProfile, prompt: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            profile,
            prompt: prompt.into(),
        }
    }
}

pub fn default_benchmark_cases() -> Vec<BenchmarkCase> {
    vec![
        BenchmarkCase::new(
            "coding_router",
            TaskProfile::Coding,
            "Design a Rust trait boundary for a self-developed Transformer runtime with KV import and export.",
        ),
        BenchmarkCase::new(
            "long_context_scheduler",
            TaskProfile::LongDocument,
            long_context_benchmark_prompt(),
        ),
        BenchmarkCase::new(
            "reflection_memory",
            TaskProfile::General,
            "Explain how a local model should decide whether a generated answer deserves to become reusable memory.",
        ),
        BenchmarkCase::new(
            "creative_consistency",
            TaskProfile::Writing,
            "Write a compact scene outline that keeps character motivation consistent across several chapters.",
        ),
    ]
}

fn long_context_benchmark_prompt() -> String {
    let repeated_sections = (0..96)
        .map(|index| {
            format!(
                "section_{index}: FHT-DKE keeps local KV memory on disk, Noiron reflection scores drafts, recursive scheduling merges chunks, and adaptive routing avoids wasted attention."
            )
        })
        .collect::<Vec<_>>()
        .join(" ");

    format!(
        "Summarize this local technical document and identify the control decisions that reduce wasted compute. {repeated_sections}"
    )
}

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

#[derive(Debug, Clone, Copy)]
pub struct BenchmarkGate {
    pub min_average_quality: f32,
    pub min_average_reward: f32,
    pub max_total_elapsed_ms: Option<u128>,
    pub max_case_recursive_chunks: Option<usize>,
    pub min_recursive_cases: Option<usize>,
    pub min_recursive_runtime_calls: Option<usize>,
    pub min_auto_replay_router_updates: Option<usize>,
    pub min_auto_replay_hierarchy_updates: Option<usize>,
    pub min_auto_replay_router_threshold_mutations: Option<usize>,
    pub min_auto_replay_hierarchy_weight_mutations: Option<usize>,
    pub min_auto_replay_router_threshold_delta: Option<f32>,
    pub min_auto_replay_hierarchy_weight_delta: Option<f32>,
    pub min_auto_replay_memory_updates: Option<usize>,
    pub min_live_memory_feedback_updates: Option<usize>,
    pub min_auto_replay_live_memory_feedback_updates: Option<usize>,
    pub min_auto_replay_live_memory_feedback_detail_items: Option<usize>,
    pub min_auto_replay_live_memory_feedback_applied: Option<usize>,
    pub min_auto_replay_live_memory_feedback_strength_delta: Option<f32>,
    pub min_auto_replay_recursive_items: Option<usize>,
    pub min_auto_replay_recursive_call_pressure: Option<f32>,
    pub max_auto_replay_recursive_call_pressure: Option<f32>,
    pub min_evolution_live_inference_runs: Option<u64>,
    pub min_evolution_live_router_threshold_mutations: Option<u64>,
    pub min_evolution_live_hierarchy_weight_mutations: Option<u64>,
    pub min_evolution_live_router_threshold_delta: Option<f32>,
    pub min_evolution_live_hierarchy_weight_delta: Option<f32>,
    pub min_evolution_live_online_reward_feedbacks: Option<u64>,
    pub min_evolution_live_online_reward_reinforcements: Option<u64>,
    pub min_evolution_live_online_reward_penalties: Option<u64>,
    pub min_evolution_live_online_reward_strength: Option<f32>,
    pub min_evolution_live_online_reward_reinforcement_strength: Option<f32>,
    pub min_evolution_live_online_reward_penalty_strength: Option<f32>,
    pub min_evolution_live_memory_updates: Option<u64>,
    pub min_evolution_live_stored_memory_updates: Option<u64>,
    pub min_evolution_live_reflection_issues: Option<u64>,
    pub min_evolution_live_critical_reflection_issues: Option<u64>,
    pub min_evolution_live_revision_actions: Option<u64>,
    pub min_evolution_live_inference_device_profiles: Option<usize>,
    pub min_evolution_live_router_threshold_mutation_device_profiles: Option<usize>,
    pub min_evolution_live_hierarchy_weight_mutation_device_profiles: Option<usize>,
    pub min_evolution_live_online_reward_device_profiles: Option<usize>,
    pub min_evolution_live_online_reward_strength_device_profiles: Option<usize>,
    pub min_evolution_live_memory_update_device_profiles: Option<usize>,
    pub min_evolution_live_stored_memory_update_device_profiles: Option<usize>,
    pub min_evolution_live_reflection_issue_device_profiles: Option<usize>,
    pub min_evolution_live_critical_reflection_issue_device_profiles: Option<usize>,
    pub min_evolution_live_revision_action_device_profiles: Option<usize>,
    pub min_evolution_replay_runs: Option<u64>,
    pub min_evolution_replay_items: Option<u64>,
    pub min_evolution_router_threshold_mutations: Option<u64>,
    pub min_evolution_hierarchy_weight_mutations: Option<u64>,
    pub min_evolution_router_threshold_delta: Option<f32>,
    pub min_evolution_hierarchy_weight_delta: Option<f32>,
    pub min_evolution_memory_updates: Option<u64>,
    pub min_evolution_replay_live_memory_feedback_updates: Option<u64>,
    pub min_evolution_replay_live_memory_feedback_detail_items: Option<u64>,
    pub min_evolution_replay_live_memory_feedback_applied: Option<u64>,
    pub min_evolution_replay_live_memory_feedback_strength_delta: Option<f32>,
    pub min_evolution_replay_live_evolution_items: Option<u64>,
    pub min_evolution_replay_live_evolution_online_reward_feedbacks: Option<u64>,
    pub min_evolution_replay_live_evolution_online_reward_reinforcements: Option<u64>,
    pub min_evolution_replay_live_evolution_online_reward_penalties: Option<u64>,
    pub min_evolution_replay_live_evolution_online_reward_strength: Option<f32>,
    pub min_evolution_replay_live_evolution_online_reward_reinforcement_strength: Option<f32>,
    pub min_evolution_replay_live_evolution_online_reward_penalty_strength: Option<f32>,
    pub min_evolution_replay_live_evolution_memory_updates: Option<u64>,
    pub min_evolution_replay_live_evolution_stored_memory_updates: Option<u64>,
    pub min_evolution_replay_live_evolution_reflection_issues: Option<u64>,
    pub min_evolution_replay_live_evolution_critical_reflection_issues: Option<u64>,
    pub min_evolution_replay_live_evolution_revision_actions: Option<u64>,
    pub min_evolution_replay_live_evolution_device_profiles: Option<usize>,
    pub min_evolution_replay_live_evolution_online_reward_device_profiles: Option<usize>,
    pub min_evolution_replay_live_evolution_online_reward_strength_device_profiles: Option<usize>,
    pub min_evolution_replay_live_evolution_memory_update_device_profiles: Option<usize>,
    pub min_evolution_replay_live_evolution_critical_reflection_issue_device_profiles:
        Option<usize>,
    pub min_evolution_replay_live_evolution_revision_action_device_profiles: Option<usize>,
    pub min_evolution_recursive_replay_items: Option<u64>,
    pub min_evolution_recursive_runtime_calls: Option<u64>,
    pub max_evolution_drift_rollbacks: Option<u64>,
    pub max_evolution_rollback_router_threshold_delta: Option<f32>,
    pub max_evolution_rollback_hierarchy_weight_delta: Option<f32>,
    pub min_sparse_skipped_cases: Option<usize>,
    pub min_sparse_skipped_tokens: Option<usize>,
    pub min_runtime_forward_cases: Option<usize>,
    pub min_runtime_forward_energy_cases: Option<usize>,
    pub min_runtime_kv_influence_cases: Option<usize>,
    pub min_runtime_kv_precision_cases: Option<usize>,
    pub min_runtime_layer_mode_cases: Option<usize>,
    pub min_runtime_all_layer_mode_cases: Option<usize>,
    pub min_runtime_global_layers: Option<usize>,
    pub min_runtime_local_window_layers: Option<usize>,
    pub min_runtime_convolutional_fusion_layers: Option<usize>,
    pub min_runtime_uncertainty_cases: Option<usize>,
    pub min_runtime_uncertainty_tokens: Option<usize>,
    pub min_runtime_uncertainty_device_profiles: Option<usize>,
    pub min_runtime_uncertainty_token_device_profiles: Option<usize>,
    pub min_runtime_kv_import_cases: Option<usize>,
    pub min_runtime_kv_imported: Option<usize>,
    pub min_runtime_kv_import_device_profiles: Option<usize>,
    pub min_runtime_kv_exported: Option<usize>,
    pub min_runtime_kv_export_device_profiles: Option<usize>,
    pub min_runtime_kv_stored: Option<usize>,
    pub min_runtime_kv_stored_device_profiles: Option<usize>,
    pub min_runtime_kv_hold_cases: Option<usize>,
    pub min_runtime_kv_held: Option<usize>,
    pub min_runtime_kv_hold_device_profiles: Option<usize>,
    pub min_runtime_adapter_contract_cases: Option<usize>,
    pub min_runtime_adapter_kinds: Option<usize>,
    pub min_runtime_adapter_observations: Option<usize>,
    pub min_runtime_adapter_best_score: Option<f32>,
    pub max_runtime_adapter_contract_violations: Option<usize>,
    pub max_runtime_adapter_selection_mismatches: Option<usize>,
    pub min_runtime_embedding_cases: Option<usize>,
    pub min_runtime_embedding_device_profiles: Option<usize>,
    pub max_embedding_fallback_cases: Option<usize>,
    pub max_embedding_evidence_failures: Option<usize>,
    pub min_runtime_device_execution_cases: Option<usize>,
    pub min_runtime_device_execution_device_profiles: Option<usize>,
    pub min_runtime_kv_precision_device_profiles: Option<usize>,
    pub max_runtime_device_execution_violations: Option<usize>,
    pub max_memory_governance_failures: Option<usize>,
    pub max_memory_feedback_evidence_failures: Option<usize>,
    pub min_memory_governance_cases: Option<usize>,
    pub min_memory_governance_device_profiles: Option<usize>,
    pub min_memory_retention_activity_cases: Option<usize>,
    pub min_memory_compaction_activity_cases: Option<usize>,
    pub min_reflection_issue_cases: Option<usize>,
    pub min_reflection_issues: Option<usize>,
    pub min_critical_reflection_issue_cases: Option<usize>,
    pub min_critical_reflection_issues: Option<usize>,
    pub min_revision_action_cases: Option<usize>,
    pub min_revision_actions: Option<usize>,
    pub min_reflection_issue_device_profiles: Option<usize>,
    pub min_critical_reflection_issue_device_profiles: Option<usize>,
    pub min_revision_action_device_profiles: Option<usize>,
    pub min_device_profiles: Option<usize>,
    pub min_recursive_device_profiles: Option<usize>,
    pub max_drift_blocks: Option<usize>,
    pub max_drift_rollbacks: Option<usize>,
}

impl Default for BenchmarkGate {
    fn default() -> Self {
        Self {
            min_average_quality: 0.50,
            min_average_reward: 0.45,
            max_total_elapsed_ms: None,
            max_case_recursive_chunks: None,
            min_recursive_cases: None,
            min_recursive_runtime_calls: None,
            min_auto_replay_router_updates: None,
            min_auto_replay_hierarchy_updates: None,
            min_auto_replay_router_threshold_mutations: None,
            min_auto_replay_hierarchy_weight_mutations: None,
            min_auto_replay_router_threshold_delta: None,
            min_auto_replay_hierarchy_weight_delta: None,
            min_auto_replay_memory_updates: None,
            min_live_memory_feedback_updates: None,
            min_auto_replay_live_memory_feedback_updates: None,
            min_auto_replay_live_memory_feedback_detail_items: None,
            min_auto_replay_live_memory_feedback_applied: None,
            min_auto_replay_live_memory_feedback_strength_delta: None,
            min_auto_replay_recursive_items: None,
            min_auto_replay_recursive_call_pressure: None,
            max_auto_replay_recursive_call_pressure: None,
            min_evolution_live_inference_runs: None,
            min_evolution_live_router_threshold_mutations: None,
            min_evolution_live_hierarchy_weight_mutations: None,
            min_evolution_live_router_threshold_delta: None,
            min_evolution_live_hierarchy_weight_delta: None,
            min_evolution_live_online_reward_feedbacks: None,
            min_evolution_live_online_reward_reinforcements: None,
            min_evolution_live_online_reward_penalties: None,
            min_evolution_live_online_reward_strength: None,
            min_evolution_live_online_reward_reinforcement_strength: None,
            min_evolution_live_online_reward_penalty_strength: None,
            min_evolution_live_memory_updates: None,
            min_evolution_live_stored_memory_updates: None,
            min_evolution_live_reflection_issues: None,
            min_evolution_live_critical_reflection_issues: None,
            min_evolution_live_revision_actions: None,
            min_evolution_live_inference_device_profiles: None,
            min_evolution_live_router_threshold_mutation_device_profiles: None,
            min_evolution_live_hierarchy_weight_mutation_device_profiles: None,
            min_evolution_live_online_reward_device_profiles: None,
            min_evolution_live_online_reward_strength_device_profiles: None,
            min_evolution_live_memory_update_device_profiles: None,
            min_evolution_live_stored_memory_update_device_profiles: None,
            min_evolution_live_reflection_issue_device_profiles: None,
            min_evolution_live_critical_reflection_issue_device_profiles: None,
            min_evolution_live_revision_action_device_profiles: None,
            min_evolution_replay_runs: None,
            min_evolution_replay_items: None,
            min_evolution_router_threshold_mutations: None,
            min_evolution_hierarchy_weight_mutations: None,
            min_evolution_router_threshold_delta: None,
            min_evolution_hierarchy_weight_delta: None,
            min_evolution_memory_updates: None,
            min_evolution_replay_live_memory_feedback_updates: None,
            min_evolution_replay_live_memory_feedback_detail_items: None,
            min_evolution_replay_live_memory_feedback_applied: None,
            min_evolution_replay_live_memory_feedback_strength_delta: None,
            min_evolution_replay_live_evolution_items: None,
            min_evolution_replay_live_evolution_online_reward_feedbacks: None,
            min_evolution_replay_live_evolution_online_reward_reinforcements: None,
            min_evolution_replay_live_evolution_online_reward_penalties: None,
            min_evolution_replay_live_evolution_online_reward_strength: None,
            min_evolution_replay_live_evolution_online_reward_reinforcement_strength: None,
            min_evolution_replay_live_evolution_online_reward_penalty_strength: None,
            min_evolution_replay_live_evolution_memory_updates: None,
            min_evolution_replay_live_evolution_stored_memory_updates: None,
            min_evolution_replay_live_evolution_reflection_issues: None,
            min_evolution_replay_live_evolution_critical_reflection_issues: None,
            min_evolution_replay_live_evolution_revision_actions: None,
            min_evolution_replay_live_evolution_device_profiles: None,
            min_evolution_replay_live_evolution_online_reward_device_profiles: None,
            min_evolution_replay_live_evolution_online_reward_strength_device_profiles: None,
            min_evolution_replay_live_evolution_memory_update_device_profiles: None,
            min_evolution_replay_live_evolution_critical_reflection_issue_device_profiles: None,
            min_evolution_replay_live_evolution_revision_action_device_profiles: None,
            min_evolution_recursive_replay_items: None,
            min_evolution_recursive_runtime_calls: None,
            max_evolution_drift_rollbacks: Some(0),
            max_evolution_rollback_router_threshold_delta: Some(0.0),
            max_evolution_rollback_hierarchy_weight_delta: Some(0.0),
            min_sparse_skipped_cases: None,
            min_sparse_skipped_tokens: None,
            min_runtime_forward_cases: None,
            min_runtime_forward_energy_cases: None,
            min_runtime_kv_influence_cases: None,
            min_runtime_kv_precision_cases: None,
            min_runtime_layer_mode_cases: None,
            min_runtime_all_layer_mode_cases: None,
            min_runtime_global_layers: None,
            min_runtime_local_window_layers: None,
            min_runtime_convolutional_fusion_layers: None,
            min_runtime_uncertainty_cases: None,
            min_runtime_uncertainty_tokens: None,
            min_runtime_uncertainty_device_profiles: None,
            min_runtime_uncertainty_token_device_profiles: None,
            min_runtime_kv_import_cases: None,
            min_runtime_kv_imported: None,
            min_runtime_kv_import_device_profiles: None,
            min_runtime_kv_exported: None,
            min_runtime_kv_export_device_profiles: None,
            min_runtime_kv_stored: None,
            min_runtime_kv_stored_device_profiles: None,
            min_runtime_kv_hold_cases: None,
            min_runtime_kv_held: None,
            min_runtime_kv_hold_device_profiles: None,
            min_runtime_adapter_contract_cases: None,
            min_runtime_adapter_kinds: None,
            min_runtime_adapter_observations: None,
            min_runtime_adapter_best_score: None,
            max_runtime_adapter_contract_violations: Some(0),
            max_runtime_adapter_selection_mismatches: None,
            min_runtime_embedding_cases: None,
            min_runtime_embedding_device_profiles: None,
            max_embedding_fallback_cases: None,
            max_embedding_evidence_failures: Some(0),
            min_runtime_device_execution_cases: None,
            min_runtime_device_execution_device_profiles: None,
            min_runtime_kv_precision_device_profiles: None,
            max_runtime_device_execution_violations: Some(0),
            max_memory_governance_failures: Some(0),
            max_memory_feedback_evidence_failures: Some(0),
            min_memory_governance_cases: None,
            min_memory_governance_device_profiles: None,
            min_memory_retention_activity_cases: None,
            min_memory_compaction_activity_cases: None,
            min_reflection_issue_cases: None,
            min_reflection_issues: None,
            min_critical_reflection_issue_cases: None,
            min_critical_reflection_issues: None,
            min_revision_action_cases: None,
            min_revision_actions: None,
            min_reflection_issue_device_profiles: None,
            min_critical_reflection_issue_device_profiles: None,
            min_revision_action_device_profiles: None,
            min_device_profiles: None,
            min_recursive_device_profiles: None,
            max_drift_blocks: Some(0),
            max_drift_rollbacks: Some(0),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkGateReport {
    pub passed: bool,
    pub failures: Vec<String>,
}

impl BenchmarkGateReport {
    pub fn summary_line(&self) -> String {
        format!(
            "benchmark_gate: passed={} failures={}",
            self.passed,
            self.failures.len()
        )
    }
}

#[derive(Debug, Clone)]
pub struct KvQuantBenchmarkCaseResult {
    pub name: String,
    pub bits: QuantizationBits,
    pub len: usize,
    pub max_abs_error: f32,
    pub mean_abs_error: f32,
    pub compression_ratio: f32,
    pub elapsed_us: u128,
}

#[derive(Debug, Clone, Copy)]
pub struct KvQuantBenchmarkGate {
    pub max_four_bit_abs_error: f32,
    pub max_four_bit_mean_error: f32,
    pub max_four_bit_compression_ratio: f32,
    pub max_eight_bit_abs_error: f32,
    pub max_eight_bit_mean_error: f32,
    pub max_eight_bit_compression_ratio: f32,
    pub max_total_elapsed_us: Option<u128>,
}

impl Default for KvQuantBenchmarkGate {
    fn default() -> Self {
        Self {
            max_four_bit_abs_error: 0.080,
            max_four_bit_mean_error: 0.035,
            max_four_bit_compression_ratio: 0.140,
            max_eight_bit_abs_error: 0.006,
            max_eight_bit_mean_error: 0.003,
            max_eight_bit_compression_ratio: 0.260,
            max_total_elapsed_us: Some(2_000_000),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KvQuantBenchmarkGateReport {
    pub passed: bool,
    pub failures: Vec<String>,
}

impl KvQuantBenchmarkGateReport {
    pub fn summary_line(&self) -> String {
        format!(
            "kv_quant_gate: passed={} failures={}",
            self.passed,
            self.failures.len()
        )
    }
}

#[derive(Debug, Clone, Default)]
pub struct KvQuantBenchmarkSummary {
    results: Vec<KvQuantBenchmarkCaseResult>,
}

impl KvQuantBenchmarkSummary {
    pub fn run_default() -> Self {
        let mut summary = Self::default();

        for (name, vector) in kv_quant_benchmark_vectors() {
            summary.record(name, QuantizationBits::Four, &vector);
            summary.record(name, QuantizationBits::Eight, &vector);
        }

        summary
    }

    pub fn record(&mut self, name: impl Into<String>, bits: QuantizationBits, vector: &[f32]) {
        let started = Instant::now();
        let quantized = QuantizedVector::quantize(vector, bits);
        let decoded = quantized.dequantize();
        let elapsed_us = started.elapsed().as_micros();
        let (max_abs_error, mean_abs_error) = quantization_error(vector, &decoded);

        self.results.push(KvQuantBenchmarkCaseResult {
            name: name.into(),
            bits,
            len: vector.len(),
            max_abs_error,
            mean_abs_error,
            compression_ratio: quantized.compression_ratio(),
            elapsed_us,
        });
    }

    pub fn results(&self) -> &[KvQuantBenchmarkCaseResult] {
        &self.results
    }

    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }

    pub fn len(&self) -> usize {
        self.results.len()
    }

    pub fn total_elapsed_us(&self) -> u128 {
        self.results.iter().map(|result| result.elapsed_us).sum()
    }

    pub fn max_abs_error_for(&self, bits: QuantizationBits) -> f32 {
        self.results
            .iter()
            .filter(|result| result.bits == bits)
            .map(|result| result.max_abs_error)
            .fold(0.0, f32::max)
    }

    pub fn max_mean_error_for(&self, bits: QuantizationBits) -> f32 {
        self.results
            .iter()
            .filter(|result| result.bits == bits)
            .map(|result| result.mean_abs_error)
            .fold(0.0, f32::max)
    }

    pub fn max_compression_ratio_for(&self, bits: QuantizationBits) -> f32 {
        self.results
            .iter()
            .filter(|result| result.bits == bits)
            .map(|result| result.compression_ratio)
            .fold(0.0, f32::max)
    }

    pub fn evaluate(&self, gate: &KvQuantBenchmarkGate) -> KvQuantBenchmarkGateReport {
        let mut failures = Vec::new();

        if self.is_empty() {
            failures.push("no KV quantization benchmark cases were recorded".to_owned());
        }

        self.evaluate_bits(
            QuantizationBits::Four,
            gate.max_four_bit_abs_error,
            gate.max_four_bit_mean_error,
            gate.max_four_bit_compression_ratio,
            &mut failures,
        );
        self.evaluate_bits(
            QuantizationBits::Eight,
            gate.max_eight_bit_abs_error,
            gate.max_eight_bit_mean_error,
            gate.max_eight_bit_compression_ratio,
            &mut failures,
        );

        if let Some(max_total_elapsed_us) = gate.max_total_elapsed_us {
            let total_elapsed_us = self.total_elapsed_us();
            if total_elapsed_us > max_total_elapsed_us {
                failures.push(format!(
                    "total_elapsed_us {} above maximum {}",
                    total_elapsed_us, max_total_elapsed_us
                ));
            }
        }

        KvQuantBenchmarkGateReport {
            passed: failures.is_empty(),
            failures,
        }
    }

    pub fn summary_line(&self) -> String {
        format!(
            "kv_quant_benchmark: cases={} total_elapsed_us={} q4_max_error={:.6} q4_mean_error={:.6} q4_max_ratio={:.3} q8_max_error={:.6} q8_mean_error={:.6} q8_max_ratio={:.3}",
            self.len(),
            self.total_elapsed_us(),
            self.max_abs_error_for(QuantizationBits::Four),
            self.max_mean_error_for(QuantizationBits::Four),
            self.max_compression_ratio_for(QuantizationBits::Four),
            self.max_abs_error_for(QuantizationBits::Eight),
            self.max_mean_error_for(QuantizationBits::Eight),
            self.max_compression_ratio_for(QuantizationBits::Eight)
        )
    }

    fn evaluate_bits(
        &self,
        bits: QuantizationBits,
        max_abs_error: f32,
        max_mean_error: f32,
        max_compression_ratio: f32,
        failures: &mut Vec<String>,
    ) {
        let width = bits.width();
        let observed_abs_error = self.max_abs_error_for(bits);
        if observed_abs_error > max_abs_error {
            failures.push(format!(
                "q{width}_max_abs_error {:.6} above maximum {:.6}",
                observed_abs_error, max_abs_error
            ));
        }

        let observed_mean_error = self.max_mean_error_for(bits);
        if observed_mean_error > max_mean_error {
            failures.push(format!(
                "q{width}_mean_abs_error {:.6} above maximum {:.6}",
                observed_mean_error, max_mean_error
            ));
        }

        let observed_ratio = self.max_compression_ratio_for(bits);
        if observed_ratio > max_compression_ratio {
            failures.push(format!(
                "q{width}_compression_ratio {:.3} above maximum {:.3}",
                observed_ratio, max_compression_ratio
            ));
        }
    }
}

#[derive(Debug, Clone)]
pub struct PersistentRoundtripInput {
    pub first_stored_memory: bool,
    pub first_runtime_kv_stored: usize,
    pub first_runtime_kv_namespace_preserved: bool,
    pub second_used_memories: usize,
    pub second_used_runtime_kv_memory: bool,
    pub second_used_experiences: usize,
    pub second_imported_runtime_kv_blocks: usize,
    pub second_imported_runtime_kv_from_namespace: bool,
    pub second_runtime_adapter_observations: usize,
    pub second_runtime_adapter_best_score: Option<f32>,
    pub second_runtime_adapter_best_adapter: Option<String>,
    pub second_runtime_selected_adapter: Option<String>,
    pub second_quality: f32,
    pub first_drift_severity: DriftSeverity,
    pub second_drift_severity: DriftSeverity,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PersistentRoundtripReport {
    pub passed: bool,
    pub first_stored_memory: bool,
    pub first_runtime_kv_stored: usize,
    pub first_runtime_kv_namespace_preserved: bool,
    pub second_used_memories: usize,
    pub second_used_runtime_kv_memory: bool,
    pub second_used_experiences: usize,
    pub second_imported_runtime_kv_blocks: usize,
    pub second_imported_runtime_kv_from_namespace: bool,
    pub second_runtime_adapter_observations: usize,
    pub second_runtime_adapter_best_score: Option<f32>,
    pub second_runtime_adapter_best_adapter: Option<String>,
    pub second_runtime_selected_adapter: Option<String>,
    pub second_quality: f32,
    pub first_drift_severity: DriftSeverity,
    pub second_drift_severity: DriftSeverity,
    pub failures: Vec<String>,
}

impl PersistentRoundtripReport {
    pub fn evaluate(input: PersistentRoundtripInput) -> Self {
        let mut failures = Vec::new();

        if !input.first_stored_memory {
            failures.push("first run did not store durable memory".to_owned());
        }
        if input.first_runtime_kv_stored == 0 {
            failures.push("first run did not store runtime KV memory".to_owned());
        }
        if !input.first_runtime_kv_namespace_preserved {
            failures.push("first run stored runtime KV without runtime_kv namespace".to_owned());
        }
        if input.second_used_memories == 0 {
            failures.push("second run did not retrieve persisted memory".to_owned());
        }
        if !input.second_used_runtime_kv_memory {
            failures.push("second run did not retrieve persisted runtime KV memory".to_owned());
        }
        if input.second_used_experiences == 0 {
            failures.push("second run did not retrieve persisted experience".to_owned());
        }
        if input.second_imported_runtime_kv_blocks == 0 {
            failures.push("second run did not import persisted runtime KV".to_owned());
        }
        if !input.second_imported_runtime_kv_from_namespace {
            failures.push(
                "second run did not import KV reconstructed from persisted runtime_kv namespace"
                    .to_owned(),
            );
        }
        if input.second_runtime_adapter_observations == 0 {
            failures.push(
                "second run did not derive runtime adapter observations from persisted experience"
                    .to_owned(),
            );
        }
        if input
            .second_runtime_adapter_best_score
            .filter(|score| score.is_finite() && *score > 0.0)
            .is_none()
        {
            failures.push(
                "second run did not expose a positive runtime adapter observation score".to_owned(),
            );
        }
        match (
            input.second_runtime_adapter_best_adapter.as_deref(),
            input.second_runtime_selected_adapter.as_deref(),
        ) {
            (Some(best_adapter), Some(selected_adapter)) if best_adapter == selected_adapter => {}
            (None, _) => failures.push(
                "second run did not expose a best runtime adapter observation".to_owned(),
            ),
            (_, None) => failures.push("second run did not select a runtime adapter".to_owned()),
            (Some(best_adapter), Some(selected_adapter)) => failures.push(format!(
                "second run selected adapter {selected_adapter} but best persisted observation was {best_adapter}"
            )),
        }
        if input.second_quality < 0.50 {
            failures.push(format!(
                "second_quality {:.3} below minimum 0.500",
                input.second_quality
            ));
        }
        if input.first_drift_severity == DriftSeverity::Rollback {
            failures.push("first run triggered drift rollback".to_owned());
        }
        if matches!(
            input.second_drift_severity,
            DriftSeverity::Block | DriftSeverity::Rollback
        ) {
            failures.push(format!(
                "second run drift severity was {}",
                input.second_drift_severity.as_str()
            ));
        }

        Self {
            passed: failures.is_empty(),
            first_stored_memory: input.first_stored_memory,
            first_runtime_kv_stored: input.first_runtime_kv_stored,
            first_runtime_kv_namespace_preserved: input.first_runtime_kv_namespace_preserved,
            second_used_memories: input.second_used_memories,
            second_used_runtime_kv_memory: input.second_used_runtime_kv_memory,
            second_used_experiences: input.second_used_experiences,
            second_imported_runtime_kv_blocks: input.second_imported_runtime_kv_blocks,
            second_imported_runtime_kv_from_namespace: input
                .second_imported_runtime_kv_from_namespace,
            second_runtime_adapter_observations: input.second_runtime_adapter_observations,
            second_runtime_adapter_best_score: input.second_runtime_adapter_best_score,
            second_runtime_adapter_best_adapter: input.second_runtime_adapter_best_adapter,
            second_runtime_selected_adapter: input.second_runtime_selected_adapter,
            second_quality: input.second_quality,
            first_drift_severity: input.first_drift_severity,
            second_drift_severity: input.second_drift_severity,
            failures,
        }
    }

    pub fn summary_line(&self) -> String {
        format!(
            "persistent_roundtrip: passed={} first_stored_memory={} first_runtime_kv_stored={} first_runtime_kv_namespace_preserved={} second_used_memories={} second_used_runtime_kv_memory={} second_used_experiences={} second_imported_runtime_kv_blocks={} second_imported_runtime_kv_from_namespace={} second_runtime_adapter_observations={} second_runtime_adapter_best_score={} second_runtime_adapter_best_adapter={} second_runtime_selected_adapter={} second_quality={:.3} first_drift={} second_drift={} failures={}",
            self.passed,
            self.first_stored_memory,
            self.first_runtime_kv_stored,
            self.first_runtime_kv_namespace_preserved,
            self.second_used_memories,
            self.second_used_runtime_kv_memory,
            self.second_used_experiences,
            self.second_imported_runtime_kv_blocks,
            self.second_imported_runtime_kv_from_namespace,
            self.second_runtime_adapter_observations,
            option_f32_display(self.second_runtime_adapter_best_score),
            option_str_display(self.second_runtime_adapter_best_adapter.as_deref()),
            option_str_display(self.second_runtime_selected_adapter.as_deref()),
            self.second_quality,
            self.first_drift_severity.as_str(),
            self.second_drift_severity.as_str(),
            self.failures.len()
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PersistentRoundtripDeviceReport {
    pub device: DeviceClass,
    pub report: PersistentRoundtripReport,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PersistentRoundtripMatrixReport {
    pub passed: bool,
    pub device_reports: Vec<PersistentRoundtripDeviceReport>,
    pub failures: Vec<String>,
}

impl PersistentRoundtripMatrixReport {
    pub fn evaluate(device_reports: Vec<PersistentRoundtripDeviceReport>) -> Self {
        let mut failures = Vec::new();

        if device_reports.is_empty() {
            failures.push("no persistent roundtrip device reports were recorded".to_owned());
        }

        let missing = missing_persistent_roundtrip_devices(&device_reports);
        if !missing.is_empty() {
            let missing_devices = missing
                .iter()
                .map(|device| device.as_str())
                .collect::<Vec<_>>()
                .join("+");
            failures.push(format!(
                "persistent_roundtrip_devices {} below expected {} missing={}",
                explicit_persistent_roundtrip_devices(&device_reports),
                DeviceClass::explicit_profiles().len(),
                missing_devices
            ));
        }

        for device_report in &device_reports {
            if !device_report.report.passed {
                failures.push(format!(
                    "device {} persistent roundtrip failed with {} failures",
                    device_report.device.as_str(),
                    device_report.report.failures.len()
                ));
            }
        }

        Self {
            passed: failures.is_empty(),
            device_reports,
            failures,
        }
    }

    pub fn covered_devices(&self) -> usize {
        explicit_persistent_roundtrip_devices(&self.device_reports)
    }

    pub fn missing_devices(&self) -> Vec<DeviceClass> {
        missing_persistent_roundtrip_devices(&self.device_reports)
    }

    pub fn failed_devices(&self) -> Vec<DeviceClass> {
        self.device_reports
            .iter()
            .filter(|device_report| !device_report.report.passed)
            .map(|device_report| device_report.device)
            .collect()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "persistent_roundtrip_matrix: passed={} devices={} expected_devices={} failed_devices={} failures={}",
            self.passed,
            self.covered_devices(),
            DeviceClass::explicit_profiles().len(),
            self.failed_devices().len(),
            self.failures.len()
        )
    }
}

fn explicit_persistent_roundtrip_devices(
    device_reports: &[PersistentRoundtripDeviceReport],
) -> usize {
    DeviceClass::explicit_profiles()
        .iter()
        .filter(|device| {
            device_reports
                .iter()
                .any(|device_report| device_report.device == **device)
        })
        .count()
}

fn missing_persistent_roundtrip_devices(
    device_reports: &[PersistentRoundtripDeviceReport],
) -> Vec<DeviceClass> {
    DeviceClass::explicit_profiles()
        .iter()
        .copied()
        .filter(|device| {
            !device_reports
                .iter()
                .any(|device_report| device_report.device == *device)
        })
        .collect()
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct BenchmarkReflectionEvidence {
    pub issue_cases: usize,
    pub total_issues: usize,
    pub critical_issue_cases: usize,
    pub total_critical_issues: usize,
    pub revision_action_cases: usize,
    pub total_revision_actions: usize,
    pub live_memory_feedback_reinforcements: usize,
    pub live_memory_feedback_penalties: usize,
    pub live_memory_feedback_applied: usize,
    pub live_memory_feedback_removed: usize,
    pub live_memory_feedback_missing: usize,
    pub live_memory_feedback_strength_delta: f32,
    pub memory_feedback_failures: Vec<String>,
    issue_devices: Vec<DeviceClass>,
    critical_issue_devices: Vec<DeviceClass>,
    revision_action_devices: Vec<DeviceClass>,
}

impl BenchmarkReflectionEvidence {
    fn record(&mut self, outcome: &InferenceOutcome) {
        let issues = outcome.report.issues.len();
        let critical_issues = outcome.report.critical_issue_count();
        let revision_actions = outcome.report.revision_actions.len();

        self.issue_cases += usize::from(issues > 0);
        self.total_issues += issues;
        self.critical_issue_cases += usize::from(critical_issues > 0);
        self.total_critical_issues += critical_issues;
        self.revision_action_cases += usize::from(revision_actions > 0);
        self.total_revision_actions += revision_actions;
        self.live_memory_feedback_reinforcements += outcome.memory_feedback.reinforced;
        self.live_memory_feedback_penalties += outcome.memory_feedback.penalized;
        self.live_memory_feedback_applied += outcome.memory_feedback.applied_updates();
        self.live_memory_feedback_removed += outcome.memory_feedback.removed_updates();
        self.live_memory_feedback_missing += outcome.memory_feedback.missing_updates();
        self.live_memory_feedback_strength_delta += outcome.memory_feedback.strength_delta();
        let expected_updates = outcome
            .memory_feedback
            .reinforced
            .saturating_add(outcome.memory_feedback.penalized);
        if outcome.memory_feedback.updates.len() != expected_updates {
            self.memory_feedback_failures.push(format!(
                "{}:{} memory feedback update reports {} do not match reinforced+penalized {}",
                outcome.hardware_plan.device.as_str(),
                outcome.experience_id,
                outcome.memory_feedback.updates.len(),
                expected_updates
            ));
        }
        if outcome
            .memory_feedback
            .applied_updates()
            .saturating_add(outcome.memory_feedback.missing_updates())
            != expected_updates
        {
            self.memory_feedback_failures.push(format!(
                "{}:{} memory feedback applied+missing {} does not match updates {}",
                outcome.hardware_plan.device.as_str(),
                outcome.experience_id,
                outcome
                    .memory_feedback
                    .applied_updates()
                    .saturating_add(outcome.memory_feedback.missing_updates()),
                expected_updates
            ));
        }
        if outcome.memory_feedback.removed_updates() > outcome.memory_feedback.applied_updates() {
            self.memory_feedback_failures.push(format!(
                "{}:{} memory feedback removed {} exceeds applied {}",
                outcome.hardware_plan.device.as_str(),
                outcome.experience_id,
                outcome.memory_feedback.removed_updates(),
                outcome.memory_feedback.applied_updates()
            ));
        }
        if outcome.memory_feedback.total_updates() > 0
            && outcome.memory_feedback.applied_updates() == 0
            && outcome.memory_feedback.missing_updates() == 0
        {
            self.memory_feedback_failures.push(format!(
                "{}:{} memory feedback has updates but no applied/missing evidence",
                outcome.hardware_plan.device.as_str(),
                outcome.experience_id
            ));
        }

        let device = outcome.hardware_plan.device;
        if issues > 0 {
            push_unique_device(&mut self.issue_devices, device);
        }
        if critical_issues > 0 {
            push_unique_device(&mut self.critical_issue_devices, device);
        }
        if revision_actions > 0 {
            push_unique_device(&mut self.revision_action_devices, device);
        }
    }

    pub fn issue_device_profiles(&self) -> usize {
        explicit_device_count(&self.issue_devices)
    }

    pub fn critical_issue_device_profiles(&self) -> usize {
        explicit_device_count(&self.critical_issue_devices)
    }

    pub fn revision_action_device_profiles(&self) -> usize {
        explicit_device_count(&self.revision_action_devices)
    }

    pub fn live_memory_feedback_updates(&self) -> usize {
        self.live_memory_feedback_reinforcements + self.live_memory_feedback_penalties
    }

    pub fn memory_feedback_evidence_failures(&self) -> usize {
        self.memory_feedback_failures.len()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BenchmarkLiveEvolutionEvidence {
    inference_devices: Vec<DeviceClass>,
    router_threshold_mutation_devices: Vec<DeviceClass>,
    hierarchy_weight_mutation_devices: Vec<DeviceClass>,
    online_reward_devices: Vec<DeviceClass>,
    online_reward_strength_devices: Vec<DeviceClass>,
    memory_update_devices: Vec<DeviceClass>,
    stored_memory_update_devices: Vec<DeviceClass>,
    reflection_issue_devices: Vec<DeviceClass>,
    critical_reflection_issue_devices: Vec<DeviceClass>,
    revision_action_devices: Vec<DeviceClass>,
    replay_live_evolution_devices: Vec<DeviceClass>,
    replay_live_evolution_online_reward_devices: Vec<DeviceClass>,
    replay_live_evolution_online_reward_strength_devices: Vec<DeviceClass>,
    replay_live_evolution_memory_update_devices: Vec<DeviceClass>,
    replay_live_evolution_critical_reflection_issue_devices: Vec<DeviceClass>,
    replay_live_evolution_revision_action_devices: Vec<DeviceClass>,
}

impl BenchmarkLiveEvolutionEvidence {
    fn record(&mut self, outcome: &InferenceOutcome) {
        let device = outcome.hardware_plan.device;
        let live = outcome.live_evolution;

        push_unique_device(&mut self.inference_devices, device);
        if live.router_threshold_delta > 0.000001 {
            push_unique_device(&mut self.router_threshold_mutation_devices, device);
        }
        if live.hierarchy_weight_delta > 0.000001 {
            push_unique_device(&mut self.hierarchy_weight_mutation_devices, device);
        }
        if live.online_reward_feedbacks > 0
            && live.online_reward_feedbacks
                == live
                    .online_reward_reinforcements
                    .saturating_add(live.online_reward_penalties)
        {
            push_unique_device(&mut self.online_reward_devices, device);
        }
        if online_reward_strength_is_consistent(
            live.online_reward_feedbacks,
            live.online_reward_reinforcements,
            live.online_reward_penalties,
            live.online_reward_strength,
            live.online_reward_reinforcement_strength,
            live.online_reward_penalty_strength,
        ) {
            push_unique_device(&mut self.online_reward_strength_devices, device);
        }
        if live.memory_reinforcements > 0 || live.memory_penalties > 0 {
            push_unique_device(&mut self.memory_update_devices, device);
        }
        if live.stored_memory
            || live.stored_gist_memories > 0
            || live.stored_runtime_kv_memories > 0
        {
            push_unique_device(&mut self.stored_memory_update_devices, device);
        }
        if live.reflection_issues > 0 {
            push_unique_device(&mut self.reflection_issue_devices, device);
        }
        if live.critical_reflection_issues > 0 {
            push_unique_device(&mut self.critical_reflection_issue_devices, device);
        }
        if live.revision_actions > 0 {
            push_unique_device(&mut self.revision_action_devices, device);
        }
        if let Some(replay) = outcome.auto_replay_report.as_ref() {
            if replay.live_evolution_items > 0 {
                push_unique_device(&mut self.replay_live_evolution_devices, device);
            }
            if replay.live_evolution_online_reward_feedbacks > 0
                && replay.live_evolution_online_reward_feedbacks
                    == replay
                        .live_evolution_online_reward_reinforcements
                        .saturating_add(replay.live_evolution_online_reward_penalties)
            {
                push_unique_device(
                    &mut self.replay_live_evolution_online_reward_devices,
                    device,
                );
            }
            if online_reward_strength_is_consistent(
                replay.live_evolution_online_reward_feedbacks,
                replay.live_evolution_online_reward_reinforcements,
                replay.live_evolution_online_reward_penalties,
                replay.live_evolution_online_reward_strength,
                replay.live_evolution_online_reward_reinforcement_strength,
                replay.live_evolution_online_reward_penalty_strength,
            ) {
                push_unique_device(
                    &mut self.replay_live_evolution_online_reward_strength_devices,
                    device,
                );
            }
            if replay.live_evolution_memory_updates > 0 {
                push_unique_device(
                    &mut self.replay_live_evolution_memory_update_devices,
                    device,
                );
            }
            if replay.live_evolution_critical_reflection_issues > 0 {
                push_unique_device(
                    &mut self.replay_live_evolution_critical_reflection_issue_devices,
                    device,
                );
            }
            if replay.live_evolution_revision_actions > 0 {
                push_unique_device(
                    &mut self.replay_live_evolution_revision_action_devices,
                    device,
                );
            }
        }
    }

    pub fn inference_device_profiles(&self) -> usize {
        explicit_device_count(&self.inference_devices)
    }

    pub fn router_threshold_mutation_device_profiles(&self) -> usize {
        explicit_device_count(&self.router_threshold_mutation_devices)
    }

    pub fn hierarchy_weight_mutation_device_profiles(&self) -> usize {
        explicit_device_count(&self.hierarchy_weight_mutation_devices)
    }

    pub fn online_reward_device_profiles(&self) -> usize {
        explicit_device_count(&self.online_reward_devices)
    }

    pub fn online_reward_strength_device_profiles(&self) -> usize {
        explicit_device_count(&self.online_reward_strength_devices)
    }

    pub fn memory_update_device_profiles(&self) -> usize {
        explicit_device_count(&self.memory_update_devices)
    }

    pub fn stored_memory_update_device_profiles(&self) -> usize {
        explicit_device_count(&self.stored_memory_update_devices)
    }

    pub fn reflection_issue_device_profiles(&self) -> usize {
        explicit_device_count(&self.reflection_issue_devices)
    }

    pub fn critical_reflection_issue_device_profiles(&self) -> usize {
        explicit_device_count(&self.critical_reflection_issue_devices)
    }

    pub fn revision_action_device_profiles(&self) -> usize {
        explicit_device_count(&self.revision_action_devices)
    }

    pub fn replay_live_evolution_device_profiles(&self) -> usize {
        explicit_device_count(&self.replay_live_evolution_devices)
    }

    pub fn replay_live_evolution_online_reward_device_profiles(&self) -> usize {
        explicit_device_count(&self.replay_live_evolution_online_reward_devices)
    }

    pub fn replay_live_evolution_online_reward_strength_device_profiles(&self) -> usize {
        explicit_device_count(&self.replay_live_evolution_online_reward_strength_devices)
    }

    pub fn replay_live_evolution_memory_update_device_profiles(&self) -> usize {
        explicit_device_count(&self.replay_live_evolution_memory_update_devices)
    }

    pub fn replay_live_evolution_critical_reflection_issue_device_profiles(&self) -> usize {
        explicit_device_count(&self.replay_live_evolution_critical_reflection_issue_devices)
    }

    pub fn replay_live_evolution_revision_action_device_profiles(&self) -> usize {
        explicit_device_count(&self.replay_live_evolution_revision_action_devices)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BenchmarkMemoryGovernanceEvidence {
    pub cases: usize,
    pub retention_activity_cases: usize,
    pub compaction_activity_cases: usize,
    pub total_retention_decayed: usize,
    pub total_retention_removed: usize,
    pub total_compaction_merged: usize,
    pub total_compaction_removed: usize,
    pub failures: Vec<String>,
    governance_devices: Vec<DeviceClass>,
    retention_activity_devices: Vec<DeviceClass>,
    compaction_activity_devices: Vec<DeviceClass>,
}

impl BenchmarkMemoryGovernanceEvidence {
    fn record(&mut self, case: &BenchmarkCase, outcome: &InferenceOutcome) {
        self.cases += 1;
        let device = outcome.hardware_plan.device;
        push_unique_device(&mut self.governance_devices, device);

        let retention = &outcome.retention_report;
        let retention_removed = retention.removed.len();
        self.total_retention_decayed += retention.decayed;
        self.total_retention_removed += retention_removed;
        if retention.decayed > 0 || retention_removed > 0 {
            self.retention_activity_cases += 1;
            push_unique_device(&mut self.retention_activity_devices, device);
        }

        if outcome.memory_retention_policy.stale_after == 0 {
            self.failures.push(format!(
                "{}:{} retention stale_after must be > 0",
                device.as_str(),
                case.name
            ));
        }
        if !(0.0..=0.95).contains(&outcome.memory_retention_policy.decay_rate) {
            self.failures.push(format!(
                "{}:{} retention decay_rate {:.6} outside 0.0..=0.95",
                device.as_str(),
                case.name,
                outcome.memory_retention_policy.decay_rate
            ));
        }
        if !(0.0..=3.0).contains(&outcome.memory_retention_policy.remove_below_strength) {
            self.failures.push(format!(
                "{}:{} retention remove_below_strength {:.6} outside 0.0..=3.0",
                device.as_str(),
                case.name,
                outcome.memory_retention_policy.remove_below_strength
            ));
        }
        if outcome.memory_retention_policy.remove_after_failures == 0 {
            self.failures.push(format!(
                "{}:{} retention remove_after_failures must be > 0",
                device.as_str(),
                case.name
            ));
        }
        if retention.decayed > retention.before {
            self.failures.push(format!(
                "{}:{} retention decayed {} exceeds before {}",
                device.as_str(),
                case.name,
                retention.decayed,
                retention.before
            ));
        }
        if retention_removed > retention.before {
            self.failures.push(format!(
                "{}:{} retention removed {} exceeds before {}",
                device.as_str(),
                case.name,
                retention_removed,
                retention.before
            ));
        }
        if retention.after > retention.before {
            self.failures.push(format!(
                "{}:{} retention after {} exceeds before {}",
                device.as_str(),
                case.name,
                retention.after,
                retention.before
            ));
        }
        if retention.after.saturating_add(retention_removed) != retention.before {
            self.failures.push(format!(
                "{}:{} retention before {} does not match after+removed {}",
                device.as_str(),
                case.name,
                retention.before,
                retention.after.saturating_add(retention_removed)
            ));
        }

        let compaction = &outcome.memory_compaction_report;
        let compaction_merged = compaction.merged.len();
        let compaction_removed = compaction.removed.len();
        self.total_compaction_merged += compaction_merged;
        self.total_compaction_removed += compaction_removed;
        if compaction_merged > 0 || compaction_removed > 0 {
            self.compaction_activity_cases += 1;
            push_unique_device(&mut self.compaction_activity_devices, device);
        }

        if !(0.10..=0.999).contains(&outcome.memory_compaction_policy.similarity_threshold) {
            self.failures.push(format!(
                "{}:{} memory_compaction similarity_threshold {:.6} outside 0.10..=0.999",
                device.as_str(),
                case.name,
                outcome.memory_compaction_policy.similarity_threshold
            ));
        }
        if compaction.merged.len() != compaction.removed.len() {
            self.failures.push(format!(
                "{}:{} memory_compaction merged {} does not match removed {}",
                device.as_str(),
                case.name,
                compaction_merged,
                compaction_removed
            ));
        }
        if compaction_merged > outcome.memory_compaction_policy.max_merges {
            self.failures.push(format!(
                "{}:{} memory_compaction merged {} exceeds max_merges {}",
                device.as_str(),
                case.name,
                compaction_merged,
                outcome.memory_compaction_policy.max_merges
            ));
        }
        if compaction_removed > compaction.before {
            self.failures.push(format!(
                "{}:{} memory_compaction removed {} exceeds before {}",
                device.as_str(),
                case.name,
                compaction_removed,
                compaction.before
            ));
        }
        if compaction.after > compaction.before {
            self.failures.push(format!(
                "{}:{} memory_compaction after {} exceeds before {}",
                device.as_str(),
                case.name,
                compaction.after,
                compaction.before
            ));
        }
        if compaction.after.saturating_add(compaction_removed) != compaction.before {
            self.failures.push(format!(
                "{}:{} memory_compaction before {} does not match after+removed {}",
                device.as_str(),
                case.name,
                compaction.before,
                compaction.after.saturating_add(compaction_removed)
            ));
        }
        if compaction.before < 2
            || outcome.memory_compaction_policy.max_candidates < 2
            || outcome.memory_compaction_policy.max_merges == 0
        {
            if compaction_merged > 0
                || compaction_removed > 0
                || compaction.after != compaction.before
            {
                self.failures.push(format!(
                    "{}:{} memory_compaction skipped state requires merged=0 removed=0 after=before, got merged={} removed={} before={} after={}",
                    device.as_str(),
                    case.name,
                    compaction_merged,
                    compaction_removed,
                    compaction.before,
                    compaction.after
                ));
            }
        }
    }

    pub fn device_profiles(&self) -> usize {
        explicit_device_count(&self.governance_devices)
    }

    pub fn retention_activity_device_profiles(&self) -> usize {
        explicit_device_count(&self.retention_activity_devices)
    }

    pub fn compaction_activity_device_profiles(&self) -> usize {
        explicit_device_count(&self.compaction_activity_devices)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BenchmarkEmbeddingEvidence {
    pub cases: usize,
    pub runtime_cases: usize,
    pub fallback_cases: usize,
    pub runtime_calls: usize,
    pub fallback_calls: usize,
    pub failures: Vec<String>,
    runtime_devices: Vec<DeviceClass>,
}

impl BenchmarkEmbeddingEvidence {
    fn record(&mut self, case: &BenchmarkCase, outcome: &InferenceOutcome) {
        let diagnostics = &outcome.embedding_diagnostics;
        let device = outcome.hardware_plan.device;
        self.cases += 1;

        if diagnostics.query.dimensions == 0 {
            self.failures.push(format!(
                "{}:{} embedding query dimensions are missing",
                device.as_str(),
                case.name
            ));
        }

        let expected_calls = diagnostics.total_calls();
        let observed_calls = diagnostics
            .runtime_calls
            .saturating_add(diagnostics.fallback_calls);
        if observed_calls != expected_calls {
            self.failures.push(format!(
                "{}:{} embedding calls {} do not match expected {}",
                device.as_str(),
                case.name,
                observed_calls,
                expected_calls
            ));
        }

        if diagnostics.runtime_embedding_available() {
            self.runtime_cases += 1;
            push_unique_device(&mut self.runtime_devices, device);
        }
        if diagnostics.fallback_embedding_used() {
            self.fallback_cases += 1;
        }
        self.runtime_calls += diagnostics.runtime_calls;
        self.fallback_calls += diagnostics.fallback_calls;
    }

    pub fn runtime_device_profiles(&self) -> usize {
        explicit_device_count(&self.runtime_devices)
    }

    pub fn runtime_devices_csv(&self) -> String {
        if self.runtime_devices.is_empty() {
            "none".to_owned()
        } else {
            self.runtime_devices
                .iter()
                .map(|device| device.as_str())
                .collect::<Vec<_>>()
                .join("+")
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BenchmarkRuntimeDeviceExecutionEvidence {
    pub cases: usize,
    pub matched_cases: usize,
    pub runtime_kv_precision_cases: usize,
    pub failures: Vec<String>,
    matched_devices: Vec<DeviceClass>,
    kv_precision_devices: Vec<DeviceClass>,
}

impl BenchmarkRuntimeDeviceExecutionEvidence {
    fn record(&mut self, case: &BenchmarkCase, outcome: &InferenceOutcome) {
        let diagnostics = &outcome.runtime_diagnostics;
        let has_forward_signal = diagnostics.has_forward_signal();
        let has_device_execution_signal = diagnostics.has_device_execution_signal();

        if !has_forward_signal && !has_device_execution_signal {
            return;
        }

        let device = outcome.hardware_plan.device;
        if !has_device_execution_signal {
            self.failures.push(format!(
                "{}:{} runtime forward signal is missing device execution diagnostics",
                device.as_str(),
                case.name
            ));
            return;
        }

        self.cases += 1;
        let execution = &outcome.hardware_plan.execution;
        let mut mismatches = Vec::new();
        record_runtime_device_execution_mismatch(
            &mut mismatches,
            "device_profile",
            diagnostics.device_profile.as_deref(),
            device.as_str(),
        );
        record_runtime_device_execution_mismatch(
            &mut mismatches,
            "primary_lane",
            diagnostics.primary_lane.as_deref(),
            execution.primary_lane.as_str(),
        );
        record_runtime_device_execution_mismatch(
            &mut mismatches,
            "fallback_lane",
            diagnostics.fallback_lane.as_deref(),
            execution.fallback_lane.as_str(),
        );
        record_runtime_device_execution_mismatch(
            &mut mismatches,
            "memory_mode",
            diagnostics.memory_mode.as_deref(),
            execution.memory_mode.as_str(),
        );
        record_runtime_device_execution_usize_mismatch(
            &mut mismatches,
            "hot_kv_precision_bits",
            diagnostics.hot_kv_precision_bits.map(usize::from),
            usize::from(execution.hot_kv_precision_bits),
        );
        record_runtime_device_execution_usize_mismatch(
            &mut mismatches,
            "cold_kv_precision_bits",
            diagnostics.cold_kv_precision_bits.map(usize::from),
            usize::from(execution.cold_kv_precision_bits),
        );

        if diagnostics.has_valid_kv_precision_signal() {
            self.runtime_kv_precision_cases += 1;
            push_unique_device(&mut self.kv_precision_devices, device);
        } else {
            self.failures.push(format!(
                "{}:{} runtime device execution is missing valid KV precision diagnostics",
                device.as_str(),
                case.name
            ));
        }

        if mismatches.is_empty() {
            self.matched_cases += 1;
            push_unique_device(&mut self.matched_devices, device);
        } else {
            self.failures.push(format!(
                "{}:{} runtime device execution mismatch: {}",
                device.as_str(),
                case.name,
                mismatches.join(", ")
            ));
        }
    }

    pub fn device_profiles(&self) -> usize {
        explicit_device_count(&self.matched_devices)
    }

    pub fn matched_devices_csv(&self) -> String {
        if self.matched_devices.is_empty() {
            "none".to_owned()
        } else {
            self.matched_devices
                .iter()
                .map(|device| device.as_str())
                .collect::<Vec<_>>()
                .join("+")
        }
    }

    pub fn runtime_kv_precision_device_profiles(&self) -> usize {
        explicit_device_count(&self.kv_precision_devices)
    }

    pub fn runtime_kv_precision_devices_csv(&self) -> String {
        if self.kv_precision_devices.is_empty() {
            "none".to_owned()
        } else {
            self.kv_precision_devices
                .iter()
                .map(|device| device.as_str())
                .collect::<Vec<_>>()
                .join("+")
        }
    }
}

fn record_runtime_device_execution_mismatch(
    mismatches: &mut Vec<String>,
    field: &str,
    actual: Option<&str>,
    expected: &str,
) {
    match actual {
        Some(actual) if actual == expected => {}
        Some(actual) => mismatches.push(format!("{field} actual={actual} expected={expected}")),
        None => mismatches.push(format!("{field} missing expected={expected}")),
    }
}

fn record_runtime_device_execution_usize_mismatch(
    mismatches: &mut Vec<String>,
    field: &str,
    actual: Option<usize>,
    expected: usize,
) {
    match actual {
        Some(actual) if actual == expected => {}
        Some(actual) => mismatches.push(format!("{field} actual={actual} expected={expected}")),
        None => mismatches.push(format!("{field} missing expected={expected}")),
    }
}

fn push_unique_device(devices: &mut Vec<DeviceClass>, device: DeviceClass) {
    if device != DeviceClass::Auto && !devices.contains(&device) {
        devices.push(device);
    }
}

fn devices_csv(devices: Vec<DeviceClass>) -> String {
    let devices = devices
        .into_iter()
        .map(DeviceClass::as_str)
        .collect::<Vec<_>>();

    if devices.is_empty() {
        "none".to_owned()
    } else {
        devices.join("+")
    }
}

fn explicit_device_count(devices: &[DeviceClass]) -> usize {
    DeviceClass::explicit_profiles()
        .iter()
        .filter(|device| devices.contains(device))
        .count()
}

fn online_reward_strength_is_consistent(
    feedbacks: usize,
    reinforcements: usize,
    penalties: usize,
    total: f32,
    reinforcement: f32,
    penalty: f32,
) -> bool {
    let has_reinforcement_strength = reinforcement > BENCHMARK_FLOAT_EPSILON;
    let has_penalty_strength = penalty > BENCHMARK_FLOAT_EPSILON;
    total.is_finite()
        && reinforcement.is_finite()
        && penalty.is_finite()
        && feedbacks > 0
        && feedbacks == reinforcements.saturating_add(penalties)
        && total > BENCHMARK_FLOAT_EPSILON
        && reinforcement >= 0.0
        && penalty >= 0.0
        && (!has_reinforcement_strength || reinforcements > 0)
        && (!has_penalty_strength || penalties > 0)
        && (total - (reinforcement + penalty)).abs() <= BENCHMARK_FLOAT_EPSILON
}

#[derive(Debug, Clone, Default)]
pub struct BenchmarkSummary {
    results: Vec<BenchmarkCaseResult>,
    evolution_ledger: EvolutionLedger,
    reflection_evidence: BenchmarkReflectionEvidence,
    live_evolution_evidence: BenchmarkLiveEvolutionEvidence,
    memory_governance_evidence: BenchmarkMemoryGovernanceEvidence,
    embedding_evidence: BenchmarkEmbeddingEvidence,
    runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence,
}

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
                !runtime_adapter_contract_ok
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
        self.memory_governance_evidence.record(case, outcome);
        self.embedding_evidence.record(case, outcome);
        self.runtime_device_execution_evidence.record(case, outcome);
    }

    pub fn results(&self) -> &[BenchmarkCaseResult] {
        &self.results
    }

    pub fn len(&self) -> usize {
        self.results.len()
    }

    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }

    pub fn total_elapsed_ms(&self) -> u128 {
        self.results.iter().map(|result| result.elapsed_ms).sum()
    }

    pub fn covered_device_profiles(&self) -> Vec<DeviceClass> {
        let mut devices = Vec::new();

        for result in &self.results {
            if result.device != DeviceClass::Auto && !devices.contains(&result.device) {
                devices.push(result.device);
            }
        }

        devices
    }

    pub fn explicit_device_profiles_covered(&self) -> usize {
        DeviceClass::explicit_profiles()
            .iter()
            .filter(|device| self.results.iter().any(|result| result.device == **device))
            .count()
    }

    pub fn missing_explicit_device_profiles(&self) -> Vec<DeviceClass> {
        DeviceClass::explicit_profiles()
            .iter()
            .copied()
            .filter(|device| !self.results.iter().any(|result| result.device == *device))
            .collect()
    }

    pub fn recursive_device_profiles_covered(&self) -> usize {
        DeviceClass::explicit_profiles()
            .iter()
            .filter(|device| {
                self.results
                    .iter()
                    .any(|result| result.device == **device && result.requires_recursion)
            })
            .count()
    }

    pub fn missing_recursive_device_profiles(&self) -> Vec<DeviceClass> {
        DeviceClass::explicit_profiles()
            .iter()
            .copied()
            .filter(|device| {
                !self
                    .results
                    .iter()
                    .any(|result| result.device == *device && result.requires_recursion)
            })
            .collect()
    }

    pub fn devices_csv(&self) -> String {
        let devices = self
            .covered_device_profiles()
            .into_iter()
            .map(DeviceClass::as_str)
            .collect::<Vec<_>>();

        if devices.is_empty() {
            "none".to_owned()
        } else {
            devices.join("+")
        }
    }

    pub fn recursive_devices_csv(&self) -> String {
        let mut devices = Vec::new();

        for result in &self.results {
            if result.requires_recursion
                && result.device != DeviceClass::Auto
                && !devices.contains(&result.device)
            {
                devices.push(result.device);
            }
        }

        if devices.is_empty() {
            "none".to_owned()
        } else {
            devices
                .into_iter()
                .map(DeviceClass::as_str)
                .collect::<Vec<_>>()
                .join("+")
        }
    }

    pub fn average_quality(&self) -> f32 {
        average(self.results.iter().map(|result| result.quality))
    }

    pub fn average_reward(&self) -> f32 {
        average(self.results.iter().map(|result| result.process_reward))
    }

    pub fn average_attention_fraction(&self) -> f32 {
        average(self.results.iter().map(|result| result.attention_fraction))
    }

    pub fn total_runtime_kv_stored(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.runtime_kv_stored)
            .sum()
    }

    pub fn runtime_kv_stored_device_profiles(&self) -> usize {
        explicit_device_count(&self.runtime_kv_stored_devices())
    }

    pub fn runtime_kv_stored_devices_csv(&self) -> String {
        devices_csv(self.runtime_kv_stored_devices())
    }

    fn runtime_kv_stored_devices(&self) -> Vec<DeviceClass> {
        let mut devices = Vec::new();

        for result in &self.results {
            if result.runtime_kv_stored > 0 {
                push_unique_device(&mut devices, result.device);
            }
        }

        devices
    }

    pub fn runtime_kv_hold_cases(&self) -> usize {
        self.results
            .iter()
            .filter(|result| runtime_kv_was_held(result))
            .count()
    }

    pub fn total_runtime_kv_held(&self) -> usize {
        self.results
            .iter()
            .filter(|result| runtime_kv_was_held(result))
            .map(|result| {
                result
                    .runtime_kv_exported
                    .saturating_sub(result.runtime_kv_stored)
            })
            .sum()
    }

    pub fn runtime_kv_hold_device_profiles(&self) -> usize {
        explicit_device_count(&self.runtime_kv_hold_devices())
    }

    pub fn runtime_kv_hold_devices_csv(&self) -> String {
        let devices = self
            .runtime_kv_hold_devices()
            .into_iter()
            .map(DeviceClass::as_str)
            .collect::<Vec<_>>();

        if devices.is_empty() {
            "none".to_owned()
        } else {
            devices.join("+")
        }
    }

    fn runtime_kv_hold_devices(&self) -> Vec<DeviceClass> {
        let mut devices = Vec::new();

        for result in &self.results {
            if runtime_kv_was_held(result)
                && result.device != DeviceClass::Auto
                && !devices.contains(&result.device)
            {
                devices.push(result.device);
            }
        }

        devices
    }

    pub fn runtime_token_cases(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.runtime_token_count > 0)
            .count()
    }

    pub fn total_runtime_tokens(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.runtime_token_count)
            .sum()
    }

    pub fn runtime_uncertainty_cases(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.runtime_uncertainty_signal)
            .count()
    }

    pub fn total_runtime_uncertainty_tokens(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.runtime_uncertainty_token_count)
            .sum()
    }

    pub fn runtime_uncertainty_device_profiles(&self) -> usize {
        explicit_device_count(&self.runtime_uncertainty_devices())
    }

    pub fn runtime_uncertainty_devices_csv(&self) -> String {
        devices_csv(self.runtime_uncertainty_devices())
    }

    pub fn runtime_uncertainty_token_device_profiles(&self) -> usize {
        explicit_device_count(&self.runtime_uncertainty_token_devices())
    }

    pub fn runtime_uncertainty_token_devices_csv(&self) -> String {
        devices_csv(self.runtime_uncertainty_token_devices())
    }

    fn runtime_uncertainty_devices(&self) -> Vec<DeviceClass> {
        let mut devices = Vec::new();

        for result in &self.results {
            if result.runtime_uncertainty_signal {
                push_unique_device(&mut devices, result.device);
            }
        }

        devices
    }

    fn runtime_uncertainty_token_devices(&self) -> Vec<DeviceClass> {
        let mut devices = Vec::new();

        for result in &self.results {
            if result.runtime_uncertainty_token_count > 0 {
                push_unique_device(&mut devices, result.device);
            }
        }

        devices
    }

    pub fn runtime_kv_import_cases(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.runtime_kv_imported > 0)
            .count()
    }

    pub fn runtime_kv_import_device_profiles(&self) -> usize {
        explicit_device_count(&self.runtime_kv_import_devices())
    }

    pub fn runtime_kv_import_devices_csv(&self) -> String {
        devices_csv(self.runtime_kv_import_devices())
    }

    fn runtime_kv_import_devices(&self) -> Vec<DeviceClass> {
        let mut devices = Vec::new();

        for result in &self.results {
            if result.runtime_kv_imported > 0 {
                push_unique_device(&mut devices, result.device);
            }
        }

        devices
    }

    pub fn total_runtime_kv_imported(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.runtime_kv_imported)
            .sum()
    }

    pub fn runtime_kv_export_device_profiles(&self) -> usize {
        explicit_device_count(&self.runtime_kv_export_devices())
    }

    pub fn runtime_kv_export_devices_csv(&self) -> String {
        devices_csv(self.runtime_kv_export_devices())
    }

    fn runtime_kv_export_devices(&self) -> Vec<DeviceClass> {
        let mut devices = Vec::new();

        for result in &self.results {
            if result.runtime_kv_exported > 0 {
                push_unique_device(&mut devices, result.device);
            }
        }

        devices
    }

    pub fn total_runtime_kv_exported(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.runtime_kv_exported)
            .sum()
    }

    pub fn runtime_adapter_contract_cases(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.runtime_forward_signal && result.runtime_adapter_contract_ok)
            .count()
    }

    pub fn runtime_adapter_kinds(&self) -> usize {
        let mut adapters = Vec::new();

        for result in &self.results {
            if result.runtime_forward_signal && result.runtime_adapter_contract_ok {
                if let Some(adapter) = result.runtime_selected_adapter.as_deref() {
                    if !adapters.contains(&adapter) {
                        adapters.push(adapter);
                    }
                }
            }
        }

        adapters.len()
    }

    pub fn total_runtime_adapter_contract_violations(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.runtime_adapter_contract_violations)
            .sum()
    }

    pub fn total_runtime_adapter_selection_mismatches(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.runtime_adapter_selection_mismatches)
            .sum()
    }

    pub fn runtime_forward_cases(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.runtime_forward_signal)
            .count()
    }

    pub fn runtime_forward_energy_cases(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.runtime_forward_energy_signal)
            .count()
    }

    pub fn runtime_kv_influence_cases(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.runtime_kv_influence_signal)
            .count()
    }

    pub fn runtime_layer_mode_cases(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.runtime_layer_mode_signal)
            .count()
    }

    pub fn runtime_all_layer_mode_cases(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.runtime_all_layer_modes_signal)
            .count()
    }

    pub fn total_runtime_global_layers(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.runtime_global_layers)
            .sum()
    }

    pub fn total_runtime_local_window_layers(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.runtime_local_window_layers)
            .sum()
    }

    pub fn total_runtime_convolutional_fusion_layers(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.runtime_convolutional_fusion_layers)
            .sum()
    }

    pub fn total_runtime_adapter_observations(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.runtime_adapter_observations)
            .sum()
    }

    pub fn max_runtime_adapter_score(&self) -> Option<f32> {
        self.results
            .iter()
            .filter_map(|result| result.runtime_adapter_best_score)
            .reduce(f32::max)
    }

    pub fn reflection_evidence(&self) -> BenchmarkReflectionEvidence {
        self.reflection_evidence.clone()
    }

    pub fn live_evolution_evidence(&self) -> BenchmarkLiveEvolutionEvidence {
        self.live_evolution_evidence.clone()
    }

    pub fn memory_governance_evidence(&self) -> BenchmarkMemoryGovernanceEvidence {
        self.memory_governance_evidence.clone()
    }

    pub fn embedding_evidence(&self) -> BenchmarkEmbeddingEvidence {
        self.embedding_evidence.clone()
    }

    pub fn runtime_embedding_cases(&self) -> usize {
        self.embedding_evidence.runtime_cases
    }

    pub fn embedding_fallback_cases(&self) -> usize {
        self.embedding_evidence.fallback_cases
    }

    pub fn runtime_embedding_device_profiles(&self) -> usize {
        self.embedding_evidence.runtime_device_profiles()
    }

    pub fn total_runtime_embedding_calls(&self) -> usize {
        self.embedding_evidence.runtime_calls
    }

    pub fn total_fallback_embedding_calls(&self) -> usize {
        self.embedding_evidence.fallback_calls
    }

    pub fn total_embedding_evidence_failures(&self) -> usize {
        self.embedding_evidence.failures.len()
    }

    pub fn runtime_device_execution_evidence(&self) -> BenchmarkRuntimeDeviceExecutionEvidence {
        self.runtime_device_execution_evidence.clone()
    }

    pub fn runtime_device_execution_cases(&self) -> usize {
        self.runtime_device_execution_evidence.cases
    }

    pub fn runtime_device_execution_matched_cases(&self) -> usize {
        self.runtime_device_execution_evidence.matched_cases
    }

    pub fn runtime_device_execution_device_profiles(&self) -> usize {
        self.runtime_device_execution_evidence.device_profiles()
    }

    pub fn runtime_kv_precision_cases(&self) -> usize {
        self.runtime_device_execution_evidence
            .runtime_kv_precision_cases
    }

    pub fn runtime_kv_precision_device_profiles(&self) -> usize {
        self.runtime_device_execution_evidence
            .runtime_kv_precision_device_profiles()
    }

    pub fn total_runtime_device_execution_violations(&self) -> usize {
        self.runtime_device_execution_evidence.failures.len()
    }

    pub fn memory_governance_cases(&self) -> usize {
        self.memory_governance_evidence.cases
    }

    pub fn memory_governance_device_profiles(&self) -> usize {
        self.memory_governance_evidence.device_profiles()
    }

    pub fn total_memory_retention_decayed(&self) -> usize {
        self.memory_governance_evidence.total_retention_decayed
    }

    pub fn total_memory_retention_removed(&self) -> usize {
        self.memory_governance_evidence.total_retention_removed
    }

    pub fn total_memory_compaction_merged(&self) -> usize {
        self.memory_governance_evidence.total_compaction_merged
    }

    pub fn total_memory_compaction_removed(&self) -> usize {
        self.memory_governance_evidence.total_compaction_removed
    }

    pub fn total_live_memory_feedback_reinforcements(&self) -> usize {
        self.reflection_evidence.live_memory_feedback_reinforcements
    }

    pub fn total_live_memory_feedback_penalties(&self) -> usize {
        self.reflection_evidence.live_memory_feedback_penalties
    }

    pub fn total_live_memory_feedback_updates(&self) -> usize {
        self.reflection_evidence.live_memory_feedback_updates()
    }

    pub fn total_live_memory_feedback_applied(&self) -> usize {
        self.reflection_evidence.live_memory_feedback_applied
    }

    pub fn total_live_memory_feedback_removed(&self) -> usize {
        self.reflection_evidence.live_memory_feedback_removed
    }

    pub fn total_live_memory_feedback_missing(&self) -> usize {
        self.reflection_evidence.live_memory_feedback_missing
    }

    pub fn total_live_memory_feedback_strength_delta(&self) -> f32 {
        self.reflection_evidence.live_memory_feedback_strength_delta
    }

    pub fn total_memory_feedback_evidence_failures(&self) -> usize {
        self.reflection_evidence.memory_feedback_evidence_failures()
    }

    pub fn total_stored_memories(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.stored_memories)
            .sum()
    }

    pub fn total_compacted_memories(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.compacted_memories)
            .sum()
    }

    pub fn sparse_skipped_cases(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.sparse_skipped > 0)
            .count()
    }

    pub fn total_sparse_skipped(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.sparse_skipped)
            .sum()
    }

    pub fn total_sparse_skipped_tokens(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.sparse_skipped_tokens)
            .sum()
    }

    pub fn drift_watches(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.drift_severity == DriftSeverity::Watch)
            .count()
    }

    pub fn drift_blocks(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.drift_severity == DriftSeverity::Block)
            .count()
    }

    pub fn drift_rollbacks(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.drift_severity == DriftSeverity::Rollback)
            .count()
    }

    pub fn max_recursive_chunks(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.recursive_chunks)
            .max()
            .unwrap_or(0)
    }

    pub fn recursive_cases(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.requires_recursion)
            .count()
    }

    pub fn max_recursive_waves(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.recursive_waves)
            .max()
            .unwrap_or(0)
    }

    pub fn total_recursive_runtime_calls(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.recursive_runtime_calls)
            .sum()
    }

    pub fn total_auto_replay_applied(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.auto_replay_applied)
            .sum()
    }

    pub fn total_auto_replay_router_updates(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.auto_replay_router_updates)
            .sum()
    }

    pub fn total_auto_replay_hierarchy_updates(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.auto_replay_hierarchy_updates)
            .sum()
    }

    pub fn total_auto_replay_router_threshold_mutations(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.auto_replay_router_threshold_mutations)
            .sum()
    }

    pub fn total_auto_replay_hierarchy_weight_mutations(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.auto_replay_hierarchy_weight_mutations)
            .sum()
    }

    pub fn total_auto_replay_router_threshold_delta(&self) -> f32 {
        self.results
            .iter()
            .map(|result| result.auto_replay_router_threshold_delta)
            .sum()
    }

    pub fn total_auto_replay_hierarchy_weight_delta(&self) -> f32 {
        self.results
            .iter()
            .map(|result| result.auto_replay_hierarchy_weight_delta)
            .sum()
    }

    pub fn total_auto_replay_memory_reinforcements(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.auto_replay_memory_reinforcements)
            .sum()
    }

    pub fn total_auto_replay_memory_penalties(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.auto_replay_memory_penalties)
            .sum()
    }

    pub fn total_auto_replay_memory_updates(&self) -> usize {
        self.total_auto_replay_memory_reinforcements() + self.total_auto_replay_memory_penalties()
    }

    pub fn total_auto_replay_live_memory_feedback_items(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.auto_replay_live_memory_feedback_items)
            .sum()
    }

    pub fn total_auto_replay_live_memory_feedback_reinforcements(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.auto_replay_live_memory_feedback_reinforcements)
            .sum()
    }

    pub fn total_auto_replay_live_memory_feedback_penalties(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.auto_replay_live_memory_feedback_penalties)
            .sum()
    }

    pub fn total_auto_replay_live_memory_feedback_updates(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.auto_replay_live_memory_feedback_updates)
            .sum()
    }

    pub fn total_auto_replay_live_memory_feedback_detail_items(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.auto_replay_live_memory_feedback_detail_items)
            .sum()
    }

    pub fn total_auto_replay_live_memory_feedback_applied(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.auto_replay_live_memory_feedback_applied)
            .sum()
    }

    pub fn total_auto_replay_live_memory_feedback_removed(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.auto_replay_live_memory_feedback_removed)
            .sum()
    }

    pub fn total_auto_replay_live_memory_feedback_missing(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.auto_replay_live_memory_feedback_missing)
            .sum()
    }

    pub fn total_auto_replay_live_memory_feedback_strength_delta(&self) -> f32 {
        self.results
            .iter()
            .map(|result| result.auto_replay_live_memory_feedback_strength_delta)
            .sum()
    }

    pub fn total_auto_replay_recursive_items(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.auto_replay_recursive_runtime_items)
            .sum()
    }

    pub fn total_auto_replay_recursive_runtime_calls(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.auto_replay_recursive_runtime_calls)
            .sum()
    }

    pub fn max_auto_replay_recursive_call_pressure(&self) -> f32 {
        self.results
            .iter()
            .map(|result| result.auto_replay_max_recursive_call_pressure)
            .fold(0.0, f32::max)
    }

    pub fn evolution_ledger(&self) -> EvolutionLedger {
        self.evolution_ledger
    }

    pub fn evaluate(&self, gate: &BenchmarkGate) -> BenchmarkGateReport {
        let mut failures = Vec::new();

        if self.is_empty() {
            failures.push("no benchmark cases were recorded".to_owned());
        }

        let average_quality = self.average_quality();
        if average_quality < gate.min_average_quality {
            failures.push(format!(
                "average_quality {:.3} below minimum {:.3}",
                average_quality, gate.min_average_quality
            ));
        }

        let average_reward = self.average_reward();
        if average_reward < gate.min_average_reward {
            failures.push(format!(
                "average_reward {:.3} below minimum {:.3}",
                average_reward, gate.min_average_reward
            ));
        }

        if let Some(max_total_elapsed_ms) = gate.max_total_elapsed_ms {
            let total_elapsed_ms = self.total_elapsed_ms();
            if total_elapsed_ms > max_total_elapsed_ms {
                failures.push(format!(
                    "total_elapsed_ms {} above maximum {}",
                    total_elapsed_ms, max_total_elapsed_ms
                ));
            }
        }

        if let Some(max_case_recursive_chunks) = gate.max_case_recursive_chunks {
            let max_recursive_chunks = self.max_recursive_chunks();
            if max_recursive_chunks > max_case_recursive_chunks {
                failures.push(format!(
                    "max_recursive_chunks {} above maximum {}",
                    max_recursive_chunks, max_case_recursive_chunks
                ));
            }
        }

        if let Some(min_recursive_cases) = gate.min_recursive_cases {
            let recursive_cases = self.recursive_cases();
            if recursive_cases < min_recursive_cases {
                failures.push(format!(
                    "recursive_cases {} below minimum {}",
                    recursive_cases, min_recursive_cases
                ));
            }
        }

        if let Some(min_recursive_runtime_calls) = gate.min_recursive_runtime_calls {
            let recursive_runtime_calls = self.total_recursive_runtime_calls();
            if recursive_runtime_calls < min_recursive_runtime_calls {
                failures.push(format!(
                    "recursive_runtime_calls {} below minimum {}",
                    recursive_runtime_calls, min_recursive_runtime_calls
                ));
            }
        }

        if let Some(min_auto_replay_router_updates) = gate.min_auto_replay_router_updates {
            let auto_replay_router_updates = self.total_auto_replay_router_updates();
            if auto_replay_router_updates < min_auto_replay_router_updates {
                failures.push(format!(
                    "auto_replay_router_updates {} below minimum {}",
                    auto_replay_router_updates, min_auto_replay_router_updates
                ));
            }
        }

        if let Some(min_auto_replay_hierarchy_updates) = gate.min_auto_replay_hierarchy_updates {
            let auto_replay_hierarchy_updates = self.total_auto_replay_hierarchy_updates();
            if auto_replay_hierarchy_updates < min_auto_replay_hierarchy_updates {
                failures.push(format!(
                    "auto_replay_hierarchy_updates {} below minimum {}",
                    auto_replay_hierarchy_updates, min_auto_replay_hierarchy_updates
                ));
            }
        }

        if let Some(min_auto_replay_router_threshold_mutations) =
            gate.min_auto_replay_router_threshold_mutations
        {
            let auto_replay_router_threshold_mutations =
                self.total_auto_replay_router_threshold_mutations();
            if auto_replay_router_threshold_mutations < min_auto_replay_router_threshold_mutations {
                failures.push(format!(
                    "auto_replay_router_threshold_mutations {} below minimum {}",
                    auto_replay_router_threshold_mutations,
                    min_auto_replay_router_threshold_mutations
                ));
            }
        }

        if let Some(min_auto_replay_hierarchy_weight_mutations) =
            gate.min_auto_replay_hierarchy_weight_mutations
        {
            let auto_replay_hierarchy_weight_mutations =
                self.total_auto_replay_hierarchy_weight_mutations();
            if auto_replay_hierarchy_weight_mutations < min_auto_replay_hierarchy_weight_mutations {
                failures.push(format!(
                    "auto_replay_hierarchy_weight_mutations {} below minimum {}",
                    auto_replay_hierarchy_weight_mutations,
                    min_auto_replay_hierarchy_weight_mutations
                ));
            }
        }

        if let Some(min_auto_replay_router_threshold_delta) =
            gate.min_auto_replay_router_threshold_delta
        {
            let auto_replay_router_threshold_delta =
                self.total_auto_replay_router_threshold_delta();
            if auto_replay_router_threshold_delta < min_auto_replay_router_threshold_delta {
                failures.push(format!(
                    "auto_replay_router_threshold_delta {:.6} below minimum {:.6}",
                    auto_replay_router_threshold_delta, min_auto_replay_router_threshold_delta
                ));
            }
        }

        if let Some(min_auto_replay_hierarchy_weight_delta) =
            gate.min_auto_replay_hierarchy_weight_delta
        {
            let auto_replay_hierarchy_weight_delta =
                self.total_auto_replay_hierarchy_weight_delta();
            if auto_replay_hierarchy_weight_delta < min_auto_replay_hierarchy_weight_delta {
                failures.push(format!(
                    "auto_replay_hierarchy_weight_delta {:.6} below minimum {:.6}",
                    auto_replay_hierarchy_weight_delta, min_auto_replay_hierarchy_weight_delta
                ));
            }
        }

        if let Some(min_auto_replay_memory_updates) = gate.min_auto_replay_memory_updates {
            let auto_replay_memory_updates = self.total_auto_replay_memory_updates();
            if auto_replay_memory_updates < min_auto_replay_memory_updates {
                failures.push(format!(
                    "auto_replay_memory_updates {} below minimum {}",
                    auto_replay_memory_updates, min_auto_replay_memory_updates
                ));
            }
        }

        if let Some(min_live_memory_feedback_updates) = gate.min_live_memory_feedback_updates {
            let live_memory_feedback_updates = self.total_live_memory_feedback_updates();
            if live_memory_feedback_updates < min_live_memory_feedback_updates {
                failures.push(format!(
                    "live_memory_feedback_updates {} below minimum {}",
                    live_memory_feedback_updates, min_live_memory_feedback_updates
                ));
            }
        }

        if let Some(min_auto_replay_live_memory_feedback_updates) =
            gate.min_auto_replay_live_memory_feedback_updates
        {
            let auto_replay_live_memory_feedback_updates =
                self.total_auto_replay_live_memory_feedback_updates();
            if auto_replay_live_memory_feedback_updates
                < min_auto_replay_live_memory_feedback_updates
            {
                failures.push(format!(
                    "auto_replay_live_memory_feedback_updates {} below minimum {}",
                    auto_replay_live_memory_feedback_updates,
                    min_auto_replay_live_memory_feedback_updates
                ));
            }
        }

        if let Some(min_auto_replay_live_memory_feedback_detail_items) =
            gate.min_auto_replay_live_memory_feedback_detail_items
        {
            let auto_replay_live_memory_feedback_detail_items =
                self.total_auto_replay_live_memory_feedback_detail_items();
            if auto_replay_live_memory_feedback_detail_items
                < min_auto_replay_live_memory_feedback_detail_items
            {
                failures.push(format!(
                    "auto_replay_live_memory_feedback_detail_items {} below minimum {}",
                    auto_replay_live_memory_feedback_detail_items,
                    min_auto_replay_live_memory_feedback_detail_items
                ));
            }
        }

        if let Some(min_auto_replay_live_memory_feedback_applied) =
            gate.min_auto_replay_live_memory_feedback_applied
        {
            let auto_replay_live_memory_feedback_applied =
                self.total_auto_replay_live_memory_feedback_applied();
            if auto_replay_live_memory_feedback_applied
                < min_auto_replay_live_memory_feedback_applied
            {
                failures.push(format!(
                    "auto_replay_live_memory_feedback_applied {} below minimum {}",
                    auto_replay_live_memory_feedback_applied,
                    min_auto_replay_live_memory_feedback_applied
                ));
            }
        }

        if let Some(min_auto_replay_live_memory_feedback_strength_delta) =
            gate.min_auto_replay_live_memory_feedback_strength_delta
        {
            let auto_replay_live_memory_feedback_strength_delta =
                self.total_auto_replay_live_memory_feedback_strength_delta();
            if auto_replay_live_memory_feedback_strength_delta
                < min_auto_replay_live_memory_feedback_strength_delta
            {
                failures.push(format!(
                    "auto_replay_live_memory_feedback_strength_delta {:.6} below minimum {:.6}",
                    auto_replay_live_memory_feedback_strength_delta,
                    min_auto_replay_live_memory_feedback_strength_delta
                ));
            }
        }

        if let Some(min_auto_replay_recursive_items) = gate.min_auto_replay_recursive_items {
            let auto_replay_recursive_items = self.total_auto_replay_recursive_items();
            if auto_replay_recursive_items < min_auto_replay_recursive_items {
                failures.push(format!(
                    "auto_replay_recursive_items {} below minimum {}",
                    auto_replay_recursive_items, min_auto_replay_recursive_items
                ));
            }
        }

        if let Some(min_auto_replay_recursive_call_pressure) =
            gate.min_auto_replay_recursive_call_pressure
        {
            let auto_replay_recursive_call_pressure =
                self.max_auto_replay_recursive_call_pressure();
            if auto_replay_recursive_call_pressure < min_auto_replay_recursive_call_pressure {
                failures.push(format!(
                    "auto_replay_recursive_call_pressure {:.3} below minimum {:.3}",
                    auto_replay_recursive_call_pressure, min_auto_replay_recursive_call_pressure
                ));
            }
        }

        if let Some(max_auto_replay_recursive_call_pressure) =
            gate.max_auto_replay_recursive_call_pressure
        {
            let auto_replay_recursive_call_pressure =
                self.max_auto_replay_recursive_call_pressure();
            if auto_replay_recursive_call_pressure > max_auto_replay_recursive_call_pressure {
                failures.push(format!(
                    "auto_replay_recursive_call_pressure {:.3} above maximum {:.3}",
                    auto_replay_recursive_call_pressure, max_auto_replay_recursive_call_pressure
                ));
            }
        }

        if let Some(min_evolution_live_inference_runs) = gate.min_evolution_live_inference_runs {
            let observed = self.evolution_ledger.live_inference_runs;
            if observed < min_evolution_live_inference_runs {
                failures.push(format!(
                    "evolution_live_inference_runs {} below minimum {}",
                    observed, min_evolution_live_inference_runs
                ));
            }
        }

        if let Some(min_evolution_live_router_threshold_mutations) =
            gate.min_evolution_live_router_threshold_mutations
        {
            let observed = self.evolution_ledger.live_router_threshold_mutations;
            if observed < min_evolution_live_router_threshold_mutations {
                failures.push(format!(
                    "evolution_live_router_threshold_mutations {} below minimum {}",
                    observed, min_evolution_live_router_threshold_mutations
                ));
            }
        }

        if let Some(min_evolution_live_hierarchy_weight_mutations) =
            gate.min_evolution_live_hierarchy_weight_mutations
        {
            let observed = self.evolution_ledger.live_hierarchy_weight_mutations;
            if observed < min_evolution_live_hierarchy_weight_mutations {
                failures.push(format!(
                    "evolution_live_hierarchy_weight_mutations {} below minimum {}",
                    observed, min_evolution_live_hierarchy_weight_mutations
                ));
            }
        }

        if let Some(min_evolution_live_router_threshold_delta) =
            gate.min_evolution_live_router_threshold_delta
        {
            let observed = self.evolution_ledger.live_router_threshold_delta;
            if observed < min_evolution_live_router_threshold_delta {
                failures.push(format!(
                    "evolution_live_router_threshold_delta {:.6} below minimum {:.6}",
                    observed, min_evolution_live_router_threshold_delta
                ));
            }
        }

        if let Some(min_evolution_live_hierarchy_weight_delta) =
            gate.min_evolution_live_hierarchy_weight_delta
        {
            let observed = self.evolution_ledger.live_hierarchy_weight_delta;
            if observed < min_evolution_live_hierarchy_weight_delta {
                failures.push(format!(
                    "evolution_live_hierarchy_weight_delta {:.6} below minimum {:.6}",
                    observed, min_evolution_live_hierarchy_weight_delta
                ));
            }
        }

        if let Some(min_evolution_live_online_reward_feedbacks) =
            gate.min_evolution_live_online_reward_feedbacks
        {
            let observed = self.evolution_ledger.live_online_reward_feedbacks;
            if observed < min_evolution_live_online_reward_feedbacks {
                failures.push(format!(
                    "evolution_live_online_reward_feedbacks {} below minimum {}",
                    observed, min_evolution_live_online_reward_feedbacks
                ));
            }
        }

        if let Some(min_evolution_live_online_reward_reinforcements) =
            gate.min_evolution_live_online_reward_reinforcements
        {
            let observed = self.evolution_ledger.live_online_reward_reinforcements;
            if observed < min_evolution_live_online_reward_reinforcements {
                failures.push(format!(
                    "evolution_live_online_reward_reinforcements {} below minimum {}",
                    observed, min_evolution_live_online_reward_reinforcements
                ));
            }
        }

        if let Some(min_evolution_live_online_reward_penalties) =
            gate.min_evolution_live_online_reward_penalties
        {
            let observed = self.evolution_ledger.live_online_reward_penalties;
            if observed < min_evolution_live_online_reward_penalties {
                failures.push(format!(
                    "evolution_live_online_reward_penalties {} below minimum {}",
                    observed, min_evolution_live_online_reward_penalties
                ));
            }
        }

        if let Some(min_evolution_live_online_reward_strength) =
            gate.min_evolution_live_online_reward_strength
        {
            let observed = self.evolution_ledger.live_online_reward_strength;
            if observed < min_evolution_live_online_reward_strength {
                failures.push(format!(
                    "evolution_live_online_reward_strength {:.6} below minimum {:.6}",
                    observed, min_evolution_live_online_reward_strength
                ));
            }
        }

        if let Some(min_evolution_live_online_reward_reinforcement_strength) =
            gate.min_evolution_live_online_reward_reinforcement_strength
        {
            let observed = self
                .evolution_ledger
                .live_online_reward_reinforcement_strength;
            if observed < min_evolution_live_online_reward_reinforcement_strength {
                failures.push(format!(
                    "evolution_live_online_reward_reinforcement_strength {:.6} below minimum {:.6}",
                    observed, min_evolution_live_online_reward_reinforcement_strength
                ));
            }
        }

        if let Some(min_evolution_live_online_reward_penalty_strength) =
            gate.min_evolution_live_online_reward_penalty_strength
        {
            let observed = self.evolution_ledger.live_online_reward_penalty_strength;
            if observed < min_evolution_live_online_reward_penalty_strength {
                failures.push(format!(
                    "evolution_live_online_reward_penalty_strength {:.6} below minimum {:.6}",
                    observed, min_evolution_live_online_reward_penalty_strength
                ));
            }
        }

        if let Some(min_evolution_live_memory_updates) = gate.min_evolution_live_memory_updates {
            let observed = self.evolution_ledger.live_memory_updates();
            if observed < min_evolution_live_memory_updates {
                failures.push(format!(
                    "evolution_live_memory_updates {} below minimum {}",
                    observed, min_evolution_live_memory_updates
                ));
            }
        }

        if let Some(min_evolution_live_stored_memory_updates) =
            gate.min_evolution_live_stored_memory_updates
        {
            let observed = self.evolution_ledger.live_stored_memory_updates();
            if observed < min_evolution_live_stored_memory_updates {
                failures.push(format!(
                    "evolution_live_stored_memory_updates {} below minimum {}",
                    observed, min_evolution_live_stored_memory_updates
                ));
            }
        }

        if let Some(min_evolution_live_reflection_issues) =
            gate.min_evolution_live_reflection_issues
        {
            let observed = self.evolution_ledger.live_reflection_issues;
            if observed < min_evolution_live_reflection_issues {
                failures.push(format!(
                    "evolution_live_reflection_issues {} below minimum {}",
                    observed, min_evolution_live_reflection_issues
                ));
            }
        }

        if let Some(min_evolution_live_critical_reflection_issues) =
            gate.min_evolution_live_critical_reflection_issues
        {
            let observed = self.evolution_ledger.live_critical_reflection_issues;
            if observed < min_evolution_live_critical_reflection_issues {
                failures.push(format!(
                    "evolution_live_critical_reflection_issues {} below minimum {}",
                    observed, min_evolution_live_critical_reflection_issues
                ));
            }
        }

        if let Some(min_evolution_live_revision_actions) = gate.min_evolution_live_revision_actions
        {
            let observed = self.evolution_ledger.live_revision_actions;
            if observed < min_evolution_live_revision_actions {
                failures.push(format!(
                    "evolution_live_revision_actions {} below minimum {}",
                    observed, min_evolution_live_revision_actions
                ));
            }
        }

        if let Some(min_evolution_live_inference_device_profiles) =
            gate.min_evolution_live_inference_device_profiles
        {
            let observed = self.live_evolution_evidence.inference_device_profiles();
            if observed < min_evolution_live_inference_device_profiles {
                failures.push(format!(
                    "evolution_live_inference_device_profiles {} below minimum {}",
                    observed, min_evolution_live_inference_device_profiles
                ));
            }
        }

        if let Some(min_evolution_live_router_threshold_mutation_device_profiles) =
            gate.min_evolution_live_router_threshold_mutation_device_profiles
        {
            let observed = self
                .live_evolution_evidence
                .router_threshold_mutation_device_profiles();
            if observed < min_evolution_live_router_threshold_mutation_device_profiles {
                failures.push(format!(
                    "evolution_live_router_threshold_mutation_device_profiles {} below minimum {}",
                    observed, min_evolution_live_router_threshold_mutation_device_profiles
                ));
            }
        }

        if let Some(min_evolution_live_hierarchy_weight_mutation_device_profiles) =
            gate.min_evolution_live_hierarchy_weight_mutation_device_profiles
        {
            let observed = self
                .live_evolution_evidence
                .hierarchy_weight_mutation_device_profiles();
            if observed < min_evolution_live_hierarchy_weight_mutation_device_profiles {
                failures.push(format!(
                    "evolution_live_hierarchy_weight_mutation_device_profiles {} below minimum {}",
                    observed, min_evolution_live_hierarchy_weight_mutation_device_profiles
                ));
            }
        }

        if let Some(min_evolution_live_online_reward_device_profiles) =
            gate.min_evolution_live_online_reward_device_profiles
        {
            let observed = self.live_evolution_evidence.online_reward_device_profiles();
            if observed < min_evolution_live_online_reward_device_profiles {
                failures.push(format!(
                    "evolution_live_online_reward_device_profiles {} below minimum {}",
                    observed, min_evolution_live_online_reward_device_profiles
                ));
            }
        }

        if let Some(min_evolution_live_online_reward_strength_device_profiles) =
            gate.min_evolution_live_online_reward_strength_device_profiles
        {
            let observed = self
                .live_evolution_evidence
                .online_reward_strength_device_profiles();
            if observed < min_evolution_live_online_reward_strength_device_profiles {
                failures.push(format!(
                    "evolution_live_online_reward_strength_device_profiles {} below minimum {}",
                    observed, min_evolution_live_online_reward_strength_device_profiles
                ));
            }
        }

        if let Some(min_evolution_live_memory_update_device_profiles) =
            gate.min_evolution_live_memory_update_device_profiles
        {
            let observed = self.live_evolution_evidence.memory_update_device_profiles();
            if observed < min_evolution_live_memory_update_device_profiles {
                failures.push(format!(
                    "evolution_live_memory_update_device_profiles {} below minimum {}",
                    observed, min_evolution_live_memory_update_device_profiles
                ));
            }
        }

        if let Some(min_evolution_live_stored_memory_update_device_profiles) =
            gate.min_evolution_live_stored_memory_update_device_profiles
        {
            let observed = self
                .live_evolution_evidence
                .stored_memory_update_device_profiles();
            if observed < min_evolution_live_stored_memory_update_device_profiles {
                failures.push(format!(
                    "evolution_live_stored_memory_update_device_profiles {} below minimum {}",
                    observed, min_evolution_live_stored_memory_update_device_profiles
                ));
            }
        }

        if let Some(min_evolution_live_reflection_issue_device_profiles) =
            gate.min_evolution_live_reflection_issue_device_profiles
        {
            let observed = self
                .live_evolution_evidence
                .reflection_issue_device_profiles();
            if observed < min_evolution_live_reflection_issue_device_profiles {
                failures.push(format!(
                    "evolution_live_reflection_issue_device_profiles {} below minimum {}",
                    observed, min_evolution_live_reflection_issue_device_profiles
                ));
            }
        }

        if let Some(min_evolution_live_critical_reflection_issue_device_profiles) =
            gate.min_evolution_live_critical_reflection_issue_device_profiles
        {
            let observed = self
                .live_evolution_evidence
                .critical_reflection_issue_device_profiles();
            if observed < min_evolution_live_critical_reflection_issue_device_profiles {
                failures.push(format!(
                    "evolution_live_critical_reflection_issue_device_profiles {} below minimum {}",
                    observed, min_evolution_live_critical_reflection_issue_device_profiles
                ));
            }
        }

        if let Some(min_evolution_live_revision_action_device_profiles) =
            gate.min_evolution_live_revision_action_device_profiles
        {
            let observed = self
                .live_evolution_evidence
                .revision_action_device_profiles();
            if observed < min_evolution_live_revision_action_device_profiles {
                failures.push(format!(
                    "evolution_live_revision_action_device_profiles {} below minimum {}",
                    observed, min_evolution_live_revision_action_device_profiles
                ));
            }
        }

        if let Some(min_evolution_replay_runs) = gate.min_evolution_replay_runs {
            let observed = self.evolution_ledger.replay_runs;
            if observed < min_evolution_replay_runs {
                failures.push(format!(
                    "evolution_replay_runs {} below minimum {}",
                    observed, min_evolution_replay_runs
                ));
            }
        }

        if let Some(min_evolution_replay_items) = gate.min_evolution_replay_items {
            let observed = self.evolution_ledger.replay_items;
            if observed < min_evolution_replay_items {
                failures.push(format!(
                    "evolution_replay_items {} below minimum {}",
                    observed, min_evolution_replay_items
                ));
            }
        }

        if let Some(min_evolution_router_threshold_mutations) =
            gate.min_evolution_router_threshold_mutations
        {
            let observed = self.evolution_ledger.router_threshold_mutations;
            if observed < min_evolution_router_threshold_mutations {
                failures.push(format!(
                    "evolution_router_threshold_mutations {} below minimum {}",
                    observed, min_evolution_router_threshold_mutations
                ));
            }
        }

        if let Some(min_evolution_hierarchy_weight_mutations) =
            gate.min_evolution_hierarchy_weight_mutations
        {
            let observed = self.evolution_ledger.hierarchy_weight_mutations;
            if observed < min_evolution_hierarchy_weight_mutations {
                failures.push(format!(
                    "evolution_hierarchy_weight_mutations {} below minimum {}",
                    observed, min_evolution_hierarchy_weight_mutations
                ));
            }
        }

        if let Some(min_evolution_router_threshold_delta) =
            gate.min_evolution_router_threshold_delta
        {
            let observed = self.evolution_ledger.router_threshold_delta;
            if observed < min_evolution_router_threshold_delta {
                failures.push(format!(
                    "evolution_router_threshold_delta {:.6} below minimum {:.6}",
                    observed, min_evolution_router_threshold_delta
                ));
            }
        }

        if let Some(min_evolution_hierarchy_weight_delta) =
            gate.min_evolution_hierarchy_weight_delta
        {
            let observed = self.evolution_ledger.hierarchy_weight_delta;
            if observed < min_evolution_hierarchy_weight_delta {
                failures.push(format!(
                    "evolution_hierarchy_weight_delta {:.6} below minimum {:.6}",
                    observed, min_evolution_hierarchy_weight_delta
                ));
            }
        }

        if let Some(min_evolution_memory_updates) = gate.min_evolution_memory_updates {
            let observed = self.evolution_ledger.memory_updates();
            if observed < min_evolution_memory_updates {
                failures.push(format!(
                    "evolution_memory_updates {} below minimum {}",
                    observed, min_evolution_memory_updates
                ));
            }
        }

        if let Some(min_evolution_replay_live_memory_feedback_updates) =
            gate.min_evolution_replay_live_memory_feedback_updates
        {
            let observed = self.evolution_ledger.replay_live_memory_feedback_updates();
            if observed < min_evolution_replay_live_memory_feedback_updates {
                failures.push(format!(
                    "evolution_replay_live_memory_feedback_updates {} below minimum {}",
                    observed, min_evolution_replay_live_memory_feedback_updates
                ));
            }
        }

        if let Some(min_evolution_replay_live_memory_feedback_detail_items) =
            gate.min_evolution_replay_live_memory_feedback_detail_items
        {
            let observed = self
                .evolution_ledger
                .replay_live_memory_feedback_detail_items;
            if observed < min_evolution_replay_live_memory_feedback_detail_items {
                failures.push(format!(
                    "evolution_replay_live_memory_feedback_detail_items {} below minimum {}",
                    observed, min_evolution_replay_live_memory_feedback_detail_items
                ));
            }
        }

        if let Some(min_evolution_replay_live_memory_feedback_applied) =
            gate.min_evolution_replay_live_memory_feedback_applied
        {
            let observed = self.evolution_ledger.replay_live_memory_feedback_applied;
            if observed < min_evolution_replay_live_memory_feedback_applied {
                failures.push(format!(
                    "evolution_replay_live_memory_feedback_applied {} below minimum {}",
                    observed, min_evolution_replay_live_memory_feedback_applied
                ));
            }
        }

        if let Some(min_evolution_replay_live_memory_feedback_strength_delta) =
            gate.min_evolution_replay_live_memory_feedback_strength_delta
        {
            let observed = self
                .evolution_ledger
                .replay_live_memory_feedback_strength_delta;
            if observed < min_evolution_replay_live_memory_feedback_strength_delta {
                failures.push(format!(
                    "evolution_replay_live_memory_feedback_strength_delta {:.6} below minimum {:.6}",
                    observed, min_evolution_replay_live_memory_feedback_strength_delta
                ));
            }
        }

        if let Some(min_evolution_replay_live_evolution_items) =
            gate.min_evolution_replay_live_evolution_items
        {
            let observed = self.evolution_ledger.replay_live_evolution_items;
            if observed < min_evolution_replay_live_evolution_items {
                failures.push(format!(
                    "evolution_replay_live_evolution_items {} below minimum {}",
                    observed, min_evolution_replay_live_evolution_items
                ));
            }
        }

        if let Some(min_evolution_replay_live_evolution_online_reward_feedbacks) =
            gate.min_evolution_replay_live_evolution_online_reward_feedbacks
        {
            let observed = self
                .evolution_ledger
                .replay_live_evolution_online_reward_feedbacks;
            if observed < min_evolution_replay_live_evolution_online_reward_feedbacks {
                failures.push(format!(
                    "evolution_replay_live_evolution_online_reward_feedbacks {} below minimum {}",
                    observed, min_evolution_replay_live_evolution_online_reward_feedbacks
                ));
            }
        }

        if let Some(min_evolution_replay_live_evolution_online_reward_reinforcements) =
            gate.min_evolution_replay_live_evolution_online_reward_reinforcements
        {
            let observed = self
                .evolution_ledger
                .replay_live_evolution_online_reward_reinforcements;
            if observed < min_evolution_replay_live_evolution_online_reward_reinforcements {
                failures.push(format!(
                    "evolution_replay_live_evolution_online_reward_reinforcements {} below minimum {}",
                    observed, min_evolution_replay_live_evolution_online_reward_reinforcements
                ));
            }
        }

        if let Some(min_evolution_replay_live_evolution_online_reward_penalties) =
            gate.min_evolution_replay_live_evolution_online_reward_penalties
        {
            let observed = self
                .evolution_ledger
                .replay_live_evolution_online_reward_penalties;
            if observed < min_evolution_replay_live_evolution_online_reward_penalties {
                failures.push(format!(
                    "evolution_replay_live_evolution_online_reward_penalties {} below minimum {}",
                    observed, min_evolution_replay_live_evolution_online_reward_penalties
                ));
            }
        }

        if let Some(min_evolution_replay_live_evolution_online_reward_strength) =
            gate.min_evolution_replay_live_evolution_online_reward_strength
        {
            let observed = self
                .evolution_ledger
                .replay_live_evolution_online_reward_strength;
            if observed < min_evolution_replay_live_evolution_online_reward_strength {
                failures.push(format!(
                    "evolution_replay_live_evolution_online_reward_strength {:.6} below minimum {:.6}",
                    observed, min_evolution_replay_live_evolution_online_reward_strength
                ));
            }
        }

        if let Some(min_evolution_replay_live_evolution_online_reward_reinforcement_strength) =
            gate.min_evolution_replay_live_evolution_online_reward_reinforcement_strength
        {
            let observed = self
                .evolution_ledger
                .replay_live_evolution_online_reward_reinforcement_strength;
            if observed < min_evolution_replay_live_evolution_online_reward_reinforcement_strength {
                failures.push(format!(
                    "evolution_replay_live_evolution_online_reward_reinforcement_strength {:.6} below minimum {:.6}",
                    observed,
                    min_evolution_replay_live_evolution_online_reward_reinforcement_strength
                ));
            }
        }

        if let Some(min_evolution_replay_live_evolution_online_reward_penalty_strength) =
            gate.min_evolution_replay_live_evolution_online_reward_penalty_strength
        {
            let observed = self
                .evolution_ledger
                .replay_live_evolution_online_reward_penalty_strength;
            if observed < min_evolution_replay_live_evolution_online_reward_penalty_strength {
                failures.push(format!(
                    "evolution_replay_live_evolution_online_reward_penalty_strength {:.6} below minimum {:.6}",
                    observed, min_evolution_replay_live_evolution_online_reward_penalty_strength
                ));
            }
        }

        if let Some(min_evolution_replay_live_evolution_memory_updates) =
            gate.min_evolution_replay_live_evolution_memory_updates
        {
            let observed = self.evolution_ledger.replay_live_evolution_memory_updates;
            if observed < min_evolution_replay_live_evolution_memory_updates {
                failures.push(format!(
                    "evolution_replay_live_evolution_memory_updates {} below minimum {}",
                    observed, min_evolution_replay_live_evolution_memory_updates
                ));
            }
        }

        if let Some(min_evolution_replay_live_evolution_stored_memory_updates) =
            gate.min_evolution_replay_live_evolution_stored_memory_updates
        {
            let observed = self
                .evolution_ledger
                .replay_live_evolution_stored_memory_updates;
            if observed < min_evolution_replay_live_evolution_stored_memory_updates {
                failures.push(format!(
                    "evolution_replay_live_evolution_stored_memory_updates {} below minimum {}",
                    observed, min_evolution_replay_live_evolution_stored_memory_updates
                ));
            }
        }

        if let Some(min_evolution_replay_live_evolution_reflection_issues) =
            gate.min_evolution_replay_live_evolution_reflection_issues
        {
            let observed = self
                .evolution_ledger
                .replay_live_evolution_reflection_issues;
            if observed < min_evolution_replay_live_evolution_reflection_issues {
                failures.push(format!(
                    "evolution_replay_live_evolution_reflection_issues {} below minimum {}",
                    observed, min_evolution_replay_live_evolution_reflection_issues
                ));
            }
        }

        if let Some(min_evolution_replay_live_evolution_critical_reflection_issues) =
            gate.min_evolution_replay_live_evolution_critical_reflection_issues
        {
            let observed = self
                .evolution_ledger
                .replay_live_evolution_critical_reflection_issues;
            if observed < min_evolution_replay_live_evolution_critical_reflection_issues {
                failures.push(format!(
                    "evolution_replay_live_evolution_critical_reflection_issues {} below minimum {}",
                    observed, min_evolution_replay_live_evolution_critical_reflection_issues
                ));
            }
        }

        if let Some(min_evolution_replay_live_evolution_revision_actions) =
            gate.min_evolution_replay_live_evolution_revision_actions
        {
            let observed = self.evolution_ledger.replay_live_evolution_revision_actions;
            if observed < min_evolution_replay_live_evolution_revision_actions {
                failures.push(format!(
                    "evolution_replay_live_evolution_revision_actions {} below minimum {}",
                    observed, min_evolution_replay_live_evolution_revision_actions
                ));
            }
        }

        if let Some(min_evolution_replay_live_evolution_device_profiles) =
            gate.min_evolution_replay_live_evolution_device_profiles
        {
            let observed = self
                .live_evolution_evidence
                .replay_live_evolution_device_profiles();
            if observed < min_evolution_replay_live_evolution_device_profiles {
                failures.push(format!(
                    "evolution_replay_live_evolution_device_profiles {} below minimum {}",
                    observed, min_evolution_replay_live_evolution_device_profiles
                ));
            }
        }

        if let Some(min_evolution_replay_live_evolution_online_reward_device_profiles) =
            gate.min_evolution_replay_live_evolution_online_reward_device_profiles
        {
            let observed = self
                .live_evolution_evidence
                .replay_live_evolution_online_reward_device_profiles();
            if observed < min_evolution_replay_live_evolution_online_reward_device_profiles {
                failures.push(format!(
                    "evolution_replay_live_evolution_online_reward_device_profiles {} below minimum {}",
                    observed,
                    min_evolution_replay_live_evolution_online_reward_device_profiles
                ));
            }
        }

        if let Some(min_evolution_replay_live_evolution_online_reward_strength_device_profiles) =
            gate.min_evolution_replay_live_evolution_online_reward_strength_device_profiles
        {
            let observed = self
                .live_evolution_evidence
                .replay_live_evolution_online_reward_strength_device_profiles();
            if observed < min_evolution_replay_live_evolution_online_reward_strength_device_profiles
            {
                failures.push(format!(
                    "evolution_replay_live_evolution_online_reward_strength_device_profiles {} below minimum {}",
                    observed,
                    min_evolution_replay_live_evolution_online_reward_strength_device_profiles
                ));
            }
        }

        if let Some(min_evolution_replay_live_evolution_memory_update_device_profiles) =
            gate.min_evolution_replay_live_evolution_memory_update_device_profiles
        {
            let observed = self
                .live_evolution_evidence
                .replay_live_evolution_memory_update_device_profiles();
            if observed < min_evolution_replay_live_evolution_memory_update_device_profiles {
                failures.push(format!(
                    "evolution_replay_live_evolution_memory_update_device_profiles {} below minimum {}",
                    observed, min_evolution_replay_live_evolution_memory_update_device_profiles
                ));
            }
        }

        if let Some(min_evolution_replay_live_evolution_critical_reflection_issue_device_profiles) =
            gate.min_evolution_replay_live_evolution_critical_reflection_issue_device_profiles
        {
            let observed = self
                .live_evolution_evidence
                .replay_live_evolution_critical_reflection_issue_device_profiles();
            if observed
                < min_evolution_replay_live_evolution_critical_reflection_issue_device_profiles
            {
                failures.push(format!(
                    "evolution_replay_live_evolution_critical_reflection_issue_device_profiles {} below minimum {}",
                    observed,
                    min_evolution_replay_live_evolution_critical_reflection_issue_device_profiles
                ));
            }
        }

        if let Some(min_evolution_replay_live_evolution_revision_action_device_profiles) =
            gate.min_evolution_replay_live_evolution_revision_action_device_profiles
        {
            let observed = self
                .live_evolution_evidence
                .replay_live_evolution_revision_action_device_profiles();
            if observed < min_evolution_replay_live_evolution_revision_action_device_profiles {
                failures.push(format!(
                    "evolution_replay_live_evolution_revision_action_device_profiles {} below minimum {}",
                    observed,
                    min_evolution_replay_live_evolution_revision_action_device_profiles
                ));
            }
        }

        if let Some(min_evolution_recursive_replay_items) =
            gate.min_evolution_recursive_replay_items
        {
            let observed = self.evolution_ledger.recursive_replay_items;
            if observed < min_evolution_recursive_replay_items {
                failures.push(format!(
                    "evolution_recursive_replay_items {} below minimum {}",
                    observed, min_evolution_recursive_replay_items
                ));
            }
        }

        if let Some(min_evolution_recursive_runtime_calls) =
            gate.min_evolution_recursive_runtime_calls
        {
            let observed = self.evolution_ledger.recursive_runtime_calls;
            if observed < min_evolution_recursive_runtime_calls {
                failures.push(format!(
                    "evolution_recursive_runtime_calls {} below minimum {}",
                    observed, min_evolution_recursive_runtime_calls
                ));
            }
        }

        if let Some(max_evolution_drift_rollbacks) = gate.max_evolution_drift_rollbacks {
            let observed = self.evolution_ledger.drift_rollbacks;
            if observed > max_evolution_drift_rollbacks {
                failures.push(format!(
                    "evolution_drift_rollbacks {} above maximum {}",
                    observed, max_evolution_drift_rollbacks
                ));
            }
        }

        if let Some(max_evolution_rollback_router_threshold_delta) =
            gate.max_evolution_rollback_router_threshold_delta
        {
            let observed = self.evolution_ledger.rollback_router_threshold_delta;
            if observed > max_evolution_rollback_router_threshold_delta {
                failures.push(format!(
                    "evolution_rollback_router_threshold_delta {:.6} above maximum {:.6}",
                    observed, max_evolution_rollback_router_threshold_delta
                ));
            }
        }

        if let Some(max_evolution_rollback_hierarchy_weight_delta) =
            gate.max_evolution_rollback_hierarchy_weight_delta
        {
            let observed = self.evolution_ledger.rollback_hierarchy_weight_delta;
            if observed > max_evolution_rollback_hierarchy_weight_delta {
                failures.push(format!(
                    "evolution_rollback_hierarchy_weight_delta {:.6} above maximum {:.6}",
                    observed, max_evolution_rollback_hierarchy_weight_delta
                ));
            }
        }

        if let Some(min_sparse_skipped_cases) = gate.min_sparse_skipped_cases {
            let sparse_skipped_cases = self.sparse_skipped_cases();
            if sparse_skipped_cases < min_sparse_skipped_cases {
                failures.push(format!(
                    "sparse_skipped_cases {} below minimum {}",
                    sparse_skipped_cases, min_sparse_skipped_cases
                ));
            }
        }

        if let Some(min_sparse_skipped_tokens) = gate.min_sparse_skipped_tokens {
            let sparse_skipped_tokens = self.total_sparse_skipped_tokens();
            if sparse_skipped_tokens < min_sparse_skipped_tokens {
                failures.push(format!(
                    "sparse_skipped_tokens {} below minimum {}",
                    sparse_skipped_tokens, min_sparse_skipped_tokens
                ));
            }
        }

        if let Some(min_runtime_forward_cases) = gate.min_runtime_forward_cases {
            let runtime_forward_cases = self.runtime_forward_cases();
            if runtime_forward_cases < min_runtime_forward_cases {
                failures.push(format!(
                    "runtime_forward_cases {} below minimum {}",
                    runtime_forward_cases, min_runtime_forward_cases
                ));
            }
        }

        if let Some(min_runtime_forward_energy_cases) = gate.min_runtime_forward_energy_cases {
            let runtime_forward_energy_cases = self.runtime_forward_energy_cases();
            if runtime_forward_energy_cases < min_runtime_forward_energy_cases {
                failures.push(format!(
                    "runtime_forward_energy_cases {} below minimum {}",
                    runtime_forward_energy_cases, min_runtime_forward_energy_cases
                ));
            }
        }

        if let Some(min_runtime_kv_influence_cases) = gate.min_runtime_kv_influence_cases {
            let runtime_kv_influence_cases = self.runtime_kv_influence_cases();
            if runtime_kv_influence_cases < min_runtime_kv_influence_cases {
                failures.push(format!(
                    "runtime_kv_influence_cases {} below minimum {}",
                    runtime_kv_influence_cases, min_runtime_kv_influence_cases
                ));
            }
        }

        if let Some(min_runtime_kv_precision_cases) = gate.min_runtime_kv_precision_cases {
            let runtime_kv_precision_cases = self.runtime_kv_precision_cases();
            if runtime_kv_precision_cases < min_runtime_kv_precision_cases {
                failures.push(format!(
                    "runtime_kv_precision_cases {} below minimum {}",
                    runtime_kv_precision_cases, min_runtime_kv_precision_cases
                ));
            }
        }

        if let Some(min_runtime_layer_mode_cases) = gate.min_runtime_layer_mode_cases {
            let runtime_layer_mode_cases = self.runtime_layer_mode_cases();
            if runtime_layer_mode_cases < min_runtime_layer_mode_cases {
                failures.push(format!(
                    "runtime_layer_mode_cases {} below minimum {}",
                    runtime_layer_mode_cases, min_runtime_layer_mode_cases
                ));
            }
        }

        if let Some(min_runtime_all_layer_mode_cases) = gate.min_runtime_all_layer_mode_cases {
            let runtime_all_layer_mode_cases = self.runtime_all_layer_mode_cases();
            if runtime_all_layer_mode_cases < min_runtime_all_layer_mode_cases {
                failures.push(format!(
                    "runtime_all_layer_mode_cases {} below minimum {}",
                    runtime_all_layer_mode_cases, min_runtime_all_layer_mode_cases
                ));
            }
        }

        if let Some(min_runtime_global_layers) = gate.min_runtime_global_layers {
            let runtime_global_layers = self.total_runtime_global_layers();
            if runtime_global_layers < min_runtime_global_layers {
                failures.push(format!(
                    "runtime_global_layers {} below minimum {}",
                    runtime_global_layers, min_runtime_global_layers
                ));
            }
        }

        if let Some(min_runtime_local_window_layers) = gate.min_runtime_local_window_layers {
            let runtime_local_window_layers = self.total_runtime_local_window_layers();
            if runtime_local_window_layers < min_runtime_local_window_layers {
                failures.push(format!(
                    "runtime_local_window_layers {} below minimum {}",
                    runtime_local_window_layers, min_runtime_local_window_layers
                ));
            }
        }

        if let Some(min_runtime_convolutional_fusion_layers) =
            gate.min_runtime_convolutional_fusion_layers
        {
            let runtime_convolutional_fusion_layers =
                self.total_runtime_convolutional_fusion_layers();
            if runtime_convolutional_fusion_layers < min_runtime_convolutional_fusion_layers {
                failures.push(format!(
                    "runtime_convolutional_fusion_layers {} below minimum {}",
                    runtime_convolutional_fusion_layers, min_runtime_convolutional_fusion_layers
                ));
            }
        }

        if let Some(min_runtime_uncertainty_cases) = gate.min_runtime_uncertainty_cases {
            let runtime_uncertainty_cases = self.runtime_uncertainty_cases();
            if runtime_uncertainty_cases < min_runtime_uncertainty_cases {
                failures.push(format!(
                    "runtime_uncertainty_cases {} below minimum {}",
                    runtime_uncertainty_cases, min_runtime_uncertainty_cases
                ));
            }
        }

        if let Some(min_runtime_uncertainty_tokens) = gate.min_runtime_uncertainty_tokens {
            let runtime_uncertainty_tokens = self.total_runtime_uncertainty_tokens();
            if runtime_uncertainty_tokens < min_runtime_uncertainty_tokens {
                failures.push(format!(
                    "runtime_uncertainty_tokens {} below minimum {}",
                    runtime_uncertainty_tokens, min_runtime_uncertainty_tokens
                ));
            }
        }

        if let Some(min_runtime_uncertainty_device_profiles) =
            gate.min_runtime_uncertainty_device_profiles
        {
            let runtime_uncertainty_device_profiles = self.runtime_uncertainty_device_profiles();
            if runtime_uncertainty_device_profiles < min_runtime_uncertainty_device_profiles {
                failures.push(format!(
                    "runtime_uncertainty_device_profiles {} below minimum {} devices={}",
                    runtime_uncertainty_device_profiles,
                    min_runtime_uncertainty_device_profiles,
                    self.runtime_uncertainty_devices_csv()
                ));
            }
        }

        if let Some(min_runtime_uncertainty_token_device_profiles) =
            gate.min_runtime_uncertainty_token_device_profiles
        {
            let runtime_uncertainty_token_device_profiles =
                self.runtime_uncertainty_token_device_profiles();
            if runtime_uncertainty_token_device_profiles
                < min_runtime_uncertainty_token_device_profiles
            {
                failures.push(format!(
                    "runtime_uncertainty_token_device_profiles {} below minimum {} devices={}",
                    runtime_uncertainty_token_device_profiles,
                    min_runtime_uncertainty_token_device_profiles,
                    self.runtime_uncertainty_token_devices_csv()
                ));
            }
        }

        if let Some(min_runtime_kv_import_cases) = gate.min_runtime_kv_import_cases {
            let runtime_kv_import_cases = self.runtime_kv_import_cases();
            if runtime_kv_import_cases < min_runtime_kv_import_cases {
                failures.push(format!(
                    "runtime_kv_import_cases {} below minimum {}",
                    runtime_kv_import_cases, min_runtime_kv_import_cases
                ));
            }
        }

        if let Some(min_runtime_kv_imported) = gate.min_runtime_kv_imported {
            let runtime_kv_imported = self.total_runtime_kv_imported();
            if runtime_kv_imported < min_runtime_kv_imported {
                failures.push(format!(
                    "runtime_kv_imported {} below minimum {}",
                    runtime_kv_imported, min_runtime_kv_imported
                ));
            }
        }

        if let Some(min_runtime_kv_import_device_profiles) =
            gate.min_runtime_kv_import_device_profiles
        {
            let runtime_kv_import_device_profiles = self.runtime_kv_import_device_profiles();
            if runtime_kv_import_device_profiles < min_runtime_kv_import_device_profiles {
                failures.push(format!(
                    "runtime_kv_import_device_profiles {} below minimum {} devices={}",
                    runtime_kv_import_device_profiles,
                    min_runtime_kv_import_device_profiles,
                    self.runtime_kv_import_devices_csv()
                ));
            }
        }

        if let Some(min_runtime_kv_exported) = gate.min_runtime_kv_exported {
            let runtime_kv_exported = self.total_runtime_kv_exported();
            if runtime_kv_exported < min_runtime_kv_exported {
                failures.push(format!(
                    "runtime_kv_exported {} below minimum {}",
                    runtime_kv_exported, min_runtime_kv_exported
                ));
            }
        }

        if let Some(min_runtime_kv_export_device_profiles) =
            gate.min_runtime_kv_export_device_profiles
        {
            let runtime_kv_export_device_profiles = self.runtime_kv_export_device_profiles();
            if runtime_kv_export_device_profiles < min_runtime_kv_export_device_profiles {
                failures.push(format!(
                    "runtime_kv_export_device_profiles {} below minimum {} devices={}",
                    runtime_kv_export_device_profiles,
                    min_runtime_kv_export_device_profiles,
                    self.runtime_kv_export_devices_csv()
                ));
            }
        }

        if let Some(min_runtime_kv_stored) = gate.min_runtime_kv_stored {
            let runtime_kv_stored = self.total_runtime_kv_stored();
            if runtime_kv_stored < min_runtime_kv_stored {
                failures.push(format!(
                    "runtime_kv_stored {} below minimum {}",
                    runtime_kv_stored, min_runtime_kv_stored
                ));
            }
        }

        if let Some(min_runtime_kv_stored_device_profiles) =
            gate.min_runtime_kv_stored_device_profiles
        {
            let runtime_kv_stored_device_profiles = self.runtime_kv_stored_device_profiles();
            if runtime_kv_stored_device_profiles < min_runtime_kv_stored_device_profiles {
                failures.push(format!(
                    "runtime_kv_stored_device_profiles {} below minimum {} devices={}",
                    runtime_kv_stored_device_profiles,
                    min_runtime_kv_stored_device_profiles,
                    self.runtime_kv_stored_devices_csv()
                ));
            }
        }

        if let Some(min_runtime_kv_hold_cases) = gate.min_runtime_kv_hold_cases {
            let runtime_kv_hold_cases = self.runtime_kv_hold_cases();
            if runtime_kv_hold_cases < min_runtime_kv_hold_cases {
                failures.push(format!(
                    "runtime_kv_hold_cases {} below minimum {}",
                    runtime_kv_hold_cases, min_runtime_kv_hold_cases
                ));
            }
        }

        if let Some(min_runtime_kv_held) = gate.min_runtime_kv_held {
            let runtime_kv_held = self.total_runtime_kv_held();
            if runtime_kv_held < min_runtime_kv_held {
                failures.push(format!(
                    "runtime_kv_held {} below minimum {}",
                    runtime_kv_held, min_runtime_kv_held
                ));
            }
        }

        if let Some(min_runtime_kv_hold_device_profiles) = gate.min_runtime_kv_hold_device_profiles
        {
            let runtime_kv_hold_device_profiles = self.runtime_kv_hold_device_profiles();
            if runtime_kv_hold_device_profiles < min_runtime_kv_hold_device_profiles {
                failures.push(format!(
                    "runtime_kv_hold_device_profiles {} below minimum {} devices={}",
                    runtime_kv_hold_device_profiles,
                    min_runtime_kv_hold_device_profiles,
                    self.runtime_kv_hold_devices_csv()
                ));
            }
        }

        if let Some(min_runtime_adapter_contract_cases) = gate.min_runtime_adapter_contract_cases {
            let runtime_adapter_contract_cases = self.runtime_adapter_contract_cases();
            if runtime_adapter_contract_cases < min_runtime_adapter_contract_cases {
                failures.push(format!(
                    "runtime_adapter_contract_cases {} below minimum {}",
                    runtime_adapter_contract_cases, min_runtime_adapter_contract_cases
                ));
            }
        }

        if let Some(min_runtime_adapter_kinds) = gate.min_runtime_adapter_kinds {
            let runtime_adapter_kinds = self.runtime_adapter_kinds();
            if runtime_adapter_kinds < min_runtime_adapter_kinds {
                failures.push(format!(
                    "runtime_adapter_kinds {} below minimum {}",
                    runtime_adapter_kinds, min_runtime_adapter_kinds
                ));
            }
        }

        if let Some(min_runtime_adapter_observations) = gate.min_runtime_adapter_observations {
            let runtime_adapter_observations = self.total_runtime_adapter_observations();
            if runtime_adapter_observations < min_runtime_adapter_observations {
                failures.push(format!(
                    "runtime_adapter_observations {} below minimum {}",
                    runtime_adapter_observations, min_runtime_adapter_observations
                ));
            }
        }

        if let Some(min_runtime_adapter_best_score) = gate.min_runtime_adapter_best_score {
            let runtime_adapter_best_score = self.max_runtime_adapter_score().unwrap_or(0.0);
            if runtime_adapter_best_score < min_runtime_adapter_best_score {
                failures.push(format!(
                    "runtime_adapter_best_score {:.3} below minimum {:.3}",
                    runtime_adapter_best_score, min_runtime_adapter_best_score
                ));
            }
        }

        if let Some(max_runtime_adapter_contract_violations) =
            gate.max_runtime_adapter_contract_violations
        {
            let runtime_adapter_contract_violations =
                self.total_runtime_adapter_contract_violations();
            if runtime_adapter_contract_violations > max_runtime_adapter_contract_violations {
                failures.push(format!(
                    "runtime_adapter_contract_violations {} above maximum {}",
                    runtime_adapter_contract_violations, max_runtime_adapter_contract_violations
                ));
            }
        }

        if let Some(max_runtime_adapter_selection_mismatches) =
            gate.max_runtime_adapter_selection_mismatches
        {
            let runtime_adapter_selection_mismatches =
                self.total_runtime_adapter_selection_mismatches();
            if runtime_adapter_selection_mismatches > max_runtime_adapter_selection_mismatches {
                failures.push(format!(
                    "runtime_adapter_selection_mismatches {} above maximum {}",
                    runtime_adapter_selection_mismatches, max_runtime_adapter_selection_mismatches
                ));
            }
        }

        if let Some(min_runtime_embedding_cases) = gate.min_runtime_embedding_cases {
            let runtime_embedding_cases = self.runtime_embedding_cases();
            if runtime_embedding_cases < min_runtime_embedding_cases {
                failures.push(format!(
                    "runtime_embedding_cases {} below minimum {}",
                    runtime_embedding_cases, min_runtime_embedding_cases
                ));
            }
        }

        if let Some(min_runtime_embedding_device_profiles) =
            gate.min_runtime_embedding_device_profiles
        {
            let runtime_embedding_device_profiles = self.runtime_embedding_device_profiles();
            if runtime_embedding_device_profiles < min_runtime_embedding_device_profiles {
                failures.push(format!(
                    "runtime_embedding_device_profiles {} below minimum {}",
                    runtime_embedding_device_profiles, min_runtime_embedding_device_profiles
                ));
            }
        }

        if let Some(max_embedding_fallback_cases) = gate.max_embedding_fallback_cases {
            let embedding_fallback_cases = self.embedding_fallback_cases();
            if embedding_fallback_cases > max_embedding_fallback_cases {
                failures.push(format!(
                    "embedding_fallback_cases {} above maximum {}",
                    embedding_fallback_cases, max_embedding_fallback_cases
                ));
            }
        }

        if let Some(max_embedding_evidence_failures) = gate.max_embedding_evidence_failures {
            let embedding_evidence_failures = self.total_embedding_evidence_failures();
            if embedding_evidence_failures > max_embedding_evidence_failures {
                failures.push(format!(
                    "embedding_evidence_failures {} above maximum {}: {}",
                    embedding_evidence_failures,
                    max_embedding_evidence_failures,
                    self.embedding_evidence.failures.join("; ")
                ));
            }
        }

        if let Some(min_runtime_device_execution_cases) = gate.min_runtime_device_execution_cases {
            let runtime_device_execution_matched_cases =
                self.runtime_device_execution_matched_cases();
            if runtime_device_execution_matched_cases < min_runtime_device_execution_cases {
                failures.push(format!(
                    "runtime_device_execution_matched_cases {} below minimum {}",
                    runtime_device_execution_matched_cases, min_runtime_device_execution_cases
                ));
            }
        }

        if let Some(min_runtime_device_execution_device_profiles) =
            gate.min_runtime_device_execution_device_profiles
        {
            let runtime_device_execution_device_profiles =
                self.runtime_device_execution_device_profiles();
            if runtime_device_execution_device_profiles
                < min_runtime_device_execution_device_profiles
            {
                failures.push(format!(
                    "runtime_device_execution_device_profiles {} below minimum {}",
                    runtime_device_execution_device_profiles,
                    min_runtime_device_execution_device_profiles
                ));
            }
        }

        if let Some(min_runtime_kv_precision_device_profiles) =
            gate.min_runtime_kv_precision_device_profiles
        {
            let runtime_kv_precision_device_profiles = self.runtime_kv_precision_device_profiles();
            if runtime_kv_precision_device_profiles < min_runtime_kv_precision_device_profiles {
                failures.push(format!(
                    "runtime_kv_precision_device_profiles {} below minimum {}",
                    runtime_kv_precision_device_profiles, min_runtime_kv_precision_device_profiles
                ));
            }
        }

        if let Some(max_runtime_device_execution_violations) =
            gate.max_runtime_device_execution_violations
        {
            let runtime_device_execution_violations =
                self.total_runtime_device_execution_violations();
            if runtime_device_execution_violations > max_runtime_device_execution_violations {
                failures.push(format!(
                    "runtime_device_execution_violations {} above maximum {}: {}",
                    runtime_device_execution_violations,
                    max_runtime_device_execution_violations,
                    self.runtime_device_execution_evidence.failures.join("; ")
                ));
            }
        }

        if let Some(max_memory_governance_failures) = gate.max_memory_governance_failures {
            let memory_governance_failures = self.memory_governance_evidence.failures.len();
            if memory_governance_failures > max_memory_governance_failures {
                failures.push(format!(
                    "memory_governance_failures {} above maximum {}: {}",
                    memory_governance_failures,
                    max_memory_governance_failures,
                    self.memory_governance_evidence.failures.join("; ")
                ));
            }
        }

        if let Some(max_memory_feedback_evidence_failures) =
            gate.max_memory_feedback_evidence_failures
        {
            let memory_feedback_evidence_failures = self.total_memory_feedback_evidence_failures();
            if memory_feedback_evidence_failures > max_memory_feedback_evidence_failures {
                failures.push(format!(
                    "memory_feedback_evidence_failures {} above maximum {}: {}",
                    memory_feedback_evidence_failures,
                    max_memory_feedback_evidence_failures,
                    self.reflection_evidence.memory_feedback_failures.join("; ")
                ));
            }
        }

        if let Some(min_memory_governance_cases) = gate.min_memory_governance_cases {
            let memory_governance_cases = self.memory_governance_cases();
            if memory_governance_cases < min_memory_governance_cases {
                failures.push(format!(
                    "memory_governance_cases {} below minimum {}",
                    memory_governance_cases, min_memory_governance_cases
                ));
            }
        }

        if let Some(min_memory_governance_device_profiles) =
            gate.min_memory_governance_device_profiles
        {
            let memory_governance_device_profiles = self.memory_governance_device_profiles();
            if memory_governance_device_profiles < min_memory_governance_device_profiles {
                failures.push(format!(
                    "memory_governance_device_profiles {} below minimum {}",
                    memory_governance_device_profiles, min_memory_governance_device_profiles
                ));
            }
        }

        if let Some(min_memory_retention_activity_cases) = gate.min_memory_retention_activity_cases
        {
            let observed = self.memory_governance_evidence.retention_activity_cases;
            if observed < min_memory_retention_activity_cases {
                failures.push(format!(
                    "memory_retention_activity_cases {} below minimum {}",
                    observed, min_memory_retention_activity_cases
                ));
            }
        }

        if let Some(min_memory_compaction_activity_cases) =
            gate.min_memory_compaction_activity_cases
        {
            let observed = self.memory_governance_evidence.compaction_activity_cases;
            if observed < min_memory_compaction_activity_cases {
                failures.push(format!(
                    "memory_compaction_activity_cases {} below minimum {}",
                    observed, min_memory_compaction_activity_cases
                ));
            }
        }

        if let Some(min_reflection_issue_cases) = gate.min_reflection_issue_cases {
            let observed = self.reflection_evidence.issue_cases;
            if observed < min_reflection_issue_cases {
                failures.push(format!(
                    "reflection_issue_cases {} below minimum {}",
                    observed, min_reflection_issue_cases
                ));
            }
        }

        if let Some(min_reflection_issues) = gate.min_reflection_issues {
            let observed = self.reflection_evidence.total_issues;
            if observed < min_reflection_issues {
                failures.push(format!(
                    "reflection_issues {} below minimum {}",
                    observed, min_reflection_issues
                ));
            }
        }

        if let Some(min_critical_reflection_issue_cases) = gate.min_critical_reflection_issue_cases
        {
            let observed = self.reflection_evidence.critical_issue_cases;
            if observed < min_critical_reflection_issue_cases {
                failures.push(format!(
                    "critical_reflection_issue_cases {} below minimum {}",
                    observed, min_critical_reflection_issue_cases
                ));
            }
        }

        if let Some(min_critical_reflection_issues) = gate.min_critical_reflection_issues {
            let observed = self.reflection_evidence.total_critical_issues;
            if observed < min_critical_reflection_issues {
                failures.push(format!(
                    "critical_reflection_issues {} below minimum {}",
                    observed, min_critical_reflection_issues
                ));
            }
        }

        if let Some(min_revision_action_cases) = gate.min_revision_action_cases {
            let observed = self.reflection_evidence.revision_action_cases;
            if observed < min_revision_action_cases {
                failures.push(format!(
                    "revision_action_cases {} below minimum {}",
                    observed, min_revision_action_cases
                ));
            }
        }

        if let Some(min_revision_actions) = gate.min_revision_actions {
            let observed = self.reflection_evidence.total_revision_actions;
            if observed < min_revision_actions {
                failures.push(format!(
                    "revision_actions {} below minimum {}",
                    observed, min_revision_actions
                ));
            }
        }

        if let Some(min_reflection_issue_device_profiles) =
            gate.min_reflection_issue_device_profiles
        {
            let observed = self.reflection_evidence.issue_device_profiles();
            if observed < min_reflection_issue_device_profiles {
                failures.push(format!(
                    "reflection_issue_device_profiles {} below minimum {}",
                    observed, min_reflection_issue_device_profiles
                ));
            }
        }

        if let Some(min_critical_reflection_issue_device_profiles) =
            gate.min_critical_reflection_issue_device_profiles
        {
            let observed = self.reflection_evidence.critical_issue_device_profiles();
            if observed < min_critical_reflection_issue_device_profiles {
                failures.push(format!(
                    "critical_reflection_issue_device_profiles {} below minimum {}",
                    observed, min_critical_reflection_issue_device_profiles
                ));
            }
        }

        if let Some(min_revision_action_device_profiles) = gate.min_revision_action_device_profiles
        {
            let observed = self.reflection_evidence.revision_action_device_profiles();
            if observed < min_revision_action_device_profiles {
                failures.push(format!(
                    "revision_action_device_profiles {} below minimum {}",
                    observed, min_revision_action_device_profiles
                ));
            }
        }

        if let Some(min_device_profiles) = gate.min_device_profiles {
            let device_profiles = self.explicit_device_profiles_covered();
            if device_profiles < min_device_profiles {
                let missing = self
                    .missing_explicit_device_profiles()
                    .into_iter()
                    .map(DeviceClass::as_str)
                    .collect::<Vec<_>>()
                    .join("+");
                failures.push(format!(
                    "device_profiles {} below minimum {} missing={}",
                    device_profiles, min_device_profiles, missing
                ));
            }
        }

        if let Some(min_recursive_device_profiles) = gate.min_recursive_device_profiles {
            let recursive_device_profiles = self.recursive_device_profiles_covered();
            if recursive_device_profiles < min_recursive_device_profiles {
                let missing = self
                    .missing_recursive_device_profiles()
                    .into_iter()
                    .map(DeviceClass::as_str)
                    .collect::<Vec<_>>()
                    .join("+");
                failures.push(format!(
                    "recursive_device_profiles {} below minimum {} missing={}",
                    recursive_device_profiles, min_recursive_device_profiles, missing
                ));
            }
        }

        if let Some(max_drift_blocks) = gate.max_drift_blocks {
            let drift_blocks = self.drift_blocks();
            if drift_blocks > max_drift_blocks {
                failures.push(format!(
                    "drift_blocks {} above maximum {}",
                    drift_blocks, max_drift_blocks
                ));
            }
        }

        if let Some(max_drift_rollbacks) = gate.max_drift_rollbacks {
            let drift_rollbacks = self.drift_rollbacks();
            if drift_rollbacks > max_drift_rollbacks {
                failures.push(format!(
                    "drift_rollbacks {} above maximum {}",
                    drift_rollbacks, max_drift_rollbacks
                ));
            }
        }

        BenchmarkGateReport {
            passed: failures.is_empty(),
            failures,
        }
    }

    pub fn summary_line(&self) -> String {
        format!(
            "cases={} total_elapsed_ms={} avg_quality={:.3} avg_reward={:.3} avg_attention_fraction={:.2} device_profiles={} devices={} recursive_device_profiles={} recursive_devices={} recursive_cases={} max_recursive_waves={} recursive_runtime_calls={} auto_replay_applied={} auto_replay_router_updates={} auto_replay_hierarchy_updates={} auto_replay_router_threshold_mutations={} auto_replay_hierarchy_weight_mutations={} auto_replay_router_threshold_delta={:.6} auto_replay_hierarchy_weight_delta={:.6} auto_replay_memory_updates={} auto_replay_memory_reinforcements={} auto_replay_memory_penalties={} live_memory_feedback_updates={} live_memory_feedback_reinforcements={} live_memory_feedback_penalties={} live_memory_feedback_applied={} live_memory_feedback_removed={} live_memory_feedback_missing={} live_memory_feedback_strength_delta={:.6} memory_feedback_evidence_failures={} auto_replay_live_memory_feedback_items={} auto_replay_live_memory_feedback_updates={} auto_replay_live_memory_feedback_reinforcements={} auto_replay_live_memory_feedback_penalties={} auto_replay_live_memory_feedback_detail_items={} auto_replay_live_memory_feedback_applied={} auto_replay_live_memory_feedback_removed={} auto_replay_live_memory_feedback_missing={} auto_replay_live_memory_feedback_strength_delta={:.6} auto_replay_recursive_items={} auto_replay_recursive_runtime_calls={} auto_replay_max_recursive_call_pressure={:.3} evolution_live_inference_runs={} evolution_live_router_threshold_mutations={} evolution_live_hierarchy_weight_mutations={} evolution_live_router_threshold_delta={:.6} evolution_live_hierarchy_weight_delta={:.6} evolution_live_online_reward_feedbacks={} evolution_live_online_reward_reinforcements={} evolution_live_online_reward_penalties={} evolution_live_online_reward_strength={:.6} evolution_live_online_reward_reinforcement_strength={:.6} evolution_live_online_reward_penalty_strength={:.6} evolution_live_memory_updates={} evolution_live_stored_memory_updates={} evolution_live_reflection_issues={} evolution_live_critical_reflection_issues={} evolution_live_revision_actions={} evolution_live_inference_device_profiles={} evolution_live_router_threshold_mutation_device_profiles={} evolution_live_hierarchy_weight_mutation_device_profiles={} evolution_live_online_reward_device_profiles={} evolution_live_online_reward_strength_device_profiles={} evolution_live_memory_update_device_profiles={} evolution_live_stored_memory_update_device_profiles={} evolution_live_reflection_issue_device_profiles={} evolution_live_critical_reflection_issue_device_profiles={} evolution_live_revision_action_device_profiles={} evolution_replay_runs={} evolution_replay_items={} evolution_router_threshold_mutations={} evolution_hierarchy_weight_mutations={} evolution_router_threshold_delta={:.6} evolution_hierarchy_weight_delta={:.6} evolution_memory_updates={} evolution_replay_live_memory_feedback_items={} evolution_replay_live_memory_feedback_updates={} evolution_replay_live_memory_feedback_reinforcements={} evolution_replay_live_memory_feedback_penalties={} evolution_replay_live_memory_feedback_detail_items={} evolution_replay_live_memory_feedback_applied={} evolution_replay_live_memory_feedback_removed={} evolution_replay_live_memory_feedback_missing={} evolution_replay_live_memory_feedback_strength_delta={:.6} evolution_replay_live_evolution_items={} evolution_replay_live_evolution_router_threshold_mutations={} evolution_replay_live_evolution_hierarchy_weight_mutations={} evolution_replay_live_evolution_router_threshold_delta={:.6} evolution_replay_live_evolution_hierarchy_weight_delta={:.6} evolution_replay_live_evolution_online_reward_feedbacks={} evolution_replay_live_evolution_online_reward_reinforcements={} evolution_replay_live_evolution_online_reward_penalties={} evolution_replay_live_evolution_online_reward_strength={:.6} evolution_replay_live_evolution_online_reward_reinforcement_strength={:.6} evolution_replay_live_evolution_online_reward_penalty_strength={:.6} evolution_replay_live_evolution_memory_updates={} evolution_replay_live_evolution_stored_memory_updates={} evolution_replay_live_evolution_reflection_issues={} evolution_replay_live_evolution_critical_reflection_issues={} evolution_replay_live_evolution_revision_actions={} evolution_replay_live_evolution_device_profiles={} evolution_replay_live_evolution_online_reward_device_profiles={} evolution_replay_live_evolution_online_reward_strength_device_profiles={} evolution_replay_live_evolution_memory_update_device_profiles={} evolution_replay_live_evolution_critical_reflection_issue_device_profiles={} evolution_replay_live_evolution_revision_action_device_profiles={} evolution_recursive_replay_items={} evolution_recursive_runtime_calls={} evolution_drift_rollbacks={} evolution_rollback_router_threshold_delta={:.6} evolution_rollback_hierarchy_weight_delta={:.6} sparse_skipped_cases={} sparse_skipped={} sparse_skipped_tokens={} stored_memories={} compacted_memories={} memory_governance_cases={} memory_governance_device_profiles={} memory_governance_failures={} memory_retention_activity_cases={} memory_retention_decayed={} memory_retention_removed={} memory_compaction_activity_cases={} memory_compaction_merged={} memory_compaction_removed={} runtime_forward_cases={} runtime_forward_energy_cases={} runtime_kv_influence_cases={} runtime_kv_precision_cases={} runtime_kv_precision_device_profiles={} runtime_kv_precision_devices={} runtime_layer_mode_cases={} runtime_all_layer_mode_cases={} runtime_global_layers={} runtime_local_window_layers={} runtime_convolutional_fusion_layers={} runtime_token_cases={} runtime_tokens={} runtime_uncertainty_cases={} runtime_uncertainty_tokens={} runtime_uncertainty_device_profiles={} runtime_uncertainty_devices={} runtime_uncertainty_token_device_profiles={} runtime_uncertainty_token_devices={} runtime_kv_import_cases={} runtime_kv_imported={} runtime_kv_import_device_profiles={} runtime_kv_import_devices={} runtime_kv_exported={} runtime_kv_export_device_profiles={} runtime_kv_export_devices={} runtime_kv_stored={} runtime_kv_stored_device_profiles={} runtime_kv_stored_devices={} runtime_kv_hold_cases={} runtime_kv_held={} runtime_kv_hold_device_profiles={} runtime_kv_hold_devices={} runtime_adapter_contract_cases={} runtime_adapter_kinds={} runtime_adapter_contract_violations={} runtime_adapter_selection_mismatches={} runtime_adapter_observations={} runtime_adapter_best_score={} runtime_embedding_cases={} runtime_embedding_device_profiles={} runtime_embedding_devices={} runtime_embedding_calls={} embedding_fallback_cases={} embedding_fallback_calls={} embedding_evidence_failures={} runtime_device_execution_cases={} runtime_device_execution_matched_cases={} runtime_device_execution_device_profiles={} runtime_device_execution_devices={} runtime_device_execution_violations={} reflection_issue_cases={} reflection_issues={} reflection_issue_device_profiles={} critical_reflection_issue_cases={} critical_reflection_issues={} critical_reflection_issue_device_profiles={} revision_action_cases={} revision_actions={} revision_action_device_profiles={} drift_watch={} drift_block={} drift_rollback={}",
            self.len(),
            self.total_elapsed_ms(),
            self.average_quality(),
            self.average_reward(),
            self.average_attention_fraction(),
            self.explicit_device_profiles_covered(),
            self.devices_csv(),
            self.recursive_device_profiles_covered(),
            self.recursive_devices_csv(),
            self.recursive_cases(),
            self.max_recursive_waves(),
            self.total_recursive_runtime_calls(),
            self.total_auto_replay_applied(),
            self.total_auto_replay_router_updates(),
            self.total_auto_replay_hierarchy_updates(),
            self.total_auto_replay_router_threshold_mutations(),
            self.total_auto_replay_hierarchy_weight_mutations(),
            self.total_auto_replay_router_threshold_delta(),
            self.total_auto_replay_hierarchy_weight_delta(),
            self.total_auto_replay_memory_updates(),
            self.total_auto_replay_memory_reinforcements(),
            self.total_auto_replay_memory_penalties(),
            self.total_live_memory_feedback_updates(),
            self.total_live_memory_feedback_reinforcements(),
            self.total_live_memory_feedback_penalties(),
            self.total_live_memory_feedback_applied(),
            self.total_live_memory_feedback_removed(),
            self.total_live_memory_feedback_missing(),
            self.total_live_memory_feedback_strength_delta(),
            self.total_memory_feedback_evidence_failures(),
            self.total_auto_replay_live_memory_feedback_items(),
            self.total_auto_replay_live_memory_feedback_updates(),
            self.total_auto_replay_live_memory_feedback_reinforcements(),
            self.total_auto_replay_live_memory_feedback_penalties(),
            self.total_auto_replay_live_memory_feedback_detail_items(),
            self.total_auto_replay_live_memory_feedback_applied(),
            self.total_auto_replay_live_memory_feedback_removed(),
            self.total_auto_replay_live_memory_feedback_missing(),
            self.total_auto_replay_live_memory_feedback_strength_delta(),
            self.total_auto_replay_recursive_items(),
            self.total_auto_replay_recursive_runtime_calls(),
            self.max_auto_replay_recursive_call_pressure(),
            self.evolution_ledger.live_inference_runs,
            self.evolution_ledger.live_router_threshold_mutations,
            self.evolution_ledger.live_hierarchy_weight_mutations,
            self.evolution_ledger.live_router_threshold_delta,
            self.evolution_ledger.live_hierarchy_weight_delta,
            self.evolution_ledger.live_online_reward_feedbacks,
            self.evolution_ledger.live_online_reward_reinforcements,
            self.evolution_ledger.live_online_reward_penalties,
            self.evolution_ledger.live_online_reward_strength,
            self.evolution_ledger
                .live_online_reward_reinforcement_strength,
            self.evolution_ledger.live_online_reward_penalty_strength,
            self.evolution_ledger.live_memory_updates(),
            self.evolution_ledger.live_stored_memory_updates(),
            self.evolution_ledger.live_reflection_issues,
            self.evolution_ledger.live_critical_reflection_issues,
            self.evolution_ledger.live_revision_actions,
            self.live_evolution_evidence.inference_device_profiles(),
            self.live_evolution_evidence
                .router_threshold_mutation_device_profiles(),
            self.live_evolution_evidence
                .hierarchy_weight_mutation_device_profiles(),
            self.live_evolution_evidence.online_reward_device_profiles(),
            self.live_evolution_evidence
                .online_reward_strength_device_profiles(),
            self.live_evolution_evidence.memory_update_device_profiles(),
            self.live_evolution_evidence
                .stored_memory_update_device_profiles(),
            self.live_evolution_evidence
                .reflection_issue_device_profiles(),
            self.live_evolution_evidence
                .critical_reflection_issue_device_profiles(),
            self.live_evolution_evidence
                .revision_action_device_profiles(),
            self.evolution_ledger.replay_runs,
            self.evolution_ledger.replay_items,
            self.evolution_ledger.router_threshold_mutations,
            self.evolution_ledger.hierarchy_weight_mutations,
            self.evolution_ledger.router_threshold_delta,
            self.evolution_ledger.hierarchy_weight_delta,
            self.evolution_ledger.memory_updates(),
            self.evolution_ledger.replay_live_memory_feedback_items,
            self.evolution_ledger.replay_live_memory_feedback_updates(),
            self.evolution_ledger
                .replay_live_memory_feedback_reinforcements,
            self.evolution_ledger.replay_live_memory_feedback_penalties,
            self.evolution_ledger
                .replay_live_memory_feedback_detail_items,
            self.evolution_ledger.replay_live_memory_feedback_applied,
            self.evolution_ledger.replay_live_memory_feedback_removed,
            self.evolution_ledger.replay_live_memory_feedback_missing,
            self.evolution_ledger
                .replay_live_memory_feedback_strength_delta,
            self.evolution_ledger.replay_live_evolution_items,
            self.evolution_ledger
                .replay_live_evolution_router_threshold_mutations,
            self.evolution_ledger
                .replay_live_evolution_hierarchy_weight_mutations,
            self.evolution_ledger
                .replay_live_evolution_router_threshold_delta,
            self.evolution_ledger
                .replay_live_evolution_hierarchy_weight_delta,
            self.evolution_ledger
                .replay_live_evolution_online_reward_feedbacks,
            self.evolution_ledger
                .replay_live_evolution_online_reward_reinforcements,
            self.evolution_ledger
                .replay_live_evolution_online_reward_penalties,
            self.evolution_ledger
                .replay_live_evolution_online_reward_strength,
            self.evolution_ledger
                .replay_live_evolution_online_reward_reinforcement_strength,
            self.evolution_ledger
                .replay_live_evolution_online_reward_penalty_strength,
            self.evolution_ledger.replay_live_evolution_memory_updates,
            self.evolution_ledger
                .replay_live_evolution_stored_memory_updates,
            self.evolution_ledger
                .replay_live_evolution_reflection_issues,
            self.evolution_ledger
                .replay_live_evolution_critical_reflection_issues,
            self.evolution_ledger.replay_live_evolution_revision_actions,
            self.live_evolution_evidence
                .replay_live_evolution_device_profiles(),
            self.live_evolution_evidence
                .replay_live_evolution_online_reward_device_profiles(),
            self.live_evolution_evidence
                .replay_live_evolution_online_reward_strength_device_profiles(),
            self.live_evolution_evidence
                .replay_live_evolution_memory_update_device_profiles(),
            self.live_evolution_evidence
                .replay_live_evolution_critical_reflection_issue_device_profiles(),
            self.live_evolution_evidence
                .replay_live_evolution_revision_action_device_profiles(),
            self.evolution_ledger.recursive_replay_items,
            self.evolution_ledger.recursive_runtime_calls,
            self.evolution_ledger.drift_rollbacks,
            self.evolution_ledger.rollback_router_threshold_delta,
            self.evolution_ledger.rollback_hierarchy_weight_delta,
            self.sparse_skipped_cases(),
            self.total_sparse_skipped(),
            self.total_sparse_skipped_tokens(),
            self.total_stored_memories(),
            self.total_compacted_memories(),
            self.memory_governance_cases(),
            self.memory_governance_device_profiles(),
            self.memory_governance_evidence.failures.len(),
            self.memory_governance_evidence.retention_activity_cases,
            self.total_memory_retention_decayed(),
            self.total_memory_retention_removed(),
            self.memory_governance_evidence.compaction_activity_cases,
            self.total_memory_compaction_merged(),
            self.total_memory_compaction_removed(),
            self.runtime_forward_cases(),
            self.runtime_forward_energy_cases(),
            self.runtime_kv_influence_cases(),
            self.runtime_kv_precision_cases(),
            self.runtime_kv_precision_device_profiles(),
            self.runtime_device_execution_evidence
                .runtime_kv_precision_devices_csv(),
            self.runtime_layer_mode_cases(),
            self.runtime_all_layer_mode_cases(),
            self.total_runtime_global_layers(),
            self.total_runtime_local_window_layers(),
            self.total_runtime_convolutional_fusion_layers(),
            self.runtime_token_cases(),
            self.total_runtime_tokens(),
            self.runtime_uncertainty_cases(),
            self.total_runtime_uncertainty_tokens(),
            self.runtime_uncertainty_device_profiles(),
            self.runtime_uncertainty_devices_csv(),
            self.runtime_uncertainty_token_device_profiles(),
            self.runtime_uncertainty_token_devices_csv(),
            self.runtime_kv_import_cases(),
            self.total_runtime_kv_imported(),
            self.runtime_kv_import_device_profiles(),
            self.runtime_kv_import_devices_csv(),
            self.total_runtime_kv_exported(),
            self.runtime_kv_export_device_profiles(),
            self.runtime_kv_export_devices_csv(),
            self.total_runtime_kv_stored(),
            self.runtime_kv_stored_device_profiles(),
            self.runtime_kv_stored_devices_csv(),
            self.runtime_kv_hold_cases(),
            self.total_runtime_kv_held(),
            self.runtime_kv_hold_device_profiles(),
            self.runtime_kv_hold_devices_csv(),
            self.runtime_adapter_contract_cases(),
            self.runtime_adapter_kinds(),
            self.total_runtime_adapter_contract_violations(),
            self.total_runtime_adapter_selection_mismatches(),
            self.total_runtime_adapter_observations(),
            option_f32_display(self.max_runtime_adapter_score()),
            self.runtime_embedding_cases(),
            self.runtime_embedding_device_profiles(),
            self.embedding_evidence.runtime_devices_csv(),
            self.total_runtime_embedding_calls(),
            self.embedding_fallback_cases(),
            self.total_fallback_embedding_calls(),
            self.total_embedding_evidence_failures(),
            self.runtime_device_execution_cases(),
            self.runtime_device_execution_matched_cases(),
            self.runtime_device_execution_device_profiles(),
            self.runtime_device_execution_evidence.matched_devices_csv(),
            self.total_runtime_device_execution_violations(),
            self.reflection_evidence.issue_cases,
            self.reflection_evidence.total_issues,
            self.reflection_evidence.issue_device_profiles(),
            self.reflection_evidence.critical_issue_cases,
            self.reflection_evidence.total_critical_issues,
            self.reflection_evidence.critical_issue_device_profiles(),
            self.reflection_evidence.revision_action_cases,
            self.reflection_evidence.total_revision_actions,
            self.reflection_evidence.revision_action_device_profiles(),
            self.drift_watches(),
            self.drift_blocks(),
            self.drift_rollbacks()
        )
    }
}

fn runtime_kv_was_held(result: &BenchmarkCaseResult) -> bool {
    result.drift_severity == DriftSeverity::Watch
        && result.runtime_kv_exported > result.runtime_kv_stored
}

fn option_f32_display(value: Option<f32>) -> String {
    value
        .filter(|value| value.is_finite())
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "none".to_owned())
}

fn option_str_display(value: Option<&str>) -> &str {
    value.filter(|value| !value.is_empty()).unwrap_or("none")
}

fn max_evolution_ledger(left: EvolutionLedger, right: EvolutionLedger) -> EvolutionLedger {
    EvolutionLedger {
        live_inference_runs: left.live_inference_runs.max(right.live_inference_runs),
        live_router_threshold_mutations: left
            .live_router_threshold_mutations
            .max(right.live_router_threshold_mutations),
        live_hierarchy_weight_mutations: left
            .live_hierarchy_weight_mutations
            .max(right.live_hierarchy_weight_mutations),
        live_router_threshold_delta: left
            .live_router_threshold_delta
            .max(right.live_router_threshold_delta),
        live_hierarchy_weight_delta: left
            .live_hierarchy_weight_delta
            .max(right.live_hierarchy_weight_delta),
        live_online_reward_feedbacks: left
            .live_online_reward_feedbacks
            .max(right.live_online_reward_feedbacks),
        live_online_reward_reinforcements: left
            .live_online_reward_reinforcements
            .max(right.live_online_reward_reinforcements),
        live_online_reward_penalties: left
            .live_online_reward_penalties
            .max(right.live_online_reward_penalties),
        live_online_reward_strength: left
            .live_online_reward_strength
            .max(right.live_online_reward_strength),
        live_online_reward_reinforcement_strength: left
            .live_online_reward_reinforcement_strength
            .max(right.live_online_reward_reinforcement_strength),
        live_online_reward_penalty_strength: left
            .live_online_reward_penalty_strength
            .max(right.live_online_reward_penalty_strength),
        live_memory_reinforcements: left
            .live_memory_reinforcements
            .max(right.live_memory_reinforcements),
        live_memory_penalties: left.live_memory_penalties.max(right.live_memory_penalties),
        live_stored_memories: left.live_stored_memories.max(right.live_stored_memories),
        live_stored_gist_memories: left
            .live_stored_gist_memories
            .max(right.live_stored_gist_memories),
        live_stored_runtime_kv_memories: left
            .live_stored_runtime_kv_memories
            .max(right.live_stored_runtime_kv_memories),
        live_reflection_issues: left
            .live_reflection_issues
            .max(right.live_reflection_issues),
        live_critical_reflection_issues: left
            .live_critical_reflection_issues
            .max(right.live_critical_reflection_issues),
        live_revision_actions: left.live_revision_actions.max(right.live_revision_actions),
        replay_runs: left.replay_runs.max(right.replay_runs),
        replay_items: left.replay_items.max(right.replay_items),
        router_threshold_mutations: left
            .router_threshold_mutations
            .max(right.router_threshold_mutations),
        hierarchy_weight_mutations: left
            .hierarchy_weight_mutations
            .max(right.hierarchy_weight_mutations),
        router_threshold_delta: left
            .router_threshold_delta
            .max(right.router_threshold_delta),
        hierarchy_weight_delta: left
            .hierarchy_weight_delta
            .max(right.hierarchy_weight_delta),
        memory_reinforcements: left.memory_reinforcements.max(right.memory_reinforcements),
        memory_penalties: left.memory_penalties.max(right.memory_penalties),
        replay_live_memory_feedback_items: left
            .replay_live_memory_feedback_items
            .max(right.replay_live_memory_feedback_items),
        replay_live_memory_feedback_reinforcements: left
            .replay_live_memory_feedback_reinforcements
            .max(right.replay_live_memory_feedback_reinforcements),
        replay_live_memory_feedback_penalties: left
            .replay_live_memory_feedback_penalties
            .max(right.replay_live_memory_feedback_penalties),
        replay_live_memory_feedback_detail_items: left
            .replay_live_memory_feedback_detail_items
            .max(right.replay_live_memory_feedback_detail_items),
        replay_live_memory_feedback_applied: left
            .replay_live_memory_feedback_applied
            .max(right.replay_live_memory_feedback_applied),
        replay_live_memory_feedback_removed: left
            .replay_live_memory_feedback_removed
            .max(right.replay_live_memory_feedback_removed),
        replay_live_memory_feedback_missing: left
            .replay_live_memory_feedback_missing
            .max(right.replay_live_memory_feedback_missing),
        replay_live_memory_feedback_strength_delta: left
            .replay_live_memory_feedback_strength_delta
            .max(right.replay_live_memory_feedback_strength_delta),
        replay_live_evolution_items: left
            .replay_live_evolution_items
            .max(right.replay_live_evolution_items),
        replay_live_evolution_router_threshold_mutations: left
            .replay_live_evolution_router_threshold_mutations
            .max(right.replay_live_evolution_router_threshold_mutations),
        replay_live_evolution_hierarchy_weight_mutations: left
            .replay_live_evolution_hierarchy_weight_mutations
            .max(right.replay_live_evolution_hierarchy_weight_mutations),
        replay_live_evolution_router_threshold_delta: left
            .replay_live_evolution_router_threshold_delta
            .max(right.replay_live_evolution_router_threshold_delta),
        replay_live_evolution_hierarchy_weight_delta: left
            .replay_live_evolution_hierarchy_weight_delta
            .max(right.replay_live_evolution_hierarchy_weight_delta),
        replay_live_evolution_online_reward_feedbacks: left
            .replay_live_evolution_online_reward_feedbacks
            .max(right.replay_live_evolution_online_reward_feedbacks),
        replay_live_evolution_online_reward_reinforcements: left
            .replay_live_evolution_online_reward_reinforcements
            .max(right.replay_live_evolution_online_reward_reinforcements),
        replay_live_evolution_online_reward_penalties: left
            .replay_live_evolution_online_reward_penalties
            .max(right.replay_live_evolution_online_reward_penalties),
        replay_live_evolution_online_reward_strength: left
            .replay_live_evolution_online_reward_strength
            .max(right.replay_live_evolution_online_reward_strength),
        replay_live_evolution_online_reward_reinforcement_strength: left
            .replay_live_evolution_online_reward_reinforcement_strength
            .max(right.replay_live_evolution_online_reward_reinforcement_strength),
        replay_live_evolution_online_reward_penalty_strength: left
            .replay_live_evolution_online_reward_penalty_strength
            .max(right.replay_live_evolution_online_reward_penalty_strength),
        replay_live_evolution_memory_updates: left
            .replay_live_evolution_memory_updates
            .max(right.replay_live_evolution_memory_updates),
        replay_live_evolution_stored_memory_updates: left
            .replay_live_evolution_stored_memory_updates
            .max(right.replay_live_evolution_stored_memory_updates),
        replay_live_evolution_reflection_issues: left
            .replay_live_evolution_reflection_issues
            .max(right.replay_live_evolution_reflection_issues),
        replay_live_evolution_critical_reflection_issues: left
            .replay_live_evolution_critical_reflection_issues
            .max(right.replay_live_evolution_critical_reflection_issues),
        replay_live_evolution_revision_actions: left
            .replay_live_evolution_revision_actions
            .max(right.replay_live_evolution_revision_actions),
        recursive_replay_items: left
            .recursive_replay_items
            .max(right.recursive_replay_items),
        recursive_runtime_calls: left
            .recursive_runtime_calls
            .max(right.recursive_runtime_calls),
        drift_rollbacks: left.drift_rollbacks.max(right.drift_rollbacks),
        rollback_router_threshold_delta: left
            .rollback_router_threshold_delta
            .max(right.rollback_router_threshold_delta),
        rollback_hierarchy_weight_delta: left
            .rollback_hierarchy_weight_delta
            .max(right.rollback_hierarchy_weight_delta),
    }
}

fn average(values: impl Iterator<Item = f32>) -> f32 {
    let mut total = 0.0;
    let mut count = 0;

    for value in values {
        total += value;
        count += 1;
    }

    if count == 0 {
        0.0
    } else {
        total / count as f32
    }
}

fn kv_quant_benchmark_vectors() -> Vec<(&'static str, Vec<f32>)> {
    vec![
        (
            "ramp_1024",
            (0..1024)
                .map(|index| -1.0 + 2.0 * index as f32 / 1023.0)
                .collect(),
        ),
        (
            "wave_1024",
            (0..1024)
                .map(|index| {
                    let x = index as f32 / 32.0;
                    (x.sin() * 0.70) + (x.cos() * 0.25)
                })
                .collect(),
        ),
        (
            "sparse_1024",
            (0..1024)
                .map(|index| {
                    if index % 29 == 0 {
                        -0.55
                    } else if index % 17 == 0 {
                        0.75
                    } else {
                        0.0
                    }
                })
                .collect(),
        ),
    ]
}

fn quantization_error(original: &[f32], decoded: &[f32]) -> (f32, f32) {
    let mut max_abs_error = 0.0_f32;
    let mut total_abs_error = 0.0_f32;
    let mut count = 0;

    for (left, right) in original.iter().zip(decoded) {
        let error = (left - right).abs();
        max_abs_error = max_abs_error.max(error);
        total_abs_error += error;
        count += 1;
    }

    if count == 0 {
        (0.0, 0.0)
    } else {
        (max_abs_error, total_abs_error / count as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{
        GenerationContext, HeuristicBackend, InferenceBackend, InferenceRequest, NoironEngine,
    };
    use crate::kv_cache::{KvFusionCache, MemoryCompactionPolicy, MemoryRetentionPolicy};
    use crate::recursive_scheduler::RecursiveScheduler;
    use crate::reflection::{InferenceDraft, ReasoningStep, RuntimeDiagnostics};

    #[derive(Debug, Clone, Copy)]
    enum RuntimeDeviceExecutionMode {
        Matching,
        Mismatching,
        Missing,
        MissingKvPrecision,
    }

    struct RuntimeDeviceExecutionBackend {
        mode: RuntimeDeviceExecutionMode,
    }

    impl RuntimeDeviceExecutionBackend {
        fn matching() -> Self {
            Self {
                mode: RuntimeDeviceExecutionMode::Matching,
            }
        }

        fn mismatching() -> Self {
            Self {
                mode: RuntimeDeviceExecutionMode::Mismatching,
            }
        }

        fn missing() -> Self {
            Self {
                mode: RuntimeDeviceExecutionMode::Missing,
            }
        }

        fn missing_kv_precision() -> Self {
            Self {
                mode: RuntimeDeviceExecutionMode::MissingKvPrecision,
            }
        }
    }

    impl InferenceBackend for RuntimeDeviceExecutionBackend {
        fn generate(&mut self, context: GenerationContext<'_>) -> InferenceDraft {
            let execution = &context.hardware_plan.execution;
            let selected_adapter = execution
                .adapter_hints
                .first()
                .map(|adapter| adapter.as_str().to_owned());
            let diagnostics = match self.mode {
                RuntimeDeviceExecutionMode::Matching => RuntimeDiagnostics {
                    model_id: Some("runtime-device-execution-test".to_owned()),
                    selected_adapter: selected_adapter.clone(),
                    layer_count: 6,
                    global_layers: 2,
                    local_window_layers: 2,
                    convolutional_fusion_layers: 2,
                    hidden_size: 64,
                    local_window_tokens: 128,
                    forward_energy: Some(0.31),
                    kv_influence: Some(0.22),
                    ..RuntimeDiagnostics::default()
                        .with_device_execution(
                            context.hardware_plan.device.as_str(),
                            execution.primary_lane.as_str(),
                            execution.fallback_lane.as_str(),
                            execution.memory_mode.as_str(),
                        )
                        .with_kv_precision(
                            execution.hot_kv_precision_bits,
                            execution.cold_kv_precision_bits,
                        )
                },
                RuntimeDeviceExecutionMode::Mismatching => RuntimeDiagnostics {
                    model_id: Some("runtime-device-execution-test".to_owned()),
                    selected_adapter: selected_adapter.clone(),
                    layer_count: 6,
                    forward_energy: Some(0.31),
                    kv_influence: Some(0.22),
                    ..RuntimeDiagnostics::default()
                        .with_device_execution("server", "cuda", "cpu-simd", "gpu-resident")
                        .with_kv_precision(
                            execution.hot_kv_precision_bits,
                            execution.cold_kv_precision_bits,
                        )
                },
                RuntimeDeviceExecutionMode::Missing => RuntimeDiagnostics {
                    model_id: Some("runtime-device-execution-test".to_owned()),
                    selected_adapter,
                    layer_count: 6,
                    forward_energy: Some(0.31),
                    kv_influence: Some(0.22),
                    ..RuntimeDiagnostics::default()
                },
                RuntimeDeviceExecutionMode::MissingKvPrecision => RuntimeDiagnostics {
                    model_id: Some("runtime-device-execution-test".to_owned()),
                    selected_adapter,
                    layer_count: 6,
                    forward_energy: Some(0.31),
                    kv_influence: Some(0.22),
                    ..RuntimeDiagnostics::default().with_device_execution(
                        context.hardware_plan.device.as_str(),
                        execution.primary_lane.as_str(),
                        execution.fallback_lane.as_str(),
                        execution.memory_mode.as_str(),
                    )
                },
            };

            InferenceDraft::new(
                "Runtime device execution diagnostics are available for benchmark gating.",
                vec![ReasoningStep::new(
                    "runtime-device-execution",
                    "Attach execution lane and memory-mode evidence to the runtime result.",
                    0.91,
                )],
            )
            .with_runtime_diagnostics(diagnostics)
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

    #[test]
    fn default_cases_cover_core_profiles() {
        let cases = default_benchmark_cases();

        assert!(cases.iter().any(|case| case.profile == TaskProfile::Coding));
        assert!(
            cases
                .iter()
                .any(|case| case.profile == TaskProfile::LongDocument)
        );
        assert!(
            cases
                .iter()
                .any(|case| case.profile == TaskProfile::Writing)
        );
        assert!(
            cases
                .iter()
                .any(|case| case.profile == TaskProfile::General)
        );
    }

    #[test]
    fn default_long_context_case_can_trigger_small_window_recursion() {
        let cases = default_benchmark_cases();
        let long_context = cases
            .iter()
            .find(|case| case.name == "long_context_scheduler")
            .expect("long-context benchmark case");

        assert!(long_context.prompt.split_whitespace().count() > 128);
    }

    #[test]
    fn summary_records_case_outcomes() {
        let mut engine = NoironEngine::new();
        let mut backend = HeuristicBackend;
        let case = BenchmarkCase::new("coding", TaskProfile::Coding, "Rust benchmark trace");
        let outcome = engine.infer(
            InferenceRequest::new(case.prompt.clone(), case.profile),
            &mut backend,
        );
        let mut summary = BenchmarkSummary::new();

        summary.record(&case, 7, &outcome);

        assert_eq!(summary.len(), 1);
        assert!(summary.average_quality() > 0.0);
        assert!(summary.summary_line().contains("cases=1"));
        assert!(
            summary
                .summary_line()
                .contains("runtime_adapter_observations=")
        );
        assert!(
            summary
                .summary_line()
                .contains("live_memory_feedback_updates=")
        );
    }

    #[test]
    fn summary_records_runtime_device_execution_evidence() {
        let mut engine = NoironEngine::new();
        engine.set_hardware_snapshot(crate::hardware::HardwareSnapshot::new(
            DeviceClass::CpuOnly,
            0.35,
            0.00,
            0.45,
            0.20,
        ));
        let mut backend = RuntimeDeviceExecutionBackend::matching();
        let case = BenchmarkCase::new(
            "runtime_device_execution",
            TaskProfile::General,
            "prove runtime device execution diagnostics match the hardware plan",
        );
        let outcome = engine.infer(
            InferenceRequest::new(case.prompt.clone(), case.profile),
            &mut backend,
        );
        let mut summary = BenchmarkSummary::new();

        summary.record(&case, 5, &outcome);

        assert_eq!(summary.runtime_device_execution_cases(), 1);
        assert_eq!(summary.runtime_device_execution_matched_cases(), 1);
        assert_eq!(summary.runtime_device_execution_device_profiles(), 1);
        assert_eq!(summary.runtime_kv_precision_cases(), 1);
        assert_eq!(summary.runtime_kv_precision_device_profiles(), 1);
        assert_eq!(summary.total_runtime_device_execution_violations(), 0);
        let report = summary.evaluate(&BenchmarkGate {
            min_runtime_device_execution_cases: Some(1),
            min_runtime_device_execution_device_profiles: Some(1),
            min_runtime_kv_precision_cases: Some(1),
            min_runtime_kv_precision_device_profiles: Some(1),
            max_runtime_device_execution_violations: Some(0),
            ..BenchmarkGate::default()
        });
        assert!(report.passed, "{:?}", report.failures);
        assert!(
            summary
                .summary_line()
                .contains("runtime_device_execution_matched_cases=1")
        );
        assert!(
            summary
                .summary_line()
                .contains("runtime_device_execution_devices=cpu")
        );
        assert!(
            summary
                .summary_line()
                .contains("runtime_kv_precision_cases=1")
        );
        assert!(
            summary
                .summary_line()
                .contains("runtime_kv_precision_device_profiles=1")
        );
        assert!(
            summary
                .summary_line()
                .contains("runtime_kv_precision_devices=cpu")
        );
    }

    #[test]
    fn gate_reports_runtime_device_execution_mismatch() {
        let mut engine = NoironEngine::new();
        engine.set_hardware_snapshot(crate::hardware::HardwareSnapshot::new(
            DeviceClass::CpuOnly,
            0.35,
            0.00,
            0.45,
            0.20,
        ));
        let mut backend = RuntimeDeviceExecutionBackend::mismatching();
        let case = BenchmarkCase::new(
            "runtime_device_execution_mismatch",
            TaskProfile::General,
            "catch runtime device execution diagnostics that drift from hardware",
        );
        let outcome = engine.infer(
            InferenceRequest::new(case.prompt.clone(), case.profile),
            &mut backend,
        );
        let mut summary = BenchmarkSummary::new();

        summary.record(&case, 5, &outcome);
        let report = summary.evaluate(&BenchmarkGate::default());

        assert!(!report.passed);
        assert_eq!(summary.runtime_device_execution_cases(), 1);
        assert_eq!(summary.runtime_device_execution_matched_cases(), 0);
        assert_eq!(summary.total_runtime_device_execution_violations(), 1);
        assert!(report.failures.iter().any(|failure| {
            failure.contains("runtime_device_execution_violations")
                && failure.contains("device_profile actual=server expected=cpu")
        }));
    }

    #[test]
    fn gate_reports_missing_runtime_device_execution_for_forward_signal() {
        let mut engine = NoironEngine::new();
        engine.set_hardware_snapshot(crate::hardware::HardwareSnapshot::new(
            DeviceClass::CpuOnly,
            0.35,
            0.00,
            0.45,
            0.20,
        ));
        let mut backend = RuntimeDeviceExecutionBackend::missing();
        let case = BenchmarkCase::new(
            "runtime_device_execution_missing",
            TaskProfile::General,
            "catch runtime forward diagnostics that omit device execution evidence",
        );
        let outcome = engine.infer(
            InferenceRequest::new(case.prompt.clone(), case.profile),
            &mut backend,
        );
        let mut summary = BenchmarkSummary::new();

        summary.record(&case, 5, &outcome);
        let report = summary.evaluate(&BenchmarkGate::default());

        assert!(!report.passed);
        assert_eq!(summary.runtime_device_execution_cases(), 0);
        assert_eq!(summary.runtime_device_execution_matched_cases(), 0);
        assert_eq!(summary.total_runtime_device_execution_violations(), 1);
        assert!(report.failures.iter().any(|failure| {
            failure.contains("runtime_device_execution_violations")
                && failure.contains("missing device execution diagnostics")
        }));
    }

    #[test]
    fn gate_reports_missing_runtime_kv_precision_diagnostics() {
        let mut engine = NoironEngine::new();
        engine.set_hardware_snapshot(crate::hardware::HardwareSnapshot::new(
            DeviceClass::CpuOnly,
            0.35,
            0.00,
            0.45,
            0.20,
        ));
        let mut backend = RuntimeDeviceExecutionBackend::missing_kv_precision();
        let case = BenchmarkCase::new(
            "runtime_kv_precision_missing",
            TaskProfile::General,
            "catch runtime device execution diagnostics that omit KV precision evidence",
        );
        let outcome = engine.infer(
            InferenceRequest::new(case.prompt.clone(), case.profile),
            &mut backend,
        );
        let mut summary = BenchmarkSummary::new();

        summary.record(&case, 5, &outcome);
        let report = summary.evaluate(&BenchmarkGate {
            min_runtime_kv_precision_cases: Some(1),
            min_runtime_kv_precision_device_profiles: Some(1),
            max_runtime_device_execution_violations: Some(0),
            ..BenchmarkGate::default()
        });

        assert!(!report.passed);
        assert_eq!(summary.runtime_device_execution_cases(), 1);
        assert_eq!(summary.runtime_device_execution_matched_cases(), 0);
        assert_eq!(summary.runtime_kv_precision_cases(), 0);
        assert_eq!(summary.runtime_kv_precision_device_profiles(), 0);
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("runtime_kv_precision_cases 0 below minimum 1"))
        );
        assert!(report.failures.iter().any(|failure| {
            failure.contains("runtime_kv_precision_device_profiles 0 below minimum 1")
        }));
        assert!(report.failures.iter().any(|failure| {
            failure.contains("runtime_device_execution_violations")
                && failure.contains("missing valid KV precision diagnostics")
        }));
    }

    #[test]
    fn summary_records_recursive_case_outcomes() {
        let mut engine = NoironEngine::new();
        engine.recursive_scheduler = RecursiveScheduler::new(64, 32, 8, 2);
        let mut backend = HeuristicBackend;
        let case = BenchmarkCase::new(
            "long_context_scheduler",
            TaskProfile::LongDocument,
            long_context_benchmark_prompt(),
        );
        let outcome = engine.infer(
            InferenceRequest::new(case.prompt.clone(), case.profile),
            &mut backend,
        );
        let mut summary = BenchmarkSummary::new();

        summary.record(&case, 7, &outcome);

        assert_eq!(summary.recursive_cases(), 1);
        assert!(summary.max_recursive_chunks() > 1);
        assert!(summary.total_recursive_runtime_calls() > summary.max_recursive_chunks());
        assert!(summary.summary_line().contains("recursive_cases=1"));
        assert!(summary.summary_line().contains("recursive_runtime_calls="));
        assert!(
            summary
                .summary_line()
                .contains("auto_replay_recursive_items=")
        );
        assert!(
            summary
                .summary_line()
                .contains("auto_replay_router_updates=")
        );
    }

    #[test]
    fn default_gate_passes_heuristic_summary() {
        let mut engine = NoironEngine::new();
        let mut backend = HeuristicBackend;
        let case = BenchmarkCase::new(
            "reflection",
            TaskProfile::General,
            "Explain benchmark gates for Noiron control loops",
        );
        let outcome = engine.infer(
            InferenceRequest::new(case.prompt.clone(), case.profile),
            &mut backend,
        );
        let mut summary = BenchmarkSummary::new();

        summary.record(&case, 3, &outcome);
        let report = summary.evaluate(&BenchmarkGate::default());

        assert!(report.passed, "{:?}", report.failures);
        assert!(report.summary_line().contains("passed=true"));
    }

    #[test]
    fn summary_records_memory_governance_evidence() {
        let mut engine = NoironEngine::new();
        engine.cache = KvFusionCache::with_limits(0.99, 4096);
        engine.set_hardware_snapshot(crate::hardware::HardwareSnapshot::new(
            DeviceClass::CpuOnly,
            0.25,
            0.0,
            0.35,
            0.15,
        ));
        engine.set_memory_retention_policy(MemoryRetentionPolicy {
            stale_after: 1,
            decay_rate: 0.50,
            remove_below_strength: 0.15,
            remove_after_failures: 1,
        });
        engine.set_memory_compaction_policy(MemoryCompactionPolicy {
            similarity_threshold: 0.90,
            max_candidates: 8,
            max_merges: 2,
        });
        let weak_id =
            engine
                .cache
                .store_or_fuse("benchmark_governance:weak", vec![1.0, 0.0, 0.0, 0.0], 0.05);
        engine.cache.penalize(weak_id, 1.0);
        engine.cache.store_or_fuse(
            "benchmark_governance:compact_a",
            vec![0.0, 1.0, 0.0, 0.0],
            0.70,
        );
        engine.cache.store_or_fuse(
            "benchmark_governance:compact_b",
            vec![0.0, 0.96, 0.28, 0.0],
            0.70,
        );
        let mut backend = HeuristicBackend;
        let case = BenchmarkCase::new(
            "memory_governance",
            TaskProfile::General,
            "Audit Noiron memory governance retention and compaction evidence.",
        );
        let outcome = engine.infer(
            InferenceRequest::new(case.prompt.clone(), case.profile),
            &mut backend,
        );
        let mut summary = BenchmarkSummary::new();

        summary.record(&case, 5, &outcome);

        assert_eq!(summary.memory_governance_cases(), 1);
        assert_eq!(summary.memory_governance_device_profiles(), 1);
        assert_eq!(summary.memory_governance_evidence().failures.len(), 0);
        assert!(summary.total_memory_retention_decayed() >= 1);
        assert!(summary.total_memory_retention_removed() >= 1);
        assert!(summary.summary_line().contains("memory_governance_cases=1"));
        assert!(
            summary
                .summary_line()
                .contains("memory_governance_failures=0")
        );
        assert!(
            summary
                .summary_line()
                .contains("memory_retention_activity_cases=1")
        );

        let gate = BenchmarkGate {
            min_memory_governance_cases: Some(1),
            min_memory_governance_device_profiles: Some(1),
            min_memory_retention_activity_cases: Some(1),
            ..BenchmarkGate::default()
        };

        let passing = summary.evaluate(&gate);

        assert!(passing.passed, "{:?}", passing.failures);
    }

    #[test]
    fn gate_accepts_memory_governance_activity_evidence() {
        let result = BenchmarkCaseResult {
            name: "memory_governance_activity".to_owned(),
            profile: TaskProfile::General,
            device: DeviceClass::CpuOnly,
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
        };
        let summary = BenchmarkSummary {
            memory_governance_evidence: BenchmarkMemoryGovernanceEvidence {
                cases: 2,
                retention_activity_cases: 1,
                compaction_activity_cases: 1,
                total_retention_decayed: 2,
                total_retention_removed: 1,
                total_compaction_merged: 1,
                total_compaction_removed: 1,
                governance_devices: vec![DeviceClass::CpuOnly, DeviceClass::IntegratedGpu],
                retention_activity_devices: vec![DeviceClass::CpuOnly],
                compaction_activity_devices: vec![DeviceClass::IntegratedGpu],
                ..BenchmarkMemoryGovernanceEvidence::default()
            },
            results: vec![
                result.clone(),
                BenchmarkCaseResult {
                    device: DeviceClass::IntegratedGpu,
                    ..result
                },
            ],
            ..BenchmarkSummary::default()
        };
        let gate = BenchmarkGate {
            min_memory_governance_cases: Some(2),
            min_memory_governance_device_profiles: Some(2),
            min_memory_retention_activity_cases: Some(1),
            min_memory_compaction_activity_cases: Some(1),
            ..BenchmarkGate::default()
        };

        let report = summary.evaluate(&gate);

        assert!(report.passed, "{:?}", report.failures);
        assert_eq!(summary.memory_governance_device_profiles(), 2);
        assert_eq!(summary.total_memory_retention_decayed(), 2);
        assert_eq!(summary.total_memory_retention_removed(), 1);
        assert_eq!(summary.total_memory_compaction_merged(), 1);
        assert_eq!(summary.total_memory_compaction_removed(), 1);
        assert!(
            summary
                .summary_line()
                .contains("memory_governance_device_profiles=2")
        );
        assert!(
            summary
                .summary_line()
                .contains("memory_compaction_activity_cases=1")
        );
    }

    #[test]
    fn gate_reports_missing_memory_governance_coverage() {
        let summary = BenchmarkSummary::new();
        let gate = BenchmarkGate {
            min_memory_governance_cases: Some(1),
            min_memory_governance_device_profiles: Some(1),
            min_memory_retention_activity_cases: Some(1),
            min_memory_compaction_activity_cases: Some(1),
            ..BenchmarkGate::default()
        };

        let report = summary.evaluate(&gate);

        assert!(!report.passed);
        for marker in [
            "memory_governance_cases",
            "memory_governance_device_profiles",
            "memory_retention_activity_cases",
            "memory_compaction_activity_cases",
        ] {
            assert!(
                report
                    .failures
                    .iter()
                    .any(|failure| failure.contains(marker)),
                "missing failure marker {marker}: {:?}",
                report.failures
            );
        }
    }

    #[test]
    fn gate_reports_memory_governance_failures() {
        let summary = BenchmarkSummary {
            memory_governance_evidence: BenchmarkMemoryGovernanceEvidence {
                cases: 1,
                failures: vec!["cpu:bad retention stale_after must be > 0".to_owned()],
                ..BenchmarkMemoryGovernanceEvidence::default()
            },
            ..BenchmarkSummary::default()
        };

        let report = summary.evaluate(&BenchmarkGate::default());

        assert!(!report.passed);
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("memory_governance_failures"))
        );
    }

    #[test]
    fn gate_reports_missing_live_evolution_device_profile_coverage() {
        let mut engine = NoironEngine::new();
        engine.set_hardware_snapshot(crate::hardware::HardwareSnapshot::new(
            DeviceClass::CpuOnly,
            0.35,
            0.00,
            0.45,
            0.20,
        ));
        let mut backend = HeuristicBackend;
        let case = BenchmarkCase::new(
            "live_evolution_cpu",
            TaskProfile::General,
            "Explain why live Noiron inference should update local memory and reflection state.",
        );
        let outcome = engine.infer(
            InferenceRequest::new(case.prompt.clone(), case.profile),
            &mut backend,
        );
        let mut summary = BenchmarkSummary::new();
        summary.record(&case, 3, &outcome);

        let mut gate = BenchmarkGate::default();
        gate.min_evolution_live_inference_device_profiles = Some(2);

        let failing = summary.evaluate(&gate);

        assert!(!failing.passed);
        assert_eq!(
            summary
                .live_evolution_evidence()
                .inference_device_profiles(),
            1
        );
        assert!(failing.failures.iter().any(|failure| {
            failure == "evolution_live_inference_device_profiles 1 below minimum 2"
        }));

        engine.set_hardware_snapshot(crate::hardware::HardwareSnapshot::new(
            DeviceClass::IntegratedGpu,
            0.35,
            0.30,
            0.45,
            0.20,
        ));
        let gpu_case = BenchmarkCase::new(
            "live_evolution_integrated",
            TaskProfile::General,
            "Explain why live Noiron inference should preserve reflection and memory evidence.",
        );
        let gpu_outcome = engine.infer(
            InferenceRequest::new(gpu_case.prompt.clone(), gpu_case.profile),
            &mut backend,
        );
        summary.record(&gpu_case, 4, &gpu_outcome);
        gate.min_evolution_live_inference_device_profiles = Some(2);

        let passing = summary.evaluate(&gate);

        assert!(passing.passed, "{:?}", passing.failures);
        assert_eq!(
            summary
                .live_evolution_evidence()
                .inference_device_profiles(),
            2
        );
        assert!(
            summary
                .summary_line()
                .contains("evolution_live_inference_device_profiles=2")
        );
    }

    #[test]
    fn gate_reports_missing_live_evolution_detail_device_profile_coverage() {
        let base_result = BenchmarkCaseResult {
            name: "live_detail".to_owned(),
            profile: TaskProfile::General,
            device: DeviceClass::CpuOnly,
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
        };
        let summary = BenchmarkSummary {
            reflection_evidence: BenchmarkReflectionEvidence::default(),
            live_evolution_evidence: BenchmarkLiveEvolutionEvidence {
                inference_devices: vec![DeviceClass::CpuOnly, DeviceClass::IntegratedGpu],
                router_threshold_mutation_devices: vec![DeviceClass::CpuOnly],
                hierarchy_weight_mutation_devices: vec![DeviceClass::CpuOnly],
                online_reward_devices: vec![DeviceClass::CpuOnly],
                online_reward_strength_devices: Vec::new(),
                memory_update_devices: vec![DeviceClass::CpuOnly, DeviceClass::IntegratedGpu],
                stored_memory_update_devices: vec![DeviceClass::CpuOnly],
                reflection_issue_devices: vec![DeviceClass::CpuOnly],
                critical_reflection_issue_devices: vec![DeviceClass::CpuOnly],
                revision_action_devices: vec![DeviceClass::CpuOnly],
                replay_live_evolution_devices: Vec::new(),
                replay_live_evolution_online_reward_devices: Vec::new(),
                replay_live_evolution_online_reward_strength_devices: Vec::new(),
                replay_live_evolution_memory_update_devices: Vec::new(),
                replay_live_evolution_critical_reflection_issue_devices: Vec::new(),
                replay_live_evolution_revision_action_devices: Vec::new(),
            },
            memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
            embedding_evidence: BenchmarkEmbeddingEvidence::default(),

            runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),
            evolution_ledger: EvolutionLedger::default(),
            results: vec![base_result],
        };
        let gate = BenchmarkGate {
            min_average_quality: 0.5,
            min_average_reward: 0.45,
            min_evolution_live_inference_device_profiles: Some(2),
            min_evolution_live_router_threshold_mutation_device_profiles: Some(2),
            min_evolution_live_hierarchy_weight_mutation_device_profiles: Some(2),
            min_evolution_live_online_reward_device_profiles: Some(1),
            min_evolution_live_online_reward_strength_device_profiles: Some(2),
            min_evolution_live_memory_update_device_profiles: Some(2),
            min_evolution_live_stored_memory_update_device_profiles: Some(2),
            min_evolution_live_reflection_issue_device_profiles: Some(2),
            min_evolution_live_critical_reflection_issue_device_profiles: Some(2),
            min_evolution_live_revision_action_device_profiles: Some(2),
            ..BenchmarkGate::default()
        };

        let failing = summary.evaluate(&gate);

        assert!(!failing.passed);
        for marker in [
            "evolution_live_router_threshold_mutation_device_profiles",
            "evolution_live_hierarchy_weight_mutation_device_profiles",
            "evolution_live_online_reward_strength_device_profiles",
            "evolution_live_stored_memory_update_device_profiles",
            "evolution_live_reflection_issue_device_profiles",
            "evolution_live_critical_reflection_issue_device_profiles",
            "evolution_live_revision_action_device_profiles",
        ] {
            assert!(
                failing
                    .failures
                    .iter()
                    .any(|failure| failure.contains(marker)),
                "missing failure marker {marker}: {:?}",
                failing.failures
            );
        }
        assert!(
            failing
                .failures
                .iter()
                .all(|failure| !failure.contains("evolution_live_inference_device_profiles"))
        );
        assert!(
            failing
                .failures
                .iter()
                .all(|failure| !failure.contains("evolution_live_memory_update_device_profiles"))
        );

        let passing = BenchmarkSummary {
            live_evolution_evidence: BenchmarkLiveEvolutionEvidence {
                router_threshold_mutation_devices: vec![
                    DeviceClass::CpuOnly,
                    DeviceClass::IntegratedGpu,
                ],
                hierarchy_weight_mutation_devices: vec![
                    DeviceClass::CpuOnly,
                    DeviceClass::IntegratedGpu,
                ],
                online_reward_devices: vec![DeviceClass::CpuOnly, DeviceClass::IntegratedGpu],
                online_reward_strength_devices: vec![
                    DeviceClass::CpuOnly,
                    DeviceClass::IntegratedGpu,
                ],
                stored_memory_update_devices: vec![
                    DeviceClass::CpuOnly,
                    DeviceClass::IntegratedGpu,
                ],
                reflection_issue_devices: vec![DeviceClass::CpuOnly, DeviceClass::IntegratedGpu],
                critical_reflection_issue_devices: vec![
                    DeviceClass::CpuOnly,
                    DeviceClass::IntegratedGpu,
                ],
                revision_action_devices: vec![DeviceClass::CpuOnly, DeviceClass::IntegratedGpu],
                ..summary.live_evolution_evidence.clone()
            },
            ..summary
        };

        let passing_report = passing.evaluate(&gate);

        assert!(passing_report.passed, "{:?}", passing_report.failures);
        assert!(
            passing
                .summary_line()
                .contains("evolution_live_online_reward_device_profiles=2")
        );
        assert!(
            passing
                .summary_line()
                .contains("evolution_live_online_reward_strength_device_profiles=2")
        );
        assert!(
            passing
                .summary_line()
                .contains("evolution_live_revision_action_device_profiles=2")
        );
    }

    #[test]
    fn gate_reports_missing_replay_live_evolution_device_profile_coverage() {
        let mut engine = NoironEngine::new();
        let mut backend = HeuristicBackend;
        let case = BenchmarkCase::new(
            "replay_live_evolution_device_coverage",
            TaskProfile::Coding,
            "Rust Noiron replay live evolution device coverage",
        );
        let outcome = engine.infer(
            InferenceRequest::new(case.prompt.clone(), case.profile),
            &mut backend,
        );
        let mut summary = BenchmarkSummary::new();
        summary.record(&case, 1, &outcome);
        summary.live_evolution_evidence = BenchmarkLiveEvolutionEvidence {
            replay_live_evolution_devices: vec![DeviceClass::CpuOnly],
            replay_live_evolution_online_reward_devices: vec![DeviceClass::CpuOnly],
            replay_live_evolution_online_reward_strength_devices: Vec::new(),
            replay_live_evolution_memory_update_devices: vec![DeviceClass::CpuOnly],
            ..BenchmarkLiveEvolutionEvidence::default()
        };
        let gate = BenchmarkGate {
            min_average_quality: 0.0,
            min_average_reward: 0.0,
            min_evolution_replay_live_evolution_device_profiles: Some(2),
            min_evolution_replay_live_evolution_online_reward_device_profiles: Some(1),
            min_evolution_replay_live_evolution_online_reward_strength_device_profiles: Some(2),
            min_evolution_replay_live_evolution_memory_update_device_profiles: Some(2),
            min_evolution_replay_live_evolution_critical_reflection_issue_device_profiles: Some(2),
            min_evolution_replay_live_evolution_revision_action_device_profiles: Some(2),
            ..BenchmarkGate::default()
        };

        let failing = summary.evaluate(&gate);

        assert!(!failing.passed);
        for marker in [
            "evolution_replay_live_evolution_device_profiles",
            "evolution_replay_live_evolution_online_reward_strength_device_profiles",
            "evolution_replay_live_evolution_memory_update_device_profiles",
            "evolution_replay_live_evolution_critical_reflection_issue_device_profiles",
            "evolution_replay_live_evolution_revision_action_device_profiles",
        ] {
            assert!(
                failing
                    .failures
                    .iter()
                    .any(|failure| failure.contains(marker)),
                "missing failure marker {marker}: {:?}",
                failing.failures
            );
        }

        let mut passing = summary.clone();
        passing.live_evolution_evidence = BenchmarkLiveEvolutionEvidence {
            replay_live_evolution_devices: vec![DeviceClass::CpuOnly, DeviceClass::IntegratedGpu],
            replay_live_evolution_online_reward_devices: vec![
                DeviceClass::CpuOnly,
                DeviceClass::IntegratedGpu,
            ],
            replay_live_evolution_online_reward_strength_devices: vec![
                DeviceClass::CpuOnly,
                DeviceClass::IntegratedGpu,
            ],
            replay_live_evolution_memory_update_devices: vec![
                DeviceClass::CpuOnly,
                DeviceClass::IntegratedGpu,
            ],
            replay_live_evolution_critical_reflection_issue_devices: vec![
                DeviceClass::CpuOnly,
                DeviceClass::IntegratedGpu,
            ],
            replay_live_evolution_revision_action_devices: vec![
                DeviceClass::CpuOnly,
                DeviceClass::IntegratedGpu,
            ],
            ..BenchmarkLiveEvolutionEvidence::default()
        };
        let passing_report = passing.evaluate(&gate);

        assert!(passing_report.passed, "{:?}", passing_report.failures);
        assert!(
            passing
                .summary_line()
                .contains("evolution_replay_live_evolution_device_profiles=2")
        );
        assert!(
            passing
                .summary_line()
                .contains("evolution_replay_live_evolution_online_reward_device_profiles=2")
        );
        assert!(
            passing.summary_line().contains(
                "evolution_replay_live_evolution_online_reward_strength_device_profiles=2"
            )
        );
        assert!(
            passing
                .summary_line()
                .contains("evolution_replay_live_evolution_memory_update_device_profiles=2")
        );
        assert!(passing.summary_line().contains(
            "evolution_replay_live_evolution_critical_reflection_issue_device_profiles=2"
        ));
        assert!(
            passing
                .summary_line()
                .contains("evolution_replay_live_evolution_revision_action_device_profiles=2")
        );
    }

    #[test]
    fn gate_reports_missing_live_memory_feedback_updates() {
        let mut engine = NoironEngine::new();
        let mut backend = HeuristicBackend;
        let case = BenchmarkCase::new(
            "live_memory_feedback",
            TaskProfile::Coding,
            "Rust Noiron benchmark live memory feedback",
        );
        let first = engine.infer(
            InferenceRequest::new(case.prompt.clone(), case.profile),
            &mut backend,
        );
        let second = engine.infer(
            InferenceRequest::new(case.prompt.clone(), case.profile),
            &mut backend,
        );
        let mut missing = BenchmarkSummary::new();
        let mut passing = BenchmarkSummary::new();
        let mut gate = BenchmarkGate::default();
        gate.min_live_memory_feedback_updates = Some(1);

        assert_eq!(first.memory_feedback.total_updates(), 0);
        assert!(second.memory_feedback.total_updates() > 0);
        missing.record(&case, 1, &first);
        passing.record(&case, 1, &second);
        let missing_report = missing.evaluate(&gate);
        let passing_report = passing.evaluate(&gate);

        assert!(!missing_report.passed);
        assert!(
            missing_report
                .failures
                .iter()
                .any(|failure| failure.contains("live_memory_feedback_updates"))
        );
        assert!(passing_report.passed, "{:?}", passing_report.failures);
        assert!(passing.total_live_memory_feedback_updates() >= 1);
        assert!(
            passing
                .summary_line()
                .contains("live_memory_feedback_updates=")
        );
    }

    #[test]
    fn gate_reports_memory_feedback_evidence_failures() {
        let mut engine = NoironEngine::new();
        let mut backend = HeuristicBackend;
        let case = BenchmarkCase::new(
            "memory_feedback_evidence",
            TaskProfile::Coding,
            "Audit reinforced KV memory feedback evidence.",
        );
        let outcome = engine.infer(
            InferenceRequest::new(case.prompt.clone(), case.profile),
            &mut backend,
        );
        let mut summary = BenchmarkSummary::new();
        summary.record(&case, 1, &outcome);
        summary
            .reflection_evidence
            .memory_feedback_failures
            .push("manual memory feedback evidence mismatch".to_owned());

        let report = summary.evaluate(&BenchmarkGate::default());

        assert!(!report.passed);
        assert_eq!(summary.total_memory_feedback_evidence_failures(), 1);
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("memory_feedback_evidence_failures")),
            "{:?}",
            report.failures
        );
        assert!(
            summary
                .summary_line()
                .contains("memory_feedback_evidence_failures=1")
        );
    }

    #[test]
    fn gate_reports_missing_auto_replay_live_memory_feedback_consumption() {
        let mut summary = BenchmarkSummary::new();
        let mut gate = BenchmarkGate::default();
        gate.min_auto_replay_live_memory_feedback_updates = Some(2);
        gate.min_auto_replay_live_memory_feedback_detail_items = Some(1);
        gate.min_auto_replay_live_memory_feedback_applied = Some(2);
        gate.min_auto_replay_live_memory_feedback_strength_delta = Some(0.42);

        let missing_report = summary.evaluate(&gate);

        assert!(!missing_report.passed);
        assert!(
            missing_report
                .failures
                .iter()
                .any(|failure| failure.contains("auto_replay_live_memory_feedback_updates"))
        );

        summary.results.push(BenchmarkCaseResult {
            name: "replay_live_feedback".to_owned(),
            profile: TaskProfile::Coding,
            device: DeviceClass::CpuOnly,
            elapsed_ms: 1,
            quality: 0.9,
            process_reward: 0.9,
            attention_fraction: 0.5,
            requires_recursion: false,
            recursive_chunks: 1,
            recursive_waves: 1,
            recursive_runtime_calls: 1,
            auto_replay_applied: 1,
            auto_replay_router_updates: 1,
            auto_replay_hierarchy_updates: 1,
            auto_replay_router_threshold_mutations: 0,
            auto_replay_hierarchy_weight_mutations: 0,
            auto_replay_router_threshold_delta: 0.0,
            auto_replay_hierarchy_weight_delta: 0.0,
            auto_replay_memory_reinforcements: 1,
            auto_replay_memory_penalties: 0,
            auto_replay_live_memory_feedback_items: 1,
            auto_replay_live_memory_feedback_updates: 2,
            auto_replay_live_memory_feedback_reinforcements: 2,
            auto_replay_live_memory_feedback_penalties: 0,
            auto_replay_live_memory_feedback_detail_items: 1,
            auto_replay_live_memory_feedback_applied: 2,
            auto_replay_live_memory_feedback_removed: 0,
            auto_replay_live_memory_feedback_missing: 0,
            auto_replay_live_memory_feedback_strength_delta: 0.42,
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
        });
        let passing_report = summary.evaluate(&gate);

        assert!(passing_report.passed, "{:?}", passing_report.failures);
        assert_eq!(summary.total_auto_replay_live_memory_feedback_items(), 1);
        assert_eq!(summary.total_auto_replay_live_memory_feedback_updates(), 2);
        assert_eq!(
            summary.total_auto_replay_live_memory_feedback_reinforcements(),
            2
        );
        assert_eq!(
            summary.total_auto_replay_live_memory_feedback_detail_items(),
            1
        );
        assert_eq!(summary.total_auto_replay_live_memory_feedback_applied(), 2);
        assert!(
            (summary.total_auto_replay_live_memory_feedback_strength_delta() - 0.42).abs()
                < f32::EPSILON
        );
        assert!(
            summary
                .summary_line()
                .contains("auto_replay_live_memory_feedback_updates=2")
        );
        assert!(
            summary
                .summary_line()
                .contains("auto_replay_live_memory_feedback_detail_items=1")
        );
    }

    #[test]
    fn gate_reports_threshold_failures() {
        let mut engine = NoironEngine::new();
        let mut backend = HeuristicBackend;
        let case = BenchmarkCase::new("coding", TaskProfile::Coding, "Rust gate failure test");
        let outcome = engine.infer(
            InferenceRequest::new(case.prompt.clone(), case.profile),
            &mut backend,
        );
        let mut summary = BenchmarkSummary::new();
        let gate = BenchmarkGate {
            min_average_quality: 1.10,
            min_average_reward: 1.10,
            max_total_elapsed_ms: Some(1),
            max_case_recursive_chunks: Some(0),
            ..BenchmarkGate::default()
        };

        summary.record(&case, 7, &outcome);
        let report = summary.evaluate(&gate);

        assert!(!report.passed);
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("average_quality"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("average_reward"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("total_elapsed_ms"))
        );
    }

    #[test]
    fn gate_reports_missing_recursive_coverage() {
        let mut engine = NoironEngine::new();
        let mut backend = HeuristicBackend;
        let case = BenchmarkCase::new("short", TaskProfile::General, "Short benchmark");
        let outcome = engine.infer(
            InferenceRequest::new(case.prompt.clone(), case.profile),
            &mut backend,
        );
        let mut summary = BenchmarkSummary::new();
        let mut gate = BenchmarkGate::default();
        gate.min_recursive_cases = Some(1);
        gate.min_recursive_runtime_calls = Some(2);

        summary.record(&case, 1, &outcome);
        let report = summary.evaluate(&gate);

        assert!(!report.passed);
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("recursive_cases"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("recursive_runtime_calls"))
        );
    }

    #[test]
    fn gate_reports_missing_reflection_diagnostics_coverage() {
        let mut engine = NoironEngine::new();
        let mut backend = HeuristicBackend;
        let case = BenchmarkCase::new(
            "reflection_gate",
            TaskProfile::General,
            "Explain how reflection gates prove closed-loop control evidence.",
        );
        let outcome = engine.infer(
            InferenceRequest::new(case.prompt.clone(), case.profile),
            &mut backend,
        );
        let mut summary = BenchmarkSummary::new();
        summary.record(&case, 1, &outcome);
        summary.reflection_evidence = BenchmarkReflectionEvidence::default();
        let mut gate = BenchmarkGate::default();
        gate.min_reflection_issue_cases = Some(2);
        gate.min_reflection_issues = Some(3);
        gate.min_critical_reflection_issue_cases = Some(1);
        gate.min_critical_reflection_issues = Some(1);
        gate.min_revision_action_cases = Some(1);
        gate.min_revision_actions = Some(2);
        gate.min_reflection_issue_device_profiles = Some(1);
        gate.min_critical_reflection_issue_device_profiles = Some(1);
        gate.min_revision_action_device_profiles = Some(1);

        let report = summary.evaluate(&gate);

        assert!(!report.passed);
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("reflection_issue_cases"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("critical_reflection_issues"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("revision_actions"))
        );

        let mut passing = summary.clone();
        passing.reflection_evidence = BenchmarkReflectionEvidence {
            issue_cases: 2,
            total_issues: 3,
            critical_issue_cases: 1,
            total_critical_issues: 1,
            revision_action_cases: 1,
            total_revision_actions: 2,
            live_memory_feedback_reinforcements: 0,
            live_memory_feedback_penalties: 0,
            issue_devices: vec![DeviceClass::CpuOnly],
            critical_issue_devices: vec![DeviceClass::CpuOnly],
            revision_action_devices: vec![DeviceClass::CpuOnly],
            ..BenchmarkReflectionEvidence::default()
        };
        let passing_report = passing.evaluate(&gate);

        assert!(passing_report.passed, "{:?}", passing_report.failures);
        assert!(
            !passing_report
                .failures
                .iter()
                .any(|failure| failure.contains("reflection"))
        );
        assert!(
            !passing_report
                .failures
                .iter()
                .any(|failure| failure.contains("revision"))
        );
        assert!(passing.summary_line().contains("reflection_issue_cases=2"));
        assert!(passing.summary_line().contains("reflection_issues=3"));
        assert!(
            passing
                .summary_line()
                .contains("reflection_issue_device_profiles=1")
        );
        assert!(
            passing
                .summary_line()
                .contains("critical_reflection_issue_cases=1")
        );
        assert!(
            passing
                .summary_line()
                .contains("critical_reflection_issues=1")
        );
        assert!(
            passing
                .summary_line()
                .contains("critical_reflection_issue_device_profiles=1")
        );
        assert!(passing.summary_line().contains("revision_action_cases=1"));
        assert!(passing.summary_line().contains("revision_actions=2"));
        assert!(
            passing
                .summary_line()
                .contains("revision_action_device_profiles=1")
        );
    }

    #[test]
    fn gate_reports_auto_replay_recursive_pressure_failures() {
        let summary = BenchmarkSummary {
            reflection_evidence: BenchmarkReflectionEvidence::default(),
            live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
            memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
            embedding_evidence: BenchmarkEmbeddingEvidence::default(),

            runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),
            evolution_ledger: EvolutionLedger::default(),
            results: vec![BenchmarkCaseResult {
                name: "replay_pressure".to_owned(),
                profile: TaskProfile::LongDocument,
                device: DeviceClass::CpuOnly,
                elapsed_ms: 1,
                quality: 0.9,
                process_reward: 0.9,
                attention_fraction: 0.5,
                requires_recursion: true,
                recursive_chunks: 4,
                recursive_waves: 2,
                recursive_runtime_calls: 7,
                auto_replay_applied: 1,
                auto_replay_router_updates: 1,
                auto_replay_hierarchy_updates: 1,
                auto_replay_router_threshold_mutations: 0,
                auto_replay_hierarchy_weight_mutations: 0,
                auto_replay_router_threshold_delta: 0.0,
                auto_replay_hierarchy_weight_delta: 0.0,
                auto_replay_memory_reinforcements: 1,
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
                auto_replay_recursive_runtime_items: 1,
                auto_replay_recursive_runtime_calls: 96,
                auto_replay_avg_recursive_call_pressure: 0.35,
                auto_replay_max_recursive_call_pressure: 0.35,
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
                runtime_adapter_observations: 0,
                runtime_selected_adapter: None,
                runtime_adapter_contract_ok: false,
                runtime_adapter_contract_violations: 0,
                runtime_adapter_best_score: None,
                runtime_adapter_best_adapter: None,
                runtime_adapter_selection_mismatches: 0,
                query_embedding_source: "fallback".to_owned(),
                query_embedding_dimensions: 64,
                runtime_embedding_calls: 0,
                fallback_embedding_calls: 1,
                embedding_fallback_used: true,
                drift_severity: DriftSeverity::Stable,
            }],
        };
        let mut gate = BenchmarkGate::default();
        gate.min_auto_replay_recursive_items = Some(2);
        gate.max_auto_replay_recursive_call_pressure = Some(0.10);

        let report = summary.evaluate(&gate);

        assert!(!report.passed);
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("auto_replay_recursive_items"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("auto_replay_recursive_call_pressure"))
        );
    }

    #[test]
    fn gate_reports_missing_auto_replay_recursive_pressure() {
        let summary = BenchmarkSummary {
            reflection_evidence: BenchmarkReflectionEvidence::default(),
            live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
            memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
            embedding_evidence: BenchmarkEmbeddingEvidence::default(),

            runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),
            evolution_ledger: EvolutionLedger::default(),
            results: vec![BenchmarkCaseResult {
                name: "missing_replay_pressure".to_owned(),
                profile: TaskProfile::LongDocument,
                device: DeviceClass::CpuOnly,
                elapsed_ms: 1,
                quality: 0.9,
                process_reward: 0.9,
                attention_fraction: 0.5,
                requires_recursion: true,
                recursive_chunks: 4,
                recursive_waves: 2,
                recursive_runtime_calls: 7,
                auto_replay_applied: 1,
                auto_replay_router_updates: 1,
                auto_replay_hierarchy_updates: 1,
                auto_replay_router_threshold_mutations: 0,
                auto_replay_hierarchy_weight_mutations: 0,
                auto_replay_router_threshold_delta: 0.0,
                auto_replay_hierarchy_weight_delta: 0.0,
                auto_replay_memory_reinforcements: 1,
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
                auto_replay_recursive_runtime_items: 1,
                auto_replay_recursive_runtime_calls: 7,
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
                runtime_adapter_observations: 0,
                runtime_selected_adapter: None,
                runtime_adapter_contract_ok: false,
                runtime_adapter_contract_violations: 0,
                runtime_adapter_best_score: None,
                runtime_adapter_best_adapter: None,
                runtime_adapter_selection_mismatches: 0,
                query_embedding_source: "fallback".to_owned(),
                query_embedding_dimensions: 64,
                runtime_embedding_calls: 0,
                fallback_embedding_calls: 1,
                embedding_fallback_used: true,
                drift_severity: DriftSeverity::Stable,
            }],
        };
        let mut gate = BenchmarkGate::default();
        gate.min_auto_replay_recursive_call_pressure = Some(0.01);

        let report = summary.evaluate(&gate);

        assert!(!report.passed);
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("below minimum"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("auto_replay_recursive_call_pressure"))
        );
    }

    #[test]
    fn gate_reports_missing_auto_replay_control_plane_coverage() {
        let summary = BenchmarkSummary {
            reflection_evidence: BenchmarkReflectionEvidence::default(),
            live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
            memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
            embedding_evidence: BenchmarkEmbeddingEvidence::default(),

            runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),
            evolution_ledger: EvolutionLedger::default(),
            results: vec![BenchmarkCaseResult {
                name: "auto_replay_control_plane".to_owned(),
                profile: TaskProfile::Coding,
                device: DeviceClass::CpuOnly,
                elapsed_ms: 1,
                quality: 0.9,
                process_reward: 0.9,
                attention_fraction: 0.5,
                requires_recursion: false,
                recursive_chunks: 1,
                recursive_waves: 1,
                recursive_runtime_calls: 1,
                auto_replay_applied: 1,
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
                runtime_adapter_observations: 0,
                runtime_selected_adapter: None,
                runtime_adapter_contract_ok: false,
                runtime_adapter_contract_violations: 0,
                runtime_adapter_best_score: None,
                runtime_adapter_best_adapter: None,
                runtime_adapter_selection_mismatches: 0,
                query_embedding_source: "fallback".to_owned(),
                query_embedding_dimensions: 64,
                runtime_embedding_calls: 0,
                fallback_embedding_calls: 1,
                embedding_fallback_used: true,
                drift_severity: DriftSeverity::Stable,
            }],
        };
        let mut gate = BenchmarkGate::default();
        gate.min_auto_replay_router_updates = Some(1);
        gate.min_auto_replay_hierarchy_updates = Some(1);
        gate.min_auto_replay_router_threshold_mutations = Some(1);
        gate.min_auto_replay_hierarchy_weight_mutations = Some(1);
        gate.min_auto_replay_router_threshold_delta = Some(0.01);
        gate.min_auto_replay_hierarchy_weight_delta = Some(0.01);
        gate.min_auto_replay_memory_updates = Some(1);

        let report = summary.evaluate(&gate);

        assert!(!report.passed);
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("auto_replay_router_updates"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("auto_replay_hierarchy_updates"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("auto_replay_router_threshold_mutations"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("auto_replay_hierarchy_weight_mutations"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("auto_replay_router_threshold_delta"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("auto_replay_hierarchy_weight_delta"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("auto_replay_memory_updates"))
        );

        let passing = BenchmarkSummary {
            reflection_evidence: BenchmarkReflectionEvidence::default(),
            live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
            memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
            embedding_evidence: BenchmarkEmbeddingEvidence::default(),

            runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),
            evolution_ledger: EvolutionLedger::default(),
            results: vec![BenchmarkCaseResult {
                auto_replay_router_updates: 1,
                auto_replay_hierarchy_updates: 1,
                auto_replay_router_threshold_mutations: 1,
                auto_replay_hierarchy_weight_mutations: 1,
                auto_replay_router_threshold_delta: 0.02,
                auto_replay_hierarchy_weight_delta: 0.03,
                auto_replay_memory_reinforcements: 1,
                ..summary.results[0].clone()
            }],
        };
        let passing_report = passing.evaluate(&gate);

        assert!(passing_report.passed, "{:?}", passing_report.failures);
        assert_eq!(passing.total_auto_replay_router_updates(), 1);
        assert_eq!(passing.total_auto_replay_hierarchy_updates(), 1);
        assert_eq!(passing.total_auto_replay_router_threshold_mutations(), 1);
        assert_eq!(passing.total_auto_replay_hierarchy_weight_mutations(), 1);
        assert!(passing.total_auto_replay_router_threshold_delta() >= 0.02);
        assert!(passing.total_auto_replay_hierarchy_weight_delta() >= 0.03);
        assert_eq!(passing.total_auto_replay_memory_updates(), 1);
        assert!(
            passing
                .summary_line()
                .contains("auto_replay_router_threshold_mutations=1")
        );
        assert!(
            passing
                .summary_line()
                .contains("auto_replay_hierarchy_weight_mutations=1")
        );
        assert!(
            passing
                .summary_line()
                .contains("auto_replay_router_threshold_delta=0.020000")
        );
        assert!(
            passing
                .summary_line()
                .contains("auto_replay_hierarchy_weight_delta=0.030000")
        );
        assert!(
            passing
                .summary_line()
                .contains("auto_replay_memory_updates=1")
        );
    }

    #[test]
    fn gate_reports_missing_evolution_ledger_coverage() {
        let summary = BenchmarkSummary {
            reflection_evidence: BenchmarkReflectionEvidence::default(),
            live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
            memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
            embedding_evidence: BenchmarkEmbeddingEvidence::default(),

            runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),
            evolution_ledger: EvolutionLedger::default(),
            results: vec![BenchmarkCaseResult {
                name: "evolution_ledger".to_owned(),
                profile: TaskProfile::LongDocument,
                device: DeviceClass::CpuOnly,
                elapsed_ms: 1,
                quality: 0.9,
                process_reward: 0.9,
                attention_fraction: 0.5,
                requires_recursion: true,
                recursive_chunks: 4,
                recursive_waves: 2,
                recursive_runtime_calls: 7,
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
                infini_local_window: 1,
                infini_global_memory: 1,
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
                runtime_adapter_observations: 0,
                runtime_selected_adapter: None,
                runtime_adapter_contract_ok: false,
                runtime_adapter_contract_violations: 0,
                runtime_adapter_best_score: None,
                runtime_adapter_best_adapter: None,
                runtime_adapter_selection_mismatches: 0,
                query_embedding_source: "fallback".to_owned(),
                query_embedding_dimensions: 64,
                runtime_embedding_calls: 0,
                fallback_embedding_calls: 1,
                embedding_fallback_used: true,
                drift_severity: DriftSeverity::Stable,
            }],
        };
        let mut gate = BenchmarkGate::default();
        gate.min_evolution_live_inference_runs = Some(1);
        gate.min_evolution_live_router_threshold_mutations = Some(1);
        gate.min_evolution_live_hierarchy_weight_mutations = Some(1);
        gate.min_evolution_live_router_threshold_delta = Some(0.01);
        gate.min_evolution_live_hierarchy_weight_delta = Some(0.01);
        gate.min_evolution_live_online_reward_feedbacks = Some(1);
        gate.min_evolution_live_online_reward_reinforcements = Some(1);
        gate.min_evolution_live_online_reward_penalties = Some(1);
        gate.min_evolution_live_online_reward_strength = Some(1.0);
        gate.min_evolution_live_online_reward_reinforcement_strength = Some(0.6);
        gate.min_evolution_live_online_reward_penalty_strength = Some(0.4);
        gate.min_evolution_live_memory_updates = Some(1);
        gate.min_evolution_live_stored_memory_updates = Some(1);
        gate.min_evolution_live_reflection_issues = Some(1);
        gate.min_evolution_live_critical_reflection_issues = Some(1);
        gate.min_evolution_live_revision_actions = Some(1);
        gate.min_evolution_replay_runs = Some(1);
        gate.min_evolution_replay_items = Some(2);
        gate.min_evolution_router_threshold_mutations = Some(3);
        gate.min_evolution_hierarchy_weight_mutations = Some(4);
        gate.min_evolution_router_threshold_delta = Some(0.05);
        gate.min_evolution_hierarchy_weight_delta = Some(0.06);
        gate.min_evolution_memory_updates = Some(5);
        gate.min_evolution_replay_live_memory_feedback_updates = Some(3);
        gate.min_evolution_replay_live_memory_feedback_detail_items = Some(2);
        gate.min_evolution_replay_live_memory_feedback_applied = Some(4);
        gate.min_evolution_replay_live_memory_feedback_strength_delta = Some(0.42);
        gate.min_evolution_replay_live_evolution_items = Some(2);
        gate.min_evolution_replay_live_evolution_online_reward_feedbacks = Some(2);
        gate.min_evolution_replay_live_evolution_online_reward_reinforcements = Some(1);
        gate.min_evolution_replay_live_evolution_online_reward_penalties = Some(1);
        gate.min_evolution_replay_live_evolution_online_reward_strength = Some(1.0);
        gate.min_evolution_replay_live_evolution_online_reward_reinforcement_strength = Some(0.6);
        gate.min_evolution_replay_live_evolution_online_reward_penalty_strength = Some(0.4);
        gate.min_evolution_replay_live_evolution_memory_updates = Some(3);
        gate.min_evolution_replay_live_evolution_stored_memory_updates = Some(2);
        gate.min_evolution_replay_live_evolution_reflection_issues = Some(2);
        gate.min_evolution_replay_live_evolution_critical_reflection_issues = Some(1);
        gate.min_evolution_replay_live_evolution_revision_actions = Some(2);
        gate.min_evolution_recursive_replay_items = Some(6);
        gate.min_evolution_recursive_runtime_calls = Some(7);
        gate.max_evolution_drift_rollbacks = Some(0);
        gate.max_evolution_rollback_router_threshold_delta = Some(0.0);
        gate.max_evolution_rollback_hierarchy_weight_delta = Some(0.0);

        let report = summary.evaluate(&gate);

        assert!(!report.passed);
        for marker in [
            "evolution_live_inference_runs",
            "evolution_live_router_threshold_mutations",
            "evolution_live_hierarchy_weight_mutations",
            "evolution_live_router_threshold_delta",
            "evolution_live_hierarchy_weight_delta",
            "evolution_live_online_reward_feedbacks",
            "evolution_live_online_reward_reinforcements",
            "evolution_live_online_reward_penalties",
            "evolution_live_online_reward_strength",
            "evolution_live_online_reward_reinforcement_strength",
            "evolution_live_online_reward_penalty_strength",
            "evolution_live_memory_updates",
            "evolution_live_stored_memory_updates",
            "evolution_live_reflection_issues",
            "evolution_live_critical_reflection_issues",
            "evolution_live_revision_actions",
            "evolution_replay_runs",
            "evolution_replay_items",
            "evolution_router_threshold_mutations",
            "evolution_hierarchy_weight_mutations",
            "evolution_router_threshold_delta",
            "evolution_hierarchy_weight_delta",
            "evolution_memory_updates",
            "evolution_replay_live_memory_feedback_updates",
            "evolution_replay_live_memory_feedback_detail_items",
            "evolution_replay_live_memory_feedback_applied",
            "evolution_replay_live_memory_feedback_strength_delta",
            "evolution_replay_live_evolution_items",
            "evolution_replay_live_evolution_online_reward_feedbacks",
            "evolution_replay_live_evolution_online_reward_reinforcements",
            "evolution_replay_live_evolution_online_reward_penalties",
            "evolution_replay_live_evolution_online_reward_strength",
            "evolution_replay_live_evolution_online_reward_reinforcement_strength",
            "evolution_replay_live_evolution_online_reward_penalty_strength",
            "evolution_replay_live_evolution_memory_updates",
            "evolution_replay_live_evolution_stored_memory_updates",
            "evolution_replay_live_evolution_reflection_issues",
            "evolution_replay_live_evolution_critical_reflection_issues",
            "evolution_replay_live_evolution_revision_actions",
            "evolution_recursive_replay_items",
            "evolution_recursive_runtime_calls",
        ] {
            assert!(
                report
                    .failures
                    .iter()
                    .any(|failure| failure.contains(marker)),
                "missing failure marker {marker}: {:?}",
                report.failures
            );
        }

        let passing = BenchmarkSummary {
            reflection_evidence: BenchmarkReflectionEvidence::default(),
            live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
            memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
            embedding_evidence: BenchmarkEmbeddingEvidence::default(),

            runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),
            evolution_ledger: EvolutionLedger {
                live_inference_runs: 8,
                live_router_threshold_mutations: 2,
                live_hierarchy_weight_mutations: 1,
                live_router_threshold_delta: 0.07,
                live_hierarchy_weight_delta: 0.04,
                live_online_reward_feedbacks: 3,
                live_online_reward_reinforcements: 2,
                live_online_reward_penalties: 1,
                live_online_reward_strength: 1.80,
                live_online_reward_reinforcement_strength: 1.20,
                live_online_reward_penalty_strength: 0.60,
                live_memory_reinforcements: 3,
                live_memory_penalties: 2,
                live_stored_memories: 1,
                live_stored_gist_memories: 2,
                live_stored_runtime_kv_memories: 1,
                live_reflection_issues: 4,
                live_critical_reflection_issues: 1,
                live_revision_actions: 5,
                replay_runs: 1,
                replay_items: 2,
                router_threshold_mutations: 3,
                hierarchy_weight_mutations: 4,
                router_threshold_delta: 0.05,
                hierarchy_weight_delta: 0.06,
                memory_reinforcements: 5,
                memory_penalties: 0,
                replay_live_memory_feedback_items: 2,
                replay_live_memory_feedback_reinforcements: 2,
                replay_live_memory_feedback_penalties: 1,
                replay_live_memory_feedback_detail_items: 2,
                replay_live_memory_feedback_applied: 4,
                replay_live_memory_feedback_removed: 1,
                replay_live_memory_feedback_missing: 1,
                replay_live_memory_feedback_strength_delta: 0.42,
                replay_live_evolution_items: 2,
                replay_live_evolution_router_threshold_mutations: 1,
                replay_live_evolution_hierarchy_weight_mutations: 1,
                replay_live_evolution_router_threshold_delta: 0.04,
                replay_live_evolution_hierarchy_weight_delta: 0.03,
                replay_live_evolution_online_reward_feedbacks: 2,
                replay_live_evolution_online_reward_reinforcements: 1,
                replay_live_evolution_online_reward_penalties: 1,
                replay_live_evolution_online_reward_strength: 1.30,
                replay_live_evolution_online_reward_reinforcement_strength: 0.80,
                replay_live_evolution_online_reward_penalty_strength: 0.50,
                replay_live_evolution_memory_updates: 3,
                replay_live_evolution_stored_memory_updates: 2,
                replay_live_evolution_reflection_issues: 2,
                replay_live_evolution_critical_reflection_issues: 1,
                replay_live_evolution_revision_actions: 2,
                recursive_replay_items: 6,
                recursive_runtime_calls: 7,
                drift_rollbacks: 0,
                rollback_router_threshold_delta: 0.0,
                rollback_hierarchy_weight_delta: 0.0,
            },
            results: summary.results.clone(),
        };
        let passing_report = passing.evaluate(&gate);

        assert!(passing_report.passed, "{:?}", passing_report.failures);
        let summary_line = passing.summary_line();
        assert!(summary_line.contains("evolution_live_online_reward_strength=1.800000"));
        assert!(
            summary_line.contains("evolution_live_online_reward_reinforcement_strength=1.200000")
        );
        assert!(summary_line.contains("evolution_live_online_reward_penalty_strength=0.600000"));
        assert!(
            summary_line
                .contains("evolution_replay_live_evolution_online_reward_strength=1.300000")
        );
        assert!(summary_line.contains(
            "evolution_replay_live_evolution_online_reward_reinforcement_strength=0.800000"
        ));
        assert!(
            summary_line.contains(
                "evolution_replay_live_evolution_online_reward_penalty_strength=0.500000"
            )
        );
        assert_eq!(passing.evolution_ledger().replay_runs, 1);
        assert_eq!(passing.evolution_ledger().live_inference_runs, 8);
        assert_eq!(passing.evolution_ledger().live_online_reward_feedbacks, 3);
        assert_eq!(
            passing
                .evolution_ledger()
                .replay_live_evolution_online_reward_feedbacks,
            2
        );
        assert_eq!(passing.evolution_ledger().live_memory_updates(), 5);
        assert_eq!(passing.evolution_ledger().live_stored_memory_updates(), 4);
        assert_eq!(passing.evolution_ledger().memory_updates(), 5);
        assert_eq!(
            passing
                .evolution_ledger()
                .replay_live_memory_feedback_updates(),
            3
        );
        assert!(passing.summary_line().contains("evolution_replay_runs=1"));
        assert!(
            passing
                .summary_line()
                .contains("evolution_live_inference_runs=8")
        );
        assert!(
            passing
                .summary_line()
                .contains("evolution_live_online_reward_feedbacks=3")
        );
        assert!(
            passing
                .summary_line()
                .contains("evolution_live_memory_updates=5")
        );
        assert!(
            passing
                .summary_line()
                .contains("evolution_live_stored_memory_updates=4")
        );
        assert!(
            passing
                .summary_line()
                .contains("evolution_replay_live_memory_feedback_updates=3")
        );
        assert!(
            passing
                .summary_line()
                .contains("evolution_replay_live_memory_feedback_detail_items=2")
        );
        assert!(
            passing
                .summary_line()
                .contains("evolution_replay_live_memory_feedback_applied=4")
        );
        assert!(
            passing
                .summary_line()
                .contains("evolution_replay_live_memory_feedback_strength_delta=0.420000")
        );
        assert!(
            passing
                .summary_line()
                .contains("evolution_replay_live_evolution_items=2")
        );
        assert!(
            passing
                .summary_line()
                .contains("evolution_replay_live_evolution_online_reward_feedbacks=2")
        );
        assert!(
            passing
                .summary_line()
                .contains("evolution_replay_live_evolution_memory_updates=3")
        );
        assert!(
            passing
                .summary_line()
                .contains("evolution_replay_live_evolution_stored_memory_updates=2")
        );
        assert!(
            passing
                .summary_line()
                .contains("evolution_replay_live_evolution_reflection_issues=2")
        );
        assert!(
            passing
                .summary_line()
                .contains("evolution_replay_live_evolution_critical_reflection_issues=1")
        );
        assert!(
            passing
                .summary_line()
                .contains("evolution_replay_live_evolution_revision_actions=2")
        );
        assert!(
            passing
                .summary_line()
                .contains("evolution_router_threshold_delta=0.050000")
        );
        assert!(
            passing
                .summary_line()
                .contains("evolution_recursive_runtime_calls=7")
        );
    }

    #[test]
    fn gate_reports_evolution_ledger_drift_rollback_failures() {
        let summary = BenchmarkSummary {
            reflection_evidence: BenchmarkReflectionEvidence::default(),
            live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
            memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
            embedding_evidence: BenchmarkEmbeddingEvidence::default(),

            runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),
            evolution_ledger: EvolutionLedger {
                drift_rollbacks: 2,
                rollback_router_threshold_delta: 0.03,
                rollback_hierarchy_weight_delta: 0.04,
                ..EvolutionLedger::default()
            },
            results: vec![BenchmarkCaseResult {
                name: "evolution_rollback_audit".to_owned(),
                profile: TaskProfile::General,
                device: DeviceClass::CpuOnly,
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
                runtime_adapter_observations: 0,
                runtime_selected_adapter: None,
                runtime_adapter_contract_ok: false,
                runtime_adapter_contract_violations: 0,
                runtime_adapter_best_score: None,
                runtime_adapter_best_adapter: None,
                runtime_adapter_selection_mismatches: 0,
                query_embedding_source: "fallback".to_owned(),
                query_embedding_dimensions: 64,
                runtime_embedding_calls: 0,
                fallback_embedding_calls: 1,
                embedding_fallback_used: true,
                drift_severity: DriftSeverity::Stable,
            }],
        };

        let report = summary.evaluate(&BenchmarkGate::default());

        assert!(!report.passed);
        for marker in [
            "evolution_drift_rollbacks",
            "evolution_rollback_router_threshold_delta",
            "evolution_rollback_hierarchy_weight_delta",
        ] {
            assert!(
                report
                    .failures
                    .iter()
                    .any(|failure| failure.contains(marker)),
                "missing failure marker {marker}: {:?}",
                report.failures
            );
        }
    }

    #[test]
    fn gate_reports_missing_runtime_forward_and_kv_export() {
        let summary = BenchmarkSummary {
            reflection_evidence: BenchmarkReflectionEvidence::default(),
            live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
            memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
            embedding_evidence: BenchmarkEmbeddingEvidence::default(),

            runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),
            evolution_ledger: EvolutionLedger::default(),
            results: vec![BenchmarkCaseResult {
                name: "runtime_boundary".to_owned(),
                profile: TaskProfile::Coding,
                device: DeviceClass::CpuOnly,
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
                runtime_adapter_observations: 0,
                runtime_selected_adapter: None,
                runtime_adapter_contract_ok: false,
                runtime_adapter_contract_violations: 0,
                runtime_adapter_best_score: None,
                runtime_adapter_best_adapter: None,
                runtime_adapter_selection_mismatches: 0,
                query_embedding_source: "fallback".to_owned(),
                query_embedding_dimensions: 64,
                runtime_embedding_calls: 0,
                fallback_embedding_calls: 1,
                embedding_fallback_used: true,
                drift_severity: DriftSeverity::Stable,
            }],
        };
        let mut gate = BenchmarkGate::default();
        gate.min_runtime_forward_cases = Some(1);
        gate.min_runtime_kv_exported = Some(1);
        gate.min_runtime_kv_export_device_profiles = Some(1);

        let report = summary.evaluate(&gate);

        assert!(!report.passed);
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("runtime_forward_cases"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("runtime_kv_exported"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("runtime_kv_export_device_profiles"))
        );

        let passing = BenchmarkSummary {
            reflection_evidence: BenchmarkReflectionEvidence::default(),
            live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
            memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
            embedding_evidence: BenchmarkEmbeddingEvidence::default(),

            runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),
            evolution_ledger: EvolutionLedger::default(),
            results: vec![BenchmarkCaseResult {
                runtime_forward_signal: true,
                runtime_token_count: 0,
                runtime_uncertainty_token_count: 0,
                runtime_uncertainty_signal: false,
                runtime_kv_imported: 0,
                runtime_kv_exported: 2,
                ..summary.results[0].clone()
            }],
        };
        let passing_report = passing.evaluate(&gate);

        assert!(passing_report.passed, "{:?}", passing_report.failures);
        assert_eq!(passing.runtime_forward_cases(), 1);
        assert_eq!(passing.total_runtime_kv_exported(), 2);
        assert_eq!(passing.runtime_kv_export_device_profiles(), 1);
        assert_eq!(passing.runtime_kv_export_devices_csv(), "cpu");
        assert!(passing.summary_line().contains("runtime_forward_cases=1"));
        assert!(passing.summary_line().contains("runtime_kv_exported=2"));
        assert!(
            passing
                .summary_line()
                .contains("runtime_kv_export_device_profiles=1")
        );
        assert!(
            passing
                .summary_line()
                .contains("runtime_kv_export_devices=cpu")
        );
    }

    #[test]
    fn gate_reports_missing_runtime_kv_export_device_profile_coverage() {
        let base = BenchmarkCaseResult {
            name: "runtime_export".to_owned(),
            profile: TaskProfile::Coding,
            device: DeviceClass::CpuOnly,
            elapsed_ms: 1,
            quality: 0.7,
            process_reward: 0.7,
            attention_fraction: 0.03,
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
            stored_memories: 1,
            compacted_memories: 0,
            runtime_forward_signal: true,
            runtime_forward_energy_signal: true,
            runtime_kv_influence_signal: true,
            runtime_global_layers: 0,
            runtime_local_window_layers: 0,
            runtime_convolutional_fusion_layers: 0,
            runtime_layer_mode_signal: false,
            runtime_all_layer_modes_signal: false,
            runtime_token_count: 1,
            runtime_uncertainty_token_count: 1,
            runtime_uncertainty_signal: true,
            runtime_kv_imported: 1,
            runtime_kv_exported: 2,
            runtime_kv_stored: 1,
            runtime_selected_adapter: Some("portable-rust".to_owned()),
            runtime_adapter_contract_ok: true,
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
        };
        let summary = BenchmarkSummary {
            results: vec![base.clone()],
            ..BenchmarkSummary::default()
        };
        let mut gate = BenchmarkGate::default();
        gate.min_runtime_kv_export_device_profiles = Some(2);

        let report = summary.evaluate(&gate);

        assert!(!report.passed);
        assert_eq!(summary.runtime_kv_export_device_profiles(), 1);
        assert!(report.failures.iter().any(|failure| {
            failure.contains("runtime_kv_export_device_profiles 1 below minimum 2")
                && failure.contains("devices=cpu")
        }));

        let passing = BenchmarkSummary {
            results: vec![
                base.clone(),
                BenchmarkCaseResult {
                    device: DeviceClass::IntegratedGpu,
                    ..base
                },
            ],
            ..BenchmarkSummary::default()
        };
        let passing_report = passing.evaluate(&gate);

        assert!(passing_report.passed, "{:?}", passing_report.failures);
        assert_eq!(passing.runtime_kv_export_device_profiles(), 2);
        assert_eq!(passing.runtime_kv_export_devices_csv(), "cpu+integrated");
    }

    #[test]
    fn gate_reports_missing_runtime_forward_diagnostics() {
        let summary = BenchmarkSummary {
            reflection_evidence: BenchmarkReflectionEvidence::default(),
            live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
            memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
            embedding_evidence: BenchmarkEmbeddingEvidence::default(),

            runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),
            evolution_ledger: EvolutionLedger::default(),
            results: vec![BenchmarkCaseResult {
                name: "runtime_diagnostics".to_owned(),
                profile: TaskProfile::Coding,
                device: DeviceClass::CpuOnly,
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
                runtime_token_count: 0,
                runtime_uncertainty_token_count: 0,
                runtime_uncertainty_signal: false,
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
                query_embedding_source: "fallback".to_owned(),
                query_embedding_dimensions: 64,
                runtime_embedding_calls: 0,
                fallback_embedding_calls: 1,
                embedding_fallback_used: true,
                drift_severity: DriftSeverity::Stable,
            }],
        };
        let mut gate = BenchmarkGate::default();
        gate.min_runtime_forward_energy_cases = Some(1);
        gate.min_runtime_kv_influence_cases = Some(1);

        let report = summary.evaluate(&gate);

        assert!(!report.passed);
        assert_eq!(summary.runtime_forward_energy_cases(), 0);
        assert_eq!(summary.runtime_kv_influence_cases(), 0);
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("runtime_forward_energy_cases"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("runtime_kv_influence_cases"))
        );

        let passing = BenchmarkSummary {
            reflection_evidence: BenchmarkReflectionEvidence::default(),
            live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
            memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
            embedding_evidence: BenchmarkEmbeddingEvidence::default(),

            runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),
            evolution_ledger: EvolutionLedger::default(),
            results: vec![BenchmarkCaseResult {
                runtime_forward_energy_signal: true,
                runtime_kv_influence_signal: true,
                runtime_global_layers: 0,
                runtime_local_window_layers: 0,
                runtime_convolutional_fusion_layers: 0,
                runtime_layer_mode_signal: false,
                runtime_all_layer_modes_signal: false,
                ..summary.results[0].clone()
            }],
        };
        let passing_report = passing.evaluate(&gate);

        assert!(passing_report.passed, "{:?}", passing_report.failures);
        assert_eq!(passing.runtime_forward_energy_cases(), 1);
        assert_eq!(passing.runtime_kv_influence_cases(), 1);
        assert!(
            passing
                .summary_line()
                .contains("runtime_forward_energy_cases=1")
        );
        assert!(
            passing
                .summary_line()
                .contains("runtime_kv_influence_cases=1")
        );
    }

    #[test]
    fn gate_reports_missing_runtime_transformer_layer_modes() {
        let summary = BenchmarkSummary {
            reflection_evidence: BenchmarkReflectionEvidence::default(),
            live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
            memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
            embedding_evidence: BenchmarkEmbeddingEvidence::default(),

            runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),
            evolution_ledger: EvolutionLedger::default(),
            results: vec![BenchmarkCaseResult {
                name: "runtime_layer_modes".to_owned(),
                profile: TaskProfile::Coding,
                device: DeviceClass::CpuOnly,
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
                runtime_forward_energy_signal: true,
                runtime_kv_influence_signal: true,
                runtime_global_layers: 0,
                runtime_local_window_layers: 0,
                runtime_convolutional_fusion_layers: 0,
                runtime_layer_mode_signal: false,
                runtime_all_layer_modes_signal: false,
                runtime_token_count: 1,
                runtime_uncertainty_token_count: 1,
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
                query_embedding_source: "fallback".to_owned(),
                query_embedding_dimensions: 64,
                runtime_embedding_calls: 0,
                fallback_embedding_calls: 1,
                embedding_fallback_used: true,
                drift_severity: DriftSeverity::Stable,
            }],
        };
        let gate = BenchmarkGate {
            min_runtime_layer_mode_cases: Some(1),
            min_runtime_all_layer_mode_cases: Some(1),
            min_runtime_global_layers: Some(2),
            min_runtime_local_window_layers: Some(3),
            min_runtime_convolutional_fusion_layers: Some(1),
            ..BenchmarkGate::default()
        };

        let report = summary.evaluate(&gate);

        assert!(!report.passed);
        assert_eq!(summary.runtime_layer_mode_cases(), 0);
        assert_eq!(summary.runtime_all_layer_mode_cases(), 0);
        assert_eq!(summary.total_runtime_global_layers(), 0);
        assert_eq!(summary.total_runtime_local_window_layers(), 0);
        assert_eq!(summary.total_runtime_convolutional_fusion_layers(), 0);
        assert!(
            report
                .failures
                .iter()
                .any(|failure| { failure.contains("runtime_layer_mode_cases") })
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| { failure.contains("runtime_all_layer_mode_cases") })
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| { failure.contains("runtime_global_layers") })
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| { failure.contains("runtime_local_window_layers") })
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| { failure.contains("runtime_convolutional_fusion_layers") })
        );

        let passing = BenchmarkSummary {
            reflection_evidence: BenchmarkReflectionEvidence::default(),
            live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
            memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
            embedding_evidence: BenchmarkEmbeddingEvidence::default(),

            runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),
            evolution_ledger: EvolutionLedger::default(),
            results: vec![BenchmarkCaseResult {
                runtime_global_layers: 2,
                runtime_local_window_layers: 3,
                runtime_convolutional_fusion_layers: 1,
                runtime_layer_mode_signal: true,
                runtime_all_layer_modes_signal: true,
                ..summary.results[0].clone()
            }],
        };
        let passing_report = passing.evaluate(&gate);

        assert!(passing_report.passed, "{:?}", passing_report.failures);
        assert_eq!(passing.runtime_layer_mode_cases(), 1);
        assert_eq!(passing.runtime_all_layer_mode_cases(), 1);
        assert_eq!(passing.total_runtime_global_layers(), 2);
        assert_eq!(passing.total_runtime_local_window_layers(), 3);
        assert_eq!(passing.total_runtime_convolutional_fusion_layers(), 1);
        assert!(
            passing
                .summary_line()
                .contains("runtime_layer_mode_cases=1")
        );
        assert!(
            passing
                .summary_line()
                .contains("runtime_all_layer_mode_cases=1")
        );
        assert!(passing.summary_line().contains("runtime_global_layers=2"));
        assert!(
            passing
                .summary_line()
                .contains("runtime_local_window_layers=3")
        );
        assert!(
            passing
                .summary_line()
                .contains("runtime_convolutional_fusion_layers=1")
        );
    }

    #[test]
    fn gate_reports_missing_runtime_uncertainty_signal() {
        let summary = BenchmarkSummary {
            reflection_evidence: BenchmarkReflectionEvidence::default(),
            live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
            memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
            embedding_evidence: BenchmarkEmbeddingEvidence::default(),

            runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),
            evolution_ledger: EvolutionLedger::default(),
            results: vec![BenchmarkCaseResult {
                name: "runtime_uncertainty".to_owned(),
                profile: TaskProfile::Coding,
                device: DeviceClass::CpuOnly,
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
                runtime_token_count: 3,
                runtime_uncertainty_token_count: 0,
                runtime_uncertainty_signal: false,
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
                query_embedding_source: "fallback".to_owned(),
                query_embedding_dimensions: 64,
                runtime_embedding_calls: 0,
                fallback_embedding_calls: 1,
                embedding_fallback_used: true,
                drift_severity: DriftSeverity::Stable,
            }],
        };
        let mut gate = BenchmarkGate::default();
        gate.min_runtime_uncertainty_cases = Some(1);
        gate.min_runtime_uncertainty_tokens = Some(2);
        gate.min_runtime_uncertainty_device_profiles = Some(1);
        gate.min_runtime_uncertainty_token_device_profiles = Some(1);

        let report = summary.evaluate(&gate);

        assert!(!report.passed);
        assert_eq!(summary.runtime_token_cases(), 1);
        assert_eq!(summary.total_runtime_tokens(), 3);
        assert_eq!(summary.runtime_uncertainty_cases(), 0);
        assert_eq!(summary.total_runtime_uncertainty_tokens(), 0);
        assert_eq!(summary.runtime_uncertainty_device_profiles(), 0);
        assert_eq!(summary.runtime_uncertainty_token_device_profiles(), 0);
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("runtime_uncertainty_cases"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("runtime_uncertainty_tokens"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("runtime_uncertainty_device_profiles"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("runtime_uncertainty_token_device_profiles"))
        );

        let passing = BenchmarkSummary {
            reflection_evidence: BenchmarkReflectionEvidence::default(),
            live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
            memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
            embedding_evidence: BenchmarkEmbeddingEvidence::default(),

            runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),
            evolution_ledger: EvolutionLedger::default(),
            results: vec![BenchmarkCaseResult {
                runtime_uncertainty_token_count: 3,
                runtime_uncertainty_signal: true,
                ..summary.results[0].clone()
            }],
        };
        let passing_report = passing.evaluate(&gate);

        assert!(passing_report.passed, "{:?}", passing_report.failures);
        assert_eq!(passing.runtime_uncertainty_cases(), 1);
        assert_eq!(passing.total_runtime_uncertainty_tokens(), 3);
        assert_eq!(passing.runtime_uncertainty_device_profiles(), 1);
        assert_eq!(passing.runtime_uncertainty_devices_csv(), "cpu");
        assert_eq!(passing.runtime_uncertainty_token_device_profiles(), 1);
        assert_eq!(passing.runtime_uncertainty_token_devices_csv(), "cpu");
        assert!(
            passing
                .summary_line()
                .contains("runtime_uncertainty_cases=1")
        );
        assert!(
            passing
                .summary_line()
                .contains("runtime_uncertainty_tokens=3")
        );
        assert!(
            passing
                .summary_line()
                .contains("runtime_uncertainty_device_profiles=1")
        );
        assert!(
            passing
                .summary_line()
                .contains("runtime_uncertainty_devices=cpu")
        );
        assert!(
            passing
                .summary_line()
                .contains("runtime_uncertainty_token_device_profiles=1")
        );
        assert!(
            passing
                .summary_line()
                .contains("runtime_uncertainty_token_devices=cpu")
        );
    }

    #[test]
    fn gate_reports_missing_runtime_uncertainty_device_profile_coverage() {
        let base = runtime_uncertainty_result(DeviceClass::CpuOnly);
        let summary = BenchmarkSummary {
            results: vec![base.clone()],
            ..BenchmarkSummary::default()
        };
        let mut gate = BenchmarkGate::default();
        gate.min_runtime_uncertainty_device_profiles = Some(2);

        let report = summary.evaluate(&gate);

        assert!(!report.passed);
        assert_eq!(summary.runtime_uncertainty_device_profiles(), 1);
        assert_eq!(summary.runtime_uncertainty_devices_csv(), "cpu");
        assert!(report.failures.iter().any(|failure| {
            failure.contains("runtime_uncertainty_device_profiles 1 below minimum 2")
                && failure.contains("devices=cpu")
        }));

        let passing = BenchmarkSummary {
            results: vec![
                base.clone(),
                BenchmarkCaseResult {
                    device: DeviceClass::IntegratedGpu,
                    ..base
                },
            ],
            ..BenchmarkSummary::default()
        };
        let passing_report = passing.evaluate(&gate);

        assert!(passing_report.passed, "{:?}", passing_report.failures);
        assert_eq!(passing.runtime_uncertainty_device_profiles(), 2);
        assert_eq!(passing.runtime_uncertainty_devices_csv(), "cpu+integrated");
        assert!(
            passing
                .summary_line()
                .contains("runtime_uncertainty_device_profiles=2")
        );
        assert!(
            passing
                .summary_line()
                .contains("runtime_uncertainty_devices=cpu+integrated")
        );
    }

    #[test]
    fn gate_reports_missing_runtime_uncertainty_token_device_profile_coverage() {
        let signal_only = BenchmarkCaseResult {
            runtime_uncertainty_token_count: 0,
            runtime_uncertainty_signal: true,
            ..runtime_uncertainty_result(DeviceClass::CpuOnly)
        };
        let token_backed = runtime_uncertainty_result(DeviceClass::IntegratedGpu);
        let summary = BenchmarkSummary {
            results: vec![signal_only.clone(), token_backed.clone()],
            ..BenchmarkSummary::default()
        };
        let mut gate = BenchmarkGate::default();
        gate.min_runtime_uncertainty_device_profiles = Some(2);
        gate.min_runtime_uncertainty_token_device_profiles = Some(2);

        let report = summary.evaluate(&gate);

        assert!(!report.passed);
        assert_eq!(summary.runtime_uncertainty_device_profiles(), 2);
        assert_eq!(summary.runtime_uncertainty_devices_csv(), "cpu+integrated");
        assert_eq!(summary.runtime_uncertainty_token_device_profiles(), 1);
        assert_eq!(
            summary.runtime_uncertainty_token_devices_csv(),
            "integrated"
        );
        assert!(
            !report
                .failures
                .iter()
                .any(|failure| failure.contains("runtime_uncertainty_device_profiles 1"))
        );
        assert!(report.failures.iter().any(|failure| {
            failure.contains("runtime_uncertainty_token_device_profiles 1 below minimum 2")
                && failure.contains("devices=integrated")
        }));

        let passing = BenchmarkSummary {
            results: vec![
                BenchmarkCaseResult {
                    runtime_uncertainty_token_count: 2,
                    ..signal_only
                },
                token_backed,
            ],
            ..BenchmarkSummary::default()
        };
        let passing_report = passing.evaluate(&gate);

        assert!(passing_report.passed, "{:?}", passing_report.failures);
        assert_eq!(passing.runtime_uncertainty_token_device_profiles(), 2);
        assert_eq!(
            passing.runtime_uncertainty_token_devices_csv(),
            "cpu+integrated"
        );
        assert!(
            passing
                .summary_line()
                .contains("runtime_uncertainty_token_device_profiles=2")
        );
        assert!(
            passing
                .summary_line()
                .contains("runtime_uncertainty_token_devices=cpu+integrated")
        );
    }

    #[test]
    fn gate_reports_missing_runtime_kv_import() {
        let summary = BenchmarkSummary {
            reflection_evidence: BenchmarkReflectionEvidence::default(),
            live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
            memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
            embedding_evidence: BenchmarkEmbeddingEvidence::default(),

            runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),
            evolution_ledger: EvolutionLedger::default(),
            results: vec![BenchmarkCaseResult {
                name: "runtime_import".to_owned(),
                profile: TaskProfile::Coding,
                device: DeviceClass::CpuOnly,
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
                runtime_token_count: 0,
                runtime_uncertainty_token_count: 0,
                runtime_uncertainty_signal: false,
                runtime_kv_imported: 0,
                runtime_kv_exported: 1,
                runtime_kv_stored: 1,
                runtime_selected_adapter: Some("portable-rust".to_owned()),
                runtime_adapter_contract_ok: true,
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
            }],
        };
        let mut gate = BenchmarkGate::default();
        gate.min_runtime_kv_import_cases = Some(1);
        gate.min_runtime_kv_imported = Some(2);
        gate.min_runtime_kv_import_device_profiles = Some(1);

        let report = summary.evaluate(&gate);

        assert!(!report.passed);
        assert_eq!(summary.runtime_kv_import_cases(), 0);
        assert_eq!(summary.total_runtime_kv_imported(), 0);
        assert_eq!(summary.runtime_kv_import_device_profiles(), 0);
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("runtime_kv_import_cases"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("runtime_kv_imported"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("runtime_kv_import_device_profiles"))
        );

        let passing = BenchmarkSummary {
            reflection_evidence: BenchmarkReflectionEvidence::default(),
            live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
            memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
            embedding_evidence: BenchmarkEmbeddingEvidence::default(),

            runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),
            evolution_ledger: EvolutionLedger::default(),
            results: vec![BenchmarkCaseResult {
                runtime_kv_imported: 3,
                ..summary.results[0].clone()
            }],
        };
        let passing_report = passing.evaluate(&gate);

        assert!(passing_report.passed, "{:?}", passing_report.failures);
        assert_eq!(passing.runtime_kv_import_cases(), 1);
        assert_eq!(passing.total_runtime_kv_imported(), 3);
        assert_eq!(passing.runtime_kv_import_device_profiles(), 1);
        assert_eq!(passing.runtime_kv_import_devices_csv(), "cpu");
        assert!(passing.summary_line().contains("runtime_kv_import_cases=1"));
        assert!(passing.summary_line().contains("runtime_kv_imported=3"));
        assert!(
            passing
                .summary_line()
                .contains("runtime_kv_import_device_profiles=1")
        );
        assert!(
            passing
                .summary_line()
                .contains("runtime_kv_import_devices=cpu")
        );
    }

    #[test]
    fn gate_reports_missing_runtime_kv_import_device_profile_coverage() {
        let base = BenchmarkCaseResult {
            name: "runtime_import".to_owned(),
            profile: TaskProfile::Coding,
            device: DeviceClass::CpuOnly,
            elapsed_ms: 1,
            quality: 0.7,
            process_reward: 0.7,
            attention_fraction: 0.03,
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
            stored_memories: 1,
            compacted_memories: 0,
            runtime_forward_signal: true,
            runtime_forward_energy_signal: true,
            runtime_kv_influence_signal: true,
            runtime_global_layers: 0,
            runtime_local_window_layers: 0,
            runtime_convolutional_fusion_layers: 0,
            runtime_layer_mode_signal: false,
            runtime_all_layer_modes_signal: false,
            runtime_token_count: 1,
            runtime_uncertainty_token_count: 1,
            runtime_uncertainty_signal: true,
            runtime_kv_imported: 2,
            runtime_kv_exported: 1,
            runtime_kv_stored: 1,
            runtime_selected_adapter: Some("portable-rust".to_owned()),
            runtime_adapter_contract_ok: true,
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
        };
        let summary = BenchmarkSummary {
            results: vec![base.clone()],
            ..BenchmarkSummary::default()
        };
        let mut gate = BenchmarkGate::default();
        gate.min_runtime_kv_import_device_profiles = Some(2);

        let report = summary.evaluate(&gate);

        assert!(!report.passed);
        assert_eq!(summary.runtime_kv_import_device_profiles(), 1);
        assert!(report.failures.iter().any(|failure| {
            failure.contains("runtime_kv_import_device_profiles 1 below minimum 2")
                && failure.contains("devices=cpu")
        }));

        let passing = BenchmarkSummary {
            results: vec![
                base.clone(),
                BenchmarkCaseResult {
                    device: DeviceClass::IntegratedGpu,
                    ..base
                },
            ],
            ..BenchmarkSummary::default()
        };
        let passing_report = passing.evaluate(&gate);

        assert!(passing_report.passed, "{:?}", passing_report.failures);
        assert_eq!(passing.runtime_kv_import_device_profiles(), 2);
        assert_eq!(passing.runtime_kv_import_devices_csv(), "cpu+integrated");
    }

    #[test]
    fn gate_reports_missing_runtime_kv_storage() {
        let summary = BenchmarkSummary {
            reflection_evidence: BenchmarkReflectionEvidence::default(),
            live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
            memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
            embedding_evidence: BenchmarkEmbeddingEvidence::default(),

            runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),
            evolution_ledger: EvolutionLedger::default(),
            results: vec![BenchmarkCaseResult {
                name: "runtime_storage".to_owned(),
                profile: TaskProfile::Coding,
                device: DeviceClass::CpuOnly,
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
                runtime_forward_energy_signal: true,
                runtime_kv_influence_signal: true,
                runtime_global_layers: 0,
                runtime_local_window_layers: 0,
                runtime_convolutional_fusion_layers: 0,
                runtime_layer_mode_signal: false,
                runtime_all_layer_modes_signal: false,
                runtime_token_count: 1,
                runtime_uncertainty_token_count: 1,
                runtime_uncertainty_signal: true,
                runtime_kv_imported: 1,
                runtime_kv_exported: 2,
                runtime_kv_stored: 0,
                runtime_selected_adapter: Some("portable-rust".to_owned()),
                runtime_adapter_contract_ok: true,
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
            }],
        };
        let mut gate = BenchmarkGate::default();
        gate.min_runtime_kv_stored = Some(1);
        gate.min_runtime_kv_stored_device_profiles = Some(1);

        let report = summary.evaluate(&gate);

        assert!(!report.passed);
        assert_eq!(summary.total_runtime_kv_stored(), 0);
        assert_eq!(summary.runtime_kv_stored_device_profiles(), 0);
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("runtime_kv_stored"))
        );
        assert!(report.failures.iter().any(|failure| {
            failure.contains("runtime_kv_stored_device_profiles")
                && failure.contains("devices=none")
        }));

        let passing = BenchmarkSummary {
            reflection_evidence: BenchmarkReflectionEvidence::default(),
            live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
            memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
            embedding_evidence: BenchmarkEmbeddingEvidence::default(),

            runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),
            evolution_ledger: EvolutionLedger::default(),
            results: vec![BenchmarkCaseResult {
                runtime_kv_stored: 2,
                ..summary.results[0].clone()
            }],
        };
        let passing_report = passing.evaluate(&gate);

        assert!(passing_report.passed, "{:?}", passing_report.failures);
        assert_eq!(passing.total_runtime_kv_stored(), 2);
        assert_eq!(passing.runtime_kv_stored_device_profiles(), 1);
        assert_eq!(passing.runtime_kv_stored_devices_csv(), "cpu");
        assert!(passing.summary_line().contains("runtime_kv_stored=2"));
        assert!(
            passing
                .summary_line()
                .contains("runtime_kv_stored_device_profiles=1")
        );
        assert!(
            passing
                .summary_line()
                .contains("runtime_kv_stored_devices=cpu")
        );
    }

    #[test]
    fn gate_reports_missing_runtime_kv_stored_device_profile_coverage() {
        let base = BenchmarkCaseResult {
            name: "runtime_storage".to_owned(),
            profile: TaskProfile::Coding,
            device: DeviceClass::CpuOnly,
            elapsed_ms: 1,
            quality: 0.7,
            process_reward: 0.7,
            attention_fraction: 0.03,
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
            stored_memories: 1,
            compacted_memories: 0,
            runtime_forward_signal: true,
            runtime_forward_energy_signal: true,
            runtime_kv_influence_signal: true,
            runtime_global_layers: 0,
            runtime_local_window_layers: 0,
            runtime_convolutional_fusion_layers: 0,
            runtime_layer_mode_signal: false,
            runtime_all_layer_modes_signal: false,
            runtime_token_count: 1,
            runtime_uncertainty_token_count: 1,
            runtime_uncertainty_signal: true,
            runtime_kv_imported: 1,
            runtime_kv_exported: 2,
            runtime_kv_stored: 1,
            runtime_selected_adapter: Some("portable-rust".to_owned()),
            runtime_adapter_contract_ok: true,
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
        };
        let summary = BenchmarkSummary {
            results: vec![base.clone()],
            ..BenchmarkSummary::default()
        };
        let mut gate = BenchmarkGate::default();
        gate.min_runtime_kv_stored_device_profiles = Some(2);

        let report = summary.evaluate(&gate);

        assert!(!report.passed);
        assert_eq!(summary.runtime_kv_stored_device_profiles(), 1);
        assert!(report.failures.iter().any(|failure| {
            failure.contains("runtime_kv_stored_device_profiles 1 below minimum 2")
                && failure.contains("devices=cpu")
        }));

        let passing = BenchmarkSummary {
            results: vec![
                base.clone(),
                BenchmarkCaseResult {
                    device: DeviceClass::IntegratedGpu,
                    ..base
                },
            ],
            ..BenchmarkSummary::default()
        };
        let passing_report = passing.evaluate(&gate);

        assert!(passing_report.passed, "{:?}", passing_report.failures);
        assert_eq!(passing.runtime_kv_stored_device_profiles(), 2);
        assert_eq!(passing.runtime_kv_stored_devices_csv(), "cpu+integrated");
    }

    #[test]
    fn gate_reports_missing_runtime_kv_hold_evidence() {
        let summary = BenchmarkSummary {
            reflection_evidence: BenchmarkReflectionEvidence::default(),
            live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
            memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
            embedding_evidence: BenchmarkEmbeddingEvidence::default(),

            runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),
            evolution_ledger: EvolutionLedger::default(),
            results: vec![BenchmarkCaseResult {
                name: "runtime_hold".to_owned(),
                profile: TaskProfile::Coding,
                device: DeviceClass::CpuOnly,
                elapsed_ms: 1,
                quality: 0.7,
                process_reward: 0.7,
                attention_fraction: 0.03,
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
                stored_memories: 1,
                compacted_memories: 0,
                runtime_forward_signal: true,
                runtime_forward_energy_signal: true,
                runtime_kv_influence_signal: true,
                runtime_global_layers: 0,
                runtime_local_window_layers: 0,
                runtime_convolutional_fusion_layers: 0,
                runtime_layer_mode_signal: false,
                runtime_all_layer_modes_signal: false,
                runtime_token_count: 1,
                runtime_uncertainty_token_count: 1,
                runtime_uncertainty_signal: true,
                runtime_kv_imported: 0,
                runtime_kv_exported: 2,
                runtime_kv_stored: 2,
                runtime_selected_adapter: Some("portable-rust".to_owned()),
                runtime_adapter_contract_ok: true,
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
            }],
        };
        let mut gate = BenchmarkGate::default();
        gate.min_runtime_kv_hold_cases = Some(1);
        gate.min_runtime_kv_held = Some(1);
        gate.min_runtime_kv_hold_device_profiles = Some(1);

        let report = summary.evaluate(&gate);

        assert!(!report.passed);
        assert_eq!(summary.runtime_kv_hold_cases(), 0);
        assert_eq!(summary.total_runtime_kv_held(), 0);
        assert_eq!(summary.runtime_kv_hold_device_profiles(), 0);
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("runtime_kv_hold_cases"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("runtime_kv_held"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("runtime_kv_hold_device_profiles"))
        );

        let passing = BenchmarkSummary {
            results: vec![BenchmarkCaseResult {
                runtime_kv_stored: 0,
                drift_severity: DriftSeverity::Watch,
                ..summary.results[0].clone()
            }],
            ..BenchmarkSummary::default()
        };
        let passing_report = passing.evaluate(&gate);

        assert!(passing_report.passed, "{:?}", passing_report.failures);
        assert_eq!(passing.runtime_kv_hold_cases(), 1);
        assert_eq!(passing.total_runtime_kv_held(), 2);
        assert_eq!(passing.runtime_kv_hold_device_profiles(), 1);
        assert_eq!(passing.runtime_kv_hold_devices_csv(), "cpu");
        assert!(passing.summary_line().contains("runtime_kv_hold_cases=1"));
        assert!(passing.summary_line().contains("runtime_kv_held=2"));
        assert!(
            passing
                .summary_line()
                .contains("runtime_kv_hold_device_profiles=1")
        );
        assert!(
            passing
                .summary_line()
                .contains("runtime_kv_hold_devices=cpu")
        );
    }

    #[test]
    fn gate_reports_missing_runtime_kv_hold_device_profile_coverage() {
        let base = BenchmarkCaseResult {
            name: "runtime_hold".to_owned(),
            profile: TaskProfile::Coding,
            device: DeviceClass::CpuOnly,
            elapsed_ms: 1,
            quality: 0.7,
            process_reward: 0.7,
            attention_fraction: 0.03,
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
            stored_memories: 1,
            compacted_memories: 0,
            runtime_forward_signal: true,
            runtime_forward_energy_signal: true,
            runtime_kv_influence_signal: true,
            runtime_global_layers: 0,
            runtime_local_window_layers: 0,
            runtime_convolutional_fusion_layers: 0,
            runtime_layer_mode_signal: false,
            runtime_all_layer_modes_signal: false,
            runtime_token_count: 1,
            runtime_uncertainty_token_count: 1,
            runtime_uncertainty_signal: true,
            runtime_kv_imported: 0,
            runtime_kv_exported: 2,
            runtime_kv_stored: 0,
            runtime_selected_adapter: Some("portable-rust".to_owned()),
            runtime_adapter_contract_ok: true,
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
            drift_severity: DriftSeverity::Watch,
        };
        let summary = BenchmarkSummary {
            results: vec![base.clone()],
            ..BenchmarkSummary::default()
        };
        let mut gate = BenchmarkGate::default();
        gate.min_runtime_kv_hold_device_profiles = Some(2);

        let report = summary.evaluate(&gate);

        assert!(!report.passed);
        assert_eq!(summary.runtime_kv_hold_device_profiles(), 1);
        assert!(report.failures.iter().any(|failure| {
            failure.contains("runtime_kv_hold_device_profiles 1 below minimum 2")
                && failure.contains("devices=cpu")
        }));

        let passing = BenchmarkSummary {
            results: vec![
                base.clone(),
                BenchmarkCaseResult {
                    device: DeviceClass::IntegratedGpu,
                    ..base
                },
            ],
            ..BenchmarkSummary::default()
        };
        let passing_report = passing.evaluate(&gate);

        assert!(passing_report.passed, "{:?}", passing_report.failures);
        assert_eq!(passing.runtime_kv_hold_device_profiles(), 2);
        assert_eq!(passing.runtime_kv_hold_devices_csv(), "cpu+integrated");
    }

    #[test]
    fn gate_reports_runtime_adapter_contract_failures() {
        let summary = BenchmarkSummary {
            reflection_evidence: BenchmarkReflectionEvidence::default(),
            live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
            memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
            embedding_evidence: BenchmarkEmbeddingEvidence::default(),

            runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),
            evolution_ledger: EvolutionLedger::default(),
            results: vec![
                BenchmarkCaseResult {
                    name: "contract_ok".to_owned(),
                    profile: TaskProfile::Coding,
                    device: DeviceClass::CpuOnly,
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
                    runtime_token_count: 0,
                    runtime_uncertainty_token_count: 0,
                    runtime_uncertainty_signal: false,
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
                    query_embedding_source: "fallback".to_owned(),
                    query_embedding_dimensions: 64,
                    runtime_embedding_calls: 0,
                    fallback_embedding_calls: 1,
                    embedding_fallback_used: true,
                    drift_severity: DriftSeverity::Stable,
                },
                BenchmarkCaseResult {
                    name: "contract_bad".to_owned(),
                    profile: TaskProfile::Coding,
                    device: DeviceClass::CpuOnly,
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
                    runtime_token_count: 0,
                    runtime_uncertainty_token_count: 0,
                    runtime_uncertainty_signal: false,
                    runtime_kv_imported: 0,
                    runtime_kv_exported: 1,
                    runtime_kv_stored: 1,
                    runtime_selected_adapter: Some("cuda".to_owned()),
                    runtime_adapter_contract_ok: false,
                    runtime_adapter_contract_violations: 1,
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
                },
            ],
        };
        let mut gate = BenchmarkGate::default();
        gate.min_runtime_adapter_contract_cases = Some(2);
        gate.min_runtime_adapter_kinds = Some(2);
        gate.max_runtime_adapter_contract_violations = Some(0);

        let report = summary.evaluate(&gate);

        assert!(!report.passed);
        assert_eq!(summary.runtime_adapter_contract_cases(), 1);
        assert_eq!(summary.runtime_adapter_kinds(), 1);
        assert_eq!(summary.total_runtime_adapter_contract_violations(), 1);
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("runtime_adapter_contract_cases"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("runtime_adapter_kinds"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("runtime_adapter_contract_violations"))
        );
        assert!(
            summary
                .summary_line()
                .contains("runtime_adapter_contract_cases=1")
        );
        assert!(summary.summary_line().contains("runtime_adapter_kinds=1"));
        assert!(
            summary
                .summary_line()
                .contains("runtime_adapter_contract_violations=1")
        );
    }

    #[test]
    fn runtime_embedding_evidence_records_model_side_vectors() {
        struct EmbeddingBackend;

        impl InferenceBackend for EmbeddingBackend {
            fn embed_text(&mut self, text: &str) -> Option<Vec<f32>> {
                Some(vec![1.0, text.len() as f32, 0.25, 0.75])
            }

            fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
                InferenceDraft::new(
                    "A stable Rust Noiron embedding benchmark answer stores runtime model-side vectors.",
                    vec![ReasoningStep::new(
                        "embedding",
                        "runtime embedding path is available",
                        0.91,
                    )],
                )
            }
        }

        let mut engine = NoironEngine::new();
        engine.set_hardware_snapshot(crate::hardware::HardwareSnapshot::new(
            DeviceClass::CpuOnly,
            0.20,
            0.0,
            0.30,
            0.10,
        ));
        let mut backend = EmbeddingBackend;
        let case = BenchmarkCase::new(
            "runtime_embedding",
            TaskProfile::Coding,
            "Benchmark runtime embedding evidence.",
        );
        let outcome = engine.infer(
            InferenceRequest::new(case.prompt.clone(), case.profile),
            &mut backend,
        );
        let mut summary = BenchmarkSummary::new();

        summary.record(&case, 1, &outcome);

        assert_eq!(summary.runtime_embedding_cases(), 1);
        assert_eq!(summary.runtime_embedding_device_profiles(), 1);
        assert_eq!(summary.embedding_fallback_cases(), 0);
        assert!(summary.total_runtime_embedding_calls() >= 1);
        assert_eq!(summary.total_fallback_embedding_calls(), 0);
        assert_eq!(summary.total_embedding_evidence_failures(), 0);
        assert!(summary.summary_line().contains("runtime_embedding_cases=1"));

        let gate = BenchmarkGate {
            min_runtime_embedding_cases: Some(1),
            min_runtime_embedding_device_profiles: Some(1),
            max_embedding_fallback_cases: Some(0),
            ..BenchmarkGate::default()
        };
        let report = summary.evaluate(&gate);

        assert!(report.passed, "{:?}", report.failures);
    }

    #[test]
    fn embedding_gate_reports_fallback_over_limit() {
        let mut engine = NoironEngine::new();
        let mut backend = HeuristicBackend;
        let case = BenchmarkCase::new(
            "fallback_embedding",
            TaskProfile::General,
            "Benchmark fallback embedding evidence.",
        );
        let outcome = engine.infer(
            InferenceRequest::new(case.prompt.clone(), case.profile),
            &mut backend,
        );
        let mut summary = BenchmarkSummary::new();
        summary.record(&case, 1, &outcome);
        let gate = BenchmarkGate {
            max_embedding_fallback_cases: Some(0),
            ..BenchmarkGate::default()
        };

        let report = summary.evaluate(&gate);

        assert!(!report.passed);
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("embedding_fallback_cases")),
            "{:?}",
            report.failures
        );
    }

    #[test]
    fn gate_reports_runtime_adapter_kind_collapse() {
        let summary = BenchmarkSummary {
            reflection_evidence: BenchmarkReflectionEvidence::default(),
            live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
            memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
            embedding_evidence: BenchmarkEmbeddingEvidence::default(),

            runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),
            evolution_ledger: EvolutionLedger::default(),
            results: vec![
                BenchmarkCaseResult {
                    name: "cpu".to_owned(),
                    profile: TaskProfile::General,
                    device: DeviceClass::CpuOnly,
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
                    runtime_token_count: 0,
                    runtime_uncertainty_token_count: 0,
                    runtime_uncertainty_signal: false,
                    runtime_kv_imported: 0,
                    runtime_kv_exported: 1,
                    runtime_kv_stored: 1,
                    runtime_selected_adapter: Some("portable-rust".to_owned()),
                    runtime_adapter_contract_ok: true,
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
                },
                BenchmarkCaseResult {
                    name: "gpu".to_owned(),
                    device: DeviceClass::DiscreteGpu,
                    ..BenchmarkCaseResult {
                        name: "template".to_owned(),
                        profile: TaskProfile::General,
                        device: DeviceClass::CpuOnly,
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
                        runtime_token_count: 0,
                        runtime_uncertainty_token_count: 0,
                        runtime_uncertainty_signal: false,
                        runtime_kv_imported: 0,
                        runtime_kv_exported: 1,
                        runtime_kv_stored: 1,
                        runtime_selected_adapter: Some("portable-rust".to_owned()),
                        runtime_adapter_contract_ok: true,
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
                },
            ],
        };
        let mut gate = BenchmarkGate::default();
        gate.min_runtime_adapter_kinds = Some(2);

        let report = summary.evaluate(&gate);

        assert!(!report.passed);
        assert_eq!(summary.runtime_adapter_kinds(), 1);
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("runtime_adapter_kinds"))
        );

        let passing = BenchmarkSummary {
            reflection_evidence: BenchmarkReflectionEvidence::default(),
            live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
            memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
            embedding_evidence: BenchmarkEmbeddingEvidence::default(),

            runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),
            results: vec![
                summary.results[0].clone(),
                BenchmarkCaseResult {
                    runtime_selected_adapter: Some("cuda".to_owned()),
                    ..summary.results[1].clone()
                },
            ],
            ..summary.clone()
        };

        assert_eq!(passing.runtime_adapter_kinds(), 2);
        assert!(passing.evaluate(&gate).passed);
    }

    #[test]
    fn gate_reports_missing_runtime_adapter_observations() {
        let summary = BenchmarkSummary {
            reflection_evidence: BenchmarkReflectionEvidence::default(),
            live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
            memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
            embedding_evidence: BenchmarkEmbeddingEvidence::default(),

            runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),
            evolution_ledger: EvolutionLedger::default(),
            results: vec![BenchmarkCaseResult {
                name: "runtime_adapter_observation".to_owned(),
                profile: TaskProfile::Coding,
                device: DeviceClass::CpuOnly,
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
                runtime_forward_energy_signal: true,
                runtime_kv_influence_signal: true,
                runtime_global_layers: 0,
                runtime_local_window_layers: 0,
                runtime_convolutional_fusion_layers: 0,
                runtime_layer_mode_signal: false,
                runtime_all_layer_modes_signal: false,
                runtime_token_count: 1,
                runtime_uncertainty_token_count: 1,
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
                query_embedding_source: "fallback".to_owned(),
                query_embedding_dimensions: 64,
                runtime_embedding_calls: 0,
                fallback_embedding_calls: 1,
                embedding_fallback_used: true,
                drift_severity: DriftSeverity::Stable,
            }],
        };
        let mut gate = BenchmarkGate::default();
        gate.min_runtime_adapter_observations = Some(1);
        gate.min_runtime_adapter_best_score = Some(0.25);

        let report = summary.evaluate(&gate);

        assert!(!report.passed);
        assert_eq!(summary.total_runtime_adapter_observations(), 0);
        assert_eq!(summary.max_runtime_adapter_score(), None);
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("runtime_adapter_observations"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("runtime_adapter_best_score"))
        );

        let passing = BenchmarkSummary {
            reflection_evidence: BenchmarkReflectionEvidence::default(),
            live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
            memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
            embedding_evidence: BenchmarkEmbeddingEvidence::default(),

            runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),
            evolution_ledger: EvolutionLedger::default(),
            results: vec![BenchmarkCaseResult {
                runtime_adapter_observations: 2,
                runtime_adapter_best_score: Some(0.51),
                ..summary.results[0].clone()
            }],
        };
        let passing_report = passing.evaluate(&gate);

        assert!(passing_report.passed, "{:?}", passing_report.failures);
        assert_eq!(passing.total_runtime_adapter_observations(), 2);
        assert_eq!(passing.max_runtime_adapter_score(), Some(0.51));
        assert!(
            passing
                .summary_line()
                .contains("runtime_adapter_observations=2")
        );
        assert!(
            passing
                .summary_line()
                .contains("runtime_adapter_best_score=0.510")
        );
    }

    #[test]
    fn gate_reports_runtime_adapter_selection_mismatches() {
        let summary = BenchmarkSummary {
            reflection_evidence: BenchmarkReflectionEvidence::default(),
            live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
            memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
            embedding_evidence: BenchmarkEmbeddingEvidence::default(),

            runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),
            evolution_ledger: EvolutionLedger::default(),
            results: vec![BenchmarkCaseResult {
                name: "runtime_adapter_selection".to_owned(),
                profile: TaskProfile::Coding,
                device: DeviceClass::CpuOnly,
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
                runtime_forward_energy_signal: true,
                runtime_kv_influence_signal: true,
                runtime_global_layers: 0,
                runtime_local_window_layers: 0,
                runtime_convolutional_fusion_layers: 0,
                runtime_layer_mode_signal: false,
                runtime_all_layer_modes_signal: false,
                runtime_token_count: 1,
                runtime_uncertainty_token_count: 1,
                runtime_uncertainty_signal: true,
                runtime_kv_imported: 1,
                runtime_kv_exported: 1,
                runtime_kv_stored: 1,
                runtime_selected_adapter: Some("portable-rust".to_owned()),
                runtime_adapter_contract_ok: true,
                runtime_adapter_contract_violations: 0,
                runtime_adapter_observations: 1,
                runtime_adapter_best_score: Some(0.80),
                runtime_adapter_best_adapter: Some("cpu-simd".to_owned()),
                runtime_adapter_selection_mismatches: 1,
                query_embedding_source: "fallback".to_owned(),
                query_embedding_dimensions: 64,
                runtime_embedding_calls: 0,
                fallback_embedding_calls: 1,
                embedding_fallback_used: true,
                drift_severity: DriftSeverity::Stable,
            }],
        };
        let mut gate = BenchmarkGate::default();
        gate.max_runtime_adapter_selection_mismatches = Some(0);

        let report = summary.evaluate(&gate);

        assert!(!report.passed);
        assert_eq!(summary.total_runtime_adapter_selection_mismatches(), 1);
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("runtime_adapter_selection_mismatches"))
        );
        assert!(
            summary
                .summary_line()
                .contains("runtime_adapter_selection_mismatches=1")
        );

        let passing = BenchmarkSummary {
            results: vec![BenchmarkCaseResult {
                runtime_adapter_best_adapter: Some("portable-rust".to_owned()),
                runtime_adapter_selection_mismatches: 0,
                ..summary.results[0].clone()
            }],
            ..summary
        };
        let passing_report = passing.evaluate(&gate);

        assert!(passing_report.passed, "{:?}", passing_report.failures);
        assert_eq!(passing.total_runtime_adapter_selection_mismatches(), 0);
    }

    #[test]
    fn gate_reports_missing_sparse_filtering_coverage() {
        let summary = BenchmarkSummary {
            reflection_evidence: BenchmarkReflectionEvidence::default(),
            live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
            memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
            embedding_evidence: BenchmarkEmbeddingEvidence::default(),

            runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),
            evolution_ledger: EvolutionLedger::default(),
            results: vec![BenchmarkCaseResult {
                name: "sparse_filter".to_owned(),
                profile: TaskProfile::LongDocument,
                device: DeviceClass::CpuOnly,
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
                used_memories: 2,
                infini_local_window: 1,
                infini_global_memory: 1,
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
                runtime_adapter_observations: 0,
                runtime_selected_adapter: None,
                runtime_adapter_contract_ok: false,
                runtime_adapter_contract_violations: 0,
                runtime_adapter_best_score: None,
                runtime_adapter_best_adapter: None,
                runtime_adapter_selection_mismatches: 0,
                query_embedding_source: "fallback".to_owned(),
                query_embedding_dimensions: 64,
                runtime_embedding_calls: 0,
                fallback_embedding_calls: 1,
                embedding_fallback_used: true,
                drift_severity: DriftSeverity::Stable,
            }],
        };
        let mut gate = BenchmarkGate::default();
        gate.min_sparse_skipped_cases = Some(1);
        gate.min_sparse_skipped_tokens = Some(3);

        let report = summary.evaluate(&gate);

        assert!(!report.passed);
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("sparse_skipped_cases"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("sparse_skipped_tokens"))
        );

        let passing = BenchmarkSummary {
            reflection_evidence: BenchmarkReflectionEvidence::default(),
            live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
            memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
            embedding_evidence: BenchmarkEmbeddingEvidence::default(),

            runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),
            evolution_ledger: EvolutionLedger::default(),
            results: vec![BenchmarkCaseResult {
                sparse_skipped: 2,
                sparse_skipped_tokens: 7,
                ..summary.results[0].clone()
            }],
        };
        let passing_report = passing.evaluate(&gate);

        assert!(passing_report.passed, "{:?}", passing_report.failures);
        assert_eq!(passing.sparse_skipped_cases(), 1);
        assert_eq!(passing.total_sparse_skipped_tokens(), 7);
        assert!(passing.summary_line().contains("sparse_skipped_cases=1"));
        assert!(passing.summary_line().contains("sparse_skipped_tokens=7"));
    }

    #[test]
    fn gate_reports_missing_device_profile_coverage() {
        let base = BenchmarkCaseResult {
            name: "device_coverage".to_owned(),
            profile: TaskProfile::General,
            device: DeviceClass::CpuOnly,
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
            runtime_adapter_observations: 0,
            runtime_selected_adapter: None,
            runtime_adapter_contract_ok: false,
            runtime_adapter_contract_violations: 0,
            runtime_adapter_best_score: None,
            runtime_adapter_best_adapter: None,
            runtime_adapter_selection_mismatches: 0,
            query_embedding_source: "fallback".to_owned(),
            query_embedding_dimensions: 64,
            runtime_embedding_calls: 0,
            fallback_embedding_calls: 1,
            embedding_fallback_used: true,
            drift_severity: DriftSeverity::Stable,
        };
        let summary = BenchmarkSummary {
            reflection_evidence: BenchmarkReflectionEvidence::default(),
            live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
            memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
            embedding_evidence: BenchmarkEmbeddingEvidence::default(),

            runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),
            evolution_ledger: EvolutionLedger::default(),
            results: vec![base.clone()],
        };
        let mut gate = BenchmarkGate::default();
        gate.min_device_profiles = Some(DeviceClass::explicit_profiles().len());

        let report = summary.evaluate(&gate);

        assert!(!report.passed);
        assert_eq!(summary.explicit_device_profiles_covered(), 1);
        assert_eq!(
            summary.missing_explicit_device_profiles().len(),
            DeviceClass::explicit_profiles().len() - 1
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("device_profiles"))
        );

        let passing = BenchmarkSummary {
            reflection_evidence: BenchmarkReflectionEvidence::default(),
            live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
            memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
            embedding_evidence: BenchmarkEmbeddingEvidence::default(),

            runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),
            evolution_ledger: EvolutionLedger::default(),
            results: DeviceClass::explicit_profiles()
                .iter()
                .map(|device| BenchmarkCaseResult {
                    device: *device,
                    ..base.clone()
                })
                .collect(),
        };
        let passing_report = passing.evaluate(&gate);

        assert!(passing_report.passed, "{:?}", passing_report.failures);
        assert_eq!(
            passing.explicit_device_profiles_covered(),
            DeviceClass::explicit_profiles().len()
        );
        assert!(passing.summary_line().contains("device_profiles=12"));
        assert!(passing.summary_line().contains("devices=cpu+integrated"));
    }

    #[test]
    fn gate_reports_missing_recursive_device_profile_coverage() {
        let base = BenchmarkCaseResult {
            name: "recursive_device_coverage".to_owned(),
            profile: TaskProfile::LongDocument,
            device: DeviceClass::CpuOnly,
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
            runtime_adapter_observations: 0,
            runtime_selected_adapter: None,
            runtime_adapter_contract_ok: false,
            runtime_adapter_contract_violations: 0,
            runtime_adapter_best_score: None,
            runtime_adapter_best_adapter: None,
            runtime_adapter_selection_mismatches: 0,
            query_embedding_source: "fallback".to_owned(),
            query_embedding_dimensions: 64,
            runtime_embedding_calls: 0,
            fallback_embedding_calls: 1,
            embedding_fallback_used: true,
            drift_severity: DriftSeverity::Stable,
        };
        let mut gate = BenchmarkGate::default();
        gate.min_recursive_device_profiles = Some(DeviceClass::explicit_profiles().len());
        let summary = BenchmarkSummary {
            reflection_evidence: BenchmarkReflectionEvidence::default(),
            live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
            memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
            embedding_evidence: BenchmarkEmbeddingEvidence::default(),

            runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),
            evolution_ledger: EvolutionLedger::default(),
            results: DeviceClass::explicit_profiles()
                .iter()
                .map(|device| BenchmarkCaseResult {
                    device: *device,
                    ..base.clone()
                })
                .collect(),
        };

        let report = summary.evaluate(&gate);

        assert!(!report.passed);
        assert_eq!(summary.explicit_device_profiles_covered(), 12);
        assert_eq!(summary.recursive_device_profiles_covered(), 0);
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("recursive_device_profiles"))
        );

        let passing = BenchmarkSummary {
            reflection_evidence: BenchmarkReflectionEvidence::default(),
            live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
            memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
            embedding_evidence: BenchmarkEmbeddingEvidence::default(),

            runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),
            evolution_ledger: EvolutionLedger::default(),
            results: DeviceClass::explicit_profiles()
                .iter()
                .map(|device| BenchmarkCaseResult {
                    device: *device,
                    requires_recursion: true,
                    recursive_chunks: 2,
                    recursive_runtime_calls: 3,
                    ..base.clone()
                })
                .collect(),
        };
        let passing_report = passing.evaluate(&gate);

        assert!(passing_report.passed, "{:?}", passing_report.failures);
        assert_eq!(passing.recursive_device_profiles_covered(), 12);
        assert!(passing.missing_recursive_device_profiles().is_empty());
        assert!(
            passing
                .summary_line()
                .contains("recursive_device_profiles=12")
        );
        assert!(
            passing
                .summary_line()
                .contains("recursive_devices=cpu+integrated")
        );
    }

    #[test]
    fn gate_reports_drift_failures() {
        let summary = BenchmarkSummary {
            reflection_evidence: BenchmarkReflectionEvidence::default(),
            live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
            memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
            embedding_evidence: BenchmarkEmbeddingEvidence::default(),

            runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),
            evolution_ledger: EvolutionLedger::default(),
            results: vec![BenchmarkCaseResult {
                name: "drift".to_owned(),
                profile: TaskProfile::General,
                device: DeviceClass::CpuOnly,
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
                runtime_adapter_observations: 0,
                runtime_selected_adapter: None,
                runtime_adapter_contract_ok: false,
                runtime_adapter_contract_violations: 0,
                runtime_adapter_best_score: None,
                runtime_adapter_best_adapter: None,
                runtime_adapter_selection_mismatches: 0,
                query_embedding_source: "fallback".to_owned(),
                query_embedding_dimensions: 64,
                runtime_embedding_calls: 0,
                fallback_embedding_calls: 1,
                embedding_fallback_used: true,
                drift_severity: DriftSeverity::Rollback,
            }],
        };
        let report = summary.evaluate(&BenchmarkGate::default());

        assert!(!report.passed);
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("drift_rollbacks"))
        );
    }

    #[test]
    fn kv_quant_benchmark_default_gate_passes() {
        let summary = KvQuantBenchmarkSummary::run_default();
        let report = summary.evaluate(&KvQuantBenchmarkGate::default());

        assert_eq!(summary.len(), 6);
        assert!(summary.max_abs_error_for(QuantizationBits::Four) > 0.0);
        assert!(summary.max_abs_error_for(QuantizationBits::Eight) > 0.0);
        assert!(report.passed, "{:?}", report.failures);
        assert!(summary.summary_line().contains("kv_quant_benchmark"));
        assert!(report.summary_line().contains("passed=true"));
    }

    #[test]
    fn kv_quant_gate_reports_accuracy_and_compression_failures() {
        let mut summary = KvQuantBenchmarkSummary::default();
        summary.record("wide", QuantizationBits::Four, &[-1.0, 0.0, 1.0]);
        let gate = KvQuantBenchmarkGate {
            max_four_bit_abs_error: 0.0,
            max_four_bit_mean_error: 0.0,
            max_four_bit_compression_ratio: 0.01,
            max_eight_bit_abs_error: 1.0,
            max_eight_bit_mean_error: 1.0,
            max_eight_bit_compression_ratio: 1.0,
            max_total_elapsed_us: None,
        };

        let report = summary.evaluate(&gate);

        assert!(!report.passed);
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("q4_max_abs_error"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("q4_compression_ratio"))
        );
    }

    #[test]
    fn persistent_roundtrip_report_requires_reuse_and_runtime_kv_import() {
        let report = PersistentRoundtripReport::evaluate(PersistentRoundtripInput {
            first_stored_memory: true,
            first_runtime_kv_stored: 1,
            first_runtime_kv_namespace_preserved: true,
            second_used_memories: 2,
            second_used_runtime_kv_memory: true,
            second_used_experiences: 1,
            second_imported_runtime_kv_blocks: 2,
            second_imported_runtime_kv_from_namespace: true,
            second_runtime_adapter_observations: 1,
            second_runtime_adapter_best_score: Some(0.84),
            second_runtime_adapter_best_adapter: Some("portable-rust".to_owned()),
            second_runtime_selected_adapter: Some("portable-rust".to_owned()),
            second_quality: 0.82,
            first_drift_severity: DriftSeverity::Watch,
            second_drift_severity: DriftSeverity::Stable,
        });

        assert!(report.passed);
        assert!(report.summary_line().contains("passed=true"));
        assert!(
            report
                .summary_line()
                .contains("second_runtime_adapter_observations=1")
        );
        assert!(
            report
                .summary_line()
                .contains("second_imported_runtime_kv_from_namespace=true")
        );

        let failed = PersistentRoundtripReport::evaluate(PersistentRoundtripInput {
            first_stored_memory: false,
            first_runtime_kv_stored: 0,
            first_runtime_kv_namespace_preserved: false,
            second_used_memories: 0,
            second_used_runtime_kv_memory: false,
            second_used_experiences: 0,
            second_imported_runtime_kv_blocks: 0,
            second_imported_runtime_kv_from_namespace: false,
            second_runtime_adapter_observations: 0,
            second_runtime_adapter_best_score: None,
            second_runtime_adapter_best_adapter: None,
            second_runtime_selected_adapter: None,
            second_quality: 0.2,
            first_drift_severity: DriftSeverity::Stable,
            second_drift_severity: DriftSeverity::Block,
        });

        assert!(!failed.passed);
        assert!(failed.failures.len() >= 7);
        assert!(
            failed
                .failures
                .iter()
                .any(|failure| failure.contains("runtime_kv namespace"))
        );
        assert!(
            failed
                .failures
                .iter()
                .any(|failure| failure.contains("persisted runtime KV memory"))
        );
        assert!(
            failed
                .failures
                .iter()
                .any(|failure| failure.contains("adapter observations"))
        );
        assert!(
            failed
                .failures
                .iter()
                .any(|failure| failure.contains("best runtime adapter observation"))
        );
    }

    #[test]
    fn persistent_roundtrip_report_requires_observed_adapter_to_drive_second_runtime() {
        let report = PersistentRoundtripReport::evaluate(PersistentRoundtripInput {
            first_stored_memory: true,
            first_runtime_kv_stored: 1,
            first_runtime_kv_namespace_preserved: true,
            second_used_memories: 2,
            second_used_runtime_kv_memory: true,
            second_used_experiences: 1,
            second_imported_runtime_kv_blocks: 2,
            second_imported_runtime_kv_from_namespace: true,
            second_runtime_adapter_observations: 1,
            second_runtime_adapter_best_score: Some(0.80),
            second_runtime_adapter_best_adapter: Some("cpu-simd".to_owned()),
            second_runtime_selected_adapter: Some("portable-rust".to_owned()),
            second_quality: 0.82,
            first_drift_severity: DriftSeverity::Stable,
            second_drift_severity: DriftSeverity::Stable,
        });

        assert!(!report.passed);
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("selected adapter portable-rust"))
        );
        assert!(
            report
                .summary_line()
                .contains("second_runtime_adapter_best_adapter=cpu-simd")
        );
        assert!(
            report
                .summary_line()
                .contains("second_runtime_selected_adapter=portable-rust")
        );
    }

    #[test]
    fn persistent_roundtrip_matrix_requires_every_explicit_device_to_pass() {
        let passing_report = PersistentRoundtripReport::evaluate(PersistentRoundtripInput {
            first_stored_memory: true,
            first_runtime_kv_stored: 1,
            first_runtime_kv_namespace_preserved: true,
            second_used_memories: 2,
            second_used_runtime_kv_memory: true,
            second_used_experiences: 1,
            second_imported_runtime_kv_blocks: 1,
            second_imported_runtime_kv_from_namespace: true,
            second_runtime_adapter_observations: 1,
            second_runtime_adapter_best_score: Some(0.72),
            second_runtime_adapter_best_adapter: Some("portable-rust".to_owned()),
            second_runtime_selected_adapter: Some("portable-rust".to_owned()),
            second_quality: 0.80,
            first_drift_severity: DriftSeverity::Stable,
            second_drift_severity: DriftSeverity::Stable,
        });
        let complete = PersistentRoundtripMatrixReport::evaluate(
            DeviceClass::explicit_profiles()
                .iter()
                .copied()
                .map(|device| PersistentRoundtripDeviceReport {
                    device,
                    report: passing_report.clone(),
                })
                .collect(),
        );

        assert!(complete.passed, "{:?}", complete.failures);
        assert_eq!(
            complete.covered_devices(),
            DeviceClass::explicit_profiles().len()
        );
        assert!(complete.missing_devices().is_empty());
        assert!(
            complete
                .summary_line()
                .contains("persistent_roundtrip_matrix: passed=true")
        );

        let failed_report = PersistentRoundtripReport::evaluate(PersistentRoundtripInput {
            first_stored_memory: true,
            first_runtime_kv_stored: 1,
            first_runtime_kv_namespace_preserved: true,
            second_used_memories: 1,
            second_used_runtime_kv_memory: false,
            second_used_experiences: 1,
            second_imported_runtime_kv_blocks: 1,
            second_imported_runtime_kv_from_namespace: false,
            second_runtime_adapter_observations: 1,
            second_runtime_adapter_best_score: Some(0.72),
            second_runtime_adapter_best_adapter: Some("portable-rust".to_owned()),
            second_runtime_selected_adapter: Some("portable-rust".to_owned()),
            second_quality: 0.80,
            first_drift_severity: DriftSeverity::Stable,
            second_drift_severity: DriftSeverity::Stable,
        });
        let incomplete = PersistentRoundtripMatrixReport::evaluate(vec![
            PersistentRoundtripDeviceReport {
                device: DeviceClass::CpuOnly,
                report: passing_report,
            },
            PersistentRoundtripDeviceReport {
                device: DeviceClass::IntegratedGpu,
                report: failed_report,
            },
        ]);

        assert!(!incomplete.passed);
        assert_eq!(incomplete.covered_devices(), 2);
        assert_eq!(
            incomplete.missing_devices().len(),
            DeviceClass::explicit_profiles().len() - 2
        );
        assert_eq!(
            incomplete.failed_devices(),
            vec![DeviceClass::IntegratedGpu]
        );
        assert!(
            incomplete
                .failures
                .iter()
                .any(|failure| failure.contains("missing="))
        );
        assert!(
            incomplete
                .failures
                .iter()
                .any(|failure| failure.contains("integrated"))
        );
    }
}
