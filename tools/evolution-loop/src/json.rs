use std::collections::BTreeMap;

pub(crate) fn json_string(value: &str) -> String {
    let mut escaped = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            other if other.is_control() => escaped.push_str(&format!("\\u{:04x}", other as u32)),
            other => escaped.push(other),
        }
    }
    escaped.push('"');
    escaped
}

pub(crate) fn json_string_array(values: &[String]) -> String {
    let items = values
        .iter()
        .map(|value| json_string(value))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

pub(crate) fn parse_json_string_array(input: &str) -> Vec<String> {
    let mut values = Vec::new();
    let Some(array) = parse_json_array(input.trim_start()) else {
        return values;
    };
    let mut index = 1usize;
    while index + 1 < array.len() {
        let Some(rest) = array.get(index..) else {
            break;
        };
        let trimmed = rest
            .trim_start_matches(|character: char| character.is_whitespace() || character == ',');
        index = array.len() - trimmed.len();
        if trimmed.starts_with(']') {
            break;
        }
        if let Some(value) = parse_json_string(trimmed) {
            let consumed = json_string(&value).len();
            values.push(value);
            index += consumed;
        } else {
            break;
        }
    }
    values
}

pub(crate) fn parse_json_string_array_map(input: &str) -> BTreeMap<String, Vec<String>> {
    let mut values_by_key = BTreeMap::new();
    let Some(object) = parse_json_object(input.trim_start()) else {
        return values_by_key;
    };
    let mut index = 1usize;
    while index + 1 < object.len() {
        let Some(rest) = object.get(index..) else {
            break;
        };
        let trimmed = rest
            .trim_start_matches(|character: char| character.is_whitespace() || character == ',');
        index = object.len() - trimmed.len();
        if trimmed.starts_with('}') {
            break;
        }
        let Some(key) = parse_json_string(trimmed) else {
            break;
        };
        let after_key_index = index + json_string(&key).len();
        let Some(after_key) = object.get(after_key_index..) else {
            break;
        };
        let after_key_trimmed = after_key.trim_start();
        if !after_key_trimmed.starts_with(':') {
            break;
        }
        let after_colon = after_key_trimmed
            .strip_prefix(':')
            .unwrap_or_default()
            .trim_start();
        let after_colon_index = object.len() - after_colon.len();
        let Some(array) = parse_json_array(after_colon) else {
            break;
        };
        values_by_key.insert(key, parse_json_string_array(array));
        index = after_colon_index + array.len();
    }
    values_by_key
}

pub(crate) fn parse_json_string_map(input: &str) -> BTreeMap<String, String> {
    let mut values_by_key = BTreeMap::new();
    let Some(object) = parse_json_object(input.trim_start()) else {
        return values_by_key;
    };
    let mut index = 1usize;
    while index + 1 < object.len() {
        let Some(rest) = object.get(index..) else {
            break;
        };
        let trimmed = rest
            .trim_start_matches(|character: char| character.is_whitespace() || character == ',');
        index = object.len() - trimmed.len();
        if trimmed.starts_with('}') {
            break;
        }
        let Some(key) = parse_json_string(trimmed) else {
            break;
        };
        let after_key_index = index + json_string(&key).len();
        let Some(after_key) = object.get(after_key_index..) else {
            break;
        };
        let after_key_trimmed = after_key.trim_start();
        if !after_key_trimmed.starts_with(':') {
            break;
        }
        let after_colon = after_key_trimmed
            .strip_prefix(':')
            .unwrap_or_default()
            .trim_start();
        let after_colon_index = object.len() - after_colon.len();
        let Some(value) = parse_json_string(after_colon) else {
            break;
        };
        let consumed = json_string(&value).len();
        values_by_key.insert(key, value);
        index = after_colon_index + consumed;
    }
    values_by_key
}

pub(crate) fn parse_json_object_map(input: &str) -> BTreeMap<String, String> {
    let mut values_by_key = BTreeMap::new();
    let Some(object) = parse_json_object(input.trim_start()) else {
        return values_by_key;
    };
    let mut index = 1usize;
    while index + 1 < object.len() {
        let Some(rest) = object.get(index..) else {
            break;
        };
        let trimmed = rest
            .trim_start_matches(|character: char| character.is_whitespace() || character == ',');
        index = object.len() - trimmed.len();
        if trimmed.starts_with('}') {
            break;
        }
        let Some(key) = parse_json_string(trimmed) else {
            break;
        };
        let after_key_index = index + json_string(&key).len();
        let Some(after_key) = object.get(after_key_index..) else {
            break;
        };
        let after_key_trimmed = after_key.trim_start();
        if !after_key_trimmed.starts_with(':') {
            break;
        }
        let after_colon = after_key_trimmed
            .strip_prefix(':')
            .unwrap_or_default()
            .trim_start();
        let after_colon_index = object.len() - after_colon.len();
        let Some(value) = parse_json_object(after_colon) else {
            break;
        };
        values_by_key.insert(key, value.to_owned());
        index = after_colon_index + value.len();
    }
    values_by_key
}

pub(crate) fn parse_json_object_array(input: &str) -> Vec<String> {
    let mut values = Vec::new();
    let Some(array) = parse_json_array(input.trim_start()) else {
        return values;
    };
    let mut index = 1usize;
    while index + 1 < array.len() {
        let Some(rest) = array.get(index..) else {
            break;
        };
        let trimmed = rest
            .trim_start_matches(|character: char| character.is_whitespace() || character == ',');
        index = array.len() - trimmed.len();
        if trimmed.starts_with(']') {
            break;
        }
        let Some(value) = parse_json_object(trimmed) else {
            break;
        };
        values.push(value.to_owned());
        index += value.len();
    }
    values
}

pub(crate) fn json_string_field(body: &str, field: &str) -> Option<String> {
    let value = json_value_after_colon(body, field)?;
    parse_json_string(value)
}

pub(crate) fn json_bool_field(body: &str, field: &str) -> Option<bool> {
    let value = json_value_after_colon(body, field)?;
    if let Some(text) = parse_json_string(value) {
        return match text.trim() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        };
    }
    if value.starts_with("true") {
        Some(true)
    } else if value.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

pub(crate) fn json_u64_field(body: &str, field: &str) -> Option<u64> {
    let value = json_value_after_colon(body, field)?;
    if let Some(text) = parse_json_string(value) {
        return text.trim().parse::<u64>().ok();
    }
    let digits = value
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect::<String>();
    digits.parse::<u64>().ok()
}

pub(crate) fn json_i32_field(body: &str, field: &str) -> Option<i32> {
    let value = json_value_after_colon(body, field)?;
    if let Some(text) = parse_json_string(value) {
        return text.trim().parse::<i32>().ok();
    }
    let number = value
        .chars()
        .take_while(|character| character.is_ascii_digit() || *character == '-')
        .collect::<String>();
    number.parse::<i32>().ok()
}

pub(crate) fn json_object_field(body: &str, field: &str) -> Option<String> {
    parse_json_object(json_value_after_colon(body, field)?).map(ToOwned::to_owned)
}

pub(crate) fn json_array_field(body: &str, field: &str) -> Option<String> {
    parse_json_array(json_value_after_colon(body, field)?).map(ToOwned::to_owned)
}

pub(crate) fn json_f64_field(body: &str, field: &str) -> Option<f64> {
    let value = json_value_after_colon(body, field)?;
    if let Some(text) = parse_json_string(value) {
        return text
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite());
    }
    let number = value
        .chars()
        .take_while(|character| {
            character.is_ascii_digit() || matches!(character, '-' | '+' | '.' | 'e' | 'E')
        })
        .collect::<String>();
    number.parse::<f64>().ok().filter(|value| value.is_finite())
}

