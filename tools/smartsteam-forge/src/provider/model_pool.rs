use std::collections::BTreeSet;

use super::json::{
    json_array_field, json_bool_field, json_number_field, json_object_field, json_object_items,
    json_string, json_string_array_field, json_string_field,
};
use model_pool_advice_core::CPU_FALLBACK_HELPER_ROLES;

mod call_projection;
mod call_summary;
mod manifest_projection;
mod manifest_summary;
mod route_projection;
mod route_summary;
mod status_projection;
mod status_summary;

pub(crate) use call_summary::{model_pool_call_summary, model_pool_worker_answer_summary};
pub(crate) use manifest_summary::model_pool_manifest_summary;
pub(crate) use route_summary::model_pool_route_summary;
pub(crate) use status_summary::model_pool_status_summary;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelPoolRouteSelection {
    pub(crate) task_kind: String,
    pub(crate) role: String,
    pub(crate) base_url: String,
    pub(crate) context_window: Option<usize>,
    pub(crate) default_max_tokens: Option<usize>,
    pub(crate) effective_max_tokens: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RuntimeShapeSummary {
    pub(super) worker_count: usize,
    pub(super) metal_worker_count: usize,
    pub(super) cpu_or_no_gpu_worker_count: usize,
    pub(super) zero_gpu_layer_worker_count: usize,
    pub(super) unknown_runtime_worker_count: usize,
    pub(super) cpu_or_no_gpu_roles: Vec<String>,
}

pub(crate) fn model_pool_route_selection(body: &str) -> Result<ModelPoolRouteSelection, String> {
    ensure_pool_contract(body, "model pool route")?;
    if json_bool_field(body, "route_allowed") != Some(true) {
        let reason = json_string_field(body, "reason").unwrap_or_else(|| "unknown".to_owned());
        return Err(format!("model pool route is blocked: {reason}"));
    }
    if json_bool_field(body, "quality_context_sufficient") == Some(false) {
        let actual = json_number_field(body, "quality_context_tokens")
            .unwrap_or_else(|| "unknown".to_owned());
        let required = json_number_field(body, "quality_context_required_tokens")
            .unwrap_or_else(|| "unknown".to_owned());
        return Err(format!(
            "model pool route is blocked: quality context insufficient actual={actual} required={required}"
        ));
    }
    if resource_precheck_allows_dispatch(body) == Some(false) {
        let reason = resource_precheck_reason(body).unwrap_or_else(|| "unknown".to_owned());
        return Err(format!(
            "model pool route is blocked: resource precheck {reason}"
        ));
    }
    if dependency_precheck_allows_dispatch(body) == Some(false) {
        let reason = dependency_precheck_reason(body).unwrap_or_else(|| "unknown".to_owned());
        return Err(format!(
            "model pool route is blocked: dependency precheck {reason}"
        ));
    }
    let task_kind = json_string_field(body, "task_kind").unwrap_or_else(|| "auto".to_owned());
    let role = json_string_field(body, "selected_role")
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "model pool route missing selected_role".to_owned())?;
    let base_url = json_string_field(body, "selected_base_url")
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "model pool route missing selected_base_url".to_owned())?;
    let context_window = json_number_field(body, "selected_context_window")
        .or_else(|| {
            json_object_field(body, "pool_dispatch")
                .and_then(|dispatch| json_number_field(dispatch, "context_window"))
        })
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0);
    let default_max_tokens = json_number_field(body, "selected_default_max_tokens")
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0);
    let effective_max_tokens = json_number_field(body, "effective_max_tokens")
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0);
    Ok(ModelPoolRouteSelection {
        task_kind,
        role,
        base_url,
        context_window,
        default_max_tokens,
        effective_max_tokens,
    })
}

fn resource_precheck_allows_dispatch(body: &str) -> Option<bool> {
    let routing_weights = json_object_field(body, "routing_weights")?;
    let resource_precheck = json_object_field(routing_weights, "resource_precheck")?;
    json_bool_field(resource_precheck, "allow_dispatch")
}

fn resource_precheck_reason(body: &str) -> Option<String> {
    let routing_weights = json_object_field(body, "routing_weights")?;
    let resource_precheck = json_object_field(routing_weights, "resource_precheck")?;
    json_string_field(resource_precheck, "reason")
}

fn dependency_precheck_allows_dispatch(body: &str) -> Option<bool> {
    let dependency_precheck = json_object_field(body, "dependency_precheck")?;
    json_bool_field(dependency_precheck, "allow_dispatch")
}

fn dependency_precheck_reason(body: &str) -> Option<String> {
    let dependency_precheck = json_object_field(body, "dependency_precheck")?;
    json_string_field(dependency_precheck, "reason")
}

pub(crate) fn model_pool_route_request_body(task_kind: &str, max_tokens: Option<usize>) -> String {
    format!(
        "{{\"task_kind\":{}{}}}",
        json_string(task_kind),
        max_tokens_json(max_tokens)
    )
}

pub(crate) fn model_pool_call_request_body(
    task_kind: &str,
    prompt: &str,
    max_tokens: Option<usize>,
) -> String {
    format!(
        "{{\"task_kind\":{},\"prompt\":{}{}}}",
        json_string(task_kind),
        json_string(prompt),
        max_tokens_json(max_tokens)
    )
}

pub(crate) fn model_pool_worker_chat_request_body(
    prompt: &str,
    max_tokens: Option<usize>,
) -> String {
    let max_tokens = max_tokens
        .map(|value| format!(",\"max_tokens\":{}", value.max(1)))
        .unwrap_or_default();
    format!(
        "{{\"model\":\"smartsteam-pool-worker\",\"messages\":[{{\"role\":\"user\",\"content\":{}}}],\"stream\":false{max_tokens}}}",
        json_string(prompt)
    )
}

fn max_tokens_json(max_tokens: Option<usize>) -> String {
    max_tokens
        .map(|value| format!(",\"max_tokens\":{}", value.max(1)))
        .unwrap_or_default()
}

fn ensure_pool_contract(body: &str, label: &str) -> Result<(), String> {
    if json_bool_field(body, "ok") == Some(false) {
        let error = json_string_field(body, "error").unwrap_or_else(|| "unknown".to_owned());
        return Err(format!("{label} failed: {error}"));
    }
    for field in ["read_only", "launches_process", "sends_prompt"] {
        let value = json_bool_field(body, field)
            .ok_or_else(|| format!("{label} response missing {field} contract field"))?;
        let expected = field == "read_only";
        if value != expected {
            return Err(format!(
                "{label} response failed safety contract: {field}={value}"
            ));
        }
    }
    Ok(())
}

fn ensure_pool_call_contract(body: &str) -> Result<(), String> {
    if json_bool_field(body, "ok") == Some(false) {
        let error = json_string_field(body, "reason")
            .or_else(|| json_string_field(body, "route_block_reason"))
            .or_else(|| json_string_field(body, "error"))
            .unwrap_or_else(|| "unknown".to_owned());
        return Err(format!("model pool call failed: {error}"));
    }
    for (field, expected) in [
        ("read_only", false),
        ("launches_process", false),
        ("sends_prompt", true),
    ] {
        let value = json_bool_field(body, field)
            .ok_or_else(|| format!("model pool call response missing {field} contract field"))?;
        if value != expected {
            return Err(format!(
                "model pool call response failed safety contract: {field}={value}"
            ));
        }
    }
    Ok(())
}

fn push_workers(lines: &mut Vec<String>, body: &str, field: &str) {
    let Some(workers) = json_array_field(body, field) else {
        return;
    };
    let items = json_object_items(workers);
    if items.is_empty() {
        lines.push("workers=none".to_owned());
        return;
    }
    lines.push(format!("workers={}", items.len()));
    for item in items {
        let role = json_string_field(item, "role").unwrap_or_else(|| "unknown".to_owned());
        let status = json_string_field(item, "status").unwrap_or_else(|| "unknown".to_owned());
        let ready = json_bool_field(item, "ready")
            .or_else(|| json_bool_field(item, "role_ready"))
            .map(bool_text)
            .unwrap_or_else(|| "unknown".to_owned());
        let base_url = json_string_field(item, "base_url").unwrap_or_else(|| "unknown".to_owned());
        let max_tokens =
            json_number_field(item, "default_max_tokens").unwrap_or_else(|| "unknown".to_owned());
        let context = json_number_field(item, "context_window")
            .or_else(|| json_number_field(item, "default_context_tokens"))
            .unwrap_or_else(|| "unknown".to_owned());
        let reason =
            json_string_field(item, "role_block_reason").unwrap_or_else(|| "unknown".to_owned());
        let runtime_backend =
            json_string_field(item, "runtime_backend").unwrap_or_else(|| "unknown".to_owned());
        let runtime_device =
            json_string_field(item, "runtime_device").unwrap_or_else(|| "unknown".to_owned());
        let runtime_accelerator =
            json_string_field(item, "runtime_accelerator").unwrap_or_else(|| "unknown".to_owned());
        let gpu_layers =
            json_number_field(item, "gpu_layers").unwrap_or_else(|| "unknown".to_owned());
        let metrics = metric_pairs(item)
            .map(|pairs| format!(" {pairs}"))
            .unwrap_or_default();
        lines.push(format!(
            "worker role={role} status={status} ready={ready} base_url={base_url} context={context} max_tokens={max_tokens} runtime_backend={runtime_backend} runtime_device={runtime_device} runtime_accelerator={runtime_accelerator} gpu_layers={gpu_layers} reason={reason}{metrics}"
        ));
    }
}

