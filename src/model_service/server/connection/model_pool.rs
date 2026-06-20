use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

mod config;
mod metrics;

use super::super::super::json::{
    option_str_service_json, option_usize_service_json, service_error_json, service_json_string,
    service_json_string_array, write_http_json,
};
use super::super::super::request::{
    ModelServiceModelPoolCallRequest, ModelServiceModelPoolRouteRequest,
};
use super::super::super::response::{
    ModelPoolCallExecutionView, ModelPoolWorkerView, model_pool_dependency_precheck,
    model_pool_launch_block_reason, model_pool_max_tokens_decision, model_pool_quality_gate,
    model_pool_route_candidates_for_context,
    model_service_model_pool_call_blocked_response_json_with_metrics,
    model_service_model_pool_call_blocked_response_json_with_metrics_and_dependency,
    model_service_model_pool_call_response_json_with_metrics,
    model_service_model_pool_route_response_json_with_context,
    model_service_model_pool_status_response_json_with_metrics,
};
use crate::Args;
use crate::model_service::json::{json_bool_field, json_string_field, json_usize_field};
#[cfg(test)]
use config::parse_port;
use config::{WorkerSpec, normalize_base_url, worker_specs};
use model_pool_advice_core::{
    CAPACITY_POLICY as MODEL_POOL_CAPACITY_POLICY,
    HELPER_TARGET_WORKERS as MODEL_POOL_HELPER_TARGET_WORKERS,
    MAX_QUALITY_12B_WORKERS as MODEL_POOL_MAX_QUALITY_12B_WORKERS,
    POLICY as MODEL_POOL_ADVICE_POLICY, RECOMMENDED_LAUNCH_ROLES,
};

const MODEL_POOL_CONNECT_TIMEOUT: Duration = Duration::from_millis(120);
const MODEL_POOL_METADATA_TIMEOUT: Duration = Duration::from_millis(600);
const MODEL_POOL_CALL_DEFAULT_TIMEOUT: Duration = Duration::from_secs(300);
const MODEL_POOL_ADVICE_SOURCE: &str = "model-pool-advice-core";
const MODEL_POOL_TARGET_HOST: &str = "apple_silicon";
const MODEL_POOL_OPERATOR_CHECKS: &str =
    "Activity Monitor GPU History and Memory Pressure must stay healthy before adding workers";

pub(super) fn handle_model_pool_manifest(
    args: &Args,
    stream: &mut TcpStream,
    request_id: usize,
) -> std::io::Result<()> {
    let specs = worker_specs(args)?;
    let body = model_pool_manifest_response_json(request_id, &specs);
    write_http_json(stream, 200, "OK", &body)
}

pub(super) fn handle_model_pool_status(
    args: &Args,
    stream: &mut TcpStream,
    request_id: usize,
) -> std::io::Result<()> {
    let workers = model_pool_workers(args)?;
    let metrics = metrics::snapshot();
    let body = model_service_model_pool_status_response_json_with_metrics(
        request_id,
        &workers,
        Some(&metrics),
    );
    write_http_json(stream, 200, "OK", &body)
}

pub(super) fn handle_model_pool_route(
    args: &Args,
    stream: &mut TcpStream,
    request_id: usize,
    request: ModelServiceModelPoolRouteRequest,
) -> std::io::Result<()> {
    let workers = model_pool_workers(args)?;
    let (route_allowed, selected_role) = model_pool_route_metrics_result(
        &request.task_kind,
        request.max_tokens,
        request.prompt.as_deref(),
        request.completed_roles.as_deref(),
        &workers,
    );
    metrics::record_route_result(selected_role.as_deref(), route_allowed);
    let metrics = metrics::snapshot();
    let body = model_service_model_pool_route_response_json_with_context(
        request_id,
        &request.task_kind,
        request.max_tokens,
        request.prompt.as_deref(),
        &workers,
        request.completed_roles.as_deref(),
        Some(&metrics),
    );
    write_http_json(stream, 200, "OK", &body)
}

pub(super) fn handle_model_pool_call(
    args: &Args,
    stream: &mut TcpStream,
    request_id: usize,
    request: ModelServiceModelPoolCallRequest,
) -> std::io::Result<()> {
    let workers = model_pool_workers(args)?;
    let quality_gate = model_pool_quality_gate(&workers);
    if !quality_gate.launch_allowed {
        metrics::record_route_result(None, false);
        let metrics = metrics::snapshot();
        let reason = model_pool_launch_block_reason(&quality_gate);
        let body = model_service_model_pool_call_blocked_response_json_with_metrics(
            request_id,
            &request.task_kind,
            &reason,
            &workers,
            Some(&metrics),
        );
        return write_http_json(stream, 409, "Conflict", &body);
    }
    let route_metrics = metrics::snapshot();
    let (candidates, routing_weights) = model_pool_route_candidates_for_context(
        &request.task_kind,
        request.max_tokens,
        Some(&request.prompt),
        &workers,
        Some(&route_metrics),
    );
    if !routing_weights.resource_precheck.allow_dispatch {
        metrics::record_route_result(None, false);
        let metrics = metrics::snapshot();
        let reason = format!(
            "resource_precheck_blocked:{}",
            routing_weights.resource_precheck.reason
        );
        let body = model_service_model_pool_call_blocked_response_json_with_metrics(
            request_id,
            &request.task_kind,
            &reason,
            &workers,
            Some(&metrics),
        );
        return write_http_json(stream, 409, "Conflict", &body);
    }
    let selected = candidates.iter().find_map(|role| {
        workers
            .iter()
            .find(|worker| worker.role == *role && worker.ready())
    });
    let Some(selected) = selected else {
        metrics::record_route_result(None, false);
        let metrics = metrics::snapshot();
        let body = model_service_model_pool_call_blocked_response_json_with_metrics(
            request_id,
            &request.task_kind,
            "no_ready_candidate",
            &workers,
            Some(&metrics),
        );
        return write_http_json(stream, 409, "Conflict", &body);
    };
    let dependency_precheck =
        model_pool_dependency_precheck(&selected.role, request.completed_roles.as_deref());
    if !dependency_precheck.allow_dispatch {
        metrics::record_route_result(None, false);
        let metrics = metrics::snapshot();
        let reason = format!("dependency_precheck_blocked:{}", dependency_precheck.reason);
        let body = model_service_model_pool_call_blocked_response_json_with_metrics_and_dependency(
            request_id,
            &request.task_kind,
            &reason,
            &workers,
            Some(&metrics),
            Some(&dependency_precheck),
        );
        return write_http_json(stream, 409, "Conflict", &body);
    }

    metrics::record_route_result(Some(&selected.role), true);
    let token_budget = model_pool_max_tokens_decision(selected, request.max_tokens);
    println!(
        "model_pool_call task_kind={} selected_role={} configured_max_tokens={} effective_max_tokens={} max_tokens_clamped={} max_tokens_clamp_reason={}",
        request.task_kind,
        selected.role,
        option_usize_log_value(token_budget.configured_max_tokens),
        token_budget.effective_max_tokens,
        token_budget.max_tokens_clamped,
        token_budget.max_tokens_clamp_reason
    );
    let call_metrics = metrics::begin_worker_call(&selected.role);
    let call_started = Instant::now();
    match call_model_pool_worker(
        selected,
        &request.prompt,
        token_budget.effective_max_tokens,
        args.runtime_timeout_ms
            .map(Duration::from_millis)
            .unwrap_or(MODEL_POOL_CALL_DEFAULT_TIMEOUT),
    ) {
        Ok(answer) => {
            let execution = ModelPoolCallExecutionView::from_answer(
                elapsed_millis_u64(call_started.elapsed()),
                &answer,
            );
            call_metrics.finish(true);
            let metrics = metrics::snapshot();
            let body = model_service_model_pool_call_response_json_with_metrics(
                request_id,
                &request.task_kind,
                selected,
                &token_budget,
                true,
                &answer,
                &execution,
                Some(&metrics),
            );
            write_http_json(stream, 200, "OK", &body)
        }
        Err(error) => {
            call_metrics.finish(false);
            let body = service_error_json(&format!("model pool call failed: {error}"));
            write_http_json(stream, 502, "Bad Gateway", &body)
        }
    }
}

