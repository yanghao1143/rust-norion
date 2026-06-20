pub(super) fn require_state_min_usize(
    failures: &mut Vec<String>,
    field: &str,
    actual: usize,
    expected: u64,
) {
    require_state_min_u64(failures, field, actual as u64, expected);
}

pub(super) fn require_state_min_u64(
    failures: &mut Vec<String>,
    field: &str,
    actual: u64,
    expected: u64,
) {
    if actual < expected {
        failures.push(format!(
            "state artifact {field} {actual} below report {expected}"
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::{require_state_min_u64, require_state_min_usize};

    #[test]
    fn require_state_min_usize_records_underflow_only() {
        let mut failures = Vec::new();

        require_state_min_usize(&mut failures, "items", 1, 2);
        require_state_min_usize(&mut failures, "items", 2, 2);
        require_state_min_usize(&mut failures, "items", 3, 2);

        assert_eq!(
            failures,
            vec!["state artifact items 1 below report 2".to_owned()]
        );
    }

    #[test]
    fn require_state_min_u64_records_underflow_only() {
        let mut failures = Vec::new();

        require_state_min_u64(&mut failures, "events", 1, 2);
        require_state_min_u64(&mut failures, "events", 2, 2);
        require_state_min_u64(&mut failures, "events", 3, 2);

        assert_eq!(
            failures,
            vec!["state artifact events 1 below report 2".to_owned()]
        );
    }
}
