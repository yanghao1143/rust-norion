use super::status_json::json_string_literal;
#[cfg(test)]
use super::status_json::{
    require_json_bool_equals, require_json_string_equals, required_json_string,
    validate_error_kind, validate_error_user_message,
};
use smartsteam_forge::session::first_disallowed_project_notes_control_char;

const CONTEXT_PREVIEW_ERROR_JSON_SCHEMA: &str = "smartsteam.forge.context_preview_error.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum ContextPreviewIndexActive {
    None,
    LatestDelimited,
    LatestLegacyUndelimited,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum ContextPreviewIndexContextActive {
    None,
    LatestTrustedDelimited,
}

#[derive(Debug, PartialEq, Eq)]
pub(in crate::app) struct ContextPreviewSummary {
    pub(in crate::app) short_history_messages: usize,
    pub(in crate::app) index_context: ContextPreviewIndexContext,
    pub(in crate::app) budget_index_active: ContextPreviewIndexActive,
    pub(in crate::app) budget_index_active_trusted: bool,
    pub(in crate::app) budget_index_notes: Option<usize>,
    pub(in crate::app) budget_index_trusted: usize,
    pub(in crate::app) budget_index_context_active: ContextPreviewIndexContextActive,
    pub(in crate::app) budget_index_chars: Option<usize>,
    pub(in crate::app) budget_index_legacy_undelimited: Option<usize>,
}

#[derive(Debug, PartialEq, Eq)]
pub(in crate::app) struct ContextPreviewIndexContext {
    pub(in crate::app) block_count: usize,
    pub(in crate::app) delimited_blocks: usize,
    pub(in crate::app) legacy_undelimited_blocks: usize,
    pub(in crate::app) active: ContextPreviewIndexActive,
    pub(in crate::app) active_trusted: bool,
    pub(in crate::app) trusted_blocks: usize,
    pub(in crate::app) context_active: ContextPreviewIndexContextActive,
    pub(in crate::app) chars: usize,
}

#[derive(Debug, PartialEq, Eq)]
#[cfg(test)]
pub(in crate::app) struct ContextPreviewErrorJsonSummary {
    pub(in crate::app) error_kind: String,
    pub(in crate::app) error: String,
    pub(in crate::app) user_message: String,
}

pub(in crate::app) fn validate_context_preview(report: &str) -> Result<(), String> {
    context_preview_summary(report).map(|_| ())
}

pub(in crate::app) fn context_preview_error_status(error_kind: &str, error: &str) -> String {
    let user_message = format!("context preview {error_kind} error: {error}");
    [
        format!("context_preview_error kind={error_kind} error={error}"),
        "section=context_preview_error_json".to_owned(),
        context_preview_error_json(error_kind, error, &user_message),
    ]
    .join("\n")
}

#[cfg(test)]
pub(in crate::app) fn context_preview_error_json_summary(
    error_json: &str,
) -> Result<ContextPreviewErrorJsonSummary, String> {
    require_json_string_equals(
        error_json,
        "schema",
        CONTEXT_PREVIEW_ERROR_JSON_SCHEMA,
        "context preview error JSON schema",
    )?;
    require_json_bool_equals(
        error_json,
        "read_only",
        true,
        "context preview error JSON read_only",
    )?;
    require_json_bool_equals(
        error_json,
        "writes_session_state",
        false,
        "context preview error JSON writes_session_state",
    )?;
    require_json_bool_equals(
        error_json,
        "streams_model",
        false,
        "context preview error JSON streams_model",
    )?;
    let error_kind = required_json_string(
        error_json,
        "error_kind",
        "context preview error JSON error_kind",
    )?;
    validate_error_kind(&error_kind, "context preview error JSON")?;
    let error = required_json_string(error_json, "error", "context preview error JSON error")?;
    let user_message = required_json_string(
        error_json,
        "user_message",
        "context preview error JSON user_message",
    )?;
    validate_error_user_message("context preview", &error_kind, &error, &user_message)?;

    Ok(ContextPreviewErrorJsonSummary {
        error_kind,
        error,
        user_message,
    })
}

