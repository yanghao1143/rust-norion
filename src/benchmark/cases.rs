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

pub(super) fn long_context_benchmark_prompt() -> String {
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