pub(crate) fn preview_text(text: &str, max_chars: usize) -> String {
    let normalized = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" / ");
    let text = if normalized.is_empty() {
        text.trim()
    } else {
        normalized.as_str()
    };
    if text.chars().count() <= max_chars {
        return text.to_owned();
    }
    let keep_chars = max_chars.saturating_sub(3);
    let mut preview = text.chars().take(keep_chars).collect::<String>();
    preview.push_str("...");
    preview
}

fn json_value_after_colon<'a>(body: &'a str, field: &str) -> Option<&'a str> {
    let needle = format!("\"{field}\"");
    let mut search_start = 0usize;
    while let Some(relative_start) = body.get(search_start..)?.find(&needle) {
        let field_start = search_start + relative_start;
        let before_field = body
            .get(..field_start)?
            .chars()
            .rev()
            .find(|character| !character.is_whitespace());
        if before_field.is_none_or(|character| matches!(character, '{' | ',')) {
            let after_field = body.get(field_start + needle.len()..)?.trim_start();
            if let Some(after_colon) = after_field.strip_prefix(':') {
                return Some(after_colon.trim_start());
            }
        }
        search_start = field_start + needle.len();
    }
    None
}

fn parse_json_string(input: &str) -> Option<String> {
    let mut chars = input.chars();
    if chars.next()? != '"' {
        return None;
    }

    let mut output = String::new();
    let mut escaped = false;
    for character in chars {
        if escaped {
            match character {
                '"' => output.push('"'),
                '\\' => output.push('\\'),
                '/' => output.push('/'),
                'n' => output.push('\n'),
                'r' => output.push('\r'),
                't' => output.push('\t'),
                'b' => output.push('\u{0008}'),
                'f' => output.push('\u{000c}'),
                other => output.push(other),
            }
            escaped = false;
            continue;
        }

        match character {
            '\\' => escaped = true,
            '"' => return Some(output),
            other => output.push(other),
        }
    }
    None
}

fn parse_json_object(input: &str) -> Option<&str> {
    let mut chars = input.char_indices();
    if chars.next()?.1 != '{' {
        return None;
    }

    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, character) in input.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }

        match character {
            '"' => in_string = true,
            '{' => depth = depth.saturating_add(1),
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return input.get(..=index);
                }
            }
            _ => {}
        }
    }
    None
}

