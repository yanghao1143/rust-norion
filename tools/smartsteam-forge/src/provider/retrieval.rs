use super::json::{
    json_array_field, json_bool_field, json_number_field, json_object_field, json_object_items,
    json_string, json_string_field,
};

const DEFAULT_MATCH_LIMIT: usize = 5;

pub(crate) fn experience_retrieval_request_body(
    prompt: &str,
    profile: &str,
    limit: usize,
    index_context: Option<&str>,
) -> String {
    let index_context_field = index_context
        .filter(|context| !context.trim().is_empty())
        .map(|context| format!(",\"index_context\":{}", json_string(context.trim())))
        .unwrap_or_default();
    format!(
        "{{\"prompt\":{},\"profile\":{},\"limit\":{}{}}}",
        json_string(prompt),
        json_string(profile),
        limit.max(1),
        index_context_field
    )
}

pub(crate) fn experience_retrieval_summary(body: &str) -> Result<String, String> {
    ensure_ok(body, "experience retrieval")?;
    let retrieval = json_object_field(body, "retrieval")
        .ok_or_else(|| "experience retrieval response missing retrieval object".to_owned())?;
    let mut lines = vec!["Noiron experience retrieval preview".to_owned()];
    push_field_line(&mut lines, "prompt", json_string_field(retrieval, "prompt"));
    push_field_line(
        &mut lines,
        "profile",
        json_string_field(retrieval, "profile"),
    );
    push_bool_field_line(
        &mut lines,
        "index_context_used",
        json_bool_field(retrieval, "index_context_used"),
    );
    push_field_line(
        &mut lines,
        "index_context_chars",
        json_number_field(retrieval, "index_context_chars"),
    );
    push_field_line(
        &mut lines,
        "total_records",
        json_number_field(retrieval, "total_records"),
    );
    push_field_line(
        &mut lines,
        "requested_limit",
        json_number_field(retrieval, "requested_limit"),
    );
    push_field_line(
        &mut lines,
        "match_count",
        json_number_field(retrieval, "match_count"),
    );
    push_field_line(
        &mut lines,
        "skipped_cross_task_pollution",
        json_number_field(retrieval, "skipped_cross_task_pollution"),
    );
    push_field_line(
        &mut lines,
        "retrieval_noise_penalized_candidates",
        json_number_field(retrieval, "retrieval_noise_penalized_candidates"),
    );
    push_field_line(
        &mut lines,
        "retrieval_noise_filtered_candidates",
        json_number_field(retrieval, "retrieval_noise_filtered_candidates"),
    );
    push_field_line(
        &mut lines,
        "suppressed_prompt_index_candidates",
        json_number_field(retrieval, "suppressed_prompt_index_candidates"),
    );
    push_field_line(
        &mut lines,
        "max_retrieval_noise_penalty",
        json_number_field(retrieval, "max_retrieval_noise_penalty"),
    );
    push_field_line(
        &mut lines,
        "max_score",
        json_number_field(retrieval, "max_score"),
    );
    push_matches(&mut lines, retrieval);
    Ok(lines.join("\n"))
}

fn ensure_ok(body: &str, label: &str) -> Result<(), String> {
    if json_bool_field(body, "ok") == Some(false) {
        let error = json_string_field(body, "error").unwrap_or_else(|| "unknown".to_owned());
        return Err(format!("{label} failed: {error}"));
    }
    Ok(())
}

