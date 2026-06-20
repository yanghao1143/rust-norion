use std::path::PathBuf;

mod case_json;
mod contract;
mod evidence;
mod files;
mod render;
mod sections;
mod summary;

use crate::Args;

use super::types::GemmaBusinessCycleCaseResult;
use case_json::gemma_business_cycle_smoke_cases_report_json;
use contract::MatrixReportContract;
use evidence::MatrixReportEvidence;
use files::MatrixReportFiles;
use render::{MatrixReportRender, render_matrix_report_json};
use sections::MatrixReportSections;
use summary::MatrixReportSummary;

#[allow(clippy::too_many_arguments)]
pub(crate) fn gemma_business_cycle_smoke_matrix_report_json(
    passed: bool,
    bind: &str,
    args: &Args,
    response_path: Option<&PathBuf>,
    health_body: &str,
    final_cycle_body: &str,
    case_results: &[GemmaBusinessCycleCaseResult],
    failures: &[String],
    runtime_token_count: u64,
    feedback_applied: u64,
    rust_check_feedback_applied: u64,
    checked_trace_lines: u64,
) -> String {
    let evidence = MatrixReportEvidence::from_cases(case_results);
    let sections = MatrixReportSections::from_cases(health_body, final_cycle_body, case_results);
    let files = MatrixReportFiles::from_args(args, response_path);
    let contract = MatrixReportContract::from_cases(case_results);
    let summary = MatrixReportSummary::from_evidence(&evidence);
    let case_json = gemma_business_cycle_smoke_cases_report_json(case_results);

    render_matrix_report_json(MatrixReportRender {
        passed,
        bind,
        evidence: &evidence,
        summary: &summary,
        runtime_token_count,
        files: &files,
        sections: &sections,
        contract: &contract,
        feedback_applied,
        rust_check_feedback_applied,
        checked_trace_lines,
        case_json: &case_json,
        failures,
    })
}