fn model_pool_route_metrics_result(
    task_kind: &str,
    configured_max_tokens: Option<usize>,
    prompt: Option<&str>,
    completed_roles: Option<&[String]>,
    workers: &[ModelPoolWorkerView],
) -> (bool, Option<String>) {
    let quality_gate = model_pool_quality_gate(workers);
    let metrics = metrics::snapshot();
    let (candidates, routing_weights) = model_pool_route_candidates_for_context(
        task_kind,
        configured_max_tokens,
        prompt,
        workers,
        Some(&metrics),
    );
    let selected_role = candidates.iter().find_map(|role| {
        workers
            .iter()
            .find(|worker| worker.role == *role && worker.ready())
            .map(|worker| worker.role.clone())
    });
    let dependency_precheck = model_pool_dependency_precheck(
        selected_role.as_deref().unwrap_or(task_kind),
        completed_roles,
    );
    let route_allowed = quality_gate.launch_allowed
        && routing_weights.resource_precheck.allow_dispatch
        && dependency_precheck.allow_dispatch
        && selected_role.is_some();
    (
        route_allowed,
        route_allowed.then_some(selected_role).flatten(),
    )
}

fn option_usize_log_value(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_owned())
}

fn elapsed_millis_u64(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}

fn model_pool_manifest_response_json(request_id: usize, specs: &[WorkerSpec]) -> String {
    let quality_required_context_tokens = specs
        .iter()
        .find(|worker| worker.role == "quality")
        .map(|worker| worker.default_context_tokens)
        .unwrap_or(262_144);
    let helper_roles = specs
        .iter()
        .filter(|worker| worker.role != "quality")
        .map(|worker| worker.role.clone())
        .collect::<Vec<_>>();
    let recommended_launch_order = recommended_launch_order(specs);
    let helper_context_tokens_total = specs
        .iter()
        .filter(|worker| worker.role != "quality")
        .map(|worker| worker.default_context_tokens)
        .sum::<usize>();
    let helper_default_max_tokens_total = specs
        .iter()
        .filter(|worker| worker.role != "quality")
        .map(|worker| worker.default_max_tokens)
        .sum::<usize>();
    let advice = model_pool_manifest_advice(specs, &helper_roles, &recommended_launch_order);
    let advice_json = model_pool_manifest_advice_json(&advice);
    let worker_shape_json = model_pool_manifest_worker_shape_json(&advice);

    format!(
        "{{\"ok\":true,\"request_id\":{},\"schema_version\":1,\"contract_version\":\"gemma-chain.v1\",\"read_only\":true,\"sends_prompt\":false,\"launches_process\":false,\"manifest_kind\":\"rust-norion.model-pool\",\"capacity_policy\":{{\"policy\":{},\"target_host\":{},\"avoid_extra_12b\":true,\"max_quality_12b_workers\":{},\"quality_role\":\"quality\",\"quality_required_context_tokens\":{},\"helper_roles\":{},\"helper_context_tokens_total\":{},\"helper_default_max_tokens_total\":{},\"helper_model_size_policy\":\"small_or_low_quant_only\",\"large_helper_model_guard\":\"AllowLargePoolWorkerModels is only for one-off manual experiments; do not default helpers to another 12B on shared Apple memory\",\"guard_validation_command\":\".\\\\tools\\\\smartsteam-forge\\\\test-remote-model-pool-guards.cmd\",\"recommended_launch_order\":{},\"expansion_gate\":\"quality worker must be reachable, prompt-ready, context>={}, and Metal/GPU accelerated before helper expansion\",\"next_step_when_quality_ready\":\"{}\"}},\"advice\":{},\"decision_source\":{},\"safe_to_enable_pool_workers\":{},\"next_step\":{},\"reason\":{},\"extra_quality_12b_detected\":{},\"quality_worker_count\":{},\"helper_worker_count\":{},\"helper_target_worker_count\":{},\"helper_roles\":{},\"capacity_recommendation\":{},\"worker_shape\":{},\"workers\":{}}}",
        request_id,
        service_json_string(MODEL_POOL_CAPACITY_POLICY),
        service_json_string(MODEL_POOL_TARGET_HOST),
        MODEL_POOL_MAX_QUALITY_12B_WORKERS,
        quality_required_context_tokens,
        service_json_string_array(&helper_roles),
        helper_context_tokens_total,
        helper_default_max_tokens_total,
        service_json_string_array(&recommended_launch_order),
        quality_required_context_tokens,
        next_step_when_quality_ready(&helper_roles),
        advice_json,
        service_json_string(MODEL_POOL_ADVICE_SOURCE),
        advice.safe_to_enable_pool_workers,
        service_json_string(advice.next_step),
        service_json_string(advice.reason),
        advice.extra_quality_12b_detected,
        advice.quality_worker_count,
        advice.helper_worker_count,
        advice.helper_target_worker_count,
        service_json_string_array(advice.helper_roles),
        service_json_string(advice.next_step),
        worker_shape_json,
        model_pool_manifest_workers_json(specs)
    )
}

