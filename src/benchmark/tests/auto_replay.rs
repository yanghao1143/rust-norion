use super::*;

#[test]
fn gate_reports_missing_auto_replay_control_plane_coverage() {
    let summary = BenchmarkSummary {
        genome_evidence: BenchmarkGenomeEvidence::default(),
        improvement_corpus_evidence: BenchmarkImprovementCorpusEvidence::default(),
        results: vec![BenchmarkCaseResult {
            auto_replay_applied: 1,
            ..baseline_benchmark_result(
                "auto_replay_control_plane",
                TaskProfile::Coding,
                DeviceClass::CpuOnly,
            )
        }],
        ..BenchmarkSummary::default()
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
        genome_evidence: BenchmarkGenomeEvidence::default(),
        improvement_corpus_evidence: BenchmarkImprovementCorpusEvidence::default(),
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
        ..BenchmarkSummary::default()
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
