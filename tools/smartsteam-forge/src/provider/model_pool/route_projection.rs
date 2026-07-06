use crate::provider::json::{
    json_array_field, json_bool_field, json_number_field, json_object_field, json_object_items,
    json_string, json_string_array_field, json_string_field,
};

const ROUTE_JSON_SCHEMA: &str = "smartsteam.forge.model_pool_route.v1";

pub(super) fn model_pool_route_json(body: &str) -> String {
    let pool_dispatch = json_object_field(body, "pool_dispatch");
    let resource_precheck = json_object_field(body, "routing_weights")
        .and_then(|weights| json_object_field(weights, "resource_precheck"));
    let dependency_precheck = json_object_field(body, "dependency_precheck");
    let agent_route_source = json_object_field(body, "agent_model_route_source");
    let candidate_roles = candidate_worker_roles(body);
    let candidate_count = candidate_worker_count(body);
    format!(
        concat!(
            "{{",
            "\"schema\":{},",
            "\"read_only\":{},",
            "\"launches_process\":{},",
            "\"sends_prompt\":{},",
            "\"task_kind\":{},",
            "\"route_allowed\":{},",
            "\"reason\":{},",
            "\"route_block_reason\":{},",
            "\"role_candidates\":{},",
            "\"quality_context_tokens\":{},",
            "\"quality_context_required_tokens\":{},",
            "\"quality_context_sufficient\":{},",
            "\"quality_block_reason\":{},",
            "\"selected_role\":{},",
            "\"selected_base_url\":{},",
            "\"selected_port\":{},",
            "\"selected_context_window\":{},",
            "\"selected_default_max_tokens\":{},",
            "\"configured_max_tokens\":{},",
            "\"effective_max_tokens\":{},",
            "\"max_tokens_clamped\":{},",
            "\"max_tokens_clamp_reason\":{},",
            "\"candidate_worker_count\":{},",
            "\"candidate_worker_roles\":{},",
            "\"resource_precheck\":{},",
            "\"dependency_precheck\":{},",
            "\"agent_model_route_source\":{},",
            "\"pool_dispatch\":{}",
            "}}"
        ),
        json_string(ROUTE_JSON_SCHEMA),
        option_bool_json(json_bool_field(body, "read_only")),
        option_bool_json(json_bool_field(body, "launches_process")),
        option_bool_json(json_bool_field(body, "sends_prompt")),
        option_string_json(json_string_field(body, "task_kind").as_deref()),
        option_bool_json(json_bool_field(body, "route_allowed")),
        option_string_json(json_string_field(body, "reason").as_deref()),
        option_string_json(json_string_field(body, "route_block_reason").as_deref()),
        option_string_array_json(json_string_array_field(body, "role_candidates").as_deref()),
        option_number_json(json_number_field(body, "quality_context_tokens").as_deref()),
        option_number_json(json_number_field(body, "quality_context_required_tokens").as_deref()),
        option_bool_json(json_bool_field(body, "quality_context_sufficient")),
        option_string_json(json_string_field(body, "quality_block_reason").as_deref()),
        option_string_json(json_string_field(body, "selected_role").as_deref()),
        option_string_json(json_string_field(body, "selected_base_url").as_deref()),
        option_number_json(json_number_field(body, "selected_port").as_deref()),
        option_number_json(json_number_field(body, "selected_context_window").as_deref()),
        option_number_json(json_number_field(body, "selected_default_max_tokens").as_deref()),
        option_number_json(json_number_field(body, "configured_max_tokens").as_deref()),
        option_number_json(json_number_field(body, "effective_max_tokens").as_deref()),
        option_bool_json(json_bool_field(body, "max_tokens_clamped")),
        option_string_json(json_string_field(body, "max_tokens_clamp_reason").as_deref()),
        option_usize_json(candidate_count),
        string_array_json(&candidate_roles),
        resource_precheck.unwrap_or("null"),
        dependency_precheck.unwrap_or("null"),
        agent_route_source.unwrap_or("null"),
        pool_dispatch.unwrap_or("null"),
    )
}

