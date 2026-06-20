use std::{
    fs, io,
    path::{Path, PathBuf},
};

use super::status_json::{
    bool_value_text, compact_line, json_bool_field, json_string_field, json_string_literal,
    scalar_value,
};

pub(super) const EVOLUTION_CANDIDATES_FILE: &str = "evolution-candidates.jsonl";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CandidateBacklogSummary {
    path: PathBuf,
    total: usize,
    invalid_count: usize,
    new_count: usize,
    accepted_count: usize,
    implemented_count: usize,
    rejected_count: usize,
    other_count: usize,
    implemented_validated_count: usize,
    implemented_unvalidated_count: usize,
    implemented_failed_count: usize,
    validation_ready: bool,
    latest: Option<CandidateBacklogItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CandidateBacklogItem {
    candidate_id: String,
    status: String,
    round: String,
    case_name: String,
    model: String,
    answer_preview: String,
    validation_command: String,
    validation_status_code: String,
    validation_passed: String,
    validation_unix: String,
}

pub(super) fn render_candidate_start_preflight(
    work_dir: &str,
    backlog_path: Option<&str>,
) -> io::Result<String> {
    let path = candidate_backlog_preflight_path(work_dir, backlog_path);
    let summary = read_candidate_backlog_summary(&path)?;
    let exists = summary.is_some();
    let ready = summary
        .as_ref()
        .map(|summary| summary.validation_ready)
        .unwrap_or(true);
    let total = summary.as_ref().map(|summary| summary.total).unwrap_or(0);
    let invalid = summary
        .as_ref()
        .map(|summary| summary.invalid_count)
        .unwrap_or(0);
    let accepted_pending = summary
        .as_ref()
        .map(|summary| summary.accepted_count)
        .unwrap_or(0);
    let implemented_validated = summary
        .as_ref()
        .map(|summary| summary.implemented_validated_count)
        .unwrap_or(0);
    let implemented_unvalidated = summary
        .as_ref()
        .map(|summary| summary.implemented_unvalidated_count)
        .unwrap_or(0);
    let implemented_failed = summary
        .as_ref()
        .map(|summary| summary.implemented_failed_count)
        .unwrap_or(0);

    Ok(format!(
        "candidate_preflight read_only=true starts_process=false sends_prompt=false writes_files=false\ncandidate_preflight backlog={} exists={} total={} invalid={}\ncandidate_preflight ready={} accepted_pending={} implemented_validated={} implemented_unvalidated={} implemented_failed={}",
        path.display(),
        bool_value_text(exists),
        total,
        invalid,
        bool_value_text(ready),
        accepted_pending,
        implemented_validated,
        implemented_unvalidated,
        implemented_failed
    ))
}

pub(super) fn candidate_start_preflight_ready(preflight: &str) -> bool {
    preflight
        .lines()
        .any(|line| line.contains("candidate_preflight ready=true "))
}

pub(super) fn candidate_backlog_preflight_path(
    work_dir: &str,
    backlog_path: Option<&str>,
) -> PathBuf {
    backlog_path
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| candidate_backlog_path(work_dir))
}

pub(super) fn candidate_backlog_path(work_dir: &str) -> PathBuf {
    Path::new(work_dir).join(EVOLUTION_CANDIDATES_FILE)
}

pub(super) fn candidate_start_blocked(summary: Option<&CandidateBacklogSummary>) -> bool {
    summary.is_some_and(|summary| !summary.validation_ready)
}

pub(super) fn daemon_start_gate_line(summary: Option<&CandidateBacklogSummary>) -> Option<String> {
    summary.map(|summary| {
        format!(
            "daemon_start_gate candidate_lifecycle_ready={} blocks_unattended_start={} accepted_pending={} implemented_unvalidated={} implemented_failed={} invalid={}",
            bool_value_text(summary.validation_ready),
            bool_value_text(!summary.validation_ready),
            summary.accepted_count,
            summary.implemented_unvalidated_count,
            summary.implemented_failed_count,
            summary.invalid_count
        )
    })
}