fn parse_json_array(input: &str) -> Option<&str> {
    let mut chars = input.char_indices();
    if chars.next()?.1 != '[' {
        return None;
    }

    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, character) in input.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }

        match character {
            '"' => in_string = true,
            '[' => depth = depth.saturating_add(1),
            ']' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return input.get(..=index);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_and_reads_json_string() {
        let body = format!("{{\"answer\":{}}}", json_string("a\"b\\c\n"));

        assert_eq!(
            json_string_field(&body, "answer").as_deref(),
            Some("a\"b\\c\n")
        );
    }

    #[test]
    fn previews_without_breaking_unicode() {
        assert_eq!(preview_text("alpha\nbeta", 64), "alpha / beta");
        assert_eq!(preview_text("abcdef", 5), "ab...");
    }

    #[test]
    fn reads_json_float_field() {
        assert_eq!(json_f64_field("{\"score\":0.125}", "score"), Some(0.125));
        assert_eq!(json_f64_field("{\"score\":\"1.5\"}", "score"), Some(1.5));
    }

    #[test]
    fn reads_json_i32_field() {
        assert_eq!(json_i32_field("{\"status\":7}", "status"), Some(7));
        assert_eq!(json_i32_field("{\"status\":\"-1\"}", "status"), Some(-1));
    }

    #[test]
    fn reads_json_object_field_with_nested_strings() {
        let body =
            r#"{"eval":{"report_only":true,"error":"brace } kept","nested":{"ok":true}},"x":1}"#;

        assert_eq!(
            json_object_field(body, "eval").as_deref(),
            Some(r#"{"report_only":true,"error":"brace } kept","nested":{"ok":true}}"#)
        );
    }

    #[test]
    fn reads_json_array_field_with_nested_strings() {
        let body =
            r#"{"workers":[{"role":"quality","note":"bracket ] kept"},{"role":"summary"}],"x":1}"#;

        assert_eq!(
            json_array_field(body, "workers").as_deref(),
            Some(r#"[{"role":"quality","note":"bracket ] kept"},{"role":"summary"}]"#)
        );
    }

    #[test]
    fn parses_json_string_arrays() {
        assert_eq!(
            parse_json_string_array(r#"["review","quality","test-gate"]"#),
            vec![
                "review".to_owned(),
                "quality".to_owned(),
                "test-gate".to_owned()
            ]
        );
        assert!(parse_json_string_array("{}").is_empty());
    }

    #[test]
    fn parses_json_object_arrays() {
        let parsed = parse_json_object_array(
            r#"[{"task_kind":"summary","note":"brace } kept"},{"task_kind":"index","nested":{"ok":true}}]"#,
        );

        assert_eq!(
            parsed,
            vec![
                r#"{"task_kind":"summary","note":"brace } kept"}"#.to_owned(),
                r#"{"task_kind":"index","nested":{"ok":true}}"#.to_owned(),
            ]
        );
        assert!(parse_json_object_array("{}").is_empty());
    }

    #[test]
    fn parses_json_string_array_maps() {
        let parsed = parse_json_string_array_map(
            r#"{"summary":["memory_update: keep"],"test-gate":["validation_command: cargo test"],"review":[]}"#,
        );

        assert_eq!(
            parsed
                .get("summary")
                .and_then(|items| items.first())
                .map(String::as_str),
            Some("memory_update: keep")
        );
        assert_eq!(
            parsed
                .get("test-gate")
                .and_then(|items| items.first())
                .map(String::as_str),
            Some("validation_command: cargo test")
        );
        assert_eq!(parsed.get("review").map(Vec::len), Some(0));
        assert!(parse_json_string_array_map("[]").is_empty());
    }

    #[test]
    fn parses_json_string_maps() {
        let parsed = parse_json_string_map(
            r#"{"risk":"stale helper","verification":"cargo test","empty":""}"#,
        );

        assert_eq!(parsed.get("risk").map(String::as_str), Some("stale helper"));
        assert_eq!(
            parsed.get("verification").map(String::as_str),
            Some("cargo test")
        );
        assert_eq!(parsed.get("empty").map(String::as_str), Some(""));
        assert!(parse_json_string_map("[]").is_empty());
    }

    #[test]
    fn parses_json_object_maps() {
        let parsed = parse_json_object_map(
            r#"{"review":{"fields":{"risk":"stale"},"matched_markers":["risk"]},"summary":{"fields":{}}}"#,
        );

        assert_eq!(
            parsed.get("review").map(String::as_str),
            Some(r#"{"fields":{"risk":"stale"},"matched_markers":["risk"]}"#)
        );
        assert_eq!(
            parsed.get("summary").map(String::as_str),
            Some(r#"{"fields":{}}"#)
        );
        assert!(parse_json_object_map("[]").is_empty());
    }

    #[test]
    fn field_lookup_ignores_matching_string_values() {
        let body = r#"{"case":"eval","eval":{"report_only":true}}"#;

        assert_eq!(
            json_object_field(body, "eval").as_deref(),
            Some(r#"{"report_only":true}"#)
        );
    }
}
