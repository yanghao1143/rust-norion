use std::path::{Path, PathBuf};

use crate::gemma_business::response_json::response_optional_string_field;

pub(super) fn require_report_artifact_file(
    report_path: &Path,
    body: &str,
    field: &str,
    failures: &mut Vec<String>,
) -> Option<PathBuf> {
    let Some(raw_path) = response_optional_string_field(body, field) else {
        push_artifact_path_failure(field, "missing", failures);
        return None;
    };
    let path = resolve_report_artifact_path(report_path, &raw_path);
    if !path.is_file() {
        push_artifact_path_failure(
            field,
            &format!("missing or not a file: {}", path.display()),
            failures,
        );
        return None;
    }
    Some(path)
}

fn push_artifact_path_failure(field: &str, message: &str, failures: &mut Vec<String>) {
    failures.push(format!("artifact path {field} {message}"));
}

fn resolve_report_artifact_path(report_path: &Path, raw_path: &str) -> PathBuf {
    let candidate = PathBuf::from(raw_path);
    if candidate.is_absolute() || candidate.exists() {
        return candidate;
    }
    if let Some(parent) = report_path.parent() {
        let joined = parent.join(&candidate);
        if joined.exists() {
            return joined;
        }
        if let Some(file_name) = candidate.file_name() {
            let sibling = parent.join(file_name);
            if sibling.exists() {
                return sibling;
            }
        }
    }
    candidate
}

#[cfg(test)]
mod tests {
    use super::push_artifact_path_failure;

    #[test]
    fn push_artifact_path_failure_formats_field_message() {
        let mut failures = Vec::new();

        push_artifact_path_failure("trace", "missing", &mut failures);

        assert_eq!(failures, vec!["artifact path trace missing".to_owned()]);
    }
}
