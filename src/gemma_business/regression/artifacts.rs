mod paths;
mod response;
mod state;
mod trace;

use std::path::Path;

use crate::gemma_business::response_json::response_optional_u64_field;

use paths::{report_artifact_path, require_report_artifact_file};
use response::require_response_artifact;
use state::require_state_artifacts;
use trace::require_trace_artifact;

pub(super) fn require_gemma_business_cycle_smoke_report_artifacts(
    report_path: &Path,
    body: &str,
    failures: &mut Vec<String>,
) {
    let expected_case_count = response_optional_u64_field(body, "case_count")
        .unwrap_or(1)
        .max(1);
    let trace_path = require_report_artifact_file(report_path, body, "trace", failures);
    let memory_path = report_artifact_path(report_path, body, "memory", failures);
    let experience_path = report_artifact_path(report_path, body, "experience", failures);
    let adaptive_path = report_artifact_path(report_path, body, "adaptive", failures);
    let response_path = require_report_artifact_file(report_path, body, "response", failures);

    if let Some(response_path) = response_path {
        require_response_artifact(&response_path, body, failures);
    }

    if let (Some(memory_path), Some(experience_path), Some(adaptive_path)) =
        (&memory_path, &experience_path, &adaptive_path)
    {
        require_state_artifacts(
            memory_path,
            experience_path,
            adaptive_path,
            body,
            expected_case_count,
            failures,
        );
    }

    if let Some(trace_path) = trace_path {
        require_trace_artifact(&trace_path, body, expected_case_count, failures);
    }
}
