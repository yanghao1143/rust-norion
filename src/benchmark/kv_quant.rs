use std::time::Instant;

use crate::kv_quant::{QuantizationBits, QuantizedVector};

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
