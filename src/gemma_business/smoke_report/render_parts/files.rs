pub(in crate::gemma_business::smoke_report) fn files_json(
    trace: &str,
    memory: &str,
    experience: &str,
    adaptive: &str,
    response: &str,
) -> String {
    format!(
        "{{\"trace\":{},\"memory\":{},\"experience\":{},\"adaptive\":{},\"response\":{}}}",
        trace, memory, experience, adaptive, response
    )
}

#[cfg(test)]
mod tests {
    use super::files_json;

    #[test]
    fn files_json_renders_all_report_artifact_paths() {
        assert_eq!(
            files_json(
                "\"trace\"",
                "\"memory\"",
                "\"experience\"",
                "\"adaptive\"",
                "\"response\""
            ),
            "{\"trace\":\"trace\",\"memory\":\"memory\",\"experience\":\"experience\",\"adaptive\":\"adaptive\",\"response\":\"response\"}"
        );
    }
}
