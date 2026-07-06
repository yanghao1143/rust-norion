use super::route_projection::model_pool_route_json;
use super::{
    bool_text, ensure_pool_contract, push_agent_route_source, push_budget_policy,
    push_dependency_precheck, push_field_line, push_metrics_array, push_metrics_object,
    push_pool_dispatch, push_resource_precheck, push_workers,
};
use crate::provider::json::{
    json_bool_field, json_number_field, json_string_array_field, json_string_field,
};

pub(crate) fn model_pool_route_summary(body: &str) -> Result<String, String> {
    ensure_pool_contract(body, "model pool route")?;
    let mut lines = vec!["SmartSteam model pool route plan".to_owned()];
    push_field_line(
        &mut lines,
        "task_kind",
        json_string_field(body, "task_kind"),
    );
    push_field_line(
        &mut lines,
        "route_allowed",
        json_bool_field(body, "route_allowed").map(bool_text),
    );
    push_field_line(&mut lines, "reason", json_string_field(body, "reason"));
    push_field_line(
        &mut lines,
        "route_block_reason",
        json_string_field(body, "route_block_reason"),
    );
    if let Some(candidates) = json_string_array_field(body, "role_candidates") {
        lines.push(format!("role_candidates={}", candidates.join(",")));
    }
    push_resource_precheck(&mut lines, body);
    push_dependency_precheck(&mut lines, body);
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
        "quality_block_reason",
        json_string_field(body, "quality_block_reason"),
    );
    push_field_line(
        &mut lines,
        "selected_role",
        json_string_field(body, "selected_role"),
    );
    push_field_line(
        &mut lines,
        "selected_base_url",
        json_string_field(body, "selected_base_url"),
    );
    push_field_line(
        &mut lines,
        "selected_port",
        json_number_field(body, "selected_port"),
    );
    push_field_line(
        &mut lines,
        "selected_default_max_tokens",
        json_number_field(body, "selected_default_max_tokens"),
    );
    push_field_line(
        &mut lines,
        "selected_context_window",
        json_number_field(body, "selected_context_window"),
    );
    push_field_line(
        &mut lines,
        "configured_max_tokens",
        json_number_field(body, "configured_max_tokens"),
    );
    push_field_line(
        &mut lines,
        "effective_max_tokens",
        json_number_field(body, "effective_max_tokens"),
    );
    push_field_line(
        &mut lines,
        "max_tokens_clamped",
        json_bool_field(body, "max_tokens_clamped").map(bool_text),
    );
    push_field_line(
        &mut lines,
        "max_tokens_clamp_reason",
        json_string_field(body, "max_tokens_clamp_reason"),
    );
    push_budget_policy(&mut lines, body);
    push_agent_route_source(&mut lines, body);
    push_pool_dispatch(&mut lines, body);
    push_metrics_object(&mut lines, body, "route_metrics", "route_metrics");
    push_metrics_array(&mut lines, body, "worker_metrics", "worker_metric");
    push_workers(&mut lines, body, "candidate_workers");
    lines.push("section=route_json".to_owned());
    lines.push(model_pool_route_json(body));
    Ok(lines.join("\n"))
}