pub(super) fn candidate_backlog_status_json(
    path: &Path,
    summary: Option<&CandidateBacklogSummary>,
) -> String {
    match summary {
        Some(summary) => {
            let latest = summary
                .latest
                .as_ref()
                .map(candidate_backlog_latest_status_json)
                .unwrap_or_else(|| "null".to_owned());
            format!(
                "{{\"read_only\":true,\"starts_process\":false,\"sends_prompt\":false,\"path\":{},\"exists\":true,\"total\":{},\"invalid\":{},\"new\":{},\"accepted\":{},\"implemented\":{},\"rejected\":{},\"other\":{},\"validation_ready\":{},\"implemented_validated\":{},\"implemented_unvalidated\":{},\"implemented_failed\":{},\"latest\":{}}}",
                json_string_literal(&summary.path.display().to_string()),
                summary.total,
                summary.invalid_count,
                summary.new_count,
                summary.accepted_count,
                summary.implemented_count,
                summary.rejected_count,
                summary.other_count,
                bool_value_text(summary.validation_ready),
                summary.implemented_validated_count,
                summary.implemented_unvalidated_count,
                summary.implemented_failed_count,
                latest
            )
        }
        None => format!(
            "{{\"read_only\":true,\"starts_process\":false,\"sends_prompt\":false,\"path\":{},\"exists\":false,\"total\":0,\"invalid\":0,\"new\":0,\"accepted\":0,\"implemented\":0,\"rejected\":0,\"other\":0,\"validation_ready\":true,\"implemented_validated\":0,\"implemented_unvalidated\":0,\"implemented_failed\":0,\"latest\":null}}",
            json_string_literal(&path.display().to_string())
        ),
    }
}

fn candidate_backlog_latest_status_json(item: &CandidateBacklogItem) -> String {
    format!(
        "{{\"candidate_id\":{},\"status\":{},\"round\":{},\"case\":{},\"model\":{},\"validation_passed\":{},\"validation_status_code\":{},\"validated_unix\":{}}}",
        json_string_literal(&item.candidate_id),
        json_string_literal(&item.status),
        json_string_literal(&item.round),
        json_string_literal(&item.case_name),
        json_string_literal(&item.model),
        json_string_literal(&item.validation_passed),
        json_string_literal(&item.validation_status_code),
        json_string_literal(&item.validation_unix)
    )
}

pub(super) fn daemon_start_gate_status_json(summary: Option<&CandidateBacklogSummary>) -> String {
    match summary {
        Some(summary) => format!(
            "{{\"read_only\":true,\"starts_process\":false,\"sends_prompt\":false,\"candidate_lifecycle_ready\":{},\"blocks_unattended_start\":{},\"accepted_pending\":{},\"implemented_unvalidated\":{},\"implemented_failed\":{},\"invalid\":{}}}",
            bool_value_text(summary.validation_ready),
            bool_value_text(!summary.validation_ready),
            summary.accepted_count,
            summary.implemented_unvalidated_count,
            summary.implemented_failed_count,
            summary.invalid_count
        ),
        None => "{\"read_only\":true,\"starts_process\":false,\"sends_prompt\":false,\"candidate_lifecycle_ready\":true,\"blocks_unattended_start\":false,\"accepted_pending\":0,\"implemented_unvalidated\":0,\"implemented_failed\":0,\"invalid\":0}".to_owned(),
    }
}

pub(super) fn candidate_backlog_lines(work_dir: &str) -> Vec<String> {
    let path = candidate_backlog_path(work_dir);
    let Ok(Some(summary)) = read_candidate_backlog_summary(&path) else {
        return Vec::new();
    };

    let mut lines = vec![format!(
        "candidate_backlog path={} total={} new={} accepted={} implemented={} rejected={} other={} invalid={}",
        summary.path.display(),
        summary.total,
        summary.new_count,
        summary.accepted_count,
        summary.implemented_count,
        summary.rejected_count,
        summary.other_count,
        summary.invalid_count
    )];
    lines.push(format!(
        "candidate_backlog_validation ready={} accepted_pending={} implemented_validated={} implemented_unvalidated={} implemented_failed={}",
        bool_value_text(summary.validation_ready),
        summary.accepted_count,
        summary.implemented_validated_count,
        summary.implemented_unvalidated_count,
        summary.implemented_failed_count
    ));
    if let Some(latest) = summary.latest {
        lines.push(format!(
            "candidate_backlog_latest id={} status={} round={} case={} model={} preview={}",
            latest.candidate_id,
            latest.status,
            latest.round,
            latest.case_name,
            latest.model,
            compact_line(&latest.answer_preview, 220)
        ));
        if !latest.validation_command.is_empty() {
            lines.push(format!(
                "candidate_backlog_latest_validation passed={} status_code={} validated_unix={} command={}",
                latest.validation_passed,
                latest.validation_status_code,
                latest.validation_unix,
                compact_line(&latest.validation_command, 180)
            ));
        }
    }
    lines
}

