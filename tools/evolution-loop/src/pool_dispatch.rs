use crate::args::Config;
use crate::pool_artifacts;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PoolDispatchDecision {
    pub(crate) selected_role: String,
    pub(crate) selected_port: Option<u64>,
    pub(crate) selected_base_url: Option<String>,
    pub(crate) context_window: Option<u64>,
    pub(crate) default_max_tokens: Option<u64>,
    pub(crate) runtime_backend: Option<String>,
    pub(crate) runtime_device: Option<String>,
    pub(crate) runtime_accelerator: Option<String>,
    pub(crate) gpu_layers: Option<u64>,
    pub(crate) can_accept_low_priority_task: bool,
    pub(crate) evidence: String,
}

pub(crate) fn preflight(config: &Config) -> Result<Option<PoolDispatchDecision>, String> {
    if !config.require_pool_route {
        return Ok(None);
    }
    let Some(path) = config.pool_route_json_path.as_deref() else {
        return Err("--require-pool-route requires --pool-route-json PATH".to_owned());
    };
    let Some(route) = pool_artifacts::load_route(Some(path))? else {
        return Err(format!(
            "pool route gate failed: {} is empty",
            path.display()
        ));
    };
    let evidence = pool_artifacts::route_context_text(&route);
    if route.route_allowed != Some(true) {
        return Err(format!("pool route gate failed: {evidence}"));
    }
    if route.quality_context_sufficient == Some(false) {
        return Err(format!(
            "pool route gate failed: quality context insufficient: {evidence}"
        ));
    }
    if route.selected_context_sufficient == Some(false) {
        return Err(format!(
            "pool route gate failed: selected context insufficient: {evidence}"
        ));
    }
    let Some(selected_role) = route
        .selected_role
        .as_deref()
        .map(str::trim)
        .filter(|role| !role.is_empty())
        .filter(|role| *role != "none")
    else {
        return Err(format!(
            "pool route gate failed: route_allowed=true but selected_role is missing: {evidence}"
        ));
    };
    if route.ready_candidates == 0 {
        return Err(format!(
            "pool route gate failed: route_allowed=true but no candidate is ready: {evidence}"
        ));
    }
    let Some(selected_worker) = pool_artifacts::selected_route_candidate(&route) else {
        return Err(format!(
            "pool route gate failed: selected role {selected_role} has no ready candidate worker: {evidence}"
        ));
    };
    Ok(Some(PoolDispatchDecision {
        selected_role: selected_role.to_owned(),
        selected_port: selected_worker.port,
        selected_base_url: selected_worker.base_url.clone(),
        context_window: selected_worker.context_window,
        default_max_tokens: selected_worker.default_max_tokens,
        runtime_backend: selected_worker.runtime_backend.clone(),
        runtime_device: selected_worker.runtime_device.clone(),
        runtime_accelerator: selected_worker.runtime_accelerator.clone(),
        gpu_layers: selected_worker.gpu_layers,
        can_accept_low_priority_task: selected_worker.can_accept_low_priority_task,
        evidence,
    }))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::*;

    fn unique_temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "smartsteam-pool-dispatch-{name}-{}.json",
            std::process::id()
        ))
    }

    #[test]
    fn disabled_preflight_does_not_require_route_artifact() {
        let decision = preflight(&Config::default()).unwrap();

        assert!(decision.is_none());
    }

    #[test]
    fn required_preflight_needs_route_artifact_path() {
        let error = preflight(&Config {
            require_pool_route: true,
            ..Config::default()
        })
        .unwrap_err();

        assert!(error.contains("--require-pool-route requires --pool-route-json"));
    }

    #[test]
    fn required_preflight_blocks_disallowed_route() {
        let path = unique_temp_path("blocked");
        let _ = fs::remove_file(&path);
        fs::write(
            &path,
            "{\"task_kind\":\"review\",\"route_allowed\":false,\"route_block_reason\":\"model_pool_launch_blocked:quality_worker_down\",\"selected_role\":null,\"role_candidates\":[\"review\",\"quality\"],\"candidate_workers\":[{\"role\":\"quality\",\"health_ok\":false,\"role_ready\":false},{\"role\":\"review\",\"health_ok\":false,\"role_ready\":false}]}\n",
        )
        .unwrap();
        let error = preflight(&Config {
            require_pool_route: true,
            pool_route_json_path: Some(path.clone()),
            ..Config::default()
        })
        .unwrap_err();

        assert!(error.contains("pool route gate failed"));
        assert!(error.contains("route_allowed:false"));
        assert!(error.contains("quality_worker_down"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn required_preflight_accepts_ready_selected_route() {
        let path = unique_temp_path("ready");
        let _ = fs::remove_file(&path);
        fs::write(
            &path,
            "{\"task_kind\":\"summary\",\"route_allowed\":true,\"route_block_reason\":null,\"selected_role\":\"summary\",\"role_candidates\":[\"summary\",\"quality\"],\"candidate_workers\":[{\"port\":8687,\"role\":\"summary\",\"base_url\":\"http://127.0.0.1:8687\",\"health_ok\":true,\"role_ready\":true,\"can_accept_low_priority_task\":true,\"context_window\":8192,\"runtime_backend\":\"llama.cpp\",\"runtime_device\":\"metal\",\"runtime_accelerator\":\"metal\",\"gpu_layers\":99,\"default_max_tokens\":768}]}\n",
        )
        .unwrap();

        let decision = preflight(&Config {
            require_pool_route: true,
            pool_route_json_path: Some(path.clone()),
            ..Config::default()
        })
        .unwrap()
        .unwrap();

        assert_eq!(decision.selected_role, "summary");
        assert_eq!(decision.selected_port, Some(8687));
        assert_eq!(
            decision.selected_base_url.as_deref(),
            Some("http://127.0.0.1:8687")
        );
        assert_eq!(decision.context_window, Some(8192));
        assert_eq!(decision.default_max_tokens, Some(768));
        assert_eq!(decision.runtime_backend.as_deref(), Some("llama.cpp"));
        assert_eq!(decision.runtime_device.as_deref(), Some("metal"));
        assert_eq!(decision.runtime_accelerator.as_deref(), Some("metal"));
        assert_eq!(decision.gpu_layers, Some(99));
        assert!(decision.can_accept_low_priority_task);
        assert!(decision.evidence.contains("route_allowed:true"));
        assert!(decision.evidence.contains("selected_role:summary"));
        assert!(
            decision
                .evidence
                .contains("selected_endpoint:http://127.0.0.1:8687")
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn required_preflight_rejects_context_insufficient_route_artifact() {
        let path = unique_temp_path("context-insufficient");
        let _ = fs::remove_file(&path);
        fs::write(
            &path,
            "{\"task_kind\":\"quality\",\"route_allowed\":true,\"route_block_reason\":null,\"quality_context_tokens\":8192,\"quality_context_required_tokens\":262144,\"quality_context_sufficient\":false,\"quality_block_reason\":\"context_window_below_quality_default\",\"selected_role\":\"quality\",\"role_candidates\":[\"quality\"],\"candidate_workers\":[{\"port\":8686,\"role\":\"quality\",\"base_url\":\"http://127.0.0.1:8686\",\"health_ok\":true,\"role_ready\":true,\"context_window\":8192,\"default_max_tokens\":262144}]}\n",
        )
        .unwrap();

        let error = preflight(&Config {
            require_pool_route: true,
            pool_route_json_path: Some(path.clone()),
            ..Config::default()
        })
        .unwrap_err();

        assert!(error.contains("quality context insufficient"));
        assert!(error.contains("quality_context_tokens:8192"));
        assert!(error.contains("quality_context_required_tokens:262144"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn required_preflight_rejects_selected_context_insufficient_route_artifact() {
        let path = unique_temp_path("selected-context-insufficient");
        let _ = fs::remove_file(&path);
        fs::write(
            &path,
            "{\"task_kind\":\"test-gate\",\"route_allowed\":true,\"route_block_reason\":null,\"selected_context_required_tokens\":4572,\"selected_context_buffer_tokens\":2048,\"selected_context_sufficient\":false,\"selected_context_block_reason\":\"selected_context_window_too_small\",\"selected_role\":\"test-gate\",\"role_candidates\":[\"test-gate\"],\"candidate_workers\":[{\"port\":8688,\"role\":\"test-gate\",\"base_url\":\"http://127.0.0.1:8688\",\"health_ok\":true,\"role_ready\":true,\"context_window\":4096,\"default_max_tokens\":1024}]}\n",
        )
        .unwrap();

        let error = preflight(&Config {
            require_pool_route: true,
            pool_route_json_path: Some(path.clone()),
            ..Config::default()
        })
        .unwrap_err();

        assert!(error.contains("selected context insufficient"));
        assert!(error.contains("selected_context_required_tokens:4572"));
        assert!(error.contains("selected_context_buffer_tokens:2048"));
        assert!(error.contains("selected_context_sufficient:false"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn required_preflight_blocks_selected_role_without_ready_worker() {
        let path = unique_temp_path("selected-not-ready");
        let _ = fs::remove_file(&path);
        fs::write(
            &path,
            "{\"task_kind\":\"review\",\"route_allowed\":true,\"route_block_reason\":null,\"selected_role\":\"review\",\"role_candidates\":[\"review\",\"quality\"],\"candidate_workers\":[{\"role\":\"review\",\"health_ok\":false,\"role_ready\":false},{\"role\":\"quality\",\"health_ok\":true,\"role_ready\":true}]}\n",
        )
        .unwrap();

        let error = preflight(&Config {
            require_pool_route: true,
            pool_route_json_path: Some(path.clone()),
            ..Config::default()
        })
        .unwrap_err();

        assert!(error.contains("selected role review has no ready candidate worker"));
        let _ = fs::remove_file(path);
    }
}
