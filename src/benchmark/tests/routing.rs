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
        min_task_hierarchy_compute_reduction_milli: Some(100),
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
            .contains("task_hierarchy_compute_reduction_milli=420")
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
        min_task_hierarchy_compute_reduction_milli: Some(1),
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
        "task_hierarchy_compute_reduction_milli",
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
