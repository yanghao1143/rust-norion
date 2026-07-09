use std::collections::BTreeSet;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use crate::json::{
    json_array_field, json_bool_field, json_object_field, json_string, json_string_array,
    json_string_field, json_u64_field, parse_json_string_array,
};
use crate::model_policy;

const MAX_ROLE_RUNTIME_TOKEN_SHARE: f64 = 0.60;
const ROLE_RUNTIME_TOKEN_SHARE_EPSILON: f64 = 0.0005;
const DEFAULT_HELPER_ROLES: [&str; 4] = ["summary", "review", "index", "test-gate"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PoolStatusSummary {
    pub(crate) generated_unix: Option<u64>,
    pub(crate) observed_unix: Option<u64>,
    pub(crate) metadata_age_seconds: Option<u64>,
    pub(crate) max_age_seconds: Option<u64>,
    pub(crate) capacity_metadata_required: bool,
    pub(crate) metadata_stale: bool,
    pub(crate) launch_allowed: Option<bool>,
    pub(crate) launch_block_reason: Option<String>,
    pub(crate) chain_classification: Option<String>,
    pub(crate) min_context_tokens: Option<u64>,
    pub(crate) capacity: Option<PoolCapacitySummary>,
    pub(crate) worker_count: usize,
    pub(crate) reachable_workers: usize,
    pub(crate) healthy_workers: usize,
    pub(crate) roles: Vec<PoolWorkerRoleState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PoolAdviceSummary {
    pub(crate) safe_to_enable_pool_workers: bool,
    pub(crate) next_step: &'static str,
    pub(crate) reason: &'static str,
    pub(crate) kind: &'static str,
    pub(crate) text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PoolManifestSummary {
    pub(crate) contract_version: Option<String>,
    pub(crate) manifest_kind: Option<String>,
    pub(crate) read_only: Option<bool>,
    pub(crate) launches_process: Option<bool>,
    pub(crate) sends_prompt: Option<bool>,
    pub(crate) capacity_policy: Option<PoolManifestCapacityPolicySummary>,
    pub(crate) advice: Option<PoolManifestAdviceSummary>,
    pub(crate) worker_count: usize,
    pub(crate) workers: Vec<PoolManifestWorkerSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PoolManifestAdviceSummary {
    pub(crate) decision_source: Option<String>,
    pub(crate) policy: Option<String>,
    pub(crate) safe_to_enable_pool_workers: Option<bool>,
    pub(crate) next_step: Option<String>,
    pub(crate) reason: Option<String>,
    pub(crate) kind: Option<String>,
    pub(crate) extra_quality_12b_detected: Option<bool>,
    pub(crate) quality_worker_count: Option<u64>,
    pub(crate) helper_worker_count: Option<u64>,
    pub(crate) helper_target_worker_count: Option<u64>,
    pub(crate) helper_roles: Vec<String>,
    pub(crate) worker_shape: Option<PoolManifestWorkerShapeSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PoolManifestWorkerShapeSummary {
    pub(crate) quality: Option<u64>,
    pub(crate) helpers_visible: Option<u64>,
    pub(crate) helper_target: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PoolManifestCapacityPolicySummary {
    pub(crate) policy: Option<String>,
    pub(crate) target_host: Option<String>,
    pub(crate) avoid_extra_12b: Option<bool>,
    pub(crate) max_quality_12b_workers: Option<u64>,
    pub(crate) quality_role: Option<String>,
    pub(crate) quality_required_context_tokens: Option<u64>,
    pub(crate) helper_roles: Vec<String>,
    pub(crate) helper_context_tokens_total: Option<u64>,
    pub(crate) helper_default_max_tokens_total: Option<u64>,
    pub(crate) recommended_launch_order: Vec<String>,
    pub(crate) expansion_gate: Option<String>,
    pub(crate) next_step_when_quality_ready: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PoolManifestWorkerSummary {
    pub(crate) role: String,
    pub(crate) port: Option<u64>,
    pub(crate) base_url: Option<String>,
    pub(crate) default_context_tokens: Option<u64>,
    pub(crate) default_max_tokens: Option<u64>,
    pub(crate) enabled_by_default: Option<bool>,
    pub(crate) low_priority: Option<bool>,
    pub(crate) runtime_backend: Option<String>,
    pub(crate) runtime_device: Option<String>,
    pub(crate) runtime_accelerator: Option<String>,
    pub(crate) gpu_layers: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PoolCapacitySummary {
    pub(crate) policy: Option<String>,
    pub(crate) expansion_allowed: Option<bool>,
    pub(crate) recommendation: Option<String>,
    pub(crate) worker_count: Option<u64>,
    pub(crate) healthy_worker_count: Option<u64>,
    pub(crate) helper_worker_count: Option<u64>,
    pub(crate) healthy_helper_worker_count: Option<u64>,
    pub(crate) metal_worker_count: Option<u64>,
    pub(crate) cpu_worker_count: Option<u64>,
    pub(crate) unknown_runtime_worker_count: Option<u64>,
    pub(crate) zero_gpu_layer_worker_count: Option<u64>,
    pub(crate) quality_runtime_accelerated: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PoolWorkerRoleState {
    pub(crate) role: String,
    pub(crate) port: Option<u64>,
    pub(crate) base_url: Option<String>,
    pub(crate) tcp_reachable: bool,
    pub(crate) health_ok: bool,
    pub(crate) ready: bool,
    pub(crate) role_ready: bool,
    pub(crate) status: Option<String>,
    pub(crate) role_block_reason: Option<String>,
    pub(crate) low_priority: Option<bool>,
    pub(crate) can_accept_low_priority_task: Option<bool>,
    pub(crate) model: Option<String>,
    pub(crate) context_window: Option<u64>,
    pub(crate) runtime_backend: Option<String>,
    pub(crate) runtime_device: Option<String>,
    pub(crate) runtime_accelerator: Option<String>,
    pub(crate) gpu_layers: Option<u64>,
    pub(crate) route_count: Option<u64>,
    pub(crate) selected_count: Option<u64>,
    pub(crate) blocked_count: Option<u64>,
    pub(crate) in_flight: Option<u64>,
    pub(crate) queued_count: Option<u64>,
    pub(crate) lease_wait_ms: Option<u64>,
    pub(crate) lease_wait_p95_ms: Option<u64>,
    pub(crate) success_count: Option<u64>,
    pub(crate) failure_count: Option<u64>,
    pub(crate) avg_latency_ms: Option<u64>,
    pub(crate) latency_p50_ms: Option<u64>,
    pub(crate) latency_p95_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PoolRouteSummary {
    pub(crate) task_kind: Option<String>,
    pub(crate) route_allowed: Option<bool>,
    pub(crate) route_block_reason: Option<String>,
    pub(crate) dependency_precheck: Option<PoolDependencyPrecheckSummary>,
    pub(crate) quality_context_tokens: Option<u64>,
    pub(crate) quality_context_required_tokens: Option<u64>,
    pub(crate) quality_context_sufficient: Option<bool>,
    pub(crate) quality_block_reason: Option<String>,
    pub(crate) selected_context_required_tokens: Option<u64>,
    pub(crate) selected_context_buffer_tokens: Option<u64>,
    pub(crate) selected_context_buffer_policy: Option<PoolRouteContextBufferPolicySummary>,
    pub(crate) selected_context_sufficient: Option<bool>,
    pub(crate) selected_context_block_reason: Option<String>,
    pub(crate) selected_role: Option<String>,
    pub(crate) role_candidates: Vec<String>,
    pub(crate) candidate_workers: Vec<PoolRouteCandidate>,
    pub(crate) candidate_count: usize,
    pub(crate) healthy_candidates: usize,
    pub(crate) ready_candidates: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PoolRouteContextBufferPolicySummary {
    pub(crate) strategy: Option<String>,
    pub(crate) base_tokens: Option<u64>,
    pub(crate) upstream_role_tokens: Option<u64>,
    pub(crate) eligible_upstream_roles: Vec<String>,
    pub(crate) completed_upstream_roles: Vec<String>,
    pub(crate) total_tokens: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PoolDependencyPrecheckSummary {
    pub(crate) strategy: Option<String>,
    pub(crate) checked: Option<bool>,
    pub(crate) requested_role: Option<String>,
    pub(crate) allow_dispatch: Option<bool>,
    pub(crate) reason: Option<String>,
    pub(crate) required_roles: Vec<String>,
    pub(crate) completed_roles: Vec<String>,
    pub(crate) missing_roles: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PoolRouteCandidate {
    pub(crate) port: Option<u64>,
    pub(crate) role: String,
    pub(crate) base_url: Option<String>,
    pub(crate) tcp_reachable: bool,
    pub(crate) health_ok: bool,
    pub(crate) status: Option<String>,
    pub(crate) role_ready: bool,
    pub(crate) role_block_reason: Option<String>,
    pub(crate) can_accept_low_priority_task: bool,
    pub(crate) model: Option<String>,
    pub(crate) context_window: Option<u64>,
    pub(crate) runtime_backend: Option<String>,
    pub(crate) runtime_device: Option<String>,
    pub(crate) runtime_accelerator: Option<String>,
    pub(crate) gpu_layers: Option<u64>,
    pub(crate) default_max_tokens: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PoolAlignmentSummary {
    pub(crate) alignment_ok: bool,
    pub(crate) manifest_roles: Vec<String>,
    pub(crate) status_roles: Vec<String>,
    pub(crate) manifest_advice_safe_to_enable_pool_workers: Option<bool>,
    pub(crate) manifest_advice_next_step: Option<String>,
    pub(crate) manifest_advice_reason: Option<String>,
    pub(crate) manifest_advice_extra_quality_12b_detected: Option<bool>,
    pub(crate) manifest_advice_worker_shape_quality: Option<u64>,
    pub(crate) manifest_advice_worker_shape_helpers_visible: Option<u64>,
    pub(crate) manifest_advice_worker_shape_helper_target: Option<u64>,
    pub(crate) manifest_advice_worker_shape_failures: Vec<String>,
    pub(crate) manifest_quality_workers: usize,
    pub(crate) status_quality_workers: usize,
    pub(crate) max_quality_workers: usize,
    pub(crate) manifest_helper_workers: usize,
    pub(crate) status_helper_workers: usize,
    pub(crate) helper_target: usize,
    pub(crate) missing_manifest_helper_roles: Vec<String>,
    pub(crate) missing_status_helper_roles: Vec<String>,
    pub(crate) missing_status_roles: Vec<String>,
    pub(crate) unplanned_status_roles: Vec<String>,
    pub(crate) route_blocked_or_failed: Vec<String>,
    pub(crate) route_dependency_failures: Vec<String>,
    pub(crate) missing_inputs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PoolBudgetFairnessSummary {
    pub(crate) worker_count: usize,
    pub(crate) successful_worker_count: usize,
    pub(crate) feedback_worker_count: usize,
    pub(crate) roles: Vec<PoolRoleBudgetSummary>,
    pub(crate) total_runtime_tokens: u64,
    pub(crate) total_latency_ms: u64,
    pub(crate) max_role_runtime_token_share: Option<f64>,
    pub(crate) budget_fairness_blocked: bool,
    pub(crate) allow_pool_expansion: bool,
    pub(crate) failure_reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PoolRoleBudgetSummary {
    pub(crate) role: String,
    pub(crate) worker_count: usize,
    pub(crate) successful_worker_count: usize,
    pub(crate) feedback_worker_count: usize,
    pub(crate) feedback_applied: u64,
    pub(crate) runtime_tokens: u64,
    pub(crate) latency_ms: u64,
    pub(crate) runtime_backend: Option<String>,
    pub(crate) runtime_device: Option<String>,
    pub(crate) runtime_accelerator: Option<String>,
    pub(crate) gpu_layers: Option<u64>,
    pub(crate) blocked_primary_12b: bool,
    pub(crate) default_max_tokens: Option<u64>,
    pub(crate) configured_max_tokens: Option<u64>,
    pub(crate) effective_max_tokens: Option<u64>,
    pub(crate) max_tokens_clamped_count: usize,
    pub(crate) low_priority_worker_count: usize,
    latest_config_round: Option<u64>,
    latest_config_index: usize,
}

impl PoolRoleBudgetSummary {
    fn new(role: String) -> Self {
        Self {
            role,
            worker_count: 0,
            successful_worker_count: 0,
            feedback_worker_count: 0,
            feedback_applied: 0,
            runtime_tokens: 0,
            latency_ms: 0,
            runtime_backend: None,
            runtime_device: None,
            runtime_accelerator: None,
            gpu_layers: None,
            blocked_primary_12b: false,
            default_max_tokens: None,
            configured_max_tokens: None,
            effective_max_tokens: None,
            max_tokens_clamped_count: 0,
            low_priority_worker_count: 0,
            latest_config_round: None,
            latest_config_index: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ModelWorkerArtifact {
    round: Option<u64>,
    role: String,
    execution_state: String,
    success: bool,
    feedback_applied: u64,
    runtime_tokens: u64,
    latency_ms: u64,
    answer_chars: Option<u64>,
    answer_bytes: Option<u64>,
    answer_approx_tokens: Option<u64>,
    runtime_backend: Option<String>,
    runtime_device: Option<String>,
    runtime_accelerator: Option<String>,
    gpu_layers: Option<u64>,
    blocked_primary_12b: bool,
    default_max_tokens: Option<u64>,
    configured_max_tokens: Option<u64>,
    effective_max_tokens: Option<u64>,
    max_tokens_clamped: Option<bool>,
    can_accept_low_priority_task: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelWorkerEvent {
    pub(crate) round: usize,
    pub(crate) case_name: String,
    pub(crate) role: String,
    pub(crate) worker_port: Option<u64>,
    pub(crate) worker_base_url: Option<String>,
    pub(crate) task_kind: String,
    pub(crate) execution_state: String,
    pub(crate) success: bool,
    pub(crate) feedback_applied: u64,
    pub(crate) runtime_tokens: u64,
    pub(crate) latency_ms: u64,
    pub(crate) answer_chars: Option<u64>,
    pub(crate) answer_bytes: Option<u64>,
    pub(crate) answer_approx_tokens: Option<u64>,
    pub(crate) runtime_backend: Option<String>,
    pub(crate) runtime_device: Option<String>,
    pub(crate) runtime_accelerator: Option<String>,
    pub(crate) gpu_layers: Option<u64>,
    pub(crate) blocked_primary_12b: bool,
    pub(crate) default_max_tokens: Option<u64>,
    pub(crate) configured_max_tokens: usize,
    pub(crate) effective_max_tokens: usize,
    pub(crate) max_tokens_clamped: bool,
    pub(crate) can_accept_low_priority_task: bool,
}

pub(crate) fn load_status(path: Option<&Path>) -> Result<Option<PoolStatusSummary>, String> {
    let Some(path) = path else {
        return Ok(None);
    };
    let text = fs::read_to_string(path)
        .map_err(|error| format!("read pool status JSON {} failed: {error}", path.display()))?;
    if text.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(parse_status(&text)))
}

pub(crate) fn load_route(path: Option<&Path>) -> Result<Option<PoolRouteSummary>, String> {
    let Some(path) = path else {
        return Ok(None);
    };
    let text = fs::read_to_string(path)
        .map_err(|error| format!("read pool route JSON {} failed: {error}", path.display()))?;
    if text.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(parse_route(&text)))
}

pub(crate) fn load_budget_fairness(
    path: Option<&Path>,
) -> Result<Option<PoolBudgetFairnessSummary>, String> {
    let Some(path) = path else {
        return Ok(None);
    };
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "read pool budget fairness JSON {} failed: {error}",
                path.display()
            ));
        }
    };
    if text.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(parse_budget_fairness(&text)))
}

pub(crate) fn load_manifest(path: Option<&Path>) -> Result<Option<PoolManifestSummary>, String> {
    let Some(path) = path else {
        return Ok(None);
    };
    let text = fs::read_to_string(path)
        .map_err(|error| format!("read pool manifest JSON {} failed: {error}", path.display()))?;
    if text.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(parse_manifest(&text)))
}

pub(crate) fn append_model_worker_event(
    path: &Path,
    event: &ModelWorkerEvent,
) -> Result<(), String> {
    let mut events = existing_model_worker_event_json(path)?;
    events.push(model_worker_event_json(event));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "create pool budget fairness directory {} failed: {error}",
                parent.display()
            )
        })?;
    }
    fs::write(path, model_worker_events_artifact_json(&events)).map_err(|error| {
        format!(
            "write pool budget fairness JSON {} failed: {error}",
            path.display()
        )
    })
}

pub(crate) fn parse_status(text: &str) -> PoolStatusSummary {
    let roles = parse_worker_role_states(text);
    let fallback_worker_count = text.matches("\"port\":").count();
    let fallback_reachable_workers = text.matches("\"tcp_reachable\":true").count();
    let fallback_healthy_workers = text.matches("\"health_ok\":true").count();
    let worker_count = if roles.is_empty() {
        fallback_worker_count
    } else {
        roles.len()
    };
    let reachable_workers = if roles.is_empty() {
        fallback_reachable_workers
    } else {
        roles.iter().filter(|worker| worker.tcp_reachable).count()
    };
    let healthy_workers = if roles.is_empty() {
        fallback_healthy_workers
    } else {
        roles.iter().filter(|worker| worker.health_ok).count()
    };
    let generated_unix = json_u64_field(text, "generated_unix")
        .or_else(|| json_u64_field(text, "timestamp_unix"))
        .or_else(|| json_u64_field(text, "updated_unix"));
    let observed_unix =
        json_u64_field(text, "observed_unix").or_else(|| json_u64_field(text, "now_unix"));
    let metadata_age_seconds = json_u64_field(text, "metadata_age_seconds")
        .or_else(|| json_u64_field(text, "age_seconds"))
        .or_else(|| json_u64_field(text, "status_age_seconds"))
        .or_else(|| json_u64_field(text, "status_age_ms").map(|value| value.div_ceil(1000)))
        .or_else(|| computed_metadata_age_seconds(generated_unix, observed_unix));
    let max_age_seconds = json_u64_field(text, "max_age_seconds")
        .or_else(|| json_u64_field(text, "max_status_age_seconds"))
        .or_else(|| json_u64_field(text, "stale_after_seconds"));
    let capacity_metadata_required = json_bool_field(text, "capacity_metadata_required")
        .or_else(|| json_bool_field(text, "require_capacity_metadata"))
        .or_else(|| json_bool_field(text, "metadata_required"))
        .unwrap_or(false);
    let metadata_stale = json_bool_field(text, "metadata_stale")
        .or_else(|| json_bool_field(text, "capacity_metadata_stale"))
        .or_else(|| json_bool_field(text, "stale"))
        .unwrap_or(false)
        || metadata_age_seconds
            .zip(max_age_seconds)
            .is_some_and(|(age, max_age)| age > max_age);
    PoolStatusSummary {
        generated_unix,
        observed_unix,
        metadata_age_seconds,
        max_age_seconds,
        capacity_metadata_required,
        metadata_stale,
        launch_allowed: json_bool_field(text, "launch_allowed"),
        launch_block_reason: json_string_field(text, "launch_block_reason"),
        chain_classification: json_string_field(text, "chain_classification"),
        min_context_tokens: json_u64_field(text, "min_context_tokens"),
        capacity: parse_capacity(text),
        worker_count,
        reachable_workers,
        healthy_workers,
        roles,
    }
}

pub(crate) fn parse_route(text: &str) -> PoolRouteSummary {
    let role_candidates = json_array_field(text, "role_candidates")
        .map(|array| parse_json_string_array(&array))
        .unwrap_or_default();
    let candidate_workers = json_array_field(text, "candidate_workers").unwrap_or_default();
    let candidate_worker_states = parse_route_candidate_workers(&candidate_workers);
    let fallback_candidate_count = candidate_workers.matches("\"role\":").count();
    let fallback_healthy_candidates = candidate_workers.matches("\"health_ok\":true").count();
    let fallback_ready_candidates = candidate_workers.matches("\"role_ready\":true").count();
    let candidate_count = if candidate_worker_states.is_empty() {
        fallback_candidate_count
    } else {
        candidate_worker_states.len()
    };
    let healthy_candidates = if candidate_worker_states.is_empty() {
        fallback_healthy_candidates
    } else {
        candidate_worker_states
            .iter()
            .filter(|worker| worker.health_ok)
            .count()
    };
    let ready_candidates = if candidate_worker_states.is_empty() {
        fallback_ready_candidates
    } else {
        candidate_worker_states
            .iter()
            .filter(|worker| worker.role_ready)
            .count()
    };
    PoolRouteSummary {
        task_kind: json_string_field(text, "task_kind"),
        route_allowed: json_bool_field(text, "route_allowed"),
        route_block_reason: json_string_field(text, "route_block_reason"),
        dependency_precheck: parse_dependency_precheck(text),
        quality_context_tokens: json_u64_field(text, "quality_context_tokens"),
        quality_context_required_tokens: json_u64_field(text, "quality_context_required_tokens"),
        quality_context_sufficient: json_bool_field(text, "quality_context_sufficient"),
        quality_block_reason: json_string_field(text, "quality_block_reason"),
        selected_context_required_tokens: json_u64_field(text, "selected_context_required_tokens"),
        selected_context_buffer_tokens: json_u64_field(text, "selected_context_buffer_tokens"),
        selected_context_buffer_policy: parse_context_buffer_policy(text),
        selected_context_sufficient: json_bool_field(text, "selected_context_sufficient"),
        selected_context_block_reason: json_string_field(text, "selected_context_block_reason"),
        selected_role: json_string_field(text, "selected_role"),
        role_candidates,
        candidate_workers: candidate_worker_states,
        candidate_count,
        healthy_candidates,
        ready_candidates,
    }
}

fn parse_context_buffer_policy(text: &str) -> Option<PoolRouteContextBufferPolicySummary> {
    let policy = json_object_field(text, "selected_context_buffer_policy")?;
    Some(PoolRouteContextBufferPolicySummary {
        strategy: json_string_field(&policy, "strategy"),
        base_tokens: json_u64_field(&policy, "base_tokens"),
        upstream_role_tokens: json_u64_field(&policy, "upstream_role_tokens"),
        eligible_upstream_roles: json_array_field(&policy, "eligible_upstream_roles")
            .map(|array| parse_json_string_array(&array))
            .unwrap_or_default(),
        completed_upstream_roles: json_array_field(&policy, "completed_upstream_roles")
            .map(|array| parse_json_string_array(&array))
            .unwrap_or_default(),
        total_tokens: json_u64_field(&policy, "total_tokens"),
    })
}

fn parse_dependency_precheck(text: &str) -> Option<PoolDependencyPrecheckSummary> {
    let dependency = json_object_field(text, "dependency_precheck")?;
    Some(PoolDependencyPrecheckSummary {
        strategy: json_string_field(&dependency, "strategy"),
        checked: json_bool_field(&dependency, "checked"),
        requested_role: json_string_field(&dependency, "requested_role"),
        allow_dispatch: json_bool_field(&dependency, "allow_dispatch"),
        reason: json_string_field(&dependency, "reason"),
        required_roles: json_array_field(&dependency, "required_roles")
            .map(|array| parse_json_string_array(&array))
            .unwrap_or_default(),
        completed_roles: json_array_field(&dependency, "completed_roles")
            .map(|array| parse_json_string_array(&array))
            .unwrap_or_default(),
        missing_roles: json_array_field(&dependency, "missing_roles")
            .map(|array| parse_json_string_array(&array))
            .unwrap_or_default(),
    })
}

pub(crate) fn parse_budget_fairness(text: &str) -> PoolBudgetFairnessSummary {
    let workers = worker_array_json(text)
        .map(|workers_json| parse_model_worker_artifacts(&workers_json))
        .unwrap_or_default();
    summarize_budget_fairness_from_workers(&workers)
}

pub(crate) fn parse_manifest(text: &str) -> PoolManifestSummary {
    let workers = json_array_field(text, "workers")
        .map(|workers_json| parse_manifest_workers(&workers_json))
        .unwrap_or_default();
    let worker_count = workers.len();
    PoolManifestSummary {
        contract_version: json_string_field(text, "contract_version"),
        manifest_kind: json_string_field(text, "manifest_kind"),
        read_only: json_bool_field(text, "read_only"),
        launches_process: json_bool_field(text, "launches_process"),
        sends_prompt: json_bool_field(text, "sends_prompt"),
        capacity_policy: parse_manifest_capacity_policy(text),
        advice: parse_manifest_advice(text),
        worker_count,
        workers,
    }
}

pub(crate) fn option_status_json(summary: Option<&PoolStatusSummary>) -> String {
    summary
        .map(status_json)
        .unwrap_or_else(|| "null".to_owned())
}

pub(crate) fn option_manifest_json(summary: Option<&PoolManifestSummary>) -> String {
    summary
        .map(manifest_json)
        .unwrap_or_else(|| "null".to_owned())
}

pub(crate) fn option_route_json(summary: Option<&PoolRouteSummary>) -> String {
    summary.map(route_json).unwrap_or_else(|| "null".to_owned())
}

pub(crate) fn option_budget_fairness_json(summary: Option<&PoolBudgetFairnessSummary>) -> String {
    summary
        .map(budget_fairness_json)
        .unwrap_or_else(|| "null".to_owned())
}

pub(crate) fn budget_policy_gate_failure(
    summary: Option<&PoolBudgetFairnessSummary>,
) -> Option<String> {
    let Some(summary) = summary else {
        return Some(
            "model pool budget policy required but budget fairness summary is missing".to_owned(),
        );
    };

    let Some(quality) = summary
        .roles
        .iter()
        .find(|role| role_is_quality(&role.role) && role.worker_count > 0)
    else {
        return Some("model pool budget policy missing quality role evidence".to_owned());
    };
    if quality.max_tokens_clamped_count > 0 {
        return Some("quality role budget was clamped".to_owned());
    }
    match (quality.configured_max_tokens, quality.effective_max_tokens) {
        (Some(configured), Some(effective)) if effective == configured => {}
        (Some(configured), Some(effective)) => {
            return Some(format!(
                "quality role budget not preserved: configured_max_tokens={configured} effective_max_tokens={effective}"
            ));
        }
        _ => {
            return Some(
                "quality role budget evidence missing configured/effective max_tokens".to_owned(),
            );
        }
    }
    if let (Some(default), Some(effective)) =
        (quality.default_max_tokens, quality.effective_max_tokens)
        && effective > default
    {
        return Some(format!(
            "quality role effective_max_tokens {effective} exceeds default_max_tokens {default}"
        ));
    }

    let helper_roles = summary
        .roles
        .iter()
        .filter(|role| role_is_helper(&role.role) && role.worker_count > 0)
        .collect::<Vec<_>>();
    if helper_roles.is_empty() {
        return Some("model pool budget policy missing helper role evidence".to_owned());
    }
    let helper_roles_requiring_clamp = helper_roles
        .iter()
        .filter(|role| low_priority_helper_requires_clamp_evidence(role))
        .collect::<Vec<_>>();
    if !helper_roles_requiring_clamp.is_empty()
        && !helper_roles_requiring_clamp
            .iter()
            .any(|role| role.max_tokens_clamped_count > 0)
    {
        return Some(
            "model pool budget policy missing clamped low-priority helper evidence".to_owned(),
        );
    }

    None
}

pub(crate) fn option_alignment_json(summary: Option<&PoolAlignmentSummary>) -> String {
    summary
        .map(alignment_json)
        .unwrap_or_else(|| "null".to_owned())
}

pub(crate) fn status_context_text(summary: &PoolStatusSummary) -> String {
    format!(
        "launch_allowed:{} classification:{} reason:{} workers_reachable:{}/{} workers_healthy:{}/{} min_context_tokens:{} metadata:{} capacity:{} roles:{} available_roles:{} blocked_roles:{} advice:{}",
        option_bool_text(summary.launch_allowed),
        summary.chain_classification.as_deref().unwrap_or("?"),
        summary.launch_block_reason.as_deref().unwrap_or("none"),
        summary.reachable_workers,
        summary.worker_count,
        summary.healthy_workers,
        summary.worker_count,
        option_u64_text(summary.min_context_tokens),
        capacity_metadata_context_text(summary),
        capacity_context_text(summary.capacity.as_ref()),
        roles_context_text(&summary.roles),
        role_list_text(&summary.roles, |role| role.health_ok),
        role_list_text(&summary.roles, |role| !role.health_ok),
        status_advice_context_text(summary)
    )
}

pub(crate) fn status_advice(summary: &PoolStatusSummary) -> PoolAdviceSummary {
    let suffix = "avoid_extra_12b=true policy=one_quality_12b_plus_small_helpers";
    let context = status_advice_source_context(summary);
    if count_status_role_workers(summary, "quality") > 1
        || matches!(
            summary.launch_block_reason.as_deref(),
            Some("extra_quality_12b_workers")
        )
        || summary
            .capacity
            .as_ref()
            .and_then(|capacity| capacity.recommendation.as_deref())
            == Some("stop_extra_quality_12b_workers_keep_one_quality_plus_helpers")
    {
        return PoolAdviceSummary::blocked(
            "stop_extra_quality_12b_workers_keep_one_quality_plus_helpers",
            "extra_quality_12b_wastes_shared_apple_memory",
            format!(
                "stop extra quality 12B workers and keep one quality plus small helpers; {context}; {suffix}"
            ),
        );
    }
    if summary.launch_allowed == Some(false)
        || matches!(
            summary.chain_classification.as_deref(),
            Some("quality_worker_down")
        )
        || matches!(
            summary.launch_block_reason.as_deref(),
            Some("quality_worker_down")
        )
    {
        return PoolAdviceSummary::blocked(
            "start_or_fix_quality_worker_8686",
            "quality_worker_not_ready",
            format!("quality worker is not ready; {context}; {suffix}"),
        );
    }
    if let Some(capacity) = summary.capacity.as_ref() {
        if matches!(
            capacity.recommendation.as_deref(),
            Some("restore_quality_gate_first")
        ) {
            return PoolAdviceSummary::blocked(
                "restore_quality_gate_first",
                "capacity_gate_blocks_expansion",
                format!("restore the quality gate before adding helpers; {context}; {suffix}"),
            );
        }
        if capacity.quality_runtime_accelerated == Some(false) {
            return PoolAdviceSummary::blocked(
                "fix_quality_metal_or_gpu_layers_before_expansion",
                "quality_worker_not_gpu_accelerated",
                format!("fix Metal/GPU or gpu_layers before adding helpers; {context}; {suffix}"),
            );
        }
        if capacity.cpu_worker_count.unwrap_or(0) > 0
            || capacity.zero_gpu_layer_worker_count.unwrap_or(0) > 0
        {
            return PoolAdviceSummary::blocked(
                "hold_cpu_helpers_for_memory_pressure",
                "cpu_helpers_preserve_shared_memory",
                format!(
                    "quality worker is accelerated; keep CPU/zero-gpu helpers as low-priority memory-pressure workers and avoid adding more helpers; {context}; {suffix}"
                ),
            );
        }
    }
    let summary_ready = healthy_role(summary, "summary");
    let review_ready = healthy_role(summary, "review");
    let index_ready = healthy_role(summary, "index");
    let test_gate_ready = healthy_role(summary, "test-gate");
    if !summary_ready {
        return PoolAdviceSummary::allowed(
            "add_summary_worker_first",
            "quality_chain_ready_no_helpers_visible",
            format!(
                "quality chain is ready; add a small summary helper first; {context}; {suffix}"
            ),
        );
    }
    if summary_ready && review_ready && index_ready && test_gate_ready {
        return PoolAdviceSummary::allowed(
            "run_short_pool_smoke_then_use_evolution_loop_helper_stage_calls",
            "full_helper_pool_visible",
            format!(
                "helper pool is ready; use helper stage calls in evolution-loop; {context}; {suffix}"
            ),
        );
    }
    if review_ready || index_ready || test_gate_ready {
        return PoolAdviceSummary::allowed(
            "add_remaining_helper_roles_one_at_a_time",
            "partial_helper_pool_visible",
            format!(
                "partial helper pool is visible; add remaining summary/router/review/index/test-gate roles one at a time; {context}; {suffix}"
            ),
        );
    }
    PoolAdviceSummary::allowed(
        "add_review_or_index_after_short_smoke",
        "summary_worker_visible",
        format!(
            "summary helper is visible; run a short smoke then add review or index; {context}; {suffix}"
        ),
    )
}

pub(crate) fn status_advice_context_text(summary: &PoolStatusSummary) -> String {
    let advice = status_advice(summary);
    format!(
        "safe_to_enable_pool_workers:{} next_step:{} reason:{} kind:{} text:{}",
        advice.safe_to_enable_pool_workers,
        advice.next_step,
        advice.reason,
        advice.kind,
        advice.text
    )
}

pub(crate) fn capacity_gate_failure(summary: &PoolStatusSummary) -> Option<String> {
    let context = status_context_text(summary);
    if let Some(metadata_failure) = capacity_metadata_gate_failure(summary) {
        return Some(format!("{metadata_failure}; {context}"));
    }
    let Some(capacity) = summary.capacity.as_ref() else {
        return Some(format!("capacity missing; {context}"));
    };
    match capacity.expansion_allowed {
        Some(true) => None,
        Some(false) => {
            let recommendation = capacity.recommendation.as_deref().unwrap_or("none");
            Some(format!(
                "expansion_allowed=false recommendation={recommendation}; {context}"
            ))
        }
        None => Some(format!("expansion_allowed missing; {context}")),
    }
}

pub(crate) fn route_context_text(summary: &PoolRouteSummary) -> String {
    let selected_worker = selected_route_candidate(summary);
    let buffer_policy = context_buffer_policy_text(summary.selected_context_buffer_policy.as_ref());
    format!(
        "task_kind:{} route_allowed:{} reason:{} dependency_precheck:{} quality_context_tokens:{} quality_context_required_tokens:{} quality_context_sufficient:{} quality_block_reason:{} selected_context_required_tokens:{} selected_context_buffer_tokens:{} selected_context_buffer_policy:{} selected_context_sufficient:{} selected_context_block_reason:{} selected_role:{} selected_endpoint:{} selected_max_tokens:{} selected_runtime_backend:{} selected_runtime_device:{} selected_runtime_accelerator:{} selected_gpu_layers:{} role_candidates:{} candidates_ready:{}/{} candidates_healthy:{}/{}",
        summary.task_kind.as_deref().unwrap_or("?"),
        option_bool_text(summary.route_allowed),
        summary.route_block_reason.as_deref().unwrap_or("none"),
        dependency_precheck_context_text(summary.dependency_precheck.as_ref()),
        option_u64_text(summary.quality_context_tokens),
        option_u64_text(summary.quality_context_required_tokens),
        option_bool_text(summary.quality_context_sufficient),
        summary.quality_block_reason.as_deref().unwrap_or("none"),
        option_u64_text(summary.selected_context_required_tokens),
        option_u64_text(summary.selected_context_buffer_tokens),
        buffer_policy,
        option_bool_text(summary.selected_context_sufficient),
        summary
            .selected_context_block_reason
            .as_deref()
            .unwrap_or("none"),
        summary.selected_role.as_deref().unwrap_or("none"),
        selected_worker
            .and_then(|worker| worker.base_url.as_deref())
            .unwrap_or("none"),
        selected_worker
            .and_then(|worker| worker.default_max_tokens)
            .map(|value| value.to_string())
            .unwrap_or_else(|| "none".to_owned()),
        selected_worker
            .and_then(|worker| worker.runtime_backend.as_deref())
            .unwrap_or("none"),
        selected_worker
            .and_then(|worker| worker.runtime_device.as_deref())
            .unwrap_or("none"),
        selected_worker
            .and_then(|worker| worker.runtime_accelerator.as_deref())
            .unwrap_or("none"),
        selected_worker
            .and_then(|worker| worker.gpu_layers)
            .map(|value| value.to_string())
            .unwrap_or_else(|| "none".to_owned()),
        role_candidates_text(&summary.role_candidates),
        summary.ready_candidates,
        summary.candidate_count,
        summary.healthy_candidates,
        summary.candidate_count
    )
}

pub(crate) fn route_requires_dependency_health_check(route: &PoolRouteSummary) -> bool {
    !dependency_health_required_roles(route).is_empty()
}

pub(crate) fn route_dependency_health_failure(
    route: &PoolRouteSummary,
    status: Option<&PoolStatusSummary>,
) -> Option<String> {
    let required_roles = dependency_health_required_roles(route);
    if required_roles.is_empty() {
        return None;
    }
    let task = route.task_kind.as_deref().unwrap_or("unknown");
    let Some(status) = status else {
        return Some(format!(
            "{task}:dependency_health_status_missing:required_roles={}",
            role_candidates_text(&required_roles)
        ));
    };

    let mut missing_roles = Vec::new();
    let mut unhealthy_roles = Vec::new();
    for required_role in &required_roles {
        match status
            .roles
            .iter()
            .find(|role| role.role.eq_ignore_ascii_case(required_role))
        {
            Some(role) if role.health_ok => {}
            Some(role) => unhealthy_roles.push(format!("{}:{}", role.role, role_status(role))),
            None => missing_roles.push(required_role.clone()),
        }
    }
    if missing_roles.is_empty() && unhealthy_roles.is_empty() {
        return None;
    }

    Some(format!(
        "{task}:dependency_health_failed:required_roles={} missing_roles={} unhealthy_roles={} status_roles={}",
        role_candidates_text(&required_roles),
        role_candidates_text(&missing_roles),
        role_candidates_text(&unhealthy_roles),
        roles_context_text(&status.roles)
    ))
}

fn dependency_health_required_roles(route: &PoolRouteSummary) -> Vec<String> {
    let Some(dependency) = route.dependency_precheck.as_ref() else {
        return Vec::new();
    };
    if dependency.checked != Some(true) {
        return Vec::new();
    }
    unique_roles(
        dependency
            .required_roles
            .iter()
            .map(String::as_str)
            .filter(|role| {
                let role = role.trim();
                !role.is_empty() && role != "none"
            }),
    )
}

fn dependency_precheck_context_text(summary: Option<&PoolDependencyPrecheckSummary>) -> String {
    let Some(summary) = summary else {
        return "none".to_owned();
    };
    format!(
        "strategy:{} checked:{} requested_role:{} allow_dispatch:{} reason:{} required_roles:{} completed_roles:{} missing_roles:{}",
        summary.strategy.as_deref().unwrap_or("none"),
        option_bool_text(summary.checked),
        summary.requested_role.as_deref().unwrap_or("none"),
        option_bool_text(summary.allow_dispatch),
        summary.reason.as_deref().unwrap_or("none"),
        role_candidates_text(&summary.required_roles),
        role_candidates_text(&summary.completed_roles),
        role_candidates_text(&summary.missing_roles)
    )
}

fn context_buffer_policy_text(summary: Option<&PoolRouteContextBufferPolicySummary>) -> String {
    let Some(summary) = summary else {
        return "none".to_owned();
    };
    format!(
        "strategy:{} base_tokens:{} upstream_role_tokens:{} eligible_upstream_roles:{} completed_upstream_roles:{} total_tokens:{}",
        summary.strategy.as_deref().unwrap_or("none"),
        option_u64_text(summary.base_tokens),
        option_u64_text(summary.upstream_role_tokens),
        role_candidates_text(&summary.eligible_upstream_roles),
        role_candidates_text(&summary.completed_upstream_roles),
        option_u64_text(summary.total_tokens)
    )
}

pub(crate) fn budget_fairness_context_text(summary: &PoolBudgetFairnessSummary) -> String {
    format!(
        "workers:{} successful:{} feedback_workers:{} roles:{} total_runtime_tokens:{} total_latency_ms:{} max_role_runtime_token_share:{} budget_fairness_blocked:{} allow_pool_expansion:{} failures:{}",
        summary.worker_count,
        summary.successful_worker_count,
        summary.feedback_worker_count,
        role_budget_context_text(&summary.roles),
        summary.total_runtime_tokens,
        summary.total_latency_ms,
        option_f64_text(summary.max_role_runtime_token_share),
        summary.budget_fairness_blocked,
        summary.allow_pool_expansion,
        failure_reasons_text(&summary.failure_reasons)
    )
}

pub(crate) fn alignment_summary(
    manifest: Option<&PoolManifestSummary>,
    status: Option<&PoolStatusSummary>,
    routes: &[PoolRouteSummary],
) -> PoolAlignmentSummary {
    let manifest_roles = manifest.map(manifest_planned_roles).unwrap_or_default();
    let status_roles = status.map(status_worker_roles).unwrap_or_default();
    let quality_role = manifest_quality_role(manifest);
    let helper_roles = manifest_helper_roles(manifest);
    let manifest_advice = manifest.and_then(|summary| summary.advice.as_ref());
    let helper_role_set = role_set(&helper_roles);
    let manifest_role_set = role_set(&manifest_roles);
    let status_role_set = role_set(&status_roles);
    let manifest_quality_workers = manifest
        .map(|summary| count_manifest_role_workers(summary, &quality_role))
        .unwrap_or(0);
    let status_quality_workers = status
        .map(|summary| count_status_role_workers(summary, &quality_role))
        .unwrap_or(0);
    let manifest_helper_workers = manifest
        .map(|summary| count_manifest_roles(summary, &helper_role_set))
        .unwrap_or(0);
    let status_helper_workers = status
        .map(|summary| count_status_roles(summary, &helper_role_set))
        .unwrap_or(0);
    let mut missing_inputs = Vec::new();
    if manifest.is_none() {
        missing_inputs.push("manifest".to_owned());
    }
    if status.is_none() {
        missing_inputs.push("status".to_owned());
    }
    if routes.is_empty() {
        missing_inputs.push("routes".to_owned());
    }

    let missing_manifest_helper_roles = if manifest.is_some() {
        helper_roles
            .iter()
            .filter(|role| !manifest_role_set.contains(role.as_str()))
            .cloned()
            .collect()
    } else {
        Vec::new()
    };
    let missing_status_helper_roles = if status.is_some() {
        helper_roles
            .iter()
            .filter(|role| !status_role_set.contains(role.as_str()))
            .cloned()
            .collect()
    } else {
        Vec::new()
    };

    let (missing_status_roles, unplanned_status_roles) = if manifest.is_some() && status.is_some() {
        (
            manifest_roles
                .iter()
                .filter(|role| !status_role_set.contains(role.as_str()))
                .cloned()
                .collect(),
            status_roles
                .iter()
                .filter(|role| !manifest_role_set.contains(role.as_str()))
                .cloned()
                .collect(),
        )
    } else {
        (Vec::new(), Vec::new())
    };

    let route_blocked_or_failed = routes
        .iter()
        .filter(|route| route.route_allowed != Some(true))
        .map(|route| route.task_kind.as_deref().unwrap_or("unknown").to_owned())
        .collect::<Vec<_>>();
    let mut route_dependency_failures = Vec::new();
    for route in routes {
        if let Some(failure) = route_dependency_failure(route) {
            route_dependency_failures.push(failure);
            continue;
        }
        if let Some(failure) = route_context_buffer_policy_failure(route) {
            route_dependency_failures.push(failure);
            continue;
        }
        if let Some(failure) = route_dependency_health_failure(route, status) {
            route_dependency_failures.push(failure);
        }
    }
    let max_quality_workers = manifest
        .and_then(|summary| summary.capacity_policy.as_ref())
        .and_then(|policy| policy.max_quality_12b_workers)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(1);
    let quality_count_ok = manifest_quality_workers <= max_quality_workers
        && status_quality_workers <= max_quality_workers;
    let manifest_advice_blocks = manifest_advice.is_some_and(|advice| {
        advice.safe_to_enable_pool_workers == Some(false)
            || advice.extra_quality_12b_detected == Some(true)
    });
    let manifest_advice_worker_shape_failures = manifest_advice
        .map(|advice| {
            manifest_advice_worker_shape_failures(
                advice,
                manifest_quality_workers,
                manifest_helper_workers,
                helper_roles.len(),
            )
        })
        .unwrap_or_default();
    let alignment_ok = missing_inputs.is_empty()
        && missing_status_roles.is_empty()
        && unplanned_status_roles.is_empty()
        && missing_manifest_helper_roles.is_empty()
        && missing_status_helper_roles.is_empty()
        && route_blocked_or_failed.is_empty()
        && route_dependency_failures.is_empty()
        && quality_count_ok
        && !manifest_advice_blocks
        && manifest_advice_worker_shape_failures.is_empty();

    PoolAlignmentSummary {
        alignment_ok,
        manifest_roles,
        status_roles,
        manifest_advice_safe_to_enable_pool_workers: manifest_advice
            .and_then(|advice| advice.safe_to_enable_pool_workers),
        manifest_advice_next_step: manifest_advice.and_then(|advice| advice.next_step.clone()),
        manifest_advice_reason: manifest_advice.and_then(|advice| advice.reason.clone()),
        manifest_advice_extra_quality_12b_detected: manifest_advice
            .and_then(|advice| advice.extra_quality_12b_detected),
        manifest_advice_worker_shape_quality: manifest_advice
            .and_then(|advice| advice.worker_shape.as_ref())
            .and_then(|shape| shape.quality),
        manifest_advice_worker_shape_helpers_visible: manifest_advice
            .and_then(|advice| advice.worker_shape.as_ref())
            .and_then(|shape| shape.helpers_visible),
        manifest_advice_worker_shape_helper_target: manifest_advice
            .and_then(|advice| advice.worker_shape.as_ref())
            .and_then(|shape| shape.helper_target),
        manifest_advice_worker_shape_failures,
        manifest_quality_workers,
        status_quality_workers,
        max_quality_workers,
        manifest_helper_workers,
        status_helper_workers,
        helper_target: helper_roles.len(),
        missing_manifest_helper_roles,
        missing_status_helper_roles,
        missing_status_roles,
        unplanned_status_roles,
        route_blocked_or_failed,
        route_dependency_failures,
        missing_inputs,
    }
}

fn route_dependency_failure(route: &PoolRouteSummary) -> Option<String> {
    let dependency = route.dependency_precheck.as_ref()?;
    if dependency.allow_dispatch != Some(false) {
        return None;
    }
    let task = route.task_kind.as_deref().unwrap_or("unknown");
    let requested_role = dependency.requested_role.as_deref().unwrap_or(task);
    let reason = dependency.reason.as_deref().unwrap_or("unknown");
    Some(format!(
        "{task}:{requested_role}:{reason}:missing={}",
        role_candidates_text(&dependency.missing_roles)
    ))
}

fn route_context_buffer_policy_failure(route: &PoolRouteSummary) -> Option<String> {
    let policy = route.selected_context_buffer_policy.as_ref()?;
    let task = route.task_kind.as_deref().unwrap_or("unknown");
    let mut failures = Vec::new();

    match (route.selected_context_buffer_tokens, policy.total_tokens) {
        (Some(buffer_tokens), Some(total_tokens)) if buffer_tokens != total_tokens => {
            failures.push(format!(
                "selected_context_buffer_tokens={buffer_tokens} policy_total_tokens={total_tokens}"
            ));
        }
        (Some(_), None) => failures.push("policy_total_tokens=missing".to_owned()),
        (None, Some(_)) => failures.push("selected_context_buffer_tokens=missing".to_owned()),
        _ => {}
    }

    match (
        policy.base_tokens,
        policy.upstream_role_tokens,
        policy.total_tokens,
    ) {
        (Some(base), Some(upstream), Some(total)) => {
            let upstream_count = u64::try_from(policy.completed_upstream_roles.len()).ok()?;
            let expected = base.saturating_add(upstream.saturating_mul(upstream_count));
            if expected != total {
                failures.push(format!(
                    "computed_total_tokens={expected} policy_total_tokens={total}"
                ));
            }
        }
        (None, _, _) => failures.push("base_tokens=missing".to_owned()),
        (_, None, _) => failures.push("upstream_role_tokens=missing".to_owned()),
        (_, _, None) => {
            if !failures
                .iter()
                .any(|failure| failure == "policy_total_tokens=missing")
            {
                failures.push("policy_total_tokens=missing".to_owned());
            }
        }
    }

    if failures.is_empty() {
        return None;
    }
    Some(format!(
        "{task}:context_buffer_policy_mismatch:{}",
        failures.join("|")
    ))
}

fn manifest_advice_worker_shape_failures(
    advice: &PoolManifestAdviceSummary,
    manifest_quality_workers: usize,
    manifest_helper_workers: usize,
    helper_target: usize,
) -> Vec<String> {
    let Some(shape) = advice.worker_shape.as_ref() else {
        return vec!["worker_shape_missing".to_owned()];
    };
    let mut failures = Vec::new();
    if shape.quality.and_then(|value| usize::try_from(value).ok()) != Some(manifest_quality_workers)
    {
        failures.push(format!(
            "worker_shape_quality={} expected={manifest_quality_workers}",
            option_u64_text(shape.quality)
        ));
    }
    if shape
        .helpers_visible
        .and_then(|value| usize::try_from(value).ok())
        != Some(manifest_helper_workers)
    {
        failures.push(format!(
            "worker_shape_helpers_visible={} expected={manifest_helper_workers}",
            option_u64_text(shape.helpers_visible)
        ));
    }
    if shape
        .helper_target
        .and_then(|value| usize::try_from(value).ok())
        != Some(helper_target)
    {
        failures.push(format!(
            "worker_shape_helper_target={} expected={helper_target}",
            option_u64_text(shape.helper_target)
        ));
    }
    failures
}

pub(crate) fn alignment_context_text(summary: &PoolAlignmentSummary) -> String {
    format!(
        "alignment_ok:{} manifest_roles:{} status_roles:{} manifest_advice_safe_to_enable_pool_workers:{} manifest_advice_next_step:{} manifest_advice_reason:{} manifest_advice_extra_quality_12b_detected:{} manifest_advice_worker_shape:quality:{} helpers_visible:{} helper_target:{} worker_shape_failures:{} quality_workers:{}/{} max_quality:{} helper_workers:{}/{} helper_target:{} missing_manifest_helper_roles:{} missing_status_helper_roles:{} missing_status_roles:{} unplanned_status_roles:{} route_blocked_or_failed:{} route_dependency_failures:{} missing_inputs:{}",
        summary.alignment_ok,
        role_candidates_text(&summary.manifest_roles),
        role_candidates_text(&summary.status_roles),
        option_bool_text(summary.manifest_advice_safe_to_enable_pool_workers),
        summary
            .manifest_advice_next_step
            .as_deref()
            .unwrap_or("none"),
        summary.manifest_advice_reason.as_deref().unwrap_or("none"),
        option_bool_text(summary.manifest_advice_extra_quality_12b_detected),
        option_u64_text(summary.manifest_advice_worker_shape_quality),
        option_u64_text(summary.manifest_advice_worker_shape_helpers_visible),
        option_u64_text(summary.manifest_advice_worker_shape_helper_target),
        role_candidates_text(&summary.manifest_advice_worker_shape_failures),
        summary.status_quality_workers,
        summary.manifest_quality_workers,
        summary.max_quality_workers,
        summary.status_helper_workers,
        summary.manifest_helper_workers,
        summary.helper_target,
        role_candidates_text(&summary.missing_manifest_helper_roles),
        role_candidates_text(&summary.missing_status_helper_roles),
        role_candidates_text(&summary.missing_status_roles),
        role_candidates_text(&summary.unplanned_status_roles),
        role_candidates_text(&summary.route_blocked_or_failed),
        role_candidates_text(&summary.route_dependency_failures),
        role_candidates_text(&summary.missing_inputs)
    )
}

pub(crate) fn alignment_gate_failure(summary: &PoolAlignmentSummary) -> Option<String> {
    if summary.alignment_ok {
        return None;
    }
    let mut reasons = Vec::new();
    if !summary.missing_inputs.is_empty() {
        reasons.push(format!(
            "missing_inputs={}",
            role_candidates_text(&summary.missing_inputs)
        ));
    }
    if !summary.missing_status_roles.is_empty() {
        reasons.push(format!(
            "missing_status_roles={}",
            role_candidates_text(&summary.missing_status_roles)
        ));
    }
    if !summary.missing_manifest_helper_roles.is_empty() {
        reasons.push(format!(
            "missing_manifest_helper_roles={}",
            role_candidates_text(&summary.missing_manifest_helper_roles)
        ));
    }
    if !summary.missing_status_helper_roles.is_empty() {
        reasons.push(format!(
            "missing_status_helper_roles={}",
            role_candidates_text(&summary.missing_status_helper_roles)
        ));
    }
    if !summary.unplanned_status_roles.is_empty() {
        reasons.push(format!(
            "unplanned_status_roles={}",
            role_candidates_text(&summary.unplanned_status_roles)
        ));
    }
    if !summary.route_blocked_or_failed.is_empty() {
        reasons.push(format!(
            "route_blocked_or_failed={}",
            role_candidates_text(&summary.route_blocked_or_failed)
        ));
    }
    if !summary.route_dependency_failures.is_empty() {
        reasons.push(format!(
            "route_dependency_failures={}",
            role_candidates_text(&summary.route_dependency_failures)
        ));
    }
    if summary.manifest_advice_safe_to_enable_pool_workers == Some(false)
        || summary.manifest_advice_extra_quality_12b_detected == Some(true)
    {
        reasons.push(format!(
            "manifest_advice_blocked next_step={} reason={} extra_quality_12b_detected={}",
            summary
                .manifest_advice_next_step
                .as_deref()
                .unwrap_or("unknown"),
            summary
                .manifest_advice_reason
                .as_deref()
                .unwrap_or("unknown"),
            option_bool_text(summary.manifest_advice_extra_quality_12b_detected)
        ));
    }
    if !summary.manifest_advice_worker_shape_failures.is_empty() {
        reasons.push(format!(
            "manifest_advice_worker_shape_mismatch={}",
            role_candidates_text(&summary.manifest_advice_worker_shape_failures)
        ));
    }
    if summary.manifest_quality_workers > summary.max_quality_workers {
        reasons.push(format!(
            "manifest_quality_workers={} exceeds max_quality_workers={}",
            summary.manifest_quality_workers, summary.max_quality_workers
        ));
    }
    if summary.status_quality_workers > summary.max_quality_workers {
        reasons.push(format!(
            "status_quality_workers={} exceeds max_quality_workers={}",
            summary.status_quality_workers, summary.max_quality_workers
        ));
    }
    if reasons.is_empty() {
        reasons.push("unknown_alignment_failure".to_owned());
    }
    Some(reasons.join("; "))
}

pub(crate) fn manifest_context_text(summary: &PoolManifestSummary) -> String {
    format!(
        "contract_version:{} kind:{} read_only:{} launches_process:{} sends_prompt:{} capacity_policy:{} advice:{} workers:{}",
        summary.contract_version.as_deref().unwrap_or("none"),
        summary.manifest_kind.as_deref().unwrap_or("none"),
        option_bool_text(summary.read_only),
        option_bool_text(summary.launches_process),
        option_bool_text(summary.sends_prompt),
        manifest_capacity_policy_context_text(summary.capacity_policy.as_ref()),
        manifest_advice_context_text(summary.advice.as_ref()),
        manifest_workers_context_text(&summary.workers)
    )
}

pub(crate) fn selected_route_candidate(summary: &PoolRouteSummary) -> Option<&PoolRouteCandidate> {
    let selected_role = summary
        .selected_role
        .as_deref()
        .map(str::trim)
        .filter(|role| !role.is_empty())
        .filter(|role| *role != "none")?;
    summary.candidate_workers.iter().find(|worker| {
        worker.role == selected_role
            && worker.role_ready
            && !worker
                .model
                .as_deref()
                .is_some_and(model_policy::is_gpt5_series_model)
    })
}

impl PoolAdviceSummary {
    fn blocked(next_step: &'static str, reason: &'static str, text: String) -> Self {
        Self {
            safe_to_enable_pool_workers: false,
            next_step,
            reason,
            kind: "error",
            text,
        }
    }

    fn allowed(next_step: &'static str, reason: &'static str, text: String) -> Self {
        Self {
            safe_to_enable_pool_workers: true,
            next_step,
            reason,
            kind: "busy",
            text,
        }
    }
}

fn healthy_role(summary: &PoolStatusSummary, role: &str) -> bool {
    summary
        .roles
        .iter()
        .any(|state| state.role == role && state.health_ok)
}

fn status_advice_source_context(summary: &PoolStatusSummary) -> String {
    let capacity = summary.capacity.as_ref();
    format!(
        "launch_allowed={} classification={} recommendation={} helpers={}/{} cpu_workers={} gpu0_workers={} quality_accelerated={}",
        option_bool_text(summary.launch_allowed),
        summary.chain_classification.as_deref().unwrap_or("?"),
        capacity
            .and_then(|capacity| capacity.recommendation.as_deref())
            .unwrap_or("none"),
        option_u64_text(capacity.and_then(|capacity| capacity.healthy_helper_worker_count)),
        option_u64_text(capacity.and_then(|capacity| capacity.helper_worker_count)),
        option_u64_text(capacity.and_then(|capacity| capacity.cpu_worker_count)),
        option_u64_text(capacity.and_then(|capacity| capacity.zero_gpu_layer_worker_count)),
        option_bool_text(capacity.and_then(|capacity| capacity.quality_runtime_accelerated))
    )
}

fn parse_worker_role_states(text: &str) -> Vec<PoolWorkerRoleState> {
    let Some(workers_json) = json_array_field(text, "workers") else {
        return Vec::new();
    };
    json_object_items(&workers_json)
        .into_iter()
        .filter_map(|worker_json| {
            let role = json_string_field(worker_json, "role")?;
            Some(PoolWorkerRoleState {
                role,
                port: json_u64_field(worker_json, "port"),
                base_url: json_string_field(worker_json, "base_url"),
                tcp_reachable: json_bool_field(worker_json, "tcp_reachable").unwrap_or(false),
                health_ok: json_bool_field(worker_json, "health_ok").unwrap_or(false),
                ready: json_bool_field(worker_json, "ready").unwrap_or(false),
                role_ready: json_bool_field(worker_json, "role_ready").unwrap_or(false),
                status: json_string_field(worker_json, "status"),
                role_block_reason: json_string_field(worker_json, "role_block_reason"),
                low_priority: json_bool_field(worker_json, "low_priority"),
                can_accept_low_priority_task: json_bool_field(
                    worker_json,
                    "can_accept_low_priority_task",
                ),
                model: json_string_field(worker_json, "model"),
                context_window: json_u64_field(worker_json, "context_window")
                    .or_else(|| json_u64_field(worker_json, "default_context_tokens")),
                runtime_backend: json_string_field(worker_json, "runtime_backend"),
                runtime_device: json_string_field(worker_json, "runtime_device"),
                runtime_accelerator: json_string_field(worker_json, "runtime_accelerator"),
                gpu_layers: json_u64_field(worker_json, "gpu_layers"),
                route_count: json_u64_field(worker_json, "route_count"),
                selected_count: json_u64_field(worker_json, "selected_count"),
                blocked_count: json_u64_field(worker_json, "blocked_count"),
                in_flight: json_u64_field(worker_json, "in_flight"),
                queued_count: json_u64_field(worker_json, "queued_count")
                    .or_else(|| json_u64_field(worker_json, "queue_depth"))
                    .or_else(|| json_u64_field(worker_json, "queued")),
                lease_wait_ms: json_u64_field(worker_json, "lease_wait_ms")
                    .or_else(|| json_u64_field(worker_json, "lease_wait_elapsed_ms")),
                lease_wait_p95_ms: json_u64_field(worker_json, "lease_wait_p95_ms")
                    .or_else(|| json_u64_field(worker_json, "lease_wait_p95"))
                    .or_else(|| json_u64_field(worker_json, "lease_wait_p95_millis")),
                success_count: json_u64_field(worker_json, "success_count"),
                failure_count: json_u64_field(worker_json, "failure_count"),
                avg_latency_ms: json_u64_field(worker_json, "avg_latency_ms"),
                latency_p50_ms: json_u64_field(worker_json, "latency_p50_ms")
                    .or_else(|| json_u64_field(worker_json, "p50_latency_ms")),
                latency_p95_ms: json_u64_field(worker_json, "latency_p95_ms")
                    .or_else(|| json_u64_field(worker_json, "p95_latency_ms")),
            })
        })
        .collect()
}

fn parse_capacity(text: &str) -> Option<PoolCapacitySummary> {
    let capacity_json = json_object_field(text, "capacity")?;
    Some(PoolCapacitySummary {
        policy: json_string_field(&capacity_json, "policy"),
        expansion_allowed: json_bool_field(&capacity_json, "expansion_allowed"),
        recommendation: json_string_field(&capacity_json, "recommendation"),
        worker_count: json_u64_field(&capacity_json, "worker_count"),
        healthy_worker_count: json_u64_field(&capacity_json, "healthy_worker_count"),
        helper_worker_count: json_u64_field(&capacity_json, "helper_worker_count"),
        healthy_helper_worker_count: json_u64_field(&capacity_json, "healthy_helper_worker_count"),
        metal_worker_count: json_u64_field(&capacity_json, "metal_worker_count"),
        cpu_worker_count: json_u64_field(&capacity_json, "cpu_worker_count"),
        unknown_runtime_worker_count: json_u64_field(
            &capacity_json,
            "unknown_runtime_worker_count",
        ),
        zero_gpu_layer_worker_count: json_u64_field(&capacity_json, "zero_gpu_layer_worker_count"),
        quality_runtime_accelerated: json_bool_field(&capacity_json, "quality_runtime_accelerated"),
    })
}

fn parse_manifest_capacity_policy(text: &str) -> Option<PoolManifestCapacityPolicySummary> {
    let policy_json = json_object_field(text, "capacity_policy")?;
    Some(PoolManifestCapacityPolicySummary {
        policy: json_string_field(&policy_json, "policy"),
        target_host: json_string_field(&policy_json, "target_host"),
        avoid_extra_12b: json_bool_field(&policy_json, "avoid_extra_12b"),
        max_quality_12b_workers: json_u64_field(&policy_json, "max_quality_12b_workers"),
        quality_role: json_string_field(&policy_json, "quality_role"),
        quality_required_context_tokens: json_u64_field(
            &policy_json,
            "quality_required_context_tokens",
        ),
        helper_roles: json_array_field(&policy_json, "helper_roles")
            .map(|array| parse_json_string_array(&array))
            .unwrap_or_default(),
        helper_context_tokens_total: json_u64_field(&policy_json, "helper_context_tokens_total"),
        helper_default_max_tokens_total: json_u64_field(
            &policy_json,
            "helper_default_max_tokens_total",
        ),
        recommended_launch_order: json_array_field(&policy_json, "recommended_launch_order")
            .map(|array| parse_json_string_array(&array))
            .unwrap_or_default(),
        expansion_gate: json_string_field(&policy_json, "expansion_gate"),
        next_step_when_quality_ready: json_string_field(
            &policy_json,
            "next_step_when_quality_ready",
        ),
    })
}

fn parse_manifest_advice(text: &str) -> Option<PoolManifestAdviceSummary> {
    let advice_json = json_object_field(text, "advice")?;
    Some(PoolManifestAdviceSummary {
        decision_source: json_string_field(&advice_json, "decision_source"),
        policy: json_string_field(&advice_json, "policy"),
        safe_to_enable_pool_workers: json_bool_field(&advice_json, "safe_to_enable_pool_workers"),
        next_step: json_string_field(&advice_json, "next_step"),
        reason: json_string_field(&advice_json, "reason"),
        kind: json_string_field(&advice_json, "kind"),
        extra_quality_12b_detected: json_bool_field(&advice_json, "extra_quality_12b_detected"),
        quality_worker_count: json_u64_field(&advice_json, "quality_worker_count"),
        helper_worker_count: json_u64_field(&advice_json, "helper_worker_count"),
        helper_target_worker_count: json_u64_field(&advice_json, "helper_target_worker_count"),
        helper_roles: json_array_field(&advice_json, "helper_roles")
            .map(|array| parse_json_string_array(&array))
            .unwrap_or_default(),
        worker_shape: parse_manifest_worker_shape(&advice_json),
    })
}

fn parse_manifest_worker_shape(advice_json: &str) -> Option<PoolManifestWorkerShapeSummary> {
    let shape_json = json_object_field(advice_json, "worker_shape")?;
    Some(PoolManifestWorkerShapeSummary {
        quality: json_u64_field(&shape_json, "quality"),
        helpers_visible: json_u64_field(&shape_json, "helpers_visible"),
        helper_target: json_u64_field(&shape_json, "helper_target"),
    })
}

fn parse_manifest_workers(workers_json: &str) -> Vec<PoolManifestWorkerSummary> {
    json_object_items(workers_json)
        .into_iter()
        .filter_map(|worker_json| {
            let role = json_string_field(worker_json, "role")?;
            let role = role.trim().to_owned();
            if role.is_empty() {
                return None;
            }
            Some(PoolManifestWorkerSummary {
                role,
                port: json_u64_field(worker_json, "port"),
                base_url: json_string_field(worker_json, "base_url"),
                default_context_tokens: json_u64_field(worker_json, "default_context_tokens")
                    .or_else(|| json_u64_field(worker_json, "context_window"))
                    .or_else(|| json_u64_field(worker_json, "runtime_context_window")),
                default_max_tokens: json_u64_field(worker_json, "default_max_tokens")
                    .or_else(|| json_u64_field(worker_json, "max_tokens")),
                enabled_by_default: json_bool_field(worker_json, "enabled_by_default"),
                low_priority: json_bool_field(worker_json, "low_priority"),
                runtime_backend: json_string_field(worker_json, "runtime_backend")
                    .or_else(|| json_string_field(worker_json, "backend"))
                    .or_else(|| json_string_field(worker_json, "engine")),
                runtime_device: json_string_field(worker_json, "runtime_device")
                    .or_else(|| json_string_field(worker_json, "device"))
                    .or_else(|| json_string_field(worker_json, "device_profile"))
                    .or_else(|| json_string_field(worker_json, "execution_device")),
                runtime_accelerator: json_string_field(worker_json, "runtime_accelerator")
                    .or_else(|| json_string_field(worker_json, "accelerator"))
                    .or_else(|| json_string_field(worker_json, "device_accelerator")),
                gpu_layers: json_u64_field(worker_json, "gpu_layers")
                    .or_else(|| json_u64_field(worker_json, "n_gpu_layers"))
                    .or_else(|| json_u64_field(worker_json, "offloaded_gpu_layers")),
            })
        })
        .collect()
}

fn parse_route_candidate_workers(candidate_workers_json: &str) -> Vec<PoolRouteCandidate> {
    if candidate_workers_json.trim().is_empty() {
        return Vec::new();
    }
    json_object_items(candidate_workers_json)
        .into_iter()
        .filter_map(|worker_json| {
            let role = json_string_field(worker_json, "role")?;
            Some(PoolRouteCandidate {
                port: json_u64_field(worker_json, "port"),
                role,
                base_url: json_string_field(worker_json, "base_url"),
                tcp_reachable: json_bool_field(worker_json, "tcp_reachable").unwrap_or(false),
                health_ok: json_bool_field(worker_json, "health_ok").unwrap_or(false),
                status: json_string_field(worker_json, "status"),
                role_ready: json_bool_field(worker_json, "role_ready").unwrap_or(false),
                role_block_reason: json_string_field(worker_json, "role_block_reason"),
                can_accept_low_priority_task: json_bool_field(
                    worker_json,
                    "can_accept_low_priority_task",
                )
                .unwrap_or(false),
                model: json_string_field(worker_json, "model"),
                context_window: json_u64_field(worker_json, "context_window"),
                runtime_backend: json_string_field(worker_json, "runtime_backend"),
                runtime_device: json_string_field(worker_json, "runtime_device"),
                runtime_accelerator: json_string_field(worker_json, "runtime_accelerator"),
                gpu_layers: json_u64_field(worker_json, "gpu_layers"),
                default_max_tokens: json_u64_field(worker_json, "default_max_tokens"),
            })
        })
        .collect()
}

fn worker_array_json(text: &str) -> Option<String> {
    let trimmed = text.trim_start();
    if trimmed.starts_with('[') {
        return Some(trimmed.to_owned());
    }
    json_array_field(text, "workers")
        .or_else(|| json_array_field(text, "model_workers"))
        .or_else(|| json_array_field(text, "model_worker_v1"))
}

fn parse_model_worker_artifacts(workers_json: &str) -> Vec<ModelWorkerArtifact> {
    json_object_items(workers_json)
        .into_iter()
        .filter_map(|worker_json| {
            let role = json_string_field(worker_json, "role")
                .or_else(|| json_string_field(worker_json, "selected_role"))?;
            let role = role.trim().to_owned();
            if role.is_empty() {
                return None;
            }
            let execution_state = json_string_field(worker_json, "execution_state")
                .unwrap_or_else(|| "executed".to_owned());
            if execution_state.trim().eq_ignore_ascii_case("planned")
                || json_bool_field(worker_json, "planned_only").unwrap_or(false)
            {
                return None;
            }
            let answer_approx_tokens = json_u64_field(worker_json, "answer_approx_tokens");
            Some(ModelWorkerArtifact {
                round: json_u64_field(worker_json, "round"),
                role,
                execution_state,
                success: json_bool_field(worker_json, "success")
                    .or_else(|| json_bool_field(worker_json, "ok"))
                    .unwrap_or(false),
                feedback_applied: json_u64_field(worker_json, "feedback_applied").unwrap_or(0),
                runtime_tokens: runtime_tokens_with_answer_fallback(
                    json_u64_field(worker_json, "runtime_tokens"),
                    answer_approx_tokens,
                ),
                latency_ms: json_u64_field(worker_json, "latency_ms")
                    .or_else(|| json_u64_field(worker_json, "elapsed_ms"))
                    .unwrap_or(0),
                answer_chars: json_u64_field(worker_json, "answer_chars"),
                answer_bytes: json_u64_field(worker_json, "answer_bytes"),
                answer_approx_tokens,
                runtime_backend: json_string_field(worker_json, "runtime_backend"),
                runtime_device: json_string_field(worker_json, "runtime_device"),
                runtime_accelerator: json_string_field(worker_json, "runtime_accelerator"),
                gpu_layers: json_u64_field(worker_json, "gpu_layers"),
                blocked_primary_12b: json_bool_field(worker_json, "blocked_primary_12b")
                    .unwrap_or(false),
                default_max_tokens: json_u64_field(worker_json, "default_max_tokens"),
                configured_max_tokens: json_u64_field(worker_json, "configured_max_tokens"),
                effective_max_tokens: json_u64_field(worker_json, "effective_max_tokens"),
                max_tokens_clamped: json_bool_field(worker_json, "max_tokens_clamped"),
                can_accept_low_priority_task: json_bool_field(
                    worker_json,
                    "can_accept_low_priority_task",
                ),
            })
        })
        .collect()
}

fn runtime_tokens_with_answer_fallback(
    runtime_tokens: Option<u64>,
    answer_approx_tokens: Option<u64>,
) -> u64 {
    match runtime_tokens {
        Some(tokens) if tokens > 0 => tokens,
        _ => answer_approx_tokens.unwrap_or(0),
    }
}

fn existing_model_worker_event_json(path: &Path) -> Result<Vec<String>, String> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(format!(
                "read pool budget fairness JSON {} failed: {error}",
                path.display()
            ));
        }
    };
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }
    let Some(workers_json) = worker_array_json(&text) else {
        return Ok(Vec::new());
    };
    Ok(json_object_items(&workers_json)
        .into_iter()
        .map(ToOwned::to_owned)
        .collect())
}

fn summarize_budget_fairness_from_workers(
    workers: &[ModelWorkerArtifact],
) -> PoolBudgetFairnessSummary {
    let mut roles_by_name = std::collections::BTreeMap::<String, PoolRoleBudgetSummary>::new();
    let mut successful_worker_count = 0usize;
    let mut feedback_worker_count = 0usize;
    let mut total_runtime_tokens = 0u64;
    let mut total_latency_ms = 0u64;

    for (worker_index, worker) in workers.iter().enumerate() {
        if worker.success {
            successful_worker_count += 1;
        }
        if worker.success && worker.feedback_applied > 0 {
            feedback_worker_count += 1;
        }
        total_runtime_tokens = total_runtime_tokens.saturating_add(worker.runtime_tokens);
        total_latency_ms = total_latency_ms.saturating_add(worker.latency_ms);

        let role = roles_by_name
            .entry(worker.role.clone())
            .or_insert_with(|| PoolRoleBudgetSummary::new(worker.role.clone()));
        role.worker_count += 1;
        if worker.success {
            role.successful_worker_count += 1;
        }
        if worker.success && worker.feedback_applied > 0 {
            role.feedback_worker_count += 1;
        }
        role.feedback_applied = role
            .feedback_applied
            .saturating_add(worker.feedback_applied);
        role.runtime_tokens = role.runtime_tokens.saturating_add(worker.runtime_tokens);
        role.latency_ms = role.latency_ms.saturating_add(worker.latency_ms);
        role.blocked_primary_12b |= worker.blocked_primary_12b;
        update_latest_role_config(role, worker, worker_index);
        if worker.max_tokens_clamped == Some(true) {
            role.max_tokens_clamped_count += 1;
        }
        if worker.can_accept_low_priority_task == Some(true) {
            role.low_priority_worker_count += 1;
        }
    }

    let roles = roles_by_name.into_values().collect::<Vec<_>>();
    let max_role_runtime_token_share = if total_runtime_tokens == 0 {
        None
    } else {
        roles
            .iter()
            .map(|role| role.runtime_tokens as f64 / total_runtime_tokens as f64)
            .max_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal))
    };
    let failure_reasons = budget_fairness_failure_reasons(workers, &roles, total_runtime_tokens);

    PoolBudgetFairnessSummary {
        worker_count: workers.len(),
        successful_worker_count,
        feedback_worker_count,
        roles,
        total_runtime_tokens,
        total_latency_ms,
        max_role_runtime_token_share,
        budget_fairness_blocked: !failure_reasons.is_empty(),
        allow_pool_expansion: failure_reasons.is_empty(),
        failure_reasons,
    }
}

fn update_latest_role_config(
    role: &mut PoolRoleBudgetSummary,
    worker: &ModelWorkerArtifact,
    worker_index: usize,
) {
    let worker_round = worker.round.unwrap_or(0);
    let is_newer = match role.latest_config_round {
        Some(latest_round) => {
            worker_round > latest_round
                || (worker_round == latest_round && worker_index >= role.latest_config_index)
        }
        None => true,
    };
    if !is_newer {
        return;
    }

    role.latest_config_round = Some(worker_round);
    role.latest_config_index = worker_index;
    role.runtime_backend = worker.runtime_backend.clone();
    role.runtime_device = worker.runtime_device.clone();
    role.runtime_accelerator = worker.runtime_accelerator.clone();
    role.gpu_layers = worker.gpu_layers;
    role.default_max_tokens = worker.default_max_tokens;
    role.configured_max_tokens = worker.configured_max_tokens;
    role.effective_max_tokens = worker.effective_max_tokens;
}

fn budget_fairness_failure_reasons(
    workers: &[ModelWorkerArtifact],
    roles: &[PoolRoleBudgetSummary],
    total_runtime_tokens: u64,
) -> Vec<String> {
    let mut failures = Vec::new();
    if workers.is_empty() {
        failures.push("no model_worker_v1 records".to_owned());
    }
    if total_runtime_tokens == 0 {
        failures.push("pool runtime tokens are zero".to_owned());
    }
    for (required, aliases) in [
        ("planner", &["planner", "summary"][..]),
        ("reviewer", &["reviewer", "review"][..]),
        ("tester", &["tester", "test-gate", "test"][..]),
    ] {
        if !roles
            .iter()
            .any(|role| role_matches_any(&role.role, aliases) && role.feedback_worker_count > 0)
        {
            failures.push(format!(
                "required role {required} has no successful feedback-bearing worker"
            ));
        }
    }
    if total_runtime_tokens > 0 {
        for role in roles {
            if !role_is_helper(&role.role) {
                continue;
            }
            let share = role.runtime_tokens as f64 / total_runtime_tokens as f64;
            if role_runtime_token_share_exceeds_limit(share) {
                failures.push(format!(
                    "helper role {} runtime token share {:.3} exceeds {:.2}",
                    role.role, share, MAX_ROLE_RUNTIME_TOKEN_SHARE
                ));
            }
        }
    }
    for role in roles {
        if role.blocked_primary_12b && role_is_helper(&role.role) {
            failures.push(format!(
                "helper role {} blocked primary 12B path",
                role.role
            ));
        }
    }
    for worker in workers {
        if role_is_helper(&worker.role)
            && worker.can_accept_low_priority_task == Some(true)
            && let (Some(configured), Some(effective)) =
                (worker.configured_max_tokens, worker.effective_max_tokens)
            && configured > effective
            && worker.max_tokens_clamped != Some(true)
        {
            failures.push(format!(
                "helper role {} reduced max_tokens from {} to {} without clamp evidence",
                worker.role, configured, effective
            ));
        }
        if role_is_helper(&worker.role)
            && worker.can_accept_low_priority_task == Some(true)
            && let (Some(configured), Some(default), Some(effective)) = (
                worker.configured_max_tokens,
                worker.default_max_tokens,
                worker.effective_max_tokens,
            )
            && configured > default
            && effective > default
        {
            failures.push(format!(
                "helper role {} exceeded worker default max_tokens {} with effective {}",
                worker.role, default, effective
            ));
        }
        if worker.role.eq_ignore_ascii_case("quality") && worker.max_tokens_clamped == Some(true) {
            failures.push("quality role budget was clamped".to_owned());
        }
    }
    failures
}

fn role_runtime_token_share_exceeds_limit(share: f64) -> bool {
    share > MAX_ROLE_RUNTIME_TOKEN_SHARE + ROLE_RUNTIME_TOKEN_SHARE_EPSILON
}

fn low_priority_helper_requires_clamp_evidence(role: &PoolRoleBudgetSummary) -> bool {
    if role.low_priority_worker_count == 0 || !role_is_helper(&role.role) {
        return false;
    }
    match (
        role.configured_max_tokens,
        role.default_max_tokens,
        role.effective_max_tokens,
    ) {
        (Some(configured), Some(default), _) => configured > default,
        (Some(configured), None, Some(effective)) => configured > effective,
        _ => false,
    }
}

fn role_matches_any(role: &str, aliases: &[&str]) -> bool {
    let role = role.to_ascii_lowercase();
    aliases.iter().any(|alias| role == *alias)
}

fn role_is_helper(role: &str) -> bool {
    !role_is_quality(role)
}

fn role_is_quality(role: &str) -> bool {
    matches!(
        role.to_ascii_lowercase().as_str(),
        "quality" | "remote-quality" | "primary" | "primary-12b"
    )
}

fn json_object_items(array_json: &str) -> Vec<&str> {
    let mut objects = Vec::new();
    let mut depth = 0usize;
    let mut start = None;
    let mut in_string = false;
    let mut escaped = false;
    for (index, character) in array_json.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }

        match character {
            '"' => in_string = true,
            '{' => {
                if depth == 0 {
                    start = Some(index);
                }
                depth = depth.saturating_add(1);
            }
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0
                    && let Some(start_index) = start.take()
                    && let Some(object) = array_json.get(start_index..=index)
                {
                    objects.push(object);
                }
            }
            _ => {}
        }
    }
    objects
}

