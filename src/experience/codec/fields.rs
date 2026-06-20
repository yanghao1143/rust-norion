pub(super) fn option_f32_to_field(value: Option<f32>) -> String {
    value
        .filter(|value| value.is_finite())
        .map(|value| format!("{value:.6}"))
        .unwrap_or_default()
}

pub(super) fn field_to_finite_f32(value: &str) -> Option<f32> {
    if value.is_empty() {
        return None;
    }
    value.parse::<f32>().ok().filter(|value| value.is_finite())
}

pub(super) fn finite_f32_to_field(value: f32) -> String {
    if value.is_finite() {
        format!("{value:.6}")
    } else {
        "0.000000".to_owned()
    }
}

pub(super) fn bool_to_field(value: bool) -> &'static str {
    if value { "1" } else { "0" }
}

pub(super) fn field_to_bool(value: &str) -> Option<bool> {
    match value {
        "1" | "true" => Some(true),
        "0" | "false" => Some(false),
        _ => None,
    }
}

pub(super) fn non_empty_string(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_owned())
}

pub(super) fn sanitize_control_part(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            '\u{1e}' | '\u{1f}' | '\t' | '\n' | '\r' => ' ',
            other => other,
        })
        .collect()
}

pub(super) fn escape_field(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('|', "\\p")
}

pub(super) fn unescape_field(value: &str) -> String {
    let mut out = String::new();
    let mut chars = value.chars();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }

        match chars.next() {
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('p') => out.push('|'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }

    out
}
