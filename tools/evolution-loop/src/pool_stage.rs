use std::env;
use std::path::PathBuf;

use crate::args::Config;
use crate::json::json_string;
use crate::pool_artifacts;

pub(crate) const TEST_GATE_STAGE_MAX_TOKENS: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PoolStageDispatchPlan {
    pub(crate) task_kind: String,
    pub(crate) selected_role: String,
    pub(crate) selected_port: Option<u64>,
    pub(crate) selected_base_url: Option<String>,
    pub(crate) context_window: Option<u64>,
    pub(crate) default_max_tokens: Option<u64>,
    pub(crate) runtime_backend: Option<String>,
    pub(crate) runtime_device: Option<String>,
    pub(crate) runtime_accelerator: Option<String>,
    pub(crate) gpu_layers: Option<u64>,
    pub(crate) configured_max_tokens: usize,
    pub(crate) effective_max_tokens: usize,
    pub(crate) max_tokens_clamped: bool,
    pub(crate) can_accept_low_priority_task: bool,
}

impl PoolStageDispatchPlan {
    pub(crate) fn request_json(&self) -> String {
        format!(
            "{{\"task_kind\":{},\"selected_role\":{},\"selected_port\":{},\"selected_base_url\":{},\"context_window\":{},\"default_max_tokens\":{},\"runtime_backend\":{},\"runtime_device\":{},\"runtime_accelerator\":{},\"gpu_layers\":{},\"configured_max_tokens\":{},\"effective_max_tokens\":{},\"max_tokens_clamped\":{},\"can_accept_low_priority_task\":{}}}",
            json_string(&self.task_kind),
            json_string(&self.selected_role),
            option_u64_json(self.selected_port),
            option_str_json(self.selected_base_url.as_deref()),
            option_u64_json(self.context_window),
            option_u64_json(self.default_max_tokens),
            option_str_json(self.runtime_backend.as_deref()),
            option_str_json(self.runtime_device.as_deref()),
            option_str_json(self.runtime_accelerator.as_deref()),
            option_u64_json(self.gpu_layers),
            self.configured_max_tokens,
            self.effective_max_tokens,
            self.max_tokens_clamped,
            self.can_accept_low_priority_task
        )
    }

    pub(crate) fn meta(&self) -> String {
        format!(
            "pool_stage_dispatch task_kind={} selected_role={} port={} endpoint={} context_window={} default_max_tokens={} runtime_backend={} runtime_device={} runtime_accelerator={} gpu_layers={} configured_max_tokens={} effective_max_tokens={} max_tokens_clamped={} low_priority={}",
            self.task_kind,
            self.selected_role,
            option_u64_text(self.selected_port),
            self.selected_base_url.as_deref().unwrap_or("none"),
            option_u64_text(self.context_window),
            option_u64_text(self.default_max_tokens),
            self.runtime_backend.as_deref().unwrap_or("none"),
            self.runtime_device.as_deref().unwrap_or("none"),
            self.runtime_accelerator.as_deref().unwrap_or("none"),
            option_u64_text(self.gpu_layers),
            self.configured_max_tokens,
            self.effective_max_tokens,
            self.max_tokens_clamped,
            self.can_accept_low_priority_task
        )
    }
}

pub(crate) fn request_json_field(plans: &[PoolStageDispatchPlan]) -> String {
    if plans.is_empty() {
        return String::new();
    }
    let items = plans
        .iter()
        .map(PoolStageDispatchPlan::request_json)
        .collect::<Vec<_>>()
        .join(",");
    format!(",\"pool_stage_dispatch\":[{items}]")
}

pub(crate) fn task_kinds(config: &Config) -> Vec<String> {
    config
        .pool_stage_route_task_kinds
        .iter()
        .filter(|kind| !kind.trim().is_empty())
        .cloned()
        .collect()
}

