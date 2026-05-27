use crate::engine::InferenceOutcome;
use crate::hierarchy::TaskProfile;

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
            "Summarize a local technical document about FHT-DKE, Noiron reflection, recursive scheduling, and persistent KV memory. Identify the control decisions that reduce wasted compute.",
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

#[derive(Debug, Clone)]
pub struct BenchmarkCaseResult {
    pub name: String,
    pub profile: TaskProfile,
    pub elapsed_ms: u128,
    pub quality: f32,
    pub process_reward: f32,
    pub attention_fraction: f32,
    pub recursive_chunks: usize,
    pub used_memories: usize,
    pub stored_memories: usize,
    pub runtime_kv_exported: usize,
    pub runtime_kv_stored: usize,
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
            recursive_chunks: outcome.recursive_schedule.chunk_count(),
            used_memories: outcome.used_memories.len(),
            stored_memories,
            runtime_kv_exported: outcome.exported_runtime_kv_blocks,
            runtime_kv_stored: outcome.stored_runtime_kv_memory_ids.len(),
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

    pub fn total_stored_memories(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.stored_memories)
            .sum()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "cases={} total_elapsed_ms={} avg_quality={:.3} avg_reward={:.3} avg_attention_fraction={:.2} stored_memories={} runtime_kv_stored={}",
            self.len(),
            self.total_elapsed_ms(),
            self.average_quality(),
            self.average_reward(),
            self.average_attention_fraction(),
            self.total_stored_memories(),
            self.total_runtime_kv_stored()
        )
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{HeuristicBackend, InferenceRequest, NoironEngine};

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
    }
}
