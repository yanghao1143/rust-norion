pub(crate) fn json_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

pub(crate) fn json_string_field(body: &str, field: &str) -> Option<String> {
    let value = json_field_value_start(body, field)?.trim_start();
    let parsed = parse_json_string(value)?;
    let literal_len = json_string_literal_len(value)?;
    let trailing = value.get(literal_len..)?.trim_start();
    if !json_value_tail_is_delimited(trailing) {
        return None;
    }
    Some(parsed)
}

pub(crate) fn json_bool_field(body: &str, field: &str) -> Option<bool> {
    let value = json_field_value_start(body, field)?.trim_start();
    if json_literal_is_delimited(value, "true") {
        Some(true)
    } else if json_literal_is_delimited(value, "false") {
        Some(false)
    } else {
        None
    }
}

pub(crate) fn json_number_field(body: &str, field: &str) -> Option<String> {
    let value = json_field_value_start(body, field)?.trim_start();
    let number_len = json_number_literal_len(value)?;
    let trailing = value.get(number_len..)?.trim_start();
    if !trailing.is_empty() && !matches!(trailing.as_bytes().first(), Some(b',' | b'}' | b']')) {
        return None;
    }
    value.get(..number_len).map(ToOwned::to_owned)
}

pub(crate) fn json_string_array_field(body: &str, field: &str) -> Option<Vec<String>> {
    let mut input = json_field_value_start(body, field)?
        .trim_start()
        .strip_prefix('[')?;
    let mut values = Vec::new();

    loop {
        input = input.trim_start();
        if input.starts_with(']') {
            return json_value_tail_is_delimited(input.get(1..)?.trim_start()).then_some(values);
        }
        let value = parse_json_string(input)?;
        let consumed = json_string_literal_len(input)?;
        values.push(value);
        input = input.get(consumed..)?.trim_start();
        match input.chars().next()? {
            ',' => input = input.get(1..)?,
            ']' => {
                return json_value_tail_is_delimited(input.get(1..)?.trim_start())
                    .then_some(values);
            }
            _ => return None,
        }
    }
}

pub(crate) fn json_array_field<'a>(body: &'a str, field: &str) -> Option<&'a str> {
    let trimmed = json_field_value_start(body, field)?.trim_start();
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

pub(crate) fn json_object_field<'a>(body: &'a str, field: &str) -> Option<&'a str> {
    let trimmed = json_field_value_start(body, field)?.trim_start();
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

pub(crate) fn json_object_items(input: &str) -> Vec<&str> {
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

fn json_field_value_start<'a>(body: &'a str, field: &str) -> Option<&'a str> {
    let mut index = 0usize;
    while index < body.len() {
        let candidate_start = body.get(index..)?.find('"')? + index;
        let candidate = body.get(candidate_start..)?;
        let key = parse_json_string(candidate)?;
        let literal_len = json_string_literal_len(candidate)?;
        if key == field {
            let after_key = candidate.get(literal_len..)?.trim_start();
            if let Some(after_colon) = after_key.strip_prefix(':') {
                return Some(after_colon);
            }
        }
        index = candidate_start + literal_len;
    }
    None
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
        let scalar = 0x1_0000 + ((high_ten << 10) | low_ten);
        out.push(char::from_u32(scalar)?);
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
        let fraction_start = index;
        while matches!(bytes.get(index), Some(b'0'..=b'9')) {
            index += 1;
        }
        if index == fraction_start {
            return None;
        }
    }

    if matches!(bytes.get(index), Some(b'e' | b'E')) {
        index += 1;
        if matches!(bytes.get(index), Some(b'+' | b'-')) {
            index += 1;
        }
        let exponent_start = index;
        while matches!(bytes.get(index), Some(b'0'..=b'9')) {
            index += 1;
        }
        if index == exponent_start {
            return None;
        }
    }

    Some(index)
}

fn json_literal_is_delimited(input: &str, literal: &str) -> bool {
    let Some(trailing) = input.strip_prefix(literal).map(str::trim_start) else {
        return false;
    };
    json_value_tail_is_delimited(trailing)
}

