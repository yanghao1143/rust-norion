pub(super) fn object_body<'a>(body: &'a str, object: &str) -> Option<&'a str> {
    enclosed_body_after_field(body, object, '{', '}')
}

pub(super) fn array_body<'a>(body: &'a str, field: &str) -> Option<&'a str> {
    enclosed_body_after_field(body, field, '[', ']')
}

fn enclosed_body_after_field<'a>(
    body: &'a str,
    field: &str,
    open: char,
    close: char,
) -> Option<&'a str> {
    let needle = format!("\"{field}\"");
    let after_field = body.get(body.find(&needle)? + needle.len()..)?;
    let after_colon = after_field.get(after_field.find(':')? + 1..)?.trim_start();
    let mut chars = after_colon.char_indices();
    if chars.next()?.1 != open {
        return None;
    }

    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, character) in after_colon.char_indices() {
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
            character if character == open => depth = depth.saturating_add(1),
            character if character == close => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return after_colon.get(..=index);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{array_body, object_body};

    #[test]
    fn enclosed_body_scanner_ignores_delimiters_inside_strings() {
        let body =
            r#"{"gate":{"note":"brace } and quote \" kept","passed":true},"items":["a]","b"]}"#;

        assert_eq!(
            object_body(body, "gate"),
            Some(r#"{"note":"brace } and quote \" kept","passed":true}"#)
        );
        assert_eq!(array_body(body, "items"), Some(r#"["a]","b"]"#));
    }
}
