use std::fs;
use std::io;
use std::path::Path;

use super::evaluate_trace_schema_line;
use super::fields::{
    extract_json_bool_field, extract_json_f32_field, extract_json_string_array_field,
    extract_json_string_field, extract_json_usize_field, extract_last_json_string_array_field,
    json_object_after_field, trace_note_bool,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TraceSchemaGateReport {
    pub passed: bool,
    pub checked_lines: usize,
    pub rust_check_events: usize,
    pub rust_check_passed: usize,
    pub rust_check_failed: usize,
    pub rust_check_feedback_updates: usize,
    pub rust_check_feedback_applied: usize,
    pub business_contract_events: usize,
    pub business_contract_event_passed: usize,
    pub business_contract_event_failed: usize,
    pub business_contract_event_missing_signals: usize,
    pub business_contract_event_protocol_leaks: usize,
    pub business_contract_event_substitutions: usize,
    pub business_contract_event_evasive_denials: usize,
    pub business_contract_event_raw_passed: usize,
    pub business_contract_event_raw_failed: usize,
    pub business_contract_event_response_normalized: usize,
    pub business_contract_event_sanitized: usize,
    pub business_contract_event_canonical_fallbacks: usize,
    pub runtime_error_events: usize,
    pub runtime_timeout_events: usize,
    pub self_evolution_admission_events: usize,
    pub self_evolution_admission_admitted: usize,
    pub self_evolution_admission_blocked: usize,
    pub self_evolution_admission_review_packets: usize,
    pub self_evolution_admission_evidence_ids: usize,
    pub self_evolution_admission_missing_review_packet_refs: usize,
    pub self_evolution_experiment_events: usize,
    pub self_evolution_experiment_admit: usize,
    pub self_evolution_experiment_hold: usize,
    pub self_evolution_experiment_reject: usize,
    pub self_evolution_experiment_rollback: usize,
    pub self_evolution_experiment_repeated: usize,
    pub self_evolution_experiment_conflicts: usize,
    pub self_evolution_experiment_rollback_replayable: usize,
    pub self_evolution_experiment_active_candidates: usize,
    pub self_evolution_experiment_write_allowed: usize,
    pub self_evolution_experiment_applied: usize,
    pub self_evolution_rollback_replay_events: usize,
    pub self_evolution_rollback_replay_items: usize,
    pub self_evolution_rollback_replay_replayable: usize,
    pub self_evolution_rollback_replay_blocked: usize,
    pub self_evolution_rollback_replay_all_replayable: usize,
    pub self_evolution_rollback_replay_rollback_anchor_ids: usize,
    pub self_evolution_rollback_replay_evidence_ids: usize,
    pub self_evolution_rollback_replay_active_candidates: usize,
    pub self_evolution_rollback_replay_item_write_allowed: usize,
    pub self_evolution_rollback_replay_item_applied: usize,
    pub self_evolution_rollback_replay_write_allowed: usize,
    pub self_evolution_rollback_replay_applied: usize,
    pub self_evolution_rollback_replay_gate_events: usize,
    pub self_evolution_rollback_replay_gate_admitted: usize,
    pub self_evolution_rollback_replay_gate_held: usize,
    pub self_evolution_rollback_replay_gate_review_packets: usize,
    pub self_evolution_rollback_replay_gate_review_evidence_ids: usize,
    pub self_evolution_rollback_replay_gate_missing_review_packet_refs: usize,
    pub self_evolution_rollback_replay_gate_items: usize,
    pub self_evolution_rollback_replay_gate_replayable: usize,
    pub self_evolution_rollback_replay_gate_blocked: usize,
    pub self_evolution_rollback_replay_gate_all_replayable: usize,
    pub self_evolution_rollback_replay_gate_rollback_anchor_ids: usize,
    pub self_evolution_rollback_replay_gate_evidence_ids: usize,
    pub self_evolution_rollback_replay_gate_active_candidates: usize,
    pub self_evolution_rollback_replay_gate_item_write_allowed: usize,
    pub self_evolution_rollback_replay_gate_item_applied: usize,
    pub self_evolution_rollback_replay_gate_plan_write_allowed: usize,
    pub self_evolution_rollback_replay_gate_plan_applied: usize,
    pub self_evolution_rollback_replay_gate_write_allowed: usize,
    pub self_evolution_rollback_replay_gate_applied: usize,
    pub self_evolution_operator_approval_events: usize,
    pub self_evolution_operator_approval_approved: usize,
    pub self_evolution_operator_approval_held: usize,
    pub self_evolution_operator_approval_review_packets: usize,
    pub self_evolution_operator_approval_evidence_ids: usize,
    pub self_evolution_operator_approval_rollback_anchor_ids: usize,
    pub self_evolution_operator_approval_content_digests: usize,
    pub self_evolution_operator_approval_source_report_schemas: usize,
    pub self_evolution_operator_approval_missing_review_packet_refs: usize,
    pub self_evolution_operator_approval_write_allowed: usize,
    pub self_evolution_operator_approval_applied: usize,
    pub self_evolution_rollback_replay_apply_events: usize,
    pub self_evolution_rollback_replay_apply_ready: usize,
    pub self_evolution_rollback_replay_apply_held: usize,
    pub self_evolution_rollback_replay_apply_items: usize,
    pub self_evolution_rollback_replay_apply_replayable: usize,
    pub self_evolution_rollback_replay_apply_blocked: usize,
    pub self_evolution_rollback_replay_apply_review_packets: usize,
    pub self_evolution_rollback_replay_apply_evidence_ids: usize,
    pub self_evolution_rollback_replay_apply_rollback_anchor_ids: usize,
    pub self_evolution_rollback_replay_apply_content_digests: usize,
    pub self_evolution_rollback_replay_apply_source_report_schemas: usize,
    pub self_evolution_rollback_replay_apply_missing_refs: usize,
    pub self_evolution_rollback_replay_apply_blocked_reasons: usize,
    pub self_evolution_rollback_replay_apply_write_allowed: usize,
    pub self_evolution_rollback_replay_apply_applied: usize,
    pub self_evolving_memory_store_events: usize,
    pub self_evolving_memory_store_retrieval_events: usize,
    pub self_evolving_memory_store_maintenance_events: usize,
    pub self_evolving_memory_store_admission_preview_events: usize,
    pub self_evolving_memory_store_contexts: usize,
    pub self_evolving_memory_store_maintenance_actions: usize,
    pub self_evolving_memory_store_admission_candidates: usize,
    pub self_evolving_memory_store_write_allowed: usize,
    pub self_evolving_memory_store_durable_write_allowed: usize,
    pub self_evolving_memory_store_applied: usize,
    pub self_evolving_memory_store_applied_to_disk: usize,
    pub improvement_corpus_events: usize,
    pub improvement_corpus_episodes: usize,
    pub improvement_corpus_active_adaptation: usize,
    pub improvement_corpus_compiler_passed: usize,
    pub improvement_corpus_test_passed: usize,
    pub improvement_corpus_benchmark_passed: usize,
    pub improvement_corpus_privacy_rejected: usize,
    pub improvement_corpus_secret_leaks: usize,
    pub adaptive_routing_events: usize,
    pub adaptive_routing_candidates: usize,
    pub adaptive_routing_include: usize,
    pub adaptive_routing_compress: usize,
    pub adaptive_routing_defer: usize,
    pub adaptive_routing_skip: usize,
    pub adaptive_routing_input_tokens: usize,
    pub adaptive_routing_retained_tokens: usize,
    pub adaptive_routing_saved_tokens: usize,
    pub task_hierarchy_events: usize,
    pub task_hierarchy_mutation_records: usize,
    pub task_hierarchy_route_pressure_milli: usize,
    pub task_hierarchy_compute_reduction_milli: usize,
    pub memory_admission_events: usize,
    pub memory_admission_candidates: usize,
    pub memory_admission_ready: usize,
    pub memory_admission_blocked: usize,
    pub memory_admission_admitted: usize,
    pub memory_admission_hold: usize,
    pub memory_admission_reject: usize,
    pub memory_admission_quarantine: usize,
    pub memory_admission_review_packets: usize,
    pub memory_admission_ledger_records: usize,
    pub memory_admission_ledger_authorized: usize,
    pub memory_admission_ledger_applied: usize,
    pub memory_admission_ledger_preview_only: usize,
    pub memory_admission_ledger_held: usize,
    pub memory_admission_ledger_rejected: usize,
    pub memory_admission_ledger_duplicate: usize,
    pub memory_admission_ledger_decayed: usize,
    pub memory_admission_ledger_merged: usize,
    pub memory_admission_ledger_rollback: usize,
    pub kv_fusion_events: usize,
    pub kv_fusion_candidates: usize,
    pub kv_fusion_fused: usize,
    pub kv_fusion_compressed: usize,
    pub kv_fusion_skipped: usize,
    pub kv_fusion_held: usize,
    pub kv_fusion_rejected: usize,
    pub kv_fusion_approval_blocked: usize,
    pub kv_fusion_input_tokens: usize,
    pub kv_fusion_retained_tokens: usize,
    pub kv_fusion_saved_tokens: usize,
    pub failures: Vec<String>,
}

impl TraceSchemaGateReport {
    pub fn summary_line(&self) -> String {
        let base = format!(
            "trace_schema_gate: passed={} lines={} failures={} rust_check_events={} rust_check_passed={} rust_check_failed={} rust_check_feedback_updates={} rust_check_feedback_applied={} business_contract_events={} business_contract_event_passed={} business_contract_event_failed={} business_contract_event_missing_signals={} business_contract_event_protocol_leaks={} business_contract_event_substitutions={} business_contract_event_evasive_denials={} business_contract_event_raw_passed={} business_contract_event_raw_failed={} business_contract_event_response_normalized={} business_contract_event_sanitized={} business_contract_event_canonical_fallbacks={} runtime_error_events={} runtime_timeout_events={} self_evolution_admission_events={} self_evolution_admission_admitted={} self_evolution_admission_blocked={} self_evolution_admission_review_packets={} self_evolution_admission_evidence_ids={} self_evolution_admission_missing_review_packet_refs={} self_evolution_experiment_events={} self_evolution_experiment_admit={} self_evolution_experiment_hold={} self_evolution_experiment_reject={} self_evolution_experiment_rollback={} self_evolution_experiment_repeated={} self_evolution_experiment_conflicts={} self_evolution_experiment_rollback_replayable={} self_evolution_experiment_active_candidates={} self_evolution_experiment_write_allowed={} self_evolution_experiment_applied={} self_evolution_rollback_replay_events={} self_evolution_rollback_replay_items={} self_evolution_rollback_replay_replayable={} self_evolution_rollback_replay_blocked={} self_evolution_rollback_replay_all_replayable={} self_evolution_rollback_replay_rollback_anchor_ids={} self_evolution_rollback_replay_evidence_ids={} self_evolution_rollback_replay_active_candidates={} self_evolution_rollback_replay_item_write_allowed={} self_evolution_rollback_replay_item_applied={} self_evolution_rollback_replay_write_allowed={} self_evolution_rollback_replay_applied={} self_evolution_rollback_replay_gate_events={} self_evolution_rollback_replay_gate_admitted={} self_evolution_rollback_replay_gate_held={} self_evolution_rollback_replay_gate_review_packets={} self_evolution_rollback_replay_gate_review_evidence_ids={} self_evolution_rollback_replay_gate_missing_review_packet_refs={} self_evolution_rollback_replay_gate_items={} self_evolution_rollback_replay_gate_replayable={} self_evolution_rollback_replay_gate_blocked={} self_evolution_rollback_replay_gate_all_replayable={} self_evolution_rollback_replay_gate_rollback_anchor_ids={} self_evolution_rollback_replay_gate_evidence_ids={} self_evolution_rollback_replay_gate_active_candidates={} self_evolution_rollback_replay_gate_item_write_allowed={} self_evolution_rollback_replay_gate_item_applied={} self_evolution_rollback_replay_gate_plan_write_allowed={} self_evolution_rollback_replay_gate_plan_applied={} self_evolution_rollback_replay_gate_write_allowed={} self_evolution_rollback_replay_gate_applied={} self_evolution_operator_approval_events={} self_evolution_operator_approval_approved={} self_evolution_operator_approval_held={} self_evolution_operator_approval_review_packets={} self_evolution_operator_approval_evidence_ids={} self_evolution_operator_approval_rollback_anchor_ids={} self_evolution_operator_approval_content_digests={} self_evolution_operator_approval_source_report_schemas={} self_evolution_operator_approval_missing_review_packet_refs={} self_evolution_operator_approval_write_allowed={} self_evolution_operator_approval_applied={} improvement_corpus_events={} improvement_corpus_episodes={} improvement_corpus_active_adaptation={} improvement_corpus_compiler_passed={} improvement_corpus_test_passed={} improvement_corpus_benchmark_passed={} improvement_corpus_privacy_rejected={} improvement_corpus_secret_leaks={} adaptive_routing_events={} adaptive_routing_candidates={} adaptive_routing_include={} adaptive_routing_compress={} adaptive_routing_defer={} adaptive_routing_skip={} adaptive_routing_input_tokens={} adaptive_routing_retained_tokens={} adaptive_routing_saved_tokens={} task_hierarchy_events={} task_hierarchy_mutation_records={} task_hierarchy_route_pressure_milli={} task_hierarchy_compute_reduction_milli={} memory_admission_events={} memory_admission_candidates={} memory_admission_ready={} memory_admission_blocked={} memory_admission_admitted={} memory_admission_hold={} memory_admission_reject={} memory_admission_quarantine={} memory_admission_review_packets={} memory_admission_ledger_records={} memory_admission_ledger_authorized={} memory_admission_ledger_applied={} memory_admission_ledger_preview_only={} memory_admission_ledger_held={} memory_admission_ledger_rejected={} memory_admission_ledger_duplicate={} memory_admission_ledger_decayed={} memory_admission_ledger_merged={} memory_admission_ledger_rollback={} kv_fusion_events={} kv_fusion_candidates={} kv_fusion_fused={} kv_fusion_compressed={} kv_fusion_skipped={} kv_fusion_held={} kv_fusion_rejected={} kv_fusion_approval_blocked={} kv_fusion_input_tokens={} kv_fusion_retained_tokens={} kv_fusion_saved_tokens={}",
            self.passed,
            self.checked_lines,
            self.failures.len(),
            self.rust_check_events,
            self.rust_check_passed,
            self.rust_check_failed,
            self.rust_check_feedback_updates,
            self.rust_check_feedback_applied,
            self.business_contract_events,
            self.business_contract_event_passed,
            self.business_contract_event_failed,
            self.business_contract_event_missing_signals,
            self.business_contract_event_protocol_leaks,
            self.business_contract_event_substitutions,
            self.business_contract_event_evasive_denials,
            self.business_contract_event_raw_passed,
            self.business_contract_event_raw_failed,
            self.business_contract_event_response_normalized,
            self.business_contract_event_sanitized,
            self.business_contract_event_canonical_fallbacks,
            self.runtime_error_events,
            self.runtime_timeout_events,
            self.self_evolution_admission_events,
            self.self_evolution_admission_admitted,
            self.self_evolution_admission_blocked,
            self.self_evolution_admission_review_packets,
            self.self_evolution_admission_evidence_ids,
            self.self_evolution_admission_missing_review_packet_refs,
            self.self_evolution_experiment_events,
            self.self_evolution_experiment_admit,
            self.self_evolution_experiment_hold,
            self.self_evolution_experiment_reject,
            self.self_evolution_experiment_rollback,
            self.self_evolution_experiment_repeated,
            self.self_evolution_experiment_conflicts,
            self.self_evolution_experiment_rollback_replayable,
            self.self_evolution_experiment_active_candidates,
            self.self_evolution_experiment_write_allowed,
            self.self_evolution_experiment_applied,
            self.self_evolution_rollback_replay_events,
            self.self_evolution_rollback_replay_items,
            self.self_evolution_rollback_replay_replayable,
            self.self_evolution_rollback_replay_blocked,
            self.self_evolution_rollback_replay_all_replayable,
            self.self_evolution_rollback_replay_rollback_anchor_ids,
            self.self_evolution_rollback_replay_evidence_ids,
            self.self_evolution_rollback_replay_active_candidates,
            self.self_evolution_rollback_replay_item_write_allowed,
            self.self_evolution_rollback_replay_item_applied,
            self.self_evolution_rollback_replay_write_allowed,
            self.self_evolution_rollback_replay_applied,
            self.self_evolution_rollback_replay_gate_events,
            self.self_evolution_rollback_replay_gate_admitted,
            self.self_evolution_rollback_replay_gate_held,
            self.self_evolution_rollback_replay_gate_review_packets,
            self.self_evolution_rollback_replay_gate_review_evidence_ids,
            self.self_evolution_rollback_replay_gate_missing_review_packet_refs,
            self.self_evolution_rollback_replay_gate_items,
            self.self_evolution_rollback_replay_gate_replayable,
            self.self_evolution_rollback_replay_gate_blocked,
            self.self_evolution_rollback_replay_gate_all_replayable,
            self.self_evolution_rollback_replay_gate_rollback_anchor_ids,
            self.self_evolution_rollback_replay_gate_evidence_ids,
            self.self_evolution_rollback_replay_gate_active_candidates,
            self.self_evolution_rollback_replay_gate_item_write_allowed,
            self.self_evolution_rollback_replay_gate_item_applied,
            self.self_evolution_rollback_replay_gate_plan_write_allowed,
            self.self_evolution_rollback_replay_gate_plan_applied,
            self.self_evolution_rollback_replay_gate_write_allowed,
            self.self_evolution_rollback_replay_gate_applied,
            self.self_evolution_operator_approval_events,
            self.self_evolution_operator_approval_approved,
            self.self_evolution_operator_approval_held,
            self.self_evolution_operator_approval_review_packets,
            self.self_evolution_operator_approval_evidence_ids,
            self.self_evolution_operator_approval_rollback_anchor_ids,
            self.self_evolution_operator_approval_content_digests,
            self.self_evolution_operator_approval_source_report_schemas,
            self.self_evolution_operator_approval_missing_review_packet_refs,
            self.self_evolution_operator_approval_write_allowed,
            self.self_evolution_operator_approval_applied,
            self.improvement_corpus_events,
            self.improvement_corpus_episodes,
            self.improvement_corpus_active_adaptation,
            self.improvement_corpus_compiler_passed,
            self.improvement_corpus_test_passed,
            self.improvement_corpus_benchmark_passed,
            self.improvement_corpus_privacy_rejected,
            self.improvement_corpus_secret_leaks,
            self.adaptive_routing_events,
            self.adaptive_routing_candidates,
            self.adaptive_routing_include,
            self.adaptive_routing_compress,
            self.adaptive_routing_defer,
            self.adaptive_routing_skip,
            self.adaptive_routing_input_tokens,
            self.adaptive_routing_retained_tokens,
            self.adaptive_routing_saved_tokens,
            self.task_hierarchy_events,
            self.task_hierarchy_mutation_records,
            self.task_hierarchy_route_pressure_milli,
            self.task_hierarchy_compute_reduction_milli,
            self.memory_admission_events,
            self.memory_admission_candidates,
            self.memory_admission_ready,
            self.memory_admission_blocked,
            self.memory_admission_admitted,
            self.memory_admission_hold,
            self.memory_admission_reject,
            self.memory_admission_quarantine,
            self.memory_admission_review_packets,
            self.memory_admission_ledger_records,
            self.memory_admission_ledger_authorized,
            self.memory_admission_ledger_applied,
            self.memory_admission_ledger_preview_only,
            self.memory_admission_ledger_held,
            self.memory_admission_ledger_rejected,
            self.memory_admission_ledger_duplicate,
            self.memory_admission_ledger_decayed,
            self.memory_admission_ledger_merged,
            self.memory_admission_ledger_rollback,
            self.kv_fusion_events,
            self.kv_fusion_candidates,
            self.kv_fusion_fused,
            self.kv_fusion_compressed,
            self.kv_fusion_skipped,
            self.kv_fusion_held,
            self.kv_fusion_rejected,
            self.kv_fusion_approval_blocked,
            self.kv_fusion_input_tokens,
            self.kv_fusion_retained_tokens,
            self.kv_fusion_saved_tokens
        );
        format!(
            "{base} self_evolution_rollback_replay_apply_events={} self_evolution_rollback_replay_apply_ready={} self_evolution_rollback_replay_apply_held={} self_evolution_rollback_replay_apply_items={} self_evolution_rollback_replay_apply_replayable={} self_evolution_rollback_replay_apply_blocked={} self_evolution_rollback_replay_apply_review_packets={} self_evolution_rollback_replay_apply_evidence_ids={} self_evolution_rollback_replay_apply_rollback_anchor_ids={} self_evolution_rollback_replay_apply_content_digests={} self_evolution_rollback_replay_apply_source_report_schemas={} self_evolution_rollback_replay_apply_missing_refs={} self_evolution_rollback_replay_apply_blocked_reasons={} self_evolution_rollback_replay_apply_write_allowed={} self_evolution_rollback_replay_apply_applied={} self_evolving_memory_store_events={} self_evolving_memory_store_retrieval_events={} self_evolving_memory_store_maintenance_events={} self_evolving_memory_store_admission_preview_events={} self_evolving_memory_store_contexts={} self_evolving_memory_store_maintenance_actions={} self_evolving_memory_store_admission_candidates={} self_evolving_memory_store_write_allowed={} self_evolving_memory_store_durable_write_allowed={} self_evolving_memory_store_applied={} self_evolving_memory_store_applied_to_disk={}",
            self.self_evolution_rollback_replay_apply_events,
            self.self_evolution_rollback_replay_apply_ready,
            self.self_evolution_rollback_replay_apply_held,
            self.self_evolution_rollback_replay_apply_items,
            self.self_evolution_rollback_replay_apply_replayable,
            self.self_evolution_rollback_replay_apply_blocked,
            self.self_evolution_rollback_replay_apply_review_packets,
            self.self_evolution_rollback_replay_apply_evidence_ids,
            self.self_evolution_rollback_replay_apply_rollback_anchor_ids,
            self.self_evolution_rollback_replay_apply_content_digests,
            self.self_evolution_rollback_replay_apply_source_report_schemas,
            self.self_evolution_rollback_replay_apply_missing_refs,
            self.self_evolution_rollback_replay_apply_blocked_reasons,
            self.self_evolution_rollback_replay_apply_write_allowed,
            self.self_evolution_rollback_replay_apply_applied,
            self.self_evolving_memory_store_events,
            self.self_evolving_memory_store_retrieval_events,
            self.self_evolving_memory_store_maintenance_events,
            self.self_evolving_memory_store_admission_preview_events,
            self.self_evolving_memory_store_contexts,
            self.self_evolving_memory_store_maintenance_actions,
            self.self_evolving_memory_store_admission_candidates,
            self.self_evolving_memory_store_write_allowed,
            self.self_evolving_memory_store_durable_write_allowed,
            self.self_evolving_memory_store_applied,
            self.self_evolving_memory_store_applied_to_disk,
        )
    }
}

pub fn evaluate_trace_schema_jsonl(path: impl AsRef<Path>) -> io::Result<TraceSchemaGateReport> {
    let content = fs::read_to_string(path)?;
    let mut checked_lines = 0;
    let mut rust_check_events = 0;
    let mut rust_check_passed = 0;
    let mut rust_check_failed = 0;
    let mut rust_check_feedback_updates = 0;
    let mut rust_check_feedback_applied = 0;
    let mut business_contract_events = 0;
    let mut business_contract_event_passed = 0;
    let mut business_contract_event_failed = 0;
    let mut business_contract_event_missing_signals = 0;
    let mut business_contract_event_protocol_leaks = 0;
    let mut business_contract_event_substitutions = 0;
    let mut business_contract_event_evasive_denials = 0;
    let mut business_contract_event_raw_passed = 0;
    let mut business_contract_event_raw_failed = 0;
    let mut business_contract_event_response_normalized = 0;
    let mut business_contract_event_sanitized = 0;
    let mut business_contract_event_canonical_fallbacks = 0;
    let mut runtime_error_events = 0;
    let mut runtime_timeout_events = 0;
    let mut self_evolution_admission_events = 0;
    let mut self_evolution_admission_admitted = 0;
    let mut self_evolution_admission_blocked = 0;
    let mut self_evolution_admission_review_packets = 0;
    let mut self_evolution_admission_evidence_ids = 0;
    let mut self_evolution_admission_missing_review_packet_refs = 0;
    let mut self_evolution_experiment_events = 0;
    let mut self_evolution_experiment_admit = 0;
    let mut self_evolution_experiment_hold = 0;
    let mut self_evolution_experiment_reject = 0;
    let mut self_evolution_experiment_rollback = 0;
    let mut self_evolution_experiment_repeated = 0;
    let mut self_evolution_experiment_conflicts = 0;
    let mut self_evolution_experiment_rollback_replayable = 0;
    let mut self_evolution_experiment_active_candidates = 0;
    let mut self_evolution_experiment_write_allowed = 0;
    let mut self_evolution_experiment_applied = 0;
    let mut self_evolution_rollback_replay_events = 0;
    let mut self_evolution_rollback_replay_items = 0;
    let mut self_evolution_rollback_replay_replayable = 0;
    let mut self_evolution_rollback_replay_blocked = 0;
    let mut self_evolution_rollback_replay_all_replayable = 0;
    let mut self_evolution_rollback_replay_rollback_anchor_ids = 0;
    let mut self_evolution_rollback_replay_evidence_ids = 0;
    let mut self_evolution_rollback_replay_active_candidates = 0;
    let mut self_evolution_rollback_replay_item_write_allowed = 0;
    let mut self_evolution_rollback_replay_item_applied = 0;
    let mut self_evolution_rollback_replay_write_allowed = 0;
    let mut self_evolution_rollback_replay_applied = 0;
    let mut self_evolution_rollback_replay_gate_events = 0;
    let mut self_evolution_rollback_replay_gate_admitted = 0;
    let mut self_evolution_rollback_replay_gate_held = 0;
    let mut self_evolution_rollback_replay_gate_review_packets = 0;
    let mut self_evolution_rollback_replay_gate_review_evidence_ids = 0;
    let mut self_evolution_rollback_replay_gate_missing_review_packet_refs = 0;
    let mut self_evolution_rollback_replay_gate_items = 0;
    let mut self_evolution_rollback_replay_gate_replayable = 0;
    let mut self_evolution_rollback_replay_gate_blocked = 0;
    let mut self_evolution_rollback_replay_gate_all_replayable = 0;
    let mut self_evolution_rollback_replay_gate_rollback_anchor_ids = 0;
    let mut self_evolution_rollback_replay_gate_evidence_ids = 0;
    let mut self_evolution_rollback_replay_gate_active_candidates = 0;
    let mut self_evolution_rollback_replay_gate_item_write_allowed = 0;
    let mut self_evolution_rollback_replay_gate_item_applied = 0;
    let mut self_evolution_rollback_replay_gate_plan_write_allowed = 0;
    let mut self_evolution_rollback_replay_gate_plan_applied = 0;
    let mut self_evolution_rollback_replay_gate_write_allowed = 0;
    let mut self_evolution_rollback_replay_gate_applied = 0;
    let mut self_evolution_operator_approval_events = 0;
    let mut self_evolution_operator_approval_approved = 0;
    let mut self_evolution_operator_approval_held = 0;
    let mut self_evolution_operator_approval_review_packets = 0;
    let mut self_evolution_operator_approval_evidence_ids = 0;
    let mut self_evolution_operator_approval_rollback_anchor_ids = 0;
    let mut self_evolution_operator_approval_content_digests = 0;
    let mut self_evolution_operator_approval_source_report_schemas = 0;
    let mut self_evolution_operator_approval_missing_review_packet_refs = 0;
    let mut self_evolution_operator_approval_write_allowed = 0;
    let mut self_evolution_operator_approval_applied = 0;
    let mut self_evolution_rollback_replay_apply_events = 0;
    let mut self_evolution_rollback_replay_apply_ready = 0;
    let mut self_evolution_rollback_replay_apply_held = 0;
    let mut self_evolution_rollback_replay_apply_items = 0;
    let mut self_evolution_rollback_replay_apply_replayable = 0;
    let mut self_evolution_rollback_replay_apply_blocked = 0;
    let mut self_evolution_rollback_replay_apply_review_packets = 0;
    let mut self_evolution_rollback_replay_apply_evidence_ids = 0;
    let mut self_evolution_rollback_replay_apply_rollback_anchor_ids = 0;
    let mut self_evolution_rollback_replay_apply_content_digests = 0;
    let mut self_evolution_rollback_replay_apply_source_report_schemas = 0;
    let mut self_evolution_rollback_replay_apply_missing_refs = 0;
    let mut self_evolution_rollback_replay_apply_blocked_reasons = 0;
    let mut self_evolution_rollback_replay_apply_write_allowed = 0;
    let mut self_evolution_rollback_replay_apply_applied = 0;
    let mut self_evolving_memory_store_events = 0;
    let mut self_evolving_memory_store_retrieval_events = 0;
    let mut self_evolving_memory_store_maintenance_events = 0;
    let mut self_evolving_memory_store_admission_preview_events = 0;
    let mut self_evolving_memory_store_contexts = 0;
    let mut self_evolving_memory_store_maintenance_actions = 0;
    let mut self_evolving_memory_store_admission_candidates = 0;
    let mut self_evolving_memory_store_write_allowed = 0;
    let mut self_evolving_memory_store_durable_write_allowed = 0;
    let mut self_evolving_memory_store_applied = 0;
    let mut self_evolving_memory_store_applied_to_disk = 0;
    let mut improvement_corpus_events = 0;
    let mut improvement_corpus_episodes = 0;
    let mut improvement_corpus_active_adaptation = 0;
    let mut improvement_corpus_compiler_passed = 0;
    let mut improvement_corpus_test_passed = 0;
    let mut improvement_corpus_benchmark_passed = 0;
    let mut improvement_corpus_privacy_rejected = 0;
    let mut improvement_corpus_secret_leaks = 0;
    let mut adaptive_routing_events = 0;
    let mut adaptive_routing_candidates = 0;
    let mut adaptive_routing_include = 0;
    let mut adaptive_routing_compress = 0;
    let mut adaptive_routing_defer = 0;
    let mut adaptive_routing_skip = 0;
    let mut adaptive_routing_input_tokens = 0;
    let mut adaptive_routing_retained_tokens = 0;
    let mut adaptive_routing_saved_tokens = 0;
    let mut task_hierarchy_events = 0;
    let mut task_hierarchy_mutation_records = 0;
    let mut task_hierarchy_route_pressure_milli = 0;
    let mut task_hierarchy_compute_reduction_milli = 0;
    let mut memory_admission_events = 0;
    let mut memory_admission_candidates = 0;
    let mut memory_admission_ready = 0;
    let mut memory_admission_blocked = 0;
    let mut memory_admission_admitted = 0;
    let mut memory_admission_hold = 0;
    let mut memory_admission_reject = 0;
    let mut memory_admission_quarantine = 0;
    let mut memory_admission_review_packets = 0;
    let mut memory_admission_ledger_records = 0;
    let mut memory_admission_ledger_authorized = 0;
    let mut memory_admission_ledger_applied = 0;
    let mut memory_admission_ledger_preview_only = 0;
    let mut memory_admission_ledger_held = 0;
    let mut memory_admission_ledger_rejected = 0;
    let mut memory_admission_ledger_duplicate = 0;
    let mut memory_admission_ledger_decayed = 0;
    let mut memory_admission_ledger_merged = 0;
    let mut memory_admission_ledger_rollback = 0;
    let mut kv_fusion_events = 0;
    let mut kv_fusion_candidates = 0;
    let mut kv_fusion_fused = 0;
    let mut kv_fusion_compressed = 0;
    let mut kv_fusion_skipped = 0;
    let mut kv_fusion_held = 0;
    let mut kv_fusion_rejected = 0;
    let mut kv_fusion_approval_blocked = 0;
    let mut kv_fusion_input_tokens = 0;
    let mut kv_fusion_retained_tokens = 0;
    let mut kv_fusion_saved_tokens = 0;
    let mut failures = Vec::new();

    for (index, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        checked_lines += 1;
        if let Some(summary) = rust_check_trace_gate_summary(line) {
            rust_check_events += summary.events;
            rust_check_passed += summary.passed;
            rust_check_failed += summary.failed;
            rust_check_feedback_updates += summary.feedback_updates;
            rust_check_feedback_applied += summary.feedback_applied;
        }
        if let Some(summary) = business_contract_trace_gate_summary(line) {
            business_contract_events += summary.events;
            business_contract_event_passed += summary.passed;
            business_contract_event_failed += summary.failed;
            business_contract_event_missing_signals += summary.missing_signals;
            business_contract_event_protocol_leaks += summary.protocol_leaks;
            business_contract_event_substitutions += summary.substitutions;
            business_contract_event_evasive_denials += summary.evasive_denials;
            business_contract_event_raw_passed += summary.raw_passed;
            business_contract_event_raw_failed += summary.raw_failed;
            business_contract_event_response_normalized += summary.response_normalized;
            business_contract_event_sanitized += summary.sanitized;
            business_contract_event_canonical_fallbacks += summary.canonical_fallbacks;
        }
        if let Some(summary) = runtime_error_trace_gate_summary(line) {
            runtime_error_events += summary.events;
            runtime_timeout_events += summary.timeouts;
        }
        if let Some(summary) = self_evolution_admission_trace_gate_summary(line) {
            self_evolution_admission_events += summary.events;
            self_evolution_admission_admitted += summary.admitted;
            self_evolution_admission_blocked += summary.blocked;
            self_evolution_admission_review_packets += summary.review_packets;
            self_evolution_admission_evidence_ids += summary.evidence_ids;
            self_evolution_admission_missing_review_packet_refs +=
                summary.missing_review_packet_refs;
        }
        if let Some(summary) = self_evolution_experiment_trace_gate_summary(line) {
            self_evolution_experiment_events += summary.events;
            self_evolution_experiment_admit += summary.admit;
            self_evolution_experiment_hold += summary.hold;
            self_evolution_experiment_reject += summary.reject;
            self_evolution_experiment_rollback += summary.rollback;
            self_evolution_experiment_repeated += summary.repeated;
            self_evolution_experiment_conflicts += summary.conflicts;
            self_evolution_experiment_rollback_replayable += summary.rollback_replayable;
            self_evolution_experiment_active_candidates += summary.active_candidates;
            self_evolution_experiment_write_allowed += summary.write_allowed;
            self_evolution_experiment_applied += summary.applied;
        }
        if let Some(summary) = self_evolution_rollback_replay_trace_gate_summary(line) {
            self_evolution_rollback_replay_events += summary.events;
            self_evolution_rollback_replay_items += summary.items;
            self_evolution_rollback_replay_replayable += summary.replayable;
            self_evolution_rollback_replay_blocked += summary.blocked;
            self_evolution_rollback_replay_all_replayable += summary.all_replayable;
            self_evolution_rollback_replay_rollback_anchor_ids += summary.rollback_anchor_ids;
            self_evolution_rollback_replay_evidence_ids += summary.evidence_ids;
            self_evolution_rollback_replay_active_candidates += summary.active_candidates;
            self_evolution_rollback_replay_item_write_allowed += summary.item_write_allowed;
            self_evolution_rollback_replay_item_applied += summary.item_applied;
            self_evolution_rollback_replay_write_allowed += summary.write_allowed;
            self_evolution_rollback_replay_applied += summary.applied;
        }
        if let Some(summary) = self_evolution_rollback_replay_gate_trace_gate_summary(line) {
            self_evolution_rollback_replay_gate_events += summary.events;
            self_evolution_rollback_replay_gate_admitted += summary.admitted;
            self_evolution_rollback_replay_gate_held += summary.held;
            self_evolution_rollback_replay_gate_review_packets += summary.review_packets;
            self_evolution_rollback_replay_gate_review_evidence_ids += summary.review_evidence_ids;
            self_evolution_rollback_replay_gate_missing_review_packet_refs +=
                summary.missing_review_packet_refs;
            self_evolution_rollback_replay_gate_items += summary.items;
            self_evolution_rollback_replay_gate_replayable += summary.replayable;
            self_evolution_rollback_replay_gate_blocked += summary.blocked;
            self_evolution_rollback_replay_gate_all_replayable += summary.all_replayable;
            self_evolution_rollback_replay_gate_rollback_anchor_ids += summary.rollback_anchor_ids;
            self_evolution_rollback_replay_gate_evidence_ids += summary.evidence_ids;
            self_evolution_rollback_replay_gate_active_candidates += summary.active_candidates;
            self_evolution_rollback_replay_gate_item_write_allowed += summary.item_write_allowed;
            self_evolution_rollback_replay_gate_item_applied += summary.item_applied;
            self_evolution_rollback_replay_gate_plan_write_allowed += summary.plan_write_allowed;
            self_evolution_rollback_replay_gate_plan_applied += summary.plan_applied;
            self_evolution_rollback_replay_gate_write_allowed += summary.write_allowed;
            self_evolution_rollback_replay_gate_applied += summary.applied;
        }
        if let Some(summary) = self_evolution_operator_approval_trace_gate_summary(line) {
            self_evolution_operator_approval_events += summary.events;
            self_evolution_operator_approval_approved += summary.approved;
            self_evolution_operator_approval_held += summary.held;
            self_evolution_operator_approval_review_packets += summary.review_packets;
            self_evolution_operator_approval_evidence_ids += summary.evidence_ids;
            self_evolution_operator_approval_rollback_anchor_ids += summary.rollback_anchor_ids;
            self_evolution_operator_approval_content_digests += summary.content_digests;
            self_evolution_operator_approval_source_report_schemas += summary.source_report_schemas;
            self_evolution_operator_approval_missing_review_packet_refs +=
                summary.missing_review_packet_refs;
            self_evolution_operator_approval_write_allowed += summary.write_allowed;
            self_evolution_operator_approval_applied += summary.applied;
        }
        if let Some(summary) = self_evolution_rollback_replay_apply_trace_gate_summary(line) {
            self_evolution_rollback_replay_apply_events += summary.events;
            self_evolution_rollback_replay_apply_ready += summary.ready;
            self_evolution_rollback_replay_apply_held += summary.held;
            self_evolution_rollback_replay_apply_items += summary.items;
            self_evolution_rollback_replay_apply_replayable += summary.replayable;
            self_evolution_rollback_replay_apply_blocked += summary.blocked;
            self_evolution_rollback_replay_apply_review_packets += summary.review_packets;
            self_evolution_rollback_replay_apply_evidence_ids += summary.evidence_ids;
            self_evolution_rollback_replay_apply_rollback_anchor_ids += summary.rollback_anchor_ids;
            self_evolution_rollback_replay_apply_content_digests += summary.content_digests;
            self_evolution_rollback_replay_apply_source_report_schemas +=
                summary.source_report_schemas;
            self_evolution_rollback_replay_apply_missing_refs += summary.missing_refs;
            self_evolution_rollback_replay_apply_blocked_reasons += summary.blocked_reasons;
            self_evolution_rollback_replay_apply_write_allowed += summary.write_allowed;
            self_evolution_rollback_replay_apply_applied += summary.applied;
        }
        if let Some(summary) = self_evolving_memory_store_trace_gate_summary(line) {
            self_evolving_memory_store_events += summary.events;
            self_evolving_memory_store_retrieval_events += summary.retrieval_events;
            self_evolving_memory_store_maintenance_events += summary.maintenance_events;
            self_evolving_memory_store_admission_preview_events += summary.admission_preview_events;
            self_evolving_memory_store_contexts += summary.contexts;
            self_evolving_memory_store_maintenance_actions += summary.maintenance_actions;
            self_evolving_memory_store_admission_candidates += summary.admission_candidates;
            self_evolving_memory_store_write_allowed += summary.write_allowed;
            self_evolving_memory_store_durable_write_allowed += summary.durable_write_allowed;
            self_evolving_memory_store_applied += summary.applied;
            self_evolving_memory_store_applied_to_disk += summary.applied_to_disk;
        }
        if let Some(summary) = improvement_corpus_trace_gate_summary(line) {
            improvement_corpus_events += summary.events;
            improvement_corpus_episodes += summary.episodes;
            improvement_corpus_active_adaptation += summary.active_adaptation;
            improvement_corpus_compiler_passed += summary.compiler_passed;
            improvement_corpus_test_passed += summary.test_passed;
            improvement_corpus_benchmark_passed += summary.benchmark_passed;
            improvement_corpus_privacy_rejected += summary.privacy_rejected;
            improvement_corpus_secret_leaks += summary.secret_leaks;
        }
        if let Some(summary) = adaptive_routing_trace_gate_summary(line) {
            adaptive_routing_events += summary.events;
            adaptive_routing_candidates += summary.candidates;
            adaptive_routing_include += summary.include;
            adaptive_routing_compress += summary.compress;
            adaptive_routing_defer += summary.defer;
            adaptive_routing_skip += summary.skip;
            adaptive_routing_input_tokens += summary.input_tokens;
            adaptive_routing_retained_tokens += summary.retained_tokens;
            adaptive_routing_saved_tokens += summary.saved_tokens;
        }
        if let Some(summary) = task_hierarchy_trace_gate_summary(line) {
            task_hierarchy_events += summary.events;
            task_hierarchy_mutation_records += summary.mutation_records;
            task_hierarchy_route_pressure_milli += summary.route_pressure_milli;
            task_hierarchy_compute_reduction_milli += summary.compute_reduction_milli;
        }
        if let Some(summary) = memory_admission_trace_gate_summary(line) {
            memory_admission_events += summary.events;
            memory_admission_candidates += summary.candidates;
            memory_admission_ready += summary.ready;
            memory_admission_blocked += summary.blocked;
            memory_admission_admitted += summary.admitted;
            memory_admission_hold += summary.hold;
            memory_admission_reject += summary.reject;
            memory_admission_quarantine += summary.quarantine;
            memory_admission_review_packets += summary.review_packets;
            memory_admission_ledger_records += summary.ledger_records;
            memory_admission_ledger_authorized += summary.ledger_authorized;
            memory_admission_ledger_applied += summary.ledger_applied;
            memory_admission_ledger_preview_only += summary.ledger_preview_only;
            memory_admission_ledger_held += summary.ledger_held;
            memory_admission_ledger_rejected += summary.ledger_rejected;
            memory_admission_ledger_duplicate += summary.ledger_duplicate;
            memory_admission_ledger_decayed += summary.ledger_decayed;
            memory_admission_ledger_merged += summary.ledger_merged;
            memory_admission_ledger_rollback += summary.ledger_rollback;
        }
        if let Some(summary) = kv_fusion_trace_gate_summary(line) {
            kv_fusion_events += summary.events;
            kv_fusion_candidates += summary.candidates;
            kv_fusion_fused += summary.fused;
            kv_fusion_compressed += summary.compressed;
            kv_fusion_skipped += summary.skipped;
            kv_fusion_held += summary.held;
            kv_fusion_rejected += summary.rejected;
            kv_fusion_approval_blocked += summary.approval_blocked;
            kv_fusion_input_tokens += summary.input_tokens;
            kv_fusion_retained_tokens += summary.retained_tokens;
            kv_fusion_saved_tokens += summary.saved_tokens;
        }
        failures.extend(
            evaluate_trace_schema_line(line)
                .into_iter()
                .map(|failure| format!("line {}: {failure}", index + 1)),
        );
    }

    if checked_lines == 0 {
        failures.push("trace file did not contain any non-empty JSONL records".to_owned());
    }

    Ok(TraceSchemaGateReport {
        passed: failures.is_empty(),
        checked_lines,
        rust_check_events,
        rust_check_passed,
        rust_check_failed,
        rust_check_feedback_updates,
        rust_check_feedback_applied,
        business_contract_events,
        business_contract_event_passed,
        business_contract_event_failed,
        business_contract_event_missing_signals,
        business_contract_event_protocol_leaks,
        business_contract_event_substitutions,
        business_contract_event_evasive_denials,
        business_contract_event_raw_passed,
        business_contract_event_raw_failed,
        business_contract_event_response_normalized,
        business_contract_event_sanitized,
        business_contract_event_canonical_fallbacks,
        runtime_error_events,
        runtime_timeout_events,
        self_evolution_admission_events,
        self_evolution_admission_admitted,
        self_evolution_admission_blocked,
        self_evolution_admission_review_packets,
        self_evolution_admission_evidence_ids,
        self_evolution_admission_missing_review_packet_refs,
        self_evolution_experiment_events,
        self_evolution_experiment_admit,
        self_evolution_experiment_hold,
        self_evolution_experiment_reject,
        self_evolution_experiment_rollback,
        self_evolution_experiment_repeated,
        self_evolution_experiment_conflicts,
        self_evolution_experiment_rollback_replayable,
        self_evolution_experiment_active_candidates,
        self_evolution_experiment_write_allowed,
        self_evolution_experiment_applied,
        self_evolution_rollback_replay_events,
        self_evolution_rollback_replay_items,
        self_evolution_rollback_replay_replayable,
        self_evolution_rollback_replay_blocked,
        self_evolution_rollback_replay_all_replayable,
        self_evolution_rollback_replay_rollback_anchor_ids,
        self_evolution_rollback_replay_evidence_ids,
        self_evolution_rollback_replay_active_candidates,
        self_evolution_rollback_replay_item_write_allowed,
        self_evolution_rollback_replay_item_applied,
        self_evolution_rollback_replay_write_allowed,
        self_evolution_rollback_replay_applied,
        self_evolution_rollback_replay_gate_events,
        self_evolution_rollback_replay_gate_admitted,
        self_evolution_rollback_replay_gate_held,
        self_evolution_rollback_replay_gate_review_packets,
        self_evolution_rollback_replay_gate_review_evidence_ids,
        self_evolution_rollback_replay_gate_missing_review_packet_refs,
        self_evolution_rollback_replay_gate_items,
        self_evolution_rollback_replay_gate_replayable,
        self_evolution_rollback_replay_gate_blocked,
        self_evolution_rollback_replay_gate_all_replayable,
        self_evolution_rollback_replay_gate_rollback_anchor_ids,
        self_evolution_rollback_replay_gate_evidence_ids,
        self_evolution_rollback_replay_gate_active_candidates,
        self_evolution_rollback_replay_gate_item_write_allowed,
        self_evolution_rollback_replay_gate_item_applied,
        self_evolution_rollback_replay_gate_plan_write_allowed,
        self_evolution_rollback_replay_gate_plan_applied,
        self_evolution_rollback_replay_gate_write_allowed,
        self_evolution_rollback_replay_gate_applied,
        self_evolution_operator_approval_events,
        self_evolution_operator_approval_approved,
        self_evolution_operator_approval_held,
        self_evolution_operator_approval_review_packets,
        self_evolution_operator_approval_evidence_ids,
        self_evolution_operator_approval_rollback_anchor_ids,
        self_evolution_operator_approval_content_digests,
        self_evolution_operator_approval_source_report_schemas,
        self_evolution_operator_approval_missing_review_packet_refs,
        self_evolution_operator_approval_write_allowed,
        self_evolution_operator_approval_applied,
        self_evolution_rollback_replay_apply_events,
        self_evolution_rollback_replay_apply_ready,
        self_evolution_rollback_replay_apply_held,
        self_evolution_rollback_replay_apply_items,
        self_evolution_rollback_replay_apply_replayable,
        self_evolution_rollback_replay_apply_blocked,
        self_evolution_rollback_replay_apply_review_packets,
        self_evolution_rollback_replay_apply_evidence_ids,
        self_evolution_rollback_replay_apply_rollback_anchor_ids,
        self_evolution_rollback_replay_apply_content_digests,
        self_evolution_rollback_replay_apply_source_report_schemas,
        self_evolution_rollback_replay_apply_missing_refs,
        self_evolution_rollback_replay_apply_blocked_reasons,
        self_evolution_rollback_replay_apply_write_allowed,
        self_evolution_rollback_replay_apply_applied,
        self_evolving_memory_store_events,
        self_evolving_memory_store_retrieval_events,
        self_evolving_memory_store_maintenance_events,
        self_evolving_memory_store_admission_preview_events,
        self_evolving_memory_store_contexts,
        self_evolving_memory_store_maintenance_actions,
        self_evolving_memory_store_admission_candidates,
        self_evolving_memory_store_write_allowed,
        self_evolving_memory_store_durable_write_allowed,
        self_evolving_memory_store_applied,
        self_evolving_memory_store_applied_to_disk,
        improvement_corpus_events,
        improvement_corpus_episodes,
        improvement_corpus_active_adaptation,
        improvement_corpus_compiler_passed,
        improvement_corpus_test_passed,
        improvement_corpus_benchmark_passed,
        improvement_corpus_privacy_rejected,
        improvement_corpus_secret_leaks,
        adaptive_routing_events,
        adaptive_routing_candidates,
        adaptive_routing_include,
        adaptive_routing_compress,
        adaptive_routing_defer,
        adaptive_routing_skip,
        adaptive_routing_input_tokens,
        adaptive_routing_retained_tokens,
        adaptive_routing_saved_tokens,
        task_hierarchy_events,
        task_hierarchy_mutation_records,
        task_hierarchy_route_pressure_milli,
        task_hierarchy_compute_reduction_milli,
        memory_admission_events,
        memory_admission_candidates,
        memory_admission_ready,
        memory_admission_blocked,
        memory_admission_admitted,
        memory_admission_hold,
        memory_admission_reject,
        memory_admission_quarantine,
        memory_admission_review_packets,
        memory_admission_ledger_records,
        memory_admission_ledger_authorized,
        memory_admission_ledger_applied,
        memory_admission_ledger_preview_only,
        memory_admission_ledger_held,
        memory_admission_ledger_rejected,
        memory_admission_ledger_duplicate,
        memory_admission_ledger_decayed,
        memory_admission_ledger_merged,
        memory_admission_ledger_rollback,
        kv_fusion_events,
        kv_fusion_candidates,
        kv_fusion_fused,
        kv_fusion_compressed,
        kv_fusion_skipped,
        kv_fusion_held,
        kv_fusion_rejected,
        kv_fusion_approval_blocked,
        kv_fusion_input_tokens,
        kv_fusion_retained_tokens,
        kv_fusion_saved_tokens,
        failures,
    })
}

#[derive(Debug, Clone, Copy, Default)]
struct RustCheckTraceGateSummary {
    events: usize,
    passed: usize,
    failed: usize,
    feedback_updates: usize,
    feedback_applied: usize,
}

fn rust_check_trace_gate_summary(line: &str) -> Option<RustCheckTraceGateSummary> {
    if !line.contains("\"schema\":\"rust-norion-rust-check-v1\"") {
        return None;
    }

    let mut summary = RustCheckTraceGateSummary {
        events: 1,
        ..RustCheckTraceGateSummary::default()
    };

    if let Some(rust_check) = json_object_after_field(line, "rust_check") {
        match extract_json_bool_field(rust_check, "passed") {
            Some(true) => summary.passed = 1,
            Some(false) => summary.failed = 1,
            None => {}
        }
    }
    if let Some(feedback) = json_object_after_field(line, "feedback") {
        let applied = extract_json_usize_field(feedback, "applied").unwrap_or(0);
        let missing = extract_json_usize_field(feedback, "missing").unwrap_or(0);
        summary.feedback_updates = applied.saturating_add(missing);
        summary.feedback_applied = applied;
    }

    Some(summary)
}

#[derive(Debug, Clone, Copy, Default)]
struct BusinessContractTraceGateSummary {
    events: usize,
    passed: usize,
    failed: usize,
    missing_signals: usize,
    protocol_leaks: usize,
    substitutions: usize,
    evasive_denials: usize,
    raw_passed: usize,
    raw_failed: usize,
    response_normalized: usize,
    sanitized: usize,
    canonical_fallbacks: usize,
}

fn business_contract_trace_gate_summary(line: &str) -> Option<BusinessContractTraceGateSummary> {
    if !line.contains("\"schema\":\"rust-norion-business-contract-v1\"") {
        return None;
    }

    let mut summary = BusinessContractTraceGateSummary {
        events: 1,
        ..BusinessContractTraceGateSummary::default()
    };
    let business_contract = json_object_after_field(line, "business_contract")?;

    match extract_json_bool_field(business_contract, "passed") {
        Some(true) => summary.passed = 1,
        Some(false) => summary.failed = 1,
        None => {}
    }
    summary.missing_signals =
        extract_json_usize_field(business_contract, "missing_signal_count").unwrap_or(0);
    summary.protocol_leaks =
        usize::from(extract_json_bool_field(business_contract, "protocol_leak").unwrap_or(false));
    summary.substitutions = usize::from(
        extract_json_bool_field(business_contract, "substituted_runtime_model_experiences")
            .unwrap_or(false),
    );
    summary.evasive_denials =
        usize::from(extract_json_bool_field(business_contract, "evasive_denial").unwrap_or(false));
    match extract_json_bool_field(business_contract, "raw_passed") {
        Some(true) => summary.raw_passed = 1,
        Some(false) => summary.raw_failed = 1,
        None => {}
    }
    summary.response_normalized = usize::from(
        extract_json_bool_field(business_contract, "response_normalized").unwrap_or(false),
    );
    let normalization =
        extract_json_string_field(business_contract, "normalization").unwrap_or_default();
    summary.sanitized = usize::from(normalization == "sanitized");
    summary.canonical_fallbacks = usize::from(
        extract_json_bool_field(business_contract, "canonical_fallback").unwrap_or(false),
    );

    Some(summary)
}

#[derive(Debug, Clone, Copy, Default)]
struct RuntimeErrorTraceGateSummary {
    events: usize,
    timeouts: usize,
}

fn runtime_error_trace_gate_summary(line: &str) -> Option<RuntimeErrorTraceGateSummary> {
    if line.contains("\"schema\":\"rust-norion-rust-check-v1\"") {
        return None;
    }
    let process_reward = json_object_after_field(line, "process_reward")?;
    let notes = extract_json_string_array_field(process_reward, "notes").unwrap_or_default();
    let mut summary = RuntimeErrorTraceGateSummary::default();
    for note in notes
        .iter()
        .filter(|note| note.starts_with("runtime_error:"))
    {
        summary.events = summary.events.saturating_add(1);
        if trace_note_bool(note, "timeout=").unwrap_or(false) {
            summary.timeouts = summary.timeouts.saturating_add(1);
        }
    }

    (summary.events > 0).then_some(summary)
}

#[derive(Debug, Clone, Copy, Default)]
struct SelfEvolutionAdmissionTraceGateSummary {
    events: usize,
    admitted: usize,
    blocked: usize,
    review_packets: usize,
    evidence_ids: usize,
    missing_review_packet_refs: usize,
}

fn self_evolution_admission_trace_gate_summary(
    line: &str,
) -> Option<SelfEvolutionAdmissionTraceGateSummary> {
    if !line.contains("\"schema\":\"rust-norion-self-evolution-admission-v1\"") {
        return None;
    }

    let admitted = extract_json_bool_field(line, "admitted_for_human_review").unwrap_or(false);
    let blocked_reasons = extract_last_json_string_array_field(line, "blocked_reasons")
        .map(|reasons| reasons.len())
        .unwrap_or(0);
    let review_packet = json_object_after_field(line, "review_packet");
    let review_packets = review_packet
        .and_then(|object| extract_json_string_array_field(object, "approval_review_packet_ids"))
        .map(|ids| ids.len())
        .unwrap_or(0);
    let evidence_ids = review_packet
        .and_then(|object| extract_json_string_array_field(object, "evidence_ids"))
        .map(|ids| ids.len())
        .unwrap_or(0);
    let missing_review_packet_refs = usize::from(review_packets == 0 || evidence_ids == 0);

    Some(SelfEvolutionAdmissionTraceGateSummary {
        events: 1,
        admitted: usize::from(admitted),
        blocked: usize::from(!admitted || blocked_reasons > 0),
        review_packets,
        evidence_ids,
        missing_review_packet_refs,
    })
}

#[derive(Debug, Clone, Copy, Default)]
struct SelfEvolutionExperimentTraceGateSummary {
    events: usize,
    admit: usize,
    hold: usize,
    reject: usize,
    rollback: usize,
    repeated: usize,
    conflicts: usize,
    rollback_replayable: usize,
    active_candidates: usize,
    write_allowed: usize,
    applied: usize,
}

fn self_evolution_experiment_trace_gate_summary(
    line: &str,
) -> Option<SelfEvolutionExperimentTraceGateSummary> {
    if !line.contains("\"schema\":\"rust-norion-self-evolution-experiment-v1\"") {
        return None;
    }

    let decision = extract_json_string_field(line, "decision").unwrap_or_default();

    Some(SelfEvolutionExperimentTraceGateSummary {
        events: 1,
        admit: usize::from(decision == "admit_for_human_review"),
        hold: usize::from(decision == "hold"),
        reject: usize::from(decision == "reject"),
        rollback: usize::from(decision == "rollback"),
        repeated: usize::from(
            extract_json_bool_field(line, "repeated_experiment").unwrap_or(false),
        ),
        conflicts: usize::from(
            extract_json_bool_field(line, "conflicting_evidence").unwrap_or(false),
        ),
        rollback_replayable: usize::from(
            extract_json_bool_field(line, "rollback_replayable").unwrap_or(false),
        ),
        active_candidates: usize::from(
            extract_json_bool_field(line, "active_candidate").unwrap_or(false),
        ),
        write_allowed: usize::from(extract_json_bool_field(line, "write_allowed").unwrap_or(false)),
        applied: usize::from(extract_json_bool_field(line, "applied").unwrap_or(false)),
    })
}

#[derive(Debug, Clone, Copy, Default)]
struct SelfEvolutionRollbackReplayTraceGateSummary {
    events: usize,
    items: usize,
    replayable: usize,
    blocked: usize,
    all_replayable: usize,
    rollback_anchor_ids: usize,
    evidence_ids: usize,
    active_candidates: usize,
    item_write_allowed: usize,
    item_applied: usize,
    write_allowed: usize,
    applied: usize,
}

fn self_evolution_rollback_replay_trace_gate_summary(
    line: &str,
) -> Option<SelfEvolutionRollbackReplayTraceGateSummary> {
    if !line.contains("\"schema\":\"rust-norion-self-evolution-rollback-replay-plan-v1\"") {
        return None;
    }

    Some(SelfEvolutionRollbackReplayTraceGateSummary {
        events: 1,
        items: extract_json_usize_field(line, "item_count").unwrap_or(0),
        replayable: extract_json_usize_field(line, "replayable").unwrap_or(0),
        blocked: extract_json_usize_field(line, "blocked").unwrap_or(0),
        all_replayable: usize::from(
            extract_json_bool_field(line, "all_replayable").unwrap_or(false),
        ),
        rollback_anchor_ids: extract_json_string_array_field(line, "rollback_anchor_ids")
            .map(|ids| ids.len())
            .unwrap_or(0),
        evidence_ids: extract_json_string_array_field(line, "evidence_ids")
            .map(|ids| ids.len())
            .unwrap_or(0),
        active_candidates: extract_json_usize_field(line, "active_candidates").unwrap_or(0),
        item_write_allowed: extract_json_usize_field(line, "item_write_allowed").unwrap_or(0),
        item_applied: extract_json_usize_field(line, "item_applied").unwrap_or(0),
        write_allowed: usize::from(extract_json_bool_field(line, "write_allowed").unwrap_or(false)),
        applied: usize::from(extract_json_bool_field(line, "applied").unwrap_or(false)),
    })
}

#[derive(Debug, Clone, Copy, Default)]
struct SelfEvolutionRollbackReplayGateTraceGateSummary {
    events: usize,
    admitted: usize,
    held: usize,
    review_packets: usize,
    review_evidence_ids: usize,
    missing_review_packet_refs: usize,
    items: usize,
    replayable: usize,
    blocked: usize,
    all_replayable: usize,
    rollback_anchor_ids: usize,
    evidence_ids: usize,
    active_candidates: usize,
    item_write_allowed: usize,
    item_applied: usize,
    plan_write_allowed: usize,
    plan_applied: usize,
    write_allowed: usize,
    applied: usize,
}

fn self_evolution_rollback_replay_gate_trace_gate_summary(
    line: &str,
) -> Option<SelfEvolutionRollbackReplayGateTraceGateSummary> {
    if !line.contains("\"schema\":\"rust-norion-self-evolution-rollback-replay-gate-v1\"") {
        return None;
    }

    let admitted = extract_json_bool_field(line, "admitted_for_human_review").unwrap_or(false);
    let review_packet = json_object_after_field(line, "review_packet");
    let review_packets = review_packet
        .and_then(|object| extract_json_string_array_field(object, "approval_review_packet_ids"))
        .map(|ids| ids.len())
        .unwrap_or(0);
    let review_evidence_ids = review_packet
        .and_then(|object| extract_json_string_array_field(object, "evidence_ids"))
        .map(|ids| ids.len())
        .unwrap_or(0);
    let missing_review_packet_refs =
        usize::from(review_packets == 0 || (admitted && review_evidence_ids == 0));

    Some(SelfEvolutionRollbackReplayGateTraceGateSummary {
        events: 1,
        admitted: usize::from(admitted),
        held: usize::from(!admitted),
        review_packets,
        review_evidence_ids,
        missing_review_packet_refs,
        items: extract_json_usize_field(line, "item_count").unwrap_or(0),
        replayable: extract_json_usize_field(line, "replayable").unwrap_or(0),
        blocked: extract_json_usize_field(line, "blocked").unwrap_or(0),
        all_replayable: usize::from(
            extract_json_bool_field(line, "all_replayable").unwrap_or(false),
        ),
        rollback_anchor_ids: extract_json_string_array_field(line, "rollback_anchor_ids")
            .map(|ids| ids.len())
            .unwrap_or(0),
        evidence_ids: extract_json_string_array_field(line, "evidence_ids")
            .map(|ids| ids.len())
            .unwrap_or(0),
        active_candidates: extract_json_usize_field(line, "active_candidates").unwrap_or(0),
        item_write_allowed: extract_json_usize_field(line, "item_write_allowed").unwrap_or(0),
        item_applied: extract_json_usize_field(line, "item_applied").unwrap_or(0),
        plan_write_allowed: usize::from(
            extract_json_bool_field(line, "plan_write_allowed").unwrap_or(false),
        ),
        plan_applied: usize::from(extract_json_bool_field(line, "plan_applied").unwrap_or(false)),
        write_allowed: usize::from(extract_json_bool_field(line, "write_allowed").unwrap_or(false)),
        applied: usize::from(extract_json_bool_field(line, "applied").unwrap_or(false)),
    })
}

#[derive(Debug, Clone, Copy, Default)]
struct SelfEvolutionOperatorApprovalTraceGateSummary {
    events: usize,
    approved: usize,
    held: usize,
    review_packets: usize,
    evidence_ids: usize,
    rollback_anchor_ids: usize,
    content_digests: usize,
    source_report_schemas: usize,
    missing_review_packet_refs: usize,
    write_allowed: usize,
    applied: usize,
}

fn self_evolution_operator_approval_trace_gate_summary(
    line: &str,
) -> Option<SelfEvolutionOperatorApprovalTraceGateSummary> {
    if !line.contains("\"schema\":\"rust-norion-self-evolution-operator-approval-v1\"") {
        return None;
    }

    let approved = extract_json_bool_field(line, "operator_approved").unwrap_or(false);
    let review_packets =
        extract_json_usize_field(line, "approved_review_packet_count").unwrap_or(0);
    let evidence_ids = extract_json_usize_field(line, "approved_evidence_count").unwrap_or(0);
    let rollback_anchor_ids =
        extract_json_usize_field(line, "approved_rollback_anchor_count").unwrap_or(0);
    let content_digests =
        extract_json_usize_field(line, "approved_content_digest_count").unwrap_or(0);
    let source_report_schemas =
        extract_json_usize_field(line, "approved_source_report_schema_count").unwrap_or(0);

    Some(SelfEvolutionOperatorApprovalTraceGateSummary {
        events: 1,
        approved: usize::from(approved),
        held: usize::from(!approved),
        review_packets,
        evidence_ids,
        rollback_anchor_ids,
        content_digests,
        source_report_schemas,
        missing_review_packet_refs: usize::from(review_packets == 0 || evidence_ids == 0),
        write_allowed: usize::from(
            extract_json_bool_field(line, "activation_write_allowed").unwrap_or(false)
                || extract_json_bool_field(line, "write_allowed").unwrap_or(false),
        ),
        applied: usize::from(
            extract_json_bool_field(line, "active_candidate").unwrap_or(false)
                || extract_json_bool_field(line, "applied").unwrap_or(false),
        ),
    })
}

#[derive(Debug, Clone, Copy, Default)]
struct SelfEvolutionRollbackReplayApplyTraceGateSummary {
    events: usize,
    ready: usize,
    held: usize,
    items: usize,
    replayable: usize,
    blocked: usize,
    review_packets: usize,
    evidence_ids: usize,
    rollback_anchor_ids: usize,
    content_digests: usize,
    source_report_schemas: usize,
    missing_refs: usize,
    blocked_reasons: usize,
    write_allowed: usize,
    applied: usize,
}

fn self_evolution_rollback_replay_apply_trace_gate_summary(
    line: &str,
) -> Option<SelfEvolutionRollbackReplayApplyTraceGateSummary> {
    if !line.contains("\"schema\":\"rust-norion-self-evolution-rollback-replay-apply-v1\"") {
        return None;
    }

    let ready = extract_json_bool_field(line, "ready_for_operator_apply").unwrap_or(false);
    let review_packets = extract_json_usize_field(line, "review_packet_count").unwrap_or(0);
    let evidence_ids = extract_json_usize_field(line, "evidence_id_count").unwrap_or(0);
    let rollback_anchor_ids = extract_json_usize_field(line, "rollback_anchor_count").unwrap_or(0);
    let content_digests = extract_json_usize_field(line, "content_digest_count").unwrap_or(0);
    let source_report_schemas =
        extract_json_usize_field(line, "source_report_schema_count").unwrap_or(0);
    let missing_refs = usize::from(
        review_packets == 0
            || evidence_ids == 0
            || rollback_anchor_ids == 0
            || content_digests == 0
            || source_report_schemas == 0,
    );

    Some(SelfEvolutionRollbackReplayApplyTraceGateSummary {
        events: 1,
        ready: usize::from(ready),
        held: usize::from(!ready),
        items: extract_json_usize_field(line, "item_count").unwrap_or(0),
        replayable: extract_json_usize_field(line, "replayable").unwrap_or(0),
        blocked: extract_json_usize_field(line, "blocked").unwrap_or(0),
        review_packets,
        evidence_ids,
        rollback_anchor_ids,
        content_digests,
        source_report_schemas,
        missing_refs,
        blocked_reasons: extract_json_usize_field(line, "blocked_reasons_count").unwrap_or(0),
        write_allowed: usize::from(
            extract_json_bool_field(line, "activation_write_allowed").unwrap_or(false)
                || extract_json_bool_field(line, "write_allowed").unwrap_or(false),
        ),
        applied: usize::from(
            extract_json_bool_field(line, "active_candidate").unwrap_or(false)
                || extract_json_bool_field(line, "applied").unwrap_or(false),
        ),
    })
}

#[derive(Debug, Clone, Copy, Default)]
struct SelfEvolvingMemoryStoreTraceGateSummary {
    events: usize,
    retrieval_events: usize,
    maintenance_events: usize,
    admission_preview_events: usize,
    contexts: usize,
    maintenance_actions: usize,
    admission_candidates: usize,
    write_allowed: usize,
    durable_write_allowed: usize,
    applied: usize,
    applied_to_disk: usize,
}

fn self_evolving_memory_store_trace_gate_summary(
    line: &str,
) -> Option<SelfEvolvingMemoryStoreTraceGateSummary> {
    if !line.contains("\"schema\":\"rust-norion-self-evolving-memory-store-v1\"") {
        return None;
    }

    let operation = extract_json_string_field(line, "operation").unwrap_or_default();

    Some(SelfEvolvingMemoryStoreTraceGateSummary {
        events: 1,
        retrieval_events: usize::from(operation == "retrieval"),
        maintenance_events: usize::from(operation == "maintenance"),
        admission_preview_events: usize::from(operation == "admission_preview"),
        contexts: extract_json_usize_field(line, "contexts").unwrap_or(0),
        maintenance_actions: extract_json_usize_field(line, "maintenance_actions").unwrap_or(0),
        admission_candidates: extract_json_usize_field(line, "candidates").unwrap_or(0),
        write_allowed: usize::from(extract_json_bool_field(line, "write_allowed").unwrap_or(false)),
        durable_write_allowed: usize::from(
            extract_json_bool_field(line, "durable_write_allowed").unwrap_or(false),
        ),
        applied: usize::from(extract_json_bool_field(line, "applied").unwrap_or(false)),
        applied_to_disk: usize::from(
            extract_json_bool_field(line, "applied_to_disk").unwrap_or(false),
        ),
    })
}

#[derive(Debug, Clone, Copy, Default)]
struct ImprovementCorpusTraceGateSummary {
    events: usize,
    episodes: usize,
    active_adaptation: usize,
    compiler_passed: usize,
    test_passed: usize,
    benchmark_passed: usize,
    privacy_rejected: usize,
    secret_leaks: usize,
}

fn improvement_corpus_trace_gate_summary(line: &str) -> Option<ImprovementCorpusTraceGateSummary> {
    if !line.contains("\"schema\":\"rust-norion-improvement-corpus-v1\"") {
        return None;
    }

    let records = json_object_after_field(line, "records");
    let active_adaptation = json_object_after_field(line, "active_adaptation");
    let evidence = json_object_after_field(line, "evidence");
    let privacy = json_object_after_field(line, "privacy");

    Some(ImprovementCorpusTraceGateSummary {
        events: 1,
        episodes: records
            .and_then(|object| extract_json_usize_field(object, "total"))
            .unwrap_or(0),
        active_adaptation: active_adaptation
            .and_then(|object| extract_json_usize_field(object, "eligible"))
            .unwrap_or(0),
        compiler_passed: evidence
            .and_then(|object| extract_json_usize_field(object, "compiler_passed"))
            .unwrap_or(0),
        test_passed: evidence
            .and_then(|object| extract_json_usize_field(object, "test_passed"))
            .unwrap_or(0),
        benchmark_passed: evidence
            .and_then(|object| extract_json_usize_field(object, "benchmark_passed"))
            .unwrap_or(0),
        privacy_rejected: privacy
            .and_then(|object| extract_json_usize_field(object, "rejected"))
            .unwrap_or(0),
        secret_leaks: privacy
            .and_then(|object| extract_json_usize_field(object, "secret_leaks"))
            .unwrap_or(0),
    })
}

#[derive(Debug, Clone, Copy, Default)]
struct AdaptiveRoutingTraceGateSummary {
    events: usize,
    candidates: usize,
    include: usize,
    compress: usize,
    defer: usize,
    skip: usize,
    input_tokens: usize,
    retained_tokens: usize,
    saved_tokens: usize,
}

fn adaptive_routing_trace_gate_summary(line: &str) -> Option<AdaptiveRoutingTraceGateSummary> {
    let routing = json_object_after_field(line, "adaptive_routing")?;

    Some(AdaptiveRoutingTraceGateSummary {
        events: 1,
        candidates: extract_json_usize_field(routing, "candidates").unwrap_or(0),
        include: extract_json_usize_field(routing, "include").unwrap_or(0),
        compress: extract_json_usize_field(routing, "compress").unwrap_or(0),
        defer: extract_json_usize_field(routing, "defer").unwrap_or(0),
        skip: extract_json_usize_field(routing, "skip").unwrap_or(0),
        input_tokens: extract_json_usize_field(routing, "input_tokens").unwrap_or(0),
        retained_tokens: extract_json_usize_field(routing, "retained_tokens").unwrap_or(0),
        saved_tokens: extract_json_usize_field(routing, "saved_tokens").unwrap_or(0),
    })
}

#[derive(Debug, Clone, Copy, Default)]
struct TaskHierarchyTraceGateSummary {
    events: usize,
    mutation_records: usize,
    route_pressure_milli: usize,
    compute_reduction_milli: usize,
}

fn task_hierarchy_trace_gate_summary(line: &str) -> Option<TaskHierarchyTraceGateSummary> {
    let task = json_object_after_field(line, "task_hierarchy")?;

    Some(TaskHierarchyTraceGateSummary {
        events: 1,
        mutation_records: extract_json_usize_field(task, "mutation_records").unwrap_or(0),
        route_pressure_milli: trace_gate_milli(
            extract_json_f32_field(task, "route_pressure").unwrap_or(0.0),
        ),
        compute_reduction_milli: trace_gate_milli(
            extract_json_f32_field(task, "compute_reduction").unwrap_or(0.0),
        ),
    })
}

#[derive(Debug, Clone, Copy, Default)]
struct MemoryAdmissionTraceGateSummary {
    events: usize,
    candidates: usize,
    ready: usize,
    blocked: usize,
    admitted: usize,
    hold: usize,
    reject: usize,
    quarantine: usize,
    review_packets: usize,
    ledger_records: usize,
    ledger_authorized: usize,
    ledger_applied: usize,
    ledger_preview_only: usize,
    ledger_held: usize,
    ledger_rejected: usize,
    ledger_duplicate: usize,
    ledger_decayed: usize,
    ledger_merged: usize,
    ledger_rollback: usize,
}

fn memory_admission_trace_gate_summary(line: &str) -> Option<MemoryAdmissionTraceGateSummary> {
    let admission = json_object_after_field(line, "memory_admission")?;

    Some(MemoryAdmissionTraceGateSummary {
        events: 1,
        candidates: extract_json_usize_field(admission, "candidates").unwrap_or(0),
        ready: extract_json_usize_field(admission, "ready").unwrap_or(0),
        blocked: extract_json_usize_field(admission, "blocked").unwrap_or(0),
        admitted: extract_json_usize_field(admission, "admitted").unwrap_or(0),
        hold: extract_json_usize_field(admission, "hold").unwrap_or(0),
        reject: extract_json_usize_field(admission, "reject").unwrap_or(0),
        quarantine: extract_json_usize_field(admission, "quarantine").unwrap_or(0),
        review_packets: extract_json_usize_field(admission, "review_packets").unwrap_or(0),
        ledger_records: extract_json_usize_field(admission, "ledger_records").unwrap_or(0),
        ledger_authorized: extract_json_usize_field(admission, "ledger_authorized").unwrap_or(0),
        ledger_applied: extract_json_usize_field(admission, "ledger_applied").unwrap_or(0),
        ledger_preview_only: extract_json_usize_field(admission, "ledger_preview_only")
            .unwrap_or(0),
        ledger_held: extract_json_usize_field(admission, "ledger_held").unwrap_or(0),
        ledger_rejected: extract_json_usize_field(admission, "ledger_rejected").unwrap_or(0),
        ledger_duplicate: extract_json_usize_field(admission, "ledger_duplicate").unwrap_or(0),
        ledger_decayed: extract_json_usize_field(admission, "ledger_decayed").unwrap_or(0),
        ledger_merged: extract_json_usize_field(admission, "ledger_merged").unwrap_or(0),
        ledger_rollback: extract_json_usize_field(admission, "ledger_rollback").unwrap_or(0),
    })
}

#[derive(Debug, Clone, Copy, Default)]
struct KvFusionTraceGateSummary {
    events: usize,
    candidates: usize,
    fused: usize,
    compressed: usize,
    skipped: usize,
    held: usize,
    rejected: usize,
    approval_blocked: usize,
    input_tokens: usize,
    retained_tokens: usize,
    saved_tokens: usize,
}

fn kv_fusion_trace_gate_summary(line: &str) -> Option<KvFusionTraceGateSummary> {
    let fusion = json_object_after_field(line, "kv_fusion")?;

    Some(KvFusionTraceGateSummary {
        events: 1,
        candidates: extract_json_usize_field(fusion, "candidates").unwrap_or(0),
        fused: extract_json_usize_field(fusion, "fused").unwrap_or(0),
        compressed: extract_json_usize_field(fusion, "compressed").unwrap_or(0),
        skipped: extract_json_usize_field(fusion, "skipped").unwrap_or(0),
        held: extract_json_usize_field(fusion, "held").unwrap_or(0),
        rejected: extract_json_usize_field(fusion, "rejected").unwrap_or(0),
        approval_blocked: extract_json_usize_field(fusion, "approval_blocked").unwrap_or(0),
        input_tokens: extract_json_usize_field(fusion, "input_tokens").unwrap_or(0),
        retained_tokens: extract_json_usize_field(fusion, "retained_tokens").unwrap_or(0),
        saved_tokens: extract_json_usize_field(fusion, "saved_tokens").unwrap_or(0),
    })
}

fn trace_gate_milli(value: f32) -> usize {
    if value.is_finite() {
        (value.clamp(0.0, 1.0) * 1000.0).round() as usize
    } else {
        0
    }
}
