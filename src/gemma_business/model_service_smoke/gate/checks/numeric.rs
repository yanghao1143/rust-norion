pub(in crate::gemma_business::model_service_smoke::gate) fn require_at_least_u64(
    actual: u64,
    expected: u64,
    message: &str,
    failures: &mut Vec<String>,
) {
    if actual < expected {
        failures.push(message.to_owned());
    }
}

pub(in crate::gemma_business::model_service_smoke::gate) fn require_zero_u64(
    value: u64,
    label: &str,
    failures: &mut Vec<String>,
) {
    if value > 0 {
        failures.push(format!("{label}={value}"));
    }
}

#[cfg(test)]
mod tests {
    use super::{require_at_least_u64, require_zero_u64};

    #[test]
    fn require_at_least_u64_records_underflow_only() {
        let mut failures = Vec::new();

        require_at_least_u64(1, 2, "below", &mut failures);
        require_at_least_u64(2, 2, "equal", &mut failures);
        require_at_least_u64(3, 2, "above", &mut failures);

        assert_eq!(failures, vec!["below".to_owned()]);
    }

    #[test]
    fn require_zero_u64_records_nonzero_values() {
        let mut failures = Vec::new();

        require_zero_u64(0, "zero", &mut failures);
        require_zero_u64(3, "nonzero", &mut failures);

        assert_eq!(failures, vec!["nonzero=3".to_owned()]);
    }
}
