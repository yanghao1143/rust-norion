use std::{fs, io, path::Path};

use super::evolution_candidate_model::{
    EvolutionCandidate, EvolutionCandidateBatch, EvolutionCandidatePaths,
};
use super::status_json::{
    bool_value_text, json_bool_field, json_number_field, json_string_field,
    json_top_level_object_field,
};

const PREVIEW_LIMIT: usize = 240;

pub(super) fn load_candidate_batch(
    paths: &EvolutionCandidatePaths,
    limit: usize,
) -> io::Result<Option<EvolutionCandidateBatch>> {
    if let Some(mut candidate) = read_report_last_candidate(&paths.report)? {
        candidate.source = "report.last".to_owned();
        return Ok(Some(EvolutionCandidateBatch {
            source: "report.last".to_owned(),
            source_path: paths.report.clone(),
            candidates: vec![candidate],
        }));
    }

    let mut candidates = read_ledger_candidates(&paths.ledger, limit)?;
    if !candidates.is_empty() {
        for candidate in &mut candidates {
            candidate.source = "ledger".to_owned();
        }
        return Ok(Some(EvolutionCandidateBatch {
            source: "ledger".to_owned(),
            source_path: paths.ledger.clone(),
            candidates,
        }));
    }

    Ok(None)
}

fn read_report_last_candidate(path: &Path) -> io::Result<Option<EvolutionCandidate>> {
    if !path.is_file() {
        return Ok(None);
    }
    let report = fs::read_to_string(path)?;
    Ok(json_top_level_object_field(&report, "last").and_then(candidate_from_json))
}

fn read_ledger_candidates(path: &Path, limit: usize) -> io::Result<Vec<EvolutionCandidate>> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let ledger = fs::read_to_string(path)?;
    Ok(ledger
        .lines()
        .rev()
        .filter_map(candidate_from_json)
        .take(limit)
        .collect())
}

fn candidate_from_json(record: &str) -> Option<EvolutionCandidate> {
    let answer = json_string_field(record, "answer")
        .or_else(|| json_string_field(record, "final_preview"))
        .unwrap_or_default();
    if answer.trim().is_empty() {
        return None;
    }

    Some(EvolutionCandidate {
        source: "unknown".to_owned(),
        round: number_value(record, "round"),
        case_name: string_value(record, "case"),
        model: json_string_field(record, "runtime_model")
            .or_else(|| json_string_field(record, "model"))
            .unwrap_or_else(|| "unknown".to_owned()),
        tokens: json_number_field(record, "runtime_tokens")
            .or_else(|| json_number_field(record, "tokens"))
            .unwrap_or_else(|| "unknown".to_owned()),
        elapsed_ms: json_number_field(record, "elapsed_ms")
            .or_else(|| json_number_field(record, "elapsed"))
            .unwrap_or_else(|| "unknown".to_owned()),
        feedback: json_number_field(record, "feedback_applied")
            .or_else(|| json_number_field(record, "feedback"))
            .unwrap_or_else(|| "unknown".to_owned()),
        self_improve: json_bool_field(record, "self_improve_passed")
            .or_else(|| json_bool_field(record, "self_improve"))
            .map(bool_value_text)
            .unwrap_or("unknown")
            .to_owned(),
        answer_preview: compact_source_preview(&answer, PREVIEW_LIMIT),
    })
}

fn number_value(object: &str, field: &str) -> String {
    json_number_field(object, field).unwrap_or_else(|| "unknown".to_owned())
}

fn string_value(object: &str, field: &str) -> String {
    json_string_field(object, field).unwrap_or_else(|| "unknown".to_owned())
}

