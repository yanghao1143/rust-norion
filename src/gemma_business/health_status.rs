use crate::gemma_business::response_json::{
    response_ok, response_optional_bool_field, response_string_array_field,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::gemma_business) struct SmokeHealthStatus {
    pub(in crate::gemma_business) ok: bool,
    pub(in crate::gemma_business) readiness_ok: Option<bool>,
    pub(in crate::gemma_business) safe_device_ok: Option<bool>,
    pub(in crate::gemma_business) readiness_failures: Vec<String>,
    pub(in crate::gemma_business) safe_device_failures: Vec<String>,
}

impl SmokeHealthStatus {
    pub(in crate::gemma_business) fn from_body(body: &str) -> Self {
        Self {
            ok: response_ok(body),
            readiness_ok: response_optional_bool_field(body, "readiness_ok"),
            safe_device_ok: response_optional_bool_field(body, "safe_device_ok"),
            readiness_failures: response_string_array_field(body, "readiness_failures"),
            safe_device_failures: response_string_array_field(body, "safe_device_failures"),
        }
    }

    pub(in crate::gemma_business) fn readiness_passed(&self) -> bool {
        self.readiness_ok.unwrap_or(true)
    }

    pub(in crate::gemma_business) fn safe_device_passed(&self) -> bool {
        self.safe_device_ok.unwrap_or(true)
    }

    pub(in crate::gemma_business) fn push_gate_failures(&self, failures: &mut Vec<String>) {
        if self.readiness_ok == Some(false) {
            failures.push(format!(
                "health readiness failed: {}",
                failure_list_or_unknown(&self.readiness_failures)
            ));
        }
        if self.safe_device_ok == Some(false) {
            failures.push(format!(
                "health safe-device failed: {}",
                failure_list_or_unknown(&self.safe_device_failures)
            ));
        }
    }
}

pub(in crate::gemma_business) fn optional_bool_label(value: Option<bool>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unknown".to_owned())
}

fn failure_list_or_unknown(failures: &[String]) -> String {
    if failures.is_empty() {
        "unknown".to_owned()
    } else {
        failures.join("|")
    }
}

#[cfg(test)]
mod tests {
    use super::{SmokeHealthStatus, optional_bool_label};

    #[test]
    fn health_status_parses_structured_preflight_fields() {
        let status = SmokeHealthStatus::from_body(
            r#"{"ok":true,"readiness_ok":false,"safe_device_ok":false,"readiness_failures":["busy"],"safe_device_failures":["cpu"]}"#,
        );

        assert!(status.ok);
        assert!(!status.readiness_passed());
        assert!(!status.safe_device_passed());
        assert_eq!(status.readiness_failures, vec!["busy".to_owned()]);
        assert_eq!(status.safe_device_failures, vec!["cpu".to_owned()]);
    }

    #[test]
    fn health_status_keeps_legacy_health_readiness_compatible() {
        let status = SmokeHealthStatus::from_body(r#"{"ok":true}"#);

        assert!(status.ok);
        assert!(status.readiness_passed());
        assert!(status.safe_device_passed());
        assert_eq!(optional_bool_label(status.readiness_ok), "unknown");
    }

    #[test]
    fn health_status_pushes_structured_gate_failures() {
        let status = SmokeHealthStatus::from_body(
            r#"{"ok":true,"readiness_ok":false,"safe_device_ok":false,"readiness_failures":["runtime"],"safe_device_failures":["cpu"]}"#,
        );
        let mut failures = Vec::new();

        status.push_gate_failures(&mut failures);

        assert_eq!(
            failures,
            vec![
                "health readiness failed: runtime".to_owned(),
                "health safe-device failed: cpu".to_owned()
            ]
        );
    }
}
