use std::fs;
use std::path::Path;

use crate::gemma_business::response_json::{response_object_bool_field, response_u64_field};

pub(super) fn require_response_artifact(
    response_path: &Path,
    report_body: &str,
    failures: &mut Vec<String>,
) {
    match fs::read_to_string(response_path) {
        Ok(response_body) => {
            if !response_object_bool_field(&response_body, "business_cycle", "passed") {
                push_response_artifact_failure(
                    response_path,
                    "did not report business_cycle.passed=true",
                    failures,
                );
            }
            if response_u64_field(&response_body, "runtime_token_count")
                < response_u64_field(report_body, "runtime_token_count")
            {
                push_response_artifact_failure(
                    response_path,
                    "lost runtime token evidence",
                    failures,
                );
            }
        }
        Err(error) => push_response_artifact_failure(
            response_path,
            &format!("could not be read: {error}"),
            failures,
        ),
    }
}

fn push_response_artifact_failure(response_path: &Path, message: &str, failures: &mut Vec<String>) {
    failures.push(format!(
        "response artifact {} {message}",
        response_path.display()
    ));
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::push_response_artifact_failure;

    #[test]
    fn push_response_artifact_failure_prefixes_artifact_path() {
        let mut failures = Vec::new();

        push_response_artifact_failure(
            Path::new("runs/gemma/response.json"),
            "lost runtime token evidence",
            &mut failures,
        );

        assert_eq!(
            failures,
            vec![
                "response artifact runs/gemma/response.json lost runtime token evidence".to_owned()
            ]
        );
    }
}