pub(in crate::app) fn context_preview_summary(
    report: &str,
) -> Result<ContextPreviewSummary, String> {
    reject_disallowed_control_chars(report)?;
    let lines = report.lines().collect::<Vec<_>>();
    require_line(&lines, 0, "Context preview")?;
    let budget_line = required_prefixed_line(&lines, "context_budget:")?;
    let index_line = required_prefixed_line(&lines, "model_pool_index_context:")?;
    let short_history_messages = required_line_value(&lines, "short_history_messages=")?
        .parse::<usize>()
        .map_err(|_| "context preview short_history_messages must be usize".to_owned())?;

    let index_context = parse_index_context_line(index_line)?;
    let budget_index_active = required_active_token(budget_line, "model_pool_index_active=")?;
    let budget_index_active_trusted =
        required_bool_token(budget_line, "model_pool_index_active_trusted=")?;
    let budget_index_notes = optional_usize_token(budget_line, "model_pool_index_notes=")?;
    let budget_index_trusted = required_usize_token(budget_line, "model_pool_index_trusted=")?;
    let budget_index_context_active =
        required_context_active_token(budget_line, "model_pool_index_context_active=")?;
    let budget_index_chars = optional_usize_token(budget_line, "model_pool_index_chars=")?;
    let budget_index_legacy_undelimited =
        optional_usize_token(budget_line, "model_pool_index_legacy_undelimited=")?;

    validate_index_context(&index_context)?;
    validate_budget_alignment(
        &index_context,
        budget_index_active,
        budget_index_active_trusted,
        budget_index_notes,
        budget_index_trusted,
        budget_index_context_active,
        budget_index_chars,
        budget_index_legacy_undelimited,
    )?;

    Ok(ContextPreviewSummary {
        short_history_messages,
        index_context,
        budget_index_active,
        budget_index_active_trusted,
        budget_index_notes,
        budget_index_trusted,
        budget_index_context_active,
        budget_index_chars,
        budget_index_legacy_undelimited,
    })
}

fn reject_disallowed_control_chars(report: &str) -> Result<(), String> {
    if let Some((index, ch)) = first_disallowed_project_notes_control_char(report) {
        return Err(format!(
            "context preview contains disallowed control character U+{:04X} at byte {index}",
            ch as u32
        ));
    }
    Ok(())
}

fn context_preview_error_json(error_kind: &str, error: &str, user_message: &str) -> String {
    format!(
        concat!(
            "{{",
            "\"schema\":{},",
            "\"read_only\":true,",
            "\"writes_session_state\":false,",
            "\"streams_model\":false,",
            "\"error_kind\":{},",
            "\"error\":{},",
            "\"user_message\":{}",
            "}}"
        ),
        json_string_literal(CONTEXT_PREVIEW_ERROR_JSON_SCHEMA),
        json_string_literal(error_kind),
        json_string_literal(error),
        json_string_literal(user_message),
    )
}

fn parse_index_context_line(line: &str) -> Result<ContextPreviewIndexContext, String> {
    let tail = line
        .strip_prefix("model_pool_index_context:")
        .ok_or_else(|| "context preview malformed model_pool_index_context line".to_owned())?
        .trim();
    if tail == "none" {
        return Ok(ContextPreviewIndexContext {
            block_count: 0,
            delimited_blocks: 0,
            legacy_undelimited_blocks: 0,
            active: ContextPreviewIndexActive::None,
            active_trusted: false,
            trusted_blocks: 0,
            context_active: ContextPreviewIndexContextActive::None,
            chars: 0,
        });
    }

    Ok(ContextPreviewIndexContext {
        block_count: required_usize_token(tail, "blocks=")?,
        delimited_blocks: required_usize_token(tail, "delimited=")?,
        legacy_undelimited_blocks: required_usize_token(tail, "legacy_undelimited=")?,
        active: required_active_token(tail, "active=")?,
        active_trusted: required_bool_token(tail, "active_trusted=")?,
        trusted_blocks: required_usize_token(tail, "trusted=")?,
        context_active: required_context_active_token(tail, "context_active=")?,
        chars: required_usize_token(tail, "chars=")?,
    })
}

