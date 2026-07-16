use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

mod config;
mod metrics;

use super::super::super::http::MODEL_POOL_CALL_CANCEL_MARKER;
use super::super::super::json::{
    option_str_service_json, option_u64_service_json, option_usize_service_json,
    service_json_string, service_json_string_array, write_http_json,
};
use super::super::super::request::{
    ModelServiceModelPoolCallRequest, ModelServiceModelPoolRouteRequest,
};
use super::super::super::response::{
    ModelPoolCallExecutionView, ModelPoolServiceBackpressureView, ModelPoolWorkerView,
    model_pool_agent_route_request_for_candidate, model_pool_dependency_precheck,
    model_pool_launch_block_reason, model_pool_max_tokens_decision, model_pool_quality_gate,
    model_pool_route_candidates_for_context, model_pool_runtime_closed_loop_counters_json,
    model_pool_select_route_worker, model_pool_select_route_worker_with_dependencies,
    model_pool_worker_id, model_service_model_pool_call_blocked_response_json_with_metrics,
    model_service_model_pool_call_blocked_response_json_with_metrics_and_dependency,
    model_service_model_pool_call_response_json_with_metrics,
    model_service_model_pool_route_response_json_with_context_and_backpressure,
    model_service_model_pool_status_response_json_with_metrics,
};
use super::super::state::{MAX_ACTIVE_STREAM_ENGINE_REQUESTS, ModelServiceServerState};
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
use norion_agent::{AgentBudget, AgentModelRouteError, AgentRole, AgentTask};

const MODEL_POOL_CONNECT_TIMEOUT: Duration = Duration::from_millis(120);
const MODEL_POOL_METADATA_TIMEOUT: Duration = Duration::from_millis(600);
const MODEL_POOL_CALL_DEFAULT_TIMEOUT: Duration = Duration::from_secs(300);
const MODEL_POOL_CALL_CANCEL_POLL_INTERVAL: Duration = Duration::from_millis(20);
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
    state: &ModelServiceServerState,
    request: ModelServiceModelPoolRouteRequest,
) -> std::io::Result<()> {
    let workers = model_pool_workers(args)?;
    let service_backpressure = ModelPoolServiceBackpressureView::new(
        state.active_engine_requests(),
        MAX_ACTIVE_STREAM_ENGINE_REQUESTS,
        state.stream_backpressure_rejections(),
    );
    let (route_allowed, selected_role) = model_pool_route_metrics_result(
        &request.task_kind,
        request.max_tokens,
        request.prompt.as_deref(),
        request.completed_roles.as_deref(),
        &workers,
    );
    let route_allowed = route_allowed && service_backpressure.allow_dispatch();
    metrics::record_route_result(
        selected_role.as_deref().filter(|_| route_allowed),
        route_allowed,
    );
    let metrics = metrics::snapshot();
    let body = model_service_model_pool_route_response_json_with_context_and_backpressure(
        request_id,
        &request.task_kind,
        request.max_tokens,
        request.prompt.as_deref(),
        &workers,
        request.completed_roles.as_deref(),
        Some(&metrics),
        Some(&service_backpressure),
    );
    write_http_json(stream, 200, "OK", &body)
}

pub(super) fn handle_model_pool_call(
    args: &Args,
    stream: &mut TcpStream,
    request_id: usize,
    request: ModelServiceModelPoolCallRequest,
) -> std::io::Result<()> {
    let isolation = metrics::WorkerIsolationConfig::from_env();
    handle_model_pool_call_at(
        args,
        stream,
        request_id,
        request,
        &isolation,
        metrics::unix_now(),
        None,
    )
}

