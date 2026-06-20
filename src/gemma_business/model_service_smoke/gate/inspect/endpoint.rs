use crate::gemma_business::model_service_smoke::gate::ModelServiceSmokeGateInputs;
use crate::gemma_business::model_service_smoke::gate::checks::{
    require_health_preflight, require_response_object_bool, require_response_ok,
};

pub(super) fn push_endpoint_gate_failures(
    input: &ModelServiceSmokeGateInputs<'_>,
    failures: &mut Vec<String>,
) {
    require_response_ok(
        input.health_body,
        "health endpoint did not return ok=true",
        failures,
    );
    require_health_preflight(input.health_body, failures);
    require_response_ok(
        input.self_improve_body,
        "self-improve endpoint did not return ok=true",
        failures,
    );
    require_response_object_bool(
        input.self_improve_body,
        "self_improve",
        "passed",
        "self-improve endpoint did not report passed=true",
        failures,
    );
    require_response_object_bool(
        input.inspect_body,
        "state_gate",
        "passed",
        "inspect endpoint state gate did not pass",
        failures,
    );
    require_response_object_bool(
        input.inspect_body,
        "trace_gate",
        "passed",
        "inspect endpoint trace gate did not pass",
        failures,
    );
}
