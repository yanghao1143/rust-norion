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
    assert_eq!(summary.adaptive_routing_cases(), 1);
    assert!(summary.total_adaptive_routing_candidates() > 0);
    assert_eq!(summary.total_adaptive_routing_failures(), 0);
    assert_eq!(summary.task_hierarchy_cases(), 1);
    assert!(summary.total_task_hierarchy_mutation_records() >= 1);
    assert!(summary.total_task_hierarchy_compute_reduction_milli() > 0);
    assert!(summary.summary_line().contains("adaptive_routing_cases=1"));
    assert!(
        summary
            .summary_line()
            .contains("adaptive_routing_candidates=")
    );
    assert!(
        summary
            .summary_line()
            .contains("adaptive_routing_saved_tokens=")
    );
    assert!(summary.summary_line().contains("task_hierarchy_cases=1"));
    assert!(
        summary
            .summary_line()
            .contains("task_hierarchy_compute_reduction_milli=")
    );
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
