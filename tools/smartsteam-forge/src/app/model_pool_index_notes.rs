use super::status_json::json_string_literal;
#[cfg(test)]
use super::status_json::{
    require_json_bool_equals, require_json_string_equals, required_json_string,
    validate_error_kind, validate_error_user_message,
};
use smartsteam_forge::session::first_disallowed_project_notes_control_char;

const MODEL_POOL_INDEX_NOTES_ERROR_JSON_SCHEMA: &str =
    "smartsteam.forge.model_pool_index_notes_error.v1";
const MODEL_POOL_INDEX_NOTES_CLEAR_ERROR_JSON_SCHEMA: &str =
    "smartsteam.forge.model_pool_index_notes_clear_error.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum ModelPoolIndexNotesActive {
    None,
    LatestDelimited,
    LatestLegacyUndelimited,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum ModelPoolIndexNotesContextActive {
    None,
    LatestTrustedDelimited,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum ModelPoolIndexNoteStatus {
    Delimited,
    LegacyUndelimited,
}

#[derive(Debug, PartialEq, Eq)]
pub(in crate::app) struct ModelPoolIndexNotesSummary {
    pub(in crate::app) block_count: usize,
    pub(in crate::app) delimited_blocks: usize,
    pub(in crate::app) legacy_undelimited_blocks: usize,
    pub(in crate::app) active: ModelPoolIndexNotesActive,
    pub(in crate::app) active_trusted: bool,
    pub(in crate::app) trusted_blocks: usize,
    pub(in crate::app) context_active: ModelPoolIndexNotesContextActive,
    pub(in crate::app) notes: Vec<ModelPoolIndexNoteSummary>,
}

#[derive(Debug, PartialEq, Eq)]
pub(in crate::app) struct ModelPoolIndexNoteSummary {
    pub(in crate::app) number: usize,
    pub(in crate::app) status: ModelPoolIndexNoteStatus,
    pub(in crate::app) chars: usize,
    pub(in crate::app) active: bool,
    pub(in crate::app) trusted: bool,
    pub(in crate::app) context_active: bool,
}

#[derive(Debug, PartialEq, Eq)]
#[cfg(test)]
pub(in crate::app) struct ModelPoolIndexNotesErrorJsonSummary {
    pub(in crate::app) error_kind: String,
    pub(in crate::app) error: String,
    pub(in crate::app) user_message: String,
}

#[derive(Debug, PartialEq, Eq)]
pub(in crate::app) struct ModelPoolIndexNotesClearSummary {
    pub(in crate::app) removed: usize,
    pub(in crate::app) legacy_undelimited: usize,
    pub(in crate::app) project_notes_chars: usize,
}

#[derive(Debug, PartialEq, Eq)]
#[cfg(test)]
pub(in crate::app) struct ModelPoolIndexNotesClearErrorJsonSummary {
    pub(in crate::app) read_only: bool,
    pub(in crate::app) writes_project_notes: bool,
    pub(in crate::app) error_kind: String,
    pub(in crate::app) error: String,
    pub(in crate::app) user_message: String,
}

pub(in crate::app) fn validate_model_pool_index_notes(report: &str) -> Result<(), String> {
    model_pool_index_notes_summary(report).map(|_| ())
}

pub(in crate::app) fn validate_model_pool_index_notes_clear(report: &str) -> Result<(), String> {
    model_pool_index_notes_clear_summary(report).map(|_| ())
}

pub(in crate::app) fn model_pool_index_notes_error_status(error_kind: &str, error: &str) -> String {
    let user_message = format!("model-pool index notes {error_kind} error: {error}");
    [
        format!("model_pool_index_notes_error kind={error_kind} error={error}"),
        "section=index_notes_error_json".to_owned(),
        model_pool_index_notes_error_json(error_kind, error, &user_message),
    ]
    .join("\n")
}

pub(in crate::app) fn model_pool_index_notes_clear_error_status(
    error_kind: &str,
    error: &str,
    writes_project_notes: bool,
) -> String {
    let user_message = format!("model-pool index notes clear {error_kind} error: {error}");
    [
        format!(
            "model_pool_index_notes_clear_error kind={error_kind} writes_project_notes={writes_project_notes} error={error}"
        ),
        "section=index_notes_clear_error_json".to_owned(),
        model_pool_index_notes_clear_error_json(
            error_kind,
            error,
            writes_project_notes,
            &user_message,
        ),
    ]
    .join("\n")
}

#[cfg(test)]
pub(in crate::app) fn model_pool_index_notes_error_json_summary(
    error_json: &str,
) -> Result<ModelPoolIndexNotesErrorJsonSummary, String> {
    require_json_string_equals(
        error_json,
        "schema",
        MODEL_POOL_INDEX_NOTES_ERROR_JSON_SCHEMA,
        "model-pool index notes error JSON schema",
    )?;
    require_json_bool_equals(
        error_json,
        "read_only",
        true,
        "model-pool index notes error JSON read_only",
    )?;
    require_json_bool_equals(
        error_json,
        "writes_project_notes",
        false,
        "model-pool index notes error JSON writes_project_notes",
    )?;
    require_json_bool_equals(
        error_json,
        "streams_model",
        false,
        "model-pool index notes error JSON streams_model",
    )?;
    let error_kind = required_json_string(
        error_json,
        "error_kind",
        "model-pool index notes error JSON error_kind",
    )?;
    validate_error_kind(&error_kind, "model-pool index notes error JSON")?;
    let error = required_json_string(
        error_json,
        "error",
        "model-pool index notes error JSON error",
    )?;
    let user_message = required_json_string(
        error_json,
        "user_message",
        "model-pool index notes error JSON user_message",
    )?;
    validate_error_user_message("model-pool index notes", &error_kind, &error, &user_message)?;

    Ok(ModelPoolIndexNotesErrorJsonSummary {
        error_kind,
        error,
        user_message,
    })
}

#[cfg(test)]
pub(in crate::app) fn model_pool_index_notes_clear_error_json_summary(
    error_json: &str,
) -> Result<ModelPoolIndexNotesClearErrorJsonSummary, String> {
    require_json_string_equals(
        error_json,
        "schema",
        MODEL_POOL_INDEX_NOTES_CLEAR_ERROR_JSON_SCHEMA,
        "model-pool index notes clear error JSON schema",
    )?;
    require_json_bool_equals(
        error_json,
        "streams_model",
        false,
        "model-pool index notes clear error JSON streams_model",
    )?;
    let read_only = match super::status_json::json_bool_field(error_json, "read_only") {
        Some(value) => value,
        None => return Err("model-pool index notes clear error JSON missing read_only".to_owned()),
    };
    let writes_project_notes =
        match super::status_json::json_bool_field(error_json, "writes_project_notes") {
            Some(value) => value,
            None => {
                return Err(
                    "model-pool index notes clear error JSON missing writes_project_notes"
                        .to_owned(),
                );
            }
        };
    let error_kind = required_json_string(
        error_json,
        "error_kind",
        "model-pool index notes clear error JSON error_kind",
    )?;
    validate_error_kind(&error_kind, "model-pool index notes clear error JSON")?;
    let error = required_json_string(
        error_json,
        "error",
        "model-pool index notes clear error JSON error",
    )?;
    let user_message = required_json_string(
        error_json,
        "user_message",
        "model-pool index notes clear error JSON user_message",
    )?;
    validate_error_user_message(
        "model-pool index notes clear",
        &error_kind,
        &error,
        &user_message,
    )?;

    Ok(ModelPoolIndexNotesClearErrorJsonSummary {
        read_only,
        writes_project_notes,
        error_kind,
        error,
        user_message,
    })
}

pub(in crate::app) fn model_pool_index_notes_summary(
    report: &str,
) -> Result<ModelPoolIndexNotesSummary, String> {
    reject_disallowed_control_chars(report)?;
    let lines = report.lines().collect::<Vec<_>>();
    let summary_line = lines
        .iter()
        .find(|line| line.starts_with("model_pool_index_notes="))
        .ok_or_else(|| "model-pool index notes missing summary line".to_owned())?;

    let block_count = required_usize_token(summary_line, "model_pool_index_notes=")?;
    let delimited_blocks = required_usize_token(summary_line, "delimited=")?;
    let legacy_undelimited_blocks = required_usize_token(summary_line, "legacy_undelimited=")?;
    let active = required_active(summary_line)?;
    let active_trusted = required_bool_token(summary_line, "active_trusted=")?;
    let trusted_blocks = required_usize_token(summary_line, "trusted=")?;
    let context_active = required_context_active(summary_line)?;
    if block_count != delimited_blocks + legacy_undelimited_blocks {
        return Err(format!(
            "model-pool index notes count mismatch: model_pool_index_notes={block_count} delimited={delimited_blocks} legacy_undelimited={legacy_undelimited_blocks}"
        ));
    }

    let notes = lines
        .iter()
        .filter(|line| line.starts_with("index_note_"))
        .map(|line| parse_index_note_line(line))
        .collect::<Result<Vec<_>, _>>()?;
    if notes.len() != block_count {
        return Err(format!(
            "model-pool index notes expected {block_count} index_note lines, got {}",
            notes.len()
        ));
    }
    for (index, note) in notes.iter().enumerate() {
        let expected_number = index + 1;
        if note.number != expected_number {
            return Err(format!(
                "model-pool index notes expected index_note_{expected_number}, got index_note_{}",
                note.number
            ));
        }
    }

    let parsed_delimited = notes
        .iter()
        .filter(|note| note.status == ModelPoolIndexNoteStatus::Delimited)
        .count();
    let parsed_legacy = notes
        .iter()
        .filter(|note| note.status == ModelPoolIndexNoteStatus::LegacyUndelimited)
        .count();
    if parsed_delimited != delimited_blocks {
        return Err(format!(
            "model-pool index notes delimited count mismatch: summary={delimited_blocks} parsed={parsed_delimited}"
        ));
    }
    if parsed_legacy != legacy_undelimited_blocks {
        return Err(format!(
            "model-pool index notes legacy_undelimited count mismatch: summary={legacy_undelimited_blocks} parsed={parsed_legacy}"
        ));
    }
    let parsed_trusted = notes.iter().filter(|note| note.trusted).count();
    if parsed_trusted != trusted_blocks {
        return Err(format!(
            "model-pool index notes trusted count mismatch: summary={trusted_blocks} parsed={parsed_trusted}"
        ));
    }

    validate_active_note(active, &notes)?;
    validate_active_trusted(active_trusted, &notes)?;
    validate_context_active_note(context_active, &notes)?;

    Ok(ModelPoolIndexNotesSummary {
        block_count,
        delimited_blocks,
        legacy_undelimited_blocks,
        active,
        active_trusted,
        trusted_blocks,
        context_active,
        notes,
    })
}

pub(in crate::app) fn model_pool_index_notes_clear_summary(
    report: &str,
) -> Result<ModelPoolIndexNotesClearSummary, String> {
    let lines = report
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    if lines.len() != 2 {
        return Err(format!(
            "model-pool index notes clear expected 2 summary lines, got {}",
            lines.len()
        ));
    }
    let clear_line = lines[0];
    if !clear_line.starts_with("model_pool_index_notes ") {
        return Err("model-pool index notes clear missing summary line".to_owned());
    }
    let project_notes_line = lines[1];
    if !project_notes_line.starts_with("project_notes=") {
        return Err("model-pool index notes clear missing project_notes line".to_owned());
    }

    let removed = required_usize_token(clear_line, "removed=")?;
    let legacy_undelimited = required_usize_token(clear_line, "legacy_undelimited=")?;
    if legacy_undelimited > removed {
        return Err(
            "model-pool index notes clear legacy_undelimited cannot exceed removed".to_owned(),
        );
    }
    let project_notes_chars = required_usize_token(project_notes_line, "project_notes=")?;
    if !project_notes_line
        .split_whitespace()
        .any(|token| token == "chars")
    {
        return Err("model-pool index notes clear project_notes line missing chars".to_owned());
    }

    Ok(ModelPoolIndexNotesClearSummary {
        removed,
        legacy_undelimited,
        project_notes_chars,
    })
}

fn reject_disallowed_control_chars(report: &str) -> Result<(), String> {
    if let Some((index, ch)) = first_disallowed_project_notes_control_char(report) {
        return Err(format!(
            "model-pool index notes contains disallowed control character U+{:04X} at byte {index}",
            ch as u32
        ));
    }
    Ok(())
}

fn model_pool_index_notes_error_json(error_kind: &str, error: &str, user_message: &str) -> String {
    format!(
        concat!(
            "{{",
            "\"schema\":{},",
            "\"read_only\":true,",
            "\"writes_project_notes\":false,",
            "\"streams_model\":false,",
            "\"error_kind\":{},",
            "\"error\":{},",
            "\"user_message\":{}",
            "}}"
        ),
        json_string_literal(MODEL_POOL_INDEX_NOTES_ERROR_JSON_SCHEMA),
        json_string_literal(error_kind),
        json_string_literal(error),
        json_string_literal(user_message),
    )
}

fn model_pool_index_notes_clear_error_json(
    error_kind: &str,
    error: &str,
    writes_project_notes: bool,
    user_message: &str,
) -> String {
    format!(
        concat!(
            "{{",
            "\"schema\":{},",
            "\"read_only\":false,",
            "\"writes_project_notes\":{},",
            "\"streams_model\":false,",
            "\"error_kind\":{},",
            "\"error\":{},",
            "\"user_message\":{}",
            "}}"
        ),
        json_string_literal(MODEL_POOL_INDEX_NOTES_CLEAR_ERROR_JSON_SCHEMA),
        if writes_project_notes {
            "true"
        } else {
            "false"
        },
        json_string_literal(error_kind),
        json_string_literal(error),
        json_string_literal(user_message),
    )
}

fn parse_index_note_line(line: &str) -> Result<ModelPoolIndexNoteSummary, String> {
    let number = line
        .split_whitespace()
        .next()
        .and_then(|token| token.strip_prefix("index_note_"))
        .ok_or_else(|| "model-pool index notes malformed index_note header".to_owned())?
        .parse::<usize>()
        .map_err(|_| "model-pool index notes index_note number must be usize".to_owned())?;
    let status = required_note_status(line)?;
    let chars = required_usize_token(line, "chars=")?;
    if chars == 0 {
        return Err("model-pool index notes index_note chars must be positive".to_owned());
    }
    let active = required_bool_token(line, "active=")?;
    let trusted = required_bool_token(line, "trusted=")?;
    let context_active = required_bool_token(line, "context_active=")?;
    if context_active && !trusted {
        return Err("model-pool index notes context_active note must be trusted".to_owned());
    }
    if trusted && status != ModelPoolIndexNoteStatus::Delimited {
        return Err("model-pool index notes trusted note must be delimited".to_owned());
    }

    Ok(ModelPoolIndexNoteSummary {
        number,
        status,
        chars,
        active,
        trusted,
        context_active,
    })
}

fn validate_active_note(
    active: ModelPoolIndexNotesActive,
    notes: &[ModelPoolIndexNoteSummary],
) -> Result<(), String> {
    let active_notes = notes.iter().filter(|note| note.active).collect::<Vec<_>>();
    match active {
        ModelPoolIndexNotesActive::None => {
            if !notes.is_empty() {
                return Err(
                    "model-pool index notes active=none requires zero index notes".to_owned(),
                );
            }
            if !active_notes.is_empty() {
                return Err("model-pool index notes active=none cannot have active note".to_owned());
            }
            Ok(())
        }
        ModelPoolIndexNotesActive::LatestDelimited => {
            require_single_active_note(&active_notes)?;
            let active_note = active_notes[0];
            if active_note.status != ModelPoolIndexNoteStatus::Delimited {
                return Err(
                    "model-pool index notes active latest_delimited requires a delimited active note"
                        .to_owned(),
                );
            }
            let last_delimited = notes
                .iter()
                .rev()
                .find(|note| note.status == ModelPoolIndexNoteStatus::Delimited)
                .ok_or_else(|| {
                    "model-pool index notes active latest_delimited requires delimited note"
                        .to_owned()
                })?;
            if active_note.number != last_delimited.number {
                return Err(
                    "model-pool index notes active note is not the latest delimited note"
                        .to_owned(),
                );
            }
            Ok(())
        }
        ModelPoolIndexNotesActive::LatestLegacyUndelimited => {
            require_single_active_note(&active_notes)?;
            let active_note = active_notes[0];
            if active_note.status != ModelPoolIndexNoteStatus::LegacyUndelimited {
                return Err(
                    "model-pool index notes active latest_legacy_undelimited requires a legacy_undelimited active note"
                        .to_owned(),
                );
            }
            if notes
                .iter()
                .any(|note| note.status == ModelPoolIndexNoteStatus::Delimited)
            {
                return Err(
                    "model-pool index notes latest_legacy_undelimited requires no delimited notes"
                        .to_owned(),
                );
            }
            if active_note.number != notes.len() {
                return Err(
                    "model-pool index notes active legacy note is not the latest index note"
                        .to_owned(),
                );
            }
            Ok(())
        }
    }
}

fn validate_context_active_note(
    context_active: ModelPoolIndexNotesContextActive,
    notes: &[ModelPoolIndexNoteSummary],
) -> Result<(), String> {
    let context_active_notes = notes
        .iter()
        .filter(|note| note.context_active)
        .collect::<Vec<_>>();
    match context_active {
        ModelPoolIndexNotesContextActive::None => {
            if notes.iter().any(|note| note.trusted) {
                return Err(
                    "model-pool index notes context_active=none requires zero trusted notes"
                        .to_owned(),
                );
            }
            if !context_active_notes.is_empty() {
                return Err(
                    "model-pool index notes context_active=none cannot have context active note"
                        .to_owned(),
                );
            }
            Ok(())
        }
        ModelPoolIndexNotesContextActive::LatestTrustedDelimited => {
            require_single_context_active_note(&context_active_notes)?;
            let context_note = context_active_notes[0];
            if !context_note.trusted || context_note.status != ModelPoolIndexNoteStatus::Delimited {
                return Err(
                    "model-pool index notes context_active latest_trusted_delimited requires a trusted delimited note"
                        .to_owned(),
                );
            }
            let latest_trusted = notes
                .iter()
                .rev()
                .find(|note| note.trusted)
                .ok_or_else(|| {
                    "model-pool index notes context_active latest_trusted_delimited requires trusted note"
                        .to_owned()
                })?;
            if context_note.number != latest_trusted.number {
                return Err(
                    "model-pool index notes context active note is not the latest trusted note"
                        .to_owned(),
                );
            }
            Ok(())
        }
    }
}

fn validate_active_trusted(
    active_trusted: bool,
    notes: &[ModelPoolIndexNoteSummary],
) -> Result<(), String> {
    let parsed_active_trusted = notes
        .iter()
        .find(|note| note.active)
        .map(|note| note.trusted)
        .unwrap_or(false);
    if active_trusted != parsed_active_trusted {
        return Err(format!(
            "model-pool index notes active_trusted mismatch: summary={active_trusted} parsed={parsed_active_trusted}"
        ));
    }
    Ok(())
}

fn require_single_active_note(active_notes: &[&ModelPoolIndexNoteSummary]) -> Result<(), String> {
    if active_notes.len() == 1 {
        return Ok(());
    }
    Err(format!(
        "model-pool index notes expected exactly one active note, got {}",
        active_notes.len()
    ))
}

fn require_single_context_active_note(
    active_notes: &[&ModelPoolIndexNoteSummary],
) -> Result<(), String> {
    if active_notes.len() == 1 {
        return Ok(());
    }
    Err(format!(
        "model-pool index notes expected exactly one context active note, got {}",
        active_notes.len()
    ))
}

fn required_active(line: &str) -> Result<ModelPoolIndexNotesActive, String> {
    match required_token(line, "active=")? {
        "none" => Ok(ModelPoolIndexNotesActive::None),
        "latest_delimited" => Ok(ModelPoolIndexNotesActive::LatestDelimited),
        "latest_legacy_undelimited" => Ok(ModelPoolIndexNotesActive::LatestLegacyUndelimited),
        value => Err(format!(
            "model-pool index notes unknown active value {value:?}"
        )),
    }
}

fn required_context_active(line: &str) -> Result<ModelPoolIndexNotesContextActive, String> {
    match required_token(line, "context_active=")? {
        "none" => Ok(ModelPoolIndexNotesContextActive::None),
        "latest_trusted_delimited" => Ok(ModelPoolIndexNotesContextActive::LatestTrustedDelimited),
        value => Err(format!(
            "model-pool index notes unknown context_active value {value:?}"
        )),
    }
}

fn required_note_status(line: &str) -> Result<ModelPoolIndexNoteStatus, String> {
    match required_token(line, "status=")? {
        "delimited" => Ok(ModelPoolIndexNoteStatus::Delimited),
        "legacy_undelimited" => Ok(ModelPoolIndexNoteStatus::LegacyUndelimited),
        value => Err(format!(
            "model-pool index notes unknown note status {value:?}"
        )),
    }
}

fn required_usize_token(line: &str, prefix: &str) -> Result<usize, String> {
    required_token(line, prefix)?
        .parse::<usize>()
        .map_err(|_| format!("model-pool index notes expected usize for {prefix}"))
}

fn required_bool_token(line: &str, prefix: &str) -> Result<bool, String> {
    match required_token(line, prefix)? {
        "true" => Ok(true),
        "false" => Ok(false),
        value => Err(format!(
            "model-pool index notes expected bool for {prefix}, got {value:?}"
        )),
    }
}

fn required_token<'a>(line: &'a str, prefix: &str) -> Result<&'a str, String> {
    line.split_whitespace()
        .find_map(|token| token.strip_prefix(prefix))
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("model-pool index notes missing {prefix}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_notes_summary_accepts_project_notes_prefix_and_active_delimited_note() {
        let report = concat!(
            "project_notes=120 chars path=state/project_notes.md\n",
            "model_pool_index_notes=2 delimited=1 legacy_undelimited=1 active=latest_delimited active_trusted=false trusted=0 context_active=none\n",
            "index_note_1 status=delimited chars=88 active=true trusted=false context_active=false\n",
            "model_pool_index:\n",
            "answer:\n",
            "src/model_service\n",
            "model_pool_index_end:\n",
            "index_note_2 status=legacy_undelimited chars=45 active=false trusted=false context_active=false\n",
            "model_pool_index:\n",
            "legacy stale"
        );

        let summary = model_pool_index_notes_summary(report).unwrap();

        assert_eq!(summary.block_count, 2);
        assert_eq!(summary.delimited_blocks, 1);
        assert_eq!(summary.legacy_undelimited_blocks, 1);
        assert_eq!(summary.active, ModelPoolIndexNotesActive::LatestDelimited);
        assert!(!summary.active_trusted);
        assert_eq!(summary.trusted_blocks, 0);
        assert_eq!(
            summary.context_active,
            ModelPoolIndexNotesContextActive::None
        );
        assert_eq!(summary.notes.len(), 2);
        assert_eq!(summary.notes[0].status, ModelPoolIndexNoteStatus::Delimited);
        assert!(summary.notes[0].active);
        assert!(!summary.notes[0].trusted);
        assert!(!summary.notes[0].context_active);
    }

    #[test]
    fn index_notes_summary_accepts_empty_report() {
        let report = "model_pool_index_notes=0 delimited=0 legacy_undelimited=0 active=none active_trusted=false trusted=0 context_active=none";

        let summary = model_pool_index_notes_summary(report).unwrap();

        assert_eq!(summary.block_count, 0);
        assert!(summary.notes.is_empty());
        assert_eq!(summary.active, ModelPoolIndexNotesActive::None);
        assert!(!summary.active_trusted);
        assert_eq!(summary.trusted_blocks, 0);
        assert_eq!(
            summary.context_active,
            ModelPoolIndexNotesContextActive::None
        );
    }

    #[test]
    fn index_notes_summary_accepts_latest_trusted_context_active_note() {
        let report = concat!(
            "model_pool_index_notes=2 delimited=2 legacy_undelimited=0 active=latest_delimited active_trusted=true trusted=1 context_active=latest_trusted_delimited\n",
            "index_note_1 status=delimited chars=88 active=false trusted=false context_active=false\n",
            "index_note_2 status=delimited chars=120 active=true trusted=true context_active=true\n"
        );

        let summary = model_pool_index_notes_summary(report).unwrap();

        assert_eq!(summary.trusted_blocks, 1);
        assert!(summary.active_trusted);
        assert_eq!(
            summary.context_active,
            ModelPoolIndexNotesContextActive::LatestTrustedDelimited
        );
        assert!(summary.notes[1].trusted);
        assert!(summary.notes[1].context_active);
    }

    #[test]
    fn index_notes_summary_accepts_untrusted_visible_active_with_prior_trusted_context() {
        let report = concat!(
            "model_pool_index_notes=2 delimited=2 legacy_undelimited=0 active=latest_delimited active_trusted=false trusted=1 context_active=latest_trusted_delimited\n",
            "index_note_1 status=delimited chars=88 active=false trusted=true context_active=true\n",
            "index_note_2 status=delimited chars=120 active=true trusted=false context_active=false\n"
        );

        let summary = model_pool_index_notes_summary(report).unwrap();

        assert_eq!(summary.active, ModelPoolIndexNotesActive::LatestDelimited);
        assert!(!summary.active_trusted);
        assert_eq!(summary.trusted_blocks, 1);
        assert_eq!(
            summary.context_active,
            ModelPoolIndexNotesContextActive::LatestTrustedDelimited
        );
        assert!(summary.notes[0].context_active);
        assert!(summary.notes[1].active);
        assert!(!summary.notes[1].trusted);
    }

    #[test]
    fn index_notes_clear_summary_accepts_removed_and_project_notes_lines() {
        let report =
            "model_pool_index_notes removed=2 legacy_undelimited=1\nproject_notes=12 chars";

        let summary = model_pool_index_notes_clear_summary(report).unwrap();

        assert_eq!(
            summary,
            ModelPoolIndexNotesClearSummary {
                removed: 2,
                legacy_undelimited: 1,
                project_notes_chars: 12,
            }
        );
    }

    #[test]
    fn index_notes_clear_summary_rejects_dirty_extra_lines() {
        let report = concat!(
            "model_pool_index_notes removed=2 legacy_undelimited=1\n",
            "project_notes=12 chars\n",
            "model_pool_index:\n",
            "stale dirty index tail"
        );

        assert!(
            model_pool_index_notes_clear_summary(report)
                .unwrap_err()
                .contains("expected 2 summary lines")
        );
    }

    #[test]
    fn index_notes_summary_rejects_summary_count_drift() {
        let report = "model_pool_index_notes=2 delimited=2 legacy_undelimited=1 active=latest_delimited active_trusted=false trusted=0 context_active=none";

        assert!(
            model_pool_index_notes_summary(report)
                .unwrap_err()
                .contains("count mismatch")
        );
    }

    #[test]
    fn index_notes_summary_rejects_missing_active_note() {
        let report = concat!(
            "model_pool_index_notes=1 delimited=1 legacy_undelimited=0 active=latest_delimited active_trusted=false trusted=0 context_active=none\n",
            "index_note_1 status=delimited chars=88 active=false trusted=false context_active=false"
        );

        assert!(
            model_pool_index_notes_summary(report)
                .unwrap_err()
                .contains("exactly one active note")
        );
    }

    #[test]
    fn index_notes_summary_rejects_control_chars_in_visible_report() {
        let report = concat!(
            "model_pool_index_notes=1 delimited=1 legacy_undelimited=0 active=latest_delimited active_trusted=false trusted=0 context_active=none\n",
            "index_note_1 status=delimited chars=88 active=true trusted=false context_active=false\n",
            "model_pool_index:\n",
            "answer:\n",
            "src\x1b[2J/model_service\n",
            "model_pool_index_end:"
        );

        assert!(
            model_pool_index_notes_summary(report)
                .unwrap_err()
                .contains("disallowed control character U+001B")
        );
    }

    #[test]
    fn index_notes_summary_rejects_legacy_active_when_delimited_exists() {
        let report = concat!(
            "model_pool_index_notes=2 delimited=1 legacy_undelimited=1 active=latest_legacy_undelimited active_trusted=false trusted=0 context_active=none\n",
            "index_note_1 status=delimited chars=88 active=false trusted=false context_active=false\n",
            "index_note_2 status=legacy_undelimited chars=45 active=true trusted=false context_active=false"
        );

        assert!(
            model_pool_index_notes_summary(report)
                .unwrap_err()
                .contains("requires no delimited notes")
        );
    }

    #[test]
    fn index_notes_summary_rejects_trusted_count_drift() {
        let report = concat!(
            "model_pool_index_notes=1 delimited=1 legacy_undelimited=0 active=latest_delimited active_trusted=true trusted=2 context_active=latest_trusted_delimited\n",
            "index_note_1 status=delimited chars=88 active=true trusted=true context_active=true"
        );

        assert!(
            model_pool_index_notes_summary(report)
                .unwrap_err()
                .contains("trusted count mismatch")
        );
    }

    #[test]
    fn index_notes_summary_rejects_active_trusted_drift() {
        let report = concat!(
            "model_pool_index_notes=1 delimited=1 legacy_undelimited=0 active=latest_delimited active_trusted=true trusted=0 context_active=none\n",
            "index_note_1 status=delimited chars=88 active=true trusted=false context_active=false"
        );

        assert!(
            model_pool_index_notes_summary(report)
                .unwrap_err()
                .contains("active_trusted mismatch")
        );
    }

    #[test]
    fn index_notes_summary_rejects_context_active_on_untrusted_note() {
        let report = concat!(
            "model_pool_index_notes=1 delimited=1 legacy_undelimited=0 active=latest_delimited active_trusted=false trusted=0 context_active=latest_trusted_delimited\n",
            "index_note_1 status=delimited chars=88 active=true trusted=false context_active=true"
        );

        assert!(
            model_pool_index_notes_summary(report)
                .unwrap_err()
                .contains("context_active note must be trusted")
        );
    }

    #[test]
    fn index_notes_summary_rejects_context_active_that_is_not_latest_trusted_note() {
        let report = concat!(
            "model_pool_index_notes=2 delimited=2 legacy_undelimited=0 active=latest_delimited active_trusted=true trusted=2 context_active=latest_trusted_delimited\n",
            "index_note_1 status=delimited chars=88 active=false trusted=true context_active=true\n",
            "index_note_2 status=delimited chars=120 active=true trusted=true context_active=false"
        );

        assert!(
            model_pool_index_notes_summary(report)
                .unwrap_err()
                .contains("context active note is not the latest trusted note")
        );
    }

    #[test]
    fn index_notes_error_status_carries_machine_readable_context() {
        let status = model_pool_index_notes_error_status("contract", "missing summary line");
        let error_json = status
            .lines()
            .skip_while(|line| *line != "section=index_notes_error_json")
            .nth(1)
            .expect("index_notes_error_json section should include body");

        assert_eq!(
            model_pool_index_notes_error_json_summary(error_json).unwrap(),
            ModelPoolIndexNotesErrorJsonSummary {
                error_kind: "contract".to_owned(),
                error: "missing summary line".to_owned(),
                user_message: "model-pool index notes contract error: missing summary line"
                    .to_owned(),
            }
        );
    }

    #[test]
    fn index_notes_error_json_rejects_unknown_error_kind() {
        let status = model_pool_index_notes_error_status("contract", "missing summary line");
        let error_json = status
            .lines()
            .skip_while(|line| *line != "section=index_notes_error_json")
            .nth(1)
            .unwrap();
        let drifted = error_json.replace("\"error_kind\":\"contract\"", "\"error_kind\":\"route\"");

        assert!(
            model_pool_index_notes_error_json_summary(&drifted)
                .unwrap_err()
                .contains("unknown error_kind")
        );
    }

    #[test]
    fn index_notes_error_json_rejects_user_message_drift() {
        let status = model_pool_index_notes_error_status("provider", "project notes unavailable");
        let error_json = status
            .lines()
            .skip_while(|line| *line != "section=index_notes_error_json")
            .nth(1)
            .unwrap();
        let drifted = error_json.replace(
            "\"error\":\"project notes unavailable\"",
            "\"error\":\"project notes ready\"",
        );

        assert!(
            model_pool_index_notes_error_json_summary(&drifted)
                .unwrap_err()
                .contains("user_message drift")
        );
    }

    #[test]
    fn index_notes_clear_error_status_carries_machine_readable_context() {
        let status = model_pool_index_notes_clear_error_status(
            "contract",
            "unexpected dirty clear summary",
            true,
        );
        let error_json = status
            .lines()
            .skip_while(|line| *line != "section=index_notes_clear_error_json")
            .nth(1)
            .expect("index_notes_clear_error_json section should include body");

        assert_eq!(
            model_pool_index_notes_clear_error_json_summary(error_json).unwrap(),
            ModelPoolIndexNotesClearErrorJsonSummary {
                read_only: false,
                writes_project_notes: true,
                error_kind: "contract".to_owned(),
                error: "unexpected dirty clear summary".to_owned(),
                user_message:
                    "model-pool index notes clear contract error: unexpected dirty clear summary"
                        .to_owned(),
            }
        );
    }

    #[test]
    fn index_notes_clear_error_json_rejects_unknown_error_kind() {
        let status =
            model_pool_index_notes_clear_error_status("contract", "unexpected dirty clear", true);
        let error_json = status
            .lines()
            .skip_while(|line| *line != "section=index_notes_clear_error_json")
            .nth(1)
            .unwrap();
        let drifted = error_json.replace("\"error_kind\":\"contract\"", "\"error_kind\":\"route\"");

        assert!(
            model_pool_index_notes_clear_error_json_summary(&drifted)
                .unwrap_err()
                .contains("unknown error_kind")
        );
    }

    #[test]
    fn index_notes_clear_error_json_rejects_user_message_drift() {
        let status =
            model_pool_index_notes_clear_error_status("provider", "project notes locked", false);
        let error_json = status
            .lines()
            .skip_while(|line| *line != "section=index_notes_clear_error_json")
            .nth(1)
            .unwrap();
        let drifted = error_json.replace(
            "\"error\":\"project notes locked\"",
            "\"error\":\"project notes unlocked\"",
        );

        assert!(
            model_pool_index_notes_clear_error_json_summary(&drifted)
                .unwrap_err()
                .contains("user_message drift")
        );
    }
}
