use crate::gemma_business::GEMMA_MODEL_SERVICE_BUSINESS_CASES;
use crate::model_service::http::model_service_http_request;

pub(super) struct ModelServiceSmokeFollowupResponses {
    pub(super) self_improve: String,
    pub(super) inspect: String,
}

pub(super) fn run_model_service_smoke_followup_requests(
    bind: &str,
) -> std::io::Result<ModelServiceSmokeFollowupResponses> {
    let self_improve_request = model_service_self_improve_request_json();
    let self_improve = model_service_http_request(
        bind,
        "POST",
        "/v1/self-improve",
        Some(&self_improve_request),
    )?;
    let inspect = model_service_http_request(
        bind,
        "POST",
        "/v1/inspect",
        Some(&model_service_inspect_request_json()),
    )?;
    Ok(ModelServiceSmokeFollowupResponses {
        self_improve,
        inspect,
    })
}

fn model_service_self_improve_request_json() -> String {
    format!(
        "{{\"limit\":{},\"gate\":\"gemma_model_service_smoke\",\"trace_gate\":true}}",
        GEMMA_MODEL_SERVICE_BUSINESS_CASES.len().max(1)
    )
}

fn model_service_inspect_request_json() -> String {
    "{\"gate\":\"gemma_model_service_smoke\",\"trace_gate\":true}".to_owned()
}
