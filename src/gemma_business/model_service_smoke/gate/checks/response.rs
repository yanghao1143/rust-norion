use crate::gemma_business::health_status::SmokeHealthStatus;
use crate::gemma_business::response_json::{response_object_bool_field, response_ok};

pub(in crate::gemma_business::model_service_smoke::gate) fn require_response_ok(
    body: &str,
    message: &str,
    failures: &mut Vec<String>,
) {
    if !response_ok(body) {
        failures.push(message.to_owned());
    }
}

pub(in crate::gemma_business::model_service_smoke::gate) fn require_health_preflight(
    body: &str,
    failures: &mut Vec<String>,
) {
    SmokeHealthStatus::from_body(body).push_gate_failures(failures);
}

pub(in crate::gemma_business::model_service_smoke::gate) fn require_response_object_bool(
    body: &str,
    object: &str,
    field: &str,
    message: &str,
    failures: &mut Vec<String>,
) {
    if !response_object_bool_field(body, object, field) {
        failures.push(message.to_owned());
    }
}

#[cfg(test)]
mod tests {
    use super::{require_health_preflight, require_response_object_bool, require_response_ok};

    #[test]
    fn require_response_ok_records_false_response_only() {
        let mut failures = Vec::new();

        require_response_ok(r#"{"ok":true}"#, "ok failed", &mut failures);
        require_response_ok(r#"{"ok":false}"#, "not ok", &mut failures);

        assert_eq!(failures, vec!["not ok".to_owned()]);
    }

    #[test]
    fn require_response_object_bool_records_missing_or_false_only() {
        let mut failures = Vec::new();

        require_response_object_bool(
            r#"{"state_gate":{"passed":true}}"#,
            "state_gate",
            "passed",
            "state failed",
            &mut failures,
        );
        require_response_object_bool(
            r#"{"state_gate":{"passed":false}}"#,
            "state_gate",
            "passed",
            "state failed",
            &mut failures,
        );
        require_response_object_bool(
            r#"{"trace_gate":{"passed":true}}"#,
            "state_gate",
            "passed",
            "state missing",
            &mut failures,
        );

        assert_eq!(
            failures,
            vec!["state failed".to_owned(), "state missing".to_owned()]
        );
    }

    #[test]
    fn require_health_preflight_records_structured_failures() {
        let mut failures = Vec::new();

        require_health_preflight(
            r#"{"ok":true,"readiness_ok":false,"safe_device_ok":false,"readiness_failures":["runtime"],"safe_device_failures":["cpu"]}"#,
            &mut failures,
        );

        assert_eq!(
            failures,
            vec![
                "health readiness failed: runtime".to_owned(),
                "health safe-device failed: cpu".to_owned()
            ]
        );
    }
}