pub(crate) fn route_path(config: &Config, task_kind: &str) -> PathBuf {
    if is_primary_route_task_kind(config, task_kind)
        && let Some(primary) = &config.pool_route_json_path
    {
        return primary.clone();
    }
    let file_name = format!("pool-route-{}.json", sanitize_route_task_kind(task_kind));
    if let Some(primary) = &config.pool_route_json_path
        && let Some(parent) = primary.parent()
        && !parent.as_os_str().is_empty()
    {
        let stage_path = parent.join(&file_name);
        if stage_path == *primary {
            return parent.join(format!(
                "pool-stage-route-{}.json",
                sanitize_route_task_kind(task_kind)
            ));
        }
        return stage_path;
    }
    PathBuf::from("target/evolution").join(file_name)
}

pub(crate) fn is_primary_route_task_kind(config: &Config, task_kind: &str) -> bool {
    config
        .pool_route_task_kind
        .trim()
        .eq_ignore_ascii_case(task_kind.trim())
}

pub(crate) fn task_kinds_text(kinds: &[String]) -> String {
    if kinds.is_empty() {
        "none".to_owned()
    } else {
        kinds.join(",")
    }
}

#[cfg(test)]
pub(crate) fn allocation_evidence(config: &Config) -> Result<Vec<String>, String> {
    let mut evidence = Vec::new();
    for (task_kind, route) in route_summaries(config)? {
        evidence.push(format!(
            "pool_stage_route[{task_kind}] {}",
            pool_artifacts::route_context_text(&route)
        ));
    }
    Ok(evidence)
}

pub(crate) fn route_summaries(
    config: &Config,
) -> Result<Vec<(String, pool_artifacts::PoolRouteSummary)>, String> {
    let mut routes = Vec::new();
    for task_kind in task_kinds(config) {
        if is_primary_route_task_kind(config, &task_kind) {
            continue;
        }
        let path = route_path(config, &task_kind);
        if path.exists()
            && let Some(route) = pool_artifacts::load_route(Some(&path))?
        {
            routes.push((task_kind, route));
        }
    }
    Ok(routes)
}

pub(crate) fn dispatch_plans(config: &Config) -> Result<Vec<PoolStageDispatchPlan>, String> {
    dispatch_plans_with_newapi_fallback(config, newapi_stage_dispatch_plan_from_env)
}

fn dispatch_plans_with_newapi_fallback<F>(
    config: &Config,
    newapi_fallback: F,
) -> Result<Vec<PoolStageDispatchPlan>, String>
where
    F: Fn(&Config, &str) -> Option<PoolStageDispatchPlan>,
{
    let mut plans = Vec::new();
    let mut dependency_health = PoolDependencyHealthCache::default();
    for task_kind in task_kinds(config) {
        let is_primary = is_primary_route_task_kind(config, &task_kind);
        let path = route_path(config, &task_kind);
        if !path.exists() {
            if let Some(plan) = newapi_fallback(config, &task_kind) {
                plans.push(plan);
            }
            continue;
        }
        let Some(route) = pool_artifacts::load_route(Some(&path))? else {
            if let Some(plan) = newapi_fallback(config, &task_kind) {
                plans.push(plan);
            }
            continue;
        };
        if route.route_allowed != Some(true)
            || route.quality_context_sufficient == Some(false)
            || route.selected_context_sufficient == Some(false)
        {
            if let Some(plan) = newapi_fallback(config, &task_kind) {
                plans.push(plan);
            }
            continue;
        }
        if dependency_health
            .failure_for_route(config, &route)
            .is_some()
        {
            if let Some(plan) = newapi_fallback(config, &task_kind) {
                plans.push(plan);
            }
            continue;
        }
        if is_primary {
            continue;
        }
        let Some(selected_role) = route
            .selected_role
            .as_deref()
            .map(str::trim)
            .filter(|role| !role.is_empty())
            .filter(|role| *role != "none")
        else {
            if let Some(plan) = newapi_fallback(config, &task_kind) {
                plans.push(plan);
            }
            continue;
        };
        let Some(worker) = pool_artifacts::selected_route_candidate(&route) else {
            if let Some(plan) = newapi_fallback(config, &task_kind) {
                plans.push(plan);
            }
            continue;
        };
        let effective_max_tokens = stage_effective_max_tokens(config, selected_role, worker);
        plans.push(PoolStageDispatchPlan {
            task_kind,
            selected_role: selected_role.to_owned(),
            selected_port: worker.port,
            selected_base_url: worker.base_url.clone(),
            context_window: worker.context_window,
            default_max_tokens: worker.default_max_tokens,
            runtime_backend: worker.runtime_backend.clone(),
            runtime_device: worker.runtime_device.clone(),
            runtime_accelerator: worker.runtime_accelerator.clone(),
            gpu_layers: worker.gpu_layers,
            configured_max_tokens: config.max_tokens,
            effective_max_tokens,
            max_tokens_clamped: effective_max_tokens < config.max_tokens,
            can_accept_low_priority_task: worker.can_accept_low_priority_task,
        });
    }
    Ok(plans)
}

