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
                "{{\"passed\":{},\"checked_lines\":{},\"rust_check_events\":{},\"rust_check_passed\":{},\"rust_check_failed\":{},\"rust_check_feedback_updates\":{},\"rust_check_feedback_applied\":{},\"business_contract_events\":{},\"business_contract_event_passed\":{},\"business_contract_event_failed\":{},\"business_contract_event_missing_signals\":{},\"business_contract_event_protocol_leaks\":{},\"business_contract_event_substitutions\":{},\"business_contract_event_evasive_denials\":{},\"business_contract_event_raw_passed\":{},\"business_contract_event_raw_failed\":{},\"business_contract_event_response_normalized\":{},\"business_contract_event_sanitized\":{},\"business_contract_event_canonical_fallbacks\":{},\"runtime_error_events\":{},\"runtime_timeout_events\":{},\"self_evolution_admission_events\":{},\"self_evolution_admission_admitted\":{},\"self_evolution_admission_blocked\":{},\"self_evolution_admission_review_packets\":{},\"self_evolution_admission_evidence_ids\":{},\"self_evolution_admission_missing_review_packet_refs\":{},\"improvement_corpus_events\":{},\"improvement_corpus_episodes\":{},\"improvement_corpus_active_adaptation\":{},\"improvement_corpus_compiler_passed\":{},\"improvement_corpus_test_passed\":{},\"improvement_corpus_benchmark_passed\":{},\"improvement_corpus_privacy_rejected\":{},\"improvement_corpus_secret_leaks\":{},\"memory_admission_events\":{},\"memory_admission_candidates\":{},\"memory_admission_ready\":{},\"memory_admission_blocked\":{},\"memory_admission_admitted\":{},\"memory_admission_hold\":{},\"memory_admission_reject\":{},\"memory_admission_quarantine\":{},\"memory_admission_review_packets\":{},\"memory_admission_ledger_records\":{},\"memory_admission_ledger_authorized\":{},\"memory_admission_ledger_applied\":{},\"memory_admission_ledger_preview_only\":{},\"memory_admission_ledger_held\":{},\"memory_admission_ledger_rejected\":{},\"memory_admission_ledger_duplicate\":{},\"memory_admission_ledger_decayed\":{},\"memory_admission_ledger_merged\":{},\"memory_admission_ledger_rollback\":{},\"kv_fusion_events\":{},\"kv_fusion_candidates\":{},\"kv_fusion_fused\":{},\"kv_fusion_compressed\":{},\"kv_fusion_skipped\":{},\"kv_fusion_held\":{},\"kv_fusion_rejected\":{},\"kv_fusion_approval_blocked\":{},\"kv_fusion_input_tokens\":{},\"kv_fusion_retained_tokens\":{},\"kv_fusion_saved_tokens\":{},\"summary\":{},\"failures\":{}}}",
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
                report.improvement_corpus_events,
                report.improvement_corpus_episodes,
                report.improvement_corpus_active_adaptation,
                report.improvement_corpus_compiler_passed,
                report.improvement_corpus_test_passed,
                report.improvement_corpus_benchmark_passed,
                report.improvement_corpus_privacy_rejected,
                report.improvement_corpus_secret_leaks,
                report.memory_admission_events,
                report.memory_admission_candidates,
                report.memory_admission_ready,
                report.memory_admission_blocked,
                report.memory_admission_admitted,
                report.memory_admission_hold,
                report.memory_admission_reject,
                report.memory_admission_quarantine,
                report.memory_admission_review_packets,
                report.memory_admission_ledger_records,
                report.memory_admission_ledger_authorized,
                report.memory_admission_ledger_applied,
                report.memory_admission_ledger_preview_only,
                report.memory_admission_ledger_held,
                report.memory_admission_ledger_rejected,
                report.memory_admission_ledger_duplicate,
                report.memory_admission_ledger_decayed,
                report.memory_admission_ledger_merged,
                report.memory_admission_ledger_rollback,
                report.kv_fusion_events,
                report.kv_fusion_candidates,
                report.kv_fusion_fused,
                report.kv_fusion_compressed,
                report.kv_fusion_skipped,
                report.kv_fusion_held,
                report.kv_fusion_rejected,
                report.kv_fusion_approval_blocked,
                report.kv_fusion_input_tokens,
                report.kv_fusion_retained_tokens,
                report.kv_fusion_saved_tokens,
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
            improvement_corpus_events: 0,
            improvement_corpus_episodes: 0,
            improvement_corpus_active_adaptation: 0,
            improvement_corpus_compiler_passed: 0,
            improvement_corpus_test_passed: 0,
            improvement_corpus_benchmark_passed: 0,
            improvement_corpus_privacy_rejected: 0,
            improvement_corpus_secret_leaks: 0,
            memory_admission_events: 1,
            memory_admission_candidates: 3,
            memory_admission_ready: 1,
            memory_admission_blocked: 2,
            memory_admission_admitted: 0,
            memory_admission_hold: 1,
            memory_admission_reject: 1,
            memory_admission_quarantine: 0,
            memory_admission_review_packets: 3,
            memory_admission_ledger_records: 3,
            memory_admission_ledger_authorized: 0,
            memory_admission_ledger_applied: 0,
            memory_admission_ledger_preview_only: 1,
            memory_admission_ledger_held: 1,
            memory_admission_ledger_rejected: 1,
            memory_admission_ledger_duplicate: 0,
            memory_admission_ledger_decayed: 0,
            memory_admission_ledger_merged: 0,
            memory_admission_ledger_rollback: 0,
            kv_fusion_events: 1,
            kv_fusion_candidates: 3,
            kv_fusion_fused: 1,
            kv_fusion_compressed: 1,
            kv_fusion_skipped: 1,
            kv_fusion_held: 0,
            kv_fusion_rejected: 0,
            kv_fusion_approval_blocked: 0,
            kv_fusion_input_tokens: 240,
            kv_fusion_retained_tokens: 140,
            kv_fusion_saved_tokens: 100,
            failures: Vec::new(),
            ..TraceSchemaGateReport::default()
        };

        let json = option_trace_gate_service_json(Some(&report));

        assert!(json.contains("\"self_evolution_admission_events\":2"));
        assert!(json.contains("\"self_evolution_admission_admitted\":1"));
        assert!(json.contains("\"self_evolution_admission_blocked\":1"));
        assert!(json.contains("\"self_evolution_admission_review_packets\":2"));
        assert!(json.contains("\"self_evolution_admission_evidence_ids\":4"));
        assert!(json.contains("\"self_evolution_admission_missing_review_packet_refs\":0"));
        assert!(json.contains("\"improvement_corpus_events\":0"));
        assert!(json.contains("\"memory_admission_events\":1"));
        assert!(json.contains("\"memory_admission_candidates\":3"));
        assert!(json.contains("\"memory_admission_ledger_records\":3"));
        assert!(json.contains("\"memory_admission_ledger_preview_only\":1"));
        assert!(json.contains("\"kv_fusion_events\":1"));
        assert!(json.contains("\"kv_fusion_candidates\":3"));
        assert!(json.contains("\"kv_fusion_saved_tokens\":100"));
        assert!(json.contains("self_evolution_admission_events=2"));
        assert!(json.contains("self_evolution_admission_review_packets=2"));
        assert!(json.contains("memory_admission_ledger_records=3"));
        assert!(json.contains("kv_fusion_saved_tokens=100"));
    }
}