fn manifest_json(summary: &PoolManifestSummary) -> String {
    format!(
        "{{\"contract_version\":{},\"manifest_kind\":{},\"read_only\":{},\"launches_process\":{},\"sends_prompt\":{},\"capacity_policy\":{},\"advice\":{},\"workers\":{{\"total\":{},\"items\":{}}}}}",
        option_str_json(summary.contract_version.as_deref()),
        option_str_json(summary.manifest_kind.as_deref()),
        option_bool_json(summary.read_only),
        option_bool_json(summary.launches_process),
        option_bool_json(summary.sends_prompt),
        option_manifest_capacity_policy_json(summary.capacity_policy.as_ref()),
        option_manifest_advice_json(summary.advice.as_ref()),
        summary.worker_count,
        manifest_workers_json(&summary.workers)
    )
}

fn option_manifest_capacity_policy_json(
    summary: Option<&PoolManifestCapacityPolicySummary>,
) -> String {
    summary
        .map(manifest_capacity_policy_json)
        .unwrap_or_else(|| "null".to_owned())
}

fn manifest_capacity_policy_json(summary: &PoolManifestCapacityPolicySummary) -> String {
    format!(
        "{{\"policy\":{},\"target_host\":{},\"avoid_extra_12b\":{},\"max_quality_12b_workers\":{},\"quality_role\":{},\"quality_required_context_tokens\":{},\"helper_roles\":{},\"helper_context_tokens_total\":{},\"helper_default_max_tokens_total\":{},\"recommended_launch_order\":{},\"expansion_gate\":{},\"next_step_when_quality_ready\":{}}}",
        option_str_json(summary.policy.as_deref()),
        option_str_json(summary.target_host.as_deref()),
        option_bool_json(summary.avoid_extra_12b),
        option_u64_json(summary.max_quality_12b_workers),
        option_str_json(summary.quality_role.as_deref()),
        option_u64_json(summary.quality_required_context_tokens),
        json_string_array(&summary.helper_roles),
        option_u64_json(summary.helper_context_tokens_total),
        option_u64_json(summary.helper_default_max_tokens_total),
        json_string_array(&summary.recommended_launch_order),
        option_str_json(summary.expansion_gate.as_deref()),
        option_str_json(summary.next_step_when_quality_ready.as_deref())
    )
}