fn push_matches(lines: &mut Vec<String>, object: &str) {
    let Some(matches) = json_array_field(object, "matches") else {
        return;
    };
    let items = json_object_items(matches);
    if items.is_empty() {
        lines.push("matches=none".to_owned());
        return;
    }
    lines.push(format!("matches={}", items.len()));
    for item in items.into_iter().take(DEFAULT_MATCH_LIMIT) {
        let id = json_number_field(item, "experience_id").unwrap_or_else(|| "unknown".to_owned());
        let score = json_number_field(item, "score").unwrap_or_else(|| "unknown".to_owned());
        let quality = json_number_field(item, "quality").unwrap_or_else(|| "unknown".to_owned());
        let reward =
            json_number_field(item, "process_reward").unwrap_or_else(|| "unknown".to_owned());
        let action =
            json_string_field(item, "reward_action").unwrap_or_else(|| "unknown".to_owned());
        let lesson =
            json_string_field(item, "lesson_preview").unwrap_or_else(|| "unknown".to_owned());
        let usable_hint =
            json_string_field(item, "usable_hint_preview").unwrap_or_else(|| lesson.clone());
        let prompt =
            json_string_field(item, "prompt_preview").unwrap_or_else(|| "unknown".to_owned());
        let mut line = format!(
            "match id={id} score={score} quality={quality} reward={reward} action={action} usable_hint={usable_hint} lesson={lesson} prompt={prompt}"
        );
        append_optional_string_segment(&mut line, item, "runtime_model");
        append_optional_string_segment(&mut line, item, "runtime_adapter");
        append_optional_string_segment(&mut line, item, "runtime_device");
        append_optional_string_segment(&mut line, item, "runtime_primary_lane");
        append_optional_string_segment(&mut line, item, "runtime_fallback_lane");
        append_optional_string_segment(&mut line, item, "runtime_memory_mode");
        append_optional_string_segment(&mut line, item, "runtime_device_execution_source");
        append_optional_number_segment(&mut line, item, "runtime_forward_energy");
        append_optional_number_segment(&mut line, item, "runtime_kv_influence");
        append_optional_number_segment(&mut line, item, "runtime_uncertainty_perplexity");
        append_optional_number_segment(&mut line, item, "recursive_runtime_calls");
        append_optional_u64_array_segment(&mut line, item, "stored_runtime_kv_memory_ids");
        lines.push(line);
    }
}

fn append_optional_string_segment(line: &mut String, object: &str, field: &str) {
    if let Some(value) = json_string_field(object, field) {
        line.push(' ');
        line.push_str(field);
        line.push('=');
        line.push_str(&value);
    }
}

fn append_optional_number_segment(line: &mut String, object: &str, field: &str) {
    if let Some(value) = json_number_field(object, field) {
        line.push(' ');
        line.push_str(field);
        line.push('=');
        line.push_str(&value);
    }
}

fn append_optional_u64_array_segment(line: &mut String, object: &str, field: &str) {
    if let Some(values) = json_u64_array_field(object, field) {
        line.push(' ');
        line.push_str(field);
        line.push('=');
        if values.is_empty() {
            line.push_str("none");
        } else {
            line.push_str(&values.join(","));
        }
    }
}

fn json_u64_array_field(body: &str, field: &str) -> Option<Vec<String>> {
    let array = json_array_field(body, field)?.trim();
    if array.is_empty() {
        return Some(Vec::new());
    }
    array
        .split(',')
        .map(|value| {
            let value = value.trim();
            value.parse::<u64>().ok().map(|number| number.to_string())
        })
        .collect()
}

fn push_field_line(lines: &mut Vec<String>, name: &str, value: Option<String>) {
    if let Some(value) = value {
        lines.push(format!("{name}={value}"));
    }
}

