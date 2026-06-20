pub(in crate::gemma_business::regression) fn require_report_min_u64(
    failures: &mut Vec<String>,
    field: &str,
    actual: u64,
    minimum: u64,
) {
    if actual < minimum {
        failures.push(format!("{field} {actual} below required {minimum}"));
    }
}

#[cfg(test)]
mod tests {
    use super::require_report_min_u64;

    #[test]
    fn require_report_min_u64_records_underflow_only() {
        let mut failures = Vec::new();

        require_report_min_u64(&mut failures, "case_count", 1, 2);
        require_report_min_u64(&mut failures, "passed_cases", 2, 2);
        require_report_min_u64(&mut failures, "runtime_tokens", 3, 2);

        assert_eq!(failures, vec!["case_count 1 below required 2".to_owned()]);
    }
}
