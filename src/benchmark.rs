use crate::drift::DriftSeverity;
use crate::engine::InferenceOutcome;
use crate::hierarchy::TaskProfile;
use crate::kv_quant::{QuantizationBits, QuantizedVector};
use std::time::Instant;

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
    pub elapsed_ms: u128,
    pub quality: f32,
    pub process_reward: f32,
    pub attention_fraction: f32,
    pub requires_recursion: bool,
    pub recursive_chunks: usize,
    pub recursive_waves: usize,
    pub recursive_runtime_calls: usize,
    pub used_memories: usize,
    pub stored_memories: usize,
    pub compacted_memories: usize,
    pub runtime_kv_exported: usize,
    pub runtime_kv_stored: usize,
    pub runtime_adapter_observations: usize,
    pub runtime_adapter_best_score: Option<f32>,
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

#[derive(Debug, Clone, Copy)]
pub struct PersistentRoundtripInput {
    pub first_stored_memory: bool,
    pub first_runtime_kv_stored: usize,
    pub second_used_memories: usize,
    pub second_used_experiences: usize,
    pub second_imported_runtime_kv_blocks: usize,
    pub second_runtime_adapter_observations: usize,
    pub second_runtime_adapter_best_score: Option<f32>,
    pub second_quality: f32,
    pub first_drift_severity: DriftSeverity,
    pub second_drift_severity: DriftSeverity,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PersistentRoundtripReport {
    pub passed: bool,
    pub first_stored_memory: bool,
    pub first_runtime_kv_stored: usize,
    pub second_used_memories: usize,
    pub second_used_experiences: usize,
    pub second_imported_runtime_kv_blocks: usize,
    pub second_runtime_adapter_observations: usize,
    pub second_runtime_adapter_best_score: Option<f32>,
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
        if input.second_used_memories == 0 {
            failures.push("second run did not retrieve persisted memory".to_owned());
        }
        if input.second_used_experiences == 0 {
            failures.push("second run did not retrieve persisted experience".to_owned());
        }
        if input.second_imported_runtime_kv_blocks == 0 {
            failures.push("second run did not import persisted runtime KV".to_owned());
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
            second_used_memories: input.second_used_memories,
            second_used_experiences: input.second_used_experiences,
            second_imported_runtime_kv_blocks: input.second_imported_runtime_kv_blocks,
            second_runtime_adapter_observations: input.second_runtime_adapter_observations,
            second_runtime_adapter_best_score: input.second_runtime_adapter_best_score,
            second_quality: input.second_quality,
            first_drift_severity: input.first_drift_severity,
            second_drift_severity: input.second_drift_severity,
            failures,
        }
    }

    pub fn summary_line(&self) -> String {
        format!(
            "persistent_roundtrip: passed={} first_stored_memory={} first_runtime_kv_stored={} second_used_memories={} second_used_experiences={} second_imported_runtime_kv_blocks={} second_runtime_adapter_observations={} second_runtime_adapter_best_score={} second_quality={:.3} first_drift={} second_drift={} failures={}",
            self.passed,
            self.first_stored_memory,
            self.first_runtime_kv_stored,
            self.second_used_memories,
            self.second_used_experiences,
            self.second_imported_runtime_kv_blocks,
            self.second_runtime_adapter_observations,
            option_f32_display(self.second_runtime_adapter_best_score),
            self.second_quality,
            self.first_drift_severity.as_str(),
            self.second_drift_severity.as_str(),
            self.failures.len()
        )
    }
}

#[derive(Debug, Clone, Default)]
pub struct BenchmarkSummary {
    results: Vec<BenchmarkCaseResult>,
}

impl BenchmarkSummary {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, case: &BenchmarkCase, elapsed_ms: u128, outcome: &InferenceOutcome) {
        let stored_memories = usize::from(outcome.stored_memory_id.is_some())
            + outcome.stored_gist_memory_ids.len()
            + outcome.stored_runtime_kv_memory_ids.len();

