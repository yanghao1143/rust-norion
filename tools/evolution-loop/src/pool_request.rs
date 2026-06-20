use crate::args::Config;
use crate::json::json_string;
use crate::pool_dispatch::PoolDispatchDecision;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PoolRequestPlan {
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

impl PoolRequestPlan {
    pub(crate) fn from_decision(config: &Config, decision: &PoolDispatchDecision) -> Self {
        let worker_default = decision
            .default_max_tokens
            .and_then(|value| usize::try_from(value).ok())
            .filter(|value| *value > 0);
        let helper_default_limit = decision.can_accept_low_priority_task
            && !decision
                .selected_role
                .trim()
                .eq_ignore_ascii_case("quality");
        let effective_max_tokens = if helper_default_limit {
            worker_default
                .map(|value| config.max_tokens.min(value))
                .unwrap_or(config.max_tokens)
        } else {
            config.max_tokens
        };
        Self {
            selected_role: decision.selected_role.clone(),
            selected_port: decision.selected_port,
            selected_base_url: decision.selected_base_url.clone(),
            context_window: decision.context_window,
            default_max_tokens: decision.default_max_tokens,
            runtime_backend: decision.runtime_backend.clone(),
            runtime_device: decision.runtime_device.clone(),
            runtime_accelerator: decision.runtime_accelerator.clone(),
            gpu_layers: decision.gpu_layers,
            configured_max_tokens: config.max_tokens,
            effective_max_tokens,
            max_tokens_clamped: effective_max_tokens < config.max_tokens,
            can_accept_low_priority_task: decision.can_accept_low_priority_task,
        }
    }

    pub(crate) fn request_json_field(&self) -> String {
        format!(
            ",\"pool_dispatch\":{{\"selected_role\":{},\"selected_port\":{},\"selected_base_url\":{},\"context_window\":{},\"default_max_tokens\":{},\"runtime_backend\":{},\"runtime_device\":{},\"runtime_accelerator\":{},\"gpu_layers\":{},\"configured_max_tokens\":{},\"effective_max_tokens\":{},\"max_tokens_clamped\":{},\"can_accept_low_priority_task\":{}}}",
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
            "pool_dispatch selected_role={} port={} endpoint={} context_window={} default_max_tokens={} runtime_backend={} runtime_device={} runtime_accelerator={} gpu_layers={} configured_max_tokens={} effective_max_tokens={} max_tokens_clamped={} low_priority={}",
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

#[cfg(test)]
mod tests {
    use super::*;

    fn decision(default_max_tokens: Option<u64>) -> PoolDispatchDecision {
        PoolDispatchDecision {
            selected_role: "summary".to_owned(),
            selected_port: Some(8687),
            selected_base_url: Some("http://127.0.0.1:8687".to_owned()),
            context_window: Some(8192),
            default_max_tokens,
            runtime_backend: Some("llama.cpp".to_owned()),
            runtime_device: Some("metal".to_owned()),
            runtime_accelerator: Some("metal".to_owned()),
            gpu_layers: Some(99),
            can_accept_low_priority_task: true,
            evidence: "route_allowed:true".to_owned(),
        }
    }

    #[test]
    fn clamps_to_selected_worker_default_budget() {
        let plan = PoolRequestPlan::from_decision(
            &Config {
                max_tokens: 4096,
                ..Config::default()
            },
            &decision(Some(768)),
        );

        assert_eq!(plan.effective_max_tokens, 768);
        assert!(plan.max_tokens_clamped);
    }

    #[test]
    fn preserves_quality_worker_budget() {
        let mut decision = decision(Some(4096));
        decision.selected_role = "quality".to_owned();
        decision.can_accept_low_priority_task = false;

        let plan = PoolRequestPlan::from_decision(
            &Config {
                max_tokens: 262_144,
                ..Config::default()
            },
            &decision,
        );

        assert_eq!(plan.effective_max_tokens, 262_144);
        assert!(!plan.max_tokens_clamped);
    }

    #[test]
    fn keeps_config_budget_when_worker_default_is_missing() {
        let plan = PoolRequestPlan::from_decision(
            &Config {
                max_tokens: 4096,
                ..Config::default()
            },
            &decision(None),
        );

        assert_eq!(plan.effective_max_tokens, 4096);
        assert!(!plan.max_tokens_clamped);
    }

    #[test]
    fn renders_request_json_and_meta() {
        let plan = PoolRequestPlan::from_decision(
            &Config {
                max_tokens: 4096,
                ..Config::default()
            },
            &decision(Some(1024)),
        );

        let json = plan.request_json_field();
        assert!(json.contains("\"pool_dispatch\""));
        assert!(json.contains("\"selected_role\":\"summary\""));
        assert!(json.contains("\"effective_max_tokens\":1024"));
        assert!(json.contains("\"runtime_backend\":\"llama.cpp\""));
        assert!(json.contains("\"runtime_device\":\"metal\""));
        assert!(json.contains("\"runtime_accelerator\":\"metal\""));
        assert!(json.contains("\"gpu_layers\":99"));
        assert!(plan.meta().contains("pool_dispatch selected_role=summary"));
        assert!(plan.meta().contains("runtime_device=metal"));
    }
}