pub(super) fn push_runtime_shape(lines: &mut Vec<String>, body: &str, field: &str) {
    let shape = runtime_shape_summary(body, field);
    if shape.worker_count == 0 {
        return;
    }
    lines.push(format!(
        "runtime_shape workers={} metal_workers={} cpu_or_no_gpu_workers={} zero_gpu_layer_workers={} unknown_runtime_workers={} cpu_or_no_gpu_roles={}",
        shape.worker_count,
        shape.metal_worker_count,
        shape.cpu_or_no_gpu_worker_count,
        shape.zero_gpu_layer_worker_count,
        shape.unknown_runtime_worker_count,
        list_text(&shape.cpu_or_no_gpu_roles)
    ));
}

pub(super) fn runtime_shape_summary(body: &str, field: &str) -> RuntimeShapeSummary {
    let Some(workers) = json_array_field(body, field) else {
        return RuntimeShapeSummary {
            worker_count: 0,
            metal_worker_count: 0,
            cpu_or_no_gpu_worker_count: 0,
            zero_gpu_layer_worker_count: 0,
            unknown_runtime_worker_count: 0,
            cpu_or_no_gpu_roles: Vec::new(),
        };
    };
    let items = json_object_items(workers);
    let mut cpu_or_no_gpu_roles = BTreeSet::new();
    let mut shape = RuntimeShapeSummary {
        worker_count: items.len(),
        metal_worker_count: 0,
        cpu_or_no_gpu_worker_count: 0,
        zero_gpu_layer_worker_count: 0,
        unknown_runtime_worker_count: 0,
        cpu_or_no_gpu_roles: Vec::new(),
    };

    for item in items {
        if worker_looks_metal_accelerated(item) {
            shape.metal_worker_count += 1;
        }
        if worker_has_zero_gpu_layers(item) {
            shape.zero_gpu_layer_worker_count += 1;
        }
        if worker_looks_cpu_or_no_gpu(item) {
            shape.cpu_or_no_gpu_worker_count += 1;
            cpu_or_no_gpu_roles
                .insert(json_string_field(item, "role").unwrap_or_else(|| "unknown".to_owned()));
        }
        if worker_runtime_unknown(item) {
            shape.unknown_runtime_worker_count += 1;
        }
    }

    shape.cpu_or_no_gpu_roles = cpu_or_no_gpu_roles.into_iter().collect();
    shape
}

pub(super) fn helper_cpu_or_no_gpu_roles(cpu_or_no_gpu_roles: &[String]) -> Vec<String> {
    cpu_or_no_gpu_roles
        .iter()
        .filter(|role| matches!(role.as_str(), "summary" | "router" | "review" | "test-gate"))
        .cloned()
        .collect()
}

fn allowed_cpu_fallback_helper_roles(cpu_or_no_gpu_roles: &[String]) -> Vec<String> {
    cpu_or_no_gpu_roles
        .iter()
        .filter(|role| CPU_FALLBACK_HELPER_ROLES.contains(&role.as_str()))
        .cloned()
        .collect()
}

fn worker_looks_metal_accelerated(worker: &str) -> bool {
    worker_runtime_token(worker, "runtime_device")
        .or_else(|| worker_runtime_token(worker, "runtime_accelerator"))
        .is_some_and(|value| matches!(value.as_str(), "metal" | "apple-metal" | "gpu"))
}

fn worker_looks_cpu_or_no_gpu(worker: &str) -> bool {
    worker_runtime_token(worker, "runtime_device")
        .is_some_and(|value| matches!(value.as_str(), "cpu" | "cpu-vector" | "none" | "no-gpu"))
        || worker_runtime_token(worker, "runtime_accelerator")
            .is_some_and(|value| matches!(value.as_str(), "cpu" | "none" | "disabled" | "no-gpu"))
        || worker_has_zero_gpu_layers(worker)
}

fn worker_has_zero_gpu_layers(worker: &str) -> bool {
    json_number_field(worker, "gpu_layers").as_deref() == Some("0")
}

fn worker_runtime_unknown(worker: &str) -> bool {
    worker_runtime_token(worker, "runtime_device").is_none()
        && worker_runtime_token(worker, "runtime_accelerator").is_none()
        && json_number_field(worker, "gpu_layers").is_none()
}

fn worker_runtime_token(worker: &str, field: &str) -> Option<String> {
    json_string_field(worker, field).map(|value| value.trim().to_ascii_lowercase())
}

fn list_text(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_owned()
    } else {
        values.join(",")
    }
}

fn push_manifest_capacity_policy(lines: &mut Vec<String>, body: &str) {
    let Some(policy) = json_object_field(body, "capacity_policy") else {
        return;
    };
    let mut pairs = Vec::new();
    push_string_pair(&mut pairs, policy, "policy");
    push_string_pair(&mut pairs, policy, "target_host");
    push_bool_pair(&mut pairs, policy, "avoid_extra_12b");
    push_metric_pair(&mut pairs, policy, "max_quality_12b_workers");
    push_string_pair(&mut pairs, policy, "quality_role");
    push_metric_pair(&mut pairs, policy, "quality_required_context_tokens");
    push_metric_pair(&mut pairs, policy, "helper_context_tokens_total");
    push_metric_pair(&mut pairs, policy, "helper_default_max_tokens_total");
    push_string_pair(&mut pairs, policy, "expansion_gate");
    push_string_pair(&mut pairs, policy, "next_step_when_quality_ready");
    if let Some(helper_roles) = json_string_array_field(policy, "helper_roles") {
        pairs.push(format!("helper_roles={}", helper_roles.join(",")));
    }
    if let Some(recommended_launch_order) =
        json_string_array_field(policy, "recommended_launch_order")
    {
        pairs.push(format!(
            "recommended_launch_order={}",
            recommended_launch_order.join(",")
        ));
    }
    if !pairs.is_empty() {
        lines.push(format!("capacity_policy {}", pairs.join(" ")));
    }
}

fn push_model_pool_advice(lines: &mut Vec<String>, body: &str) {
    let Some(advice) = json_object_field(body, "advice") else {
        return;
    };
    let runtime_shape = runtime_shape_summary(body, "workers");
    let runtime_cpu_or_no_gpu_roles = runtime_shape.cpu_or_no_gpu_roles;
    let helper_runtime_roles = helper_cpu_or_no_gpu_roles(&runtime_cpu_or_no_gpu_roles);
    let allowed_runtime_roles = allowed_cpu_fallback_helper_roles(&runtime_cpu_or_no_gpu_roles);
    let helper_runtime_block = !helper_runtime_roles.is_empty();
    let mut pairs = Vec::new();
    push_string_pair(&mut pairs, &advice, "decision_source");
    push_string_pair(&mut pairs, &advice, "policy");
    if let Some(safe_to_enable_pool_workers) =
        projected_advice_safe_to_enable_pool_workers(&advice, helper_runtime_block)
    {
        pairs.push(format!(
            "safe_to_enable_pool_workers={}",
            bool_text(safe_to_enable_pool_workers)
        ));
    }
    if let Some(next_step) = projected_advice_next_step(&advice, helper_runtime_block) {
        pairs.push(format!("next_step={next_step}"));
    }
    if let Some(reason) = projected_advice_reason(&advice, helper_runtime_block) {
        pairs.push(format!("reason={reason}"));
    }
    if let Some(kind) = projected_advice_kind(&advice, helper_runtime_block) {
        pairs.push(format!("kind={kind}"));
    }
    push_bool_pair(&mut pairs, &advice, "extra_quality_12b_detected");
    push_metric_pair(&mut pairs, &advice, "max_quality_12b_workers");
    push_metric_pair(&mut pairs, &advice, "quality_worker_count");
    push_metric_pair(&mut pairs, &advice, "helper_worker_count");
    push_metric_pair(&mut pairs, &advice, "helper_target_worker_count");
    push_string_pair(&mut pairs, &advice, "operator_checks");
    if let Some(helper_roles) = json_string_array_field(&advice, "helper_roles") {
        pairs.push(format!("helper_roles={}", helper_roles.join(",")));
    }
    if let Some(expected_helper_roles) = json_string_array_field(&advice, "expected_helper_roles") {
        pairs.push(format!(
            "expected_helper_roles={}",
            expected_helper_roles.join(",")
        ));
    }
    if let Some(missing_helper_roles) = json_string_array_field(&advice, "missing_helper_roles") {
        pairs.push(format!(
            "missing_helper_roles={}",
            missing_helper_roles.join(",")
        ));
    }
    if !runtime_cpu_or_no_gpu_roles.is_empty() {
        pairs.push(format!(
            "helper_cpu_or_no_gpu_roles={}",
            runtime_cpu_or_no_gpu_roles.join(",")
        ));
    } else if json_string_array_field(&advice, "helper_cpu_or_no_gpu_roles").is_some() {
        pairs.push("helper_cpu_or_no_gpu_roles=".to_owned());
    }
    if !helper_runtime_roles.is_empty() {
        pairs.push(format!(
            "blocking_helper_cpu_or_no_gpu_roles={}",
            helper_runtime_roles.join(",")
        ));
    } else if json_string_array_field(&advice, "blocking_helper_cpu_or_no_gpu_roles").is_some() {
        pairs.push("blocking_helper_cpu_or_no_gpu_roles=".to_owned());
    }
    if !allowed_runtime_roles.is_empty() {
        pairs.push(format!(
            "allowed_cpu_fallback_helper_roles={}",
            allowed_runtime_roles.join(",")
        ));
    } else if json_string_array_field(&advice, "allowed_cpu_fallback_helper_roles").is_some() {
        pairs.push("allowed_cpu_fallback_helper_roles=".to_owned());
    }
    if let Some(recommended_launch_order) =
        json_string_array_field(&advice, "recommended_launch_order")
    {
        pairs.push(format!(
            "recommended_launch_order={}",
            recommended_launch_order.join(",")
        ));
    }
    if let Some(worker_shape) = json_object_field(&advice, "worker_shape") {
        let quality =
            json_number_field(&worker_shape, "quality").unwrap_or_else(|| "unknown".to_owned());
        let helpers_visible = json_number_field(&worker_shape, "helpers_visible")
            .unwrap_or_else(|| "unknown".to_owned());
        let helper_target = json_number_field(&worker_shape, "helper_target")
            .unwrap_or_else(|| "unknown".to_owned());
        pairs.push(format!(
            "worker_shape=quality:{quality},helpers_visible:{helpers_visible},helper_target:{helper_target}"
        ));
    }
    if !pairs.is_empty() {
        lines.push(format!("advice {}", pairs.join(" ")));
    }
}

