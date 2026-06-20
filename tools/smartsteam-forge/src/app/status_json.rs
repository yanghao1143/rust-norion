pub(super) fn scalar_value(object: &str, field: &str) -> String {
    json_number_field(object, field)
        .or_else(|| json_bool_field(object, field).map(|value| bool_value_text(value).to_owned()))
        .or_else(|| json_string_field(object, field))
        .unwrap_or_else(|| "unknown".to_owned())
}

pub(super) fn bool_value_text(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

pub(super) fn require_json_string_equals(
    object: &str,
    field: &str,
    expected: &str,
    label: &str,
) -> Result<(), String> {
    match json_string_field(object, field) {
        Some(value) if value == expected => Ok(()),
        Some(value) => Err(format!("{label} expected {expected:?}, got {value:?}")),
        None => Err(format!("{label} missing {field}")),
    }
}

pub(super) fn require_json_bool_equals(
    object: &str,
    field: &str,
    expected: bool,
    label: &str,
) -> Result<(), String> {
    match json_bool_field(object, field) {
        Some(value) if value == expected => Ok(()),
        Some(value) => Err(format!(
            "{label} expected {}, got {}",
            bool_value_text(expected),
            bool_value_text(value)
        )),
        None => Err(format!("{label} missing {field}")),
    }
}

pub(super) fn required_json_string(
    object: &str,
    field: &str,
    label: &str,
) -> Result<String, String> {
    json_string_field(object, field)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{label} missing {field}"))
}

#[cfg(test)]
pub(super) fn required_json_number(
    object: &str,
    field: &str,
    label: &str,
) -> Result<String, String> {
    json_number_field(object, field)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{label} missing {field}"))
}

pub(super) fn json_string_literal(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            character if character.is_control() => {
                out.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => out.push(character),
        }
    }
    out.push('"');
    out
}

pub(super) fn compact_line(value: &str, limit: usize) -> String {
    let mut out = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if out.len() > limit {
        out.truncate(limit.saturating_sub(3));
        out.push_str("...");
    }
    out
}

#[cfg(test)]
pub(super) fn validate_error_kind(error_kind: &str, label: &str) -> Result<(), String> {
    match error_kind {
        "contract" | "provider" => Ok(()),
        _ => Err(format!("{label} unknown error_kind {error_kind:?}")),
    }
}

#[cfg(test)]
pub(super) fn validate_error_user_message(
    surface: &str,
    error_kind: &str,
    error: &str,
    user_message: &str,
) -> Result<(), String> {
    let expected = format!("{surface} {error_kind} error: {error}");
    if user_message == expected {
        return Ok(());
    }
    Err(format!(
        "{surface} error JSON user_message drift: expected {expected:?}, got {user_message:?}"
    ))
}

pub(super) fn json_string_field(body: &str, field: &str) -> Option<String> {
    let value = json_field_value_start(body, field)?.trim_start();
    let parsed = parse_json_string(value)?;
    let consumed = json_string_literal_len(value)?;
    json_tail_is_delimited(value.get(consumed..)?.trim_start()).then_some(parsed)
}

pub(super) fn json_top_level_string_field(body: &str, field: &str) -> Option<String> {
    let value = json_top_level_value_start(body, field)?.trim_start();
    let parsed = parse_json_string(value)?;
    let consumed = json_string_literal_len(value)?;
    json_tail_is_delimited(value.get(consumed..)?.trim_start()).then_some(parsed)
}

pub(super) fn json_string_array_field(body: &str, field: &str) -> Option<Vec<String>> {
    parse_json_string_array_items(json_array_field(body, field)?)
}

pub(super) fn json_object_array_field<'a>(body: &'a str, field: &str) -> Option<Vec<&'a str>> {
    parse_json_object_array_items(json_array_field(body, field)?)
}

pub(super) fn json_top_level_string_array_field(body: &str, field: &str) -> Option<Vec<String>> {
    parse_json_string_array_items(json_top_level_array_field(body, field)?)
}

