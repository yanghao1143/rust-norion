use super::json::{json_bool_field, json_number_field, json_string_array_field, json_string_field};

const DEFAULT_FINDING_LIMIT: usize = 3;

pub(crate) fn experience_hygiene_report_summary(body: &str) -> Result<String, String> {
    ensure_ok(body, "experience hygiene report")?;
    let mut lines = vec!["Noiron experience hygiene".to_owned()];
    push_field_line(
        &mut lines,
        "experience_file",
        json_string_field(body, "experience_file"),
    );
    push_field_line(
        &mut lines,
        "checked",
        json_bool_field(body, "checked").map(|value| value.to_string()),
    );
    push_field_line(&mut lines, "error", json_string_field(body, "error"));

    if let Some(report) = json_object_field(body, "report") {
        lines.push(counts_line("report", report));
        push_findings(&mut lines, report);
    } else {
        lines.push("report=none".to_owned());
    }
    if let Some(index_report) = json_object_field(body, "index_report") {
        lines.push(index_report_line(index_report));
        push_index_findings(&mut lines, index_report);
    }
    if let Some(plan) = json_object_field(body, "quarantine_plan") {
        lines.push(counts_line("quarantine_plan", plan));
        push_candidate_ids(&mut lines, plan);
    }

    Ok(lines.join("\n"))
}

pub(crate) fn experience_hygiene_quarantine_summary(body: &str) -> Result<String, String> {
    ensure_ok(body, "experience hygiene quarantine")?;
    let mut lines = vec!["Noiron experience hygiene quarantine dry-run".to_owned()];
    push_field_line(
        &mut lines,
        "experience_file",
        json_string_field(body, "experience_file"),
    );
    push_field_line(
        &mut lines,
        "applied",
        json_bool_field(body, "applied").map(|value| value.to_string()),
    );
    push_field_line(
        &mut lines,
        "backup_file",
        json_string_field(body, "backup_file"),
    );
    push_field_line(
        &mut lines,
        "quarantine_file",
        json_string_field(body, "quarantine_file"),
    );

    if let Some(plan) = json_object_field(body, "plan") {
        lines.push(counts_line("plan", plan));
        push_candidate_ids(&mut lines, plan);
        push_findings(&mut lines, plan);
    } else {
        lines.push("plan=none".to_owned());
    }

    Ok(lines.join("\n"))
}

fn ensure_ok(body: &str, label: &str) -> Result<(), String> {
    if json_bool_field(body, "ok") == Some(false) {
        let error = json_string_field(body, "error").unwrap_or_else(|| "unknown".to_owned());
        return Err(format!("{label} failed: {error}"));
    }
    Ok(())
}

fn counts_line(label: &str, object: &str) -> String {
    let total = json_number_field(object, "total_records").unwrap_or_else(|| "unknown".to_owned());
    let retained =
        json_number_field(object, "retained_records").unwrap_or_else(|| "unknown".to_owned());
    let findings = json_number_field(object, "findings").unwrap_or_else(|| "unknown".to_owned());
    let candidates =
        json_number_field(object, "quarantine_candidates").unwrap_or_else(|| "unknown".to_owned());
    let clean = json_bool_field(object, "clean")
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unknown".to_owned());

    if retained == "unknown" {
        format!(
            "{label}: total_records={total} findings={findings} quarantine_candidates={candidates} clean={clean}"
        )
    } else {
        format!(
            "{label}: total_records={total} retained_records={retained} quarantine_candidates={candidates}"
        )
    }
}