fn projected_advice_safe_to_enable_pool_workers(
    advice: &str,
    helper_runtime_block: bool,
) -> Option<bool> {
    let safe_to_enable_pool_workers = json_bool_field(advice, "safe_to_enable_pool_workers");
    if helper_runtime_block && safe_to_enable_pool_workers != Some(false) {
        Some(false)
    } else {
        safe_to_enable_pool_workers
    }
}

fn projected_advice_next_step(advice: &str, helper_runtime_block: bool) -> Option<String> {
    if helper_runtime_block && json_bool_field(advice, "safe_to_enable_pool_workers") != Some(false)
    {
        return Some("fix_helper_metal_or_gpu_layers_before_more_pool_workers".to_owned());
    }
    json_string_field(advice, "next_step")
}

fn projected_advice_reason(advice: &str, helper_runtime_block: bool) -> Option<String> {
    if helper_runtime_block && json_bool_field(advice, "safe_to_enable_pool_workers") != Some(false)
    {
        return Some("helper_workers_not_gpu_accelerated".to_owned());
    }
    json_string_field(advice, "reason")
}

fn projected_advice_kind(advice: &str, helper_runtime_block: bool) -> Option<String> {
    if helper_runtime_block && json_bool_field(advice, "safe_to_enable_pool_workers") != Some(false)
    {
        return Some("error".to_owned());
    }
    json_string_field(advice, "kind")
}

fn push_model_pool_status_contract(lines: &mut Vec<String>, body: &str) {
    let mut pairs = Vec::new();
    if let Some(helper_roles) = json_string_array_field(body, "helper_roles") {
        pairs.push(format!("helper_roles={}", helper_roles.join(",")));
    }
    if let Some(expected_helper_roles) = json_string_array_field(body, "expected_helper_roles") {
        pairs.push(format!(
            "expected_helper_roles={}",
            expected_helper_roles.join(",")
        ));
    }
    if let Some(missing_helper_roles) = json_string_array_field(body, "missing_helper_roles") {
        pairs.push(format!(
            "missing_helper_roles={}",
            missing_helper_roles.join(",")
        ));
    }
    if let Some(helper_cpu_or_no_gpu_roles) =
        json_string_array_field(body, "helper_cpu_or_no_gpu_roles")
    {
        pairs.push(format!(
            "helper_cpu_or_no_gpu_roles={}",
            helper_cpu_or_no_gpu_roles.join(",")
        ));
    }
    if let Some(blocking_helper_cpu_or_no_gpu_roles) =
        json_string_array_field(body, "blocking_helper_cpu_or_no_gpu_roles")
    {
        pairs.push(format!(
            "blocking_helper_cpu_or_no_gpu_roles={}",
            blocking_helper_cpu_or_no_gpu_roles.join(",")
        ));
    }
    if let Some(allowed_cpu_fallback_helper_roles) =
        json_string_array_field(body, "allowed_cpu_fallback_helper_roles")
    {
        pairs.push(format!(
            "allowed_cpu_fallback_helper_roles={}",
            allowed_cpu_fallback_helper_roles.join(",")
        ));
    }
    if let Some(recommended_launch_order) =
        json_string_array_field(body, "recommended_launch_order")
    {
        pairs.push(format!(
            "recommended_launch_order={}",
            recommended_launch_order.join(",")
        ));
    }
    if !pairs.is_empty() {
        lines.push(format!("status_contract {}", pairs.join(" ")));
    }
}

fn push_manifest_workers(lines: &mut Vec<String>, body: &str) {
    let Some(workers) = json_array_field(body, "workers") else {
        return;
    };
    let items = json_object_items(workers);
    if items.is_empty() {
        lines.push("manifest_workers=none".to_owned());
        return;
    }
    lines.push(format!("manifest_workers={}", items.len()));
    for item in items {
        let role = json_string_field(item, "role").unwrap_or_else(|| "unknown".to_owned());
        let port = json_number_field(item, "port").unwrap_or_else(|| "unknown".to_owned());
        let base_url = json_string_field(item, "base_url").unwrap_or_else(|| "unknown".to_owned());
        let enabled = json_bool_field(item, "enabled_by_default")
            .or_else(|| json_bool_field(item, "enabled"))
            .map(bool_text)
            .unwrap_or_else(|| "unknown".to_owned());
        let low_priority = json_bool_field(item, "low_priority")
            .map(bool_text)
            .unwrap_or_else(|| "unknown".to_owned());
        let model_class =
            json_string_field(item, "model_class").unwrap_or_else(|| "unknown".to_owned());
        let suggested_quant =
            json_string_field(item, "suggested_quant").unwrap_or_else(|| "unknown".to_owned());
        let context = json_number_field(item, "default_context_tokens")
            .or_else(|| json_number_field(item, "context_window"))
            .unwrap_or_else(|| "unknown".to_owned());
        let max_tokens =
            json_number_field(item, "default_max_tokens").unwrap_or_else(|| "unknown".to_owned());
        let runtime_backend =
            json_string_field(item, "runtime_backend").unwrap_or_else(|| "unknown".to_owned());
        let runtime_device =
            json_string_field(item, "runtime_device").unwrap_or_else(|| "unknown".to_owned());
        let runtime_accelerator =
            json_string_field(item, "runtime_accelerator").unwrap_or_else(|| "unknown".to_owned());
        let gpu_layers =
            json_number_field(item, "gpu_layers").unwrap_or_else(|| "unknown".to_owned());
        lines.push(format!(
            "manifest_worker role={role} port={port} enabled={enabled} low_priority={low_priority} base_url={base_url} context={context} max_tokens={max_tokens} model_class={model_class} suggested_quant={suggested_quant} runtime_backend={runtime_backend} runtime_device={runtime_device} runtime_accelerator={runtime_accelerator} gpu_layers={gpu_layers}"
        ));
    }
}

fn push_metrics_object(lines: &mut Vec<String>, body: &str, field: &str, label: &str) {
    let Some(metrics) = json_object_field(body, field).and_then(metric_pairs) else {
        return;
    };
    lines.push(format!("{label} {metrics}"));
}

fn push_metrics_array(lines: &mut Vec<String>, body: &str, field: &str, label: &str) {
    let Some(metrics) = json_array_field(body, field) else {
        return;
    };
    let items = json_object_items(metrics);
    if items.is_empty() {
        return;
    }
    lines.push(format!("{label}s={}", items.len()));
    for item in items {
        let role = json_string_field(item, "role").unwrap_or_else(|| "unknown".to_owned());
        if let Some(metrics) = metric_pairs(item) {
            lines.push(format!("{label} role={role} {metrics}"));
        }
    }
}

fn push_capacity_object(lines: &mut Vec<String>, body: &str) {
    let Some(capacity) = json_object_field(body, "capacity") else {
        return;
    };
    let mut pairs = Vec::new();
    push_string_pair(&mut pairs, capacity, "policy");
    push_bool_pair(&mut pairs, capacity, "expansion_allowed");
    push_string_pair(&mut pairs, capacity, "recommendation");
    for field in [
        "worker_count",
        "healthy_worker_count",
        "helper_worker_count",
        "healthy_helper_worker_count",
        "metal_worker_count",
        "cpu_worker_count",
        "unknown_runtime_worker_count",
        "zero_gpu_layer_worker_count",
    ] {
        push_metric_pair(&mut pairs, capacity, field);
    }
    push_bool_pair(&mut pairs, capacity, "quality_runtime_accelerated");
    if !pairs.is_empty() {
        lines.push(format!("capacity {}", pairs.join(" ")));
    }
}