fn parse_json_object_array_items(mut input: &str) -> Option<Vec<&str>> {
    input = input.trim_start();
    let mut values = Vec::new();

    loop {
        input = input.trim_start();
        if input.is_empty() {
            return Some(values);
        }
        if !input.starts_with('{') {
            return None;
        }
        let close = find_matching_json_close(input, '{', '}')?;
        values.push(input.get(..=close)?);
        input = input.get(close + 1..)?.trim_start();
        if input.is_empty() {
            return Some(values);
        }
        input = input.strip_prefix(',')?;
    }
}

fn parse_json_string_array_items(mut input: &str) -> Option<Vec<String>> {
    input = input.trim_start();
    let mut values = Vec::new();

    loop {
        input = input.trim_start();
        if input.is_empty() {
            return Some(values);
        }
        let value = parse_json_string(input)?;
        let consumed = json_string_literal_len(input)?;
        values.push(value);
        input = input.get(consumed..)?.trim_start();
        if input.is_empty() {
            return Some(values);
        }
        input = input.strip_prefix(',')?;
    }
}

fn json_array_field<'a>(body: &'a str, field: &str) -> Option<&'a str> {
    let value = json_field_value_start(body, field)?.trim_start();
    if !value.starts_with('[') {
        return None;
    }
    let close = find_matching_json_close(value, '[', ']')?;
    json_tail_is_delimited(value.get(close + 1..)?.trim_start()).then(|| value.get(1..close))?
}

fn json_top_level_array_field<'a>(body: &'a str, field: &str) -> Option<&'a str> {
    let value = json_top_level_value_start(body, field)?.trim_start();
    if !value.starts_with('[') {
        return None;
    }
    let close = find_matching_json_close(value, '[', ']')?;
    json_tail_is_delimited(value.get(close + 1..)?.trim_start()).then(|| value.get(1..close))?
}

pub(super) fn json_bool_field(body: &str, field: &str) -> Option<bool> {
    let value = json_field_value_start(body, field)?.trim_start();
    parse_json_bool(value)
}

pub(super) fn json_top_level_bool_field(body: &str, field: &str) -> Option<bool> {
    let value = json_top_level_value_start(body, field)?.trim_start();
    parse_json_bool(value)
}

fn parse_json_bool(value: &str) -> Option<bool> {
    if json_literal_is_delimited(value, "true") {
        Some(true)
    } else if json_literal_is_delimited(value, "false") {
        Some(false)
    } else {
        None
    }
}

pub(super) fn json_null_field(body: &str, field: &str) -> Option<()> {
    let value = json_field_value_start(body, field)?.trim_start();
    json_literal_is_delimited(value, "null").then_some(())
}

pub(super) fn json_number_field(body: &str, field: &str) -> Option<String> {
    let value = json_field_value_start(body, field)?.trim_start();
    parse_json_number(value)
}

pub(super) fn json_top_level_number_field(body: &str, field: &str) -> Option<String> {
    let value = json_top_level_value_start(body, field)?.trim_start();
    parse_json_number(value)
}

fn parse_json_number(value: &str) -> Option<String> {
    let consumed = json_number_literal_len(value)?;
    json_tail_is_delimited(value.get(consumed..)?.trim_start())
        .then(|| value.get(..consumed).unwrap_or_default().to_owned())
}

pub(super) fn json_object_field<'a>(body: &'a str, field: &str) -> Option<&'a str> {
    let value = json_field_value_start(body, field)?.trim_start();
    if !value.starts_with('{') {
        return None;
    }
    let close = find_matching_json_close(value, '{', '}')?;
    json_tail_is_delimited(value.get(close + 1..)?.trim_start()).then(|| value.get(..=close))?
}

