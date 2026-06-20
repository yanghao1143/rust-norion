use std::{io, path::Path};

use super::evolution_candidate_backlog::read_candidate_backlog_items;
use super::evolution_candidate_model::{CandidateLifecycleGate, EvolutionCandidateBacklogItem};

pub(super) fn select_apply_check_candidate(
    items: &[EvolutionCandidateBacklogItem],
    candidate_selector: &str,
    backlog_path: &Path,
) -> io::Result<EvolutionCandidateBacklogItem> {
    if candidate_selector.eq_ignore_ascii_case("next")
        || candidate_selector.eq_ignore_ascii_case("accepted")
    {
        return items
            .iter()
            .find(|item| item.status == "accepted")
            .cloned()
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!(
                        "no accepted candidate found in backlog {}",
                        backlog_path.display()
                    ),
                )
            });
    }

    items
        .iter()
        .find(|item| item.candidate_id == candidate_selector)
        .cloned()
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "candidate {candidate_selector} not found in backlog {}",
                    backlog_path.display()
                ),
            )
        })
}

pub(super) fn candidate_lifecycle_gate(path: &Path) -> io::Result<CandidateLifecycleGate> {
    let exists = path.is_file();
    let (items, invalid_count) = read_candidate_backlog_items(path)?;
    let mut gate = CandidateLifecycleGate {
        path: path.to_path_buf(),
        exists,
        total: items.len(),
        invalid_count,
        implemented_validated_count: 0,
        accepted_pending_ids: Vec::new(),
        implemented_unvalidated_ids: Vec::new(),
        implemented_failed_ids: Vec::new(),
    };

    for item in &items {
        match item.status.as_str() {
            "accepted" => gate.accepted_pending_ids.push(item.candidate_id.clone()),
            "implemented" => match item.validation_passed.as_str() {
                "true" => gate.implemented_validated_count += 1,
                "false" => gate.implemented_failed_ids.push(item.candidate_id.clone()),
                _ => gate
                    .implemented_unvalidated_ids
                    .push(item.candidate_id.clone()),
            },
            _ => {}
        }
    }

    Ok(gate)
}

pub(super) fn suggested_candidate_scope(answer_preview: &str) -> &'static str {
    let lower = answer_preview.to_ascii_lowercase();
    if lower.contains("context rot") || lower.contains("ledger gate") {
        "crates/norion-eval,tools/evolution-loop"
    } else if lower.contains("memory_pressure")
        || lower.contains("memory pressure")
        || lower.contains("test-gate")
        || lower.contains("model-pool")
        || lower.contains("pool")
    {
        "tools/evolution-loop,tools/smartsteam-forge"
    } else if lower.contains("candidate") || lower.contains("backlog") {
        "tools/smartsteam-forge"
    } else {
        "inspect_candidate_preview_before_editing"
    }
}

pub(super) fn suggested_candidate_validation_command(answer_preview: &str) -> &'static str {
    let lower = answer_preview.to_ascii_lowercase();
    if lower.contains("context rot") || lower.contains("ledger gate") {
        "cargo test -q --manifest-path crates/norion-eval/Cargo.toml && cargo test -q --manifest-path tools/evolution-loop/Cargo.toml"
    } else if lower.contains("memory_pressure")
        || lower.contains("memory pressure")
        || lower.contains("test-gate")
        || lower.contains("model-pool")
        || lower.contains("pool")
    {
        "cargo test -q --manifest-path tools/evolution-loop/Cargo.toml && cargo test -q --manifest-path tools/smartsteam-forge/Cargo.toml"
    } else if lower.contains("candidate") || lower.contains("backlog") {
        "cargo test -q --manifest-path tools/smartsteam-forge/Cargo.toml"
    } else {
        "cargo test -q --manifest-path tools/evolution-loop/Cargo.toml && cargo test -q --manifest-path tools/smartsteam-forge/Cargo.toml"
    }
}

pub(super) fn normalize_candidate_list_status_filter(
    status: Option<&str>,
) -> io::Result<Option<String>> {
    let Some(status) = status.map(str::trim).filter(|status| !status.is_empty()) else {
        return Ok(None);
    };
    if status.eq_ignore_ascii_case("all") {
        return Ok(None);
    }
    normalize_candidate_status(status).map(Some)
}

pub(super) fn normalize_candidate_status(status: &str) -> io::Result<String> {
    let normalized = status.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "new" | "accepted" | "implemented" | "rejected" => Ok(normalized),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "unsupported candidate status '{status}'; expected new, accepted, implemented, or rejected"
            ),
        )),
    }
}

