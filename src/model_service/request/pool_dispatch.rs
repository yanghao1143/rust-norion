use super::super::json::{
    json_bool_field, json_object_array_field, json_object_field, json_string_field, json_u64_field,
    json_usize_field,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelServicePoolDispatchRequest {
    pub(crate) selected_role: String,
    pub(crate) selected_port: Option<u64>,
    pub(crate) selected_base_url: Option<String>,
    pub(crate) context_window: Option<u64>,
    pub(crate) default_max_tokens: Option<u64>,
    pub(crate) runtime_backend: Option<String>,
    pub(crate) runtime_device: Option<String>,
    pub(crate) runtime_accelerator: Option<String>,
    pub(crate) gpu_layers: Option<u64>,
    pub(crate) configured_max_tokens: Option<usize>,
    pub(crate) effective_max_tokens: Option<usize>,
    pub(crate) max_tokens_clamped: bool,
    pub(crate) max_tokens_clamp_reason: Option<String>,
    pub(crate) can_accept_low_priority_task: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelServicePoolStageDispatchRequest {
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
    pub(crate) configured_max_tokens: Option<usize>,
    pub(crate) effective_max_tokens: Option<usize>,
    pub(crate) max_tokens_clamped: bool,
    pub(crate) max_tokens_clamp_reason: Option<String>,
    pub(crate) can_accept_low_priority_task: bool,
}

impl ModelServicePoolDispatchRequest {
    pub(crate) fn summary(&self, worker_forwarded: bool) -> String {
        format!(
            "pool_dispatch selected_role={} port={} endpoint={} context_window={} default_max_tokens={} runtime_backend={} runtime_device={} runtime_accelerator={} gpu_layers={} configured_max_tokens={} effective_max_tokens={} max_tokens_clamped={} max_tokens_clamp_reason={} low_priority={} forwarded={} dispatch_mode={} dispatch_reason={}",
            self.selected_role,
            option_u64_text(self.selected_port),
            self.selected_base_url.as_deref().unwrap_or("none"),
            option_u64_text(self.context_window),
            option_u64_text(self.default_max_tokens),
            self.runtime_backend.as_deref().unwrap_or("none"),
            self.runtime_device.as_deref().unwrap_or("none"),
            self.runtime_accelerator.as_deref().unwrap_or("none"),
            option_u64_text(self.gpu_layers),
            option_usize_text(self.configured_max_tokens),
            option_usize_text(self.effective_max_tokens),
            self.max_tokens_clamped,
            self.max_tokens_clamp_reason.as_deref().unwrap_or("none"),
            self.can_accept_low_priority_task,
            worker_forwarded,
            Self::dispatch_mode(worker_forwarded),
            self.dispatch_reason(worker_forwarded)
        )
    }

    pub(crate) fn dispatch_mode(worker_forwarded: bool) -> &'static str {
        if worker_forwarded {
            "runtime_endpoint_override"
        } else {
            "backend_budget_only"
        }
    }

    pub(crate) fn dispatch_reason(&self, worker_forwarded: bool) -> &'static str {
        if worker_forwarded {
            "runtime_endpoint_override_active"
        } else if self.selected_base_url.is_some() {
            "runtime_endpoint_override_unavailable"
        } else {
            "selected_endpoint_missing"
        }
    }
}

impl ModelServicePoolStageDispatchRequest {
    pub(crate) fn summary(&self) -> String {
        format!(
            "pool_stage_dispatch task_kind={} selected_role={} port={} endpoint={} context_window={} default_max_tokens={} runtime_backend={} runtime_device={} runtime_accelerator={} gpu_layers={} configured_max_tokens={} effective_max_tokens={} max_tokens_clamped={} max_tokens_clamp_reason={} low_priority={} dispatch_mode={} dispatch_reason={}",
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
            option_usize_text(self.configured_max_tokens),
            option_usize_text(self.effective_max_tokens),
            self.max_tokens_clamped,
            self.max_tokens_clamp_reason.as_deref().unwrap_or("none"),
            self.can_accept_low_priority_task,
            Self::dispatch_mode(),
            Self::dispatch_reason()
        )
    }

    pub(crate) fn dispatch_mode() -> &'static str {
        "stage_plan_only"
    }

    pub(crate) fn dispatch_reason() -> &'static str {
        "stage_dispatch_observed"
    }
}

