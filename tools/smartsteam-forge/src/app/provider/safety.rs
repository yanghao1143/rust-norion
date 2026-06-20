pub(super) fn unsafe_device_summary(summary: &str) -> bool {
    let summary = summary.to_lowercase();
    summary.contains("gemma_12b_device")
        || summary.contains("cpu/disk-first")
        || (summary.contains("runtime=gemma") && summary.contains("lane=cpu-"))
        || (summary.contains("runtime=gemma") && summary.contains("lane=disk-backed-streaming"))
}
