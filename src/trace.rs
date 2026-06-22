#[cfg(test)]
use std::fs;

const TRACE_FLOAT_EPSILON: f32 = 0.000_001;
mod adapter;
mod admission;
mod agent_team;
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
use device_contract::evaluate_trace_device_contract;
use embedding::evaluate_trace_embedding;
use evolution::{evaluate_trace_auto_replay, evaluate_trace_live_evolution};
use genome::evaluate_trace_reasoning_genome;
use improvement_corpus::evaluate_improvement_corpus_schema_line;
use required_fields::trace_required_fields;
use runtime_device::evaluate_trace_runtime_device_execution;
use runtime_kv::evaluate_trace_runtime_kv;
use self_goal::{
    evaluate_evolution_goal_queue_store_write_schema_line,
    evaluate_self_goal_queue_append_execution_schema_line,
    evaluate_self_goal_queue_apply_schema_line, evaluate_self_goal_queue_continuation_schema_line,
    evaluate_self_goal_queue_evidence_collection_schema_line,
    evaluate_self_goal_queue_evidence_plan_schema_line,
};
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
    append_business_contract_trace_jsonl, append_evolution_goal_queue_store_write_trace_jsonl,
    append_improvement_corpus_trace_jsonl, append_memory_residency_trace_jsonl,
    append_rust_check_trace_jsonl, append_self_evolution_admission_trace_jsonl,
    append_self_evolution_experiment_trace_jsonl,
    append_self_evolution_operator_approval_trace_jsonl,
    append_self_evolution_promotion_preflight_trace_jsonl,
    append_self_evolution_rollback_replay_apply_trace_jsonl,
    append_self_evolution_rollback_replay_gate_trace_jsonl,
    append_self_evolution_rollback_replay_trace_jsonl,
    append_self_goal_queue_append_execution_trace_jsonl, append_self_goal_queue_apply_trace_jsonl,
    append_trace_jsonl, append_trace_jsonl_with_case, append_unified_writer_gate_trace_jsonl,
    business_contract_trace_json_line, evolution_goal_queue_store_write_trace_json_line,
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
    if line.contains("\"schema\":\"rust-norion-evolution-goal-queue-store-write-v1\"") {
        failures.extend(evaluate_evolution_goal_queue_store_write_schema_line(line));
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
    failures.extend(evaluate_trace_reasoning_genome(line));
    failures.extend(evaluate_trace_agent_team(line));
    failures.extend(evaluate_trace_auto_replay(line));
    failures.extend(evaluate_trace_live_evolution(line));

    failures
}

#[cfg(test)]
mod tests;
