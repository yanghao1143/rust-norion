use super::status_json::{
    bool_value_text, json_bool_field, json_object_field, json_string_array_field,
    json_string_literal,
};

pub(super) fn render_readiness_start_preflight(status: &str) -> String {
    let readiness = ReadinessStartStatus::from_status(status);
    format!(
        "readiness_preflight read_only=true starts_process=false sends_prompt=false status_ready={} start_ready={} blocks_start={} failures={} start_blocking_failures={}",
        bool_value_text(readiness.status_ready),
        bool_value_text(readiness.start_ready()),
        bool_value_text(!readiness.start_ready()),
        readiness.failures_text(),
        readiness.start_blocking_failures_text()
    )
}

pub(super) fn readiness_start_preflight_ready(preflight: &str) -> bool {
    preflight
        .lines()
        .any(|line| line.contains("readiness_preflight ") && line.contains(" blocks_start=false "))
}

pub(super) fn readiness_start_gate_json(readiness: &ReadinessStartStatus) -> String {
    format!(
        "{{\"read_only\":true,\"starts_process\":false,\"sends_prompt\":false,\"status_ready\":{},\"start_ready\":{},\"blocks_start\":{},\"failures\":{},\"start_blocking_failures\":{},\"block_reason\":{}}}",
        bool_value_text(readiness.status_ready),
        bool_value_text(readiness.start_ready()),
        bool_value_text(!readiness.start_ready()),
        json_string_array(&readiness.failures),
        json_string_array(&readiness.start_blocking_failures),
        if readiness.start_ready() {
            "null".to_owned()
        } else {
            json_string_literal("readiness_not_ready")
        }
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ReadinessStartStatus {
    status_ready: bool,
    pub(super) failures: Vec<String>,
    start_blocking_failures: Vec<String>,
}

impl ReadinessStartStatus {
    pub(super) fn from_status(status: &str) -> Self {
        Self::from_loop_status(json_object_field(status, "loop"))
    }

    pub(super) fn from_loop_status(loop_status: Option<&str>) -> Self {
        let readiness =
            loop_status.and_then(|loop_status| json_object_field(loop_status, "readiness"));
        let status_ready = readiness
            .and_then(|readiness| json_bool_field(readiness, "ready"))
            .unwrap_or(true);
        let failures = readiness
            .and_then(|readiness| json_string_array_field(readiness, "failures"))
            .unwrap_or_default();
        let start_blocking_failures = failures
            .iter()
            .filter(|failure| start_blocking_failure(failure))
            .cloned()
            .collect();
        Self {
            status_ready,
            failures,
            start_blocking_failures,
        }
    }

    pub(super) fn start_ready(&self) -> bool {
        self.start_blocking_failures.is_empty()
    }

    pub(super) fn failures_text(&self) -> String {
        if self.failures.is_empty() {
            "none".to_owned()
        } else {
            self.failures.join(",")
        }
    }

    pub(super) fn start_blocking_failures_text(&self) -> String {
        if self.start_blocking_failures.is_empty() {
            "none".to_owned()
        } else {
            self.start_blocking_failures.join(",")
        }
    }
}

fn start_blocking_failure(failure: &str) -> bool {
    matches!(
        failure,
        "backend_not_ready"
            | "remote_chain_not_ready"
            | "ledger_has_invalid_records"
            | "strict_ledger_hygiene_failed"
    )
}

fn json_string_array(values: &[String]) -> String {
    let values = values
        .iter()
        .map(|value| json_string_literal(value))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{values}]")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readiness_preflight_blocks_backend_not_ready() {
        let status = r#"{
            "loop": {
                "readiness": {
                    "ready": false,
                    "failures": ["backend_not_ready", "model_pool_alignment"]
                }
            }
        }"#;

        let preflight = render_readiness_start_preflight(status);
        let readiness = ReadinessStartStatus::from_status(status);

        assert!(!readiness.status_ready);
        assert!(!readiness.start_ready());
        assert_eq!(
            readiness.failures_text(),
            "backend_not_ready,model_pool_alignment"
        );
        assert!(!readiness_start_preflight_ready(&preflight));
        assert!(preflight.contains("status_ready=false"));
        assert!(preflight.contains("start_ready=false"));
        assert!(preflight.contains("blocks_start=true"));
        assert!(preflight.contains("failures=backend_not_ready,model_pool_alignment"));
        assert!(preflight.contains("start_blocking_failures=backend_not_ready"));
    }

    #[test]
    fn readiness_start_gate_json_surfaces_only_start_blockers() {
        let status = r#"{
            "loop": {
                "readiness": {
                    "ready": false,
                    "failures": [
                        "ledger_missing",
                        "rounds_below_minimum",
                        "backend_not_ready",
                        "remote_chain_not_ready"
                    ]
                }
            }
        }"#;
        let readiness = ReadinessStartStatus::from_status(status);

        let json = readiness_start_gate_json(&readiness);

        assert!(json.contains("\"read_only\":true"));
        assert!(json.contains("\"starts_process\":false"));
        assert!(json.contains("\"sends_prompt\":false"));
        assert!(json.contains("\"status_ready\":false"));
        assert!(json.contains("\"start_ready\":false"));
        assert!(json.contains("\"blocks_start\":true"));
        assert!(json.contains(
            "\"failures\":[\"ledger_missing\",\"rounds_below_minimum\",\"backend_not_ready\",\"remote_chain_not_ready\"]"
        ));
        assert!(json.contains(
            "\"start_blocking_failures\":[\"backend_not_ready\",\"remote_chain_not_ready\"]"
        ));
        assert!(json.contains("\"block_reason\":\"readiness_not_ready\""));
    }

    #[test]
    fn readiness_preflight_allows_first_round_bootstrap_failures() {
        let status = r#"{
            "loop": {
                "readiness": {
                    "ready": false,
                    "failures": [
                        "ledger_missing",
                        "rounds_below_minimum",
                        "feedback_below_minimum",
                        "latest_round_not_successful"
                    ]
                }
            }
        }"#;

        let preflight = render_readiness_start_preflight(status);
        let readiness = ReadinessStartStatus::from_status(status);

        assert!(!readiness.status_ready);
        assert!(readiness.start_ready());
        assert!(readiness_start_preflight_ready(&preflight));
        assert!(preflight.contains("status_ready=false"));
        assert!(preflight.contains("start_ready=true"));
        assert!(preflight.contains("blocks_start=false"));
        assert!(preflight.contains("start_blocking_failures=none"));
    }

    #[test]
    fn readiness_preflight_allows_missing_legacy_readiness() {
        let preflight = render_readiness_start_preflight("{}");

        assert!(readiness_start_preflight_ready(&preflight));
        assert!(preflight.contains("status_ready=true"));
        assert!(preflight.contains("start_ready=true"));
        assert!(preflight.contains("blocks_start=false"));
        assert!(preflight.contains("failures=none"));
    }
}