fn validate_index_context(index_context: &ContextPreviewIndexContext) -> Result<(), String> {
    if index_context.block_count
        != index_context.delimited_blocks + index_context.legacy_undelimited_blocks
    {
        return Err(format!(
            "context preview index count mismatch: blocks={} delimited={} legacy_undelimited={}",
            index_context.block_count,
            index_context.delimited_blocks,
            index_context.legacy_undelimited_blocks
        ));
    }
    if index_context.trusted_blocks > index_context.delimited_blocks {
        return Err("context preview trusted blocks cannot exceed delimited blocks".to_owned());
    }

    match index_context.active {
        ContextPreviewIndexActive::None => {
            if index_context.block_count != 0 {
                return Err(
                    "context preview active=none requires zero index context blocks".to_owned(),
                );
            }
            if index_context.chars != 0 {
                return Err("context preview index chars must be zero when active=none".to_owned());
            }
            if index_context.trusted_blocks != 0 {
                return Err(
                    "context preview trusted blocks must be zero when active=none".to_owned(),
                );
            }
            if index_context.active_trusted {
                return Err(
                    "context preview active_trusted must be false when active=none".to_owned(),
                );
            }
            if index_context.context_active != ContextPreviewIndexContextActive::None {
                return Err(
                    "context preview context_active must be none when active=none".to_owned(),
                );
            }
        }
        ContextPreviewIndexActive::LatestDelimited => {
            if index_context.delimited_blocks == 0 {
                return Err(
                    "context preview latest_delimited requires delimited index context".to_owned(),
                );
            }
            if index_context.trusted_blocks == 0 {
                return Err(
                    "context preview latest_delimited requires trusted index context".to_owned(),
                );
            }
            if index_context.context_active
                != ContextPreviewIndexContextActive::LatestTrustedDelimited
            {
                return Err(
                    "context preview latest_delimited requires latest trusted context active"
                        .to_owned(),
                );
            }
            if index_context.chars == 0 {
                return Err(
                    "context preview latest_delimited requires positive index chars".to_owned(),
                );
            }
        }
        ContextPreviewIndexActive::LatestLegacyUndelimited => {
            if index_context.active_trusted {
                return Err(
                    "context preview legacy active index cannot be trusted context".to_owned(),
                );
            }
            return Err(
                "context preview latest_legacy_undelimited is not trusted context eligible"
                    .to_owned(),
            );
        }
    }
    Ok(())
}

fn validate_budget_alignment(
    index_context: &ContextPreviewIndexContext,
    budget_index_active: ContextPreviewIndexActive,
    budget_index_active_trusted: bool,
    budget_index_notes: Option<usize>,
    budget_index_trusted: usize,
    budget_index_context_active: ContextPreviewIndexContextActive,
    budget_index_chars: Option<usize>,
    budget_index_legacy_undelimited: Option<usize>,
) -> Result<(), String> {
    if index_context.active != budget_index_active {
        return Err(format!(
            "context preview index active mismatch: context={} budget={}",
            index_context.active.label(),
            budget_index_active.label()
        ));
    }
    if let Some(budget_index_notes) = budget_index_notes
        && budget_index_notes != index_context.block_count
    {
        return Err(format!(
            "context preview index notes mismatch: context={} budget={budget_index_notes}",
            index_context.block_count
        ));
    }
    if budget_index_active_trusted != index_context.active_trusted {
        return Err(format!(
            "context preview index active_trusted mismatch: context={} budget={budget_index_active_trusted}",
            index_context.active_trusted
        ));
    }
    if budget_index_trusted != index_context.trusted_blocks {
        return Err(format!(
            "context preview index trusted mismatch: context={} budget={budget_index_trusted}",
            index_context.trusted_blocks
        ));
    }
    if budget_index_context_active != index_context.context_active {
        return Err(format!(
            "context preview index context_active mismatch: context={} budget={}",
            index_context.context_active.label(),
            budget_index_context_active.label()
        ));
    }
    if let Some(budget_index_chars) = budget_index_chars
        && budget_index_chars != index_context.chars
    {
        return Err(format!(
            "context preview index chars mismatch: context={} budget={budget_index_chars}",
            index_context.chars
        ));
    }
    if let Some(budget_index_legacy_undelimited) = budget_index_legacy_undelimited
        && budget_index_legacy_undelimited != index_context.legacy_undelimited_blocks
    {
        return Err(format!(
            "context preview index legacy_undelimited mismatch: context={} budget={budget_index_legacy_undelimited}",
            index_context.legacy_undelimited_blocks
        ));
    }
    Ok(())
}

