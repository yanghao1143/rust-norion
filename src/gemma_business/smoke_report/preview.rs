pub(crate) fn compact_business_answer_preview(answer: &str, max_chars: usize) -> String {
    let mut preview = answer.chars().take(max_chars).collect::<String>();
    if answer.chars().count() > max_chars {
        preview.push_str("...");
    }
    preview.split_whitespace().collect::<Vec<_>>().join(" ")
}
