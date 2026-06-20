pub(in crate::gemma_business::smoke_report) fn feedback_json(
    applied: u64,
    rust_check_feedback_applied: u64,
) -> String {
    format!(
        "{{\"applied\":{},\"rust_check_feedback_applied\":{}}}",
        applied, rust_check_feedback_applied
    )
}

#[cfg(test)]
mod tests {
    use super::feedback_json;

    #[test]
    fn feedback_json_renders_feedback_counters() {
        assert_eq!(
            feedback_json(8, 5),
            "{\"applied\":8,\"rust_check_feedback_applied\":5}"
        );
    }
}