fn newapi_stage_dispatch_plan_from_env(
    config: &Config,
    task_kind: &str,
) -> Option<PoolStageDispatchPlan> {
    let base_url = env_value(["NORION_NEWAPI_BASE_URL", "NORION_MODEL_POOL_ENDPOINT"])?;
    let api_key_present =
        env_value(["NORION_NEWAPI_API_KEY", "NORION_MODEL_POOL_API_KEY"]).is_some();
    let models = env_value(["NORION_NEWAPI_ALLOWED_MODELS", "NORION_MODEL_POOL_MODELS"])?;
    newapi_stage_dispatch_plan(config, task_kind, &base_url, &models, api_key_present)
}

fn newapi_stage_dispatch_plan(
    config: &Config,
    task_kind: &str,
    base_url: &str,
    models: &str,
    api_key_present: bool,
) -> Option<PoolStageDispatchPlan> {
    let role = task_kind.trim();
    if role.is_empty()
        || base_url.trim().is_empty()
        || !api_key_present
        || !models
            .split([',', ';', '\n', '\r'])
            .any(|model| !model.trim().is_empty())
    {
        return None;
    }
    let default_max_tokens = newapi_stage_default_max_tokens(role);
    let effective_max_tokens = config.max_tokens.min(default_max_tokens);
    Some(PoolStageDispatchPlan {
        task_kind: role.to_owned(),
        selected_role: role.to_owned(),
        selected_port: None,
        selected_base_url: Some(base_url.trim().to_owned()),
        context_window: None,
        default_max_tokens: Some(default_max_tokens as u64),
        runtime_backend: Some("newapi".to_owned()),
        runtime_device: Some("remote".to_owned()),
        runtime_accelerator: Some("provider".to_owned()),
        gpu_layers: None,
        configured_max_tokens: config.max_tokens,
        effective_max_tokens,
        max_tokens_clamped: effective_max_tokens < config.max_tokens,
        can_accept_low_priority_task: true,
    })
}

fn newapi_stage_default_max_tokens(role: &str) -> usize {
    match role.trim().to_ascii_lowercase().as_str() {
        "summary" => 768,
        "router" | "index" => 512,
        "review" => 1024,
        "test-gate" => TEST_GATE_STAGE_MAX_TOKENS,
        _ => 768,
    }
}

fn env_value<const N: usize>(names: [&str; N]) -> Option<String> {
    names.into_iter().find_map(|name| {
        env::var(name)
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
    })
}

