use super::super::super::json::{
    option_str_service_json, option_u64_service_json, service_json_string,
};
use super::super::super::request::{
    ModelServicePoolDispatchRequest, ModelServicePoolStageDispatchRequest,
};

pub(super) fn option_pool_dispatch_service_json(
    request: Option<&ModelServicePoolDispatchRequest>,
    worker_forwarded: bool,
) -> String {
    request
        .map(|request| pool_dispatch_service_json(request, worker_forwarded))
        .unwrap_or_else(|| "null".to_owned())
}

pub(super) fn pool_stage_dispatch_service_json(
    requests: &[ModelServicePoolStageDispatchRequest],
) -> String {
    let items = requests
        .iter()
        .map(pool_stage_dispatch_item_service_json)
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

fn pool_dispatch_service_json(
    request: &ModelServicePoolDispatchRequest,
    worker_forwarded: bool,
) -> String {
    format!(
        "{{\"selected_role\":{},\"selected_port\":{},\"selected_base_url\":{},\"context_window\":{},\"default_max_tokens\":{},\"runtime_backend\":{},\"runtime_device\":{},\"runtime_accelerator\":{},\"gpu_layers\":{},\"configured_max_tokens\":{},\"effective_max_tokens\":{},\"max_tokens_clamped\":{},\"max_tokens_clamp_reason\":{},\"can_accept_low_priority_task\":{},\"worker_forwarded\":{},\"dispatch_mode\":{},\"dispatch_reason\":{}}}",
        service_json_string(&request.selected_role),
        option_u64_service_json(request.selected_port),
        option_str_service_json(request.selected_base_url.as_deref()),
        option_u64_service_json(request.context_window),
        option_u64_service_json(request.default_max_tokens),
        option_str_service_json(request.runtime_backend.as_deref()),
        option_str_service_json(request.runtime_device.as_deref()),
        option_str_service_json(request.runtime_accelerator.as_deref()),
        option_u64_service_json(request.gpu_layers),
        option_usize_service_json(request.configured_max_tokens),
        option_usize_service_json(request.effective_max_tokens),
        request.max_tokens_clamped,
        option_str_service_json(request.max_tokens_clamp_reason.as_deref()),
        request.can_accept_low_priority_task,
        worker_forwarded,
        service_json_string(ModelServicePoolDispatchRequest::dispatch_mode(
            worker_forwarded
        )),
        service_json_string(request.dispatch_reason(worker_forwarded))
    )
}

fn pool_stage_dispatch_item_service_json(request: &ModelServicePoolStageDispatchRequest) -> String {
    format!(
        "{{\"task_kind\":{},\"selected_role\":{},\"selected_port\":{},\"selected_base_url\":{},\"context_window\":{},\"default_max_tokens\":{},\"runtime_backend\":{},\"runtime_device\":{},\"runtime_accelerator\":{},\"gpu_layers\":{},\"configured_max_tokens\":{},\"effective_max_tokens\":{},\"max_tokens_clamped\":{},\"max_tokens_clamp_reason\":{},\"can_accept_low_priority_task\":{},\"dispatch_mode\":{},\"dispatch_reason\":{}}}",
        service_json_string(&request.task_kind),
        service_json_string(&request.selected_role),
        option_u64_service_json(request.selected_port),
        option_str_service_json(request.selected_base_url.as_deref()),
        option_u64_service_json(request.context_window),
        option_u64_service_json(request.default_max_tokens),
        option_str_service_json(request.runtime_backend.as_deref()),
        option_str_service_json(request.runtime_device.as_deref()),
        option_str_service_json(request.runtime_accelerator.as_deref()),
        option_u64_service_json(request.gpu_layers),
        option_usize_service_json(request.configured_max_tokens),
        option_usize_service_json(request.effective_max_tokens),
        request.max_tokens_clamped,
        option_str_service_json(request.max_tokens_clamp_reason.as_deref()),
        request.can_accept_low_priority_task,
        service_json_string(ModelServicePoolStageDispatchRequest::dispatch_mode()),
        service_json_string(ModelServicePoolStageDispatchRequest::dispatch_reason())
    )
}

fn option_usize_service_json(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_pool_dispatch_as_backend_budget_only() {
        let json = pool_dispatch_service_json(
            &ModelServicePoolDispatchRequest {
                selected_role: "summary".to_owned(),
                selected_port: Some(8687),
                selected_base_url: Some("http://127.0.0.1:8687".to_owned()),
                context_window: Some(8192),
                default_max_tokens: Some(768),
                runtime_backend: Some("llama.cpp".to_owned()),
                runtime_device: Some("metal".to_owned()),
                runtime_accelerator: Some("metal".to_owned()),
                gpu_layers: Some(99),
                configured_max_tokens: Some(4096),
                effective_max_tokens: Some(768),
                max_tokens_clamped: true,
                max_tokens_clamp_reason: Some("low_priority_worker_default_max_tokens".to_owned()),
                can_accept_low_priority_task: true,
            },
            false,
        );

        assert!(json.contains("\"selected_role\":\"summary\""));
        assert!(json.contains("\"effective_max_tokens\":768"));
        assert!(json.contains("\"runtime_backend\":\"llama.cpp\""));
        assert!(json.contains("\"runtime_device\":\"metal\""));
        assert!(json.contains("\"runtime_accelerator\":\"metal\""));
        assert!(json.contains("\"gpu_layers\":99"));
        assert!(
            json.contains("\"max_tokens_clamp_reason\":\"low_priority_worker_default_max_tokens\"")
        );
        assert!(json.contains("\"worker_forwarded\":false"));
        assert!(json.contains("\"dispatch_mode\":\"backend_budget_only\""));
        assert!(json.contains("\"dispatch_reason\":\"runtime_endpoint_override_unavailable\""));
    }

    #[test]
    fn renders_missing_pool_dispatch_as_null() {
        assert_eq!(option_pool_dispatch_service_json(None, false), "null");
    }

    #[test]
    fn renders_forwarded_pool_dispatch_mode() {
        let json = pool_dispatch_service_json(
            &ModelServicePoolDispatchRequest {
                selected_role: "summary".to_owned(),
                selected_port: Some(8687),
                selected_base_url: Some("http://127.0.0.1:8687".to_owned()),
                context_window: Some(8192),
                default_max_tokens: Some(768),
                runtime_backend: None,
                runtime_device: None,
                runtime_accelerator: None,
                gpu_layers: None,
                configured_max_tokens: Some(4096),
                effective_max_tokens: Some(768),
                max_tokens_clamped: true,
                max_tokens_clamp_reason: Some("low_priority_worker_default_max_tokens".to_owned()),
                can_accept_low_priority_task: true,
            },
            true,
        );

        assert!(json.contains("\"worker_forwarded\":true"));
        assert!(json.contains("\"dispatch_mode\":\"runtime_endpoint_override\""));
        assert!(json.contains("\"dispatch_reason\":\"runtime_endpoint_override_active\""));
    }

    #[test]
    fn renders_missing_selected_endpoint_reason() {
        let json = pool_dispatch_service_json(
            &ModelServicePoolDispatchRequest {
                selected_role: "quality".to_owned(),
                selected_port: None,
                selected_base_url: None,
                context_window: Some(8192),
                default_max_tokens: Some(262_144),
                runtime_backend: None,
                runtime_device: None,
                runtime_accelerator: None,
                gpu_layers: None,
                configured_max_tokens: Some(262_144),
                effective_max_tokens: Some(262_144),
                max_tokens_clamped: false,
                max_tokens_clamp_reason: Some("quality_worker_request_budget_preserved".to_owned()),
                can_accept_low_priority_task: false,
            },
            false,
        );

        assert!(json.contains("\"dispatch_reason\":\"selected_endpoint_missing\""));
    }

    #[test]
    fn renders_pool_stage_dispatch_array() {
        let json = pool_stage_dispatch_service_json(&[ModelServicePoolStageDispatchRequest {
            task_kind: "summary".to_owned(),
            selected_role: "summary".to_owned(),
            selected_port: Some(8687),
            selected_base_url: Some("http://127.0.0.1:8687".to_owned()),
            context_window: Some(8192),
            default_max_tokens: Some(768),
            runtime_backend: Some("llama.cpp".to_owned()),
            runtime_device: Some("metal".to_owned()),
            runtime_accelerator: Some("metal".to_owned()),
            gpu_layers: Some(99),
            configured_max_tokens: Some(4096),
            effective_max_tokens: Some(768),
            max_tokens_clamped: true,
            max_tokens_clamp_reason: Some("low_priority_worker_default_max_tokens".to_owned()),
            can_accept_low_priority_task: true,
        }]);

        assert!(json.starts_with('['));
        assert!(json.contains("\"task_kind\":\"summary\""));
        assert!(json.contains("\"selected_role\":\"summary\""));
        assert!(json.contains("\"runtime_device\":\"metal\""));
        assert!(json.contains("\"effective_max_tokens\":768"));
        assert!(json.contains("\"dispatch_mode\":\"stage_plan_only\""));
        assert!(json.contains("\"dispatch_reason\":\"stage_dispatch_observed\""));
    }
}
