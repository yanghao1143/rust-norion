use std::path::Path;

use super::evolution_candidate_model::{
    BacklogAppendResult, CandidateLifecycleGate, EvolutionCandidateBacklogItem,
    EvolutionCandidateBatch, EvolutionCandidatePaths,
};
use super::evolution_candidate_updates::{CandidateMarkUpdate, CandidateValidationUpdate};

const PREVIEW_LIMIT: usize = 240;

pub(super) struct CandidateApplyCheckRender<'a> {
    pub(super) work_dir: &'a str,
    pub(super) backlog_path: &'a Path,
    pub(super) exists: bool,
    pub(super) total: usize,
    pub(super) invalid_count: usize,
    pub(super) candidate_selector: &'a str,
    pub(super) item: &'a EvolutionCandidateBacklogItem,
    pub(super) apply_ready: bool,
    pub(super) status_gate: &'a str,
    pub(super) block_reason: &'a str,
    pub(super) suggested_scope: &'a str,
    pub(super) suggested_validation: &'a str,
}

pub(super) fn render_candidate_gate_text(
    work_dir: &str,
    gate: &CandidateLifecycleGate,
    ready: bool,
) -> String {
    let mut lines = vec![
        "SmartSteam evolution candidate gate".to_owned(),
        "read_only=true starts_process=false sends_prompt=false writes_files=false".to_owned(),
        format!("work_dir={work_dir}"),
        format!(
            "backlog path={} exists={} total={} invalid={}",
            gate.path.display(),
            bool_text(gate.exists),
            gate.total,
            gate.invalid_count
        ),
        format!(
            "candidate_lifecycle ready={} accepted_pending={} implemented_validated={} implemented_unvalidated={} implemented_failed={}",
            bool_text(ready),
            gate.accepted_pending_ids.len(),
            gate.implemented_validated_count,
            gate.implemented_unvalidated_ids.len(),
            gate.implemented_failed_ids.len()
        ),
    ];
    if !gate.accepted_pending_ids.is_empty() {
        lines.push(format!(
            "accepted_pending_ids={}",
            gate.accepted_pending_ids.join(",")
        ));
    }
    if !gate.implemented_unvalidated_ids.is_empty() {
        lines.push(format!(
            "implemented_unvalidated_ids={}",
            gate.implemented_unvalidated_ids.join(",")
        ));
    }
    if !gate.implemented_failed_ids.is_empty() {
        lines.push(format!(
            "implemented_failed_ids={}",
            gate.implemented_failed_ids.join(",")
        ));
    }
    if gate.invalid_count > 0 {
        lines.push(format!("invalid_records={}", gate.invalid_count));
    }

    lines.join("\n")
}

pub(super) fn render_candidate_apply_check_text(view: CandidateApplyCheckRender<'_>) -> String {
    let mut lines = vec![
        "SmartSteam evolution candidate apply check".to_owned(),
        "read_only=true starts_process=false sends_prompt=false writes_files=false".to_owned(),
        format!("work_dir={}", view.work_dir),
        format!(
            "backlog path={} exists={} total={} invalid={}",
            view.backlog_path.display(),
            bool_text(view.exists),
            view.total,
            view.invalid_count
        ),
        format!("candidate_selector={}", view.candidate_selector),
        format!(
            "candidate_id={} status={} apply_ready={} status_gate={} block_reason={}",
            view.item.candidate_id,
            view.item.status,
            bool_text(view.apply_ready),
            view.status_gate,
            view.block_reason
        ),
        format!(
            "round={} case={} model={} source={}",
            view.item.round, view.item.case_name, view.item.model, view.item.source
        ),
        format!("suggested_scope={}", view.suggested_scope),
        format!("suggested_validation_command={}", view.suggested_validation),
        "suggested_next_status=implemented after validation evidence; rejected if scope risk is too high".to_owned(),
    ];
    if !view.item.note.is_empty() {
        lines.push(format!(
            "note={}",
            compact_line(&view.item.note, PREVIEW_LIMIT)
        ));
    }
    lines.push(format!(
        "answer_preview={}",
        compact_line(&view.item.answer_preview, PREVIEW_LIMIT)
    ));

    lines.join("\n")
}

