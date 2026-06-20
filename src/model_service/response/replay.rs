mod replay_json;
mod self_improve_json;

use rust_norion::{ExperienceReplayReport, StateInspectionReport};

use self::replay_json::model_service_replay_json;
pub(crate) use self::self_improve_json::model_service_self_improve_response_json;
use super::state::model_service_state_json;

pub(crate) fn model_service_replay_response_json(
    request_id: usize,
    limit: usize,
    report: &ExperienceReplayReport,
    inspection: &StateInspectionReport,
) -> String {
    format!(
        "{{\"ok\":true,\"request_id\":{},\"limit\":{},\"replay\":{},\"state\":{}}}",
        request_id,
        limit,
        model_service_replay_json(report),
        model_service_state_json(inspection)
    )
}

pub(super) fn option_replay_service_json(report: Option<&ExperienceReplayReport>) -> String {
    report
        .map(model_service_replay_json)
        .unwrap_or_else(|| "null".to_owned())
}
