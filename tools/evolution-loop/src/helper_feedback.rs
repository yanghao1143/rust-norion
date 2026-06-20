use std::collections::BTreeMap;

use crate::json::{
    json_object_field, json_string, json_string_array, parse_json_object_map,
    parse_json_string_map, preview_text,
};

pub(crate) const HELPER_STAGE_FEEDBACK_PREFIX: &str = "pool_stage_call_answer ";
pub(crate) const MAX_HELPER_STAGE_FEEDBACK_CHARS: usize = 1200;

pub(crate) fn meta_entries(meta: &[String]) -> Vec<String> {
    meta.iter()
        .filter(|entry| entry.starts_with(HELPER_STAGE_FEEDBACK_PREFIX))
        .map(|entry| sanitize_feedback_item(&preview_text(entry, MAX_HELPER_STAGE_FEEDBACK_CHARS)))
        .collect()
}

pub(crate) fn sanitize_feedback_by_role(
    feedback_by_role: BTreeMap<String, Vec<String>>,
) -> BTreeMap<String, Vec<String>> {
    feedback_by_role
        .into_iter()
        .map(|(role, feedback)| {
            (
                role,
                feedback
                    .into_iter()
                    .map(|item| sanitize_feedback_item(&item))
                    .collect(),
            )
        })
        .collect()
}

pub(crate) fn feedback_by_role_from_meta(meta: &[String]) -> BTreeMap<String, Vec<String>> {
    let mut feedback_by_role = BTreeMap::<String, Vec<String>>::new();
    for entry in meta {
        let Some((role, feedback)) = feedback_entry(entry) else {
            continue;
        };
        feedback_by_role.entry(role).or_default().push(feedback);
    }
    feedback_by_role
}

pub(crate) fn feedback_by_role_json(meta: &[String]) -> String {
    let items = feedback_by_role_from_meta(meta)
        .iter()
        .map(|(role, feedback)| format!("{}:{}", json_string(role), json_string_array(feedback)))
        .collect::<Vec<_>>()
        .join(",");
    format!("{{{items}}}")
}

