use crate::provider::json::{
    json_bool_field, json_number_field, json_object_field, json_string, json_string_array_field,
    json_string_field,
};

use super::{helper_cpu_or_no_gpu_roles, runtime_shape_summary};

const STATUS_JSON_SCHEMA: &str = "smartsteam.forge.model_pool_status.v1";

pub(super) fn model_pool_status_json(body: &str) -> String {
    let advice = json_object_field(body, "advice");
    let runtime_shape = runtime_shape_summary(body, "workers");
    let helper_cpu_or_no_gpu_roles = helper_cpu_or_no_gpu_roles(&runtime_shape.cpu_or_no_gpu_roles);
    let helper_runtime_block = !helper_cpu_or_no_gpu_roles.is_empty();
    let advice_safe_to_enable_pool_workers =
        advice.and_then(|value| json_bool_field(value, "safe_to_enable_pool_workers"));
    format!(
        concat!(
            "{{",
            "\"schema\":{},",
            "\"read_only\":{},",
            "\"launches_process\":{},",
            "\"sends_prompt\":{},",
            "\"launch_allowed\":{},",
            "\"reason\":{},",
            "\"launch_block_reason\":{},",
            "\"chain_classification\":{},",
            "\"quality_ready\":{},",
            "\"quality_context_tokens\":{},",
            "\"quality_context_required_tokens\":{},",
            "\"quality_context_sufficient\":{},",
            "\"worker_count\":{},",
            "\"healthy_worker_count\":{},",
            "\"safe_to_enable_pool_workers\":{},",
            "\"next_step\":{},",
            "\"reason_detail\":{},",
            "\"extra_quality_12b_detected\":{},",
            "\"quality_worker_count\":{},",
            "\"helper_worker_count\":{},",
            "\"healthy_helper_worker_count\":{},",
            "\"helper_target_worker_count\":{},",
            "\"helper_roles\":{},",
            "\"expected_helper_roles\":{},",
            "\"missing_helper_roles\":{},",
            "\"helper_cpu_or_no_gpu_roles\":{},",
            "\"recommended_launch_order\":{},",
            "\"runtime_shape\":{},",
            "\"worker_shape\":{}",
            "}}"
        ),
        json_string(STATUS_JSON_SCHEMA),
        option_bool_json(json_bool_field(body, "read_only")),
        option_bool_json(json_bool_field(body, "launches_process")),
        option_bool_json(json_bool_field(body, "sends_prompt")),
        option_bool_json(json_bool_field(body, "launch_allowed")),
        option_string_json(json_string_field(body, "reason").as_deref()),
        option_string_json(json_string_field(body, "launch_block_reason").as_deref()),
        option_string_json(json_string_field(body, "chain_classification").as_deref()),
        option_bool_json(json_bool_field(body, "quality_ready")),
        option_number_json(json_number_field(body, "quality_context_tokens").as_deref()),
        option_number_json(json_number_field(body, "quality_context_required_tokens").as_deref()),
        option_bool_json(json_bool_field(body, "quality_context_sufficient")),
        option_number_json(json_number_field(body, "worker_count").as_deref()),
        option_number_json(json_number_field(body, "healthy_worker_count").as_deref()),
        option_bool_json(projected_safe_to_enable_pool_workers(
            advice_safe_to_enable_pool_workers,
            helper_runtime_block,
        )),
        option_string_json(projected_next_step(advice, helper_runtime_block).as_deref()),
        option_string_json(projected_reason_detail(advice, helper_runtime_block).as_deref()),
        option_bool_json(
            advice.and_then(|value| json_bool_field(value, "extra_quality_12b_detected"))
        ),
        option_number_json(
            advice
                .and_then(|value| json_number_field(value, "quality_worker_count"))
                .as_deref()
        ),
        option_number_json(
            advice
                .and_then(|value| json_number_field(value, "helper_worker_count"))
                .as_deref()
        ),
        option_number_json(
            advice
                .and_then(|value| json_number_field(value, "healthy_helper_worker_count"))
                .as_deref()
        ),
        option_number_json(
            advice
                .and_then(|value| json_number_field(value, "helper_target_worker_count"))
                .as_deref()
        ),
        option_string_array_json(
            json_string_array_field(body, "helper_roles")
                .or_else(|| advice.and_then(|value| json_string_array_field(value, "helper_roles")))
                .as_deref()
        ),
        option_string_array_json(
            json_string_array_field(body, "expected_helper_roles")
                .or_else(|| {
                    advice.and_then(|value| json_string_array_field(value, "expected_helper_roles"))
                })
                .as_deref()
        ),
        option_string_array_json(
            json_string_array_field(body, "missing_helper_roles")
                .or_else(|| {
                    advice.and_then(|value| json_string_array_field(value, "missing_helper_roles"))
                })
                .as_deref()
        ),
        string_array_json(&helper_cpu_or_no_gpu_roles),
        option_string_array_json(
            json_string_array_field(body, "recommended_launch_order")
                .or_else(|| {
                    advice.and_then(|value| {
                        json_string_array_field(value, "recommended_launch_order")
                    })
                })
                .as_deref()
        ),
        runtime_shape_json(&runtime_shape),
        advice
            .and_then(|value| json_object_field(value, "worker_shape"))
            .unwrap_or("null")
    )
}

