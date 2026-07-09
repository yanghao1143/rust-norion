use rust_norion::TenantScope;

use super::super::json::{json_bool_field, json_usize_field};
use super::inspect::{ModelServiceInspectRequest, parse_model_service_gate_request};
use super::scope::require_tenant_scope;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelServiceReplayRequest {
    pub(crate) limit: usize,
    pub(crate) tenant_scope: Option<TenantScope>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelServiceSelfImproveRequest {
    pub(crate) limit: usize,
    pub(crate) require_deep_self_evolution: bool,
    pub(crate) inspect: ModelServiceInspectRequest,
}

pub(super) fn parse_replay_request(body: &str) -> Result<ModelServiceReplayRequest, String> {
    Ok(ModelServiceReplayRequest {
        limit: json_usize_field(body, "limit").unwrap_or(1).max(1),
        tenant_scope: Some(require_tenant_scope(body)?),
    })
}

pub(super) fn parse_self_improve_request(
    body: &str,
) -> Result<ModelServiceSelfImproveRequest, String> {
    let limit = json_usize_field(body, "limit").unwrap_or(1).max(1);
    let require_deep_self_evolution =
        json_bool_field(body, "require_deep_self_evolution").unwrap_or(true);
    let inspect = parse_model_service_gate_request(body, "self-improve")?;

    Ok(ModelServiceSelfImproveRequest {
        limit,
        require_deep_self_evolution,
        inspect,
    })
}
