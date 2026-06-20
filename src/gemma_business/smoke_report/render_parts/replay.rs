pub(in crate::gemma_business::smoke_report) fn replay_json(
    live_memory_feedback_applied: u64,
    live_evolution_items: u64,
) -> String {
    format!(
        "{{\"live_memory_feedback_applied\":{},\"live_evolution_items\":{}}}",
        live_memory_feedback_applied, live_evolution_items
    )
}

#[cfg(test)]
mod tests {
    use super::replay_json;

    #[test]
    fn replay_json_renders_live_replay_counters() {
        assert_eq!(
            replay_json(4, 7),
            "{\"live_memory_feedback_applied\":4,\"live_evolution_items\":7}"
        );
    }
}
