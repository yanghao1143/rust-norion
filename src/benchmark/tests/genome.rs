use super::*;
use crate::engine::{HeuristicBackend, InferenceRequest, NoironEngine};

#[test]
fn summary_records_reasoning_genome_expression_evidence() {
    let mut engine = NoironEngine::new();
    engine.set_hardware_snapshot(crate::hardware::HardwareSnapshot::new(
        DeviceClass::CpuOnly,
        0.25,
        0.0,
        0.35,
        0.15,
    ));
    let mut backend = HeuristicBackend;
    let case = BenchmarkCase::new(
        "reasoning_genome",
        TaskProfile::Coding,
        "Use Rust tests to validate a Noiron reasoning genome chain.",
    );
    let outcome = engine.infer(
        InferenceRequest::new(case.prompt.clone(), case.profile),
        &mut backend,
    );
    let mut summary = BenchmarkSummary::new();

    summary.record(&case, 5, &outcome);

    assert_eq!(summary.reasoning_genome_expression_cases(), 1);
    assert_eq!(summary.reasoning_genome_expression_device_profiles(), 1);
    assert_eq!(summary.total_reasoning_genome_failures(), 0);
    assert!(summary.genome_evidence().total_genes >= 7);
    assert!(
        summary
            .summary_line()
            .contains("reasoning_genome_expression_cases=1")
    );
    assert!(
        summary
            .summary_line()
            .contains("reasoning_genome_failures=0")
    );

    let report = summary.evaluate(&BenchmarkGate {
        min_reasoning_genome_expression_cases: Some(1),
        min_reasoning_genome_expression_device_profiles: Some(1),
        ..BenchmarkGate::default()
    });

    assert!(report.passed, "{:?}", report.failures);
}

#[test]
fn gate_reports_missing_reasoning_genome_and_gene_scissors_coverage() {
    let summary = BenchmarkSummary::new();
    let gate = BenchmarkGate {
        min_reasoning_genome_expression_cases: Some(1),
        min_reasoning_genome_expression_device_profiles: Some(1),
        min_gene_scissors_proposal_cases: Some(1),
        min_gene_scissors_proposal_device_profiles: Some(1),
        ..BenchmarkGate::default()
    };

    let report = summary.evaluate(&gate);

    assert!(!report.passed);
    for marker in [
        "reasoning_genome_expression_cases",
        "reasoning_genome_expression_device_profiles",
        "gene_scissors_proposal_cases",
        "gene_scissors_proposal_device_profiles",
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
