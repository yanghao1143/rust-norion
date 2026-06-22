mod admission;
mod business;
mod coding_service_eval;
mod core;
mod improvement_corpus;
mod json;
mod memory_residency;
mod rust_check;
mod self_goal;
mod summary;
mod writer;
mod writer_gate;

pub use admission::{
    append_self_evolution_admission_trace_jsonl, append_self_evolution_experiment_trace_jsonl,
    append_self_evolution_operator_approval_trace_jsonl,
    append_self_evolution_promotion_preflight_trace_jsonl,
    append_self_evolution_rollback_replay_apply_trace_jsonl,
    append_self_evolution_rollback_replay_gate_trace_jsonl,
    append_self_evolution_rollback_replay_trace_jsonl, self_evolution_admission_trace_json_line,
    self_evolution_experiment_trace_json_line, self_evolution_operator_approval_trace_json_line,
    self_evolution_promotion_preflight_trace_json_line,
    self_evolution_rollback_replay_apply_trace_json_line,
    self_evolution_rollback_replay_gate_trace_json_line,
    self_evolution_rollback_replay_trace_json_line,
};
pub use business::{append_business_contract_trace_jsonl, business_contract_trace_json_line};
pub use coding_service_eval::{
    append_coding_service_eval_readiness_trace_jsonl,
    append_coding_service_eval_runner_trace_jsonl, coding_service_eval_readiness_trace_json_line,
    coding_service_eval_runner_trace_json_line,
};
pub use core::{trace_json_line, trace_json_line_with_case};
pub use improvement_corpus::{
    append_improvement_corpus_trace_jsonl, improvement_corpus_trace_json_line,
};
pub use memory_residency::{append_memory_residency_trace_jsonl, memory_residency_trace_json_line};
pub use rust_check::{append_rust_check_trace_jsonl, rust_check_trace_json_line};
pub use self_goal::{
    append_evolution_goal_queue_store_write_trace_jsonl,
    append_self_goal_queue_append_execution_trace_jsonl, append_self_goal_queue_apply_trace_jsonl,
    evolution_goal_queue_store_write_trace_json_line,
    self_goal_queue_append_execution_trace_json_line, self_goal_queue_apply_trace_json_line,
};
pub use writer::{append_trace_jsonl, append_trace_jsonl_with_case};
pub use writer_gate::{
    append_unified_writer_gate_trace_jsonl, unified_writer_gate_trace_json_line,
};
