use std::path::Path;

pub(super) fn require_trace_min_u64(
    trace_path: &Path,
    field: &str,
    actual: u64,
    expected_label: &str,
    expected: u64,
    failures: &mut Vec<String>,
) {
    if actual < expected {
        failures.push(format!(
            "trace artifact {} {field} {actual} below {expected_label} {expected}",
            trace_path.display()
        ));
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::require_trace_min_u64;

    #[test]
    fn require_trace_min_u64_records_underflow_only() {
        let trace_path = Path::new("runs/gemma/trace.jsonl");
        let mut failures = Vec::new();

        require_trace_min_u64(trace_path, "checked_lines", 1, "report", 2, &mut failures);
        require_trace_min_u64(trace_path, "checked_lines", 2, "report", 2, &mut failures);
        require_trace_min_u64(trace_path, "checked_lines", 3, "report", 2, &mut failures);

        assert_eq!(
            failures,
            vec!["trace artifact runs/gemma/trace.jsonl checked_lines 1 below report 2".to_owned()]
        );
    }
}