fn push_pool_dispatch(lines: &mut Vec<String>, body: &str) {
    let Some(dispatch) = json_object_field(body, "pool_dispatch") else {
        return;
    };
    let mut pairs = Vec::new();
    push_string_pair(&mut pairs, dispatch, "selected_role");
    push_metric_pair(&mut pairs, dispatch, "selected_port");
    push_string_pair(&mut pairs, dispatch, "selected_base_url");
    push_metric_pair(&mut pairs, dispatch, "context_window");
    push_metric_pair(&mut pairs, dispatch, "default_max_tokens");
    push_string_pair(&mut pairs, dispatch, "runtime_backend");
    push_string_pair(&mut pairs, dispatch, "runtime_device");
    push_string_pair(&mut pairs, dispatch, "runtime_accelerator");
    push_metric_pair(&mut pairs, dispatch, "gpu_layers");
    push_metric_pair(&mut pairs, dispatch, "configured_max_tokens");
    push_metric_pair(&mut pairs, dispatch, "effective_max_tokens");
    push_bool_pair(&mut pairs, dispatch, "max_tokens_clamped");
    push_string_pair(&mut pairs, dispatch, "max_tokens_clamp_reason");
    push_bool_pair(&mut pairs, dispatch, "can_accept_low_priority_task");
    if !pairs.is_empty() {
        lines.push(format!("pool_dispatch {}", pairs.join(" ")));
    }
}

fn push_resource_precheck(lines: &mut Vec<String>, body: &str) {
    let Some(routing_weights) = json_object_field(body, "routing_weights") else {
        return;
    };
    let Some(resource_precheck) = json_object_field(routing_weights, "resource_precheck") else {
        return;
    };
    let mut pairs = Vec::new();
    push_string_pair(&mut pairs, resource_precheck, "strategy");
    push_string_pair(&mut pairs, resource_precheck, "pressure");
    push_bool_pair(&mut pairs, resource_precheck, "allow_dispatch");
    push_string_pair(&mut pairs, resource_precheck, "reason");
    push_metric_pair(&mut pairs, resource_precheck, "total_in_flight");
    if let Some(avoid_roles) = json_string_array_field(resource_precheck, "avoid_roles") {
        pairs.push(format!("avoid_roles={}", avoid_roles.join(",")));
    }
    if !pairs.is_empty() {
        lines.push(format!("resource_precheck {}", pairs.join(" ")));
    }
}

fn push_dependency_precheck(lines: &mut Vec<String>, body: &str) {
    let Some(dependency_precheck) = json_object_field(body, "dependency_precheck") else {
        return;
    };
    let mut pairs = Vec::new();
    push_string_pair(&mut pairs, dependency_precheck, "strategy");
    push_bool_pair(&mut pairs, dependency_precheck, "checked");
    push_string_pair(&mut pairs, dependency_precheck, "requested_role");
    push_bool_pair(&mut pairs, dependency_precheck, "allow_dispatch");
    push_string_pair(&mut pairs, dependency_precheck, "reason");
    if let Some(required_roles) = json_string_array_field(dependency_precheck, "required_roles") {
        pairs.push(format!("required_roles={}", required_roles.join(",")));
    }
    if let Some(completed_roles) = json_string_array_field(dependency_precheck, "completed_roles") {
        pairs.push(format!("completed_roles={}", completed_roles.join(",")));
    }
    if let Some(missing_roles) = json_string_array_field(dependency_precheck, "missing_roles") {
        pairs.push(format!("missing_roles={}", missing_roles.join(",")));
    }
    if !pairs.is_empty() {
        lines.push(format!("dependency_precheck {}", pairs.join(" ")));
    }
}

fn push_budget_policy(lines: &mut Vec<String>, body: &str) {
    let dispatch = json_object_field(body, "pool_dispatch");
    let role = json_string_field(body, "selected_role")
        .or_else(|| dispatch.and_then(|dispatch| json_string_field(dispatch, "selected_role")));
    let configured_max_tokens = json_number_field(body, "configured_max_tokens").or_else(|| {
        dispatch.and_then(|dispatch| json_number_field(dispatch, "configured_max_tokens"))
    });
    let effective_max_tokens = json_number_field(body, "effective_max_tokens").or_else(|| {
        dispatch.and_then(|dispatch| json_number_field(dispatch, "effective_max_tokens"))
    });
    let selected_default_max_tokens = json_number_field(body, "selected_default_max_tokens")
        .or_else(|| {
            dispatch.and_then(|dispatch| json_number_field(dispatch, "default_max_tokens"))
        });
    let max_tokens_clamped = json_bool_field(body, "max_tokens_clamped")
        .or_else(|| dispatch.and_then(|dispatch| json_bool_field(dispatch, "max_tokens_clamped")));
    let clamp_reason = json_string_field(body, "max_tokens_clamp_reason").or_else(|| {
        dispatch.and_then(|dispatch| json_string_field(dispatch, "max_tokens_clamp_reason"))
    });
    let can_accept_low_priority = dispatch
        .and_then(|dispatch| json_bool_field(dispatch, "can_accept_low_priority_task"))
        .map(bool_text);

    if role.is_none()
        && configured_max_tokens.is_none()
        && effective_max_tokens.is_none()
        && selected_default_max_tokens.is_none()
        && max_tokens_clamped.is_none()
    {
        return;
    }

    let role_value = role.as_deref().unwrap_or("unknown");
    let policy = budget_policy_name(role_value, max_tokens_clamped);
    let mut pairs = vec![format!("budget_policy={policy}")];
    pairs.push(format!("role={role_value}"));
    push_optional_pair(&mut pairs, "configured_max_tokens", configured_max_tokens);
    push_optional_pair(&mut pairs, "effective_max_tokens", effective_max_tokens);
    push_optional_pair(
        &mut pairs,
        "selected_default_max_tokens",
        selected_default_max_tokens,
    );
    if let Some(clamped) = max_tokens_clamped {
        pairs.push(format!("max_tokens_clamped={}", bool_text(clamped)));
    }
    push_optional_pair(&mut pairs, "reason", clamp_reason);
    push_optional_pair(
        &mut pairs,
        "can_accept_low_priority_task",
        can_accept_low_priority,
    );
    lines.push(pairs.join(" "));
}

fn push_route_budget_policy(lines: &mut Vec<String>, route: &ModelPoolRouteSelection) {
    let policy = if route.role == "quality" {
        "quality_worker_budget_preserved"
    } else {
        "helper_worker_budget"
    };
    let mut pairs = vec![
        format!("budget_policy={policy}"),
        format!("role={}", route.role),
    ];
    if let Some(effective_max_tokens) = route.effective_max_tokens.or(route.default_max_tokens) {
        pairs.push(format!("effective_max_tokens={effective_max_tokens}"));
    }
    if let Some(default_max_tokens) = route.default_max_tokens {
        pairs.push(format!("selected_default_max_tokens={default_max_tokens}"));
    }
    lines.push(pairs.join(" "));
}

fn budget_policy_name(role: &str, max_tokens_clamped: Option<bool>) -> &'static str {
    match (role, max_tokens_clamped) {
        ("quality", Some(false) | None) => "quality_worker_budget_preserved",
        ("quality", Some(true)) => "quality_worker_limited",
        (_, Some(true)) => "helper_worker_limited",
        _ => "helper_worker_budget",
    }
}

fn metric_pairs(body: &str) -> Option<String> {
    let mut pairs = Vec::new();
    for field in [
        "route_count",
        "selected_count",
        "blocked_count",
        "in_flight",
        "queue_depth",
        "avg_latency_ms",
        "p95_latency_ms",
        "tokens_per_sec",
        "runtime_tokens",
        "latency_ms",
        "success_count",
        "failure_count",
    ] {
        push_metric_pair(&mut pairs, body, field);
    }
    for field in ["worker_forwarded", "budget_fairness_blocked"] {
        if let Some(value) = json_bool_field(body, field) {
            pairs.push(format!("{field}={}", bool_text(value)));
        }
    }
    (!pairs.is_empty()).then(|| pairs.join(" "))
}

fn push_string_pair(pairs: &mut Vec<String>, body: &str, field: &str) {
    if let Some(value) = json_string_field(body, field) {
        pairs.push(format!("{field}={value}"));
    }
}

fn push_bool_pair(pairs: &mut Vec<String>, body: &str, field: &str) {
    if let Some(value) = json_bool_field(body, field) {
        pairs.push(format!("{field}={}", bool_text(value)));
    }
}

fn push_metric_pair(pairs: &mut Vec<String>, body: &str, field: &str) {
    if let Some(value) = json_number_field(body, field) {
        pairs.push(format!("{field}={value}"));
    }
}

fn push_field_line(lines: &mut Vec<String>, name: &str, value: Option<String>) {
    if let Some(value) = value {
        lines.push(format!("{name}={value}"));
    }
}

fn push_optional_pair(pairs: &mut Vec<String>, name: &str, value: Option<String>) {
    if let Some(value) = value {
        pairs.push(format!("{name}={value}"));
    }
}

