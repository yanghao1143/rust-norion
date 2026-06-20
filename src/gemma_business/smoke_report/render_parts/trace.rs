pub(in crate::gemma_business::smoke_report) fn trace_json(checked_lines: u64) -> String {
    format!("{{\"checked_lines\":{}}}", checked_lines)
}

#[cfg(test)]
mod tests {
    use super::trace_json;

    #[test]
    fn trace_json_renders_checked_line_count() {
        assert_eq!(trace_json(42), "{\"checked_lines\":42}");
    }
}
