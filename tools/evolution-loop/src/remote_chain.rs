use std::fs;
use std::path::Path;

use crate::json::{
    json_array_field, json_bool_field, json_object_field, json_string, json_string_array,
    json_string_field, json_u64_field, parse_json_string_array,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RemoteChainStatusSummary {
    pub(crate) contract_version: Option<String>,
    pub(crate) ready: Option<bool>,
    pub(crate) model_api: Option<bool>,
    pub(crate) backend: Option<bool>,
    pub(crate) backend_model: Option<String>,
    pub(crate) web_lab: Option<bool>,
    pub(crate) quality_worker: Option<bool>,
    pub(crate) quality_model_cache_name: Option<String>,
    pub(crate) model_pool_launch_allowed: Option<bool>,
    pub(crate) capacity_expansion_allowed: Option<bool>,
    pub(crate) required_roles_ready: Option<bool>,
    pub(crate) model_pool_available: Option<bool>,
    pub(crate) model_pool_reason: Option<String>,
    pub(crate) worker_count: Option<u64>,
    pub(crate) healthy_worker_count: Option<u64>,
    pub(crate) min_context_tokens: Option<u64>,
    pub(crate) required_roles: Vec<String>,
    pub(crate) missing_required_roles: Vec<String>,
    pub(crate) capacity_recommendation: Option<String>,
    pub(crate) model_cache_present: bool,
    pub(crate) model_cache_exists: Option<bool>,
    pub(crate) model_cache_all_ok: Option<bool>,
    pub(crate) model_cache_ok_count: Option<u64>,
    pub(crate) model_cache_model_count: Option<u64>,
    pub(crate) model_cache_remote_error_count: Option<u64>,
    pub(crate) model_cache_read_only: Option<bool>,
    pub(crate) model_cache_path: Option<String>,
    pub(crate) remote_runtime_present: bool,
    pub(crate) remote_runtime_probed: Option<bool>,
    pub(crate) remote_runtime_touches_remote: Option<bool>,
    pub(crate) remote_runtime_worker_count: Option<u64>,
    pub(crate) remote_runtime_cpu_or_no_gpu_count: Option<u64>,
    pub(crate) remote_runtime_cpu_or_no_gpu_roles: Vec<String>,
    pub(crate) remote_runtime_backend_metadata_may_differ_roles: Vec<String>,
    pub(crate) remote_runtime_acceleration_ok: Option<bool>,
    pub(crate) remote_runtime_next_step: Option<String>,
    pub(crate) remote_runtime_error: Option<String>,
    pub(crate) next_step: Option<String>,
}

pub(crate) fn load_status(path: Option<&Path>) -> Result<Option<RemoteChainStatusSummary>, String> {
    let Some(path) = path else {
        return Ok(None);
    };
    let text = fs::read_to_string(path).map_err(|error| {
        format!(
            "read remote chain status JSON {} failed: {error}",
            path.display()
        )
    })?;
    if text.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(parse_status(&text)))
}

pub(crate) fn context(path: &Path) -> Result<Option<String>, String> {
    let Some(summary) = load_status(Some(path))? else {
        return Ok(None);
    };
    Ok(Some(context_text(&summary)))
}