fn index_report_line(object: &str) -> String {
    let total = json_number_field(object, "total_records").unwrap_or_else(|| "unknown".to_owned());
    let compacted =
        json_number_field(object, "compacted_records").unwrap_or_else(|| "unknown".to_owned());
    let noisy = json_number_field(object, "noisy_records").unwrap_or_else(|| "unknown".to_owned());
    let duplicates =
        json_number_field(object, "duplicate_outputs").unwrap_or_else(|| "unknown".to_owned());
    let max_noise =
        json_number_field(object, "max_noise_penalty").unwrap_or_else(|| "unknown".to_owned());
    let quality =
        json_number_field(object, "quality_score").unwrap_or_else(|| "unknown".to_owned());
    let retrieval_ready = json_bool_field(object, "retrieval_ready")
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unknown".to_owned());
    let risk_level =
        json_string_field(object, "risk_level").unwrap_or_else(|| "unknown".to_owned());
    format!(
        "index_report: total_records={total} compacted_records={compacted} noisy_records={noisy} duplicate_outputs={duplicates} max_noise_penalty={max_noise} quality_score={quality} retrieval_ready={retrieval_ready} risk_level={risk_level}"
    )
}

fn push_candidate_ids(lines: &mut Vec<String>, object: &str) {
    let ids = json_number_array_field(object, "candidate_ids").unwrap_or_default();
    if ids.is_empty() {
        return;
    }
    let preview = ids.iter().take(12).cloned().collect::<Vec<_>>().join(",");
    let suffix = if ids.len() > 12 { ",..." } else { "" };
    lines.push(format!("candidate_ids=[{preview}{suffix}]"));
}

fn push_findings(lines: &mut Vec<String>, object: &str) {
    let Some(findings) = json_array_field(object, "listed_findings") else {
        return;
    };
    let items = json_object_items(findings);
    if items.is_empty() {
        return;
    }
    lines.push(format!("listed_findings={}", items.len()));
    for item in items.into_iter().take(DEFAULT_FINDING_LIMIT) {
        let id = json_number_field(item, "experience_id").unwrap_or_else(|| "unknown".to_owned());
        let severity = json_string_field(item, "severity").unwrap_or_else(|| "unknown".to_owned());
        let reason = json_string_field(item, "reason").unwrap_or_else(|| "unknown".to_owned());
        let markers = json_string_array_field(item, "markers")
            .unwrap_or_default()
            .join(",");
        let prompt = compact_preview(
            &json_string_field(item, "prompt_preview").unwrap_or_else(|| "unknown".to_owned()),
        );
        lines.push(format!(
            "finding id={id} severity={severity} reason={reason} markers={markers} prompt={prompt}"
        ));
    }
}

fn push_index_findings(lines: &mut Vec<String>, object: &str) {
    let Some(findings) = json_array_field(object, "listed_findings") else {
        return;
    };
    let items = json_object_items(findings);
    if items.is_empty() {
        return;
    }
    lines.push(format!("index_findings={}", items.len()));
    for item in items.into_iter().take(DEFAULT_FINDING_LIMIT) {
        let id = json_number_field(item, "experience_id").unwrap_or_else(|| "unknown".to_owned());
        let reason = json_string_field(item, "reason").unwrap_or_else(|| "unknown".to_owned());
        let compacted = json_bool_field(item, "compacted")
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unknown".to_owned());
        let noise =
            json_number_field(item, "noise_penalty").unwrap_or_else(|| "unknown".to_owned());
        let prompt_chars =
            json_number_field(item, "prompt_chars").unwrap_or_else(|| "unknown".to_owned());
        let lesson_chars =
            json_number_field(item, "lesson_chars").unwrap_or_else(|| "unknown".to_owned());
        let prompt = compact_preview(
            &json_string_field(item, "prompt_preview").unwrap_or_else(|| "unknown".to_owned()),
        );
        lines.push(format!(
            "index_finding id={id} reason={reason} compacted={compacted} noise_penalty={noise} prompt_chars={prompt_chars} lesson_chars={lesson_chars} prompt={prompt}"
        ));
    }
}

fn push_field_line(lines: &mut Vec<String>, name: &str, value: Option<String>) {
    if let Some(value) = value {
        lines.push(format!("{name}={value}"));
    }
}

fn compact_preview(value: &str) -> String {
    const MAX_CHARS: usize = 160;
    let compacted = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut preview = compacted.chars().take(MAX_CHARS).collect::<String>();
    if compacted.chars().count() > MAX_CHARS {
        preview.push_str("...");
    }
    preview
}

