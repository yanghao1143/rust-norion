mod render;
mod scan;

use std::fs;
use std::path::PathBuf;

use super::SessionRecord;
use super::catalog::{list_recent_sessions, select_record};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSummary {
    pub record: SessionRecord,
    pub summary_path: PathBuf,
    pub line_count: usize,
    pub message_count: usize,
    pub user_count: usize,
    pub assistant_count: usize,
    pub health_check_count: usize,
    pub preflight_count: usize,
    pub diagnostic_count: usize,
    pub final_payload_count: usize,
    pub gate_report_count: usize,
    pub error_count: usize,
    pub latest_user: Option<String>,
    pub latest_assistant: Option<String>,
    pub latest_preflight: Option<String>,
    pub latest_diagnostic: Option<String>,
    pub latest_final_status: Option<String>,
    pub latest_gate_report: Option<String>,
}

pub fn summarize_recent_session(
    root: &std::path::Path,
    selector: &str,
) -> Result<SessionSummary, String> {
    let records = list_recent_sessions(root, 100)?;
    let record = select_record(&records, selector)?;
    write_session_summary(record)
}

pub(super) fn write_session_summary(record: SessionRecord) -> Result<SessionSummary, String> {
    let summary = scan::summarize_session_record(record)?;
    fs::write(&summary.summary_path, summary.to_markdown()).map_err(|error| {
        format!(
            "write summary {} failed: {error}",
            summary.summary_path.display()
        )
    })?;
    Ok(summary)
}