pub(crate) fn gate_failure(summary: &RemoteChainStatusSummary) -> Option<String> {
    let context = context_text(summary);
    let mut failures = Vec::new();
    match summary.ready {
        Some(true) => {}
        Some(false) => failures.push("ready=false".to_owned()),
        None => failures.push("ready missing".to_owned()),
    }
    if summary.model_pool_launch_allowed == Some(false) {
        failures.push("model_pool_launch_allowed=false".to_owned());
    }
    if summary.capacity_expansion_allowed == Some(false) {
        failures.push("capacity_expansion_allowed=false".to_owned());
    }
    if summary.required_roles_ready == Some(false) {
        failures.push(format!(
            "required_roles_ready=false missing_required_roles={}",
            string_list_text(&summary.missing_required_roles)
        ));
    }
    if summary.model_cache_present {
        if summary.model_cache_exists == Some(false) {
            failures.push("model_cache missing".to_owned());
        }
        if summary.model_cache_all_ok == Some(false) {
            failures.push("model_cache_all_ok=false".to_owned());
        }
        if summary
            .model_cache_remote_error_count
            .is_some_and(|count| count > 0)
        {
            failures.push(format!(
                "model_cache_remote_errors={}",
                option_u64_text(summary.model_cache_remote_error_count)
            ));
        }
    }
    if let Some(failure) = model_identity_failure(summary) {
        failures.push(failure);
    }
    if summary.remote_runtime_present {
        match summary.remote_runtime_probed {
            Some(true) => {}
            Some(false) => failures.push("remote_runtime_probed=false".to_owned()),
            None => failures.push("remote_runtime_probed missing".to_owned()),
        }
        if summary.remote_runtime_acceleration_ok == Some(false) {
            failures.push(format!(
                "remote_runtime_acceleration_ok=false next_step={}",
                summary
                    .remote_runtime_next_step
                    .as_deref()
                    .unwrap_or("refresh-runtime-probe")
            ));
        }
        if summary
            .remote_runtime_cpu_or_no_gpu_count
            .is_some_and(|count| count > 0)
        {
            failures.push(format!(
                "remote_runtime_cpu_or_no_gpu={} roles={}",
                option_u64_text(summary.remote_runtime_cpu_or_no_gpu_count),
                string_list_text(&summary.remote_runtime_cpu_or_no_gpu_roles)
            ));
        }
    }
    if failures.is_empty() {
        None
    } else {
        Some(format!("{}; {context}", failures.join("; ")))
    }
}

pub(crate) fn context_text(summary: &RemoteChainStatusSummary) -> String {
    format!(
        "ready:{} model_api:{} backend:{} backend_model:{} web_lab:{} quality_worker:{} quality_model_cache_name:{} launch_allowed:{} capacity_expansion_allowed:{} required_roles_ready:{} required_roles:{} missing_required_roles:{} pool_available:{} pool_reason:{} workers:{}/{} min_context_tokens:{} capacity_recommendation:{} model_cache_exists:{} model_cache_all_ok:{} model_cache_ok:{}/{} model_cache_remote_errors:{} model_cache_read_only:{} model_cache_path:{} remote_runtime_probed:{} remote_runtime_workers:{} remote_runtime_cpu_or_no_gpu:{} remote_runtime_cpu_or_no_gpu_roles:{} remote_runtime_backend_metadata_may_differ_roles:{} remote_runtime_acceleration_ok:{} remote_runtime_next_step:{} remote_runtime_error:{} next_step:{}",
        option_bool_text(summary.ready),
        option_bool_text(summary.model_api),
        option_bool_text(summary.backend),
        summary.backend_model.as_deref().unwrap_or("none"),
        option_bool_text(summary.web_lab),
        option_bool_text(summary.quality_worker),
        summary
            .quality_model_cache_name
            .as_deref()
            .unwrap_or("none"),
        option_bool_text(summary.model_pool_launch_allowed),
        option_bool_text(summary.capacity_expansion_allowed),
        option_bool_text(summary.required_roles_ready),
        string_list_text(&summary.required_roles),
        string_list_text(&summary.missing_required_roles),
        option_bool_text(summary.model_pool_available),
        summary.model_pool_reason.as_deref().unwrap_or("none"),
        option_u64_text(summary.healthy_worker_count),
        option_u64_text(summary.worker_count),
        option_u64_text(summary.min_context_tokens),
        summary.capacity_recommendation.as_deref().unwrap_or("none"),
        option_bool_text(summary.model_cache_exists),
        option_bool_text(summary.model_cache_all_ok),
        option_u64_text(summary.model_cache_ok_count),
        option_u64_text(summary.model_cache_model_count),
        option_u64_text(summary.model_cache_remote_error_count),
        option_bool_text(summary.model_cache_read_only),
        summary.model_cache_path.as_deref().unwrap_or("none"),
        option_bool_text(summary.remote_runtime_probed),
        option_u64_text(summary.remote_runtime_worker_count),
        option_u64_text(summary.remote_runtime_cpu_or_no_gpu_count),
        string_list_text(&summary.remote_runtime_cpu_or_no_gpu_roles),
        string_list_text(&summary.remote_runtime_backend_metadata_may_differ_roles),
        option_bool_text(summary.remote_runtime_acceleration_ok),
        summary
            .remote_runtime_next_step
            .as_deref()
            .unwrap_or("none"),
        summary.remote_runtime_error.as_deref().unwrap_or("none"),
        summary.next_step.as_deref().unwrap_or("none")
    )
}

