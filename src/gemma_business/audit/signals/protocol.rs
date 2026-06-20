pub(super) fn business_answer_contains_protocol_leak_impl(answer: &str, lower: &str) -> bool {
    lower.contains(".thought")
        || lower.contains("<channel")
        || lower.contains("</channel")
        || lower.contains("hidden/thought")
        || answer.contains("<|channel|>")
}
