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
                "{{\"passed\":{},\"checked_lines\":{},\"rust_check_events\":{},\"rust_check_passed\":{},\"rust_check_failed\":{},\"rust_check_feedback_updates\":{},\"rust_check_feedback_applied\":{},\"business_contract_events\":{},\"business_contract_event_passed\":{},\"business_contract_event_failed\":{},\"business_contract_event_missing_signals\":{},\"business_contract_event_protocol_leaks\":{},\"business_contract_event_substitutions\":{},\"business_contract_event_evasive_denials\":{},\"business_contract_event_raw_passed\":{},\"business_contract_event_raw_failed\":{},\"business_contract_event_response_normalized\":{},\"business_contract_event_sanitized\":{},\"business_contract_event_canonical_fallbacks\":{},\"runtime_error_events\":{},\"runtime_timeout_events\":{},\"self_evolution_admission_events\":{},\"self_evolution_admission_admitted\":{},\"self_evolution_admission_blocked\":{},\"self_evolution_admission_review_packets\":{},\"self_evolution_admission_evidence_ids\":{},\"self_evolution_admission_missing_review_packet_refs\":{},\"summary\":{},\"failures\":{}}}",
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
                report.self_evolution_admission_events,
                report.self_evolution_admission_admitted,
                report.self_evolution_admission_blocked,
                report.self_evolution_admission_review_packets,
                report.self_evolution_admission_evidence_ids,
                report.self_evolution_admission_missing_review_packet_refs,
                service_json_string(&report.summary_line()),
                service_json_string_array(&report.failures)
            )
        })
        .unwrap_or_else(|| "null".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_gate_service_json_exposes_self_evolution_admission_counts() {
        let report = TraceSchemaGateReport {
            passed: true,
            checked_lines: 4,
            rust_check_events: 0,
            rust_check_passed: 0,
            rust_check_failed: 0,
            rust_check_feedback_updates: 0,
            rust_check_feedback_applied: 0,
            business_contract_events: 0,
            business_contract_event_passed: 0,
            business_contract_event_failed: 0,
            business_contract_event_missing_signals: 0,
            business_contract_event_protocol_leaks: 0,
            business_contract_event_substitutions: 0,
            business_contract_event_evasive_denials: 0,
            business_contract_event_raw_passed: 0,
            business_contract_event_raw_failed: 0,
            business_contract_event_response_normalized: 0,
            business_contract_event_sanitized: 0,
            business_contract_event_canonical_fallbacks: 0,
            runtime_error_events: 0,
            runtime_timeout_events: 0,
            self_evolution_admission_events: 2,
            self_evolution_admission_admitted: 1,
            self_evolution_admission_blocked: 1,
            self_evolution_admission_review_packets: 2,
            self_evolution_admission_evidence_ids: 4,
            self_evolution_admission_missing_review_packet_refs: 0,
            failures: Vec::new(),
        };

        let json = option_trace_gate_service_json(Some(&report));

        assert!(json.contains("\"self_evolution_admission_events\":2"));
        assert!(json.contains("\"self_evolution_admission_admitted\":1"));
        assert!(json.contains("\"self_evolution_admission_blocked\":1"));
        assert!(json.contains("\"self_evolution_admission_review_packets\":2"));
        assert!(json.contains("\"self_evolution_admission_evidence_ids\":4"));
        assert!(json.contains("\"self_evolution_admission_missing_review_packet_refs\":0"));
        assert!(json.contains("self_evolution_admission_events=2"));
        assert!(json.contains("self_evolution_admission_review_packets=2"));
    }
}