fn compact_source_preview(value: &str, limit: usize) -> String {
    let compact = value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace('\0', "");
    if compact.chars().count() <= limit {
        return compact;
    }
    let mut truncated = compact
        .chars()
        .take(limit.saturating_sub(3))
        .collect::<String>();
    truncated.push_str("...");
    truncated
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::evolution_candidate_model::{LEDGER_FILE, REPORT_FILE};
    use std::{
        env,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn prefers_report_last_candidate_over_ledger() {
        let work_dir = temp_work_dir("report-over-ledger");
        fs::create_dir_all(&work_dir).unwrap();
        fs::write(
            work_dir.join(REPORT_FILE),
            r#"{
                "last": {
                    "round": 9,
                    "case": "report",
                    "runtime_model": "google/gemma-4-12B-it",
                    "runtime_tokens": 42,
                    "elapsed_ms": 12345,
                    "feedback_applied": 4,
                    "self_improve_passed": true,
                    "answer": "**Improvement Candidate:** report wins"
                }
            }"#,
        )
        .unwrap();
        fs::write(
            work_dir.join(LEDGER_FILE),
            r#"{"round":8,"case":"ledger","runtime_model":"ledger-model","runtime_tokens":1,"elapsed_ms":2,"feedback_applied":3,"self_improve_passed":false,"answer":"ledger fallback"}"#,
        )
        .unwrap();

        let paths = EvolutionCandidatePaths::new(&work_dir.to_string_lossy());
        let batch = load_candidate_batch(&paths, 5).unwrap().unwrap();
        let _ = fs::remove_dir_all(&work_dir);

        assert_eq!(batch.source, "report.last");
        assert_eq!(batch.candidates.len(), 1);
        assert_eq!(batch.candidates[0].round, "9");
        assert_eq!(
            batch.candidates[0].answer_preview,
            "**Improvement Candidate:** report wins"
        );
    }

    #[test]
    fn reads_recent_ledger_candidates_newest_first_when_report_missing() {
        let work_dir = temp_work_dir("ledger-recent");
        fs::create_dir_all(&work_dir).unwrap();
        fs::write(
            work_dir.join(LEDGER_FILE),
            [
                r#"{"round":1,"case":"case-1","runtime_model":"model-a","runtime_tokens":11,"elapsed_ms":101,"feedback_applied":1,"self_improve_passed":true,"answer":"candidate one"}"#,
                r#"{"round":2,"case":"case-2","runtime_model":"model-b","runtime_tokens":22,"elapsed_ms":202,"feedback_applied":2,"self_improve_passed":false,"answer":"candidate two"}"#,
                r#"{"round":3,"case":"case-3","runtime_model":"model-c","runtime_tokens":33,"elapsed_ms":303,"feedback_applied":3,"self_improve_passed":true,"answer":"candidate three"}"#,
            ]
            .join("\n"),
        )
        .unwrap();

        let paths = EvolutionCandidatePaths::new(&work_dir.to_string_lossy());
        let batch = load_candidate_batch(&paths, 2).unwrap().unwrap();
        let _ = fs::remove_dir_all(&work_dir);

        assert_eq!(batch.source, "ledger");
        assert_eq!(batch.candidates.len(), 2);
        assert_eq!(batch.candidates[0].round, "3");
        assert_eq!(batch.candidates[1].round, "2");
        assert_eq!(batch.candidates[0].self_improve, "true");
        assert_eq!(batch.candidates[1].self_improve, "false");
    }

    #[test]
    fn decodes_json_string_escapes_for_answer_preview() {
        let candidate = candidate_from_json(
            r#"{"round":4,"case":"case-4","runtime_model":"model-d","runtime_tokens":44,"elapsed_ms":404,"feedback_applied":4,"self_improve_passed":true,"answer":"line one\nline two with \"quote\""}"#,
        )
        .unwrap();

        assert_eq!(candidate.answer_preview, "line one line two with \"quote\"");
    }

    fn temp_work_dir(name: &str) -> std::path::PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!(
            "smartsteam-forge-evolution-candidate-sources-{name}-{}-{now}",
            std::process::id()
        ))
    }
}
