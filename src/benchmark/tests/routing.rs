use super::*;

#[test]
fn gate_accepts_adaptive_routing_evidence() {
    let summary = BenchmarkSummary {
        routing_evidence: BenchmarkRoutingEvidence {
            cases: 2,
            candidates: 4,
            included: 1,
            compressed: 1,
            deferred: 1,
            skipped: 1,
            input_tokens: 128,
            retained_tokens: 64,
            saved_tokens: 64,
            task_hierarchy_cases: 2,
            task_hierarchy_mutation_records: 6,
            task_hierarchy_route_pressure_milli: 780,
            task_hierarchy_compute_reduction_milli: 420,
            task_hierarchy_modes: vec!["rust_coding".to_owned(), "benchmark_analysis".to_owned()],
            compute_budget_cases: 2,
            compute_budget_low_value_skipped: 1,
            compute_budget_kv_lookups_skipped: 2,
            compute_budget_validation_cost_tokens: 96,
            compute_budget_saved_tokens: 64,
            compute_budget_avoided_tokens: 96,
            compute_budget_fanout_before: 6,
            compute_budget_fanout_after: 3,
            compute_budget_fanout_reduction: 3,
            devices: vec![DeviceClass::CpuOnly, DeviceClass::IntegratedGpu],
            saved_token_devices: vec![DeviceClass::CpuOnly],
            ..BenchmarkRoutingEvidence::default()
        },
        results: vec![
            baseline_benchmark_result("routing_cpu", TaskProfile::Coding, DeviceClass::CpuOnly),
            baseline_benchmark_result(
                "routing_integrated",
                TaskProfile::Coding,
                DeviceClass::IntegratedGpu,
            ),
        ],
        ..BenchmarkSummary::default()
    };
    let gate = BenchmarkGate {
        min_adaptive_routing_cases: Some(2),
        min_adaptive_routing_device_profiles: Some(2),
        min_adaptive_routing_saved_tokens: Some(32),
        min_adaptive_routing_saved_token_device_profiles: Some(1),
        min_task_hierarchy_cases: Some(2),
        min_task_hierarchy_modes: Some(2),
        min_task_hierarchy_mutation_records: Some(6),
        min_task_hierarchy_route_pressure_milli: Some(700),
        max_task_hierarchy_route_pressure_milli: Some(800),
        min_task_hierarchy_compute_reduction_milli: Some(100),
        min_compute_budget_avoided_tokens: Some(64),
        min_compute_budget_fanout_reduction: Some(2),
        ..BenchmarkGate::default()
    };

    let report = summary.evaluate(&gate);

    assert!(report.passed, "{:?}", report.failures);
    assert_eq!(summary.total_adaptive_routing_saved_tokens(), 64);
    assert!(
        summary
            .summary_line()
            .contains("adaptive_routing_saved_tokens=64")
    );
    assert_eq!(summary.task_hierarchy_mode_count(), 2);
    assert!(
        summary
            .summary_line()
            .contains("task_hierarchy_mutation_records=6")
    );
    assert!(
        summary
            .summary_line()
            .contains("task_hierarchy_route_pressure_milli=780")
    );
    assert!(
        summary
            .summary_line()
            .contains("task_hierarchy_compute_reduction_milli=420")
    );
    assert_eq!(summary.total_compute_budget_avoided_tokens(), 96);
    assert!(
        summary
            .summary_line()
            .contains("compute_budget_avoided_tokens=96")
    );
    assert_eq!(summary.total_compute_budget_fanout_before(), 6);
    assert_eq!(summary.total_compute_budget_fanout_after(), 3);
    assert_eq!(summary.total_compute_budget_fanout_reduction(), 3);
    assert!(
        summary
            .summary_line()
            .contains("compute_budget_fanout_reduction=3")
    );
}

#[test]
fn gate_reports_missing_adaptive_routing_evidence() {
    let summary = BenchmarkSummary::new();
    let gate = BenchmarkGate {
        min_adaptive_routing_cases: Some(1),
        min_adaptive_routing_device_profiles: Some(1),
        min_adaptive_routing_saved_tokens: Some(1),
        min_adaptive_routing_saved_token_device_profiles: Some(1),
        min_task_hierarchy_cases: Some(1),
        min_task_hierarchy_modes: Some(1),
        min_task_hierarchy_mutation_records: Some(1),
        min_task_hierarchy_route_pressure_milli: Some(1),
        min_task_hierarchy_compute_reduction_milli: Some(1),
        min_compute_budget_avoided_tokens: Some(1),
        min_compute_budget_fanout_reduction: Some(1),
        ..BenchmarkGate::default()
    };

    let report = summary.evaluate(&gate);

    assert!(!report.passed);
    for marker in [
        "adaptive_routing_cases",
        "adaptive_routing_device_profiles",
        "adaptive_routing_saved_tokens",
        "adaptive_routing_saved_token_device_profiles",
        "task_hierarchy_cases",
        "task_hierarchy_modes",
        "task_hierarchy_mutation_records",
        "task_hierarchy_route_pressure_milli",
        "task_hierarchy_compute_reduction_milli",
        "compute_budget_avoided_tokens",
        "compute_budget_fanout_reduction",
    ] {
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains(marker)),
            "missing failure marker {marker}: {:?}",
            report.failures
        );
    }
}

#[test]
fn gate_reports_task_hierarchy_route_pressure_above_maximum() {
    let summary = BenchmarkSummary {
        routing_evidence: BenchmarkRoutingEvidence {
            task_hierarchy_route_pressure_milli: 1_250,
            ..BenchmarkRoutingEvidence::default()
        },
        ..BenchmarkSummary::default()
    };
    let gate = BenchmarkGate {
        max_task_hierarchy_route_pressure_milli: Some(1_000),
        ..BenchmarkGate::default()
    };

    let report = summary.evaluate(&gate);

    assert!(!report.passed);
    assert!(
        report.failures.iter().any(|failure| failure
            .contains("task_hierarchy_route_pressure_milli 1250 above maximum 1000")),
        "{:?}",
        report.failures
    );
}

#[test]
fn gate_reports_adaptive_routing_failures() {
    let summary = BenchmarkSummary {
        routing_evidence: BenchmarkRoutingEvidence {
            cases: 1,
            failures: vec![
                "cpu:routing adaptive_routing skipped a required task anchor".to_owned(),
            ],
            ..BenchmarkRoutingEvidence::default()
        },
        ..BenchmarkSummary::default()
    };

    let report = summary.evaluate(&BenchmarkGate::default());

    assert!(!report.passed);
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.contains("adaptive_routing_failures")),
        "{:?}",
        report.failures
    );
}
