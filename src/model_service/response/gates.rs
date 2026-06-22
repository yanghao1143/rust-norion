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
            let json = format!(
                "{{\"passed\":{},\"checked_lines\":{},\"rust_check_events\":{},\"rust_check_passed\":{},\"rust_check_failed\":{},\"rust_check_feedback_updates\":{},\"rust_check_feedback_applied\":{},\"business_contract_events\":{},\"business_contract_event_passed\":{},\"business_contract_event_failed\":{},\"business_contract_event_missing_signals\":{},\"business_contract_event_protocol_leaks\":{},\"business_contract_event_substitutions\":{},\"business_contract_event_evasive_denials\":{},\"business_contract_event_raw_passed\":{},\"business_contract_event_raw_failed\":{},\"business_contract_event_response_normalized\":{},\"business_contract_event_sanitized\":{},\"business_contract_event_canonical_fallbacks\":{},\"runtime_error_events\":{},\"runtime_timeout_events\":{},\"self_evolution_admission_events\":{},\"self_evolution_admission_admitted\":{},\"self_evolution_admission_blocked\":{},\"self_evolution_admission_review_packets\":{},\"self_evolution_admission_evidence_ids\":{},\"self_evolution_admission_missing_review_packet_refs\":{},\"self_evolution_experiment_events\":{},\"self_evolution_experiment_admit\":{},\"self_evolution_experiment_hold\":{},\"self_evolution_experiment_reject\":{},\"self_evolution_experiment_rollback\":{},\"self_evolution_experiment_repeated\":{},\"self_evolution_experiment_conflicts\":{},\"self_evolution_experiment_rollback_replayable\":{},\"self_evolution_experiment_active_candidates\":{},\"self_evolution_experiment_write_allowed\":{},\"self_evolution_experiment_applied\":{},\"self_evolution_rollback_replay_events\":{},\"self_evolution_rollback_replay_items\":{},\"self_evolution_rollback_replay_replayable\":{},\"self_evolution_rollback_replay_blocked\":{},\"self_evolution_rollback_replay_all_replayable\":{},\"self_evolution_rollback_replay_rollback_anchor_ids\":{},\"self_evolution_rollback_replay_evidence_ids\":{},\"self_evolution_rollback_replay_active_candidates\":{},\"self_evolution_rollback_replay_item_write_allowed\":{},\"self_evolution_rollback_replay_item_applied\":{},\"self_evolution_rollback_replay_write_allowed\":{},\"self_evolution_rollback_replay_applied\":{},\"self_evolution_rollback_replay_gate_events\":{},\"self_evolution_rollback_replay_gate_admitted\":{},\"self_evolution_rollback_replay_gate_held\":{},\"self_evolution_rollback_replay_gate_review_packets\":{},\"self_evolution_rollback_replay_gate_review_evidence_ids\":{},\"self_evolution_rollback_replay_gate_missing_review_packet_refs\":{},\"self_evolution_rollback_replay_gate_items\":{},\"self_evolution_rollback_replay_gate_replayable\":{},\"self_evolution_rollback_replay_gate_blocked\":{},\"self_evolution_rollback_replay_gate_all_replayable\":{},\"self_evolution_rollback_replay_gate_rollback_anchor_ids\":{},\"self_evolution_rollback_replay_gate_evidence_ids\":{},\"self_evolution_rollback_replay_gate_active_candidates\":{},\"self_evolution_rollback_replay_gate_item_write_allowed\":{},\"self_evolution_rollback_replay_gate_item_applied\":{},\"self_evolution_rollback_replay_gate_plan_write_allowed\":{},\"self_evolution_rollback_replay_gate_plan_applied\":{},\"self_evolution_rollback_replay_gate_write_allowed\":{},\"self_evolution_rollback_replay_gate_applied\":{},\"self_evolution_operator_approval_events\":{},\"self_evolution_operator_approval_approved\":{},\"self_evolution_operator_approval_held\":{},\"self_evolution_operator_approval_review_packets\":{},\"self_evolution_operator_approval_evidence_ids\":{},\"self_evolution_operator_approval_rollback_anchor_ids\":{},\"self_evolution_operator_approval_content_digests\":{},\"self_evolution_operator_approval_source_report_schemas\":{},\"self_evolution_operator_approval_missing_review_packet_refs\":{},\"self_evolution_operator_approval_write_allowed\":{},\"self_evolution_operator_approval_applied\":{},\"improvement_corpus_events\":{},\"improvement_corpus_episodes\":{},\"improvement_corpus_active_adaptation\":{},\"improvement_corpus_compiler_passed\":{},\"improvement_corpus_test_passed\":{},\"improvement_corpus_benchmark_passed\":{},\"improvement_corpus_privacy_rejected\":{},\"improvement_corpus_secret_leaks\":{},\"adaptive_routing_events\":{},\"adaptive_routing_candidates\":{},\"adaptive_routing_include\":{},\"adaptive_routing_compress\":{},\"adaptive_routing_defer\":{},\"adaptive_routing_skip\":{},\"adaptive_routing_input_tokens\":{},\"adaptive_routing_retained_tokens\":{},\"adaptive_routing_saved_tokens\":{},\"task_hierarchy_events\":{},\"task_hierarchy_mutation_records\":{},\"task_hierarchy_route_pressure_milli\":{},\"task_hierarchy_compute_reduction_milli\":{},\"memory_admission_events\":{},\"memory_admission_candidates\":{},\"memory_admission_ready\":{},\"memory_admission_blocked\":{},\"memory_admission_admitted\":{},\"memory_admission_hold\":{},\"memory_admission_reject\":{},\"memory_admission_quarantine\":{},\"memory_admission_review_packets\":{},\"memory_admission_ledger_records\":{},\"memory_admission_ledger_authorized\":{},\"memory_admission_ledger_applied\":{},\"memory_admission_ledger_preview_only\":{},\"memory_admission_ledger_held\":{},\"memory_admission_ledger_rejected\":{},\"memory_admission_ledger_duplicate\":{},\"memory_admission_ledger_decayed\":{},\"memory_admission_ledger_merged\":{},\"memory_admission_ledger_rollback\":{},\"kv_fusion_events\":{},\"kv_fusion_candidates\":{},\"kv_fusion_fused\":{},\"kv_fusion_compressed\":{},\"kv_fusion_skipped\":{},\"kv_fusion_held\":{},\"kv_fusion_rejected\":{},\"kv_fusion_approval_blocked\":{},\"kv_fusion_input_tokens\":{},\"kv_fusion_retained_tokens\":{},\"kv_fusion_saved_tokens\":{},\"summary\":{},\"failures\":{}}}",
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
                report.self_evolution_experiment_events,
                report.self_evolution_experiment_admit,
                report.self_evolution_experiment_hold,
                report.self_evolution_experiment_reject,
                report.self_evolution_experiment_rollback,
                report.self_evolution_experiment_repeated,
                report.self_evolution_experiment_conflicts,
                report.self_evolution_experiment_rollback_replayable,
                report.self_evolution_experiment_active_candidates,
                report.self_evolution_experiment_write_allowed,
                report.self_evolution_experiment_applied,
                report.self_evolution_rollback_replay_events,
                report.self_evolution_rollback_replay_items,
                report.self_evolution_rollback_replay_replayable,
                report.self_evolution_rollback_replay_blocked,
                report.self_evolution_rollback_replay_all_replayable,
                report.self_evolution_rollback_replay_rollback_anchor_ids,
                report.self_evolution_rollback_replay_evidence_ids,
                report.self_evolution_rollback_replay_active_candidates,
                report.self_evolution_rollback_replay_item_write_allowed,
                report.self_evolution_rollback_replay_item_applied,
                report.self_evolution_rollback_replay_write_allowed,
                report.self_evolution_rollback_replay_applied,
                report.self_evolution_rollback_replay_gate_events,
                report.self_evolution_rollback_replay_gate_admitted,
                report.self_evolution_rollback_replay_gate_held,
                report.self_evolution_rollback_replay_gate_review_packets,
                report.self_evolution_rollback_replay_gate_review_evidence_ids,
                report.self_evolution_rollback_replay_gate_missing_review_packet_refs,
                report.self_evolution_rollback_replay_gate_items,
                report.self_evolution_rollback_replay_gate_replayable,
                report.self_evolution_rollback_replay_gate_blocked,
                report.self_evolution_rollback_replay_gate_all_replayable,
                report.self_evolution_rollback_replay_gate_rollback_anchor_ids,
                report.self_evolution_rollback_replay_gate_evidence_ids,
                report.self_evolution_rollback_replay_gate_active_candidates,
                report.self_evolution_rollback_replay_gate_item_write_allowed,
                report.self_evolution_rollback_replay_gate_item_applied,
                report.self_evolution_rollback_replay_gate_plan_write_allowed,
                report.self_evolution_rollback_replay_gate_plan_applied,
                report.self_evolution_rollback_replay_gate_write_allowed,
                report.self_evolution_rollback_replay_gate_applied,
                report.self_evolution_operator_approval_events,
                report.self_evolution_operator_approval_approved,
                report.self_evolution_operator_approval_held,
                report.self_evolution_operator_approval_review_packets,
                report.self_evolution_operator_approval_evidence_ids,
                report.self_evolution_operator_approval_rollback_anchor_ids,
                report.self_evolution_operator_approval_content_digests,
                report.self_evolution_operator_approval_source_report_schemas,
                report.self_evolution_operator_approval_missing_review_packet_refs,
                report.self_evolution_operator_approval_write_allowed,
                report.self_evolution_operator_approval_applied,
                report.improvement_corpus_events,
                report.improvement_corpus_episodes,
                report.improvement_corpus_active_adaptation,
                report.improvement_corpus_compiler_passed,
                report.improvement_corpus_test_passed,
                report.improvement_corpus_benchmark_passed,
                report.improvement_corpus_privacy_rejected,
                report.improvement_corpus_secret_leaks,
                report.adaptive_routing_events,
                report.adaptive_routing_candidates,
                report.adaptive_routing_include,
                report.adaptive_routing_compress,
                report.adaptive_routing_defer,
                report.adaptive_routing_skip,
                report.adaptive_routing_input_tokens,
                report.adaptive_routing_retained_tokens,
                report.adaptive_routing_saved_tokens,
                report.task_hierarchy_events,
                report.task_hierarchy_mutation_records,
                report.task_hierarchy_route_pressure_milli,
                report.task_hierarchy_compute_reduction_milli,
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
            );
            let apply_fields = format!(
                "\"self_evolution_rollback_replay_apply_events\":{},\"self_evolution_rollback_replay_apply_ready\":{},\"self_evolution_rollback_replay_apply_held\":{},\"self_evolution_rollback_replay_apply_items\":{},\"self_evolution_rollback_replay_apply_replayable\":{},\"self_evolution_rollback_replay_apply_blocked\":{},\"self_evolution_rollback_replay_apply_review_packets\":{},\"self_evolution_rollback_replay_apply_evidence_ids\":{},\"self_evolution_rollback_replay_apply_rollback_anchor_ids\":{},\"self_evolution_rollback_replay_apply_content_digests\":{},\"self_evolution_rollback_replay_apply_source_report_schemas\":{},\"self_evolution_rollback_replay_apply_missing_refs\":{},\"self_evolution_rollback_replay_apply_blocked_reasons\":{},\"self_evolution_rollback_replay_apply_write_allowed\":{},\"self_evolution_rollback_replay_apply_applied\":{}",
                report.self_evolution_rollback_replay_apply_events,
                report.self_evolution_rollback_replay_apply_ready,
                report.self_evolution_rollback_replay_apply_held,
                report.self_evolution_rollback_replay_apply_items,
                report.self_evolution_rollback_replay_apply_replayable,
                report.self_evolution_rollback_replay_apply_blocked,
                report.self_evolution_rollback_replay_apply_review_packets,
                report.self_evolution_rollback_replay_apply_evidence_ids,
                report.self_evolution_rollback_replay_apply_rollback_anchor_ids,
                report.self_evolution_rollback_replay_apply_content_digests,
                report.self_evolution_rollback_replay_apply_source_report_schemas,
                report.self_evolution_rollback_replay_apply_missing_refs,
                report.self_evolution_rollback_replay_apply_blocked_reasons,
                report.self_evolution_rollback_replay_apply_write_allowed,
                report.self_evolution_rollback_replay_apply_applied,
            );
            let json = json.replacen(
                "\"improvement_corpus_events\"",
                &format!("{apply_fields},\"improvement_corpus_events\""),
                1,
            );
            let operator_approval_counters = format!(
                "\"self_evolution_operator_approval_counters\":{}",
                report
                    .self_evolution_operator_approval_service_counters()
                    .json_object()
            );
            let experiment_counters = format!(
                "\"self_evolution_experiment_counters\":{{\"events\":{},\"admit\":{},\"hold\":{},\"reject\":{},\"rollback\":{},\"repeated\":{},\"conflicts\":{},\"rollback_replayable\":{},\"active_candidates\":{},\"write_allowed\":{},\"applied\":{}}}",
                report.self_evolution_experiment_events,
                report.self_evolution_experiment_admit,
                report.self_evolution_experiment_hold,
                report.self_evolution_experiment_reject,
                report.self_evolution_experiment_rollback,
                report.self_evolution_experiment_repeated,
                report.self_evolution_experiment_conflicts,
                report.self_evolution_experiment_rollback_replayable,
                report.self_evolution_experiment_active_candidates,
                report.self_evolution_experiment_write_allowed,
                report.self_evolution_experiment_applied,
            );
            let rollback_replay_counters = format!(
                "\"self_evolution_rollback_replay_counters\":{{\"events\":{},\"items\":{},\"replayable\":{},\"blocked\":{},\"all_replayable\":{},\"rollback_anchor_ids\":{},\"evidence_ids\":{},\"active_candidates\":{},\"item_write_allowed\":{},\"item_applied\":{},\"write_allowed\":{},\"applied\":{},\"gate_events\":{},\"gate_admitted\":{},\"gate_held\":{},\"gate_review_packets\":{},\"gate_review_evidence_ids\":{},\"gate_missing_review_packet_refs\":{},\"gate_item_write_allowed\":{},\"gate_item_applied\":{},\"gate_plan_write_allowed\":{},\"gate_plan_applied\":{},\"gate_write_allowed\":{},\"gate_applied\":{}}}",
                report.self_evolution_rollback_replay_events,
                report.self_evolution_rollback_replay_items,
                report.self_evolution_rollback_replay_replayable,
                report.self_evolution_rollback_replay_blocked,
                report.self_evolution_rollback_replay_all_replayable,
                report.self_evolution_rollback_replay_rollback_anchor_ids,
                report.self_evolution_rollback_replay_evidence_ids,
                report.self_evolution_rollback_replay_active_candidates,
                report.self_evolution_rollback_replay_item_write_allowed,
                report.self_evolution_rollback_replay_item_applied,
                report.self_evolution_rollback_replay_write_allowed,
                report.self_evolution_rollback_replay_applied,
                report.self_evolution_rollback_replay_gate_events,
                report.self_evolution_rollback_replay_gate_admitted,
                report.self_evolution_rollback_replay_gate_held,
                report.self_evolution_rollback_replay_gate_review_packets,
                report.self_evolution_rollback_replay_gate_review_evidence_ids,
                report.self_evolution_rollback_replay_gate_missing_review_packet_refs,
                report.self_evolution_rollback_replay_gate_item_write_allowed,
                report.self_evolution_rollback_replay_gate_item_applied,
                report.self_evolution_rollback_replay_gate_plan_write_allowed,
                report.self_evolution_rollback_replay_gate_plan_applied,
                report.self_evolution_rollback_replay_gate_write_allowed,
                report.self_evolution_rollback_replay_gate_applied,
            );
            let rollback_replay_apply_counters = format!(
                "\"self_evolution_rollback_replay_apply_counters\":{{\"events\":{},\"ready\":{},\"held\":{},\"items\":{},\"replayable\":{},\"blocked\":{},\"review_packets\":{},\"evidence_ids\":{},\"rollback_anchor_ids\":{},\"content_digests\":{},\"source_report_schemas\":{},\"missing_refs\":{},\"blocked_reasons\":{},\"write_allowed\":{},\"applied\":{}}}",
                report.self_evolution_rollback_replay_apply_events,
                report.self_evolution_rollback_replay_apply_ready,
                report.self_evolution_rollback_replay_apply_held,
                report.self_evolution_rollback_replay_apply_items,
                report.self_evolution_rollback_replay_apply_replayable,
                report.self_evolution_rollback_replay_apply_blocked,
                report.self_evolution_rollback_replay_apply_review_packets,
                report.self_evolution_rollback_replay_apply_evidence_ids,
                report.self_evolution_rollback_replay_apply_rollback_anchor_ids,
                report.self_evolution_rollback_replay_apply_content_digests,
                report.self_evolution_rollback_replay_apply_source_report_schemas,
                report.self_evolution_rollback_replay_apply_missing_refs,
                report.self_evolution_rollback_replay_apply_blocked_reasons,
                report.self_evolution_rollback_replay_apply_write_allowed,
                report.self_evolution_rollback_replay_apply_applied,
            );
            json.replacen(
                "\"summary\"",
                &format!(
                    "{experiment_counters},{rollback_replay_counters},{operator_approval_counters},{rollback_replay_apply_counters},\"summary\""
                ),
                1,
            )
        })
        .unwrap_or_else(|| "null".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_norion::SelfEvolutionOperatorApprovalServiceCounters;

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
            self_evolution_experiment_events: 4,
            self_evolution_experiment_admit: 1,
            self_evolution_experiment_hold: 1,
            self_evolution_experiment_reject: 1,
            self_evolution_experiment_rollback: 1,
            self_evolution_experiment_repeated: 1,
            self_evolution_experiment_conflicts: 1,
            self_evolution_experiment_rollback_replayable: 1,
            self_evolution_experiment_active_candidates: 0,
            self_evolution_experiment_write_allowed: 0,
            self_evolution_experiment_applied: 0,
            self_evolution_rollback_replay_events: 1,
            self_evolution_rollback_replay_items: 2,
            self_evolution_rollback_replay_replayable: 1,
            self_evolution_rollback_replay_blocked: 1,
            self_evolution_rollback_replay_all_replayable: 0,
            self_evolution_rollback_replay_rollback_anchor_ids: 3,
            self_evolution_rollback_replay_evidence_ids: 4,
            self_evolution_rollback_replay_active_candidates: 0,
            self_evolution_rollback_replay_item_write_allowed: 0,
            self_evolution_rollback_replay_item_applied: 0,
            self_evolution_rollback_replay_write_allowed: 0,
            self_evolution_rollback_replay_applied: 0,
            self_evolution_rollback_replay_gate_events: 2,
            self_evolution_rollback_replay_gate_admitted: 1,
            self_evolution_rollback_replay_gate_held: 1,
            self_evolution_rollback_replay_gate_review_packets: 2,
            self_evolution_rollback_replay_gate_review_evidence_ids: 3,
            self_evolution_rollback_replay_gate_missing_review_packet_refs: 0,
            self_evolution_rollback_replay_gate_items: 3,
            self_evolution_rollback_replay_gate_replayable: 2,
            self_evolution_rollback_replay_gate_blocked: 1,
            self_evolution_rollback_replay_gate_all_replayable: 1,
            self_evolution_rollback_replay_gate_rollback_anchor_ids: 4,
            self_evolution_rollback_replay_gate_evidence_ids: 5,
            self_evolution_rollback_replay_gate_active_candidates: 0,
            self_evolution_rollback_replay_gate_item_write_allowed: 0,
            self_evolution_rollback_replay_gate_item_applied: 0,
            self_evolution_rollback_replay_gate_plan_write_allowed: 0,
            self_evolution_rollback_replay_gate_plan_applied: 0,
            self_evolution_rollback_replay_gate_write_allowed: 0,
            self_evolution_rollback_replay_gate_applied: 0,
            self_evolution_operator_approval_events: 2,
            self_evolution_operator_approval_approved: 1,
            self_evolution_operator_approval_held: 1,
            self_evolution_operator_approval_review_packets: 2,
            self_evolution_operator_approval_evidence_ids: 3,
            self_evolution_operator_approval_rollback_anchor_ids: 4,
            self_evolution_operator_approval_content_digests: 5,
            self_evolution_operator_approval_source_report_schemas: 2,
            self_evolution_operator_approval_missing_review_packet_refs: 0,
            self_evolution_operator_approval_write_allowed: 0,
            self_evolution_operator_approval_applied: 0,
            self_evolution_rollback_replay_apply_events: 2,
            self_evolution_rollback_replay_apply_ready: 1,
            self_evolution_rollback_replay_apply_held: 1,
            self_evolution_rollback_replay_apply_items: 2,
            self_evolution_rollback_replay_apply_replayable: 2,
            self_evolution_rollback_replay_apply_blocked: 0,
            self_evolution_rollback_replay_apply_review_packets: 2,
            self_evolution_rollback_replay_apply_evidence_ids: 4,
            self_evolution_rollback_replay_apply_rollback_anchor_ids: 4,
            self_evolution_rollback_replay_apply_content_digests: 6,
            self_evolution_rollback_replay_apply_source_report_schemas: 4,
            self_evolution_rollback_replay_apply_missing_refs: 0,
            self_evolution_rollback_replay_apply_blocked_reasons: 1,
            self_evolution_rollback_replay_apply_write_allowed: 0,
            self_evolution_rollback_replay_apply_applied: 0,
            improvement_corpus_events: 0,
            improvement_corpus_episodes: 0,
            improvement_corpus_active_adaptation: 0,
            improvement_corpus_compiler_passed: 0,
            improvement_corpus_test_passed: 0,
            improvement_corpus_benchmark_passed: 0,
            improvement_corpus_privacy_rejected: 0,
            improvement_corpus_secret_leaks: 0,
            adaptive_routing_events: 2,
            adaptive_routing_candidates: 5,
            adaptive_routing_include: 2,
            adaptive_routing_compress: 1,
            adaptive_routing_defer: 1,
            adaptive_routing_skip: 1,
            adaptive_routing_input_tokens: 512,
            adaptive_routing_retained_tokens: 320,
            adaptive_routing_saved_tokens: 192,
            task_hierarchy_events: 2,
            task_hierarchy_mutation_records: 4,
            task_hierarchy_route_pressure_milli: 730,
            task_hierarchy_compute_reduction_milli: 280,
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
        assert!(json.contains("\"self_evolution_experiment_events\":4"));
        assert!(json.contains("\"self_evolution_experiment_admit\":1"));
        assert!(json.contains("\"self_evolution_experiment_rollback\":1"));
        assert!(json.contains("\"self_evolution_experiment_repeated\":1"));
        assert!(json.contains("\"self_evolution_experiment_conflicts\":1"));
        assert!(json.contains("\"self_evolution_experiment_write_allowed\":0"));
        assert!(json.contains("\"self_evolution_rollback_replay_events\":1"));
        assert!(json.contains("\"self_evolution_rollback_replay_items\":2"));
        assert!(json.contains("\"self_evolution_rollback_replay_replayable\":1"));
        assert!(json.contains("\"self_evolution_rollback_replay_blocked\":1"));
        assert!(json.contains("\"self_evolution_rollback_replay_all_replayable\":0"));
        assert!(json.contains("\"self_evolution_rollback_replay_rollback_anchor_ids\":3"));
        assert!(json.contains("\"self_evolution_rollback_replay_evidence_ids\":4"));
        assert!(json.contains("\"self_evolution_rollback_replay_active_candidates\":0"));
        assert!(json.contains("\"self_evolution_rollback_replay_item_write_allowed\":0"));
        assert!(json.contains("\"self_evolution_rollback_replay_item_applied\":0"));
        assert!(json.contains("\"self_evolution_rollback_replay_write_allowed\":0"));
        assert!(json.contains("\"self_evolution_rollback_replay_applied\":0"));
        assert!(json.contains("\"self_evolution_rollback_replay_gate_events\":2"));
        assert!(json.contains("\"self_evolution_rollback_replay_gate_admitted\":1"));
        assert!(json.contains("\"self_evolution_rollback_replay_gate_held\":1"));
        assert!(json.contains("\"self_evolution_rollback_replay_gate_review_packets\":2"));
        assert!(json.contains("\"self_evolution_rollback_replay_gate_review_evidence_ids\":3"));
        assert!(
            json.contains("\"self_evolution_rollback_replay_gate_missing_review_packet_refs\":0")
        );
        assert!(json.contains("\"self_evolution_rollback_replay_gate_items\":3"));
        assert!(json.contains("\"self_evolution_rollback_replay_gate_replayable\":2"));
        assert!(json.contains("\"self_evolution_rollback_replay_gate_blocked\":1"));
        assert!(json.contains("\"self_evolution_rollback_replay_gate_all_replayable\":1"));
        assert!(json.contains("\"self_evolution_rollback_replay_gate_rollback_anchor_ids\":4"));
        assert!(json.contains("\"self_evolution_rollback_replay_gate_evidence_ids\":5"));
        assert!(json.contains("\"self_evolution_rollback_replay_gate_active_candidates\":0"));
        assert!(json.contains("\"self_evolution_rollback_replay_gate_item_write_allowed\":0"));
        assert!(json.contains("\"self_evolution_rollback_replay_gate_item_applied\":0"));
        assert!(json.contains("\"self_evolution_rollback_replay_gate_plan_write_allowed\":0"));
        assert!(json.contains("\"self_evolution_rollback_replay_gate_plan_applied\":0"));
        assert!(json.contains("\"self_evolution_rollback_replay_gate_write_allowed\":0"));
        assert!(json.contains("\"self_evolution_rollback_replay_gate_applied\":0"));
        assert!(json.contains("\"self_evolution_operator_approval_events\":2"));
        assert!(json.contains("\"self_evolution_operator_approval_approved\":1"));
        assert!(json.contains("\"self_evolution_operator_approval_held\":1"));
        assert!(json.contains("\"self_evolution_operator_approval_review_packets\":2"));
        assert!(json.contains("\"self_evolution_operator_approval_evidence_ids\":3"));
        assert!(json.contains("\"self_evolution_operator_approval_rollback_anchor_ids\":4"));
        assert!(json.contains("\"self_evolution_operator_approval_content_digests\":5"));
        assert!(json.contains("\"self_evolution_operator_approval_source_report_schemas\":2"));
        assert!(json.contains("\"self_evolution_operator_approval_missing_review_packet_refs\":0"));
        assert!(json.contains("\"self_evolution_operator_approval_write_allowed\":0"));
        assert!(json.contains("\"self_evolution_operator_approval_applied\":0"));
        assert!(json.contains("\"self_evolution_rollback_replay_apply_events\":2"));
        assert!(json.contains("\"self_evolution_rollback_replay_apply_ready\":1"));
        assert!(json.contains("\"self_evolution_rollback_replay_apply_held\":1"));
        assert!(json.contains("\"self_evolution_rollback_replay_apply_items\":2"));
        assert!(json.contains("\"self_evolution_rollback_replay_apply_replayable\":2"));
        assert!(json.contains("\"self_evolution_rollback_replay_apply_blocked\":0"));
        assert!(json.contains("\"self_evolution_rollback_replay_apply_review_packets\":2"));
        assert!(json.contains("\"self_evolution_rollback_replay_apply_evidence_ids\":4"));
        assert!(json.contains("\"self_evolution_rollback_replay_apply_rollback_anchor_ids\":4"));
        assert!(json.contains("\"self_evolution_rollback_replay_apply_content_digests\":6"));
        assert!(json.contains("\"self_evolution_rollback_replay_apply_source_report_schemas\":4"));
        assert!(json.contains("\"self_evolution_rollback_replay_apply_missing_refs\":0"));
        assert!(json.contains("\"self_evolution_rollback_replay_apply_blocked_reasons\":1"));
        assert!(json.contains("\"self_evolution_rollback_replay_apply_write_allowed\":0"));
        assert!(json.contains("\"self_evolution_rollback_replay_apply_applied\":0"));
        assert!(json.contains("\"improvement_corpus_events\":0"));
        assert!(json.contains("\"adaptive_routing_events\":2"));
        assert!(json.contains("\"adaptive_routing_candidates\":5"));
        assert!(json.contains("\"adaptive_routing_saved_tokens\":192"));
        assert!(json.contains("\"task_hierarchy_events\":2"));
        assert!(json.contains("\"task_hierarchy_mutation_records\":4"));
        assert!(json.contains("\"task_hierarchy_compute_reduction_milli\":280"));
        assert!(json.contains("\"memory_admission_events\":1"));
        assert!(json.contains("\"memory_admission_candidates\":3"));
        assert!(json.contains("\"memory_admission_ledger_records\":3"));
        assert!(json.contains("\"memory_admission_ledger_preview_only\":1"));
        assert!(json.contains("\"kv_fusion_events\":1"));
        assert!(json.contains("\"kv_fusion_candidates\":3"));
        assert!(json.contains("\"kv_fusion_saved_tokens\":100"));
        assert!(json.contains("self_evolution_admission_events=2"));
        assert!(json.contains("self_evolution_admission_review_packets=2"));
        assert!(json.contains("self_evolution_experiment_events=4"));
        assert!(json.contains("self_evolution_experiment_rollback=1"));
        assert!(json.contains("self_evolution_rollback_replay_events=1"));
        assert!(json.contains("self_evolution_rollback_replay_blocked=1"));
        assert!(json.contains("self_evolution_rollback_replay_gate_events=2"));
        assert!(json.contains("self_evolution_rollback_replay_gate_held=1"));
        assert!(json.contains("self_evolution_rollback_replay_gate_review_packets=2"));
        assert!(json.contains("self_evolution_operator_approval_events=2"));
        assert!(json.contains("self_evolution_operator_approval_held=1"));
        assert!(json.contains("self_evolution_operator_approval_review_packets=2"));
        assert!(json.contains("self_evolution_rollback_replay_apply_events=2"));
        assert!(json.contains("self_evolution_rollback_replay_apply_ready=1"));
        assert!(json.contains("adaptive_routing_candidates=5"));
        assert!(json.contains("task_hierarchy_mutation_records=4"));
        assert!(json.contains("memory_admission_ledger_records=3"));
        assert!(json.contains("kv_fusion_saved_tokens=100"));
    }

    #[test]
    fn trace_gate_service_json_exposes_operator_approval_counter_object() {
        let report = TraceSchemaGateReport {
            passed: true,
            checked_lines: 2,
            self_evolution_operator_approval_events: 2,
            self_evolution_operator_approval_approved: 1,
            self_evolution_operator_approval_held: 1,
            self_evolution_operator_approval_review_packets: 3,
            self_evolution_operator_approval_evidence_ids: 4,
            self_evolution_operator_approval_rollback_anchor_ids: 5,
            self_evolution_operator_approval_content_digests: 6,
            self_evolution_operator_approval_source_report_schemas: 2,
            self_evolution_operator_approval_missing_review_packet_refs: 1,
            self_evolution_operator_approval_write_allowed: 0,
            self_evolution_operator_approval_applied: 0,
            failures: Vec::new(),
            ..TraceSchemaGateReport::default()
        };

        let json = option_trace_gate_service_json(Some(&report));
        let counters: SelfEvolutionOperatorApprovalServiceCounters =
            report.self_evolution_operator_approval_service_counters();

        assert!(counters.review_required);
        assert!(counters.blocked);
        assert!(!counters.approval_ready);
        assert!(json.contains("\"self_evolution_operator_approval_counters\":{"));
        assert!(json.contains("\"trace_gate_passed\":true"));
        assert!(json.contains("\"data_present\":true"));
        assert!(json.contains("\"approval_ready\":false"));
        assert!(json.contains("\"review_required\":true"));
        assert!(json.contains("\"blocked\":true"));
        assert!(json.contains("\"events\":2"));
        assert!(json.contains("\"approved\":1"));
        assert!(json.contains("\"held\":1"));
        assert!(json.contains("\"review_packets\":3"));
        assert!(json.contains("\"evidence_ids\":4"));
        assert!(json.contains("\"rollback_anchor_ids\":5"));
        assert!(json.contains("\"content_digests\":6"));
        assert!(json.contains("\"source_report_schemas\":2"));
        assert!(json.contains("\"missing_review_packet_refs\":1"));
        assert!(json.contains("\"write_allowed\":0"));
        assert!(json.contains("\"applied\":0"));
        assert!(json.contains("\"activation_allowed\":false"));
        assert!(json.contains("\"memory_write_allowed\":false"));
        assert!(json.contains("\"genome_write_allowed\":false"));
        assert!(json.contains("\"kv_write_allowed\":false"));
        assert!(json.contains("\"validation_failures\":["));
        assert!(json.contains("self_evolution_operator_approval_missing_review_packet_refs"));
        assert!(json.contains("\"summary\":"));
    }

    #[test]
    fn trace_gate_service_json_exposes_self_evolution_gate_counter_objects() {
        let report = TraceSchemaGateReport {
            passed: true,
            checked_lines: 5,
            self_evolution_experiment_events: 4,
            self_evolution_experiment_admit: 1,
            self_evolution_experiment_hold: 1,
            self_evolution_experiment_reject: 1,
            self_evolution_experiment_rollback: 1,
            self_evolution_experiment_repeated: 1,
            self_evolution_experiment_conflicts: 1,
            self_evolution_experiment_rollback_replayable: 1,
            self_evolution_experiment_active_candidates: 0,
            self_evolution_experiment_write_allowed: 0,
            self_evolution_experiment_applied: 0,
            self_evolution_rollback_replay_events: 1,
            self_evolution_rollback_replay_items: 2,
            self_evolution_rollback_replay_replayable: 2,
            self_evolution_rollback_replay_blocked: 0,
            self_evolution_rollback_replay_all_replayable: 1,
            self_evolution_rollback_replay_rollback_anchor_ids: 2,
            self_evolution_rollback_replay_evidence_ids: 3,
            self_evolution_rollback_replay_active_candidates: 0,
            self_evolution_rollback_replay_item_write_allowed: 0,
            self_evolution_rollback_replay_item_applied: 0,
            self_evolution_rollback_replay_write_allowed: 0,
            self_evolution_rollback_replay_applied: 0,
            self_evolution_rollback_replay_gate_events: 1,
            self_evolution_rollback_replay_gate_admitted: 1,
            self_evolution_rollback_replay_gate_held: 0,
            self_evolution_rollback_replay_gate_review_packets: 1,
            self_evolution_rollback_replay_gate_review_evidence_ids: 4,
            self_evolution_rollback_replay_gate_missing_review_packet_refs: 0,
            self_evolution_rollback_replay_gate_item_write_allowed: 0,
            self_evolution_rollback_replay_gate_item_applied: 0,
            self_evolution_rollback_replay_gate_plan_write_allowed: 0,
            self_evolution_rollback_replay_gate_plan_applied: 0,
            self_evolution_rollback_replay_gate_write_allowed: 0,
            self_evolution_rollback_replay_gate_applied: 0,
            self_evolution_rollback_replay_apply_events: 2,
            self_evolution_rollback_replay_apply_ready: 1,
            self_evolution_rollback_replay_apply_held: 1,
            self_evolution_rollback_replay_apply_items: 2,
            self_evolution_rollback_replay_apply_replayable: 2,
            self_evolution_rollback_replay_apply_blocked: 0,
            self_evolution_rollback_replay_apply_review_packets: 1,
            self_evolution_rollback_replay_apply_evidence_ids: 4,
            self_evolution_rollback_replay_apply_rollback_anchor_ids: 2,
            self_evolution_rollback_replay_apply_content_digests: 3,
            self_evolution_rollback_replay_apply_source_report_schemas: 2,
            self_evolution_rollback_replay_apply_missing_refs: 0,
            self_evolution_rollback_replay_apply_blocked_reasons: 1,
            self_evolution_rollback_replay_apply_write_allowed: 0,
            self_evolution_rollback_replay_apply_applied: 0,
            failures: Vec::new(),
            ..TraceSchemaGateReport::default()
        };

        let json = option_trace_gate_service_json(Some(&report));

        assert!(json.contains(
            "\"self_evolution_experiment_counters\":{\"events\":4,\"admit\":1,\"hold\":1,\"reject\":1,\"rollback\":1"
        ));
        assert!(json.contains("\"repeated\":1"));
        assert!(json.contains("\"conflicts\":1"));
        assert!(json.contains("\"rollback_replayable\":1"));
        assert!(json.contains("\"active_candidates\":0"));
        assert!(json.contains("\"self_evolution_rollback_replay_counters\":{"));
        assert!(json.contains("\"items\":2"));
        assert!(json.contains("\"all_replayable\":1"));
        assert!(json.contains("\"gate_admitted\":1"));
        assert!(json.contains("\"gate_held\":0"));
        assert!(json.contains("\"gate_plan_write_allowed\":0"));
        assert!(json.contains("\"self_evolution_rollback_replay_apply_counters\":{"));
        assert!(json.contains("\"ready\":1"));
        assert!(json.contains("\"held\":1"));
        assert!(json.contains("\"missing_refs\":0"));
        assert!(json.contains("\"blocked_reasons\":1"));
        assert!(json.contains("\"write_allowed\":0"));
        assert!(json.contains("\"applied\":0"));
    }
}