        self.results.push(BenchmarkCaseResult {
            name: case.name.clone(),
            profile: case.profile,
            elapsed_ms,
            quality: outcome.report.quality,
            process_reward: outcome.process_reward.total,
            attention_fraction: outcome.route_budget.attention_fraction,
            requires_recursion: outcome.recursive_schedule.requires_recursion,
            recursive_chunks: outcome.recursive_schedule.chunk_count(),
            recursive_waves: outcome.recursive_schedule.execution_wave_count(),
            recursive_runtime_calls: outcome.recursive_runtime_calls,
            used_memories: outcome.used_memories.len(),
            stored_memories,
            compacted_memories: outcome.memory_compaction_report.merged.len(),
            runtime_kv_exported: outcome.exported_runtime_kv_blocks,
            runtime_kv_stored: outcome.stored_runtime_kv_memory_ids.len(),
            runtime_adapter_observations: outcome.runtime_adapter_observations.len(),
            runtime_adapter_best_score: outcome
                .runtime_adapter_observations
                .first()
                .map(|observation| observation.score),
            drift_severity: outcome.drift_report.severity,
        });
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
            "cases={} total_elapsed_ms={} avg_quality={:.3} avg_reward={:.3} avg_attention_fraction={:.2} recursive_cases={} max_recursive_waves={} recursive_runtime_calls={} stored_memories={} compacted_memories={} runtime_kv_stored={} runtime_adapter_observations={} runtime_adapter_best_score={} drift_watch={} drift_block={} drift_rollback={}",
            self.len(),
            self.total_elapsed_ms(),
            self.average_quality(),
            self.average_reward(),
            self.average_attention_fraction(),
            self.recursive_cases(),
            self.max_recursive_waves(),
            self.total_recursive_runtime_calls(),
            self.total_stored_memories(),
            self.total_compacted_memories(),
            self.total_runtime_kv_stored(),
            self.total_runtime_adapter_observations(),
            option_f32_display(self.max_runtime_adapter_score()),
            self.drift_watches(),
            self.drift_blocks(),
            self.drift_rollbacks()
        )
    }
}

fn option_f32_display(value: Option<f32>) -> String {
    value
        .filter(|value| value.is_finite())
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "none".to_owned())
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
    use crate::engine::{HeuristicBackend, InferenceRequest, NoironEngine};
    use crate::recursive_scheduler::RecursiveScheduler;

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
            min_recursive_cases: None,
            min_recursive_runtime_calls: None,
            max_drift_blocks: Some(0),
            max_drift_rollbacks: Some(0),
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
    fn gate_reports_drift_failures() {
        let summary = BenchmarkSummary {
            results: vec![BenchmarkCaseResult {
                name: "drift".to_owned(),
                profile: TaskProfile::General,
                elapsed_ms: 1,
                quality: 0.9,
                process_reward: 0.9,
                attention_fraction: 0.5,
                requires_recursion: false,
                recursive_chunks: 1,
                recursive_waves: 1,
                recursive_runtime_calls: 1,
                used_memories: 0,
                stored_memories: 0,
                compacted_memories: 0,
                runtime_kv_exported: 0,
                runtime_kv_stored: 0,
                runtime_adapter_observations: 0,
                runtime_adapter_best_score: None,
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
            second_used_memories: 2,
            second_used_experiences: 1,
            second_imported_runtime_kv_blocks: 2,
            second_runtime_adapter_observations: 1,
            second_runtime_adapter_best_score: Some(0.84),
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

        let failed = PersistentRoundtripReport::evaluate(PersistentRoundtripInput {
            first_stored_memory: false,
            first_runtime_kv_stored: 0,
            second_used_memories: 0,
            second_used_experiences: 0,
            second_imported_runtime_kv_blocks: 0,
            second_runtime_adapter_observations: 0,
            second_runtime_adapter_best_score: None,
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
                .any(|failure| failure.contains("adapter observations"))
        );
    }
}