#[derive(Debug, Clone, Copy)]
struct ModelPoolManifestAdvice<'a> {
    safe_to_enable_pool_workers: bool,
    next_step: &'static str,
    reason: &'static str,
    kind: &'static str,
    extra_quality_12b_detected: bool,
    quality_worker_count: usize,
    helper_worker_count: usize,
    helper_target_worker_count: usize,
    helper_roles: &'a [String],
    recommended_launch_order: &'a [String],
}

fn model_pool_manifest_advice<'a>(
    specs: &[WorkerSpec],
    helper_roles: &'a [String],
    recommended_launch_order: &'a [String],
) -> ModelPoolManifestAdvice<'a> {
    let quality_worker_count = specs
        .iter()
        .filter(|worker| worker.role == "quality")
        .count();
    let helper_worker_count = specs
        .iter()
        .filter(|worker| worker.role != "quality")
        .count();
    let extra_quality_12b_detected = quality_worker_count > MODEL_POOL_MAX_QUALITY_12B_WORKERS;
    let safe_to_enable_pool_workers = !extra_quality_12b_detected;
    let next_step = if extra_quality_12b_detected {
        "stop_extra_quality_12b_workers_keep_one_quality_plus_helpers"
    } else {
        manifest_helper_next_step(helper_roles)
    };
    let reason = if extra_quality_12b_detected {
        "extra_quality_12b_wastes_shared_apple_memory"
    } else {
        manifest_helper_reason(helper_roles)
    };
    ModelPoolManifestAdvice {
        safe_to_enable_pool_workers,
        next_step,
        reason,
        kind: if safe_to_enable_pool_workers {
            "busy"
        } else {
            "error"
        },
        extra_quality_12b_detected,
        quality_worker_count,
        helper_worker_count,
        helper_target_worker_count: MODEL_POOL_HELPER_TARGET_WORKERS,
        helper_roles,
        recommended_launch_order,
    }
}

fn model_pool_manifest_advice_json(advice: &ModelPoolManifestAdvice<'_>) -> String {
    format!(
        "{{\"decision_source\":{},\"policy\":{},\"safe_to_enable_pool_workers\":{},\"next_step\":{},\"reason\":{},\"kind\":{},\"extra_quality_12b_detected\":{},\"avoid_extra_12b\":true,\"max_quality_12b_workers\":{},\"quality_worker_count\":{},\"helper_worker_count\":{},\"helper_target_worker_count\":{},\"helper_roles\":{},\"recommended_launch_order\":{},\"worker_shape\":{},\"operator_checks\":{}}}",
        service_json_string(MODEL_POOL_ADVICE_SOURCE),
        service_json_string(MODEL_POOL_ADVICE_POLICY),
        advice.safe_to_enable_pool_workers,
        service_json_string(advice.next_step),
        service_json_string(advice.reason),
        service_json_string(advice.kind),
        advice.extra_quality_12b_detected,
        MODEL_POOL_MAX_QUALITY_12B_WORKERS,
        advice.quality_worker_count,
        advice.helper_worker_count,
        advice.helper_target_worker_count,
        service_json_string_array(advice.helper_roles),
        service_json_string_array(advice.recommended_launch_order),
        model_pool_manifest_worker_shape_json(advice),
        service_json_string(MODEL_POOL_OPERATOR_CHECKS)
    )
}

fn model_pool_manifest_worker_shape_json(advice: &ModelPoolManifestAdvice<'_>) -> String {
    format!(
        "{{\"quality\":{},\"helpers_visible\":{},\"helper_target\":{}}}",
        advice.quality_worker_count, advice.helper_worker_count, advice.helper_target_worker_count
    )
}

fn recommended_launch_order(_specs: &[WorkerSpec]) -> Vec<String> {
    RECOMMENDED_LAUNCH_ROLES
        .iter()
        .copied()
        .map(str::to_owned)
        .collect()
}

fn next_step_when_quality_ready(helper_roles: &[String]) -> &str {
    if helper_roles.is_empty() {
        "hold_quality_only_until_helper_manifest_is_configured"
    } else {
        manifest_helper_next_step(helper_roles)
    }
}

fn manifest_helper_next_step(helper_roles: &[String]) -> &'static str {
    if all_helper_roles_visible(helper_roles) {
        "run_short_pool_smoke_then_use_evolution_loop_helper_stage_calls"
    } else if has_helper_role(helper_roles, "summary")
        && (has_helper_role(helper_roles, "review")
            || has_helper_role(helper_roles, "index")
            || has_helper_role(helper_roles, "test-gate"))
    {
        "add_remaining_helper_roles_one_at_a_time"
    } else if has_helper_role(helper_roles, "summary") {
        "add_review_or_index_after_short_smoke"
    } else if helper_roles.is_empty() {
        "hold_quality_only_until_helper_manifest_is_configured"
    } else {
        "add_first_manifest_helper_worker"
    }
}

fn manifest_helper_reason(helper_roles: &[String]) -> &'static str {
    if all_helper_roles_visible(helper_roles) {
        "full_helper_pool_visible"
    } else if has_helper_role(helper_roles, "summary") && helper_roles.len() > 1 {
        "partial_helper_pool_visible"
    } else if has_helper_role(helper_roles, "summary") {
        "summary_worker_visible"
    } else if helper_roles.is_empty() {
        "quality_chain_ready_no_helper_manifest"
    } else {
        "manifest_helper_visible_without_summary"
    }
}

fn all_helper_roles_visible(helper_roles: &[String]) -> bool {
    has_helper_role(helper_roles, "summary")
        && has_helper_role(helper_roles, "review")
        && has_helper_role(helper_roles, "index")
        && has_helper_role(helper_roles, "test-gate")
}

fn has_helper_role(helper_roles: &[String], role: &str) -> bool {
    helper_roles.iter().any(|helper_role| helper_role == role)
}

