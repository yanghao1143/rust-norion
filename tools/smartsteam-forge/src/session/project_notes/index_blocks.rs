pub const MODEL_POOL_INDEX_NOTE_MARKER: &str = "model_pool_index:";
pub const MODEL_POOL_INDEX_NOTE_END_MARKER: &str = "model_pool_index_end:";

const ESCAPED_MODEL_POOL_INDEX_MARKER_PREFIX: &str = "[escaped model_pool_index marker] ";
const MAX_INDEX_NOTE_PREVIEW_CHARS: usize = 1200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelPoolIndexClearResult {
    pub content: String,
    pub removed_blocks: usize,
    pub legacy_undelimited_blocks: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ModelPoolIndexNoteActive {
    #[default]
    None,
    LatestDelimited,
    LatestLegacyUndelimited,
}

impl ModelPoolIndexNoteActive {
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::LatestDelimited => "latest_delimited",
            Self::LatestLegacyUndelimited => "latest_legacy_undelimited",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ModelPoolIndexNoteContextActive {
    #[default]
    None,
    LatestTrustedDelimited,
}

impl ModelPoolIndexNoteContextActive {
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::LatestTrustedDelimited => "latest_trusted_delimited",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ModelPoolIndexNoteStats {
    pub block_count: usize,
    pub delimited_blocks: usize,
    pub legacy_undelimited_blocks: usize,
    pub active: ModelPoolIndexNoteActive,
    pub active_trusted: bool,
    pub trusted_blocks: usize,
    pub context_active: ModelPoolIndexNoteContextActive,
    pub total_chars: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedModelPoolIndexNoteSummary {
    pub source_prompt: String,
    pub selected_role: String,
    pub selected_base_url: String,
    pub answer_chars: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IndexBlockSpan {
    start: usize,
    end: usize,
    delimited: bool,
}

pub fn render_model_pool_index_notes(content: &str) -> String {
    let spans = collect_index_block_spans(content);
    let stats = model_pool_index_note_stats_for_spans(content, &spans);
    let active_index = active_index_block_index(&spans);
    let trusted_blocks = spans
        .iter()
        .map(|span| span.delimited && trusted_span_text(content, span))
        .collect::<Vec<_>>();
    let context_active_index = trusted_blocks.iter().rposition(|trusted| *trusted);
    let active_trusted = active_index
        .and_then(|index| trusted_blocks.get(index))
        .copied()
        .unwrap_or(false);
    let mut lines = vec![format!(
        "model_pool_index_notes={} delimited={} legacy_undelimited={} active={} active_trusted={} trusted={} context_active={}",
        stats.block_count,
        stats.delimited_blocks,
        stats.legacy_undelimited_blocks,
        stats.active.label(),
        active_trusted,
        stats.trusted_blocks,
        stats.context_active.label()
    )];

    for (index, span) in spans.iter().enumerate() {
        let text = sanitized_span_text(content, span);
        lines.push(format!(
            "index_note_{} status={} chars={} active={} trusted={} context_active={}",
            index + 1,
            if span.delimited {
                "delimited"
            } else {
                "legacy_undelimited"
            },
            text.chars().count(),
            active_index == Some(index),
            trusted_blocks.get(index).copied().unwrap_or(false),
            context_active_index == Some(index)
        ));
        lines.push(trim_chars(&text, MAX_INDEX_NOTE_PREVIEW_CHARS));
    }

    lines.join("\n")
}

pub fn model_pool_index_note_stats(content: &str) -> ModelPoolIndexNoteStats {
    let spans = collect_index_block_spans(content);
    model_pool_index_note_stats_for_spans(content, &spans)
}

pub fn latest_model_pool_index_block(content: &str) -> Option<&str> {
    let spans = collect_index_block_spans(content);
    let span = active_index_block_index(&spans).and_then(|index| spans.get(index))?;
    content
        .get(span.start..span.end)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub fn latest_trusted_model_pool_index_block(content: &str) -> Option<&str> {
    let spans = collect_index_block_spans(content);
    let span = spans
        .iter()
        .rev()
        .find(|span| span.delimited && trusted_span_text(content, span))?;
    content
        .get(span.start..span.end)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub fn latest_safe_model_pool_index_block(content: &str) -> Option<String> {
    latest_trusted_model_pool_index_block(content)
        .map(sanitize_project_notes_control_chars)
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

pub fn validate_trusted_model_pool_index_note_block(note: &str) -> Result<(), String> {
    trusted_model_pool_index_note_summary(note).map(|_| ())
}

pub fn trusted_model_pool_index_note_summary(
    note: &str,
) -> Result<TrustedModelPoolIndexNoteSummary, String> {
    if let Some((index, ch)) = first_disallowed_project_notes_control_char(note) {
        return Err(format!(
            "model pool index pin contains disallowed control character U+{:04X} at byte {index}",
            ch as u32
        ));
    }

    let lines = note.lines().collect::<Vec<_>>();
    require_line(&lines, 0, MODEL_POOL_INDEX_NOTE_MARKER)?;
    require_line(
        &lines,
        lines.len().saturating_sub(1),
        MODEL_POOL_INDEX_NOTE_END_MARKER,
    )?;

    let start_markers = marker_line_count(&lines, MODEL_POOL_INDEX_NOTE_MARKER);
    let end_markers = marker_line_count(&lines, MODEL_POOL_INDEX_NOTE_END_MARKER);
    if start_markers != 1 || end_markers != 1 {
        return Err(format!(
            "model pool index pin expected one delimited block, got start_markers={start_markers} end_markers={end_markers}"
        ));
    }

    let source_prompt = required_prefixed_line_value(&lines, "source_prompt: ")?;
    let selected_role = required_prefixed_line_value(&lines, "selected_role: ")?;
    if selected_role != "index" {
        return Err(format!(
            "model pool index pin selected_role must be index, got {selected_role:?}"
        ));
    }
    let selected_base_url = required_prefixed_line_value(&lines, "selected_base_url: ")?;

    let answer_marker = lines
        .iter()
        .position(|line| *line == "answer:")
        .ok_or_else(|| "model pool index pin missing answer marker".to_owned())?;
    let answer_end = lines
        .iter()
        .rposition(|line| *line == MODEL_POOL_INDEX_NOTE_END_MARKER)
        .ok_or_else(|| "model pool index pin missing end marker".to_owned())?;
    if answer_marker >= answer_end {
        return Err("model pool index pin answer marker must precede end marker".to_owned());
    }
    let answer = lines[answer_marker + 1..answer_end].join("\n");
    let answer_chars = answer.trim().chars().count();
    if answer_chars == 0 {
        return Err("model pool index pin answer must be non-empty".to_owned());
    }

    Ok(TrustedModelPoolIndexNoteSummary {
        source_prompt: source_prompt.to_owned(),
        selected_role: selected_role.to_owned(),
        selected_base_url: selected_base_url.to_owned(),
        answer_chars,
    })
}

pub fn sanitize_project_notes_control_chars(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                out.push('\n');
            }
            '\n' | '\t' => out.push(ch),
            ch if ch.is_control() => out.push(' '),
            ch => out.push(ch),
        }
    }
    out
}

pub fn sanitize_manual_project_notes_content(value: &str) -> String {
    escape_model_pool_index_marker_lines(&sanitize_project_notes_control_chars(value))
}

pub fn first_disallowed_project_notes_control_char(value: &str) -> Option<(usize, char)> {
    value
        .char_indices()
        .find(|(_, ch)| ch.is_control() && *ch != '\n' && *ch != '\t')
}

pub fn project_notes_without_model_pool_index_blocks(content: &str) -> String {
    let spans = collect_index_block_spans(content);
    let mut output = String::new();
    let mut cursor = 0usize;

    for span in spans {
        if let Some(prefix) = content.get(cursor..span.start) {
            output.push_str(prefix);
        }
        cursor = span.end;
    }
    if let Some(suffix) = content.get(cursor..) {
        output.push_str(suffix);
    }

    trim_redundant_edge_blank_lines(output)
}

pub fn clear_model_pool_index_note_blocks(content: &str) -> ModelPoolIndexClearResult {
    let spans = collect_index_block_spans(content);
    let mut output = String::new();
    let mut cursor = 0usize;
    let mut removed_blocks = 0usize;
    let mut legacy_undelimited_blocks = 0usize;

    for span in spans {
        if let Some(prefix) = content.get(cursor..span.start) {
            output.push_str(prefix);
        }
        cursor = span.end;
        removed_blocks += 1;
        if !span.delimited {
            legacy_undelimited_blocks += 1;
        }
    }
    if let Some(suffix) = content.get(cursor..) {
        output.push_str(suffix);
    }

    let content = trim_redundant_edge_blank_lines(output);
    ModelPoolIndexClearResult {
        content: sanitize_project_notes_control_chars(&content),
        removed_blocks,
        legacy_undelimited_blocks,
    }
}

fn model_pool_index_note_stats_for_spans(
    content: &str,
    spans: &[IndexBlockSpan],
) -> ModelPoolIndexNoteStats {
    let delimited_blocks = spans.iter().filter(|span| span.delimited).count();
    let trusted_blocks = spans
        .iter()
        .filter(|span| span.delimited && trusted_span_text(content, span))
        .count();
    let total_chars = spans
        .iter()
        .map(|span| sanitized_span_text(content, span).chars().count())
        .sum();
    let active_index = active_index_block_index(spans);
    let active_trusted = active_index
        .and_then(|index| spans.get(index))
        .is_some_and(|span| span.delimited && trusted_span_text(content, span));
    ModelPoolIndexNoteStats {
        block_count: spans.len(),
        delimited_blocks,
        legacy_undelimited_blocks: spans.len().saturating_sub(delimited_blocks),
        active: active_index
            .and_then(|index| spans.get(index))
            .map(|span| {
                if span.delimited {
                    ModelPoolIndexNoteActive::LatestDelimited
                } else {
                    ModelPoolIndexNoteActive::LatestLegacyUndelimited
                }
            })
            .unwrap_or(ModelPoolIndexNoteActive::None),
        active_trusted,
        trusted_blocks,
        context_active: if trusted_blocks == 0 {
            ModelPoolIndexNoteContextActive::None
        } else {
            ModelPoolIndexNoteContextActive::LatestTrustedDelimited
        },
        total_chars,
    }
}

fn active_index_block_index(spans: &[IndexBlockSpan]) -> Option<usize> {
    spans
        .iter()
        .rposition(|span| span.delimited)
        .or_else(|| spans.len().checked_sub(1))
}

fn collect_index_block_spans(content: &str) -> Vec<IndexBlockSpan> {
    let mut spans = Vec::new();
    let mut active_start = None;

    for (line_start, line_end, line) in line_spans(content) {
        if is_marker_line(line, MODEL_POOL_INDEX_NOTE_MARKER) {
            if let Some(start) = active_start.replace(line_start)
                && start < line_start
            {
                spans.push(IndexBlockSpan {
                    start,
                    end: line_start,
                    delimited: false,
                });
            }
        } else if is_marker_line(line, MODEL_POOL_INDEX_NOTE_END_MARKER)
            && let Some(start) = active_start.take()
        {
            spans.push(IndexBlockSpan {
                start,
                end: line_end,
                delimited: true,
            });
        }
    }

    if let Some(start) = active_start
        && start < content.len()
    {
        spans.push(IndexBlockSpan {
            start,
            end: content.len(),
            delimited: false,
        });
    }

    spans
}

fn sanitized_span_text(content: &str, span: &IndexBlockSpan) -> String {
    sanitize_project_notes_control_chars(content.get(span.start..span.end).unwrap_or_default())
        .trim()
        .to_owned()
}

fn trusted_span_text(content: &str, span: &IndexBlockSpan) -> bool {
    content
        .get(span.start..span.end)
        .map(sanitize_project_notes_control_chars)
        .as_deref()
        .map(validate_trusted_model_pool_index_note_block)
        .is_some_and(|result| result.is_ok())
}

fn line_spans(content: &str) -> impl Iterator<Item = (usize, usize, &str)> {
    let mut offset = 0usize;
    content.split_inclusive('\n').map(move |line| {
        let start = offset;
        offset += line.len();
        (start, offset, line)
    })
}

fn is_marker_line(line: &str, marker: &str) -> bool {
    line.trim() == marker
}

fn marker_line_count(lines: &[&str], marker: &str) -> usize {
    lines.iter().filter(|line| line.trim() == marker).count()
}

fn require_line(lines: &[&str], index: usize, expected: &str) -> Result<(), String> {
    match lines.get(index) {
        Some(line) if *line == expected => Ok(()),
        Some(line) => Err(format!(
            "model pool index pin line {index} expected {expected:?}, got {line:?}"
        )),
        None => Err(format!(
            "model pool index pin missing line {index} expected {expected:?}"
        )),
    }
}

fn required_prefixed_line_value<'a>(lines: &'a [&str], prefix: &str) -> Result<&'a str, String> {
    lines
        .iter()
        .find_map(|line| line.strip_prefix(prefix))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("model pool index pin missing {prefix}"))
}

fn escape_model_pool_index_marker_lines(value: &str) -> String {
    let mut out = String::new();
    for line in value.split_inclusive('\n') {
        let (body, newline) = line
            .strip_suffix('\n')
            .map(|body| (body, "\n"))
            .unwrap_or((line, ""));
        if is_marker_line(body, MODEL_POOL_INDEX_NOTE_MARKER)
            || is_marker_line(body, MODEL_POOL_INDEX_NOTE_END_MARKER)
        {
            out.push_str(ESCAPED_MODEL_POOL_INDEX_MARKER_PREFIX);
            out.push_str(body.trim());
        } else {
            out.push_str(body);
        }
        out.push_str(newline);
    }
    out
}

fn trim_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_owned();
    }
    let suffix = "\n[index note preview truncated]";
    let keep_chars = max_chars.saturating_sub(suffix.chars().count());
    let mut out = value.chars().take(keep_chars).collect::<String>();
    out.push_str(suffix);
    out
}