pub(crate) fn contract_by_role_json(meta: &[String]) -> String {
    let items = feedback_by_role_from_meta(meta)
        .iter()
        .map(|(role, feedback)| {
            format!(
                "{}:{{\"fields\":{},\"matched_markers\":{},\"expected_markers\":{}}}",
                json_string(role),
                string_map_json(&contract_fields(role, feedback)),
                json_string_array(&matched_contract_markers(role, feedback)),
                json_string_array(
                    &contract_markers(role)
                        .iter()
                        .map(|marker| (*marker).to_owned())
                        .collect::<Vec<_>>()
                )
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("{{{items}}}")
}

pub(crate) fn contract_fields_by_role_from_json(
    object: &str,
) -> BTreeMap<String, BTreeMap<String, String>> {
    parse_json_object_map(object)
        .into_iter()
        .filter_map(|(role, contract)| {
            let fields = json_object_field(&contract, "fields")
                .map(|fields| parse_json_string_map(&fields))
                .unwrap_or_default();
            (!fields.is_empty()).then_some((role, fields))
        })
        .collect()
}

#[cfg(test)]
pub(crate) fn contract_markers_by_role_from_json(
    object: &str,
    marker_field: &str,
) -> BTreeMap<String, Vec<String>> {
    parse_json_object_map(object)
        .into_iter()
        .filter_map(|(role, contract)| {
            let markers = crate::json::json_array_field(&contract, marker_field)
                .map(|array| crate::json::parse_json_string_array(&array))
                .unwrap_or_default();
            (!markers.is_empty()).then_some((role, markers))
        })
        .collect()
}

pub(crate) fn feedback_entry(entry: &str) -> Option<(String, String)> {
    let body = entry.strip_prefix(HELPER_STAGE_FEEDBACK_PREFIX)?.trim();
    let role = meta_value(body, "role")
        .or_else(|| meta_value(body, "task_kind"))
        .unwrap_or_else(|| "unknown".to_owned());
    let task_kind = meta_value(body, "task_kind").unwrap_or_else(|| role.clone());
    let elapsed_ms = meta_value(body, "elapsed_ms").unwrap_or_else(|| "?".to_owned());
    let answer_approx_tokens =
        meta_value(body, "answer_approx_tokens").unwrap_or_else(|| "?".to_owned());
    let preview = body
        .split_once(" preview=")
        .map(|(_, preview)| preview.trim())
        .filter(|preview| !preview.is_empty())
        .unwrap_or(body);
    let preview = sanitize_helper_stage_preview(preview);
    let feedback = format!(
        "task_kind={} elapsed_ms={} answer_approx_tokens={} preview={}",
        task_kind,
        elapsed_ms,
        answer_approx_tokens,
        preview_text(&preview, MAX_HELPER_STAGE_FEEDBACK_CHARS)
    );
    Some((
        role,
        preview_text(&feedback, MAX_HELPER_STAGE_FEEDBACK_CHARS),
    ))
}

pub(crate) fn sanitize_feedback_item(feedback: &str) -> String {
    if let Some((metadata, preview)) = feedback.split_once(" preview=") {
        let preview = sanitize_helper_stage_preview(preview);
        format!("{metadata} preview={preview}")
    } else {
        sanitize_helper_stage_preview(feedback)
    }
}

fn sanitize_helper_stage_preview(preview: &str) -> String {
    preview_text(preview, MAX_HELPER_STAGE_FEEDBACK_CHARS)
        .split(" / ")
        .map(str::trim)
        .filter(|segment| !is_markdown_fence_segment(segment))
        .collect::<Vec<_>>()
        .join(" / ")
}

fn is_markdown_fence_segment(segment: &str) -> bool {
    trim_contract_bullet(segment).starts_with("```")
}

pub(crate) fn latest_feedback_preview(feedback: &[String]) -> Option<String> {
    feedback
        .iter()
        .rev()
        .map(|item| feedback_preview(item))
        .find(|preview| !preview.is_empty())
        .map(|preview| preview_text(preview, MAX_HELPER_STAGE_FEEDBACK_CHARS))
}

pub(crate) fn matched_contract_markers(role: &str, feedback: &[String]) -> Vec<String> {
    contract_markers(role)
        .iter()
        .filter(|marker| {
            feedback
                .iter()
                .map(|item| feedback_preview(item))
                .any(|preview| contract_field_for_role(role, preview, marker).is_some())
        })
        .map(|marker| (*marker).to_owned())
        .collect()
}

pub(crate) fn contract_fields(role: &str, feedback: &[String]) -> BTreeMap<String, String> {
    let mut fields = BTreeMap::new();
    for preview in feedback.iter().map(|item| feedback_preview(item)) {
        for marker in contract_markers(role) {
            if let Some(value) = contract_field_for_role(role, preview, marker) {
                fields.insert(
                    (*marker).to_owned(),
                    preview_text(&value, MAX_HELPER_STAGE_FEEDBACK_CHARS),
                );
            }
        }
    }
    fields
}

pub(crate) fn feedback_preview(feedback: &str) -> &str {
    feedback
        .split_once(" preview=")
        .map(|(_, preview)| preview.trim())
        .unwrap_or(feedback.trim())
}

pub(crate) fn contract_markers(role: &str) -> &'static [&'static str] {
    match role {
        "summary" => &["memory_update", "next_context", "duplicate_guard"],
        "router" => &["route_intent", "tool_call", "preflight"],
        "review" => &["risk", "change_request", "verification"],
        "test-gate" => &["verdict", "validation_command", "failure_kind"],
        "index" => &[
            "clean_gist",
            "tags",
            "dependency_link",
            "source_origin",
            "validation_timestamp",
            "retention",
        ],
        _ => &["observation", "next_action", "verification"],
    }
}

fn contract_field_for_role(role: &str, text: &str, field: &str) -> Option<String> {
    contract_field(text, field, contract_field_allows_none(role, field))
}

fn contract_field_allows_none(role: &str, field: &str) -> bool {
    matches!(
        (role, field),
        ("review", "risk") | ("test-gate", "failure_kind")
    )
}

fn contract_field(text: &str, field: &str, allow_none: bool) -> Option<String> {
    let field = field.to_ascii_lowercase();
    for line in text.lines() {
        for segment in line.split(" / ") {
            let candidate = trim_contract_bullet(segment);
            let lower = candidate.to_ascii_lowercase();
            if !lower.starts_with(&field) {
                continue;
            }
            let Some(after_field) = candidate.get(field.len()..) else {
                continue;
            };
            let after_separator = after_field.trim_start();
            let Some(value_body) = after_separator
                .strip_prefix(':')
                .or_else(|| after_separator.strip_prefix('='))
                .or_else(|| after_separator.strip_prefix('-'))
            else {
                continue;
            };
            let value = value_body
                .split(" ; ")
                .next()
                .unwrap_or_default()
                .trim()
                .trim_matches(|character| matches!(character, '"' | '\''));
            if !value.is_empty() && (allow_none || !value.eq_ignore_ascii_case("none")) {
                return Some(value.to_owned());
            }
        }
    }
    None
}

fn trim_contract_bullet(text: &str) -> &str {
    text.trim()
        .strip_prefix("- ")
        .or_else(|| text.trim().strip_prefix("* "))
        .unwrap_or_else(|| text.trim())
}

fn meta_value(body: &str, key: &str) -> Option<String> {
    let metadata = body
        .split_once(" preview=")
        .map(|(metadata, _)| metadata)
        .unwrap_or(body);
    let needle = format!("{key}=");
    let start = metadata.find(&needle)? + needle.len();
    let value = metadata.get(start..)?.split_whitespace().next()?.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

fn string_map_json(values: &BTreeMap<String, String>) -> String {
    let items = values
        .iter()
        .map(|(key, value)| format!("{}:{}", json_string(key), json_string(value)))
        .collect::<Vec<_>>()
        .join(",");
    format!("{{{items}}}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexes_helper_feedback_by_role_from_meta() {
        let meta = vec![
            "pool_stage_call_answer task_kind=summary role=summary elapsed_ms=111 answer_approx_tokens=4 preview=memory_update: keep Metal evidence".to_owned(),
            "pool_stage_call_answer task_kind=test-gate role=test-gate elapsed_ms=222 answer_approx_tokens=8 preview=validation_command: cargo test".to_owned(),
            "pool_stage_call_skipped task_kind=review role=review reason=busy".to_owned(),
        ];

        let by_role = feedback_by_role_from_meta(&meta);
        let json = feedback_by_role_json(&meta);
        let contract_json = contract_by_role_json(&meta);

        assert_eq!(by_role.len(), 2);
        assert!(by_role["summary"][0].contains("memory_update: keep Metal evidence"));
        assert!(json.contains("\"summary\":[\"task_kind=summary elapsed_ms=111"));
        assert!(!json.contains("\"review\""));
        assert!(contract_json.contains("\"summary\":{\"fields\":{\"memory_update\""));
        assert!(contract_json.contains("\"expected_markers\":[\"memory_update\""));
    }

    #[test]
    fn sanitizes_markdown_fence_wrapped_helper_feedback() {
        let meta = vec![
            "pool_stage_call_answer task_kind=summary role=summary elapsed_ms=111 answer_approx_tokens=4 preview=```python / memory_update: keep Metal evidence / next_context: preserve stage evidence / duplicate_guard: do not emit code fences / ```"
                .to_owned(),
        ];

        let by_role = feedback_by_role_from_meta(&meta);
        let feedback = &by_role["summary"][0];
        let fields = contract_fields("summary", &by_role["summary"]);

        assert!(!feedback.contains("```"));
        assert_eq!(
            fields.get("memory_update").map(String::as_str),
            Some("keep Metal evidence")
        );
        assert_eq!(
            fields.get("duplicate_guard").map(String::as_str),
            Some("do not emit code fences")
        );
    }

    #[test]
    fn parses_contract_fields_by_role_from_json() {
        let parsed = contract_fields_by_role_from_json(
            r#"{"review":{"fields":{"risk":"stale","verification":"cargo test"},"matched_markers":["risk","verification"]},"summary":{"fields":{}}}"#,
        );
        let markers = contract_markers_by_role_from_json(
            r#"{"review":{"fields":{"risk":"stale"},"matched_markers":["risk"],"expected_markers":["risk","change_request","verification"]}}"#,
            "expected_markers",
        );

        assert_eq!(
            parsed
                .get("review")
                .and_then(|fields| fields.get("risk"))
                .map(String::as_str),
            Some("stale")
        );
        assert!(!parsed.contains_key("summary"));
        assert_eq!(
            markers.get("review"),
            Some(&vec![
                "risk".to_owned(),
                "change_request".to_owned(),
                "verification".to_owned()
            ])
        );
    }

    #[test]
    fn contract_fields_parse_bullet_feedback() {
        let feedback = vec![
            "task_kind=review preview=- risk: stale index feedback\n- change_request: persist helper fields\n- verification: cargo test -q --manifest-path tools/evolution-loop/Cargo.toml"
                .to_owned(),
        ];
        let fields = contract_fields("review", &feedback);

        assert_eq!(
            fields.get("risk").map(String::as_str),
            Some("stale index feedback")
        );
        assert_eq!(
            fields.get("change_request").map(String::as_str),
            Some("persist helper fields")
        );
        assert_eq!(
            fields.get("verification").map(String::as_str),
            Some("cargo test -q --manifest-path tools/evolution-loop/Cargo.toml")
        );
        assert_eq!(
            matched_contract_markers("review", &feedback),
            vec![
                "risk".to_owned(),
                "change_request".to_owned(),
                "verification".to_owned()
            ]
        );
    }

    #[test]
    fn contract_fields_accept_explicit_no_risk_review_feedback() {
        let feedback = vec![
            "task_kind=review preview=risk: None / change_request: keep current worker routing / verification: cargo test -q --manifest-path tools/evolution-loop/Cargo.toml"
                .to_owned(),
        ];
        let fields = contract_fields("review", &feedback);

        assert_eq!(fields.get("risk").map(String::as_str), Some("None"));
        assert_eq!(
            matched_contract_markers("review", &feedback),
            vec![
                "risk".to_owned(),
                "change_request".to_owned(),
                "verification".to_owned()
            ]
        );
    }

    #[test]
    fn contract_fields_parse_index_dependency_link_feedback() {
        let feedback = vec![
            "task_kind=index preview=clean_gist: stable tags are searchable\ntags: role=index;case=case-42;round=42;primary=present;final_json=present;dependency=review.change_request;source_origin=review.change_request;validation_timestamp=1781770123\ndependency_link: review.change_request\nsource_origin: review.change_request\nvalidation_timestamp: 1781770123\nretention: keep; compact retrieval evidence"
                .to_owned(),
        ];
        let fields = contract_fields("index", &feedback);

        assert_eq!(
            contract_markers("index"),
            &[
                "clean_gist",
                "tags",
                "dependency_link",
                "source_origin",
                "validation_timestamp",
                "retention"
            ]
        );
        assert_eq!(
            fields.get("clean_gist").map(String::as_str),
            Some("stable tags are searchable")
        );
        assert_eq!(
            fields.get("tags").map(String::as_str),
            Some(
                "role=index;case=case-42;round=42;primary=present;final_json=present;dependency=review.change_request;source_origin=review.change_request;validation_timestamp=1781770123"
            )
        );
        assert_eq!(
            fields.get("dependency_link").map(String::as_str),
            Some("review.change_request")
        );
        assert_eq!(
            fields.get("source_origin").map(String::as_str),
            Some("review.change_request")
        );
        assert_eq!(
            fields.get("validation_timestamp").map(String::as_str),
            Some("1781770123")
        );
        assert_eq!(
            fields.get("retention").map(String::as_str),
            Some("keep; compact retrieval evidence")
        );
        assert_eq!(
            matched_contract_markers("index", &feedback),
            vec![
                "clean_gist".to_owned(),
                "tags".to_owned(),
                "dependency_link".to_owned(),
                "source_origin".to_owned(),
                "validation_timestamp".to_owned(),
                "retention".to_owned()
            ]
        );
    }

    #[test]
    fn matched_contract_markers_require_field_boundary() {
        let feedback = vec![
            "task_kind=index preview=clean_gist: validation_timestamp is only a tag label\ntags: role=index;case=case-42;round=42;primary=present;final_json=present;dependency=review.change_request;validation_timestamp=1781770123\ndependency_link: review.change_request\nretention: keep"
                .to_owned(),
        ];

        assert_eq!(
            matched_contract_markers("index", &feedback),
            vec![
                "clean_gist".to_owned(),
                "tags".to_owned(),
                "dependency_link".to_owned(),
                "retention".to_owned()
            ]
        );
    }

    #[test]
    fn contract_fields_parse_router_feedback() {
        let feedback = vec![
            "task_kind=router preview=- route_intent: summary\n- tool_call: {\"name\":\"summarize\"}\n- preflight: allow because request is read-only"
                .to_owned(),
        ];
        let fields = contract_fields("router", &feedback);

        assert_eq!(
            fields.get("route_intent").map(String::as_str),
            Some("summary")
        );
        assert_eq!(
            fields.get("tool_call").map(String::as_str),
            Some("{\"name\":\"summarize\"}")
        );
        assert_eq!(
            fields.get("preflight").map(String::as_str),
            Some("allow because request is read-only")
        );
        assert_eq!(
            matched_contract_markers("router", &feedback),
            vec![
                "route_intent".to_owned(),
                "tool_call".to_owned(),
                "preflight".to_owned()
            ]
        );
    }

    #[test]
    fn contract_fields_keep_test_gate_failure_kind_none() {
        let feedback = vec![
            "task_kind=test-gate preview=* verdict: pass / * validation_command: cargo test -q --manifest-path tools/evolution-loop/Cargo.toml / * failure_kind: none"
                .to_owned(),
        ];
        let fields = contract_fields("test-gate", &feedback);

        assert_eq!(fields.get("verdict").map(String::as_str), Some("pass"));
        assert_eq!(
            fields.get("validation_command").map(String::as_str),
            Some("cargo test -q --manifest-path tools/evolution-loop/Cargo.toml")
        );
        assert_eq!(fields.get("failure_kind").map(String::as_str), Some("none"));
        assert_eq!(
            matched_contract_markers("test-gate", &feedback),
            vec![
                "verdict".to_owned(),
                "validation_command".to_owned(),
                "failure_kind".to_owned()
            ]
        );
    }

    #[test]
    fn feedback_preview_keeps_late_contract_fields_after_verbose_helper_answer() {
        let verbose_memory = "role separation evidence ".repeat(18);
        let entry = format!(
            "pool_stage_call_answer task_kind=summary role=summary elapsed_ms=111 answer_approx_tokens=99 preview=* memory_update: {verbose_memory} / * next_context: keep E2B for summary and index while E4B handles review and test-gate / * duplicate_guard: do not route simple summary work to the 12B quality model"
        );

        let (_role, feedback) = feedback_entry(&entry).expect("feedback entry");
        let fields = contract_fields("summary", &[feedback]);

        assert_eq!(
            fields.get("duplicate_guard").map(String::as_str),
            Some("do not route simple summary work to the 12B quality model")
        );
        assert_eq!(
            matched_contract_markers("summary", &[entry]),
            vec![
                "memory_update".to_owned(),
                "next_context".to_owned(),
                "duplicate_guard".to_owned()
            ]
        );
    }
}