pub(crate) fn gate_failures(config: &Config) -> Vec<String> {
    let mut failures = Vec::new();
    let stage_kinds = task_kinds(config);
    let mut dependency_health = PoolDependencyHealthCache::default();
    if stage_kinds.is_empty() {
        failures.push("--pool-stage-route-gate requires --pool-stage-route-task-kinds".to_owned());
        return failures;
    }
    for task_kind in stage_kinds {
        if is_primary_route_task_kind(config, &task_kind) {
            continue;
        }
        let path = route_path(config, &task_kind);
        let route = match pool_artifacts::load_route(Some(&path)) {
            Ok(Some(route)) => route,
            Ok(None) => {
                failures.push(format!(
                    "stage route {task_kind} artifact is empty ({})",
                    path.display()
                ));
                continue;
            }
            Err(error) => {
                failures.push(format!(
                    "stage route {task_kind} artifact unreadable: {error}"
                ));
                continue;
            }
        };
        let evidence = pool_artifacts::route_context_text(&route);
        if route.route_allowed != Some(true) {
            failures.push(format!(
                "stage route {task_kind} is not allowed: {evidence}"
            ));
            continue;
        }
        if route.quality_context_sufficient == Some(false) {
            failures.push(format!(
                "stage route {task_kind} quality context insufficient: {evidence}"
            ));
            continue;
        }
        if route.selected_context_sufficient == Some(false) {
            failures.push(format!(
                "stage route {task_kind} selected context insufficient: {evidence}"
            ));
            continue;
        }
        if let Some(failure) = dependency_health.failure_for_route(config, &route) {
            failures.push(format!(
                "stage route {task_kind} dependency health failed: {failure}"
            ));
            continue;
        }
        let selected_role = route
            .selected_role
            .as_deref()
            .map(str::trim)
            .filter(|role| !role.is_empty())
            .filter(|role| *role != "none");
        if selected_role.is_none() {
            failures.push(format!(
                "stage route {task_kind} route_allowed=true but selected_role is missing: {evidence}"
            ));
            continue;
        }
        if route.ready_candidates == 0 {
            failures.push(format!(
                "stage route {task_kind} route_allowed=true but no candidate is ready: {evidence}"
            ));
            continue;
        }
        if pool_artifacts::selected_route_candidate(&route).is_none() {
            failures.push(format!(
                "stage route {task_kind} selected role has no ready candidate worker: {evidence}"
            ));
        }
    }
    failures
}

#[derive(Default)]
struct PoolDependencyHealthCache {
    loaded: bool,
    status: Option<pool_artifacts::PoolStatusSummary>,
    load_error: Option<String>,
}

impl PoolDependencyHealthCache {
    fn failure_for_route(
        &mut self,
        config: &Config,
        route: &pool_artifacts::PoolRouteSummary,
    ) -> Option<String> {
        if !pool_artifacts::route_requires_dependency_health_check(route) {
            return None;
        }
        self.load_status(config);
        if let Some(error) = self.load_error.as_deref() {
            return Some(format!("dependency health status unreadable: {error}"));
        }
        pool_artifacts::route_dependency_health_failure(route, self.status.as_ref())
    }

    fn load_status(&mut self, config: &Config) {
        if self.loaded {
            return;
        }
        self.loaded = true;
        match pool_artifacts::load_status(config.pool_status_json_path.as_deref()) {
            Ok(status) => self.status = status,
            Err(error) => self.load_error = Some(error),
        }
    }
}

fn stage_effective_max_tokens(
    config: &Config,
    selected_role: &str,
    worker: &pool_artifacts::PoolRouteCandidate,
) -> usize {
    let worker_default = worker
        .default_max_tokens
        .and_then(|value| usize::try_from(value).ok())
        .filter(|value| *value > 0);
    let helper_default_limit = worker.can_accept_low_priority_task
        && !selected_role.trim().eq_ignore_ascii_case("quality");
    if helper_default_limit {
        let helper_limit = worker_default
            .map(|value| config.max_tokens.min(value))
            .unwrap_or(config.max_tokens);
        if selected_role.trim().eq_ignore_ascii_case("test-gate") {
            helper_limit.min(TEST_GATE_STAGE_MAX_TOKENS)
        } else {
            helper_limit
        }
    } else {
        config.max_tokens
    }
}