fn model_pool_manifest_workers_json(specs: &[WorkerSpec]) -> String {
    let items = specs
        .iter()
        .map(model_pool_manifest_worker_json)
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

fn model_pool_manifest_worker_json(spec: &WorkerSpec) -> String {
    format!(
        "{{\"role\":{},\"port\":{},\"base_url\":{},\"enabled_by_default\":{},\"model_class\":{},\"suggested_quant\":{},\"default_context_tokens\":{},\"default_max_tokens\":{},\"low_priority\":{},\"runtime_backend\":{},\"runtime_device\":{},\"runtime_accelerator\":{},\"gpu_layers\":{}}}",
        service_json_string(&spec.role),
        spec.port,
        service_json_string(&spec.base_url),
        spec.enabled_by_default,
        service_json_string(&spec.model_class),
        service_json_string(&spec.suggested_quant),
        spec.default_context_tokens,
        spec.default_max_tokens,
        spec.low_priority,
        option_str_service_json(spec.runtime_backend.as_deref()),
        option_str_service_json(spec.runtime_device.as_deref()),
        option_str_service_json(spec.runtime_accelerator.as_deref()),
        option_usize_service_json(spec.gpu_layers)
    )
}

fn model_pool_workers(args: &Args) -> std::io::Result<Vec<ModelPoolWorkerView>> {
    Ok(worker_specs(args)?
        .into_iter()
        .map(|spec| {
            let metadata = probe_model_metadata(&spec.base_url);
            ModelPoolWorkerView {
                role: spec.role,
                port: spec.port,
                base_url: spec.base_url,
                enabled_by_default: spec.enabled_by_default,
                model_class: spec.model_class,
                suggested_quant: spec.suggested_quant,
                default_context_tokens: spec.default_context_tokens,
                default_max_tokens: spec.default_max_tokens,
                low_priority: spec.low_priority,
                reachable: metadata.reachable,
                model: metadata.model,
                context_window: metadata.context_window,
                runtime_backend: metadata.runtime_backend.or(spec.runtime_backend),
                runtime_device: metadata.runtime_device.or(spec.runtime_device),
                runtime_accelerator: metadata.runtime_accelerator.or(spec.runtime_accelerator),
                gpu_layers: metadata.gpu_layers.or(spec.gpu_layers),
                error: metadata.error,
            }
        })
        .collect())
}

#[derive(Default)]
struct WorkerMetadata {
    reachable: bool,
    model: Option<String>,
    context_window: Option<usize>,
    runtime_backend: Option<String>,
    runtime_device: Option<String>,
    runtime_accelerator: Option<String>,
    gpu_layers: Option<usize>,
    error: Option<String>,
}

fn probe_model_metadata(base_url: &str) -> WorkerMetadata {
    let reachable = match tcp_reachable(base_url) {
        Ok(reachable) => reachable,
        Err(error) => {
            return WorkerMetadata {
                reachable: false,
                model: None,
                context_window: None,
                runtime_backend: None,
                runtime_device: None,
                runtime_accelerator: None,
                gpu_layers: None,
                error: Some(error),
            };
        }
    };
    if !reachable {
        return WorkerMetadata {
            reachable: false,
            model: None,
            context_window: None,
            runtime_backend: None,
            runtime_device: None,
            runtime_accelerator: None,
            gpu_layers: None,
            error: Some("tcp port unreachable".to_owned()),
        };
    }

    match get_http_json(base_url, "/v1/models") {
        Ok(body) => WorkerMetadata {
            reachable: true,
            model: json_string_field(&body, "id")
                .or_else(|| json_string_field(&body, "model"))
                .or_else(|| json_string_field(&body, "name")),
            context_window: json_usize_field(&body, "n_ctx")
                .or_else(|| json_usize_field(&body, "context_window"))
                .or_else(|| json_usize_field(&body, "default_context_tokens"))
                .or_else(|| json_usize_field(&body, "runtime_context_window")),
            runtime_backend: json_string_field(&body, "runtime_backend")
                .or_else(|| json_string_field(&body, "backend"))
                .or_else(|| json_string_field(&body, "engine")),
            runtime_device: json_string_field(&body, "runtime_device")
                .or_else(|| json_string_field(&body, "device"))
                .or_else(|| json_string_field(&body, "device_profile"))
                .or_else(|| json_string_field(&body, "execution_device")),
            runtime_accelerator: runtime_accelerator_from_metadata(&body),
            gpu_layers: json_usize_field(&body, "gpu_layers")
                .or_else(|| json_usize_field(&body, "n_gpu_layers"))
                .or_else(|| json_usize_field(&body, "offloaded_gpu_layers")),
            error: None,
        },
        Err(error) => WorkerMetadata {
            reachable,
            model: None,
            context_window: None,
            runtime_backend: None,
            runtime_device: None,
            runtime_accelerator: None,
            gpu_layers: None,
            error: Some(error),
        },
    }
}

fn runtime_accelerator_from_metadata(body: &str) -> Option<String> {
    json_string_field(body, "runtime_accelerator")
        .or_else(|| json_string_field(body, "accelerator"))
        .or_else(|| json_string_field(body, "device_accelerator"))
        .or_else(|| {
            json_bool_field(body, "metal")
                .filter(|enabled| *enabled)
                .map(|_| "metal".to_owned())
        })
        .or_else(|| {
            json_bool_field(body, "gpu")
                .filter(|enabled| *enabled)
                .map(|_| "gpu".to_owned())
        })
}

fn tcp_reachable(base_url: &str) -> Result<bool, String> {
    let endpoint = parse_http_endpoint(base_url)?;
    Ok(TcpStream::connect_timeout(&endpoint.address, MODEL_POOL_CONNECT_TIMEOUT).is_ok())
}

fn get_http_json(base_url: &str, path: &str) -> Result<String, String> {
    let endpoint = parse_http_endpoint(base_url)?;
    let mut stream = TcpStream::connect_timeout(&endpoint.address, MODEL_POOL_CONNECT_TIMEOUT)
        .map_err(|error| {
            format!(
                "connect model worker {} failed: {error}",
                endpoint.authority
            )
        })?;
    stream
        .set_read_timeout(Some(MODEL_POOL_METADATA_TIMEOUT))
        .map_err(|error| format!("set model worker read timeout failed: {error}"))?;
    stream
        .set_write_timeout(Some(MODEL_POOL_METADATA_TIMEOUT))
        .map_err(|error| format!("set model worker write timeout failed: {error}"))?;
    let request_path = endpoint.request_path(path);
    let request = format!(
        "GET {request_path} HTTP/1.1\r\nhost: {}\r\naccept: application/json\r\nconnection: close\r\n\r\n",
        endpoint.authority
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|error| format!("write model worker metadata request failed: {error}"))?;
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .map_err(|error| format!("read model worker metadata response failed: {error}"))?;
    parse_http_json_response(&response)
}

fn call_model_pool_worker(
    worker: &ModelPoolWorkerView,
    prompt: &str,
    max_tokens: usize,
    timeout: Duration,
) -> Result<String, String> {
    let body = format!(
        "{{\"model\":\"smartsteam-pool-worker\",\"messages\":[{{\"role\":\"user\",\"content\":{}}}],\"stream\":false,\"max_tokens\":{}}}",
        service_json_string(prompt),
        max_tokens.max(1)
    );
    let response = post_http_json(&worker.base_url, "/v1/chat/completions", &body, timeout)?;
    json_string_field(&response, "content")
        .or_else(|| json_string_field(&response, "text"))
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            json_string_field(&response, "message")
                .map(|message| format!("model worker returned error: {message}"))
                .unwrap_or_else(|| "model worker response missing answer content".to_owned())
        })
}