fn option_manifest_advice_json(summary: Option<&PoolManifestAdviceSummary>) -> String {
    summary
        .map(manifest_advice_json)
        .unwrap_or_else(|| "null".to_owned())
}

fn manifest_advice_json(summary: &PoolManifestAdviceSummary) -> String {
    format!(
        "{{\"decision_source\":{},\"policy\":{},\"safe_to_enable_pool_workers\":{},\"next_step\":{},\"reason\":{},\"kind\":{},\"extra_quality_12b_detected\":{},\"quality_worker_count\":{},\"helper_worker_count\":{},\"helper_target_worker_count\":{},\"helper_roles\":{},\"worker_shape\":{}}}",
        option_str_json(summary.decision_source.as_deref()),
        option_str_json(summary.policy.as_deref()),
        option_bool_json(summary.safe_to_enable_pool_workers),
        option_str_json(summary.next_step.as_deref()),
        option_str_json(summary.reason.as_deref()),
        option_str_json(summary.kind.as_deref()),
        option_bool_json(summary.extra_quality_12b_detected),
        option_u64_json(summary.quality_worker_count),
        option_u64_json(summary.helper_worker_count),
        option_u64_json(summary.helper_target_worker_count),
        json_string_array(&summary.helper_roles),
        option_manifest_worker_shape_json(summary.worker_shape.as_ref())
    )
}