fn projected_safe_to_enable_pool_workers(
    advice_safe_to_enable_pool_workers: Option<bool>,
    helper_runtime_block: bool,
) -> Option<bool> {
    if helper_runtime_block {
        Some(false)
    } else {
        advice_safe_to_enable_pool_workers
    }
}

fn projected_next_step(advice: Option<&str>, helper_runtime_block: bool) -> Option<String> {
    if helper_runtime_block
        && advice.and_then(|value| json_bool_field(value, "safe_to_enable_pool_workers"))
            != Some(false)
    {
        return Some("fix_helper_metal_or_gpu_layers_before_more_pool_workers".to_owned());
    }
    advice.and_then(|value| json_string_field(value, "next_step"))
}

fn projected_reason_detail(advice: Option<&str>, helper_runtime_block: bool) -> Option<String> {
    if helper_runtime_block
        && advice.and_then(|value| json_bool_field(value, "safe_to_enable_pool_workers"))
            != Some(false)
    {
        return Some("helper_workers_not_gpu_accelerated".to_owned());
    }
    advice.and_then(|value| json_string_field(value, "reason"))
}

fn runtime_shape_json(shape: &super::RuntimeShapeSummary) -> String {
    format!(
        concat!(
            "{{",
            "\"worker_count\":{},",
            "\"metal_worker_count\":{},",
            "\"cpu_or_no_gpu_worker_count\":{},",
            "\"zero_gpu_layer_worker_count\":{},",
            "\"unknown_runtime_worker_count\":{},",
            "\"cpu_or_no_gpu_roles\":{}",
            "}}"
        ),
        shape.worker_count,
        shape.metal_worker_count,
        shape.cpu_or_no_gpu_worker_count,
        shape.zero_gpu_layer_worker_count,
        shape.unknown_runtime_worker_count,
        string_array_json(&shape.cpu_or_no_gpu_roles)
    )
}

fn option_string_json(value: Option<&str>) -> String {
    value.map(json_string).unwrap_or_else(|| "null".to_owned())
}

