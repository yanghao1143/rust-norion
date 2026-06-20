use crate::gemma_business::model_service_smoke::case_flow::ModelServiceCaseRun;
use crate::gemma_business::model_service_smoke::evidence::{InspectEvidence, ReplayEvidence};

pub(in crate::gemma_business::model_service_smoke) struct ModelServiceSmokeGateInputs<'a> {
    pub(in crate::gemma_business::model_service_smoke) health_body: &'a str,
    pub(in crate::gemma_business::model_service_smoke) self_improve_body: &'a str,
    pub(in crate::gemma_business::model_service_smoke) inspect_body: &'a str,
    pub(in crate::gemma_business::model_service_smoke) case_run: &'a ModelServiceCaseRun,
    pub(in crate::gemma_business::model_service_smoke) replay: &'a ReplayEvidence,
    pub(in crate::gemma_business::model_service_smoke) inspect: &'a InspectEvidence,
}
