pub(super) fn sanitize_gemma_model_service_protocol_artifacts(answer: &str) -> String {
    let mut cleaned = answer
        .replace("<|channel|>", "")
        .replace("</channel>", "")
        .replace("</Channel>", "")
        .replace(".thought", "");

    loop {
        let lower = cleaned.to_ascii_lowercase();
        let Some(start) = lower.find("<channel") else {
            break;
        };
        let end = cleaned[start..]
            .find('>')
            .map(|offset| start + offset + 1)
            .unwrap_or(cleaned.len());
        cleaned.replace_range(start..end, "");
    }

    cleaned
        .trim()
        .trim_start_matches("thought ")
        .trim_start_matches("Thought ")
        .trim()
        .to_owned()
}
