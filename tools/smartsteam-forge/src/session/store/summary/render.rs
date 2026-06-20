use super::super::helpers::{gate_outcome, short_preview};
use super::SessionSummary;

impl SessionSummary {
    pub fn summary_line(&self) -> String {
        let latest_user = self
            .latest_user
            .as_deref()
            .map(short_preview)
            .unwrap_or_else(|| "none".to_owned());
        let latest_assistant = self
            .latest_assistant
            .as_deref()
            .map(short_preview)
            .unwrap_or_else(|| "none".to_owned());
        let latest_gate = self
            .latest_gate_report
            .as_deref()
            .and_then(gate_outcome)
            .unwrap_or("none");
        let latest_final = self
            .latest_final_status
            .as_deref()
            .map(short_preview)
            .unwrap_or_else(|| "none".to_owned());
        format!(
            "session={} messages={} user={} assistant={} health_checks={} preflights={} diagnostics={} final_payloads={} gate_reports={} gate={} errors={} latest_user=\"{}\" latest_assistant=\"{}\" latest_final=\"{}\" summary={}",
            self.record.id,
            self.message_count,
            self.user_count,
            self.assistant_count,
            self.health_check_count,
            self.preflight_count,
            self.diagnostic_count,
            self.final_payload_count,
            self.gate_report_count,
            latest_gate,
            self.error_count,
            latest_user,
            latest_assistant,
            latest_final,
            self.summary_path.display()
        )
    }

    pub fn to_markdown(&self) -> String {
        let latest_user = self.latest_user.as_deref().unwrap_or("");
        let latest_assistant = self.latest_assistant.as_deref().unwrap_or("");
        let latest_preflight = self.latest_preflight.as_deref().unwrap_or("");
        let latest_diagnostic = self.latest_diagnostic.as_deref().unwrap_or("");
        let latest_final_status = self.latest_final_status.as_deref().unwrap_or("");
        let latest_gate_report = self.latest_gate_report.as_deref().unwrap_or("");
        format!(
            "# SmartSteam Forge Session Summary\n\n\
             - Session: `{}`\n\
             - Transcript: `{}`\n\
             - Lines: {}\n\
             - Messages: {}\n\
             - User messages: {}\n\
             - Assistant messages: {}\n\
             - Health checks: {}\n\
             - Preflights: {}\n\
             - Diagnostics: {}\n\
             - Final payloads: {}\n\
             - Gate reports: {}\n\
             - Errors: {}\n\n\
             ## Latest User\n\n{}\n\n\
             ## Latest Assistant\n\n{}\n\n\
             ## Latest Preflight\n\n{}\n\n\
             ## Latest Diagnostic\n\n{}\n\n\
             ## Latest Final Payload\n\n{}\n\n\
             ## Latest Gate Report\n\n{}\n",
            self.record.id,
            self.record.transcript_path.display(),
            self.line_count,
            self.message_count,
            self.user_count,
            self.assistant_count,
            self.health_check_count,
            self.preflight_count,
            self.diagnostic_count,
            self.final_payload_count,
            self.gate_report_count,
            self.error_count,
            latest_user,
            latest_assistant,
            latest_preflight,
            latest_diagnostic,
            latest_final_status,
            latest_gate_report
        )
    }

    pub fn to_context_prompt(&self) -> String {
        let latest_user = preview_or_none(self.latest_user.as_deref());
        let latest_assistant = preview_or_none(self.latest_assistant.as_deref());
        let latest_gate_report = preview_or_none(self.latest_gate_report.as_deref());
        let latest_preflight = preview_or_none(self.latest_preflight.as_deref());
        let latest_final = preview_or_none(self.latest_final_status.as_deref());
        format!(
            "SmartSteam Forge resumed transcript summary.\n\
             Session: {}\n\
             Transcript: {}\n\
             Messages: {} (user={}, assistant={})\n\
             Health checks: {}\n\
             Preflights: {}\n\
             Diagnostics: {}\n\
             Final payloads: {}\n\
             Gate reports: {}\n\
             Errors: {}\n\
             Latest user: {}\n\
             Latest assistant: {}\n\
             Latest preflight: {}\n\
             Latest final: {}\n\
             Latest gate report: {}",
            self.record.id,
            self.record.transcript_path.display(),
            self.message_count,
            self.user_count,
            self.assistant_count,
            self.health_check_count,
            self.preflight_count,
            self.diagnostic_count,
            self.final_payload_count,
            self.gate_report_count,
            self.error_count,
            latest_user,
            latest_assistant,
            latest_preflight,
            latest_final,
            latest_gate_report
        )
    }
}

fn preview_or_none(value: Option<&str>) -> String {
    value
        .map(short_preview)
        .unwrap_or_else(|| "none".to_owned())
}