pub(super) fn json_top_level_object_field<'a>(body: &'a str, field: &str) -> Option<&'a str> {
    let value = json_top_level_value_start(body, field)?.trim_start();
    if !value.starts_with('{') {
        return None;
    }
    let close = find_matching_json_close(value, '{', '}')?;
    json_tail_is_delimited(value.get(close + 1..)?.trim_start()).then(|| value.get(..=close))?
}

fn json_top_level_value_start<'a>(body: &'a str, field: &str) -> Option<&'a str> {
    let input = body.trim_start();
    if !input.starts_with('{') {
        return None;
    }
    let mut index = 1usize;

    loop {
        index = skip_json_whitespace(input, index);
        match input.as_bytes().get(index)? {
            b'}' => return None,
            b'"' => {}
            _ => return None,
        }

        let key_input = input.get(index..)?;
        let key = parse_json_string(key_input)?;
        let key_len = json_string_literal_len(key_input)?;
        let mut value_start = skip_json_whitespace(input, index + key_len);
        if input.as_bytes().get(value_start) != Some(&b':') {
            return None;
        }
        value_start = skip_json_whitespace(input, value_start + 1);
        if key == field {
            return input.get(value_start..);
        }

        let value_len = skip_json_value_len(input.get(value_start..)?)?;
        index = skip_json_whitespace(input, value_start + value_len);
        match input.as_bytes().get(index)? {
            b',' => index += 1,
            b'}' => return None,
            _ => return None,
        }
    }
}

pub(super) fn json_object_keys(object: &str) -> Vec<String> {
    let input = object.trim_start();
    if !input.starts_with('{') {
        return Vec::new();
    }
    let mut index = 1usize;
    let mut keys = Vec::new();

    loop {
        index = skip_json_whitespace(input, index);
        match input.as_bytes().get(index) {
            Some(b'}') | None => return keys,
            Some(b'"') => {}
            _ => return Vec::new(),
        }

        let key_input = match input.get(index..) {
            Some(value) => value,
            None => return Vec::new(),
        };
        let Some(key) = parse_json_string(key_input) else {
            return Vec::new();
        };
        let Some(key_len) = json_string_literal_len(key_input) else {
            return Vec::new();
        };
        let mut value_start = skip_json_whitespace(input, index + key_len);
        if input.as_bytes().get(value_start) != Some(&b':') {
            return Vec::new();
        }
        value_start = skip_json_whitespace(input, value_start + 1);
        let Some(value) = input.get(value_start..) else {
            return Vec::new();
        };
        let Some(value_len) = skip_json_value_len(value) else {
            return Vec::new();
        };
        keys.push(key);
        index = skip_json_whitespace(input, value_start + value_len);
        match input.as_bytes().get(index) {
            Some(b',') => index += 1,
            Some(b'}') | None => return keys,
            _ => return Vec::new(),
        }
    }
}

fn json_field_value_start<'a>(body: &'a str, field: &str) -> Option<&'a str> {
    let mut index = 0usize;
    while index < body.len() {
        let key_start = body.get(index..)?.find('"')? + index;
        let candidate = body.get(key_start..)?;
        let key = parse_json_string(candidate)?;
        let key_len = json_string_literal_len(candidate)?;
        let after_key = candidate.get(key_len..)?.trim_start();
        if key == field {
            return after_key.strip_prefix(':');
        }
        index = key_start + key_len;
    }
    None
}

fn skip_json_whitespace(input: &str, mut index: usize) -> usize {
    while matches!(
        input.as_bytes().get(index),
        Some(b' ' | b'\n' | b'\r' | b'\t')
    ) {
        index += 1;
    }
    index
}

fn skip_json_value_len(input: &str) -> Option<usize> {
    let input = input.trim_start();
    if input.starts_with('{') {
        return find_matching_json_close(input, '{', '}').map(|index| index + 1);
    }
    if input.starts_with('[') {
        return find_matching_json_close(input, '[', ']').map(|index| index + 1);
    }
    if input.starts_with('"') {
        return json_string_literal_len(input);
    }
    input
        .char_indices()
        .find_map(|(index, character)| matches!(character, ',' | '}' | ']').then_some(index))
}

