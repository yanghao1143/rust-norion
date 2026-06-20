use std::io;

use super::RustSnippetCheck;

pub(crate) const DEFAULT_RUSTC_PROGRAM: &str = "rustc";
pub(crate) const DEFAULT_EDITION: &str = "2021";
pub(crate) const MAX_RUST_SNIPPET_BYTES: usize = 256 * 1024;
pub(crate) const MAX_DIAGNOSTIC_CHARS: usize = 12_000;

pub(crate) fn validate_snippet_input(request: &RustSnippetCheck) -> io::Result<()> {
    if request.code.trim().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "rust check code must be non-empty",
        ));
    }
    if request.code.len() > MAX_RUST_SNIPPET_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "rust check code exceeds 256 KiB limit",
        ));
    }
    Ok(())
}

pub(crate) fn normalize_edition(edition: &str) -> io::Result<String> {
    let edition = edition.trim();
    match edition {
        "" => Ok(DEFAULT_EDITION.to_owned()),
        "2018" | "2021" | "2024" => Ok(edition.to_owned()),
        other => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unsupported Rust edition: {other}"),
        )),
    }
}

pub(crate) fn bounded_output(bytes: &[u8], max_chars: usize) -> String {
    let text = String::from_utf8_lossy(bytes);
    let mut out = text.chars().take(max_chars).collect::<String>();
    if text.chars().count() > max_chars {
        out.push_str("\n[diagnostics truncated]");
    }
    out
}

pub(crate) fn sanitize_path_segment(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_owned();
    if sanitized.is_empty() {
        "rust-check".to_owned()
    } else {
        sanitized.chars().take(48).collect()
    }
}
