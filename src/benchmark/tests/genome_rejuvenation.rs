use super::*;

#[test]
fn default_genome_rejuvenation_simulation_covers_all_decisions() {
    let report = run_default_genome_rejuvenation_simulation();
    let gate_report = report.evaluate(&GenomeRejuvenationSimulationGate::default());

    assert!(gate_report.passed, "{:?}", gate_report.failures);
    assert_eq!(report.case_count(), 4);
    assert!(report.decision_count() >= 6);
    for kind in GenomeRejuvenationDecisionKind::required_coverage() {
        assert!(
            report.covered_decision_kinds().contains(&kind),
            "missing decision kind {} in {:?}",
            kind.as_str(),
            report.covered_decision_kinds()
        );
    }
    assert_eq!(report.write_allowed_count(), 0);
    assert_eq!(report.applied_count(), 0);
    assert_eq!(report.rollback_ready_count(), report.case_count());
    assert_eq!(report.replay_digest_count(), report.case_count());
    assert!(report.total_wasted_compute_reduction() > 0);
    assert!(report.average_memory_usefulness_delta() > 0.0);
    assert!(report.ledger_is_digest_only());
    assert!(report.summary_line().contains("genome_rejuvenation"));
}

#[test]
fn genome_rejuvenation_ledger_is_replayable_and_redacted() {
    let report = run_default_genome_rejuvenation_simulation();

    assert!(
        report
            .ledger_lines()
            .iter()
            .all(|line| line.contains("ledger_input=redaction-digest:")
                && line.contains("replay_digest=redaction-digest:")
                && line.contains("write_allowed=false")
                && line.contains("applied=false")
                && !line.contains("polluted safety guard")),
        "{:?}",
        report.ledger_lines()
    );
    assert!(
        report
            .ledger_lines()
            .iter()
            .any(|line| line.contains("decisions=quarantine|regenerate|tombstone"))
    );
}

#[test]
fn genome_rejuvenation_gate_reports_missing_coverage() {
    let case = default_genome_rejuvenation_cases()
        .into_iter()
        .next()
        .expect("default keep case");
    let report = run_genome_rejuvenation_simulation(&[case]);

    let gate_report = report.evaluate(&GenomeRejuvenationSimulationGate::default());

    assert!(!gate_report.passed);
    for marker in [
        "cases",
        "decisions",
        "decision_kind_missing:relabel",
        "decision_kind_missing:tombstone",
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

#[test]
fn malignant_genome_rejuvenation_stays_preview_only() {
    let report = run_default_genome_rejuvenation_simulation();
    let malignant = report
        .results
        .iter()
        .find(|result| {
            let kinds = result.decision_kinds();
            kinds.contains(&GenomeRejuvenationDecisionKind::Quarantine)
                && kinds.contains(&GenomeRejuvenationDecisionKind::Regenerate)
                && kinds.contains(&GenomeRejuvenationDecisionKind::Tombstone)
        })
        .expect("malignant rejuvenation result");

    assert!(malignant.rollback_ready);
    assert_eq!(malignant.write_allowed, false);
    assert_eq!(malignant.applied, false);
    assert!(malignant.after.average_fitness > malignant.before.average_fitness);
    assert!(malignant.after.average_drift < malignant.before.average_drift);
    assert!(malignant.after.wasted_compute_proxy < malignant.before.wasted_compute_proxy);
    assert!(malignant.decisions.iter().all(|decision| {
        decision.preview_only
            && decision.approval_required
            && !decision.write_allowed
            && !decision.applied
            && decision
                .rollback_anchor_id
                .starts_with("genome:rejuvenation:")
    }));
}
