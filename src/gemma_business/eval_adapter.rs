use crate::model_service::json::{
    json_bool_field, json_string_field, json_u64_field, service_json_string,
};
use norion_eval::{
    RootAdapterFailureEvidence, RootAdapterFailureKind, classify_root_adapter_failure,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RootBusinessCycleEvalProjection {
    pub(crate) evidence: RootAdapterFailureEvidence,
    pub(crate) failure_kind: RootAdapterFailureKind,
}

pub(crate) fn project_root_business_cycle_eval(
    health_body: Option<&str>,
    final_json: Option<&str>,
    error: Option<&str>,
) -> RootBusinessCycleEvalProjection {
    let evidence = root_adapter_failure_evidence(health_body, final_json, error);
    let failure_kind = classify_root_adapter_failure(&evidence);

    RootBusinessCycleEvalProjection {
        evidence,
        failure_kind,
    }
}

pub(crate) fn root_business_cycle_eval_json(
    projection: &RootBusinessCycleEvalProjection,
) -> String {
    let evidence = &projection.evidence;
    format!(
        "{{\"report_only\":true,\"failure_kind\":\"{}\",\"backend_8686_reachable\":{},\"prompt_gate_blocked\":{},\"final_json_present\":{},\"runtime_model_present\":{},\"runtime_tokens\":{},\"business_cycle_passed\":{},\"error\":{}}}",
        projection.failure_kind.as_code(),
        option_bool_json(evidence.backend_8686_reachable),
        evidence.prompt_gate_blocked,
        evidence.final_json_present,
        evidence.runtime_model_present,
        option_u64_json(evidence.runtime_tokens),
        option_bool_json(evidence.business_cycle_passed),
        option_string_json(evidence.error.as_deref()),
    )
}

fn root_adapter_failure_evidence(
    health_body: Option<&str>,
    final_json: Option<&str>,
    error: Option<&str>,
) -> RootAdapterFailureEvidence {
    let gemma_runtime_reachable =
        health_body.and_then(|body| json_bool_field(body, "gemma_runtime_reachable"));
    let readiness_ok = health_body.and_then(|body| json_bool_field(body, "readiness_ok"));
    let engine_busy = health_body
        .and_then(|body| json_bool_field(body, "engine_busy"))
        .unwrap_or(false);
    let prompt_gate_blocked = engine_busy || readiness_ok == Some(false);
    let final_json_present = final_json
        .map(|body| json_bool_field(body, "ok").unwrap_or(false))
        .unwrap_or(false);
    let generate_body = final_json.and_then(|body| object_body(body, "generate"));
    let runtime_model_present = generate_body
        .and_then(|body| json_string_field(body, "runtime_model"))
        .map(|model| !model.trim().is_empty())
        .unwrap_or(false);
    let runtime_tokens = generate_body.and_then(|body| json_u64_field(body, "runtime_token_count"));
    let business_cycle_body = final_json.and_then(|body| object_body(body, "business_cycle"));
    let business_cycle_passed =
        business_cycle_body.and_then(|body| json_bool_field(body, "passed"));

    RootAdapterFailureEvidence {
        backend_8686_reachable: gemma_runtime_reachable,
        prompt_gate_blocked,
        final_json_present,
        runtime_model_present,
        runtime_tokens,
        business_cycle_passed,
        error: error.map(ToOwned::to_owned),
    }
}

fn object_body<'a>(body: &'a str, field: &str) -> Option<&'a str> {
    let needle = format!("\"{field}\"");
    let after_field = body.get(body.find(&needle)? + needle.len()..)?;
    let after_colon = after_field.get(after_field.find(':')? + 1..)?.trim_start();
    let mut chars = after_colon.char_indices();
    if chars.next()?.1 != '{' {
        return None;
    }

    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, character) in after_colon.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }

        match character {
            '"' => in_string = true,
            '{' => depth = depth.saturating_add(1),
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return after_colon.get(..=index);
                }
            }
            _ => {}
        }
    }
    None
}

