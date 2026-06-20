use std::{fs, io, io::Write, path::Path};

use super::evolution_candidate_events::{candidate_backlog_json, candidate_id};
use super::evolution_candidate_model::{
    BacklogAppendResult, EvolutionCandidate, EvolutionCandidateBacklogItem,
};
use super::status_json::{
    bool_value_text, json_bool_field, json_number_field, json_string_field, scalar_value,
};

pub(super) fn append_candidate_backlog(
    path: &Path,
    candidates: &[EvolutionCandidate],
) -> io::Result<BacklogAppendResult> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    let existing_text = fs::read_to_string(path).unwrap_or_default();
    let mut existing_ids = existing_candidate_ids(&existing_text);
    let existing = existing_ids.len();
    let mut appended = Vec::new();
    let mut skipped = 0usize;

    for candidate in candidates {
        let id = candidate_id(candidate);
        if existing_ids.iter().any(|existing| existing == &id) {
            skipped += 1;
            continue;
        }
        existing_ids.push(id.clone());
        appended.push(candidate_backlog_json(candidate, &id));
    }

    if !appended.is_empty() {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        if !existing_text.is_empty() && !existing_text.ends_with('\n') {
            writeln!(file)?;
        }
        for line in &appended {
            writeln!(file, "{line}")?;
        }
    }

    Ok(BacklogAppendResult {
        path: path.to_path_buf(),
        existing,
        appended: appended.len(),
        skipped,
    })
}

fn existing_candidate_ids(text: &str) -> Vec<String> {
    candidate_record_ids(text)
}

pub(super) fn candidate_record_ids(text: &str) -> Vec<String> {
    text.lines()
        .filter(|line| {
            json_string_field(line, "schema").as_deref()
                == Some("smartsteam.evolution_candidate.v1")
        })
        .filter_map(|line| json_string_field(line, "candidate_id"))
        .collect()
}

pub(super) fn current_candidate_status(text: &str, candidate_id: &str) -> Option<String> {
    text.lines()
        .filter(|line| json_string_field(line, "candidate_id").as_deref() == Some(candidate_id))
        .filter_map(|line| json_string_field(line, "status"))
        .last()
}

pub(super) fn read_candidate_backlog_items(
    path: &Path,
) -> io::Result<(Vec<EvolutionCandidateBacklogItem>, usize)> {
    if !path.is_file() {
        return Ok((Vec::new(), 0));
    }
    let text = fs::read_to_string(path)?;
    let mut invalid_count = 0usize;
    let mut items = Vec::<EvolutionCandidateBacklogItem>::new();

    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        match json_string_field(line, "schema").as_deref() {
            Some("smartsteam.evolution_candidate.v1") => {
                let Some(item) = candidate_backlog_item_from_json(line) else {
                    invalid_count += 1;
                    continue;
                };
                if items
                    .iter()
                    .any(|existing| existing.candidate_id == item.candidate_id)
                {
                    invalid_count += 1;
                    continue;
                }
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
                    item.note = json_string_field(line, "note").unwrap_or_default();
                    item.changed_unix = scalar_value(line, "changed_unix");
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
                    item.validation_status_code = string_or_scalar_value(line, "status_code");
                    item.validation_passed = json_bool_field(line, "passed")
                        .map(bool_value_text)
                        .unwrap_or("unknown")
                        .to_owned();
                    item.validation_note = json_string_field(line, "note").unwrap_or_default();
                    item.validation_unix = scalar_value(line, "validated_unix");
                } else {
                    invalid_count += 1;
                }
            }
            _ => invalid_count += 1,
        }
    }

    Ok((items, invalid_count))
}