fn post_http_json(
    base_url: &str,
    path: &str,
    body: &str,
    timeout: Duration,
) -> Result<String, String> {
    let endpoint = parse_http_endpoint(base_url)?;
    let mut stream = TcpStream::connect_timeout(&endpoint.address, MODEL_POOL_CONNECT_TIMEOUT)
        .map_err(|error| {
            format!(
                "connect model worker {} failed: {error}",
                endpoint.authority
            )
        })?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|error| format!("set model worker read timeout failed: {error}"))?;
    stream
        .set_write_timeout(Some(MODEL_POOL_METADATA_TIMEOUT))
        .map_err(|error| format!("set model worker write timeout failed: {error}"))?;
    let request_path = endpoint.request_path(path);
    let request = format!(
        "POST {request_path} HTTP/1.1\r\nhost: {}\r\ncontent-type: application/json; charset=utf-8\r\naccept: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        endpoint.authority,
        body.len()
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|error| format!("write model worker call request failed: {error}"))?;
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .map_err(|error| format!("read model worker call response failed: {error}"))?;
    parse_http_json_response(&response)
}

struct HttpEndpoint {
    authority: String,
    address: SocketAddr,
    base_path: String,
}

impl HttpEndpoint {
    fn request_path(&self, path: &str) -> String {
        let path = if path.starts_with('/') {
            path.to_owned()
        } else {
            format!("/{path}")
        };
        let base_path = self.base_path.trim_end_matches('/');
        if base_path.is_empty() {
            path
        } else if base_path == "/v1" && path.starts_with("/v1/") {
            path
        } else {
            format!("{base_path}{path}")
        }
    }
}

fn parse_http_endpoint(base_url: &str) -> Result<HttpEndpoint, String> {
    let normalized = normalize_base_url(base_url);
    let without_scheme = normalized
        .strip_prefix("http://")
        .ok_or_else(|| "model pool workers must use http:// endpoints".to_owned())?;
    let (authority, base_path) = without_scheme
        .split_once('/')
        .map(|(authority, path)| (authority.to_owned(), format!("/{path}")))
        .unwrap_or_else(|| (without_scheme.to_owned(), String::new()));
    let address = authority
        .to_socket_addrs()
        .map_err(|error| format!("resolve model worker {authority} failed: {error}"))?
        .next()
        .ok_or_else(|| format!("resolve model worker {authority} returned no address"))?;
    Ok(HttpEndpoint {
        authority,
        address,
        base_path,
    })
}