fn candidate_worker_roles(body: &str) -> Vec<String> {
    let Some(workers) = json_array_field(body, "candidate_workers") else {
        return Vec::new();
    };
    json_object_items(workers)
        .into_iter()
        .filter_map(|worker| json_string_field(worker, "role"))
        .collect()
}

fn candidate_worker_count(body: &str) -> Option<usize> {
    let workers = json_array_field(body, "candidate_workers")?;
    Some(json_object_items(workers).len())
}

fn option_string_json(value: Option<&str>) -> String {
    value.map(json_string).unwrap_or_else(|| "null".to_owned())
}

fn option_number_json(value: Option<&str>) -> &str {
    value.unwrap_or("null")
}

fn option_usize_json(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

fn option_bool_json(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "true",
        Some(false) => "false",
        None => "null",
    }
}

fn option_string_array_json(values: Option<&[String]>) -> String {
    values
        .map(string_array_json)
        .unwrap_or_else(|| "null".to_owned())
}

fn string_array_json(values: &[String]) -> String {
    let values = values
        .iter()
        .map(|value| json_string(value))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{values}]")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::json::{
        json_bool_field, json_number_field, json_object_field, json_string_array_field,
        json_string_field,
    };

    #[test]
    fn route_json_projects_selected_worker_and_prechecks() {
        let json = model_pool_route_json(
            "{\"ok\":true,\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"task_kind\":\"index\",\"route_allowed\":true,\"reason\":\"ready\",\"role_candidates\":[\"index\",\"summary\"],\"routing_weights\":{\"resource_precheck\":{\"strategy\":\"resource_precheck_v1\",\"allow_dispatch\":true}},\"dependency_precheck\":{\"strategy\":\"role_dependency_graph_v1\",\"allow_dispatch\":true},\"agent_model_route_source\":{\"route_allowed\":true,\"proof_ready\":true,\"selected_role\":\"index\",\"model_registry_id\":\"registry.index\",\"model_profile_id\":\"profile.index\",\"inference_backend_id\":\"backend.index\",\"model_pool_id\":\"pool.main\"},\"selected_role\":\"index\",\"selected_base_url\":\"http://127.0.0.1:8690\",\"selected_port\":8690,\"selected_default_max_tokens\":512,\"selected_context_window\":4096,\"configured_max_tokens\":262144,\"effective_max_tokens\":512,\"max_tokens_clamped\":true,\"pool_dispatch\":{\"selected_role\":\"index\",\"selected_port\":8690},\"candidate_workers\":[{\"role\":\"index\"},{\"role\":\"summary\"}]}",
        );

        assert_eq!(
            json_string_field(&json, "schema").as_deref(),
            Some(ROUTE_JSON_SCHEMA)
        );
        assert_eq!(json_bool_field(&json, "route_allowed"), Some(true));
        assert_eq!(
            json_string_field(&json, "selected_role").as_deref(),
            Some("index")
        );
        assert_eq!(
            json_number_field(&json, "effective_max_tokens").as_deref(),
            Some("512")
        );
        assert_eq!(json_bool_field(&json, "max_tokens_clamped"), Some(true));
        assert_eq!(
            json_number_field(&json, "candidate_worker_count").as_deref(),
            Some("2")
        );
        assert_eq!(
            json_string_array_field(&json, "candidate_worker_roles"),
            Some(vec!["index".to_owned(), "summary".to_owned()])
        );
        assert!(json_object_field(&json, "resource_precheck").is_some());
        assert!(json_object_field(&json, "dependency_precheck").is_some());
        assert!(json_object_field(&json, "agent_model_route_source").is_some());
        assert!(json_object_field(&json, "pool_dispatch").is_some());
    }
}
