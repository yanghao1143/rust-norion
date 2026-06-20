use super::ModelPoolRouteSelection;
use crate::provider::json::{
    json_bool_field, json_number_field, json_object_field, json_string, json_string_field,
};

const CALL_JSON_SCHEMA: &str = "smartsteam.forge.model_pool_call.v1";

pub(super) fn model_pool_backend_call_json(body: &str, answer: &str) -> String {
    let pool_dispatch = json_object_field(body, "pool_dispatch");
    format!(
        concat!(
            "{{",
            "\"schema\":{},",
            "\"source\":\"backend_call\",",
            "\"read_only\":{},",
            "\"launches_process\":{},",
            "\"sends_prompt\":{},",
            "\"task_kind\":{},",
            "\"selected_role\":{},",
            "\"selected_base_url\":{},",
            "\"selected_port\":{},",
            "\"selected_context_window\":{},",
            "\"selected_default_max_tokens\":{},",
            "\"configured_max_tokens\":{},",
            "\"effective_max_tokens\":{},",
            "\"max_tokens_clamped\":{},",
            "\"max_tokens_clamp_reason\":{},",
            "\"answer\":{},",
            "\"pool_dispatch\":{}",
            "}}"
        ),
        json_string(CALL_JSON_SCHEMA),
        option_bool_json(json_bool_field(body, "read_only")),
        option_bool_json(json_bool_field(body, "launches_process")),
        option_bool_json(json_bool_field(body, "sends_prompt")),
        option_string_json(json_string_field(body, "task_kind").as_deref()),
        option_string_json(json_string_field(body, "selected_role").as_deref()),
        option_string_json(json_string_field(body, "selected_base_url").as_deref()),
        option_number_json(json_number_field(body, "selected_port").as_deref()),
        option_number_json(json_number_field(body, "selected_context_window").as_deref()),
        option_number_json(json_number_field(body, "selected_default_max_tokens").as_deref()),
        option_number_json(json_number_field(body, "configured_max_tokens").as_deref()),
        option_number_json(json_number_field(body, "effective_max_tokens").as_deref()),
        option_bool_json(json_bool_field(body, "max_tokens_clamped")),
        option_string_json(json_string_field(body, "max_tokens_clamp_reason").as_deref()),
        json_string(answer.trim()),
        pool_dispatch.unwrap_or("null"),
    )
}

pub(super) fn model_pool_worker_answer_json(
    route: &ModelPoolRouteSelection,
    answer: &str,
) -> String {
    format!(
        concat!(
            "{{",
            "\"schema\":{},",
            "\"source\":\"worker_chat\",",
            "\"read_only\":false,",
            "\"launches_process\":false,",
            "\"sends_prompt\":true,",
            "\"task_kind\":{},",
            "\"selected_role\":{},",
            "\"selected_base_url\":{},",
            "\"selected_context_window\":{},",
            "\"selected_default_max_tokens\":{},",
            "\"effective_max_tokens\":{},",
            "\"answer\":{},",
            "\"pool_dispatch\":null",
            "}}"
        ),
        json_string(CALL_JSON_SCHEMA),
        json_string(&route.task_kind),
        json_string(&route.role),
        json_string(&route.base_url),
        option_usize_json(route.context_window),
        option_usize_json(route.default_max_tokens),
        option_usize_json(route.effective_max_tokens.or(route.default_max_tokens)),
        json_string(answer.trim()),
    )
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::json::{json_bool_field, json_number_field, json_string_field};

    #[test]
    fn backend_call_json_projects_answer_and_budget() {
        let json = model_pool_backend_call_json(
            "{\"read_only\":false,\"launches_process\":false,\"sends_prompt\":true,\"task_kind\":\"review\",\"selected_role\":\"review\",\"selected_base_url\":\"http://127.0.0.1:8688\",\"selected_port\":8688,\"selected_default_max_tokens\":1024,\"effective_max_tokens\":1024,\"max_tokens_clamped\":true}",
            "review\nok",
        );

        assert_eq!(
            json_string_field(&json, "schema").as_deref(),
            Some(CALL_JSON_SCHEMA)
        );
        assert_eq!(
            json_string_field(&json, "source").as_deref(),
            Some("backend_call")
        );
        assert_eq!(json_bool_field(&json, "sends_prompt"), Some(true));
        assert_eq!(
            json_string_field(&json, "selected_role").as_deref(),
            Some("review")
        );
        assert_eq!(
            json_number_field(&json, "effective_max_tokens").as_deref(),
            Some("1024")
        );
        assert_eq!(
            json_string_field(&json, "answer").as_deref(),
            Some("review\nok")
        );
    }

    #[test]
    fn worker_answer_json_projects_route_budget() {
        let route = ModelPoolRouteSelection {
            task_kind: "summary".to_owned(),
            role: "summary".to_owned(),
            base_url: "http://127.0.0.1:8687".to_owned(),
            context_window: Some(8192),
            default_max_tokens: Some(768),
            effective_max_tokens: None,
        };
        let json = model_pool_worker_answer_json(&route, "短摘要");

        assert_eq!(
            json_string_field(&json, "source").as_deref(),
            Some("worker_chat")
        );
        assert_eq!(
            json_number_field(&json, "selected_context_window").as_deref(),
            Some("8192")
        );
        assert_eq!(
            json_number_field(&json, "effective_max_tokens").as_deref(),
            Some("768")
        );
        assert_eq!(
            json_string_field(&json, "answer").as_deref(),
            Some("短摘要")
        );
    }
}