fn parse_json_string(input: &str) -> Option<String> {
    let mut chars = input.chars();
    if chars.next()? != '"' {
        return None;
    }
    let mut out = String::new();
    let mut escaped = false;
    while let Some(character) = chars.next() {
        if escaped {
            match character {
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                '/' => out.push('/'),
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                'b' => out.push('\u{0008}'),
                'f' => out.push('\u{000c}'),
                'u' => push_json_unicode_escape(&mut chars, &mut out)?,
                _ => return None,
            }
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '"' {
            return Some(out);
        } else if character.is_control() {
            return None;
        } else {
            out.push(character);
        }
    }
    None
}

fn push_json_unicode_escape(chars: &mut std::str::Chars<'_>, out: &mut String) -> Option<()> {
    let code = read_json_hex_escape(chars)?;
    if (0xd800..=0xdbff).contains(&code) {
        if chars.next()? != '\\' || chars.next()? != 'u' {
            return None;
        }
        let low = read_json_hex_escape(chars)?;
        if !(0xdc00..=0xdfff).contains(&low) {
            return None;
        }
        let high_ten = u32::from(code) - 0xd800;
        let low_ten = u32::from(low) - 0xdc00;
        out.push(char::from_u32(0x1_0000 + ((high_ten << 10) | low_ten))?);
    } else if (0xdc00..=0xdfff).contains(&code) {
        return None;
    } else {
        out.push(char::from_u32(u32::from(code))?);
    }
    Some(())
}

fn read_json_hex_escape(chars: &mut std::str::Chars<'_>) -> Option<u16> {
    let mut value = 0_u16;
    for _ in 0..4 {
        let digit = chars.next()?.to_digit(16)?;
        value = (value << 4) | u16::try_from(digit).ok()?;
    }
    Some(value)
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

fn json_number_literal_len(input: &str) -> Option<usize> {
    let bytes = input.as_bytes();
    let mut index = 0usize;
    if bytes.get(index) == Some(&b'-') {
        index += 1;
    }
    match bytes.get(index)? {
        b'0' => index += 1,
        b'1'..=b'9' => {
            index += 1;
            while matches!(bytes.get(index), Some(b'0'..=b'9')) {
                index += 1;
            }
        }
        _ => return None,
    }
    if bytes.get(index) == Some(&b'.') {
        index += 1;
        let start = index;
        while matches!(bytes.get(index), Some(b'0'..=b'9')) {
            index += 1;
        }
        if index == start {
            return None;
        }
    }
    if matches!(bytes.get(index), Some(b'e' | b'E')) {
        index += 1;
        if matches!(bytes.get(index), Some(b'+' | b'-')) {
            index += 1;
        }
        let start = index;
        while matches!(bytes.get(index), Some(b'0'..=b'9')) {
            index += 1;
        }
        if index == start {
            return None;
        }
    }
    Some(index)
}

fn json_literal_is_delimited(input: &str, literal: &str) -> bool {
    input
        .strip_prefix(literal)
        .map(str::trim_start)
        .is_some_and(json_tail_is_delimited)
}

fn json_tail_is_delimited(tail: &str) -> bool {
    tail.is_empty() || matches!(tail.as_bytes().first(), Some(b',' | b'}' | b']'))
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

fn skip_json_string_literal(
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
) -> Option<()> {
    let mut escaped = false;
    while let Some((_, character)) = chars.next() {
        if escaped {
            if character == 'u' {
                skip_json_unicode_escape(chars)?;
            }
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '"' {
            return Some(());
        } else if character.is_control() {
            return None;
        }
    }
    None
}

fn skip_json_unicode_escape(
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
) -> Option<()> {
    for _ in 0..4 {
        chars.next()?.1.to_digit(16)?;
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_contract_helpers_accept_expected_scalar_fields() {
        let object = "{\"schema\":\"demo.v1\",\"ok\":true,\"name\":\"worker\",\"count\":3,\"ratio\":1.25e-3}";

        require_json_string_equals(object, "schema", "demo.v1", "demo schema").unwrap();
        require_json_bool_equals(object, "ok", true, "demo ok").unwrap();
        assert_eq!(
            required_json_string(object, "name", "demo name").unwrap(),
            "worker"
        );
        assert_eq!(
            required_json_number(object, "count", "demo count").unwrap(),
            "3"
        );
        assert_eq!(
            required_json_number(object, "ratio", "demo ratio").unwrap(),
            "1.25e-3"
        );
    }

    #[test]
    fn json_top_level_fields_ignore_nested_fields() {
        let object = r#"{
            "nested": {
                "next_step": "legacy nested value",
                "ok": false,
                "block_reasons": ["nested"],
                "count": 1
            },
            "next_step": "top level value",
            "ok": true,
            "block_reasons": ["top"],
            "count": 2
        }"#;

        assert_eq!(
            json_string_field(object, "next_step").as_deref(),
            Some("legacy nested value")
        );
        assert_eq!(
            json_top_level_string_field(object, "next_step").as_deref(),
            Some("top level value")
        );
        assert_eq!(json_bool_field(object, "ok"), Some(false));
        assert_eq!(json_top_level_bool_field(object, "ok"), Some(true));
        assert_eq!(
            json_string_array_field(object, "block_reasons"),
            Some(vec!["nested".to_owned()])
        );
        assert_eq!(
            json_top_level_string_array_field(object, "block_reasons"),
            Some(vec!["top".to_owned()])
        );
        assert_eq!(json_number_field(object, "count").as_deref(), Some("1"));
        assert_eq!(
            json_top_level_number_field(object, "count").as_deref(),
            Some("2")
        );
    }

    #[test]
    fn json_object_array_field_reads_top_level_objects() {
        let object = r#"{
            "rows": [
                {"id": "one", "nested": {"ok": true}},
                {"id": "two", "items": ["a", "b"]}
            ]
        }"#;

        let rows = json_object_array_field(object, "rows").unwrap();

        assert_eq!(rows.len(), 2);
        assert_eq!(json_string_field(rows[0], "id").as_deref(), Some("one"));
        assert_eq!(json_string_field(rows[1], "id").as_deref(), Some("two"));
        assert_eq!(
            json_string_array_field(rows[1], "items"),
            Some(vec!["a".to_owned(), "b".to_owned()])
        );
    }

    #[test]
    fn json_number_field_accepts_exponent_forms_and_rejects_malformed_exponents() {
        let object = "{\"small\":1e-3,\"large\":6E+2,\"negative\":-4.2e1,\"bad\":1e}";

        assert_eq!(json_number_field(object, "small").as_deref(), Some("1e-3"));
        assert_eq!(json_number_field(object, "large").as_deref(), Some("6E+2"));
        assert_eq!(
            json_number_field(object, "negative").as_deref(),
            Some("-4.2e1")
        );
        assert_eq!(json_number_field(object, "bad"), None);
    }

    #[test]
    fn json_contract_helpers_report_mismatch_and_missing_fields() {
        let object = "{\"schema\":\"demo.v2\",\"ok\":false,\"name\":\"\"}";

        assert!(
            require_json_string_equals(object, "schema", "demo.v1", "demo schema")
                .unwrap_err()
                .contains("expected \"demo.v1\"")
        );
        assert!(
            require_json_bool_equals(object, "ok", true, "demo ok")
                .unwrap_err()
                .contains("expected true, got false")
        );
        assert!(
            required_json_string(object, "name", "demo name")
                .unwrap_err()
                .contains("demo name missing name")
        );
        assert!(
            required_json_number(object, "count", "demo count")
                .unwrap_err()
                .contains("demo count missing count")
        );
    }
}