fn option_bool_json(value: Option<bool>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

fn option_u64_json(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

fn option_string_json(value: Option<&str>) -> String {
    value
        .map(service_json_string)
        .unwrap_or_else(|| "null".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    const READY_HEALTH: &str = r#"{
        "engine_busy": false,
        "gemma_runtime_reachable": true,
        "readiness_ok": true,
        "safe_device_ok": true
    }"#;

    #[test]
    fn prompt_gate_blocked_outage_is_chain_not_ready() {
        let projection = project_root_business_cycle_eval(
            Some(
                r#"{
                    "engine_busy": false,
                    "gemma_runtime_reachable": false,
                    "readiness_ok": false,
                    "safe_device_ok": true
                }"#,
            ),
            None,
            Some("prompt gate blocked before generation"),
        );

        assert_eq!(projection.evidence.backend_8686_reachable, Some(false));
        assert!(projection.evidence.prompt_gate_blocked);
        assert_eq!(
            projection.failure_kind,
            RootAdapterFailureKind::ChainNotReady
        );
    }

    #[test]
    fn unreachable_runtime_after_ready_gate_is_model_unavailable() {
        let projection = project_root_business_cycle_eval(
            Some(
                r#"{
                    "engine_busy": false,
                    "gemma_runtime_reachable": false,
                    "readiness_ok": true,
                    "safe_device_ok": true
                }"#,
            ),
            None,
            Some("runtime connection refused"),
        );

        assert!(!projection.evidence.prompt_gate_blocked);
        assert_eq!(
            projection.failure_kind,
            RootAdapterFailureKind::ModelUnavailable
        );
    }

    #[test]
    fn final_business_cycle_json_projects_clean_runtime_evidence() {
        let projection = project_root_business_cycle_eval(
            Some(READY_HEALTH),
            Some(
                r#"{
                    "ok": true,
                    "request_id": 9,
                    "business_cycle": {"passed": true, "feedback_applied": 3},
                    "generate": {
                        "elapsed_ms": 241,
                        "runtime_model": "google/gemma-4-12B-it",
                        "runtime_token_count": 512,
                        "answer": "ok"
                    }
                }"#,
            ),
            None,
        );

        assert_eq!(projection.evidence.runtime_tokens, Some(512));
        assert!(projection.evidence.runtime_model_present);
        assert_eq!(projection.evidence.business_cycle_passed, Some(true));
        assert_eq!(projection.failure_kind, RootAdapterFailureKind::None);
    }

    #[test]
    fn model_quality_failure_requires_final_json_runtime_and_business_failure() {
        let projection = project_root_business_cycle_eval(
            Some(READY_HEALTH),
            Some(
                r#"{
                    "ok": true,
                    "business_cycle": {"passed": false},
                    "generate": {
                        "runtime_model": "google/gemma-4-12B-it",
                        "runtime_token_count": 128
                    }
                }"#,
            ),
            Some("business gate failed"),
        );

        assert!(!projection.evidence.prompt_gate_blocked);
        assert_eq!(
            projection.failure_kind,
            RootAdapterFailureKind::ModelQualityFailure
        );
    }

    #[test]
    fn object_body_ignores_nested_braces_inside_strings() {
        let body = r#"{"generate":{"answer":"brace } kept","runtime_token_count":3},"x":1}"#;

        assert_eq!(
            object_body(body, "generate"),
            Some(r#"{"answer":"brace } kept","runtime_token_count":3}"#)
        );
    }

    #[test]
    fn eval_json_is_report_only_and_uses_stable_failure_code() {
        let projection = project_root_business_cycle_eval(
            Some(
                r#"{
                    "engine_busy": false,
                    "gemma_runtime_reachable": false,
                    "readiness_ok": false
                }"#,
            ),
            None,
            Some("backend said \"not ready\""),
        );

        let json = root_business_cycle_eval_json(&projection);

        assert!(json.contains("\"report_only\":true"));
        assert!(json.contains("\"failure_kind\":\"chain_not_ready\""));
        assert!(json.contains("\"backend_8686_reachable\":false"));
        assert!(json.contains("\"prompt_gate_blocked\":true"));
        assert!(json.contains("\"error\":\"backend said \\\"not ready\\\"\""));
    }
}