fn option_u64_json(value: Option<u64>) -> String {
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
        .unwrap_or_else(|| "none".to_owned())
}

fn sanitize_route_task_kind(task_kind: &str) -> String {
    task_kind
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '-'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn derives_stage_route_paths_next_to_primary_route() {
        let config = Config {
            pool_route_json_path: Some(PathBuf::from("target/evolution/pool-route-review.json")),
            pool_route_task_kind: "review".to_owned(),
            ..Config::default()
        };

        assert_eq!(
            route_path(&config, "summary"),
            PathBuf::from("target/evolution/pool-route-summary.json")
        );
        assert_eq!(
            route_path(&config, "review"),
            PathBuf::from("target/evolution/pool-route-review.json")
        );
    }

    #[test]
    fn newapi_stage_plan_falls_back_when_local_route_is_blocked() {
        let config = Config {
            max_tokens: 4096,
            ..Config::default()
        };

        let plan = newapi_stage_dispatch_plan(
            &config,
            "test-gate",
            "https://provider.example/v1",
            "meta/llama-3.1-8b-instruct",
            true,
        )
        .unwrap();

        assert_eq!(plan.task_kind, "test-gate");
        assert_eq!(plan.selected_role, "test-gate");
        assert_eq!(plan.selected_port, None);
        assert_eq!(
            plan.selected_base_url.as_deref(),
            Some("https://provider.example/v1")
        );
        assert_eq!(plan.runtime_backend.as_deref(), Some("newapi"));
        assert_eq!(plan.runtime_device.as_deref(), Some("remote"));
        assert_eq!(
            plan.default_max_tokens,
            Some(TEST_GATE_STAGE_MAX_TOKENS as u64)
        );
        assert_eq!(plan.effective_max_tokens, TEST_GATE_STAGE_MAX_TOKENS);
        assert!(plan.max_tokens_clamped);
    }

    #[test]
    fn newapi_stage_plan_requires_key_and_models() {
        let config = Config::default();

        assert!(
            newapi_stage_dispatch_plan(&config, "summary", "https://provider.example/v1", "", true)
                .is_none()
        );
        assert!(
            newapi_stage_dispatch_plan(
                &config,
                "summary",
                "https://provider.example/v1",
                "meta/llama-3.1-8b-instruct",
                false,
            )
            .is_none()
        );
    }

    #[test]
    fn newapi_stage_plan_falls_back_for_blocked_primary_route() {
        let dir = std::env::temp_dir().join(format!(
            "smartsteam-primary-newapi-fallback-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let primary = dir.join("pool-route-review.json");
        fs::write(
            &primary,
            "{\"task_kind\":\"review\",\"route_allowed\":false,\"route_block_reason\":\"worker_down\",\"selected_role\":null,\"candidate_workers\":[]}\n",
        )
        .unwrap();
        let config = Config {
            pool_route_json_path: Some(primary),
            pool_route_task_kind: "review".to_owned(),
            pool_stage_route_task_kinds: vec!["review".to_owned()],
            ..Config::default()
        };

        let plans = dispatch_plans_with_newapi_fallback(&config, |config, task_kind| {
            newapi_stage_dispatch_plan(
                config,
                task_kind,
                "https://provider.example/v1",
                "meta/llama-3.1-8b-instruct",
                true,
            )
        })
        .unwrap();

        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].task_kind, "review");
        assert_eq!(plans[0].runtime_backend.as_deref(), Some("newapi"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn gate_blocks_unready_stage_route() {
        let dir = std::env::temp_dir().join(format!(
            "smartsteam-stage-route-gate-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let primary = dir.join("pool-route-review.json");
        let summary = dir.join("pool-route-summary.json");
        let test_gate = dir.join("pool-route-test-gate.json");
        fs::write(
            &primary,
            "{\"task_kind\":\"review\",\"route_allowed\":true,\"selected_role\":\"review\",\"role_candidates\":[\"review\"],\"candidate_workers\":[{\"role\":\"review\",\"health_ok\":true,\"role_ready\":true}]}\n",
        )
        .unwrap();
        fs::write(
            &summary,
            "{\"task_kind\":\"summary\",\"route_allowed\":true,\"selected_role\":\"summary\",\"role_candidates\":[\"summary\"],\"candidate_workers\":[{\"role\":\"summary\",\"health_ok\":true,\"role_ready\":true}]}\n",
        )
        .unwrap();
        fs::write(
            &test_gate,
            "{\"task_kind\":\"test-gate\",\"route_allowed\":false,\"route_block_reason\":\"worker_down\",\"selected_role\":null,\"role_candidates\":[\"test-gate\"],\"candidate_workers\":[{\"role\":\"test-gate\",\"health_ok\":false,\"role_ready\":false}]}\n",
        )
        .unwrap();
        let config = Config {
            pool_route_json_path: Some(primary),
            pool_route_task_kind: "review".to_owned(),
            pool_stage_route_task_kinds: vec![
                "summary".to_owned(),
                "review".to_owned(),
                "test-gate".to_owned(),
            ],
            ..Config::default()
        };

        let failures = gate_failures(&config);
        let evidence = allocation_evidence(&config).unwrap();
        let plans = dispatch_plans(&config).unwrap();

        assert_eq!(failures.len(), 1);
        assert!(failures[0].contains("stage route test-gate is not allowed"));
        assert!(
            evidence
                .iter()
                .any(|item| item.contains("pool_stage_route[summary]"))
        );
        assert!(
            evidence
                .iter()
                .any(|item| item.contains("pool_stage_route[test-gate]"))
        );
        assert!(
            evidence
                .iter()
                .all(|item| !item.contains("pool_stage_route[review]"))
        );
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].task_kind, "summary");
        assert_eq!(plans[0].selected_role, "summary");
        assert!(request_json_field(&plans).contains("\"pool_stage_dispatch\""));
        assert!(request_json_field(&plans).contains("\"task_kind\":\"summary\""));
        assert!(
            plans[0]
                .meta()
                .contains("pool_stage_dispatch task_kind=summary")
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn stage_route_path_avoids_primary_route_file_collision() {
        let dir = std::env::temp_dir().join(format!(
            "smartsteam-stage-route-path-collision-{}",
            std::process::id()
        ));
        let primary = dir.join("pool-route-review.json");
        let config = Config {
            pool_route_json_path: Some(primary.clone()),
            pool_route_task_kind: "quality".to_owned(),
            pool_stage_route_task_kinds: vec!["review".to_owned()],
            ..Config::default()
        };

        assert_eq!(
            route_path(&config, "review"),
            dir.join("pool-stage-route-review.json")
        );
        assert_ne!(route_path(&config, "review"), primary);
    }

    #[test]
    fn gate_blocks_stage_route_when_selected_context_is_insufficient() {
        let dir = std::env::temp_dir().join(format!(
            "smartsteam-stage-selected-context-gate-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let primary = dir.join("pool-route-review.json");
        let test_gate = dir.join("pool-route-test-gate.json");
        fs::write(
            &primary,
            "{\"task_kind\":\"review\",\"route_allowed\":true,\"selected_role\":\"review\",\"role_candidates\":[\"review\"],\"candidate_workers\":[{\"role\":\"review\",\"health_ok\":true,\"role_ready\":true}]}\n",
        )
        .unwrap();
        fs::write(
            &test_gate,
            "{\"task_kind\":\"test-gate\",\"route_allowed\":true,\"selected_context_required_tokens\":4572,\"selected_context_buffer_tokens\":2048,\"selected_context_sufficient\":false,\"selected_context_block_reason\":\"selected_context_window_too_small\",\"selected_role\":\"test-gate\",\"role_candidates\":[\"test-gate\"],\"candidate_workers\":[{\"role\":\"test-gate\",\"health_ok\":true,\"role_ready\":true,\"context_window\":4096,\"default_max_tokens\":1024}]}\n",
        )
        .unwrap();
        let config = Config {
            pool_route_json_path: Some(primary),
            pool_route_task_kind: "review".to_owned(),
            pool_stage_route_task_kinds: vec!["test-gate".to_owned()],
            ..Config::default()
        };

        let failures = gate_failures(&config);
        let plans = dispatch_plans(&config).unwrap();

        assert_eq!(plans.len(), 0);
        assert_eq!(failures.len(), 1);
        assert!(failures[0].contains("stage route test-gate selected context insufficient"));
        assert!(failures[0].contains("selected_context_required_tokens:4572"));
        assert!(failures[0].contains("selected_context_buffer_tokens:2048"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn gate_blocks_stage_route_when_dependency_role_is_not_healthy() {
        let dir = std::env::temp_dir().join(format!(
            "smartsteam-stage-dependency-health-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let primary = dir.join("pool-route-review.json");
        let index = dir.join("pool-route-index.json");
        let status = dir.join("pool-status.json");
        fs::write(
            &primary,
            "{\"task_kind\":\"review\",\"route_allowed\":true,\"selected_role\":\"review\",\"candidate_workers\":[{\"role\":\"review\",\"health_ok\":true,\"role_ready\":true}]}\n",
        )
        .unwrap();
        fs::write(
            &index,
            "{\"task_kind\":\"index\",\"route_allowed\":true,\"dependency_precheck\":{\"strategy\":\"role_dependency_graph_v1\",\"checked\":true,\"requested_role\":\"index\",\"allow_dispatch\":true,\"reason\":\"dependencies_satisfied\",\"required_roles\":[\"summary\",\"router\"],\"completed_roles\":[\"quality\",\"summary\",\"router\"],\"missing_roles\":[]},\"selected_role\":\"index\",\"candidate_workers\":[{\"role\":\"index\",\"health_ok\":true,\"role_ready\":true}]}\n",
        )
        .unwrap();
        fs::write(
            &status,
            "{\"workers\":[{\"role\":\"summary\",\"tcp_reachable\":true,\"health_ok\":false},{\"role\":\"index\",\"tcp_reachable\":true,\"health_ok\":true}]}\n",
        )
        .unwrap();
        let config = Config {
            pool_route_json_path: Some(primary),
            pool_status_json_path: Some(status),
            pool_route_task_kind: "review".to_owned(),
            pool_stage_route_task_kinds: vec!["index".to_owned()],
            ..Config::default()
        };

        let failures = gate_failures(&config);
        let plans = dispatch_plans(&config).unwrap();

        assert_eq!(plans.len(), 0);
        assert_eq!(failures.len(), 1);
        assert!(failures[0].contains("stage route index dependency health failed"));
        assert!(failures[0].contains("missing_roles=router"));
        assert!(failures[0].contains("unhealthy_roles=summary:tcp_only"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn gate_accepts_stage_route_when_dependency_roles_are_healthy() {
        let dir = std::env::temp_dir().join(format!(
            "smartsteam-stage-dependency-health-ok-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let primary = dir.join("pool-route-review.json");
        let index = dir.join("pool-route-index.json");
        let status = dir.join("pool-status.json");
        fs::write(
            &primary,
            "{\"task_kind\":\"review\",\"route_allowed\":true,\"selected_role\":\"review\",\"candidate_workers\":[{\"role\":\"review\",\"health_ok\":true,\"role_ready\":true}]}\n",
        )
        .unwrap();
        fs::write(
            &index,
            "{\"task_kind\":\"index\",\"route_allowed\":true,\"dependency_precheck\":{\"strategy\":\"role_dependency_graph_v1\",\"checked\":true,\"requested_role\":\"index\",\"allow_dispatch\":true,\"reason\":\"dependencies_satisfied\",\"required_roles\":[\"summary\",\"router\"],\"completed_roles\":[\"quality\",\"summary\",\"router\"],\"missing_roles\":[]},\"selected_role\":\"index\",\"candidate_workers\":[{\"role\":\"index\",\"health_ok\":true,\"role_ready\":true,\"can_accept_low_priority_task\":true,\"default_max_tokens\":512}]}\n",
        )
        .unwrap();
        fs::write(
            &status,
            "{\"workers\":[{\"role\":\"summary\",\"tcp_reachable\":true,\"health_ok\":true},{\"role\":\"router\",\"tcp_reachable\":true,\"health_ok\":true},{\"role\":\"index\",\"tcp_reachable\":true,\"health_ok\":true}]}\n",
        )
        .unwrap();
        let config = Config {
            pool_route_json_path: Some(primary),
            pool_status_json_path: Some(status),
            pool_route_task_kind: "review".to_owned(),
            pool_stage_route_task_kinds: vec!["index".to_owned()],
            ..Config::default()
        };

        let failures = gate_failures(&config);
        let plans = dispatch_plans(&config).unwrap();

        assert!(failures.is_empty(), "{failures:?}");
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].task_kind, "index");
        assert_eq!(plans[0].effective_max_tokens, 512);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn stage_effective_max_tokens_preserves_quality_budget() {
        let config = Config {
            max_tokens: 262_144,
            ..Config::default()
        };
        let worker = pool_artifacts::PoolRouteCandidate {
            port: Some(8686),
            role: "quality".to_owned(),
            base_url: Some("http://127.0.0.1:8686".to_owned()),
            tcp_reachable: true,
            health_ok: true,
            status: None,
            role_ready: true,
            role_block_reason: None,
            can_accept_low_priority_task: false,
            model: None,
            context_window: Some(262_144),
            runtime_backend: Some("llama.cpp".to_owned()),
            runtime_device: Some("metal".to_owned()),
            runtime_accelerator: Some("metal".to_owned()),
            gpu_layers: Some(99),
            default_max_tokens: Some(262_144),
        };

        assert_eq!(
            stage_effective_max_tokens(&config, "quality", &worker),
            262_144
        );
    }

    #[test]
    fn stage_effective_max_tokens_clamps_low_priority_helper() {
        let config = Config {
            max_tokens: 262_144,
            ..Config::default()
        };
        let worker = pool_artifacts::PoolRouteCandidate {
            port: Some(8687),
            role: "summary".to_owned(),
            base_url: Some("http://127.0.0.1:8687".to_owned()),
            tcp_reachable: true,
            health_ok: true,
            status: None,
            role_ready: true,
            role_block_reason: None,
            can_accept_low_priority_task: true,
            model: None,
            context_window: Some(8192),
            runtime_backend: Some("llama.cpp".to_owned()),
            runtime_device: Some("metal".to_owned()),
            runtime_accelerator: Some("metal".to_owned()),
            gpu_layers: Some(80),
            default_max_tokens: Some(768),
        };

        assert_eq!(stage_effective_max_tokens(&config, "summary", &worker), 768);
    }

    #[test]
    fn stage_effective_max_tokens_keeps_test_gate_small() {
        let config = Config {
            max_tokens: 1024,
            ..Config::default()
        };
        let worker = pool_artifacts::PoolRouteCandidate {
            port: Some(8688),
            role: "test-gate".to_owned(),
            base_url: Some("http://127.0.0.1:8688".to_owned()),
            tcp_reachable: true,
            health_ok: true,
            status: None,
            role_ready: true,
            role_block_reason: None,
            can_accept_low_priority_task: true,
            model: None,
            context_window: Some(4096),
            runtime_backend: Some("llama.cpp".to_owned()),
            runtime_device: Some("metal".to_owned()),
            runtime_accelerator: Some("metal".to_owned()),
            gpu_layers: Some(999),
            default_max_tokens: Some(1536),
        };

        assert_eq!(
            stage_effective_max_tokens(&config, "test-gate", &worker),
            TEST_GATE_STAGE_MAX_TOKENS
        );
    }
}