pub(crate) fn option_status_json(summary: Option<&RemoteChainStatusSummary>) -> String {
    summary
        .map(status_json)
        .unwrap_or_else(|| "null".to_owned())
}

pub(crate) fn parse_status(text: &str) -> RemoteChainStatusSummary {
    let readiness = top_level_object_field(text, "readiness").unwrap_or_default();
    let backend = top_level_object_field(text, "backend").unwrap_or_default();
    let model_pool = top_level_object_field(text, "model_pool").unwrap_or_default();
    let capacity = json_object_field(&model_pool, "capacity").unwrap_or_default();
    let model_cache = top_level_object_field(text, "model_cache");
    let model_cache_body = model_cache.as_deref().unwrap_or_default();
    let remote_runtime = top_level_object_field(text, "remote_runtime");
    let remote_runtime_body = remote_runtime.as_deref().unwrap_or_default();
    let required_roles = json_array_field(&model_pool, "required_roles")
        .map(|array| parse_json_string_array(&array))
        .unwrap_or_default();
    let missing_required_roles = json_array_field(&model_pool, "missing_required_roles")
        .map(|array| parse_json_string_array(&array))
        .unwrap_or_default();
    RemoteChainStatusSummary {
        contract_version: json_string_field(text, "contract_version"),
        ready: json_bool_field(&readiness, "ready"),
        model_api: json_bool_field(&readiness, "model_api"),
        backend: json_bool_field(&readiness, "backend"),
        backend_model: json_string_field(&backend, "model"),
        web_lab: json_bool_field(&readiness, "web_lab"),
        quality_worker: json_bool_field(&readiness, "quality_worker"),
        quality_model_cache_name: json_string_field(&model_pool, "quality_model_cache_name"),
        model_pool_launch_allowed: json_bool_field(&readiness, "model_pool_launch_allowed"),
        capacity_expansion_allowed: json_bool_field(&readiness, "capacity_expansion_allowed"),
        required_roles_ready: json_bool_field(&readiness, "required_roles_ready")
            .or_else(|| json_bool_field(&model_pool, "required_roles_ready")),
        model_pool_available: json_bool_field(&model_pool, "available"),
        model_pool_reason: json_string_field(&model_pool, "reason"),
        worker_count: json_u64_field(&model_pool, "worker_count"),
        healthy_worker_count: json_u64_field(&model_pool, "healthy_worker_count"),
        min_context_tokens: json_u64_field(&model_pool, "min_context_tokens"),
        required_roles,
        missing_required_roles,
        capacity_recommendation: json_string_field(&capacity, "recommendation"),
        model_cache_present: model_cache.is_some(),
        model_cache_exists: json_bool_field(model_cache_body, "exists"),
        model_cache_all_ok: json_bool_field(&readiness, "model_cache_all_ok")
            .or_else(|| json_bool_field(model_cache_body, "all_ok")),
        model_cache_ok_count: json_u64_field(model_cache_body, "ok_count"),
        model_cache_model_count: json_u64_field(model_cache_body, "model_count"),
        model_cache_remote_error_count: json_u64_field(model_cache_body, "remote_error_count"),
        model_cache_read_only: json_bool_field(model_cache_body, "read_only"),
        model_cache_path: json_string_field(model_cache_body, "path"),
        remote_runtime_present: remote_runtime.is_some(),
        remote_runtime_probed: json_bool_field(remote_runtime_body, "probed"),
        remote_runtime_touches_remote: json_bool_field(remote_runtime_body, "touches_remote"),
        remote_runtime_worker_count: json_u64_field(remote_runtime_body, "worker_count"),
        remote_runtime_cpu_or_no_gpu_count: json_u64_field(
            remote_runtime_body,
            "cpu_or_no_gpu_count",
        ),
        remote_runtime_cpu_or_no_gpu_roles: json_array_field(
            remote_runtime_body,
            "cpu_or_no_gpu_roles",
        )
        .map(|array| parse_json_string_array(&array))
        .unwrap_or_default(),
        remote_runtime_backend_metadata_may_differ_roles: json_array_field(
            remote_runtime_body,
            "backend_metadata_may_differ_roles",
        )
        .map(|array| parse_json_string_array(&array))
        .unwrap_or_default(),
        remote_runtime_acceleration_ok: json_bool_field(remote_runtime_body, "acceleration_ok"),
        remote_runtime_next_step: json_string_field(remote_runtime_body, "acceleration_next_step"),
        remote_runtime_error: json_string_field(remote_runtime_body, "error"),
        next_step: json_string_field(text, "next_step"),
    }
}

