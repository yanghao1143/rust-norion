use super::*;
use crate::engine::{HeuristicBackend, InferenceRequest, NoironEngine};

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
        auto_replay_applied: 1,
        auto_replay_router_updates: 1,
        auto_replay_hierarchy_updates: 1,
        auto_replay_memory_reinforcements: 1,
        auto_replay_live_memory_feedback_items: 1,
        auto_replay_live_memory_feedback_updates: 2,
        auto_replay_live_memory_feedback_reinforcements: 2,
        auto_replay_live_memory_feedback_detail_items: 1,
        auto_replay_live_memory_feedback_applied: 2,
        auto_replay_live_memory_feedback_strength_delta: 0.42,
        ..baseline_benchmark_result(
            "replay_live_feedback",
            TaskProfile::Coding,
            DeviceClass::CpuOnly,
        )
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
