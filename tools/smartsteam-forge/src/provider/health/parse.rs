use super::{
    ActiveRequest, ExperienceHygieneHealth, ExperienceHygieneRepairHealth, ExperienceIndexHealth,
    LastInference, ProviderHealth,
};
use crate::provider::json::{
    json_bool_field, json_number_field, json_string_array_field, json_string_field,
};

pub(crate) fn parse_provider_health(body: &str) -> ProviderHealth {
    ProviderHealth {
        ok: json_bool_field(body, "ok").unwrap_or(false),
        service: json_string_field(body, "service"),
        requests_seen: json_number_field(body, "requests_seen"),
        active_engine_requests: json_number_field(body, "active_engine_requests"),
        engine_busy: json_bool_field(body, "engine_busy"),
        active_requests: parse_active_requests(body),
        runtime_mode: json_string_field(body, "runtime_mode"),
        gemma_runtime_server: json_string_field(body, "gemma_runtime_server"),
        gemma_runtime_reachable: json_bool_field(body, "gemma_runtime_reachable"),
        readiness_ok: json_bool_field(body, "readiness_ok"),
        safe_device_ok: json_bool_field(body, "safe_device_ok"),
        readiness_failures: json_string_array_field(body, "readiness_failures").unwrap_or_default(),
        safe_device_failures: json_string_array_field(body, "safe_device_failures")
            .unwrap_or_default(),
        device_profile: json_string_field(body, "device_profile"),
        device_accelerators: json_number_field(body, "device_accelerators"),
        device_pressure: json_number_field(body, "device_pressure"),
        device_primary_lane: json_string_field(body, "device_primary_lane"),
        device_memory_mode: json_string_field(body, "device_memory_mode"),
        device_plan_summary: json_string_field(body, "device_plan_summary"),
        device_probe_summary: json_string_field(body, "device_probe_summary"),
        readiness_warnings: json_string_array_field(body, "readiness_warnings").unwrap_or_default(),
        experience_hygiene: parse_experience_hygiene(body),
        last_inference: parse_last_inference(body),
        error: top_level_json_string_field(body, "error"),
    }
}

fn parse_experience_hygiene(body: &str) -> ExperienceHygieneHealth {
    let Some(object) = json_object_field(body, "experience_hygiene") else {
        return ExperienceHygieneHealth::default();
    };
    ExperienceHygieneHealth {
        checked: json_bool_field(object, "checked"),
        clean: json_bool_field(object, "clean"),
        findings: json_number_field(object, "findings"),
        watch: json_number_field(object, "watch"),
        quarantine_candidates: json_number_field(object, "quarantine_candidates"),
        legacy_metadata_lessons: json_number_field(object, "legacy_metadata_lessons"),
        legacy_metadata_without_clean_gist: json_number_field(
            object,
            "legacy_metadata_without_clean_gist",
        ),
        repair: parse_experience_hygiene_repair(object),
        index: parse_experience_index(object),
        error: json_string_field(object, "error"),
    }
}

fn parse_experience_index(object: &str) -> ExperienceIndexHealth {
    let Some(index) = json_object_field(object, "index") else {
        return ExperienceIndexHealth::default();
    };
    ExperienceIndexHealth {
        total_records: json_number_field(index, "total_records"),
        noisy_records: json_number_field(index, "noisy_records"),
        duplicate_outputs: json_number_field(index, "duplicate_outputs"),
        quality_score: json_number_field(index, "quality_score"),
        retrieval_ready: json_bool_field(index, "retrieval_ready"),
        risk_level: json_string_field(index, "risk_level"),
    }
}

fn parse_experience_hygiene_repair(object: &str) -> ExperienceHygieneRepairHealth {
    let Some(repair) = json_object_field(object, "repair") else {
        return ExperienceHygieneRepairHealth::default();
    };
    ExperienceHygieneRepairHealth {
        repairable_legacy_metadata_lessons: json_number_field(
            repair,
            "repairable_legacy_metadata_lessons",
        ),
        repairable_index_records: json_number_field(repair, "repairable_index_records"),
        projected_findings_after_repair: json_number_field(
            repair,
            "projected_findings_after_repair",
        ),
        projected_watch_after_repair: json_number_field(repair, "projected_watch_after_repair"),
        projected_quarantine_candidates_after_repair: json_number_field(
            repair,
            "projected_quarantine_candidates_after_repair",
        ),
        projected_legacy_metadata_lessons_after_repair: json_number_field(
            repair,
            "projected_legacy_metadata_lessons_after_repair",
        ),
        projected_legacy_metadata_without_clean_gist_after_repair: json_number_field(
            repair,
            "projected_legacy_metadata_without_clean_gist_after_repair",
        ),
    }
}

