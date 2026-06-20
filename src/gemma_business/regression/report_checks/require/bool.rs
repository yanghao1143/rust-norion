pub(in crate::gemma_business::regression) fn require_report_bool(
    failures: &mut Vec<String>,
    field: &str,
    actual: Option<bool>,
) {
    if actual != Some(true) {
        push_report_bool_failure(failures, field, true, actual);
    }
}

pub(in crate::gemma_business::regression) fn require_report_bool_false(
    failures: &mut Vec<String>,
    field: &str,
    actual: Option<bool>,
) {
    if actual != Some(false) {
        push_report_bool_failure(failures, field, false, actual);
    }
}

fn push_report_bool_failure(
    failures: &mut Vec<String>,
    field: &str,
    expected: bool,
    actual: Option<bool>,
) {
    failures.push(format!(
        "{field} expected {expected}, got {}",
        report_bool_value_label(actual)
    ));
}

fn report_bool_value_label(actual: Option<bool>) -> String {
    actual
        .map(|value| value.to_string())
        .unwrap_or_else(|| "missing".to_owned())
}

#[cfg(test)]
mod tests {
    use super::{push_report_bool_failure, require_report_bool, require_report_bool_false};

    #[test]
    fn require_report_bool_records_wrong_and_missing_values() {
        let mut failures = Vec::new();

        require_report_bool(&mut failures, "contract_passed", Some(true));
        require_report_bool(&mut failures, "gate_passed", Some(false));
        require_report_bool(&mut failures, "runtime_ok", None);
        require_report_bool_false(&mut failures, "missing_signals", Some(false));
        require_report_bool_false(&mut failures, "failed", Some(true));

        assert_eq!(
            failures,
            vec![
                "gate_passed expected true, got false".to_owned(),
                "runtime_ok expected true, got missing".to_owned(),
                "failed expected false, got true".to_owned()
            ]
        );
    }

    #[test]
    fn push_report_bool_failure_formats_expected_and_actual_labels() {
        let mut failures = Vec::new();

        push_report_bool_failure(&mut failures, "ok", true, Some(false));
        push_report_bool_failure(&mut failures, "present", false, None);

        assert_eq!(
            failures,
            vec![
                "ok expected true, got false".to_owned(),
                "present expected false, got missing".to_owned()
            ]
        );
    }
}