pub(super) fn parse_pool_dispatch_request(
    body: &str,
) -> Result<Option<ModelServicePoolDispatchRequest>, String> {
    let Some(pool_dispatch) = json_object_field(body, "pool_dispatch") else {
        return Ok(None);
    };
    let selected_role = json_string_field(&pool_dispatch, "selected_role")
        .map(|role| role.trim().to_owned())
        .filter(|role| !role.is_empty() && role != "none")
        .ok_or_else(|| "pool_dispatch requires a non-empty selected_role".to_owned())?;
    Ok(Some(ModelServicePoolDispatchRequest {
        selected_role,
        selected_port: json_u64_field(&pool_dispatch, "selected_port"),
        selected_base_url: json_string_field(&pool_dispatch, "selected_base_url")
            .filter(|value| !value.trim().is_empty() && value.trim() != "none"),
        context_window: json_u64_field(&pool_dispatch, "context_window"),
        default_max_tokens: json_u64_field(&pool_dispatch, "default_max_tokens"),
        runtime_backend: json_string_field(&pool_dispatch, "runtime_backend")
            .filter(|value| !value.trim().is_empty() && value.trim() != "none"),
        runtime_device: json_string_field(&pool_dispatch, "runtime_device")
            .filter(|value| !value.trim().is_empty() && value.trim() != "none"),
        runtime_accelerator: json_string_field(&pool_dispatch, "runtime_accelerator")
            .filter(|value| !value.trim().is_empty() && value.trim() != "none"),
        gpu_layers: json_u64_field(&pool_dispatch, "gpu_layers"),
        configured_max_tokens: json_usize_field(&pool_dispatch, "configured_max_tokens")
            .map(|value| value.max(1)),
        effective_max_tokens: json_usize_field(&pool_dispatch, "effective_max_tokens")
            .map(|value| value.max(1)),
        max_tokens_clamped: json_bool_field(&pool_dispatch, "max_tokens_clamped").unwrap_or(false),
        max_tokens_clamp_reason: json_string_field(&pool_dispatch, "max_tokens_clamp_reason")
            .or_else(|| json_string_field(&pool_dispatch, "clamp_reason"))
            .map(|reason| reason.trim().to_owned())
            .filter(|reason| !reason.is_empty()),
        can_accept_low_priority_task: json_bool_field(
            &pool_dispatch,
            "can_accept_low_priority_task",
        )
        .unwrap_or(false),
    }))
}

pub(super) fn parse_pool_stage_dispatch_requests(
    body: &str,
) -> Result<Vec<ModelServicePoolStageDispatchRequest>, String> {
    let Some(stage_dispatches) = json_object_array_field(body, "pool_stage_dispatch") else {
        return Ok(Vec::new());
    };
    stage_dispatches
        .iter()
        .map(|stage_dispatch| parse_pool_stage_dispatch_request(stage_dispatch))
        .collect()
}

fn parse_pool_stage_dispatch_request(
    stage_dispatch: &str,
) -> Result<ModelServicePoolStageDispatchRequest, String> {
    let task_kind = json_string_field(stage_dispatch, "task_kind")
        .map(|kind| kind.trim().to_owned())
        .filter(|kind| !kind.is_empty() && kind != "none")
        .ok_or_else(|| "pool_stage_dispatch requires a non-empty task_kind".to_owned())?;
    let selected_role = json_string_field(stage_dispatch, "selected_role")
        .map(|role| role.trim().to_owned())
        .filter(|role| !role.is_empty() && role != "none")
        .ok_or_else(|| "pool_stage_dispatch requires a non-empty selected_role".to_owned())?;
    Ok(ModelServicePoolStageDispatchRequest {
        task_kind,
        selected_role,
        selected_port: json_u64_field(stage_dispatch, "selected_port"),
        selected_base_url: json_string_field(stage_dispatch, "selected_base_url")
            .filter(|value| !value.trim().is_empty() && value.trim() != "none"),
        context_window: json_u64_field(stage_dispatch, "context_window"),
        default_max_tokens: json_u64_field(stage_dispatch, "default_max_tokens"),
        runtime_backend: json_string_field(stage_dispatch, "runtime_backend")
            .filter(|value| !value.trim().is_empty() && value.trim() != "none"),
        runtime_device: json_string_field(stage_dispatch, "runtime_device")
            .filter(|value| !value.trim().is_empty() && value.trim() != "none"),
        runtime_accelerator: json_string_field(stage_dispatch, "runtime_accelerator")
            .filter(|value| !value.trim().is_empty() && value.trim() != "none"),
        gpu_layers: json_u64_field(stage_dispatch, "gpu_layers"),
        configured_max_tokens: json_usize_field(stage_dispatch, "configured_max_tokens")
            .map(|value| value.max(1)),
        effective_max_tokens: json_usize_field(stage_dispatch, "effective_max_tokens")
            .map(|value| value.max(1)),
        max_tokens_clamped: json_bool_field(stage_dispatch, "max_tokens_clamped").unwrap_or(false),
        max_tokens_clamp_reason: json_string_field(stage_dispatch, "max_tokens_clamp_reason")
            .or_else(|| json_string_field(stage_dispatch, "clamp_reason"))
            .map(|reason| reason.trim().to_owned())
            .filter(|reason| !reason.is_empty()),
        can_accept_low_priority_task: json_bool_field(
            stage_dispatch,
            "can_accept_low_priority_task",
        )
        .unwrap_or(false),
    })
}