pub(super) fn render_candidate_list_text(
    work_dir: &str,
    backlog_path: &Path,
    exists: bool,
    filter_label: &str,
    total: usize,
    matched: &[&EvolutionCandidateBacklogItem],
    invalid_count: usize,
    limit: usize,
) -> String {
    let mut lines = vec![
        "SmartSteam evolution candidate backlog".to_owned(),
        "read_only=true starts_process=false sends_prompt=false writes_files=false".to_owned(),
        format!("work_dir={work_dir}"),
        format!(
            "backlog path={} exists={}",
            backlog_path.display(),
            bool_text(exists)
        ),
        format!(
            "status_filter={filter_label} total={} matched={} invalid={} limit={} order=oldest_first",
            total,
            matched.len(),
            invalid_count,
            limit
        ),
    ];

    for item in matched {
        lines.push(format!(
            "- id={} status={} round={} case={}",
            item.candidate_id, item.status, item.round, item.case_name
        ));
        lines.push(format!(
            "  model={} source={} tokens={} elapsed_ms={} feedback={} self_improve={}",
            item.model, item.source, item.tokens, item.elapsed_ms, item.feedback, item.self_improve
        ));
        if !item.note.is_empty() {
            lines.push(format!(
                "  note={}",
                compact_line(&item.note, PREVIEW_LIMIT)
            ));
        }
        if item.changed_unix != "unknown" {
            lines.push(format!("  changed_unix={}", item.changed_unix));
        }
        if !item.validation_command.is_empty() {
            lines.push(format!(
                "  validation_passed={} validation_status_code={} validation_unix={}",
                item.validation_passed, item.validation_status_code, item.validation_unix
            ));
            lines.push(format!(
                "  validation_command={}",
                compact_line(&item.validation_command, PREVIEW_LIMIT)
            ));
            if !item.validation_note.is_empty() {
                lines.push(format!(
                    "  validation_note={}",
                    compact_line(&item.validation_note, PREVIEW_LIMIT)
                ));
            }
        }
        lines.push(format!(
            "  answer_preview={}",
            compact_line(&item.answer_preview, PREVIEW_LIMIT)
        ));
    }

    lines.join("\n")
}

pub(super) fn render_candidate_validation_text(
    work_dir: &str,
    update: &CandidateValidationUpdate,
) -> String {
    format!(
        "SmartSteam evolution candidate validation\n\
         read_only=false starts_process=false sends_prompt=false writes_files=true\n\
         work_dir={work_dir}\n\
         backlog path={}\n\
         candidate_id={}\n\
         validation_command={}\n\
         validation_status_code={}\n\
         validation_passed={}\n\
         appended=true",
        update.backlog_path.display(),
        update.candidate_id,
        compact_line(&update.command, PREVIEW_LIMIT),
        update.status_code,
        bool_text(update.passed)
    )
}

pub(super) fn render_candidate_mark_text(work_dir: &str, update: &CandidateMarkUpdate) -> String {
    format!(
        "SmartSteam evolution candidate mark\n\
         read_only=false starts_process=false sends_prompt=false writes_files=true\n\
         work_dir={work_dir}\n\
         backlog path={}\n\
         candidate_id={}\n\
         previous_status={}\n\
         status={}\n\
         appended=true",
        update.backlog_path.display(),
        update.candidate_id,
        update.previous_status,
        update.status
    )
}

pub(super) fn render_empty_candidates_text(
    work_dir: &str,
    limit: usize,
    paths: &EvolutionCandidatePaths,
    backlog: Option<&BacklogAppendResult>,
) -> String {
    let mut lines = vec![
        "SmartSteam evolution candidates".to_owned(),
        format!(
            "read_only=true starts_process=false sends_prompt=false writes_files={}",
            bool_text(backlog.is_some())
        ),
        format!("work_dir={work_dir}"),
        format!("source=none count=0 limit={limit}"),
        format!(
            "report={} exists={}",
            paths.report.display(),
            paths.report.is_file()
        ),
        format!(
            "ledger={} exists={}",
            paths.ledger.display(),
            paths.ledger.is_file()
        ),
    ];
    if let Some(backlog) = backlog {
        lines.push(backlog.summary_line());
    }
    lines.join("\n")
}

