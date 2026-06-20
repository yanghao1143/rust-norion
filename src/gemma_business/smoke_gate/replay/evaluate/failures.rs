use rust_norion::ExperienceReplayReport;

pub(super) fn gemma_business_smoke_replay_failures(
    replay_report: &ExperienceReplayReport,
) -> Vec<String> {
    let mut failures = Vec::new();
    require_replay_applied(replay_report, &mut failures);
    require_business_contract_evidence(replay_report, &mut failures);
    require_business_contract_pass_counts(replay_report, &mut failures);
    require_raw_business_contract_audits(replay_report, &mut failures);
    failures
}

fn require_replay_applied(replay_report: &ExperienceReplayReport, failures: &mut Vec<String>) {
    if replay_report.applied == 0 {
        failures.push("replay did not apply any experience".to_owned());
    }
}

fn require_business_contract_evidence(
    replay_report: &ExperienceReplayReport,
    failures: &mut Vec<String>,
) {
    if replay_report.business_contract_items == 0 {
        failures.push("replay did not consume business contract evidence".to_owned());
    }
}

fn require_business_contract_pass_counts(
    replay_report: &ExperienceReplayReport,
    failures: &mut Vec<String>,
) {
    if replay_report.business_contract_passed < replay_report.business_contract_items {
        failures.push(format!(
            "replay business_contract_passed={} below items={}",
            replay_report.business_contract_passed, replay_report.business_contract_items
        ));
    }
    if replay_report.business_contract_failed > 0 {
        failures.push(format!(
            "replay business_contract_failed={}",
            replay_report.business_contract_failed
        ));
    }
}

fn require_raw_business_contract_audits(
    replay_report: &ExperienceReplayReport,
    failures: &mut Vec<String>,
) {
    if replay_report
        .business_contract_raw_passed
        .saturating_add(replay_report.business_contract_raw_failed)
        < replay_report.business_contract_items
    {
        failures.push("replay did not preserve every raw business contract audit".to_owned());
    }
}

#[cfg(test)]
mod tests {
    use rust_norion::ExperienceReplayReport;

    use super::gemma_business_smoke_replay_failures;

    #[test]
    fn replay_failures_report_all_business_contract_gate_gaps() {
        let failures = gemma_business_smoke_replay_failures(&ExperienceReplayReport {
            business_contract_items: 2,
            business_contract_passed: 1,
            business_contract_failed: 1,
            business_contract_raw_passed: 1,
            business_contract_raw_failed: 0,
            ..ExperienceReplayReport::default()
        });

        assert_eq!(
            failures,
            vec![
                "replay did not apply any experience".to_owned(),
                "replay business_contract_passed=1 below items=2".to_owned(),
                "replay business_contract_failed=1".to_owned(),
                "replay did not preserve every raw business contract audit".to_owned(),
            ]
        );
    }

    #[test]
    fn replay_failures_accept_complete_business_contract_evidence() {
        let failures = gemma_business_smoke_replay_failures(&ExperienceReplayReport {
            applied: 1,
            business_contract_items: 2,
            business_contract_passed: 2,
            business_contract_raw_passed: 1,
            business_contract_raw_failed: 1,
            ..ExperienceReplayReport::default()
        });

        assert!(failures.is_empty());
    }
}