fn handle_model_pool_call_at(
    args: &Args,
    stream: &mut TcpStream,
    request_id: usize,
    request: ModelServiceModelPoolCallRequest,
    isolation: &metrics::WorkerIsolationConfig,
    worker_state_now_unix: u64,
    outcome_now_unix: Option<u64>,
) -> std::io::Result<()> {
    let workers = model_pool_workers_at(args, isolation, worker_state_now_unix)?;
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
    let (call_candidates, _) = model_pool_route_candidates_for_context(
        &request.task_kind,
        request.max_tokens,
        Some(&request.prompt),
        &workers,
        Some(&route_metrics),
    );
    let candidate_workers = call_candidates
        .iter()
        .filter_map(|role| {
            model_pool_select_route_worker(
                &workers,
                std::slice::from_ref(role),
                quality_gate.launch_allowed,
                true,
            )
        })
        .collect::<Vec<_>>();
    if candidate_workers.is_empty() {
        metrics::record_route_result(None, false);
        let metrics = metrics::snapshot();
        let body = model_service_model_pool_call_blocked_response_json_with_metrics(
            request_id,
            &request.task_kind,
            "no_route_profile_candidate",
            &workers,
            Some(&metrics),
        );
        return write_http_json(stream, 409, "Conflict", &body);
    }

    let mut eligible_candidates = Vec::new();
    let mut first_blocked = None;
    for (candidate_index, selected) in candidate_workers.iter().copied().enumerate() {
        let dependency_precheck =
            model_pool_dependency_precheck(&selected.role, request.completed_roles.as_deref());
        if !dependency_precheck.allow_dispatch {
            if first_blocked.is_none() {
                first_blocked = Some((
                    format!("dependency_precheck_blocked:{}", dependency_precheck.reason),
                    dependency_precheck,
                ));
            }
            continue;
        }
        let (_, candidate_routing_weights) = model_pool_route_candidates_for_context(
            &selected.role,
            request.max_tokens,
            Some(&request.prompt),
            &workers,
            Some(&route_metrics),
        );
        if !candidate_routing_weights.resource_precheck.allow_dispatch {
            if first_blocked.is_none() {
                first_blocked = Some((
                    format!(
                        "resource_precheck_blocked:{}",
                        candidate_routing_weights.resource_precheck.reason
                    ),
                    dependency_precheck,
                ));
            }
            continue;
        }
        let token_budget = model_pool_max_tokens_decision(selected, request.max_tokens);
        if let Err(error) = model_pool_agent_route_request_for_candidate(
            AgentTask::new(
                format!("model-pool-call-{request_id}-{candidate_index}"),
                AgentRole::Custom(selected.role.clone()),
                format!("model-pool call {}", request.task_kind),
                AgentBudget::new(
                    agent_route_budget_tokens(token_budget.effective_max_tokens),
                    1,
                    1,
                ),
            ),
            &request.prompt,
            &selected.role,
            request.max_tokens,
            &workers,
            request.completed_roles.as_deref(),
            Some(&route_metrics),
            None,
        ) {
            if first_blocked.is_none() {
                first_blocked = Some((
                    agent_route_request_block_reason(&error),
                    dependency_precheck,
                ));
            }
            continue;
        }
        eligible_candidates.push((selected, token_budget));
    }
    if eligible_candidates.is_empty() {
        metrics::record_route_result(None, false);
        let metrics = metrics::snapshot();
        let (reason, dependency_precheck) = first_blocked.unwrap_or_else(|| {
            let selected = candidate_workers[0];
            (
                "no_route_profile_candidate".to_owned(),
                model_pool_dependency_precheck(&selected.role, request.completed_roles.as_deref()),
            )
        });
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

    let call_timeout = args
        .runtime_timeout_ms
        .map(Duration::from_millis)
        .unwrap_or(MODEL_POOL_CALL_DEFAULT_TIMEOUT);
    stream.set_nonblocking(true)?;
    if model_pool_caller_cancelled(stream) {
        return Ok(());
    }
    let pool_started = Instant::now();
    let mut last_failure = None;
    let mut isolated_worker_ids = BTreeSet::new();
    let mut worker_outcome_persisted = false;
    let mut dispatch_attempted = false;
    let attempt_count = eligible_candidates.len();
    for (attempt_index, (selected, token_budget)) in eligible_candidates.iter().enumerate() {
        if model_pool_caller_cancelled(stream) {
            return Ok(());
        }
        let selected = *selected;
        let worker_id = model_pool_worker_id(&selected.base_url);
        if isolated_worker_ids.contains(&worker_id) {
            continue;
        }
        let remaining_timeout = call_timeout.saturating_sub(pool_started.elapsed());
        let remaining_attempts = attempt_count.saturating_sub(attempt_index).max(1) as u32;
        let attempt_timeout = if attempt_count == 1 {
            call_timeout
        } else {
            remaining_timeout / remaining_attempts
        };
        if attempt_timeout.is_zero() {
            break;
        }
        println!(
            "model_pool_call task_kind={} selected_role={} attempt={}/{} configured_max_tokens={} effective_max_tokens={} max_tokens_clamped={} max_tokens_clamp_reason={}",
            request.task_kind,
            selected.role,
            attempt_index + 1,
            attempt_count,
            option_usize_log_value(token_budget.configured_max_tokens),
            token_budget.effective_max_tokens,
            token_budget.max_tokens_clamped,
            token_budget.max_tokens_clamp_reason
        );
        let Some(call_metrics) = metrics::try_begin_worker_call(
            selected,
            isolation,
            outcome_now_unix.unwrap_or_else(metrics::unix_now),
        ) else {
            last_failure.get_or_insert_with(|| {
                (
                    selected,
                    token_budget.clone(),
                    "worker entered failure cooldown before dispatch".to_owned(),
                )
            });
            continue;
        };
        dispatch_attempted = true;
        let call = {
            let mut should_cancel = || model_pool_caller_cancelled(stream);
            call_model_pool_worker(
                selected,
                &request.prompt,
                token_budget.effective_max_tokens,
                request.stream,
                attempt_timeout,
                &mut should_cancel,
            )
        };
        match call {
            Ok(call) => {
                if model_pool_caller_cancelled(stream) {
                    return Ok(());
                }
                stream.set_nonblocking(false)?;
                let execution = ModelPoolCallExecutionView::from_answer(
                    elapsed_millis_u64(pool_started.elapsed()),
                    &call.answer,
                );
                let _ = call_metrics.finish_with_reason_at(
                    true,
                    None,
                    outcome_now_unix.unwrap_or_else(metrics::unix_now),
                );
                metrics::record_route_result(Some(&selected.role), true);
                let metrics = metrics::snapshot();
                let body = model_service_model_pool_call_response_json_with_metrics(
                    request_id,
                    &request.task_kind,
                    selected,
                    &token_budget,
                    true,
                    &call.answer,
                    &execution,
                    &call.streamed_tokens,
                    Some(&metrics),
                );
                return write_http_json(stream, 200, "OK", &body);
            }
            Err(ModelPoolWorkerCallError::Cancelled) => return Ok(()),
            Err(ModelPoolWorkerCallError::Failed(error)) => {
                if model_pool_caller_cancelled(stream) {
                    return Ok(());
                }
                let failure_reason = model_pool_worker_failure_reason(&error);
                worker_outcome_persisted |= call_metrics.finish_with_reason_at(
                    false,
                    failure_reason,
                    outcome_now_unix.unwrap_or_else(metrics::unix_now),
                );
                if failure_reason.is_some() {
                    isolated_worker_ids.insert(worker_id);
                }
                last_failure = Some((selected, token_budget.clone(), error));
            }
        }
    }
    stream.set_nonblocking(false)?;
    let (selected, token_budget, error) = last_failure.unwrap_or_else(|| {
        let (selected, token_budget) = &eligible_candidates[0];
        (
            *selected,
            token_budget.clone(),
            "timeout budget exhausted before model-pool worker call".to_owned(),
        )
    });
    metrics::record_route_result(
        dispatch_attempted.then_some(selected.role.as_str()),
        dispatch_attempted,
    );
    let body = model_pool_call_failure_json(
        request_id,
        &request.task_kind,
        &selected.role,
        token_budget.configured_max_tokens,
        token_budget.effective_max_tokens,
        token_budget.max_tokens_clamped,
        &error,
        dispatch_attempted,
        worker_outcome_persisted,
    );
    write_http_json(stream, 502, "Bad Gateway", &body)
}

fn model_pool_worker_failure_reason(error: &str) -> Option<&'static str> {
    let error = error.to_ascii_lowercase();
    if error.contains("timed out") {
        return Some("timeout");
    }
    if let Some(status) = error
        .split_once("model worker returned http ")
        .and_then(|(_, status)| status.split_whitespace().next())
        .and_then(|status| status.parse::<u16>().ok())
    {
        return match status {
            401 | 403 => Some("worker_access"),
            408 | 504 => Some("timeout"),
            429 => Some("rate_limit"),
            500..=599 => Some("worker_http_5xx"),
            _ => None,
        };
    }
    if error.contains("response missing answer content")
        || error.contains("model worker returned error")
        || error.contains("response missing http headers")
        || error.contains("body was not utf-8")
    {
        return Some("response_shape");
    }
    if error.contains("connect model worker")
        || error.contains("resolve model worker")
        || error.contains("read model worker")
        || error.contains("write model worker")
        || error.contains("set model worker")
    {
        return Some("transport");
    }
    None
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
    let (selected, dependency_precheck) = model_pool_select_route_worker_with_dependencies(
        workers,
        &candidates,
        quality_gate.launch_allowed,
        routing_weights.resource_precheck.allow_dispatch,
        completed_roles,
    );
    let selected_role = selected.map(|worker| worker.role.clone());
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

fn agent_route_budget_tokens(tokens: usize) -> u32 {
    tokens.min(u32::MAX as usize) as u32
}

fn agent_route_request_block_reason(error: &AgentModelRouteError) -> String {
    match error {
        AgentModelRouteError::MissingField(field) => {
            format!("agent_route_request_missing_{field}")
        }
        AgentModelRouteError::RouteNotAllowed => "agent_route_request_route_not_allowed".to_owned(),
    }
}

fn elapsed_millis_u64(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}

fn model_pool_caller_cancelled(stream: &TcpStream) -> bool {
    let mut buffer = [0_u8; 64];
    match stream.peek(&mut buffer) {
        Ok(read) => {
            read >= MODEL_POOL_CALL_CANCEL_MARKER.len()
                && buffer[..MODEL_POOL_CALL_CANCEL_MARKER.len()] == *MODEL_POOL_CALL_CANCEL_MARKER
        }
        Err(error) => !matches!(
            error.kind(),
            std::io::ErrorKind::Interrupted
                | std::io::ErrorKind::TimedOut
                | std::io::ErrorKind::WouldBlock
        ),
    }
}

fn model_pool_call_failure_json(
    request_id: usize,
    task_kind: &str,
    selected_role: &str,
    configured_max_tokens: Option<usize>,
    effective_max_tokens: usize,
    max_tokens_clamped: bool,
    error: &str,
    dispatch_attempted: bool,
    worker_outcome_persisted: bool,
) -> String {
    let message = format!("model pool call failed: {error}");
    let saved_tokens = configured_max_tokens
        .unwrap_or(effective_max_tokens)
        .saturating_sub(effective_max_tokens);
    let runtime_closed_loop_counters =
        model_pool_runtime_closed_loop_counters_json(saved_tokens, max_tokens_clamped, true);
    format!(
        "{{\"ok\":false,\"request_id\":{},\"schema_version\":1,\"contract_version\":\"model-pool.v1\",\"task_kind\":{},\"read_only\":false,\"launches_process\":false,\"sends_prompt\":{},\"endpoint\":\"model-pool-call\",\"selected_role\":{},\"call_state\":\"failed\",\"cancelled\":false,\"timeout\":{},\"partial_result\":false,\"partial_finalized\":true,\"queue_time_ms\":0,\"compute_budget_summary\":{},\"compute_budget_configured_max_tokens\":{},\"compute_budget_effective_max_tokens\":{},\"compute_budget_saved_tokens\":{},\"compute_budget_avoided_tokens\":{},\"compute_budget_max_tokens_clamped\":{},{},\"error\":{},\"retryable\":true,\"dispatch_attempted\":{},\"persistent_writes\":{},\"memory_write_allowed\":false,\"genome_write_allowed\":false,\"self_evolution_write_allowed\":false}}",
        request_id,
        service_json_string(task_kind),
        dispatch_attempted,
        service_json_string(selected_role),
        model_pool_call_error_is_timeout(error),
        service_json_string(&format!(
            "model_pool_call selected_role={} effective_max_tokens={} saved_tokens={} max_tokens_clamped={} failed_before_worker_answer",
            selected_role, effective_max_tokens, saved_tokens, max_tokens_clamped
        )),
        option_usize_service_json(configured_max_tokens),
        effective_max_tokens,
        saved_tokens,
        saved_tokens,
        max_tokens_clamped,
        runtime_closed_loop_counters,
        service_json_string(&message),
        dispatch_attempted,
        worker_outcome_persisted
    )
}

fn model_pool_call_error_is_timeout(error: &str) -> bool {
    let error = error.to_ascii_lowercase();
    error.contains("timeout") || error.contains("timed out")
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
        && has_helper_role(helper_roles, "router")
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
        "{{\"role\":{},\"port\":{},\"base_url\":{},\"enabled_by_default\":{},\"model_class\":{},\"suggested_quant\":{},\"default_context_tokens\":{},\"default_max_tokens\":{},\"low_priority\":{},\"runtime_backend\":{},\"runtime_device\":{},\"runtime_accelerator\":{},\"gpu_layers\":{},\"input_cost_per_1k_micro_usd\":{},\"output_cost_per_1k_micro_usd\":{},\"remaining_budget_micro_usd\":{}}}",
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
        option_usize_service_json(spec.gpu_layers),
        option_u64_service_json(spec.input_cost_per_1k_micro_usd),
        option_u64_service_json(spec.output_cost_per_1k_micro_usd),
        option_u64_service_json(spec.remaining_budget_micro_usd)
    )
}

