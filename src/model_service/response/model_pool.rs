use super::model_pool_routing::{
    ModelPoolDependencyPrecheckView, dependency_precheck_json, model_pool_dependency_precheck,
    model_pool_route_candidates_with_weights, routing_weights_json,
};
use crate::model_service::json::{
    option_str_service_json, option_usize_service_json, service_json_string,
};
use model_pool_advice_core::{
    CAPACITY_POLICY as MODEL_POOL_CAPACITY_POLICY, HELPER_ROLES as MODEL_POOL_HELPER_ROLES,
    HELPER_TARGET_WORKERS as MODEL_POOL_HELPER_TARGET_WORKERS,
    MAX_QUALITY_12B_WORKERS as MODEL_POOL_MAX_QUALITY_12B_WORKERS, ModelPoolFacts,
    POLICY as MODEL_POOL_ADVICE_POLICY, RECOMMENDED_LAUNCH_ROLES, missing_helper_roles,
    model_pool_decision,
};
use rust_norion::homeostasis::{AllostaticLoadCounters, HomeostaticSetpoints};

const MODEL_POOL_ADVICE_SOURCE: &str = "model-pool-advice-core";
const TEST_GATE_BASE_CONTEXT_BUFFER_TOKENS: usize = 2048;
const TEST_GATE_UPSTREAM_ROLE_CONTEXT_BUFFER_TOKENS: usize = 256;
const TEST_GATE_UPSTREAM_CONTEXT_BUFFER_ROLES: [&str; 2] = ["review", "index"];