fn parse_http_json_response(response: &[u8]) -> Result<String, String> {
    let header_end = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| "model worker response missing HTTP headers".to_owned())?;
    let headers = String::from_utf8_lossy(&response[..header_end]);
    let status = headers
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|status| status.parse::<u16>().ok())
        .unwrap_or(0);
    let body = response.get(header_end + 4..).unwrap_or_default();
    if !(200..300).contains(&status) {
        return Err(format!("model worker returned HTTP {status}"));
    }
    std::str::from_utf8(body)
        .map(|body| body.to_owned())
        .map_err(|error| format!("model worker body was not UTF-8: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::net::TcpListener;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    fn ready_worker(base_url: String) -> ModelPoolWorkerView {
        ModelPoolWorkerView {
            role: "review".to_owned(),
            port: 0,
            base_url,
            enabled_by_default: true,
            model_class: "test".to_owned(),
            suggested_quant: "Q4".to_owned(),
            default_context_tokens: 8192,
            default_max_tokens: 1536,
            low_priority: true,
            reachable: true,
            model: Some("fake-worker".to_owned()),
            context_window: Some(8192),
            runtime_backend: None,
            runtime_device: None,
            runtime_accelerator: None,
            gpu_layers: None,
            error: None,
        }
    }

    fn http_json_response(status: &str, body: &str) -> String {
        format!(
            "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        )
    }

    fn spawn_fake_model_worker(
        context_window: usize,
        chat_seen: Arc<AtomicBool>,
    ) -> (String, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            listener.set_nonblocking(true).unwrap();
            let mut metadata_seen = false;
            for _ in 0..150 {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let Some(request) = read_optional_http_request(&mut stream) else {
                            continue;
                        };
                        if request.starts_with("GET /v1/models HTTP/1.1") {
                            metadata_seen = true;
                            let body = format!(
                                "{{\"id\":\"fake-worker\",\"n_ctx\":{context_window},\"backend\":\"llama.cpp\",\"device\":\"metal\",\"metal\":true,\"n_gpu_layers\":99}}"
                            );
                            stream
                                .write_all(http_json_response("200 OK", &body).as_bytes())
                                .unwrap();
                            continue;
                        }
                        if request.starts_with("POST /v1/chat/completions HTTP/1.1") {
                            chat_seen.store(true, Ordering::SeqCst);
                            let body = "{\"content\":\"unexpected chat call\"}";
                            stream
                                .write_all(http_json_response("200 OK", body).as_bytes())
                                .unwrap();
                            return;
                        }
                        panic!("fake model worker received unexpected request: {request}");
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => panic!("fake model worker accept failed: {error}"),
                }
            }
            assert!(
                metadata_seen,
                "fake model worker did not receive metadata probe"
            );
        });
        (format!("http://{address}"), handle)
    }

    fn model_pool_manifest_path(
        quality_base_url: &str,
        review_base_url: &str,
    ) -> std::path::PathBuf {
        let thread_id = format!("{:?}", std::thread::current().id());
        let path = std::env::temp_dir().join(format!(
            "smartsteam-model-pool-call-block-{}-{thread_id}.json",
            std::process::id(),
        ));
        let manifest = format!(
            r#"{{
                "workers": [
                    {{"role":"quality","base_url":"{quality_base_url}","default_context_tokens":262144,"default_max_tokens":262144,"low_priority":false}},
                    {{"role":"review","base_url":"{review_base_url}","default_context_tokens":8192,"default_max_tokens":1536,"low_priority":true}}
                ]
            }}"#
        );
        fs::write(&path, manifest).unwrap();
        path
    }

    fn model_pool_manifest_path_with_runtime(
        quality_base_url: &str,
        review_base_url: &str,
    ) -> std::path::PathBuf {
        let thread_id = format!("{:?}", std::thread::current().id());
        let path = std::env::temp_dir().join(format!(
            "smartsteam-model-pool-runtime-{}-{thread_id}.json",
            std::process::id(),
        ));
        let manifest = format!(
            r#"{{
                "workers": [
                    {{"role":"quality","base_url":"{quality_base_url}","default_context_tokens":262144,"default_max_tokens":262144,"low_priority":false,"runtime_backend":"llama.cpp","runtime_device":"metal","runtime_accelerator":"metal","gpu_layers":999}},
                    {{"role":"review","base_url":"{review_base_url}","default_context_tokens":8192,"default_max_tokens":1536,"low_priority":true,"runtime_backend":"llama.cpp","runtime_device":"metal","runtime_accelerator":"metal","gpu_layers":80}}
                ]
            }}"#
        );
        fs::write(&path, manifest).unwrap();
        path
    }

    fn duplicate_quality_model_pool_manifest_path() -> std::path::PathBuf {
        let thread_id = format!("{:?}", std::thread::current().id());
        let path = std::env::temp_dir().join(format!(
            "smartsteam-model-pool-duplicate-quality-{}-{thread_id}.json",
            std::process::id(),
        ));
        let manifest = r#"{
            "workers": [
                {"role":"quality","base_url":"http://127.0.0.1:8686","default_context_tokens":262144,"default_max_tokens":262144,"low_priority":false},
                {"role":"quality","base_url":"http://127.0.0.1:9696","default_context_tokens":262144,"default_max_tokens":262144,"low_priority":false},
                {"role":"summary","base_url":"http://127.0.0.1:8687","default_context_tokens":8192,"default_max_tokens":768,"low_priority":true}
            ]
        }"#;
        fs::write(&path, manifest).unwrap();
        path
    }

    fn read_http_response(stream: &mut TcpStream) -> String {
        let mut response = Vec::new();
        stream.read_to_end(&mut response).unwrap();
        String::from_utf8(response).unwrap()
    }

    fn quality_worker_with_context(context_window: usize) -> ModelPoolWorkerView {
        ModelPoolWorkerView {
            role: "quality".to_owned(),
            port: 0,
            base_url: "http://127.0.0.1:8686".to_owned(),
            enabled_by_default: true,
            model_class: "Gemma 12B".to_owned(),
            suggested_quant: "Q8".to_owned(),
            default_context_tokens: 262_144,
            default_max_tokens: 262_144,
            low_priority: false,
            reachable: true,
            model: Some("gemma".to_owned()),
            context_window: Some(context_window),
            runtime_backend: None,
            runtime_device: None,
            runtime_accelerator: None,
            gpu_layers: None,
            error: None,
        }
    }

    fn read_single_http_request(stream: &mut TcpStream) -> String {
        read_http_request(stream, true).expect("request should be present")
    }

    fn read_optional_http_request(stream: &mut TcpStream) -> Option<String> {
        let _ = stream.set_read_timeout(Some(Duration::from_millis(100)));
        read_http_request(stream, false)
    }

    fn read_http_request(stream: &mut TcpStream, panic_on_empty: bool) -> Option<String> {
        let mut data = Vec::new();
        let mut chunk = [0_u8; 1024];
        loop {
            let read = match stream.read(&mut chunk) {
                Ok(0) if panic_on_empty => {
                    panic!("fake worker connection closed before request body")
                }
                Ok(0) => return None,
                Ok(read) => read,
                Err(error)
                    if !panic_on_empty
                        && matches!(
                            error.kind(),
                            std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                        ) =>
                {
                    return None;
                }
                Err(error) => panic!("fake worker request read failed: {error}"),
            };
            data.extend_from_slice(&chunk[..read]);
            let Some(header_end) = data.windows(4).position(|window| window == b"\r\n\r\n") else {
                continue;
            };
            let headers = String::from_utf8_lossy(&data[..header_end]);
            let content_length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
                .unwrap_or(0);
            if data.len() >= header_end + 4 + content_length {
                return Some(String::from_utf8(data).unwrap());
            }
        }
    }

    #[test]
    fn parse_port_accepts_http_and_https_endpoints() {
        assert_eq!(parse_port("http://127.0.0.1:8686"), Some(8686));
        assert_eq!(parse_port("https://example.local:9443/v1"), Some(9443));
        assert_eq!(parse_port("127.0.0.1:8688"), Some(8688));
    }

    #[test]
    fn request_path_does_not_duplicate_v1_prefix() {
        let endpoint = parse_http_endpoint("http://127.0.0.1:8688/v1").unwrap();

        assert_eq!(
            endpoint.request_path("/v1/chat/completions"),
            "/v1/chat/completions"
        );
        assert_eq!(endpoint.request_path("/models"), "/v1/models");
    }

    #[test]
    fn call_model_pool_worker_posts_openai_compatible_body() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_single_http_request(&mut stream);
            let body = "{\"content\":\"worker answer\"}";
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
            request
        });
        let worker = ready_worker(format!("http://{address}/v1"));

        let answer =
            call_model_pool_worker(&worker, "hello from test", 77, Duration::from_secs(2)).unwrap();
        let request = server.join().unwrap();

        assert_eq!(answer, "worker answer");
        assert!(request.starts_with("POST /v1/chat/completions HTTP/1.1"));
        assert!(request.contains("\"model\":\"smartsteam-pool-worker\""));
        assert!(
            request.contains("\"messages\":[{\"role\":\"user\",\"content\":\"hello from test\"}]")
        );
        assert!(request.contains("\"stream\":false"));
        assert!(request.contains("\"max_tokens\":77"));
    }

    #[test]
    fn route_metrics_block_when_quality_context_is_too_small() {
        let workers = vec![
            quality_worker_with_context(8192),
            ready_worker("http://127.0.0.1:8688".to_owned()),
        ];

        let (allowed, selected_role) =
            model_pool_route_metrics_result("review", None, None, None, &workers);

        assert!(!allowed);
        assert_eq!(selected_role, None);
    }

    #[test]
    fn route_metrics_select_worker_after_quality_context_gate_passes() {
        let workers = vec![
            quality_worker_with_context(262_144),
            ready_worker("http://127.0.0.1:8688".to_owned()),
        ];

        let (allowed, selected_role) =
            model_pool_route_metrics_result("review", None, None, None, &workers);

        assert!(allowed);
        assert_eq!(selected_role.as_deref(), Some("review"));
    }

    #[test]
    fn model_pool_call_blocks_when_quality_context_window_is_too_small() {
        let quality_chat_seen = Arc::new(AtomicBool::new(false));
        let review_chat_seen = Arc::new(AtomicBool::new(false));
        let (quality_base_url, quality_worker) =
            spawn_fake_model_worker(8192, Arc::clone(&quality_chat_seen));
        let (review_base_url, review_worker) =
            spawn_fake_model_worker(8192, Arc::clone(&review_chat_seen));
        let manifest_path = model_pool_manifest_path(&quality_base_url, &review_base_url);
        let args = Args::parse(vec![
            "--model-pool-manifest".to_owned(),
            manifest_path.display().to_string(),
            "--runtime-timeout-ms".to_owned(),
            "1000".to_owned(),
        ]);
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let request = ModelServiceModelPoolCallRequest {
            task_kind: "review".to_owned(),
            prompt: "do not send this prompt".to_owned(),
            max_tokens: Some(1024),
            completed_roles: None,
        };
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            handle_model_pool_call(&args, &mut stream, 77, request).unwrap();
        });
        let mut client = TcpStream::connect(address).unwrap();

        let response = read_http_response(&mut client);

        server.join().unwrap();
        quality_worker.join().unwrap();
        review_worker.join().unwrap();
        let _ = fs::remove_file(manifest_path);
        assert!(response.starts_with("HTTP/1.1 409 Conflict"));
        assert!(response.contains("\"sends_prompt\":false"));
        assert!(response.contains(
            "\"route_block_reason\":\"model_pool_launch_blocked:quality_context_window_too_small\""
        ));
        assert!(response.contains("\"quality_context_tokens\":8192"));
        assert!(response.contains("\"quality_context_required_tokens\":262144"));
        assert!(response.contains("\"quality_context_sufficient\":false"));
        assert!(response.contains("\"runtime_backend\":\"llama.cpp\""));
        assert!(response.contains("\"runtime_device\":\"metal\""));
        assert!(response.contains("\"runtime_accelerator\":\"metal\""));
        assert!(response.contains("\"gpu_layers\":99"));
        assert!(!quality_chat_seen.load(Ordering::SeqCst));
        assert!(!review_chat_seen.load(Ordering::SeqCst));
    }

    #[test]
    fn model_pool_status_uses_manifest_runtime_fallback_when_workers_are_offline() {
        let manifest_path =
            model_pool_manifest_path_with_runtime("http://127.0.0.1:1", "http://127.0.0.1:2");
        let args = Args::parse(vec![
            "--model-pool-manifest".to_owned(),
            manifest_path.display().to_string(),
        ]);

        let workers = model_pool_workers(&args).unwrap();
        let json = model_service_model_pool_status_response_json_with_metrics(91, &workers, None);

        let _ = fs::remove_file(manifest_path);
        assert_eq!(workers.len(), 2);
        assert!(!workers[0].ready());
        assert_eq!(workers[0].runtime_backend.as_deref(), Some("llama.cpp"));
        assert_eq!(workers[0].runtime_device.as_deref(), Some("metal"));
        assert_eq!(workers[0].runtime_accelerator.as_deref(), Some("metal"));
        assert_eq!(workers[0].gpu_layers, Some(999));
        assert!(json.contains("\"runtime_backend\":\"llama.cpp\""));
        assert!(json.contains("\"runtime_device\":\"metal\""));
        assert!(json.contains("\"runtime_accelerator\":\"metal\""));
        assert!(json.contains("\"gpu_layers\":999"));
        assert!(json.contains("\"launch_allowed\":false"));
        assert!(json.contains("\"healthy_worker_count\":0"));
    }

    #[test]
    fn model_pool_manifest_endpoint_json_is_read_only_and_preserves_runtime_hints() {
        let manifest_path =
            model_pool_manifest_path_with_runtime("http://127.0.0.1:1", "http://127.0.0.1:2");
        let args = Args::parse(vec![
            "--model-pool-manifest".to_owned(),
            manifest_path.display().to_string(),
        ]);
        let specs = worker_specs(&args).unwrap();

        let json = model_pool_manifest_response_json(92, &specs);

        let _ = fs::remove_file(manifest_path);
        assert!(json.contains("\"request_id\":92"));
        assert!(json.contains("\"contract_version\":\"gemma-chain.v1\""));
        assert!(json.contains("\"read_only\":true"));
        assert!(json.contains("\"sends_prompt\":false"));
        assert!(json.contains("\"launches_process\":false"));
        assert!(json.contains("\"manifest_kind\":\"rust-norion.model-pool\""));
        assert!(json.contains("\"policy\":\"one_quality_plus_small_helpers\""));
        assert!(json.contains("\"avoid_extra_12b\":true"));
        assert!(json.contains("\"max_quality_12b_workers\":1"));
        assert!(json.contains("\"quality_required_context_tokens\":262144"));
        assert!(json.contains("\"helper_roles\":[\"review\"]"));
        assert!(json.contains("\"helper_model_size_policy\":\"small_or_low_quant_only\""));
        assert!(json.contains("\"guard_validation_command\""));
        assert!(json.contains(
            "\"recommended_launch_order\":[\"quality\",\"summary\",\"router\",\"review\",\"index\",\"test-gate\"]"
        ));
        assert!(
            json.contains("\"next_step_when_quality_ready\":\"add_first_manifest_helper_worker\"")
        );
        assert!(json.contains("\"advice\":{"));
        assert!(json.contains("\"decision_source\":\"model-pool-advice-core\""));
        assert!(json.contains("\"policy\":\"one_quality_12b_plus_small_helpers\""));
        assert!(json.contains("\"safe_to_enable_pool_workers\":true"));
        assert!(json.contains("\"next_step\":\"add_first_manifest_helper_worker\""));
        assert!(json.contains("\"reason\":\"manifest_helper_visible_without_summary\""));
        assert!(json.contains("\"extra_quality_12b_detected\":false"));
        assert!(json.contains("\"quality_worker_count\":1"));
        assert!(json.contains("\"helper_worker_count\":1"));
        assert!(json.contains("\"helper_target_worker_count\":5"));
        assert!(json.contains("\"capacity_recommendation\":\"add_first_manifest_helper_worker\""));
        assert!(json.contains(
            "\"worker_shape\":{\"quality\":1,\"helpers_visible\":1,\"helper_target\":5}"
        ));
        assert!(json.contains("\"role\":\"quality\""));
        assert!(json.contains("\"runtime_backend\":\"llama.cpp\""));
        assert!(json.contains("\"runtime_device\":\"metal\""));
        assert!(json.contains("\"runtime_accelerator\":\"metal\""));
        assert!(json.contains("\"gpu_layers\":999"));
    }

    #[test]
    fn model_pool_manifest_advice_uses_full_helper_next_step_when_all_helpers_visible() {
        let args = Args::parse(Vec::new());
        let specs = worker_specs(&args).unwrap();

        let json = model_pool_manifest_response_json(94, &specs);

        assert!(json.contains("\"request_id\":94"));
        assert!(json.contains("\"helper_roles\":[\"summary\",\"review\",\"test-gate\",\"index\"]"));
        assert!(json.contains(
            "\"next_step_when_quality_ready\":\"run_short_pool_smoke_then_use_evolution_loop_helper_stage_calls\""
        ));
        assert!(json.contains(
            "\"next_step\":\"run_short_pool_smoke_then_use_evolution_loop_helper_stage_calls\""
        ));
        assert!(json.contains("\"reason\":\"full_helper_pool_visible\""));
        assert!(json.contains(
            "\"capacity_recommendation\":\"run_short_pool_smoke_then_use_evolution_loop_helper_stage_calls\""
        ));
    }

    #[test]
    fn model_pool_manifest_advice_blocks_extra_quality_12b_workers() {
        let manifest_path = duplicate_quality_model_pool_manifest_path();
        let args = Args::parse(vec![
            "--model-pool-manifest".to_owned(),
            manifest_path.display().to_string(),
        ]);
        let specs = worker_specs(&args).unwrap();

        let json = model_pool_manifest_response_json(93, &specs);

        let _ = fs::remove_file(manifest_path);
        assert!(json.contains("\"request_id\":93"));
        assert!(json.contains("\"advice\":{"));
        assert!(json.contains("\"decision_source\":\"model-pool-advice-core\""));
        assert!(json.contains("\"safe_to_enable_pool_workers\":false"));
        assert!(json.contains(
            "\"next_step\":\"stop_extra_quality_12b_workers_keep_one_quality_plus_helpers\""
        ));
        assert!(json.contains("\"reason\":\"extra_quality_12b_wastes_shared_apple_memory\""));
        assert!(json.contains("\"kind\":\"error\""));
        assert!(json.contains("\"extra_quality_12b_detected\":true"));
        assert!(json.contains("\"quality_worker_count\":2"));
        assert!(json.contains("\"helper_worker_count\":1"));
        assert!(json.contains(
            "\"worker_shape\":{\"quality\":2,\"helpers_visible\":1,\"helper_target\":5}"
        ));
        assert!(json.contains(
            "\"capacity_recommendation\":\"stop_extra_quality_12b_workers_keep_one_quality_plus_helpers\""
        ));
    }

    #[test]
    fn model_pool_call_success_includes_execution_metrics() {
        metrics::reset();
        let quality_chat_seen = Arc::new(AtomicBool::new(false));
        let review_chat_seen = Arc::new(AtomicBool::new(false));
        let (quality_base_url, quality_worker) =
            spawn_fake_model_worker(262_144, Arc::clone(&quality_chat_seen));
        let (review_base_url, review_worker) =
            spawn_fake_model_worker(8192, Arc::clone(&review_chat_seen));
        let manifest_path = model_pool_manifest_path(&quality_base_url, &review_base_url);
        let args = Args::parse(vec![
            "--model-pool-manifest".to_owned(),
            manifest_path.display().to_string(),
            "--runtime-timeout-ms".to_owned(),
            "1000".to_owned(),
        ]);
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let request = ModelServiceModelPoolCallRequest {
            task_kind: "review".to_owned(),
            prompt: "send this prompt".to_owned(),
            max_tokens: Some(1024),
            completed_roles: None,
        };
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            handle_model_pool_call(&args, &mut stream, 78, request).unwrap();
        });
        let mut client = TcpStream::connect(address).unwrap();

        let response = read_http_response(&mut client);

        server.join().unwrap();
        quality_worker.join().unwrap();
        review_worker.join().unwrap();
        let _ = fs::remove_file(manifest_path);
        assert!(
            response.starts_with("HTTP/1.1 200 OK"),
            "unexpected response: {response}"
        );
        assert!(response.contains("\"sends_prompt\":true"));
        assert!(response.contains("\"selected_role\":\"review\""));
        assert!(response.contains("\"elapsed_ms\":"));
        assert!(response.contains("\"answer_chars\":20"));
        assert!(response.contains("\"answer_bytes\":20"));
        assert!(response.contains("\"answer_approx_tokens\":5"));
        assert!(response.contains("\"answer\":\"unexpected chat call\""));
        assert!(response.contains("\"success_count\":"));
        assert!(!quality_chat_seen.load(Ordering::SeqCst));
        assert!(review_chat_seen.load(Ordering::SeqCst));
    }
}
