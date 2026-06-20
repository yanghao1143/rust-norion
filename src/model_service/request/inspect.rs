use super::super::json::{json_bool_field, json_string_field};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelServiceInspectRequest {
    pub(crate) state_gate: bool,
    pub(crate) business_gate: bool,
    pub(crate) business_cycle_gate: bool,
    pub(crate) model_service_gate: bool,
    pub(crate) trace_gate: Option<bool>,
}

impl Default for ModelServiceInspectRequest {
    fn default() -> Self {
        Self {
            state_gate: true,
            business_gate: false,
            business_cycle_gate: false,
            model_service_gate: false,
            trace_gate: None,
        }
    }
}

pub(super) fn parse_model_service_gate_request(
    body: &str,
    endpoint_name: &str,
) -> Result<ModelServiceInspectRequest, String> {
    let gate = json_string_field(body, "gate").unwrap_or_default();
    let normalized_gate = gate.trim().to_ascii_lowercase();
    let business_gate_from_name = matches!(
        normalized_gate.as_str(),
        "gemma_business_smoke" | "gemma-business-smoke" | "gemma_business" | "business"
    );
    let gemma_business_cycle_gate_from_name = matches!(
        normalized_gate.as_str(),
        "gemma_business_cycle"
            | "gemma-business-cycle"
            | "gemma_business_cycle_smoke"
            | "gemma-business-cycle-smoke"
    );
    let business_cycle_gate_from_name = gemma_business_cycle_gate_from_name
        || matches!(
            normalized_gate.as_str(),
            "business_cycle" | "business-cycle" | "cycle"
        );
    let model_service_gate_from_name = matches!(
        normalized_gate.as_str(),
        "gemma_model_service_smoke"
            | "gemma-model-service-smoke"
            | "gemma_service_smoke"
            | "gemma-service-smoke"
            | "model_service"
            | "model-service"
            | "service"
    );
    if !normalized_gate.is_empty()
        && !business_gate_from_name
        && !business_cycle_gate_from_name
        && !model_service_gate_from_name
        && normalized_gate != "state"
        && normalized_gate != "cli"
    {
        return Err(format!("unsupported {endpoint_name} gate: {gate}"));
    }

    Ok(ModelServiceInspectRequest {
        state_gate: json_bool_field(body, "state_gate").unwrap_or(true),
        business_gate: json_bool_field(body, "business_gate")
            .unwrap_or(business_gate_from_name || gemma_business_cycle_gate_from_name),
        business_cycle_gate: json_bool_field(body, "business_cycle_gate")
            .unwrap_or(business_cycle_gate_from_name),
        model_service_gate: json_bool_field(body, "model_service_gate")
            .unwrap_or(model_service_gate_from_name),
        trace_gate: json_bool_field(body, "trace_gate"),
    })
}