fn option_u64_text(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_owned())
}

fn option_usize_text(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_pool_dispatch_is_optional() {
        assert!(
            parse_pool_dispatch_request("{\"prompt\":\"hi\"}")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn parses_pool_dispatch_contract() {
        let request = parse_pool_dispatch_request(
            "{\"pool_dispatch\":{\"selected_role\":\"summary\",\"selected_port\":8687,\"selected_base_url\":\"http://127.0.0.1:8687\",\"context_window\":8192,\"default_max_tokens\":768,\"runtime_backend\":\"llama.cpp\",\"runtime_device\":\"metal\",\"runtime_accelerator\":\"metal\",\"gpu_layers\":99,\"configured_max_tokens\":4096,\"effective_max_tokens\":768,\"max_tokens_clamped\":true,\"max_tokens_clamp_reason\":\"low_priority_worker_default_max_tokens\",\"can_accept_low_priority_task\":true}}",
        )
        .unwrap()
        .unwrap();

        assert_eq!(request.selected_role, "summary");
        assert_eq!(request.selected_port, Some(8687));
        assert_eq!(
            request.selected_base_url.as_deref(),
            Some("http://127.0.0.1:8687")
        );
        assert_eq!(request.effective_max_tokens, Some(768));
        assert_eq!(request.runtime_backend.as_deref(), Some("llama.cpp"));
        assert_eq!(request.runtime_device.as_deref(), Some("metal"));
        assert_eq!(request.runtime_accelerator.as_deref(), Some("metal"));
        assert_eq!(request.gpu_layers, Some(99));
        assert!(request.max_tokens_clamped);
        assert_eq!(
            request.max_tokens_clamp_reason.as_deref(),
            Some("low_priority_worker_default_max_tokens")
        );
        assert!(request.can_accept_low_priority_task);
        assert!(request.summary(false).contains("forwarded=false"));
        assert!(request.summary(true).contains("forwarded=true"));
        assert!(
            request
                .summary(true)
                .contains("max_tokens_clamp_reason=low_priority_worker_default_max_tokens")
        );
        assert!(
            request
                .summary(false)
                .contains("dispatch_reason=runtime_endpoint_override_unavailable")
        );
        assert!(
            request
                .summary(true)
                .contains("dispatch_reason=runtime_endpoint_override_active")
        );
        assert!(request.summary(false).contains("runtime_device=metal"));
    }

    #[test]
    fn rejects_incomplete_pool_dispatch_contract() {
        let error = parse_pool_dispatch_request("{\"pool_dispatch\":{\"selected_role\":\"\"}}")
            .unwrap_err();

        assert!(error.contains("selected_role"));
    }

    #[test]
    fn parses_pool_stage_dispatch_contract() {
        let requests = parse_pool_stage_dispatch_requests(
            "{\"pool_stage_dispatch\":[{\"task_kind\":\"summary\",\"selected_role\":\"summary\",\"selected_port\":8687,\"selected_base_url\":\"http://127.0.0.1:8687\",\"context_window\":8192,\"default_max_tokens\":768,\"runtime_backend\":\"llama.cpp\",\"runtime_device\":\"metal\",\"runtime_accelerator\":\"metal\",\"gpu_layers\":99,\"configured_max_tokens\":4096,\"effective_max_tokens\":768,\"max_tokens_clamped\":true,\"max_tokens_clamp_reason\":\"low_priority_worker_default_max_tokens\",\"can_accept_low_priority_task\":true},{\"task_kind\":\"test-gate\",\"selected_role\":\"test-gate\"}]}",
        )
        .unwrap();

        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].task_kind, "summary");
        assert_eq!(requests[0].selected_role, "summary");
        assert_eq!(requests[0].selected_port, Some(8687));
        assert_eq!(requests[0].runtime_device.as_deref(), Some("metal"));
        assert_eq!(requests[0].effective_max_tokens, Some(768));
        assert!(requests[0].max_tokens_clamped);
        assert!(requests[0].can_accept_low_priority_task);
        assert!(
            requests[0]
                .summary()
                .contains("dispatch_mode=stage_plan_only")
        );
        assert_eq!(requests[1].task_kind, "test-gate");
    }

    #[test]
    fn rejects_incomplete_pool_stage_dispatch_contract() {
        let error = parse_pool_stage_dispatch_requests(
            "{\"pool_stage_dispatch\":[{\"task_kind\":\"summary\",\"selected_role\":\"\"}]}",
        )
        .unwrap_err();

        assert!(error.contains("selected_role"));
    }
}
