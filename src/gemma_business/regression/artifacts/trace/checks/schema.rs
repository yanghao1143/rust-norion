use std::path::Path;

use rust_norion::TraceSchemaGateReport;

pub(super) fn require_schema_gate_passed(
    trace_path: &Path,
    trace_report: &TraceSchemaGateReport,
    failures: &mut Vec<String>,
) {
    if !trace_report.passed {
        push_trace_schema_failure(trace_path, &trace_report.failures, failures);
    }
}

fn push_trace_schema_failure(
    trace_path: &Path,
    schema_failures: &[String],
    failures: &mut Vec<String>,
) {
    failures.push(format!(
        "trace artifact {} schema gate failed: {}",
        trace_path.display(),
        schema_failures.join("; ")
    ));
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::push_trace_schema_failure;

    #[test]
    fn push_trace_schema_failure_joins_schema_failures() {
        let mut failures = Vec::new();
        let schema_failures = vec![
            "missing event_type".to_owned(),
            "invalid payload".to_owned(),
        ];

        push_trace_schema_failure(
            Path::new("run/trace.jsonl"),
            &schema_failures,
            &mut failures,
        );

        assert_eq!(
            failures,
            [
                "trace artifact run/trace.jsonl schema gate failed: missing event_type; invalid payload"
            ]
        );
    }
}
