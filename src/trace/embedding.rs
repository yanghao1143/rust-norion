use super::fields::{
    extract_json_bool_field, extract_json_nullable_string_field, extract_json_string_field,
    extract_json_usize_field, json_object_after_field,
};

pub(super) fn evaluate_trace_embedding(line: &str) -> Vec<String> {
    let mut failures = Vec::new();
    let Some(embedding) = json_object_after_field(line, "embedding") else {
        failures.push("embedding object is missing or invalid".to_owned());
        return failures;
    };

    let query_source = extract_json_string_field(embedding, "query_source");
    match query_source.as_deref() {
        Some("runtime") | Some("fallback") => {}
        Some(other) => failures.push(format!("embedding query_source {other} is not recognized")),
        None => failures.push("embedding query_source is missing".to_owned()),
    }

    let query_dimensions = extract_json_usize_field(embedding, "query_dimensions").unwrap_or(0);
    if query_dimensions == 0 {
        failures.push("embedding query_dimensions must be > 0".to_owned());
    }

    let memory_write_source = extract_json_nullable_string_field(embedding, "memory_write_source");
    if let Some(source) = memory_write_source.as_deref()
        && !matches!(source, "runtime" | "fallback")
    {
        failures.push(format!(
            "embedding memory_write_source {source} is not recognized"
        ));
    }

    let memory_write_dimensions =
        extract_json_usize_field(embedding, "memory_write_dimensions").unwrap_or(0);
    let memory_write_present = memory_write_source.is_some();
    if memory_write_present && memory_write_dimensions == 0 {
        failures
            .push("embedding memory_write_dimensions must be > 0 when source exists".to_owned());
    }
    if !memory_write_present && memory_write_dimensions != 0 {
        failures.push(format!(
            "embedding memory_write_dimensions {memory_write_dimensions} requires memory_write_source"
        ));
    }

    let gist_writes = extract_json_usize_field(embedding, "gist_writes").unwrap_or(0);
    let gist_write_runtime_calls =
        extract_json_usize_field(embedding, "gist_write_runtime_calls").unwrap_or(0);
    let gist_write_fallback_calls =
        extract_json_usize_field(embedding, "gist_write_fallback_calls").unwrap_or(0);
    if gist_write_runtime_calls.saturating_add(gist_write_fallback_calls) != gist_writes {
        failures.push(format!(
            "embedding gist write calls {} do not match gist_writes {gist_writes}",
            gist_write_runtime_calls.saturating_add(gist_write_fallback_calls)
        ));
    }

    let runtime_embedding_calls =
        extract_json_usize_field(embedding, "runtime_embedding_calls").unwrap_or(0);
    let fallback_embedding_calls =
        extract_json_usize_field(embedding, "fallback_embedding_calls").unwrap_or(0);
    let expected_calls = 1 + usize::from(memory_write_present) + gist_writes;
    let observed_calls = runtime_embedding_calls.saturating_add(fallback_embedding_calls);
    if observed_calls != expected_calls {
        failures.push(format!(
            "embedding calls {observed_calls} do not match expected {expected_calls}"
        ));
    }

    let fallback_used = extract_json_bool_field(embedding, "fallback_used").unwrap_or(false);
    if fallback_used != (fallback_embedding_calls > 0) {
        failures.push(format!(
            "embedding fallback_used {fallback_used} does not match fallback_embedding_calls {fallback_embedding_calls}"
        ));
    }

    let runtime_available =
        extract_json_bool_field(embedding, "runtime_embedding_available").unwrap_or(false);
    if runtime_available != (runtime_embedding_calls > 0) {
        failures.push(format!(
            "embedding runtime_embedding_available {runtime_available} does not match runtime_embedding_calls {runtime_embedding_calls}"
        ));
    }

    let mut expected_runtime_calls = usize::from(query_source.as_deref() == Some("runtime"))
        + usize::from(memory_write_source.as_deref() == Some("runtime"))
        + gist_write_runtime_calls;
    let expected_fallback_calls = usize::from(query_source.as_deref() == Some("fallback"))
        + usize::from(memory_write_source.as_deref() == Some("fallback"))
        + gist_write_fallback_calls;
    if query_source.is_none() {
        expected_runtime_calls = expected_runtime_calls.saturating_add(0);
    }
    if runtime_embedding_calls != expected_runtime_calls {
        failures.push(format!(
            "embedding runtime_embedding_calls {runtime_embedding_calls} do not match sources {expected_runtime_calls}"
        ));
    }
    if fallback_embedding_calls != expected_fallback_calls {
        failures.push(format!(
            "embedding fallback_embedding_calls {fallback_embedding_calls} do not match sources {expected_fallback_calls}"
        ));
    }

    failures
}
