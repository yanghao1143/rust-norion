use std::io::Write;
use std::net::TcpStream;
use std::path::PathBuf;

use rust_norion::MemoryUpdateReport;

pub(crate) fn json_string_field(body: &str, field: &str) -> Option<String> {
    let needle = format!("\"{field}\"");
    let after_field = body.get(body.find(&needle)? + needle.len()..)?;
    let after_colon = after_field.get(after_field.find(':')? + 1..)?;
    let trimmed = after_colon.trim_start();
    parse_json_string(trimmed)
}

pub(crate) fn json_bool_field(body: &str, field: &str) -> Option<bool> {
    let value = json_value_after_colon(body, field)?;
    if let Some(text) = parse_json_string(value) {
        return match text.trim().to_ascii_lowercase().as_str() {
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

pub(crate) fn json_usize_field(body: &str, field: &str) -> Option<usize> {
    let value = json_value_after_colon(body, field)?;
    if let Some(text) = parse_json_string(value) {
        return text.trim().parse::<usize>().ok();
    }
    let digits = value
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect::<String>();
    if digits.is_empty() {
        None
    } else {
        digits.parse::<usize>().ok()
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
    if digits.is_empty() {
        None
    } else {
        digits.parse::<u64>().ok()
    }
}

pub(crate) fn json_u64_array_field(body: &str, field: &str) -> Option<Vec<u64>> {
    let value = json_value_after_colon(body, field)?;
    let inner = value.trim_start().strip_prefix('[')?;
    let end = inner.find(']')?;
    let items = inner.get(..end)?.trim();
    if items.is_empty() {
        return Some(Vec::new());
    }
    items
        .split(',')
        .map(|item| item.trim().parse::<u64>().ok())
        .collect()
}

pub(crate) fn json_string_array_field(body: &str, field: &str) -> Option<Vec<String>> {
    let mut input = json_value_after_colon(body, field)?
        .trim_start()
        .strip_prefix('[')?;
    let mut values = Vec::new();
    loop {
        input = input.trim_start();
        if input.starts_with(']') {
            return Some(values);
        }
        let value = parse_json_string(input)?;
        let literal_len = json_string_literal_len(input)?;
        values.push(value);
        input = input.get(literal_len..)?.trim_start();
        match input.chars().next()? {
            ',' => input = input.get(1..)?,
            ']' => return Some(values),
            _ => return None,
        }
    }
}

pub(crate) fn json_object_field(body: &str, field: &str) -> Option<String> {
    parse_json_object(json_value_after_colon(body, field)?).map(ToOwned::to_owned)
}

pub(crate) fn json_object_array_field(body: &str, field: &str) -> Option<Vec<String>> {
    parse_json_object_array(json_value_after_colon(body, field)?).map(|objects| {
        objects
            .into_iter()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>()
    })
}

pub(crate) fn json_f32_field(body: &str, field: &str) -> Option<f32> {
    let value = json_value_after_colon(body, field)?;
    if let Some(text) = parse_json_string(value) {
        return text.trim().parse::<f32>().ok();
    }
    let number = value
        .chars()
        .take_while(|character| {
            character.is_ascii_digit()
                || *character == '.'
                || *character == '-'
                || *character == '+'
                || *character == 'e'
                || *character == 'E'
        })
        .collect::<String>();
    if number.is_empty() {
        None
    } else {
        number.parse::<f32>().ok().filter(|value| value.is_finite())
    }
}

pub(crate) fn service_error_json(message: &str) -> String {
    format!(
        "{{\"ok\":false,\"error\":{}}}",
        service_json_string(message)
    )
}

pub(crate) fn write_http_json(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    body: &str,
) -> std::io::Result<()> {
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json; charset=utf-8\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes())
}

pub(crate) fn write_http_sse_headers(stream: &mut TcpStream) -> std::io::Result<()> {
    stream.write_all(
        b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream; charset=utf-8\r\ncache-control: no-cache\r\nconnection: close\r\n\r\n",
    )?;
    stream.flush()
}

pub(crate) fn write_sse_event(
    stream: &mut TcpStream,
    event: &str,
    data: &str,
) -> std::io::Result<()> {
    stream.write_all(format!("event: {event}\n").as_bytes())?;
    for line in data.lines() {
        stream.write_all(format!("data: {line}\n").as_bytes())?;
    }
    if data.is_empty() {
        stream.write_all(b"data: \n")?;
    }
    stream.write_all(b"\n")?;
    stream.flush()
}

pub(crate) fn service_json_string(value: &str) -> String {
    let mut escaped = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            other if other.is_control() => {
                escaped.push_str(&format!("\\u{:04x}", other as u32));
            }
            other => escaped.push(other),
        }
    }
    escaped.push('"');
    escaped
}

pub(crate) fn service_json_string_array(values: &[String]) -> String {
    let items = values
        .iter()
        .map(|value| service_json_string(value))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

pub(crate) fn service_u64_array(values: &[u64]) -> String {
    let items = values
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

pub(crate) fn service_memory_update_array(values: &[MemoryUpdateReport]) -> String {
    let items = values
        .iter()
        .map(|update| {
            format!(
                "{{\"id\":{},\"action\":\"{}\",\"requested_amount\":{:.6},\"applied\":{},\"removed\":{},\"strength_before\":{},\"strength_after\":{},\"strength_delta\":{:.6}}}",
                update.id,
                update.action.as_str(),
                update.requested_amount,
                update.was_applied(),
                update.removed,
                option_f32_service_json(update.strength_before),
                option_f32_service_json(update.strength_after),
                update.strength_delta
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

pub(crate) fn option_f32_service_json(value: Option<f32>) -> String {
    value
        .filter(|value| value.is_finite())
        .map(|value| format!("{value:.6}"))
        .unwrap_or_else(|| "null".to_owned())
}

pub(crate) fn option_str_service_json(value: Option<&str>) -> String {
    value
        .map(service_json_string)
        .unwrap_or_else(|| "null".to_owned())
}

pub(crate) fn option_path_service_json(path: Option<&PathBuf>) -> String {
    path.map(|path| service_json_string(&path.display().to_string()))
        .unwrap_or_else(|| "null".to_owned())
}

pub(crate) fn option_u64_service_json(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

pub(crate) fn option_usize_service_json(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

pub(crate) fn option_i32_service_json(value: Option<i32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

fn json_value_after_colon<'a>(body: &'a str, field: &str) -> Option<&'a str> {
    let needle = format!("\"{field}\"");
    let after_field = body.get(body.find(&needle)? + needle.len()..)?;
    let after_colon = after_field.get(after_field.find(':')? + 1..)?;
    Some(after_colon.trim_start())
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

fn json_string_literal_len(input: &str) -> Option<usize> {
    let mut chars = input.char_indices();
    if chars.next()?.1 != '"' {
        return None;
    }
    let mut escaped = false;
    for (index, character) in chars {
        if escaped {
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '"' {
            return Some(index + character.len_utf8());
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

fn parse_json_object_array(input: &str) -> Option<Vec<&str>> {
    let mut rest = input.trim_start().strip_prefix('[')?;
    let mut objects = Vec::new();
    loop {
        rest = rest.trim_start();
        if rest.starts_with(']') {
            return Some(objects);
        }
        let object = parse_json_object(rest)?;
        objects.push(object);
        rest = rest.get(object.len()..)?.trim_start();
        if let Some(after_comma) = rest.strip_prefix(',') {
            rest = after_comma;
            continue;
        }
        if rest.starts_with(']') {
            return Some(objects);
        }
        return None;
    }
}

#[cfg(test)]
mod tests {
    use super::{json_object_array_field, json_string_array_field};

    #[test]
    fn extracts_object_array_field_with_nested_values() {
        let objects = json_object_array_field(
            "{\"pool_stage_dispatch\":[{\"task_kind\":\"summary\",\"worker\":{\"role\":\"summary\"}},{\"task_kind\":\"review\",\"selected_role\":\"review\"}]}",
            "pool_stage_dispatch",
        )
        .unwrap();

        assert_eq!(objects.len(), 2);
        assert!(objects[0].contains("\"worker\":{\"role\":\"summary\"}"));
        assert!(objects[1].contains("\"task_kind\":\"review\""));
    }

    #[test]
    fn extracts_empty_object_array_field() {
        assert_eq!(
            json_object_array_field("{\"pool_stage_dispatch\":[]}", "pool_stage_dispatch"),
            Some(Vec::new())
        );
    }

    #[test]
    fn extracts_string_array_field() {
        assert_eq!(
            json_string_array_field(
                "{\"completed_roles\":[\"quality\",\"summary\",\"test-gate\"]}",
                "completed_roles"
            ),
            Some(vec![
                "quality".to_owned(),
                "summary".to_owned(),
                "test-gate".to_owned()
            ])
        );
    }
}