fn push_bool_field_line(lines: &mut Vec<String>, name: &str, value: Option<bool>) {
    if let Some(value) = value {
        lines.push(format!("{name}={value}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarizes_experience_retrieval() {
        let summary = experience_retrieval_summary(
            "{\"ok\":true,\"retrieval\":{\"prompt\":\"rust loop\",\"profile\":\"coding\",\"index_context_used\":true,\"index_context_chars\":88,\"total_records\":10,\"requested_limit\":2,\"matches\":[{\"experience_id\":7,\"score\":0.9,\"quality\":0.8,\"process_reward\":0.7,\"reward_action\":\"reinforce\",\"lesson_preview\":\"accepted_pattern quality=0.9\",\"usable_hint_preview\":\"usable clean loop\",\"prompt_preview\":\"Rust for loop\",\"runtime_model\":\"gemma-3-12b\",\"runtime_adapter\":\"llama.cpp\",\"runtime_device\":\"metal\",\"runtime_primary_lane\":\"quality\",\"runtime_fallback_lane\":\"summary\",\"runtime_memory_mode\":\"kv\",\"runtime_device_execution_source\":\"metal\",\"runtime_forward_energy\":0.72,\"runtime_kv_influence\":0.61,\"runtime_uncertainty_perplexity\":1.25,\"recursive_runtime_calls\":2,\"stored_runtime_kv_memory_ids\":[11,13]}],\"match_count\":1,\"skipped_cross_task_pollution\":4,\"retrieval_noise_penalized_candidates\":2,\"retrieval_noise_filtered_candidates\":1,\"suppressed_prompt_index_candidates\":2,\"max_retrieval_noise_penalty\":0.44,\"max_score\":0.9}}",
        )
        .unwrap();

        assert!(summary.contains("Noiron experience retrieval preview"));
        assert!(summary.contains("index_context_used=true"));
        assert!(summary.contains("index_context_chars=88"));
        assert!(summary.contains("skipped_cross_task_pollution=4"));
        assert!(summary.contains("retrieval_noise_penalized_candidates=2"));
        assert!(summary.contains("retrieval_noise_filtered_candidates=1"));
        assert!(summary.contains("suppressed_prompt_index_candidates=2"));
        assert!(summary.contains("max_retrieval_noise_penalty=0.44"));
        assert!(summary.contains("match id=7"));
        assert!(summary.contains("usable_hint=usable clean loop"));
        assert!(summary.contains("runtime_model=gemma-3-12b"));
        assert!(summary.contains("runtime_adapter=llama.cpp"));
        assert!(summary.contains("runtime_device=metal"));
        assert!(summary.contains("runtime_primary_lane=quality"));
        assert!(summary.contains("runtime_fallback_lane=summary"));
        assert!(summary.contains("runtime_memory_mode=kv"));
        assert!(summary.contains("runtime_device_execution_source=metal"));
        assert!(summary.contains("runtime_forward_energy=0.72"));
        assert!(summary.contains("runtime_kv_influence=0.61"));
        assert!(summary.contains("runtime_uncertainty_perplexity=1.25"));
        assert!(summary.contains("recursive_runtime_calls=2"));
        assert!(summary.contains("stored_runtime_kv_memory_ids=11,13"));
    }

    #[test]
    fn retrieval_request_body_includes_structured_index_context() {
        let body = experience_retrieval_request_body(
            "model pool route code",
            "coding",
            0,
            Some(" model_pool_index:\nsrc/model_service "),
        );

        assert!(body.contains("\"prompt\":\"model pool route code\""));
        assert!(body.contains("\"profile\":\"coding\""));
        assert!(body.contains("\"limit\":1"));
        assert!(body.contains("\"index_context\":\"model_pool_index:\\nsrc/model_service\""));
    }

    #[test]
    fn retrieval_summary_ignores_retrieval_key_inside_string_values() {
        let summary = experience_retrieval_summary(
            r#"{"ok":true,"note":"\"retrieval\":{\"prompt\":\"poison\",\"matches\":[{\"experience_id\":999}]},","retrieval":{"prompt":"real prompt","profile":"coding","total_records":0,"matches":[]}}"#,
        )
        .unwrap();

        assert!(summary.contains("prompt=real prompt"));
        assert!(summary.contains("profile=coding"));
        assert!(summary.contains("matches=none"));
        assert!(!summary.contains("poison"));
        assert!(!summary.contains("999"));
    }

    #[test]
    fn retrieval_summary_omits_null_max_score_for_empty_matches() {
        let summary = experience_retrieval_summary(
            "{\"ok\":true,\"retrieval\":{\"prompt\":\"rust loop\",\"profile\":\"coding\",\"total_records\":0,\"requested_limit\":2,\"matches\":[],\"match_count\":0,\"max_score\":null}}",
        )
        .unwrap();

        assert!(summary.contains("total_records=0"));
        assert!(summary.contains("matches=none"));
        assert!(summary.contains("match_count=0"));
        assert!(!summary.contains("max_score="));
    }
}
