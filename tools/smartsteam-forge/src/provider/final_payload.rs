use super::json::{json_bool_field, json_number_field, json_string_field};

mod render;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinalPayloadSummary {
    pub ok: Option<bool>,
    pub passed: Option<bool>,
    pub answer: Option<String>,
    pub runtime_model: Option<String>,
    pub runtime_token_count: Option<String>,
    pub runtime_uncertainty_signal: Option<bool>,
    pub runtime_device_execution_source: Option<String>,
    pub pool_dispatch_selected_role: Option<String>,
    pub pool_dispatch_worker_forwarded: Option<bool>,
    pub pool_dispatch_mode: Option<String>,
    pub pool_dispatch_reason: Option<String>,
    pub generate_passed: Option<bool>,
    pub feedback_passed: Option<bool>,
    pub feedback_applied: Option<bool>,
    pub feedback_applied_count: Option<String>,
    pub rust_check_checked: Option<bool>,
    pub rust_check_passed: Option<bool>,
    pub rust_check_feedback_applied_count: Option<String>,
    pub self_improve_checked: Option<bool>,
    pub self_improve_passed: Option<bool>,
    pub state_gate_checked: Option<bool>,
    pub state_gate_passed: Option<bool>,
    pub trace_gate_checked: Option<bool>,
    pub trace_gate_passed: Option<bool>,
    pub error: Option<String>,
}

impl FinalPayloadSummary {
    pub fn parse(payload: &str) -> Self {
        Self {
            ok: json_bool_field(payload, "ok"),
            passed: json_bool_field(payload, "passed"),
            answer: json_string_field(payload, "answer"),
            runtime_model: json_string_field(payload, "runtime_model"),
            runtime_token_count: json_number_field(payload, "runtime_token_count"),
            runtime_uncertainty_signal: json_bool_field(payload, "runtime_uncertainty_signal"),
            runtime_device_execution_source: json_string_field(
                payload,
                "runtime_device_execution_source",
            ),
            pool_dispatch_selected_role: json_string_field(payload, "selected_role"),
            pool_dispatch_worker_forwarded: json_bool_field(payload, "worker_forwarded"),
            pool_dispatch_mode: json_string_field(payload, "dispatch_mode"),
            pool_dispatch_reason: json_string_field(payload, "dispatch_reason"),
            generate_passed: json_bool_field(payload, "generate_passed"),
            feedback_passed: json_bool_field(payload, "feedback_passed"),
            feedback_applied: json_bool_field(payload, "feedback_applied"),
            feedback_applied_count: json_number_field(payload, "feedback_applied"),
            rust_check_checked: json_bool_field(payload, "rust_check_checked"),
            rust_check_passed: json_bool_field(payload, "rust_check_passed"),
            rust_check_feedback_applied_count: json_number_field(
                payload,
                "rust_check_feedback_applied",
            ),
            self_improve_checked: json_bool_field(payload, "self_improve_checked"),
            self_improve_passed: json_bool_field(payload, "self_improve_passed"),
            state_gate_checked: json_bool_field(payload, "state_gate_checked"),
            state_gate_passed: json_bool_field(payload, "state_gate_passed"),
            trace_gate_checked: json_bool_field(payload, "trace_gate_checked"),
            trace_gate_passed: json_bool_field(payload, "trace_gate_passed"),
            error: json_string_field(payload, "error"),
        }
    }