fn json_number_array_field(body: &str, field: &str) -> Option<Vec<String>> {
    let array = json_array_field(body, field)?;
    let mut input = array.trim();
    if input.is_empty() {
        return Some(Vec::new());
    }
    let mut values = Vec::new();
    loop {
        let number_len = json_unsigned_integer_literal_len(input)?;
        values.push(input.get(..number_len)?.to_owned());
        input = input.get(number_len..)?.trim_start();
        if input.is_empty() {
            return Some(values);
        }
        input = input.strip_prefix(',')?.trim_start();
        if input.is_empty() {
            return None;
        }
    }
}

fn json_unsigned_integer_literal_len(input: &str) -> Option<usize> {
    let bytes = input.as_bytes();
    match bytes.first()? {
        b'0' => Some(1),
        b'1'..=b'9' => {
            let mut index = 1usize;
            while matches!(bytes.get(index), Some(b'0'..=b'9')) {
                index += 1;
            }
            Some(index)
        }
        _ => None,
    }
}

fn json_array_field<'a>(body: &'a str, field: &str) -> Option<&'a str> {
    let needle = format!("\"{field}\"");
    let after_field = body.get(body.find(&needle)? + needle.len()..)?;
    let after_colon = after_field.get(after_field.find(':')? + 1..)?;
    let trimmed = after_colon.trim_start();
    if !trimmed.starts_with('[') {
        return None;
    }
    let close = find_matching_json_close(trimmed, '[', ']')?;
    let trailing = trimmed.get(close + 1..)?.trim_start();
    if !json_value_tail_is_delimited(trailing) {
        return None;
    }
    trimmed.get(1..close)
}

fn json_object_field<'a>(body: &'a str, field: &str) -> Option<&'a str> {
    let needle = format!("\"{field}\"");
    let after_field = body.get(body.find(&needle)? + needle.len()..)?;
    let after_colon = after_field.get(after_field.find(':')? + 1..)?;
    let trimmed = after_colon.trim_start();
    if !trimmed.starts_with('{') {
        return None;
    }
    let close = find_matching_json_close(trimmed, '{', '}')?;
    let trailing = trimmed.get(close + 1..)?.trim_start();
    if !json_value_tail_is_delimited(trailing) {
        return None;
    }
    trimmed.get(..=close)
}

fn json_object_items(input: &str) -> Vec<&str> {
    let mut items = Vec::new();
    let mut start = None;
    let mut depth = 0usize;
    let mut chars = input.char_indices().peekable();

    while let Some((index, character)) = chars.next() {
        match character {
            '"' => {
                if skip_json_string_literal(&mut chars).is_none() {
                    return items;
                }
            }
            '{' => {
                if depth == 0 {
                    start = Some(index);
                }
                depth = depth.saturating_add(1);
            }
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0
                    && let Some(start_index) = start.take()
                    && let Some(item) = input.get(start_index..=index)
                {
                    items.push(item);
                }
            }
            _ => {}
        }
    }

    items
}

