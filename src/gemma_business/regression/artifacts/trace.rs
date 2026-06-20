use std::path::Path;

mod checks;

use checks::require_trace_report_checks;
use rust_norion::evaluate_trace_schema_jsonl;

pub(super) fn require_trace_artifact(
    trace_path: &Path,
    report_body: &str,
    expected_case_count: u64,
    failures: &mut Vec<String>,
) {
    match evaluate_trace_schema_jsonl(trace_path) {
        Ok(trace_report) => {
            require_trace_report_checks(
                trace_path,
                &trace_report,
                report_body,
                expected_case_count,
                failures,
            );
        }
        Err(error) => {
            push_trace_artifact_evaluation_failure(trace_path, &error.to_string(), failures);
        }
    }
}

fn push_trace_artifact_evaluation_failure(
    trace_path: &Path,
    error: &str,
    failures: &mut Vec<String>,
) {
    failures.push(format!(
        "trace artifact {} could not be evaluated: {error}",
        trace_path.display()
    ));
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::push_trace_artifact_evaluation_failure;

    #[test]
    fn push_trace_artifact_evaluation_failure_formats_path_and_error() {
        let mut failures = Vec::new();

        push_trace_artifact_evaluation_failure(Path::new("trace.jsonl"), "bad json", &mut failures);

        assert_eq!(
            failures,
            ["trace artifact trace.jsonl could not be evaluated: bad json"]
        );
    }
}
