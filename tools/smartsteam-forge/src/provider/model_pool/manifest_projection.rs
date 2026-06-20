use crate::provider::json::{
    json_array_field, json_bool_field, json_object_field, json_object_items, json_string,
    json_string_array_field, json_string_field,
};

const MANIFEST_JSON_SCHEMA: &str = "smartsteam.forge.model_pool_manifest.v1";

pub(super) fn model_pool_manifest_json(body: &str) -> String {
    let capacity_policy = json_object_field(body, "capacity_policy");
    let advice = json_object_field(body, "advice");
    let worker_roles = manifest_worker_roles(body);
    let worker_count = manifest_worker_count(body);
    format!(
        concat!(
            "{{",
            "\"schema\":{},",
            "\"read_only\":{},",
            "\"launches_process\":{},",
            "\"sends_prompt\":{},",
            "\"contract_version\":{},",
            "\"manifest_kind\":{},",
            "\"worker_count\":{},",
            "\"worker_roles\":{},",
            "\"capacity_policy\":{},",
            "\"quality_role\":{},",
            "\"helper_roles\":{},",
            "\"recommended_launch_order\":{},",
            "\"advice\":{}",
            "}}"
        ),
        json_string(MANIFEST_JSON_SCHEMA),
        option_bool_json(json_bool_field(body, "read_only")),
        option_bool_json(json_bool_field(body, "launches_process")),
        option_bool_json(json_bool_field(body, "sends_prompt")),
        option_string_json(json_string_field(body, "contract_version").as_deref()),
        option_string_json(json_string_field(body, "manifest_kind").as_deref()),
        option_number_json(worker_count),
        string_array_json(&worker_roles),
        capacity_policy.unwrap_or("null"),
        option_string_json(
            capacity_policy
                .and_then(|value| json_string_field(value, "quality_role"))
                .as_deref(),
        ),
        option_string_array_json(
            capacity_policy
                .and_then(|value| json_string_array_field(value, "helper_roles"))
                .or_else(|| advice.and_then(|value| json_string_array_field(value, "helper_roles")))
                .as_deref(),
        ),
        option_string_array_json(
            capacity_policy
                .and_then(|value| json_string_array_field(value, "recommended_launch_order"))
                .or_else(|| {
                    advice.and_then(|value| {
                        json_string_array_field(value, "recommended_launch_order")
                    })
                })
                .as_deref(),
        ),
        advice.unwrap_or("null")
    )
}

fn manifest_worker_roles(body: &str) -> Vec<String> {
    let Some(workers) = json_array_field(body, "workers") else {
        return Vec::new();
    };
    json_object_items(workers)
        .into_iter()
        .filter_map(|worker| json_string_field(worker, "role"))
        .collect()
}

fn manifest_worker_count(body: &str) -> Option<usize> {
    let workers = json_array_field(body, "workers")?;
    Some(json_object_items(workers).len())
}

fn option_string_json(value: Option<&str>) -> String {
    value.map(json_string).unwrap_or_else(|| "null".to_owned())
}

fn option_number_json(value: Option<usize>) -> String {
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
    use crate::provider::json::{json_number_field, json_string_array_field, json_string_field};

    #[test]
    fn manifest_json_projects_capacity_policy_and_workers() {
        let json = model_pool_manifest_json(
            "{\"ok\":true,\"contract_version\":\"gemma-chain.v1\",\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"manifest_kind\":\"rust-norion.model-pool\",\"capacity_policy\":{\"policy\":\"one_quality_plus_small_helpers\",\"quality_role\":\"quality\",\"helper_roles\":[\"summary\",\"router\"],\"recommended_launch_order\":[\"quality\",\"summary\",\"router\"]},\"advice\":{\"reason\":\"partial_helper_pool_visible\"},\"workers\":[{\"role\":\"quality\"},{\"role\":\"summary\"}]}",
        );

        assert_eq!(
            json_string_field(&json, "schema").as_deref(),
            Some(MANIFEST_JSON_SCHEMA)
        );
        assert_eq!(
            json_number_field(&json, "worker_count").as_deref(),
            Some("2")
        );
        assert_eq!(
            json_string_array_field(&json, "worker_roles"),
            Some(vec!["quality".to_owned(), "summary".to_owned()])
        );
        assert_eq!(
            json_string_array_field(&json, "helper_roles"),
            Some(vec!["summary".to_owned(), "router".to_owned()])
        );
        assert_eq!(
            json_string_array_field(&json, "recommended_launch_order"),
            Some(vec![
                "quality".to_owned(),
                "summary".to_owned(),
                "router".to_owned()
            ])
        );
    }
}
