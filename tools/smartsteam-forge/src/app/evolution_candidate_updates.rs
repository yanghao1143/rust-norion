use std::{
    fs, io,
    io::Write,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use super::evolution_candidate_backlog::{candidate_record_ids, current_candidate_status};
use super::evolution_candidate_events::{
    candidate_status_event_json, candidate_validation_event_json,
};
use super::evolution_candidate_lifecycle::{
    normalize_candidate_status, parse_validation_status_code,
};
use super::evolution_candidate_model::{EvolutionCandidatePaths, candidate_backlog_path};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CandidateValidationUpdate {
    pub(super) backlog_path: PathBuf,
    pub(super) candidate_id: String,
    pub(super) command: String,
    pub(super) status_code: i32,
    pub(super) passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CandidateMarkUpdate {
    pub(super) backlog_path: PathBuf,
    pub(super) candidate_id: String,
    pub(super) previous_status: String,
    pub(super) status: String,
}

pub(super) fn validate_candidate_backlog(
    work_dir: &str,
    backlog_path: Option<&str>,
    candidate_id: &str,
    command: &str,
    status_code: &str,
    note: Option<&str>,
) -> io::Result<CandidateValidationUpdate> {
    let candidate_id = candidate_id.trim();
    if candidate_id.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "candidate id is required",
        ));
    }
    let command = command.trim();
    if command.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "validation command is required",
        ));
    }
    let status_code = parse_validation_status_code(status_code)?;
    let paths = EvolutionCandidatePaths::new(work_dir);
    let backlog_path = candidate_backlog_path(&paths, backlog_path);
    let text = fs::read_to_string(&backlog_path).unwrap_or_default();
    ensure_candidate_exists(&text, candidate_id, &backlog_path)?;

    ensure_parent_dir(&backlog_path)?;
    let validated_unix = unix_now()?;
    let passed = status_code == 0;
    let event = candidate_validation_event_json(
        candidate_id,
        command,
        status_code,
        passed,
        note.unwrap_or(""),
        validated_unix,
    );
    append_jsonl_event(&backlog_path, &text, &event)?;

    Ok(CandidateValidationUpdate {
        backlog_path,
        candidate_id: candidate_id.to_owned(),
        command: command.to_owned(),
        status_code,
        passed,
    })
}

pub(super) fn mark_candidate_backlog(
    work_dir: &str,
    backlog_path: Option<&str>,
    candidate_id: &str,
    status: &str,
    note: Option<&str>,
) -> io::Result<CandidateMarkUpdate> {
    let candidate_id = candidate_id.trim();
    if candidate_id.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "candidate id is required",
        ));
    }
    let status = normalize_candidate_status(status)?;
    let paths = EvolutionCandidatePaths::new(work_dir);
    let backlog_path = candidate_backlog_path(&paths, backlog_path);
    let text = fs::read_to_string(&backlog_path).unwrap_or_default();
    ensure_candidate_exists(&text, candidate_id, &backlog_path)?;
    let previous_status =
        current_candidate_status(&text, candidate_id).unwrap_or_else(|| "unknown".to_owned());

    ensure_parent_dir(&backlog_path)?;
    let changed_unix = unix_now()?;
    let event =
        candidate_status_event_json(candidate_id, &status, note.unwrap_or(""), changed_unix);
    append_jsonl_event(&backlog_path, &text, &event)?;

    Ok(CandidateMarkUpdate {
        backlog_path,
        candidate_id: candidate_id.to_owned(),
        previous_status,
        status,
    })
}

fn ensure_candidate_exists(
    text: &str,
    candidate_id: &str,
    backlog_path: &PathBuf,
) -> io::Result<()> {
    let ids = candidate_record_ids(text);
    if ids.iter().any(|id| id == candidate_id) {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "candidate {candidate_id} not found in backlog {}",
                backlog_path.display()
            ),
        ))
    }
}

fn ensure_parent_dir(path: &PathBuf) -> io::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn append_jsonl_event(path: &PathBuf, existing_text: &str, event: &str) -> io::Result<()> {
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    if !existing_text.is_empty() && !existing_text.ends_with('\n') {
        writeln!(file)?;
    }
    writeln!(file, "{event}")
}

fn unix_now() -> io::Result<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| io::Error::other(format!("clock error: {error}")))
        .map(|duration| duration.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_work_dir(name: &str) -> PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "smartsteam-candidate-updates-{name}-{}-{now}",
            std::process::id()
        ))
    }

    #[test]
    fn validation_update_appends_structured_event() {
        let work_dir = temp_work_dir("validation");
        fs::create_dir_all(&work_dir).unwrap();
        let backlog = work_dir.join("evolution-candidates.jsonl");
        fs::write(
            &backlog,
            r#"{"schema":"smartsteam.evolution_candidate.v1","candidate_id":"candidate-a","status":"new","source":"report.last","round":"1","case":"case-1","model":"model","tokens":"1","elapsed_ms":"1","feedback":"1","self_improve":"true","answer_preview":"candidate"}"#,
        )
        .unwrap();

        let update = validate_candidate_backlog(
            &work_dir.to_string_lossy(),
            None,
            "candidate-a",
            "cargo test",
            "0",
            Some("green"),
        )
        .unwrap();
        let text = fs::read_to_string(&backlog).unwrap();
        let _ = fs::remove_dir_all(&work_dir);

        assert_eq!(update.candidate_id, "candidate-a");
        assert_eq!(update.command, "cargo test");
        assert_eq!(update.status_code, 0);
        assert!(update.passed);
        assert!(text.contains("\"schema\":\"smartsteam.evolution_candidate_validation.v1\""));
        assert!(text.contains("\"passed\":true"));
        assert!(text.contains("\"note\":\"green\""));
    }

    #[test]
    fn mark_update_tracks_previous_status() {
        let work_dir = temp_work_dir("mark");
        fs::create_dir_all(&work_dir).unwrap();
        let backlog = work_dir.join("evolution-candidates.jsonl");
        fs::write(
            &backlog,
            [
                r#"{"schema":"smartsteam.evolution_candidate.v1","candidate_id":"candidate-a","status":"new","source":"report.last","round":"1","case":"case-1","model":"model","tokens":"1","elapsed_ms":"1","feedback":"1","self_improve":"true","answer_preview":"candidate"}"#,
                r#"{"schema":"smartsteam.evolution_candidate_status.v1","candidate_id":"candidate-a","status":"accepted","note":"ready","changed_unix":123}"#,
            ]
            .join("\n"),
        )
        .unwrap();

        let update = mark_candidate_backlog(
            &work_dir.to_string_lossy(),
            None,
            "candidate-a",
            "implemented",
            Some("done"),
        )
        .unwrap();
        let text = fs::read_to_string(&backlog).unwrap();
        let _ = fs::remove_dir_all(&work_dir);

        assert_eq!(update.previous_status, "accepted");
        assert_eq!(update.status, "implemented");
        assert!(text.contains("\"schema\":\"smartsteam.evolution_candidate_status.v1\""));
        assert!(text.contains("\"status\":\"implemented\""));
        assert!(text.contains("\"note\":\"done\""));
    }
}
