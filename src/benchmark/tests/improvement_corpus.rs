use super::*;
use crate::improvement_corpus::{
    ImprovementCorpus, ImprovementEpisodeClass, ImprovementEpisodeInput, ImprovementEvidenceLane,
    ImprovementValidationStatus,
};

#[test]
fn benchmark_summary_records_improvement_corpus_evidence() {
    let mut corpus = ImprovementCorpus::new("benchmark-self-training");
    corpus.push_episode(
        ImprovementEpisodeInput::accepted("accepted-rust-fix")
            .with_evidence_id("compiler:passed")
            .with_evidence_id("tests:passed")
            .with_evidence_id("benchmark:won"),
    );
    corpus.push_episode(
        ImprovementEpisodeInput::new("flaky-rust-fix", ImprovementEpisodeClass::Flaky)
            .with_validation_status(ImprovementValidationStatus::Flaky)
            .with_compiler(ImprovementEvidenceLane::new(1, 1, 0, 0))
            .with_tests(ImprovementEvidenceLane::new(2, 1, 0, 1))
            .with_benchmarks(ImprovementEvidenceLane::new(1, 1, 0, 0))
            .with_rollback_anchor("rollback:flaky")
            .with_rollback_replayed(true),
    );
    let report = corpus.report();

    let mut summary = BenchmarkSummary {
        results: vec![baseline_benchmark_result(
            "improvement-corpus-summary",
            TaskProfile::Coding,
            DeviceClass::CpuOnly,
        )],
        ..BenchmarkSummary::default()
    };
    summary.record_improvement_corpus_report(&report);

    assert_eq!(summary.improvement_corpus_reports(), 1);
    assert_eq!(summary.improvement_corpus_episodes(), 2);
    assert_eq!(summary.improvement_corpus_active_adaptation(), 1);
    assert_eq!(summary.improvement_corpus_compiler_passed(), 2);
    assert_eq!(summary.improvement_corpus_test_passed(), 2);
    assert_eq!(summary.improvement_corpus_benchmark_passed(), 2);
    assert!(
        summary
            .summary_line()
            .contains("improvement_corpus_active_adaptation=1")
    );
    assert!(
        summary
            .summary_line()
            .contains("improvement_corpus_test_passed=2")
    );

    let gate = BenchmarkGate {
        min_average_quality: 0.0,
        min_average_reward: 0.0,
        min_improvement_corpus_reports: Some(1),
        min_improvement_corpus_episodes: Some(2),
        min_improvement_corpus_active_adaptation: Some(1),
        min_improvement_corpus_compiler_passed: Some(2),
        min_improvement_corpus_test_passed: Some(2),
        min_improvement_corpus_benchmark_passed: Some(2),
        min_improvement_corpus_rollback_replayed: Some(2),
        ..BenchmarkGate::default()
    };
    let gate_report = summary.evaluate(&gate);

    assert!(gate_report.passed, "{:?}", gate_report.failures);
}

#[test]
fn benchmark_gate_reports_missing_active_improvement_corpus_evidence() {
    let mut corpus = ImprovementCorpus::new("benchmark-blocked-self-training");
    corpus.push_episode(
        ImprovementEpisodeInput::new("failed-rust-fix", ImprovementEpisodeClass::Failed)
            .with_validation_status(ImprovementValidationStatus::Failed)
            .with_compiler(ImprovementEvidenceLane::new(1, 0, 1, 0))
            .with_tests(ImprovementEvidenceLane::new(1, 0, 1, 0))
            .with_benchmarks(ImprovementEvidenceLane::new(1, 0, 1, 0))
            .with_rollback_anchor("rollback:failed")
            .with_rollback_replayed(true),
    );
    let report = corpus.report();
    let mut summary = BenchmarkSummary {
        results: vec![baseline_benchmark_result(
            "improvement-corpus-blocked",
            TaskProfile::Coding,
            DeviceClass::CpuOnly,
        )],
        ..BenchmarkSummary::default()
    };
    summary.record_improvement_corpus_report(&report);

    let gate = BenchmarkGate {
        min_average_quality: 0.0,
        min_average_reward: 0.0,
        min_improvement_corpus_active_adaptation: Some(1),
        min_improvement_corpus_compiler_passed: Some(1),
        min_improvement_corpus_test_passed: Some(1),
        min_improvement_corpus_benchmark_passed: Some(1),
        ..BenchmarkGate::default()
    };
    let gate_report = summary.evaluate(&gate);

    assert!(!gate_report.passed);
    for marker in [
        "improvement_corpus_active_adaptation",
        "improvement_corpus_compiler_passed",
        "improvement_corpus_test_passed",
        "improvement_corpus_benchmark_passed",
    ] {
        assert!(
            gate_report
                .failures
                .iter()
                .any(|failure| failure.contains(marker)),
            "missing marker {marker}: {:?}",
            gate_report.failures
        );
    }
}
