use super::evidence::{InspectEvidence, ReplayEvidence};
use super::requests::ModelServiceSmokeFollowupResponses;
use crate::model_service::http::model_service_http_body;

pub(super) struct ModelServiceSmokeResponses<'a> {
    pub(super) health_body: &'a str,
    pub(super) self_improve_body: &'a str,
    pub(super) inspect_body: &'a str,
    pub(super) replay: ReplayEvidence,
    pub(super) inspect: InspectEvidence,
}

impl<'a> ModelServiceSmokeResponses<'a> {
    pub(super) fn from_followup(
        health: &'a str,
        followup: &'a ModelServiceSmokeFollowupResponses,
    ) -> Self {
        let health_body = model_service_http_body(health);
        let self_improve_body = model_service_http_body(&followup.self_improve);
        let inspect_body = model_service_http_body(&followup.inspect);
        Self {
            health_body,
            self_improve_body,
            inspect_body,
            replay: ReplayEvidence::from_body(self_improve_body),
            inspect: InspectEvidence::from_body(inspect_body),
        }
    }
}
