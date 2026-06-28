use std::fs;
use std::io;
use std::path::Path;

use super::evaluate_trace_schema_line;
use super::fields::{
    extract_json_bool_field, extract_json_f32_field, extract_json_nullable_f32_field,
    extract_json_nullable_u64_field, extract_json_string_array_field, extract_json_string_field,
    extract_json_usize_field, extract_last_json_string_array_field, json_escape,
    json_object_after_field, trace_note_bool,
};

pub const OPERATOR_HEALTH_SCHEMA: &str = "rust-norion-operator-health-v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorHealthMetric {
    pub name: &'static str,
    pub value: usize,
}

impl OperatorHealthMetric {
    fn new(name: &'static str, value: usize) -> Self {
        Self { name, value }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorHealthSection {
    pub name: &'static str,
    pub data_present: bool,
    pub ready: bool,
    pub review_required: bool,
    pub blocked: bool,
    pub events: usize,
    pub metrics: Vec<OperatorHealthMetric>,
}

impl OperatorHealthSection {
    fn new(
        name: &'static str,
        data_present: bool,
        review_required: bool,
        blocked: bool,
        events: usize,
        metrics: Vec<OperatorHealthMetric>,
        gate_passed: bool,
    ) -> Self {
        Self {
            name,
            data_present,
            ready: data_present && gate_passed && !review_required && !blocked,
            review_required,
            blocked,
            events,
            metrics,
        }
    }

    pub fn status(&self) -> &'static str {
        if !self.data_present {
            "missing"
        } else if self.blocked {
            "blocked"
        } else if self.review_required {
            "review_required"
        } else if self.ready {
            "ready"
        } else {
            "observed"
        }
    }

    pub fn metric(&self, name: &str) -> Option<usize> {
        self.metrics
            .iter()
            .find(|metric| metric.name == name)
            .map(|metric| metric.value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorHealthSnapshot {
    pub schema: &'static str,
    pub passed: bool,
    pub checked_lines: usize,
    pub failure_count: usize,
    pub trace_ids: Vec<u64>,
    pub sections: Vec<OperatorHealthSection>,
}

impl OperatorHealthSnapshot {
    pub fn section(&self, name: &str) -> Option<&OperatorHealthSection> {
        self.sections.iter().find(|section| section.name == name)
    }

    pub fn json_line(&self) -> String {
        let sections = self
            .sections
            .iter()
            .map(operator_health_section_json)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"schema\":\"{}\",\"passed\":{},\"checked_lines\":{},\"failure_count\":{},\"trace_id_count\":{},\"trace_ids\":{},\"sections\":[{}]}}",
            json_escape(self.schema),
            self.passed,
            self.checked_lines,
            self.failure_count,
            self.trace_ids.len(),
            u64_array_json(&self.trace_ids),
            sections
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SelfEvolutionOperatorApprovalServiceCounters {
    pub trace_gate_passed: bool,
    pub data_present: bool,
    pub approval_ready: bool,
    pub review_required: bool,
    pub blocked: bool,
    pub events: usize,
    pub approved: usize,
    pub held: usize,
    pub review_packets: usize,
    pub evidence_ids: usize,
    pub rollback_anchor_ids: usize,
    pub content_digests: usize,
    pub source_report_schemas: usize,
    pub missing_review_packet_refs: usize,
    pub write_allowed: usize,
    pub applied: usize,
    pub activation_allowed: bool,
    pub memory_write_allowed: bool,
    pub genome_write_allowed: bool,
    pub kv_write_allowed: bool,
}

impl SelfEvolutionOperatorApprovalServiceCounters {
    pub fn from_trace_gate_report(report: &TraceSchemaGateReport) -> Self {
        let mut counters = Self {
            trace_gate_passed: report.passed,
            data_present: report.self_evolution_operator_approval_events > 0,
            events: report.self_evolution_operator_approval_events,
            approved: report.self_evolution_operator_approval_approved,
            held: report.self_evolution_operator_approval_held,
            review_packets: report.self_evolution_operator_approval_review_packets,
            evidence_ids: report.self_evolution_operator_approval_evidence_ids,
            rollback_anchor_ids: report.self_evolution_operator_approval_rollback_anchor_ids,
            content_digests: report.self_evolution_operator_approval_content_digests,
            source_report_schemas: report.self_evolution_operator_approval_source_report_schemas,
            missing_review_packet_refs: report
                .self_evolution_operator_approval_missing_review_packet_refs,
            write_allowed: report.self_evolution_operator_approval_write_allowed,
            applied: report.self_evolution_operator_approval_applied,
            activation_allowed: false,
            memory_write_allowed: false,
            genome_write_allowed: false,
            kv_write_allowed: false,
            ..Self::default()
        };
        counters.blocked = counters.data_present && !counters.validation_failures().is_empty();
        counters.review_required = counters.data_present
            && (counters.held > 0 || counters.missing_review_packet_refs > 0 || counters.blocked);
        counters.approval_ready = counters.data_present
            && counters.approved > 0
            && !counters.review_required
            && !counters.blocked;
        counters
    }

    pub fn validation_failures(&self) -> Vec<String> {
        if !self.data_present {
            return Vec::new();
        }

        let mut failures = Vec::new();
        if !self.trace_gate_passed {
            failures.push("self_evolution_operator_approval_trace_gate_failed".to_owned());
        }
        if self.approved.saturating_add(self.held) != self.events {
            failures.push("self_evolution_operator_approval_decision_count_mismatch".to_owned());
        }
        if self.approved > 0 && self.review_packets == 0 {
            failures.push(
                "self_evolution_operator_approval_approved_missing_review_packets".to_owned(),
            );
        }
        if self.approved > 0 && self.evidence_ids == 0 {
            failures
                .push("self_evolution_operator_approval_approved_missing_evidence_ids".to_owned());
        }
        if self.approved > 0 && self.rollback_anchor_ids == 0 {
            failures.push(
                "self_evolution_operator_approval_approved_missing_rollback_anchors".to_owned(),
            );
        }
        if self.approved > 0 && self.content_digests == 0 {
            failures.push(
                "self_evolution_operator_approval_approved_missing_content_digests".to_owned(),
            );
        }
        if self.approved > 0 && self.source_report_schemas == 0 {
            failures.push(
                "self_evolution_operator_approval_approved_missing_source_report_schemas"
                    .to_owned(),
            );
        }
        if self.missing_review_packet_refs > 0 {
            failures.push("self_evolution_operator_approval_missing_review_packet_refs".to_owned());
        }
        if self.write_allowed > 0 {
            failures.push("self_evolution_operator_approval_write_allowed".to_owned());
        }
        if self.applied > 0 {
            failures.push("self_evolution_operator_approval_applied".to_owned());
        }
        if self.activation_allowed
            || self.memory_write_allowed
            || self.genome_write_allowed
            || self.kv_write_allowed
        {
            failures.push("self_evolution_operator_approval_service_write_capability".to_owned());
        }
        failures
    }

    pub fn summary_line(&self) -> String {
        format!(
            "self_evolution_operator_approval_service_counters: data_present={} approval_ready={} review_required={} blocked={} events={} approved={} held={} review_packets={} evidence_ids={} rollback_anchor_ids={} content_digests={} source_report_schemas={} missing_review_packet_refs={} write_allowed={} applied={} activation_allowed={} memory_write_allowed={} genome_write_allowed={} kv_write_allowed={} validation_failures={}",
            self.data_present,
            self.approval_ready,
            self.review_required,
            self.blocked,
            self.events,
            self.approved,
            self.held,
            self.review_packets,
            self.evidence_ids,
            self.rollback_anchor_ids,
            self.content_digests,
            self.source_report_schemas,
            self.missing_review_packet_refs,
            self.write_allowed,
            self.applied,
            self.activation_allowed,
            self.memory_write_allowed,
            self.genome_write_allowed,
            self.kv_write_allowed,
            self.validation_failures().len()
        )
    }

    pub fn json_object(&self) -> String {
        let validation_failures = self.validation_failures();
        format!(
            "{{\"trace_gate_passed\":{},\"data_present\":{},\"approval_ready\":{},\"review_required\":{},\"blocked\":{},\"events\":{},\"approved\":{},\"held\":{},\"review_packets\":{},\"evidence_ids\":{},\"rollback_anchor_ids\":{},\"content_digests\":{},\"source_report_schemas\":{},\"missing_review_packet_refs\":{},\"write_allowed\":{},\"applied\":{},\"activation_allowed\":{},\"memory_write_allowed\":{},\"genome_write_allowed\":{},\"kv_write_allowed\":{},\"validation_failures\":{},\"summary\":\"{}\"}}",
            self.trace_gate_passed,
            self.data_present,
            self.approval_ready,
            self.review_required,
            self.blocked,
            self.events,
            self.approved,
            self.held,
            self.review_packets,
            self.evidence_ids,
            self.rollback_anchor_ids,
            self.content_digests,
            self.source_report_schemas,
            self.missing_review_packet_refs,
            self.write_allowed,
            self.applied,
            self.activation_allowed,
            self.memory_write_allowed,
            self.genome_write_allowed,
            self.kv_write_allowed,
            string_array_json(&validation_failures),
            json_escape(&self.summary_line())
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TraceSchemaGateReport {
    pub passed: bool,
    pub checked_lines: usize,
    pub used_experiences: usize,
    pub imported_kv_blocks: usize,
    pub runtime_kv_weak_import_pressure_milli: usize,
    pub trace_experience_ids: Vec<u64>,
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
    pub business_contract_event_missing_handling_signals: usize,
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
    pub self_evolution_promotion_preflight_events: usize,
    pub self_evolution_promotion_preflight_ready: usize,
    pub self_evolution_promotion_preflight_held: usize,
    pub self_evolution_promotion_preflight_review_packets: usize,
    pub self_evolution_promotion_preflight_evidence_ids: usize,
    pub self_evolution_promotion_preflight_rollback_anchor_ids: usize,
    pub self_evolution_promotion_preflight_content_digests: usize,
    pub self_evolution_promotion_preflight_source_report_schemas: usize,
    pub self_evolution_promotion_preflight_missing_refs: usize,
    pub self_evolution_promotion_preflight_blocked_reasons: usize,
    pub self_evolution_promotion_preflight_write_allowed: usize,
    pub self_evolution_promotion_preflight_applied: usize,
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
    pub self_evolving_memory_store_consolidation_events: usize,
    pub self_evolving_memory_store_consolidation_actions: usize,
    pub self_evolving_memory_store_merge_previews: usize,
    pub self_evolving_memory_store_decay_previews: usize,
    pub self_evolving_memory_store_tombstone_previews: usize,
    pub self_evolving_memory_store_merge_rejections: usize,
    pub self_evolving_memory_store_contexts: usize,
    pub self_evolving_memory_store_saved_tokens: usize,
    pub self_evolving_memory_store_maintenance_actions: usize,
    pub self_evolving_memory_store_admission_candidates: usize,
    pub self_evolving_memory_store_write_allowed: usize,
    pub self_evolving_memory_store_durable_write_allowed: usize,
    pub self_evolving_memory_store_applied: usize,
    pub self_evolving_memory_store_applied_to_disk: usize,
    pub self_evolving_memory_store_source_quarantine_events: usize,
    pub self_evolving_memory_store_source_quarantine_actions: usize,
    pub self_evolving_memory_writeback_events: usize,
    pub self_evolving_memory_writeback_source_case_digests: usize,
    pub self_evolving_memory_writeback_attempted_records: usize,
    pub self_evolving_memory_writeback_accepted_records: usize,
    pub self_evolving_memory_writeback_records_before: usize,
    pub self_evolving_memory_writeback_records_after: usize,
    pub self_evolving_memory_writeback_tool_reliability_after: usize,
    pub self_evolving_memory_writeback_tool_observations_after: usize,
    pub self_evolving_memory_writeback_maintenance_actions: usize,
    pub self_evolving_memory_writeback_merged_duplicate_episodes: usize,
    pub self_evolving_memory_writeback_write_allowed: usize,
    pub self_evolving_memory_writeback_durable_write_allowed: usize,
    pub self_evolving_memory_writeback_applied: usize,
    pub self_evolving_memory_writeback_applied_to_disk: usize,
    pub self_evolving_memory_writeback_snapshot_changes: usize,
    pub memory_runtime_kv_exported: usize,
    pub memory_runtime_kv_stored: usize,
    pub memory_runtime_kv_hold: usize,
    pub memory_runtime_kv_held: usize,
    pub memory_feedback_reinforced: usize,
    pub memory_feedback_penalized: usize,
    pub memory_feedback_reinforcement_milli: usize,
    pub memory_feedback_penalty_milli: usize,
    pub memory_feedback_updates: usize,
    pub memory_feedback_applied: usize,
    pub memory_feedback_removed: usize,
    pub memory_feedback_missing: usize,
    pub memory_feedback_strength_delta_milli: usize,
    pub memory_residency_events: usize,
    pub memory_residency_decisions: usize,
    pub memory_residency_hot: usize,
    pub memory_residency_warm: usize,
    pub memory_residency_cold: usize,
    pub memory_residency_quarantined: usize,
    pub memory_residency_retired: usize,
    pub memory_residency_protected_rollback_anchors: usize,
    pub memory_residency_blocked_reasons: usize,
    pub memory_residency_token_estimate: usize,
    pub memory_residency_write_allowed: usize,
    pub memory_residency_durable_write_allowed: usize,
    pub memory_residency_applied: usize,
    pub unified_writer_gate_events: usize,
    pub unified_writer_gate_records: usize,
    pub unified_writer_gate_memory_records: usize,
    pub unified_writer_gate_genome_records: usize,
    pub unified_writer_gate_experiment_ledger_records: usize,
    pub unified_writer_gate_evolution_goal_queue_records: usize,
    pub unified_writer_gate_ready_records: usize,
    pub unified_writer_gate_held_records: usize,
    pub unified_writer_gate_rejected_records: usize,
    pub unified_writer_gate_preview_only_records: usize,
    pub unified_writer_gate_reason_codes: usize,
    pub unified_writer_gate_explicit_apply_required: usize,
    pub unified_writer_gate_write_allowed: usize,
    pub unified_writer_gate_durable_write_allowed: usize,
    pub unified_writer_gate_applied: usize,
    pub self_goal_queue_apply_events: usize,
    pub self_goal_queue_apply_records: usize,
    pub self_goal_queue_apply_ready_records: usize,
    pub self_goal_queue_apply_held_records: usize,
    pub self_goal_queue_apply_rejected_records: usize,
    pub self_goal_queue_apply_reason_codes: usize,
    pub self_goal_queue_apply_explicit_apply_required: usize,
    pub self_goal_queue_apply_write_allowed: usize,
    pub self_goal_queue_apply_applied: usize,
    pub self_goal_queue_continuation_events: usize,
    pub self_goal_queue_continuation_ready: usize,
    pub self_goal_queue_continuation_held: usize,
    pub self_goal_queue_continuation_current_queue: usize,
    pub self_goal_queue_continuation_completion_resulting_queue: usize,
    pub self_goal_queue_continuation_goals: usize,
    pub self_goal_queue_continuation_required_evidence: usize,
    pub self_goal_queue_continuation_reason_codes: usize,
    pub self_goal_queue_continuation_budget_attempts: usize,
    pub self_goal_queue_continuation_budget_steps: usize,
    pub self_goal_queue_continuation_budget_tokens: usize,
    pub self_goal_queue_continuation_budget_runtime_ms: usize,
    pub self_goal_queue_continuation_write_allowed: usize,
    pub self_goal_queue_continuation_applied: usize,
    pub self_goal_queue_evidence_plan_events: usize,
    pub self_goal_queue_evidence_plan_ready: usize,
    pub self_goal_queue_evidence_plan_held: usize,
    pub self_goal_queue_evidence_plan_steps: usize,
    pub self_goal_queue_evidence_plan_auto_collectible: usize,
    pub self_goal_queue_evidence_plan_manual: usize,
    pub self_goal_queue_evidence_plan_required_evidence: usize,
    pub self_goal_queue_evidence_plan_packet_templates: usize,
    pub self_goal_queue_evidence_plan_command_templates: usize,
    pub self_goal_queue_evidence_plan_write_allowed: usize,
    pub self_goal_queue_evidence_plan_applied: usize,
    pub self_goal_queue_evidence_collection_events: usize,
    pub self_goal_queue_evidence_collection_ready: usize,
    pub self_goal_queue_evidence_collection_complete: usize,
    pub self_goal_queue_evidence_collection_steps: usize,
    pub self_goal_queue_evidence_collection_collected: usize,
    pub self_goal_queue_evidence_collection_passed: usize,
    pub self_goal_queue_evidence_collection_failed: usize,
    pub self_goal_queue_evidence_collection_missing: usize,
    pub self_goal_queue_evidence_collection_manual_missing: usize,
    pub self_goal_queue_evidence_collection_write_allowed: usize,
    pub self_goal_queue_evidence_collection_applied: usize,
    pub self_goal_local_evidence_events: usize,
    pub self_goal_local_evidence_enabled: usize,
    pub self_goal_local_evidence_dry_run: usize,
    pub self_goal_local_evidence_ready: usize,
    pub self_goal_local_evidence_steps: usize,
    pub self_goal_local_evidence_attempted: usize,
    pub self_goal_local_evidence_generated: usize,
    pub self_goal_local_evidence_passed: usize,
    pub self_goal_local_evidence_failed: usize,
    pub self_goal_local_evidence_skipped: usize,
    pub self_goal_local_evidence_manual: usize,
    pub self_goal_local_evidence_planned_status: usize,
    pub self_goal_local_evidence_write_allowed: usize,
    pub self_goal_local_evidence_applied: usize,
    pub coding_service_eval_events: usize,
    pub coding_service_eval_readiness_events: usize,
    pub coding_service_eval_runner_events: usize,
    pub coding_service_eval_passed: usize,
    pub coding_service_eval_requests: usize,
    pub coding_service_eval_completed: usize,
    pub coding_service_eval_evidence_packets: usize,
    pub coding_service_eval_rust_validation_checked: usize,
    pub coding_service_eval_compile_checked: usize,
    pub coding_service_eval_unit_test_checked: usize,
    pub coding_service_eval_write_allowed: usize,
    pub coding_service_eval_applied: usize,
    pub evolution_goal_queue_store_write_events: usize,
    pub evolution_goal_queue_store_write_applied: usize,
    pub evolution_goal_queue_store_write_held: usize,
    pub evolution_goal_queue_store_write_rejected: usize,
    pub evolution_goal_queue_store_write_reason_codes: usize,
    pub evolution_goal_queue_store_write_durable_write_allowed: usize,
    pub evolution_goal_queue_store_write_applied_to_disk: usize,
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
    pub task_hierarchy_depth_total: usize,
    pub task_hierarchy_route_fanout_total: usize,
    pub task_hierarchy_threshold_delta_milli: usize,
    pub task_hierarchy_selected_lanes: usize,
    pub task_hierarchy_skipped_lanes: usize,
    pub task_hierarchy_memory_lanes: usize,
    pub task_hierarchy_skipped_memory_lanes: usize,
    pub fht_dke_events: usize,
    pub fht_dke_enabled: usize,
    pub fht_dke_total_tokens: usize,
    pub fht_dke_dense_tokens: usize,
    pub fht_dke_routed_tokens: usize,
    pub fht_dke_kv_exchange_blocks: usize,
    pub fht_dke_token_split_invalid: usize,
    pub fht_dke_attention_threshold_milli: usize,
    pub fht_dke_route_pressure_milli: usize,
    pub compute_budget_events: usize,
    pub compute_budget_threshold_delta_milli: usize,
    pub compute_budget_runtime_kv_budget_pressure_milli: usize,
    pub compute_budget_low: usize,
    pub compute_budget_normal: usize,
    pub compute_budget_expanded: usize,
    pub compute_budget_selected_candidates: usize,
    pub compute_budget_low_value_skipped: usize,
    pub compute_budget_kv_lookups_skipped: usize,
    pub compute_budget_validation_cost_tokens: usize,
    pub compute_budget_saved_tokens: usize,
    pub compute_budget_self_evolving_memory_fusion_saved_tokens: usize,
    pub compute_budget_avoided_tokens: usize,
    pub compute_budget_fanout_before: usize,
    pub compute_budget_fanout_after: usize,
    pub compute_budget_fanout_reduction: usize,
    pub compute_budget_estimated_budget_tokens: usize,
    pub compute_budget_estimated_spent_tokens: usize,
    pub compute_budget_estimated_saved_tokens: usize,
    pub compute_budget_anchor_count: usize,
    pub compute_budget_anchors_preserved: usize,
    pub compute_budget_anchor_preservation_failures: usize,
    pub compute_budget_fallback_triggered: usize,
    pub compute_budget_write_allowed: usize,
    pub compute_budget_applied: usize,
    pub auto_replay_recursive_runtime_items: usize,
    pub auto_replay_recursive_runtime_calls: usize,
    pub auto_replay_avg_recursive_call_pressure_milli: usize,
    pub auto_replay_max_recursive_call_pressure_milli: usize,
    pub evolution_recursive_replay_items: usize,
    pub evolution_recursive_runtime_calls: usize,
    pub process_reward_events: usize,
    pub process_reward_positive: usize,
    pub process_reward_reinforce: usize,
    pub process_reward_hold: usize,
    pub process_reward_penalize: usize,
    pub process_reward_total_milli: usize,
    pub live_evolution_events: usize,
    pub live_router_threshold_delta_milli: usize,
    pub live_hierarchy_weight_delta_milli: usize,
    pub live_online_reward_feedbacks: usize,
    pub live_online_reward_reinforcements: usize,
    pub live_online_reward_penalties: usize,
    pub live_online_reward_strength_milli: usize,
    pub live_memory_reinforcements: usize,
    pub live_memory_penalties: usize,
    pub live_memory_updates: usize,
    pub live_stored_memories: usize,
    pub live_stored_gist_memories: usize,
    pub live_stored_runtime_kv_memories: usize,
    pub live_stored_memory_updates: usize,
    pub live_reflection_issues: usize,
    pub live_critical_reflection_issues: usize,
    pub live_revision_actions: usize,
    pub evolution_live_inference_runs: usize,
    pub evolution_live_router_threshold_mutations: usize,
    pub evolution_live_hierarchy_weight_mutations: usize,
    pub evolution_live_router_threshold_delta_milli: usize,
    pub evolution_live_hierarchy_weight_delta_milli: usize,
    pub evolution_live_online_reward_feedbacks: usize,
    pub evolution_live_online_reward_reinforcements: usize,
    pub evolution_live_online_reward_penalties: usize,
    pub evolution_live_online_reward_strength_milli: usize,
    pub evolution_live_online_reward_reinforcement_strength_milli: usize,
    pub evolution_live_online_reward_penalty_strength_milli: usize,
    pub evolution_live_memory_reinforcements: usize,
    pub evolution_live_memory_penalties: usize,
    pub evolution_live_memory_updates: usize,
    pub evolution_live_stored_memories: usize,
    pub evolution_live_stored_gist_memories: usize,
    pub evolution_live_stored_runtime_kv_memories: usize,
    pub evolution_live_stored_memory_updates: usize,
    pub evolution_live_reflection_issues: usize,
    pub evolution_live_critical_reflection_issues: usize,
    pub evolution_live_revision_actions: usize,
    pub auto_replay_live_memory_feedback_items: usize,
    pub auto_replay_live_memory_feedback_updates: usize,
    pub auto_replay_live_memory_feedback_reinforcements: usize,
    pub auto_replay_live_memory_feedback_penalties: usize,
    pub auto_replay_live_memory_feedback_detail_items: usize,
    pub auto_replay_live_memory_feedback_applied: usize,
    pub auto_replay_live_memory_feedback_removed: usize,
    pub auto_replay_live_memory_feedback_missing: usize,
    pub auto_replay_live_memory_feedback_strength_delta_milli: usize,
    pub replay_live_memory_feedback_items: usize,
    pub replay_live_memory_feedback_updates: usize,
    pub replay_live_memory_feedback_reinforcements: usize,
    pub replay_live_memory_feedback_penalties: usize,
    pub replay_live_memory_feedback_detail_items: usize,
    pub replay_live_memory_feedback_applied: usize,
    pub replay_live_memory_feedback_removed: usize,
    pub replay_live_memory_feedback_missing: usize,
    pub replay_live_memory_feedback_strength_delta_milli: usize,
    pub replay_live_evolution_items: usize,
    pub replay_live_evolution_router_threshold_mutations: usize,
    pub replay_live_evolution_hierarchy_weight_mutations: usize,
    pub replay_live_evolution_router_threshold_delta_milli: usize,
    pub replay_live_evolution_hierarchy_weight_delta_milli: usize,
    pub replay_live_evolution_online_reward_feedbacks: usize,
    pub replay_live_evolution_online_reward_reinforcements: usize,
    pub replay_live_evolution_online_reward_penalties: usize,
    pub replay_live_evolution_online_reward_strength_milli: usize,
    pub replay_live_evolution_online_reward_reinforcement_strength_milli: usize,
    pub replay_live_evolution_online_reward_penalty_strength_milli: usize,
    pub replay_live_evolution_memory_updates: usize,
    pub replay_live_evolution_stored_memory_updates: usize,
    pub replay_live_evolution_reflection_issues: usize,
    pub replay_live_evolution_critical_reflection_issues: usize,
    pub replay_live_evolution_revision_actions: usize,
    pub reasoning_genome_events: usize,
    pub reasoning_genome_genes: usize,
    pub reasoning_genome_active_genes: usize,
    pub reasoning_genome_aged_genes: usize,
    pub reasoning_genome_malignant_genes: usize,
    pub reasoning_genome_relabel_candidates: usize,
    pub reasoning_genome_regeneration_candidates: usize,
    pub reasoning_genome_gene_scissors_proposals: usize,
    pub reasoning_genome_repair_payloads: usize,
    pub reasoning_genome_regeneration_payloads: usize,
    pub reasoning_genome_lifecycle_records: usize,
    pub reasoning_genome_lifecycle_tombstone_candidates: usize,
    pub reasoning_genome_lifecycle_pending_validations: usize,
    pub reasoning_genome_lifecycle_source_evidence: usize,
    pub reasoning_genome_splice_segments: usize,
    pub reasoning_genome_splice_exons: usize,
    pub reasoning_genome_splice_introns: usize,
    pub reasoning_genome_splice_variants: usize,
    pub reasoning_genome_splice_quarantined: usize,
    pub reasoning_genome_splice_repair_candidates: usize,
    pub reasoning_genome_splice_findings: usize,
    pub reasoning_genome_splice_proposals: usize,
    pub reasoning_genome_write_allowed: usize,
    pub reasoning_genome_mutation_applied: usize,
    pub reasoning_genome_splice_write_allowed: usize,
    pub reasoning_genome_splice_applied: usize,
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
    pub noiron_orchestration_events: usize,
    pub noiron_orchestration_stages: usize,
    pub noiron_orchestration_failed_stages: usize,
    pub noiron_orchestration_writes_gated: usize,
    pub noiron_orchestration_fht_dke_total_tokens: usize,
    pub orchestration_audit_events: usize,
    pub orchestration_audit_checked_fields: usize,
    pub orchestration_audit_failed_fields: usize,
    pub orchestration_audit_failed_stages: usize,
    pub orchestration_audit_integrity_failed_fields: usize,
    pub failures: Vec<String>,
}

impl TraceSchemaGateReport {
    pub fn self_evolving_memory_writeback_rejected_records(&self) -> usize {
        self.self_evolving_memory_writeback_attempted_records
            .saturating_sub(self.self_evolving_memory_writeback_accepted_records)
    }

    pub fn self_evolution_operator_approval_service_counters(
        &self,
    ) -> SelfEvolutionOperatorApprovalServiceCounters {
        SelfEvolutionOperatorApprovalServiceCounters::from_trace_gate_report(self)
    }

    pub fn summary_line(&self) -> String {
        let base = format!(
            "trace_schema_gate: passed={} lines={} failures={} used_experiences={} imported_kv_blocks={} runtime_kv_weak_import_pressure_milli={} rust_check_events={} rust_check_passed={} rust_check_failed={} rust_check_feedback_updates={} rust_check_feedback_applied={} business_contract_events={} business_contract_event_passed={} business_contract_event_failed={} business_contract_event_missing_signals={} business_contract_event_protocol_leaks={} business_contract_event_substitutions={} business_contract_event_evasive_denials={} business_contract_event_missing_handling_signals={} business_contract_event_raw_passed={} business_contract_event_raw_failed={} business_contract_event_response_normalized={} business_contract_event_sanitized={} business_contract_event_canonical_fallbacks={} runtime_error_events={} runtime_timeout_events={} self_evolution_admission_events={} self_evolution_admission_admitted={} self_evolution_admission_blocked={} self_evolution_admission_review_packets={} self_evolution_admission_evidence_ids={} self_evolution_admission_missing_review_packet_refs={} self_evolution_experiment_events={} self_evolution_experiment_admit={} self_evolution_experiment_hold={} self_evolution_experiment_reject={} self_evolution_experiment_rollback={} self_evolution_experiment_repeated={} self_evolution_experiment_conflicts={} self_evolution_experiment_rollback_replayable={} self_evolution_experiment_active_candidates={} self_evolution_experiment_write_allowed={} self_evolution_experiment_applied={} self_evolution_rollback_replay_events={} self_evolution_rollback_replay_items={} self_evolution_rollback_replay_replayable={} self_evolution_rollback_replay_blocked={} self_evolution_rollback_replay_all_replayable={} self_evolution_rollback_replay_rollback_anchor_ids={} self_evolution_rollback_replay_evidence_ids={} self_evolution_rollback_replay_active_candidates={} self_evolution_rollback_replay_item_write_allowed={} self_evolution_rollback_replay_item_applied={} self_evolution_rollback_replay_write_allowed={} self_evolution_rollback_replay_applied={} self_evolution_rollback_replay_gate_events={} self_evolution_rollback_replay_gate_admitted={} self_evolution_rollback_replay_gate_held={} self_evolution_rollback_replay_gate_review_packets={} self_evolution_rollback_replay_gate_review_evidence_ids={} self_evolution_rollback_replay_gate_missing_review_packet_refs={} self_evolution_rollback_replay_gate_items={} self_evolution_rollback_replay_gate_replayable={} self_evolution_rollback_replay_gate_blocked={} self_evolution_rollback_replay_gate_all_replayable={} self_evolution_rollback_replay_gate_rollback_anchor_ids={} self_evolution_rollback_replay_gate_evidence_ids={} self_evolution_rollback_replay_gate_active_candidates={} self_evolution_rollback_replay_gate_item_write_allowed={} self_evolution_rollback_replay_gate_item_applied={} self_evolution_rollback_replay_gate_plan_write_allowed={} self_evolution_rollback_replay_gate_plan_applied={} self_evolution_rollback_replay_gate_write_allowed={} self_evolution_rollback_replay_gate_applied={} self_evolution_operator_approval_events={} self_evolution_operator_approval_approved={} self_evolution_operator_approval_held={} self_evolution_operator_approval_review_packets={} self_evolution_operator_approval_evidence_ids={} self_evolution_operator_approval_rollback_anchor_ids={} self_evolution_operator_approval_content_digests={} self_evolution_operator_approval_source_report_schemas={} self_evolution_operator_approval_missing_review_packet_refs={} self_evolution_operator_approval_write_allowed={} self_evolution_operator_approval_applied={} self_evolution_promotion_preflight_events={} self_evolution_promotion_preflight_ready={} self_evolution_promotion_preflight_held={} self_evolution_promotion_preflight_review_packets={} self_evolution_promotion_preflight_evidence_ids={} self_evolution_promotion_preflight_rollback_anchor_ids={} self_evolution_promotion_preflight_content_digests={} self_evolution_promotion_preflight_source_report_schemas={} self_evolution_promotion_preflight_missing_refs={} self_evolution_promotion_preflight_blocked_reasons={} self_evolution_promotion_preflight_write_allowed={} self_evolution_promotion_preflight_applied={} improvement_corpus_events={} improvement_corpus_episodes={} improvement_corpus_active_adaptation={} improvement_corpus_compiler_passed={} improvement_corpus_test_passed={} improvement_corpus_benchmark_passed={} improvement_corpus_privacy_rejected={} improvement_corpus_secret_leaks={} adaptive_routing_events={} adaptive_routing_candidates={} adaptive_routing_include={} adaptive_routing_compress={} adaptive_routing_defer={} adaptive_routing_skip={} adaptive_routing_input_tokens={} adaptive_routing_retained_tokens={} adaptive_routing_saved_tokens={} task_hierarchy_events={} task_hierarchy_mutation_records={} task_hierarchy_route_pressure_milli={} task_hierarchy_compute_reduction_milli={} task_hierarchy_depth_total={} task_hierarchy_route_fanout_total={} task_hierarchy_threshold_delta_milli={} task_hierarchy_selected_lanes={} task_hierarchy_skipped_lanes={} task_hierarchy_memory_lanes={} task_hierarchy_skipped_memory_lanes={} compute_budget_events={} compute_budget_threshold_delta_milli={} compute_budget_runtime_kv_budget_pressure_milli={} compute_budget_low={} compute_budget_normal={} compute_budget_expanded={} compute_budget_selected_candidates={} compute_budget_low_value_skipped={} compute_budget_kv_lookups_skipped={} compute_budget_validation_cost_tokens={} compute_budget_saved_tokens={} compute_budget_self_evolving_memory_fusion_saved_tokens={} compute_budget_avoided_tokens={} compute_budget_fanout_before={} compute_budget_fanout_after={} compute_budget_fanout_reduction={} compute_budget_estimated_budget_tokens={} compute_budget_estimated_spent_tokens={} compute_budget_estimated_saved_tokens={} compute_budget_anchor_count={} compute_budget_anchors_preserved={} compute_budget_anchor_preservation_failures={} compute_budget_fallback_triggered={} compute_budget_write_allowed={} compute_budget_applied={} memory_admission_events={} memory_admission_candidates={} memory_admission_ready={} memory_admission_blocked={} memory_admission_admitted={} memory_admission_hold={} memory_admission_reject={} memory_admission_quarantine={} memory_admission_review_packets={} memory_admission_ledger_records={} memory_admission_ledger_authorized={} memory_admission_ledger_applied={} memory_admission_ledger_preview_only={} memory_admission_ledger_held={} memory_admission_ledger_rejected={} memory_admission_ledger_duplicate={} memory_admission_ledger_decayed={} memory_admission_ledger_merged={} memory_admission_ledger_rollback={} kv_fusion_events={} kv_fusion_candidates={} kv_fusion_fused={} kv_fusion_compressed={} kv_fusion_skipped={} kv_fusion_held={} kv_fusion_rejected={} kv_fusion_approval_blocked={} kv_fusion_input_tokens={} kv_fusion_retained_tokens={} kv_fusion_saved_tokens={}",
            self.passed,
            self.checked_lines,
            self.failures.len(),
            self.used_experiences,
            self.imported_kv_blocks,
            self.runtime_kv_weak_import_pressure_milli,
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
            self.business_contract_event_missing_handling_signals,
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
            self.self_evolution_promotion_preflight_events,
            self.self_evolution_promotion_preflight_ready,
            self.self_evolution_promotion_preflight_held,
            self.self_evolution_promotion_preflight_review_packets,
            self.self_evolution_promotion_preflight_evidence_ids,
            self.self_evolution_promotion_preflight_rollback_anchor_ids,
            self.self_evolution_promotion_preflight_content_digests,
            self.self_evolution_promotion_preflight_source_report_schemas,
            self.self_evolution_promotion_preflight_missing_refs,
            self.self_evolution_promotion_preflight_blocked_reasons,
            self.self_evolution_promotion_preflight_write_allowed,
            self.self_evolution_promotion_preflight_applied,
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
            self.task_hierarchy_depth_total,
            self.task_hierarchy_route_fanout_total,
            self.task_hierarchy_threshold_delta_milli,
            self.task_hierarchy_selected_lanes,
            self.task_hierarchy_skipped_lanes,
            self.task_hierarchy_memory_lanes,
            self.task_hierarchy_skipped_memory_lanes,
            self.compute_budget_events,
            self.compute_budget_threshold_delta_milli,
            self.compute_budget_runtime_kv_budget_pressure_milli,
            self.compute_budget_low,
            self.compute_budget_normal,
            self.compute_budget_expanded,
            self.compute_budget_selected_candidates,
            self.compute_budget_low_value_skipped,
            self.compute_budget_kv_lookups_skipped,
            self.compute_budget_validation_cost_tokens,
            self.compute_budget_saved_tokens,
            self.compute_budget_self_evolving_memory_fusion_saved_tokens,
            self.compute_budget_avoided_tokens,
            self.compute_budget_fanout_before,
            self.compute_budget_fanout_after,
            self.compute_budget_fanout_reduction,
            self.compute_budget_estimated_budget_tokens,
            self.compute_budget_estimated_spent_tokens,
            self.compute_budget_estimated_saved_tokens,
            self.compute_budget_anchor_count,
            self.compute_budget_anchors_preserved,
            self.compute_budget_anchor_preservation_failures,
            self.compute_budget_fallback_triggered,
            self.compute_budget_write_allowed,
            self.compute_budget_applied,
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
        let extended = format!(
            "{base} fht_dke_events={} fht_dke_enabled={} fht_dke_total_tokens={} fht_dke_dense_tokens={} fht_dke_routed_tokens={} fht_dke_kv_exchange_blocks={} fht_dke_token_split_invalid={} fht_dke_attention_threshold_milli={} fht_dke_route_pressure_milli={} noiron_orchestration_events={} noiron_orchestration_stages={} noiron_orchestration_failed_stages={} noiron_orchestration_writes_gated={} noiron_orchestration_fht_dke_total_tokens={} self_evolution_rollback_replay_apply_events={} self_evolution_rollback_replay_apply_ready={} self_evolution_rollback_replay_apply_held={} self_evolution_rollback_replay_apply_items={} self_evolution_rollback_replay_apply_replayable={} self_evolution_rollback_replay_apply_blocked={} self_evolution_rollback_replay_apply_review_packets={} self_evolution_rollback_replay_apply_evidence_ids={} self_evolution_rollback_replay_apply_rollback_anchor_ids={} self_evolution_rollback_replay_apply_content_digests={} self_evolution_rollback_replay_apply_source_report_schemas={} self_evolution_rollback_replay_apply_missing_refs={} self_evolution_rollback_replay_apply_blocked_reasons={} self_evolution_rollback_replay_apply_write_allowed={} self_evolution_rollback_replay_apply_applied={} self_evolving_memory_store_events={} self_evolving_memory_store_retrieval_events={} self_evolving_memory_store_maintenance_events={} self_evolving_memory_store_admission_preview_events={} self_evolving_memory_store_consolidation_events={} self_evolving_memory_store_consolidation_actions={} self_evolving_memory_store_merge_previews={} self_evolving_memory_store_decay_previews={} self_evolving_memory_store_tombstone_previews={} self_evolving_memory_store_merge_rejections={} self_evolving_memory_store_contexts={} self_evolving_memory_store_saved_tokens={} self_evolving_memory_store_maintenance_actions={} self_evolving_memory_store_admission_candidates={} self_evolving_memory_store_write_allowed={} self_evolving_memory_store_durable_write_allowed={} self_evolving_memory_store_applied={} self_evolving_memory_store_applied_to_disk={} self_evolving_memory_store_source_quarantine_events={} self_evolving_memory_store_source_quarantine_actions={} self_evolving_memory_writeback_events={} self_evolving_memory_writeback_source_case_digests={} self_evolving_memory_writeback_attempted_records={} self_evolving_memory_writeback_accepted_records={} self_evolving_memory_writeback_records_before={} self_evolving_memory_writeback_records_after={} self_evolving_memory_writeback_tool_reliability_after={} self_evolving_memory_writeback_tool_observations_after={} self_evolving_memory_writeback_maintenance_actions={} self_evolving_memory_writeback_merged_duplicate_episodes={} self_evolving_memory_writeback_write_allowed={} self_evolving_memory_writeback_durable_write_allowed={} self_evolving_memory_writeback_applied={} self_evolving_memory_writeback_applied_to_disk={} self_evolving_memory_writeback_snapshot_changes={} memory_runtime_kv_exported={} memory_runtime_kv_stored={} memory_runtime_kv_hold={} memory_runtime_kv_held={} memory_feedback_reinforced={} memory_feedback_penalized={} memory_feedback_reinforcement_milli={} memory_feedback_penalty_milli={} memory_feedback_updates={} memory_feedback_applied={} memory_feedback_removed={} memory_feedback_missing={} memory_feedback_strength_delta_milli={} memory_residency_events={} memory_residency_decisions={} memory_residency_hot={} memory_residency_warm={} memory_residency_cold={} memory_residency_quarantined={} memory_residency_retired={} memory_residency_protected_rollback_anchors={} memory_residency_blocked_reasons={} memory_residency_token_estimate={} memory_residency_write_allowed={} memory_residency_durable_write_allowed={} memory_residency_applied={} unified_writer_gate_events={} unified_writer_gate_records={} unified_writer_gate_memory_records={} unified_writer_gate_genome_records={} unified_writer_gate_experiment_ledger_records={} unified_writer_gate_evolution_goal_queue_records={} unified_writer_gate_ready_records={} unified_writer_gate_held_records={} unified_writer_gate_rejected_records={} unified_writer_gate_preview_only_records={} unified_writer_gate_reason_codes={} unified_writer_gate_explicit_apply_required={} unified_writer_gate_write_allowed={} unified_writer_gate_durable_write_allowed={} unified_writer_gate_applied={} self_goal_queue_apply_events={} self_goal_queue_apply_records={} self_goal_queue_apply_ready_records={} self_goal_queue_apply_held_records={} self_goal_queue_apply_rejected_records={} self_goal_queue_apply_reason_codes={} self_goal_queue_apply_explicit_apply_required={} self_goal_queue_apply_write_allowed={} self_goal_queue_apply_applied={} self_goal_queue_continuation_events={} self_goal_queue_continuation_ready={} self_goal_queue_continuation_held={} self_goal_queue_continuation_current_queue={} self_goal_queue_continuation_completion_resulting_queue={} self_goal_queue_continuation_goals={} self_goal_queue_continuation_required_evidence={} self_goal_queue_continuation_reason_codes={} self_goal_queue_continuation_budget_attempts={} self_goal_queue_continuation_budget_steps={} self_goal_queue_continuation_budget_tokens={} self_goal_queue_continuation_budget_runtime_ms={} self_goal_queue_continuation_write_allowed={} self_goal_queue_continuation_applied={} self_goal_queue_evidence_plan_events={} self_goal_queue_evidence_plan_ready={} self_goal_queue_evidence_plan_held={} self_goal_queue_evidence_plan_steps={} self_goal_queue_evidence_plan_auto_collectible={} self_goal_queue_evidence_plan_manual={} self_goal_queue_evidence_plan_required_evidence={} self_goal_queue_evidence_plan_packet_templates={} self_goal_queue_evidence_plan_command_templates={} self_goal_queue_evidence_plan_write_allowed={} self_goal_queue_evidence_plan_applied={} self_goal_queue_evidence_collection_events={} self_goal_queue_evidence_collection_ready={} self_goal_queue_evidence_collection_complete={} self_goal_queue_evidence_collection_steps={} self_goal_queue_evidence_collection_collected={} self_goal_queue_evidence_collection_passed={} self_goal_queue_evidence_collection_failed={} self_goal_queue_evidence_collection_missing={} self_goal_queue_evidence_collection_manual_missing={} self_goal_queue_evidence_collection_write_allowed={} self_goal_queue_evidence_collection_applied={} self_goal_local_evidence_events={} self_goal_local_evidence_enabled={} self_goal_local_evidence_dry_run={} self_goal_local_evidence_ready={} self_goal_local_evidence_steps={} self_goal_local_evidence_attempted={} self_goal_local_evidence_generated={} self_goal_local_evidence_passed={} self_goal_local_evidence_failed={} self_goal_local_evidence_skipped={} self_goal_local_evidence_manual={} self_goal_local_evidence_planned_status={} self_goal_local_evidence_write_allowed={} self_goal_local_evidence_applied={} evolution_goal_queue_store_write_events={} evolution_goal_queue_store_write_applied={} evolution_goal_queue_store_write_held={} evolution_goal_queue_store_write_rejected={} evolution_goal_queue_store_write_reason_codes={} evolution_goal_queue_store_write_durable_write_allowed={} evolution_goal_queue_store_write_applied_to_disk={}",
            self.fht_dke_events,
            self.fht_dke_enabled,
            self.fht_dke_total_tokens,
            self.fht_dke_dense_tokens,
            self.fht_dke_routed_tokens,
            self.fht_dke_kv_exchange_blocks,
            self.fht_dke_token_split_invalid,
            self.fht_dke_attention_threshold_milli,
            self.fht_dke_route_pressure_milli,
            self.noiron_orchestration_events,
            self.noiron_orchestration_stages,
            self.noiron_orchestration_failed_stages,
            self.noiron_orchestration_writes_gated,
            self.noiron_orchestration_fht_dke_total_tokens,
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
            self.self_evolving_memory_store_consolidation_events,
            self.self_evolving_memory_store_consolidation_actions,
            self.self_evolving_memory_store_merge_previews,
            self.self_evolving_memory_store_decay_previews,
            self.self_evolving_memory_store_tombstone_previews,
            self.self_evolving_memory_store_merge_rejections,
            self.self_evolving_memory_store_contexts,
            self.self_evolving_memory_store_saved_tokens,
            self.self_evolving_memory_store_maintenance_actions,
            self.self_evolving_memory_store_admission_candidates,
            self.self_evolving_memory_store_write_allowed,
            self.self_evolving_memory_store_durable_write_allowed,
            self.self_evolving_memory_store_applied,
            self.self_evolving_memory_store_applied_to_disk,
            self.self_evolving_memory_store_source_quarantine_events,
            self.self_evolving_memory_store_source_quarantine_actions,
            self.self_evolving_memory_writeback_events,
            self.self_evolving_memory_writeback_source_case_digests,
            self.self_evolving_memory_writeback_attempted_records,
            self.self_evolving_memory_writeback_accepted_records,
            self.self_evolving_memory_writeback_records_before,
            self.self_evolving_memory_writeback_records_after,
            self.self_evolving_memory_writeback_tool_reliability_after,
            self.self_evolving_memory_writeback_tool_observations_after,
            self.self_evolving_memory_writeback_maintenance_actions,
            self.self_evolving_memory_writeback_merged_duplicate_episodes,
            self.self_evolving_memory_writeback_write_allowed,
            self.self_evolving_memory_writeback_durable_write_allowed,
            self.self_evolving_memory_writeback_applied,
            self.self_evolving_memory_writeback_applied_to_disk,
            self.self_evolving_memory_writeback_snapshot_changes,
            self.memory_runtime_kv_exported,
            self.memory_runtime_kv_stored,
            self.memory_runtime_kv_hold,
            self.memory_runtime_kv_held,
            self.memory_feedback_reinforced,
            self.memory_feedback_penalized,
            self.memory_feedback_reinforcement_milli,
            self.memory_feedback_penalty_milli,
            self.memory_feedback_updates,
            self.memory_feedback_applied,
            self.memory_feedback_removed,
            self.memory_feedback_missing,
            self.memory_feedback_strength_delta_milli,
            self.memory_residency_events,
            self.memory_residency_decisions,
            self.memory_residency_hot,
            self.memory_residency_warm,
            self.memory_residency_cold,
            self.memory_residency_quarantined,
            self.memory_residency_retired,
            self.memory_residency_protected_rollback_anchors,
            self.memory_residency_blocked_reasons,
            self.memory_residency_token_estimate,
            self.memory_residency_write_allowed,
            self.memory_residency_durable_write_allowed,
            self.memory_residency_applied,
            self.unified_writer_gate_events,
            self.unified_writer_gate_records,
            self.unified_writer_gate_memory_records,
            self.unified_writer_gate_genome_records,
            self.unified_writer_gate_experiment_ledger_records,
            self.unified_writer_gate_evolution_goal_queue_records,
            self.unified_writer_gate_ready_records,
            self.unified_writer_gate_held_records,
            self.unified_writer_gate_rejected_records,
            self.unified_writer_gate_preview_only_records,
            self.unified_writer_gate_reason_codes,
            self.unified_writer_gate_explicit_apply_required,
            self.unified_writer_gate_write_allowed,
            self.unified_writer_gate_durable_write_allowed,
            self.unified_writer_gate_applied,
            self.self_goal_queue_apply_events,
            self.self_goal_queue_apply_records,
            self.self_goal_queue_apply_ready_records,
            self.self_goal_queue_apply_held_records,
            self.self_goal_queue_apply_rejected_records,
            self.self_goal_queue_apply_reason_codes,
            self.self_goal_queue_apply_explicit_apply_required,
            self.self_goal_queue_apply_write_allowed,
            self.self_goal_queue_apply_applied,
            self.self_goal_queue_continuation_events,
            self.self_goal_queue_continuation_ready,
            self.self_goal_queue_continuation_held,
            self.self_goal_queue_continuation_current_queue,
            self.self_goal_queue_continuation_completion_resulting_queue,
            self.self_goal_queue_continuation_goals,
            self.self_goal_queue_continuation_required_evidence,
            self.self_goal_queue_continuation_reason_codes,
            self.self_goal_queue_continuation_budget_attempts,
            self.self_goal_queue_continuation_budget_steps,
            self.self_goal_queue_continuation_budget_tokens,
            self.self_goal_queue_continuation_budget_runtime_ms,
            self.self_goal_queue_continuation_write_allowed,
            self.self_goal_queue_continuation_applied,
            self.self_goal_queue_evidence_plan_events,
            self.self_goal_queue_evidence_plan_ready,
            self.self_goal_queue_evidence_plan_held,
            self.self_goal_queue_evidence_plan_steps,
            self.self_goal_queue_evidence_plan_auto_collectible,
            self.self_goal_queue_evidence_plan_manual,
            self.self_goal_queue_evidence_plan_required_evidence,
            self.self_goal_queue_evidence_plan_packet_templates,
            self.self_goal_queue_evidence_plan_command_templates,
            self.self_goal_queue_evidence_plan_write_allowed,
            self.self_goal_queue_evidence_plan_applied,
            self.self_goal_queue_evidence_collection_events,
            self.self_goal_queue_evidence_collection_ready,
            self.self_goal_queue_evidence_collection_complete,
            self.self_goal_queue_evidence_collection_steps,
            self.self_goal_queue_evidence_collection_collected,
            self.self_goal_queue_evidence_collection_passed,
            self.self_goal_queue_evidence_collection_failed,
            self.self_goal_queue_evidence_collection_missing,
            self.self_goal_queue_evidence_collection_manual_missing,
            self.self_goal_queue_evidence_collection_write_allowed,
            self.self_goal_queue_evidence_collection_applied,
            self.self_goal_local_evidence_events,
            self.self_goal_local_evidence_enabled,
            self.self_goal_local_evidence_dry_run,
            self.self_goal_local_evidence_ready,
            self.self_goal_local_evidence_steps,
            self.self_goal_local_evidence_attempted,
            self.self_goal_local_evidence_generated,
            self.self_goal_local_evidence_passed,
            self.self_goal_local_evidence_failed,
            self.self_goal_local_evidence_skipped,
            self.self_goal_local_evidence_manual,
            self.self_goal_local_evidence_planned_status,
            self.self_goal_local_evidence_write_allowed,
            self.self_goal_local_evidence_applied,
            self.evolution_goal_queue_store_write_events,
            self.evolution_goal_queue_store_write_applied,
            self.evolution_goal_queue_store_write_held,
            self.evolution_goal_queue_store_write_rejected,
            self.evolution_goal_queue_store_write_reason_codes,
            self.evolution_goal_queue_store_write_durable_write_allowed,
            self.evolution_goal_queue_store_write_applied_to_disk,
        );
        let learning = format!(
            "{extended} coding_service_eval_events={} coding_service_eval_readiness_events={} coding_service_eval_runner_events={} coding_service_eval_passed={} coding_service_eval_requests={} coding_service_eval_completed={} coding_service_eval_evidence_packets={} coding_service_eval_rust_validation_checked={} coding_service_eval_compile_checked={} coding_service_eval_unit_test_checked={} coding_service_eval_write_allowed={} coding_service_eval_applied={} process_reward_events={} process_reward_positive={} process_reward_reinforce={} process_reward_hold={} process_reward_penalize={} process_reward_total_milli={} live_evolution_events={} live_router_threshold_delta_milli={} live_hierarchy_weight_delta_milli={} live_online_reward_feedbacks={} live_online_reward_reinforcements={} live_online_reward_penalties={} live_online_reward_strength_milli={} live_memory_reinforcements={} live_memory_penalties={} live_memory_updates={} live_stored_memories={} live_stored_gist_memories={} live_stored_runtime_kv_memories={} live_stored_memory_updates={} live_reflection_issues={} live_critical_reflection_issues={} live_revision_actions={} evolution_live_inference_runs={} evolution_live_router_threshold_mutations={} evolution_live_hierarchy_weight_mutations={} evolution_live_router_threshold_delta_milli={} evolution_live_hierarchy_weight_delta_milli={} evolution_live_online_reward_feedbacks={} evolution_live_online_reward_reinforcements={} evolution_live_online_reward_penalties={} evolution_live_online_reward_strength_milli={} evolution_live_online_reward_reinforcement_strength_milli={} evolution_live_online_reward_penalty_strength_milli={} evolution_live_memory_reinforcements={} evolution_live_memory_penalties={} evolution_live_memory_updates={} evolution_live_stored_memories={} evolution_live_stored_gist_memories={} evolution_live_stored_runtime_kv_memories={} evolution_live_stored_memory_updates={} evolution_live_reflection_issues={} evolution_live_critical_reflection_issues={} evolution_live_revision_actions={} replay_live_evolution_items={} replay_live_evolution_router_threshold_mutations={} replay_live_evolution_hierarchy_weight_mutations={} replay_live_evolution_router_threshold_delta_milli={} replay_live_evolution_hierarchy_weight_delta_milli={} replay_live_evolution_online_reward_feedbacks={} replay_live_evolution_online_reward_reinforcements={} replay_live_evolution_online_reward_penalties={} replay_live_evolution_online_reward_strength_milli={} replay_live_evolution_online_reward_reinforcement_strength_milli={} replay_live_evolution_online_reward_penalty_strength_milli={} replay_live_evolution_memory_updates={} replay_live_evolution_stored_memory_updates={} replay_live_evolution_reflection_issues={} replay_live_evolution_critical_reflection_issues={} replay_live_evolution_revision_actions={}",
            self.coding_service_eval_events,
            self.coding_service_eval_readiness_events,
            self.coding_service_eval_runner_events,
            self.coding_service_eval_passed,
            self.coding_service_eval_requests,
            self.coding_service_eval_completed,
            self.coding_service_eval_evidence_packets,
            self.coding_service_eval_rust_validation_checked,
            self.coding_service_eval_compile_checked,
            self.coding_service_eval_unit_test_checked,
            self.coding_service_eval_write_allowed,
            self.coding_service_eval_applied,
            self.process_reward_events,
            self.process_reward_positive,
            self.process_reward_reinforce,
            self.process_reward_hold,
            self.process_reward_penalize,
            self.process_reward_total_milli,
            self.live_evolution_events,
            self.live_router_threshold_delta_milli,
            self.live_hierarchy_weight_delta_milli,
            self.live_online_reward_feedbacks,
            self.live_online_reward_reinforcements,
            self.live_online_reward_penalties,
            self.live_online_reward_strength_milli,
            self.live_memory_reinforcements,
            self.live_memory_penalties,
            self.live_memory_updates,
            self.live_stored_memories,
            self.live_stored_gist_memories,
            self.live_stored_runtime_kv_memories,
            self.live_stored_memory_updates,
            self.live_reflection_issues,
            self.live_critical_reflection_issues,
            self.live_revision_actions,
            self.evolution_live_inference_runs,
            self.evolution_live_router_threshold_mutations,
            self.evolution_live_hierarchy_weight_mutations,
            self.evolution_live_router_threshold_delta_milli,
            self.evolution_live_hierarchy_weight_delta_milli,
            self.evolution_live_online_reward_feedbacks,
            self.evolution_live_online_reward_reinforcements,
            self.evolution_live_online_reward_penalties,
            self.evolution_live_online_reward_strength_milli,
            self.evolution_live_online_reward_reinforcement_strength_milli,
            self.evolution_live_online_reward_penalty_strength_milli,
            self.evolution_live_memory_reinforcements,
            self.evolution_live_memory_penalties,
            self.evolution_live_memory_updates,
            self.evolution_live_stored_memories,
            self.evolution_live_stored_gist_memories,
            self.evolution_live_stored_runtime_kv_memories,
            self.evolution_live_stored_memory_updates,
            self.evolution_live_reflection_issues,
            self.evolution_live_critical_reflection_issues,
            self.evolution_live_revision_actions,
            self.replay_live_evolution_items,
            self.replay_live_evolution_router_threshold_mutations,
            self.replay_live_evolution_hierarchy_weight_mutations,
            self.replay_live_evolution_router_threshold_delta_milli,
            self.replay_live_evolution_hierarchy_weight_delta_milli,
            self.replay_live_evolution_online_reward_feedbacks,
            self.replay_live_evolution_online_reward_reinforcements,
            self.replay_live_evolution_online_reward_penalties,
            self.replay_live_evolution_online_reward_strength_milli,
            self.replay_live_evolution_online_reward_reinforcement_strength_milli,
            self.replay_live_evolution_online_reward_penalty_strength_milli,
            self.replay_live_evolution_memory_updates,
            self.replay_live_evolution_stored_memory_updates,
            self.replay_live_evolution_reflection_issues,
            self.replay_live_evolution_critical_reflection_issues,
            self.replay_live_evolution_revision_actions,
        );
        format!(
            "{learning} auto_replay_live_memory_feedback_items={} auto_replay_live_memory_feedback_updates={} auto_replay_live_memory_feedback_reinforcements={} auto_replay_live_memory_feedback_penalties={} auto_replay_live_memory_feedback_detail_items={} auto_replay_live_memory_feedback_applied={} auto_replay_live_memory_feedback_removed={} auto_replay_live_memory_feedback_missing={} auto_replay_live_memory_feedback_strength_delta_milli={} replay_live_memory_feedback_items={} replay_live_memory_feedback_updates={} replay_live_memory_feedback_reinforcements={} replay_live_memory_feedback_penalties={} replay_live_memory_feedback_detail_items={} replay_live_memory_feedback_applied={} replay_live_memory_feedback_removed={} replay_live_memory_feedback_missing={} replay_live_memory_feedback_strength_delta_milli={} auto_replay_recursive_runtime_items={} auto_replay_recursive_runtime_calls={} auto_replay_avg_recursive_call_pressure_milli={} auto_replay_max_recursive_call_pressure_milli={} evolution_recursive_replay_items={} evolution_recursive_runtime_calls={}",
            self.auto_replay_live_memory_feedback_items,
            self.auto_replay_live_memory_feedback_updates,
            self.auto_replay_live_memory_feedback_reinforcements,
            self.auto_replay_live_memory_feedback_penalties,
            self.auto_replay_live_memory_feedback_detail_items,
            self.auto_replay_live_memory_feedback_applied,
            self.auto_replay_live_memory_feedback_removed,
            self.auto_replay_live_memory_feedback_missing,
            self.auto_replay_live_memory_feedback_strength_delta_milli,
            self.replay_live_memory_feedback_items,
            self.replay_live_memory_feedback_updates,
            self.replay_live_memory_feedback_reinforcements,
            self.replay_live_memory_feedback_penalties,
            self.replay_live_memory_feedback_detail_items,
            self.replay_live_memory_feedback_applied,
            self.replay_live_memory_feedback_removed,
            self.replay_live_memory_feedback_missing,
            self.replay_live_memory_feedback_strength_delta_milli,
            self.auto_replay_recursive_runtime_items,
            self.auto_replay_recursive_runtime_calls,
            self.auto_replay_avg_recursive_call_pressure_milli,
            self.auto_replay_max_recursive_call_pressure_milli,
            self.evolution_recursive_replay_items,
            self.evolution_recursive_runtime_calls,
        )
    }

    pub fn operator_health_snapshot(&self) -> OperatorHealthSnapshot {
        let mut sections = Vec::new();

        let trace_blocked = !self.passed;
        sections.push(OperatorHealthSection::new(
            "trace_gate",
            self.checked_lines > 0,
            trace_blocked,
            trace_blocked,
            self.checked_lines,
            vec![
                OperatorHealthMetric::new("checked_lines", self.checked_lines),
                OperatorHealthMetric::new("failure_count", self.failures.len()),
                OperatorHealthMetric::new("trace_id_count", self.trace_experience_ids.len()),
                OperatorHealthMetric::new("runtime_error_events", self.runtime_error_events),
                OperatorHealthMetric::new("runtime_timeout_events", self.runtime_timeout_events),
            ],
            self.passed,
        ));

        let memory_events = self
            .memory_admission_events
            .saturating_add(self.memory_runtime_kv_exported)
            .saturating_add(self.memory_runtime_kv_stored)
            .saturating_add(self.memory_runtime_kv_hold)
            .saturating_add(self.memory_runtime_kv_held)
            .saturating_add(self.memory_feedback_updates)
            .saturating_add(self.self_evolving_memory_store_events)
            .saturating_add(self.self_evolving_memory_writeback_events)
            .saturating_add(self.memory_residency_events)
            .saturating_add(self.kv_fusion_events);
        let memory_review = self
            .memory_admission_review_packets
            .saturating_add(self.memory_admission_ledger_preview_only)
            .saturating_add(self.memory_admission_hold)
            .saturating_add(self.memory_admission_ledger_held)
            .saturating_add(self.memory_runtime_kv_hold)
            .saturating_add(self.memory_feedback_missing)
            .saturating_add(self.self_evolving_memory_store_admission_preview_events)
            .saturating_add(self.memory_residency_protected_rollback_anchors);
        let memory_blocked = self
            .memory_admission_blocked
            .saturating_add(self.memory_admission_reject)
            .saturating_add(self.memory_admission_quarantine)
            .saturating_add(self.memory_admission_ledger_rejected)
            .saturating_add(self.memory_residency_quarantined)
            .saturating_add(self.kv_fusion_approval_blocked)
            .saturating_add(self.kv_fusion_rejected);
        sections.push(OperatorHealthSection::new(
            "memory",
            memory_events > 0,
            memory_review > 0,
            memory_blocked > 0,
            memory_events,
            vec![
                OperatorHealthMetric::new("admission_events", self.memory_admission_events),
                OperatorHealthMetric::new("admission_candidates", self.memory_admission_candidates),
                OperatorHealthMetric::new("admission_ready", self.memory_admission_ready),
                OperatorHealthMetric::new("admission_blocked", self.memory_admission_blocked),
                OperatorHealthMetric::new("review_packets", self.memory_admission_review_packets),
                OperatorHealthMetric::new(
                    "ledger_preview_only",
                    self.memory_admission_ledger_preview_only,
                ),
                OperatorHealthMetric::new("ledger_applied", self.memory_admission_ledger_applied),
                OperatorHealthMetric::new("runtime_kv_exported", self.memory_runtime_kv_exported),
                OperatorHealthMetric::new("runtime_kv_stored", self.memory_runtime_kv_stored),
                OperatorHealthMetric::new("runtime_kv_hold", self.memory_runtime_kv_hold),
                OperatorHealthMetric::new("runtime_kv_held", self.memory_runtime_kv_held),
                OperatorHealthMetric::new("feedback_reinforced", self.memory_feedback_reinforced),
                OperatorHealthMetric::new("feedback_penalized", self.memory_feedback_penalized),
                OperatorHealthMetric::new(
                    "feedback_reinforcement_milli",
                    self.memory_feedback_reinforcement_milli,
                ),
                OperatorHealthMetric::new(
                    "feedback_penalty_milli",
                    self.memory_feedback_penalty_milli,
                ),
                OperatorHealthMetric::new("feedback_updates", self.memory_feedback_updates),
                OperatorHealthMetric::new("feedback_applied", self.memory_feedback_applied),
                OperatorHealthMetric::new("feedback_removed", self.memory_feedback_removed),
                OperatorHealthMetric::new("feedback_missing", self.memory_feedback_missing),
                OperatorHealthMetric::new(
                    "feedback_strength_delta_milli",
                    self.memory_feedback_strength_delta_milli,
                ),
                OperatorHealthMetric::new(
                    "self_evolving_store_events",
                    self.self_evolving_memory_store_events,
                ),
                OperatorHealthMetric::new(
                    "self_evolving_store_retrieval_events",
                    self.self_evolving_memory_store_retrieval_events,
                ),
                OperatorHealthMetric::new(
                    "self_evolving_store_maintenance_events",
                    self.self_evolving_memory_store_maintenance_events,
                ),
                OperatorHealthMetric::new(
                    "self_evolving_store_admission_preview_events",
                    self.self_evolving_memory_store_admission_preview_events,
                ),
                OperatorHealthMetric::new(
                    "self_evolving_store_contexts",
                    self.self_evolving_memory_store_contexts,
                ),
                OperatorHealthMetric::new(
                    "self_evolving_store_saved_tokens",
                    self.self_evolving_memory_store_saved_tokens,
                ),
                OperatorHealthMetric::new(
                    "self_evolving_store_maintenance_actions",
                    self.self_evolving_memory_store_maintenance_actions,
                ),
                OperatorHealthMetric::new(
                    "self_evolving_store_admission_candidates",
                    self.self_evolving_memory_store_admission_candidates,
                ),
                OperatorHealthMetric::new(
                    "self_evolving_store_write_allowed",
                    self.self_evolving_memory_store_write_allowed,
                ),
                OperatorHealthMetric::new(
                    "self_evolving_store_durable_write_allowed",
                    self.self_evolving_memory_store_durable_write_allowed,
                ),
                OperatorHealthMetric::new(
                    "self_evolving_store_applied",
                    self.self_evolving_memory_store_applied,
                ),
                OperatorHealthMetric::new(
                    "self_evolving_store_applied_to_disk",
                    self.self_evolving_memory_store_applied_to_disk,
                ),
                OperatorHealthMetric::new(
                    "self_evolving_store_source_quarantine_events",
                    self.self_evolving_memory_store_source_quarantine_events,
                ),
                OperatorHealthMetric::new(
                    "self_evolving_store_source_quarantine_actions",
                    self.self_evolving_memory_store_source_quarantine_actions,
                ),
                OperatorHealthMetric::new(
                    "self_evolving_writeback_events",
                    self.self_evolving_memory_writeback_events,
                ),
                OperatorHealthMetric::new(
                    "self_evolving_writeback_source_case_digests",
                    self.self_evolving_memory_writeback_source_case_digests,
                ),
                OperatorHealthMetric::new(
                    "self_evolving_writeback_attempted_records",
                    self.self_evolving_memory_writeback_attempted_records,
                ),
                OperatorHealthMetric::new(
                    "self_evolving_writeback_accepted_records",
                    self.self_evolving_memory_writeback_accepted_records,
                ),
                OperatorHealthMetric::new(
                    "self_evolving_writeback_rejected_records",
                    self.self_evolving_memory_writeback_rejected_records(),
                ),
                OperatorHealthMetric::new(
                    "self_evolving_writeback_records_before",
                    self.self_evolving_memory_writeback_records_before,
                ),
                OperatorHealthMetric::new(
                    "self_evolving_writeback_records_after",
                    self.self_evolving_memory_writeback_records_after,
                ),
                OperatorHealthMetric::new(
                    "self_evolving_writeback_tool_reliability_after",
                    self.self_evolving_memory_writeback_tool_reliability_after,
                ),
                OperatorHealthMetric::new(
                    "self_evolving_writeback_tool_observations_after",
                    self.self_evolving_memory_writeback_tool_observations_after,
                ),
                OperatorHealthMetric::new(
                    "self_evolving_writeback_write_allowed",
                    self.self_evolving_memory_writeback_write_allowed,
                ),
                OperatorHealthMetric::new(
                    "self_evolving_writeback_durable_write_allowed",
                    self.self_evolving_memory_writeback_durable_write_allowed,
                ),
                OperatorHealthMetric::new(
                    "self_evolving_writeback_applied",
                    self.self_evolving_memory_writeback_applied,
                ),
                OperatorHealthMetric::new(
                    "self_evolving_writeback_applied_to_disk",
                    self.self_evolving_memory_writeback_applied_to_disk,
                ),
                OperatorHealthMetric::new(
                    "self_evolving_writeback_snapshot_changes",
                    self.self_evolving_memory_writeback_snapshot_changes,
                ),
                OperatorHealthMetric::new(
                    "self_evolving_consolidation_events",
                    self.self_evolving_memory_store_consolidation_events,
                ),
                OperatorHealthMetric::new(
                    "self_evolving_consolidation_actions",
                    self.self_evolving_memory_store_consolidation_actions,
                ),
                OperatorHealthMetric::new(
                    "self_evolving_merge_previews",
                    self.self_evolving_memory_store_merge_previews,
                ),
                OperatorHealthMetric::new(
                    "self_evolving_decay_previews",
                    self.self_evolving_memory_store_decay_previews,
                ),
                OperatorHealthMetric::new(
                    "self_evolving_tombstone_previews",
                    self.self_evolving_memory_store_tombstone_previews,
                ),
                OperatorHealthMetric::new(
                    "self_evolving_merge_rejections",
                    self.self_evolving_memory_store_merge_rejections,
                ),
                OperatorHealthMetric::new("residency_events", self.memory_residency_events),
                OperatorHealthMetric::new("residency_decisions", self.memory_residency_decisions),
                OperatorHealthMetric::new("residency_hot", self.memory_residency_hot),
                OperatorHealthMetric::new("residency_warm", self.memory_residency_warm),
                OperatorHealthMetric::new("residency_cold", self.memory_residency_cold),
                OperatorHealthMetric::new(
                    "residency_quarantined",
                    self.memory_residency_quarantined,
                ),
                OperatorHealthMetric::new("residency_retired", self.memory_residency_retired),
                OperatorHealthMetric::new(
                    "residency_protected_rollback_anchors",
                    self.memory_residency_protected_rollback_anchors,
                ),
                OperatorHealthMetric::new(
                    "residency_blocked_reasons",
                    self.memory_residency_blocked_reasons,
                ),
                OperatorHealthMetric::new(
                    "residency_token_estimate",
                    self.memory_residency_token_estimate,
                ),
                OperatorHealthMetric::new(
                    "residency_write_allowed",
                    self.memory_residency_write_allowed,
                ),
                OperatorHealthMetric::new(
                    "residency_durable_write_allowed",
                    self.memory_residency_durable_write_allowed,
                ),
                OperatorHealthMetric::new("residency_applied", self.memory_residency_applied),
                OperatorHealthMetric::new("kv_fusion_events", self.kv_fusion_events),
                OperatorHealthMetric::new("kv_fusion_candidates", self.kv_fusion_candidates),
                OperatorHealthMetric::new("kv_fusion_fused", self.kv_fusion_fused),
                OperatorHealthMetric::new("kv_fusion_compressed", self.kv_fusion_compressed),
                OperatorHealthMetric::new("kv_fusion_skipped", self.kv_fusion_skipped),
                OperatorHealthMetric::new("kv_fusion_held", self.kv_fusion_held),
                OperatorHealthMetric::new("kv_fusion_rejected", self.kv_fusion_rejected),
                OperatorHealthMetric::new("kv_fusion_input_tokens", self.kv_fusion_input_tokens),
                OperatorHealthMetric::new(
                    "kv_fusion_retained_tokens",
                    self.kv_fusion_retained_tokens,
                ),
                OperatorHealthMetric::new("kv_fusion_saved_tokens", self.kv_fusion_saved_tokens),
                OperatorHealthMetric::new(
                    "kv_fusion_approval_blocked",
                    self.kv_fusion_approval_blocked,
                ),
            ],
            self.passed,
        ));

        let genome_review = self
            .reasoning_genome_relabel_candidates
            .saturating_add(self.reasoning_genome_regeneration_candidates)
            .saturating_add(self.reasoning_genome_gene_scissors_proposals)
            .saturating_add(self.reasoning_genome_repair_payloads)
            .saturating_add(self.reasoning_genome_regeneration_payloads)
            .saturating_add(self.reasoning_genome_lifecycle_pending_validations)
            .saturating_add(self.reasoning_genome_splice_repair_candidates)
            .saturating_add(self.reasoning_genome_splice_proposals);
        let genome_blocked = self
            .reasoning_genome_malignant_genes
            .saturating_add(self.reasoning_genome_splice_quarantined)
            .saturating_add(self.reasoning_genome_write_allowed)
            .saturating_add(self.reasoning_genome_mutation_applied)
            .saturating_add(self.reasoning_genome_splice_write_allowed)
            .saturating_add(self.reasoning_genome_splice_applied);
        sections.push(OperatorHealthSection::new(
            "genome",
            self.reasoning_genome_events > 0,
            genome_review > 0,
            genome_blocked > 0,
            self.reasoning_genome_events,
            vec![
                OperatorHealthMetric::new("events", self.reasoning_genome_events),
                OperatorHealthMetric::new("genes", self.reasoning_genome_genes),
                OperatorHealthMetric::new("active_genes", self.reasoning_genome_active_genes),
                OperatorHealthMetric::new("aged_genes", self.reasoning_genome_aged_genes),
                OperatorHealthMetric::new("malignant_genes", self.reasoning_genome_malignant_genes),
                OperatorHealthMetric::new(
                    "gene_scissors_proposals",
                    self.reasoning_genome_gene_scissors_proposals,
                ),
                OperatorHealthMetric::new("repair_payloads", self.reasoning_genome_repair_payloads),
                OperatorHealthMetric::new(
                    "regeneration_payloads",
                    self.reasoning_genome_regeneration_payloads,
                ),
                OperatorHealthMetric::new("splice_segments", self.reasoning_genome_splice_segments),
                OperatorHealthMetric::new(
                    "splice_quarantined",
                    self.reasoning_genome_splice_quarantined,
                ),
                OperatorHealthMetric::new(
                    "splice_repair_candidates",
                    self.reasoning_genome_splice_repair_candidates,
                ),
                OperatorHealthMetric::new("write_allowed", self.reasoning_genome_write_allowed),
                OperatorHealthMetric::new(
                    "mutation_applied",
                    self.reasoning_genome_mutation_applied,
                ),
            ],
            self.passed,
        ));

        let routing_events = self
            .adaptive_routing_events
            .saturating_add(self.task_hierarchy_events)
            .saturating_add(self.fht_dke_events)
            .saturating_add(self.noiron_orchestration_events)
            .saturating_add(self.compute_budget_events)
            .saturating_add(self.auto_replay_recursive_runtime_items)
            .saturating_add(self.evolution_recursive_replay_items);
        let routing_review = self
            .compute_budget_low
            .saturating_add(self.compute_budget_runtime_kv_budget_pressure_milli)
            .saturating_add(self.compute_budget_kv_lookups_skipped)
            .saturating_add(self.compute_budget_low_value_skipped)
            .saturating_add(self.compute_budget_write_allowed)
            .saturating_add(self.compute_budget_applied)
            .saturating_add(self.auto_replay_max_recursive_call_pressure_milli)
            .saturating_add(self.noiron_orchestration_failed_stages);
        sections.push(OperatorHealthSection::new(
            "routing",
            routing_events > 0,
            routing_review > 0,
            false,
            routing_events,
            vec![
                OperatorHealthMetric::new("adaptive_routing_events", self.adaptive_routing_events),
                OperatorHealthMetric::new(
                    "adaptive_routing_candidates",
                    self.adaptive_routing_candidates,
                ),
                OperatorHealthMetric::new(
                    "adaptive_routing_include",
                    self.adaptive_routing_include,
                ),
                OperatorHealthMetric::new(
                    "adaptive_routing_compress",
                    self.adaptive_routing_compress,
                ),
                OperatorHealthMetric::new("adaptive_routing_defer", self.adaptive_routing_defer),
                OperatorHealthMetric::new("adaptive_routing_skip", self.adaptive_routing_skip),
                OperatorHealthMetric::new(
                    "adaptive_routing_input_tokens",
                    self.adaptive_routing_input_tokens,
                ),
                OperatorHealthMetric::new(
                    "adaptive_routing_retained_tokens",
                    self.adaptive_routing_retained_tokens,
                ),
                OperatorHealthMetric::new(
                    "adaptive_routing_saved_tokens",
                    self.adaptive_routing_saved_tokens,
                ),
                OperatorHealthMetric::new("task_hierarchy_events", self.task_hierarchy_events),
                OperatorHealthMetric::new(
                    "task_hierarchy_mutation_records",
                    self.task_hierarchy_mutation_records,
                ),
                OperatorHealthMetric::new(
                    "task_hierarchy_route_pressure_milli",
                    self.task_hierarchy_route_pressure_milli,
                ),
                OperatorHealthMetric::new(
                    "task_hierarchy_compute_reduction_milli",
                    self.task_hierarchy_compute_reduction_milli,
                ),
                OperatorHealthMetric::new(
                    "task_hierarchy_depth_total",
                    self.task_hierarchy_depth_total,
                ),
                OperatorHealthMetric::new(
                    "task_hierarchy_route_fanout_total",
                    self.task_hierarchy_route_fanout_total,
                ),
                OperatorHealthMetric::new(
                    "task_hierarchy_threshold_delta_milli",
                    self.task_hierarchy_threshold_delta_milli,
                ),
                OperatorHealthMetric::new(
                    "task_hierarchy_selected_lanes",
                    self.task_hierarchy_selected_lanes,
                ),
                OperatorHealthMetric::new(
                    "task_hierarchy_skipped_lanes",
                    self.task_hierarchy_skipped_lanes,
                ),
                OperatorHealthMetric::new(
                    "task_hierarchy_memory_lanes",
                    self.task_hierarchy_memory_lanes,
                ),
                OperatorHealthMetric::new(
                    "task_hierarchy_skipped_memory_lanes",
                    self.task_hierarchy_skipped_memory_lanes,
                ),
                OperatorHealthMetric::new("fht_dke_events", self.fht_dke_events),
                OperatorHealthMetric::new("fht_dke_enabled", self.fht_dke_enabled),
                OperatorHealthMetric::new("fht_dke_total_tokens", self.fht_dke_total_tokens),
                OperatorHealthMetric::new("fht_dke_dense_tokens", self.fht_dke_dense_tokens),
                OperatorHealthMetric::new("fht_dke_routed_tokens", self.fht_dke_routed_tokens),
                OperatorHealthMetric::new(
                    "fht_dke_kv_exchange_blocks",
                    self.fht_dke_kv_exchange_blocks,
                ),
                OperatorHealthMetric::new(
                    "fht_dke_token_split_invalid",
                    self.fht_dke_token_split_invalid,
                ),
                OperatorHealthMetric::new(
                    "fht_dke_attention_threshold_milli",
                    self.fht_dke_attention_threshold_milli,
                ),
                OperatorHealthMetric::new(
                    "fht_dke_route_pressure_milli",
                    self.fht_dke_route_pressure_milli,
                ),
                OperatorHealthMetric::new(
                    "noiron_orchestration_events",
                    self.noiron_orchestration_events,
                ),
                OperatorHealthMetric::new(
                    "noiron_orchestration_stages",
                    self.noiron_orchestration_stages,
                ),
                OperatorHealthMetric::new(
                    "noiron_orchestration_failed_stages",
                    self.noiron_orchestration_failed_stages,
                ),
                OperatorHealthMetric::new(
                    "noiron_orchestration_writes_gated",
                    self.noiron_orchestration_writes_gated,
                ),
                OperatorHealthMetric::new(
                    "noiron_orchestration_fht_dke_total_tokens",
                    self.noiron_orchestration_fht_dke_total_tokens,
                ),
                OperatorHealthMetric::new("compute_budget_events", self.compute_budget_events),
                OperatorHealthMetric::new(
                    "compute_budget_threshold_delta_milli",
                    self.compute_budget_threshold_delta_milli,
                ),
                OperatorHealthMetric::new(
                    "compute_budget_runtime_kv_budget_pressure_milli",
                    self.compute_budget_runtime_kv_budget_pressure_milli,
                ),
                OperatorHealthMetric::new(
                    "compute_budget_selected_candidates",
                    self.compute_budget_selected_candidates,
                ),
                OperatorHealthMetric::new(
                    "compute_budget_saved_tokens",
                    self.compute_budget_saved_tokens,
                ),
                OperatorHealthMetric::new(
                    "compute_budget_self_evolving_memory_fusion_saved_tokens",
                    self.compute_budget_self_evolving_memory_fusion_saved_tokens,
                ),
                OperatorHealthMetric::new(
                    "compute_budget_avoided_tokens",
                    self.compute_budget_avoided_tokens,
                ),
                OperatorHealthMetric::new(
                    "compute_budget_fanout_before",
                    self.compute_budget_fanout_before,
                ),
                OperatorHealthMetric::new(
                    "compute_budget_fanout_after",
                    self.compute_budget_fanout_after,
                ),
                OperatorHealthMetric::new(
                    "compute_budget_fanout_reduction",
                    self.compute_budget_fanout_reduction,
                ),
                OperatorHealthMetric::new(
                    "compute_budget_estimated_budget_tokens",
                    self.compute_budget_estimated_budget_tokens,
                ),
                OperatorHealthMetric::new(
                    "compute_budget_estimated_spent_tokens",
                    self.compute_budget_estimated_spent_tokens,
                ),
                OperatorHealthMetric::new(
                    "compute_budget_estimated_saved_tokens",
                    self.compute_budget_estimated_saved_tokens,
                ),
                OperatorHealthMetric::new(
                    "compute_budget_anchor_count",
                    self.compute_budget_anchor_count,
                ),
                OperatorHealthMetric::new(
                    "compute_budget_anchors_preserved",
                    self.compute_budget_anchors_preserved,
                ),
                OperatorHealthMetric::new(
                    "compute_budget_anchor_preservation_failures",
                    self.compute_budget_anchor_preservation_failures,
                ),
                OperatorHealthMetric::new(
                    "compute_budget_fallback_triggered",
                    self.compute_budget_fallback_triggered,
                ),
                OperatorHealthMetric::new(
                    "compute_budget_write_allowed",
                    self.compute_budget_write_allowed,
                ),
                OperatorHealthMetric::new("compute_budget_applied", self.compute_budget_applied),
                OperatorHealthMetric::new(
                    "auto_replay_recursive_runtime_items",
                    self.auto_replay_recursive_runtime_items,
                ),
                OperatorHealthMetric::new(
                    "auto_replay_recursive_runtime_calls",
                    self.auto_replay_recursive_runtime_calls,
                ),
                OperatorHealthMetric::new(
                    "auto_replay_avg_recursive_call_pressure_milli",
                    self.auto_replay_avg_recursive_call_pressure_milli,
                ),
                OperatorHealthMetric::new(
                    "auto_replay_max_recursive_call_pressure_milli",
                    self.auto_replay_max_recursive_call_pressure_milli,
                ),
                OperatorHealthMetric::new(
                    "evolution_recursive_replay_items",
                    self.evolution_recursive_replay_items,
                ),
                OperatorHealthMetric::new(
                    "evolution_recursive_runtime_calls",
                    self.evolution_recursive_runtime_calls,
                ),
            ],
            self.passed,
        ));

        let learning_events = self
            .process_reward_events
            .saturating_add(self.live_evolution_events)
            .saturating_add(self.live_router_threshold_delta_milli)
            .saturating_add(self.live_hierarchy_weight_delta_milli)
            .saturating_add(self.live_memory_reinforcements)
            .saturating_add(self.live_memory_penalties)
            .saturating_add(self.live_stored_memories)
            .saturating_add(self.live_stored_gist_memories)
            .saturating_add(self.live_stored_runtime_kv_memories)
            .saturating_add(self.evolution_live_router_threshold_mutations)
            .saturating_add(self.evolution_live_hierarchy_weight_mutations)
            .saturating_add(self.evolution_live_online_reward_feedbacks)
            .saturating_add(self.evolution_live_memory_reinforcements)
            .saturating_add(self.evolution_live_memory_penalties)
            .saturating_add(self.evolution_live_stored_memory_updates)
            .saturating_add(self.auto_replay_live_memory_feedback_items)
            .saturating_add(self.auto_replay_live_memory_feedback_updates)
            .saturating_add(self.auto_replay_live_memory_feedback_reinforcements)
            .saturating_add(self.auto_replay_live_memory_feedback_penalties)
            .saturating_add(self.auto_replay_live_memory_feedback_detail_items)
            .saturating_add(self.auto_replay_live_memory_feedback_applied)
            .saturating_add(self.auto_replay_live_memory_feedback_removed)
            .saturating_add(self.replay_live_memory_feedback_items)
            .saturating_add(self.replay_live_memory_feedback_updates)
            .saturating_add(self.replay_live_memory_feedback_reinforcements)
            .saturating_add(self.replay_live_memory_feedback_penalties)
            .saturating_add(self.replay_live_memory_feedback_detail_items)
            .saturating_add(self.replay_live_memory_feedback_applied)
            .saturating_add(self.replay_live_memory_feedback_removed)
            .saturating_add(self.replay_live_evolution_items)
            .saturating_add(self.replay_live_evolution_online_reward_feedbacks)
            .saturating_add(self.used_experiences)
            .saturating_add(self.imported_kv_blocks)
            .saturating_add(self.runtime_kv_weak_import_pressure_milli);
        let learning_review = self
            .live_reflection_issues
            .saturating_add(self.live_revision_actions)
            .saturating_add(self.evolution_live_router_threshold_delta_milli)
            .saturating_add(self.evolution_live_hierarchy_weight_delta_milli)
            .saturating_add(self.evolution_live_reflection_issues)
            .saturating_add(self.evolution_live_revision_actions)
            .saturating_add(self.auto_replay_live_memory_feedback_missing)
            .saturating_add(self.auto_replay_live_memory_feedback_strength_delta_milli)
            .saturating_add(self.replay_live_memory_feedback_missing)
            .saturating_add(self.replay_live_memory_feedback_strength_delta_milli)
            .saturating_add(self.replay_live_evolution_router_threshold_delta_milli)
            .saturating_add(self.replay_live_evolution_hierarchy_weight_delta_milli)
            .saturating_add(self.replay_live_evolution_reflection_issues)
            .saturating_add(self.replay_live_evolution_revision_actions)
            .saturating_add(self.runtime_kv_weak_import_pressure_milli);
        sections.push(OperatorHealthSection::new(
            "learning",
            learning_events > 0,
            learning_review > 0,
            self.live_critical_reflection_issues
                .saturating_add(self.evolution_live_critical_reflection_issues)
                .saturating_add(self.replay_live_evolution_critical_reflection_issues)
                > 0,
            learning_events,
            vec![
                OperatorHealthMetric::new("used_experiences", self.used_experiences),
                OperatorHealthMetric::new("imported_kv_blocks", self.imported_kv_blocks),
                OperatorHealthMetric::new(
                    "runtime_kv_weak_import_pressure_milli",
                    self.runtime_kv_weak_import_pressure_milli,
                ),
                OperatorHealthMetric::new("process_reward_events", self.process_reward_events),
                OperatorHealthMetric::new("process_reward_positive", self.process_reward_positive),
                OperatorHealthMetric::new(
                    "process_reward_reinforce",
                    self.process_reward_reinforce,
                ),
                OperatorHealthMetric::new("process_reward_hold", self.process_reward_hold),
                OperatorHealthMetric::new("process_reward_penalize", self.process_reward_penalize),
                OperatorHealthMetric::new(
                    "process_reward_total_milli",
                    self.process_reward_total_milli,
                ),
                OperatorHealthMetric::new("live_evolution_events", self.live_evolution_events),
                OperatorHealthMetric::new(
                    "live_router_threshold_delta_milli",
                    self.live_router_threshold_delta_milli,
                ),
                OperatorHealthMetric::new(
                    "live_hierarchy_weight_delta_milli",
                    self.live_hierarchy_weight_delta_milli,
                ),
                OperatorHealthMetric::new(
                    "live_online_reward_feedbacks",
                    self.live_online_reward_feedbacks,
                ),
                OperatorHealthMetric::new(
                    "live_online_reward_reinforcements",
                    self.live_online_reward_reinforcements,
                ),
                OperatorHealthMetric::new(
                    "live_online_reward_penalties",
                    self.live_online_reward_penalties,
                ),
                OperatorHealthMetric::new(
                    "live_online_reward_strength_milli",
                    self.live_online_reward_strength_milli,
                ),
                OperatorHealthMetric::new(
                    "live_memory_reinforcements",
                    self.live_memory_reinforcements,
                ),
                OperatorHealthMetric::new("live_memory_penalties", self.live_memory_penalties),
                OperatorHealthMetric::new("live_memory_updates", self.live_memory_updates),
                OperatorHealthMetric::new("live_stored_memories", self.live_stored_memories),
                OperatorHealthMetric::new(
                    "live_stored_gist_memories",
                    self.live_stored_gist_memories,
                ),
                OperatorHealthMetric::new(
                    "live_stored_runtime_kv_memories",
                    self.live_stored_runtime_kv_memories,
                ),
                OperatorHealthMetric::new(
                    "live_stored_memory_updates",
                    self.live_stored_memory_updates,
                ),
                OperatorHealthMetric::new("live_reflection_issues", self.live_reflection_issues),
                OperatorHealthMetric::new(
                    "live_critical_reflection_issues",
                    self.live_critical_reflection_issues,
                ),
                OperatorHealthMetric::new("live_revision_actions", self.live_revision_actions),
                OperatorHealthMetric::new(
                    "evolution_live_inference_runs",
                    self.evolution_live_inference_runs,
                ),
                OperatorHealthMetric::new(
                    "evolution_live_router_threshold_mutations",
                    self.evolution_live_router_threshold_mutations,
                ),
                OperatorHealthMetric::new(
                    "evolution_live_hierarchy_weight_mutations",
                    self.evolution_live_hierarchy_weight_mutations,
                ),
                OperatorHealthMetric::new(
                    "evolution_live_router_threshold_delta_milli",
                    self.evolution_live_router_threshold_delta_milli,
                ),
                OperatorHealthMetric::new(
                    "evolution_live_hierarchy_weight_delta_milli",
                    self.evolution_live_hierarchy_weight_delta_milli,
                ),
                OperatorHealthMetric::new(
                    "evolution_live_online_reward_feedbacks",
                    self.evolution_live_online_reward_feedbacks,
                ),
                OperatorHealthMetric::new(
                    "evolution_live_online_reward_reinforcements",
                    self.evolution_live_online_reward_reinforcements,
                ),
                OperatorHealthMetric::new(
                    "evolution_live_online_reward_penalties",
                    self.evolution_live_online_reward_penalties,
                ),
                OperatorHealthMetric::new(
                    "evolution_live_online_reward_strength_milli",
                    self.evolution_live_online_reward_strength_milli,
                ),
                OperatorHealthMetric::new(
                    "evolution_live_online_reward_reinforcement_strength_milli",
                    self.evolution_live_online_reward_reinforcement_strength_milli,
                ),
                OperatorHealthMetric::new(
                    "evolution_live_online_reward_penalty_strength_milli",
                    self.evolution_live_online_reward_penalty_strength_milli,
                ),
                OperatorHealthMetric::new(
                    "evolution_live_memory_reinforcements",
                    self.evolution_live_memory_reinforcements,
                ),
                OperatorHealthMetric::new(
                    "evolution_live_memory_penalties",
                    self.evolution_live_memory_penalties,
                ),
                OperatorHealthMetric::new(
                    "evolution_live_memory_updates",
                    self.evolution_live_memory_updates,
                ),
                OperatorHealthMetric::new(
                    "evolution_live_stored_memories",
                    self.evolution_live_stored_memories,
                ),
                OperatorHealthMetric::new(
                    "evolution_live_stored_gist_memories",
                    self.evolution_live_stored_gist_memories,
                ),
                OperatorHealthMetric::new(
                    "evolution_live_stored_runtime_kv_memories",
                    self.evolution_live_stored_runtime_kv_memories,
                ),
                OperatorHealthMetric::new(
                    "evolution_live_stored_memory_updates",
                    self.evolution_live_stored_memory_updates,
                ),
                OperatorHealthMetric::new(
                    "evolution_live_reflection_issues",
                    self.evolution_live_reflection_issues,
                ),
                OperatorHealthMetric::new(
                    "evolution_live_critical_reflection_issues",
                    self.evolution_live_critical_reflection_issues,
                ),
                OperatorHealthMetric::new(
                    "evolution_live_revision_actions",
                    self.evolution_live_revision_actions,
                ),
                OperatorHealthMetric::new(
                    "auto_replay_live_memory_feedback_items",
                    self.auto_replay_live_memory_feedback_items,
                ),
                OperatorHealthMetric::new(
                    "auto_replay_live_memory_feedback_updates",
                    self.auto_replay_live_memory_feedback_updates,
                ),
                OperatorHealthMetric::new(
                    "auto_replay_live_memory_feedback_reinforcements",
                    self.auto_replay_live_memory_feedback_reinforcements,
                ),
                OperatorHealthMetric::new(
                    "auto_replay_live_memory_feedback_penalties",
                    self.auto_replay_live_memory_feedback_penalties,
                ),
                OperatorHealthMetric::new(
                    "auto_replay_live_memory_feedback_detail_items",
                    self.auto_replay_live_memory_feedback_detail_items,
                ),
                OperatorHealthMetric::new(
                    "auto_replay_live_memory_feedback_applied",
                    self.auto_replay_live_memory_feedback_applied,
                ),
                OperatorHealthMetric::new(
                    "auto_replay_live_memory_feedback_removed",
                    self.auto_replay_live_memory_feedback_removed,
                ),
                OperatorHealthMetric::new(
                    "auto_replay_live_memory_feedback_missing",
                    self.auto_replay_live_memory_feedback_missing,
                ),
                OperatorHealthMetric::new(
                    "auto_replay_live_memory_feedback_strength_delta_milli",
                    self.auto_replay_live_memory_feedback_strength_delta_milli,
                ),
                OperatorHealthMetric::new(
                    "replay_live_memory_feedback_items",
                    self.replay_live_memory_feedback_items,
                ),
                OperatorHealthMetric::new(
                    "replay_live_memory_feedback_updates",
                    self.replay_live_memory_feedback_updates,
                ),
                OperatorHealthMetric::new(
                    "replay_live_memory_feedback_reinforcements",
                    self.replay_live_memory_feedback_reinforcements,
                ),
                OperatorHealthMetric::new(
                    "replay_live_memory_feedback_penalties",
                    self.replay_live_memory_feedback_penalties,
                ),
                OperatorHealthMetric::new(
                    "replay_live_memory_feedback_detail_items",
                    self.replay_live_memory_feedback_detail_items,
                ),
                OperatorHealthMetric::new(
                    "replay_live_memory_feedback_applied",
                    self.replay_live_memory_feedback_applied,
                ),
                OperatorHealthMetric::new(
                    "replay_live_memory_feedback_removed",
                    self.replay_live_memory_feedback_removed,
                ),
                OperatorHealthMetric::new(
                    "replay_live_memory_feedback_missing",
                    self.replay_live_memory_feedback_missing,
                ),
                OperatorHealthMetric::new(
                    "replay_live_memory_feedback_strength_delta_milli",
                    self.replay_live_memory_feedback_strength_delta_milli,
                ),
                OperatorHealthMetric::new(
                    "replay_live_evolution_items",
                    self.replay_live_evolution_items,
                ),
                OperatorHealthMetric::new(
                    "replay_live_evolution_router_threshold_mutations",
                    self.replay_live_evolution_router_threshold_mutations,
                ),
                OperatorHealthMetric::new(
                    "replay_live_evolution_hierarchy_weight_mutations",
                    self.replay_live_evolution_hierarchy_weight_mutations,
                ),
                OperatorHealthMetric::new(
                    "replay_live_evolution_router_threshold_delta_milli",
                    self.replay_live_evolution_router_threshold_delta_milli,
                ),
                OperatorHealthMetric::new(
                    "replay_live_evolution_hierarchy_weight_delta_milli",
                    self.replay_live_evolution_hierarchy_weight_delta_milli,
                ),
                OperatorHealthMetric::new(
                    "replay_live_evolution_online_reward_feedbacks",
                    self.replay_live_evolution_online_reward_feedbacks,
                ),
                OperatorHealthMetric::new(
                    "replay_live_evolution_online_reward_reinforcements",
                    self.replay_live_evolution_online_reward_reinforcements,
                ),
                OperatorHealthMetric::new(
                    "replay_live_evolution_online_reward_penalties",
                    self.replay_live_evolution_online_reward_penalties,
                ),
                OperatorHealthMetric::new(
                    "replay_live_evolution_online_reward_strength_milli",
                    self.replay_live_evolution_online_reward_strength_milli,
                ),
                OperatorHealthMetric::new(
                    "replay_live_evolution_online_reward_reinforcement_strength_milli",
                    self.replay_live_evolution_online_reward_reinforcement_strength_milli,
                ),
                OperatorHealthMetric::new(
                    "replay_live_evolution_online_reward_penalty_strength_milli",
                    self.replay_live_evolution_online_reward_penalty_strength_milli,
                ),
                OperatorHealthMetric::new(
                    "replay_live_evolution_memory_updates",
                    self.replay_live_evolution_memory_updates,
                ),
                OperatorHealthMetric::new(
                    "replay_live_evolution_stored_memory_updates",
                    self.replay_live_evolution_stored_memory_updates,
                ),
                OperatorHealthMetric::new(
                    "replay_live_evolution_reflection_issues",
                    self.replay_live_evolution_reflection_issues,
                ),
                OperatorHealthMetric::new(
                    "replay_live_evolution_critical_reflection_issues",
                    self.replay_live_evolution_critical_reflection_issues,
                ),
                OperatorHealthMetric::new(
                    "replay_live_evolution_revision_actions",
                    self.replay_live_evolution_revision_actions,
                ),
            ],
            self.passed,
        ));

        let self_goal_queue_events = self
            .self_goal_queue_apply_events
            .saturating_add(self.self_goal_queue_continuation_events)
            .saturating_add(self.self_goal_queue_evidence_plan_events)
            .saturating_add(self.self_goal_queue_evidence_collection_events)
            .saturating_add(self.self_goal_local_evidence_events)
            .saturating_add(self.evolution_goal_queue_store_write_events);
        let self_goal_queue_review = self
            .self_goal_queue_apply_ready_records
            .saturating_add(self.self_goal_queue_apply_held_records)
            .saturating_add(self.self_goal_queue_continuation_held)
            .saturating_add(self.self_goal_queue_evidence_plan_ready)
            .saturating_add(self.self_goal_queue_evidence_plan_held)
            .saturating_add(self.self_goal_queue_evidence_plan_manual)
            .saturating_add(self.self_goal_queue_evidence_collection_ready)
            .saturating_add(self.self_goal_queue_evidence_collection_missing)
            .saturating_add(self.self_goal_queue_evidence_collection_manual_missing)
            .saturating_add(self.self_goal_local_evidence_ready)
            .saturating_add(self.self_goal_local_evidence_manual)
            .saturating_add(self.self_goal_local_evidence_skipped)
            .saturating_add(self.evolution_goal_queue_store_write_held);
        let self_goal_queue_blocked = self
            .self_goal_queue_apply_rejected_records
            .saturating_add(self.self_goal_queue_apply_write_allowed)
            .saturating_add(self.self_goal_queue_apply_applied)
            .saturating_add(self.self_goal_queue_continuation_write_allowed)
            .saturating_add(self.self_goal_queue_continuation_applied)
            .saturating_add(self.self_goal_queue_evidence_plan_write_allowed)
            .saturating_add(self.self_goal_queue_evidence_plan_applied)
            .saturating_add(self.self_goal_queue_evidence_collection_write_allowed)
            .saturating_add(self.self_goal_queue_evidence_collection_applied)
            .saturating_add(self.self_goal_local_evidence_failed)
            .saturating_add(self.self_goal_local_evidence_write_allowed)
            .saturating_add(self.self_goal_local_evidence_applied)
            .saturating_add(self.evolution_goal_queue_store_write_rejected);
        sections.push(OperatorHealthSection::new(
            "self_goal_queue",
            self_goal_queue_events > 0,
            self_goal_queue_review > 0,
            self_goal_queue_blocked > 0,
            self_goal_queue_events,
            vec![
                OperatorHealthMetric::new("apply_events", self.self_goal_queue_apply_events),
                OperatorHealthMetric::new(
                    "apply_ready_records",
                    self.self_goal_queue_apply_ready_records,
                ),
                OperatorHealthMetric::new(
                    "apply_held_records",
                    self.self_goal_queue_apply_held_records,
                ),
                OperatorHealthMetric::new(
                    "continuation_events",
                    self.self_goal_queue_continuation_events,
                ),
                OperatorHealthMetric::new(
                    "continuation_ready",
                    self.self_goal_queue_continuation_ready,
                ),
                OperatorHealthMetric::new(
                    "continuation_held",
                    self.self_goal_queue_continuation_held,
                ),
                OperatorHealthMetric::new(
                    "continuation_required_evidence",
                    self.self_goal_queue_continuation_required_evidence,
                ),
                OperatorHealthMetric::new(
                    "continuation_budget_steps",
                    self.self_goal_queue_continuation_budget_steps,
                ),
                OperatorHealthMetric::new(
                    "evidence_plan_events",
                    self.self_goal_queue_evidence_plan_events,
                ),
                OperatorHealthMetric::new(
                    "evidence_plan_ready",
                    self.self_goal_queue_evidence_plan_ready,
                ),
                OperatorHealthMetric::new(
                    "evidence_plan_steps",
                    self.self_goal_queue_evidence_plan_steps,
                ),
                OperatorHealthMetric::new(
                    "evidence_plan_auto_collectible",
                    self.self_goal_queue_evidence_plan_auto_collectible,
                ),
                OperatorHealthMetric::new(
                    "evidence_plan_manual",
                    self.self_goal_queue_evidence_plan_manual,
                ),
                OperatorHealthMetric::new(
                    "evidence_collection_events",
                    self.self_goal_queue_evidence_collection_events,
                ),
                OperatorHealthMetric::new(
                    "evidence_collection_complete",
                    self.self_goal_queue_evidence_collection_complete,
                ),
                OperatorHealthMetric::new(
                    "evidence_collection_passed",
                    self.self_goal_queue_evidence_collection_passed,
                ),
                OperatorHealthMetric::new(
                    "evidence_collection_missing",
                    self.self_goal_queue_evidence_collection_missing,
                ),
                OperatorHealthMetric::new(
                    "evidence_collection_manual_missing",
                    self.self_goal_queue_evidence_collection_manual_missing,
                ),
                OperatorHealthMetric::new(
                    "local_evidence_events",
                    self.self_goal_local_evidence_events,
                ),
                OperatorHealthMetric::new(
                    "local_evidence_generated",
                    self.self_goal_local_evidence_generated,
                ),
                OperatorHealthMetric::new(
                    "local_evidence_failed",
                    self.self_goal_local_evidence_failed,
                ),
                OperatorHealthMetric::new(
                    "store_write_events",
                    self.evolution_goal_queue_store_write_events,
                ),
                OperatorHealthMetric::new(
                    "store_write_applied",
                    self.evolution_goal_queue_store_write_applied,
                ),
            ],
            self.passed,
        ));

        let approval_events = self
            .self_evolution_admission_events
            .saturating_add(self.self_evolution_experiment_events)
            .saturating_add(self.self_evolution_operator_approval_events)
            .saturating_add(self.self_evolution_promotion_preflight_events)
            .saturating_add(self.self_evolution_rollback_replay_gate_events)
            .saturating_add(self.self_evolution_rollback_replay_apply_events);
        let approval_review = self
            .self_evolution_admission_review_packets
            .saturating_add(self.self_evolution_operator_approval_review_packets)
            .saturating_add(self.self_evolution_promotion_preflight_review_packets)
            .saturating_add(self.self_evolution_rollback_replay_gate_review_packets)
            .saturating_add(self.self_evolution_rollback_replay_apply_review_packets);
        let approval_missing_refs = self
            .self_evolution_operator_approval_missing_review_packet_refs
            .saturating_add(self.self_evolution_promotion_preflight_missing_refs)
            .saturating_add(self.self_evolution_rollback_replay_gate_missing_review_packet_refs)
            .saturating_add(self.self_evolution_rollback_replay_apply_missing_refs);
        let approval_blocked = self
            .self_evolution_admission_blocked
            .saturating_add(self.self_evolution_experiment_reject)
            .saturating_add(self.self_evolution_experiment_conflicts)
            .saturating_add(self.self_evolution_operator_approval_held)
            .saturating_add(self.self_evolution_promotion_preflight_held)
            .saturating_add(self.self_evolution_promotion_preflight_blocked_reasons)
            .saturating_add(self.self_evolution_rollback_replay_gate_held)
            .saturating_add(self.self_evolution_rollback_replay_apply_held)
            .saturating_add(self.self_evolution_rollback_replay_apply_blocked)
            .saturating_add(approval_missing_refs);
        sections.push(OperatorHealthSection::new(
            "approval",
            approval_events > 0,
            approval_review > 0,
            approval_blocked > 0,
            approval_events,
            vec![
                OperatorHealthMetric::new("admission_events", self.self_evolution_admission_events),
                OperatorHealthMetric::new(
                    "admission_admitted",
                    self.self_evolution_admission_admitted,
                ),
                OperatorHealthMetric::new(
                    "admission_blocked",
                    self.self_evolution_admission_blocked,
                ),
                OperatorHealthMetric::new(
                    "experiment_events",
                    self.self_evolution_experiment_events,
                ),
                OperatorHealthMetric::new("experiment_admit", self.self_evolution_experiment_admit),
                OperatorHealthMetric::new("experiment_hold", self.self_evolution_experiment_hold),
                OperatorHealthMetric::new(
                    "experiment_reject",
                    self.self_evolution_experiment_reject,
                ),
                OperatorHealthMetric::new(
                    "operator_approval_events",
                    self.self_evolution_operator_approval_events,
                ),
                OperatorHealthMetric::new(
                    "operator_approved",
                    self.self_evolution_operator_approval_approved,
                ),
                OperatorHealthMetric::new(
                    "operator_held",
                    self.self_evolution_operator_approval_held,
                ),
                OperatorHealthMetric::new(
                    "promotion_preflight_events",
                    self.self_evolution_promotion_preflight_events,
                ),
                OperatorHealthMetric::new(
                    "promotion_ready",
                    self.self_evolution_promotion_preflight_ready,
                ),
                OperatorHealthMetric::new(
                    "promotion_held",
                    self.self_evolution_promotion_preflight_held,
                ),
                OperatorHealthMetric::new("review_packets", approval_review),
                OperatorHealthMetric::new("missing_review_refs", approval_missing_refs),
            ],
            self.passed,
        ));

        let rollback_events = self
            .self_evolution_rollback_replay_events
            .saturating_add(self.self_evolution_rollback_replay_gate_events)
            .saturating_add(self.self_evolution_rollback_replay_apply_events);
        let rollback_review = self
            .self_evolution_rollback_replay_gate_review_packets
            .saturating_add(self.self_evolution_rollback_replay_apply_review_packets)
            .saturating_add(self.self_evolution_rollback_replay_rollback_anchor_ids)
            .saturating_add(self.self_evolution_rollback_replay_gate_rollback_anchor_ids)
            .saturating_add(self.self_evolution_rollback_replay_apply_rollback_anchor_ids);
        let rollback_blocked = self
            .self_evolution_rollback_replay_blocked
            .saturating_add(self.self_evolution_rollback_replay_gate_blocked)
            .saturating_add(self.self_evolution_rollback_replay_gate_missing_review_packet_refs)
            .saturating_add(self.self_evolution_rollback_replay_apply_blocked)
            .saturating_add(self.self_evolution_rollback_replay_apply_missing_refs);
        sections.push(OperatorHealthSection::new(
            "rollback",
            rollback_events > 0,
            rollback_review > 0,
            rollback_blocked > 0,
            rollback_events,
            vec![
                OperatorHealthMetric::new(
                    "replay_events",
                    self.self_evolution_rollback_replay_events,
                ),
                OperatorHealthMetric::new(
                    "replay_items",
                    self.self_evolution_rollback_replay_items,
                ),
                OperatorHealthMetric::new(
                    "replayable",
                    self.self_evolution_rollback_replay_replayable,
                ),
                OperatorHealthMetric::new("blocked", self.self_evolution_rollback_replay_blocked),
                OperatorHealthMetric::new(
                    "rollback_anchor_ids",
                    self.self_evolution_rollback_replay_rollback_anchor_ids,
                ),
                OperatorHealthMetric::new(
                    "gate_events",
                    self.self_evolution_rollback_replay_gate_events,
                ),
                OperatorHealthMetric::new(
                    "apply_events",
                    self.self_evolution_rollback_replay_apply_events,
                ),
                OperatorHealthMetric::new(
                    "apply_ready",
                    self.self_evolution_rollback_replay_apply_ready,
                ),
                OperatorHealthMetric::new(
                    "apply_held",
                    self.self_evolution_rollback_replay_apply_held,
                ),
            ],
            self.passed,
        ));

        let privacy_events = self
            .improvement_corpus_events
            .saturating_add(self.business_contract_events);
        let privacy_review = self
            .improvement_corpus_privacy_rejected
            .saturating_add(self.business_contract_event_protocol_leaks)
            .saturating_add(self.business_contract_event_response_normalized)
            .saturating_add(self.business_contract_event_sanitized)
            .saturating_add(self.business_contract_event_canonical_fallbacks);
        let privacy_blocked = self
            .improvement_corpus_secret_leaks
            .saturating_add(self.business_contract_event_protocol_leaks);
        sections.push(OperatorHealthSection::new(
            "privacy",
            privacy_events > 0,
            privacy_review > 0,
            privacy_blocked > 0,
            privacy_events,
            vec![
                OperatorHealthMetric::new(
                    "improvement_corpus_events",
                    self.improvement_corpus_events,
                ),
                OperatorHealthMetric::new(
                    "privacy_rejected",
                    self.improvement_corpus_privacy_rejected,
                ),
                OperatorHealthMetric::new("secret_leaks", self.improvement_corpus_secret_leaks),
                OperatorHealthMetric::new(
                    "business_contract_events",
                    self.business_contract_events,
                ),
                OperatorHealthMetric::new(
                    "protocol_leaks",
                    self.business_contract_event_protocol_leaks,
                ),
                OperatorHealthMetric::new(
                    "response_normalized",
                    self.business_contract_event_response_normalized,
                ),
                OperatorHealthMetric::new("sanitized", self.business_contract_event_sanitized),
            ],
            self.passed,
        ));

        let benchmark_events = self
            .rust_check_events
            .saturating_add(self.business_contract_events)
            .saturating_add(self.improvement_corpus_events);
        let benchmark_review = self
            .rust_check_feedback_updates
            .saturating_add(self.business_contract_event_response_normalized)
            .saturating_add(self.business_contract_event_sanitized)
            .saturating_add(self.business_contract_event_canonical_fallbacks);
        let benchmark_blocked = self
            .rust_check_failed
            .saturating_add(self.business_contract_event_failed)
            .saturating_add(self.improvement_corpus_secret_leaks);
        sections.push(OperatorHealthSection::new(
            "benchmark",
            benchmark_events > 0,
            benchmark_review > 0,
            benchmark_blocked > 0,
            benchmark_events,
            vec![
                OperatorHealthMetric::new("rust_check_events", self.rust_check_events),
                OperatorHealthMetric::new("rust_check_passed", self.rust_check_passed),
                OperatorHealthMetric::new("rust_check_failed", self.rust_check_failed),
                OperatorHealthMetric::new(
                    "business_contract_events",
                    self.business_contract_events,
                ),
                OperatorHealthMetric::new(
                    "business_contract_passed",
                    self.business_contract_event_passed,
                ),
                OperatorHealthMetric::new(
                    "business_contract_failed",
                    self.business_contract_event_failed,
                ),
                OperatorHealthMetric::new(
                    "improvement_corpus_events",
                    self.improvement_corpus_events,
                ),
                OperatorHealthMetric::new(
                    "compiler_passed",
                    self.improvement_corpus_compiler_passed,
                ),
                OperatorHealthMetric::new("test_passed", self.improvement_corpus_test_passed),
                OperatorHealthMetric::new(
                    "benchmark_passed",
                    self.improvement_corpus_benchmark_passed,
                ),
            ],
            self.passed,
        ));

        OperatorHealthSnapshot {
            schema: OPERATOR_HEALTH_SCHEMA,
            passed: self.passed,
            checked_lines: self.checked_lines,
            failure_count: self.failures.len(),
            trace_ids: self.trace_experience_ids.clone(),
            sections,
        }
    }

    pub fn operator_health_json(&self) -> String {
        self.operator_health_snapshot().json_line()
    }
}

fn operator_health_section_json(section: &OperatorHealthSection) -> String {
    let metrics = section
        .metrics
        .iter()
        .map(|metric| format!("\"{}\":{}", json_escape(metric.name), metric.value))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"name\":\"{}\",\"status\":\"{}\",\"data_present\":{},\"ready\":{},\"review_required\":{},\"blocked\":{},\"events\":{},\"metrics\":{{{}}}}}",
        json_escape(section.name),
        json_escape(section.status()),
        section.data_present,
        section.ready,
        section.review_required,
        section.blocked,
        section.events,
        metrics
    )
}

fn u64_array_json(values: &[u64]) -> String {
    let values = values
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(",");
    format!("[{values}]")
}

fn string_array_json(values: &[String]) -> String {
    let values = values
        .iter()
        .map(|value| format!("\"{}\"", json_escape(value)))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{values}]")
}

pub fn evaluate_trace_schema_jsonl(path: impl AsRef<Path>) -> io::Result<TraceSchemaGateReport> {
    let content = fs::read_to_string(path)?;
    let mut checked_lines = 0;
    let mut used_experiences = 0;
    let mut imported_kv_blocks = 0;
    let mut runtime_kv_weak_import_pressure_milli = 0;
    let mut trace_experience_ids = Vec::new();
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
    let mut business_contract_event_missing_handling_signals = 0;
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
    let mut self_evolution_promotion_preflight_events = 0;
    let mut self_evolution_promotion_preflight_ready = 0;
    let mut self_evolution_promotion_preflight_held = 0;
    let mut self_evolution_promotion_preflight_review_packets = 0;
    let mut self_evolution_promotion_preflight_evidence_ids = 0;
    let mut self_evolution_promotion_preflight_rollback_anchor_ids = 0;
    let mut self_evolution_promotion_preflight_content_digests = 0;
    let mut self_evolution_promotion_preflight_source_report_schemas = 0;
    let mut self_evolution_promotion_preflight_missing_refs = 0;
    let mut self_evolution_promotion_preflight_blocked_reasons = 0;
    let mut self_evolution_promotion_preflight_write_allowed = 0;
    let mut self_evolution_promotion_preflight_applied = 0;
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
    let mut self_evolving_memory_store_consolidation_events = 0;
    let mut self_evolving_memory_store_consolidation_actions = 0;
    let mut self_evolving_memory_store_merge_previews = 0;
    let mut self_evolving_memory_store_decay_previews = 0;
    let mut self_evolving_memory_store_tombstone_previews = 0;
    let mut self_evolving_memory_store_merge_rejections = 0;
    let mut self_evolving_memory_store_contexts = 0;
    let mut self_evolving_memory_store_saved_tokens = 0;
    let mut self_evolving_memory_store_maintenance_actions = 0;
    let mut self_evolving_memory_store_admission_candidates = 0;
    let mut self_evolving_memory_store_write_allowed = 0;
    let mut self_evolving_memory_store_durable_write_allowed = 0;
    let mut self_evolving_memory_store_applied = 0;
    let mut self_evolving_memory_store_applied_to_disk = 0;
    let mut self_evolving_memory_store_source_quarantine_events = 0;
    let mut self_evolving_memory_store_source_quarantine_actions = 0;
    let mut self_evolving_memory_writeback_events = 0;
    let mut self_evolving_memory_writeback_source_case_digests = 0;
    let mut self_evolving_memory_writeback_attempted_records = 0;
    let mut self_evolving_memory_writeback_accepted_records = 0;
    let mut self_evolving_memory_writeback_records_before = 0;
    let mut self_evolving_memory_writeback_records_after = 0;
    let mut self_evolving_memory_writeback_tool_reliability_after = 0;
    let mut self_evolving_memory_writeback_tool_observations_after = 0;
    let mut self_evolving_memory_writeback_maintenance_actions = 0;
    let mut self_evolving_memory_writeback_merged_duplicate_episodes = 0;
    let mut self_evolving_memory_writeback_write_allowed = 0;
    let mut self_evolving_memory_writeback_durable_write_allowed = 0;
    let mut self_evolving_memory_writeback_applied = 0;
    let mut self_evolving_memory_writeback_applied_to_disk = 0;
    let mut self_evolving_memory_writeback_snapshot_changes = 0;
    let mut memory_runtime_kv_exported = 0;
    let mut memory_runtime_kv_stored = 0;
    let mut memory_runtime_kv_hold = 0;
    let mut memory_runtime_kv_held = 0;
    let mut memory_feedback_reinforced = 0;
    let mut memory_feedback_penalized = 0;
    let mut memory_feedback_reinforcement_milli = 0;
    let mut memory_feedback_penalty_milli = 0;
    let mut memory_feedback_updates = 0;
    let mut memory_feedback_applied = 0;
    let mut memory_feedback_removed = 0;
    let mut memory_feedback_missing = 0;
    let mut memory_feedback_strength_delta_milli = 0;
    let mut memory_residency_events = 0;
    let mut memory_residency_decisions = 0;
    let mut memory_residency_hot = 0;
    let mut memory_residency_warm = 0;
    let mut memory_residency_cold = 0;
    let mut memory_residency_quarantined = 0;
    let mut memory_residency_retired = 0;
    let mut memory_residency_protected_rollback_anchors = 0;
    let mut memory_residency_blocked_reasons = 0;
    let mut memory_residency_token_estimate = 0;
    let mut memory_residency_write_allowed = 0;
    let mut memory_residency_durable_write_allowed = 0;
    let mut memory_residency_applied = 0;
    let mut unified_writer_gate_events = 0;
    let mut unified_writer_gate_records = 0;
    let mut unified_writer_gate_memory_records = 0;
    let mut unified_writer_gate_genome_records = 0;
    let mut unified_writer_gate_experiment_ledger_records = 0;
    let mut unified_writer_gate_evolution_goal_queue_records = 0;
    let mut unified_writer_gate_ready_records = 0;
    let mut unified_writer_gate_held_records = 0;
    let mut unified_writer_gate_rejected_records = 0;
    let mut unified_writer_gate_preview_only_records = 0;
    let mut unified_writer_gate_reason_codes = 0;
    let mut unified_writer_gate_explicit_apply_required = 0;
    let mut unified_writer_gate_write_allowed = 0;
    let mut unified_writer_gate_durable_write_allowed = 0;
    let mut unified_writer_gate_applied = 0;
    let mut self_goal_queue_apply_events = 0;
    let mut self_goal_queue_apply_records = 0;
    let mut self_goal_queue_apply_ready_records = 0;
    let mut self_goal_queue_apply_held_records = 0;
    let mut self_goal_queue_apply_rejected_records = 0;
    let mut self_goal_queue_apply_reason_codes = 0;
    let mut self_goal_queue_apply_explicit_apply_required = 0;
    let mut self_goal_queue_apply_write_allowed = 0;
    let mut self_goal_queue_apply_applied = 0;
    let mut self_goal_queue_continuation_events = 0;
    let mut self_goal_queue_continuation_ready = 0;
    let mut self_goal_queue_continuation_held = 0;
    let mut self_goal_queue_continuation_current_queue = 0;
    let mut self_goal_queue_continuation_completion_resulting_queue = 0;
    let mut self_goal_queue_continuation_goals = 0;
    let mut self_goal_queue_continuation_required_evidence = 0;
    let mut self_goal_queue_continuation_reason_codes = 0;
    let mut self_goal_queue_continuation_budget_attempts = 0;
    let mut self_goal_queue_continuation_budget_steps = 0;
    let mut self_goal_queue_continuation_budget_tokens = 0;
    let mut self_goal_queue_continuation_budget_runtime_ms = 0;
    let mut self_goal_queue_continuation_write_allowed = 0;
    let mut self_goal_queue_continuation_applied = 0;
    let mut self_goal_queue_evidence_plan_events = 0;
    let mut self_goal_queue_evidence_plan_ready = 0;
    let mut self_goal_queue_evidence_plan_held = 0;
    let mut self_goal_queue_evidence_plan_steps = 0;
    let mut self_goal_queue_evidence_plan_auto_collectible = 0;
    let mut self_goal_queue_evidence_plan_manual = 0;
    let mut self_goal_queue_evidence_plan_required_evidence = 0;
    let mut self_goal_queue_evidence_plan_packet_templates = 0;
    let mut self_goal_queue_evidence_plan_command_templates = 0;
    let mut self_goal_queue_evidence_plan_write_allowed = 0;
    let mut self_goal_queue_evidence_plan_applied = 0;
    let mut self_goal_queue_evidence_collection_events = 0;
    let mut self_goal_queue_evidence_collection_ready = 0;
    let mut self_goal_queue_evidence_collection_complete = 0;
    let mut self_goal_queue_evidence_collection_steps = 0;
    let mut self_goal_queue_evidence_collection_collected = 0;
    let mut self_goal_queue_evidence_collection_passed = 0;
    let mut self_goal_queue_evidence_collection_failed = 0;
    let mut self_goal_queue_evidence_collection_missing = 0;
    let mut self_goal_queue_evidence_collection_manual_missing = 0;
    let mut self_goal_queue_evidence_collection_write_allowed = 0;
    let mut self_goal_queue_evidence_collection_applied = 0;
    let mut self_goal_local_evidence_events = 0;
    let mut self_goal_local_evidence_enabled = 0;
    let mut self_goal_local_evidence_dry_run = 0;
    let mut self_goal_local_evidence_ready = 0;
    let mut self_goal_local_evidence_steps = 0;
    let mut self_goal_local_evidence_attempted = 0;
    let mut self_goal_local_evidence_generated = 0;
    let mut self_goal_local_evidence_passed = 0;
    let mut self_goal_local_evidence_failed = 0;
    let mut self_goal_local_evidence_skipped = 0;
    let mut self_goal_local_evidence_manual = 0;
    let mut self_goal_local_evidence_planned_status = 0;
    let mut self_goal_local_evidence_write_allowed = 0;
    let mut self_goal_local_evidence_applied = 0;
    let mut coding_service_eval_events = 0;
    let mut coding_service_eval_readiness_events = 0;
    let mut coding_service_eval_runner_events = 0;
    let mut coding_service_eval_passed = 0;
    let mut coding_service_eval_requests = 0;
    let mut coding_service_eval_completed = 0;
    let mut coding_service_eval_evidence_packets = 0;
    let mut coding_service_eval_rust_validation_checked = 0;
    let mut coding_service_eval_compile_checked = 0;
    let mut coding_service_eval_unit_test_checked = 0;
    let mut coding_service_eval_write_allowed = 0;
    let mut coding_service_eval_applied = 0;
    let mut evolution_goal_queue_store_write_events = 0;
    let mut evolution_goal_queue_store_write_applied = 0;
    let mut evolution_goal_queue_store_write_held = 0;
    let mut evolution_goal_queue_store_write_rejected = 0;
    let mut evolution_goal_queue_store_write_reason_codes = 0;
    let mut evolution_goal_queue_store_write_durable_write_allowed = 0;
    let mut evolution_goal_queue_store_write_applied_to_disk = 0;
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
    let mut task_hierarchy_depth_total = 0usize;
    let mut task_hierarchy_route_fanout_total = 0usize;
    let mut task_hierarchy_threshold_delta_milli = 0usize;
    let mut task_hierarchy_selected_lanes = 0usize;
    let mut task_hierarchy_skipped_lanes = 0usize;
    let mut task_hierarchy_memory_lanes = 0usize;
    let mut task_hierarchy_skipped_memory_lanes = 0usize;
    let mut fht_dke_events = 0;
    let mut fht_dke_enabled = 0;
    let mut fht_dke_total_tokens = 0;
    let mut fht_dke_dense_tokens = 0;
    let mut fht_dke_routed_tokens = 0;
    let mut fht_dke_kv_exchange_blocks = 0;
    let mut fht_dke_token_split_invalid = 0;
    let mut fht_dke_attention_threshold_milli = 0;
    let mut fht_dke_route_pressure_milli = 0;
    let mut compute_budget_events = 0;
    let mut compute_budget_threshold_delta_milli = 0;
    let mut compute_budget_runtime_kv_budget_pressure_milli = 0;
    let mut compute_budget_low = 0;
    let mut compute_budget_normal = 0;
    let mut compute_budget_expanded = 0;
    let mut compute_budget_selected_candidates = 0;
    let mut compute_budget_low_value_skipped = 0;
    let mut compute_budget_kv_lookups_skipped = 0;
    let mut compute_budget_validation_cost_tokens = 0;
    let mut compute_budget_saved_tokens = 0;
    let mut compute_budget_self_evolving_memory_fusion_saved_tokens = 0;
    let mut compute_budget_avoided_tokens = 0;
    let mut compute_budget_fanout_before = 0usize;
    let mut compute_budget_fanout_after = 0usize;
    let mut compute_budget_fanout_reduction = 0usize;
    let mut compute_budget_estimated_budget_tokens = 0usize;
    let mut compute_budget_estimated_spent_tokens = 0usize;
    let mut compute_budget_estimated_saved_tokens = 0usize;
    let mut compute_budget_anchor_count = 0usize;
    let mut compute_budget_anchors_preserved = 0usize;
    let mut compute_budget_anchor_preservation_failures = 0usize;
    let mut compute_budget_fallback_triggered = 0usize;
    let mut compute_budget_write_allowed = 0;
    let mut compute_budget_applied = 0;
    let mut auto_replay_recursive_runtime_items = 0;
    let mut auto_replay_recursive_runtime_calls = 0;
    let mut auto_replay_avg_recursive_call_pressure_milli = 0;
    let mut auto_replay_max_recursive_call_pressure_milli = 0;
    let mut evolution_recursive_replay_items = 0;
    let mut evolution_recursive_runtime_calls = 0;
    let mut process_reward_events = 0;
    let mut process_reward_positive = 0;
    let mut process_reward_reinforce = 0;
    let mut process_reward_hold = 0;
    let mut process_reward_penalize = 0;
    let mut process_reward_total_milli = 0;
    let mut live_evolution_events = 0;
    let mut live_router_threshold_delta_milli = 0;
    let mut live_hierarchy_weight_delta_milli = 0;
    let mut live_online_reward_feedbacks = 0;
    let mut live_online_reward_reinforcements = 0;
    let mut live_online_reward_penalties = 0;
    let mut live_online_reward_strength_milli = 0;
    let mut live_memory_reinforcements = 0;
    let mut live_memory_penalties = 0;
    let mut live_memory_updates = 0;
    let mut live_stored_memories = 0;
    let mut live_stored_gist_memories = 0;
    let mut live_stored_runtime_kv_memories = 0;
    let mut live_stored_memory_updates = 0;
    let mut live_reflection_issues = 0;
    let mut live_critical_reflection_issues = 0;
    let mut live_revision_actions = 0;
    let mut evolution_live_inference_runs = 0;
    let mut evolution_live_router_threshold_mutations = 0;
    let mut evolution_live_hierarchy_weight_mutations = 0;
    let mut evolution_live_router_threshold_delta_milli = 0;
    let mut evolution_live_hierarchy_weight_delta_milli = 0;
    let mut evolution_live_online_reward_feedbacks = 0;
    let mut evolution_live_online_reward_reinforcements = 0;
    let mut evolution_live_online_reward_penalties = 0;
    let mut evolution_live_online_reward_strength_milli = 0;
    let mut evolution_live_online_reward_reinforcement_strength_milli = 0;
    let mut evolution_live_online_reward_penalty_strength_milli = 0;
    let mut evolution_live_memory_reinforcements = 0;
    let mut evolution_live_memory_penalties = 0;
    let mut evolution_live_memory_updates = 0;
    let mut evolution_live_stored_memories = 0;
    let mut evolution_live_stored_gist_memories = 0;
    let mut evolution_live_stored_runtime_kv_memories = 0;
    let mut evolution_live_stored_memory_updates = 0;
    let mut evolution_live_reflection_issues = 0;
    let mut evolution_live_critical_reflection_issues = 0;
    let mut evolution_live_revision_actions = 0;
    let mut auto_replay_live_memory_feedback_items = 0;
    let mut auto_replay_live_memory_feedback_updates = 0;
    let mut auto_replay_live_memory_feedback_reinforcements = 0;
    let mut auto_replay_live_memory_feedback_penalties = 0;
    let mut auto_replay_live_memory_feedback_detail_items = 0;
    let mut auto_replay_live_memory_feedback_applied = 0;
    let mut auto_replay_live_memory_feedback_removed = 0;
    let mut auto_replay_live_memory_feedback_missing = 0;
    let mut auto_replay_live_memory_feedback_strength_delta_milli = 0;
    let mut replay_live_memory_feedback_items = 0;
    let mut replay_live_memory_feedback_updates = 0;
    let mut replay_live_memory_feedback_reinforcements = 0;
    let mut replay_live_memory_feedback_penalties = 0;
    let mut replay_live_memory_feedback_detail_items = 0;
    let mut replay_live_memory_feedback_applied = 0;
    let mut replay_live_memory_feedback_removed = 0;
    let mut replay_live_memory_feedback_missing = 0;
    let mut replay_live_memory_feedback_strength_delta_milli = 0;
    let mut replay_live_evolution_items = 0;
    let mut replay_live_evolution_router_threshold_mutations = 0;
    let mut replay_live_evolution_hierarchy_weight_mutations = 0;
    let mut replay_live_evolution_router_threshold_delta_milli = 0;
    let mut replay_live_evolution_hierarchy_weight_delta_milli = 0;
    let mut replay_live_evolution_online_reward_feedbacks = 0;
    let mut replay_live_evolution_online_reward_reinforcements = 0;
    let mut replay_live_evolution_online_reward_penalties = 0;
    let mut replay_live_evolution_online_reward_strength_milli = 0;
    let mut replay_live_evolution_online_reward_reinforcement_strength_milli = 0;
    let mut replay_live_evolution_online_reward_penalty_strength_milli = 0;
    let mut replay_live_evolution_memory_updates = 0;
    let mut replay_live_evolution_stored_memory_updates = 0;
    let mut replay_live_evolution_reflection_issues = 0;
    let mut replay_live_evolution_critical_reflection_issues = 0;
    let mut replay_live_evolution_revision_actions = 0;
    let mut reasoning_genome_events = 0;
    let mut reasoning_genome_genes = 0;
    let mut reasoning_genome_active_genes = 0;
    let mut reasoning_genome_aged_genes = 0;
    let mut reasoning_genome_malignant_genes = 0;
    let mut reasoning_genome_relabel_candidates = 0;
    let mut reasoning_genome_regeneration_candidates = 0;
    let mut reasoning_genome_gene_scissors_proposals = 0;
    let mut reasoning_genome_repair_payloads = 0;
    let mut reasoning_genome_regeneration_payloads = 0;
    let mut reasoning_genome_lifecycle_records = 0;
    let mut reasoning_genome_lifecycle_tombstone_candidates = 0;
    let mut reasoning_genome_lifecycle_pending_validations = 0;
    let mut reasoning_genome_lifecycle_source_evidence = 0;
    let mut reasoning_genome_splice_segments = 0;
    let mut reasoning_genome_splice_exons = 0;
    let mut reasoning_genome_splice_introns = 0;
    let mut reasoning_genome_splice_variants = 0;
    let mut reasoning_genome_splice_quarantined = 0;
    let mut reasoning_genome_splice_repair_candidates = 0;
    let mut reasoning_genome_splice_findings = 0;
    let mut reasoning_genome_splice_proposals = 0;
    let mut reasoning_genome_write_allowed = 0;
    let mut reasoning_genome_mutation_applied = 0;
    let mut reasoning_genome_splice_write_allowed = 0;
    let mut reasoning_genome_splice_applied = 0;
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
    let mut noiron_orchestration_events = 0;
    let mut noiron_orchestration_stages = 0;
    let mut noiron_orchestration_failed_stages = 0;
    let mut noiron_orchestration_writes_gated = 0;
    let mut noiron_orchestration_fht_dke_total_tokens = 0;
    let mut orchestration_audit_events = 0;
    let mut orchestration_audit_checked_fields = 0;
    let mut orchestration_audit_failed_fields = 0;
    let mut orchestration_audit_failed_stages = 0;
    let mut orchestration_audit_integrity_failed_fields = 0;
    let mut failures = Vec::new();

    for (index, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        checked_lines += 1;
        if let Some(experience_id) = extract_json_nullable_u64_field(line, "experience_id") {
            trace_experience_ids.push(experience_id);
        }
        used_experiences += extract_json_usize_field(line, "used_experiences").unwrap_or(0);
        imported_kv_blocks += extract_json_usize_field(line, "imported_kv_blocks").unwrap_or(0);
        if let Some(runtime_diagnostics) = json_object_after_field(line, "runtime_diagnostics") {
            runtime_kv_weak_import_pressure_milli += extract_json_nullable_f32_field(
                runtime_diagnostics,
                "runtime_kv_weak_import_pressure",
            )
            .map(trace_gate_milli)
            .unwrap_or(0);
        }
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
            business_contract_event_missing_handling_signals += summary.missing_handling_signals;
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
        if let Some(summary) = self_evolution_promotion_preflight_trace_gate_summary(line) {
            self_evolution_promotion_preflight_events += summary.events;
            self_evolution_promotion_preflight_ready += summary.ready;
            self_evolution_promotion_preflight_held += summary.held;
            self_evolution_promotion_preflight_review_packets += summary.review_packets;
            self_evolution_promotion_preflight_evidence_ids += summary.evidence_ids;
            self_evolution_promotion_preflight_rollback_anchor_ids += summary.rollback_anchor_ids;
            self_evolution_promotion_preflight_content_digests += summary.content_digests;
            self_evolution_promotion_preflight_source_report_schemas +=
                summary.source_report_schemas;
            self_evolution_promotion_preflight_missing_refs += summary.missing_refs;
            self_evolution_promotion_preflight_blocked_reasons += summary.blocked_reasons;
            self_evolution_promotion_preflight_write_allowed += summary.write_allowed;
            self_evolution_promotion_preflight_applied += summary.applied;
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
            self_evolving_memory_store_consolidation_events += summary.consolidation_events;
            self_evolving_memory_store_consolidation_actions += summary.consolidation_actions;
            self_evolving_memory_store_merge_previews += summary.merge_previews;
            self_evolving_memory_store_decay_previews += summary.decay_previews;
            self_evolving_memory_store_tombstone_previews += summary.tombstone_previews;
            self_evolving_memory_store_merge_rejections += summary.merge_rejections;
            self_evolving_memory_store_contexts += summary.contexts;
            self_evolving_memory_store_saved_tokens += summary.saved_tokens;
            self_evolving_memory_store_maintenance_actions += summary.maintenance_actions;
            self_evolving_memory_store_admission_candidates += summary.admission_candidates;
            self_evolving_memory_store_write_allowed += summary.write_allowed;
            self_evolving_memory_store_durable_write_allowed += summary.durable_write_allowed;
            self_evolving_memory_store_applied += summary.applied;
            self_evolving_memory_store_applied_to_disk += summary.applied_to_disk;
            self_evolving_memory_store_source_quarantine_events += summary.source_quarantine_events;
            self_evolving_memory_store_source_quarantine_actions +=
                summary.source_quarantine_actions;
        }
        if let Some(summary) = self_evolving_memory_writeback_trace_gate_summary(line) {
            self_evolving_memory_writeback_events += summary.events;
            self_evolving_memory_writeback_source_case_digests += summary.source_case_digests;
            self_evolving_memory_writeback_attempted_records += summary.attempted_records;
            self_evolving_memory_writeback_accepted_records += summary.accepted_records;
            self_evolving_memory_writeback_records_before += summary.records_before;
            self_evolving_memory_writeback_records_after += summary.records_after;
            self_evolving_memory_writeback_tool_reliability_after += summary.tool_reliability_after;
            self_evolving_memory_writeback_tool_observations_after +=
                summary.tool_observations_after;
            self_evolving_memory_writeback_maintenance_actions += summary.maintenance_actions;
            self_evolving_memory_writeback_merged_duplicate_episodes +=
                summary.merged_duplicate_episodes;
            self_evolving_memory_writeback_write_allowed += summary.write_allowed;
            self_evolving_memory_writeback_durable_write_allowed += summary.durable_write_allowed;
            self_evolving_memory_writeback_applied += summary.applied;
            self_evolving_memory_writeback_applied_to_disk += summary.applied_to_disk;
            self_evolving_memory_writeback_snapshot_changes += summary.snapshot_changes;
        }
        if let Some(summary) = memory_trace_gate_summary(line) {
            memory_runtime_kv_exported += summary.runtime_kv_exported;
            memory_runtime_kv_stored += summary.runtime_kv_stored;
            memory_runtime_kv_hold += summary.runtime_kv_hold;
            memory_runtime_kv_held += summary.runtime_kv_held;
            memory_feedback_reinforced += summary.feedback_reinforced;
            memory_feedback_penalized += summary.feedback_penalized;
            memory_feedback_reinforcement_milli += summary.feedback_reinforcement_milli;
            memory_feedback_penalty_milli += summary.feedback_penalty_milli;
            memory_feedback_updates += summary.feedback_updates;
            memory_feedback_applied += summary.feedback_applied;
            memory_feedback_removed += summary.feedback_removed;
            memory_feedback_missing += summary.feedback_missing;
            memory_feedback_strength_delta_milli += summary.feedback_strength_delta_milli;
        }
        if let Some(summary) = memory_residency_trace_gate_summary(line) {
            memory_residency_events += summary.events;
            memory_residency_decisions += summary.decisions;
            memory_residency_hot += summary.hot;
            memory_residency_warm += summary.warm;
            memory_residency_cold += summary.cold;
            memory_residency_quarantined += summary.quarantined;
            memory_residency_retired += summary.retired;
            memory_residency_protected_rollback_anchors += summary.protected_rollback_anchors;
            memory_residency_blocked_reasons += summary.blocked_reasons;
            memory_residency_token_estimate += summary.token_estimate;
            memory_residency_write_allowed += summary.write_allowed;
            memory_residency_durable_write_allowed += summary.durable_write_allowed;
            memory_residency_applied += summary.applied;
        }
        if let Some(summary) = unified_writer_gate_trace_gate_summary(line) {
            unified_writer_gate_events += summary.events;
            unified_writer_gate_records += summary.records;
            unified_writer_gate_memory_records += summary.memory_records;
            unified_writer_gate_genome_records += summary.genome_records;
            unified_writer_gate_experiment_ledger_records += summary.experiment_ledger_records;
            unified_writer_gate_evolution_goal_queue_records +=
                summary.evolution_goal_queue_records;
            unified_writer_gate_ready_records += summary.ready_records;
            unified_writer_gate_held_records += summary.held_records;
            unified_writer_gate_rejected_records += summary.rejected_records;
            unified_writer_gate_preview_only_records += summary.preview_only_records;
            unified_writer_gate_reason_codes += summary.reason_codes;
            unified_writer_gate_explicit_apply_required += summary.explicit_apply_required;
            unified_writer_gate_write_allowed += summary.write_allowed;
            unified_writer_gate_durable_write_allowed += summary.durable_write_allowed;
            unified_writer_gate_applied += summary.applied;
        }
        if let Some(summary) = self_goal_queue_apply_trace_gate_summary(line) {
            self_goal_queue_apply_events += summary.events;
            self_goal_queue_apply_records += summary.records;
            self_goal_queue_apply_ready_records += summary.ready_records;
            self_goal_queue_apply_held_records += summary.held_records;
            self_goal_queue_apply_rejected_records += summary.rejected_records;
            self_goal_queue_apply_reason_codes += summary.reason_codes;
            self_goal_queue_apply_explicit_apply_required += summary.explicit_apply_required;
            self_goal_queue_apply_write_allowed += summary.write_allowed;
            self_goal_queue_apply_applied += summary.applied;
        }
        if let Some(summary) = self_goal_queue_continuation_trace_gate_summary(line) {
            self_goal_queue_continuation_events += summary.events;
            self_goal_queue_continuation_ready += summary.ready;
            self_goal_queue_continuation_held += summary.held;
            self_goal_queue_continuation_current_queue += summary.current_queue;
            self_goal_queue_continuation_completion_resulting_queue +=
                summary.completion_resulting_queue;
            self_goal_queue_continuation_goals += summary.goals;
            self_goal_queue_continuation_required_evidence += summary.required_evidence;
            self_goal_queue_continuation_reason_codes += summary.reason_codes;
            self_goal_queue_continuation_budget_attempts += summary.budget_attempts;
            self_goal_queue_continuation_budget_steps += summary.budget_steps;
            self_goal_queue_continuation_budget_tokens += summary.budget_tokens;
            self_goal_queue_continuation_budget_runtime_ms += summary.budget_runtime_ms;
            self_goal_queue_continuation_write_allowed += summary.write_allowed;
            self_goal_queue_continuation_applied += summary.applied;
        }
        if let Some(summary) = self_goal_queue_evidence_plan_trace_gate_summary(line) {
            self_goal_queue_evidence_plan_events += summary.events;
            self_goal_queue_evidence_plan_ready += summary.ready;
            self_goal_queue_evidence_plan_held += summary.held;
            self_goal_queue_evidence_plan_steps += summary.steps;
            self_goal_queue_evidence_plan_auto_collectible += summary.auto_collectible;
            self_goal_queue_evidence_plan_manual += summary.manual;
            self_goal_queue_evidence_plan_required_evidence += summary.required_evidence;
            self_goal_queue_evidence_plan_packet_templates += summary.packet_templates;
            self_goal_queue_evidence_plan_command_templates += summary.command_templates;
            self_goal_queue_evidence_plan_write_allowed += summary.write_allowed;
            self_goal_queue_evidence_plan_applied += summary.applied;
        }
        if let Some(summary) = self_goal_queue_evidence_collection_trace_gate_summary(line) {
            self_goal_queue_evidence_collection_events += summary.events;
            self_goal_queue_evidence_collection_ready += summary.ready;
            self_goal_queue_evidence_collection_complete += summary.complete;
            self_goal_queue_evidence_collection_steps += summary.steps;
            self_goal_queue_evidence_collection_collected += summary.collected;
            self_goal_queue_evidence_collection_passed += summary.passed;
            self_goal_queue_evidence_collection_failed += summary.failed;
            self_goal_queue_evidence_collection_missing += summary.missing;
            self_goal_queue_evidence_collection_manual_missing += summary.manual_missing;
            self_goal_queue_evidence_collection_write_allowed += summary.write_allowed;
            self_goal_queue_evidence_collection_applied += summary.applied;
        }
        if let Some(summary) = self_goal_local_evidence_trace_gate_summary(line) {
            self_goal_local_evidence_events += summary.events;
            self_goal_local_evidence_enabled += summary.enabled;
            self_goal_local_evidence_dry_run += summary.dry_run;
            self_goal_local_evidence_ready += summary.ready;
            self_goal_local_evidence_steps += summary.steps;
            self_goal_local_evidence_attempted += summary.attempted;
            self_goal_local_evidence_generated += summary.generated;
            self_goal_local_evidence_passed += summary.passed;
            self_goal_local_evidence_failed += summary.failed;
            self_goal_local_evidence_skipped += summary.skipped;
            self_goal_local_evidence_manual += summary.manual;
            self_goal_local_evidence_planned_status += summary.planned_status;
            self_goal_local_evidence_write_allowed += summary.write_allowed;
            self_goal_local_evidence_applied += summary.applied;
        }
        if let Some(summary) = coding_service_eval_trace_gate_summary(line) {
            coding_service_eval_events += summary.events;
            coding_service_eval_readiness_events += summary.readiness_events;
            coding_service_eval_runner_events += summary.runner_events;
            coding_service_eval_passed += summary.passed;
            coding_service_eval_requests += summary.requests;
            coding_service_eval_completed += summary.completed;
            coding_service_eval_evidence_packets += summary.evidence_packets;
            coding_service_eval_rust_validation_checked += summary.rust_validation_checked;
            coding_service_eval_compile_checked += summary.compile_checked;
            coding_service_eval_unit_test_checked += summary.unit_test_checked;
            coding_service_eval_write_allowed += summary.write_allowed;
            coding_service_eval_applied += summary.applied;
        }
        if let Some(summary) = evolution_goal_queue_store_write_trace_gate_summary(line) {
            evolution_goal_queue_store_write_events += summary.events;
            evolution_goal_queue_store_write_applied += summary.applied;
            evolution_goal_queue_store_write_held += summary.held;
            evolution_goal_queue_store_write_rejected += summary.rejected;
            evolution_goal_queue_store_write_reason_codes += summary.reason_codes;
            evolution_goal_queue_store_write_durable_write_allowed += summary.durable_write_allowed;
            evolution_goal_queue_store_write_applied_to_disk += summary.applied_to_disk;
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
            task_hierarchy_depth_total += summary.depth_total;
            task_hierarchy_route_fanout_total += summary.route_fanout_total;
            task_hierarchy_threshold_delta_milli += summary.threshold_delta_milli;
            task_hierarchy_selected_lanes += summary.selected_lanes;
            task_hierarchy_skipped_lanes += summary.skipped_lanes;
            task_hierarchy_memory_lanes += summary.memory_lanes;
            task_hierarchy_skipped_memory_lanes += summary.skipped_memory_lanes;
        }
        if let Some(summary) = fht_dke_trace_gate_summary(line) {
            fht_dke_events += summary.events;
            fht_dke_enabled += summary.enabled;
            fht_dke_total_tokens += summary.total_tokens;
            fht_dke_dense_tokens += summary.dense_tokens;
            fht_dke_routed_tokens += summary.routed_tokens;
            fht_dke_kv_exchange_blocks += summary.kv_exchange_blocks;
            fht_dke_token_split_invalid += summary.token_split_invalid;
            fht_dke_attention_threshold_milli += summary.attention_threshold_milli;
            fht_dke_route_pressure_milli += summary.route_pressure_milli;
        }
        if let Some(summary) = compute_budget_trace_gate_summary(line) {
            compute_budget_events += summary.events;
            compute_budget_threshold_delta_milli += summary.threshold_delta_milli;
            compute_budget_runtime_kv_budget_pressure_milli +=
                summary.runtime_kv_budget_pressure_milli;
            compute_budget_low += summary.low;
            compute_budget_normal += summary.normal;
            compute_budget_expanded += summary.expanded;
            compute_budget_selected_candidates += summary.selected_candidates;
            compute_budget_low_value_skipped += summary.low_value_skipped;
            compute_budget_kv_lookups_skipped += summary.kv_lookups_skipped;
            compute_budget_validation_cost_tokens += summary.validation_cost_tokens;
            compute_budget_saved_tokens += summary.saved_tokens;
            compute_budget_self_evolving_memory_fusion_saved_tokens +=
                summary.self_evolving_memory_fusion_saved_tokens;
            compute_budget_avoided_tokens += summary.avoided_tokens;
            compute_budget_fanout_before =
                compute_budget_fanout_before.saturating_add(summary.fanout_before);
            compute_budget_fanout_after =
                compute_budget_fanout_after.saturating_add(summary.fanout_after);
            compute_budget_fanout_reduction = compute_budget_fanout_reduction
                .saturating_add(summary.fanout_before.saturating_sub(summary.fanout_after));
            compute_budget_estimated_budget_tokens = compute_budget_estimated_budget_tokens
                .saturating_add(summary.estimated_budget_tokens);
            compute_budget_estimated_spent_tokens = compute_budget_estimated_spent_tokens
                .saturating_add(summary.estimated_spent_tokens);
            compute_budget_estimated_saved_tokens = compute_budget_estimated_saved_tokens
                .saturating_add(
                    summary
                        .estimated_budget_tokens
                        .saturating_sub(summary.estimated_spent_tokens),
                );
            compute_budget_anchor_count =
                compute_budget_anchor_count.saturating_add(summary.anchor_count);
            compute_budget_anchors_preserved =
                compute_budget_anchors_preserved.saturating_add(summary.anchors_preserved);
            compute_budget_anchor_preservation_failures =
                compute_budget_anchor_preservation_failures.saturating_add(usize::from(
                    summary.anchors_preserved < summary.anchor_count,
                ));
            compute_budget_fallback_triggered =
                compute_budget_fallback_triggered.saturating_add(summary.fallback_triggered);
            compute_budget_write_allowed += summary.write_allowed;
            compute_budget_applied += summary.applied;
        }
        if let Some(summary) = process_reward_trace_gate_summary(line) {
            process_reward_events += summary.events;
            process_reward_positive += summary.positive;
            process_reward_reinforce += summary.reinforce;
            process_reward_hold += summary.hold;
            process_reward_penalize += summary.penalize;
            process_reward_total_milli += summary.total_milli;
        }
        if let Some(summary) = live_evolution_trace_gate_summary(line) {
            live_evolution_events += summary.events;
            live_router_threshold_delta_milli += summary.router_threshold_delta_milli;
            live_hierarchy_weight_delta_milli += summary.hierarchy_weight_delta_milli;
            live_online_reward_feedbacks += summary.online_reward_feedbacks;
            live_online_reward_reinforcements += summary.online_reward_reinforcements;
            live_online_reward_penalties += summary.online_reward_penalties;
            live_online_reward_strength_milli += summary.online_reward_strength_milli;
            live_memory_reinforcements += summary.memory_reinforcements;
            live_memory_penalties += summary.memory_penalties;
            live_memory_updates += summary.memory_updates;
            live_stored_memories += summary.stored_memories;
            live_stored_gist_memories += summary.stored_gist_memories;
            live_stored_runtime_kv_memories += summary.stored_runtime_kv_memories;
            live_stored_memory_updates += summary.stored_memory_updates;
            live_reflection_issues += summary.reflection_issues;
            live_critical_reflection_issues += summary.critical_reflection_issues;
            live_revision_actions += summary.revision_actions;
        }
        if let Some(auto_replay) = json_object_after_field(line, "auto_replay") {
            auto_replay_live_memory_feedback_items +=
                extract_json_usize_field(auto_replay, "live_memory_feedback_items").unwrap_or(0);
            auto_replay_live_memory_feedback_updates +=
                extract_json_usize_field(auto_replay, "live_memory_feedback_updates").unwrap_or(0);
            auto_replay_live_memory_feedback_reinforcements +=
                extract_json_usize_field(auto_replay, "live_memory_feedback_reinforcements")
                    .unwrap_or(0);
            auto_replay_live_memory_feedback_penalties +=
                extract_json_usize_field(auto_replay, "live_memory_feedback_penalties")
                    .unwrap_or(0);
            auto_replay_live_memory_feedback_detail_items +=
                extract_json_usize_field(auto_replay, "live_memory_feedback_detail_items")
                    .unwrap_or(0);
            auto_replay_live_memory_feedback_applied +=
                extract_json_usize_field(auto_replay, "live_memory_feedback_applied").unwrap_or(0);
            auto_replay_live_memory_feedback_removed +=
                extract_json_usize_field(auto_replay, "live_memory_feedback_removed").unwrap_or(0);
            auto_replay_live_memory_feedback_missing +=
                extract_json_usize_field(auto_replay, "live_memory_feedback_missing").unwrap_or(0);
            auto_replay_live_memory_feedback_strength_delta_milli += trace_gate_milli(
                extract_json_f32_field(auto_replay, "live_memory_feedback_strength_delta")
                    .unwrap_or(0.0),
            );
            auto_replay_recursive_runtime_items +=
                extract_json_usize_field(auto_replay, "recursive_runtime_items").unwrap_or(0);
            auto_replay_recursive_runtime_calls +=
                extract_json_usize_field(auto_replay, "recursive_runtime_calls").unwrap_or(0);
            auto_replay_avg_recursive_call_pressure_milli += trace_gate_milli(
                extract_json_f32_field(auto_replay, "avg_recursive_call_pressure").unwrap_or(0.0),
            );
            auto_replay_max_recursive_call_pressure_milli += trace_gate_milli(
                extract_json_f32_field(auto_replay, "max_recursive_call_pressure").unwrap_or(0.0),
            );
        }
        if let Some(ledger) = json_object_after_field(line, "evolution_ledger") {
            evolution_live_inference_runs = evolution_live_inference_runs
                .max(extract_json_usize_field(ledger, "live_inference_runs").unwrap_or(0));
            evolution_live_router_threshold_mutations = evolution_live_router_threshold_mutations
                .max(
                    extract_json_usize_field(ledger, "cumulative_live_router_threshold_mutations")
                        .unwrap_or(0),
                );
            evolution_live_hierarchy_weight_mutations = evolution_live_hierarchy_weight_mutations
                .max(
                    extract_json_usize_field(ledger, "cumulative_live_hierarchy_weight_mutations")
                        .unwrap_or(0),
                );
            evolution_live_router_threshold_delta_milli =
                evolution_live_router_threshold_delta_milli.max(trace_gate_milli(
                    extract_json_f32_field(ledger, "cumulative_live_router_threshold_delta")
                        .unwrap_or(0.0),
                ));
            evolution_live_hierarchy_weight_delta_milli =
                evolution_live_hierarchy_weight_delta_milli.max(trace_gate_milli(
                    extract_json_f32_field(ledger, "cumulative_live_hierarchy_weight_delta")
                        .unwrap_or(0.0),
                ));
            evolution_live_online_reward_feedbacks = evolution_live_online_reward_feedbacks.max(
                extract_json_usize_field(ledger, "cumulative_live_online_reward_feedbacks")
                    .unwrap_or(0),
            );
            evolution_live_online_reward_reinforcements =
                evolution_live_online_reward_reinforcements.max(
                    extract_json_usize_field(
                        ledger,
                        "cumulative_live_online_reward_reinforcements",
                    )
                    .unwrap_or(0),
                );
            evolution_live_online_reward_penalties = evolution_live_online_reward_penalties.max(
                extract_json_usize_field(ledger, "cumulative_live_online_reward_penalties")
                    .unwrap_or(0),
            );
            evolution_live_online_reward_strength_milli =
                evolution_live_online_reward_strength_milli.max(trace_gate_milli(
                    extract_json_f32_field(ledger, "cumulative_live_online_reward_strength")
                        .unwrap_or(0.0),
                ));
            evolution_live_online_reward_reinforcement_strength_milli =
                evolution_live_online_reward_reinforcement_strength_milli.max(trace_gate_milli(
                    extract_json_f32_field(
                        ledger,
                        "cumulative_live_online_reward_reinforcement_strength",
                    )
                    .unwrap_or(0.0),
                ));
            evolution_live_online_reward_penalty_strength_milli =
                evolution_live_online_reward_penalty_strength_milli.max(trace_gate_milli(
                    extract_json_f32_field(
                        ledger,
                        "cumulative_live_online_reward_penalty_strength",
                    )
                    .unwrap_or(0.0),
                ));
            evolution_live_memory_reinforcements = evolution_live_memory_reinforcements.max(
                extract_json_usize_field(ledger, "cumulative_live_memory_reinforcements")
                    .unwrap_or(0),
            );
            evolution_live_memory_penalties = evolution_live_memory_penalties.max(
                extract_json_usize_field(ledger, "cumulative_live_memory_penalties").unwrap_or(0),
            );
            evolution_live_memory_updates = evolution_live_memory_updates.max(
                extract_json_usize_field(ledger, "cumulative_live_memory_updates").unwrap_or(0),
            );
            evolution_live_stored_memories = evolution_live_stored_memories.max(
                extract_json_usize_field(ledger, "cumulative_live_stored_memories").unwrap_or(0),
            );
            evolution_live_stored_gist_memories = evolution_live_stored_gist_memories.max(
                extract_json_usize_field(ledger, "cumulative_live_stored_gist_memories")
                    .unwrap_or(0),
            );
            evolution_live_stored_runtime_kv_memories = evolution_live_stored_runtime_kv_memories
                .max(
                    extract_json_usize_field(ledger, "cumulative_live_stored_runtime_kv_memories")
                        .unwrap_or(0),
                );
            evolution_live_stored_memory_updates = evolution_live_stored_memory_updates.max(
                extract_json_usize_field(ledger, "cumulative_live_stored_memory_updates")
                    .unwrap_or(0),
            );
            evolution_live_reflection_issues = evolution_live_reflection_issues.max(
                extract_json_usize_field(ledger, "cumulative_live_reflection_issues").unwrap_or(0),
            );
            evolution_live_critical_reflection_issues = evolution_live_critical_reflection_issues
                .max(
                    extract_json_usize_field(ledger, "cumulative_live_critical_reflection_issues")
                        .unwrap_or(0),
                );
            evolution_live_revision_actions = evolution_live_revision_actions.max(
                extract_json_usize_field(ledger, "cumulative_live_revision_actions").unwrap_or(0),
            );
            replay_live_memory_feedback_items = replay_live_memory_feedback_items.max(
                extract_json_usize_field(ledger, "cumulative_replay_live_memory_feedback_items")
                    .unwrap_or(0),
            );
            replay_live_memory_feedback_updates = replay_live_memory_feedback_updates.max(
                extract_json_usize_field(ledger, "cumulative_replay_live_memory_feedback_updates")
                    .unwrap_or(0),
            );
            replay_live_memory_feedback_reinforcements = replay_live_memory_feedback_reinforcements
                .max(
                    extract_json_usize_field(
                        ledger,
                        "cumulative_replay_live_memory_feedback_reinforcements",
                    )
                    .unwrap_or(0),
                );
            replay_live_memory_feedback_penalties = replay_live_memory_feedback_penalties.max(
                extract_json_usize_field(
                    ledger,
                    "cumulative_replay_live_memory_feedback_penalties",
                )
                .unwrap_or(0),
            );
            replay_live_memory_feedback_detail_items = replay_live_memory_feedback_detail_items
                .max(
                    extract_json_usize_field(
                        ledger,
                        "cumulative_replay_live_memory_feedback_detail_items",
                    )
                    .unwrap_or(0),
                );
            replay_live_memory_feedback_applied = replay_live_memory_feedback_applied.max(
                extract_json_usize_field(ledger, "cumulative_replay_live_memory_feedback_applied")
                    .unwrap_or(0),
            );
            replay_live_memory_feedback_removed = replay_live_memory_feedback_removed.max(
                extract_json_usize_field(ledger, "cumulative_replay_live_memory_feedback_removed")
                    .unwrap_or(0),
            );
            replay_live_memory_feedback_missing = replay_live_memory_feedback_missing.max(
                extract_json_usize_field(ledger, "cumulative_replay_live_memory_feedback_missing")
                    .unwrap_or(0),
            );
            replay_live_memory_feedback_strength_delta_milli =
                replay_live_memory_feedback_strength_delta_milli.max(trace_gate_milli(
                    extract_json_f32_field(
                        ledger,
                        "cumulative_replay_live_memory_feedback_strength_delta",
                    )
                    .unwrap_or(0.0),
                ));
            replay_live_evolution_items = replay_live_evolution_items.max(
                extract_json_usize_field(ledger, "cumulative_replay_live_evolution_items")
                    .unwrap_or(0),
            );
            replay_live_evolution_router_threshold_mutations =
                replay_live_evolution_router_threshold_mutations.max(
                    extract_json_usize_field(
                        ledger,
                        "cumulative_replay_live_evolution_router_threshold_mutations",
                    )
                    .unwrap_or(0),
                );
            replay_live_evolution_hierarchy_weight_mutations =
                replay_live_evolution_hierarchy_weight_mutations.max(
                    extract_json_usize_field(
                        ledger,
                        "cumulative_replay_live_evolution_hierarchy_weight_mutations",
                    )
                    .unwrap_or(0),
                );
            replay_live_evolution_router_threshold_delta_milli =
                replay_live_evolution_router_threshold_delta_milli.max(trace_gate_milli(
                    extract_json_f32_field(
                        ledger,
                        "cumulative_replay_live_evolution_router_threshold_delta",
                    )
                    .unwrap_or(0.0),
                ));
            replay_live_evolution_hierarchy_weight_delta_milli =
                replay_live_evolution_hierarchy_weight_delta_milli.max(trace_gate_milli(
                    extract_json_f32_field(
                        ledger,
                        "cumulative_replay_live_evolution_hierarchy_weight_delta",
                    )
                    .unwrap_or(0.0),
                ));
            replay_live_evolution_online_reward_feedbacks =
                replay_live_evolution_online_reward_feedbacks.max(
                    extract_json_usize_field(
                        ledger,
                        "cumulative_replay_live_evolution_online_reward_feedbacks",
                    )
                    .unwrap_or(0),
                );
            replay_live_evolution_online_reward_reinforcements =
                replay_live_evolution_online_reward_reinforcements.max(
                    extract_json_usize_field(
                        ledger,
                        "cumulative_replay_live_evolution_online_reward_reinforcements",
                    )
                    .unwrap_or(0),
                );
            replay_live_evolution_online_reward_penalties =
                replay_live_evolution_online_reward_penalties.max(
                    extract_json_usize_field(
                        ledger,
                        "cumulative_replay_live_evolution_online_reward_penalties",
                    )
                    .unwrap_or(0),
                );
            replay_live_evolution_online_reward_strength_milli =
                replay_live_evolution_online_reward_strength_milli.max(trace_gate_milli(
                    extract_json_f32_field(
                        ledger,
                        "cumulative_replay_live_evolution_online_reward_strength",
                    )
                    .unwrap_or(0.0),
                ));
            replay_live_evolution_online_reward_reinforcement_strength_milli =
                replay_live_evolution_online_reward_reinforcement_strength_milli.max(
                    trace_gate_milli(
                        extract_json_f32_field(
                            ledger,
                            "cumulative_replay_live_evolution_online_reward_reinforcement_strength",
                        )
                        .unwrap_or(0.0),
                    ),
                );
            replay_live_evolution_online_reward_penalty_strength_milli =
                replay_live_evolution_online_reward_penalty_strength_milli.max(trace_gate_milli(
                    extract_json_f32_field(
                        ledger,
                        "cumulative_replay_live_evolution_online_reward_penalty_strength",
                    )
                    .unwrap_or(0.0),
                ));
            replay_live_evolution_memory_updates = replay_live_evolution_memory_updates.max(
                extract_json_usize_field(ledger, "cumulative_replay_live_evolution_memory_updates")
                    .unwrap_or(0),
            );
            replay_live_evolution_stored_memory_updates =
                replay_live_evolution_stored_memory_updates.max(
                    extract_json_usize_field(
                        ledger,
                        "cumulative_replay_live_evolution_stored_memory_updates",
                    )
                    .unwrap_or(0),
                );
            replay_live_evolution_reflection_issues = replay_live_evolution_reflection_issues.max(
                extract_json_usize_field(
                    ledger,
                    "cumulative_replay_live_evolution_reflection_issues",
                )
                .unwrap_or(0),
            );
            replay_live_evolution_critical_reflection_issues =
                replay_live_evolution_critical_reflection_issues.max(
                    extract_json_usize_field(
                        ledger,
                        "cumulative_replay_live_evolution_critical_reflection_issues",
                    )
                    .unwrap_or(0),
                );
            replay_live_evolution_revision_actions = replay_live_evolution_revision_actions.max(
                extract_json_usize_field(
                    ledger,
                    "cumulative_replay_live_evolution_revision_actions",
                )
                .unwrap_or(0),
            );
            evolution_recursive_replay_items = evolution_recursive_replay_items.max(
                extract_json_usize_field(ledger, "cumulative_recursive_replay_items").unwrap_or(0),
            );
            evolution_recursive_runtime_calls = evolution_recursive_runtime_calls.max(
                extract_json_usize_field(ledger, "cumulative_recursive_runtime_calls").unwrap_or(0),
            );
        }
        if let Some(summary) = reasoning_genome_trace_gate_summary(line) {
            reasoning_genome_events += summary.events;
            reasoning_genome_genes += summary.genes;
            reasoning_genome_active_genes += summary.active_genes;
            reasoning_genome_aged_genes += summary.aged_genes;
            reasoning_genome_malignant_genes += summary.malignant_genes;
            reasoning_genome_relabel_candidates += summary.relabel_candidates;
            reasoning_genome_regeneration_candidates += summary.regeneration_candidates;
            reasoning_genome_gene_scissors_proposals += summary.gene_scissors_proposals;
            reasoning_genome_repair_payloads += summary.repair_payloads;
            reasoning_genome_regeneration_payloads += summary.regeneration_payloads;
            reasoning_genome_lifecycle_records += summary.lifecycle_records;
            reasoning_genome_lifecycle_tombstone_candidates +=
                summary.lifecycle_tombstone_candidates;
            reasoning_genome_lifecycle_pending_validations += summary.lifecycle_pending_validations;
            reasoning_genome_lifecycle_source_evidence += summary.lifecycle_source_evidence;
            reasoning_genome_splice_segments += summary.splice_segments;
            reasoning_genome_splice_exons += summary.splice_exons;
            reasoning_genome_splice_introns += summary.splice_introns;
            reasoning_genome_splice_variants += summary.splice_variants;
            reasoning_genome_splice_quarantined += summary.splice_quarantined;
            reasoning_genome_splice_repair_candidates += summary.splice_repair_candidates;
            reasoning_genome_splice_findings += summary.splice_findings;
            reasoning_genome_splice_proposals += summary.splice_proposals;
            reasoning_genome_write_allowed += summary.write_allowed;
            reasoning_genome_mutation_applied += summary.mutation_applied;
            reasoning_genome_splice_write_allowed += summary.splice_write_allowed;
            reasoning_genome_splice_applied += summary.splice_applied;
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
        if let Some(summary) = orchestration_audit_trace_gate_summary(line) {
            orchestration_audit_events += summary.events;
            orchestration_audit_checked_fields += summary.checked_fields;
            orchestration_audit_failed_fields += summary.failed_fields;
            orchestration_audit_failed_stages += summary.failed_stages;
            orchestration_audit_integrity_failed_fields += summary.integrity_failed_fields;
        }
        if let Some(summary) = noiron_orchestration_trace_gate_summary(line) {
            noiron_orchestration_events += summary.events;
            noiron_orchestration_stages += summary.stages;
            noiron_orchestration_failed_stages += summary.failed_stages;
            noiron_orchestration_writes_gated += summary.writes_gated;
            noiron_orchestration_fht_dke_total_tokens += summary.fht_dke_total_tokens;
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
        used_experiences,
        imported_kv_blocks,
        runtime_kv_weak_import_pressure_milli,
        trace_experience_ids,
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
        business_contract_event_missing_handling_signals,
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
        self_evolution_promotion_preflight_events,
        self_evolution_promotion_preflight_ready,
        self_evolution_promotion_preflight_held,
        self_evolution_promotion_preflight_review_packets,
        self_evolution_promotion_preflight_evidence_ids,
        self_evolution_promotion_preflight_rollback_anchor_ids,
        self_evolution_promotion_preflight_content_digests,
        self_evolution_promotion_preflight_source_report_schemas,
        self_evolution_promotion_preflight_missing_refs,
        self_evolution_promotion_preflight_blocked_reasons,
        self_evolution_promotion_preflight_write_allowed,
        self_evolution_promotion_preflight_applied,
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
        self_evolving_memory_store_consolidation_events,
        self_evolving_memory_store_consolidation_actions,
        self_evolving_memory_store_merge_previews,
        self_evolving_memory_store_decay_previews,
        self_evolving_memory_store_tombstone_previews,
        self_evolving_memory_store_merge_rejections,
        self_evolving_memory_store_contexts,
        self_evolving_memory_store_saved_tokens,
        self_evolving_memory_store_maintenance_actions,
        self_evolving_memory_store_admission_candidates,
        self_evolving_memory_store_write_allowed,
        self_evolving_memory_store_durable_write_allowed,
        self_evolving_memory_store_applied,
        self_evolving_memory_store_applied_to_disk,
        self_evolving_memory_store_source_quarantine_events,
        self_evolving_memory_store_source_quarantine_actions,
        self_evolving_memory_writeback_events,
        self_evolving_memory_writeback_source_case_digests,
        self_evolving_memory_writeback_attempted_records,
        self_evolving_memory_writeback_accepted_records,
        self_evolving_memory_writeback_records_before,
        self_evolving_memory_writeback_records_after,
        self_evolving_memory_writeback_tool_reliability_after,
        self_evolving_memory_writeback_tool_observations_after,
        self_evolving_memory_writeback_maintenance_actions,
        self_evolving_memory_writeback_merged_duplicate_episodes,
        self_evolving_memory_writeback_write_allowed,
        self_evolving_memory_writeback_durable_write_allowed,
        self_evolving_memory_writeback_applied,
        self_evolving_memory_writeback_applied_to_disk,
        self_evolving_memory_writeback_snapshot_changes,
        memory_runtime_kv_exported,
        memory_runtime_kv_stored,
        memory_runtime_kv_hold,
        memory_runtime_kv_held,
        memory_feedback_reinforced,
        memory_feedback_penalized,
        memory_feedback_reinforcement_milli,
        memory_feedback_penalty_milli,
        memory_feedback_updates,
        memory_feedback_applied,
        memory_feedback_removed,
        memory_feedback_missing,
        memory_feedback_strength_delta_milli,
        memory_residency_events,
        memory_residency_decisions,
        memory_residency_hot,
        memory_residency_warm,
        memory_residency_cold,
        memory_residency_quarantined,
        memory_residency_retired,
        memory_residency_protected_rollback_anchors,
        memory_residency_blocked_reasons,
        memory_residency_token_estimate,
        memory_residency_write_allowed,
        memory_residency_durable_write_allowed,
        memory_residency_applied,
        unified_writer_gate_events,
        unified_writer_gate_records,
        unified_writer_gate_memory_records,
        unified_writer_gate_genome_records,
        unified_writer_gate_experiment_ledger_records,
        unified_writer_gate_evolution_goal_queue_records,
        unified_writer_gate_ready_records,
        unified_writer_gate_held_records,
        unified_writer_gate_rejected_records,
        unified_writer_gate_preview_only_records,
        unified_writer_gate_reason_codes,
        unified_writer_gate_explicit_apply_required,
        unified_writer_gate_write_allowed,
        unified_writer_gate_durable_write_allowed,
        unified_writer_gate_applied,
        self_goal_queue_apply_events,
        self_goal_queue_apply_records,
        self_goal_queue_apply_ready_records,
        self_goal_queue_apply_held_records,
        self_goal_queue_apply_rejected_records,
        self_goal_queue_apply_reason_codes,
        self_goal_queue_apply_explicit_apply_required,
        self_goal_queue_apply_write_allowed,
        self_goal_queue_apply_applied,
        self_goal_queue_continuation_events,
        self_goal_queue_continuation_ready,
        self_goal_queue_continuation_held,
        self_goal_queue_continuation_current_queue,
        self_goal_queue_continuation_completion_resulting_queue,
        self_goal_queue_continuation_goals,
        self_goal_queue_continuation_required_evidence,
        self_goal_queue_continuation_reason_codes,
        self_goal_queue_continuation_budget_attempts,
        self_goal_queue_continuation_budget_steps,
        self_goal_queue_continuation_budget_tokens,
        self_goal_queue_continuation_budget_runtime_ms,
        self_goal_queue_continuation_write_allowed,
        self_goal_queue_continuation_applied,
        self_goal_queue_evidence_plan_events,
        self_goal_queue_evidence_plan_ready,
        self_goal_queue_evidence_plan_held,
        self_goal_queue_evidence_plan_steps,
        self_goal_queue_evidence_plan_auto_collectible,
        self_goal_queue_evidence_plan_manual,
        self_goal_queue_evidence_plan_required_evidence,
        self_goal_queue_evidence_plan_packet_templates,
        self_goal_queue_evidence_plan_command_templates,
        self_goal_queue_evidence_plan_write_allowed,
        self_goal_queue_evidence_plan_applied,
        self_goal_queue_evidence_collection_events,
        self_goal_queue_evidence_collection_ready,
        self_goal_queue_evidence_collection_complete,
        self_goal_queue_evidence_collection_steps,
        self_goal_queue_evidence_collection_collected,
        self_goal_queue_evidence_collection_passed,
        self_goal_queue_evidence_collection_failed,
        self_goal_queue_evidence_collection_missing,
        self_goal_queue_evidence_collection_manual_missing,
        self_goal_queue_evidence_collection_write_allowed,
        self_goal_queue_evidence_collection_applied,
        self_goal_local_evidence_events,
        self_goal_local_evidence_enabled,
        self_goal_local_evidence_dry_run,
        self_goal_local_evidence_ready,
        self_goal_local_evidence_steps,
        self_goal_local_evidence_attempted,
        self_goal_local_evidence_generated,
        self_goal_local_evidence_passed,
        self_goal_local_evidence_failed,
        self_goal_local_evidence_skipped,
        self_goal_local_evidence_manual,
        self_goal_local_evidence_planned_status,
        self_goal_local_evidence_write_allowed,
        self_goal_local_evidence_applied,
        coding_service_eval_events,
        coding_service_eval_readiness_events,
        coding_service_eval_runner_events,
        coding_service_eval_passed,
        coding_service_eval_requests,
        coding_service_eval_completed,
        coding_service_eval_evidence_packets,
        coding_service_eval_rust_validation_checked,
        coding_service_eval_compile_checked,
        coding_service_eval_unit_test_checked,
        coding_service_eval_write_allowed,
        coding_service_eval_applied,
        evolution_goal_queue_store_write_events,
        evolution_goal_queue_store_write_applied,
        evolution_goal_queue_store_write_held,
        evolution_goal_queue_store_write_rejected,
        evolution_goal_queue_store_write_reason_codes,
        evolution_goal_queue_store_write_durable_write_allowed,
        evolution_goal_queue_store_write_applied_to_disk,
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
        task_hierarchy_depth_total,
        task_hierarchy_route_fanout_total,
        task_hierarchy_threshold_delta_milli,
        task_hierarchy_selected_lanes,
        task_hierarchy_skipped_lanes,
        task_hierarchy_memory_lanes,
        task_hierarchy_skipped_memory_lanes,
        fht_dke_events,
        fht_dke_enabled,
        fht_dke_total_tokens,
        fht_dke_dense_tokens,
        fht_dke_routed_tokens,
        fht_dke_kv_exchange_blocks,
        fht_dke_token_split_invalid,
        fht_dke_attention_threshold_milli,
        fht_dke_route_pressure_milli,
        compute_budget_events,
        compute_budget_threshold_delta_milli,
        compute_budget_runtime_kv_budget_pressure_milli,
        compute_budget_low,
        compute_budget_normal,
        compute_budget_expanded,
        compute_budget_selected_candidates,
        compute_budget_low_value_skipped,
        compute_budget_kv_lookups_skipped,
        compute_budget_validation_cost_tokens,
        compute_budget_saved_tokens,
        compute_budget_self_evolving_memory_fusion_saved_tokens,
        compute_budget_avoided_tokens,
        compute_budget_fanout_before,
        compute_budget_fanout_after,
        compute_budget_fanout_reduction,
        compute_budget_estimated_budget_tokens,
        compute_budget_estimated_spent_tokens,
        compute_budget_estimated_saved_tokens,
        compute_budget_anchor_count,
        compute_budget_anchors_preserved,
        compute_budget_anchor_preservation_failures,
        compute_budget_fallback_triggered,
        compute_budget_write_allowed,
        compute_budget_applied,
        auto_replay_recursive_runtime_items,
        auto_replay_recursive_runtime_calls,
        auto_replay_avg_recursive_call_pressure_milli,
        auto_replay_max_recursive_call_pressure_milli,
        evolution_recursive_replay_items,
        evolution_recursive_runtime_calls,
        process_reward_events,
        process_reward_positive,
        process_reward_reinforce,
        process_reward_hold,
        process_reward_penalize,
        process_reward_total_milli,
        live_evolution_events,
        live_router_threshold_delta_milli,
        live_hierarchy_weight_delta_milli,
        live_online_reward_feedbacks,
        live_online_reward_reinforcements,
        live_online_reward_penalties,
        live_online_reward_strength_milli,
        live_memory_reinforcements,
        live_memory_penalties,
        live_memory_updates,
        live_stored_memories,
        live_stored_gist_memories,
        live_stored_runtime_kv_memories,
        live_stored_memory_updates,
        live_reflection_issues,
        live_critical_reflection_issues,
        live_revision_actions,
        evolution_live_inference_runs,
        evolution_live_router_threshold_mutations,
        evolution_live_hierarchy_weight_mutations,
        evolution_live_router_threshold_delta_milli,
        evolution_live_hierarchy_weight_delta_milli,
        evolution_live_online_reward_feedbacks,
        evolution_live_online_reward_reinforcements,
        evolution_live_online_reward_penalties,
        evolution_live_online_reward_strength_milli,
        evolution_live_online_reward_reinforcement_strength_milli,
        evolution_live_online_reward_penalty_strength_milli,
        evolution_live_memory_reinforcements,
        evolution_live_memory_penalties,
        evolution_live_memory_updates,
        evolution_live_stored_memories,
        evolution_live_stored_gist_memories,
        evolution_live_stored_runtime_kv_memories,
        evolution_live_stored_memory_updates,
        evolution_live_reflection_issues,
        evolution_live_critical_reflection_issues,
        evolution_live_revision_actions,
        auto_replay_live_memory_feedback_items,
        auto_replay_live_memory_feedback_updates,
        auto_replay_live_memory_feedback_reinforcements,
        auto_replay_live_memory_feedback_penalties,
        auto_replay_live_memory_feedback_detail_items,
        auto_replay_live_memory_feedback_applied,
        auto_replay_live_memory_feedback_removed,
        auto_replay_live_memory_feedback_missing,
        auto_replay_live_memory_feedback_strength_delta_milli,
        replay_live_memory_feedback_items,
        replay_live_memory_feedback_updates,
        replay_live_memory_feedback_reinforcements,
        replay_live_memory_feedback_penalties,
        replay_live_memory_feedback_detail_items,
        replay_live_memory_feedback_applied,
        replay_live_memory_feedback_removed,
        replay_live_memory_feedback_missing,
        replay_live_memory_feedback_strength_delta_milli,
        replay_live_evolution_items,
        replay_live_evolution_router_threshold_mutations,
        replay_live_evolution_hierarchy_weight_mutations,
        replay_live_evolution_router_threshold_delta_milli,
        replay_live_evolution_hierarchy_weight_delta_milli,
        replay_live_evolution_online_reward_feedbacks,
        replay_live_evolution_online_reward_reinforcements,
        replay_live_evolution_online_reward_penalties,
        replay_live_evolution_online_reward_strength_milli,
        replay_live_evolution_online_reward_reinforcement_strength_milli,
        replay_live_evolution_online_reward_penalty_strength_milli,
        replay_live_evolution_memory_updates,
        replay_live_evolution_stored_memory_updates,
        replay_live_evolution_reflection_issues,
        replay_live_evolution_critical_reflection_issues,
        replay_live_evolution_revision_actions,
        reasoning_genome_events,
        reasoning_genome_genes,
        reasoning_genome_active_genes,
        reasoning_genome_aged_genes,
        reasoning_genome_malignant_genes,
        reasoning_genome_relabel_candidates,
        reasoning_genome_regeneration_candidates,
        reasoning_genome_gene_scissors_proposals,
        reasoning_genome_repair_payloads,
        reasoning_genome_regeneration_payloads,
        reasoning_genome_lifecycle_records,
        reasoning_genome_lifecycle_tombstone_candidates,
        reasoning_genome_lifecycle_pending_validations,
        reasoning_genome_lifecycle_source_evidence,
        reasoning_genome_splice_segments,
        reasoning_genome_splice_exons,
        reasoning_genome_splice_introns,
        reasoning_genome_splice_variants,
        reasoning_genome_splice_quarantined,
        reasoning_genome_splice_repair_candidates,
        reasoning_genome_splice_findings,
        reasoning_genome_splice_proposals,
        reasoning_genome_write_allowed,
        reasoning_genome_mutation_applied,
        reasoning_genome_splice_write_allowed,
        reasoning_genome_splice_applied,
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
        noiron_orchestration_events,
        noiron_orchestration_stages,
        noiron_orchestration_failed_stages,
        noiron_orchestration_writes_gated,
        noiron_orchestration_fht_dke_total_tokens,
        orchestration_audit_events,
        orchestration_audit_checked_fields,
        orchestration_audit_failed_fields,
        orchestration_audit_failed_stages,
        orchestration_audit_integrity_failed_fields,
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
struct UnifiedWriterGateTraceGateSummary {
    events: usize,
    records: usize,
    memory_records: usize,
    genome_records: usize,
    experiment_ledger_records: usize,
    evolution_goal_queue_records: usize,
    ready_records: usize,
    held_records: usize,
    rejected_records: usize,
    preview_only_records: usize,
    reason_codes: usize,
    explicit_apply_required: usize,
    write_allowed: usize,
    durable_write_allowed: usize,
    applied: usize,
}

fn unified_writer_gate_trace_gate_summary(line: &str) -> Option<UnifiedWriterGateTraceGateSummary> {
    if !line.contains("\"schema\":\"rust-norion-unified-writer-gate-v1\"") {
        return None;
    }

    Some(UnifiedWriterGateTraceGateSummary {
        events: 1,
        records: extract_json_usize_field(line, "records").unwrap_or(0),
        memory_records: extract_json_usize_field(line, "memory_records").unwrap_or(0),
        genome_records: extract_json_usize_field(line, "genome_records").unwrap_or(0),
        experiment_ledger_records: extract_json_usize_field(line, "experiment_ledger_records")
            .unwrap_or(0),
        evolution_goal_queue_records: extract_json_usize_field(
            line,
            "evolution_goal_queue_records",
        )
        .unwrap_or(0),
        ready_records: extract_json_usize_field(line, "ready_records").unwrap_or(0),
        held_records: extract_json_usize_field(line, "held_records").unwrap_or(0),
        rejected_records: extract_json_usize_field(line, "rejected_records").unwrap_or(0),
        preview_only_records: extract_json_usize_field(line, "preview_only_records").unwrap_or(0),
        reason_codes: extract_json_usize_field(line, "reason_code_count").unwrap_or(0),
        explicit_apply_required: usize::from(
            extract_json_bool_field(line, "explicit_apply_required").unwrap_or(false),
        ),
        write_allowed: usize::from(extract_json_bool_field(line, "write_allowed").unwrap_or(false)),
        durable_write_allowed: usize::from(
            extract_json_bool_field(line, "durable_write_allowed").unwrap_or(false),
        ),
        applied: usize::from(extract_json_bool_field(line, "applied").unwrap_or(false)),
    })
}

#[derive(Debug, Clone, Copy, Default)]
struct SelfGoalQueueApplyTraceGateSummary {
    events: usize,
    records: usize,
    ready_records: usize,
    held_records: usize,
    rejected_records: usize,
    reason_codes: usize,
    explicit_apply_required: usize,
    write_allowed: usize,
    applied: usize,
}

fn self_goal_queue_apply_trace_gate_summary(
    line: &str,
) -> Option<SelfGoalQueueApplyTraceGateSummary> {
    if !line.contains("\"schema\":\"rust-norion-self-goal-queue-apply-plan-v1\"") {
        return None;
    }

    Some(SelfGoalQueueApplyTraceGateSummary {
        events: 1,
        records: extract_json_usize_field(line, "records").unwrap_or(0),
        ready_records: extract_json_usize_field(line, "ready_records").unwrap_or(0),
        held_records: extract_json_usize_field(line, "held_records").unwrap_or(0),
        rejected_records: extract_json_usize_field(line, "rejected_records").unwrap_or(0),
        reason_codes: extract_json_usize_field(line, "reason_code_count").unwrap_or(0),
        explicit_apply_required: usize::from(
            extract_json_bool_field(line, "explicit_apply_required").unwrap_or(false),
        ),
        write_allowed: usize::from(extract_json_bool_field(line, "write_allowed").unwrap_or(false)),
        applied: usize::from(extract_json_bool_field(line, "applied").unwrap_or(false)),
    })
}

#[derive(Debug, Clone, Copy, Default)]
struct SelfGoalQueueContinuationTraceGateSummary {
    events: usize,
    ready: usize,
    held: usize,
    current_queue: usize,
    completion_resulting_queue: usize,
    goals: usize,
    required_evidence: usize,
    reason_codes: usize,
    budget_attempts: usize,
    budget_steps: usize,
    budget_tokens: usize,
    budget_runtime_ms: usize,
    write_allowed: usize,
    applied: usize,
}

fn self_goal_queue_continuation_trace_gate_summary(
    line: &str,
) -> Option<SelfGoalQueueContinuationTraceGateSummary> {
    if !line.contains("\"schema\":\"rust-norion-self-goal-queue-continuation-plan-v1\"") {
        return None;
    }

    let ready = extract_json_bool_field(line, "ready").unwrap_or(false);
    let source = extract_json_string_field(line, "source").unwrap_or_default();

    Some(SelfGoalQueueContinuationTraceGateSummary {
        events: 1,
        ready: usize::from(ready),
        held: usize::from(!ready),
        current_queue: usize::from(source == "current_queue"),
        completion_resulting_queue: usize::from(source == "completion_resulting_queue"),
        goals: extract_json_usize_field(line, "goals").unwrap_or(0),
        required_evidence: extract_json_usize_field(line, "required_evidence_count").unwrap_or(0),
        reason_codes: extract_json_usize_field(line, "reason_code_count").unwrap_or(0),
        budget_attempts: extract_json_usize_field(line, "budget_attempts").unwrap_or(0),
        budget_steps: extract_json_usize_field(line, "budget_steps").unwrap_or(0),
        budget_tokens: extract_json_usize_field(line, "budget_tokens").unwrap_or(0),
        budget_runtime_ms: extract_json_usize_field(line, "budget_runtime_ms").unwrap_or(0),
        write_allowed: usize::from(extract_json_bool_field(line, "write_allowed").unwrap_or(false)),
        applied: usize::from(extract_json_bool_field(line, "applied").unwrap_or(false)),
    })
}

#[derive(Debug, Clone, Copy, Default)]
struct SelfGoalQueueEvidencePlanTraceGateSummary {
    events: usize,
    ready: usize,
    held: usize,
    steps: usize,
    auto_collectible: usize,
    manual: usize,
    required_evidence: usize,
    packet_templates: usize,
    command_templates: usize,
    write_allowed: usize,
    applied: usize,
}

fn self_goal_queue_evidence_plan_trace_gate_summary(
    line: &str,
) -> Option<SelfGoalQueueEvidencePlanTraceGateSummary> {
    if !line.contains("\"schema\":\"rust-norion-self-goal-queue-evidence-plan-v1\"") {
        return None;
    }

    let ready = extract_json_bool_field(line, "ready").unwrap_or(false);
    let packet_templates =
        extract_json_string_array_field(line, "packet_template_digests").unwrap_or_default();
    let command_templates =
        extract_json_string_array_field(line, "command_digests").unwrap_or_default();

    Some(SelfGoalQueueEvidencePlanTraceGateSummary {
        events: 1,
        ready: usize::from(ready),
        held: usize::from(!ready),
        steps: extract_json_usize_field(line, "planned_step_count").unwrap_or(0),
        auto_collectible: extract_json_usize_field(line, "auto_collectible_steps").unwrap_or(0),
        manual: extract_json_usize_field(line, "manual_steps").unwrap_or(0),
        required_evidence: extract_json_usize_field(line, "required_evidence_count").unwrap_or(0),
        packet_templates: packet_templates.len(),
        command_templates: command_templates.len(),
        write_allowed: usize::from(extract_json_bool_field(line, "write_allowed").unwrap_or(false)),
        applied: usize::from(extract_json_bool_field(line, "applied").unwrap_or(false)),
    })
}

#[derive(Debug, Clone, Copy, Default)]
struct SelfGoalQueueEvidenceCollectionTraceGateSummary {
    events: usize,
    ready: usize,
    complete: usize,
    steps: usize,
    collected: usize,
    passed: usize,
    failed: usize,
    missing: usize,
    manual_missing: usize,
    write_allowed: usize,
    applied: usize,
}

fn self_goal_queue_evidence_collection_trace_gate_summary(
    line: &str,
) -> Option<SelfGoalQueueEvidenceCollectionTraceGateSummary> {
    if !line.contains("\"schema\":\"rust-norion-self-goal-queue-evidence-collection-v1\"") {
        return None;
    }

    Some(SelfGoalQueueEvidenceCollectionTraceGateSummary {
        events: 1,
        ready: usize::from(extract_json_bool_field(line, "ready").unwrap_or(false)),
        complete: usize::from(
            extract_json_bool_field(line, "collection_complete").unwrap_or(false),
        ),
        steps: extract_json_usize_field(line, "planned_step_count").unwrap_or(0),
        collected: extract_json_usize_field(line, "collected_evidence_count").unwrap_or(0),
        passed: extract_json_usize_field(line, "passed_steps").unwrap_or(0),
        failed: extract_json_usize_field(line, "failed_steps").unwrap_or(0),
        missing: extract_json_usize_field(line, "missing_steps").unwrap_or(0),
        manual_missing: extract_json_usize_field(line, "manual_missing_steps").unwrap_or(0),
        write_allowed: usize::from(extract_json_bool_field(line, "write_allowed").unwrap_or(false)),
        applied: usize::from(extract_json_bool_field(line, "applied").unwrap_or(false)),
    })
}

#[derive(Debug, Clone, Copy, Default)]
struct SelfGoalLocalEvidenceTraceGateSummary {
    events: usize,
    enabled: usize,
    dry_run: usize,
    ready: usize,
    steps: usize,
    attempted: usize,
    generated: usize,
    passed: usize,
    failed: usize,
    skipped: usize,
    manual: usize,
    planned_status: usize,
    write_allowed: usize,
    applied: usize,
}

fn self_goal_local_evidence_trace_gate_summary(
    line: &str,
) -> Option<SelfGoalLocalEvidenceTraceGateSummary> {
    if !line.contains("\"schema\":\"rust-norion-self-goal-local-evidence-v1\"") {
        return None;
    }

    Some(SelfGoalLocalEvidenceTraceGateSummary {
        events: 1,
        enabled: usize::from(extract_json_bool_field(line, "enabled").unwrap_or(false)),
        dry_run: usize::from(extract_json_bool_field(line, "dry_run").unwrap_or(false)),
        ready: usize::from(extract_json_bool_field(line, "ready").unwrap_or(false)),
        steps: extract_json_usize_field(line, "planned_step_count").unwrap_or(0),
        attempted: extract_json_usize_field(line, "attempted_step_count").unwrap_or(0),
        generated: extract_json_usize_field(line, "generated_packet_count").unwrap_or(0),
        passed: extract_json_usize_field(line, "passed_step_count").unwrap_or(0),
        failed: extract_json_usize_field(line, "failed_step_count").unwrap_or(0),
        skipped: extract_json_usize_field(line, "skipped_step_count").unwrap_or(0),
        manual: extract_json_usize_field(line, "manual_step_count").unwrap_or(0),
        planned_status: extract_json_usize_field(line, "planned_status_count").unwrap_or(0),
        write_allowed: usize::from(extract_json_bool_field(line, "write_allowed").unwrap_or(false)),
        applied: usize::from(extract_json_bool_field(line, "applied").unwrap_or(false)),
    })
}

#[derive(Debug, Clone, Copy, Default)]
struct CodingServiceEvalTraceGateSummary {
    events: usize,
    readiness_events: usize,
    runner_events: usize,
    passed: usize,
    requests: usize,
    completed: usize,
    evidence_packets: usize,
    rust_validation_checked: usize,
    compile_checked: usize,
    unit_test_checked: usize,
    write_allowed: usize,
    applied: usize,
}

fn coding_service_eval_trace_gate_summary(line: &str) -> Option<CodingServiceEvalTraceGateSummary> {
    if !line.contains("\"schema\":\"rust-norion-coding-service-eval-readiness-v1\"") {
        return None;
    }

    let kind = extract_json_string_field(line, "report_kind").unwrap_or_default();

    Some(CodingServiceEvalTraceGateSummary {
        events: 1,
        readiness_events: usize::from(kind == "readiness"),
        runner_events: usize::from(kind == "runner"),
        passed: usize::from(extract_json_bool_field(line, "passed").unwrap_or(false)),
        requests: extract_json_usize_field(line, "request_plan_count").unwrap_or(0),
        completed: extract_json_usize_field(line, "completed_count").unwrap_or(0),
        evidence_packets: extract_json_usize_field(line, "evidence_packet_count").unwrap_or(0),
        rust_validation_checked: extract_json_usize_field(line, "rust_validation_checked_count")
            .unwrap_or(0),
        compile_checked: extract_json_usize_field(line, "compile_checked_count").unwrap_or(0),
        unit_test_checked: extract_json_usize_field(line, "unit_test_checked_count").unwrap_or(0),
        write_allowed: usize::from(extract_json_bool_field(line, "write_allowed").unwrap_or(false)),
        applied: usize::from(extract_json_bool_field(line, "applied").unwrap_or(false)),
    })
}

#[derive(Debug, Clone, Copy, Default)]
struct EvolutionGoalQueueStoreWriteTraceGateSummary {
    events: usize,
    applied: usize,
    held: usize,
    rejected: usize,
    reason_codes: usize,
    durable_write_allowed: usize,
    applied_to_disk: usize,
}

fn evolution_goal_queue_store_write_trace_gate_summary(
    line: &str,
) -> Option<EvolutionGoalQueueStoreWriteTraceGateSummary> {
    if !line.contains("\"schema\":\"rust-norion-evolution-goal-queue-store-write-v1\"") {
        return None;
    }

    let decision = extract_json_string_field(line, "decision").unwrap_or_default();
    let applied = extract_json_bool_field(line, "applied").unwrap_or(false);

    Some(EvolutionGoalQueueStoreWriteTraceGateSummary {
        events: 1,
        applied: usize::from(decision == "applied"),
        held: usize::from(decision == "hold"),
        rejected: usize::from(decision == "rejected"),
        reason_codes: extract_json_usize_field(line, "reason_code_count").unwrap_or(0),
        durable_write_allowed: usize::from(
            extract_json_bool_field(line, "durable_write_allowed").unwrap_or(false),
        ),
        applied_to_disk: usize::from(
            decision == "applied"
                && applied
                && extract_json_bool_field(line, "durable_write_allowed").unwrap_or(false),
        ),
    })
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
    missing_handling_signals: usize,
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
    summary.missing_handling_signals = usize::from(
        !extract_json_bool_field(business_contract, "handling_signal").unwrap_or(false),
    );
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
struct SelfEvolutionPromotionPreflightTraceGateSummary {
    events: usize,
    ready: usize,
    held: usize,
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

fn self_evolution_promotion_preflight_trace_gate_summary(
    line: &str,
) -> Option<SelfEvolutionPromotionPreflightTraceGateSummary> {
    if !line.contains("\"schema\":\"rust-norion-self-evolution-promotion-preflight-v1\"") {
        return None;
    }

    let ready = extract_json_bool_field(line, "ready_for_explicit_promotion").unwrap_or(false);
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

    Some(SelfEvolutionPromotionPreflightTraceGateSummary {
        events: 1,
        ready: usize::from(ready),
        held: usize::from(!ready),
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
    consolidation_events: usize,
    consolidation_actions: usize,
    merge_previews: usize,
    decay_previews: usize,
    tombstone_previews: usize,
    merge_rejections: usize,
    contexts: usize,
    saved_tokens: usize,
    maintenance_actions: usize,
    admission_candidates: usize,
    write_allowed: usize,
    durable_write_allowed: usize,
    applied: usize,
    applied_to_disk: usize,
    source_quarantine_events: usize,
    source_quarantine_actions: usize,
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
        consolidation_events: usize::from(operation == "consolidation_preview"),
        source_quarantine_events: usize::from(operation == "source_quarantine"),
        consolidation_actions: extract_json_usize_field(line, "consolidation_actions").unwrap_or(0),
        source_quarantine_actions: usize::from(operation == "source_quarantine")
            * extract_json_usize_field(line, "action_count").unwrap_or(0),
        merge_previews: extract_json_usize_field(line, "merge_previews").unwrap_or(0),
        decay_previews: extract_json_usize_field(line, "decay_previews").unwrap_or(0),
        tombstone_previews: extract_json_usize_field(line, "tombstone_previews").unwrap_or(0),
        merge_rejections: extract_json_usize_field(line, "merge_rejections").unwrap_or(0),
        contexts: extract_json_usize_field(line, "contexts").unwrap_or(0),
        saved_tokens: extract_json_usize_field(line, "saved_tokens").unwrap_or(0),
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
struct SelfEvolvingMemoryWritebackTraceGateSummary {
    events: usize,
    source_case_digests: usize,
    attempted_records: usize,
    accepted_records: usize,
    records_before: usize,
    records_after: usize,
    tool_reliability_after: usize,
    tool_observations_after: usize,
    maintenance_actions: usize,
    merged_duplicate_episodes: usize,
    write_allowed: usize,
    durable_write_allowed: usize,
    applied: usize,
    applied_to_disk: usize,
    snapshot_changes: usize,
}

fn self_evolving_memory_writeback_trace_gate_summary(
    line: &str,
) -> Option<SelfEvolvingMemoryWritebackTraceGateSummary> {
    if !line.contains("\"schema\":\"rust-norion-self-evolving-memory-writeback-v1\"") {
        return None;
    }

    let snapshot_before_digest =
        extract_json_string_field(line, "snapshot_before_digest").unwrap_or_default();
    let snapshot_digest = extract_json_string_field(line, "snapshot_digest").unwrap_or_default();
    let source_case_digest =
        extract_json_string_field(line, "source_case_digest").unwrap_or_default();

    Some(SelfEvolvingMemoryWritebackTraceGateSummary {
        events: 1,
        source_case_digests: usize::from(is_stable_fnv64_digest(&source_case_digest)),
        attempted_records: extract_json_usize_field(line, "attempted_records").unwrap_or(0),
        accepted_records: extract_json_usize_field(line, "accepted_records").unwrap_or(0),
        records_before: extract_json_usize_field(line, "records_before").unwrap_or(0),
        records_after: extract_json_usize_field(line, "records_after").unwrap_or(0),
        tool_reliability_after: extract_json_usize_field(line, "tool_reliability_after")
            .unwrap_or(0),
        tool_observations_after: extract_json_usize_field(line, "tool_observations_after")
            .unwrap_or(0),
        maintenance_actions: extract_json_usize_field(line, "maintenance_actions").unwrap_or(0),
        merged_duplicate_episodes: extract_json_usize_field(line, "merged_duplicate_episodes")
            .unwrap_or(0),
        write_allowed: usize::from(extract_json_bool_field(line, "write_allowed").unwrap_or(false)),
        durable_write_allowed: usize::from(
            extract_json_bool_field(line, "durable_write_allowed").unwrap_or(false),
        ),
        applied: usize::from(extract_json_bool_field(line, "applied").unwrap_or(false)),
        applied_to_disk: usize::from(
            extract_json_bool_field(line, "applied_to_disk").unwrap_or(false),
        ),
        snapshot_changes: usize::from(
            !snapshot_before_digest.is_empty() && snapshot_before_digest != snapshot_digest,
        ),
    })
}

fn is_stable_fnv64_digest(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("fnv64:") else {
        return false;
    };
    hex.len() == 16 && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[derive(Debug, Clone, Copy, Default)]
struct MemoryTraceGateSummary {
    runtime_kv_exported: usize,
    runtime_kv_stored: usize,
    runtime_kv_hold: usize,
    runtime_kv_held: usize,
    feedback_reinforced: usize,
    feedback_penalized: usize,
    feedback_reinforcement_milli: usize,
    feedback_penalty_milli: usize,
    feedback_updates: usize,
    feedback_applied: usize,
    feedback_removed: usize,
    feedback_missing: usize,
    feedback_strength_delta_milli: usize,
}

fn memory_trace_gate_summary(line: &str) -> Option<MemoryTraceGateSummary> {
    let memory = json_object_after_field(line, "memory")?;

    Some(MemoryTraceGateSummary {
        runtime_kv_exported: extract_json_usize_field(memory, "runtime_kv_exported").unwrap_or(0),
        runtime_kv_stored: extract_json_usize_field(memory, "runtime_kv_stored").unwrap_or(0),
        runtime_kv_hold: usize::from(
            extract_json_bool_field(memory, "runtime_kv_hold").unwrap_or(false),
        ),
        runtime_kv_held: extract_json_usize_field(memory, "runtime_kv_held").unwrap_or(0),
        feedback_reinforced: extract_json_usize_field(memory, "feedback_reinforced").unwrap_or(0),
        feedback_penalized: extract_json_usize_field(memory, "feedback_penalized").unwrap_or(0),
        feedback_reinforcement_milli: trace_gate_milli(
            extract_json_f32_field(memory, "feedback_reinforcement_amount").unwrap_or(0.0),
        ),
        feedback_penalty_milli: trace_gate_milli(
            extract_json_f32_field(memory, "feedback_penalty_amount").unwrap_or(0.0),
        ),
        feedback_updates: extract_json_usize_field(memory, "feedback_updates").unwrap_or(0),
        feedback_applied: extract_json_usize_field(memory, "feedback_applied").unwrap_or(0),
        feedback_removed: extract_json_usize_field(memory, "feedback_removed").unwrap_or(0),
        feedback_missing: extract_json_usize_field(memory, "feedback_missing").unwrap_or(0),
        feedback_strength_delta_milli: trace_gate_milli(
            extract_json_f32_field(memory, "feedback_strength_delta").unwrap_or(0.0),
        ),
    })
}

#[derive(Debug, Clone, Copy, Default)]
struct MemoryResidencyTraceGateSummary {
    events: usize,
    decisions: usize,
    hot: usize,
    warm: usize,
    cold: usize,
    quarantined: usize,
    retired: usize,
    protected_rollback_anchors: usize,
    blocked_reasons: usize,
    token_estimate: usize,
    write_allowed: usize,
    durable_write_allowed: usize,
    applied: usize,
}

fn memory_residency_trace_gate_summary(line: &str) -> Option<MemoryResidencyTraceGateSummary> {
    if !line.contains("\"schema\":\"rust-norion-memory-residency-plan-v1\"") {
        return None;
    }

    Some(MemoryResidencyTraceGateSummary {
        events: 1,
        decisions: extract_json_usize_field(line, "decision_count").unwrap_or(0),
        hot: extract_json_usize_field(line, "hot").unwrap_or(0),
        warm: extract_json_usize_field(line, "warm").unwrap_or(0),
        cold: extract_json_usize_field(line, "cold").unwrap_or(0),
        quarantined: extract_json_usize_field(line, "quarantined").unwrap_or(0),
        retired: extract_json_usize_field(line, "retired").unwrap_or(0),
        protected_rollback_anchors: extract_json_usize_field(line, "protected_rollback_anchors")
            .unwrap_or(0),
        blocked_reasons: extract_json_usize_field(line, "blocked_reasons").unwrap_or(0),
        token_estimate: extract_json_usize_field(line, "token_estimate").unwrap_or(0),
        write_allowed: usize::from(extract_json_bool_field(line, "write_allowed").unwrap_or(false)),
        durable_write_allowed: usize::from(
            extract_json_bool_field(line, "durable_write_allowed").unwrap_or(false),
        ),
        applied: usize::from(extract_json_bool_field(line, "applied").unwrap_or(false)),
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
    depth_total: usize,
    route_fanout_total: usize,
    threshold_delta_milli: usize,
    selected_lanes: usize,
    skipped_lanes: usize,
    memory_lanes: usize,
    skipped_memory_lanes: usize,
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
        depth_total: extract_json_usize_field(task, "hierarchy_depth").unwrap_or(0),
        route_fanout_total: extract_json_usize_field(task, "route_fanout").unwrap_or(0),
        threshold_delta_milli: trace_gate_milli(
            extract_json_f32_field(task, "threshold_delta").unwrap_or(0.0),
        ),
        selected_lanes: extract_json_string_array_field(task, "selected_lanes")
            .map(|lanes| lanes.len())
            .unwrap_or(0),
        skipped_lanes: extract_json_string_array_field(task, "skipped_lanes")
            .map(|lanes| lanes.len())
            .unwrap_or(0),
        memory_lanes: extract_json_string_array_field(task, "memory_lanes")
            .map(|lanes| lanes.len())
            .unwrap_or(0),
        skipped_memory_lanes: extract_json_string_array_field(task, "skipped_memory_lanes")
            .map(|lanes| lanes.len())
            .unwrap_or(0),
    })
}

#[derive(Debug, Clone, Copy, Default)]
struct FhtDkeTraceGateSummary {
    events: usize,
    enabled: usize,
    total_tokens: usize,
    dense_tokens: usize,
    routed_tokens: usize,
    kv_exchange_blocks: usize,
    token_split_invalid: usize,
    attention_threshold_milli: usize,
    route_pressure_milli: usize,
}

fn fht_dke_trace_gate_summary(line: &str) -> Option<FhtDkeTraceGateSummary> {
    let fht = json_object_after_field(line, "fht_dke")?;

    Some(FhtDkeTraceGateSummary {
        events: 1,
        enabled: usize::from(extract_json_bool_field(fht, "enabled").unwrap_or(false)),
        total_tokens: extract_json_usize_field(fht, "total_tokens").unwrap_or(0),
        dense_tokens: extract_json_usize_field(fht, "dense_tokens").unwrap_or(0),
        routed_tokens: extract_json_usize_field(fht, "routed_tokens").unwrap_or(0),
        kv_exchange_blocks: extract_json_usize_field(fht, "kv_exchange_blocks").unwrap_or(0),
        token_split_invalid: usize::from(
            !extract_json_bool_field(fht, "token_split_valid").unwrap_or(false),
        ),
        attention_threshold_milli: trace_gate_milli(
            extract_json_f32_field(fht, "attention_threshold").unwrap_or(0.0),
        ),
        route_pressure_milli: trace_gate_milli(
            extract_json_f32_field(fht, "route_pressure").unwrap_or(0.0),
        ),
    })
}

#[derive(Debug, Clone, Copy, Default)]
struct ComputeBudgetTraceGateSummary {
    events: usize,
    threshold_delta_milli: usize,
    runtime_kv_budget_pressure_milli: usize,
    low: usize,
    normal: usize,
    expanded: usize,
    selected_candidates: usize,
    low_value_skipped: usize,
    kv_lookups_skipped: usize,
    validation_cost_tokens: usize,
    saved_tokens: usize,
    self_evolving_memory_fusion_saved_tokens: usize,
    avoided_tokens: usize,
    fanout_before: usize,
    fanout_after: usize,
    estimated_budget_tokens: usize,
    estimated_spent_tokens: usize,
    anchor_count: usize,
    anchors_preserved: usize,
    fallback_triggered: usize,
    write_allowed: usize,
    applied: usize,
}

fn compute_budget_trace_gate_summary(line: &str) -> Option<ComputeBudgetTraceGateSummary> {
    let budget = json_object_after_field(line, "compute_budget")?;
    let budget_kind = extract_json_string_field(budget, "budget").unwrap_or_default();

    Some(ComputeBudgetTraceGateSummary {
        events: 1,
        threshold_delta_milli: trace_gate_milli(
            extract_json_f32_field(budget, "threshold_delta").unwrap_or(0.0),
        ),
        runtime_kv_budget_pressure_milli: trace_gate_milli(
            extract_json_f32_field(budget, "runtime_kv_budget_pressure").unwrap_or(0.0),
        ),
        low: usize::from(budget_kind == "low"),
        normal: usize::from(budget_kind == "normal"),
        expanded: usize::from(budget_kind == "expanded"),
        selected_candidates: extract_json_usize_field(budget, "selected_candidates").unwrap_or(0),
        low_value_skipped: extract_json_usize_field(budget, "low_value_skipped").unwrap_or(0),
        kv_lookups_skipped: extract_json_usize_field(budget, "kv_lookups_skipped").unwrap_or(0),
        validation_cost_tokens: extract_json_usize_field(budget, "validation_cost_tokens")
            .unwrap_or(0),
        saved_tokens: extract_json_usize_field(budget, "saved_tokens").unwrap_or(0),
        self_evolving_memory_fusion_saved_tokens: extract_json_usize_field(
            budget,
            "self_evolving_memory_fusion_saved_tokens",
        )
        .unwrap_or(0),
        avoided_tokens: extract_json_usize_field(budget, "wasted_compute_avoided_tokens")
            .unwrap_or(0),
        fanout_before: extract_json_usize_field(budget, "route_fanout_before").unwrap_or(0),
        fanout_after: extract_json_usize_field(budget, "route_fanout_after").unwrap_or(0),
        estimated_budget_tokens: extract_json_usize_field(budget, "estimated_budget_tokens")
            .unwrap_or(0),
        estimated_spent_tokens: extract_json_usize_field(budget, "estimated_spent_tokens")
            .unwrap_or(0),
        anchor_count: extract_json_usize_field(budget, "anchor_count").unwrap_or(0),
        anchors_preserved: extract_json_usize_field(budget, "anchors_preserved_count").unwrap_or(0),
        fallback_triggered: usize::from(
            extract_json_bool_field(budget, "fallback_triggered").unwrap_or(false),
        ),
        write_allowed: usize::from(
            extract_json_bool_field(budget, "write_allowed").unwrap_or(false),
        ),
        applied: usize::from(extract_json_bool_field(budget, "applied").unwrap_or(false)),
    })
}

#[derive(Debug, Clone, Copy, Default)]
struct ProcessRewardTraceGateSummary {
    events: usize,
    positive: usize,
    reinforce: usize,
    hold: usize,
    penalize: usize,
    total_milli: usize,
}

fn process_reward_trace_gate_summary(line: &str) -> Option<ProcessRewardTraceGateSummary> {
    let reward = json_object_after_field(line, "process_reward")?;
    let total = extract_json_f32_field(reward, "total").unwrap_or(0.0);
    let action = extract_json_string_field(reward, "action").unwrap_or_default();

    Some(ProcessRewardTraceGateSummary {
        events: 1,
        positive: usize::from(total > 0.0),
        reinforce: usize::from(action == "reinforce"),
        hold: usize::from(action == "hold"),
        penalize: usize::from(action == "penalize"),
        total_milli: trace_gate_milli(total),
    })
}

#[derive(Debug, Clone, Copy, Default)]
struct LiveEvolutionTraceGateSummary {
    events: usize,
    router_threshold_delta_milli: usize,
    hierarchy_weight_delta_milli: usize,
    online_reward_feedbacks: usize,
    online_reward_reinforcements: usize,
    online_reward_penalties: usize,
    online_reward_strength_milli: usize,
    memory_reinforcements: usize,
    memory_penalties: usize,
    memory_updates: usize,
    stored_memories: usize,
    stored_gist_memories: usize,
    stored_runtime_kv_memories: usize,
    stored_memory_updates: usize,
    reflection_issues: usize,
    critical_reflection_issues: usize,
    revision_actions: usize,
}

fn live_evolution_trace_gate_summary(line: &str) -> Option<LiveEvolutionTraceGateSummary> {
    let live = json_object_after_field(line, "live_evolution")?;

    Some(LiveEvolutionTraceGateSummary {
        events: usize::from(
            extract_json_bool_field(live, "live_inference_recorded").unwrap_or(false),
        ),
        router_threshold_delta_milli: trace_gate_milli(
            extract_json_f32_field(live, "live_router_threshold_delta").unwrap_or(0.0),
        ),
        hierarchy_weight_delta_milli: trace_gate_milli(
            extract_json_f32_field(live, "live_hierarchy_weight_delta").unwrap_or(0.0),
        ),
        online_reward_feedbacks: extract_json_usize_field(live, "live_online_reward_feedbacks")
            .unwrap_or(0),
        online_reward_reinforcements: extract_json_usize_field(
            live,
            "live_online_reward_reinforcements",
        )
        .unwrap_or(0),
        online_reward_penalties: extract_json_usize_field(live, "live_online_reward_penalties")
            .unwrap_or(0),
        online_reward_strength_milli: trace_gate_milli(
            extract_json_f32_field(live, "live_online_reward_strength").unwrap_or(0.0),
        ),
        memory_reinforcements: extract_json_usize_field(live, "live_memory_reinforcements")
            .unwrap_or(0),
        memory_penalties: extract_json_usize_field(live, "live_memory_penalties").unwrap_or(0),
        memory_updates: extract_json_usize_field(live, "live_memory_updates").unwrap_or(0),
        stored_memories: usize::from(
            extract_json_bool_field(live, "live_stored_memory").unwrap_or(false),
        ),
        stored_gist_memories: extract_json_usize_field(live, "live_stored_gist_memories")
            .unwrap_or(0),
        stored_runtime_kv_memories: extract_json_usize_field(
            live,
            "live_stored_runtime_kv_memories",
        )
        .unwrap_or(0),
        stored_memory_updates: extract_json_usize_field(live, "live_stored_memory_updates")
            .unwrap_or(0),
        reflection_issues: extract_json_usize_field(live, "live_reflection_issues").unwrap_or(0),
        critical_reflection_issues: extract_json_usize_field(
            live,
            "live_critical_reflection_issues",
        )
        .unwrap_or(0),
        revision_actions: extract_json_usize_field(live, "live_revision_actions").unwrap_or(0),
    })
}

#[derive(Debug, Clone, Copy, Default)]
struct ReasoningGenomeTraceGateSummary {
    events: usize,
    genes: usize,
    active_genes: usize,
    aged_genes: usize,
    malignant_genes: usize,
    relabel_candidates: usize,
    regeneration_candidates: usize,
    gene_scissors_proposals: usize,
    repair_payloads: usize,
    regeneration_payloads: usize,
    lifecycle_records: usize,
    lifecycle_tombstone_candidates: usize,
    lifecycle_pending_validations: usize,
    lifecycle_source_evidence: usize,
    splice_segments: usize,
    splice_exons: usize,
    splice_introns: usize,
    splice_variants: usize,
    splice_quarantined: usize,
    splice_repair_candidates: usize,
    splice_findings: usize,
    splice_proposals: usize,
    write_allowed: usize,
    mutation_applied: usize,
    splice_write_allowed: usize,
    splice_applied: usize,
}

fn reasoning_genome_trace_gate_summary(line: &str) -> Option<ReasoningGenomeTraceGateSummary> {
    let genome = json_object_after_field(line, "reasoning_genome")?;

    Some(ReasoningGenomeTraceGateSummary {
        events: 1,
        genes: extract_json_usize_field(genome, "gene_count").unwrap_or(0),
        active_genes: extract_json_usize_field(genome, "active_genes").unwrap_or(0),
        aged_genes: extract_json_usize_field(genome, "aged_genes").unwrap_or(0),
        malignant_genes: extract_json_usize_field(genome, "malignant_genes").unwrap_or(0),
        relabel_candidates: extract_json_usize_field(genome, "relabel_candidates").unwrap_or(0),
        regeneration_candidates: extract_json_usize_field(genome, "regeneration_candidates")
            .unwrap_or(0),
        gene_scissors_proposals: extract_json_usize_field(genome, "gene_scissors_proposals")
            .unwrap_or(0),
        repair_payloads: extract_json_usize_field(genome, "repair_payloads").unwrap_or(0),
        regeneration_payloads: extract_json_usize_field(genome, "regeneration_payloads")
            .unwrap_or(0),
        lifecycle_records: extract_json_usize_field(genome, "lifecycle_records").unwrap_or(0),
        lifecycle_tombstone_candidates: extract_json_usize_field(
            genome,
            "lifecycle_tombstone_candidates",
        )
        .unwrap_or(0),
        lifecycle_pending_validations: extract_json_usize_field(
            genome,
            "lifecycle_pending_validations",
        )
        .unwrap_or(0),
        lifecycle_source_evidence: extract_json_usize_field(genome, "lifecycle_source_evidence")
            .unwrap_or(0),
        splice_segments: extract_json_usize_field(genome, "splice_segments").unwrap_or(0),
        splice_exons: extract_json_usize_field(genome, "splice_exons").unwrap_or(0),
        splice_introns: extract_json_usize_field(genome, "splice_introns").unwrap_or(0),
        splice_variants: extract_json_usize_field(genome, "splice_variants").unwrap_or(0),
        splice_quarantined: extract_json_usize_field(genome, "splice_quarantined").unwrap_or(0),
        splice_repair_candidates: extract_json_usize_field(genome, "splice_repair_candidates")
            .unwrap_or(0),
        splice_findings: extract_json_usize_field(genome, "splice_findings").unwrap_or(0),
        splice_proposals: extract_json_usize_field(genome, "splice_proposals").unwrap_or(0),
        write_allowed: usize::from(
            extract_json_bool_field(genome, "write_allowed").unwrap_or(false),
        ),
        mutation_applied: usize::from(
            extract_json_bool_field(genome, "mutation_applied").unwrap_or(false),
        ),
        splice_write_allowed: usize::from(
            extract_json_bool_field(genome, "splice_write_allowed").unwrap_or(false),
        ),
        splice_applied: usize::from(
            extract_json_bool_field(genome, "splice_applied").unwrap_or(false),
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

#[derive(Debug, Clone, Copy, Default)]
struct NoironOrchestrationTraceGateSummary {
    events: usize,
    stages: usize,
    failed_stages: usize,
    writes_gated: usize,
    fht_dke_total_tokens: usize,
}

fn noiron_orchestration_trace_gate_summary(
    line: &str,
) -> Option<NoironOrchestrationTraceGateSummary> {
    let orchestration = json_object_after_field(line, "noiron_orchestration")?;

    Some(NoironOrchestrationTraceGateSummary {
        events: 1,
        stages: extract_json_usize_field(orchestration, "stages").unwrap_or(0),
        failed_stages: extract_json_usize_field(orchestration, "failed_stages").unwrap_or(0),
        writes_gated: usize::from(
            extract_json_bool_field(orchestration, "writes_gated").unwrap_or(false),
        ),
        fht_dke_total_tokens: extract_json_usize_field(orchestration, "fht_dke_total_tokens")
            .unwrap_or(0),
    })
}

#[derive(Debug, Clone, Copy, Default)]
struct OrchestrationAuditTraceGateSummary {
    events: usize,
    checked_fields: usize,
    failed_fields: usize,
    failed_stages: usize,
    integrity_failed_fields: usize,
}

fn orchestration_audit_trace_gate_summary(
    line: &str,
) -> Option<OrchestrationAuditTraceGateSummary> {
    let audit = json_object_after_field(line, "orchestration_audit")?;

    Some(OrchestrationAuditTraceGateSummary {
        events: 1,
        checked_fields: extract_json_usize_field(audit, "checked_fields").unwrap_or(0),
        failed_fields: extract_json_usize_field(audit, "failed_field_count").unwrap_or(0),
        failed_stages: extract_json_usize_field(audit, "failed_stage_count").unwrap_or(0),
        integrity_failed_fields: extract_json_usize_field(audit, "integrity_failed_field_count")
            .unwrap_or(0),
    })
}

fn trace_gate_milli(value: f32) -> usize {
    if value.is_finite() {
        (value.clamp(0.0, 1.0) * 1000.0).round() as usize
    } else {
        0
    }
}