fn status_json(summary: &RemoteChainStatusSummary) -> String {
    format!(
        "{{\"contract_version\":{},\"readiness\":{{\"ready\":{},\"model_api\":{},\"backend\":{},\"web_lab\":{},\"quality_worker\":{},\"model_pool_launch_allowed\":{},\"capacity_expansion_allowed\":{},\"required_roles_ready\":{},\"model_cache_all_ok\":{}}},\"backend\":{{\"model\":{}}},\"model_pool\":{{\"available\":{},\"reason\":{},\"worker_count\":{},\"healthy_worker_count\":{},\"min_context_tokens\":{},\"quality_model_cache_name\":{},\"required_roles\":{},\"required_roles_ready\":{},\"missing_required_roles\":{},\"capacity_recommendation\":{}}},\"model_cache\":{},\"remote_runtime\":{},\"next_step\":{}}}",
        option_str_json(summary.contract_version.as_deref()),
        option_bool_json(summary.ready),
        option_bool_json(summary.model_api),
        option_bool_json(summary.backend),
        option_bool_json(summary.web_lab),
        option_bool_json(summary.quality_worker),
        option_bool_json(summary.model_pool_launch_allowed),
        option_bool_json(summary.capacity_expansion_allowed),
        option_bool_json(summary.required_roles_ready),
        option_bool_json(summary.model_cache_all_ok),
        option_str_json(summary.backend_model.as_deref()),
        option_bool_json(summary.model_pool_available),
        option_str_json(summary.model_pool_reason.as_deref()),
        option_u64_json(summary.worker_count),
        option_u64_json(summary.healthy_worker_count),
        option_u64_json(summary.min_context_tokens),
        option_str_json(summary.quality_model_cache_name.as_deref()),
        json_string_array(&summary.required_roles),
        option_bool_json(summary.required_roles_ready),
        json_string_array(&summary.missing_required_roles),
        option_str_json(summary.capacity_recommendation.as_deref()),
        model_cache_json(summary),
        remote_runtime_json(summary),
        option_str_json(summary.next_step.as_deref())
    )
}

fn model_cache_json(summary: &RemoteChainStatusSummary) -> String {
    if !summary.model_cache_present {
        return "null".to_owned();
    }
    format!(
        "{{\"exists\":{},\"all_ok\":{},\"ok_count\":{},\"model_count\":{},\"remote_error_count\":{},\"read_only\":{},\"path\":{}}}",
        option_bool_json(summary.model_cache_exists),
        option_bool_json(summary.model_cache_all_ok),
        option_u64_json(summary.model_cache_ok_count),
        option_u64_json(summary.model_cache_model_count),
        option_u64_json(summary.model_cache_remote_error_count),
        option_bool_json(summary.model_cache_read_only),
        option_str_json(summary.model_cache_path.as_deref())
    )
}