fn candidate_backlog_item_from_json(line: &str) -> Option<EvolutionCandidateBacklogItem> {
    let candidate_id = json_string_field(line, "candidate_id")?;
    let status = json_string_field(line, "status")
        .unwrap_or_else(|| "new".to_owned())
        .trim()
        .to_ascii_lowercase();
    Some(EvolutionCandidateBacklogItem {
        candidate_id,
        status: if status.is_empty() {
            "new".to_owned()
        } else {
            status
        },
        source: string_value(line, "source"),
        round: string_or_scalar_value(line, "round"),
        case_name: string_value(line, "case"),
        model: string_value(line, "model"),
        tokens: string_value(line, "tokens"),
        elapsed_ms: string_value(line, "elapsed_ms"),
        feedback: string_value(line, "feedback"),
        self_improve: string_value(line, "self_improve"),
        answer_preview: json_string_field(line, "answer_preview").unwrap_or_default(),
        note: String::new(),
        changed_unix: "unknown".to_owned(),
        validation_command: String::new(),
        validation_status_code: "unknown".to_owned(),
        validation_passed: "unknown".to_owned(),
        validation_note: String::new(),
        validation_unix: "unknown".to_owned(),
    })
}

fn string_or_scalar_value(object: &str, field: &str) -> String {
    json_string_field(object, field)
        .or_else(|| json_number_field(object, field))
        .unwrap_or_else(|| "unknown".to_owned())
}

fn string_value(object: &str, field: &str) -> String {
    json_string_field(object, field).unwrap_or_else(|| "unknown".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_candidate(round: &str, answer_preview: &str) -> EvolutionCandidate {
        EvolutionCandidate {
            source: "report.last".to_owned(),
            round: round.to_owned(),
            case_name: format!("case-{round}"),
            model: "model-a".to_owned(),
            tokens: "64".to_owned(),
            elapsed_ms: "100".to_owned(),
            feedback: "4".to_owned(),
            self_improve: "true".to_owned(),
            answer_preview: answer_preview.to_owned(),
        }
    }

    #[test]
    fn candidate_ids_only_include_backlog_candidate_records() {
        let text = [
            r#"{"schema":"smartsteam.evolution_candidate.v1","candidate_id":"one"}"#,
            r#"{"schema":"smartsteam.evolution_candidate_status.v1","candidate_id":"two","status":"accepted"}"#,
            r#"{"schema":"smartsteam.evolution_candidate.v1","candidate_id":"three"}"#,
        ]
        .join("\n");

        assert_eq!(
            candidate_record_ids(&text),
            vec!["one".to_owned(), "three".to_owned()]
        );
        assert_eq!(
            current_candidate_status(&text, "two").as_deref(),
            Some("accepted")
        );
    }

    #[test]
    fn backlog_reader_replays_status_and_validation_events() {
        let dir =
            std::env::temp_dir().join(format!("smartsteam-backlog-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("evolution-candidates.jsonl");
        fs::write(
            &path,
            [
                r#"{"schema":"smartsteam.evolution_candidate.v1","candidate_id":"candidate-a","status":"new","source":"ledger","round":7,"case":"case-a","model":"model-a","tokens":"64","elapsed_ms":"100","feedback":"4","self_improve":"true","answer_preview":"candidate"}"#,
                r#"{"schema":"smartsteam.evolution_candidate_status.v1","candidate_id":"candidate-a","status":"implemented","note":"done","changed_unix":123}"#,
                r#"{"schema":"smartsteam.evolution_candidate_validation.v1","candidate_id":"candidate-a","command":"cargo test","status_code":0,"passed":true,"note":"green","validated_unix":456}"#,
            ]
            .join("\n"),
        )
        .unwrap();

        let (items, invalid_count) = read_candidate_backlog_items(&path).unwrap();

        assert_eq!(invalid_count, 0);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].status, "implemented");
        assert_eq!(items[0].round, "7");
        assert_eq!(items[0].validation_passed, "true");
        assert_eq!(items[0].validation_status_code, "0");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn append_candidate_backlog_skips_existing_candidate_ids() {
        let dir = std::env::temp_dir().join(format!(
            "smartsteam-backlog-append-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        let path = dir.join("evolution-candidates.jsonl");
        let first = sample_candidate("1", "same candidate");

        let initial = append_candidate_backlog(&path, std::slice::from_ref(&first)).unwrap();
        let duplicate = append_candidate_backlog(&path, &[first]).unwrap();

        assert_eq!(initial.appended, 1);
        assert_eq!(duplicate.existing, 1);
        assert_eq!(duplicate.appended, 0);
        assert_eq!(duplicate.skipped, 1);

        let _ = fs::remove_dir_all(&dir);
    }
}
