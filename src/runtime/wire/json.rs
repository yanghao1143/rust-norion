pub(super) fn option_u64_json(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

pub(super) fn option_f32_json(value: Option<f32>) -> String {
    value
        .filter(|value| value.is_finite())
        .map(|value| format!("{value:.6}"))
        .unwrap_or_else(|| "null".to_owned())
}

pub(super) fn json_f32_array(values: &[f32]) -> String {
    let values = values
        .iter()
        .filter(|value| value.is_finite())
        .map(|value| format!("{value:.6}"))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{values}]")
}

pub(super) fn json_str_array<'a, I>(items: I) -> String
where
    I: IntoIterator<Item = &'a str>,
{
    let values = items
        .into_iter()
        .map(json_string)
        .collect::<Vec<_>>()
        .join(",");
    format!("[{values}]")
}

pub(in crate::runtime) fn json_string(value: &str) -> String {
    format!("\"{}\"", json_escape(value))
}

fn json_escape(value: &str) -> String {
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

pub(in crate::runtime) fn extract_json_string_field(source: &str, field: &str) -> Option<String> {
    extract_json_field(source, field).and_then(parse_json_string)
}

pub(in crate::runtime) fn extract_json_number_field(source: &str, field: &str) -> Option<f32> {
    extract_json_field(source, field).and_then(|value| value.trim().parse::<f32>().ok())
}

pub(super) fn extract_json_finite_number_field(source: &str, field: &str) -> Option<f32> {
    extract_json_number_field(source, field).filter(|value| value.is_finite())
}

pub(in crate::runtime) fn extract_json_usize_field(source: &str, field: &str) -> Option<usize> {
    extract_json_field(source, field).and_then(|value| value.trim().parse::<usize>().ok())
}

pub(super) fn extract_json_kv_precision_bits(source: &str, field: &str) -> Option<u8> {
    extract_json_field(source, field)
        .and_then(|value| value.trim().parse::<u8>().ok())
        .filter(|value| matches!(value, 4 | 8))
}

pub(super) fn extract_json_f32_array_field(source: &str, field: &str) -> Option<Vec<f32>> {
    let value = extract_json_array_field(source, field)?;
    parse_json_f32_array(value)
}

pub(in crate::runtime) fn extract_json_array_field<'a>(
    source: &'a str,
    field: &str,
) -> Option<&'a str> {
    extract_json_field(source, field).filter(|value| value.trim_start().starts_with('['))
}

pub(super) fn extract_json_array_field_by_value_kind<'a>(
    source: &'a str,
    field: &str,
) -> Option<&'a str> {
    let needle = json_string(field);
    let mut search_start = 0;

    while search_start < source.len() {
        let offset = source[search_start..].find(&needle)?;
        let key_start = search_start + offset;
        let after_key = key_start + needle.len();
        let colon_offset = source[after_key..].find(':')?;
        let mut value_start = after_key + colon_offset + 1;
        while source[value_start..]
            .chars()
            .next()
            .is_some_and(char::is_whitespace)
        {
            value_start += source[value_start..].chars().next()?.len_utf8();
        }
        let value_end = json_value_end(source, value_start)?;
        let value = &source[value_start..value_end];
        if value.trim_start().starts_with('[') {
            return Some(value);
        }
        search_start = after_key;
    }

    None
}

pub(in crate::runtime) fn extract_json_object_field<'a>(
    source: &'a str,
    field: &str,
) -> Option<&'a str> {
    extract_json_field(source, field).filter(|value| value.trim_start().starts_with('{'))
}

fn extract_json_field<'a>(source: &'a str, field: &str) -> Option<&'a str> {
    let needle = json_string(field);
    let key_start = source.find(&needle)?;
    let after_key = key_start + needle.len();
    let colon_offset = source[after_key..].find(':')?;
    let mut value_start = after_key + colon_offset + 1;
    while source[value_start..]
        .chars()
        .next()
        .is_some_and(char::is_whitespace)
    {
        value_start += source[value_start..].chars().next()?.len_utf8();
    }
    let value_end = json_value_end(source, value_start)?;
    Some(&source[value_start..value_end])
}

pub(in crate::runtime) fn split_json_objects(array_value: &str) -> Vec<&str> {
    let trimmed = array_value.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return Vec::new();
    }

    let inner_start = 1;
    let inner_end = trimmed.len().saturating_sub(1);
    let inner = &trimmed[inner_start..inner_end];
    let mut objects = Vec::new();
    let mut index = 0;

    while index < inner.len() {
        while index < inner.len() {
            let Some(ch) = inner[index..].chars().next() else {
                break;
            };
            if ch == ',' || ch.is_whitespace() {
                index += ch.len_utf8();
            } else {
                break;
            }
        }
        if index >= inner.len() {
            break;
        }
        if !inner[index..].starts_with('{') {
            break;
        }
        let Some(end) = json_value_end(inner, index) else {
            break;
        };
        objects.push(&inner[index..end]);
        index = end;
    }

    objects
}

fn parse_json_f32_array(array_value: &str) -> Option<Vec<f32>> {
    let trimmed = array_value.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return None;
    }

    let inner = &trimmed[1..trimmed.len().saturating_sub(1)];
    if inner.trim().is_empty() {
        return Some(Vec::new());
    }

    inner
        .split(',')
        .map(|value| {
            value
                .trim()
                .parse::<f32>()
                .ok()
                .filter(|value| value.is_finite())
        })
        .collect()
}

fn json_value_end(source: &str, start: usize) -> Option<usize> {
    let first = source[start..].chars().next()?;
    match first {
        '"' => scan_json_string_end(source, start),
        '[' => scan_json_compound_end(source, start, '[', ']'),
        '{' => scan_json_compound_end(source, start, '{', '}'),
        _ => {
            let mut end = start;
            while end < source.len() {
                let ch = source[end..].chars().next()?;
                if ch == ',' || ch == '}' || ch == ']' || ch.is_whitespace() {
                    break;
                }
                end += ch.len_utf8();
            }
            Some(end)
        }
    }
}

fn scan_json_string_end(source: &str, start: usize) -> Option<usize> {
    let mut escaped = false;
    let mut index = start + 1;
    while index < source.len() {
        let ch = source[index..].chars().next()?;
        index += ch.len_utf8();
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            return Some(index);
        }
    }
    None
}

fn scan_json_compound_end(source: &str, start: usize, open: char, close: char) -> Option<usize> {
    let mut depth = 0_usize;
    let mut index = start;
    while index < source.len() {
        let ch = source[index..].chars().next()?;
        if ch == '"' {
            index = scan_json_string_end(source, index)?;
            continue;
        }
        if ch == open {
            depth += 1;
        } else if ch == close {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some(index + ch.len_utf8());
            }
        }
        index += ch.len_utf8();
    }
    None
}

fn parse_json_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if !trimmed.starts_with('"') || !trimmed.ends_with('"') {
        return None;
    }

    let mut out = String::new();
    let mut chars = trimmed[1..trimmed.len().saturating_sub(1)].chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next()? {
            '"' => out.push('"'),
            '\\' => out.push('\\'),
            '/' => out.push('/'),
            'b' => out.push('\u{0008}'),
            'f' => out.push('\u{000c}'),
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            't' => out.push('\t'),
            'u' => {
                let code = (0..4).filter_map(|_| chars.next()).collect::<String>();
                let value = u32::from_str_radix(&code, 16).ok()?;
                out.push(char::from_u32(value)?);
            }
            other => out.push(other),
        }
    }
    Some(out)
}