pub(super) fn render_candidate_batch_text(
    work_dir: &str,
    limit: usize,
    batch: &EvolutionCandidateBatch,
    backlog: Option<&BacklogAppendResult>,
) -> String {
    let mut lines = vec![
        "SmartSteam evolution candidates".to_owned(),
        format!(
            "read_only=true starts_process=false sends_prompt=false writes_files={}",
            bool_text(backlog.is_some())
        ),
        format!("work_dir={work_dir}"),
        format!(
            "source={source} path={} count={} limit={} order=newest_first",
            batch.source_path.display(),
            batch.candidates.len(),
            limit,
            source = batch.source
        ),
    ];

    for candidate in &batch.candidates {
        lines.push(format!(
            "- round={} case={} model={} tokens={} elapsed_ms={} feedback={} self_improve={}",
            candidate.round,
            candidate.case_name,
            candidate.model,
            candidate.tokens,
            candidate.elapsed_ms,
            candidate.feedback,
            candidate.self_improve
        ));
        lines.push(format!("  answer_preview={}", candidate.answer_preview));
    }

    if let Some(backlog) = backlog {
        lines.push(backlog.summary_line());
    }

    lines.join("\n")
}

fn bool_text(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

fn compact_line(value: &str, limit: usize) -> String {
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
    use std::path::PathBuf;

    fn backlog_item() -> EvolutionCandidateBacklogItem {
        EvolutionCandidateBacklogItem {
            candidate_id: "candidate-a".to_owned(),
            status: "accepted".to_owned(),
            source: "ledger".to_owned(),
            round: "7".to_owned(),
            case_name: "case-a".to_owned(),
            model: "model-a".to_owned(),
            tokens: "64".to_owned(),
            elapsed_ms: "100".to_owned(),
            feedback: "4".to_owned(),
            self_improve: "true".to_owned(),
            answer_preview: "candidate preview".to_owned(),
            note: "ready".to_owned(),
            changed_unix: "123".to_owned(),
            validation_command: String::new(),
            validation_status_code: "unknown".to_owned(),
            validation_passed: "unknown".to_owned(),
            validation_note: String::new(),
            validation_unix: "unknown".to_owned(),
        }
    }

    #[test]
    fn apply_check_render_preserves_gate_fields() {
        let item = backlog_item();
        let text = render_candidate_apply_check_text(CandidateApplyCheckRender {
            work_dir: "target/evolution",
            backlog_path: Path::new("target/evolution/evolution-candidates.jsonl"),
            exists: true,
            total: 1,
            invalid_count: 0,
            candidate_selector: "next",
            item: &item,
            apply_ready: true,
            status_gate: "pass",
            block_reason: "none",
            suggested_scope: "tools/smartsteam-forge",
            suggested_validation: "cargo test",
        });

        assert!(text.contains("candidate_selector=next"));
        assert!(
            text.contains("candidate_id=candidate-a status=accepted apply_ready=true status_gate=pass block_reason=none")
        );
        assert!(text.contains("note=ready"));
    }

    #[test]
    fn validation_and_mark_render_keep_append_only_contract_text() {
        let validation = CandidateValidationUpdate {
            backlog_path: PathBuf::from("candidate-backlog.jsonl"),
            candidate_id: "candidate-a".to_owned(),
            command: "cargo test".to_owned(),
            status_code: 0,
            passed: true,
        };
        let mark = CandidateMarkUpdate {
            backlog_path: PathBuf::from("candidate-backlog.jsonl"),
            candidate_id: "candidate-a".to_owned(),
            previous_status: "accepted".to_owned(),
            status: "implemented".to_owned(),
        };

        let validation_text = render_candidate_validation_text("work", &validation);
        let mark_text = render_candidate_mark_text("work", &mark);

        assert!(validation_text.contains("validation_passed=true"));
        assert!(validation_text.contains("appended=true"));
        assert!(mark_text.contains("previous_status=accepted"));
        assert!(mark_text.contains("status=implemented"));
    }
}
