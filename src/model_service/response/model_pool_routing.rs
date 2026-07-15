use crate::model_service::json::service_json_string;

use super::model_pool::{ModelPoolMetricsSnapshotView, ModelPoolMetricsView, ModelPoolWorkerView};

const STRATEGY: &str = "rwaf_v1";
const UNKNOWN_COST_PENALTY: i32 = 25;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelPoolRoutingWeightsView {
    pub(crate) strategy: &'static str,
    pub(crate) prompt_chars: usize,
    pub(crate) task_complexity_score: u16,
    pub(crate) task_complexity_band: &'static str,
    pub(crate) history_penalty_applied: bool,
    pub(crate) resource_precheck: ModelPoolResourcePrecheckView,
    pub(crate) role_scores: Vec<ModelPoolRoleWeightView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelPoolRoleWeightView {
    pub(crate) role: String,
    pub(crate) base_rank: usize,
    pub(crate) score: i32,
    pub(crate) complexity_boost: i32,
    pub(crate) history_penalty: i32,
    pub(crate) resource_penalty: i32,
    pub(crate) latency_penalty: i32,
    pub(crate) latency_ms: Option<u64>,
    pub(crate) cost_known: bool,
    pub(crate) cost_per_1k_micro_usd: Option<u64>,
    pub(crate) remaining_budget_micro_usd: Option<u64>,
    pub(crate) cost_penalty: i32,
    pub(crate) resource_pressure: &'static str,
    pub(crate) in_flight: u64,
    pub(crate) failure_count: u64,
    pub(crate) success_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelPoolResourcePrecheckView {
    pub(crate) strategy: &'static str,
    pub(crate) pressure: &'static str,
    pub(crate) allow_dispatch: bool,
    pub(crate) reason: &'static str,
    pub(crate) total_in_flight: u64,
    pub(crate) avoid_roles: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelPoolDependencyPrecheckView {
    pub(crate) strategy: &'static str,
    pub(crate) checked: bool,
    pub(crate) requested_role: String,
    pub(crate) allow_dispatch: bool,
    pub(crate) reason: &'static str,
    pub(crate) required_roles: Vec<String>,
    pub(crate) completed_roles: Vec<String>,
    pub(crate) missing_roles: Vec<String>,
}

pub(super) fn model_pool_route_candidates_with_weights(
    task_kind: &str,
    configured_max_tokens: Option<usize>,
    prompt: Option<&str>,
    workers: &[ModelPoolWorkerView],
    metrics: Option<&ModelPoolMetricsSnapshotView>,
    base_candidates: Vec<String>,
) -> (Vec<String>, ModelPoolRoutingWeightsView) {
    let complexity = task_complexity(prompt, configured_max_tokens);
    let mut candidates = base_candidates;
    if should_allow_quality_for_complex_auto(task_kind, &complexity, &candidates, workers) {
        candidates.insert(0, "quality".to_owned());
    }
    let explicit_single_role = candidates.len() <= 1;
    let mut scored = candidates
        .iter()
        .enumerate()
        .map(|(base_rank, role)| {
            role_weight(
                role,
                base_rank,
                candidates.len(),
                &complexity,
                workers.iter().find(|worker| worker.role == *role),
                worker_metrics(metrics, role),
            )
        })
        .collect::<Vec<_>>();
    let history_penalty_applied = scored.iter().any(|role| role.history_penalty > 0);
    let resource_precheck = resource_precheck_view(&scored);
    if !explicit_single_role {
        scored.sort_by(|left, right| {
            right
                .score
                .cmp(&left.score)
                .then_with(|| left.base_rank.cmp(&right.base_rank))
        });
        candidates = scored.iter().map(|role| role.role.clone()).collect();
    }
    let role_scores = candidates
        .iter()
        .filter_map(|role| scored.iter().find(|score| &score.role == role).cloned())
        .collect();
    (
        candidates,
        ModelPoolRoutingWeightsView {
            strategy: STRATEGY,
            prompt_chars: complexity.prompt_chars,
            task_complexity_score: complexity.score,
            task_complexity_band: complexity.band(),
            history_penalty_applied,
            resource_precheck,
            role_scores,
        },
    )
}

pub(super) fn routing_weights_json(weights: &ModelPoolRoutingWeightsView) -> String {
    let role_scores = weights
        .role_scores
        .iter()
        .map(role_weight_json)
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"strategy\":{},\"prompt_chars\":{},\"task_complexity_score\":{},\"task_complexity_band\":{},\"history_penalty_applied\":{},\"resource_precheck\":{},\"role_scores\":[{}]}}",
        service_json_string(weights.strategy),
        weights.prompt_chars,
        weights.task_complexity_score,
        service_json_string(weights.task_complexity_band),
        weights.history_penalty_applied,
        resource_precheck_json(&weights.resource_precheck),
        role_scores
    )
}

fn role_weight_json(weight: &ModelPoolRoleWeightView) -> String {
    format!(
        "{{\"role\":{},\"base_rank\":{},\"score\":{},\"complexity_boost\":{},\"history_penalty\":{},\"resource_penalty\":{},\"latency_penalty\":{},\"latency_ms\":{},\"cost_known\":{},\"cost_per_1k_micro_usd\":{},\"remaining_budget_micro_usd\":{},\"cost_penalty\":{},\"resource_pressure\":{},\"in_flight\":{},\"failure_count\":{},\"success_count\":{}}}",
        service_json_string(&weight.role),
        weight.base_rank,
        weight.score,
        weight.complexity_boost,
        weight.history_penalty,
        weight.resource_penalty,
        weight.latency_penalty,
        option_u64_json(weight.latency_ms),
        weight.cost_known,
        option_u64_json(weight.cost_per_1k_micro_usd),
        option_u64_json(weight.remaining_budget_micro_usd),
        weight.cost_penalty,
        service_json_string(weight.resource_pressure),
        weight.in_flight,
        weight.failure_count,
        weight.success_count
    )
}

fn option_u64_json(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

fn resource_precheck_json(precheck: &ModelPoolResourcePrecheckView) -> String {
    let avoid_roles = precheck
        .avoid_roles
        .iter()
        .map(|role| service_json_string(role))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"strategy\":{},\"pressure\":{},\"allow_dispatch\":{},\"reason\":{},\"total_in_flight\":{},\"avoid_roles\":[{}]}}",
        service_json_string(precheck.strategy),
        service_json_string(precheck.pressure),
        precheck.allow_dispatch,
        service_json_string(precheck.reason),
        precheck.total_in_flight,
        avoid_roles
    )
}

pub(crate) fn dependency_precheck_json(precheck: &ModelPoolDependencyPrecheckView) -> String {
    format!(
        "{{\"strategy\":{},\"checked\":{},\"requested_role\":{},\"allow_dispatch\":{},\"reason\":{},\"required_roles\":{},\"completed_roles\":{},\"missing_roles\":{}}}",
        service_json_string(precheck.strategy),
        precheck.checked,
        service_json_string(&precheck.requested_role),
        precheck.allow_dispatch,
        service_json_string(precheck.reason),
        string_array_json(&precheck.required_roles),
        string_array_json(&precheck.completed_roles),
        string_array_json(&precheck.missing_roles)
    )
}

pub(crate) fn model_pool_dependency_precheck(
    requested_role: &str,
    completed_roles: Option<&[String]>,
) -> ModelPoolDependencyPrecheckView {
    let requested_role = canonical_role(requested_role);
    let required_roles = dependency_required_roles(&requested_role);
    let Some(completed_roles) = completed_roles else {
        return ModelPoolDependencyPrecheckView {
            strategy: "role_dependency_graph_v1",
            checked: false,
            requested_role,
            allow_dispatch: true,
            reason: "completed_roles_not_provided",
            required_roles,
            completed_roles: Vec::new(),
            missing_roles: Vec::new(),
        };
    };
    let completed_roles = normalized_completed_roles(completed_roles);
    let missing_roles = required_roles
        .iter()
        .filter(|role| !completed_roles.iter().any(|completed| completed == *role))
        .cloned()
        .collect::<Vec<_>>();
    ModelPoolDependencyPrecheckView {
        strategy: "role_dependency_graph_v1",
        checked: true,
        requested_role,
        allow_dispatch: missing_roles.is_empty(),
        reason: if required_roles.is_empty() {
            "no_prerequisites"
        } else if missing_roles.is_empty() {
            "dependencies_satisfied"
        } else {
            "missing_required_roles"
        },
        required_roles,
        completed_roles,
        missing_roles,
    }
}

fn dependency_required_roles(role: &str) -> Vec<String> {
    match canonical_role(role).as_str() {
        "summary" => vec!["quality".to_owned()],
        "router" => vec!["summary".to_owned()],
        "review" => vec!["summary".to_owned()],
        "index" => vec!["summary".to_owned(), "router".to_owned()],
        "test-gate" => vec!["review".to_owned(), "index".to_owned()],
        _ => Vec::new(),
    }
}

fn normalized_completed_roles(roles: &[String]) -> Vec<String> {
    let mut normalized = Vec::new();
    for role in roles {
        let role = canonical_role(role);
        if matches!(role.as_str(), "" | "auto" | "chat" | "business-cycle") {
            continue;
        }
        if !normalized.iter().any(|existing| existing == &role) {
            normalized.push(role);
        }
    }
    normalized
}

fn canonical_role(role: &str) -> String {
    match role.trim().to_ascii_lowercase().as_str() {
        "primary" | "generate" | "generation" => "quality".to_owned(),
        "route" | "intent" | "intent-classify" | "preflight" | "tool-call" | "tool_calls"
        | "function" | "function-call" | "function_call" => "router".to_owned(),
        "test" | "gate" => "test-gate".to_owned(),
        "repo-index" | "repository-index" | "spare" => "index".to_owned(),
        "business_cycle" | "business" => "business-cycle".to_owned(),
        other => other.to_owned(),
    }
}

fn string_array_json(values: &[String]) -> String {
    let items = values
        .iter()
        .map(|value| service_json_string(value))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TaskComplexity {
    prompt_chars: usize,
    score: u16,
}

impl TaskComplexity {
    fn band(self) -> &'static str {
        if self.score >= 130 {
            "complex"
        } else if self.score >= 85 {
            "moderate"
        } else {
            "simple"
        }
    }
}

fn task_complexity(prompt: Option<&str>, configured_max_tokens: Option<usize>) -> TaskComplexity {
    let prompt = prompt.unwrap_or_default();
    let prompt_chars = prompt.chars().count();
    let mut score = 50_u16;
    score = score.saturating_add(length_score(prompt_chars));
    score = score.saturating_add(intent_marker_score(prompt));
    if configured_max_tokens.is_some_and(|tokens| tokens > 4096) {
        score = score.saturating_add(30);
    }
    TaskComplexity {
        prompt_chars,
        score: score.min(180),
    }
}

fn length_score(prompt_chars: usize) -> u16 {
    match prompt_chars {
        0..=280 => 0,
        281..=1200 => 20,
        1201..=4000 => 45,
        _ => 70,
    }
}

fn intent_marker_score(prompt: &str) -> u16 {
    let lower = prompt.to_ascii_lowercase();
    let markers = [
        "rust",
        "cargo",
        "test",
        "error",
        "trace",
        "json",
        "stream",
        "index",
        "routing",
        "architecture",
        "代码",
        "测试",
        "错误",
        "日志",
        "索引",
        "路由",
        "架构",
        "长文本",
    ];
    let hits = markers
        .iter()
        .filter(|marker| lower.contains(**marker) || prompt.contains(**marker))
        .count();
    (hits.min(5) as u16) * 10
}

fn should_allow_quality_for_complex_auto(
    task_kind: &str,
    complexity: &TaskComplexity,
    candidates: &[String],
    workers: &[ModelPoolWorkerView],
) -> bool {
    matches!(task_kind.trim().to_ascii_lowercase().as_str(), "" | "auto")
        && complexity.band() == "complex"
        && !candidates.iter().any(|role| role == "quality")
        && workers.iter().any(|worker| worker.role == "quality")
}

fn role_weight(
    role: &str,
    base_rank: usize,
    candidate_count: usize,
    complexity: &TaskComplexity,
    worker: Option<&ModelPoolWorkerView>,
    metrics: Option<&ModelPoolMetricsView>,
) -> ModelPoolRoleWeightView {
    let base_score = ((candidate_count.saturating_sub(base_rank)) as i32) * 100;
    let complexity_boost = complexity_boost(role, complexity);
    let (history_penalty, failure_count, success_count, in_flight) = history_penalty(metrics);
    let (resource_penalty, resource_pressure) = resource_penalty(worker, metrics);
    let (latency_penalty, latency_ms) = latency_penalty(metrics);
    let (cost_penalty, cost_known, cost_per_1k_micro_usd, remaining_budget_micro_usd) =
        cost_penalty(worker);
    let score = base_score + complexity_boost
        - history_penalty
        - resource_penalty
        - latency_penalty
        - cost_penalty;
    ModelPoolRoleWeightView {
        role: role.to_owned(),
        base_rank,
        score,
        complexity_boost,
        history_penalty,
        resource_penalty,
        latency_penalty,
        latency_ms,
        cost_known,
        cost_per_1k_micro_usd,
        remaining_budget_micro_usd,
        cost_penalty,
        resource_pressure,
        in_flight,
        failure_count,
        success_count,
    }
}

fn complexity_boost(role: &str, complexity: &TaskComplexity) -> i32 {
    match (role, complexity.band()) {
        ("quality", "complex") => 180,
        ("quality", "moderate") => 40,
        ("summary", "simple") => 25,
        ("router", "simple") => 20,
        ("review", "moderate" | "complex") => 35,
        ("index", "moderate" | "complex") => 25,
        ("test-gate", "complex") => 15,
        _ => 0,
    }
}

fn history_penalty(metrics: Option<&ModelPoolMetricsView>) -> (i32, u64, u64, u64) {
    let Some(metrics) = metrics else {
        return (0, 0, 0, 0);
    };
    let calls = metrics.success_count.saturating_add(metrics.failure_count);
    if calls == 0 {
        return (
            0,
            metrics.failure_count,
            metrics.success_count,
            metrics.in_flight,
        );
    }
    let failure_rate_penalty = ((metrics.failure_count.saturating_mul(100)) / calls) as i32;
    let repeated_failure_penalty = if metrics.failure_count > metrics.success_count {
        60
    } else {
        0
    };
    let in_flight_penalty = metrics.in_flight.min(10) as i32 * 5;
    (
        failure_rate_penalty + repeated_failure_penalty + in_flight_penalty,
        metrics.failure_count,
        metrics.success_count,
        metrics.in_flight,
    )
}

fn resource_penalty(
    worker: Option<&ModelPoolWorkerView>,
    metrics: Option<&ModelPoolMetricsView>,
) -> (i32, &'static str) {
    let in_flight = metrics.map(|metrics| metrics.in_flight).unwrap_or(0);
    let runtime_penalty = worker.map(runtime_resource_penalty).unwrap_or(0);
    let in_flight_penalty = match in_flight {
        0 => 0,
        1 => 25,
        2 => 80,
        3..=5 => 160,
        _ => 260,
    };
    let penalty = in_flight_penalty + runtime_penalty;
    let pressure = if penalty >= 160 {
        "high"
    } else if penalty >= 40 {
        "medium"
    } else {
        "green"
    };
    (penalty, pressure)
}

fn latency_penalty(metrics: Option<&ModelPoolMetricsView>) -> (i32, Option<u64>) {
    let latency_ms = metrics.and_then(|metrics| metrics.latency_p95_ms.or(metrics.avg_latency_ms));
    let penalty = latency_ms
        .map(|latency_ms| (latency_ms / 50).min(300) as i32)
        .unwrap_or(0);
    (penalty, latency_ms)
}

fn cost_penalty(worker: Option<&ModelPoolWorkerView>) -> (i32, bool, Option<u64>, Option<u64>) {
    let Some(worker) = worker else {
        return (UNKNOWN_COST_PENALTY, false, None, None);
    };
    let Some(cost_per_1k_micro_usd) = worker.configured_cost_per_1k_micro_usd() else {
        return (
            UNKNOWN_COST_PENALTY,
            false,
            None,
            worker.remaining_budget_micro_usd,
        );
    };
    let cost_penalty = ((cost_per_1k_micro_usd.saturating_add(24)) / 25).min(500) as i32;
    let budget_penalty = match worker.remaining_budget_micro_usd {
        Some(0) => 500,
        Some(budget) if budget < cost_per_1k_micro_usd => 120,
        _ => 0,
    };
    (
        cost_penalty.saturating_add(budget_penalty).min(700),
        true,
        Some(cost_per_1k_micro_usd),
        worker.remaining_budget_micro_usd,
    )
}

fn runtime_resource_penalty(worker: &ModelPoolWorkerView) -> i32 {
    if !worker.ready() {
        return 0;
    }
    match (
        worker.runtime_device.as_deref(),
        worker.runtime_accelerator.as_deref(),
        worker.gpu_layers,
    ) {
        (Some("metal"), Some("metal"), Some(layers)) if layers > 0 => 0,
        (_, _, Some(0)) => 120,
        (Some("cpu"), _, _) => 90,
        (None, _, _) | (_, None, _) => 40,
        _ => 30,
    }
}

fn resource_precheck_view(scored: &[ModelPoolRoleWeightView]) -> ModelPoolResourcePrecheckView {
    let total_in_flight = scored.iter().map(|role| role.in_flight).sum();
    let avoid_roles = scored
        .iter()
        .filter(|role| role.resource_pressure == "high")
        .map(|role| role.role.clone())
        .collect::<Vec<_>>();
    let pressure = if scored.iter().any(|role| role.resource_pressure == "high") {
        "high"
    } else if scored.iter().any(|role| role.resource_pressure == "medium") {
        "medium"
    } else {
        "green"
    };
    let all_candidates_high =
        !scored.is_empty() && scored.iter().all(|role| role.resource_pressure == "high");
    ModelPoolResourcePrecheckView {
        strategy: "resource_precheck_v1",
        pressure,
        allow_dispatch: !all_candidates_high,
        reason: if all_candidates_high {
            "all_candidates_resource_constrained"
        } else if !avoid_roles.is_empty() {
            "resource_constrained_candidates_demoted"
        } else {
            "resource_pressure_green"
        },
        total_in_flight,
        avoid_roles,
    }
}

fn worker_metrics<'a>(
    metrics: Option<&'a ModelPoolMetricsSnapshotView>,
    role: &str,
) -> Option<&'a ModelPoolMetricsView> {
    metrics?
        .worker_metrics
        .iter()
        .find(|worker| worker.role == role)
        .map(|worker| &worker.metrics)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn worker(role: &str) -> ModelPoolWorkerView {
        ModelPoolWorkerView {
            role: role.to_owned(),
            port: 8686,
            base_url: format!("http://127.0.0.1:{}", 8686),
            enabled_by_default: true,
            model_class: "helper".to_owned(),
            suggested_quant: "Q4".to_owned(),
            default_context_tokens: 4096,
            default_max_tokens: 512,
            low_priority: role != "quality",
            reachable: true,
            model: Some(format!("{role}.gguf")),
            context_window: Some(4096),
            runtime_backend: Some("llama.cpp".to_owned()),
            runtime_device: Some("metal".to_owned()),
            runtime_accelerator: Some("metal".to_owned()),
            gpu_layers: Some(999),
            input_cost_per_1k_micro_usd: None,
            output_cost_per_1k_micro_usd: None,
            remaining_budget_micro_usd: None,
            error: None,
            quarantine: None,
        }
    }

    #[test]
    fn complex_auto_prompt_promotes_quality_candidate() {
        let workers = vec![worker("quality"), worker("summary"), worker("review")];
        let prompt =
            "请审查这段 Rust 架构和测试日志，包含 routing、index、json、trace、error，并给出改进。"
                .repeat(45);
        let (candidates, weights) = model_pool_route_candidates_with_weights(
            "auto",
            None,
            Some(&prompt),
            &workers,
            None,
            vec!["summary".to_owned(), "review".to_owned()],
        );

        assert_eq!(weights.task_complexity_band, "complex");
        assert_eq!(candidates[0], "quality");
        assert!(routing_weights_json(&weights).contains("\"strategy\":\"rwaf_v1\""));
    }

    #[test]
    fn history_penalty_can_demote_failed_fallback() {
        let workers = vec![worker("test-gate"), worker("review")];
        let metrics = ModelPoolMetricsSnapshotView {
            route_metrics: ModelPoolMetricsView::default(),
            worker_metrics: vec![
                super::super::model_pool::ModelPoolWorkerMetricsView {
                    role: "test-gate".to_owned(),
                    metrics: ModelPoolMetricsView {
                        failure_count: 4,
                        success_count: 0,
                        ..ModelPoolMetricsView::default()
                    },
                },
                super::super::model_pool::ModelPoolWorkerMetricsView {
                    role: "review".to_owned(),
                    metrics: ModelPoolMetricsView {
                        failure_count: 0,
                        success_count: 4,
                        ..ModelPoolMetricsView::default()
                    },
                },
            ],
        };
        let (candidates, weights) = model_pool_route_candidates_with_weights(
            "test-gate",
            None,
            None,
            &workers,
            Some(&metrics),
            vec!["test-gate".to_owned(), "review".to_owned()],
        );

        assert_eq!(candidates, vec!["review", "test-gate"]);
        assert!(weights.history_penalty_applied);
    }

    #[test]
    fn resource_precheck_demotes_busy_candidate_before_dispatch() {
        let workers = vec![worker("summary"), worker("router")];
        let metrics = ModelPoolMetricsSnapshotView {
            route_metrics: ModelPoolMetricsView::default(),
            worker_metrics: vec![
                super::super::model_pool::ModelPoolWorkerMetricsView {
                    role: "summary".to_owned(),
                    metrics: ModelPoolMetricsView {
                        in_flight: 4,
                        success_count: 8,
                        ..ModelPoolMetricsView::default()
                    },
                },
                super::super::model_pool::ModelPoolWorkerMetricsView {
                    role: "router".to_owned(),
                    metrics: ModelPoolMetricsView {
                        success_count: 8,
                        ..ModelPoolMetricsView::default()
                    },
                },
            ],
        };
        let (candidates, weights) = model_pool_route_candidates_with_weights(
            "auto",
            None,
            Some("short route request"),
            &workers,
            Some(&metrics),
            vec!["summary".to_owned(), "router".to_owned()],
        );
        let json = routing_weights_json(&weights);

        assert_eq!(candidates, vec!["router", "summary"]);
        assert_eq!(weights.resource_precheck.pressure, "high");
        assert_eq!(weights.resource_precheck.avoid_roles, vec!["summary"]);
        assert!(json.contains("\"resource_precheck\":{\"strategy\":\"resource_precheck_v1\""));
        assert!(json.contains("\"reason\":\"resource_constrained_candidates_demoted\""));
        assert!(json.contains("\"resource_penalty\":160"));
        assert!(json.contains("\"resource_pressure\":\"high\""));
    }

    #[test]
    fn resource_precheck_marks_all_candidates_constrained() {
        let workers = vec![worker("summary"), worker("router")];
        let metrics = ModelPoolMetricsSnapshotView {
            route_metrics: ModelPoolMetricsView::default(),
            worker_metrics: vec![
                super::super::model_pool::ModelPoolWorkerMetricsView {
                    role: "summary".to_owned(),
                    metrics: ModelPoolMetricsView {
                        in_flight: 4,
                        success_count: 8,
                        ..ModelPoolMetricsView::default()
                    },
                },
                super::super::model_pool::ModelPoolWorkerMetricsView {
                    role: "router".to_owned(),
                    metrics: ModelPoolMetricsView {
                        in_flight: 4,
                        success_count: 8,
                        ..ModelPoolMetricsView::default()
                    },
                },
            ],
        };
        let (_, weights) = model_pool_route_candidates_with_weights(
            "auto",
            None,
            Some("short route request"),
            &workers,
            Some(&metrics),
            vec!["summary".to_owned(), "router".to_owned()],
        );
        let json = routing_weights_json(&weights);

        assert!(!weights.resource_precheck.allow_dispatch);
        assert_eq!(
            weights.resource_precheck.reason,
            "all_candidates_resource_constrained"
        );
        assert_eq!(
            weights.resource_precheck.avoid_roles,
            vec!["summary", "router"]
        );
        assert!(json.contains("\"allow_dispatch\":false"));
        assert!(json.contains("\"reason\":\"all_candidates_resource_constrained\""));
    }

    #[test]
    fn latency_penalty_demotes_slow_p95_candidate() {
        let workers = vec![worker("summary"), worker("router")];
        let metrics = ModelPoolMetricsSnapshotView {
            route_metrics: ModelPoolMetricsView::default(),
            worker_metrics: vec![
                super::super::model_pool::ModelPoolWorkerMetricsView {
                    role: "summary".to_owned(),
                    metrics: ModelPoolMetricsView {
                        success_count: 4,
                        latency_p95_ms: Some(10_000),
                        ..ModelPoolMetricsView::default()
                    },
                },
                super::super::model_pool::ModelPoolWorkerMetricsView {
                    role: "router".to_owned(),
                    metrics: ModelPoolMetricsView {
                        success_count: 4,
                        latency_p95_ms: Some(250),
                        ..ModelPoolMetricsView::default()
                    },
                },
            ],
        };

        let (candidates, weights) = model_pool_route_candidates_with_weights(
            "auto",
            None,
            Some("route a short request"),
            &workers,
            Some(&metrics),
            vec!["summary".to_owned(), "router".to_owned()],
        );
        let json = routing_weights_json(&weights);

        assert_eq!(candidates, vec!["router", "summary"]);
        assert!(json.contains("\"latency_penalty\":200"));
        assert!(json.contains("\"latency_ms\":10000"));
    }

    #[test]
    fn latency_penalty_falls_back_to_average_latency() {
        let (_, latency_ms) = latency_penalty(Some(&ModelPoolMetricsView {
            avg_latency_ms: Some(750),
            latency_p95_ms: None,
            ..ModelPoolMetricsView::default()
        }));

        assert_eq!(latency_ms, Some(750));
    }

    #[test]
    fn unknown_cost_is_not_treated_as_free_route_advantage() {
        let workers = vec![worker("summary"), worker("router")];
        let (_, weights) = model_pool_route_candidates_with_weights(
            "auto",
            None,
            Some("route a short request"),
            &workers,
            None,
            vec!["summary".to_owned(), "router".to_owned()],
        );
        let json = routing_weights_json(&weights);

        assert!(weights.role_scores.iter().all(|role| !role.cost_known));
        assert!(
            weights
                .role_scores
                .iter()
                .all(|role| role.cost_penalty == UNKNOWN_COST_PENALTY)
        );
        assert!(json.contains("\"cost_known\":false"));
        assert!(json.contains("\"cost_per_1k_micro_usd\":null"));
        assert!(json.contains("\"cost_penalty\":25"));
    }

    #[test]
    fn configured_lower_cost_can_win_when_route_signals_match() {
        let mut expensive = worker("summary");
        expensive.input_cost_per_1k_micro_usd = Some(4_000);
        expensive.output_cost_per_1k_micro_usd = Some(6_000);
        expensive.remaining_budget_micro_usd = Some(1_000_000);
        let mut cheap = worker("router");
        cheap.input_cost_per_1k_micro_usd = Some(20);
        cheap.output_cost_per_1k_micro_usd = Some(30);
        cheap.remaining_budget_micro_usd = Some(1_000_000);

        let (candidates, weights) = model_pool_route_candidates_with_weights(
            "auto",
            None,
            Some("route a short request"),
            &[expensive, cheap],
            None,
            vec!["summary".to_owned(), "router".to_owned()],
        );
        let json = routing_weights_json(&weights);

        assert_eq!(candidates, vec!["router", "summary"]);
        assert!(weights.role_scores.iter().all(|role| role.cost_known));
        assert!(json.contains("\"role\":\"summary\""));
        assert!(json.contains("\"cost_per_1k_micro_usd\":10000"));
        assert!(json.contains("\"cost_penalty\":400"));
        assert!(json.contains("\"role\":\"router\""));
        assert!(json.contains("\"cost_per_1k_micro_usd\":50"));
        assert!(json.contains("\"cost_penalty\":2"));
        assert!(json.contains("\"remaining_budget_micro_usd\":1000000"));
    }

    #[test]
    fn dependency_precheck_reports_missing_prerequisites() {
        let completed = vec!["quality".to_owned(), "summary".to_owned()];
        let precheck = model_pool_dependency_precheck("test-gate", Some(&completed));
        let json = dependency_precheck_json(&precheck);

        assert!(precheck.checked);
        assert!(!precheck.allow_dispatch);
        assert_eq!(precheck.required_roles, vec!["review", "index"]);
        assert_eq!(precheck.missing_roles, vec!["review", "index"]);
        assert!(json.contains("\"strategy\":\"role_dependency_graph_v1\""));
        assert!(json.contains("\"reason\":\"missing_required_roles\""));
    }

    #[test]
    fn dependency_precheck_allows_satisfied_stage_order() {
        let completed = vec![
            "primary".to_owned(),
            "summary".to_owned(),
            "route".to_owned(),
            "review".to_owned(),
            "index".to_owned(),
        ];
        let precheck = model_pool_dependency_precheck("test", Some(&completed));

        assert!(precheck.checked);
        assert!(precheck.allow_dispatch);
        assert_eq!(
            precheck.completed_roles,
            vec![
                "quality".to_owned(),
                "summary".to_owned(),
                "router".to_owned(),
                "review".to_owned(),
                "index".to_owned()
            ]
        );
        assert_eq!(precheck.reason, "dependencies_satisfied");
    }

    #[test]
    fn dependency_precheck_is_compatible_when_client_omits_completed_roles() {
        let precheck = model_pool_dependency_precheck("index", None);

        assert!(!precheck.checked);
        assert!(precheck.allow_dispatch);
        assert_eq!(precheck.required_roles, vec!["summary", "router"]);
        assert_eq!(precheck.reason, "completed_roles_not_provided");
    }
}
