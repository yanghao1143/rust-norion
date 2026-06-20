pub(in crate::gemma_business::smoke_report) fn state_json(
    runtime_tokens: u64,
    external_feedbacks: u64,
    feedback_memory_updates: u64,
    replay_rust_check_passed: u64,
) -> String {
    format!(
        "{{\"runtime_tokens\":{},\"external_feedbacks\":{},\"feedback_memory_updates\":{},\"replay_rust_check_passed\":{}}}",
        runtime_tokens, external_feedbacks, feedback_memory_updates, replay_rust_check_passed
    )
}

#[cfg(test)]
mod tests {
    use super::state_json;

    #[test]
    fn state_json_renders_runtime_and_feedback_counters() {
        assert_eq!(
            state_json(10, 2, 3, 1),
            "{\"runtime_tokens\":10,\"external_feedbacks\":2,\"feedback_memory_updates\":3,\"replay_rust_check_passed\":1}"
        );
    }
}