pub(super) fn read_candidate_backlog_summary(
    path: &Path,
) -> io::Result<Option<CandidateBacklogSummary>> {
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(path)?;
    let mut invalid_count = 0usize;
    let mut items = Vec::<CandidateBacklogItem>::new();
    let mut latest_id = None::<String>;

    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        match json_string_field(line, "schema").as_deref() {
            Some("smartsteam.evolution_candidate.v1") => {
                let Some(item) = candidate_backlog_item_from_json(line) else {
                    invalid_count += 1;
                    continue;
                };
                latest_id = Some(item.candidate_id.clone());
                items.push(item);
            }
            Some("smartsteam.evolution_candidate_status.v1") => {
                let Some(candidate_id) = json_string_field(line, "candidate_id") else {
                    invalid_count += 1;
                    continue;
                };
                let status = json_string_field(line, "status")
                    .unwrap_or_else(|| "new".to_owned())
                    .trim()
                    .to_ascii_lowercase();
                if let Some(item) = items
                    .iter_mut()
                    .find(|item| item.candidate_id == candidate_id)
                {
                    item.status = if status.is_empty() {
                        "new".to_owned()
                    } else {
                        status
                    };
                    latest_id = Some(candidate_id);
                } else {
                    invalid_count += 1;
                }
            }
            Some("smartsteam.evolution_candidate_validation.v1") => {
                let Some(candidate_id) = json_string_field(line, "candidate_id") else {
                    invalid_count += 1;
                    continue;
                };
                if let Some(item) = items
                    .iter_mut()
                    .find(|item| item.candidate_id == candidate_id)
                {
                    item.validation_command =
                        json_string_field(line, "command").unwrap_or_default();
                    item.validation_status_code = scalar_value(line, "status_code");
                    item.validation_passed = json_bool_field(line, "passed")
                        .map(bool_value_text)
                        .unwrap_or("unknown")
                        .to_owned();
                    item.validation_unix = scalar_value(line, "validated_unix");
                    latest_id = Some(candidate_id);
                } else {
                    invalid_count += 1;
                }
            }
            _ => {
                invalid_count += 1;
            }
        }
    }

    let mut summary = CandidateBacklogSummary {
        path: path.to_path_buf(),
        total: items.len(),
        invalid_count,
        new_count: 0,
        accepted_count: 0,
        implemented_count: 0,
        rejected_count: 0,
        other_count: 0,
        implemented_validated_count: 0,
        implemented_unvalidated_count: 0,
        implemented_failed_count: 0,
        validation_ready: false,
        latest: None,
    };
    for item in &items {
        match item.status.as_str() {
            "new" => summary.new_count += 1,
            "accepted" => summary.accepted_count += 1,
            "implemented" => {
                summary.implemented_count += 1;
                match item.validation_passed.as_str() {
                    "true" => summary.implemented_validated_count += 1,
                    "false" => summary.implemented_failed_count += 1,
                    _ => summary.implemented_unvalidated_count += 1,
                }
            }
            "rejected" => summary.rejected_count += 1,
            _ => summary.other_count += 1,
        }
    }
    summary.validation_ready = summary.invalid_count == 0
        && summary.accepted_count == 0
        && summary.implemented_unvalidated_count == 0
        && summary.implemented_failed_count == 0;
    summary.latest = latest_id
        .and_then(|id| items.iter().find(|item| item.candidate_id == id).cloned())
        .or_else(|| items.last().cloned());

    Ok(Some(summary))
}

