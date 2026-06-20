use crate::gemma_business::health_status::SmokeHealthStatus;
use crate::model_service::json::service_json_string_array;

pub(in crate::gemma_business::smoke_report) fn http_json(
    health: &SmokeHealthStatus,
    business_cycle_ok: bool,
    business_cycle_passed: bool,
    state_gate_passed: bool,
    trace_gate_passed: bool,
) -> String {
    format!(
        "{{\"health_ok\":{},\"readiness_ok\":{},\"readiness_passed\":{},\"readiness_failures\":{},\"readiness_failure_count\":{},\"safe_device_ok\":{},\"safe_device_passed\":{},\"safe_device_failures\":{},\"safe_device_failure_count\":{},\"business_cycle_ok\":{},\"business_cycle_passed\":{},\"state_gate_passed\":{},\"trace_gate_passed\":{}}}",
        health.ok,
        option_bool_json(health.readiness_ok),
        health.readiness_passed(),
        service_json_string_array(&health.readiness_failures),
        health.readiness_failures.len(),
        option_bool_json(health.safe_device_ok),
        health.safe_device_passed(),
        service_json_string_array(&health.safe_device_failures),
        health.safe_device_failures.len(),
        business_cycle_ok,
        business_cycle_passed,
        state_gate_passed,
        trace_gate_passed
    )
}

fn option_bool_json(value: Option<bool>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

#[cfg(test)]
mod tests {
    use crate::gemma_business::health_status::SmokeHealthStatus;

    use super::http_json;

    #[test]
    fn http_json_renders_cycle_gate_statuses() {
        let health = SmokeHealthStatus::from_body(
            r#"{"ok":true,"readiness_ok":true,"safe_device_ok":false,"safe_device_failures":["cpu"]}"#,
        );

        assert_eq!(
            http_json(&health, true, false, true, false),
            "{\"health_ok\":true,\"readiness_ok\":true,\"readiness_passed\":true,\"readiness_failures\":[],\"readiness_failure_count\":0,\"safe_device_ok\":false,\"safe_device_passed\":false,\"safe_device_failures\":[\"cpu\"],\"safe_device_failure_count\":1,\"business_cycle_ok\":true,\"business_cycle_passed\":false,\"state_gate_passed\":true,\"trace_gate_passed\":false}"
        );
    }
}