fn trim_redundant_edge_blank_lines(mut value: String) -> String {
    while value.starts_with("\n\n") {
        value.remove(0);
    }
    while value.ends_with("\n\n\n") {
        value.pop();
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_delimited_and_legacy_index_notes() {
        let content = concat!(
            "manual note\n",
            "model_pool_index:\n",
            "answer:\n",
            "src/model_service\n",
            "model_pool_index_end:\n",
            "model_pool_index:\n",
            "legacy\n"
        );

        let rendered = render_model_pool_index_notes(content);

        assert!(rendered.contains("model_pool_index_notes=2"));
        assert!(rendered.contains("delimited=1"));
        assert!(rendered.contains("legacy_undelimited=1"));
        assert!(rendered.contains("active=latest_delimited"));
        assert!(rendered.contains("trusted=0"));
        assert!(rendered.contains("context_active=none"));
        assert!(rendered.contains("index_note_1 status=delimited"));
        assert!(rendered.contains("index_note_1 status=delimited chars="));
        assert!(rendered.contains("index_note_2 status=legacy_undelimited"));
        assert!(rendered.lines().any(|line| line.contains("index_note_1")
            && line.contains("status=delimited")
            && line.contains("active=true")
            && line.contains("trusted=false")
            && line.contains("context_active=false")));
        assert!(rendered.lines().any(|line| line.contains("index_note_2")
            && line.contains("status=legacy_undelimited")
            && line.contains("active=false")
            && line.contains("trusted=false")
            && line.contains("context_active=false")));
        assert!(rendered.contains("src/model_service"));

        let stats = model_pool_index_note_stats(content);
        assert_eq!(stats.block_count, 2);
        assert_eq!(stats.delimited_blocks, 1);
        assert_eq!(stats.legacy_undelimited_blocks, 1);
        assert_eq!(stats.active, ModelPoolIndexNoteActive::LatestDelimited);
        assert!(stats.total_chars > 0);
    }

    #[test]
    fn renders_trusted_context_active_separately_from_visible_active() {
        let content = concat!(
            "model_pool_index:\n",
            "source_prompt: old repo map\n",
            "selected_role: index\n",
            "selected_base_url: http://127.0.0.1:8687\n",
            "answer:\n",
            "OLD_TRUSTED src/old\n",
            "model_pool_index_end:\n",
            "model_pool_index:\n",
            "answer:\n",
            "UNTRUSTED src/manual\n",
            "model_pool_index_end:\n",
            "model_pool_index:\n",
            "source_prompt: latest repo map\n",
            "selected_role: index\n",
            "selected_base_url: http://127.0.0.1:8687\n",
            "answer:\n",
            "LATEST_TRUSTED src/new\n",
            "model_pool_index_end:\n",
            "model_pool_index:\n",
            "legacy stale\n"
        );

        let rendered = render_model_pool_index_notes(content);

        assert!(rendered.contains(
            "model_pool_index_notes=4 delimited=3 legacy_undelimited=1 active=latest_delimited active_trusted=true trusted=2 context_active=latest_trusted_delimited"
        ));
        assert!(rendered.lines().any(|line| line.contains("index_note_1")
            && line.contains("trusted=true")
            && line.contains("context_active=false")));
        assert!(rendered.lines().any(|line| line.contains("index_note_2")
            && line.contains("trusted=false")
            && line.contains("context_active=false")));
        assert!(rendered.lines().any(|line| line.contains("index_note_3")
            && line.contains("active=true")
            && line.contains("trusted=true")
            && line.contains("context_active=true")));
        assert!(rendered.lines().any(|line| line.contains("index_note_4")
            && line.contains("status=legacy_undelimited")
            && line.contains("trusted=false")
            && line.contains("context_active=false")));
    }

    #[test]
    fn renders_untrusted_visible_active_separately_from_context_active() {
        let content = concat!(
            "model_pool_index:\n",
            "source_prompt: old repo map\n",
            "selected_role: index\n",
            "selected_base_url: http://127.0.0.1:8687\n",
            "answer:\n",
            "OLD_TRUSTED src/old\n",
            "model_pool_index_end:\n",
            "model_pool_index:\n",
            "answer:\n",
            "UNTRUSTED_LATEST src/manual\n",
            "model_pool_index_end:\n"
        );

        let rendered = render_model_pool_index_notes(content);
        let stats = model_pool_index_note_stats(content);

        assert_eq!(stats.active, ModelPoolIndexNoteActive::LatestDelimited);
        assert!(!stats.active_trusted);
        assert_eq!(stats.trusted_blocks, 1);
        assert_eq!(
            stats.context_active,
            ModelPoolIndexNoteContextActive::LatestTrustedDelimited
        );
        assert!(rendered.contains(
            "model_pool_index_notes=2 delimited=2 legacy_undelimited=0 active=latest_delimited active_trusted=false trusted=1 context_active=latest_trusted_delimited"
        ));
        assert!(rendered.lines().any(|line| line.contains("index_note_1")
            && line.contains("active=false")
            && line.contains("trusted=true")
            && line.contains("context_active=true")));
        assert!(rendered.lines().any(|line| line.contains("index_note_2")
            && line.contains("active=true")
            && line.contains("trusted=false")
            && line.contains("context_active=false")));
    }

    #[test]
    fn clears_all_index_notes() {
        let content = concat!(
            "manual before\n",
            "model_pool_index:\n",
            "answer:\n",
            "remove me\n",
            "model_pool_index_end:\n",
            "manual after\n",
            "model_pool_index:\n",
            "legacy remains\n"
        );

        let result = clear_model_pool_index_note_blocks(content);

        assert_eq!(result.removed_blocks, 2);
        assert_eq!(result.legacy_undelimited_blocks, 1);
        assert!(result.content.contains("manual before"));
        assert!(result.content.contains("manual after"));
        assert!(!result.content.contains("legacy remains"));
        assert!(!result.content.contains("remove me"));
    }

    #[test]
    fn clear_index_notes_sanitizes_remaining_manual_notes() {
        let content = concat!(
            "manual\x1b[31m before\r\n",
            "model_pool_index:\n",
            "answer:\n",
            "remove me\n",
            "model_pool_index_end:\n",
            "manual after\x07\n"
        );

        let result = clear_model_pool_index_note_blocks(content);

        assert_eq!(result.removed_blocks, 1);
        assert!(result.content.contains("manual [31m before\n"));
        assert!(result.content.contains("manual after "));
        assert!(!result.content.contains('\x1b'));
        assert!(!result.content.contains('\x07'));
        assert!(!result.content.contains('\r'));
        assert!(first_disallowed_project_notes_control_char(&result.content).is_none());
    }

    #[test]
    fn removes_all_index_blocks_from_preview_content() {
        let content = concat!(
            "manual before\n",
            "model_pool_index:\n",
            "source_prompt: private index prompt\n",
            "answer:\n",
            "src/model_service\n",
            "model_pool_index_end:\n",
            "manual after\n",
            "model_pool_index:\n",
            "legacy source_prompt\n"
        );

        let preview = project_notes_without_model_pool_index_blocks(content);

        assert!(preview.contains("manual before"));
        assert!(preview.contains("manual after"));
        assert!(!preview.contains("source_prompt"));
        assert!(!preview.contains("src/model_service"));
        assert!(!preview.contains("legacy source_prompt"));
    }

    #[test]
    fn returns_latest_index_block() {
        let content = concat!(
            "manual\n",
            "model_pool_index:\n",
            "answer:\n",
            "old src/old\n",
            "model_pool_index_end:\n",
            "manual after old\n",
            "model_pool_index:\n",
            "answer:\n",
            "new src/model_service\n",
            "model_pool_index_end:\n"
        );

        let latest = latest_model_pool_index_block(content).unwrap();

        assert!(latest.contains("new src/model_service"));
        assert!(!latest.contains("old src/old"));
        assert!(latest.starts_with(MODEL_POOL_INDEX_NOTE_MARKER));
        assert!(latest.ends_with(MODEL_POOL_INDEX_NOTE_END_MARKER));
    }

    #[test]
    fn latest_safe_index_block_uses_latest_trusted_delimited_block() {
        let content = concat!(
            "model_pool_index:\n",
            "answer:\n",
            "UNTRUSTED_OLD src/old\n",
            "model_pool_index_end:\n",
            "manual\n",
            "model_pool_index:\n",
            "source_prompt: repo map\n",
            "selected_role: index\n",
            "selected_base_url: http://127.0.0.1:8687\n",
            "answer:\n",
            "TRUSTED_NEW src/model_service\n",
            "model_pool_index_end:\n",
            "model_pool_index:\n",
            "legacy stale\n"
        );

        let latest = latest_safe_model_pool_index_block(content).unwrap();

        assert!(latest.contains("TRUSTED_NEW src/model_service"));
        assert!(!latest.contains("UNTRUSTED_OLD src/old"));
        assert!(!latest.contains("legacy stale"));
    }

    #[test]
    fn latest_safe_index_block_ignores_untrusted_delimited_blocks() {
        let content = concat!(
            "manual\n",
            "model_pool_index:\n",
            "answer:\n",
            "manual fake index\n",
            "model_pool_index_end:\n"
        );

        assert_eq!(latest_safe_model_pool_index_block(content), None);
    }

    #[test]
    fn latest_safe_index_block_sanitizes_control_chars() {
        let content = concat!(
            "manual\n",
            "model_pool_index:\n",
            "source_prompt: repo map\n",
            "selected_role: index\n",
            "selected_base_url: http://127.0.0.1:8687\n",
            "answer:\n",
            "src\x1b[2J/model_service\x08 handles\r\nrouting\n",
            "model_pool_index_end:\n"
        );

        let latest = latest_safe_model_pool_index_block(content).unwrap();

        assert!(latest.contains("src [2J/model_service  handles\nrouting"));
        assert!(!latest.contains('\x1b'));
        assert!(!latest.contains('\x08'));
        assert!(!latest.contains('\r'));
        assert!(first_disallowed_project_notes_control_char(&latest).is_none());
    }

    #[test]
    fn trusted_index_block_validation_requires_pin_metadata() {
        let error = validate_trusted_model_pool_index_note_block(concat!(
            "model_pool_index:\n",
            "answer:\n",
            "old index\n",
            "model_pool_index_end:\n"
        ))
        .unwrap_err();

        assert!(error.contains("missing source_prompt"));
    }

    #[test]
    fn trusted_index_block_summary_extracts_pin_contract_fields() {
        let summary = trusted_model_pool_index_note_summary(concat!(
            "model_pool_index:\n",
            "source_prompt: map repo\n",
            "selected_role: index\n",
            "selected_base_url: http://127.0.0.1:8687\n",
            "answer:\n",
            "src/session handles context\n",
            "model_pool_index_end:\n"
        ))
        .unwrap();

        assert_eq!(summary.source_prompt, "map repo");
        assert_eq!(summary.selected_role, "index");
        assert_eq!(summary.selected_base_url, "http://127.0.0.1:8687");
        assert_eq!(
            summary.answer_chars,
            "src/session handles context".chars().count()
        );
    }

    #[test]
    fn manual_project_notes_content_escapes_index_marker_lines() {
        let content = sanitize_manual_project_notes_content(concat!(
            "manual\n",
            " model_pool_index: \n",
            "answer:\n",
            "manual fake index\n",
            "\tmodel_pool_index_end:\n"
        ));

        assert!(content.contains("[escaped model_pool_index marker] model_pool_index:"));
        assert!(content.contains("[escaped model_pool_index marker] model_pool_index_end:"));
        assert_eq!(
            model_pool_index_note_stats(&content).active,
            ModelPoolIndexNoteActive::None
        );
    }

    #[test]
    fn render_index_notes_sanitizes_control_chars_in_preview() {
        let content = concat!(
            "model_pool_index:\n",
            "answer:\n",
            "src\x1b[2J/model_service\x07\n",
            "model_pool_index_end:\n"
        );

        let rendered = render_model_pool_index_notes(content);

        assert!(rendered.contains("src [2J/model_service "));
        assert!(!rendered.contains('\x1b'));
        assert!(!rendered.contains('\x07'));
        assert!(first_disallowed_project_notes_control_char(&rendered).is_none());
    }

    #[test]
    fn latest_index_block_prefers_complete_block_over_trailing_legacy() {
        let content = concat!(
            "model_pool_index:\n",
            "answer:\n",
            "complete src/model_service\n",
            "model_pool_index_end:\n",
            "manual after\n",
            "model_pool_index:\n",
            "legacy stale src/old\n"
        );

        let latest = latest_model_pool_index_block(content).unwrap();

        assert!(latest.contains("complete src/model_service"));
        assert!(!latest.contains("legacy stale src/old"));
        assert!(latest.ends_with(MODEL_POOL_INDEX_NOTE_END_MARKER));
    }

    #[test]
    fn render_marks_latest_complete_index_active_over_trailing_legacy() {
        let content = concat!(
            "model_pool_index:\n",
            "answer:\n",
            "complete src/model_service\n",
            "model_pool_index_end:\n",
            "manual after\n",
            "model_pool_index:\n",
            "legacy stale src/old\n"
        );

        let rendered = render_model_pool_index_notes(content);

        assert!(rendered.contains("active=latest_delimited"));
        assert!(rendered.contains("index_note_1 status=delimited"));
        assert!(rendered.contains("index_note_2 status=legacy_undelimited"));
        assert!(rendered.lines().any(|line| line.contains("index_note_1")
            && line.contains("status=delimited")
            && line.contains("active=true")));
        assert!(rendered.lines().any(|line| line.contains("index_note_2")
            && line.contains("status=legacy_undelimited")
            && line.contains("active=false")));
    }

    #[test]
    fn render_marks_legacy_active_only_when_no_complete_index_exists() {
        let content = "manual\nmodel_pool_index:\nlegacy-only src/old\n";

        let rendered = render_model_pool_index_notes(content);
        let stats = model_pool_index_note_stats(content);

        assert_eq!(
            stats.active,
            ModelPoolIndexNoteActive::LatestLegacyUndelimited
        );
        assert!(rendered.contains("active=latest_legacy_undelimited"));
        assert!(rendered.contains("index_note_1 status=legacy_undelimited"));
        assert!(rendered.contains("active=true"));
    }
}