fn model_pool_workers(args: &Args) -> std::io::Result<Vec<ModelPoolWorkerView>> {
    let isolation = metrics::WorkerIsolationConfig::from_env();
    model_pool_workers_at(args, &isolation, metrics::unix_now())
}

fn model_pool_workers_at(
    args: &Args,
    isolation: &metrics::WorkerIsolationConfig,
    now_unix: u64,
) -> std::io::Result<Vec<ModelPoolWorkerView>> {
    let specs = worker_specs(args)?;
    let quarantines = metrics::worker_quarantines(&specs, isolation, now_unix);
    Ok(specs
        .into_iter()
        .map(|spec| {
            let worker_id = model_pool_worker_id(&spec.base_url);
            let quarantine = quarantines.get(&worker_id).cloned();
            let metadata = if quarantine.is_some() {
                WorkerMetadata::default()
            } else {
                probe_model_metadata(&spec.base_url)
            };
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
                input_cost_per_1k_micro_usd: spec.input_cost_per_1k_micro_usd,
                output_cost_per_1k_micro_usd: spec.output_cost_per_1k_micro_usd,
                remaining_budget_micro_usd: spec.remaining_budget_micro_usd,
                error: metadata.error,
                quarantine,
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct ModelPoolWorkerCall {
    answer: String,
    streamed_tokens: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
enum ModelPoolWorkerCallError {
    Cancelled,
    Failed(String),
}

impl From<String> for ModelPoolWorkerCallError {
    fn from(error: String) -> Self {
        Self::Failed(error)
    }
}

fn call_model_pool_worker(
    worker: &ModelPoolWorkerView,
    prompt: &str,
    max_tokens: usize,
    stream: bool,
    timeout: Duration,
    should_cancel: &mut dyn FnMut() -> bool,
) -> Result<ModelPoolWorkerCall, ModelPoolWorkerCallError> {
    let body = format!(
        "{{\"model\":\"smartsteam-pool-worker\",\"messages\":[{{\"role\":\"user\",\"content\":{}}}],\"stream\":{},\"max_tokens\":{}}}",
        service_json_string(prompt),
        stream,
        max_tokens.max(1)
    );
    let response = post_http_json(
        &worker.base_url,
        "/v1/chat/completions",
        &body,
        timeout,
        should_cancel,
    )?;
    if stream {
        let streamed_tokens = model_pool_worker_streamed_tokens(&response);
        let answer = streamed_tokens.join("");
        if !answer.trim().is_empty() {
            return Ok(ModelPoolWorkerCall {
                answer,
                streamed_tokens,
            });
        }
    }
    let answer = json_string_field(&response, "content")
        .or_else(|| json_string_field(&response, "text"))
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            json_string_field(&response, "message")
                .map(|message| format!("model worker returned error: {message}"))
                .unwrap_or_else(|| "model worker response missing answer content".to_owned())
        })?;
    Ok(ModelPoolWorkerCall {
        answer,
        streamed_tokens: Vec::new(),
    })
}

fn model_pool_worker_streamed_tokens(body: &str) -> Vec<String> {
    body.lines()
        .filter_map(|line| line.trim().strip_prefix("data:"))
        .map(str::trim)
        .filter(|line| !line.is_empty() && *line != "[DONE]")
        .filter_map(|line| {
            json_string_field(line, "content").or_else(|| json_string_field(line, "text"))
        })
        .filter(|token| !token.is_empty())
        .collect()
}

fn post_http_json(
    base_url: &str,
    path: &str,
    body: &str,
    timeout: Duration,
    should_cancel: &mut dyn FnMut() -> bool,
) -> Result<String, ModelPoolWorkerCallError> {
    if timeout.is_zero() {
        return Err("model worker call timed out after 0ms".to_owned().into());
    }
    let started = Instant::now();
    let endpoint = parse_http_endpoint(base_url)?;
    if should_cancel() {
        return Err(ModelPoolWorkerCallError::Cancelled);
    }
    let remaining = remaining_model_pool_call_timeout(started, timeout, "connect model worker")?;
    let connect_timeout = remaining.min(MODEL_POOL_CONNECT_TIMEOUT);
    let mut stream = match TcpStream::connect_timeout(&endpoint.address, connect_timeout) {
        Ok(stream) => stream,
        Err(_) if should_cancel() => return Err(ModelPoolWorkerCallError::Cancelled),
        Err(error) => {
            let message = if matches!(
                error.kind(),
                std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
            ) && connect_timeout == remaining
            {
                format!(
                    "connect model worker {} timed out after {}ms",
                    endpoint.authority,
                    timeout.as_millis()
                )
            } else {
                format!(
                    "connect model worker {} failed: {error}",
                    endpoint.authority
                )
            };
            return Err(message.into());
        }
    };
    if should_cancel() {
        let _ = stream.shutdown(Shutdown::Both);
        return Err(ModelPoolWorkerCallError::Cancelled);
    }
    let request_path = endpoint.request_path(path);
    let request = format!(
        "POST {request_path} HTTP/1.1\r\nhost: {}\r\ncontent-type: application/json; charset=utf-8\r\naccept: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        endpoint.authority,
        body.len()
    );
    write_model_pool_call_request(
        &mut stream,
        request.as_bytes(),
        started,
        timeout,
        should_cancel,
    )?;
    let response = read_model_pool_call_response(&mut stream, started, timeout, should_cancel)?;
    Ok(parse_http_json_response(&response)?)
}

fn write_model_pool_call_request(
    stream: &mut TcpStream,
    mut request: &[u8],
    started: Instant,
    timeout: Duration,
    should_cancel: &mut dyn FnMut() -> bool,
) -> Result<(), ModelPoolWorkerCallError> {
    while !request.is_empty() {
        if should_cancel() {
            let _ = stream.shutdown(Shutdown::Both);
            return Err(ModelPoolWorkerCallError::Cancelled);
        }
        let remaining =
            remaining_model_pool_call_timeout(started, timeout, "write model worker call request")?;
        stream
            .set_write_timeout(Some(remaining.min(MODEL_POOL_CALL_CANCEL_POLL_INTERVAL)))
            .map_err(|error| format!("set model worker write timeout failed: {error}"))?;
        match stream.write(request) {
            Ok(0) => {
                return Err("write model worker call request returned zero bytes"
                    .to_owned()
                    .into());
            }
            Ok(written) => request = &request[written..],
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::Interrupted
                        | std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::WouldBlock
                ) => {}
            Err(error) => {
                return Err(format!("write model worker call request failed: {error}").into());
            }
        }
    }
    Ok(())
}

fn read_model_pool_call_response(
    stream: &mut TcpStream,
    started: Instant,
    timeout: Duration,
    should_cancel: &mut dyn FnMut() -> bool,
) -> Result<Vec<u8>, ModelPoolWorkerCallError> {
    let mut response = Vec::new();
    let mut buffer = [0_u8; 8192];
    loop {
        if should_cancel() {
            let _ = stream.shutdown(Shutdown::Both);
            return Err(ModelPoolWorkerCallError::Cancelled);
        }
        let remaining =
            remaining_model_pool_call_timeout(started, timeout, "read model worker call response")?;
        stream
            .set_read_timeout(Some(remaining.min(MODEL_POOL_CALL_CANCEL_POLL_INTERVAL)))
            .map_err(|error| format!("set model worker read timeout failed: {error}"))?;
        match stream.read(&mut buffer) {
            Ok(0) => return Ok(response),
            Ok(read) => response.extend_from_slice(&buffer[..read]),
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::Interrupted
                        | std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::WouldBlock
                ) => {}
            Err(error) => {
                return Err(format!("read model worker call response failed: {error}").into());
            }
        }
    }
}

fn remaining_model_pool_call_timeout(
    started: Instant,
    timeout: Duration,
    stage: &str,
) -> Result<Duration, String> {
    let remaining = timeout.saturating_sub(started.elapsed());
    if remaining.is_zero() {
        Err(format!("{stage} timed out after {}ms", timeout.as_millis()))
    } else {
        Ok(remaining)
    }
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
        atomic::{AtomicBool, AtomicUsize, Ordering},
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
            runtime_backend: Some("test-worker".to_owned()),
            runtime_device: None,
            runtime_accelerator: None,
            gpu_layers: None,
            input_cost_per_1k_micro_usd: None,
            output_cost_per_1k_micro_usd: None,
            remaining_budget_micro_usd: None,
            error: None,
            quarantine: None,
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
        spawn_model_worker(context_window, chat_seen, None)
    }

    fn spawn_slow_model_worker(
        context_window: usize,
        chat_seen: Arc<AtomicBool>,
        delay: Duration,
    ) -> (String, std::thread::JoinHandle<()>) {
        spawn_model_worker(context_window, chat_seen, Some(delay))
    }

    fn spawn_model_worker(
        context_window: usize,
        chat_seen: Arc<AtomicBool>,
        chat_delay: Option<Duration>,
    ) -> (String, std::thread::JoinHandle<()>) {
        spawn_model_worker_with_metadata(context_window, chat_seen, chat_delay, true, None, None)
    }

    fn spawn_streaming_model_worker(
        context_window: usize,
        chat_seen: Arc<AtomicBool>,
        stream_tokens: Vec<&'static str>,
    ) -> (String, std::thread::JoinHandle<()>) {
        spawn_model_worker_with_metadata(
            context_window,
            chat_seen,
            None,
            true,
            Some(stream_tokens),
            None,
        )
    }

    fn spawn_model_worker_without_runtime(
        context_window: usize,
        chat_seen: Arc<AtomicBool>,
    ) -> (String, std::thread::JoinHandle<()>) {
        spawn_model_worker_with_metadata(context_window, chat_seen, None, false, None, None)
    }

    fn spawn_disconnect_observing_model_worker(
        context_window: usize,
        chat_seen: Arc<AtomicBool>,
        disconnect_seen: Arc<AtomicBool>,
    ) -> (String, std::thread::JoinHandle<()>) {
        spawn_model_worker_with_metadata(
            context_window,
            chat_seen,
            None,
            true,
            None,
            Some(disconnect_seen),
        )
    }

    fn spawn_model_worker_with_metadata(
        context_window: usize,
        chat_seen: Arc<AtomicBool>,
        chat_delay: Option<Duration>,
        include_runtime: bool,
        stream_tokens: Option<Vec<&'static str>>,
        disconnect_seen: Option<Arc<AtomicBool>>,
    ) -> (String, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            listener.set_nonblocking(true).unwrap();
            let mut metadata_seen = false;
            let mut metadata_idle_polls = 0;
            let max_metadata_idle_polls = if chat_delay.is_some() { 500 } else { 100 };
            for _ in 0..500 {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        metadata_idle_polls = 0;
                        let Some(request) = read_optional_http_request(&mut stream) else {
                            continue;
                        };
                        if request.starts_with("GET /v1/models HTTP/1.1") {
                            metadata_seen = true;
                            let body = if include_runtime {
                                format!(
                                    "{{\"id\":\"fake-worker\",\"n_ctx\":{context_window},\"backend\":\"llama.cpp\",\"device\":\"metal\",\"metal\":true,\"n_gpu_layers\":99}}"
                                )
                            } else {
                                format!("{{\"id\":\"fake-worker\",\"n_ctx\":{context_window}}}")
                            };
                            stream
                                .write_all(http_json_response("200 OK", &body).as_bytes())
                                .unwrap();
                            continue;
                        }
                        if request.starts_with("POST /v1/chat/completions HTTP/1.1") {
                            chat_seen.store(true, Ordering::SeqCst);
                            if let Some(disconnect_seen) = disconnect_seen.as_ref() {
                                stream
                                    .set_read_timeout(Some(Duration::from_millis(20)))
                                    .unwrap();
                                let mut buffer = [0_u8; 1];
                                let started = Instant::now();
                                let disconnected = loop {
                                    match stream.read(&mut buffer) {
                                        Ok(0) => break true,
                                        Ok(_) => {}
                                        Err(error)
                                            if matches!(
                                                error.kind(),
                                                std::io::ErrorKind::TimedOut
                                                    | std::io::ErrorKind::WouldBlock
                                            ) && started.elapsed() < Duration::from_secs(2) => {}
                                        Err(error)
                                            if matches!(
                                                error.kind(),
                                                std::io::ErrorKind::TimedOut
                                                    | std::io::ErrorKind::WouldBlock
                                            ) =>
                                        {
                                            break false;
                                        }
                                        Err(_) => break true,
                                    }
                                };
                                disconnect_seen.store(disconnected, Ordering::SeqCst);
                                return;
                            }
                            if let Some(delay) = chat_delay {
                                std::thread::sleep(delay);
                                return;
                            }
                            if let Some(tokens) = stream_tokens.as_ref() {
                                assert!(request.contains("\"stream\":true"), "{request}");
                                let mut body = String::new();
                                for token in tokens {
                                    body.push_str(&format!(
                                        "data: {{\"choices\":[{{\"delta\":{{\"content\":{}}}}}]}}\n\n",
                                        service_json_string(token)
                                    ));
                                }
                                body.push_str("data: [DONE]\n\n");
                                let response = format!(
                                    "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                                    body.len(),
                                    body
                                );
                                stream.write_all(response.as_bytes()).unwrap();
                                return;
                            }
                            let body = "{\"content\":\"unexpected chat call\"}";
                            stream
                                .write_all(http_json_response("200 OK", body).as_bytes())
                                .unwrap();
                            return;
                        }
                        panic!("fake model worker received unexpected request: {request}");
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        if metadata_seen {
                            metadata_idle_polls += 1;
                            if metadata_idle_polls >= max_metadata_idle_polls {
                                return;
                            }
                        }
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

    fn spawn_scripted_model_worker(
        context_window: usize,
        fail_first_chat: bool,
    ) -> (
        String,
        Arc<AtomicBool>,
        Arc<AtomicUsize>,
        Arc<AtomicUsize>,
        std::thread::JoinHandle<()>,
    ) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let metadata_calls = Arc::new(AtomicUsize::new(0));
        let chat_calls = Arc::new(AtomicUsize::new(0));
        let worker_stop = Arc::clone(&stop);
        let worker_metadata_calls = Arc::clone(&metadata_calls);
        let worker_chat_calls = Arc::clone(&chat_calls);
        let handle = std::thread::spawn(move || {
            listener.set_nonblocking(true).unwrap();
            while !worker_stop.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let Some(request) = read_optional_http_request(&mut stream) else {
                            continue;
                        };
                        if request.starts_with("GET /v1/models HTTP/1.1") {
                            worker_metadata_calls.fetch_add(1, Ordering::SeqCst);
                            let body = format!(
                                "{{\"id\":\"scripted-worker\",\"n_ctx\":{context_window},\"backend\":\"llama.cpp\",\"device\":\"metal\",\"metal\":true,\"n_gpu_layers\":99}}"
                            );
                            stream
                                .write_all(http_json_response("200 OK", &body).as_bytes())
                                .unwrap();
                            continue;
                        }
                        if request.starts_with("POST /v1/chat/completions HTTP/1.1") {
                            let call = worker_chat_calls.fetch_add(1, Ordering::SeqCst);
                            let (status, body) = if fail_first_chat && call == 0 {
                                (
                                    "500 Internal Server Error",
                                    "{\"error\":\"scripted failure\"}",
                                )
                            } else {
                                ("200 OK", "{\"content\":\"scripted answer\"}")
                            };
                            stream
                                .write_all(http_json_response(status, body).as_bytes())
                                .unwrap();
                            continue;
                        }
                        panic!("scripted model worker received unexpected request: {request}");
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("scripted model worker accept failed: {error}"),
                }
            }
        });
        (
            format!("http://{address}"),
            stop,
            metadata_calls,
            chat_calls,
            handle,
        )
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
                    {{"role":"quality","base_url":"{quality_base_url}","default_context_tokens":262144,"default_max_tokens":262144,"low_priority":false,"runtime_backend":"llama.cpp","runtime_device":"metal","runtime_accelerator":"metal","gpu_layers":999,"input_cost_per_1k_micro_usd":400,"output_cost_per_1k_micro_usd":1200,"remaining_budget_micro_usd":900000}},
                    {{"role":"review","base_url":"{review_base_url}","default_context_tokens":8192,"default_max_tokens":1536,"low_priority":true,"runtime_backend":"llama.cpp","runtime_device":"metal","runtime_accelerator":"metal","gpu_layers":80,"input_cost_per_1k_micro_usd":40,"output_cost_per_1k_micro_usd":80,"remaining_budget_micro_usd":10000}}
                ]
            }}"#
        );
        fs::write(&path, manifest).unwrap();
        path
    }

    fn model_pool_failover_manifest_path(
        quality_base_url: &str,
        index_base_url: &str,
        summary_base_url: &str,
    ) -> std::path::PathBuf {
        let thread_id = format!("{:?}", std::thread::current().id());
        let path = std::env::temp_dir().join(format!(
            "smartsteam-model-pool-failover-{}-{thread_id}.json",
            std::process::id(),
        ));
        let manifest = format!(
            r#"{{
                "workers": [
                    {{"role":"quality","base_url":"{quality_base_url}","default_context_tokens":262144,"default_max_tokens":262144,"low_priority":false}},
                    {{"role":"index","base_url":"{index_base_url}","default_context_tokens":4096,"default_max_tokens":512,"low_priority":true}},
                    {{"role":"summary","base_url":"{summary_base_url}","default_context_tokens":8192,"default_max_tokens":768,"low_priority":true}}
                ]
            }}"#
        );
        fs::write(&path, manifest).unwrap();
        path
    }

    fn run_model_pool_failover_call(
        index_delay: Option<Duration>,
        summary_delay: Option<Duration>,
        timeout_ms: u64,
    ) -> (String, Duration, bool, bool, bool) {
        let quality_chat_seen = Arc::new(AtomicBool::new(false));
        let index_chat_seen = Arc::new(AtomicBool::new(false));
        let summary_chat_seen = Arc::new(AtomicBool::new(false));
        let (quality_base_url, quality_worker) =
            spawn_model_worker(262_144, Arc::clone(&quality_chat_seen), None);
        let (index_base_url, index_worker) =
            spawn_model_worker(4096, Arc::clone(&index_chat_seen), index_delay);
        let (summary_base_url, summary_worker) =
            spawn_model_worker(8192, Arc::clone(&summary_chat_seen), summary_delay);
        let manifest_path = model_pool_failover_manifest_path(
            &quality_base_url,
            &index_base_url,
            &summary_base_url,
        );
        let args = Args::parse(vec![
            "--model-pool-manifest".to_owned(),
            manifest_path.display().to_string(),
            "--runtime-timeout-ms".to_owned(),
            timeout_ms.to_string(),
        ]);
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let request = ModelServiceModelPoolCallRequest {
            task_kind: "index".to_owned(),
            prompt: "exercise model-pool failover".to_owned(),
            max_tokens: Some(64),
            stream: false,
            completed_roles: None,
        };
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            handle_model_pool_call(&args, &mut stream, 81, request).unwrap();
        });
        let mut client = TcpStream::connect(address).unwrap();
        let started = Instant::now();
        let response = read_http_response(&mut client);
        let elapsed = started.elapsed();

        server.join().unwrap();
        quality_worker.join().unwrap();
        index_worker.join().unwrap();
        summary_worker.join().unwrap();
        let _ = fs::remove_file(manifest_path);
        (
            response,
            elapsed,
            quality_chat_seen.load(Ordering::SeqCst),
            index_chat_seen.load(Ordering::SeqCst),
            summary_chat_seen.load(Ordering::SeqCst),
        )
    }

    fn run_model_pool_call_request_at(
        args: &Args,
        request_id: usize,
        isolation: &metrics::WorkerIsolationConfig,
        worker_state_now_unix: u64,
        outcome_now_unix: u64,
    ) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        std::thread::scope(|scope| {
            let server = scope.spawn(|| {
                let (mut stream, _) = listener.accept().unwrap();
                handle_model_pool_call_at(
                    args,
                    &mut stream,
                    request_id,
                    ModelServiceModelPoolCallRequest {
                        task_kind: "index".to_owned(),
                        prompt: "exercise persistent model-pool failover".to_owned(),
                        max_tokens: Some(64),
                        stream: false,
                        completed_roles: None,
                    },
                    isolation,
                    worker_state_now_unix,
                    Some(outcome_now_unix),
                )
                .unwrap();
            });
            let mut client = TcpStream::connect(address).unwrap();
            let response = read_http_response(&mut client);
            server.join().unwrap();
            response
        })
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
            input_cost_per_1k_micro_usd: None,
            output_cost_per_1k_micro_usd: None,
            remaining_budget_micro_usd: None,
            error: None,
            quarantine: None,
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
    fn caller_cancel_marker_preserves_write_half_closed_clients() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (server, _) = listener.accept().unwrap();
        server.set_nonblocking(true).unwrap();

        client.shutdown(Shutdown::Write).unwrap();
        std::thread::sleep(Duration::from_millis(20));

        assert!(!model_pool_caller_cancelled(&server));
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

        let call = call_model_pool_worker(
            &worker,
            "hello from test",
            77,
            false,
            Duration::from_secs(2),
            &mut || false,
        )
        .unwrap();
        let request = server.join().unwrap();

        assert_eq!(call.answer, "worker answer");
        assert!(call.streamed_tokens.is_empty());
        assert!(request.starts_with("POST /v1/chat/completions HTTP/1.1"));
        assert!(request.contains("\"model\":\"smartsteam-pool-worker\""));
        assert!(
            request.contains("\"messages\":[{\"role\":\"user\",\"content\":\"hello from test\"}]")
        );
        assert!(request.contains("\"stream\":false"));
        assert!(request.contains("\"max_tokens\":77"));
    }

    #[test]
    fn call_model_pool_worker_captures_openai_stream_tokens() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_single_http_request(&mut stream);
            let body = "data: {\"choices\":[{\"delta\":{\"content\":\"stream \"}}]}\n\ndata: {\"choices\":[{\"delta\":{\"content\":\"answer\"}}]}\n\ndata: [DONE]\n\n";
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
            request
        });
        let worker = ready_worker(format!("http://{address}/v1"));

        let call = call_model_pool_worker(
            &worker,
            "stream from test",
            32,
            true,
            Duration::from_secs(2),
            &mut || false,
        )
        .unwrap();
        let request = server.join().unwrap();

        assert_eq!(call.answer, "stream answer");
        assert_eq!(call.streamed_tokens, vec!["stream ", "answer"]);
        assert!(request.contains("\"stream\":true"));
        assert!(request.contains("\"max_tokens\":32"));
    }

    #[test]
    fn call_model_pool_worker_reports_deterministic_read_timeout() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let _request = read_single_http_request(&mut stream);
            std::thread::sleep(Duration::from_millis(150));
        });
        let worker = ready_worker(format!("http://{address}/v1"));

        let error = call_model_pool_worker(
            &worker,
            "timeout please",
            8,
            false,
            Duration::from_millis(25),
            &mut || false,
        )
        .unwrap_err();

        server.join().unwrap();
        assert_eq!(
            error,
            ModelPoolWorkerCallError::Failed(
                "read model worker call response timed out after 25ms".to_owned()
            )
        );
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
        let _metrics_guard = metrics::test_guard();
        metrics::reset();
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
            stream: false,
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
        assert!(response.contains("\"endpoint\":\"model-pool-call\""));
        assert!(response.contains("\"call_state\":\"blocked\""));
        assert!(response.contains("\"cancelled\":false"));
        assert!(response.contains("\"timeout\":false"));
        assert!(response.contains("\"partial_result\":false"));
        assert!(response.contains("\"partial_finalized\":false"));
        assert!(response.contains("\"queue_time_ms\":0"));
        assert!(
            response.contains(
                "\"error\":\"model_pool_launch_blocked:quality_context_window_too_small\""
            )
        );
        assert!(response.contains("\"retryable\":false"));
        assert!(response.contains("\"dispatch_attempted\":false"));
        assert!(response.contains("\"persistent_writes\":false"));
        assert!(response.contains("\"memory_write_allowed\":false"));
        assert!(response.contains("\"genome_write_allowed\":false"));
        assert!(response.contains("\"self_evolution_write_allowed\":false"));
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
    fn model_pool_call_blocks_when_agent_route_profile_is_not_routeable() {
        let _metrics_guard = metrics::test_guard();
        metrics::reset();
        let quality_chat_seen = Arc::new(AtomicBool::new(false));
        let review_chat_seen = Arc::new(AtomicBool::new(false));
        let (quality_base_url, quality_worker) =
            spawn_fake_model_worker(262_144, Arc::clone(&quality_chat_seen));
        let (review_base_url, review_worker) =
            spawn_model_worker_without_runtime(8192, Arc::clone(&review_chat_seen));
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
            prompt: "do not send this prompt without route proof".to_owned(),
            max_tokens: Some(1024),
            stream: false,
            completed_roles: None,
        };
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            handle_model_pool_call(&args, &mut stream, 87, request).unwrap();
        });
        let mut client = TcpStream::connect(address).unwrap();

        let response = read_http_response(&mut client);

        server.join().unwrap();
        quality_worker.join().unwrap();
        review_worker.join().unwrap();
        let _ = fs::remove_file(manifest_path);
        assert!(response.starts_with("HTTP/1.1 409 Conflict"));
        assert!(response.contains("\"sends_prompt\":false"));
        assert!(response.contains("\"route_block_reason\":\"no_route_profile_candidate\""));
        assert!(response.contains("\"error\":\"no_route_profile_candidate\""));
        assert!(response.contains("\"dispatch_attempted\":false"));
        assert!(response.contains("\"runtime_backend\":null"));
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
        assert_eq!(workers[0].input_cost_per_1k_micro_usd, Some(400));
        assert_eq!(workers[0].output_cost_per_1k_micro_usd, Some(1200));
        assert_eq!(workers[0].remaining_budget_micro_usd, Some(900_000));
        assert_eq!(workers[1].input_cost_per_1k_micro_usd, Some(40));
        assert_eq!(workers[1].output_cost_per_1k_micro_usd, Some(80));
        assert_eq!(workers[1].remaining_budget_micro_usd, Some(10_000));
        assert!(json.contains("\"runtime_backend\":\"llama.cpp\""));
        assert!(json.contains("\"runtime_device\":\"metal\""));
        assert!(json.contains("\"runtime_accelerator\":\"metal\""));
        assert!(json.contains("\"gpu_layers\":999"));
        assert!(json.contains("\"input_cost_per_1k_micro_usd\":400"));
        assert!(json.contains("\"output_cost_per_1k_micro_usd\":1200"));
        assert!(json.contains("\"remaining_budget_micro_usd\":900000"));
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
        assert!(json.contains("\"input_cost_per_1k_micro_usd\":400"));
        assert!(json.contains("\"output_cost_per_1k_micro_usd\":1200"));
        assert!(json.contains("\"remaining_budget_micro_usd\":900000"));
    }

    #[test]
    fn model_pool_manifest_advice_uses_full_helper_next_step_when_all_helpers_visible() {
        let args = Args::parse(Vec::new());
        let specs = worker_specs(&args).unwrap();

        let json = model_pool_manifest_response_json(94, &specs);

        assert!(json.contains("\"request_id\":94"));
        assert!(json.contains(
            "\"helper_roles\":[\"summary\",\"router\",\"review\",\"test-gate\",\"index\"]"
        ));
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
    fn model_pool_call_timeout_returns_structured_failure_json() {
        let _metrics_guard = metrics::test_guard();
        metrics::reset();
        let quality_chat_seen = Arc::new(AtomicBool::new(false));
        let review_chat_seen = Arc::new(AtomicBool::new(false));
        let (quality_base_url, quality_worker) =
            spawn_fake_model_worker(262_144, Arc::clone(&quality_chat_seen));
        let (review_base_url, review_worker) = spawn_slow_model_worker(
            8192,
            Arc::clone(&review_chat_seen),
            Duration::from_millis(150),
        );
        let manifest_path = model_pool_manifest_path(&quality_base_url, &review_base_url);
        let args = Args::parse(vec![
            "--model-pool-manifest".to_owned(),
            manifest_path.display().to_string(),
            "--runtime-timeout-ms".to_owned(),
            "25".to_owned(),
        ]);
        let outcomes_path = manifest_path.with_extension("timeout-outcomes.jsonl");
        let isolation = metrics::WorkerIsolationConfig::new(outcomes_path.clone(), 60);
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let request = ModelServiceModelPoolCallRequest {
            task_kind: "review".to_owned(),
            prompt: "timeout this prompt".to_owned(),
            max_tokens: Some(64),
            stream: false,
            completed_roles: None,
        };
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            handle_model_pool_call_at(
                &args,
                &mut stream,
                79,
                request,
                &isolation,
                1_000_000,
                Some(1_000_025),
            )
            .unwrap();
        });
        let mut client = TcpStream::connect(address).unwrap();

        let response = read_http_response(&mut client);

        server.join().unwrap();
        quality_worker.join().unwrap();
        review_worker.join().unwrap();
        let _ = fs::remove_file(manifest_path);
        let outcomes = fs::read_to_string(&outcomes_path).unwrap();
        let _ = fs::remove_file(outcomes_path);
        assert!(
            response.starts_with("HTTP/1.1 502 Bad Gateway"),
            "unexpected response: {response}"
        );
        assert!(response.contains("\"ok\":false"));
        assert!(response.contains("\"request_id\":79"));
        assert!(response.contains("\"schema_version\":1"));
        assert!(response.contains("\"contract_version\":\"model-pool.v1\""));
        assert!(response.contains("\"task_kind\":\"review\""));
        assert!(response.contains("\"read_only\":false"));
        assert!(response.contains("\"launches_process\":false"));
        assert!(response.contains("\"sends_prompt\":true"));
        assert!(response.contains("\"endpoint\":\"model-pool-call\""));
        assert!(response.contains("\"selected_role\":\"review\""));
        assert!(response.contains("\"call_state\":\"failed\""));
        assert!(response.contains("\"timeout\":true"));
        assert!(response.contains("\"partial_result\":false"));
        assert!(response.contains("\"partial_finalized\":true"));
        assert!(response.contains("\"retryable\":true"));
        assert!(response.contains("\"dispatch_attempted\":true"));
        assert!(response.contains("\"persistent_writes\":true"));
        assert!(response.contains("\"memory_write_allowed\":false"));
        assert!(response.contains("\"genome_write_allowed\":false"));
        assert!(response.contains("\"self_evolution_write_allowed\":false"));
        assert!(response.contains("\"compute_budget_summary\":\"model_pool_call selected_role=review effective_max_tokens=64 saved_tokens=0 max_tokens_clamped=false failed_before_worker_answer\""));
        assert!(response.contains("\"compute_budget_configured_max_tokens\":64"));
        assert!(response.contains("\"compute_budget_effective_max_tokens\":64"));
        assert!(response.contains("\"compute_budget_saved_tokens\":0"));
        assert!(response.contains("\"compute_budget_avoided_tokens\":0"));
        assert!(response.contains("\"compute_budget_max_tokens_clamped\":false"));
        assert!(response.contains(
            "\"runtime_closed_loop_counters\":{\"compute_budget_saved_tokens\":0,\"compute_budget_avoided_tokens\":0,\"compute_budget_max_tokens_clamped\":false,\"model_pool_budget_applied\":true}"
        ));
        assert!(response.contains(
            "\"error\":\"model pool call failed: read model worker call response timed out after 25ms\""
        ));
        assert!(!quality_chat_seen.load(Ordering::SeqCst));
        assert!(review_chat_seen.load(Ordering::SeqCst));
        assert!(outcomes.contains("\"reason\":\"timeout\""));
    }

    #[test]
    fn model_pool_call_fails_over_to_next_ranked_worker() {
        let _metrics_guard = metrics::test_guard();
        metrics::reset();
        let (response, _, quality_chat_seen, index_chat_seen, summary_chat_seen) =
            run_model_pool_failover_call(Some(Duration::from_millis(300)), None, 200);
        assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
        assert!(
            response.contains("\"selected_role\":\"summary\""),
            "{response}"
        );
        assert!(index_chat_seen);
        assert!(summary_chat_seen);
        assert!(!quality_chat_seen);
        let snapshot = metrics::snapshot();
        let index = snapshot
            .worker_metrics
            .iter()
            .find(|worker| worker.role == "index")
            .expect("index metrics should be present");
        let summary = snapshot
            .worker_metrics
            .iter()
            .find(|worker| worker.role == "summary")
            .expect("summary metrics should be present");
        assert!(index.metrics.failure_count >= 1);
        assert!(summary.metrics.success_count >= 1);
    }

    #[test]
    fn model_pool_call_persists_worker_cooldown_across_metric_reset() {
        let _metrics_guard = metrics::test_guard();
        metrics::reset();
        let (quality_url, quality_stop, _, quality_chat_calls, quality_worker) =
            spawn_scripted_model_worker(262_144, false);
        let (index_url, index_stop, index_metadata_calls, index_chat_calls, index_worker) =
            spawn_scripted_model_worker(4096, true);
        let (summary_url, summary_stop, _, summary_chat_calls, summary_worker) =
            spawn_scripted_model_worker(8192, false);
        let manifest_path =
            model_pool_failover_manifest_path(&quality_url, &index_url, &summary_url);
        let args = Args::parse(vec![
            "--model-pool-manifest".to_owned(),
            manifest_path.display().to_string(),
            "--runtime-timeout-ms".to_owned(),
            "300".to_owned(),
        ]);
        let outcomes_path = manifest_path.with_extension("outcomes.jsonl");
        let isolation = metrics::WorkerIsolationConfig::new(outcomes_path.clone(), 60);

        let first = run_model_pool_call_request_at(&args, 91, &isolation, 1_000_000, 1_000_300);
        metrics::reset();
        let second = run_model_pool_call_request_at(&args, 92, &isolation, 1_000_301, 1_000_301);
        metrics::reset();
        let third = run_model_pool_call_request_at(&args, 93, &isolation, 1_000_361, 1_000_361);
        let index_metadata_calls = index_metadata_calls.load(Ordering::SeqCst);
        let index_chat_calls = index_chat_calls.load(Ordering::SeqCst);
        let summary_chat_calls = summary_chat_calls.load(Ordering::SeqCst);
        let quality_chat_calls = quality_chat_calls.load(Ordering::SeqCst);
        let outcomes = fs::read_to_string(&outcomes_path).unwrap();

        quality_stop.store(true, Ordering::SeqCst);
        index_stop.store(true, Ordering::SeqCst);
        summary_stop.store(true, Ordering::SeqCst);
        quality_worker.join().unwrap();
        index_worker.join().unwrap();
        summary_worker.join().unwrap();
        let _ = fs::remove_file(manifest_path);
        let _ = fs::remove_file(outcomes_path);

        assert!(first.contains("\"selected_role\":\"summary\""), "{first}");
        assert!(second.contains("\"selected_role\":\"summary\""), "{second}");
        assert!(third.contains("\"selected_role\":\"index\""), "{third}");
        assert_eq!(index_metadata_calls, 2);
        assert_eq!(index_chat_calls, 2);
        assert_eq!(summary_chat_calls, 2);
        assert_eq!(quality_chat_calls, 0);
        assert!(outcomes.contains("\"reason\":\"worker_http_5xx\""));
        assert!(
            outcomes.lines().any(|line| {
                line.contains("\"role\":\"index\"") && line.contains("\"ok\":true")
            })
        );
    }

    #[test]
    fn model_pool_call_does_not_retry_failed_physical_endpoint_under_another_role() {
        let _metrics_guard = metrics::test_guard();
        metrics::reset();
        let (quality_url, quality_stop, _, quality_chat_calls, quality_worker) =
            spawn_scripted_model_worker(262_144, false);
        let (shared_url, shared_stop, _, shared_chat_calls, shared_worker) =
            spawn_scripted_model_worker(8192, true);
        let manifest_path =
            model_pool_failover_manifest_path(&quality_url, &shared_url, &shared_url);
        let args = Args::parse(vec![
            "--model-pool-manifest".to_owned(),
            manifest_path.display().to_string(),
            "--runtime-timeout-ms".to_owned(),
            "300".to_owned(),
        ]);
        let outcomes_path = manifest_path.with_extension("shared-outcomes.jsonl");
        let isolation = metrics::WorkerIsolationConfig::new(outcomes_path.clone(), 60);

        let response = run_model_pool_call_request_at(&args, 94, &isolation, 1_000_000, 1_000_000);
        let shared_chat_calls = shared_chat_calls.load(Ordering::SeqCst);
        let quality_chat_calls = quality_chat_calls.load(Ordering::SeqCst);

        quality_stop.store(true, Ordering::SeqCst);
        shared_stop.store(true, Ordering::SeqCst);
        quality_worker.join().unwrap();
        shared_worker.join().unwrap();
        let _ = fs::remove_file(manifest_path);
        let _ = fs::remove_file(outcomes_path);

        assert!(
            response.starts_with("HTTP/1.1 502 Bad Gateway"),
            "{response}"
        );
        assert!(response.contains("\"persistent_writes\":true"));
        assert_eq!(shared_chat_calls, 1);
        assert_eq!(quality_chat_calls, 0);
    }

    #[test]
    fn model_pool_worker_failure_quarantine_ignores_ordinary_request_errors() {
        assert_eq!(
            model_pool_worker_failure_reason("model worker returned HTTP 400"),
            None
        );
        assert_eq!(
            model_pool_worker_failure_reason("model worker returned HTTP 404"),
            None
        );
        assert_eq!(
            model_pool_worker_failure_reason("model worker returned HTTP 429"),
            Some("rate_limit")
        );
        assert_eq!(
            model_pool_worker_failure_reason("model worker returned HTTP 500"),
            Some("worker_http_5xx")
        );
        assert_eq!(
            model_pool_worker_failure_reason("model worker response missing answer content"),
            Some("response_shape")
        );
    }

    #[test]
    fn worker_dispatch_gate_rechecks_recent_quarantine() {
        let _metrics_guard = metrics::test_guard();
        metrics::reset();
        let worker = ready_worker("http://127.0.0.1:8688".to_owned());
        let outcomes_path = std::env::temp_dir().join(format!(
            "rust-norion-model-pool-dispatch-gate-{}.jsonl",
            std::process::id()
        ));
        let _ = fs::remove_file(&outcomes_path);
        let isolation = metrics::WorkerIsolationConfig::new(outcomes_path.clone(), 60);

        let first = metrics::try_begin_worker_call(&worker, &isolation, 100).unwrap();
        assert!(first.finish_with_reason_at(false, Some("transport"), 100));
        assert!(metrics::try_begin_worker_call(&worker, &isolation, 101).is_none());
        let half_open = metrics::try_begin_worker_call(&worker, &isolation, 160).unwrap();
        assert!(metrics::try_begin_worker_call(&worker, &isolation, 160).is_none());
        drop(half_open);
        let next_half_open = metrics::try_begin_worker_call(&worker, &isolation, 160).unwrap();
        drop(next_half_open);
        let _ = fs::remove_file(outcomes_path);
    }

    #[test]
    fn worker_outcome_write_failure_keeps_in_memory_quarantine() {
        let _metrics_guard = metrics::test_guard();
        metrics::reset();
        let review_url = "http://127.0.0.1:8688";
        let manifest_path = model_pool_manifest_path("http://127.0.0.1:8686", review_url);
        let args = Args::parse(vec![
            "--model-pool-manifest".to_owned(),
            manifest_path.display().to_string(),
        ]);
        let specs = worker_specs(&args).unwrap();
        let worker = ready_worker(review_url.to_owned());
        let outcomes_path = std::env::temp_dir().join(format!(
            "rust-norion-model-pool-unwritable-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&outcomes_path);
        fs::create_dir_all(&outcomes_path).unwrap();
        let isolation = metrics::WorkerIsolationConfig::new(outcomes_path.clone(), 60);

        let call = metrics::try_begin_worker_call(&worker, &isolation, 100).unwrap();
        assert!(!call.finish_with_reason_at(false, Some("transport"), 100));
        let quarantines = metrics::worker_quarantines(&specs, &isolation, 101);
        let quarantine = quarantines
            .get(&model_pool_worker_id(review_url))
            .expect("in-memory quarantine should survive persistence failure");

        assert_eq!(quarantine.consecutive_failures, 1);
        assert_eq!(quarantine.reason, "transport");
        assert!(!quarantine.persisted);
        assert!(metrics::try_begin_worker_call(&worker, &isolation, 101).is_none());

        let _ = fs::remove_file(manifest_path);
        let _ = fs::remove_dir_all(outcomes_path);
    }

    #[test]
    fn model_pool_failure_json_reports_when_dispatch_was_skipped() {
        let json = model_pool_call_failure_json(
            95,
            "index",
            "index",
            Some(64),
            64,
            false,
            "worker entered failure cooldown before dispatch",
            false,
            false,
        );

        assert!(json.contains("\"sends_prompt\":false"));
        assert!(json.contains("\"dispatch_attempted\":false"));
        assert!(json.contains("\"persistent_writes\":false"));
    }

    #[test]
    fn model_pool_call_cancel_marker_abandons_worker_without_failure_or_fallback() {
        let _metrics_guard = metrics::test_guard();
        metrics::reset();
        let quality_chat_seen = Arc::new(AtomicBool::new(false));
        let index_chat_seen = Arc::new(AtomicBool::new(false));
        let index_disconnect_seen = Arc::new(AtomicBool::new(false));
        let summary_chat_seen = Arc::new(AtomicBool::new(false));
        let (quality_base_url, quality_worker) =
            spawn_fake_model_worker(262_144, Arc::clone(&quality_chat_seen));
        let (index_base_url, index_worker) = spawn_disconnect_observing_model_worker(
            4096,
            Arc::clone(&index_chat_seen),
            Arc::clone(&index_disconnect_seen),
        );
        let (summary_base_url, summary_worker) =
            spawn_fake_model_worker(8192, Arc::clone(&summary_chat_seen));
        let manifest_path = model_pool_failover_manifest_path(
            &quality_base_url,
            &index_base_url,
            &summary_base_url,
        );
        let args = Args::parse(vec![
            "--model-pool-manifest".to_owned(),
            manifest_path.display().to_string(),
            "--runtime-timeout-ms".to_owned(),
            "2000".to_owned(),
        ]);
        let outcomes_path = manifest_path.with_extension("cancel-outcomes.jsonl");
        let isolation = metrics::WorkerIsolationConfig::new(outcomes_path.clone(), 60);
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let raw = crate::model_service::http::read_http_request(&mut stream).unwrap();
            let request = match crate::model_service::request::parse_model_service_http_request(
                &raw,
            )
            .unwrap()
            {
                crate::model_service::request::ModelServiceHttpRequest::ModelPoolCall(request) => {
                    request
                }
                _ => panic!("expected model-pool call request"),
            };
            handle_model_pool_call_at(
                &args,
                &mut stream,
                82,
                request,
                &isolation,
                1_000_000,
                Some(1_000_000),
            )
        });
        let mut client = TcpStream::connect(address).unwrap();
        let body = r#"{"task_kind":"index","prompt":"cancel this model-pool call","max_tokens":64,"stream":false}"#;
        let request = format!(
            "POST /v1/model-pool/call HTTP/1.1\r\nhost: {address}\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
            body.len()
        );
        client.write_all(request.as_bytes()).unwrap();
        let wait_started = Instant::now();
        while !index_chat_seen.load(Ordering::SeqCst)
            && wait_started.elapsed() < Duration::from_secs(2)
        {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(index_chat_seen.load(Ordering::SeqCst));

        let cancel_started = Instant::now();
        client.write_all(MODEL_POOL_CALL_CANCEL_MARKER).unwrap();
        client.shutdown(Shutdown::Both).unwrap();
        drop(client);
        server.join().unwrap().unwrap();
        let cancel_elapsed = cancel_started.elapsed();

        quality_worker.join().unwrap();
        index_worker.join().unwrap();
        summary_worker.join().unwrap();
        let _ = fs::remove_file(manifest_path);
        assert!(
            cancel_elapsed < Duration::from_secs(1),
            "{cancel_elapsed:?}"
        );
        assert!(index_disconnect_seen.load(Ordering::SeqCst));
        assert!(!quality_chat_seen.load(Ordering::SeqCst));
        assert!(!summary_chat_seen.load(Ordering::SeqCst));
        let snapshot = metrics::snapshot();
        let index = snapshot
            .worker_metrics
            .iter()
            .find(|worker| worker.role == "index")
            .expect("index metrics should be present");
        assert_eq!(index.metrics.in_flight, 0);
        assert_eq!(index.metrics.success_count, 0);
        assert_eq!(index.metrics.failure_count, 0);
        assert_eq!(snapshot.route_metrics.failure_count, 0);
        assert!(!outcomes_path.exists());
    }

    #[test]
    fn model_pool_call_skips_resource_constrained_primary_for_ready_fallback() {
        let _metrics_guard = metrics::test_guard();
        metrics::reset();
        let mut index_worker = ready_worker("http://127.0.0.1:8690".to_owned());
        index_worker.role = "index".to_owned();
        let isolation = metrics::WorkerIsolationConfig::new(
            std::env::temp_dir().join(format!(
                "rust-norion-model-pool-pressure-{}.jsonl",
                std::process::id()
            )),
            60,
        );
        let index_pressure = (0..3)
            .map(|_| metrics::begin_worker_call(&index_worker, &isolation))
            .collect::<Vec<_>>();
        let (response, _, quality_chat_seen, index_chat_seen, summary_chat_seen) =
            run_model_pool_failover_call(None, None, 200);
        assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
        assert!(
            response.contains("\"selected_role\":\"summary\""),
            "{response}"
        );
        assert!(!index_chat_seen);
        assert!(summary_chat_seen);
        assert!(!quality_chat_seen);
        drop(index_pressure);
    }

    #[test]
    fn model_pool_call_returns_502_after_all_eligible_workers_fail() {
        let _metrics_guard = metrics::test_guard();
        metrics::reset();
        let (response, call_elapsed, quality_chat_seen, index_chat_seen, summary_chat_seen) =
            run_model_pool_failover_call(
                Some(Duration::from_millis(300)),
                Some(Duration::from_millis(300)),
                120,
            );
        assert!(
            response.starts_with("HTTP/1.1 502 Bad Gateway"),
            "{response}"
        );
        assert!(
            response.contains("\"selected_role\":\"summary\""),
            "{response}"
        );
        assert!(response.contains("\"timeout\":true"), "{response}");
        assert!(index_chat_seen);
        assert!(summary_chat_seen);
        assert!(!quality_chat_seen);
        assert!(
            call_elapsed < Duration::from_millis(300),
            "{call_elapsed:?}"
        );
        let snapshot = metrics::snapshot();
        let index = snapshot
            .worker_metrics
            .iter()
            .find(|worker| worker.role == "index")
            .expect("index metrics should be present");
        let summary = snapshot
            .worker_metrics
            .iter()
            .find(|worker| worker.role == "summary")
            .expect("summary metrics should be present");
        assert!(snapshot.route_metrics.failure_count >= 2);
        assert!(index.metrics.failure_count >= 1);
        assert!(summary.metrics.failure_count >= 1);
    }

    #[test]
    fn model_pool_call_success_includes_execution_metrics() {
        let _metrics_guard = metrics::test_guard();
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
            stream: false,
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
        assert!(response.contains("\"endpoint\":\"model-pool-call\""));
        assert!(response.contains("\"call_state\":\"completed\""));
        assert!(response.contains("\"cancelled\":false"));
        assert!(response.contains("\"timeout\":false"));
        assert!(response.contains("\"partial_result\":false"));
        assert!(response.contains("\"partial_finalized\":false"));
        assert!(response.contains("\"queue_time_ms\":0"));
        assert!(response.contains("\"error\":null"));
        assert!(response.contains("\"retryable\":false"));
        assert!(response.contains("\"dispatch_attempted\":true"));
        assert!(response.contains("\"compute_budget_summary\":\"model_pool_call selected_role=review effective_max_tokens=1024 saved_tokens=0 max_tokens_clamped=false\""));
        assert!(response.contains("\"compute_budget_configured_max_tokens\":1024"));
        assert!(response.contains("\"compute_budget_effective_max_tokens\":1024"));
        assert!(response.contains("\"compute_budget_saved_tokens\":0"));
        assert!(response.contains("\"compute_budget_avoided_tokens\":0"));
        assert!(response.contains("\"compute_budget_max_tokens_clamped\":false"));
        assert!(response.contains("\"persistent_writes\":true"));
        assert!(response.contains("\"memory_write_allowed\":true"));
        assert!(response.contains("\"genome_write_allowed\":true"));
        assert!(response.contains("\"self_evolution_write_allowed\":true"));
        assert!(response.contains("\"answer\":\"unexpected chat call\""));
        assert!(response.contains("\"success_count\":"));
        assert!(!quality_chat_seen.load(Ordering::SeqCst));
        assert!(review_chat_seen.load(Ordering::SeqCst));
    }

    #[test]
    fn model_pool_call_stream_request_exposes_worker_stream_tokens() {
        let _metrics_guard = metrics::test_guard();
        metrics::reset();
        let quality_chat_seen = Arc::new(AtomicBool::new(false));
        let review_chat_seen = Arc::new(AtomicBool::new(false));
        let (quality_base_url, quality_worker) =
            spawn_fake_model_worker(262_144, Arc::clone(&quality_chat_seen));
        let (review_base_url, review_worker) = spawn_streaming_model_worker(
            8192,
            Arc::clone(&review_chat_seen),
            vec!["review ", "stream ", "answer"],
        );
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
            prompt: "stream this prompt".to_owned(),
            max_tokens: Some(128),
            stream: true,
            completed_roles: None,
        };
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            handle_model_pool_call(&args, &mut stream, 80, request).unwrap();
        });
        let mut client = TcpStream::connect(address).unwrap();

        let response = read_http_response(&mut client);

        server.join().unwrap();
        quality_worker.join().unwrap();
        review_worker.join().unwrap();
        let _ = fs::remove_file(manifest_path);
        assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
        assert!(response.contains("\"worker_streamed\":true"), "{response}");
        assert!(
            response.contains("\"worker_streamed_token_count\":3"),
            "{response}"
        );
        assert!(
            response.contains("\"worker_streamed_tokens\":[\"review \",\"stream \",\"answer\"]"),
            "{response}"
        );
        assert!(
            response.contains("\"answer\":\"review stream answer\""),
            "{response}"
        );
        assert!(!quality_chat_seen.load(Ordering::SeqCst));
        assert!(review_chat_seen.load(Ordering::SeqCst));
    }
}