fn json_value_tail_is_delimited(trailing: &str) -> bool {
    trailing.is_empty() || matches!(trailing.as_bytes().first(), Some(b',' | b'}' | b']'))
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
    fn escapes_json_string_content() {
        assert_eq!(json_string("a\"b\\c\n"), "\"a\\\"b\\\\c\\n\"");
    }

    #[test]
    fn extracts_simple_json_string_field() {
        assert_eq!(
            json_string_field("{\"answer\":\"你好\\nSteam Forge\"}", "answer").as_deref(),
            Some("你好\nSteam Forge")
        );
    }

    #[test]
    fn decodes_unicode_json_string_field() {
        assert_eq!(
            json_string_field("{\"answer\":\"\\u4f60\\u597d\"}", "answer").as_deref(),
            Some("\u{4f60}\u{597d}")
        );
    }

    #[test]
    fn decodes_unicode_surrogate_pair_json_string_field() {
        assert_eq!(
            json_string_field("{\"answer\":\"forge \\ud83e\\udd16\"}", "answer").as_deref(),
            Some("forge \u{1f916}")
        );
    }

    #[test]
    fn rejects_unpaired_unicode_surrogate_json_string_field() {
        assert_eq!(
            json_string_field("{\"answer\":\"\\ud83e\"}", "answer"),
            None
        );
    }

    #[test]
    fn rejects_invalid_json_string_escape_field() {
        assert_eq!(
            json_string_field("{\"answer\":\"bad \\q escape\"}", "answer"),
            None
        );
    }

    #[test]
    fn rejects_unescaped_json_string_control_characters() {
        assert_eq!(
            json_string_field("{\"answer\":\"bad\nnewline\"}", "answer"),
            None
        );
    }

    #[test]
    fn rejects_trailing_garbage_after_json_string_field() {
        assert_eq!(json_string_field("{\"answer\":\"ok\"x}", "answer"), None);
    }

    #[test]
    fn extracts_bool_and_number_fields() {
        let body = "{\"ok\":true,\"count\":42,\"ratio\":0.5}";

        assert_eq!(json_bool_field(body, "ok"), Some(true));
        assert_eq!(json_number_field(body, "count").as_deref(), Some("42"));
        assert_eq!(json_number_field(body, "ratio").as_deref(), Some("0.5"));
    }

    #[test]
    fn extracts_signed_and_exponent_json_number_fields() {
        let body = "{\"negative\":-0.125,\"small\":1.25e-3,\"large\":6E+2}";

        assert_eq!(
            json_number_field(body, "negative").as_deref(),
            Some("-0.125")
        );
        assert_eq!(json_number_field(body, "small").as_deref(), Some("1.25e-3"));
        assert_eq!(json_number_field(body, "large").as_deref(), Some("6E+2"));
    }

    #[test]
    fn rejects_malformed_json_number_fields() {
        assert_eq!(json_number_field("{\"score\":-}", "score"), None);
        assert_eq!(json_number_field("{\"score\":1.}", "score"), None);
        assert_eq!(json_number_field("{\"score\":1e}", "score"), None);
        assert_eq!(json_number_field("{\"score\":01}", "score"), None);
    }

    #[test]
    fn rejects_malformed_json_bool_fields() {
        assert_eq!(
            json_bool_field("{\"ok\":true,\"next\":1}", "ok"),
            Some(true)
        );
        assert_eq!(json_bool_field("{\"ok\":false }", "ok"), Some(false));
        assert_eq!(json_bool_field("{\"ok\":trueish}", "ok"), None);
        assert_eq!(json_bool_field("{\"ok\":falsehood}", "ok"), None);
    }

    #[test]
    fn extracts_json_string_array_field() {
        let body = "{\"readiness_warnings\":[\"cpu first\",\"gpu\\nmissing\"]}";

        assert_eq!(
            json_string_array_field(body, "readiness_warnings"),
            Some(vec!["cpu first".to_owned(), "gpu\nmissing".to_owned()])
        );
    }

    #[test]
    fn json_string_array_field_requires_tail_delimiter() {
        assert_eq!(
            json_string_array_field("{\"roles\":[\"user\"]x}", "roles"),
            None
        );
        assert_eq!(json_string_array_field("{\"roles\":[]x}", "roles"), None);
        assert_eq!(
            json_string_array_field("{\"roles\":[\"user\"],\"next\":true}", "roles"),
            Some(vec!["user".to_owned()])
        );
    }

    #[test]
    fn rejects_trailing_garbage_after_json_array_and_object_fields() {
        assert_eq!(json_array_field("{\"items\":[1]x}", "items"), None);
        assert_eq!(json_object_field("{\"item\":{\"ok\":true}x}", "item"), None);
    }

    #[test]
    fn ignores_field_names_inside_json_string_values() {
        let body = "{\"note\":\"\\\"answer\\\":\\\"poison\\\",\",\"answer\":\"ok\",\"count\":7,\"enabled\":true,\"items\":[1],\"item\":{\"id\":1}}";

        assert_eq!(json_string_field(body, "answer").as_deref(), Some("ok"));
        assert_eq!(json_number_field(body, "count").as_deref(), Some("7"));
        assert_eq!(json_bool_field(body, "enabled"), Some(true));
        assert_eq!(json_array_field(body, "items"), Some("1"));
        assert_eq!(json_object_field(body, "item"), Some("{\"id\":1}"));
    }

    #[test]
    fn requires_colon_after_json_field_name() {
        let body = "{\"answer\" \"poison\", \"next\":1, \"answer\":\"ok\"}";

        assert_eq!(json_string_field(body, "answer").as_deref(), Some("ok"));
    }

    #[test]
    fn rejects_invalid_nested_strings_in_json_array_and_object_fields() {
        assert_eq!(json_array_field("{\"items\":[\"bad\\q\"]}", "items"), None);
        assert_eq!(
            json_object_field("{\"item\":{\"value\":\"bad\nstring\"}}", "item"),
            None
        );
        assert_eq!(json_array_field("{\"items\":[\"\\ud800\"]}", "items"), None);
    }

    #[test]
    fn extracts_nested_object_and_array_items() {
        let body =
            "{\"retrieval\":{\"matches\":[{\"id\":1,\"text\":\"a,b\"},{\"id\":2}],\"ok\":true}}";
        let retrieval = json_object_field(body, "retrieval").unwrap();
        let matches = json_array_field(retrieval, "matches").unwrap();
        let items = json_object_items(matches);

        assert_eq!(items.len(), 2);
        assert_eq!(json_number_field(items[0], "id").as_deref(), Some("1"));
        assert_eq!(json_string_field(items[0], "text").as_deref(), Some("a,b"));
    }

    #[test]
    fn json_object_items_keeps_braces_inside_valid_strings() {
        let body = r#"{"matches":[{"lesson":"literal { brace } text"}]}"#;
        let matches = json_array_field(body, "matches").unwrap();
        let items = json_object_items(matches);

        assert_eq!(items.len(), 1);
        assert_eq!(
            json_string_field(items[0], "lesson").as_deref(),
            Some("literal { brace } text")
        );
    }

    #[test]
    fn json_object_items_does_not_emit_item_with_invalid_string() {
        let items = json_object_items(r#"{"lesson":"bad\q"}"#);

        assert!(items.is_empty());
    }
}
