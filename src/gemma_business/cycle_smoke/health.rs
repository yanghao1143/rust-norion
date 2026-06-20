use crate::gemma_business::health_status::SmokeHealthStatus;
use crate::model_service::http::wait_for_model_service_http_response;

pub(super) fn fetch_business_cycle_health(bind: &str, failures: &mut Vec<String>) -> String {
    match wait_for_model_service_http_response(bind, "GET", "/health", None) {
        Ok(response) => response,
        Err(error) => {
            failures.push(format!("health endpoint failed: {error}"));
            String::new()
        }
    }
}

pub(super) fn require_business_cycle_health_ok(health_body: &str, failures: &mut Vec<String>) {
    let health = SmokeHealthStatus::from_body(health_body);
    if !health.ok {
        failures.push("health endpoint did not return ok=true".to_owned());
    }
    health.push_gate_failures(failures);
}
