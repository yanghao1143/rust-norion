use std::fs;
use std::io::{BufRead, BufReader};

use crate::provider::FinalPayloadSummary;
use crate::provider::json::json_string_field;

use super::super::SessionRecord;
use super::super::helpers::{is_diagnostic_kind, is_health_check_kind, is_preflight_kind};
use super::SessionSummary;

pub(super) fn summarize_session_record(record: SessionRecord) -> Result<SessionSummary, String> {
    let path = record.transcript_path.clone();
    let file = fs::File::open(&path)
        .map_err(|error| format!("open session {} failed: {error}", path.display()))?;
    let mut scan = SummaryScan::new(record);

    for line in BufReader::new(file).lines() {
        let line =
            line.map_err(|error| format!("read session {} failed: {error}", path.display()))?;
        scan.record_line(&line);
    }

    Ok(scan.into_summary())
}

#[derive(Debug)]
struct SummaryScan {
    record: SessionRecord,
    line_count: usize,
    message_count: usize,
    user_count: usize,
    assistant_count: usize,
    health_check_count: usize,
    preflight_count: usize,
    diagnostic_count: usize,
    final_payload_count: usize,
    gate_report_count: usize,
    error_count: usize,
    latest_user: Option<String>,
    latest_assistant: Option<String>,
    latest_preflight: Option<String>,
    latest_diagnostic: Option<String>,
    latest_final_status: Option<String>,
    latest_gate_report: Option<String>,
}

impl SummaryScan {
    fn new(record: SessionRecord) -> Self {
        Self {
            record,
            line_count: 0,
            message_count: 0,
            user_count: 0,
            assistant_count: 0,
            health_check_count: 0,
            preflight_count: 0,
            diagnostic_count: 0,
            final_payload_count: 0,
            gate_report_count: 0,
            error_count: 0,
            latest_user: None,
            latest_assistant: None,
            latest_preflight: None,
            latest_diagnostic: None,
            latest_final_status: None,
            latest_gate_report: None,
        }
    }

    fn record_line(&mut self, line: &str) {
        self.line_count += 1;
        let kind = json_string_field(line, "kind");
        match kind.as_deref() {
            Some("message") => self.record_message(line),
            Some("final_payload") => self.record_final_payload(line),
            Some("gate_report") => self.record_gate_report(line),
            Some(kind) if is_health_check_kind(kind) => self.health_check_count += 1,
            Some(kind) if is_preflight_kind(kind) => self.record_preflight(line),
            Some(kind) if is_diagnostic_kind(kind) => self.record_diagnostic(line),
            Some("error") => self.error_count += 1,
            _ => {}
        }
    }

    fn record_message(&mut self, line: &str) {
        self.message_count += 1;
        let role = json_string_field(line, "role");
        let content = json_string_field(line, "content").unwrap_or_default();
        match role.as_deref() {
            Some("user") => {
                self.user_count += 1;
                self.latest_user = Some(content);
            }
            Some("assistant") => {
                self.assistant_count += 1;
                self.latest_assistant = Some(content);
            }
            _ => {}
        }
    }

    fn record_final_payload(&mut self, line: &str) {
        self.final_payload_count += 1;
        self.latest_final_status = json_string_field(line, "content")
            .map(|payload| FinalPayloadSummary::parse(&payload).status_line());
    }

    fn record_gate_report(&mut self, line: &str) {
        self.gate_report_count += 1;
        self.latest_gate_report = json_string_field(line, "content");
    }

    fn record_preflight(&mut self, line: &str) {
        self.preflight_count += 1;
        self.latest_preflight = json_string_field(line, "content");
    }

    fn record_diagnostic(&mut self, line: &str) {
        self.diagnostic_count += 1;
        self.latest_diagnostic = json_string_field(line, "content");
    }

    fn into_summary(self) -> SessionSummary {
        let summary_path = self.record.transcript_path.with_extension("summary.md");
        SessionSummary {
            record: self.record,
            summary_path,
            line_count: self.line_count,
            message_count: self.message_count,
            user_count: self.user_count,
            assistant_count: self.assistant_count,
            health_check_count: self.health_check_count,
            preflight_count: self.preflight_count,
            diagnostic_count: self.diagnostic_count,
            final_payload_count: self.final_payload_count,
            gate_report_count: self.gate_report_count,
            error_count: self.error_count,
            latest_user: self.latest_user,
            latest_assistant: self.latest_assistant,
            latest_preflight: self.latest_preflight,
            latest_diagnostic: self.latest_diagnostic,
            latest_final_status: self.latest_final_status,
            latest_gate_report: self.latest_gate_report,
        }
    }
}