pub(super) fn parse_validation_status_code(status_code: &str) -> io::Result<i32> {
    status_code.trim().parse::<i32>().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("validation status code must be an integer: {status_code}"),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn backlog_item(candidate_id: &str, status: &str) -> EvolutionCandidateBacklogItem {
        EvolutionCandidateBacklogItem {
            candidate_id: candidate_id.to_owned(),
            status: status.to_owned(),
            source: "ledger".to_owned(),
            round: "1".to_owned(),
            case_name: "case".to_owned(),
            model: "model".to_owned(),
            tokens: "1".to_owned(),
            elapsed_ms: "1".to_owned(),
            feedback: "1".to_owned(),
            self_improve: "true".to_owned(),
            answer_preview: "candidate backlog".to_owned(),
            note: String::new(),
            changed_unix: "unknown".to_owned(),
            validation_command: String::new(),
            validation_status_code: "unknown".to_owned(),
            validation_passed: "unknown".to_owned(),
            validation_note: String::new(),
            validation_unix: "unknown".to_owned(),
        }
    }

    #[test]
    fn select_apply_check_prefers_first_accepted_candidate() {
        let items = vec![
            backlog_item("candidate-new", "new"),
            backlog_item("candidate-accepted", "accepted"),
        ];
        let selected =
            select_apply_check_candidate(&items, "next", Path::new("backlog.jsonl")).unwrap();

        assert_eq!(selected.candidate_id, "candidate-accepted");
        assert_eq!(
            select_apply_check_candidate(&items, "candidate-new", Path::new("backlog.jsonl"))
                .unwrap()
                .candidate_id,
            "candidate-new"
        );
    }

    #[test]
    fn lifecycle_gate_classifies_pending_validation_and_failures() {
        let dir = std::env::temp_dir().join(format!(
            "smartsteam-lifecycle-gate-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let backlog = dir.join("evolution-candidates.jsonl");
        fs::write(
            &backlog,
            [
                r#"{"schema":"smartsteam.evolution_candidate.v1","candidate_id":"accepted","status":"new","source":"report.last","round":"1","case":"case-1","model":"model-a","tokens":"64","elapsed_ms":"100","feedback":"4","self_improve":"true","answer_preview":"accepted"}"#,
                r#"{"schema":"smartsteam.evolution_candidate_status.v1","candidate_id":"accepted","status":"accepted","note":"todo","changed_unix":111}"#,
                r#"{"schema":"smartsteam.evolution_candidate.v1","candidate_id":"implemented-ok","status":"implemented","source":"report.last","round":"2","case":"case-2","model":"model-b","tokens":"64","elapsed_ms":"100","feedback":"4","self_improve":"true","answer_preview":"ok"}"#,
                r#"{"schema":"smartsteam.evolution_candidate_validation.v1","candidate_id":"implemented-ok","command":"cargo test","status_code":0,"passed":true,"note":"green","validated_unix":222}"#,
                r#"{"schema":"smartsteam.evolution_candidate.v1","candidate_id":"implemented-failed","status":"implemented","source":"report.last","round":"3","case":"case-3","model":"model-c","tokens":"64","elapsed_ms":"100","feedback":"4","self_improve":"true","answer_preview":"failed"}"#,
                r#"{"schema":"smartsteam.evolution_candidate_validation.v1","candidate_id":"implemented-failed","command":"cargo test","status_code":1,"passed":false,"note":"red","validated_unix":333}"#,
            ]
            .join("\n"),
        )
        .unwrap();

        let gate = candidate_lifecycle_gate(&backlog).unwrap();
        let _ = fs::remove_dir_all(&dir);

        assert_eq!(gate.total, 3);
        assert_eq!(gate.implemented_validated_count, 1);
        assert_eq!(gate.accepted_pending_ids, vec!["accepted".to_owned()]);
        assert_eq!(
            gate.implemented_failed_ids,
            vec!["implemented-failed".to_owned()]
        );
        assert!(!gate.ready());
    }

    #[test]
    fn status_and_validation_code_normalization_are_strict() {
        assert_eq!(
            normalize_candidate_list_status_filter(Some(" All "))
                .unwrap()
                .as_deref(),
            None
        );
        assert_eq!(
            normalize_candidate_status(" IMPLEMENTED ").unwrap(),
            "implemented"
        );
        assert_eq!(parse_validation_status_code(" 0 ").unwrap(), 0);
        assert_eq!(
            normalize_candidate_status("maybe").unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
        assert_eq!(
            parse_validation_status_code("ok").unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
    }

    #[test]
    fn candidate_preview_suggestions_route_known_work() {
        assert_eq!(
            suggested_candidate_scope("Context Rot ledger gate cleanup"),
            "crates/norion-eval,tools/evolution-loop"
        );
        assert_eq!(
            suggested_candidate_scope("model-pool memory pressure guard"),
            "tools/evolution-loop,tools/smartsteam-forge"
        );
        assert_eq!(
            suggested_candidate_validation_command("candidate backlog cleanup"),
            "cargo test -q --manifest-path tools/smartsteam-forge/Cargo.toml"
        );
    }
}
