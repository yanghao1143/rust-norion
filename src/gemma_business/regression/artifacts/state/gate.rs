use rust_norion::StateInspectionReport;

use crate::gemma_business::regression::state_gate::business_cycle_report_state_gate_from_base;

pub(super) fn require_state_gate_report(
    inspection: &StateInspectionReport,
    expected_case_count: u64,
    failures: &mut Vec<String>,
) {
    let state_gate =
        business_cycle_report_state_gate_from_base(Default::default(), expected_case_count);
    let state_gate_report = inspection.evaluate(&state_gate);
    if !state_gate_report.passed {
        for failure in state_gate_report.failures {
            push_state_gate_failure(&failure, failures);
        }
    }
}

fn push_state_gate_failure(failure: &str, failures: &mut Vec<String>) {
    failures.push(format!("state artifact gate failure: {failure}"));
}

#[cfg(test)]
mod tests {
    use super::push_state_gate_failure;

    #[test]
    fn push_state_gate_failure_prefixes_artifact_gate_context() {
        let mut failures = Vec::new();

        push_state_gate_failure("missing business_contract_experiences", &mut failures);

        assert_eq!(
            failures,
            vec!["state artifact gate failure: missing business_contract_experiences".to_owned()]
        );
    }
}
