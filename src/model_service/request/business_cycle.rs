use rust_norion::{RewardAction, TaskProfile, TenantScope};

use super::super::json::{json_bool_field, json_f32_field, json_string_field, json_usize_field};
use super::inspect::{ModelServiceInspectRequest, parse_model_service_gate_request};
use super::pool_dispatch::{
    ModelServicePoolDispatchRequest, ModelServicePoolStageDispatchRequest,
    parse_pool_dispatch_request, parse_pool_stage_dispatch_requests,
};
use super::scope::require_tenant_scope;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ModelServiceBusinessCycleRequest {
    pub(crate) prompt: String,
    pub(crate) profile: Option<TaskProfile>,
    pub(crate) case_name: Option<String>,
    pub(crate) max_tokens: Option<usize>,
    pub(crate) feedback_action: RewardAction,
    pub(crate) feedback_amount: f32,
    pub(crate) rust_check_code: Option<String>,
    pub(crate) rust_check_edition: String,
    pub(crate) rust_check_case_name: Option<String>,
    pub(crate) self_improve: bool,
    pub(crate) self_improve_limit: usize,
    pub(crate) pool_dispatch: Option<ModelServicePoolDispatchRequest>,
    pub(crate) pool_stage_dispatch: Vec<ModelServicePoolStageDispatchRequest>,
    pub(crate) inspect: ModelServiceInspectRequest,
    pub(crate) tenant_scope: Option<TenantScope>,
}

pub(super) fn parse_business_cycle_request(
    body: &str,
) -> Result<ModelServiceBusinessCycleRequest, String> {
    let prompt = json_string_field(body, "prompt")
        .filter(|prompt| !prompt.trim().is_empty())
        .ok_or_else(|| "business_cycle requires a non-empty prompt string".to_owned())?;
    let profile = json_string_field(body, "profile")
        .map(|value| value.parse::<TaskProfile>())
        .transpose()
        .map_err(|error| error.to_string())?;
    let case_name = json_string_field(body, "case").filter(|case| !case.trim().is_empty());
    let max_tokens = json_usize_field(body, "max_tokens")
        .or_else(|| json_usize_field(body, "max"))
        .map(|value| value.max(1));
    let action_text = json_string_field(body, "feedback_action")
        .or_else(|| json_string_field(body, "action"))
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "reinforce".to_owned());
    let feedback_action = action_text
        .trim()
        .parse::<RewardAction>()
        .map_err(|_| format!("unsupported business_cycle feedback action: {action_text}"))?;
    if feedback_action == RewardAction::Hold {
        return Err("business_cycle feedback action must be reinforce or penalize".to_owned());
    }
    let feedback_amount = json_f32_field(body, "feedback_amount")
        .or_else(|| json_f32_field(body, "amount"))
        .unwrap_or(0.5)
        .clamp(0.0, 1.0);
    let rust_check_code = json_string_field(body, "rust_check_code")
        .or_else(|| json_string_field(body, "code"))
        .filter(|code| !code.trim().is_empty());
    let rust_check_edition = json_string_field(body, "rust_check_edition")
        .or_else(|| json_string_field(body, "edition"))
        .filter(|edition| !edition.trim().is_empty())
        .unwrap_or_else(|| "2021".to_owned());
    let rust_check_case_name = json_string_field(body, "rust_check_case")
        .or_else(|| json_string_field(body, "rust_case"))
        .filter(|case| !case.trim().is_empty());
    let self_improve = json_bool_field(body, "self_improve").unwrap_or(true);
    let self_improve_limit = json_usize_field(body, "self_improve_limit")
        .or_else(|| json_usize_field(body, "limit"))
        .unwrap_or(1)
        .max(1);
    let pool_dispatch = parse_pool_dispatch_request(body)?;
    let pool_stage_dispatch = parse_pool_stage_dispatch_requests(body)?;
    let inspect = parse_model_service_gate_request(body, "business-cycle")?;
    let tenant_scope = require_tenant_scope(body)?;

    Ok(ModelServiceBusinessCycleRequest {
        prompt,
        profile,
        case_name,
        max_tokens,
        feedback_action,
        feedback_amount,
        rust_check_code,
        rust_check_edition,
        rust_check_case_name,
        self_improve,
        self_improve_limit,
        pool_dispatch,
        pool_stage_dispatch,
        inspect,
        tenant_scope: Some(tenant_scope),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_business_cycle_pool_dispatch() {
        let request = parse_business_cycle_request(
            "{\"prompt\":\"review this\",\"max_tokens\":4096,\"tenant_id\":\"tenant-a\",\"workspace_id\":\"workspace\",\"session_id\":\"cycle-dispatch\",\"pool_dispatch\":{\"selected_role\":\"review\",\"selected_port\":8688,\"selected_base_url\":\"http://127.0.0.1:8688\",\"default_max_tokens\":1024,\"effective_max_tokens\":1024,\"max_tokens_clamped\":true}}",
        )
        .unwrap();

        let dispatch = request.pool_dispatch.unwrap();
        assert_eq!(dispatch.selected_role, "review");
        assert_eq!(dispatch.selected_port, Some(8688));
        assert_eq!(dispatch.effective_max_tokens, Some(1024));
        assert!(dispatch.max_tokens_clamped);
    }

    #[test]
    fn parses_business_cycle_pool_stage_dispatch() {
        let request = parse_business_cycle_request(
            "{\"prompt\":\"review this\",\"tenant_id\":\"tenant-a\",\"workspace_id\":\"workspace\",\"session_id\":\"cycle-stage\",\"pool_stage_dispatch\":[{\"task_kind\":\"summary\",\"selected_role\":\"summary\",\"selected_port\":8687,\"selected_base_url\":\"http://127.0.0.1:8687\",\"effective_max_tokens\":768,\"max_tokens_clamped\":true},{\"task_kind\":\"test-gate\",\"selected_role\":\"test-gate\",\"selected_port\":8689}]}",
        )
        .unwrap();

        assert_eq!(request.pool_stage_dispatch.len(), 2);
        assert_eq!(request.pool_stage_dispatch[0].task_kind, "summary");
        assert_eq!(request.pool_stage_dispatch[0].selected_role, "summary");
        assert_eq!(request.pool_stage_dispatch[0].selected_port, Some(8687));
        assert_eq!(
            request.pool_stage_dispatch[0].selected_base_url.as_deref(),
            Some("http://127.0.0.1:8687")
        );
        assert_eq!(
            request.pool_stage_dispatch[0].effective_max_tokens,
            Some(768)
        );
        assert!(request.pool_stage_dispatch[0].max_tokens_clamped);
        assert_eq!(request.pool_stage_dispatch[1].task_kind, "test-gate");
    }

    #[test]
    fn parses_business_cycle_tenant_scope() {
        let request = parse_business_cycle_request(
            "{\"prompt\":\"review this\",\"tenant_id\":\"tenant-a\",\"workspace_id\":\"workspace\",\"session_id\":\"cycle-1\"}",
        )
        .unwrap();

        assert_eq!(
            request.tenant_scope,
            Some(TenantScope::new("tenant-a", "workspace", "cycle-1"))
        );
    }

    #[test]
    fn rejects_missing_business_cycle_tenant_scope() {
        assert_eq!(
            parse_business_cycle_request("{\"prompt\":\"review this\"}").unwrap_err(),
            "tenant scope requires tenant_id, workspace_id, and session_id"
        );
    }
}
