use super::status_projection::model_pool_status_json;
use super::{
    bool_text, ensure_pool_contract, push_capacity_object, push_field_line, push_metrics_array,
    push_metrics_object, push_model_pool_advice, push_model_pool_status_contract,
    push_runtime_shape, push_workers,
};
use crate::provider::json::{json_bool_field, json_number_field, json_string_field};

pub(crate) fn model_pool_status_summary(body: &str) -> Result<String, String> {
    ensure_pool_contract(body, "model pool status")?;
    let mut lines = vec!["SmartSteam model pool status".to_owned()];
    push_field_line(
        &mut lines,
        "contract_version",
        json_string_field(body, "contract_version"),
    );
    push_field_line(
        &mut lines,
        "launch_allowed",
        json_bool_field(body, "launch_allowed").map(bool_text),
    );
    push_field_line(&mut lines, "reason", json_string_field(body, "reason"));
    push_field_line(
        &mut lines,
        "launch_block_reason",
        json_string_field(body, "launch_block_reason"),
    );
    push_field_line(
        &mut lines,
        "chain_classification",
        json_string_field(body, "chain_classification"),
    );
    push_field_line(
        &mut lines,
        "min_context_tokens",
        json_number_field(body, "min_context_tokens"),
    );
    push_field_line(
        &mut lines,
        "quality_ready",
        json_bool_field(body, "quality_ready").map(bool_text),
    );
    push_field_line(
        &mut lines,
        "quality_context_tokens",
        json_number_field(body, "quality_context_tokens"),
    );
    push_field_line(
        &mut lines,
        "quality_context_required_tokens",
        json_number_field(body, "quality_context_required_tokens"),
    );
    push_field_line(
        &mut lines,
        "quality_context_sufficient",
        json_bool_field(body, "quality_context_sufficient").map(bool_text),
    );
    push_field_line(
        &mut lines,
        "quality_default_context_tokens",
        json_number_field(body, "quality_default_context_tokens"),
    );
    push_field_line(
        &mut lines,
        "quality_default_max_tokens",
        json_number_field(body, "quality_default_max_tokens"),
    );
    push_field_line(
        &mut lines,
        "quality_block_reason",
        json_string_field(body, "quality_block_reason"),
    );
    push_field_line(
        &mut lines,
        "blocked_policy",
        json_string_field(body, "blocked_policy"),
    );
    push_field_line(
        &mut lines,
        "worker_count",
        json_number_field(body, "worker_count"),
    );
    push_field_line(
        &mut lines,
        "healthy_worker_count",
        json_number_field(body, "healthy_worker_count"),
    );
    push_model_pool_status_contract(&mut lines, body);
    push_capacity_object(&mut lines, body);
    push_runtime_shape(&mut lines, body, "workers");
    push_model_pool_advice(&mut lines, body);
    push_metrics_object(&mut lines, body, "route_metrics", "route_metrics");
    push_metrics_array(&mut lines, body, "worker_metrics", "worker_metric");
    push_workers(&mut lines, body, "workers");
    lines.push("section=status_json".to_owned());
    lines.push(model_pool_status_json(body));
    Ok(lines.join("\n"))
}
