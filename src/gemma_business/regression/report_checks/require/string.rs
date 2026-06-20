pub(in crate::gemma_business::regression) fn require_report_string(
    failures: &mut Vec<String>,
    field: &str,
    actual: Option<&str>,
    expected: &str,
) {
    if actual != Some(expected) {
        failures.push(format!(
            "{field} expected {expected}, got {}",
            actual.unwrap_or("missing")
        ));
    }
}

pub(in crate::gemma_business::regression) fn require_report_nonempty_string(
    failures: &mut Vec<String>,
    field: &str,
    actual: Option<&str>,
) {
    if actual.map(|value| value.trim().is_empty()).unwrap_or(true) {
        failures.push(format!("{field} missing or empty"));
    }
}

#[cfg(test)]
mod tests {
    use super::{require_report_nonempty_string, require_report_string};

    #[test]
    fn require_report_string_records_wrong_and_missing_values() {
        let mut failures = Vec::new();

        require_report_string(&mut failures, "schema", Some("v1"), "v1");
        require_report_string(&mut failures, "gate", Some("smoke"), "cycle");
        require_report_string(&mut failures, "runtime_model", None, "gemma");

        assert_eq!(
            failures,
            vec![
                "gate expected cycle, got smoke".to_owned(),
                "runtime_model expected gemma, got missing".to_owned()
            ]
        );
    }

    #[test]
    fn require_report_nonempty_string_records_missing_and_blank_values() {
        let mut failures = Vec::new();

        require_report_nonempty_string(&mut failures, "missing", None);
        require_report_nonempty_string(&mut failures, "blank", Some("   "));
        require_report_nonempty_string(&mut failures, "present", Some("gemma"));

        assert_eq!(
            failures,
            vec![
                "missing missing or empty".to_owned(),
                "blank missing or empty".to_owned()
            ]
        );
    }
}
