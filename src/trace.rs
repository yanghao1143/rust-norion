#[cfg(test)]
use std::fs;

const TRACE_FLOAT_EPSILON: f32 = 0.000_001;
mod adapter;
mod admission;
mod agent_team;
mod chaperone;
mod coding_service_eval;
mod device_contract;
mod embedding;
mod evolution;
mod fields;
mod genome;
mod improvement_corpus;
mod jsonl;
mod memory;
mod required_fields;
mod routing;
mod runtime_device;
mod runtime_kv;
mod schema_jsonl_gate;
mod self_goal;
mod specialized;
mod tool_build_report;
mod writer_gate;

use adapter::evaluate_trace_adapter_observations;
use admission::{
    evaluate_self_evolution_admission_schema_line, evaluate_self_evolution_experiment_schema_line,
    evaluate_self_evolution_operator_approval_schema_line,
    evaluate_self_evolution_promotion_preflight_schema_line,
    evaluate_self_evolution_rollback_replay_apply_schema_line,
    evaluate_self_evolution_rollback_replay_gate_schema_line,
    evaluate_self_evolution_rollback_replay_schema_line,
};
use agent_team::evaluate_trace_agent_team;
use chaperone::evaluate_reasoning_chaperone_fold_guard_schema_line;
use coding_service_eval::evaluate_coding_service_eval_schema_line;
use device_contract::evaluate_trace_device_contract;
use embedding::evaluate_trace_embedding;
use evolution::{evaluate_trace_auto_replay, evaluate_trace_live_evolution};
use fields::{extract_json_bool_field, extract_json_usize_field, json_object_after_field};
use genome::{
    evaluate_dna_evolution_apply_plan_schema_line, evaluate_dna_evolution_controller_schema_line,
    evaluate_trace_reasoning_genome,
};
use improvement_corpus::evaluate_improvement_corpus_schema_line;
use required_fields::trace_required_fields;
use runtime_device::evaluate_trace_runtime_device_execution;
use runtime_kv::evaluate_trace_runtime_kv;
use self_goal::{
    evaluate_evolution_goal_queue_store_write_schema_line,
    evaluate_self_goal_local_evidence_schema_line,
    evaluate_self_goal_queue_append_execution_schema_line,
    evaluate_self_goal_queue_apply_schema_line, evaluate_self_goal_queue_continuation_schema_line,
    evaluate_self_goal_queue_evidence_collection_schema_line,
    evaluate_self_goal_queue_evidence_plan_schema_line,
};
use tool_build_report::evaluate_agent_tool_build_report_schema_line;
use writer_gate::evaluate_unified_writer_gate_schema_line;

#[cfg(test)]
use fields::*;
use memory::{
    evaluate_self_evolving_memory_store_schema_line, evaluate_trace_drift,
    evaluate_trace_kv_fusion, evaluate_trace_memory_admission, evaluate_trace_memory_feedback,
    evaluate_trace_memory_governance, evaluate_trace_memory_residency_schema_line,
};
use routing::{
    evaluate_trace_adaptive_routing, evaluate_trace_compute_budget, evaluate_trace_task_hierarchy,
};
use specialized::{
    evaluate_business_contract_trace_schema_line, evaluate_rust_check_trace_schema_line,
};

pub use jsonl::{
    agent_tool_build_report_trace_json_line, append_agent_tool_build_report_trace_jsonl,
    append_business_contract_trace_jsonl, append_coding_service_eval_readiness_trace_jsonl,
    append_coding_service_eval_runner_trace_jsonl,
    append_evolution_goal_queue_store_write_trace_jsonl, append_improvement_corpus_trace_jsonl,
    append_memory_residency_trace_jsonl, append_rust_check_trace_jsonl,
    append_self_evolution_admission_trace_jsonl, append_self_evolution_experiment_trace_jsonl,
    append_self_evolution_operator_approval_trace_jsonl,
    append_self_evolution_promotion_preflight_trace_jsonl,
    append_self_evolution_rollback_replay_apply_trace_jsonl,
    append_self_evolution_rollback_replay_gate_trace_jsonl,
    append_self_evolution_rollback_replay_trace_jsonl,
    append_self_goal_queue_append_execution_trace_jsonl, append_self_goal_queue_apply_trace_jsonl,
    append_trace_jsonl, append_trace_jsonl_with_case, append_unified_writer_gate_trace_jsonl,
    business_contract_trace_json_line, coding_service_eval_readiness_trace_json_line,
    coding_service_eval_runner_trace_json_line, evolution_goal_queue_store_write_trace_json_line,
    improvement_corpus_trace_json_line, memory_residency_trace_json_line,
    rust_check_trace_json_line, self_evolution_admission_trace_json_line,
    self_evolution_experiment_trace_json_line, self_evolution_operator_approval_trace_json_line,
    self_evolution_promotion_preflight_trace_json_line,
    self_evolution_rollback_replay_apply_trace_json_line,
    self_evolution_rollback_replay_gate_trace_json_line,
    self_evolution_rollback_replay_trace_json_line,
    self_goal_queue_append_execution_trace_json_line, self_goal_queue_apply_trace_json_line,
    trace_json_line, trace_json_line_with_case, unified_writer_gate_trace_json_line,
};
pub use schema_jsonl_gate::{
    OPERATOR_HEALTH_SCHEMA, OperatorHealthMetric, OperatorHealthSection, OperatorHealthSnapshot,
    SelfEvolutionOperatorApprovalServiceCounters, TraceSchemaGateReport,
    evaluate_trace_schema_jsonl,
};