    pub fn answer(&self) -> Option<&str> {
        self.answer.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarizes_generate_final_payload() {
        let payload = "{\"ok\":true,\"answer\":\"最终答案\",\"runtime_model\":\"gemma-3-12b\",\"runtime_token_count\":42,\"runtime_uncertainty_signal\":true,\"runtime_device_execution_source\":\"cpu\"}";

        let summary = FinalPayloadSummary::parse(payload);

        assert_eq!(summary.answer(), Some("最终答案"));
        assert_eq!(summary.runtime_model.as_deref(), Some("gemma-3-12b"));
        assert_eq!(summary.runtime_token_count.as_deref(), Some("42"));
        assert!(summary.status_line().contains("runtime_tokens=42"));
        assert!(summary.status_line().contains("answer=\"最终答案\""));
        assert_eq!(summary.gate_report(), None);
    }

    #[test]
    fn summarizes_business_cycle_gate_fields() {
        let payload = "{\"ok\":true,\"business_cycle\":{\"passed\":false,\"generate_passed\":true,\"feedback_passed\":true,\"feedback_applied\":true,\"rust_check_checked\":true,\"rust_check_passed\":false,\"self_improve_passed\":true,\"state_gate_passed\":true,\"trace_gate_passed\":false},\"generate\":{\"answer\":\"修复建议\"}}";

        let summary = FinalPayloadSummary::parse(payload);

        assert_eq!(summary.passed, Some(false));
        assert_eq!(summary.rust_check_checked, Some(true));
        assert_eq!(summary.rust_check_passed, Some(false));
        assert_eq!(summary.answer(), Some("修复建议"));
        assert!(summary.status_line().contains("trace_gate_passed=false"));
        let report = summary.gate_report().unwrap();
        assert!(report.contains("overall: FAIL"));
        assert!(report.contains("generate: PASS"));
        assert!(report.contains("rust check: FAIL"));
        assert!(report.contains("trace gate: FAIL"));
    }

    #[test]
    fn summarizes_business_cycle_numeric_feedback_counts() {
        let payload = "{\"ok\":true,\"business_cycle\":{\"passed\":true,\"generate_passed\":true,\"feedback_passed\":true,\"feedback_applied\":2,\"rust_check_checked\":true,\"rust_check_passed\":true,\"rust_check_feedback_applied\":1,\"self_improve_checked\":true,\"self_improve_passed\":true,\"state_gate_checked\":true,\"state_gate_passed\":true,\"trace_gate_checked\":true,\"trace_gate_passed\":true},\"generate\":{\"runtime_model\":\"gemma\",\"runtime_token_count\":12,\"runtime_uncertainty_signal\":false,\"answer\":\"完成\"}}";

        let summary = FinalPayloadSummary::parse(payload);

        assert_eq!(summary.feedback_applied_count.as_deref(), Some("2"));
        assert_eq!(
            summary.rust_check_feedback_applied_count.as_deref(),
            Some("1")
        );
        assert_eq!(summary.self_improve_checked, Some(true));
        assert_eq!(summary.state_gate_checked, Some(true));
        assert!(summary.status_line().contains("feedback_applied_count=2"));
        assert!(
            summary
                .status_line()
                .contains("rust_check_feedback_applied=1")
        );
        let report = summary.gate_report().unwrap();
        assert!(report.contains("feedback applied: PASS count=2"));
        assert!(report.contains("rust check feedback applied: PASS count=1"));
        assert!(report.contains("self improve: PASS"));
    }

    #[test]
    fn summarizes_pool_dispatch_final_payload() {
        let payload = "{\"ok\":true,\"pool_dispatch\":{\"selected_role\":\"review\",\"worker_forwarded\":false,\"dispatch_mode\":\"backend_budget_only\",\"dispatch_reason\":\"runtime_endpoint_override_unavailable\"},\"business_cycle\":{\"passed\":true},\"generate\":{\"answer\":\"完成\"}}";

        let summary = FinalPayloadSummary::parse(payload);

        assert_eq!(
            summary.pool_dispatch_selected_role.as_deref(),
            Some("review")
        );
        assert_eq!(summary.pool_dispatch_worker_forwarded, Some(false));
        assert_eq!(
            summary.pool_dispatch_mode.as_deref(),
            Some("backend_budget_only")
        );
        assert_eq!(
            summary.pool_dispatch_reason.as_deref(),
            Some("runtime_endpoint_override_unavailable")
        );
        assert!(summary.status_line().contains("pool_role=review"));
        assert!(summary.status_line().contains("pool_forwarded=false"));
        assert!(
            summary
                .status_line()
                .contains("pool_reason=runtime_endpoint_override_unavailable")
        );
        assert!(
            summary
                .gate_report()
                .unwrap()
                .contains("pool dispatch: role=review forwarded=false")
        );
    }
}
