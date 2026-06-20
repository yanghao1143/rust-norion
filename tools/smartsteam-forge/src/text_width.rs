pub(crate) fn terminal_width(text: &str) -> usize {
    text.chars().map(terminal_char_width).sum()
}

fn terminal_char_width(ch: char) -> usize {
    match ch {
        '\t' => 4,
        ch if ch.is_ascii() => 1,
        _ => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_ascii_tabs_and_wide_chars_as_terminal_columns() {
        assert_eq!(terminal_width("abc"), 3);
        assert_eq!(terminal_width("\t"), 4);
        assert_eq!(terminal_width("你a"), 3);
    }
}