impl ContextPreviewIndexContextActive {
    fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::LatestTrustedDelimited => "latest_trusted_delimited",
        }
    }
}

impl ContextPreviewIndexActive {
    fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::LatestDelimited => "latest_delimited",
            Self::LatestLegacyUndelimited => "latest_legacy_undelimited",
        }
    }
}

fn require_line(lines: &[&str], index: usize, expected: &str) -> Result<(), String> {
    match lines.get(index) {
        Some(line) if *line == expected => Ok(()),
        Some(line) => Err(format!(
            "context preview line {index} expected {expected:?}, got {line:?}"
        )),
        None => Err(format!(
            "context preview missing line {index} expected {expected:?}"
        )),
    }
}

fn required_prefixed_line<'a>(lines: &'a [&str], prefix: &str) -> Result<&'a str, String> {
    lines
        .iter()
        .find(|line| line.starts_with(prefix))
        .copied()
        .ok_or_else(|| format!("context preview missing {prefix}"))
}

fn required_line_value<'a>(lines: &'a [&str], prefix: &str) -> Result<&'a str, String> {
    lines
        .iter()
        .find_map(|line| line.strip_prefix(prefix))
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("context preview missing {prefix}"))
}

fn required_usize_token(line: &str, prefix: &str) -> Result<usize, String> {
    required_token(line, prefix)?
        .parse::<usize>()
        .map_err(|_| format!("context preview expected usize for {prefix}"))
}

fn required_bool_token(line: &str, prefix: &str) -> Result<bool, String> {
    match required_token(line, prefix)? {
        "true" => Ok(true),
        "false" => Ok(false),
        value => Err(format!(
            "context preview expected bool for {prefix}, got {value:?}"
        )),
    }
}

fn optional_usize_token(line: &str, prefix: &str) -> Result<Option<usize>, String> {
    optional_token(line, prefix)
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|_| format!("context preview expected usize for {prefix}"))
        })
        .transpose()
}

fn required_active_token(line: &str, prefix: &str) -> Result<ContextPreviewIndexActive, String> {
    match required_token(line, prefix)? {
        "none" => Ok(ContextPreviewIndexActive::None),
        "latest_delimited" => Ok(ContextPreviewIndexActive::LatestDelimited),
        "latest_legacy_undelimited" => Ok(ContextPreviewIndexActive::LatestLegacyUndelimited),
        value => Err(format!("context preview unknown active value {value:?}")),
    }
}

fn required_context_active_token(
    line: &str,
    prefix: &str,
) -> Result<ContextPreviewIndexContextActive, String> {
    match required_token(line, prefix)? {
        "none" => Ok(ContextPreviewIndexContextActive::None),
        "latest_trusted_delimited" => Ok(ContextPreviewIndexContextActive::LatestTrustedDelimited),
        value => Err(format!(
            "context preview unknown context_active value {value:?}"
        )),
    }
}

fn required_token<'a>(line: &'a str, prefix: &str) -> Result<&'a str, String> {
    optional_token(line, prefix).ok_or_else(|| format!("context preview missing {prefix}"))
}

