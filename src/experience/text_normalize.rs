pub(crate) fn normalize_full_width_ascii(value: &str) -> String {
    value.chars().map(normalize_full_width_ascii_char).collect()
}

pub(crate) fn normalize_full_width_ascii_char(ch: char) -> char {
    match ch {
        '\u{ff01}'..='\u{ff5e}' => char::from_u32(ch as u32 - 0xfee0).unwrap_or(ch),
        '\u{3000}' => ' ',
        _ => ch,
    }
}

pub(crate) fn normalized_marker_span(value: &str, marker: &str) -> Option<(usize, usize)> {
    let marker = normalize_full_width_ascii(marker).to_ascii_lowercase();
    if marker.is_empty() {
        return Some((0, 0));
    }

    for (start, _) in value.char_indices() {
        let mut normalized = String::new();
        let mut end = start;
        for (relative_index, ch) in value[start..].char_indices() {
            normalized.push(normalize_full_width_ascii_char(ch).to_ascii_lowercase());
            end = start + relative_index + ch.len_utf8();
            if normalized.len() >= marker.len() {
                break;
            }
        }
        if normalized == marker {
            return Some((start, end));
        }
    }

    None
}

pub(crate) fn normalized_marker_spans(value: &str, marker: &str) -> Vec<(usize, usize)> {
    if normalize_full_width_ascii(marker).is_empty() {
        return vec![(0, 0)];
    }

    let mut spans = Vec::new();
    let mut offset = 0usize;
    while offset < value.len() {
        let Some((start, end)) = normalized_marker_span(&value[offset..], marker) else {
            break;
        };
        let absolute_start = offset + start;
        let absolute_end = offset + end;
        spans.push((absolute_start, absolute_end));
        offset = absolute_end;
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_full_width_ascii_without_touching_cjk_text() {
        assert_eq!(
            normalize_full_width_ascii("Ｒｕｓｔ＿ｌｏｏｐ　保持提示门控"),
            "Rust_loop 保持提示门控"
        );
        assert_eq!(normalize_full_width_ascii_char('：'), ':');
        assert_eq!(normalize_full_width_ascii_char('证'), '证');
    }

    #[test]
    fn marker_span_matches_full_width_ascii_and_returns_original_span() {
        let value = "prefix ｓｕｍｍａｒｙ＝Ｒｅｕｓｅ＿Ｒｅｓｐｏｎｓｅ： ok";
        let (start, end) = normalized_marker_span(value, " summary=").unwrap();

        assert_eq!(&value[start..end], " ｓｕｍｍａｒｙ＝");
        assert_eq!(&value[end..], "Ｒｅｕｓｅ＿Ｒｅｓｐｏｎｓｅ： ok");
    }

    #[test]
    fn marker_spans_find_repeated_mixed_width_markers() {
        let value = "summary=literal metadata ｓｕｍｍａｒｙ＝clean summary";
        let spans = normalized_marker_spans(value, "summary=");

        assert_eq!(spans.len(), 2);
        assert_eq!(&value[spans[0].0..spans[0].1], "summary=");
        assert_eq!(&value[spans[1].0..spans[1].1], "ｓｕｍｍａｒｙ＝");
    }

    #[test]
    fn marker_spans_handle_empty_marker_without_looping() {
        assert_eq!(normalized_marker_span("abc", ""), Some((0, 0)));
        assert_eq!(normalized_marker_spans("abc", ""), vec![(0, 0)]);
    }
}
