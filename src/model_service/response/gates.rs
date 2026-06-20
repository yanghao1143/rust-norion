use rust_norion::{StateInspectionGateReport, TraceSchemaGateReport};

use super::super::json::{service_json_string, service_json_string_array};

pub(super) fn option_state_gate_service_json(report: Option<&StateInspectionGateReport>) -> String {
    report
        .map(|report| {
            format!(
                "{{\"passed\":{},\"summary\":{},\"failures\":{}}}",
                report.passed,
                service_json_string(&report.summary_line()),
                service_json_string_array(&report.failures)
            )
        })
        .unwrap_or_else(|| "null".to_owned())
}

pub(super) fn option_trace_gate_service_json(report: Option<&TraceSchemaGateReport>) -> String {
    report
        .map(|report| {
            format!(
                "{{\"passed\":{},\"checked_lines\":{},\"rust_check_events\":{},\"rust_check_passed\":{},\"rust_check_failed\":{},\"rust_check_feedback_updates\":{},\"rust_check_feedback_applied\":{},\"business_contract_events\":{},\"business_contract_event_passed\":{},\"business_contract_event_failed\":{},\"business_contract_event_missing_signals\":{},\"business_contract_event_protocol_leaks\":{},\"business_contract_event_substitutions\":{},\"business_contract_event_evasive_denials\":{},\"business_contract_event_raw_passed\":{},\"business_contract_event_raw_failed\":{},\"business_contract_event_response_normalized\":{},\"business_contract_event_sanitized\":{},\"business_contract_event_canonical_fallbacks\":{},\"runtime_error_events\":{},\"runtime_timeout_events\":{},\"summary\":{},\"failures\":{}}}",
                report.passed,
                report.checked_lines,
                report.rust_check_events,
                report.rust_check_passed,
                report.rust_check_failed,
                report.rust_check_feedback_updates,
                report.rust_check_feedback_applied,
                report.business_contract_events,
                report.business_contract_event_passed,
                report.business_contract_event_failed,
                report.business_contract_event_missing_signals,
                report.business_contract_event_protocol_leaks,
                report.business_contract_event_substitutions,
                report.business_contract_event_evasive_denials,
                report.business_contract_event_raw_passed,
                report.business_contract_event_raw_failed,
                report.business_contract_event_response_normalized,
                report.business_contract_event_sanitized,
                report.business_contract_event_canonical_fallbacks,
                report.runtime_error_events,
                report.runtime_timeout_events,
                service_json_string(&report.summary_line()),
                service_json_string_array(&report.failures)
            )
        })
        .unwrap_or_else(|| "null".to_owned())
}
