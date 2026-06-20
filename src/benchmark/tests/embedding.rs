use super::*;
use crate::engine::{
    GenerationContext, HeuristicBackend, InferenceBackend, InferenceRequest, NoironEngine,
};
use crate::reflection::{InferenceDraft, ReasoningStep};

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