fn option_number_json(value: Option<&str>) -> &str {
    value.unwrap_or("null")
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
        json_bool_field, json_object_field, json_string_array_field, json_string_field,
    };

    #[test]
    fn status_json_projects_top_level_and_advice_fields() {
        let json = model_pool_status_json(
            "{\"ok\":true,\"contract_version\":\"model-pool.v1\",\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"launch_allowed\":false,\"reason\":\"quality_worker_down\",\"launch_block_reason\":\"quality_worker_down\",\"chain_classification\":\"quality_worker_down\",\"quality_ready\":false,\"quality_context_tokens\":262144,\"quality_context_required_tokens\":262144,\"quality_context_sufficient\":false,\"worker_count\":2,\"healthy_worker_count\":1,\"helper_roles\":[],\"expected_helper_roles\":[\"summary\",\"router\",\"review\",\"index\",\"test-gate\"],\"missing_helper_roles\":[\"summary\",\"router\",\"review\",\"index\",\"test-gate\"],\"recommended_launch_order\":[\"quality\",\"summary\",\"router\",\"review\",\"index\",\"test-gate\"],\"workers\":[{\"role\":\"quality\",\"runtime_device\":\"metal\",\"runtime_accelerator\":\"metal\",\"gpu_layers\":99},{\"role\":\"review\",\"runtime_device\":\"cpu\",\"runtime_accelerator\":\"none\",\"gpu_layers\":0}],\"advice\":{\"safe_to_enable_pool_workers\":false,\"next_step\":\"start_or_fix_quality_worker_8686\",\"reason\":\"quality_worker_not_ready\",\"extra_quality_12b_detected\":false,\"quality_worker_count\":1,\"helper_worker_count\":0,\"healthy_helper_worker_count\":0,\"helper_target_worker_count\":5,\"worker_shape\":{\"quality\":1,\"helpers_visible\":0,\"helpers_healthy\":0,\"helper_target\":5}}}",
        );

        assert_eq!(
            json_string_field(&json, "schema").as_deref(),
            Some(STATUS_JSON_SCHEMA)
        );
        assert_eq!(json_bool_field(&json, "read_only"), Some(true));
        assert_eq!(json_bool_field(&json, "launch_allowed"), Some(false));
        assert_eq!(
            json_bool_field(&json, "safe_to_enable_pool_workers"),
            Some(false)
        );
        assert_eq!(
            json_string_field(&json, "next_step").as_deref(),
            Some("start_or_fix_quality_worker_8686")
        );
        assert_eq!(
            json_string_array_field(&json, "expected_helper_roles"),
            Some(vec![
                "summary".to_owned(),
                "router".to_owned(),
                "review".to_owned(),
                "index".to_owned(),
                "test-gate".to_owned()
            ])
        );
        let runtime_shape = json_object_field(&json, "runtime_shape")
            .expect("status JSON should include runtime_shape");
        assert_eq!(
            crate::provider::json::json_number_field(runtime_shape, "worker_count").as_deref(),
            Some("2")
        );
        assert_eq!(
            crate::provider::json::json_number_field(runtime_shape, "metal_worker_count")
                .as_deref(),
            Some("1")
        );
        assert_eq!(
            crate::provider::json::json_number_field(runtime_shape, "cpu_or_no_gpu_worker_count")
                .as_deref(),
            Some("1")
        );
        assert_eq!(
            json_string_array_field(runtime_shape, "cpu_or_no_gpu_roles"),
            Some(vec!["review".to_owned()])
        );
        assert_eq!(
            json_string_array_field(&json, "helper_cpu_or_no_gpu_roles"),
            Some(vec!["review".to_owned()])
        );
        assert!(json_object_field(&json, "worker_shape").is_some());
    }

    #[test]
    fn status_json_overrides_stale_safe_advice_when_helpers_are_cpu_or_no_gpu() {
        let json = model_pool_status_json(
            "{\"ok\":true,\"contract_version\":\"model-pool.v1\",\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"launch_allowed\":true,\"reason\":\"ready\",\"launch_block_reason\":\"ready\",\"chain_classification\":\"ready\",\"quality_ready\":true,\"quality_context_tokens\":65536,\"quality_context_required_tokens\":65536,\"quality_context_sufficient\":true,\"worker_count\":3,\"healthy_worker_count\":3,\"helper_roles\":[\"summary\",\"review\"],\"expected_helper_roles\":[\"summary\",\"router\",\"review\",\"index\",\"test-gate\"],\"missing_helper_roles\":[\"router\",\"index\",\"test-gate\"],\"recommended_launch_order\":[\"quality\",\"summary\",\"router\",\"review\",\"index\",\"test-gate\"],\"workers\":[{\"role\":\"quality\",\"runtime_device\":\"metal\",\"runtime_accelerator\":\"metal\",\"gpu_layers\":999},{\"role\":\"summary\",\"runtime_device\":\"metal\",\"runtime_accelerator\":\"metal\",\"gpu_layers\":999},{\"role\":\"review\",\"runtime_device\":\"cpu\",\"runtime_accelerator\":\"accelerate\",\"gpu_layers\":0}],\"advice\":{\"safe_to_enable_pool_workers\":true,\"next_step\":\"run_short_pool_smoke_then_use_evolution_loop_helper_stage_calls\",\"reason\":\"full_helper_pool_visible\",\"extra_quality_12b_detected\":false,\"quality_worker_count\":1,\"helper_worker_count\":2,\"healthy_helper_worker_count\":2,\"helper_target_worker_count\":5,\"worker_shape\":{\"quality\":1,\"helpers_visible\":2,\"helpers_healthy\":2,\"helper_target\":5}}}",
        );

        assert_eq!(
            json_bool_field(&json, "safe_to_enable_pool_workers"),
            Some(false)
        );
        assert_eq!(
            json_string_field(&json, "next_step").as_deref(),
            Some("fix_helper_metal_or_gpu_layers_before_more_pool_workers")
        );
        assert_eq!(
            json_string_field(&json, "reason_detail").as_deref(),
            Some("helper_workers_not_gpu_accelerated")
        );
        assert_eq!(
            json_string_array_field(&json, "helper_cpu_or_no_gpu_roles"),
            Some(vec!["review".to_owned()])
        );
    }
}