fn parse_active_requests(body: &str) -> Vec<ActiveRequest> {
    let Some(array) = json_array_field(body, "active_requests") else {
        return Vec::new();
    };
    json_object_items(array)
        .into_iter()
        .map(|object| ActiveRequest {
            request_id: json_number_field(object, "request_id"),
            endpoint: json_string_field(object, "endpoint"),
            elapsed_ms: json_number_field(object, "elapsed_ms"),
            prompt_preview: json_string_field(object, "prompt_preview"),
        })
        .collect()
}

fn parse_last_inference(body: &str) -> Option<LastInference> {
    let object = json_object_field(body, "last_inference")?;
    Some(LastInference {
        endpoint: json_string_field(object, "endpoint"),
        elapsed_ms: json_number_field(object, "elapsed_ms"),
        runtime_model: json_string_field(object, "runtime_model"),
        runtime_token_count: json_number_field(object, "runtime_token_count"),
        error: json_string_field(object, "error"),
    })
}

fn json_array_field<'a>(body: &'a str, field: &str) -> Option<&'a str> {
    let needle = format!("\"{field}\"");
    let after_field = body.get(body.find(&needle)? + needle.len()..)?;
    let after_colon = after_field.get(after_field.find(':')? + 1..)?;
    let trimmed = after_colon.trim_start();
    if !trimmed.starts_with('[') {
        return None;
    }
    let close = find_matching_json_close(trimmed, '[', ']')?;
    let trailing = trimmed.get(close + 1..)?.trim_start();
    if !json_value_tail_is_delimited(trailing) {
        return None;
    }
    trimmed.get(1..close)
}

fn json_object_items(input: &str) -> Vec<&str> {
    let mut items = Vec::new();
    let mut start = None;
    let mut depth = 0usize;
    let mut chars = input.char_indices().peekable();

    while let Some((index, character)) = chars.next() {
        match character {
            '"' => {
                if skip_json_string_literal(&mut chars).is_none() {
                    return items;
                }
            }
            '{' => {
                if depth == 0 {
                    start = Some(index);
                }
                depth = depth.saturating_add(1);
            }
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0
                    && let Some(start_index) = start.take()
                    && let Some(item) = input.get(start_index..=index)
                {
                    items.push(item);
                }
            }
            _ => {}
        }
    }

    items
}

fn json_object_field<'a>(body: &'a str, field: &str) -> Option<&'a str> {
    let needle = format!("\"{field}\"");
    let after_field = body.get(body.find(&needle)? + needle.len()..)?;
    let after_colon = after_field.get(after_field.find(':')? + 1..)?;
    let trimmed = after_colon.trim_start();
    if !trimmed.starts_with('{') {
        return None;
    }

    let close = find_matching_json_close(trimmed, '{', '}')?;
    let trailing = trimmed.get(close + 1..)?.trim_start();
    if !json_value_tail_is_delimited(trailing) {
        return None;
    }
    trimmed.get(..=close)
}

fn top_level_json_string_field(body: &str, field: &str) -> Option<String> {
    let value = top_level_json_value(body, field)?.trim_start();
    crate::provider::json::json_string_field(&format!("{{\"{field}\":{value}}}"), field)
}