fn optional_token<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
    line.split_whitespace()
        .find_map(|token| token.strip_prefix(prefix))
        .filter(|value| !value.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_preview_summary_accepts_complete_index_context() {
        let report = concat!(
            "Context preview\n",
            "mode=chat output=raw\n",
            "context_budget: mode=chat messages_sent=3 model_pool_index_notes=1 model_pool_index_active=latest_delimited model_pool_index_active_trusted=true model_pool_index_trusted=1 model_pool_index_context_active=latest_trusted_delimited model_pool_index_chars=88 model_pool_index_legacy_undelimited=0\n",
            "short_history_messages=1\n",
            "1. user [sent_next_request]: hello\n",
            "model_pool_index_context: blocks=1 delimited=1 legacy_undelimited=0 active=latest_delimited active_trusted=true trusted=1 context_active=latest_trusted_delimited chars=88\n",
            "project_notes_preview: manual note"
        );

        let summary = context_preview_summary(report).unwrap();

        assert_eq!(summary.short_history_messages, 1);
        assert_eq!(summary.index_context.block_count, 1);
        assert_eq!(summary.index_context.delimited_blocks, 1);
        assert_eq!(summary.index_context.legacy_undelimited_blocks, 0);
        assert_eq!(
            summary.index_context.active,
            ContextPreviewIndexActive::LatestDelimited
        );
        assert!(summary.index_context.active_trusted);
        assert_eq!(
            summary.budget_index_active,
            ContextPreviewIndexActive::LatestDelimited
        );
        assert!(summary.budget_index_active_trusted);
        assert_eq!(summary.budget_index_notes, Some(1));
        assert_eq!(summary.index_context.trusted_blocks, 1);
        assert_eq!(
            summary.index_context.context_active,
            ContextPreviewIndexContextActive::LatestTrustedDelimited
        );
        assert_eq!(summary.budget_index_trusted, 1);
        assert_eq!(
            summary.budget_index_context_active,
            ContextPreviewIndexContextActive::LatestTrustedDelimited
        );
        assert_eq!(summary.budget_index_chars, Some(88));
        assert_eq!(summary.budget_index_legacy_undelimited, Some(0));
    }

    #[test]
    fn context_preview_summary_accepts_untrusted_visible_active_with_prior_trusted_context() {
        let report = concat!(
            "Context preview\n",
            "context_budget: model_pool_index_active=latest_delimited model_pool_index_active_trusted=false model_pool_index_notes=2 model_pool_index_trusted=1 model_pool_index_context_active=latest_trusted_delimited model_pool_index_chars=120 model_pool_index_legacy_undelimited=0\n",
            "short_history_messages=1\n",
            "model_pool_index_context: blocks=2 delimited=2 legacy_undelimited=0 active=latest_delimited active_trusted=false trusted=1 context_active=latest_trusted_delimited chars=120"
        );

        let summary = context_preview_summary(report).unwrap();

        assert_eq!(summary.index_context.block_count, 2);
        assert_eq!(
            summary.index_context.active,
            ContextPreviewIndexActive::LatestDelimited
        );
        assert!(!summary.index_context.active_trusted);
        assert_eq!(summary.index_context.trusted_blocks, 1);
        assert_eq!(
            summary.index_context.context_active,
            ContextPreviewIndexContextActive::LatestTrustedDelimited
        );
        assert!(!summary.budget_index_active_trusted);
    }

    #[test]
    fn context_preview_summary_accepts_none_index_context() {
        let report = concat!(
            "Context preview\n",
            "context_budget: model_pool_index_active=none model_pool_index_active_trusted=false model_pool_index_notes=0 model_pool_index_trusted=0 model_pool_index_context_active=none model_pool_index_chars=0 model_pool_index_legacy_undelimited=0\n",
            "short_history_messages=0\n",
            "short_history=empty\n",
            "model_pool_index_context: none"
        );

        let summary = context_preview_summary(report).unwrap();

        assert_eq!(summary.index_context.block_count, 0);
        assert_eq!(
            summary.index_context.active,
            ContextPreviewIndexActive::None
        );
        assert_eq!(summary.index_context.trusted_blocks, 0);
        assert_eq!(
            summary.index_context.context_active,
            ContextPreviewIndexContextActive::None
        );
    }

    #[test]
    fn context_preview_summary_rejects_index_active_drift() {
        let report = concat!(
            "Context preview\n",
            "context_budget: model_pool_index_active=none model_pool_index_active_trusted=false model_pool_index_trusted=1 model_pool_index_context_active=latest_trusted_delimited\n",
            "short_history_messages=1\n",
            "model_pool_index_context: blocks=1 delimited=1 legacy_undelimited=0 active=latest_delimited active_trusted=true trusted=1 context_active=latest_trusted_delimited chars=88"
        );

        assert!(
            context_preview_summary(report)
                .unwrap_err()
                .contains("index active mismatch")
        );
    }

    #[test]
    fn context_preview_summary_rejects_control_chars_in_visible_report() {
        let report = concat!(
            "Context preview\n",
            "context_budget: model_pool_index_active=latest_delimited model_pool_index_active_trusted=true model_pool_index_notes=1 model_pool_index_trusted=1 model_pool_index_context_active=latest_trusted_delimited model_pool_index_chars=88 model_pool_index_legacy_undelimited=0\n",
            "short_history_messages=1\n",
            "model_pool_index_context: blocks=1 delimited=1 legacy_undelimited=0 active=latest_delimited active_trusted=true trusted=1 context_active=latest_trusted_delimited chars=88\n",
            "DIRTY\x1b[2J_INDEX_CONTEXT_SHOULD_NOT_RENDER"
        );

        assert!(
            context_preview_summary(report)
                .unwrap_err()
                .contains("disallowed control character U+001B")
        );
    }

    #[test]
    fn context_preview_summary_rejects_legacy_active_when_delimited_exists() {
        let report = concat!(
            "Context preview\n",
            "context_budget: model_pool_index_active=latest_legacy_undelimited model_pool_index_active_trusted=false model_pool_index_trusted=0 model_pool_index_context_active=none\n",
            "short_history_messages=1\n",
            "model_pool_index_context: blocks=2 delimited=1 legacy_undelimited=1 active=latest_legacy_undelimited active_trusted=false trusted=0 context_active=none chars=88"
        );

        assert!(
            context_preview_summary(report)
                .unwrap_err()
                .contains("not trusted context eligible")
        );
    }

    #[test]
    fn context_preview_summary_rejects_index_trusted_budget_drift() {
        let report = concat!(
            "Context preview\n",
            "context_budget: model_pool_index_active=latest_delimited model_pool_index_active_trusted=true model_pool_index_trusted=0 model_pool_index_context_active=latest_trusted_delimited\n",
            "short_history_messages=1\n",
            "model_pool_index_context: blocks=1 delimited=1 legacy_undelimited=0 active=latest_delimited active_trusted=true trusted=1 context_active=latest_trusted_delimited chars=88"
        );

        assert!(
            context_preview_summary(report)
                .unwrap_err()
                .contains("index trusted mismatch")
        );
    }

    #[test]
    fn context_preview_summary_rejects_untrusted_delimited_context() {
        let report = concat!(
            "Context preview\n",
            "context_budget: model_pool_index_active=latest_delimited model_pool_index_active_trusted=false model_pool_index_trusted=0 model_pool_index_context_active=none\n",
            "short_history_messages=1\n",
            "model_pool_index_context: blocks=1 delimited=1 legacy_undelimited=0 active=latest_delimited active_trusted=false trusted=0 context_active=none chars=88"
        );

        assert!(
            context_preview_summary(report)
                .unwrap_err()
                .contains("requires trusted index context")
        );
    }

    #[test]
    fn context_preview_error_status_carries_machine_readable_context() {
        let status = context_preview_error_status("contract", "index active mismatch");
        let error_json = status
            .lines()
            .skip_while(|line| *line != "section=context_preview_error_json")
            .nth(1)
            .expect("context_preview_error_json section should include body");

        assert_eq!(
            context_preview_error_json_summary(error_json).unwrap(),
            ContextPreviewErrorJsonSummary {
                error_kind: "contract".to_owned(),
                error: "index active mismatch".to_owned(),
                user_message: "context preview contract error: index active mismatch".to_owned(),
            }
        );
    }

    #[test]
    fn context_preview_error_json_rejects_unknown_error_kind() {
        let status = context_preview_error_status("contract", "index active mismatch");
        let error_json = status
            .lines()
            .skip_while(|line| *line != "section=context_preview_error_json")
            .nth(1)
            .unwrap();
        let drifted = error_json.replace("\"error_kind\":\"contract\"", "\"error_kind\":\"route\"");

        assert!(
            context_preview_error_json_summary(&drifted)
                .unwrap_err()
                .contains("unknown error_kind")
        );
    }

    #[test]
    fn context_preview_error_json_rejects_user_message_drift() {
        let status = context_preview_error_status("provider", "context service down");
        let error_json = status
            .lines()
            .skip_while(|line| *line != "section=context_preview_error_json")
            .nth(1)
            .unwrap();
        let drifted = error_json.replace(
            "\"error\":\"context service down\"",
            "\"error\":\"context service ready\"",
        );

        assert!(
            context_preview_error_json_summary(&drifted)
                .unwrap_err()
                .contains("user_message drift")
        );
    }
}