fn option_manifest_worker_shape_json(summary: Option<&PoolManifestWorkerShapeSummary>) -> String {
    summary
        .map(manifest_worker_shape_json)
        .unwrap_or_else(|| "null".to_owned())
}

fn manifest_worker_shape_json(summary: &PoolManifestWorkerShapeSummary) -> String {
    format!(
        "{{\"quality\":{},\"helpers_visible\":{},\"helper_target\":{}}}",
        option_u64_json(summary.quality),
        option_u64_json(summary.helpers_visible),
        option_u64_json(summary.helper_target)
    )
}

fn manifest_workers_json(workers: &[PoolManifestWorkerSummary]) -> String {
    let items = workers
        .iter()
        .map(manifest_worker_json)
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

fn manifest_worker_json(worker: &PoolManifestWorkerSummary) -> String {
    format!(
        "{{\"role\":{},\"port\":{},\"base_url\":{},\"default_context_tokens\":{},\"default_max_tokens\":{},\"enabled_by_default\":{},\"low_priority\":{},\"runtime_backend\":{},\"runtime_device\":{},\"runtime_accelerator\":{},\"gpu_layers\":{}}}",
        json_string(&worker.role),
        option_u64_json(worker.port),
        option_str_json(worker.base_url.as_deref()),
        option_u64_json(worker.default_context_tokens),
        option_u64_json(worker.default_max_tokens),
        option_bool_json(worker.enabled_by_default),
        option_bool_json(worker.low_priority),
        option_str_json(worker.runtime_backend.as_deref()),
        option_str_json(worker.runtime_device.as_deref()),
        option_str_json(worker.runtime_accelerator.as_deref()),
        option_u64_json(worker.gpu_layers)
    )
}

fn budget_fairness_json(summary: &PoolBudgetFairnessSummary) -> String {
    format!(
        "{{\"schema\":\"model_pool_budget_fairness_report_v1\",\"workers\":{{\"total\":{},\"successful\":{},\"feedback_bearing\":{}}},\"roles\":{},\"total_runtime_tokens\":{},\"total_latency_ms\":{},\"max_role_runtime_token_share\":{},\"budget_fairness_blocked\":{},\"allow_pool_expansion\":{},\"failure_reasons\":{}}}",
        summary.worker_count,
        summary.successful_worker_count,
        summary.feedback_worker_count,
        role_budget_array_json(&summary.roles),
        summary.total_runtime_tokens,
        summary.total_latency_ms,
        option_f64_json(summary.max_role_runtime_token_share),
        summary.budget_fairness_blocked,
        summary.allow_pool_expansion,
        json_string_array(&summary.failure_reasons)
    )
}

fn model_worker_events_artifact_json(events: &[String]) -> String {
    format!(
        "{{\"schema\":\"model_worker_v1\",\"workers\":[{}]}}\n",
        events.join(",")
    )
}

fn model_worker_event_json(event: &ModelWorkerEvent) -> String {
    format!(
        "{{\"worker_id\":{},\"round\":{},\"case\":{},\"role\":{},\"worker_port\":{},\"worker_base_url\":{},\"task_kind\":{},\"execution_state\":{},\"success\":{},\"feedback_applied\":{},\"runtime_tokens\":{},\"latency_ms\":{},\"answer_chars\":{},\"answer_bytes\":{},\"answer_approx_tokens\":{},\"runtime_backend\":{},\"runtime_device\":{},\"runtime_accelerator\":{},\"gpu_layers\":{},\"default_max_tokens\":{},\"configured_max_tokens\":{},\"effective_max_tokens\":{},\"max_tokens_clamped\":{},\"can_accept_low_priority_task\":{},\"blocked_primary_12b\":{}}}",
        json_string(&format!(
            "round-{}-{}-{}",
            event.round,
            event
                .role
                .replace(|character: char| !character.is_ascii_alphanumeric(), "-"),
            event
                .execution_state
                .replace(|character: char| !character.is_ascii_alphanumeric(), "-")
        )),
        event.round,
        json_string(&event.case_name),
        json_string(&event.role),
        option_u64_json(event.worker_port),
        option_str_json(event.worker_base_url.as_deref()),
        json_string(&event.task_kind),
        json_string(&event.execution_state),
        event.success,
        event.feedback_applied,
        event.runtime_tokens,
        event.latency_ms,
        option_u64_json(event.answer_chars),
        option_u64_json(event.answer_bytes),
        option_u64_json(event.answer_approx_tokens),
        option_str_json(event.runtime_backend.as_deref()),
        option_str_json(event.runtime_device.as_deref()),
        option_str_json(event.runtime_accelerator.as_deref()),
        option_u64_json(event.gpu_layers),
        option_u64_json(event.default_max_tokens),
        event.configured_max_tokens,
        event.effective_max_tokens,
        event.max_tokens_clamped,
        event.can_accept_low_priority_task,
        event.blocked_primary_12b
    )
}

fn role_budget_array_json(roles: &[PoolRoleBudgetSummary]) -> String {
    let items = roles
        .iter()
        .map(role_budget_json)
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

fn role_budget_json(role: &PoolRoleBudgetSummary) -> String {
    format!(
        "{{\"role\":{},\"worker_count\":{},\"successful_worker_count\":{},\"feedback_worker_count\":{},\"feedback_applied\":{},\"runtime_tokens\":{},\"latency_ms\":{},\"runtime_backend\":{},\"runtime_device\":{},\"runtime_accelerator\":{},\"gpu_layers\":{},\"default_max_tokens\":{},\"configured_max_tokens\":{},\"effective_max_tokens\":{},\"max_tokens_clamped_count\":{},\"low_priority_worker_count\":{},\"blocked_primary_12b\":{}}}",
        json_string(&role.role),
        role.worker_count,
        role.successful_worker_count,
        role.feedback_worker_count,
        role.feedback_applied,
        role.runtime_tokens,
        role.latency_ms,
        option_str_json(role.runtime_backend.as_deref()),
        option_str_json(role.runtime_device.as_deref()),
        option_str_json(role.runtime_accelerator.as_deref()),
        option_u64_json(role.gpu_layers),
        option_u64_json(role.default_max_tokens),
        option_u64_json(role.configured_max_tokens),
        option_u64_json(role.effective_max_tokens),
        role.max_tokens_clamped_count,
        role.low_priority_worker_count,
        role.blocked_primary_12b
    )
}

fn status_json(summary: &PoolStatusSummary) -> String {
    format!(
        "{{\"launch_allowed\":{},\"launch_block_reason\":{},\"chain_classification\":{},\"min_context_tokens\":{},\"metadata\":{},\"capacity\":{},\"workers\":{{\"total\":{},\"reachable\":{},\"healthy\":{}}},\"roles\":{},\"advice\":{}}}",
        option_bool_json(summary.launch_allowed),
        option_str_json(summary.launch_block_reason.as_deref()),
        option_str_json(summary.chain_classification.as_deref()),
        option_u64_json(summary.min_context_tokens),
        capacity_metadata_json(summary),
        option_capacity_json(summary.capacity.as_ref()),
        summary.worker_count,
        summary.reachable_workers,
        summary.healthy_workers,
        role_states_json(&summary.roles),
        status_advice_json(summary)
    )
}

fn status_advice_json(summary: &PoolStatusSummary) -> String {
    let advice = status_advice(summary);
    format!(
        "{{\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"policy\":\"one_quality_12b_plus_small_helpers\",\"avoid_extra_12b\":true,\"safe_to_enable_pool_workers\":{},\"next_step\":{},\"reason\":{},\"kind\":{},\"advice\":{}}}",
        advice.safe_to_enable_pool_workers,
        json_string(advice.next_step),
        json_string(advice.reason),
        json_string(advice.kind),
        json_string(&advice.text)
    )
}

fn route_json(summary: &PoolRouteSummary) -> String {
    format!(
        "{{\"task_kind\":{},\"route_allowed\":{},\"route_block_reason\":{},\"dependency_precheck\":{},\"quality_context_tokens\":{},\"quality_context_required_tokens\":{},\"quality_context_sufficient\":{},\"quality_block_reason\":{},\"selected_context_required_tokens\":{},\"selected_context_buffer_tokens\":{},\"selected_context_buffer_policy\":{},\"selected_context_sufficient\":{},\"selected_context_block_reason\":{},\"selected_role\":{},\"selected_worker\":{},\"role_candidates\":{},\"candidates\":{{\"total\":{},\"healthy\":{},\"ready\":{}}},\"candidate_workers\":{}}}",
        option_str_json(summary.task_kind.as_deref()),
        option_bool_json(summary.route_allowed),
        option_str_json(summary.route_block_reason.as_deref()),
        dependency_precheck_json(summary.dependency_precheck.as_ref()),
        option_u64_json(summary.quality_context_tokens),
        option_u64_json(summary.quality_context_required_tokens),
        option_bool_json(summary.quality_context_sufficient),
        option_str_json(summary.quality_block_reason.as_deref()),
        option_u64_json(summary.selected_context_required_tokens),
        option_u64_json(summary.selected_context_buffer_tokens),
        context_buffer_policy_json(summary.selected_context_buffer_policy.as_ref()),
        option_bool_json(summary.selected_context_sufficient),
        option_str_json(summary.selected_context_block_reason.as_deref()),
        option_str_json(summary.selected_role.as_deref()),
        selected_route_candidate(summary)
            .map(route_candidate_json)
            .unwrap_or_else(|| "null".to_owned()),
        json_string_array(&summary.role_candidates),
        summary.candidate_count,
        summary.healthy_candidates,
        summary.ready_candidates,
        route_candidates_json(&summary.candidate_workers)
    )
}

fn dependency_precheck_json(summary: Option<&PoolDependencyPrecheckSummary>) -> String {
    let Some(summary) = summary else {
        return "null".to_owned();
    };
    format!(
        "{{\"strategy\":{},\"checked\":{},\"requested_role\":{},\"allow_dispatch\":{},\"reason\":{},\"required_roles\":{},\"completed_roles\":{},\"missing_roles\":{}}}",
        option_str_json(summary.strategy.as_deref()),
        option_bool_json(summary.checked),
        option_str_json(summary.requested_role.as_deref()),
        option_bool_json(summary.allow_dispatch),
        option_str_json(summary.reason.as_deref()),
        json_string_array(&summary.required_roles),
        json_string_array(&summary.completed_roles),
        json_string_array(&summary.missing_roles)
    )
}

fn context_buffer_policy_json(summary: Option<&PoolRouteContextBufferPolicySummary>) -> String {
    let Some(summary) = summary else {
        return "null".to_owned();
    };
    format!(
        "{{\"strategy\":{},\"base_tokens\":{},\"upstream_role_tokens\":{},\"eligible_upstream_roles\":{},\"completed_upstream_roles\":{},\"total_tokens\":{}}}",
        option_str_json(summary.strategy.as_deref()),
        option_u64_json(summary.base_tokens),
        option_u64_json(summary.upstream_role_tokens),
        json_string_array(&summary.eligible_upstream_roles),
        json_string_array(&summary.completed_upstream_roles),
        option_u64_json(summary.total_tokens)
    )
}

fn route_candidates_json(workers: &[PoolRouteCandidate]) -> String {
    let items = workers
        .iter()
        .map(route_candidate_json)
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

fn route_candidate_json(worker: &PoolRouteCandidate) -> String {
    format!(
        "{{\"port\":{},\"role\":{},\"base_url\":{},\"tcp_reachable\":{},\"health_ok\":{},\"status\":{},\"role_ready\":{},\"role_block_reason\":{},\"can_accept_low_priority_task\":{},\"model\":{},\"context_window\":{},\"runtime_backend\":{},\"runtime_device\":{},\"runtime_accelerator\":{},\"gpu_layers\":{},\"default_max_tokens\":{}}}",
        option_u64_json(worker.port),
        json_string(&worker.role),
        option_str_json(worker.base_url.as_deref()),
        worker.tcp_reachable,
        worker.health_ok,
        option_str_json(worker.status.as_deref()),
        worker.role_ready,
        option_str_json(worker.role_block_reason.as_deref()),
        worker.can_accept_low_priority_task,
        option_str_json(worker.model.as_deref()),
        option_u64_json(worker.context_window),
        option_str_json(worker.runtime_backend.as_deref()),
        option_str_json(worker.runtime_device.as_deref()),
        option_str_json(worker.runtime_accelerator.as_deref()),
        option_u64_json(worker.gpu_layers),
        option_u64_json(worker.default_max_tokens)
    )
}

fn alignment_json(summary: &PoolAlignmentSummary) -> String {
    format!(
        "{{\"alignment_ok\":{},\"manifest_roles\":{},\"status_roles\":{},\"manifest_advice\":{{\"safe_to_enable_pool_workers\":{},\"next_step\":{},\"reason\":{},\"extra_quality_12b_detected\":{},\"worker_shape\":{{\"quality\":{},\"helpers_visible\":{},\"helper_target\":{},\"failures\":{}}}}},\"quality_workers\":{{\"manifest\":{},\"status\":{},\"max\":{}}},\"helper_workers\":{{\"manifest\":{},\"status\":{},\"target\":{}}},\"missing_manifest_helper_roles\":{},\"missing_status_helper_roles\":{},\"missing_status_roles\":{},\"unplanned_status_roles\":{},\"route_blocked_or_failed\":{},\"route_dependency_failures\":{},\"missing_inputs\":{}}}",
        summary.alignment_ok,
        json_string_array(&summary.manifest_roles),
        json_string_array(&summary.status_roles),
        option_bool_json(summary.manifest_advice_safe_to_enable_pool_workers),
        option_str_json(summary.manifest_advice_next_step.as_deref()),
        option_str_json(summary.manifest_advice_reason.as_deref()),
        option_bool_json(summary.manifest_advice_extra_quality_12b_detected),
        option_u64_json(summary.manifest_advice_worker_shape_quality),
        option_u64_json(summary.manifest_advice_worker_shape_helpers_visible),
        option_u64_json(summary.manifest_advice_worker_shape_helper_target),
        json_string_array(&summary.manifest_advice_worker_shape_failures),
        summary.manifest_quality_workers,
        summary.status_quality_workers,
        summary.max_quality_workers,
        summary.manifest_helper_workers,
        summary.status_helper_workers,
        summary.helper_target,
        json_string_array(&summary.missing_manifest_helper_roles),
        json_string_array(&summary.missing_status_helper_roles),
        json_string_array(&summary.missing_status_roles),
        json_string_array(&summary.unplanned_status_roles),
        json_string_array(&summary.route_blocked_or_failed),
        json_string_array(&summary.route_dependency_failures),
        json_string_array(&summary.missing_inputs)
    )
}

fn role_states_json(roles: &[PoolWorkerRoleState]) -> String {
    let items = roles
        .iter()
        .map(|role| {
            format!(
                "{{\"role\":{},\"port\":{},\"base_url\":{},\"tcp_reachable\":{},\"health_ok\":{},\"ready\":{},\"busy\":{},\"role_ready\":{},\"status\":{},\"reported_status\":{},\"role_block_reason\":{},\"low_priority\":{},\"can_accept_low_priority_task\":{},\"model\":{},\"context_window\":{},\"runtime_backend\":{},\"runtime_device\":{},\"runtime_accelerator\":{},\"gpu_layers\":{},\"route_count\":{},\"selected_count\":{},\"blocked_count\":{},\"in_flight\":{},\"queued_count\":{},\"lease_wait_ms\":{},\"lease_wait_p95_ms\":{},\"success_count\":{},\"failure_count\":{},\"avg_latency_ms\":{},\"latency_p50_ms\":{},\"latency_p95_ms\":{}}}",
                json_string(&role.role),
                option_u64_json(role.port),
                option_str_json(role.base_url.as_deref()),
                role.tcp_reachable,
                role.health_ok,
                role.ready,
                role_busy(role),
                role.role_ready,
                json_string(role_status(role)),
                option_str_json(role.status.as_deref()),
                option_str_json(role.role_block_reason.as_deref()),
                option_bool_json(role.low_priority),
                option_bool_json(role.can_accept_low_priority_task),
                option_str_json(role.model.as_deref()),
                option_u64_json(role.context_window),
                option_str_json(role.runtime_backend.as_deref()),
                option_str_json(role.runtime_device.as_deref()),
                option_str_json(role.runtime_accelerator.as_deref()),
                option_u64_json(role.gpu_layers),
                option_u64_json(role.route_count),
                option_u64_json(role.selected_count),
                option_u64_json(role.blocked_count),
                option_u64_json(role.in_flight),
                option_u64_json(role.queued_count),
                option_u64_json(role.lease_wait_ms),
                option_u64_json(role.lease_wait_p95_ms),
                option_u64_json(role.success_count),
                option_u64_json(role.failure_count),
                option_u64_json(role.avg_latency_ms),
                option_u64_json(role.latency_p50_ms),
                option_u64_json(role.latency_p95_ms)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

fn role_budget_context_text(roles: &[PoolRoleBudgetSummary]) -> String {
    if roles.is_empty() {
        return "none".to_owned();
    }
    roles
        .iter()
        .map(|role| {
            format!(
                "{}:workers:{}/{} feedback:{} tokens:{} latency_ms:{} runtime_backend:{} runtime_device:{} runtime_accelerator:{} gpu_layers:{} default_max_tokens:{} configured_max_tokens:{} effective_max_tokens:{} max_tokens_clamped_count:{} low_priority_worker_count:{} blocked_primary_12b:{}",
                role.role,
                role.successful_worker_count,
                role.worker_count,
                role.feedback_applied,
                role.runtime_tokens,
                role.latency_ms,
                role.runtime_backend.as_deref().unwrap_or("none"),
                role.runtime_device.as_deref().unwrap_or("none"),
                role.runtime_accelerator.as_deref().unwrap_or("none"),
                option_u64_text(role.gpu_layers),
                option_u64_text(role.default_max_tokens),
                option_u64_text(role.configured_max_tokens),
                option_u64_text(role.effective_max_tokens),
                role.max_tokens_clamped_count,
                role.low_priority_worker_count,
                role.blocked_primary_12b
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn manifest_capacity_policy_context_text(
    summary: Option<&PoolManifestCapacityPolicySummary>,
) -> String {
    let Some(summary) = summary else {
        return "none".to_owned();
    };
    format!(
        "policy:{} target_host:{} avoid_extra_12b:{} max_quality_12b_workers:{} quality_role:{} quality_required_context_tokens:{} helper_roles:{} helper_context_tokens_total:{} helper_default_max_tokens_total:{} recommended_launch_order:{} expansion_gate:{} next_step_when_quality_ready:{}",
        summary.policy.as_deref().unwrap_or("none"),
        summary.target_host.as_deref().unwrap_or("none"),
        option_bool_text(summary.avoid_extra_12b),
        option_u64_text(summary.max_quality_12b_workers),
        summary.quality_role.as_deref().unwrap_or("none"),
        option_u64_text(summary.quality_required_context_tokens),
        role_candidates_text(&summary.helper_roles),
        option_u64_text(summary.helper_context_tokens_total),
        option_u64_text(summary.helper_default_max_tokens_total),
        role_candidates_text(&summary.recommended_launch_order),
        summary.expansion_gate.as_deref().unwrap_or("none"),
        summary
            .next_step_when_quality_ready
            .as_deref()
            .unwrap_or("none")
    )
}

fn manifest_advice_context_text(summary: Option<&PoolManifestAdviceSummary>) -> String {
    let Some(summary) = summary else {
        return "none".to_owned();
    };
    format!(
        "source:{} policy:{} safe_to_enable_pool_workers:{} next_step:{} reason:{} kind:{} extra_quality_12b_detected:{} quality_workers:{} helper_workers:{} helper_target:{} helper_roles:{} worker_shape:{}",
        summary.decision_source.as_deref().unwrap_or("none"),
        summary.policy.as_deref().unwrap_or("none"),
        option_bool_text(summary.safe_to_enable_pool_workers),
        summary.next_step.as_deref().unwrap_or("none"),
        summary.reason.as_deref().unwrap_or("none"),
        summary.kind.as_deref().unwrap_or("none"),
        option_bool_text(summary.extra_quality_12b_detected),
        option_u64_text(summary.quality_worker_count),
        option_u64_text(summary.helper_worker_count),
        option_u64_text(summary.helper_target_worker_count),
        role_candidates_text(&summary.helper_roles),
        manifest_worker_shape_context_text(summary.worker_shape.as_ref())
    )
}

fn manifest_worker_shape_context_text(summary: Option<&PoolManifestWorkerShapeSummary>) -> String {
    let Some(summary) = summary else {
        return "none".to_owned();
    };
    format!(
        "quality:{} helpers_visible:{} helper_target:{}",
        option_u64_text(summary.quality),
        option_u64_text(summary.helpers_visible),
        option_u64_text(summary.helper_target)
    )
}

fn manifest_workers_context_text(workers: &[PoolManifestWorkerSummary]) -> String {
    if workers.is_empty() {
        return "none".to_owned();
    }
    workers
        .iter()
        .map(|worker| {
            format!(
                "{}@{} endpoint:{} ctx:{} max:{} enabled:{} low_priority:{} runtime_backend:{} runtime_device:{} runtime_accelerator:{} gpu_layers:{}",
                worker.role,
                option_u64_text(worker.port),
                worker.base_url.as_deref().unwrap_or("none"),
                option_u64_text(worker.default_context_tokens),
                option_u64_text(worker.default_max_tokens),
                option_bool_text(worker.enabled_by_default),
                option_bool_text(worker.low_priority),
                worker.runtime_backend.as_deref().unwrap_or("none"),
                worker.runtime_device.as_deref().unwrap_or("none"),
                worker.runtime_accelerator.as_deref().unwrap_or("none"),
                option_u64_text(worker.gpu_layers)
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn capacity_context_text(summary: Option<&PoolCapacitySummary>) -> String {
    let Some(summary) = summary else {
        return "none".to_owned();
    };
    format!(
        "policy:{} expansion_allowed:{} recommendation:{} helpers:{}/{} runtime:metal:{} cpu:{} unknown:{} gpu0:{} quality_accelerated:{}",
        summary.policy.as_deref().unwrap_or("none"),
        option_bool_text(summary.expansion_allowed),
        summary.recommendation.as_deref().unwrap_or("none"),
        option_u64_text(summary.healthy_helper_worker_count),
        option_u64_text(summary.helper_worker_count),
        option_u64_text(summary.metal_worker_count),
        option_u64_text(summary.cpu_worker_count),
        option_u64_text(summary.unknown_runtime_worker_count),
        option_u64_text(summary.zero_gpu_layer_worker_count),
        option_bool_text(summary.quality_runtime_accelerated)
    )
}

fn capacity_metadata_context_text(summary: &PoolStatusSummary) -> String {
    format!(
        "generated_unix:{} observed_unix:{} age_seconds:{} max_age_seconds:{} required:{} stale:{}",
        option_u64_text(summary.generated_unix),
        option_u64_text(summary.observed_unix),
        option_u64_text(summary.metadata_age_seconds),
        option_u64_text(summary.max_age_seconds),
        summary.capacity_metadata_required,
        summary.metadata_stale
    )
}

fn capacity_metadata_gate_failure(summary: &PoolStatusSummary) -> Option<String> {
    if summary.metadata_stale {
        return Some(format!(
            "capacity metadata stale age_seconds={} max_age_seconds={}",
            option_u64_text(summary.metadata_age_seconds),
            option_u64_text(summary.max_age_seconds)
        ));
    }
    if summary.max_age_seconds.is_some() && summary.metadata_age_seconds.is_none() {
        return Some(format!(
            "capacity metadata max_age_seconds={} but age_seconds missing",
            option_u64_text(summary.max_age_seconds)
        ));
    }
    if summary.capacity_metadata_required && summary.metadata_age_seconds.is_none() {
        return Some("capacity metadata required but age_seconds missing".to_owned());
    }
    if summary.capacity_metadata_required {
        let missing_role_metadata = summary
            .roles
            .iter()
            .filter_map(required_role_capacity_metadata_failure)
            .collect::<Vec<_>>();
        if !missing_role_metadata.is_empty() {
            return Some(format!(
                "capacity role metadata missing:{}",
                missing_role_metadata.join("|")
            ));
        }
    }
    None
}

fn required_role_capacity_metadata_failure(role: &PoolWorkerRoleState) -> Option<String> {
    let mut missing = Vec::new();
    if role.in_flight.is_none() {
        missing.push("in_flight");
    }
    if role.queued_count.is_none() {
        missing.push("queued_count");
    }
    if role.lease_wait_ms.is_none() {
        missing.push("lease_wait_ms");
    }
    if role.lease_wait_p95_ms.is_none() {
        missing.push("lease_wait_p95_ms");
    }
    if role.latency_p50_ms.is_none() {
        missing.push("latency_p50_ms");
    }
    if role.latency_p95_ms.is_none() {
        missing.push("latency_p95_ms");
    }
    if missing.is_empty() {
        None
    } else {
        Some(format!("{}={}", role.role, missing.join(",")))
    }
}

fn option_capacity_json(summary: Option<&PoolCapacitySummary>) -> String {
    summary
        .map(capacity_json)
        .unwrap_or_else(|| "null".to_owned())
}

fn capacity_metadata_json(summary: &PoolStatusSummary) -> String {
    format!(
        "{{\"generated_unix\":{},\"observed_unix\":{},\"age_seconds\":{},\"max_age_seconds\":{},\"required\":{},\"stale\":{}}}",
        option_u64_json(summary.generated_unix),
        option_u64_json(summary.observed_unix),
        option_u64_json(summary.metadata_age_seconds),
        option_u64_json(summary.max_age_seconds),
        summary.capacity_metadata_required,
        summary.metadata_stale
    )
}

fn capacity_json(summary: &PoolCapacitySummary) -> String {
    format!(
        "{{\"policy\":{},\"expansion_allowed\":{},\"recommendation\":{},\"worker_count\":{},\"healthy_worker_count\":{},\"helper_worker_count\":{},\"healthy_helper_worker_count\":{},\"metal_worker_count\":{},\"cpu_worker_count\":{},\"unknown_runtime_worker_count\":{},\"zero_gpu_layer_worker_count\":{},\"quality_runtime_accelerated\":{}}}",
        option_str_json(summary.policy.as_deref()),
        option_bool_json(summary.expansion_allowed),
        option_str_json(summary.recommendation.as_deref()),
        option_u64_json(summary.worker_count),
        option_u64_json(summary.healthy_worker_count),
        option_u64_json(summary.helper_worker_count),
        option_u64_json(summary.healthy_helper_worker_count),
        option_u64_json(summary.metal_worker_count),
        option_u64_json(summary.cpu_worker_count),
        option_u64_json(summary.unknown_runtime_worker_count),
        option_u64_json(summary.zero_gpu_layer_worker_count),
        option_bool_json(summary.quality_runtime_accelerated)
    )
}

fn failure_reasons_text(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_owned()
    } else {
        values.join("|")
    }
}

fn role_candidates_text(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_owned()
    } else {
        values.join(",")
    }
}

fn manifest_planned_roles(summary: &PoolManifestSummary) -> Vec<String> {
    unique_roles(summary.workers.iter().map(|worker| worker.role.as_str()))
}

fn status_worker_roles(summary: &PoolStatusSummary) -> Vec<String> {
    unique_roles(summary.roles.iter().map(|role| role.role.as_str()))
}

fn unique_roles<'a>(roles: impl Iterator<Item = &'a str>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut unique = Vec::new();
    for role in roles {
        let role = role.trim();
        if role.is_empty() {
            continue;
        }
        if seen.insert(role.to_owned()) {
            unique.push(role.to_owned());
        }
    }
    unique
}

fn role_set(roles: &[String]) -> BTreeSet<&str> {
    roles.iter().map(String::as_str).collect()
}

fn manifest_quality_role(manifest: Option<&PoolManifestSummary>) -> String {
    manifest
        .and_then(|summary| summary.capacity_policy.as_ref())
        .and_then(|policy| policy.quality_role.as_deref())
        .map(str::trim)
        .filter(|role| !role.is_empty())
        .unwrap_or("quality")
        .to_owned()
}

fn manifest_helper_roles(manifest: Option<&PoolManifestSummary>) -> Vec<String> {
    manifest
        .and_then(|summary| summary.capacity_policy.as_ref())
        .map(|policy| {
            unique_roles(
                policy
                    .helper_roles
                    .iter()
                    .map(String::as_str)
                    .filter(|role| !role.trim().is_empty()),
            )
        })
        .filter(|roles| !roles.is_empty())
        .unwrap_or_else(|| {
            DEFAULT_HELPER_ROLES
                .iter()
                .map(|role| (*role).to_owned())
                .collect()
        })
}

fn count_manifest_role_workers(summary: &PoolManifestSummary, role: &str) -> usize {
    summary
        .workers
        .iter()
        .filter(|worker| worker.role.trim() == role)
        .count()
}

fn count_status_role_workers(summary: &PoolStatusSummary, role: &str) -> usize {
    summary
        .roles
        .iter()
        .filter(|worker| worker.role.trim() == role)
        .count()
}

fn count_manifest_roles(summary: &PoolManifestSummary, roles: &BTreeSet<&str>) -> usize {
    summary
        .workers
        .iter()
        .filter(|worker| roles.contains(worker.role.trim()))
        .count()
}

fn count_status_roles(summary: &PoolStatusSummary, roles: &BTreeSet<&str>) -> usize {
    summary
        .roles
        .iter()
        .filter(|worker| roles.contains(worker.role.trim()))
        .count()
}

fn roles_context_text(roles: &[PoolWorkerRoleState]) -> String {
    if roles.is_empty() {
        return "unknown".to_owned();
    }
    let statuses = roles
        .iter()
        .map(|role| format!("{}:{}", role.role, role_status(role)))
        .collect::<Vec<_>>()
        .join(",");
    let portraits = roles
        .iter()
        .map(role_capacity_portrait_text)
        .collect::<Vec<_>>()
        .join(";");
    format!("{statuses} portraits:{portraits}")
}

fn role_capacity_portrait_text(role: &PoolWorkerRoleState) -> String {
    format!(
        "{}@{} status:{} ready:{} busy:{} role_ready:{} reported_status:{} block:{} in_flight:{} queued:{} lease_wait_ms:{} lease_wait_p95_ms:{} routes:{}/{}/{} success_failure:{}/{} latency_ms:avg:{} p50:{} p95:{} runtime:{}/{}/{} gpu_layers:{} model:{} context:{} low_priority:{} accepts_low_priority:{}",
        role.role,
        option_u64_text(role.port),
        role_status(role),
        role.ready,
        role_busy(role),
        role.role_ready,
        role.status.as_deref().unwrap_or("none"),
        role.role_block_reason.as_deref().unwrap_or("none"),
        option_u64_text(role.in_flight),
        option_u64_text(role.queued_count),
        option_u64_text(role.lease_wait_ms),
        option_u64_text(role.lease_wait_p95_ms),
        option_u64_text(role.route_count),
        option_u64_text(role.selected_count),
        option_u64_text(role.blocked_count),
        option_u64_text(role.success_count),
        option_u64_text(role.failure_count),
        option_u64_text(role.avg_latency_ms),
        option_u64_text(role.latency_p50_ms),
        option_u64_text(role.latency_p95_ms),
        role.runtime_backend.as_deref().unwrap_or("none"),
        role.runtime_device.as_deref().unwrap_or("none"),
        role.runtime_accelerator.as_deref().unwrap_or("none"),
        option_u64_text(role.gpu_layers),
        role.model.as_deref().unwrap_or("none"),
        option_u64_text(role.context_window),
        option_bool_text(role.low_priority),
        option_bool_text(role.can_accept_low_priority_task)
    )
}

fn role_list_text<F>(roles: &[PoolWorkerRoleState], predicate: F) -> String
where
    F: Fn(&PoolWorkerRoleState) -> bool,
{
    let values = roles
        .iter()
        .filter(|role| predicate(role))
        .map(|role| role.role.as_str())
        .collect::<Vec<_>>();
    if values.is_empty() {
        "none".to_owned()
    } else {
        values.join(",")
    }
}

fn role_status(role: &PoolWorkerRoleState) -> &'static str {
    if role.health_ok {
        "healthy"
    } else if role.tcp_reachable {
        "tcp_only"
    } else {
        "unreachable"
    }
}

fn role_busy(role: &PoolWorkerRoleState) -> bool {
    role.in_flight.unwrap_or(0) > 0 || role.queued_count.unwrap_or(0) > 0
}

fn computed_metadata_age_seconds(
    generated_unix: Option<u64>,
    observed_unix: Option<u64>,
) -> Option<u64> {
    observed_unix?.checked_sub(generated_unix?)
}

fn option_f64_json(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.6}"))
        .unwrap_or_else(|| "null".to_owned())
}

fn option_f64_text(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "?".to_owned())
}

fn option_u64_json(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

fn option_bool_json(value: Option<bool>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

fn option_str_json(value: Option<&str>) -> String {
    value.map(json_string).unwrap_or_else(|| "null".to_owned())
}

fn option_u64_text(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "?".to_owned())
}

fn option_bool_text(value: Option<bool>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "?".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pool_manifest_capacity_policy_as_read_only_context() {
        let text = r#"{"contract_version":"gemma-chain.v1","read_only":true,"launches_process":false,"sends_prompt":false,"manifest_kind":"rust-norion.model-pool","capacity_policy":{"policy":"one_quality_plus_small_helpers","target_host":"apple_silicon","avoid_extra_12b":true,"max_quality_12b_workers":1,"quality_role":"quality","quality_required_context_tokens":262144,"helper_roles":["summary","router","review","index","test-gate"],"helper_context_tokens_total":28672,"helper_default_max_tokens_total":3328,"recommended_launch_order":["quality","summary","router","review","index","test-gate"],"expansion_gate":"quality worker must be Metal/GPU accelerated before helper expansion","next_step_when_quality_ready":"run_short_pool_smoke_then_use_evolution_loop_helper_stage_calls"},"workers":[{"role":"quality","port":8686,"base_url":"http://127.0.0.1:8686","default_context_tokens":262144,"default_max_tokens":262144,"enabled_by_default":true,"low_priority":false,"runtime_backend":"llama.cpp","runtime_device":"metal","runtime_accelerator":"metal","gpu_layers":99},{"role":"summary","port":8687,"base_url":"http://127.0.0.1:8687","default_context_tokens":8192,"default_max_tokens":768,"enabled_by_default":true,"low_priority":true,"runtime_backend":"llama.cpp","runtime_device":"metal","runtime_accelerator":"metal","gpu_layers":80}]}"#;

        let manifest = parse_manifest(text);
        let context = manifest_context_text(&manifest);
        let json = option_manifest_json(Some(&manifest));

        assert_eq!(manifest.contract_version.as_deref(), Some("gemma-chain.v1"));
        assert_eq!(
            manifest.manifest_kind.as_deref(),
            Some("rust-norion.model-pool")
        );
        assert_eq!(manifest.read_only, Some(true));
        assert_eq!(manifest.launches_process, Some(false));
        assert_eq!(manifest.sends_prompt, Some(false));
        let policy = manifest.capacity_policy.as_ref().unwrap();
        assert_eq!(
            policy.policy.as_deref(),
            Some("one_quality_plus_small_helpers")
        );
        assert_eq!(policy.target_host.as_deref(), Some("apple_silicon"));
        assert_eq!(policy.avoid_extra_12b, Some(true));
        assert_eq!(policy.max_quality_12b_workers, Some(1));
        assert_eq!(policy.quality_required_context_tokens, Some(262_144));
        assert_eq!(
            policy.helper_roles,
            vec![
                "summary".to_owned(),
                "router".to_owned(),
                "review".to_owned(),
                "index".to_owned(),
                "test-gate".to_owned()
            ]
        );
        assert_eq!(
            policy.recommended_launch_order,
            vec![
                "quality".to_owned(),
                "summary".to_owned(),
                "router".to_owned(),
                "review".to_owned(),
                "index".to_owned(),
                "test-gate".to_owned()
            ]
        );
        assert_eq!(manifest.worker_count, 2);
        assert_eq!(manifest.workers[0].role, "quality");
        assert_eq!(manifest.workers[0].default_context_tokens, Some(262_144));
        assert_eq!(manifest.workers[0].runtime_device.as_deref(), Some("metal"));
        assert_eq!(manifest.workers[1].role, "summary");
        assert_eq!(manifest.workers[1].low_priority, Some(true));
        assert!(context.contains("policy:one_quality_plus_small_helpers"));
        assert!(context.contains("avoid_extra_12b:true"));
        assert!(context.contains("max_quality_12b_workers:1"));
        assert!(context.contains("helper_roles:summary,router,review,index,test-gate"));
        assert!(
            context
                .contains("recommended_launch_order:quality,summary,router,review,index,test-gate")
        );
        assert!(context.contains("quality@8686"));
        assert!(context.contains("summary@8687"));
        assert!(json.contains("\"manifest_kind\":\"rust-norion.model-pool\""));
        assert!(json.contains("\"avoid_extra_12b\":true"));
        assert!(json.contains("\"max_quality_12b_workers\":1"));
        assert!(json.contains(
            "\"helper_roles\":[\"summary\",\"router\",\"review\",\"index\",\"test-gate\"]"
        ));
        assert!(json.contains("\"runtime_device\":\"metal\""));
    }

    #[test]
    fn missing_pool_manifest_renders_null() {
        assert_eq!(option_manifest_json(None), "null");
    }

    #[test]
    fn parses_pool_manifest_advice_as_read_only_context() {
        let text = r#"{"contract_version":"gemma-chain.v1","manifest_kind":"rust-norion.model-pool","advice":{"decision_source":"model-pool-advice-core","policy":"one_quality_12b_plus_small_helpers","safe_to_enable_pool_workers":true,"next_step":"run_short_pool_smoke_then_use_evolution_loop_helper_stage_calls","reason":"full_helper_pool_visible","kind":"busy","extra_quality_12b_detected":false,"quality_worker_count":1,"helper_worker_count":5,"helper_target_worker_count":5,"helper_roles":["summary","router","review","index","test-gate"],"worker_shape":{"quality":1,"helpers_visible":5,"helper_target":5}},"workers":[{"role":"quality","port":8686},{"role":"summary","port":8687}]}"#;
        let manifest = parse_manifest(text);
        let advice = manifest.advice.as_ref().unwrap();
        let worker_shape = advice.worker_shape.as_ref().unwrap();
        let context = manifest_context_text(&manifest);
        let json = option_manifest_json(Some(&manifest));

        assert_eq!(
            advice.decision_source.as_deref(),
            Some("model-pool-advice-core")
        );
        assert_eq!(
            advice.policy.as_deref(),
            Some("one_quality_12b_plus_small_helpers")
        );
        assert_eq!(advice.safe_to_enable_pool_workers, Some(true));
        assert_eq!(
            advice.next_step.as_deref(),
            Some("run_short_pool_smoke_then_use_evolution_loop_helper_stage_calls")
        );
        assert_eq!(advice.reason.as_deref(), Some("full_helper_pool_visible"));
        assert_eq!(advice.extra_quality_12b_detected, Some(false));
        assert_eq!(advice.quality_worker_count, Some(1));
        assert_eq!(advice.helper_worker_count, Some(5));
        assert_eq!(advice.helper_target_worker_count, Some(5));
        assert_eq!(
            advice.helper_roles,
            vec!["summary", "router", "review", "index", "test-gate"]
        );
        assert_eq!(worker_shape.quality, Some(1));
        assert_eq!(worker_shape.helpers_visible, Some(5));
        assert_eq!(worker_shape.helper_target, Some(5));
        assert!(context.contains("advice:source:model-pool-advice-core"));
        assert!(context.contains("safe_to_enable_pool_workers:true"));
        assert!(
            context.contains(
                "next_step:run_short_pool_smoke_then_use_evolution_loop_helper_stage_calls"
            )
        );
        assert!(context.contains("worker_shape:quality:1 helpers_visible:5 helper_target:5"));
        assert!(json.contains("\"decision_source\":\"model-pool-advice-core\""));
        assert!(json.contains("\"safe_to_enable_pool_workers\":true"));
        assert!(json.contains("\"helper_target_worker_count\":5"));
        assert!(json.contains(
            "\"worker_shape\":{\"quality\":1,\"helpers_visible\":5,\"helper_target\":5}"
        ));
    }

    #[test]
    fn parses_pool_status_as_read_only_context() {
        let text = r#"{"summary":"pool","generated_unix":1000,"observed_unix":1012,"max_age_seconds":30,"capacity_metadata_required":true,"launch_allowed":false,"launch_block_reason":"quality_worker_down","chain_classification":"quality_worker_down","min_context_tokens":262144,"capacity":{"policy":"one_quality_plus_small_helpers","expansion_allowed":false,"recommendation":"restore_quality_gate_first","worker_count":2,"healthy_worker_count":1,"helper_worker_count":1,"healthy_helper_worker_count":1,"metal_worker_count":1,"cpu_worker_count":0,"unknown_runtime_worker_count":0,"zero_gpu_layer_worker_count":0,"quality_runtime_accelerated":null},"workers":[{"port":8686,"base_url":"http://127.0.0.1:8686","role":"quality","tcp_reachable":false,"health_ok":false,"ready":false,"role_ready":false,"status":"blocked","role_block_reason":"quality_worker_down","low_priority":false,"can_accept_low_priority_task":false,"model":"qwen-quality","context_window":262144,"runtime_backend":"llama.cpp","runtime_device":"metal","runtime_accelerator":"metal","gpu_layers":99,"route_count":8,"selected_count":7,"blocked_count":1,"in_flight":0,"queued_count":1,"lease_wait_ms":0,"lease_wait_p95_ms":2,"success_count":6,"failure_count":1,"avg_latency_ms":1210,"latency_p50_ms":1000,"latency_p95_ms":1400},{"port":8687,"base_url":"http://127.0.0.1:8687","role":"summary","tcp_reachable":true,"health_ok":true,"ready":true,"role_ready":true,"status":"ready","low_priority":true,"can_accept_low_priority_task":true,"model":"qwen-summary","context_window":8192,"runtime_backend":"llama.cpp","runtime_device":"metal","runtime_accelerator":"metal","gpu_layers":80,"route_count":12,"selected_count":10,"blocked_count":2,"in_flight":1,"queued_count":2,"lease_wait_ms":14,"lease_wait_p95_ms":80,"success_count":9,"failure_count":1,"avg_latency_ms":320,"latency_p50_ms":250,"latency_p95_ms":700}]}
"#;
        let pool = parse_status(text);
        let context = status_context_text(&pool);
        let json = option_status_json(Some(&pool));

        assert_eq!(pool.generated_unix, Some(1000));
        assert_eq!(pool.observed_unix, Some(1012));
        assert_eq!(pool.metadata_age_seconds, Some(12));
        assert_eq!(pool.max_age_seconds, Some(30));
        assert!(pool.capacity_metadata_required);
        assert!(!pool.metadata_stale);
        assert_eq!(pool.launch_allowed, Some(false));
        assert_eq!(
            pool.chain_classification.as_deref(),
            Some("quality_worker_down")
        );
        assert_eq!(pool.worker_count, 2);
        assert_eq!(pool.reachable_workers, 1);
        assert_eq!(pool.healthy_workers, 1);
        assert_eq!(
            pool.capacity
                .as_ref()
                .and_then(|capacity| capacity.policy.as_deref()),
            Some("one_quality_plus_small_helpers")
        );
        assert_eq!(
            pool.capacity
                .as_ref()
                .and_then(|capacity| capacity.expansion_allowed),
            Some(false)
        );
        assert_eq!(
            pool.capacity
                .as_ref()
                .and_then(|capacity| capacity.recommendation.as_deref()),
            Some("restore_quality_gate_first")
        );
        assert_eq!(pool.roles.len(), 2);
        assert_eq!(pool.roles[0].role, "quality");
        assert_eq!(pool.roles[0].tcp_reachable, false);
        assert_eq!(pool.roles[0].port, Some(8686));
        assert_eq!(pool.roles[0].queued_count, Some(1));
        assert_eq!(pool.roles[0].lease_wait_ms, Some(0));
        assert_eq!(pool.roles[0].latency_p95_ms, Some(1400));
        assert_eq!(pool.roles[1].role, "summary");
        assert_eq!(pool.roles[1].health_ok, true);
        assert_eq!(pool.roles[1].ready, true);
        assert_eq!(pool.roles[1].role_ready, true);
        assert_eq!(pool.roles[1].in_flight, Some(1));
        assert_eq!(pool.roles[1].lease_wait_p95_ms, Some(80));
        assert_eq!(pool.roles[1].runtime_accelerator.as_deref(), Some("metal"));
        assert!(context.contains("launch_allowed:false"));
        assert!(context.contains(
            "metadata:generated_unix:1000 observed_unix:1012 age_seconds:12 max_age_seconds:30 required:true stale:false"
        ));
        assert!(context.contains("workers_reachable:1/2"));
        assert!(context.contains("capacity:policy:one_quality_plus_small_helpers"));
        assert!(context.contains("expansion_allowed:false"));
        assert!(context.contains("recommendation:restore_quality_gate_first"));
        assert!(context.contains("helpers:1/1"));
        assert!(context.contains("runtime:metal:1 cpu:0 unknown:0 gpu0:0"));
        assert!(context.contains("roles:quality:unreachable,summary:healthy"));
        assert!(context.contains("portraits:quality@8686"));
        assert!(
            context.contains("summary@8687 status:healthy ready:true busy:true role_ready:true")
        );
        assert!(context.contains("in_flight:1 queued:2"));
        assert!(context.contains("lease_wait_ms:14 lease_wait_p95_ms:80"));
        assert!(context.contains("latency_ms:avg:320 p50:250 p95:700"));
        assert!(context.contains("runtime:llama.cpp/metal/metal"));
        assert!(context.contains("available_roles:summary"));
        assert!(context.contains("blocked_roles:quality"));
        assert!(context.contains("advice:safe_to_enable_pool_workers:false"));
        assert!(context.contains("next_step:start_or_fix_quality_worker_8686"));
        assert!(context.contains("reason:quality_worker_not_ready"));
        assert!(json.contains("\"launch_allowed\":false"));
        assert!(json.contains("\"metadata\":{\"generated_unix\":1000"));
        assert!(json.contains("\"age_seconds\":12"));
        assert!(json.contains("\"required\":true"));
        assert!(json.contains("\"capacity\":{\"policy\":\"one_quality_plus_small_helpers\""));
        assert!(json.contains("\"expansion_allowed\":false"));
        assert!(json.contains("\"quality_runtime_accelerated\":null"));
        assert!(json.contains("\"reachable\":1"));
        assert!(json.contains("\"roles\":[{\"role\":\"quality\""));
        assert!(json.contains("\"port\":8687"));
        assert!(json.contains("\"ready\":true"));
        assert!(json.contains("\"busy\":true"));
        assert!(json.contains("\"role_ready\":true"));
        assert!(json.contains("\"reported_status\":\"ready\""));
        assert!(json.contains("\"in_flight\":1"));
        assert!(json.contains("\"queued_count\":2"));
        assert!(json.contains("\"lease_wait_p95_ms\":80"));
        assert!(json.contains("\"avg_latency_ms\":320"));
        assert!(json.contains("\"runtime_accelerator\":\"metal\""));
        assert!(json.contains("\"status\":\"healthy\""));
        assert!(json.contains("\"advice\":{\"read_only\":true"));
        assert!(json.contains("\"avoid_extra_12b\":true"));
        assert!(json.contains("\"next_step\":\"start_or_fix_quality_worker_8686\""));
    }

    #[test]
    fn capacity_gate_failure_reuses_status_context() {
        let blocked = parse_status(
            "{\"launch_allowed\":false,\"launch_block_reason\":\"quality_worker_down\",\"capacity\":{\"expansion_allowed\":false,\"recommendation\":\"restore_quality_gate_first\"},\"workers\":[{\"role\":\"quality\",\"tcp_reachable\":false,\"health_ok\":false}]}\n",
        );
        let allowed = parse_status(
            "{\"launch_allowed\":true,\"capacity\":{\"expansion_allowed\":true},\"workers\":[{\"role\":\"quality\",\"tcp_reachable\":true,\"health_ok\":true}]}\n",
        );
        let missing = parse_status(
            "{\"launch_allowed\":true,\"workers\":[{\"role\":\"quality\",\"tcp_reachable\":true,\"health_ok\":true}]}\n",
        );

        let failure = capacity_gate_failure(&blocked).unwrap();
        assert!(failure.contains("expansion_allowed=false"));
        assert!(failure.contains("recommendation=restore_quality_gate_first"));
        assert!(failure.contains("launch_allowed:false"));
        assert!(capacity_gate_failure(&allowed).is_none());
        assert!(
            capacity_gate_failure(&missing)
                .unwrap()
                .contains("capacity missing")
        );
    }

    #[test]
    fn capacity_gate_blocks_stale_capacity_metadata() {
        let stale = parse_status(
            "{\"metadata_age_seconds\":120,\"max_age_seconds\":60,\"capacity\":{\"expansion_allowed\":true},\"workers\":[{\"role\":\"quality\",\"tcp_reachable\":true,\"health_ok\":true}]}\n",
        );
        let explicit_stale = parse_status(
            "{\"metadata_stale\":true,\"capacity\":{\"expansion_allowed\":true},\"workers\":[{\"role\":\"quality\",\"tcp_reachable\":true,\"health_ok\":true}]}\n",
        );

        let stale_failure = capacity_gate_failure(&stale).unwrap();
        assert!(stale.metadata_stale);
        assert!(stale_failure.contains("capacity metadata stale"));
        assert!(stale_failure.contains("age_seconds=120"));
        assert!(stale_failure.contains("max_age_seconds=60"));
        assert!(
            capacity_gate_failure(&explicit_stale)
                .unwrap()
                .contains("capacity metadata stale")
        );
    }

    #[test]
    fn capacity_gate_blocks_missing_required_role_capacity_metadata() {
        let missing_role_metadata = parse_status(
            "{\"capacity_metadata_required\":true,\"metadata_age_seconds\":10,\"capacity\":{\"expansion_allowed\":true},\"workers\":[{\"role\":\"quality\",\"tcp_reachable\":true,\"health_ok\":true,\"in_flight\":0,\"queued_count\":0,\"lease_wait_ms\":0,\"latency_p50_ms\":1,\"latency_p95_ms\":2}]}\n",
        );

        let failure = capacity_gate_failure(&missing_role_metadata).unwrap();
        assert!(failure.contains("capacity role metadata missing"));
        assert!(failure.contains("quality=lease_wait_p95_ms"));
    }

    #[test]
    fn status_advice_distinguishes_quality_cpu_fallback_from_cpu_helpers() {
        let cpu_fallback = parse_status(
            "{\"launch_allowed\":true,\"capacity\":{\"expansion_allowed\":true,\"recommendation\":\"add_summary_worker_first\",\"cpu_worker_count\":1,\"zero_gpu_layer_worker_count\":0,\"quality_runtime_accelerated\":false},\"workers\":[{\"role\":\"quality\",\"tcp_reachable\":true,\"health_ok\":true}]}\n",
        );
        let cpu_helpers = parse_status(
            "{\"launch_allowed\":true,\"capacity\":{\"expansion_allowed\":true,\"recommendation\":\"add_summary_worker_first\",\"cpu_worker_count\":0,\"zero_gpu_layer_worker_count\":1,\"quality_runtime_accelerated\":true},\"workers\":[{\"role\":\"quality\",\"tcp_reachable\":true,\"health_ok\":true}]}\n",
        );

        let cpu_advice = status_advice(&cpu_fallback);
        let helper_advice = status_advice(&cpu_helpers);
        let helper_context = status_context_text(&cpu_helpers);

        assert!(!cpu_advice.safe_to_enable_pool_workers);
        assert_eq!(
            cpu_advice.next_step,
            "fix_quality_metal_or_gpu_layers_before_expansion"
        );
        assert_eq!(cpu_advice.reason, "quality_worker_not_gpu_accelerated");
        assert!(!helper_advice.safe_to_enable_pool_workers);
        assert_eq!(
            helper_advice.next_step,
            "hold_cpu_helpers_for_memory_pressure"
        );
        assert_eq!(helper_advice.reason, "cpu_helpers_preserve_shared_memory");
        assert!(
            !helper_context.contains("next_step:fix_quality_metal_or_gpu_layers_before_expansion")
        );
        assert!(helper_context.contains("next_step:hold_cpu_helpers_for_memory_pressure"));
        assert!(helper_context.contains("quality worker is accelerated"));
        assert!(helper_context.contains("gpu0_workers=1"));
    }

    #[test]
    fn status_advice_blocks_extra_quality_12b_workers() {
        let duplicate_quality = parse_status(
            "{\"launch_allowed\":false,\"launch_block_reason\":\"extra_quality_12b_workers\",\"capacity\":{\"expansion_allowed\":false,\"recommendation\":\"stop_extra_quality_12b_workers_keep_one_quality_plus_helpers\",\"quality_runtime_accelerated\":true},\"workers\":[{\"role\":\"quality\",\"tcp_reachable\":true,\"health_ok\":true},{\"role\":\"quality\",\"tcp_reachable\":true,\"health_ok\":true},{\"role\":\"summary\",\"tcp_reachable\":true,\"health_ok\":true}]}\n",
        );

        let advice = status_advice(&duplicate_quality);
        let context = status_context_text(&duplicate_quality);
        let json = option_status_json(Some(&duplicate_quality));

        assert_eq!(count_status_role_workers(&duplicate_quality, "quality"), 2);
        assert!(!advice.safe_to_enable_pool_workers);
        assert_eq!(
            advice.next_step,
            "stop_extra_quality_12b_workers_keep_one_quality_plus_helpers"
        );
        assert_eq!(
            advice.reason,
            "extra_quality_12b_wastes_shared_apple_memory"
        );
        assert!(
            context
                .contains("next_step:stop_extra_quality_12b_workers_keep_one_quality_plus_helpers")
        );
        assert!(context.contains("reason:extra_quality_12b_wastes_shared_apple_memory"));
        assert!(json.contains("\"safe_to_enable_pool_workers\":false"));
        assert!(json.contains(
            "\"next_step\":\"stop_extra_quality_12b_workers_keep_one_quality_plus_helpers\""
        ));
    }

    #[test]
    fn status_advice_adds_helpers_one_role_at_a_time() {
        let summary_first = parse_status(
            "{\"launch_allowed\":true,\"capacity\":{\"expansion_allowed\":true,\"quality_runtime_accelerated\":true},\"workers\":[{\"role\":\"quality\",\"tcp_reachable\":true,\"health_ok\":true}]}\n",
        );
        let review_or_index = parse_status(
            "{\"launch_allowed\":true,\"capacity\":{\"expansion_allowed\":true,\"quality_runtime_accelerated\":true},\"workers\":[{\"role\":\"quality\",\"tcp_reachable\":true,\"health_ok\":true},{\"role\":\"summary\",\"tcp_reachable\":true,\"health_ok\":true}]}\n",
        );
        let partial_pool = parse_status(
            "{\"launch_allowed\":true,\"capacity\":{\"expansion_allowed\":true,\"quality_runtime_accelerated\":true},\"workers\":[{\"role\":\"quality\",\"tcp_reachable\":true,\"health_ok\":true},{\"role\":\"summary\",\"tcp_reachable\":true,\"health_ok\":true},{\"role\":\"test-gate\",\"tcp_reachable\":true,\"health_ok\":true}]}\n",
        );
        let full_pool = parse_status(
            "{\"launch_allowed\":true,\"capacity\":{\"expansion_allowed\":true,\"quality_runtime_accelerated\":true},\"workers\":[{\"role\":\"quality\",\"tcp_reachable\":true,\"health_ok\":true},{\"role\":\"summary\",\"tcp_reachable\":true,\"health_ok\":true},{\"role\":\"review\",\"tcp_reachable\":true,\"health_ok\":true},{\"role\":\"index\",\"tcp_reachable\":true,\"health_ok\":true},{\"role\":\"test-gate\",\"tcp_reachable\":true,\"health_ok\":true}]}\n",
        );

        assert_eq!(
            status_advice(&summary_first).next_step,
            "add_summary_worker_first"
        );
        assert!(status_advice(&summary_first).safe_to_enable_pool_workers);
        assert_eq!(
            status_advice(&review_or_index).next_step,
            "add_review_or_index_after_short_smoke"
        );
        assert_eq!(
            status_advice(&partial_pool).next_step,
            "add_remaining_helper_roles_one_at_a_time"
        );
        assert_eq!(
            status_advice(&partial_pool).reason,
            "partial_helper_pool_visible"
        );
        assert!(
            status_context_text(&partial_pool)
                .contains("next_step:add_remaining_helper_roles_one_at_a_time")
        );
        assert_eq!(
            status_advice(&full_pool).next_step,
            "run_short_pool_smoke_then_use_evolution_loop_helper_stage_calls"
        );
        assert!(
            option_status_json(Some(&full_pool)).contains("\"safe_to_enable_pool_workers\":true")
        );
    }

    #[test]
    fn parses_pool_route_as_read_only_context() {
        let text = "{\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"task_kind\":\"review\",\"route_allowed\":false,\"route_block_reason\":\"model_pool_launch_blocked:quality_worker_down\",\"selected_role\":null,\"role_candidates\":[\"review\",\"quality\"],\"candidate_workers\":[{\"port\":8686,\"role\":\"quality\",\"base_url\":\"http://127.0.0.1:8686\",\"health_ok\":false,\"role_ready\":false,\"context_window\":262144,\"default_max_tokens\":262144},{\"port\":8688,\"role\":\"review\",\"base_url\":\"http://127.0.0.1:8688\",\"health_ok\":true,\"role_ready\":true,\"can_accept_low_priority_task\":true,\"context_window\":8192,\"default_max_tokens\":1024}]}\n";
        let route = parse_route(text);
        let context = route_context_text(&route);
        let json = option_route_json(Some(&route));

        assert_eq!(route.task_kind.as_deref(), Some("review"));
        assert_eq!(route.route_allowed, Some(false));
        assert_eq!(route.role_candidates, vec!["review", "quality"]);
        assert_eq!(route.candidate_count, 2);
        assert_eq!(route.healthy_candidates, 1);
        assert_eq!(route.ready_candidates, 1);
        assert_eq!(route.candidate_workers.len(), 2);
        assert_eq!(route.candidate_workers[1].role, "review");
        assert_eq!(route.candidate_workers[1].port, Some(8688));
        assert_eq!(
            route.candidate_workers[1].base_url.as_deref(),
            Some("http://127.0.0.1:8688")
        );
        assert_eq!(route.candidate_workers[1].context_window, Some(8192));
        assert_eq!(route.candidate_workers[1].default_max_tokens, Some(1024));
        assert!(route.candidate_workers[1].can_accept_low_priority_task);
        assert!(selected_route_candidate(&route).is_none());
        assert!(context.contains("task_kind:review"));
        assert!(context.contains("route_allowed:false"));
        assert!(context.contains("selected_endpoint:none"));
        assert!(context.contains("role_candidates:review,quality"));
        assert!(json.contains("\"task_kind\":\"review\""));
        assert!(json.contains("\"selected_role\":null"));
        assert!(json.contains("\"ready\":1"));
        assert!(json.contains("\"candidate_workers\":[{\"port\":8686"));
        assert!(json.contains("\"default_max_tokens\":1024"));
    }

    #[test]
    fn route_context_exposes_selected_ready_worker() {
        let text = "{\"task_kind\":\"summary\",\"route_allowed\":true,\"selected_context_required_tokens\":2816,\"selected_context_buffer_tokens\":2048,\"selected_context_sufficient\":true,\"selected_context_block_reason\":\"none\",\"selected_role\":\"summary\",\"role_candidates\":[\"summary\",\"quality\"],\"candidate_workers\":[{\"port\":8687,\"role\":\"summary\",\"base_url\":\"http://127.0.0.1:8687\",\"health_ok\":true,\"role_ready\":true,\"can_accept_low_priority_task\":true,\"context_window\":8192,\"runtime_backend\":\"llama.cpp\",\"runtime_device\":\"metal\",\"runtime_accelerator\":\"metal\",\"gpu_layers\":99,\"default_max_tokens\":768}]}\n";
        let route = parse_route(text);
        let worker = selected_route_candidate(&route).unwrap();
        let context = route_context_text(&route);
        let json = option_route_json(Some(&route));

        assert_eq!(worker.role, "summary");
        assert_eq!(worker.port, Some(8687));
        assert_eq!(worker.default_max_tokens, Some(768));
        assert_eq!(worker.runtime_backend.as_deref(), Some("llama.cpp"));
        assert_eq!(worker.runtime_device.as_deref(), Some("metal"));
        assert_eq!(worker.runtime_accelerator.as_deref(), Some("metal"));
        assert_eq!(worker.gpu_layers, Some(99));
        assert_eq!(route.selected_context_required_tokens, Some(2816));
        assert_eq!(route.selected_context_buffer_tokens, Some(2048));
        assert_eq!(route.selected_context_sufficient, Some(true));
        assert_eq!(route.selected_context_block_reason.as_deref(), Some("none"));
        assert!(context.contains("selected_endpoint:http://127.0.0.1:8687"));
        assert!(context.contains("selected_context_required_tokens:2816"));
        assert!(context.contains("selected_context_buffer_tokens:2048"));
        assert!(context.contains("selected_context_sufficient:true"));
        assert!(context.contains("selected_context_block_reason:none"));
        assert!(context.contains("selected_max_tokens:768"));
        assert!(context.contains("selected_runtime_backend:llama.cpp"));
        assert!(context.contains("selected_runtime_device:metal"));
        assert!(context.contains("selected_runtime_accelerator:metal"));
        assert!(context.contains("selected_gpu_layers:99"));
        assert!(json.contains("\"selected_worker\":{\"port\":8687"));
        assert!(json.contains("\"selected_context_required_tokens\":2816"));
        assert!(json.contains("\"selected_context_buffer_tokens\":2048"));
        assert!(json.contains("\"selected_context_sufficient\":true"));
        assert!(json.contains("\"selected_context_block_reason\":\"none\""));
        assert!(json.contains("\"runtime_backend\":\"llama.cpp\""));
        assert!(json.contains("\"runtime_device\":\"metal\""));
        assert!(json.contains("\"runtime_accelerator\":\"metal\""));
        assert!(json.contains("\"gpu_layers\":99"));
    }

    #[test]
    fn route_selection_skips_gpt5_series_model_candidates() {
        let route = parse_route(
            r#"{"task_kind":"review","route_allowed":true,"selected_role":"review","candidate_workers":[{"role":"review","model":"gpt-5-mini","health_ok":true,"role_ready":true},{"role":"review","model":"qwen/qwen3.5-397b-a17b","health_ok":true,"role_ready":true,"base_url":"http://127.0.0.1:8688"}]}"#,
        );

        let selected = selected_route_candidate(&route).unwrap();

        assert_eq!(selected.model.as_deref(), Some("qwen/qwen3.5-397b-a17b"));
        assert_eq!(selected.base_url.as_deref(), Some("http://127.0.0.1:8688"));
    }

    #[test]
    fn route_context_and_json_include_context_buffer_policy() {
        let text = r#"{"task_kind":"test-gate","route_allowed":true,"selected_context_required_tokens":3584,"selected_context_buffer_tokens":2560,"selected_context_buffer_policy":{"strategy":"test_gate_dynamic_upstream_buffer_v1","base_tokens":2048,"upstream_role_tokens":256,"eligible_upstream_roles":["review","index"],"completed_upstream_roles":["review","index"],"total_tokens":2560},"selected_context_sufficient":true,"selected_context_block_reason":"none","selected_role":"test-gate","role_candidates":["test-gate","review"],"candidate_workers":[{"port":8688,"role":"test-gate","base_url":"http://127.0.0.1:8688","health_ok":true,"role_ready":true,"runtime_backend":"llama.cpp","runtime_device":"cpu","runtime_accelerator":"accelerate","gpu_layers":0,"default_max_tokens":1024}]}"#;
        let route = parse_route(text);
        let policy = route.selected_context_buffer_policy.as_ref().unwrap();
        let context = route_context_text(&route);
        let json = option_route_json(Some(&route));

        assert_eq!(
            policy.strategy.as_deref(),
            Some("test_gate_dynamic_upstream_buffer_v1")
        );
        assert_eq!(policy.base_tokens, Some(2048));
        assert_eq!(policy.upstream_role_tokens, Some(256));
        assert_eq!(policy.eligible_upstream_roles, vec!["review", "index"]);
        assert_eq!(policy.completed_upstream_roles, vec!["review", "index"]);
        assert_eq!(policy.total_tokens, Some(2560));
        assert!(context.contains(
            "selected_context_buffer_policy:strategy:test_gate_dynamic_upstream_buffer_v1"
        ));
        assert!(context.contains("base_tokens:2048"));
        assert!(context.contains("upstream_role_tokens:256"));
        assert!(context.contains("eligible_upstream_roles:review,index"));
        assert!(context.contains("completed_upstream_roles:review,index"));
        assert!(context.contains("total_tokens:2560"));
        assert!(json.contains("\"selected_context_buffer_policy\":{\"strategy\":\"test_gate_dynamic_upstream_buffer_v1\""));
        assert!(json.contains("\"base_tokens\":2048"));
        assert!(json.contains("\"upstream_role_tokens\":256"));
        assert!(json.contains("\"eligible_upstream_roles\":[\"review\",\"index\"]"));
        assert!(json.contains("\"completed_upstream_roles\":[\"review\",\"index\"]"));
        assert!(json.contains("\"total_tokens\":2560"));
    }

    #[test]
    fn alignment_summary_blocks_context_buffer_policy_mismatch() {
        let manifest = parse_manifest(
            r#"{"capacity_policy":{"policy":"one_quality_plus_small_helpers","helper_roles":["test-gate"]},"workers":[{"role":"quality","port":8686},{"role":"test-gate","port":8688}]}"#,
        );
        let status = parse_status(
            r#"{"workers":[{"role":"quality","tcp_reachable":true,"health_ok":true},{"role":"test-gate","tcp_reachable":true,"health_ok":true}]}"#,
        );
        let good_route = parse_route(
            r#"{"task_kind":"test-gate","route_allowed":true,"selected_context_required_tokens":3584,"selected_context_buffer_tokens":2560,"selected_context_buffer_policy":{"strategy":"test_gate_dynamic_upstream_buffer_v1","base_tokens":2048,"upstream_role_tokens":256,"eligible_upstream_roles":["review","index"],"completed_upstream_roles":["review","index"],"total_tokens":2560},"selected_context_sufficient":true,"selected_role":"test-gate","candidate_workers":[{"role":"test-gate","health_ok":true,"role_ready":true}]}"#,
        );
        let bad_route = parse_route(
            r#"{"task_kind":"test-gate","route_allowed":true,"selected_context_required_tokens":3584,"selected_context_buffer_tokens":2560,"selected_context_buffer_policy":{"strategy":"test_gate_dynamic_upstream_buffer_v1","base_tokens":2048,"upstream_role_tokens":256,"eligible_upstream_roles":["review","index"],"completed_upstream_roles":["review","index"],"total_tokens":2304},"selected_context_sufficient":true,"selected_role":"test-gate","candidate_workers":[{"role":"test-gate","health_ok":true,"role_ready":true}]}"#,
        );

        let good_alignment = alignment_summary(Some(&manifest), Some(&status), &[good_route]);
        let bad_alignment = alignment_summary(Some(&manifest), Some(&status), &[bad_route]);
        let context = alignment_context_text(&bad_alignment);
        let failure = alignment_gate_failure(&bad_alignment).unwrap();
        let json = option_alignment_json(Some(&bad_alignment));

        assert!(good_alignment.alignment_ok);
        assert_eq!(alignment_gate_failure(&good_alignment), None);
        assert!(!bad_alignment.alignment_ok);
        assert_eq!(bad_alignment.route_dependency_failures.len(), 1);
        let reason = &bad_alignment.route_dependency_failures[0];
        assert!(reason.contains("test-gate:context_buffer_policy_mismatch"));
        assert!(reason.contains("selected_context_buffer_tokens=2560"));
        assert!(reason.contains("policy_total_tokens=2304"));
        assert!(reason.contains("computed_total_tokens=2560"));
        assert!(
            context.contains("route_dependency_failures:test-gate:context_buffer_policy_mismatch")
        );
        assert!(
            failure.contains("route_dependency_failures=test-gate:context_buffer_policy_mismatch")
        );
        assert!(json.contains("\"alignment_ok\":false"));
        assert!(json.contains("context_buffer_policy_mismatch"));
    }

    #[test]
    fn route_context_and_json_include_dependency_precheck() {
        let text = "{\"task_kind\":\"index\",\"route_allowed\":false,\"route_block_reason\":\"dependency_precheck_blocked:missing_required_roles\",\"dependency_precheck\":{\"strategy\":\"role_dependency_graph_v1\",\"checked\":true,\"requested_role\":\"index\",\"allow_dispatch\":false,\"reason\":\"missing_required_roles\",\"required_roles\":[\"summary\",\"router\"],\"completed_roles\":[\"quality\",\"summary\"],\"missing_roles\":[\"router\"]},\"selected_role\":null,\"role_candidates\":[\"index\",\"summary\"],\"candidate_workers\":[{\"port\":8690,\"role\":\"index\",\"base_url\":\"http://127.0.0.1:8690\",\"health_ok\":true,\"role_ready\":true}]}\n";
        let route = parse_route(text);
        let dependency = route.dependency_precheck.as_ref().unwrap();
        let context = route_context_text(&route);
        let json = option_route_json(Some(&route));

        assert_eq!(
            dependency.strategy.as_deref(),
            Some("role_dependency_graph_v1")
        );
        assert_eq!(dependency.checked, Some(true));
        assert_eq!(dependency.requested_role.as_deref(), Some("index"));
        assert_eq!(dependency.allow_dispatch, Some(false));
        assert_eq!(dependency.reason.as_deref(), Some("missing_required_roles"));
        assert_eq!(dependency.required_roles, vec!["summary", "router"]);
        assert_eq!(dependency.completed_roles, vec!["quality", "summary"]);
        assert_eq!(dependency.missing_roles, vec!["router"]);
        assert!(context.contains("dependency_precheck:strategy:role_dependency_graph_v1"));
        assert!(context.contains("requested_role:index"));
        assert!(context.contains("missing_roles:router"));
        assert!(
            json.contains("\"dependency_precheck\":{\"strategy\":\"role_dependency_graph_v1\"")
        );
        assert!(json.contains("\"missing_roles\":[\"router\"]"));
    }

    #[test]
    fn route_dependency_health_checks_required_roles_against_pool_status() {
        let route = parse_route(
            "{\"task_kind\":\"index\",\"route_allowed\":true,\"dependency_precheck\":{\"strategy\":\"role_dependency_graph_v1\",\"checked\":true,\"requested_role\":\"index\",\"allow_dispatch\":true,\"reason\":\"dependencies_satisfied\",\"required_roles\":[\"summary\",\"router\"],\"completed_roles\":[\"quality\",\"summary\",\"router\"],\"missing_roles\":[]},\"selected_role\":\"index\",\"candidate_workers\":[{\"role\":\"index\",\"health_ok\":true,\"role_ready\":true}]}\n",
        );
        let healthy_status = parse_status(
            "{\"workers\":[{\"role\":\"summary\",\"tcp_reachable\":true,\"health_ok\":true},{\"role\":\"router\",\"tcp_reachable\":true,\"health_ok\":true},{\"role\":\"index\",\"tcp_reachable\":true,\"health_ok\":true}]}\n",
        );
        let stale_status = parse_status(
            "{\"workers\":[{\"role\":\"summary\",\"tcp_reachable\":true,\"health_ok\":false},{\"role\":\"index\",\"tcp_reachable\":true,\"health_ok\":true}]}\n",
        );

        assert!(route_requires_dependency_health_check(&route));
        assert!(route_dependency_health_failure(&route, Some(&healthy_status)).is_none());

        let missing_status = route_dependency_health_failure(&route, None).unwrap();
        assert!(missing_status.contains("dependency_health_status_missing"));
        assert!(missing_status.contains("required_roles=summary,router"));

        let stale_failure = route_dependency_health_failure(&route, Some(&stale_status)).unwrap();
        assert!(stale_failure.contains("dependency_health_failed"));
        assert!(stale_failure.contains("missing_roles=router"));
        assert!(stale_failure.contains("unhealthy_roles=summary:tcp_only"));
    }

    #[test]
    fn route_dependency_health_is_advisory_when_dependency_precheck_was_not_checked() {
        let route = parse_route(
            "{\"task_kind\":\"review\",\"route_allowed\":true,\"dependency_precheck\":{\"strategy\":\"role_dependency_graph_v1\",\"checked\":false,\"requested_role\":\"review\",\"allow_dispatch\":true,\"reason\":\"completed_roles_not_provided\",\"required_roles\":[\"summary\"],\"completed_roles\":[],\"missing_roles\":[]},\"selected_role\":\"review\",\"candidate_workers\":[{\"role\":\"review\",\"health_ok\":true,\"role_ready\":true}]}\n",
        );

        assert!(!route_requires_dependency_health_check(&route));
        assert!(route_dependency_health_failure(&route, None).is_none());
    }

    #[test]
    fn alignment_summary_detects_manifest_status_and_route_mismatch() {
        let manifest = parse_manifest(
            "{\"capacity_policy\":{\"policy\":\"one_quality_plus_small_helpers\",\"max_quality_12b_workers\":1,\"quality_role\":\"quality\",\"helper_roles\":[\"summary\",\"review\",\"index\",\"test-gate\"]},\"workers\":[{\"role\":\"quality\",\"port\":8686},{\"role\":\"summary\",\"port\":8687},{\"role\":\"review\",\"port\":8688}]}\n",
        );
        let status = parse_status(
            "{\"workers\":[{\"role\":\"quality\",\"tcp_reachable\":true,\"health_ok\":true},{\"role\":\"summary\",\"tcp_reachable\":true,\"health_ok\":true},{\"role\":\"extra\",\"tcp_reachable\":true,\"health_ok\":true}]}\n",
        );
        let review_route = parse_route(
            "{\"task_kind\":\"review\",\"route_allowed\":false,\"route_block_reason\":\"worker_down\",\"selected_role\":null,\"candidate_workers\":[{\"role\":\"review\",\"health_ok\":false,\"role_ready\":false}]}\n",
        );
        let summary_route = parse_route(
            "{\"task_kind\":\"summary\",\"route_allowed\":true,\"selected_role\":\"summary\",\"candidate_workers\":[{\"role\":\"summary\",\"health_ok\":true,\"role_ready\":true}]}\n",
        );

        let alignment = alignment_summary(
            Some(&manifest),
            Some(&status),
            &[review_route, summary_route],
        );
        let context = alignment_context_text(&alignment);
        let json = option_alignment_json(Some(&alignment));

        assert!(!alignment.alignment_ok);
        assert_eq!(
            alignment.manifest_roles,
            vec!["quality", "summary", "review"]
        );
        assert_eq!(alignment.status_roles, vec!["quality", "summary", "extra"]);
        assert_eq!(alignment.manifest_quality_workers, 1);
        assert_eq!(alignment.status_quality_workers, 1);
        assert_eq!(alignment.manifest_helper_workers, 2);
        assert_eq!(alignment.status_helper_workers, 1);
        assert_eq!(alignment.helper_target, 4);
        assert_eq!(
            alignment.missing_manifest_helper_roles,
            vec!["index", "test-gate"]
        );
        assert_eq!(
            alignment.missing_status_helper_roles,
            vec!["review", "index", "test-gate"]
        );
        assert_eq!(alignment.missing_status_roles, vec!["review"]);
        assert_eq!(alignment.unplanned_status_roles, vec!["extra"]);
        assert_eq!(alignment.route_blocked_or_failed, vec!["review"]);
        assert!(alignment.route_dependency_failures.is_empty());
        assert!(alignment.missing_inputs.is_empty());
        assert!(context.contains("alignment_ok:false"));
        assert!(context.contains("missing_manifest_helper_roles:index,test-gate"));
        assert!(context.contains("missing_status_helper_roles:review,index,test-gate"));
        assert!(context.contains("missing_status_roles:review"));
        assert!(context.contains("unplanned_status_roles:extra"));
        assert!(context.contains("route_blocked_or_failed:review"));
        assert!(context.contains("route_dependency_failures:none"));
        assert!(json.contains("\"alignment_ok\":false"));
        assert!(json.contains("\"missing_manifest_helper_roles\":[\"index\",\"test-gate\"]"));
        assert!(
            json.contains("\"missing_status_helper_roles\":[\"review\",\"index\",\"test-gate\"]")
        );
        assert!(json.contains("\"missing_status_roles\":[\"review\"]"));
        assert!(json.contains("\"unplanned_status_roles\":[\"extra\"]"));
        assert!(json.contains("\"route_blocked_or_failed\":[\"review\"]"));
        assert!(json.contains("\"route_dependency_failures\":[]"));
    }

    #[test]
    fn alignment_summary_surfaces_dependency_precheck_failures() {
        let manifest = parse_manifest(
            "{\"capacity_policy\":{\"policy\":\"one_quality_plus_small_helpers\",\"helper_roles\":[\"summary\",\"review\",\"index\",\"test-gate\"]},\"workers\":[{\"role\":\"quality\"},{\"role\":\"summary\"},{\"role\":\"index\"}]}\n",
        );
        let status = parse_status(
            "{\"workers\":[{\"role\":\"quality\",\"tcp_reachable\":true,\"health_ok\":true},{\"role\":\"summary\",\"tcp_reachable\":true,\"health_ok\":true},{\"role\":\"index\",\"tcp_reachable\":true,\"health_ok\":true}]}\n",
        );
        let index_route = parse_route(
            "{\"task_kind\":\"index\",\"route_allowed\":false,\"route_block_reason\":\"dependency_precheck_blocked:missing_required_roles\",\"dependency_precheck\":{\"strategy\":\"role_dependency_graph_v1\",\"checked\":true,\"requested_role\":\"index\",\"allow_dispatch\":false,\"reason\":\"missing_required_roles\",\"required_roles\":[\"summary\",\"router\"],\"completed_roles\":[\"quality\",\"summary\"],\"missing_roles\":[\"router\"]},\"selected_role\":null,\"candidate_workers\":[{\"role\":\"index\",\"health_ok\":true,\"role_ready\":true}]}\n",
        );

        let alignment = alignment_summary(Some(&manifest), Some(&status), &[index_route]);
        let context = alignment_context_text(&alignment);
        let failure = alignment_gate_failure(&alignment).unwrap();
        let json = option_alignment_json(Some(&alignment));

        assert!(!alignment.alignment_ok);
        assert_eq!(alignment.route_blocked_or_failed, vec!["index"]);
        assert_eq!(
            alignment.route_dependency_failures,
            vec!["index:index:missing_required_roles:missing=router"]
        );
        assert!(context.contains(
            "route_dependency_failures:index:index:missing_required_roles:missing=router"
        ));
        assert!(failure.contains(
            "route_dependency_failures=index:index:missing_required_roles:missing=router"
        ));
        assert!(json.contains(
            "\"route_dependency_failures\":[\"index:index:missing_required_roles:missing=router\"]"
        ));
    }

    #[test]
    fn alignment_summary_surfaces_dependency_health_failures_for_allowed_route() {
        let manifest = parse_manifest(
            "{\"capacity_policy\":{\"policy\":\"one_quality_plus_small_helpers\",\"helper_roles\":[\"summary\",\"index\"]},\"workers\":[{\"role\":\"quality\"},{\"role\":\"summary\"},{\"role\":\"index\"}]}\n",
        );
        let status = parse_status(
            "{\"workers\":[{\"role\":\"quality\",\"tcp_reachable\":true,\"health_ok\":true},{\"role\":\"summary\",\"tcp_reachable\":true,\"health_ok\":false},{\"role\":\"index\",\"tcp_reachable\":true,\"health_ok\":true}]}\n",
        );
        let index_route = parse_route(
            "{\"task_kind\":\"index\",\"route_allowed\":true,\"dependency_precheck\":{\"strategy\":\"role_dependency_graph_v1\",\"checked\":true,\"requested_role\":\"index\",\"allow_dispatch\":true,\"reason\":\"dependencies_satisfied\",\"required_roles\":[\"summary\",\"router\"],\"completed_roles\":[\"quality\",\"summary\",\"router\"],\"missing_roles\":[]},\"selected_role\":\"index\",\"candidate_workers\":[{\"role\":\"index\",\"health_ok\":true,\"role_ready\":true}]}\n",
        );

        let alignment = alignment_summary(Some(&manifest), Some(&status), &[index_route]);
        let context = alignment_context_text(&alignment);
        let failure = alignment_gate_failure(&alignment).unwrap();
        let json = option_alignment_json(Some(&alignment));

        assert!(!alignment.alignment_ok);
        assert!(alignment.route_blocked_or_failed.is_empty());
        assert_eq!(alignment.route_dependency_failures.len(), 1);
        let dependency_failure = &alignment.route_dependency_failures[0];
        assert!(dependency_failure.contains("index:dependency_health_failed"));
        assert!(dependency_failure.contains("required_roles=summary,router"));
        assert!(dependency_failure.contains("missing_roles=router"));
        assert!(dependency_failure.contains("unhealthy_roles=summary:tcp_only"));
        assert!(context.contains("route_dependency_failures:index:dependency_health_failed"));
        assert!(failure.contains("route_dependency_failures=index:dependency_health_failed"));
        assert!(json.contains("\"alignment_ok\":false"));
        assert!(json.contains("index:dependency_health_failed"));
    }

    #[test]
    fn alignment_summary_blocks_policy_helpers_missing_from_otherwise_matching_pool() {
        let manifest = parse_manifest(
            "{\"capacity_policy\":{\"policy\":\"one_quality_plus_small_helpers\",\"helper_roles\":[\"summary\",\"review\",\"index\",\"test-gate\"]},\"workers\":[{\"role\":\"quality\",\"port\":8686},{\"role\":\"summary\",\"port\":8687}]}\n",
        );
        let status = parse_status(
            "{\"workers\":[{\"role\":\"quality\",\"tcp_reachable\":true,\"health_ok\":true},{\"role\":\"summary\",\"tcp_reachable\":true,\"health_ok\":true}]}\n",
        );
        let summary_route = parse_route(
            "{\"task_kind\":\"summary\",\"route_allowed\":true,\"selected_role\":\"summary\",\"candidate_workers\":[{\"role\":\"summary\",\"health_ok\":true,\"role_ready\":true}]}\n",
        );

        let alignment = alignment_summary(Some(&manifest), Some(&status), &[summary_route]);
        let failure = alignment_gate_failure(&alignment).unwrap();

        assert!(!alignment.alignment_ok);
        assert_eq!(
            alignment.missing_manifest_helper_roles,
            vec!["review", "index", "test-gate"]
        );
        assert_eq!(
            alignment.missing_status_helper_roles,
            vec!["review", "index", "test-gate"]
        );
        assert!(alignment.missing_status_roles.is_empty());
        assert!(alignment.unplanned_status_roles.is_empty());
        assert!(alignment.route_blocked_or_failed.is_empty());
        assert!(failure.contains("missing_manifest_helper_roles=review,index,test-gate"));
        assert!(failure.contains("missing_status_helper_roles=review,index,test-gate"));
    }

    #[test]
    fn alignment_gate_blocks_manifest_advice_that_detects_extra_quality_12b() {
        let manifest = parse_manifest(
            r#"{"capacity_policy":{"policy":"one_quality_plus_small_helpers","max_quality_12b_workers":1,"quality_role":"quality","helper_roles":["summary","review","index","test-gate"]},"advice":{"decision_source":"model-pool-advice-core","safe_to_enable_pool_workers":false,"next_step":"stop_extra_quality_12b_workers_keep_one_quality_plus_helpers","reason":"extra_quality_12b_wastes_shared_apple_memory","extra_quality_12b_detected":true,"worker_shape":{"quality":1,"helpers_visible":4,"helper_target":4}},"workers":[{"role":"quality","port":8686},{"role":"summary","port":8687},{"role":"review","port":8688},{"role":"index","port":8690},{"role":"test-gate","port":8688}]}"#,
        );
        let status = parse_status(
            r#"{"workers":[{"role":"quality","tcp_reachable":true,"health_ok":true},{"role":"summary","tcp_reachable":true,"health_ok":true},{"role":"review","tcp_reachable":true,"health_ok":true},{"role":"index","tcp_reachable":true,"health_ok":true},{"role":"test-gate","tcp_reachable":true,"health_ok":true}]}"#,
        );
        let summary_route = parse_route(
            r#"{"task_kind":"summary","route_allowed":true,"selected_role":"summary","candidate_workers":[{"role":"summary","health_ok":true,"role_ready":true}]}"#,
        );

        let alignment = alignment_summary(Some(&manifest), Some(&status), &[summary_route]);
        let failure = alignment_gate_failure(&alignment).unwrap();
        let context = alignment_context_text(&alignment);
        let json = option_alignment_json(Some(&alignment));

        assert!(!alignment.alignment_ok);
        assert_eq!(
            alignment.manifest_advice_safe_to_enable_pool_workers,
            Some(false)
        );
        assert_eq!(
            alignment.manifest_advice_extra_quality_12b_detected,
            Some(true)
        );
        assert!(failure.contains("manifest_advice_blocked"));
        assert!(failure.contains("extra_quality_12b_wastes_shared_apple_memory"));
        assert!(context.contains("manifest_advice_safe_to_enable_pool_workers:false"));
        assert!(json.contains("\"manifest_advice\":{\"safe_to_enable_pool_workers\":false"));
    }

    #[test]
    fn alignment_summary_accepts_matching_manifest_advice_worker_shape() {
        let manifest = parse_manifest(
            r#"{"capacity_policy":{"policy":"one_quality_plus_small_helpers","max_quality_12b_workers":1,"quality_role":"quality","helper_roles":["summary","router","review","index","test-gate"]},"advice":{"decision_source":"model-pool-advice-core","safe_to_enable_pool_workers":true,"next_step":"run_short_pool_smoke_then_use_evolution_loop_helper_stage_calls","reason":"quality_plus_helpers_ready","extra_quality_12b_detected":false,"worker_shape":{"quality":1,"helpers_visible":5,"helper_target":5}},"workers":[{"role":"quality","port":8686},{"role":"summary","port":8687},{"role":"router","port":8689},{"role":"review","port":8688},{"role":"index","port":8690},{"role":"test-gate","port":8688}]}"#,
        );
        let status = parse_status(
            r#"{"workers":[{"role":"quality","tcp_reachable":true,"health_ok":true},{"role":"summary","tcp_reachable":true,"health_ok":true},{"role":"router","tcp_reachable":true,"health_ok":true},{"role":"review","tcp_reachable":true,"health_ok":true},{"role":"index","tcp_reachable":true,"health_ok":true},{"role":"test-gate","tcp_reachable":true,"health_ok":true}]}"#,
        );
        let summary_route = parse_route(
            r#"{"task_kind":"summary","route_allowed":true,"selected_role":"summary","candidate_workers":[{"role":"summary","health_ok":true,"role_ready":true}]}"#,
        );

        let alignment = alignment_summary(Some(&manifest), Some(&status), &[summary_route]);
        let context = alignment_context_text(&alignment);
        let json = option_alignment_json(Some(&alignment));

        assert!(alignment.alignment_ok);
        assert_eq!(alignment.manifest_advice_worker_shape_quality, Some(1));
        assert_eq!(
            alignment.manifest_advice_worker_shape_helpers_visible,
            Some(5)
        );
        assert_eq!(
            alignment.manifest_advice_worker_shape_helper_target,
            Some(5)
        );
        assert!(alignment.manifest_advice_worker_shape_failures.is_empty());
        assert!(alignment_gate_failure(&alignment).is_none());
        assert!(
            context.contains(
                "manifest_advice_worker_shape:quality:1 helpers_visible:5 helper_target:5"
            )
        );
        assert!(context.contains("worker_shape_failures:none"));
        assert!(json.contains("\"worker_shape\":{\"quality\":1,\"helpers_visible\":5,\"helper_target\":5,\"failures\":[]}"));
    }

    #[test]
    fn alignment_gate_blocks_missing_manifest_advice_worker_shape() {
        let manifest = parse_manifest(
            r#"{"capacity_policy":{"policy":"one_quality_plus_small_helpers","max_quality_12b_workers":1,"quality_role":"quality","helper_roles":["summary","router","review","index","test-gate"]},"advice":{"decision_source":"model-pool-advice-core","safe_to_enable_pool_workers":true,"next_step":"run_short_pool_smoke_then_use_evolution_loop_helper_stage_calls","reason":"quality_plus_helpers_ready","extra_quality_12b_detected":false},"workers":[{"role":"quality","port":8686},{"role":"summary","port":8687},{"role":"router","port":8689},{"role":"review","port":8688},{"role":"index","port":8690},{"role":"test-gate","port":8688}]}"#,
        );
        let status = parse_status(
            r#"{"workers":[{"role":"quality","tcp_reachable":true,"health_ok":true},{"role":"summary","tcp_reachable":true,"health_ok":true},{"role":"router","tcp_reachable":true,"health_ok":true},{"role":"review","tcp_reachable":true,"health_ok":true},{"role":"index","tcp_reachable":true,"health_ok":true},{"role":"test-gate","tcp_reachable":true,"health_ok":true}]}"#,
        );
        let summary_route = parse_route(
            r#"{"task_kind":"summary","route_allowed":true,"selected_role":"summary","candidate_workers":[{"role":"summary","health_ok":true,"role_ready":true}]}"#,
        );

        let alignment = alignment_summary(Some(&manifest), Some(&status), &[summary_route]);
        let failure = alignment_gate_failure(&alignment).unwrap();

        assert!(!alignment.alignment_ok);
        assert_eq!(
            alignment.manifest_advice_worker_shape_failures,
            vec!["worker_shape_missing"]
        );
        assert!(failure.contains("manifest_advice_worker_shape_mismatch=worker_shape_missing"));
    }

    #[test]
    fn alignment_gate_blocks_manifest_advice_worker_shape_mismatch() {
        let manifest = parse_manifest(
            r#"{"capacity_policy":{"policy":"one_quality_plus_small_helpers","max_quality_12b_workers":1,"quality_role":"quality","helper_roles":["summary","review","index","test-gate"]},"advice":{"decision_source":"model-pool-advice-core","safe_to_enable_pool_workers":true,"next_step":"run_short_pool_smoke_then_use_evolution_loop_helper_stage_calls","reason":"quality_plus_helpers_ready","extra_quality_12b_detected":false,"worker_shape":{"quality":2,"helpers_visible":3,"helper_target":2}},"workers":[{"role":"quality","port":8686},{"role":"summary","port":8687},{"role":"review","port":8688},{"role":"index","port":8690},{"role":"test-gate","port":8688}]}"#,
        );
        let status = parse_status(
            r#"{"workers":[{"role":"quality","tcp_reachable":true,"health_ok":true},{"role":"summary","tcp_reachable":true,"health_ok":true},{"role":"review","tcp_reachable":true,"health_ok":true},{"role":"index","tcp_reachable":true,"health_ok":true},{"role":"test-gate","tcp_reachable":true,"health_ok":true}]}"#,
        );
        let summary_route = parse_route(
            r#"{"task_kind":"summary","route_allowed":true,"selected_role":"summary","candidate_workers":[{"role":"summary","health_ok":true,"role_ready":true}]}"#,
        );

        let alignment = alignment_summary(Some(&manifest), Some(&status), &[summary_route]);
        let failure = alignment_gate_failure(&alignment).unwrap();

        assert!(!alignment.alignment_ok);
        assert!(
            alignment
                .manifest_advice_worker_shape_failures
                .contains(&"worker_shape_quality=2 expected=1".to_owned())
        );
        assert!(
            alignment
                .manifest_advice_worker_shape_failures
                .contains(&"worker_shape_helpers_visible=3 expected=4".to_owned())
        );
        assert!(
            alignment
                .manifest_advice_worker_shape_failures
                .contains(&"worker_shape_helper_target=2 expected=4".to_owned())
        );
        assert!(failure.contains("manifest_advice_worker_shape_mismatch="));
        assert!(failure.contains("worker_shape_quality=2 expected=1"));
        assert!(failure.contains("worker_shape_helpers_visible=3 expected=4"));
        assert!(failure.contains("worker_shape_helper_target=2 expected=4"));
    }

    #[test]
    fn budget_fairness_allows_balanced_feedback_workers() {
        let text = "{\"workers\":[{\"worker_id\":\"p1\",\"role\":\"summary\",\"success\":true,\"feedback_applied\":2,\"runtime_tokens\":900,\"latency_ms\":1200,\"blocked_primary_12b\":false},{\"worker_id\":\"r1\",\"role\":\"review\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":700,\"latency_ms\":900,\"blocked_primary_12b\":false},{\"worker_id\":\"t1\",\"role\":\"test-gate\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":600,\"latency_ms\":800,\"blocked_primary_12b\":false}]}\n";
        let summary = parse_budget_fairness(text);
        let context = budget_fairness_context_text(&summary);
        let json = option_budget_fairness_json(Some(&summary));

        assert_eq!(summary.worker_count, 3);
        assert_eq!(summary.successful_worker_count, 3);
        assert_eq!(summary.feedback_worker_count, 3);
        assert_eq!(summary.total_runtime_tokens, 2200);
        assert_eq!(summary.total_latency_ms, 2900);
        assert_eq!(summary.max_role_runtime_token_share, Some(900.0 / 2200.0));
        assert!(!summary.budget_fairness_blocked);
        assert!(summary.allow_pool_expansion);
        assert!(summary.failure_reasons.is_empty());
        assert!(context.contains("allow_pool_expansion:true"));
        assert!(json.contains("\"schema\":\"model_pool_budget_fairness_report_v1\""));
        assert!(json.contains("\"role\":\"summary\""));
        assert!(json.contains("\"max_role_runtime_token_share\":0.409091"));
    }

    #[test]
    fn budget_fairness_blocks_missing_required_feedback_role() {
        let text = "{\"workers\":[{\"role\":\"summary\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":500,\"latency_ms\":200},{\"role\":\"review\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":500,\"latency_ms\":200}]}\n";
        let summary = parse_budget_fairness(text);

        assert!(summary.budget_fairness_blocked);
        assert!(!summary.allow_pool_expansion);
        assert!(
            summary
                .failure_reasons
                .iter()
                .any(|reason| reason.contains("required role tester"))
        );
    }

    #[test]
    fn budget_fairness_blocks_dominant_role_token_share() {
        let text = "{\"workers\":[{\"role\":\"planner\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":800,\"latency_ms\":200},{\"role\":\"reviewer\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":100,\"latency_ms\":200},{\"role\":\"tester\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":100,\"latency_ms\":200}]}\n";
        let summary = parse_budget_fairness(text);

        assert_eq!(summary.max_role_runtime_token_share, Some(0.8));
        assert!(summary.budget_fairness_blocked);
        assert!(
            summary
                .failure_reasons
                .iter()
                .any(|reason| reason.contains("runtime token share"))
        );
    }

    #[test]
    fn budget_fairness_uses_display_precision_for_share_boundary() {
        assert!(!role_runtime_token_share_exceeds_limit(0.60049));
        assert!(role_runtime_token_share_exceeds_limit(0.601));
    }

    #[test]
    fn budget_fairness_allows_quality_12b_budget_to_dominate_when_preserved() {
        let text = "{\"workers\":[{\"role\":\"quality\",\"success\":true,\"feedback_applied\":0,\"runtime_tokens\":262144,\"latency_ms\":5000,\"default_max_tokens\":262144,\"configured_max_tokens\":262144,\"effective_max_tokens\":262144,\"max_tokens_clamped\":false,\"can_accept_low_priority_task\":false},{\"role\":\"summary\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":300,\"latency_ms\":120,\"default_max_tokens\":768,\"configured_max_tokens\":262144,\"effective_max_tokens\":768,\"max_tokens_clamped\":true,\"can_accept_low_priority_task\":true},{\"role\":\"review\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":250,\"latency_ms\":100,\"default_max_tokens\":1024,\"configured_max_tokens\":262144,\"effective_max_tokens\":1024,\"max_tokens_clamped\":true,\"can_accept_low_priority_task\":true},{\"role\":\"test-gate\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":150,\"latency_ms\":80,\"default_max_tokens\":1024,\"configured_max_tokens\":262144,\"effective_max_tokens\":1024,\"max_tokens_clamped\":true,\"can_accept_low_priority_task\":true}]}\n";
        let summary = parse_budget_fairness(text);
        let context = budget_fairness_context_text(&summary);

        assert_eq!(
            summary.max_role_runtime_token_share,
            Some(262144.0 / 262844.0)
        );
        assert!(!summary.budget_fairness_blocked);
        assert!(summary.allow_pool_expansion);
        assert!(summary.failure_reasons.is_empty());
        assert!(context.contains("quality:workers:1/1"));
        assert!(context.contains("default_max_tokens:262144"));
        assert!(context.contains("summary:workers:1/1"));
        assert!(context.contains("effective_max_tokens:768"));
        assert!(context.contains("max_tokens_clamped_count:1"));
    }

    #[test]
    fn budget_fairness_reports_latest_role_config_instead_of_historic_max() {
        let text = r#"{"workers":[
            {"round":1,"role":"quality","success":true,"feedback_applied":0,"runtime_tokens":50,"latency_ms":100,"runtime_backend":"llama.cpp","runtime_device":"metal","runtime_accelerator":"metal","gpu_layers":999,"default_max_tokens":262144,"configured_max_tokens":262144,"effective_max_tokens":262144,"max_tokens_clamped":false,"can_accept_low_priority_task":false},
            {"round":2,"role":"quality","success":true,"feedback_applied":0,"runtime_tokens":60,"latency_ms":100,"runtime_backend":"llama.cpp","runtime_device":"metal","runtime_accelerator":"metal","gpu_layers":999,"default_max_tokens":4096,"configured_max_tokens":4096,"effective_max_tokens":4096,"max_tokens_clamped":false,"can_accept_low_priority_task":false},
            {"round":2,"role":"summary","success":true,"feedback_applied":1,"runtime_tokens":300,"latency_ms":120,"default_max_tokens":768,"configured_max_tokens":4096,"effective_max_tokens":768,"max_tokens_clamped":true,"can_accept_low_priority_task":true},
            {"round":2,"role":"review","success":true,"feedback_applied":1,"runtime_tokens":250,"latency_ms":100,"default_max_tokens":1536,"configured_max_tokens":4096,"effective_max_tokens":1536,"max_tokens_clamped":true,"can_accept_low_priority_task":true},
            {"round":2,"role":"test-gate","success":true,"feedback_applied":1,"runtime_tokens":150,"latency_ms":80,"default_max_tokens":1536,"configured_max_tokens":4096,"effective_max_tokens":1536,"max_tokens_clamped":true,"can_accept_low_priority_task":true}
        ]}"#;
        let summary = parse_budget_fairness(text);
        let context = budget_fairness_context_text(&summary);
        let quality = summary
            .roles
            .iter()
            .find(|role| role.role == "quality")
            .unwrap();

        assert_eq!(quality.worker_count, 2);
        assert_eq!(quality.runtime_tokens, 110);
        assert_eq!(quality.default_max_tokens, Some(4096));
        assert_eq!(quality.configured_max_tokens, Some(4096));
        assert_eq!(quality.effective_max_tokens, Some(4096));
        assert_eq!(budget_policy_gate_failure(Some(&summary)), None);
        assert!(context.contains("quality:workers:2/2"));
        assert!(context.contains("quality:workers:2/2 feedback:0 tokens:110"));
        assert!(context.contains("default_max_tokens:4096"));
        assert!(!context.contains("quality:workers:2/2 feedback:0 tokens:110 latency_ms:200 runtime_backend:llama.cpp runtime_device:metal runtime_accelerator:metal gpu_layers:999 default_max_tokens:262144"));
    }

    #[test]
    fn budget_fairness_blocks_helper_primary_12b_blockers() {
        let text = "{\"workers\":[{\"role\":\"planner\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":300,\"latency_ms\":200},{\"role\":\"reviewer\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":300,\"latency_ms\":200,\"blocked_primary_12b\":true},{\"role\":\"tester\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":300,\"latency_ms\":200}]}\n";
        let summary = parse_budget_fairness(text);

        assert!(summary.budget_fairness_blocked);
        assert!(
            summary
                .failure_reasons
                .iter()
                .any(|reason| reason.contains("blocked primary 12B"))
        );
    }

    #[test]
    fn budget_fairness_accepts_pool_call_execution_fields() {
        let text = "{\"workers\":[{\"selected_role\":\"summary\",\"ok\":true,\"feedback_applied\":1,\"elapsed_ms\":120,\"answer_chars\":1200,\"answer_bytes\":1200,\"answer_approx_tokens\":300,\"runtime_backend\":\"llama.cpp\",\"runtime_device\":\"metal\",\"runtime_accelerator\":\"metal\",\"gpu_layers\":99,\"default_max_tokens\":768,\"configured_max_tokens\":4096,\"effective_max_tokens\":768,\"max_tokens_clamped\":true,\"can_accept_low_priority_task\":true},{\"selected_role\":\"review\",\"ok\":true,\"feedback_applied\":1,\"elapsed_ms\":100,\"answer_chars\":800,\"answer_bytes\":800,\"answer_approx_tokens\":200,\"default_max_tokens\":1024,\"configured_max_tokens\":4096,\"effective_max_tokens\":1024,\"max_tokens_clamped\":true,\"can_accept_low_priority_task\":true},{\"selected_role\":\"test-gate\",\"ok\":true,\"feedback_applied\":1,\"elapsed_ms\":80,\"answer_chars\":600,\"answer_bytes\":600,\"answer_approx_tokens\":150,\"default_max_tokens\":1024,\"configured_max_tokens\":4096,\"effective_max_tokens\":1024,\"max_tokens_clamped\":true,\"can_accept_low_priority_task\":true}]}\n";
        let summary = parse_budget_fairness(text);
        let context = budget_fairness_context_text(&summary);
        let json = option_budget_fairness_json(Some(&summary));

        assert_eq!(summary.worker_count, 3);
        assert_eq!(summary.successful_worker_count, 3);
        assert_eq!(summary.feedback_worker_count, 3);
        assert_eq!(summary.total_runtime_tokens, 650);
        assert_eq!(summary.total_latency_ms, 300);
        assert!(!summary.budget_fairness_blocked);
        assert!(summary.allow_pool_expansion);
        let summary_role = summary
            .roles
            .iter()
            .find(|role| role.role == "summary")
            .unwrap();
        assert_eq!(summary_role.runtime_tokens, 300);
        assert_eq!(summary_role.latency_ms, 120);
        assert_eq!(summary_role.runtime_backend.as_deref(), Some("llama.cpp"));
        assert_eq!(summary_role.runtime_device.as_deref(), Some("metal"));
        assert_eq!(summary_role.runtime_accelerator.as_deref(), Some("metal"));
        assert_eq!(summary_role.gpu_layers, Some(99));
        assert_eq!(summary_role.default_max_tokens, Some(768));
        assert_eq!(summary_role.configured_max_tokens, Some(4096));
        assert_eq!(summary_role.effective_max_tokens, Some(768));
        assert_eq!(summary_role.max_tokens_clamped_count, 1);
        assert_eq!(summary_role.low_priority_worker_count, 1);
        assert!(context.contains("configured_max_tokens:4096"));
        assert!(context.contains("effective_max_tokens:768"));
        assert!(context.contains("max_tokens_clamped_count:1"));
        assert!(json.contains("\"configured_max_tokens\":4096"));
        assert!(json.contains("\"effective_max_tokens\":768"));
        assert!(json.contains("\"max_tokens_clamped_count\":1"));
    }

    #[test]
    fn budget_fairness_blocks_helper_budget_reduction_without_clamp_evidence() {
        let text = "{\"workers\":[{\"role\":\"summary\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":300,\"latency_ms\":100,\"default_max_tokens\":768,\"configured_max_tokens\":4096,\"effective_max_tokens\":768,\"max_tokens_clamped\":false,\"can_accept_low_priority_task\":true},{\"role\":\"review\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":300,\"latency_ms\":100},{\"role\":\"test-gate\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":300,\"latency_ms\":100}]}\n";
        let summary = parse_budget_fairness(text);

        assert!(summary.budget_fairness_blocked);
        assert!(
            summary
                .failure_reasons
                .iter()
                .any(|reason| reason.contains("without clamp evidence"))
        );
    }

    #[test]
    fn budget_policy_allows_low_budget_helpers_without_clamp_evidence() {
        let text = "{\"workers\":[{\"role\":\"quality\",\"success\":true,\"feedback_applied\":0,\"runtime_tokens\":64,\"latency_ms\":5000,\"default_max_tokens\":262144,\"configured_max_tokens\":64,\"effective_max_tokens\":64,\"max_tokens_clamped\":false,\"can_accept_low_priority_task\":false},{\"role\":\"summary\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":10,\"latency_ms\":120,\"default_max_tokens\":768,\"configured_max_tokens\":64,\"effective_max_tokens\":64,\"max_tokens_clamped\":false,\"can_accept_low_priority_task\":true},{\"role\":\"review\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":10,\"latency_ms\":100,\"default_max_tokens\":1024,\"configured_max_tokens\":64,\"effective_max_tokens\":64,\"max_tokens_clamped\":false,\"can_accept_low_priority_task\":true},{\"role\":\"test-gate\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":10,\"latency_ms\":80,\"default_max_tokens\":1024,\"configured_max_tokens\":64,\"effective_max_tokens\":64,\"max_tokens_clamped\":false,\"can_accept_low_priority_task\":true}]}\n";
        let summary = parse_budget_fairness(text);

        assert!(!summary.budget_fairness_blocked);
        assert_eq!(budget_policy_gate_failure(Some(&summary)), None);
    }

    #[test]
    fn budget_policy_requires_clamp_when_helper_budget_exceeds_default() {
        let text = "{\"workers\":[{\"role\":\"quality\",\"success\":true,\"feedback_applied\":0,\"runtime_tokens\":262144,\"latency_ms\":5000,\"default_max_tokens\":262144,\"configured_max_tokens\":262144,\"effective_max_tokens\":262144,\"max_tokens_clamped\":false,\"can_accept_low_priority_task\":false},{\"role\":\"summary\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":300,\"latency_ms\":120,\"default_max_tokens\":768,\"configured_max_tokens\":262144,\"effective_max_tokens\":768,\"max_tokens_clamped\":false,\"can_accept_low_priority_task\":true},{\"role\":\"review\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":250,\"latency_ms\":100,\"default_max_tokens\":1024,\"configured_max_tokens\":262144,\"effective_max_tokens\":1024,\"max_tokens_clamped\":false,\"can_accept_low_priority_task\":true},{\"role\":\"test-gate\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":150,\"latency_ms\":80,\"default_max_tokens\":1024,\"configured_max_tokens\":262144,\"effective_max_tokens\":1024,\"max_tokens_clamped\":false,\"can_accept_low_priority_task\":true}]}\n";
        let summary = parse_budget_fairness(text);

        assert_eq!(
            budget_policy_gate_failure(Some(&summary)),
            Some(
                "model pool budget policy missing clamped low-priority helper evidence".to_owned()
            )
        );
    }

    #[test]
    fn budget_fairness_blocks_quality_budget_clamping() {
        let text = "{\"workers\":[{\"role\":\"summary\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":300,\"latency_ms\":100},{\"role\":\"review\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":300,\"latency_ms\":100},{\"role\":\"test-gate\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":300,\"latency_ms\":100},{\"role\":\"quality\",\"success\":true,\"feedback_applied\":0,\"runtime_tokens\":1,\"latency_ms\":1,\"configured_max_tokens\":262144,\"effective_max_tokens\":1024,\"max_tokens_clamped\":true,\"can_accept_low_priority_task\":false}]}\n";
        let summary = parse_budget_fairness(text);

        assert!(summary.budget_fairness_blocked);
        assert!(
            summary
                .failure_reasons
                .iter()
                .any(|reason| reason.contains("quality role budget was clamped"))
        );
    }

    #[test]
    fn budget_fairness_missing_artifact_renders_null() {
        assert_eq!(option_budget_fairness_json(None), "null");
    }

    #[test]
    fn load_budget_fairness_missing_file_returns_none() {
        let path = std::env::temp_dir().join(format!(
            "smartsteam-missing-budget-fairness-{}.json",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);

        assert_eq!(load_budget_fairness(Some(&path)).unwrap(), None);
    }

    #[test]
    fn appends_model_worker_events_as_budget_fairness_artifact() {
        let path = std::env::temp_dir().join(format!(
            "smartsteam-model-worker-events-{}.json",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);

        append_model_worker_event(
            &path,
            &ModelWorkerEvent {
                round: 1,
                case_name: "case-1".to_owned(),
                role: "summary".to_owned(),
                worker_port: Some(8687),
                worker_base_url: Some("http://127.0.0.1:8687".to_owned()),
                task_kind: "summary".to_owned(),
                execution_state: "executed".to_owned(),
                success: true,
                feedback_applied: 1,
                runtime_tokens: 400,
                latency_ms: 100,
                answer_chars: Some(12),
                answer_bytes: Some(12),
                answer_approx_tokens: Some(3),
                runtime_backend: Some("llama.cpp".to_owned()),
                runtime_device: Some("metal".to_owned()),
                runtime_accelerator: Some("metal".to_owned()),
                gpu_layers: Some(99),
                blocked_primary_12b: false,
                default_max_tokens: Some(768),
                configured_max_tokens: 4096,
                effective_max_tokens: 768,
                max_tokens_clamped: true,
                can_accept_low_priority_task: true,
            },
        )
        .unwrap();
        append_model_worker_event(
            &path,
            &ModelWorkerEvent {
                round: 2,
                case_name: "case-2".to_owned(),
                role: "review".to_owned(),
                worker_port: Some(8688),
                worker_base_url: Some("http://127.0.0.1:8688".to_owned()),
                task_kind: "review".to_owned(),
                execution_state: "executed".to_owned(),
                success: true,
                feedback_applied: 1,
                runtime_tokens: 300,
                latency_ms: 90,
                answer_chars: None,
                answer_bytes: None,
                answer_approx_tokens: None,
                runtime_backend: Some("llama.cpp".to_owned()),
                runtime_device: Some("metal".to_owned()),
                runtime_accelerator: Some("metal".to_owned()),
                gpu_layers: Some(80),
                blocked_primary_12b: false,
                default_max_tokens: Some(1024),
                configured_max_tokens: 4096,
                effective_max_tokens: 1024,
                max_tokens_clamped: true,
                can_accept_low_priority_task: true,
            },
        )
        .unwrap();

        let text = fs::read_to_string(&path).unwrap();
        let summary = parse_budget_fairness(&text);

        assert!(text.contains("\"schema\":\"model_worker_v1\""));
        assert!(text.contains("\"runtime_backend\":\"llama.cpp\""));
        assert!(text.contains("\"runtime_device\":\"metal\""));
        assert!(text.contains("\"runtime_accelerator\":\"metal\""));
        assert!(text.contains("\"gpu_layers\":99"));
        assert!(text.contains("\"default_max_tokens\":768"));
        assert!(text.contains("\"configured_max_tokens\":4096"));
        assert!(text.contains("\"effective_max_tokens\":768"));
        assert!(text.contains("\"max_tokens_clamped\":true"));
        assert!(text.contains("\"can_accept_low_priority_task\":true"));
        assert!(text.contains("\"answer_chars\":12"));
        assert!(text.contains("\"answer_bytes\":12"));
        assert!(text.contains("\"answer_approx_tokens\":3"));
        assert_eq!(summary.worker_count, 2);
        assert_eq!(summary.feedback_worker_count, 2);
        assert_eq!(summary.total_runtime_tokens, 700);
        let summary_role = summary
            .roles
            .iter()
            .find(|role| role.role == "summary")
            .unwrap();
        assert_eq!(summary_role.runtime_backend.as_deref(), Some("llama.cpp"));
        assert_eq!(summary_role.runtime_device.as_deref(), Some("metal"));
        assert_eq!(summary_role.runtime_accelerator.as_deref(), Some("metal"));
        assert_eq!(summary_role.gpu_layers, Some(99));
        assert_eq!(summary_role.default_max_tokens, Some(768));
        assert_eq!(summary_role.configured_max_tokens, Some(4096));
        assert_eq!(summary_role.effective_max_tokens, Some(768));
        assert_eq!(summary_role.max_tokens_clamped_count, 1);
        assert_eq!(summary_role.low_priority_worker_count, 1);
        assert!(summary.budget_fairness_blocked);
        assert!(
            summary
                .failure_reasons
                .iter()
                .any(|reason| reason.contains("required role tester"))
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn planned_model_worker_events_are_preserved_but_ignored_for_fairness() {
        let path = std::env::temp_dir().join(format!(
            "smartsteam-model-worker-planned-events-{}.json",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);

        append_model_worker_event(
            &path,
            &ModelWorkerEvent {
                round: 1,
                case_name: "case-1".to_owned(),
                role: "summary".to_owned(),
                worker_port: Some(8687),
                worker_base_url: Some("http://127.0.0.1:8687".to_owned()),
                task_kind: "summary".to_owned(),
                execution_state: "planned".to_owned(),
                success: false,
                feedback_applied: 0,
                runtime_tokens: 0,
                latency_ms: 0,
                answer_chars: None,
                answer_bytes: None,
                answer_approx_tokens: None,
                runtime_backend: Some("llama.cpp".to_owned()),
                runtime_device: Some("metal".to_owned()),
                runtime_accelerator: Some("metal".to_owned()),
                gpu_layers: Some(99),
                blocked_primary_12b: false,
                default_max_tokens: Some(768),
                configured_max_tokens: 4096,
                effective_max_tokens: 768,
                max_tokens_clamped: true,
                can_accept_low_priority_task: true,
            },
        )
        .unwrap();
        append_model_worker_event(
            &path,
            &ModelWorkerEvent {
                round: 1,
                case_name: "case-1".to_owned(),
                role: "review".to_owned(),
                worker_port: Some(8688),
                worker_base_url: Some("http://127.0.0.1:8688".to_owned()),
                task_kind: "review".to_owned(),
                execution_state: "executed".to_owned(),
                success: true,
                feedback_applied: 1,
                runtime_tokens: 300,
                latency_ms: 90,
                answer_chars: None,
                answer_bytes: None,
                answer_approx_tokens: None,
                runtime_backend: Some("llama.cpp".to_owned()),
                runtime_device: Some("metal".to_owned()),
                runtime_accelerator: Some("metal".to_owned()),
                gpu_layers: Some(80),
                blocked_primary_12b: false,
                default_max_tokens: Some(1024),
                configured_max_tokens: 4096,
                effective_max_tokens: 1024,
                max_tokens_clamped: true,
                can_accept_low_priority_task: true,
            },
        )
        .unwrap();

        let text = fs::read_to_string(&path).unwrap();
        let summary = parse_budget_fairness(&text);

        assert!(text.contains("\"execution_state\":\"planned\""));
        assert!(text.contains("\"role\":\"summary\""));
        assert_eq!(summary.worker_count, 1);
        assert_eq!(summary.roles.len(), 1);
        assert_eq!(summary.roles[0].role, "review");
        assert_eq!(summary.total_runtime_tokens, 300);
        let _ = fs::remove_file(path);
    }
}