fn bool_text(value: bool) -> String {
    if value { "true" } else { "false" }.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::json::{
        json_bool_field, json_number_field, json_string_array_field, json_string_field,
    };

    #[test]
    fn summarizes_model_pool_manifest() {
        let summary = model_pool_manifest_summary(
            "{\"ok\":true,\"contract_version\":\"gemma-chain.v1\",\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"manifest_kind\":\"rust-norion.model-pool\",\"capacity_policy\":{\"policy\":\"one_quality_plus_small_helpers\",\"target_host\":\"apple_silicon\",\"avoid_extra_12b\":true,\"max_quality_12b_workers\":1,\"quality_role\":\"quality\",\"quality_required_context_tokens\":262144,\"helper_roles\":[\"summary\",\"router\",\"review\",\"index\",\"test-gate\"],\"helper_context_tokens_total\":24576,\"helper_default_max_tokens_total\":2816,\"recommended_launch_order\":[\"quality\",\"summary\",\"router\",\"review\",\"index\",\"test-gate\"],\"expansion_gate\":\"quality worker must be reachable\",\"next_step_when_quality_ready\":\"run_short_pool_smoke_then_use_evolution_loop_helper_stage_calls\"},\"advice\":{\"decision_source\":\"model-pool-advice-core\",\"policy\":\"one_quality_12b_plus_small_helpers\",\"safe_to_enable_pool_workers\":true,\"next_step\":\"run_short_pool_smoke_then_use_evolution_loop_helper_stage_calls\",\"reason\":\"full_helper_pool_visible\",\"kind\":\"busy\",\"extra_quality_12b_detected\":false,\"max_quality_12b_workers\":1,\"quality_worker_count\":1,\"helper_worker_count\":5,\"helper_target_worker_count\":5,\"helper_roles\":[\"summary\",\"router\",\"review\",\"index\",\"test-gate\"],\"recommended_launch_order\":[\"quality\",\"summary\",\"router\",\"review\",\"index\",\"test-gate\"],\"worker_shape\":{\"quality\":1,\"helpers_visible\":5,\"helper_target\":5},\"operator_checks\":\"Activity Monitor GPU History and Memory Pressure must stay healthy before adding workers\"},\"workers\":[{\"role\":\"quality\",\"port\":8686,\"base_url\":\"http://127.0.0.1:8686\",\"enabled_by_default\":true,\"model_class\":\"Gemma 12B Q8\",\"suggested_quant\":\"Q8\",\"default_context_tokens\":262144,\"default_max_tokens\":262144,\"low_priority\":false,\"runtime_backend\":\"llama.cpp\",\"runtime_device\":\"metal\",\"runtime_accelerator\":\"metal\",\"gpu_layers\":99},{\"role\":\"summary\",\"port\":8687,\"base_url\":\"http://127.0.0.1:8687\",\"enabled_by_default\":true,\"model_class\":\"small Gemma\",\"suggested_quant\":\"Q4\",\"default_context_tokens\":8192,\"default_max_tokens\":768,\"low_priority\":true,\"runtime_backend\":\"llama.cpp\",\"runtime_device\":\"metal\",\"runtime_accelerator\":\"metal\",\"gpu_layers\":80}]}",
        )
        .unwrap();

        assert!(summary.contains("SmartSteam model pool manifest"));
        assert!(summary.contains("section=manifest_json"));
        let manifest_json = summary
            .lines()
            .skip_while(|line| *line != "section=manifest_json")
            .nth(1)
            .expect("manifest_json section should include a JSON payload line");
        assert_eq!(
            json_string_field(manifest_json, "schema").as_deref(),
            Some("smartsteam.forge.model_pool_manifest.v1")
        );
        assert_eq!(
            json_number_field(manifest_json, "worker_count").as_deref(),
            Some("2")
        );
        assert_eq!(
            json_string_array_field(manifest_json, "worker_roles"),
            Some(vec!["quality".to_owned(), "summary".to_owned()])
        );
        assert!(summary.contains("contract_version=gemma-chain.v1"));
        assert!(summary.contains("manifest_kind=rust-norion.model-pool"));
        assert!(summary.contains("capacity_policy policy=one_quality_plus_small_helpers"));
        assert!(summary.contains("target_host=apple_silicon"));
        assert!(summary.contains("avoid_extra_12b=true"));
        assert!(summary.contains("max_quality_12b_workers=1"));
        assert!(summary.contains("helper_roles=summary,router,review,index,test-gate"));
        assert!(
            summary
                .contains("recommended_launch_order=quality,summary,router,review,index,test-gate")
        );
        assert!(summary.contains("advice decision_source=model-pool-advice-core"));
        assert!(summary.contains("policy=one_quality_12b_plus_small_helpers"));
        assert!(summary.contains("safe_to_enable_pool_workers=true"));
        assert!(
            summary.contains(
                "next_step=run_short_pool_smoke_then_use_evolution_loop_helper_stage_calls"
            )
        );
        assert!(summary.contains("reason=full_helper_pool_visible"));
        assert!(summary.contains("extra_quality_12b_detected=false"));
        assert!(summary.contains("quality_worker_count=1"));
        assert!(summary.contains("helper_worker_count=5"));
        assert!(summary.contains("helper_target_worker_count=5"));
        assert!(summary.contains("worker_shape=quality:1,helpers_visible:5,helper_target:5"));
        assert!(summary.contains("manifest_workers=2"));
        assert!(summary.contains("manifest_worker role=quality port=8686"));
        assert!(summary.contains("context=262144"));
        assert!(summary.contains("max_tokens=262144"));
        assert!(summary.contains("manifest_worker role=summary port=8687"));
        assert!(summary.contains("low_priority=true"));
        assert!(summary.contains("runtime_accelerator=metal"));
    }

    #[test]
    fn summarizes_model_pool_status() {
        let summary = model_pool_status_summary(
            "{\"ok\":true,\"contract_version\":\"model-pool.v1\",\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"launch_allowed\":false,\"reason\":\"quality_worker_down\",\"launch_block_reason\":\"quality_worker_down\",\"chain_classification\":\"quality_worker_down\",\"min_context_tokens\":null,\"quality_ready\":false,\"quality_context_tokens\":262144,\"quality_context_required_tokens\":262144,\"quality_context_sufficient\":false,\"quality_default_context_tokens\":262144,\"quality_default_max_tokens\":262144,\"quality_block_reason\":\"tcp_unreachable\",\"blocked_policy\":\"model-pool launch is blocked until the quality worker is reachable, healthy, and has the required context window\",\"worker_count\":1,\"healthy_worker_count\":0,\"helper_roles\":[],\"expected_helper_roles\":[\"summary\",\"router\",\"review\",\"index\",\"test-gate\"],\"missing_helper_roles\":[\"summary\",\"router\",\"review\",\"index\",\"test-gate\"],\"recommended_launch_order\":[\"quality\",\"summary\",\"router\",\"review\",\"index\",\"test-gate\"],\"capacity\":{\"policy\":\"one_quality_plus_small_helpers\",\"expansion_allowed\":false,\"recommendation\":\"restore_quality_gate_first\",\"worker_count\":1,\"healthy_worker_count\":0,\"helper_worker_count\":0,\"healthy_helper_worker_count\":0,\"metal_worker_count\":0,\"cpu_worker_count\":0,\"unknown_runtime_worker_count\":0,\"zero_gpu_layer_worker_count\":0,\"quality_runtime_accelerated\":null},\"advice\":{\"decision_source\":\"model-pool-advice-core\",\"policy\":\"one_quality_12b_plus_small_helpers\",\"safe_to_enable_pool_workers\":false,\"next_step\":\"start_or_fix_quality_worker_8686\",\"reason\":\"quality_worker_not_ready\",\"kind\":\"error\",\"extra_quality_12b_detected\":false,\"max_quality_12b_workers\":1,\"quality_worker_count\":1,\"helper_worker_count\":0,\"healthy_helper_worker_count\":0,\"helper_target_worker_count\":5,\"helper_roles\":[],\"expected_helper_roles\":[\"summary\",\"router\",\"review\",\"index\",\"test-gate\"],\"missing_helper_roles\":[\"summary\",\"router\",\"review\",\"index\",\"test-gate\"],\"recommended_launch_order\":[\"quality\",\"summary\",\"router\",\"review\",\"index\",\"test-gate\"],\"worker_shape\":{\"quality\":1,\"helpers_visible\":0,\"helpers_healthy\":0,\"helper_target\":5}},\"workers\":[{\"role\":\"quality\",\"status\":\"unreachable\",\"ready\":false,\"base_url\":\"http://127.0.0.1:8686\",\"default_context_tokens\":262144,\"default_max_tokens\":262144,\"runtime_backend\":\"llama.cpp\",\"runtime_device\":\"metal\",\"runtime_accelerator\":\"metal\",\"gpu_layers\":99,\"role_block_reason\":\"tcp_unreachable\"}]}",
        )
        .unwrap();

        assert!(summary.contains("SmartSteam model pool status"));
        assert!(summary.contains("section=status_json"));
        let status_json = summary
            .lines()
            .skip_while(|line| *line != "section=status_json")
            .nth(1)
            .expect("status_json section should include a JSON payload line");
        assert_eq!(
            json_string_field(status_json, "schema").as_deref(),
            Some("smartsteam.forge.model_pool_status.v1")
        );
        assert_eq!(
            json_bool_field(status_json, "safe_to_enable_pool_workers"),
            Some(false)
        );
        assert_eq!(
            json_string_field(status_json, "next_step").as_deref(),
            Some("start_or_fix_quality_worker_8686")
        );
        assert!(summary.contains("launch_allowed=false"));
        assert!(summary.contains("launch_block_reason=quality_worker_down"));
        assert!(summary.contains("chain_classification=quality_worker_down"));
        assert!(summary.contains("quality_ready=false"));
        assert!(summary.contains("quality_context_tokens=262144"));
        assert!(summary.contains("quality_context_required_tokens=262144"));
        assert!(summary.contains("quality_context_sufficient=false"));
        assert!(summary.contains("quality_default_context_tokens=262144"));
        assert!(summary.contains("quality_default_max_tokens=262144"));
        assert!(summary.contains("quality_block_reason=tcp_unreachable"));
        assert!(summary.contains("blocked_policy=model-pool launch is blocked"));
        assert!(
            summary.contains(
                "status_contract helper_roles= expected_helper_roles=summary,router,review,index,test-gate missing_helper_roles=summary,router,review,index,test-gate recommended_launch_order=quality,summary,router,review,index,test-gate"
            )
        );
        assert!(summary.contains("capacity policy=one_quality_plus_small_helpers"));
        assert!(summary.contains("expansion_allowed=false"));
        assert!(summary.contains("recommendation=restore_quality_gate_first"));
        assert!(summary.contains("advice decision_source=model-pool-advice-core"));
        assert!(summary.contains("safe_to_enable_pool_workers=false"));
        assert!(summary.contains("next_step=start_or_fix_quality_worker_8686"));
        assert!(summary.contains("reason=quality_worker_not_ready"));
        assert!(summary.contains("extra_quality_12b_detected=false"));
        assert!(summary.contains("expected_helper_roles=summary,router,review,index,test-gate"));
        assert!(summary.contains("missing_helper_roles=summary,router,review,index,test-gate"));
        assert!(summary.contains("worker_shape=quality:1,helpers_visible:0,helper_target:5"));
        assert!(summary.contains("worker role=quality"));
        assert!(summary.contains("runtime_backend=llama.cpp"));
        assert!(summary.contains("runtime_device=metal"));
        assert!(summary.contains("runtime_accelerator=metal"));
        assert!(summary.contains("gpu_layers=99"));
    }

    #[test]
    fn summarizes_model_pool_status_metrics() {
        let summary = model_pool_status_summary(
            "{\"ok\":true,\"contract_version\":\"model-pool.v1\",\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"launch_allowed\":true,\"reason\":\"ready\",\"worker_count\":2,\"healthy_worker_count\":2,\"route_metrics\":{\"route_count\":12,\"blocked_count\":2,\"in_flight\":1,\"avg_latency_ms\":250.5,\"tokens_per_sec\":18.25},\"worker_metrics\":[{\"role\":\"summary\",\"route_count\":7,\"success_count\":6,\"failure_count\":1,\"avg_latency_ms\":120,\"tokens_per_sec\":22.5},{\"role\":\"review\",\"route_count\":5,\"success_count\":5,\"failure_count\":0,\"queue_depth\":1}],\"workers\":[{\"role\":\"summary\",\"status\":\"healthy\",\"role_ready\":true,\"base_url\":\"http://127.0.0.1:8687\",\"default_context_tokens\":8192,\"default_max_tokens\":768,\"runtime_device\":\"metal\",\"runtime_accelerator\":\"metal\",\"gpu_layers\":99,\"role_block_reason\":\"none\",\"in_flight\":1,\"tokens_per_sec\":22.5},{\"role\":\"review\",\"status\":\"healthy\",\"role_ready\":true,\"base_url\":\"http://127.0.0.1:8688\",\"default_context_tokens\":4096,\"default_max_tokens\":1024,\"runtime_device\":\"cpu\",\"runtime_accelerator\":\"none\",\"gpu_layers\":0,\"role_block_reason\":\"none\"}]}",
        )
        .unwrap();

        assert!(summary.contains("route_metrics route_count=12"));
        assert!(summary.contains("blocked_count=2"));
        assert!(summary.contains("avg_latency_ms=250.5"));
        assert!(summary.contains("worker_metrics=2"));
        assert!(summary.contains("worker_metric role=summary route_count=7"));
        assert!(summary.contains("success_count=6"));
        assert!(summary.contains("worker_metric role=review route_count=5"));
        assert!(summary.contains("queue_depth=1"));
        assert!(summary.contains("worker role=summary"));
        assert!(summary.contains("worker role=review"));
        assert!(summary.contains("ready=true"));
        assert!(summary.contains("in_flight=1"));
        assert!(summary.contains(
            "runtime_shape workers=2 metal_workers=1 cpu_or_no_gpu_workers=1 zero_gpu_layer_workers=1 unknown_runtime_workers=0 cpu_or_no_gpu_roles=review"
        ));
    }

    #[test]
    fn summarizes_model_pool_status_projects_runtime_guard_into_text_advice() {
        let summary = model_pool_status_summary(
            "{\"ok\":true,\"contract_version\":\"model-pool.v1\",\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"launch_allowed\":true,\"reason\":\"ready\",\"worker_count\":3,\"healthy_worker_count\":3,\"workers\":[{\"role\":\"quality\",\"status\":\"healthy\",\"ready\":true,\"runtime_device\":\"metal\",\"runtime_accelerator\":\"metal\",\"gpu_layers\":999},{\"role\":\"summary\",\"status\":\"healthy\",\"ready\":true,\"runtime_device\":\"metal\",\"runtime_accelerator\":\"metal\",\"gpu_layers\":999},{\"role\":\"review\",\"status\":\"healthy\",\"ready\":true,\"runtime_device\":\"cpu\",\"runtime_accelerator\":\"accelerate\",\"gpu_layers\":0}],\"advice\":{\"decision_source\":\"model-pool-advice-core\",\"policy\":\"one_quality_12b_plus_small_helpers\",\"safe_to_enable_pool_workers\":true,\"next_step\":\"run_short_pool_smoke_then_use_evolution_loop_helper_stage_calls\",\"reason\":\"full_helper_pool_visible\",\"kind\":\"busy\",\"extra_quality_12b_detected\":false,\"quality_worker_count\":1,\"helper_worker_count\":2,\"helper_target_worker_count\":5,\"helper_roles\":[\"summary\",\"review\"],\"expected_helper_roles\":[\"summary\",\"router\",\"review\",\"index\",\"test-gate\"],\"missing_helper_roles\":[\"router\",\"index\",\"test-gate\"],\"recommended_launch_order\":[\"quality\",\"summary\",\"router\",\"review\",\"index\",\"test-gate\"],\"worker_shape\":{\"quality\":1,\"helpers_visible\":2,\"helper_target\":5}}}",
        )
        .unwrap();

        assert!(
            summary.contains("runtime_shape workers=3 metal_workers=2 cpu_or_no_gpu_workers=1")
        );
        assert!(summary.contains("cpu_or_no_gpu_roles=review"));
        assert!(summary.contains("safe_to_enable_pool_workers=false"));
        assert!(
            summary.contains("next_step=fix_helper_metal_or_gpu_layers_before_more_pool_workers")
        );
        assert!(summary.contains("reason=helper_workers_not_gpu_accelerated"));
        assert!(summary.contains("kind=error"));
        assert!(summary.contains("helper_cpu_or_no_gpu_roles=review"));
        let status_json = summary
            .lines()
            .skip_while(|line| *line != "section=status_json")
            .nth(1)
            .expect("status_json section should include a JSON payload line");
        assert_eq!(
            json_bool_field(status_json, "safe_to_enable_pool_workers"),
            Some(false)
        );
        assert_eq!(
            json_string_field(status_json, "reason_detail").as_deref(),
            Some("helper_workers_not_gpu_accelerated")
        );
    }

    #[test]
    fn summarizes_model_pool_status_distinguishes_index_cpu_fallback() {
        let summary = model_pool_status_summary(
            "{\"ok\":true,\"contract_version\":\"model-pool.v1\",\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"launch_allowed\":true,\"reason\":\"ready\",\"worker_count\":3,\"healthy_worker_count\":3,\"helper_cpu_or_no_gpu_roles\":[\"index\"],\"blocking_helper_cpu_or_no_gpu_roles\":[],\"allowed_cpu_fallback_helper_roles\":[\"index\"],\"workers\":[{\"role\":\"quality\",\"status\":\"healthy\",\"ready\":true,\"runtime_device\":\"metal\",\"runtime_accelerator\":\"metal\",\"gpu_layers\":999},{\"role\":\"summary\",\"status\":\"healthy\",\"ready\":true,\"runtime_device\":\"metal\",\"runtime_accelerator\":\"metal\",\"gpu_layers\":999},{\"role\":\"index\",\"status\":\"healthy\",\"ready\":true,\"runtime_device\":\"cpu\",\"runtime_accelerator\":\"accelerate\",\"gpu_layers\":0}],\"advice\":{\"decision_source\":\"model-pool-advice-core\",\"policy\":\"one_quality_12b_plus_small_helpers\",\"safe_to_enable_pool_workers\":true,\"next_step\":\"run_short_pool_smoke_then_use_evolution_loop_helper_stage_calls\",\"reason\":\"full_helper_pool_visible\",\"kind\":\"busy\",\"extra_quality_12b_detected\":false,\"quality_worker_count\":1,\"helper_worker_count\":2,\"helper_target_worker_count\":5,\"helper_roles\":[\"summary\",\"index\"],\"expected_helper_roles\":[\"summary\",\"router\",\"review\",\"index\",\"test-gate\"],\"missing_helper_roles\":[\"router\",\"review\",\"test-gate\"],\"helper_cpu_or_no_gpu_roles\":[\"index\"],\"blocking_helper_cpu_or_no_gpu_roles\":[],\"allowed_cpu_fallback_helper_roles\":[\"index\"],\"recommended_launch_order\":[\"quality\",\"summary\",\"router\",\"review\",\"index\",\"test-gate\"],\"worker_shape\":{\"quality\":1,\"helpers_visible\":2,\"helper_target\":5}}}",
        )
        .unwrap();

        assert!(summary.contains("cpu_or_no_gpu_roles=index"));
        assert!(summary.contains("safe_to_enable_pool_workers=true"));
        assert!(summary.contains("helper_cpu_or_no_gpu_roles=index"));
        assert!(summary.contains("blocking_helper_cpu_or_no_gpu_roles="));
        assert!(summary.contains("allowed_cpu_fallback_helper_roles=index"));
        let status_json = summary
            .lines()
            .skip_while(|line| *line != "section=status_json")
            .nth(1)
            .expect("status_json section should include a JSON payload line");
        assert_eq!(
            json_bool_field(status_json, "safe_to_enable_pool_workers"),
            Some(true)
        );
        assert_eq!(
            json_string_field(status_json, "reason_detail").as_deref(),
            Some("full_helper_pool_visible")
        );
    }

    #[test]
    fn summarizes_model_pool_route() {
        let summary = model_pool_route_summary(
            "{\"ok\":true,\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"task_kind\":\"review\",\"route_allowed\":true,\"reason\":\"ready\",\"role_candidates\":[\"review\",\"quality\"],\"quality_context_tokens\":262144,\"quality_context_required_tokens\":262144,\"quality_context_sufficient\":true,\"quality_block_reason\":\"none\",\"selected_role\":\"quality\",\"selected_base_url\":\"http://127.0.0.1:8686\",\"selected_port\":8686,\"selected_default_max_tokens\":262144,\"selected_context_window\":262144,\"pool_dispatch\":{\"selected_role\":\"quality\",\"selected_port\":8686,\"selected_base_url\":\"http://127.0.0.1:8686\",\"context_window\":262144,\"default_max_tokens\":262144,\"runtime_backend\":\"llama.cpp\",\"runtime_device\":\"metal\",\"runtime_accelerator\":\"metal\",\"gpu_layers\":99,\"configured_max_tokens\":262144,\"effective_max_tokens\":262144,\"max_tokens_clamped\":false,\"max_tokens_clamp_reason\":\"quality_worker_request_budget_preserved\",\"can_accept_low_priority_task\":false},\"candidate_workers\":[]}",
        )
        .unwrap();

        assert!(summary.contains("SmartSteam model pool route plan"));
        assert!(summary.contains("section=route_json"));
        let route_json = summary
            .lines()
            .skip_while(|line| *line != "section=route_json")
            .nth(1)
            .expect("route_json section should include a JSON payload line");
        assert_eq!(
            json_string_field(route_json, "schema").as_deref(),
            Some("smartsteam.forge.model_pool_route.v1")
        );
        assert_eq!(
            json_string_field(route_json, "selected_role").as_deref(),
            Some("quality")
        );
        assert_eq!(
            json_number_field(route_json, "effective_max_tokens").as_deref(),
            Some("262144")
        );
        assert!(summary.contains("role_candidates=review,quality"));
        assert!(summary.contains("quality_context_tokens=262144"));
        assert!(summary.contains("quality_context_required_tokens=262144"));
        assert!(summary.contains("quality_context_sufficient=true"));
        assert!(summary.contains("selected_role=quality"));
        assert!(summary.contains("pool_dispatch selected_role=quality"));
        assert!(summary.contains("budget_policy=quality_worker_budget_preserved"));
        assert!(summary.contains("max_tokens_clamped=false"));
        assert!(summary.contains("runtime_backend=llama.cpp"));
        assert!(summary.contains("runtime_device=metal"));
        assert!(summary.contains("runtime_accelerator=metal"));
        assert!(summary.contains("gpu_layers=99"));
    }

    #[test]
    fn summarizes_model_pool_route_metrics() {
        let summary = model_pool_route_summary(
            "{\"ok\":true,\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"task_kind\":\"index\",\"route_allowed\":true,\"reason\":\"ready\",\"role_candidates\":[\"index\",\"summary\"],\"selected_role\":\"index\",\"selected_base_url\":\"http://127.0.0.1:8690\",\"selected_port\":8690,\"selected_default_max_tokens\":512,\"selected_context_window\":4096,\"route_metrics\":{\"route_count\":4,\"selected_count\":3,\"blocked_count\":1,\"worker_forwarded\":true},\"candidate_workers\":[{\"role\":\"index\",\"status\":\"healthy\",\"role_ready\":true,\"base_url\":\"http://127.0.0.1:8690\",\"context_window\":4096,\"default_max_tokens\":512,\"role_block_reason\":\"none\",\"avg_latency_ms\":95,\"tokens_per_sec\":30.5}]}",
        )
        .unwrap();

        assert!(summary.contains("task_kind=index"));
        assert!(summary.contains("route_metrics route_count=4"));
        assert!(summary.contains("selected_count=3"));
        assert!(summary.contains("worker_forwarded=true"));
        assert!(summary.contains("budget_policy=helper_worker_budget"));
        assert!(summary.contains("selected_default_max_tokens=512"));
        assert!(summary.contains("worker role=index"));
        assert!(summary.contains("ready=true"));
        assert!(summary.contains("avg_latency_ms=95"));
        assert!(summary.contains("tokens_per_sec=30.5"));
    }

    #[test]
    fn summarizes_resource_precheck_route_evidence() {
        let summary = model_pool_route_summary(
            "{\"ok\":true,\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"task_kind\":\"auto\",\"route_allowed\":true,\"reason\":\"ready\",\"role_candidates\":[\"router\",\"summary\"],\"routing_weights\":{\"resource_precheck\":{\"strategy\":\"resource_precheck_v1\",\"pressure\":\"high\",\"allow_dispatch\":true,\"reason\":\"resource_constrained_candidates_demoted\",\"total_in_flight\":4,\"avoid_roles\":[\"summary\"]}},\"selected_role\":\"router\",\"selected_base_url\":\"http://127.0.0.1:8689\",\"selected_default_max_tokens\":512}",
        )
        .unwrap();

        assert!(summary.contains("resource_precheck strategy=resource_precheck_v1"));
        assert!(summary.contains("pressure=high"));
        assert!(summary.contains("allow_dispatch=true"));
        assert!(summary.contains("avoid_roles=summary"));
    }

    #[test]
    fn summarizes_dependency_precheck_route_evidence() {
        let summary = model_pool_route_summary(
            "{\"ok\":true,\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"task_kind\":\"index\",\"route_allowed\":true,\"reason\":\"ready\",\"role_candidates\":[\"index\"],\"dependency_precheck\":{\"strategy\":\"role_dependency_graph_v1\",\"checked\":true,\"requested_role\":\"index\",\"allow_dispatch\":true,\"reason\":\"dependencies_satisfied\",\"required_roles\":[\"summary\",\"router\"],\"completed_roles\":[\"quality\",\"summary\",\"router\"],\"missing_roles\":[]},\"selected_role\":\"index\",\"selected_base_url\":\"http://127.0.0.1:8690\",\"selected_default_max_tokens\":512}",
        )
        .unwrap();

        assert!(summary.contains("dependency_precheck strategy=role_dependency_graph_v1"));
        assert!(summary.contains("checked=true"));
        assert!(summary.contains("requested_role=index"));
        assert!(summary.contains("allow_dispatch=true"));
        assert!(summary.contains("required_roles=summary,router"));
        assert!(summary.contains("completed_roles=quality,summary,router"));
    }

    #[test]
    fn extracts_route_selection_for_pool_call() {
        let route = model_pool_route_selection(
            "{\"ok\":true,\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"task_kind\":\"review\",\"route_allowed\":true,\"reason\":\"ready\",\"selected_role\":\"review\",\"selected_base_url\":\"http://127.0.0.1:8688\",\"selected_default_max_tokens\":1024}",
        )
        .unwrap();

        assert_eq!(route.task_kind, "review");
        assert_eq!(route.role, "review");
        assert_eq!(route.base_url, "http://127.0.0.1:8688");
        assert_eq!(route.default_max_tokens, Some(1024));
        assert_eq!(route.context_window, None);
    }

    #[test]
    fn blocked_route_selection_does_not_send_prompt() {
        let error = model_pool_route_selection(
            "{\"ok\":true,\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"task_kind\":\"review\",\"route_allowed\":false,\"reason\":\"quality_worker_down\"}",
        )
        .unwrap_err();

        assert!(error.contains("quality_worker_down"));
    }

    #[test]
    fn route_selection_rejects_context_insufficient_route() {
        let error = model_pool_route_selection(
            "{\"ok\":true,\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"task_kind\":\"quality\",\"route_allowed\":true,\"reason\":\"ready\",\"quality_context_tokens\":8192,\"quality_context_required_tokens\":262144,\"quality_context_sufficient\":false,\"selected_role\":\"quality\",\"selected_base_url\":\"http://127.0.0.1:8686\"}",
        )
        .unwrap_err();

        assert!(error.contains("quality context insufficient"));
        assert!(error.contains("actual=8192"));
        assert!(error.contains("required=262144"));
    }

    #[test]
    fn route_selection_rejects_resource_precheck_denied_route() {
        let error = model_pool_route_selection(
            "{\"ok\":true,\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"task_kind\":\"router\",\"route_allowed\":true,\"reason\":\"ready\",\"routing_weights\":{\"resource_precheck\":{\"strategy\":\"resource_precheck_v1\",\"pressure\":\"high\",\"allow_dispatch\":false,\"reason\":\"all_candidates_resource_constrained\",\"total_in_flight\":6,\"avoid_roles\":[\"router\"]}},\"selected_role\":\"router\",\"selected_base_url\":\"http://127.0.0.1:8689\"}",
        )
        .unwrap_err();

        assert!(error.contains("resource precheck"));
        assert!(error.contains("all_candidates_resource_constrained"));
    }

    #[test]
    fn route_selection_rejects_dependency_precheck_denied_route() {
        let error = model_pool_route_selection(
            "{\"ok\":true,\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"task_kind\":\"test-gate\",\"route_allowed\":true,\"reason\":\"ready\",\"dependency_precheck\":{\"strategy\":\"role_dependency_graph_v1\",\"checked\":true,\"requested_role\":\"test-gate\",\"allow_dispatch\":false,\"reason\":\"missing_required_roles\",\"required_roles\":[\"review\",\"index\"],\"completed_roles\":[\"quality\",\"summary\"],\"missing_roles\":[\"review\",\"index\"]},\"selected_role\":\"test-gate\",\"selected_base_url\":\"http://127.0.0.1:8688\"}",
        )
        .unwrap_err();

        assert!(error.contains("dependency precheck"));
        assert!(error.contains("missing_required_roles"));
    }

    #[test]
    fn summarizes_worker_chat_response() {
        let route = ModelPoolRouteSelection {
            task_kind: "summary".to_owned(),
            role: "summary".to_owned(),
            base_url: "http://127.0.0.1:8687".to_owned(),
            context_window: Some(8192),
            default_max_tokens: Some(768),
            effective_max_tokens: Some(768),
        };
        let summary = model_pool_worker_answer_summary(
            &route,
            "{\"choices\":[{\"message\":{\"role\":\"assistant\",\"content\":\"短摘要\"}}]}",
        )
        .unwrap();

        assert!(summary.contains("task_kind=summary"));
        assert!(summary.contains("selected_role=summary"));
        assert!(summary.contains("selected_context_window=8192"));
        assert!(summary.contains("selected_default_max_tokens=768"));
        assert!(summary.contains("effective_max_tokens=768"));
        assert!(summary.contains("budget_policy=helper_worker_budget"));
        assert!(summary.contains("answer=短摘要"));
        assert!(summary.contains("section=call_json"));
        let call_json = summary
            .lines()
            .skip_while(|line| *line != "section=call_json")
            .nth(1)
            .expect("call_json section should include a JSON payload line");
        assert_eq!(
            json_string_field(call_json, "schema").as_deref(),
            Some("smartsteam.forge.model_pool_call.v1")
        );
        assert_eq!(
            json_string_field(call_json, "source").as_deref(),
            Some("worker_chat")
        );
        assert_eq!(
            json_string_field(call_json, "answer").as_deref(),
            Some("短摘要")
        );
    }

    #[test]
    fn worker_chat_request_is_openai_compatible() {
        let body = model_pool_worker_chat_request_body("summarize", Some(768));

        assert!(body.contains("\"model\":\"smartsteam-pool-worker\""));
        assert!(body.contains("\"messages\":[{\"role\":\"user\",\"content\":\"summarize\"}]"));
        assert!(body.contains("\"stream\":false"));
        assert!(body.contains("\"max_tokens\":768"));
    }

    #[test]
    fn model_pool_call_request_contains_prompt() {
        let body = model_pool_call_request_body("review", "review this patch", Some(4096));

        assert!(body.contains("\"task_kind\":\"review\""));
        assert!(body.contains("\"prompt\":\"review this patch\""));
        assert!(body.contains("\"max_tokens\":4096"));
    }

    #[test]
    fn summarizes_backend_model_pool_call() {
        let summary = model_pool_call_summary(
            "{\"ok\":true,\"read_only\":false,\"launches_process\":false,\"sends_prompt\":true,\"task_kind\":\"summary\",\"selected_role\":\"summary\",\"selected_base_url\":\"http://127.0.0.1:8687\",\"answer\":\"short summary\"}",
        )
        .unwrap();

        assert!(summary.contains("SmartSteam model pool call"));
        assert!(summary.contains("task_kind=summary"));
        assert!(summary.contains("selected_role=summary"));
        assert!(summary.contains("answer=short summary"));
        assert!(summary.contains("section=call_json"));
        let call_json = summary
            .lines()
            .skip_while(|line| *line != "section=call_json")
            .nth(1)
            .expect("call_json section should include a JSON payload line");
        assert_eq!(
            json_string_field(call_json, "schema").as_deref(),
            Some("smartsteam.forge.model_pool_call.v1")
        );
        assert_eq!(
            json_string_field(call_json, "source").as_deref(),
            Some("backend_call")
        );
        assert_eq!(
            json_string_field(call_json, "answer").as_deref(),
            Some("short summary")
        );
    }

    #[test]
    fn summarizes_backend_model_pool_call_budget() {
        let summary = model_pool_call_summary(
            "{\"ok\":true,\"read_only\":false,\"launches_process\":false,\"sends_prompt\":true,\"task_kind\":\"review\",\"selected_role\":\"review\",\"selected_base_url\":\"http://127.0.0.1:8688\",\"selected_port\":8688,\"selected_default_max_tokens\":1024,\"configured_max_tokens\":262144,\"effective_max_tokens\":1024,\"max_tokens_clamped\":true,\"max_tokens_clamp_reason\":\"low_priority_worker_default_max_tokens\",\"pool_dispatch\":{\"selected_role\":\"review\",\"selected_port\":8688,\"selected_base_url\":\"http://127.0.0.1:8688\",\"context_window\":8192,\"default_max_tokens\":1024,\"configured_max_tokens\":262144,\"effective_max_tokens\":1024,\"max_tokens_clamped\":true,\"max_tokens_clamp_reason\":\"low_priority_worker_default_max_tokens\",\"can_accept_low_priority_task\":true},\"answer\":\"review\"}",
        )
        .unwrap();

        assert!(summary.contains("selected_port=8688"));
        assert!(summary.contains("selected_default_max_tokens=1024"));
        assert!(summary.contains("configured_max_tokens=262144"));
        assert!(summary.contains("effective_max_tokens=1024"));
        assert!(summary.contains("max_tokens_clamped=true"));
        assert!(summary.contains("max_tokens_clamp_reason=low_priority_worker_default_max_tokens"));
        assert!(summary.contains("budget_policy=helper_worker_limited"));
        assert!(summary.contains("reason=low_priority_worker_default_max_tokens"));
        assert!(summary.contains("pool_dispatch selected_role=review"));
        assert!(summary.contains("context_window=8192"));
        assert!(summary.contains("can_accept_low_priority_task=true"));
    }

    #[test]
    fn backend_call_contract_requires_prompt_sending_call_shape() {
        let error = model_pool_call_summary(
            "{\"ok\":true,\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"answer\":\"bad\"}",
        )
        .unwrap_err();

        assert!(error.contains("failed safety contract"));
    }

    #[test]
    fn worker_answer_summary_reports_error_message() {
        let route = ModelPoolRouteSelection {
            task_kind: "review".to_owned(),
            role: "review".to_owned(),
            base_url: "http://127.0.0.1:8688".to_owned(),
            context_window: Some(8192),
            default_max_tokens: Some(1024),
            effective_max_tokens: Some(1024),
        };
        let error = model_pool_worker_answer_summary(
            &route,
            "{\"error\":{\"message\":\"model is still loading\"}}",
        )
        .unwrap_err();

        assert!(error.contains("model is still loading"));
    }

    #[test]
    fn rejects_non_read_only_pool_contract() {
        let error = model_pool_status_summary(
            "{\"ok\":true,\"read_only\":false,\"launches_process\":false,\"sends_prompt\":false}",
        )
        .unwrap_err();

        assert!(error.contains("failed safety contract"));
    }
}
