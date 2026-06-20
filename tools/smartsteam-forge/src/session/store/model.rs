use std::path::PathBuf;

use super::helpers::{gate_outcome, short_preview};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredSession {
    pub id: String,
    pub transcript_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumedSession {
    pub record: SessionRecord,
    pub messages: Vec<TranscriptMessage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionFilter {
    All,
    Passed,
    Failed,
}

impl SessionFilter {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "" | "all" | "any" => Some(Self::All),
            "pass" | "passed" | "passing" => Some(Self::Passed),
            "fail" | "failed" | "failing" => Some(Self::Failed),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Passed => "passed",
            Self::Failed => "failed",
        }
    }

    pub(super) fn matches(self, record: &SessionRecord) -> bool {
        match self {
            Self::All => true,
            Self::Passed => record.gate_outcome() == Some("PASS"),
            Self::Failed => record.gate_outcome() == Some("FAIL"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRecord {
    pub id: String,
    pub transcript_path: PathBuf,
    pub modified_secs: u64,
    pub line_count: usize,
    pub first_user: Option<String>,
    pub last_assistant: Option<String>,
    pub preflight_count: usize,
    pub latest_preflight: Option<String>,
    pub final_payload_count: usize,
    pub latest_final_status: Option<String>,
    pub gate_report_count: usize,
    pub latest_gate_report: Option<String>,
}

impl SessionRecord {
    pub fn gate_outcome(&self) -> Option<&'static str> {
        self.latest_gate_report.as_deref().and_then(gate_outcome)
    }

    pub fn summary_line(&self) -> String {
        let first = self
            .first_user
            .as_deref()
            .map(short_preview)
            .unwrap_or_else(|| "no user prompt".to_owned());
        let last = self
            .last_assistant
            .as_deref()
            .map(short_preview)
            .unwrap_or_else(|| "no assistant answer".to_owned());
        let gate = self.gate_outcome().unwrap_or("none");
        let gate_preview = self
            .latest_gate_report
            .as_deref()
            .map(short_preview)
            .unwrap_or_else(|| "no gate report".to_owned());
        let preflight_preview = self
            .latest_preflight
            .as_deref()
            .map(short_preview)
            .unwrap_or_else(|| "no preflight".to_owned());
        let final_preview = self
            .latest_final_status
            .as_deref()
            .map(short_preview)
            .unwrap_or_else(|| "no final payload".to_owned());
        format!(
            "{} lines={} modified={} preflights={} final_payloads={} gate={} gate_reports={} first=\"{}\" last=\"{}\" preflight_preview=\"{}\" final_preview=\"{}\" gate_preview=\"{}\" path={}",
            self.id,
            self.line_count,
            self.modified_secs,
            self.preflight_count,
            self.final_payload_count,
            gate,
            self.gate_report_count,
            first,
            last,
            preflight_preview,
            final_preview,
            gate_preview,
            self.transcript_path.display()
        )
    }
}
