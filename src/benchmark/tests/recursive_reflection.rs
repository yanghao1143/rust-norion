use super::*;
use crate::engine::{HeuristicBackend, InferenceRequest, NoironEngine};

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
        results: vec![BenchmarkCaseResult {
            requires_recursion: true,
            recursive_chunks: 4,
            recursive_waves: 2,
            recursive_runtime_calls: 7,
            auto_replay_applied: 1,
            auto_replay_router_updates: 1,
            auto_replay_hierarchy_updates: 1,
            auto_replay_memory_reinforcements: 1,
            auto_replay_recursive_runtime_items: 1,
            auto_replay_recursive_runtime_calls: 96,
            auto_replay_avg_recursive_call_pressure: 0.35,
            auto_replay_max_recursive_call_pressure: 0.35,
            ..baseline_benchmark_result(
                "replay_pressure",
                TaskProfile::LongDocument,
                DeviceClass::CpuOnly,
            )
        }],
        ..BenchmarkSummary::default()
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
        results: vec![BenchmarkCaseResult {
            requires_recursion: true,
            recursive_chunks: 4,
            recursive_waves: 2,
            recursive_runtime_calls: 7,
            auto_replay_applied: 1,
            auto_replay_router_updates: 1,
            auto_replay_hierarchy_updates: 1,
            auto_replay_memory_reinforcements: 1,
            auto_replay_recursive_runtime_items: 1,
            auto_replay_recursive_runtime_calls: 7,
            ..baseline_benchmark_result(
                "missing_replay_pressure",
                TaskProfile::LongDocument,
                DeviceClass::CpuOnly,
            )
        }],
        ..BenchmarkSummary::default()
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