pub(crate) struct ModelPoolWorkerView {
    pub(crate) role: String,
    pub(crate) port: u16,
    pub(crate) base_url: String,
    pub(crate) enabled_by_default: bool,
    pub(crate) model_class: String,
    pub(crate) suggested_quant: String,
    pub(crate) default_context_tokens: usize,
    pub(crate) default_max_tokens: usize,
    pub(crate) low_priority: bool,
    pub(crate) reachable: bool,
    pub(crate) model: Option<String>,
    pub(crate) context_window: Option<usize>,
    pub(crate) runtime_backend: Option<String>,
    pub(crate) runtime_device: Option<String>,
    pub(crate) runtime_accelerator: Option<String>,
    pub(crate) gpu_layers: Option<usize>,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ModelPoolQualityGate {
    pub(crate) launch_allowed: bool,
    pub(crate) reason: &'static str,
    pub(crate) quality_ready: bool,
    pub(crate) quality_context_tokens: Option<usize>,
    pub(crate) quality_context_required_tokens: Option<usize>,
    pub(crate) quality_context_sufficient: bool,
    pub(crate) quality_block_reason: &'static str,
    pub(crate) quality_worker_count: usize,
    pub(crate) extra_quality_12b_detected: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ModelPoolMetricsView {
    pub(crate) route_count: u64,
    pub(crate) selected_count: u64,
    pub(crate) blocked_count: u64,
    pub(crate) in_flight: u64,
    pub(crate) queued_count: u64,
    pub(crate) lease_wait_ms: Option<u64>,
    pub(crate) lease_wait_p95_ms: Option<u64>,
    pub(crate) success_count: u64,
    pub(crate) failure_count: u64,
    pub(crate) avg_latency_ms: Option<u64>,
    pub(crate) latency_p50_ms: Option<u64>,
    pub(crate) latency_p95_ms: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ModelPoolWorkerMetricsView {
    pub(crate) role: String,
    pub(crate) metrics: ModelPoolMetricsView,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ModelPoolMetricsSnapshotView {
    pub(crate) route_metrics: ModelPoolMetricsView,
    pub(crate) worker_metrics: Vec<ModelPoolWorkerMetricsView>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ModelPoolServiceBackpressureView {
    pub(crate) active_engine_requests: usize,
    pub(crate) max_active_stream_engine_requests: usize,
    pub(crate) stream_backpressure_rejections: usize,
}

impl ModelPoolServiceBackpressureView {
    pub(crate) fn new(
        active_engine_requests: usize,
        max_active_stream_engine_requests: usize,
        stream_backpressure_rejections: usize,
    ) -> Self {
        Self {
            active_engine_requests,
            max_active_stream_engine_requests,
            stream_backpressure_rejections,
        }
    }

    pub(crate) fn allow_dispatch(self) -> bool {
        self.active_engine_requests < self.max_active_stream_engine_requests
    }

    fn pressure(self) -> &'static str {
        if self.allow_dispatch() {
            "available"
        } else {
            "saturated"
        }
    }

    fn reason(self) -> &'static str {
        if self.allow_dispatch() {
            "stream_slots_available"
        } else {
            "stream_slots_saturated"
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelPoolMaxTokensDecision {
    pub(crate) configured_max_tokens: Option<usize>,
    pub(crate) effective_max_tokens: usize,
    pub(crate) max_tokens_clamped: bool,
    pub(crate) max_tokens_clamp_reason: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ModelPoolRouteContextDecision {
    pub(crate) selected_context_required_tokens: Option<usize>,
    pub(crate) selected_context_buffer_tokens: Option<usize>,
    pub(crate) selected_context_sufficient: bool,
    pub(crate) selected_context_block_reason: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ModelPoolRouteContextBufferPolicy {
    strategy: &'static str,
    base_tokens: usize,
    upstream_role_tokens: usize,
    eligible_upstream_roles: Vec<String>,
    completed_upstream_roles: Vec<String>,
    total_tokens: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ModelPoolCallExecutionView {
    pub(crate) elapsed_ms: u64,
    pub(crate) answer_chars: usize,
    pub(crate) answer_bytes: usize,
    pub(crate) answer_approx_tokens: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ModelPoolCapacitySummary {
    worker_count: usize,
    healthy_worker_count: usize,
    failed_worker_count: usize,
    helper_worker_count: usize,
    healthy_helper_worker_count: usize,
    metal_worker_count: usize,
    cpu_worker_count: usize,
    unknown_runtime_worker_count: usize,
    zero_gpu_layer_worker_count: usize,
    quality_runtime_accelerated: Option<bool>,
    model_pool_saturation_milli: u16,
    homeostatic_model_cell_expansion_allowed: bool,
    homeostatic_decision: &'static str,
    expansion_allowed: bool,
    recommendation: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ModelPoolStatusAdvice {
    safe_to_enable_pool_workers: bool,
    next_step: &'static str,
    reason: &'static str,
    kind: &'static str,
    extra_quality_12b_detected: bool,
    quality_worker_count: usize,
    helper_worker_count: usize,
    healthy_helper_worker_count: usize,
    helper_roles: Vec<String>,
    expected_helper_roles: Vec<String>,
    missing_helper_roles: Vec<String>,
    helper_cpu_or_no_gpu_roles: Vec<String>,
    blocking_helper_cpu_or_no_gpu_roles: Vec<String>,
    allowed_cpu_fallback_helper_roles: Vec<String>,
    recommended_launch_order: Vec<String>,
}

impl ModelPoolWorkerView {
    pub(crate) fn effective_context_tokens(&self) -> usize {
        self.context_window.unwrap_or(self.default_context_tokens)
    }

    pub(crate) fn ready(&self) -> bool {
        self.reachable && self.error.is_none()
    }

    pub(crate) fn status(&self) -> &'static str {
        if self.ready() {
            "healthy"
        } else if self.reachable {
            "tcp_only"
        } else {
            "unreachable"
        }
    }

    pub(crate) fn reason(&self) -> &'static str {
        if self.ready() {
            "none"
        } else if self.reachable {
            "health_failed"
        } else {
            "tcp_unreachable"
        }
    }
}

impl ModelPoolCallExecutionView {
    pub(crate) fn from_answer(elapsed_ms: u64, answer: &str) -> Self {
        let answer_chars = answer.chars().count();
        Self {
            elapsed_ms,
            answer_chars,
            answer_bytes: answer.len(),
            answer_approx_tokens: approximate_answer_tokens(answer_chars),
        }
    }
}

impl ModelPoolMaxTokensDecision {
    fn saved_tokens(&self) -> usize {
        self.configured_max_tokens
            .unwrap_or(self.effective_max_tokens)
            .saturating_sub(self.effective_max_tokens)
    }
}

fn approximate_answer_tokens(answer_chars: usize) -> usize {
    if answer_chars == 0 {
        0
    } else {
        (answer_chars + 3) / 4
    }
}

fn approximate_prompt_tokens(prompt: Option<&str>) -> usize {
    prompt
        .map(|prompt| approximate_answer_tokens(prompt.chars().count()))
        .unwrap_or(0)
}

fn route_context_buffer_tokens(task_kind: &str, completed_roles: Option<&[String]>) -> usize {
    route_context_buffer_policy(task_kind, completed_roles).total_tokens
}

fn is_test_gate_task_kind(task_kind: &str) -> bool {
    matches!(
        task_kind.trim().to_ascii_lowercase().as_str(),
        "test-gate" | "test_gate" | "testgate"
    )
}

fn route_context_buffer_policy(
    task_kind: &str,
    completed_roles: Option<&[String]>,
) -> ModelPoolRouteContextBufferPolicy {
    if !is_test_gate_task_kind(task_kind) {
        return ModelPoolRouteContextBufferPolicy {
            strategy: "none",
            base_tokens: 0,
            upstream_role_tokens: 0,
            eligible_upstream_roles: Vec::new(),
            completed_upstream_roles: Vec::new(),
            total_tokens: 0,
        };
    }
    let completed_upstream_roles = test_gate_completed_upstream_context_roles(completed_roles);
    let upstream_tokens = completed_upstream_roles
        .len()
        .saturating_mul(TEST_GATE_UPSTREAM_ROLE_CONTEXT_BUFFER_TOKENS);
    ModelPoolRouteContextBufferPolicy {
        strategy: "test_gate_dynamic_upstream_buffer_v1",
        base_tokens: TEST_GATE_BASE_CONTEXT_BUFFER_TOKENS,
        upstream_role_tokens: TEST_GATE_UPSTREAM_ROLE_CONTEXT_BUFFER_TOKENS,
        eligible_upstream_roles: TEST_GATE_UPSTREAM_CONTEXT_BUFFER_ROLES
            .iter()
            .copied()
            .map(str::to_owned)
            .collect(),
        completed_upstream_roles,
        total_tokens: TEST_GATE_BASE_CONTEXT_BUFFER_TOKENS.saturating_add(upstream_tokens),
    }
}

fn test_gate_completed_upstream_context_roles(completed_roles: Option<&[String]>) -> Vec<String> {
    let Some(completed_roles) = completed_roles else {
        return Vec::new();
    };
    TEST_GATE_UPSTREAM_CONTEXT_BUFFER_ROLES
        .into_iter()
        .filter(|role| {
            completed_roles
                .iter()
                .any(|completed_role| completed_role.eq_ignore_ascii_case(role))
        })
        .map(str::to_owned)
        .collect()
}

pub(crate) fn model_pool_route_context_decision(
    task_kind: &str,
    prompt: Option<&str>,
    completed_roles: Option<&[String]>,
    selected_context_window: Option<usize>,
    selected_token_budget: Option<&ModelPoolMaxTokensDecision>,
) -> ModelPoolRouteContextDecision {
    let (Some(selected_context_window), Some(selected_token_budget)) =
        (selected_context_window, selected_token_budget)
    else {
        return ModelPoolRouteContextDecision {
            selected_context_required_tokens: None,
            selected_context_buffer_tokens: None,
            selected_context_sufficient: false,
            selected_context_block_reason: "no_selected_worker",
        };
    };
    let buffer_tokens = route_context_buffer_tokens(task_kind, completed_roles);
    let required_tokens = approximate_prompt_tokens(prompt)
        .saturating_add(selected_token_budget.effective_max_tokens)
        .saturating_add(buffer_tokens);
    let selected_context_sufficient = selected_context_window >= required_tokens;
    ModelPoolRouteContextDecision {
        selected_context_required_tokens: Some(required_tokens),
        selected_context_buffer_tokens: Some(buffer_tokens),
        selected_context_sufficient,
        selected_context_block_reason: if selected_context_sufficient {
            "none"
        } else {
            "selected_context_window_too_small"
        },
    }
}

pub(crate) fn model_pool_max_tokens_decision(
    worker: &ModelPoolWorkerView,
    configured_max_tokens: Option<usize>,
) -> ModelPoolMaxTokensDecision {
    let configured_max_tokens = configured_max_tokens.map(|value| value.max(1));
    let Some(configured) = configured_max_tokens else {
        return ModelPoolMaxTokensDecision {
            configured_max_tokens,
            effective_max_tokens: worker.default_max_tokens.max(1),
            max_tokens_clamped: false,
            max_tokens_clamp_reason: "request_max_tokens_missing_used_worker_default",
        };
    };
    if worker_uses_default_max_tokens_as_limit(worker) && configured > worker.default_max_tokens {
        return ModelPoolMaxTokensDecision {
            configured_max_tokens,
            effective_max_tokens: worker.default_max_tokens.max(1),
            max_tokens_clamped: true,
            max_tokens_clamp_reason: "low_priority_worker_default_max_tokens",
        };
    }
    ModelPoolMaxTokensDecision {
        configured_max_tokens,
        effective_max_tokens: configured,
        max_tokens_clamped: false,
        max_tokens_clamp_reason: if worker.role == "quality" {
            "quality_worker_request_budget_preserved"
        } else if worker.low_priority {
            "request_within_low_priority_worker_default"
        } else {
            "selected_worker_request_budget_preserved"
        },
    }
}

fn worker_uses_default_max_tokens_as_limit(worker: &ModelPoolWorkerView) -> bool {
    worker.low_priority && worker.role != "quality"
}

pub(crate) fn model_pool_quality_gate(workers: &[ModelPoolWorkerView]) -> ModelPoolQualityGate {
    let quality_worker = workers.iter().find(|worker| worker.role == "quality");
    let quality_worker_count = workers
        .iter()
        .filter(|worker| worker.role == "quality")
        .count();
    let extra_quality_12b_detected = quality_worker_count > MODEL_POOL_MAX_QUALITY_12B_WORKERS;
    let quality_ready = quality_worker.is_some_and(ModelPoolWorkerView::ready);
    let quality_context_tokens = quality_worker.map(ModelPoolWorkerView::effective_context_tokens);
    let quality_context_required_tokens =
        quality_worker.map(|worker| worker.default_context_tokens);
    let quality_context_sufficient = match (quality_context_tokens, quality_context_required_tokens)
    {
        (Some(actual), Some(required)) => actual >= required,
        _ => false,
    };
    let launch_allowed = quality_ready && quality_context_sufficient && !extra_quality_12b_detected;
    let reason = if launch_allowed {
        "ready"
    } else if extra_quality_12b_detected {
        "extra_quality_12b_workers"
    } else if quality_worker.is_none() {
        "quality_worker_missing"
    } else if !quality_ready {
        "quality_worker_down"
    } else {
        "quality_context_window_too_small"
    };
    let quality_block_reason = if extra_quality_12b_detected {
        "extra_quality_12b_workers"
    } else if quality_ready && !quality_context_sufficient {
        "context_window_below_quality_default"
    } else {
        quality_worker
            .map(ModelPoolWorkerView::reason)
            .unwrap_or("quality_worker_missing")
    };
    ModelPoolQualityGate {
        launch_allowed,
        reason,
        quality_ready,
        quality_context_tokens,
        quality_context_required_tokens,
        quality_context_sufficient,
        quality_block_reason,
        quality_worker_count,
        extra_quality_12b_detected,
    }
}

pub(crate) fn model_pool_launch_block_reason(gate: &ModelPoolQualityGate) -> String {
    format!("model_pool_launch_blocked:{}", gate.reason)
}

fn model_pool_capacity_summary(
    workers: &[ModelPoolWorkerView],
    gate: &ModelPoolQualityGate,
) -> ModelPoolCapacitySummary {
    let healthy_worker_count = workers.iter().filter(|worker| worker.ready()).count();
    let failed_worker_count = workers.len().saturating_sub(healthy_worker_count);
    let helper_worker_count = workers
        .iter()
        .filter(|worker| worker.role != "quality")
        .count();
    let healthy_helper_worker_count = workers
        .iter()
        .filter(|worker| worker.role != "quality" && worker.ready())
        .count();
    let metal_worker_count = workers
        .iter()
        .filter(|worker| worker.ready() && worker_reports_metal(worker))
        .count();
    let cpu_worker_count = workers
        .iter()
        .filter(|worker| worker.ready() && worker_reports_cpu(worker))
        .count();
    let unknown_runtime_worker_count = workers
        .iter()
        .filter(|worker| worker.ready() && worker_runtime_unknown(worker))
        .count();
    let zero_gpu_layer_worker_count = workers
        .iter()
        .filter(|worker| worker.ready() && worker.gpu_layers == Some(0))
        .count();
    let quality_runtime_accelerated = workers
        .iter()
        .find(|worker| worker.role == "quality" && worker.ready())
        .and_then(worker_acceleration_state);
    let model_pool_saturation_milli =
        model_pool_saturation_milli(workers.len(), healthy_worker_count);
    let homeostatic_gate = HomeostaticSetpoints::default().evaluate(AllostaticLoadCounters {
        model_pool_saturation_milli,
        failed_model_workers: failed_worker_count,
        ..AllostaticLoadCounters::default()
    });
    let expansion_allowed = gate.launch_allowed
        && quality_runtime_accelerated != Some(false)
        && unknown_runtime_worker_count == 0
        && homeostatic_gate.model_cell_expansion_allowed;
    let recommendation = if gate.extra_quality_12b_detected {
        "stop_extra_quality_12b_workers_keep_one_quality_plus_helpers"
    } else if !gate.launch_allowed {
        "restore_quality_gate_first"
    } else if quality_runtime_accelerated == Some(false) {
        "fix_runtime_acceleration_before_adding_workers"
    } else if unknown_runtime_worker_count > 0 {
        "verify_worker_runtime_metadata_before_expansion"
    } else if !homeostatic_gate.model_cell_expansion_allowed {
        "restore_failed_model_workers_before_expansion"
    } else if cpu_worker_count > 0 || zero_gpu_layer_worker_count > 0 {
        "hold_cpu_helpers_for_memory_pressure"
    } else if healthy_helper_worker_count == 0 {
        "add_summary_worker_first"
    } else if healthy_helper_worker_count == 1 {
        "add_review_or_index_worker_after_short_smoke"
    } else if healthy_helper_worker_count < helper_worker_count {
        "restore_missing_helper_workers_before_more_concurrency"
    } else {
        "hold_or_add_optional_test_gate_if_memory_pressure_green"
    };
    ModelPoolCapacitySummary {
        worker_count: workers.len(),
        healthy_worker_count,
        failed_worker_count,
        helper_worker_count,
        healthy_helper_worker_count,
        metal_worker_count,
        cpu_worker_count,
        unknown_runtime_worker_count,
        zero_gpu_layer_worker_count,
        quality_runtime_accelerated,
        model_pool_saturation_milli,
        homeostatic_model_cell_expansion_allowed: homeostatic_gate.model_cell_expansion_allowed,
        homeostatic_decision: homeostatic_gate.decision.as_str(),
        expansion_allowed,
        recommendation,
    }
}

fn model_pool_saturation_milli(worker_count: usize, healthy_worker_count: usize) -> u16 {
    if worker_count == 0 {
        return 0;
    }

    let failed_worker_count = worker_count.saturating_sub(healthy_worker_count);
    ((failed_worker_count * 1000) / worker_count).min(1000) as u16
}

fn model_pool_status_advice(
    workers: &[ModelPoolWorkerView],
    gate: &ModelPoolQualityGate,
    capacity: &ModelPoolCapacitySummary,
) -> ModelPoolStatusAdvice {
    let helper_roles = visible_helper_roles(workers);
    let expected_helper_roles = expected_helper_roles();
    let facts = model_pool_facts(workers, gate, capacity, &helper_roles);
    let missing_helper_roles = missing_helper_roles(&facts)
        .into_iter()
        .map(str::to_owned)
        .collect();
    let recommended_launch_order = recommended_launch_order();
    let helper_worker_count = helper_roles.len();
    let decision = model_pool_decision(&facts);
    let helper_cpu_or_no_gpu_roles = facts.helper_cpu_or_no_gpu_roles.clone();
    let blocking_helper_cpu_or_no_gpu_roles = facts
        .blocking_helper_cpu_or_no_gpu_roles()
        .into_iter()
        .map(str::to_owned)
        .collect();
    let allowed_cpu_fallback_helper_roles = facts
        .allowed_cpu_fallback_helper_roles()
        .into_iter()
        .map(str::to_owned)
        .collect();
    ModelPoolStatusAdvice {
        safe_to_enable_pool_workers: decision.safe_to_enable_pool_workers,
        next_step: decision.next_step,
        reason: decision.reason,
        kind: decision.kind.as_str(),
        extra_quality_12b_detected: gate.extra_quality_12b_detected,
        quality_worker_count: gate.quality_worker_count,
        helper_worker_count,
        healthy_helper_worker_count: capacity.healthy_helper_worker_count,
        helper_roles,
        expected_helper_roles,
        missing_helper_roles,
        helper_cpu_or_no_gpu_roles,
        blocking_helper_cpu_or_no_gpu_roles,
        allowed_cpu_fallback_helper_roles,
        recommended_launch_order,
    }
}

fn model_pool_facts(
    workers: &[ModelPoolWorkerView],
    gate: &ModelPoolQualityGate,
    capacity: &ModelPoolCapacitySummary,
    helper_roles: &[String],
) -> ModelPoolFacts {
    ModelPoolFacts {
        quality_ready: Some(gate.quality_ready),
        quality_context_sufficient: Some(gate.quality_context_sufficient),
        quality_context_tokens: gate.quality_context_tokens.map(|value| value.to_string()),
        quality_required_context_tokens: gate
            .quality_context_required_tokens
            .map(|value| value.to_string()),
        quality_runtime_accelerated: capacity.quality_runtime_accelerated,
        capacity_recommendation: Some(capacity.recommendation.to_owned()),
        expansion_allowed: Some(capacity.expansion_allowed),
        healthy_helper_worker_count: Some(capacity.healthy_helper_worker_count),
        unknown_runtime_worker_count: Some(capacity.unknown_runtime_worker_count),
        has_summary: helper_role_visible(helper_roles, "summary"),
        has_router: helper_role_visible(helper_roles, "router"),
        has_review: helper_role_visible(helper_roles, "review"),
        has_index: helper_role_visible(helper_roles, "index"),
        has_test_gate: helper_role_visible(helper_roles, "test-gate"),
        quality_worker_count: gate.quality_worker_count,
        helper_worker_count: helper_roles.len(),
        quality_cpu_fallback: quality_worker_cpu_fallback(workers),
        quality_zero_gpu_layers: quality_worker_zero_gpu_layers(workers),
        helper_cpu_or_no_gpu_roles: helper_cpu_or_no_gpu_roles(workers),
    }
}

fn visible_helper_roles(workers: &[ModelPoolWorkerView]) -> Vec<String> {
    let discovered = workers
        .iter()
        .filter(|worker| worker.role != "quality")
        .map(|worker| worker.role.as_str())
        .collect::<Vec<_>>();
    MODEL_POOL_HELPER_ROLES
        .iter()
        .copied()
        .filter(|role| discovered.iter().any(|discovered| discovered == role))
        .map(str::to_owned)
        .collect()
}

fn recommended_launch_order() -> Vec<String> {
    RECOMMENDED_LAUNCH_ROLES
        .iter()
        .copied()
        .map(str::to_owned)
        .collect()
}

fn expected_helper_roles() -> Vec<String> {
    MODEL_POOL_HELPER_ROLES
        .iter()
        .copied()
        .map(str::to_owned)
        .collect()
}

fn helper_role_visible(helper_roles: &[String], role: &str) -> bool {
    helper_roles.iter().any(|helper_role| helper_role == role)
}

fn quality_worker_cpu_fallback(workers: &[ModelPoolWorkerView]) -> bool {
    workers
        .iter()
        .any(|worker| worker.role == "quality" && worker.ready() && worker_reports_cpu(worker))
}

fn quality_worker_zero_gpu_layers(workers: &[ModelPoolWorkerView]) -> bool {
    workers
        .iter()
        .any(|worker| worker.role == "quality" && worker.ready() && worker.gpu_layers == Some(0))
}

fn helper_cpu_or_no_gpu_roles(workers: &[ModelPoolWorkerView]) -> Vec<String> {
    MODEL_POOL_HELPER_ROLES
        .iter()
        .copied()
        .filter(|role| {
            workers.iter().any(|worker| {
                worker.role == *role
                    && worker.ready()
                    && worker_acceleration_state(worker) == Some(false)
            })
        })
        .map(str::to_owned)
        .collect()
}

fn worker_acceleration_state(worker: &ModelPoolWorkerView) -> Option<bool> {
    if worker_reports_metal(worker) || worker.gpu_layers.is_some_and(|layers| layers > 0) {
        Some(true)
    } else if worker_reports_cpu(worker) || worker.gpu_layers == Some(0) {
        Some(false)
    } else {
        None
    }
}

fn worker_reports_metal(worker: &ModelPoolWorkerView) -> bool {
    option_eq_ignore_ascii_case(worker.runtime_accelerator.as_deref(), "metal")
        || option_eq_ignore_ascii_case(worker.runtime_device.as_deref(), "metal")
}

fn worker_reports_cpu(worker: &ModelPoolWorkerView) -> bool {
    option_eq_ignore_ascii_case(worker.runtime_accelerator.as_deref(), "cpu")
        || option_eq_ignore_ascii_case(worker.runtime_device.as_deref(), "cpu")
}

fn worker_runtime_unknown(worker: &ModelPoolWorkerView) -> bool {
    worker.runtime_backend.is_none()
        && worker.runtime_device.is_none()
        && worker.runtime_accelerator.is_none()
        && worker.gpu_layers.is_none()
}

fn option_eq_ignore_ascii_case(value: Option<&str>, expected: &str) -> bool {
    value.is_some_and(|value| value.eq_ignore_ascii_case(expected))
}

#[cfg(test)]
fn model_service_model_pool_status_response_json(
    request_id: usize,
    workers: &[ModelPoolWorkerView],
) -> String {
    model_service_model_pool_status_response_json_with_metrics(request_id, workers, None)
}

pub(crate) fn model_service_model_pool_status_response_json_with_metrics(
    request_id: usize,
    workers: &[ModelPoolWorkerView],
    metrics: Option<&ModelPoolMetricsSnapshotView>,
) -> String {
    let quality_gate = model_pool_quality_gate(workers);
    let min_context_tokens = workers
        .iter()
        .filter(|worker| worker.ready())
        .map(ModelPoolWorkerView::effective_context_tokens)
        .min();
    let quality_default_context_tokens = quality_gate.quality_context_required_tokens;
    let quality_default_max_tokens = workers
        .iter()
        .find(|worker| worker.role == "quality")
        .map(|worker| worker.default_max_tokens);
    let capacity = model_pool_capacity_summary(workers, &quality_gate);
    let advice = model_pool_status_advice(workers, &quality_gate, &capacity);
    format!(
        "{{\"ok\":true,\"request_id\":{},\"schema_version\":1,\"contract_version\":\"model-pool.v1\",\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"launch_allowed\":{},\"reason\":{},\"launch_block_reason\":{},\"chain_classification\":{},\"min_context_tokens\":{},\"quality_ready\":{},\"quality_context_tokens\":{},\"quality_context_required_tokens\":{},\"quality_context_sufficient\":{},\"quality_default_context_tokens\":{},\"quality_default_max_tokens\":{},\"quality_block_reason\":{},\"quality_worker_count\":{},\"extra_quality_12b_detected\":{},\"blocked_policy\":{},\"capacity\":{},\"advice\":{},\"decision_source\":{},\"safe_to_enable_pool_workers\":{},\"next_step\":{},\"reason_detail\":{},\"helper_worker_count\":{},\"healthy_helper_worker_count\":{},\"helper_target_worker_count\":{},\"helper_roles\":{},\"expected_helper_roles\":{},\"missing_helper_roles\":{},\"helper_cpu_or_no_gpu_roles\":{},\"blocking_helper_cpu_or_no_gpu_roles\":{},\"allowed_cpu_fallback_helper_roles\":{},\"recommended_launch_order\":{},\"capacity_recommendation\":{},\"worker_shape\":{}{},\"workers\":{},\"worker_count\":{},\"healthy_worker_count\":{}}}",
        request_id,
        quality_gate.launch_allowed,
        service_json_string(quality_gate.reason),
        service_json_string(quality_gate.reason),
        service_json_string(quality_gate.reason),
        option_usize_service_json(min_context_tokens),
        quality_gate.quality_ready,
        option_usize_service_json(quality_gate.quality_context_tokens),
        option_usize_service_json(quality_gate.quality_context_required_tokens),
        quality_gate.quality_context_sufficient,
        option_usize_service_json(quality_default_context_tokens),
        option_usize_service_json(quality_default_max_tokens),
        service_json_string(quality_gate.quality_block_reason),
        quality_gate.quality_worker_count,
        quality_gate.extra_quality_12b_detected,
        service_json_string(
            "model-pool launch is blocked until the quality worker is reachable, healthy, and has the required context window"
        ),
        capacity_summary_json(&capacity),
        model_pool_status_advice_json(&advice),
        service_json_string(MODEL_POOL_ADVICE_SOURCE),
        advice.safe_to_enable_pool_workers,
        service_json_string(advice.next_step),
        service_json_string(advice.reason),
        advice.helper_worker_count,
        advice.healthy_helper_worker_count,
        MODEL_POOL_HELPER_TARGET_WORKERS,
        string_array_json(&advice.helper_roles),
        string_array_json(&advice.expected_helper_roles),
        string_array_json(&advice.missing_helper_roles),
        string_array_json(&advice.helper_cpu_or_no_gpu_roles),
        string_array_json(&advice.blocking_helper_cpu_or_no_gpu_roles),
        string_array_json(&advice.allowed_cpu_fallback_helper_roles),
        string_array_json(&advice.recommended_launch_order),
        service_json_string(advice.next_step),
        model_pool_status_worker_shape_json(&advice),
        metrics_fields_json(metrics),
        model_pool_workers_json_with_metrics(workers, metrics),
        workers.len(),
        workers.iter().filter(|worker| worker.ready()).count()
    )
}

#[cfg(test)]
fn model_service_model_pool_route_response_json(
    request_id: usize,
    task_kind: &str,
    configured_max_tokens: Option<usize>,
    workers: &[ModelPoolWorkerView],
) -> String {
    model_service_model_pool_route_response_json_with_metrics(
        request_id,
        task_kind,
        configured_max_tokens,
        workers,
        None,
    )
}

#[cfg(test)]
fn model_service_model_pool_route_response_json_with_metrics(
    request_id: usize,
    task_kind: &str,
    configured_max_tokens: Option<usize>,
    workers: &[ModelPoolWorkerView],
    metrics: Option<&ModelPoolMetricsSnapshotView>,
) -> String {
    model_service_model_pool_route_response_json_with_context(
        request_id,
        task_kind,
        configured_max_tokens,
        None,
        workers,
        None,
        metrics,
    )
}

pub(crate) fn model_service_model_pool_route_response_json_with_context(
    request_id: usize,
    task_kind: &str,
    configured_max_tokens: Option<usize>,
    prompt: Option<&str>,
    workers: &[ModelPoolWorkerView],
    completed_roles: Option<&[String]>,
    metrics: Option<&ModelPoolMetricsSnapshotView>,
) -> String {
    model_service_model_pool_route_response_json_with_context_and_backpressure(
        request_id,
        task_kind,
        configured_max_tokens,
        prompt,
        workers,
        completed_roles,
        metrics,
        None,
    )
}

pub(crate) fn model_service_model_pool_route_response_json_with_context_and_backpressure(
    request_id: usize,
    task_kind: &str,
    configured_max_tokens: Option<usize>,
    prompt: Option<&str>,
    workers: &[ModelPoolWorkerView],
    completed_roles: Option<&[String]>,
    metrics: Option<&ModelPoolMetricsSnapshotView>,
    service_backpressure: Option<&ModelPoolServiceBackpressureView>,
) -> String {
    let (role_candidates, routing_weights) = model_pool_route_candidates_for_context(
        task_kind,
        configured_max_tokens,
        prompt,
        workers,
        metrics,
    );
    let quality_gate = model_pool_quality_gate(workers);
    let resource_precheck_allowed = routing_weights.resource_precheck.allow_dispatch;
    let selected_candidate = if quality_gate.launch_allowed && resource_precheck_allowed {
        role_candidates.iter().find_map(|role| {
            workers
                .iter()
                .find(|worker| worker.role == *role && worker.ready())
        })
    } else {
        None
    };
    let dependency_precheck = model_pool_dependency_precheck(
        selected_candidate
            .map(|worker| worker.role.as_str())
            .unwrap_or(task_kind),
        completed_roles,
    );
    let selected = if dependency_precheck.allow_dispatch {
        selected_candidate
    } else {
        None
    };
    let selected_role = selected.map(|worker| worker.role.as_str());
    let selected_base_url = selected.map(|worker| worker.base_url.as_str());
    let selected_port = selected.map(|worker| worker.port as usize);
    let selected_default_max_tokens = selected.map(|worker| worker.default_max_tokens);
    let selected_context_window = selected.map(|worker| {
        worker
            .context_window
            .unwrap_or(worker.default_context_tokens)
    });
    let selected_token_budget =
        selected.map(|worker| model_pool_max_tokens_decision(worker, configured_max_tokens));
    let selected_context_decision = model_pool_route_context_decision(
        task_kind,
        prompt,
        completed_roles,
        selected_context_window,
        selected_token_budget.as_ref(),
    );
    let selected_context_buffer_policy = route_context_buffer_policy(task_kind, completed_roles);
    let service_backpressure_allowed = service_backpressure
        .map(|backpressure| backpressure.allow_dispatch())
        .unwrap_or(true);
    let route_allowed = quality_gate.launch_allowed
        && resource_precheck_allowed
        && service_backpressure_allowed
        && dependency_precheck.allow_dispatch
        && selected.is_some()
        && selected_context_decision.selected_context_sufficient;
    let reason = if !quality_gate.launch_allowed {
        model_pool_launch_block_reason(&quality_gate)
    } else if !resource_precheck_allowed {
        format!(
            "resource_precheck_blocked:{}",
            routing_weights.resource_precheck.reason
        )
    } else if !service_backpressure_allowed {
        format!(
            "service_backpressure_blocked:{}",
            service_backpressure
                .map(|backpressure| backpressure.reason())
                .unwrap_or("unknown")
        )
    } else if !dependency_precheck.allow_dispatch {
        format!("dependency_precheck_blocked:{}", dependency_precheck.reason)
    } else if selected.is_none() {
        "no_ready_candidate".to_owned()
    } else if !selected_context_decision.selected_context_sufficient {
        selected_context_decision
            .selected_context_block_reason
            .to_owned()
    } else {
        "ready".to_owned()
    };
    let (
        compute_budget_summary,
        compute_budget_configured_max_tokens,
        compute_budget_effective_max_tokens,
        compute_budget_saved_tokens,
        compute_budget_max_tokens_clamped,
    ) = selected
        .zip(selected_token_budget.as_ref())
        .map(|(worker, budget)| {
            let saved_tokens = budget.saved_tokens();
            (
                service_json_string(&format!(
                    "model_pool_route_plan selected_role={} effective_max_tokens={} saved_tokens={} max_tokens_clamped={}",
                    worker.role,
                    budget.effective_max_tokens,
                    saved_tokens,
                    budget.max_tokens_clamped
                )),
                option_usize_service_json(budget.configured_max_tokens),
                budget.effective_max_tokens.to_string(),
                saved_tokens.to_string(),
                budget.max_tokens_clamped,
            )
        })
        .unwrap_or_else(|| {
            (
                service_json_string("model_pool_route_plan unavailable_no_selected_worker"),
                "null".to_owned(),
                "null".to_owned(),
                "0".to_owned(),
                false,
            )
        });
    format!(
        "{{\"ok\":true,\"request_id\":{},\"schema_version\":1,\"contract_version\":\"model-pool.v1\",\"task_kind\":{},\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"route_allowed\":{},\"reason\":{},\"route_block_reason\":{},\"role_candidates\":{},\"routing_weights\":{},\"service_backpressure\":{},\"dependency_precheck\":{},\"quality_context_tokens\":{},\"quality_context_required_tokens\":{},\"quality_context_sufficient\":{},\"quality_block_reason\":{},\"selected_role\":{},\"selected_base_url\":{},\"selected_port\":{},\"selected_default_max_tokens\":{},\"selected_context_window\":{},\"selected_context_required_tokens\":{},\"selected_context_buffer_tokens\":{},\"selected_context_buffer_policy\":{},\"selected_context_sufficient\":{},\"selected_context_block_reason\":{},\"configured_max_tokens\":{},\"effective_max_tokens\":{},\"max_tokens_clamped\":{},\"max_tokens_clamp_reason\":{},\"compute_budget_summary\":{},\"compute_budget_configured_max_tokens\":{},\"compute_budget_effective_max_tokens\":{},\"compute_budget_saved_tokens\":{},\"compute_budget_avoided_tokens\":{},\"compute_budget_max_tokens_clamped\":{},\"pool_dispatch\":{}{},\"candidate_workers\":{}}}",
        request_id,
        service_json_string(task_kind),
        route_allowed,
        service_json_string(&reason),
        service_json_string(&reason),
        string_array_json(&role_candidates),
        routing_weights_json(&routing_weights),
        service_backpressure_json(service_backpressure),
        dependency_precheck_json(&dependency_precheck),
        option_usize_service_json(quality_gate.quality_context_tokens),
        option_usize_service_json(quality_gate.quality_context_required_tokens),
        quality_gate.quality_context_sufficient,
        service_json_string(quality_gate.quality_block_reason),
        option_str_service_json(selected_role),
        option_str_service_json(selected_base_url),
        selected_port
            .map(|port| port.to_string())
            .unwrap_or_else(|| "null".to_owned()),
        selected_default_max_tokens
            .map(|tokens| tokens.to_string())
            .unwrap_or_else(|| "null".to_owned()),
        option_usize_service_json(selected_context_window),
        option_usize_service_json(selected_context_decision.selected_context_required_tokens),
        option_usize_service_json(selected_context_decision.selected_context_buffer_tokens),
        route_context_buffer_policy_json(&selected_context_buffer_policy),
        selected_context_decision.selected_context_sufficient,
        service_json_string(selected_context_decision.selected_context_block_reason),
        option_usize_service_json(configured_max_tokens),
        option_usize_service_json(
            selected_token_budget
                .as_ref()
                .map(|budget| { budget.effective_max_tokens })
        ),
        selected_token_budget
            .as_ref()
            .map(|budget| budget.max_tokens_clamped)
            .unwrap_or(false),
        service_json_string(
            selected_token_budget
                .as_ref()
                .map(|budget| budget.max_tokens_clamp_reason)
                .unwrap_or("no_selected_worker")
        ),
        compute_budget_summary,
        compute_budget_configured_max_tokens,
        compute_budget_effective_max_tokens,
        compute_budget_saved_tokens,
        compute_budget_saved_tokens,
        compute_budget_max_tokens_clamped,
        if route_allowed {
            selected
                .zip(selected_token_budget.as_ref())
                .map(|(worker, budget)| model_pool_dispatch_json(worker, budget))
                .unwrap_or_else(|| "null".to_owned())
        } else {
            "null".to_owned()
        },
        metrics_fields_json(metrics),
        model_pool_workers_json_with_metrics(workers, metrics)
    )
}

fn service_backpressure_json(
    service_backpressure: Option<&ModelPoolServiceBackpressureView>,
) -> String {
    let Some(backpressure) = service_backpressure else {
        return "null".to_owned();
    };
    format!(
        "{{\"strategy\":\"service_stream_backpressure_v1\",\"active_engine_requests\":{},\"max_active_stream_engine_requests\":{},\"stream_backpressure_rejections\":{},\"pressure\":\"{}\",\"allow_dispatch\":{},\"reason\":\"{}\",\"read_only\":true}}",
        backpressure.active_engine_requests,
        backpressure.max_active_stream_engine_requests,
        backpressure.stream_backpressure_rejections,
        backpressure.pressure(),
        backpressure.allow_dispatch(),
        backpressure.reason()
    )
}

#[cfg(test)]
fn model_service_model_pool_call_response_json(
    request_id: usize,
    task_kind: &str,
    worker: &ModelPoolWorkerView,
    token_budget: &ModelPoolMaxTokensDecision,
    prompt_sent: bool,
    answer: &str,
) -> String {
    model_service_model_pool_call_response_json_with_metrics(
        request_id,
        task_kind,
        worker,
        token_budget,
        prompt_sent,
        answer,
        &ModelPoolCallExecutionView::from_answer(0, answer),
        None,
    )
}

pub(crate) fn model_service_model_pool_call_response_json_with_metrics(
    request_id: usize,
    task_kind: &str,
    worker: &ModelPoolWorkerView,
    token_budget: &ModelPoolMaxTokensDecision,
    prompt_sent: bool,
    answer: &str,
    execution: &ModelPoolCallExecutionView,
    metrics: Option<&ModelPoolMetricsSnapshotView>,
) -> String {
    let saved_tokens = token_budget.saved_tokens();
    format!(
        "{{\"ok\":true,\"request_id\":{},\"schema_version\":1,\"contract_version\":\"model-pool.v1\",\"task_kind\":{},\"read_only\":false,\"launches_process\":false,\"sends_prompt\":{},\"elapsed_ms\":{},\"answer_chars\":{},\"answer_bytes\":{},\"answer_approx_tokens\":{},\"selected_role\":{},\"selected_base_url\":{},\"selected_port\":{},\"selected_default_max_tokens\":{},\"configured_max_tokens\":{},\"effective_max_tokens\":{},\"max_tokens_clamped\":{},\"max_tokens_clamp_reason\":{},\"compute_budget_summary\":{},\"compute_budget_configured_max_tokens\":{},\"compute_budget_effective_max_tokens\":{},\"compute_budget_saved_tokens\":{},\"compute_budget_avoided_tokens\":{},\"compute_budget_max_tokens_clamped\":{},\"pool_dispatch\":{}{},\"answer\":{}}}",
        request_id,
        service_json_string(task_kind),
        prompt_sent,
        execution.elapsed_ms,
        execution.answer_chars,
        execution.answer_bytes,
        execution.answer_approx_tokens,
        service_json_string(&worker.role),
        service_json_string(&worker.base_url),
        worker.port as usize,
        worker.default_max_tokens,
        option_usize_service_json(token_budget.configured_max_tokens),
        token_budget.effective_max_tokens,
        token_budget.max_tokens_clamped,
        service_json_string(token_budget.max_tokens_clamp_reason),
        service_json_string(&format!(
            "model_pool_call selected_role={} effective_max_tokens={} saved_tokens={} max_tokens_clamped={}",
            worker.role,
            token_budget.effective_max_tokens,
            saved_tokens,
            token_budget.max_tokens_clamped
        )),
        option_usize_service_json(token_budget.configured_max_tokens),
        token_budget.effective_max_tokens,
        saved_tokens,
        saved_tokens,
        token_budget.max_tokens_clamped,
        model_pool_dispatch_json(worker, token_budget),
        metrics_fields_json(metrics),
        service_json_string(answer)
    )
}

#[cfg(test)]
fn model_service_model_pool_call_blocked_response_json(
    request_id: usize,
    task_kind: &str,
    reason: &str,
    workers: &[ModelPoolWorkerView],
) -> String {
    model_service_model_pool_call_blocked_response_json_with_metrics(
        request_id, task_kind, reason, workers, None,
    )
}

pub(crate) fn model_service_model_pool_call_blocked_response_json_with_metrics(
    request_id: usize,
    task_kind: &str,
    reason: &str,
    workers: &[ModelPoolWorkerView],
    metrics: Option<&ModelPoolMetricsSnapshotView>,
) -> String {
    model_service_model_pool_call_blocked_response_json_with_metrics_and_dependency(
        request_id, task_kind, reason, workers, metrics, None,
    )
}

pub(crate) fn model_service_model_pool_call_blocked_response_json_with_metrics_and_dependency(
    request_id: usize,
    task_kind: &str,
    reason: &str,
    workers: &[ModelPoolWorkerView],
    metrics: Option<&ModelPoolMetricsSnapshotView>,
    dependency_precheck: Option<&ModelPoolDependencyPrecheckView>,
) -> String {
    let role_candidates = model_pool_route_candidates(task_kind, workers);
    let quality_gate = model_pool_quality_gate(workers);
    let dependency_precheck = dependency_precheck
        .map(|precheck| {
            format!(
                ",\"dependency_precheck\":{}",
                dependency_precheck_json(precheck)
            )
        })
        .unwrap_or_default();
    format!(
        "{{\"ok\":false,\"request_id\":{},\"schema_version\":1,\"contract_version\":\"model-pool.v1\",\"task_kind\":{},\"read_only\":false,\"launches_process\":false,\"sends_prompt\":false,\"route_allowed\":false,\"reason\":{},\"route_block_reason\":{},\"role_candidates\":{},\"quality_context_tokens\":{},\"quality_context_required_tokens\":{},\"quality_context_sufficient\":{},\"quality_block_reason\":{}{}{},\"candidate_workers\":{}}}",
        request_id,
        service_json_string(task_kind),
        service_json_string(reason),
        service_json_string(reason),
        string_array_json(&role_candidates),
        option_usize_service_json(quality_gate.quality_context_tokens),
        option_usize_service_json(quality_gate.quality_context_required_tokens),
        quality_gate.quality_context_sufficient,
        service_json_string(quality_gate.quality_block_reason),
        dependency_precheck,
        metrics_fields_json(metrics),
        model_pool_workers_json_with_metrics(workers, metrics)
    )
}

fn model_pool_workers_json_with_metrics(
    workers: &[ModelPoolWorkerView],
    metrics: Option<&ModelPoolMetricsSnapshotView>,
) -> String {
    let items = workers
        .iter()
        .map(|worker| {
            model_pool_worker_json(worker, worker_metrics_for_role(metrics, &worker.role))
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

fn model_pool_worker_json(
    worker: &ModelPoolWorkerView,
    metrics: Option<&ModelPoolWorkerMetricsView>,
) -> String {
    let context_window = worker
        .context_window
        .unwrap_or(worker.default_context_tokens);
    format!(
        "{{\"role\":\"{}\",\"port\":{},\"base_url\":{},\"enabled_by_default\":{},\"model_class\":{},\"suggested_quant\":{},\"default_context_tokens\":{},\"default_max_tokens\":{},\"low_priority\":{},\"can_accept_low_priority_task\":{},\"status\":\"{}\",\"ready\":{},\"role_ready\":{},\"reachable\":{},\"tcp_reachable\":{},\"health_ok\":{},\"role_block_reason\":\"{}\",\"model\":{},\"context_window\":{},\"runtime_backend\":{},\"runtime_device\":{},\"runtime_accelerator\":{},\"gpu_layers\":{},\"error\":{}{} }}",
        worker.role,
        worker.port,
        service_json_string(&worker.base_url),
        worker.enabled_by_default,
        service_json_string(&worker.model_class),
        service_json_string(&worker.suggested_quant),
        worker.default_context_tokens,
        worker.default_max_tokens,
        worker.low_priority,
        worker.low_priority,
        worker.status(),
        worker.ready(),
        worker.ready(),
        worker.reachable,
        worker.reachable,
        worker.ready(),
        worker.reason(),
        option_str_service_json(worker.model.as_deref()),
        option_usize_service_json(Some(context_window)),
        option_str_service_json(worker.runtime_backend.as_deref()),
        option_str_service_json(worker.runtime_device.as_deref()),
        option_str_service_json(worker.runtime_accelerator.as_deref()),
        option_usize_service_json(worker.gpu_layers),
        option_str_service_json(worker.error.as_deref()),
        metrics
            .map(|metrics| metric_object_fields_json(&metrics.metrics))
            .unwrap_or_default()
    )
}

fn metrics_fields_json(metrics: Option<&ModelPoolMetricsSnapshotView>) -> String {
    metrics
        .map(|metrics| {
            format!(
                ",\"route_metrics\":{},\"worker_metrics\":{}",
                metrics_object_json(&metrics.route_metrics),
                worker_metrics_array_json(&metrics.worker_metrics)
            )
        })
        .unwrap_or_default()
}

fn worker_metrics_array_json(metrics: &[ModelPoolWorkerMetricsView]) -> String {
    let items = metrics
        .iter()
        .map(|worker| {
            format!(
                "{{\"role\":{}{} }}",
                service_json_string(&worker.role),
                metric_object_fields_json(&worker.metrics)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

fn metrics_object_json(metrics: &ModelPoolMetricsView) -> String {
    format!("{{{} }}", metric_object_body_json(metrics))
}

fn metric_object_fields_json(metrics: &ModelPoolMetricsView) -> String {
    format!(",{}", metric_object_body_json(metrics))
}

fn metric_object_body_json(metrics: &ModelPoolMetricsView) -> String {
    format!(
        "\"route_count\":{},\"selected_count\":{},\"blocked_count\":{},\"in_flight\":{},\"queued_count\":{},\"lease_wait_ms\":{},\"lease_wait_p95_ms\":{},\"success_count\":{},\"failure_count\":{},\"avg_latency_ms\":{},\"latency_p50_ms\":{},\"latency_p95_ms\":{}",
        metrics.route_count,
        metrics.selected_count,
        metrics.blocked_count,
        metrics.in_flight,
        metrics.queued_count,
        option_u64_json(metrics.lease_wait_ms),
        option_u64_json(metrics.lease_wait_p95_ms),
        metrics.success_count,
        metrics.failure_count,
        option_u64_json(metrics.avg_latency_ms),
        option_u64_json(metrics.latency_p50_ms),
        option_u64_json(metrics.latency_p95_ms)
    )
}

fn capacity_summary_json(summary: &ModelPoolCapacitySummary) -> String {
    format!(
        "{{\"policy\":{},\"expansion_allowed\":{},\"recommendation\":{},\"worker_count\":{},\"healthy_worker_count\":{},\"failed_worker_count\":{},\"helper_worker_count\":{},\"healthy_helper_worker_count\":{},\"metal_worker_count\":{},\"cpu_worker_count\":{},\"unknown_runtime_worker_count\":{},\"zero_gpu_layer_worker_count\":{},\"quality_runtime_accelerated\":{},\"model_pool_saturation_milli\":{},\"homeostatic_model_cell_expansion_allowed\":{},\"homeostatic_decision\":{}}}",
        service_json_string(MODEL_POOL_CAPACITY_POLICY),
        summary.expansion_allowed,
        service_json_string(summary.recommendation),
        summary.worker_count,
        summary.healthy_worker_count,
        summary.failed_worker_count,
        summary.helper_worker_count,
        summary.healthy_helper_worker_count,
        summary.metal_worker_count,
        summary.cpu_worker_count,
        summary.unknown_runtime_worker_count,
        summary.zero_gpu_layer_worker_count,
        option_bool_json(summary.quality_runtime_accelerated),
        summary.model_pool_saturation_milli,
        summary.homeostatic_model_cell_expansion_allowed,
        service_json_string(summary.homeostatic_decision)
    )
}

fn route_context_buffer_policy_json(policy: &ModelPoolRouteContextBufferPolicy) -> String {
    format!(
        "{{\"strategy\":{},\"base_tokens\":{},\"upstream_role_tokens\":{},\"eligible_upstream_roles\":{},\"completed_upstream_roles\":{},\"total_tokens\":{}}}",
        service_json_string(policy.strategy),
        policy.base_tokens,
        policy.upstream_role_tokens,
        string_array_json(&policy.eligible_upstream_roles),
        string_array_json(&policy.completed_upstream_roles),
        policy.total_tokens
    )
}

fn model_pool_status_advice_json(advice: &ModelPoolStatusAdvice) -> String {
    format!(
        "{{\"decision_source\":{},\"policy\":{},\"safe_to_enable_pool_workers\":{},\"next_step\":{},\"reason\":{},\"kind\":{},\"extra_quality_12b_detected\":{},\"avoid_extra_12b\":true,\"max_quality_12b_workers\":{},\"quality_worker_count\":{},\"helper_worker_count\":{},\"healthy_helper_worker_count\":{},\"helper_target_worker_count\":{},\"helper_roles\":{},\"expected_helper_roles\":{},\"missing_helper_roles\":{},\"helper_cpu_or_no_gpu_roles\":{},\"blocking_helper_cpu_or_no_gpu_roles\":{},\"allowed_cpu_fallback_helper_roles\":{},\"recommended_launch_order\":{},\"worker_shape\":{}}}",
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
        advice.healthy_helper_worker_count,
        MODEL_POOL_HELPER_TARGET_WORKERS,
        string_array_json(&advice.helper_roles),
        string_array_json(&advice.expected_helper_roles),
        string_array_json(&advice.missing_helper_roles),
        string_array_json(&advice.helper_cpu_or_no_gpu_roles),
        string_array_json(&advice.blocking_helper_cpu_or_no_gpu_roles),
        string_array_json(&advice.allowed_cpu_fallback_helper_roles),
        string_array_json(&advice.recommended_launch_order),
        model_pool_status_worker_shape_json(advice)
    )
}

fn model_pool_status_worker_shape_json(advice: &ModelPoolStatusAdvice) -> String {
    format!(
        "{{\"quality\":{},\"helpers_visible\":{},\"helpers_healthy\":{},\"helper_target\":{}}}",
        advice.quality_worker_count,
        advice.helper_worker_count,
        advice.healthy_helper_worker_count,
        MODEL_POOL_HELPER_TARGET_WORKERS
    )
}

fn worker_metrics_for_role<'a>(
    metrics: Option<&'a ModelPoolMetricsSnapshotView>,
    role: &str,
) -> Option<&'a ModelPoolWorkerMetricsView> {
    metrics?
        .worker_metrics
        .iter()
        .find(|worker| worker.role == role)
}

fn option_u64_json(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

fn option_bool_json(value: Option<bool>) -> String {
    value
        .map(|value| if value { "true" } else { "false" }.to_owned())
        .unwrap_or_else(|| "null".to_owned())
}

pub(crate) fn model_pool_route_candidates(
    task_kind: &str,
    workers: &[ModelPoolWorkerView],
) -> Vec<String> {
    model_pool_route_candidates_for_budget(task_kind, None, workers)
}

pub(crate) fn model_pool_route_candidates_for_budget(
    task_kind: &str,
    configured_max_tokens: Option<usize>,
    workers: &[ModelPoolWorkerView],
) -> Vec<String> {
    let normalized = match task_kind.trim().to_ascii_lowercase().as_str() {
        "spare" => "index".to_owned(),
        other => other.to_owned(),
    };
    if workers.iter().any(|worker| worker.role == normalized) {
        return vec![normalized];
    }
    match normalized.as_str() {
        "primary" | "chat" | "generate" | "generation" | "business-cycle" | "business_cycle" => {
            vec!["quality".to_owned()]
        }
        "auto" | "" if configured_budget_needs_quality_worker(configured_max_tokens, workers) => {
            vec![
                "quality".to_owned(),
                "summary".to_owned(),
                "router".to_owned(),
                "review".to_owned(),
                "test-gate".to_owned(),
                "index".to_owned(),
            ]
        }
        "summary" => vec!["summary".to_owned()],
        "router" | "tool-call" | "function-call" | "preflight" => {
            vec!["router".to_owned(), "summary".to_owned()]
        }
        "review" => vec!["review".to_owned()],
        "test-gate" => vec!["test-gate".to_owned(), "review".to_owned()],
        "index" => vec!["index".to_owned(), "summary".to_owned()],
        "quality" => vec!["quality".to_owned()],
        _ => vec![
            "summary".to_owned(),
            "router".to_owned(),
            "review".to_owned(),
            "test-gate".to_owned(),
            "index".to_owned(),
        ],
    }
}

pub(crate) fn model_pool_route_candidates_for_context(
    task_kind: &str,
    configured_max_tokens: Option<usize>,
    prompt: Option<&str>,
    workers: &[ModelPoolWorkerView],
    metrics: Option<&ModelPoolMetricsSnapshotView>,
) -> (
    Vec<String>,
    super::model_pool_routing::ModelPoolRoutingWeightsView,
) {
    let base_candidates =
        model_pool_route_candidates_for_budget(task_kind, configured_max_tokens, workers);
    model_pool_route_candidates_with_weights(
        task_kind,
        configured_max_tokens,
        prompt,
        workers,
        metrics,
        base_candidates,
    )
}

fn configured_budget_needs_quality_worker(
    configured_max_tokens: Option<usize>,
    workers: &[ModelPoolWorkerView],
) -> bool {
    let Some(configured_max_tokens) = configured_max_tokens else {
        return false;
    };
    let helper_limit = workers
        .iter()
        .filter(|worker| worker.low_priority && worker.role != "quality")
        .map(|worker| worker.default_max_tokens)
        .max()
        .unwrap_or(0);
    helper_limit > 0 && configured_max_tokens > helper_limit
}

fn model_pool_dispatch_json(
    worker: &ModelPoolWorkerView,
    token_budget: &ModelPoolMaxTokensDecision,
) -> String {
    let context_window = worker
        .context_window
        .unwrap_or(worker.default_context_tokens);
    format!(
        "{{\"selected_role\":{},\"selected_port\":{},\"selected_base_url\":{},\"context_window\":{},\"default_max_tokens\":{},\"runtime_backend\":{},\"runtime_device\":{},\"runtime_accelerator\":{},\"gpu_layers\":{},\"configured_max_tokens\":{},\"effective_max_tokens\":{},\"max_tokens_clamped\":{},\"max_tokens_clamp_reason\":{},\"can_accept_low_priority_task\":{}}}",
        service_json_string(&worker.role),
        worker.port as usize,
        service_json_string(&worker.base_url),
        context_window,
        worker.default_max_tokens,
        option_str_service_json(worker.runtime_backend.as_deref()),
        option_str_service_json(worker.runtime_device.as_deref()),
        option_str_service_json(worker.runtime_accelerator.as_deref()),
        option_usize_service_json(worker.gpu_layers),
        option_usize_service_json(token_budget.configured_max_tokens),
        token_budget.effective_max_tokens,
        token_budget.max_tokens_clamped,
        service_json_string(token_budget.max_tokens_clamp_reason),
        worker.low_priority,
    )
}

fn string_array_json(values: &[String]) -> String {
    let items = values
        .iter()
        .map(|value| service_json_string(value))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workers() -> Vec<ModelPoolWorkerView> {
        vec![
            ModelPoolWorkerView {
                role: "quality".to_owned(),
                port: 8686,
                base_url: "http://127.0.0.1:8686".to_owned(),
                enabled_by_default: true,
                model_class: "Gemma 12B".to_owned(),
                suggested_quant: "Q8".to_owned(),
                default_context_tokens: 262_144,
                default_max_tokens: 262_144,
                low_priority: false,
                reachable: true,
                model: Some("gemma".to_owned()),
                context_window: Some(8192),
                runtime_backend: Some("llama.cpp".to_owned()),
                runtime_device: Some("metal".to_owned()),
                runtime_accelerator: Some("metal".to_owned()),
                gpu_layers: Some(99),
                error: None,
            },
            ModelPoolWorkerView {
                role: "index".to_owned(),
                port: 8690,
                base_url: "http://127.0.0.1:8690".to_owned(),
                enabled_by_default: true,
                model_class: "small index helper".to_owned(),
                suggested_quant: "Q4".to_owned(),
                default_context_tokens: 4096,
                default_max_tokens: 512,
                low_priority: true,
                reachable: true,
                model: Some("gemma-index".to_owned()),
                context_window: Some(4096),
                runtime_backend: None,
                runtime_device: None,
                runtime_accelerator: None,
                gpu_layers: None,
                error: None,
            },
            ModelPoolWorkerView {
                role: "review".to_owned(),
                port: 8688,
                base_url: "http://127.0.0.1:8688".to_owned(),
                enabled_by_default: true,
                model_class: "small Gemma".to_owned(),
                suggested_quant: "Q4".to_owned(),
                default_context_tokens: 8192,
                default_max_tokens: 1536,
                low_priority: true,
                reachable: false,
                model: None,
                context_window: None,
                runtime_backend: None,
                runtime_device: None,
                runtime_accelerator: None,
                gpu_layers: None,
                error: Some("tcp_unreachable".to_owned()),
            },
        ]
    }

    fn full_context_workers() -> Vec<ModelPoolWorkerView> {
        let mut workers = workers();
        workers[0].context_window = Some(262_144);
        workers
    }

    fn duplicate_quality_workers() -> Vec<ModelPoolWorkerView> {
        let mut workers = full_context_workers();
        workers.push(ModelPoolWorkerView {
            role: "quality".to_owned(),
            port: 9696,
            base_url: "http://127.0.0.1:9696".to_owned(),
            enabled_by_default: true,
            model_class: "Gemma 12B duplicate".to_owned(),
            suggested_quant: "Q8".to_owned(),
            default_context_tokens: 262_144,
            default_max_tokens: 262_144,
            low_priority: false,
            reachable: true,
            model: Some("gemma-duplicate".to_owned()),
            context_window: Some(262_144),
            runtime_backend: Some("llama.cpp".to_owned()),
            runtime_device: Some("metal".to_owned()),
            runtime_accelerator: Some("metal".to_owned()),
            gpu_layers: Some(99),
            error: None,
        });
        workers
    }

    fn test_gate_worker(context_window: usize) -> ModelPoolWorkerView {
        ModelPoolWorkerView {
            role: "test-gate".to_owned(),
            port: 8688,
            base_url: "http://127.0.0.1:8688".to_owned(),
            enabled_by_default: true,
            model_class: "Gemma small test gate".to_owned(),
            suggested_quant: "Q4".to_owned(),
            default_context_tokens: context_window,
            default_max_tokens: 1536,
            low_priority: true,
            reachable: true,
            model: Some("test-gate.gguf".to_owned()),
            context_window: Some(context_window),
            runtime_backend: Some("llama.cpp".to_owned()),
            runtime_device: Some("cpu".to_owned()),
            runtime_accelerator: Some("accelerate".to_owned()),
            gpu_layers: Some(0),
            error: None,
        }
    }

    #[test]
    fn status_json_is_read_only_and_launch_safe() {
        let json = model_service_model_pool_status_response_json(3, &workers());

        assert!(json.contains("\"read_only\":true"));
        assert!(json.contains("\"launches_process\":false"));
        assert!(json.contains("\"sends_prompt\":false"));
        assert!(json.contains("\"launch_allowed\":false"));
        assert!(json.contains("\"launch_block_reason\":\"quality_context_window_too_small\""));
        assert!(json.contains("\"chain_classification\":\"quality_context_window_too_small\""));
        assert!(json.contains("\"min_context_tokens\":4096"));
        assert!(json.contains("\"quality_ready\":true"));
        assert!(json.contains("\"quality_context_tokens\":8192"));
        assert!(json.contains("\"quality_context_required_tokens\":262144"));
        assert!(json.contains("\"quality_context_sufficient\":false"));
        assert!(json.contains("\"quality_default_context_tokens\":262144"));
        assert!(json.contains("\"quality_default_max_tokens\":262144"));
        assert!(json.contains("\"quality_block_reason\":\"context_window_below_quality_default\""));
        assert!(json.contains("\"quality_worker_count\":1"));
        assert!(json.contains("\"extra_quality_12b_detected\":false"));
        assert!(json.contains("\"capacity\""));
        assert!(json.contains("\"policy\":\"one_quality_plus_small_helpers\""));
        assert!(json.contains("\"expansion_allowed\":false"));
        assert!(json.contains("\"recommendation\":\"restore_quality_gate_first\""));
        assert!(json.contains("\"advice\""));
        assert!(json.contains("\"decision_source\":\"model-pool-advice-core\""));
        assert!(json.contains("\"policy\":\"one_quality_12b_plus_small_helpers\""));
        assert!(json.contains("\"safe_to_enable_pool_workers\":false"));
        assert!(json.contains("\"next_step\":\"restart_quality_with_required_context_tokens\""));
        assert!(json.contains("\"reason\":\"quality_context_window_insufficient\""));
        assert!(json.contains("\"reason_detail\":\"quality_context_window_insufficient\""));
        assert!(json.contains("\"helper_target_worker_count\":5"));
        assert!(json.contains(
            "\"expected_helper_roles\":[\"summary\",\"router\",\"review\",\"index\",\"test-gate\"]"
        ));
        assert!(json.contains("\"missing_helper_roles\":[\"summary\",\"router\",\"test-gate\"]"));
        assert!(json.contains("\"helper_cpu_or_no_gpu_roles\":[]"));
        assert!(json.contains("\"blocking_helper_cpu_or_no_gpu_roles\":[]"));
        assert!(json.contains("\"allowed_cpu_fallback_helper_roles\":[]"));
        assert!(json.contains(
            "\"recommended_launch_order\":[\"quality\",\"summary\",\"router\",\"review\",\"index\",\"test-gate\"]"
        ));
        assert!(json.contains(
            "\"worker_shape\":{\"quality\":1,\"helpers_visible\":2,\"helpers_healthy\":1,\"helper_target\":5}"
        ));
        assert!(json.contains("\"worker_count\":3"));
        assert!(json.contains("\"healthy_worker_count\":2"));
        assert!(json.contains("\"failed_worker_count\":1"));
        assert!(json.contains("\"helper_worker_count\":2"));
        assert!(json.contains("\"healthy_helper_worker_count\":1"));
        assert!(json.contains("\"metal_worker_count\":1"));
        assert!(json.contains("\"unknown_runtime_worker_count\":1"));
        assert!(json.contains("\"quality_runtime_accelerated\":true"));
        assert!(json.contains("\"model_pool_saturation_milli\":333"));
        assert!(json.contains("\"homeostatic_model_cell_expansion_allowed\":false"));
        assert!(json.contains("\"homeostatic_decision\":\"reject_new_spawn\""));
        assert!(json.contains("\"tcp_reachable\":true"));
        assert!(json.contains("\"health_ok\":true"));
        assert!(json.contains("\"runtime_backend\":\"llama.cpp\""));
        assert!(json.contains("\"runtime_device\":\"metal\""));
        assert!(json.contains("\"runtime_accelerator\":\"metal\""));
        assert!(json.contains("\"gpu_layers\":99"));
    }

    #[test]
    fn status_capacity_allows_expansion_after_runtime_metadata_is_clear() {
        let mut workers = full_context_workers();
        workers[1].runtime_backend = Some("llama.cpp".to_owned());
        workers[1].runtime_device = Some("metal".to_owned());
        workers[1].runtime_accelerator = Some("metal".to_owned());
        workers[1].gpu_layers = Some(32);
        workers[2].reachable = true;
        workers[2].model = Some("gemma-review".to_owned());
        workers[2].context_window = Some(8192);
        workers[2].error = None;
        workers[2].runtime_backend = Some("llama.cpp".to_owned());
        workers[2].runtime_device = Some("metal".to_owned());
        workers[2].runtime_accelerator = Some("metal".to_owned());
        workers[2].gpu_layers = Some(32);
        let json = model_service_model_pool_status_response_json(4, &workers);

        assert!(json.contains("\"launch_allowed\":true"));
        assert!(json.contains("\"expansion_allowed\":true"));
        assert!(json.contains(
            "\"recommendation\":\"hold_or_add_optional_test_gate_if_memory_pressure_green\""
        ));
        assert!(json.contains("\"safe_to_enable_pool_workers\":true"));
        assert!(json.contains("\"next_step\":\"add_summary_worker_first\""));
        assert!(json.contains("\"reason_detail\":\"quality_chain_ready_no_helpers_visible\""));
        assert!(json.contains("\"failed_worker_count\":0"));
        assert!(json.contains("\"healthy_helper_worker_count\":2"));
        assert!(json.contains("\"metal_worker_count\":3"));
        assert!(json.contains("\"unknown_runtime_worker_count\":0"));
        assert!(json.contains("\"model_pool_saturation_milli\":0"));
        assert!(json.contains("\"homeostatic_model_cell_expansion_allowed\":true"));
        assert!(json.contains("\"homeostatic_decision\":\"normal\""));
    }

    #[test]
    fn status_capacity_blocks_expansion_when_worker_health_fails() {
        let mut workers = full_context_workers();
        workers[1].runtime_backend = Some("llama.cpp".to_owned());
        workers[1].runtime_device = Some("metal".to_owned());
        workers[1].runtime_accelerator = Some("metal".to_owned());
        workers[1].gpu_layers = Some(32);
        let json = model_service_model_pool_status_response_json(21, &workers);

        assert!(json.contains("\"launch_allowed\":true"));
        assert!(json.contains("\"expansion_allowed\":false"));
        assert!(
            json.contains("\"recommendation\":\"restore_failed_model_workers_before_expansion\"")
        );
        assert!(json.contains("\"failed_worker_count\":1"));
        assert!(json.contains("\"model_pool_saturation_milli\":333"));
        assert!(json.contains("\"homeostatic_model_cell_expansion_allowed\":false"));
        assert!(json.contains("\"homeostatic_decision\":\"reject_new_spawn\""));
    }

    #[test]
    fn status_advice_blocks_non_index_cpu_helpers_when_quality_is_accelerated() {
        let mut workers = full_context_workers();
        workers[2].reachable = true;
        workers[2].model = Some("gemma-review".to_owned());
        workers[2].context_window = Some(4096);
        workers[2].error = None;
        workers[2].runtime_backend = Some("llama.cpp".to_owned());
        workers[2].runtime_device = Some("cpu".to_owned());
        workers[2].runtime_accelerator = Some("accelerate".to_owned());
        workers[2].gpu_layers = Some(0);
        let json = model_service_model_pool_status_response_json(9, &workers);

        assert!(json.contains("\"launch_allowed\":true"));
        assert!(json.contains("\"expansion_allowed\":false"));
        assert!(json.contains("\"safe_to_enable_pool_workers\":false"));
        assert!(
            json.contains(
                "\"next_step\":\"fix_helper_metal_or_gpu_layers_before_more_pool_workers\""
            )
        );
        assert!(json.contains("\"reason\":\"helper_workers_not_gpu_accelerated\""));
        assert!(json.contains("\"helper_cpu_or_no_gpu_roles\":[\"review\"]"));
        assert!(json.contains("\"blocking_helper_cpu_or_no_gpu_roles\":[\"review\"]"));
        assert!(json.contains("\"allowed_cpu_fallback_helper_roles\":[]"));
        assert!(json.contains("\"quality_runtime_accelerated\":true"));
        assert!(json.contains("\"cpu_worker_count\":1"));
        assert!(json.contains("\"zero_gpu_layer_worker_count\":1"));
        assert!(json.contains("\"runtime_device\":\"cpu\""));
        assert!(json.contains("\"runtime_accelerator\":\"accelerate\""));
        assert!(json.contains("\"gpu_layers\":0"));
    }

    #[test]
    fn status_advice_allows_index_cpu_fallback_when_quality_is_accelerated() {
        let mut workers = full_context_workers();
        workers[1].runtime_backend = Some("llama.cpp".to_owned());
        workers[1].runtime_device = Some("cpu".to_owned());
        workers[1].runtime_accelerator = Some("accelerate".to_owned());
        workers[1].gpu_layers = Some(0);
        let json = model_service_model_pool_status_response_json(19, &workers);

        assert!(json.contains("\"launch_allowed\":true"));
        assert!(json.contains("\"safe_to_enable_pool_workers\":true"));
        assert!(json.contains("\"reason\":\"quality_chain_ready_no_helpers_visible\""));
        assert!(json.contains("\"helper_cpu_or_no_gpu_roles\":[\"index\"]"));
        assert!(json.contains("\"blocking_helper_cpu_or_no_gpu_roles\":[]"));
        assert!(json.contains("\"allowed_cpu_fallback_helper_roles\":[\"index\"]"));
        assert!(json.contains("\"cpu_worker_count\":1"));
        assert!(json.contains("\"zero_gpu_layer_worker_count\":1"));
    }

    #[test]
    fn status_advice_treats_summary_and_test_gate_as_partial_helper_pool() {
        let mut workers = full_context_workers();
        workers[1].role = "summary".to_owned();
        workers[1].runtime_backend = Some("llama.cpp".to_owned());
        workers[1].runtime_device = Some("metal".to_owned());
        workers[1].runtime_accelerator = Some("metal".to_owned());
        workers[1].gpu_layers = Some(32);
        workers[2].role = "test-gate".to_owned();
        workers[2].reachable = true;
        workers[2].error = None;
        workers[2].runtime_backend = Some("llama.cpp".to_owned());
        workers[2].runtime_device = Some("metal".to_owned());
        workers[2].runtime_accelerator = Some("metal".to_owned());
        workers[2].gpu_layers = Some(16);

        let json = model_service_model_pool_status_response_json(17, &workers);

        assert!(json.contains("\"launch_allowed\":true"));
        assert!(json.contains("\"safe_to_enable_pool_workers\":true"));
        assert!(json.contains("\"next_step\":\"add_remaining_helper_roles_one_at_a_time\""));
        assert!(json.contains("\"reason\":\"partial_helper_pool_visible\""));
        assert!(json.contains("\"reason_detail\":\"partial_helper_pool_visible\""));
        assert!(json.contains("\"helper_roles\":[\"summary\",\"test-gate\"]"));
        assert!(json.contains(
            "\"expected_helper_roles\":[\"summary\",\"router\",\"review\",\"index\",\"test-gate\"]"
        ));
        assert!(json.contains("\"missing_helper_roles\":[\"router\",\"review\",\"index\"]"));
        assert!(json.contains("\"helper_cpu_or_no_gpu_roles\":[]"));
        assert!(json.contains("\"blocking_helper_cpu_or_no_gpu_roles\":[]"));
        assert!(json.contains("\"allowed_cpu_fallback_helper_roles\":[]"));
        assert!(json.contains(
            "\"recommended_launch_order\":[\"quality\",\"summary\",\"router\",\"review\",\"index\",\"test-gate\"]"
        ));
        assert!(json.contains(
            "\"worker_shape\":{\"quality\":1,\"helpers_visible\":2,\"helpers_healthy\":2,\"helper_target\":5}"
        ));
    }

    #[test]
    fn status_advice_emits_helper_roles_in_canonical_order() {
        let helper = |role: &str, port: u16| ModelPoolWorkerView {
            role: role.to_owned(),
            port,
            base_url: format!("http://127.0.0.1:{port}"),
            enabled_by_default: true,
            model_class: "small helper".to_owned(),
            suggested_quant: "Q4".to_owned(),
            default_context_tokens: 4096,
            default_max_tokens: 512,
            low_priority: true,
            reachable: true,
            model: Some(format!("{role}.gguf")),
            context_window: Some(4096),
            runtime_backend: Some("llama.cpp".to_owned()),
            runtime_device: Some("metal".to_owned()),
            runtime_accelerator: Some("metal".to_owned()),
            gpu_layers: Some(999),
            error: None,
        };
        let workers = vec![
            full_context_workers().remove(0),
            helper("review", 8688),
            helper("router", 8689),
            helper("test-gate", 8688),
            helper("index", 8690),
            helper("summary", 8687),
        ];

        let json = model_service_model_pool_status_response_json(19, &workers);

        assert!(json.contains(
            "\"helper_roles\":[\"summary\",\"router\",\"review\",\"index\",\"test-gate\"]"
        ));
        assert!(json.contains("\"missing_helper_roles\":[]"));
        assert!(json.contains(
            "\"worker_shape\":{\"quality\":1,\"helpers_visible\":5,\"helpers_healthy\":5,\"helper_target\":5}"
        ));
    }

    #[test]
    fn status_and_route_block_extra_quality_12b_workers() {
        let workers = duplicate_quality_workers();
        let status = model_service_model_pool_status_response_json(15, &workers);
        let route = model_service_model_pool_route_response_json(16, "review", None, &workers);

        assert!(status.contains("\"launch_allowed\":false"));
        assert!(status.contains("\"launch_block_reason\":\"extra_quality_12b_workers\""));
        assert!(status.contains("\"quality_block_reason\":\"extra_quality_12b_workers\""));
        assert!(status.contains("\"quality_worker_count\":2"));
        assert!(status.contains("\"extra_quality_12b_detected\":true"));
        assert!(status.contains("\"safe_to_enable_pool_workers\":false"));
        assert!(status.contains(
            "\"next_step\":\"stop_extra_quality_12b_workers_keep_one_quality_plus_helpers\""
        ));
        assert!(status.contains("\"reason\":\"extra_quality_12b_wastes_shared_apple_memory\""));
        assert!(
            status.contains("\"reason_detail\":\"extra_quality_12b_wastes_shared_apple_memory\"")
        );
        assert!(status.contains(
            "\"capacity_recommendation\":\"stop_extra_quality_12b_workers_keep_one_quality_plus_helpers\""
        ));
        assert!(status.contains(
            "\"worker_shape\":{\"quality\":2,\"helpers_visible\":2,\"helpers_healthy\":1,\"helper_target\":5}"
        ));
        assert!(route.contains("\"route_allowed\":false"));
        assert!(route.contains(
            "\"route_block_reason\":\"model_pool_launch_blocked:extra_quality_12b_workers\""
        ));
        assert!(route.contains("\"selected_role\":null"));
    }

    #[test]
    fn status_json_includes_model_pool_metrics_when_supplied() {
        let metrics = ModelPoolMetricsSnapshotView {
            route_metrics: ModelPoolMetricsView {
                route_count: 3,
                selected_count: 2,
                blocked_count: 1,
                in_flight: 1,
                queued_count: 0,
                lease_wait_ms: Some(0),
                lease_wait_p95_ms: Some(0),
                success_count: 4,
                failure_count: 1,
                avg_latency_ms: Some(250),
                latency_p50_ms: Some(240),
                latency_p95_ms: Some(310),
            },
            worker_metrics: vec![ModelPoolWorkerMetricsView {
                role: "quality".to_owned(),
                metrics: ModelPoolMetricsView {
                    route_count: 2,
                    selected_count: 2,
                    blocked_count: 0,
                    in_flight: 1,
                    queued_count: 0,
                    lease_wait_ms: Some(0),
                    lease_wait_p95_ms: Some(0),
                    success_count: 4,
                    failure_count: 0,
                    avg_latency_ms: Some(220),
                    latency_p50_ms: Some(200),
                    latency_p95_ms: Some(300),
                },
            }],
        };
        let json = model_service_model_pool_status_response_json_with_metrics(
            12,
            &workers(),
            Some(&metrics),
        );

        assert!(json.contains("\"route_metrics\""));
        assert!(json.contains("\"route_count\":3"));
        assert!(json.contains("\"worker_metrics\""));
        assert!(json.contains("\"role\":\"quality\",\"route_count\":2"));
        assert!(json.contains("\"success_count\":4"));
        assert!(json.contains("\"queued_count\":0"));
        assert!(json.contains("\"lease_wait_p95_ms\":0"));
        assert!(json.contains("\"avg_latency_ms\":220"));
        assert!(json.contains("\"latency_p50_ms\":200"));
        assert!(json.contains("\"latency_p95_ms\":300"));
    }

    #[test]
    fn route_json_blocks_review_when_review_worker_is_down() {
        let json = model_service_model_pool_route_response_json(
            4,
            "review",
            None,
            &full_context_workers(),
        );

        assert!(json.contains("\"route_allowed\":false"));
        assert!(json.contains("\"route_block_reason\":\"no_ready_candidate\""));
        assert!(json.contains("\"selected_role\":null"));
        assert!(json.contains("\"role_candidates\":[\"review\"]"));
        assert!(json.contains("\"can_accept_low_priority_task\":true"));
    }

    #[test]
    fn route_json_blocks_when_quality_context_window_is_too_small() {
        let json = model_service_model_pool_route_response_json(13, "quality", None, &workers());

        assert!(json.contains("\"route_allowed\":false"));
        assert!(json.contains(
            "\"route_block_reason\":\"model_pool_launch_blocked:quality_context_window_too_small\""
        ));
        assert!(json.contains("\"selected_role\":null"));
        assert!(json.contains("\"pool_dispatch\":null"));
    }

    #[test]
    fn route_json_still_selects_quality_for_quality_task() {
        let json = model_service_model_pool_route_response_json(
            5,
            "quality",
            None,
            &full_context_workers(),
        );

        assert!(json.contains("\"route_allowed\":true"));
        assert!(json.contains("\"selected_role\":\"quality\""));
        assert!(json.contains("\"role_candidates\":[\"quality\"]"));
        assert!(json.contains("\"compute_budget_summary\":\"model_pool_route_plan selected_role=quality effective_max_tokens=262144 saved_tokens=0 max_tokens_clamped=false\""));
        assert!(json.contains("\"compute_budget_configured_max_tokens\":null"));
        assert!(json.contains("\"compute_budget_effective_max_tokens\":262144"));
        assert!(json.contains("\"compute_budget_saved_tokens\":0"));
        assert!(json.contains("\"compute_budget_avoided_tokens\":0"));
        assert!(json.contains("\"compute_budget_max_tokens_clamped\":false"));
    }

    #[test]
    fn call_json_marks_prompt_sending_contract() {
        let workers = full_context_workers();
        let token_budget = model_pool_max_tokens_decision(&workers[0], Some(262_144));
        let json = model_service_model_pool_call_response_json(
            6,
            "quality",
            &workers[0],
            &token_budget,
            true,
            "answer",
        );

        assert!(json.contains("\"read_only\":false"));
        assert!(json.contains("\"sends_prompt\":true"));
        assert!(json.contains("\"elapsed_ms\":0"));
        assert!(json.contains("\"answer_chars\":6"));
        assert!(json.contains("\"answer_bytes\":6"));
        assert!(json.contains("\"answer_approx_tokens\":2"));
        assert!(json.contains("\"selected_role\":\"quality\""));
        assert!(json.contains("\"effective_max_tokens\":262144"));
        assert!(json.contains("\"max_tokens_clamped\":false"));
        assert!(json.contains("\"compute_budget_summary\":\"model_pool_call selected_role=quality effective_max_tokens=262144 saved_tokens=0 max_tokens_clamped=false\""));
        assert!(json.contains("\"compute_budget_configured_max_tokens\":262144"));
        assert!(json.contains("\"compute_budget_effective_max_tokens\":262144"));
        assert!(json.contains("\"compute_budget_saved_tokens\":0"));
        assert!(json.contains("\"compute_budget_avoided_tokens\":0"));
        assert!(json.contains("\"compute_budget_max_tokens_clamped\":false"));
        assert!(json.contains("\"runtime_backend\":\"llama.cpp\""));
        assert!(json.contains("\"runtime_device\":\"metal\""));
        assert!(json.contains("\"runtime_accelerator\":\"metal\""));
        assert!(json.contains("\"gpu_layers\":99"));
        assert!(
            json.contains(
                "\"max_tokens_clamp_reason\":\"quality_worker_request_budget_preserved\""
            )
        );
        assert!(json.contains("\"answer\":\"answer\""));
    }

    #[test]
    fn call_json_uses_supplied_execution_metrics() {
        let workers = full_context_workers();
        let token_budget = model_pool_max_tokens_decision(&workers[0], Some(262_144));
        let execution = ModelPoolCallExecutionView::from_answer(1234, "hi 你");
        let json = model_service_model_pool_call_response_json_with_metrics(
            14,
            "quality",
            &workers[0],
            &token_budget,
            true,
            "hi 你",
            &execution,
            None,
        );

        assert!(json.contains("\"elapsed_ms\":1234"));
        assert!(json.contains("\"answer_chars\":4"));
        assert!(json.contains("\"answer_bytes\":6"));
        assert!(json.contains("\"answer_approx_tokens\":1"));
        assert!(json.contains("\"answer\":\"hi 你\""));
    }

    #[test]
    fn low_priority_worker_clamps_to_worker_default_budget() {
        let mut workers = full_context_workers();
        workers[2].reachable = true;
        workers[2].error = None;

        let budget = model_pool_max_tokens_decision(&workers[2], Some(262_144));
        let json =
            model_service_model_pool_route_response_json(8, "review", Some(262_144), &workers);

        assert_eq!(budget.effective_max_tokens, 1536);
        assert!(budget.max_tokens_clamped);
        assert_eq!(
            budget.max_tokens_clamp_reason,
            "low_priority_worker_default_max_tokens"
        );
        assert!(json.contains("\"selected_role\":\"review\""));
        assert!(json.contains("\"configured_max_tokens\":262144"));
        assert!(json.contains("\"effective_max_tokens\":1536"));
        assert!(json.contains("\"max_tokens_clamped\":true"));
        assert!(json.contains("\"compute_budget_summary\":\"model_pool_route_plan selected_role=review effective_max_tokens=1536 saved_tokens=260608 max_tokens_clamped=true\""));
        assert!(json.contains("\"compute_budget_configured_max_tokens\":262144"));
        assert!(json.contains("\"compute_budget_effective_max_tokens\":1536"));
        assert!(json.contains("\"compute_budget_saved_tokens\":260608"));
        assert!(json.contains("\"compute_budget_avoided_tokens\":260608"));
        assert!(json.contains("\"compute_budget_max_tokens_clamped\":true"));

        let call_json = model_service_model_pool_call_response_json(
            15,
            "review",
            &workers[2],
            &budget,
            true,
            "reviewed",
        );

        assert!(call_json.contains("\"compute_budget_effective_max_tokens\":1536"));
        assert!(call_json.contains("\"compute_budget_saved_tokens\":260608"));
        assert!(call_json.contains("\"compute_budget_avoided_tokens\":260608"));
        assert!(call_json.contains("\"compute_budget_max_tokens_clamped\":true"));
    }

    #[test]
    fn high_budget_auto_route_prefers_quality_worker() {
        let mut workers = full_context_workers();
        workers[2].reachable = true;
        workers[2].error = None;

        let json = model_service_model_pool_route_response_json(9, "auto", Some(262_144), &workers);

        assert!(json.contains(
            "\"role_candidates\":[\"quality\",\"summary\",\"router\",\"review\",\"test-gate\",\"index\"]"
        ));
        assert!(json.contains("\"selected_role\":\"quality\""));
        assert!(json.contains("\"effective_max_tokens\":262144"));
        assert!(json.contains("\"max_tokens_clamped\":false"));
    }

    #[test]
    fn complex_auto_prompt_uses_routing_weights_to_select_quality() {
        let mut workers = full_context_workers();
        workers[2].reachable = true;
        workers[2].error = None;
        let prompt = "请审查这个 Rust routing 架构、stream 日志、json 输出和测试错误，然后给出索引与门禁改进。".repeat(45);

        let json = model_service_model_pool_route_response_json_with_context(
            20,
            "auto",
            None,
            Some(&prompt),
            &workers,
            None,
            None,
        );

        assert!(json.contains("\"routing_weights\":{\"strategy\":\"rwaf_v1\""));
        assert!(json.contains("\"task_complexity_band\":\"complex\""));
        assert!(json.contains("\"role_candidates\":[\"quality\""));
        assert!(json.contains("\"selected_role\":\"quality\""));
    }

    #[test]
    fn auto_route_weights_demote_failed_helper_candidate() {
        let mut workers = full_context_workers();
        workers.push(ModelPoolWorkerView {
            role: "summary".to_owned(),
            port: 8687,
            base_url: "http://127.0.0.1:8687".to_owned(),
            enabled_by_default: true,
            model_class: "Gemma small summary".to_owned(),
            suggested_quant: "Q4".to_owned(),
            default_context_tokens: 8192,
            default_max_tokens: 768,
            low_priority: true,
            reachable: true,
            model: Some("summary.gguf".to_owned()),
            context_window: Some(8192),
            runtime_backend: Some("llama.cpp".to_owned()),
            runtime_device: Some("metal".to_owned()),
            runtime_accelerator: Some("metal".to_owned()),
            gpu_layers: Some(999),
            error: None,
        });
        workers.push(ModelPoolWorkerView {
            role: "router".to_owned(),
            port: 8689,
            base_url: "http://127.0.0.1:8689".to_owned(),
            enabled_by_default: true,
            model_class: "FunctionGemma".to_owned(),
            suggested_quant: "Q4".to_owned(),
            default_context_tokens: 4096,
            default_max_tokens: 512,
            low_priority: true,
            reachable: true,
            model: Some("router.gguf".to_owned()),
            context_window: Some(4096),
            runtime_backend: Some("llama.cpp".to_owned()),
            runtime_device: Some("metal".to_owned()),
            runtime_accelerator: Some("metal".to_owned()),
            gpu_layers: Some(999),
            error: None,
        });
        let metrics = ModelPoolMetricsSnapshotView {
            route_metrics: ModelPoolMetricsView::default(),
            worker_metrics: vec![
                ModelPoolWorkerMetricsView {
                    role: "summary".to_owned(),
                    metrics: ModelPoolMetricsView {
                        failure_count: 4,
                        success_count: 0,
                        ..ModelPoolMetricsView::default()
                    },
                },
                ModelPoolWorkerMetricsView {
                    role: "router".to_owned(),
                    metrics: ModelPoolMetricsView {
                        failure_count: 0,
                        success_count: 4,
                        ..ModelPoolMetricsView::default()
                    },
                },
            ],
        };

        let json = model_service_model_pool_route_response_json_with_context(
            21,
            "auto",
            None,
            Some("route a short request"),
            &workers,
            None,
            Some(&metrics),
        );

        assert!(json.contains("\"history_penalty_applied\":true"));
        assert!(json.contains("\"role_candidates\":[\"router\",\"summary\""));
        assert!(json.contains("\"selected_role\":\"router\""));
    }

    #[test]
    fn auto_route_resource_precheck_demotes_busy_helper_candidate() {
        let mut workers = full_context_workers();
        workers.push(ModelPoolWorkerView {
            role: "summary".to_owned(),
            port: 8687,
            base_url: "http://127.0.0.1:8687".to_owned(),
            enabled_by_default: true,
            model_class: "Gemma small summary".to_owned(),
            suggested_quant: "Q4".to_owned(),
            default_context_tokens: 8192,
            default_max_tokens: 768,
            low_priority: true,
            reachable: true,
            model: Some("summary.gguf".to_owned()),
            context_window: Some(8192),
            runtime_backend: Some("llama.cpp".to_owned()),
            runtime_device: Some("metal".to_owned()),
            runtime_accelerator: Some("metal".to_owned()),
            gpu_layers: Some(999),
            error: None,
        });
        workers.push(ModelPoolWorkerView {
            role: "router".to_owned(),
            port: 8689,
            base_url: "http://127.0.0.1:8689".to_owned(),
            enabled_by_default: true,
            model_class: "FunctionGemma".to_owned(),
            suggested_quant: "Q4".to_owned(),
            default_context_tokens: 4096,
            default_max_tokens: 512,
            low_priority: true,
            reachable: true,
            model: Some("router.gguf".to_owned()),
            context_window: Some(4096),
            runtime_backend: Some("llama.cpp".to_owned()),
            runtime_device: Some("metal".to_owned()),
            runtime_accelerator: Some("metal".to_owned()),
            gpu_layers: Some(999),
            error: None,
        });
        let metrics = ModelPoolMetricsSnapshotView {
            route_metrics: ModelPoolMetricsView::default(),
            worker_metrics: vec![
                ModelPoolWorkerMetricsView {
                    role: "summary".to_owned(),
                    metrics: ModelPoolMetricsView {
                        in_flight: 4,
                        success_count: 8,
                        ..ModelPoolMetricsView::default()
                    },
                },
                ModelPoolWorkerMetricsView {
                    role: "router".to_owned(),
                    metrics: ModelPoolMetricsView {
                        success_count: 8,
                        ..ModelPoolMetricsView::default()
                    },
                },
            ],
        };

        let json = model_service_model_pool_route_response_json_with_context(
            22,
            "auto",
            None,
            Some("route a short request"),
            &workers,
            None,
            Some(&metrics),
        );

        assert!(json.contains("\"resource_precheck\":{\"strategy\":\"resource_precheck_v1\""));
        assert!(json.contains("\"pressure\":\"high\""));
        assert!(json.contains("\"reason\":\"resource_constrained_candidates_demoted\""));
        assert!(json.contains("\"avoid_roles\":[\"summary\"]"));
        assert!(json.contains("\"role_candidates\":[\"router\",\"summary\""));
        assert!(json.contains("\"selected_role\":\"router\""));
    }

    #[test]
    fn route_json_blocks_when_only_candidate_is_resource_constrained() {
        let mut workers = full_context_workers();
        workers.push(ModelPoolWorkerView {
            role: "router".to_owned(),
            port: 8689,
            base_url: "http://127.0.0.1:8689".to_owned(),
            enabled_by_default: true,
            model_class: "FunctionGemma".to_owned(),
            suggested_quant: "Q4".to_owned(),
            default_context_tokens: 4096,
            default_max_tokens: 512,
            low_priority: true,
            reachable: true,
            model: Some("router.gguf".to_owned()),
            context_window: Some(4096),
            runtime_backend: Some("llama.cpp".to_owned()),
            runtime_device: Some("metal".to_owned()),
            runtime_accelerator: Some("metal".to_owned()),
            gpu_layers: Some(999),
            error: None,
        });
        let metrics = ModelPoolMetricsSnapshotView {
            route_metrics: ModelPoolMetricsView::default(),
            worker_metrics: vec![ModelPoolWorkerMetricsView {
                role: "router".to_owned(),
                metrics: ModelPoolMetricsView {
                    in_flight: 6,
                    success_count: 8,
                    ..ModelPoolMetricsView::default()
                },
            }],
        };

        let json = model_service_model_pool_route_response_json_with_context(
            23,
            "router",
            None,
            Some("route this"),
            &workers,
            None,
            Some(&metrics),
        );

        assert!(json.contains("\"route_allowed\":false"));
        assert!(json.contains(
            "\"route_block_reason\":\"resource_precheck_blocked:all_candidates_resource_constrained\""
        ));
        assert!(json.contains("\"allow_dispatch\":false"));
        assert!(json.contains("\"selected_role\":null"));
        assert!(json.contains("\"pool_dispatch\":null"));
    }

    #[test]
    fn route_json_blocks_when_dependency_precheck_missing_roles() {
        let workers = full_context_workers();
        let completed = vec!["quality".to_owned(), "summary".to_owned()];

        let json = model_service_model_pool_route_response_json_with_context(
            24,
            "index",
            None,
            Some("index this change"),
            &workers,
            Some(&completed),
            None,
        );

        assert!(
            json.contains("\"dependency_precheck\":{\"strategy\":\"role_dependency_graph_v1\"")
        );
        assert!(json.contains("\"requested_role\":\"index\""));
        assert!(json.contains("\"required_roles\":[\"summary\",\"router\"]"));
        assert!(json.contains("\"missing_roles\":[\"router\"]"));
        assert!(json.contains("\"route_allowed\":false"));
        assert!(json.contains(
            "\"route_block_reason\":\"dependency_precheck_blocked:missing_required_roles\""
        ));
        assert!(json.contains("\"selected_role\":null"));
        assert!(json.contains("\"pool_dispatch\":null"));
    }

    #[test]
    fn route_json_allows_when_dependency_precheck_is_satisfied() {
        let workers = full_context_workers();
        let completed = vec![
            "quality".to_owned(),
            "summary".to_owned(),
            "router".to_owned(),
        ];

        let json = model_service_model_pool_route_response_json_with_context(
            25,
            "index",
            None,
            Some("index this change"),
            &workers,
            Some(&completed),
            None,
        );

        assert!(
            json.contains("\"dependency_precheck\":{\"strategy\":\"role_dependency_graph_v1\"")
        );
        assert!(json.contains("\"reason\":\"dependencies_satisfied\""));
        assert!(json.contains("\"route_allowed\":true"));
        assert!(json.contains("\"selected_role\":\"index\""));
        assert!(json.contains("\"selected_context_buffer_policy\":{\"strategy\":\"none\""));
    }

    #[test]
    fn route_json_blocks_when_service_stream_slots_are_saturated() {
        let workers = full_context_workers();
        let service_backpressure = ModelPoolServiceBackpressureView::new(4, 4, 2);

        let json = model_service_model_pool_route_response_json_with_context_and_backpressure(
            26,
            "quality",
            None,
            Some("route this quality request"),
            &workers,
            None,
            None,
            Some(&service_backpressure),
        );

        assert!(json.contains("\"route_allowed\":false"));
        assert!(json.contains(
            "\"route_block_reason\":\"service_backpressure_blocked:stream_slots_saturated\""
        ));
        assert!(
            json.contains(
                "\"service_backpressure\":{\"strategy\":\"service_stream_backpressure_v1\""
            )
        );
        assert!(json.contains("\"active_engine_requests\":4"));
        assert!(json.contains("\"max_active_stream_engine_requests\":4"));
        assert!(json.contains("\"stream_backpressure_rejections\":2"));
        assert!(json.contains("\"pressure\":\"saturated\""));
        assert!(json.contains("\"allow_dispatch\":false"));
        assert!(json.contains("\"pool_dispatch\":null"));
    }

    #[test]
    fn test_gate_route_reserves_dynamic_context_buffer_when_sufficient() {
        let mut workers = full_context_workers();
        workers.push(test_gate_worker(4096));
        let completed = vec![
            "quality".to_owned(),
            "summary".to_owned(),
            "router".to_owned(),
            "review".to_owned(),
            "index".to_owned(),
        ];

        let json = model_service_model_pool_route_response_json_with_context(
            27,
            "test-gate",
            None,
            None,
            &workers,
            Some(&completed),
            None,
        );

        assert!(json.contains("\"route_allowed\":true"));
        assert!(json.contains("\"selected_role\":\"test-gate\""));
        assert!(json.contains("\"selected_default_max_tokens\":1536"));
        assert!(json.contains("\"selected_context_window\":4096"));
        assert!(json.contains("\"selected_context_required_tokens\":4096"));
        assert!(json.contains("\"selected_context_buffer_tokens\":2560"));
        assert!(json.contains(
            "\"selected_context_buffer_policy\":{\"strategy\":\"test_gate_dynamic_upstream_buffer_v1\",\"base_tokens\":2048,\"upstream_role_tokens\":256"
        ));
        assert!(json.contains("\"eligible_upstream_roles\":[\"review\",\"index\"]"));
        assert!(json.contains("\"completed_upstream_roles\":[\"review\",\"index\"]"));
        assert!(json.contains("\"total_tokens\":2560"));
        assert!(json.contains("\"selected_context_sufficient\":true"));
        assert!(json.contains("\"selected_context_block_reason\":\"none\""));
        assert!(json.contains("\"pool_dispatch\":{\"selected_role\":\"test-gate\""));
    }

    #[test]
    fn test_gate_context_buffer_grows_with_review_and_index_evidence() {
        let completed = vec![
            "quality".to_owned(),
            "summary".to_owned(),
            "review".to_owned(),
            "index".to_owned(),
        ];

        assert_eq!(route_context_buffer_tokens("summary", Some(&completed)), 0);
        assert_eq!(route_context_buffer_tokens("test-gate", None), 2048);
        assert_eq!(
            route_context_buffer_tokens("test-gate", Some(&completed)),
            2560
        );
        assert_eq!(
            route_context_buffer_policy("test-gate", Some(&completed)).completed_upstream_roles,
            vec!["review".to_owned(), "index".to_owned()]
        );
    }

    #[test]
    fn test_gate_route_blocks_when_dynamic_context_buffer_does_not_fit() {
        let mut workers = full_context_workers();
        workers.push(test_gate_worker(4096));
        let completed = vec![
            "quality".to_owned(),
            "summary".to_owned(),
            "router".to_owned(),
            "review".to_owned(),
            "index".to_owned(),
        ];
        let prompt = "x".repeat(6000);

        let json = model_service_model_pool_route_response_json_with_context(
            28,
            "test-gate",
            None,
            Some(&prompt),
            &workers,
            Some(&completed),
            None,
        );

        assert!(json.contains("\"route_allowed\":false"));
        assert!(json.contains("\"route_block_reason\":\"selected_context_window_too_small\""));
        assert!(json.contains("\"selected_role\":\"test-gate\""));
        assert!(json.contains("\"selected_context_window\":4096"));
        assert!(json.contains("\"selected_context_required_tokens\":5596"));
        assert!(json.contains("\"selected_context_buffer_tokens\":2560"));
        assert!(json.contains("\"selected_context_sufficient\":false"));
        assert!(
            json.contains(
                "\"selected_context_block_reason\":\"selected_context_window_too_small\""
            )
        );
        assert!(json.contains("\"pool_dispatch\":null"));
    }

    #[test]
    fn call_blocked_json_can_emit_dependency_precheck_evidence() {
        let workers = full_context_workers();
        let completed = vec!["quality".to_owned(), "summary".to_owned()];
        let dependency = model_pool_dependency_precheck("test-gate", Some(&completed));

        let json = model_service_model_pool_call_blocked_response_json_with_metrics_and_dependency(
            26,
            "test-gate",
            "dependency_precheck_blocked:missing_required_roles",
            &workers,
            None,
            Some(&dependency),
        );

        assert!(json.contains("\"ok\":false"));
        assert!(
            json.contains("\"dependency_precheck\":{\"strategy\":\"role_dependency_graph_v1\"")
        );
        assert!(json.contains("\"requested_role\":\"test-gate\""));
        assert!(json.contains("\"missing_roles\":[\"review\",\"index\"]"));
    }

    #[test]
    fn router_route_prefers_function_router_then_summary() {
        let mut workers = full_context_workers();
        workers.push(ModelPoolWorkerView {
            role: "router".to_owned(),
            port: 8689,
            base_url: "http://127.0.0.1:8689".to_owned(),
            enabled_by_default: true,
            model_class: "FunctionGemma 270M".to_owned(),
            suggested_quant: "Q4".to_owned(),
            default_context_tokens: 4096,
            default_max_tokens: 512,
            low_priority: true,
            reachable: true,
            model: Some("functiongemma".to_owned()),
            context_window: Some(4096),
            runtime_backend: Some("llama.cpp".to_owned()),
            runtime_device: Some("metal".to_owned()),
            runtime_accelerator: Some("metal".to_owned()),
            gpu_layers: Some(999),
            error: None,
        });

        let json = model_service_model_pool_route_response_json(18, "router", None, &workers);

        assert!(json.contains("\"route_allowed\":true"));
        assert!(json.contains("\"role_candidates\":[\"router\"]"));
        assert!(json.contains("\"selected_role\":\"router\""));
        assert!(json.contains("\"selected_port\":8689"));
        assert!(json.contains("\"effective_max_tokens\":512"));
    }

    #[test]
    fn index_route_prefers_index_then_summary() {
        let json = model_service_model_pool_route_response_json(
            10,
            "index",
            None,
            &full_context_workers(),
        );

        assert!(json.contains("\"route_allowed\":true"));
        assert!(json.contains("\"role_candidates\":[\"index\"]"));
        assert!(json.contains("\"selected_role\":\"index\""));
        assert!(json.contains("\"effective_max_tokens\":512"));
    }

    #[test]
    fn spare_route_aliases_to_index() {
        let json = model_service_model_pool_route_response_json(
            11,
            "spare",
            None,
            &full_context_workers(),
        );

        assert!(json.contains("\"route_allowed\":true"));
        assert!(json.contains("\"role_candidates\":[\"index\"]"));
        assert!(json.contains("\"selected_role\":\"index\""));
    }

    #[test]
    fn blocked_call_json_marks_no_prompt_sent() {
        let json = model_service_model_pool_call_blocked_response_json(
            7,
            "review",
            "no_ready_candidate",
            &workers(),
        );

        assert!(json.contains("\"ok\":false"));
        assert!(json.contains("\"sends_prompt\":false"));
        assert!(json.contains("\"route_block_reason\":\"no_ready_candidate\""));
        assert!(json.contains("\"role_candidates\":[\"review\"]"));
        assert!(json.contains("\"quality_context_tokens\":8192"));
        assert!(json.contains("\"quality_context_required_tokens\":262144"));
        assert!(json.contains("\"quality_context_sufficient\":false"));
        assert!(json.contains("\"quality_block_reason\":\"context_window_below_quality_default\""));
    }
}
