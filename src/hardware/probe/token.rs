pub(super) fn sanitize_probe_token(value: &str) -> String {
    let mut sanitized = value
        .trim()
        .chars()
        .filter_map(|ch| {
            let lower = ch.to_ascii_lowercase();
            if lower.is_ascii_alphanumeric() || matches!(lower, '-' | '_' | '.' | ':' | '/') {
                Some(lower)
            } else if lower.is_ascii_whitespace() {
                Some('-')
            } else {
                None
            }
        })
        .take(64)
        .collect::<String>();
    while sanitized.contains("--") {
        sanitized = sanitized.replace("--", "-");
    }
    sanitized.trim_matches('-').to_owned()
}