fn remote_runtime_json(summary: &RemoteChainStatusSummary) -> String {
    if !summary.remote_runtime_present {
        return "null".to_owned();
    }
    format!(
        "{{\"probed\":{},\"touches_remote\":{},\"worker_count\":{},\"cpu_or_no_gpu_count\":{},\"cpu_or_no_gpu_roles\":{},\"backend_metadata_may_differ_roles\":{},\"acceleration_ok\":{},\"acceleration_next_step\":{},\"error\":{}}}",
        option_bool_json(summary.remote_runtime_probed),
        option_bool_json(summary.remote_runtime_touches_remote),
        option_u64_json(summary.remote_runtime_worker_count),
        option_u64_json(summary.remote_runtime_cpu_or_no_gpu_count),
        json_string_array(&summary.remote_runtime_cpu_or_no_gpu_roles),
        json_string_array(&summary.remote_runtime_backend_metadata_may_differ_roles),
        option_bool_json(summary.remote_runtime_acceleration_ok),
        option_str_json(summary.remote_runtime_next_step.as_deref()),
        option_str_json(summary.remote_runtime_error.as_deref())
    )
}

fn option_bool_text(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "true",
        Some(false) => "false",
        None => "?",
    }
}

fn option_u64_text(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "?".to_owned())
}

fn string_list_text(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_owned()
    } else {
        values.join(",")
    }
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

fn option_str_json(value: Option<&str>) -> String {
    value.map(json_string).unwrap_or_else(|| "null".to_owned())
}

fn top_level_object_field(body: &str, field: &str) -> Option<String> {
    let needle = json_string(field);
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, character) in body.char_indices() {
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

        if depth == 1 && character == '"' && body.get(index..)?.starts_with(&needle) {
            let before_field = body
                .get(..index)?
                .chars()
                .rev()
                .find(|character| !character.is_whitespace());
            if before_field.is_none_or(|character| matches!(character, '{' | ',')) {
                let after_field = body.get(index + needle.len()..)?.trim_start();
                if let Some(after_colon) = after_field.strip_prefix(':') {
                    let value = after_colon.trim_start();
                    if let Some(object) = balanced_json_object(value) {
                        return Some(object.to_owned());
                    }
                }
            }
        }

        match character {
            '"' => in_string = true,
            '{' => depth = depth.saturating_add(1),
            '}' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    None
}

fn balanced_json_object(input: &str) -> Option<&str> {
    let mut chars = input.char_indices();
    if chars.next()?.1 != '{' {
        return None;
    }

    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, character) in input.char_indices() {
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
                    return input.get(..=index);
                }
            }
            _ => {}
        }
    }
    None
}

