use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::UNIX_EPOCH;

use crate::provider::FinalPayloadSummary;
use crate::provider::json::json_string_field;

use super::helpers::{is_preflight_kind, select_record_error};
use super::{SessionFilter, SessionRecord, TranscriptMessage};

pub fn list_recent_sessions(root: &Path, limit: usize) -> Result<Vec<SessionRecord>, String> {
    list_recent_sessions_filtered(root, SessionFilter::All, limit)
}

pub fn list_recent_sessions_filtered(
    root: &Path,
    filter: SessionFilter,
    limit: usize,
) -> Result<Vec<SessionRecord>, String> {
    let mut records = Vec::new();
    if !root.exists() {
        return Ok(records);
    }
    let entries = fs::read_dir(root)
        .map_err(|error| format!("read session directory {} failed: {error}", root.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("read session entry failed: {error}"))?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("jsonl") {
            continue;
        }
        records.push(read_session_record(&path)?);
    }
    records.sort_by(|left, right| {
        right
            .modified_secs
            .cmp(&left.modified_secs)
            .then_with(|| right.id.cmp(&left.id))
    });
    if filter != SessionFilter::All {
        records.retain(|record| filter.matches(record));
    }
    records.truncate(limit);
    Ok(records)
}

pub(super) fn select_record(
    records: &[SessionRecord],
    selector: &str,
) -> Result<SessionRecord, String> {
    if records.is_empty() {
        return Err("no recorded sessions".to_owned());
    }
    let selector = selector.trim();
    if selector.is_empty() {
        return Ok(records[0].clone());
    }
    if let Ok(index) = selector.parse::<usize>() {
        return records
            .get(index.saturating_sub(1))
            .cloned()
            .ok_or_else(|| format!("session index {index} is out of range"));
    }
    let mut matches = records
        .iter()
        .filter(|record| record.id == selector || record.id.starts_with(selector))
        .cloned()
        .collect::<Vec<_>>();
    if matches.len() == 1 {
        return Ok(matches.remove(0));
    }
    select_record_error(selector, matches.is_empty())
}

pub(super) fn read_session_record(path: &Path) -> Result<SessionRecord, String> {
    let file = fs::File::open(path)
        .map_err(|error| format!("open session {} failed: {error}", path.display()))?;
    let mut first_user = None;
    let mut last_assistant = None;
    let mut preflight_count = 0_usize;
    let mut latest_preflight = None;
    let mut final_payload_count = 0_usize;
    let mut latest_final_status = None;
    let mut gate_report_count = 0_usize;
    let mut latest_gate_report = None;
    let mut line_count = 0_usize;
    for line in BufReader::new(file).lines() {
        let line =
            line.map_err(|error| format!("read session {} failed: {error}", path.display()))?;
        line_count += 1;
        match json_string_field(&line, "kind").as_deref() {
            Some("message") => {
                let role = json_string_field(&line, "role");
                let content = json_string_field(&line, "content");
                match (role.as_deref(), content) {
                    (Some("user"), Some(content)) if first_user.is_none() => {
                        first_user = Some(content)
                    }
                    (Some("assistant"), Some(content)) => last_assistant = Some(content),
                    _ => {}
                }
            }
            Some("gate_report") => {
                gate_report_count += 1;
                latest_gate_report = json_string_field(&line, "content");
            }
            Some(kind) if is_preflight_kind(kind) => {
                preflight_count += 1;
                latest_preflight = json_string_field(&line, "content");
            }
            Some("final_payload") => {
                final_payload_count += 1;
                latest_final_status = json_string_field(&line, "content")
                    .map(|payload| FinalPayloadSummary::parse(&payload).status_line());
            }
            _ => {}
        }
    }
    let metadata = fs::metadata(path)
        .map_err(|error| format!("read session metadata {} failed: {error}", path.display()))?;
    let modified_secs = metadata
        .modified()
        .unwrap_or(UNIX_EPOCH)
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let id = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("unknown")
        .to_owned();
    Ok(SessionRecord {
        id,
        transcript_path: path.to_path_buf(),
        modified_secs,
        line_count,
        first_user,
        last_assistant,
        preflight_count,
        latest_preflight,
        final_payload_count,
        latest_final_status,
        gate_report_count,
        latest_gate_report,
    })
}

pub(super) fn read_transcript_messages(
    path: &Path,
    max_messages: usize,
) -> Result<Vec<TranscriptMessage>, String> {
    let file = fs::File::open(path)
        .map_err(|error| format!("open session {} failed: {error}", path.display()))?;
    let mut messages = Vec::new();
    for line in BufReader::new(file).lines() {
        let line =
            line.map_err(|error| format!("read session {} failed: {error}", path.display()))?;
        if json_string_field(&line, "kind").as_deref() != Some("message") {
            continue;
        }
        let Some(role) = json_string_field(&line, "role") else {
            continue;
        };
        if !matches!(role.as_str(), "user" | "assistant") {
            continue;
        }
        let Some(content) = json_string_field(&line, "content") else {
            continue;
        };
        if !content.trim().is_empty() {
            messages.push(TranscriptMessage { role, content });
        }
    }
    if messages.len() > max_messages {
        let drop_count = messages.len() - max_messages;
        messages.drain(..drop_count);
    }
    Ok(messages)
}
