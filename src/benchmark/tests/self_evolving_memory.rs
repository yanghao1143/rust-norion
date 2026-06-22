use super::*;

#[test]
fn default_self_evolving_memory_ab_suite_records_wins_regressions_and_digest_ledgers() {
    let report = run_default_self_evolving_memory_ab_suite();
    let gate = SelfEvolvingMemoryAbGate::default();
    let gate_report = report.evaluate(&gate);

    assert!(gate_report.passed, "{:?}", gate_report.failures);
    assert_eq!(report.case_count(), 4);
    assert_eq!(report.mode_count(), 5);
    assert_eq!(report.language_count(), 3);
    assert!(report.win_count() >= 3);
    assert!(report.regression_count() >= 1);
    assert!(report.total_token_savings() > 0);
    assert!(report.candidate_previews() > 0);
    assert_eq!(report.admitted_candidates(), 0);
    assert!(report.unsafe_write_rejections() >= 1);
    assert!(report.quarantine_recommendations() >= 1 || report.rollback_recommendations() >= 1);
    assert!(report.compiler_passed() > 0);
    assert!(report.tests_passed() > 0);
    assert!(report.benchmark_passed() > 0);
    assert!(report.ledger_is_digest_only());
    assert!(report.summary_line().contains("self_evolving_memory_ab"));
    assert!(
        report
            .ledger_lines()
            .iter()
            .all(|line| line.contains("case=fnv64:") && !line.contains("请用中文"))
    );
}

#[test]
fn self_evolving_memory_ab_gate_reports_missing_coverage() {
    let store = seeded_self_evolving_memory_ab_store();
    let case = default_self_evolving_memory_ab_cases()
        .into_iter()
        .next()
        .expect("default case");
    let report = SelfEvolvingMemoryAbHarness {
        modes: vec![SelfEvolvingMemoryEvalMode::Baseline],
        ..SelfEvolvingMemoryAbHarness::default()
    }
    .run(&store, &[case]);

    let gate_report = report.evaluate(&SelfEvolvingMemoryAbGate::default());

    assert!(!gate_report.passed);
    for marker in ["languages", "wins", "regressions", "missing memory mode"] {
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

#[test]
fn self_evolving_memory_ab_regression_is_preview_only_and_quarantined() {
    let report = run_default_self_evolving_memory_ab_suite();
    let regression = report
        .results
        .iter()
        .find(|result| {
            result.mode == SelfEvolvingMemoryEvalMode::ToolReliability && result.is_regression()
        })
        .expect("tool reliability regression");

    assert!(regression.preview_only);
    assert_eq!(regression.admitted_candidates, 0);
    assert_eq!(regression.unsafe_write_rejections, 1);
    assert_eq!(
        regression.recommendation,
        SelfEvolvingMemoryAbRecommendation::Quarantine
    );
    assert!(!regression.validation_passed());
    assert!(
        regression
            .ledger_line()
            .contains("recommendation=quarantine")
    );
}
