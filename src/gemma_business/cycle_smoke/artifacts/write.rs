use std::fs;
use std::path::PathBuf;

use super::BusinessCycleSmokeArtifacts;
use crate::gemma_business::smoke_report::{
    gemma_business_cycle_smoke_aggregate_response_json,
    gemma_business_cycle_smoke_matrix_report_json,
};

pub(super) fn write_business_cycle_response_artifact(
    response_path: Option<&PathBuf>,
    artifacts: &BusinessCycleSmokeArtifacts<'_>,
) -> std::io::Result<()> {
    let Some(response_path) = response_path else {
        return Ok(());
    };
    fs::write(
        response_path,
        gemma_business_cycle_smoke_aggregate_response_json(
            artifacts.passed,
            artifacts.case_results,
            artifacts.metrics.runtime_token_count,
            artifacts.metrics.feedback_applied,
            artifacts.metrics.rust_check_feedback_applied,
            artifacts.metrics.checked_trace_lines,
        ),
    )
}

pub(super) fn write_business_cycle_report_artifact(
    response_path: Option<&PathBuf>,
    report_path: &Option<PathBuf>,
    artifacts: &BusinessCycleSmokeArtifacts<'_>,
) -> std::io::Result<()> {
    let Some(report_path) = report_path else {
        return Ok(());
    };
    let report_json = gemma_business_cycle_smoke_matrix_report_json(
        artifacts.passed,
        artifacts.bind,
        artifacts.service_args,
        response_path,
        artifacts.health_body,
        artifacts.final_cycle_body,
        artifacts.case_results,
        artifacts.failures,
        artifacts.metrics.runtime_token_count,
        artifacts.metrics.feedback_applied,
        artifacts.metrics.rust_check_feedback_applied,
        artifacts.metrics.checked_trace_lines,
    );
    fs::write(report_path, report_json)
}