fn top_level_json_value<'a>(body: &'a str, field: &str) -> Option<&'a str> {
    let needle = format!("\"{field}\"");
    let mut depth = 0usize;
    let mut chars = body.char_indices().peekable();

    while let Some((index, character)) = chars.next() {
        match character {
            '"' => {
                if depth == 1
                    && body
                        .get(index..)
                        .map(|remaining| remaining.starts_with(&needle))
                        .unwrap_or(false)
                {
                    let after_field = body.get(index + needle.len()..)?.trim_start();
                    if let Some(after_colon) = after_field.strip_prefix(':') {
                        return Some(after_colon.trim_start());
                    }
                }
                skip_json_string_literal(&mut chars)?;
            }
            '{' | '[' => depth = depth.saturating_add(1),
            '}' | ']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    None
}

fn find_matching_json_close(input: &str, open: char, close: char) -> Option<usize> {
    let mut depth = 0usize;
    let mut chars = input.char_indices().peekable();

    while let Some((index, character)) = chars.next() {
        match character {
            '"' => skip_json_string_literal(&mut chars)?,
            value if value == open => depth = depth.saturating_add(1),
            value if value == close => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }

    None
}

fn json_value_tail_is_delimited(trailing: &str) -> bool {
    trailing.is_empty() || matches!(trailing.as_bytes().first(), Some(b',' | b'}' | b']'))
}

fn skip_json_string_literal(
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
) -> Option<()> {
    let mut escaped = false;
    while let Some((_, character)) = chars.next() {
        if escaped {
            match character {
                '"' | '\\' | '/' | 'n' | 'r' | 't' | 'b' | 'f' => {}
                'u' => skip_json_unicode_escape(chars)?,
                _ => return None,
            }
            escaped = false;
            continue;
        }

        match character {
            '\\' => escaped = true,
            '"' => return Some(()),
            value if value.is_control() => return None,
            _ => {}
        }
    }
    None
}

fn skip_json_unicode_escape(
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
) -> Option<()> {
    let code = skip_json_hex_escape(chars)?;
    if (0xd800..=0xdbff).contains(&code) {
        if chars.next()?.1 != '\\' || chars.next()?.1 != 'u' {
            return None;
        }
        let low = skip_json_hex_escape(chars)?;
        if !(0xdc00..=0xdfff).contains(&low) {
            return None;
        }
    } else if (0xdc00..=0xdfff).contains(&code) {
        return None;
    }
    Some(())
}

fn skip_json_hex_escape(chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>) -> Option<u16> {
    let mut value = 0_u16;
    for _ in 0..4 {
        let digit = chars.next()?.1.to_digit(16)?;
        value = (value << 4) | u16::try_from(digit).ok()?;
    }
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_active_requests_with_braces_inside_prompt_preview() {
        let health = parse_provider_health(
            r#"{"ok":true,"active_requests":[{"request_id":7,"endpoint":"chat","elapsed_ms":42,"prompt_preview":"literal { brace } text"}]}"#,
        );

        assert_eq!(health.active_requests.len(), 1);
        assert_eq!(
            health.active_requests[0].prompt_preview.as_deref(),
            Some("literal { brace } text")
        );
    }

    #[test]
    fn rejects_active_requests_with_invalid_nested_string() {
        let health = parse_provider_health(
            "{\"ok\":true,\"active_requests\":[{\"prompt_preview\":\"bad\\q\"}]}",
        );

        assert!(health.active_requests.is_empty());
    }

    #[test]
    fn rejects_trailing_garbage_after_active_requests_array() {
        let health = parse_provider_health(
            r#"{"ok":true,"active_requests":[{"request_id":7}]x,"error":"still parsed"}"#,
        );

        assert!(health.active_requests.is_empty());
        assert_eq!(health.error.as_deref(), Some("still parsed"));
    }

    #[test]
    fn rejects_trailing_garbage_after_experience_hygiene_object() {
        let health = parse_provider_health(
            r#"{"ok":true,"experience_hygiene":{"checked":true}x,"error":"still parsed"}"#,
        );

        assert_eq!(health.experience_hygiene.checked, None);
        assert_eq!(health.error.as_deref(), Some("still parsed"));
    }

    #[test]
    fn top_level_error_ignores_string_value_named_error() {
        let health = parse_provider_health(r#"{"ok":false,"message":"error","error":"actual"}"#);

        assert_eq!(health.error.as_deref(), Some("actual"));
    }

    #[test]
    fn top_level_error_rejects_invalid_string_before_field() {
        let health =
            parse_provider_health("{\"ok\":false,\"message\":\"bad\\q\",\"error\":\"actual\"}");

        assert_eq!(health.error, None);
    }
}
