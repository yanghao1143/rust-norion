use super::super::json::json_usize_field;
use super::inspect::{ModelServiceInspectRequest, parse_model_service_gate_request};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelServiceReplayRequest {
    pub(crate) limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelServiceSelfImproveRequest {
    pub(crate) limit: usize,
    pub(crate) inspect: ModelServiceInspectRequest,
}

pub(super) fn parse_replay_request(body: &str) -> ModelServiceReplayRequest {
    ModelServiceReplayRequest {
        limit: json_usize_field(body, "limit").unwrap_or(1).max(1),
    }
}

pub(super) fn parse_self_improve_request(
    body: &str,
) -> Result<ModelServiceSelfImproveRequest, String> {
    let limit = json_usize_field(body, "limit").unwrap_or(1).max(1);
    let inspect = parse_model_service_gate_request(body, "self-improve")?;

    Ok(ModelServiceSelfImproveRequest { limit, inspect })
}
