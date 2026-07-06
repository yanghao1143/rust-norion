use super::call_projection::{model_pool_backend_call_json, model_pool_worker_answer_json};
use super::{
    ModelPoolRouteSelection, bool_text, ensure_pool_call_contract, push_budget_policy,
    push_field_line, push_pool_dispatch, push_route_agent_source, push_route_budget_policy,
};
use crate::provider::json::{json_bool_field, json_number_field, json_string_field};

pub(crate) fn model_pool_call_summary(body: &str) -> Result<String, String> {
    ensure_pool_call_contract(body)?;
    let answer = json_string_field(body, "answer")
        .or_else(|| json_string_field(body, "content"))
        .or_else(|| json_string_field(body, "text"))
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "model pool call response missing answer".to_owned())?;
    let task_kind = json_string_field(body, "task_kind").unwrap_or_else(|| "auto".to_owned());
    let selected_role =
        json_string_field(body, "selected_role").unwrap_or_else(|| "unknown".to_owned());
    let selected_base_url =
        json_string_field(body, "selected_base_url").unwrap_or_else(|| "unknown".to_owned());
    let mut lines = vec![
        "SmartSteam model pool call".to_owned(),
        format!("task_kind={task_kind}"),
        format!("selected_role={selected_role}"),
        format!("selected_base_url={selected_base_url}"),
    ];
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
    push_pool_dispatch(&mut lines, body);
    lines.push(format!("answer={}", answer.trim()));
    lines.push("section=call_json".to_owned());
    lines.push(model_pool_backend_call_json(body, &answer));
    Ok(lines.join("\n"))
}

pub(crate) fn model_pool_worker_answer_summary(
    route: &ModelPoolRouteSelection,
    response_body: &str,
) -> Result<String, String> {
    let answer = json_string_field(response_body, "content")
        .or_else(|| json_string_field(response_body, "text"))
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            json_string_field(response_body, "message")
                .map(|message| format!("model worker returned error: {message}"))
                .unwrap_or_else(|| "model worker response missing answer content".to_owned())
        })?;
    let mut lines = vec![
        "SmartSteam model pool call".to_owned(),
        format!("task_kind={}", route.task_kind),
        format!("selected_role={}", route.role),
        format!("selected_base_url={}", route.base_url),
    ];
    if let Some(context_window) = route.context_window {
        lines.push(format!("selected_context_window={context_window}"));
    }
    if let Some(default_max_tokens) = route.default_max_tokens {
        lines.push(format!("selected_default_max_tokens={default_max_tokens}"));
    }
    if let Some(effective_max_tokens) = route.effective_max_tokens.or(route.default_max_tokens) {
        lines.push(format!("effective_max_tokens={effective_max_tokens}"));
    }
    push_route_budget_policy(&mut lines, route);
    push_route_agent_source(&mut lines, route);
    lines.push(format!("answer={}", answer.trim()));
    lines.push("section=call_json".to_owned());
    lines.push(model_pool_worker_answer_json(route, &answer));
    Ok(lines.join("\n"))
}