fn candidate_backlog_item_from_json(line: &str) -> Option<CandidateBacklogItem> {
    let candidate_id = json_string_field(line, "candidate_id")?;
    let status = json_string_field(line, "status")
        .unwrap_or_else(|| "new".to_owned())
        .trim()
        .to_ascii_lowercase();
    Some(CandidateBacklogItem {
        candidate_id,
        status: if status.is_empty() {
            "new".to_owned()
        } else {
            status
        },
        round: scalar_value(line, "round"),
        case_name: json_string_field(line, "case").unwrap_or_else(|| "unknown".to_owned()),
        model: json_string_field(line, "model").unwrap_or_else(|| "unknown".to_owned()),
        answer_preview: json_string_field(line, "answer_preview").unwrap_or_default(),
        validation_command: String::new(),
        validation_status_code: "unknown".to_owned(),
        validation_passed: "unknown".to_owned(),
        validation_unix: "unknown".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    #[test]
    fn start_preflight_allows_missing_or_validated_candidate_backlog() {
        let work_dir = std::env::temp_dir().join(format!(
            "smartsteam-forge-start-preflight-ready-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&work_dir).unwrap();

        let empty_preflight =
            render_candidate_start_preflight(&work_dir.to_string_lossy(), None).unwrap();
        assert!(candidate_start_preflight_ready(&empty_preflight));
        assert!(empty_preflight.contains("candidate_preflight ready=true"));
        assert!(empty_preflight.contains("exists=false total=0 invalid=0"));

        fs::write(
            work_dir.join(EVOLUTION_CANDIDATES_FILE),
            [
                r#"{"schema":"smartsteam.evolution_candidate.v1","candidate_id":"smartsteam-candidate-one","status":"new","round":"1","case":"case-1","model":"model-a","answer_preview":"candidate"}"#,
                r#"{"schema":"smartsteam.evolution_candidate_status.v1","candidate_id":"smartsteam-candidate-one","status":"implemented","note":"done"}"#,
                r#"{"schema":"smartsteam.evolution_candidate_validation.v1","candidate_id":"smartsteam-candidate-one","command":"cargo test -q --manifest-path tools/smartsteam-forge/Cargo.toml","status_code":0,"passed":true,"note":"green","validated_unix":456}"#,
            ]
            .join("\n"),
        )
        .unwrap();

        let ready_preflight =
            render_candidate_start_preflight(&work_dir.to_string_lossy(), None).unwrap();
        let _ = fs::remove_dir_all(&work_dir);

        assert!(candidate_start_preflight_ready(&ready_preflight));
        assert!(ready_preflight.contains("candidate_preflight ready=true accepted_pending=0 implemented_validated=1 implemented_unvalidated=0 implemented_failed=0"));
    }

    #[test]
    fn start_preflight_blocks_dirty_candidate_backlog() {
        let work_dir = std::env::temp_dir().join(format!(
            "smartsteam-forge-start-preflight-blocks-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&work_dir).unwrap();
        fs::write(
            work_dir.join(EVOLUTION_CANDIDATES_FILE),
            [
                r#"{"schema":"smartsteam.evolution_candidate.v1","candidate_id":"smartsteam-candidate-accepted","status":"accepted","round":"1","case":"case-1","model":"model-a","answer_preview":"accepted"}"#,
                r#"{"schema":"smartsteam.evolution_candidate.v1","candidate_id":"smartsteam-candidate-unvalidated","status":"implemented","round":"2","case":"case-2","model":"model-b","answer_preview":"unvalidated"}"#,
                r#"{"schema":"smartsteam.evolution_candidate.v1","candidate_id":"smartsteam-candidate-failed","status":"implemented","round":"3","case":"case-3","model":"model-c","answer_preview":"failed"}"#,
                r#"{"schema":"smartsteam.evolution_candidate_validation.v1","candidate_id":"smartsteam-candidate-failed","command":"cargo test","status_code":1,"passed":false,"validated_unix":789}"#,
                "not json",
            ]
            .join("\n"),
        )
        .unwrap();

        let preflight =
            render_candidate_start_preflight(&work_dir.to_string_lossy(), None).unwrap();
        let _ = fs::remove_dir_all(&work_dir);

        assert!(!candidate_start_preflight_ready(&preflight));
        assert!(preflight.contains("candidate_preflight backlog="));
        assert!(preflight.contains("exists=true total=3 invalid=1"));
        assert!(preflight.contains("candidate_preflight ready=false accepted_pending=1 implemented_validated=0 implemented_unvalidated=1 implemented_failed=1"));
    }

    #[test]
    fn daemon_start_gate_json_is_read_only() {
        let json = daemon_start_gate_status_json(None);

        assert!(json.contains("\"read_only\":true"));
        assert!(json.contains("\"starts_process\":false"));
        assert!(json.contains("\"sends_prompt\":false"));
        assert!(json.contains("\"candidate_lifecycle_ready\":true"));
        assert!(json.contains("\"blocks_unattended_start\":false"));
    }

    #[test]
    fn candidate_backlog_status_json_is_read_only() {
        let path = Path::new("target\\evolution\\daemon\\evolution-candidates.jsonl");
        let json = candidate_backlog_status_json(path, None);

        assert!(json.contains("\"read_only\":true"));
        assert!(json.contains("\"starts_process\":false"));
        assert!(json.contains("\"sends_prompt\":false"));
        assert!(json.contains("\"validation_ready\":true"));
    }
}
