pub(super) fn has_non_empty_trace_text(value: &str) -> bool {
    !value.trim().is_empty()
}

pub(super) fn require_contract_string(
    failures: &mut Vec<String>,
    contract: &str,
    key: &str,
    expected: Option<String>,
) {
    let Some(expected) = expected else {
        return;
    };
    match contract_value(contract, key) {
        Some(actual) if actual == expected => {}
        Some(actual) => failures.push(format!(
            "runtime_device_contract {key}={actual} does not match trace value {expected}"
        )),
        None => failures.push(format!("runtime_device_contract missing {key}")),
    }
}

pub(super) fn require_contract_usize(
    failures: &mut Vec<String>,
    contract: &str,
    key: &str,
    expected: Option<usize>,
) {
    require_contract_string(
        failures,
        contract,
        key,
        expected.map(|value| value.to_string()),
    );
}

pub(super) fn contract_value<'a>(contract: &'a str, key: &str) -> Option<&'a str> {
    let prefix = format!("{key}=");
    contract
        .split_whitespace()
        .find_map(|part| part.strip_prefix(&prefix))
}

pub(super) fn split_contract_adapters(value: &str) -> Vec<String> {
    value
        .split('+')
        .filter(|item| !item.trim().is_empty())
        .map(|item| item.trim().to_owned())
        .collect()
}

pub(super) fn extract_json_string_field(line: &str, field: &str) -> Option<String> {
    let value = value_after_json_field(line, field)?;
    parse_json_string(value).map(|(parsed, _)| parsed)
}

pub(super) fn extract_json_nullable_string_field(line: &str, field: &str) -> Option<String> {
    let value = value_after_json_field(line, field)?;
    if value.starts_with("null") {
        return None;
    }
    parse_json_string(value).map(|(parsed, _)| parsed)
}

pub(super) fn extract_json_usize_field(line: &str, field: &str) -> Option<usize> {
    let value = value_after_json_field(line, field)?;
    let digits = value
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    if digits.is_empty() {
        return None;
    }
    digits.parse().ok()
}

pub(super) fn extract_json_nullable_u64_field(line: &str, field: &str) -> Option<u64> {
    let value = value_after_json_field(line, field)?;
    if value.starts_with("null") {
        return None;
    }
    let digits = value
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    if digits.is_empty() {
        return None;
    }
    digits.parse().ok()
}

pub(super) fn extract_json_nullable_f32_field(line: &str, field: &str) -> Option<f32> {
    let value = value_after_json_field(line, field)?;
    if value.starts_with("null") {
        return None;
    }
    let number = value
        .chars()
        .take_while(|ch| ch.is_ascii_digit() || matches!(ch, '-' | '+' | '.' | 'e' | 'E'))
        .collect::<String>();
    if number.is_empty() {
        return None;
    }
    number.parse::<f32>().ok().filter(|value| value.is_finite())
}

pub(super) fn extract_json_f32_field(line: &str, field: &str) -> Option<f32> {
    extract_json_nullable_f32_field(line, field)
}

pub(super) fn extract_json_bool_field(line: &str, field: &str) -> Option<bool> {
    let value = value_after_json_field(line, field)?;
    if value.starts_with("true") {
        Some(true)
    } else if value.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

pub(super) fn extract_json_string_array_field(line: &str, field: &str) -> Option<Vec<String>> {
    parse_json_string_array(value_after_json_field(line, field)?)
}

pub(super) fn extract_last_json_string_array_field(line: &str, field: &str) -> Option<Vec<String>> {
    let marker = format!("\"{field}\":");
    let start = line.rfind(&marker)? + marker.len();
    parse_json_string_array(line[start..].trim_start())
}

fn parse_json_string_array(mut value: &str) -> Option<Vec<String>> {
    value = value.strip_prefix('[')?.trim_start();
    let mut out = Vec::new();

    loop {
        if let Some(rest) = value.strip_prefix(']') {
            let _ = rest;
            return Some(out);
        }

        let (item, consumed) = parse_json_string(value)?;
        out.push(item);
        value = value[consumed..].trim_start();

        if let Some(rest) = value.strip_prefix(',') {
            value = rest.trim_start();
        } else if value.starts_with(']') {
            continue;
        } else {
            return None;
        }
    }
}

pub(super) fn extract_json_u64_array_field(line: &str, field: &str) -> Option<Vec<u64>> {
    let mut value = value_after_json_field(line, field)?;
    value = value.strip_prefix('[')?.trim_start();
    let mut out = Vec::new();

    loop {
        value = value.trim_start();
        if let Some(rest) = value.strip_prefix(']') {
            let _ = rest;
            return Some(out);
        }

        let digits = value
            .chars()
            .take_while(|character| character.is_ascii_digit())
            .collect::<String>();
        if digits.is_empty() {
            return None;
        }
        out.push(digits.parse().ok()?);
        value = &value[digits.len()..];
        value = value.trim_start();

        if let Some(rest) = value.strip_prefix(',') {
            value = rest;
        } else if value.starts_with(']') {
            continue;
        } else {
            return None;
        }
    }
}

pub(super) fn value_after_json_field<'a>(line: &'a str, field: &str) -> Option<&'a str> {
    let marker = format!("\"{field}\":");
    let start = line.find(&marker)? + marker.len();
    Some(line[start..].trim_start())
}

pub(super) fn json_object_after_field<'a>(line: &'a str, field: &str) -> Option<&'a str> {
    let value = value_after_json_field(line, field)?;
    if !value.starts_with('{') {
        return None;
    }

    let mut depth = 0usize;
    let mut inner_start = None;
    let mut in_string = false;
    let mut escaped = false;

    for (index, ch) in value.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            match ch {
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '{' => {
                if depth == 0 {
                    inner_start = Some(index + ch.len_utf8());
                }
                depth = depth.saturating_add(1);
            }
            '}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return inner_start.map(|start| &value[start..index]);
                }
            }
            _ => {}
        }
    }

    None
}

fn parse_json_string(value: &str) -> Option<(String, usize)> {
    let mut chars = value.char_indices();
    let (_, first) = chars.next()?;
    if first != '"' {
        return None;
    }

    let mut out = String::new();
    let mut escaped = false;
    for (index, ch) in chars {
        if escaped {
            match ch {
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                '/' => out.push('/'),
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                'b' => out.push('\u{0008}'),
                'f' => out.push('\u{000c}'),
                'u' => out.push_str("\\u"),
                other => out.push(other),
            }
            escaped = false;
            continue;
        }

        match ch {
            '\\' => escaped = true,
            '"' => return Some((out, index + ch.len_utf8())),
            other => out.push(other),
        }
    }

    None
}

pub(super) fn trace_note_f32(note: &str, key: &str) -> Option<f32> {
    note.split(':')
        .find_map(|part| part.strip_prefix(key))
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite())
}

pub(super) fn trace_note_bool(note: &str, key: &str) -> Option<bool> {
    match note.split(':').find_map(|part| part.strip_prefix(key))? {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

pub(super) fn json_escape(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out
}