pub fn evaluate_trace_schema_line(line: &str) -> Vec<String> {
    let mut failures = Vec::new();
    let line = line.trim();

    if !line.starts_with('{') || !line.ends_with('}') {
        failures.push("record is not a single JSON object line".to_owned());
    }

    if line.contains("\"schema\":\"rust-norion-rust-check-v1\"") {
        failures.extend(evaluate_rust_check_trace_schema_line(line));
        return failures;
    }
    if line.contains("\"schema\":\"rust-norion-business-contract-v1\"") {
        failures.extend(evaluate_business_contract_trace_schema_line(line));
        return failures;
    }
    if line.contains("\"schema\":\"rust-norion-self-evolution-admission-v1\"") {
        failures.extend(evaluate_self_evolution_admission_schema_line(line));
        return failures;
    }
    if line.contains("\"schema\":\"rust-norion-self-evolution-experiment-v1\"") {
        failures.extend(evaluate_self_evolution_experiment_schema_line(line));
        return failures;
    }
    if line.contains("\"schema\":\"rust-norion-self-evolution-rollback-replay-plan-v1\"") {
        failures.extend(evaluate_self_evolution_rollback_replay_schema_line(line));
        return failures;
    }
    if line.contains("\"schema\":\"rust-norion-self-evolution-rollback-replay-gate-v1\"") {
        failures.extend(evaluate_self_evolution_rollback_replay_gate_schema_line(
            line,
        ));
        return failures;
    }
    if line.contains("\"schema\":\"rust-norion-self-evolution-operator-approval-v1\"") {
        failures.extend(evaluate_self_evolution_operator_approval_schema_line(line));
        return failures;
    }
    if line.contains("\"schema\":\"rust-norion-self-evolution-promotion-preflight-v1\"") {
        failures.extend(evaluate_self_evolution_promotion_preflight_schema_line(
            line,
        ));
        return failures;
    }
    if line.contains("\"schema\":\"rust-norion-self-evolution-rollback-replay-apply-v1\"") {
        failures.extend(evaluate_self_evolution_rollback_replay_apply_schema_line(
            line,
        ));
        return failures;
    }
    if line.contains("\"schema\":\"rust-norion-self-evolving-memory-store-v1\"") {
        failures.extend(evaluate_self_evolving_memory_store_schema_line(line));
        return failures;
    }
    if line.contains("\"schema\":\"rust-norion-memory-residency-plan-v1\"") {
        failures.extend(evaluate_trace_memory_residency_schema_line(line));
        return failures;
    }
    if line.contains("\"schema\":\"rust-norion-improvement-corpus-v1\"") {
        failures.extend(evaluate_improvement_corpus_schema_line(line));
        return failures;
    }
    if line.contains("\"schema\":\"rust-norion-unified-writer-gate-v1\"") {
        failures.extend(evaluate_unified_writer_gate_schema_line(line));
        return failures;
    }
    if line.contains("\"schema\":\"rust-norion-dna-evolution-apply-plan-v1\"") {
        failures.extend(evaluate_dna_evolution_apply_plan_schema_line(line));
        return failures;
    }
    if line.contains("\"schema\":\"dna_evolution_controller_v1\"") {
        failures.extend(evaluate_dna_evolution_controller_schema_line(line));
        return failures;
    }
    if line.contains("\"schema\":\"rust-norion-self-goal-queue-apply-plan-v1\"") {
        failures.extend(evaluate_self_goal_queue_apply_schema_line(line));
        return failures;
    }
    if line.contains("\"schema\":\"rust-norion-self-goal-queue-append-execution-v1\"") {
        failures.extend(evaluate_self_goal_queue_append_execution_schema_line(line));
        return failures;
    }
    if line.contains("\"schema\":\"rust-norion-self-goal-queue-continuation-plan-v1\"") {
        failures.extend(evaluate_self_goal_queue_continuation_schema_line(line));
        return failures;
    }
    if line.contains("\"schema\":\"rust-norion-self-goal-queue-evidence-plan-v1\"") {
        failures.extend(evaluate_self_goal_queue_evidence_plan_schema_line(line));
        return failures;
    }
    if line.contains("\"schema\":\"rust-norion-self-goal-queue-evidence-collection-v1\"") {
        failures.extend(evaluate_self_goal_queue_evidence_collection_schema_line(
            line,
        ));
        return failures;
    }
    if line.contains("\"schema\":\"rust-norion-self-goal-local-evidence-v1\"") {
        failures.extend(evaluate_self_goal_local_evidence_schema_line(line));
        return failures;
    }
    if line.contains("\"schema\":\"rust-norion-coding-service-eval-readiness-v1\"") {
        failures.extend(evaluate_coding_service_eval_schema_line(line));
        return failures;
    }
    if line.contains("\"schema\":\"rust-norion-evolution-goal-queue-store-write-v1\"") {
        failures.extend(evaluate_evolution_goal_queue_store_write_schema_line(line));
        return failures;
    }
    if line.contains("\"schema\":\"rust-norion-reasoning-chaperone-fold-guard-v1\"") {
        failures.extend(evaluate_reasoning_chaperone_fold_guard_schema_line(line));
        return failures;
    }
    if line.contains("\"schema\":\"rust-norion-agent-tool-build-report-v1\"") {
        failures.extend(evaluate_agent_tool_build_report_schema_line(line));
        return failures;
    }
    if line.contains("\"schema\":\"rust-norion-clean-room-audit-v1\"") {
        failures.extend(evaluate_clean_room_audit_schema_line(line));
        return failures;
    }

    for field in trace_required_fields() {
        if !line.contains(field.marker) {
            failures.push(format!("missing trace field {}", field.name));
        }
    }

    failures.extend(evaluate_trace_device_contract(line));
    failures.extend(evaluate_trace_embedding(line));
    failures.extend(evaluate_trace_runtime_device_execution(line));
    failures.extend(evaluate_trace_adapter_observations(line));
    failures.extend(evaluate_trace_runtime_kv(line));
    failures.extend(evaluate_trace_adaptive_routing(line));
    failures.extend(evaluate_trace_compute_budget(line));
    failures.extend(evaluate_trace_task_hierarchy(line));
    failures.extend(evaluate_trace_memory_feedback(line));
    failures.extend(evaluate_trace_memory_admission(line));
    failures.extend(evaluate_trace_kv_fusion(line));
    failures.extend(evaluate_trace_memory_governance(line));
    failures.extend(evaluate_trace_drift(line));
    failures.extend(evaluate_trace_development_evidence_surface(line));
    failures.extend(evaluate_trace_reasoning_genome(line));
    failures.extend(evaluate_trace_agent_team(line));
    failures.extend(evaluate_trace_auto_replay(line));
    failures.extend(evaluate_trace_live_evolution(line));

    failures
}

