pub(crate) fn compact_business_answer_preview(answer: &str, _max_chars: usize) -> String {
    format!("answer_chars={}", answer.chars().count())
}

#[cfg(test)]
mod tests {
    use super::compact_business_answer_preview;

    #[test]
    fn compact_business_answer_preview_reports_count_without_answer_text() {
        let preview = compact_business_answer_preview("line one\n\nline two", 180);

        assert_eq!(preview, "answer_chars=18");
        assert!(!preview.contains("line one"));
        assert!(!preview.contains("line two"));
    }
}
