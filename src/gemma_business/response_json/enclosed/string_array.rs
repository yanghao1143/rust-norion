pub(super) fn parse_json_string_array(array_body: &str) -> Option<Vec<String>> {
    let inner = array_body.strip_prefix('[')?.strip_suffix(']')?;
    let mut items = Vec::new();
    let mut chars = inner.char_indices().peekable();
    loop {
        while matches!(chars.peek().map(|(_, character)| character), Some(character) if character.is_whitespace() || *character == ',')
        {
            chars.next();
        }
        let Some((_, character)) = chars.peek().copied() else {
            return Some(items);
        };
        if character != '"' {
            return None;
        }
        chars.next();
        items.push(parse_json_string_item(&mut chars)?);
    }
}

fn parse_json_string_item(
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
) -> Option<String> {
    let mut item = String::new();
    let mut escaped = false;
    for (_, character) in chars.by_ref() {
        if escaped {
            match character {
                '"' => item.push('"'),
                '\\' => item.push('\\'),
                '/' => item.push('/'),
                'n' => item.push('\n'),
                'r' => item.push('\r'),
                't' => item.push('\t'),
                'b' => item.push('\u{0008}'),
                'f' => item.push('\u{000c}'),
                other => item.push(other),
            }
            escaped = false;
            continue;
        }
        match character {
            '\\' => escaped = true,
            '"' => return Some(item),
            other => item.push(other),
        }
    }
    None
}