fn find_matching_json_close(input: &str, open: char, close: char) -> Option<usize> {
    let mut depth = 0usize;
    let mut chars = input.char_indices().peekable();

    while let Some((index, character)) = chars.next() {
        match character {
            '"' => skip_json_string_literal(&mut chars)?,
            value if value == open => depth = depth.saturating_add(1),
            value if value == close => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }

    None
}

fn json_value_tail_is_delimited(trailing: &str) -> bool {
    trailing.is_empty() || matches!(trailing.as_bytes().first(), Some(b',' | b'}' | b']'))
}

fn skip_json_string_literal(
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
) -> Option<()> {
    let mut escaped = false;
    while let Some((_, character)) = chars.next() {
        if escaped {
            match character {
                '"' | '\\' | '/' | 'n' | 'r' | 't' | 'b' | 'f' => {}
                'u' => skip_json_unicode_escape(chars)?,
                _ => return None,
            }
            escaped = false;
            continue;
        }

        match character {
            '\\' => escaped = true,
            '"' => return Some(()),
            value if value.is_control() => return None,
            _ => {}
        }
    }
    None
}

fn skip_json_unicode_escape(
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
) -> Option<()> {
    let code = skip_json_hex_escape(chars)?;
    if (0xd800..=0xdbff).contains(&code) {
        if chars.next()?.1 != '\\' || chars.next()?.1 != 'u' {
            return None;
        }
        let low = skip_json_hex_escape(chars)?;
        if !(0xdc00..=0xdfff).contains(&low) {
            return None;
        }
    } else if (0xdc00..=0xdfff).contains(&code) {
        return None;
    }
    Some(())
}

fn skip_json_hex_escape(chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>) -> Option<u16> {
    let mut value = 0_u16;
    for _ in 0..4 {
        let digit = chars.next()?.1.to_digit(16)?;
        value = (value << 4) | u16::try_from(digit).ok()?;
    }
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarizes_experience_hygiene_report() {
        let summary = experience_hygiene_report_summary(
            "{\"ok\":true,\"experience_file\":\"noiron-experience.ndkv\",\"checked\":true,\"error\":null,\"report\":{\"total_records\":863,\"findings\":4,\"quarantine_candidates\":4,\"clean\":false,\"listed_findings\":[{\"experience_id\":861,\"severity\":\"quarantine_candidate\",\"reason\":\"cross_task_shell_transcript\",\"markers\":[\"gitlab_local\",\"bash_command\"],\"prompt_preview\":\"Conversation transcript\",\"lesson_preview\":\"lesson\"}]},\"index_report\":{\"total_records\":863,\"compacted_records\":194,\"noisy_records\":1,\"duplicate_outputs\":1,\"max_noise_penalty\":0.18,\"quality_score\":0.805,\"retrieval_ready\":true,\"risk_level\":\"watch\",\"listed_findings\":[{\"experience_id\":861,\"reason\":\"unstructured_long_transcript\",\"compacted\":true,\"noise_penalty\":0.18,\"prompt_chars\":4096,\"lesson_chars\":512,\"prompt_preview\":\"Conversation transcript\",\"lesson_preview\":\"lesson\"}]},\"quarantine_plan\":{\"applied\":false,\"total_records\":863,\"retained_records\":859,\"quarantine_candidates\":4,\"candidate_ids\":[851,861,862,863],\"listed_findings\":[]}}",
        )
        .unwrap();

        assert!(summary.contains("experience_file=noiron-experience.ndkv"));
        assert!(summary.contains("report: total_records=863"));
        assert!(summary.contains("index_report: total_records=863"));
        assert!(summary.contains("compacted_records=194"));
        assert!(summary.contains("noisy_records=1"));
        assert!(summary.contains("duplicate_outputs=1"));
        assert!(summary.contains("max_noise_penalty=0.18"));
        assert!(summary.contains("quality_score=0.805"));
        assert!(summary.contains("retrieval_ready=true"));
        assert!(summary.contains("risk_level=watch"));
        assert!(summary.contains("index_findings=1"));
        assert!(summary.contains("index_finding id=861"));
        assert!(summary.contains("reason=unstructured_long_transcript"));
        assert!(summary.contains("quarantine_candidates=4"));
        assert!(summary.contains("candidate_ids=[851,861,862,863]"));
        assert!(summary.contains("finding id=861"));
    }

    #[test]
    fn compacts_long_finding_previews() {
        let long_prompt = "rust ".repeat(80);
        let body = format!(
            "{{\"ok\":true,\"checked\":true,\"report\":{{\"total_records\":1,\"findings\":1,\"quarantine_candidates\":1,\"clean\":false,\"listed_findings\":[{{\"experience_id\":1,\"severity\":\"watch\",\"reason\":\"long\",\"markers\":[],\"prompt_preview\":\"{}\"}}]}}}}",
            long_prompt.trim()
        );

        let summary = experience_hygiene_report_summary(&body).unwrap();

        assert!(summary.contains("prompt=rust rust"));
        assert!(summary.contains("..."));
        assert!(summary.lines().all(|line| line.len() < 260));
    }

    #[test]
    fn listed_findings_keep_braces_inside_prompt_preview() {
        let summary = experience_hygiene_report_summary(
            r#"{"ok":true,"report":{"total_records":1,"findings":1,"quarantine_candidates":1,"clean":false,"listed_findings":[{"experience_id":1,"severity":"watch","reason":"brace","markers":[],"prompt_preview":"literal { brace } text"}]}}"#,
        )
        .unwrap();

        assert!(summary.contains("listed_findings=1"));
        assert!(summary.contains("prompt=literal { brace } text"));
    }

    #[test]
    fn invalid_listed_finding_string_is_not_summarized() {
        let summary = experience_hygiene_report_summary(
            "{\"ok\":true,\"report\":{\"total_records\":1,\"findings\":1,\"quarantine_candidates\":1,\"clean\":false,\"listed_findings\":[{\"experience_id\":1,\"prompt_preview\":\"bad\\q\"}]}}",
        )
        .unwrap();

        assert!(summary.contains("report=none"));
        assert!(!summary.contains("listed_findings=1"));
    }

    #[test]
    fn report_object_rejects_trailing_garbage() {
        let summary = experience_hygiene_report_summary(
            r#"{"ok":true,"report":{"total_records":1,"findings":1,"quarantine_candidates":1,"clean":false}x}"#,
        )
        .unwrap();

        assert!(summary.contains("report=none"));
    }

    #[test]
    fn listed_findings_array_rejects_trailing_garbage() {
        let summary = experience_hygiene_report_summary(
            r#"{"ok":true,"report":{"total_records":1,"findings":1,"quarantine_candidates":1,"clean":false,"listed_findings":[{"experience_id":1}]x}}"#,
        )
        .unwrap();

        assert!(summary.contains("report: total_records=1"));
        assert!(!summary.contains("listed_findings=1"));
    }

    #[test]
    fn summarizes_quarantine_dry_run() {
        let summary = experience_hygiene_quarantine_summary(
            "{\"ok\":true,\"experience_file\":\"noiron-experience.ndkv\",\"applied\":false,\"backup_file\":null,\"quarantine_file\":null,\"plan\":{\"applied\":false,\"total_records\":863,\"retained_records\":859,\"quarantine_candidates\":4,\"candidate_ids\":[851,861],\"listed_findings\":[]}}",
        )
        .unwrap();

        assert!(summary.contains("quarantine dry-run"));
        assert!(summary.contains("applied=false"));
        assert!(summary.contains("retained_records=859"));
        assert!(summary.contains("candidate_ids=[851,861]"));
    }

    #[test]
    fn candidate_ids_accept_zero_and_spaced_unsigned_integers() {
        let summary = experience_hygiene_quarantine_summary(
            "{\"ok\":true,\"plan\":{\"applied\":false,\"total_records\":2,\"retained_records\":2,\"quarantine_candidates\":2,\"candidate_ids\":[0, 42],\"listed_findings\":[]}}",
        )
        .unwrap();

        assert!(summary.contains("candidate_ids=[0,42]"));
    }

    #[test]
    fn candidate_ids_reject_malformed_number_tokens() {
        let summary = experience_hygiene_quarantine_summary(
            "{\"ok\":true,\"plan\":{\"applied\":false,\"total_records\":2,\"retained_records\":2,\"quarantine_candidates\":2,\"candidate_ids\":[851x,861],\"listed_findings\":[]}}",
        )
        .unwrap();
        let leading_zero = experience_hygiene_quarantine_summary(
            "{\"ok\":true,\"plan\":{\"applied\":false,\"total_records\":1,\"retained_records\":1,\"quarantine_candidates\":1,\"candidate_ids\":[01],\"listed_findings\":[]}}",
        )
        .unwrap();
        let decimal = experience_hygiene_quarantine_summary(
            "{\"ok\":true,\"plan\":{\"applied\":false,\"total_records\":1,\"retained_records\":1,\"quarantine_candidates\":1,\"candidate_ids\":[1.2],\"listed_findings\":[]}}",
        )
        .unwrap();

        assert!(!summary.contains("candidate_ids="));
        assert!(!leading_zero.contains("candidate_ids="));
        assert!(!decimal.contains("candidate_ids="));
    }
}