fn model_identity_failure(summary: &RemoteChainStatusSummary) -> Option<String> {
    let backend_model = non_empty(summary.backend_model.as_deref())?;
    let quality_model = non_empty(summary.quality_model_cache_name.as_deref())?;
    if comparable_model_name(backend_model) == comparable_model_name(quality_model) {
        return None;
    }
    Some(format!(
        "backend_model={} differs from quality_model_cache_name={}",
        backend_model, quality_model
    ))
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn comparable_model_name(value: &str) -> String {
    value
        .trim()
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(value)
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_remote_chain_status_context_and_gate() {
        let text = "{\"contract_version\":\"smartsteam.remote-gemma-chain.status.v1\",\"readiness\":{\"ready\":false,\"model_api\":false,\"backend\":true,\"web_lab\":true,\"quality_worker\":null,\"model_pool_launch_allowed\":null,\"capacity_expansion_allowed\":null},\"model_pool\":{\"available\":false,\"reason\":null,\"worker_count\":null,\"healthy_worker_count\":null,\"min_context_tokens\":null,\"capacity\":null},\"next_step\":\"start-remote\"}\n";
        let summary = parse_status(text);
        let context = context_text(&summary);
        let json = option_status_json(Some(&summary));
        let failure = gate_failure(&summary).unwrap();

        assert_eq!(summary.ready, Some(false));
        assert_eq!(summary.model_api, Some(false));
        assert_eq!(summary.backend, Some(true));
        assert_eq!(summary.web_lab, Some(true));
        assert_eq!(summary.next_step.as_deref(), Some("start-remote"));
        assert!(context.contains("ready:false"));
        assert!(context.contains("next_step:start-remote"));
        assert!(failure.contains("ready=false"));
        assert!(json.contains("\"contract_version\":\"smartsteam.remote-gemma-chain.status.v1\""));
        assert!(json.contains("\"ready\":false"));
    }

    #[test]
    fn gate_allows_ready_status() {
        let summary = parse_status(
            "{\"readiness\":{\"ready\":true,\"model_api\":true,\"backend\":true,\"web_lab\":true,\"quality_worker\":true,\"model_pool_launch_allowed\":true,\"capacity_expansion_allowed\":true},\"next_step\":\"ready\"}",
        );

        assert!(gate_failure(&summary).is_none());
    }

    #[test]
    fn gate_blocks_missing_required_pool_roles() {
        let summary = parse_status(
            "{\"readiness\":{\"ready\":true,\"model_api\":true,\"backend\":true,\"web_lab\":true,\"quality_worker\":true,\"model_pool_launch_allowed\":true,\"capacity_expansion_allowed\":true,\"required_roles_ready\":false},\"model_pool\":{\"required_roles\":[\"summary\",\"review\",\"test-gate\"],\"required_roles_ready\":false,\"missing_required_roles\":[\"review\",\"test-gate\"]},\"next_step\":\"start-review-workers\"}",
        );
        let context = context_text(&summary);
        let failure = gate_failure(&summary).unwrap();
        let json = option_status_json(Some(&summary));

        assert_eq!(
            summary.required_roles,
            vec![
                "summary".to_owned(),
                "review".to_owned(),
                "test-gate".to_owned()
            ]
        );
        assert_eq!(
            summary.missing_required_roles,
            vec!["review".to_owned(), "test-gate".to_owned()]
        );
        assert_eq!(summary.required_roles_ready, Some(false));
        assert!(context.contains("required_roles_ready:false"));
        assert!(context.contains("missing_required_roles:review,test-gate"));
        assert!(failure.contains("required_roles_ready=false"));
        assert!(failure.contains("missing_required_roles=review,test-gate"));
        assert!(json.contains("\"required_roles\":[\"summary\",\"review\",\"test-gate\"]"));
        assert!(json.contains("\"missing_required_roles\":[\"review\",\"test-gate\"]"));
    }

    #[test]
    fn parses_model_cache_provenance_into_context_and_json() {
        let summary = parse_status(
            "{\"readiness\":{\"ready\":true,\"model_api\":true,\"backend\":true,\"web_lab\":true,\"quality_worker\":true,\"model_pool_launch_allowed\":true,\"capacity_expansion_allowed\":true,\"model_cache_all_ok\":true},\"backend\":{\"model\":\"Qwen3.6-14B-A3B-FableVibes-Q4_K_M.gguf\"},\"model_pool\":{\"available\":true,\"worker_count\":6,\"healthy_worker_count\":6,\"quality_model_cache_name\":\"Qwen3.6-14B-A3B-FableVibes-Q4_K_M.gguf\"},\"model_cache\":{\"exists\":true,\"all_ok\":true,\"ok_count\":5,\"model_count\":5,\"remote_error_count\":0,\"read_only\":true,\"path\":\"D:\\\\rust-norion\\\\target\\\\remote-gemma-chain\\\\model-cache-status.json\"},\"remote_runtime\":{\"probed\":true,\"touches_remote\":true,\"worker_count\":6,\"cpu_or_no_gpu_count\":0,\"cpu_or_no_gpu_roles\":[],\"backend_metadata_may_differ_roles\":[],\"acceleration_ok\":true,\"acceleration_next_step\":\"\",\"error\":\"\"},\"next_step\":\"ready\"}",
        );
        let context = context_text(&summary);
        let json = option_status_json(Some(&summary));

        assert!(summary.model_cache_present);
        assert_eq!(summary.model_cache_exists, Some(true));
        assert_eq!(
            summary.backend_model.as_deref(),
            Some("Qwen3.6-14B-A3B-FableVibes-Q4_K_M.gguf")
        );
        assert_eq!(
            summary.quality_model_cache_name.as_deref(),
            Some("Qwen3.6-14B-A3B-FableVibes-Q4_K_M.gguf")
        );
        assert_eq!(summary.model_cache_all_ok, Some(true));
        assert_eq!(summary.model_cache_ok_count, Some(5));
        assert_eq!(summary.model_cache_model_count, Some(5));
        assert_eq!(summary.model_cache_remote_error_count, Some(0));
        assert_eq!(summary.model_cache_read_only, Some(true));
        assert!(summary.remote_runtime_present);
        assert_eq!(summary.remote_runtime_probed, Some(true));
        assert_eq!(summary.remote_runtime_touches_remote, Some(true));
        assert_eq!(summary.remote_runtime_worker_count, Some(6));
        assert_eq!(summary.remote_runtime_cpu_or_no_gpu_count, Some(0));
        assert_eq!(summary.remote_runtime_acceleration_ok, Some(true));
        assert!(gate_failure(&summary).is_none());
        assert!(context.contains("model_cache_exists:true"));
        assert!(context.contains("backend_model:Qwen3.6-14B-A3B-FableVibes-Q4_K_M.gguf"));
        assert!(
            context.contains("quality_model_cache_name:Qwen3.6-14B-A3B-FableVibes-Q4_K_M.gguf")
        );
        assert!(context.contains("model_cache_all_ok:true"));
        assert!(context.contains("model_cache_ok:5/5"));
        assert!(context.contains("model_cache_remote_errors:0"));
        assert!(context.contains("remote_runtime_probed:true"));
        assert!(context.contains("remote_runtime_workers:6"));
        assert!(context.contains("remote_runtime_cpu_or_no_gpu:0"));
        assert!(context.contains("remote_runtime_acceleration_ok:true"));
        assert!(json.contains("\"model_cache_all_ok\":true"));
        assert!(
            json.contains("\"backend\":{\"model\":\"Qwen3.6-14B-A3B-FableVibes-Q4_K_M.gguf\"}")
        );
        assert!(
            json.contains(
                "\"quality_model_cache_name\":\"Qwen3.6-14B-A3B-FableVibes-Q4_K_M.gguf\""
            )
        );
        assert!(json.contains("\"model_cache\":{\"exists\":true"));
        assert!(json.contains("\"ok_count\":5"));
        assert!(json.contains("\"remote_runtime\":{\"probed\":true"));
        assert!(json.contains("\"cpu_or_no_gpu_count\":0"));
        assert!(json.contains("\"acceleration_ok\":true"));
    }

    #[test]
    fn gate_blocks_remote_runtime_cpu_or_unprobed_status() {
        let cpu_summary = parse_status(
            "{\"readiness\":{\"ready\":true,\"model_api\":true,\"backend\":true,\"web_lab\":true,\"quality_worker\":true,\"model_pool_launch_allowed\":true,\"capacity_expansion_allowed\":true},\"model_pool\":{\"available\":true,\"worker_count\":6,\"healthy_worker_count\":6},\"remote_runtime\":{\"probed\":true,\"touches_remote\":true,\"worker_count\":6,\"cpu_or_no_gpu_count\":1,\"cpu_or_no_gpu_roles\":[\"index\"],\"backend_metadata_may_differ_roles\":[\"index\"],\"acceleration_ok\":false,\"acceleration_next_step\":\"restart-index\",\"error\":\"\"},\"next_step\":\"ready\"}",
        );
        let cpu_failure = gate_failure(&cpu_summary).unwrap();
        assert!(cpu_failure.contains("remote_runtime_acceleration_ok=false"));
        assert!(cpu_failure.contains("remote_runtime_cpu_or_no_gpu=1 roles=index"));
        assert!(context_text(&cpu_summary).contains("remote_runtime_cpu_or_no_gpu_roles:index"));

        let unprobed_summary = parse_status(
            "{\"readiness\":{\"ready\":true,\"model_api\":true,\"backend\":true,\"web_lab\":true,\"quality_worker\":true,\"model_pool_launch_allowed\":true,\"capacity_expansion_allowed\":true},\"model_pool\":{\"available\":true,\"worker_count\":6,\"healthy_worker_count\":6},\"remote_runtime\":{\"probed\":false,\"touches_remote\":false,\"worker_count\":0,\"cpu_or_no_gpu_count\":0,\"cpu_or_no_gpu_roles\":[],\"backend_metadata_may_differ_roles\":[],\"acceleration_ok\":null,\"acceleration_next_step\":\"\",\"error\":\"\"},\"next_step\":\"ready\"}",
        );
        let unprobed_failure = gate_failure(&unprobed_summary).unwrap();
        assert!(unprobed_failure.contains("remote_runtime_probed=false"));
    }

    #[test]
    fn gate_blocks_failed_model_cache_provenance_when_present() {
        let summary = parse_status(
            "{\"readiness\":{\"ready\":true,\"model_api\":true,\"backend\":true,\"web_lab\":true,\"quality_worker\":true,\"model_pool_launch_allowed\":true,\"capacity_expansion_allowed\":true,\"model_cache_all_ok\":false},\"model_pool\":{\"available\":true,\"worker_count\":6,\"healthy_worker_count\":6},\"model_cache\":{\"exists\":true,\"all_ok\":false,\"ok_count\":4,\"model_count\":5,\"remote_error_count\":1,\"read_only\":true,\"path\":\"model-cache-status.json\"},\"next_step\":\"refresh-model-cache\"}",
        );
        let failure = gate_failure(&summary).unwrap();

        assert!(failure.contains("model_cache_all_ok=false"));
        assert!(failure.contains("model_cache_remote_errors=1"));
        assert!(failure.contains("model_cache_ok:4/5"));
    }

    #[test]
    fn gate_blocks_explicit_missing_model_cache_artifact() {
        let summary = parse_status(
            "{\"readiness\":{\"ready\":true,\"model_api\":true,\"backend\":true,\"web_lab\":true,\"quality_worker\":true,\"model_pool_launch_allowed\":true,\"capacity_expansion_allowed\":true},\"model_cache\":{\"exists\":false,\"all_ok\":null,\"ok_count\":0,\"model_count\":0,\"remote_error_count\":0,\"read_only\":null,\"path\":\"missing.json\"},\"next_step\":\"sync-model-cache\"}",
        );
        let failure = gate_failure(&summary).unwrap();

        assert!(failure.contains("model_cache missing"));
        assert!(failure.contains("model_cache_exists:false"));
    }

    #[test]
    fn gate_blocks_backend_quality_model_mismatch() {
        let summary = parse_status(
            "{\"readiness\":{\"ready\":true,\"model_api\":true,\"backend\":true,\"web_lab\":true,\"quality_worker\":true,\"model_pool_launch_allowed\":true,\"capacity_expansion_allowed\":true},\"backend\":{\"model\":\"google/gemma-4-12B-it\"},\"model_pool\":{\"available\":true,\"worker_count\":6,\"healthy_worker_count\":6,\"quality_model_cache_name\":\"Qwen3.6-14B-A3B-FableVibes-Q4_K_M.gguf\"},\"model_cache\":{\"exists\":true,\"all_ok\":true,\"ok_count\":5,\"model_count\":5,\"remote_error_count\":0,\"read_only\":true,\"path\":\"model-cache-status.json\"},\"next_step\":\"ready\"}",
        );
        let failure = gate_failure(&summary).unwrap();

        assert!(failure.contains("backend_model=google/gemma-4-12B-it"));
        assert!(
            failure.contains("quality_model_cache_name=Qwen3.6-14B-A3B-FableVibes-Q4_K_M.gguf")
        );
    }
}