fn evaluate_trace_development_evidence_surface(line: &str) -> Vec<String> {
    let Some(surface) = json_object_after_field(line, "development_evidence_surface") else {
        return Vec::new();
    };

    if extract_json_bool_field(surface, "allowed").unwrap_or(true) {
        Vec::new()
    } else {
        vec!["development_evidence_surface blocked trace evidence".to_owned()]
    }
}

fn evaluate_clean_room_audit_schema_line(line: &str) -> Vec<String> {
    let mut failures = Vec::new();
    let records = extract_json_usize_field(line, "records").unwrap_or(0);
    let external_agent_references =
        extract_json_usize_field(line, "external_agent_references").unwrap_or(0);
    let rust_code_references = extract_json_usize_field(line, "rust_code_references").unwrap_or(0);
    let claurst_references = extract_json_usize_field(line, "claurst_references").unwrap_or(0);
    let copied_external_material =
        extract_json_usize_field(line, "copied_external_material").unwrap_or(0);
    let vendored_external_source =
        extract_json_usize_field(line, "vendored_external_source").unwrap_or(0);
    let generated_from_external_source =
        extract_json_usize_field(line, "generated_from_external_source").unwrap_or(0);
    let private_payload = extract_json_usize_field(line, "private_payload").unwrap_or(0);
    let failures_count = extract_json_usize_field(line, "failures").unwrap_or(0);

    if !extract_json_bool_field(line, "passed").unwrap_or(false) {
        failures.push("clean-room audit did not pass".to_owned());
    }
    if records == 0 {
        failures.push("clean-room audit has no records".to_owned());
    }
    if external_agent_references < 2 || rust_code_references == 0 || claurst_references == 0 {
        failures.push("clean-room audit missing external agent references".to_owned());
    }
    if copied_external_material > 0 || vendored_external_source > 0 {
        failures.push("clean-room audit contains copied or vendored external material".to_owned());
    }
    if generated_from_external_source > 0 || private_payload > 0 {
        failures.push(
            "clean-room audit contains generated external source or private payload".to_owned(),
        );
    }
    if failures_count > 0 {
        failures.push("clean-room audit reports failures".to_owned());
    }
    if !extract_json_bool_field(line, "preview_only").unwrap_or(false)
        || extract_json_bool_field(line, "write_allowed").unwrap_or(true)
        || extract_json_bool_field(line, "applied").unwrap_or(true)
    {
        failures.push("clean-room audit must remain preview-only and unapplied".to_owned());
    }

    failures
}

#[cfg(test)]
mod tests;
