use crate::gemma_business::response_json::{
    response_bool_field, response_object_bool_field, response_ok, response_optional_string_field,
};
use crate::gemma_business::response_metrics::{
    cycle_external_feedbacks, cycle_feedback_memory_updates, cycle_replay_rust_check_passed,
    live_evolution_items as cycle_live_evolution_items,
    live_memory_feedback_applied as cycle_live_memory_feedback_applied,
    runtime_tokens as cycle_runtime_tokens,
};
pub(super) fn business_cycle_ok(body: &str) -> bool {
    response_ok(body)
}

pub(super) fn business_cycle_passed(body: &str) -> bool {
    response_object_bool_field(body, "business_cycle", "passed")
}

pub(super) fn state_gate_passed(body: &str) -> bool {
    response_object_bool_field(body, "state_gate", "passed")
}

pub(super) fn trace_gate_passed(body: &str) -> bool {
    response_object_bool_field(body, "trace_gate", "passed")
}

pub(super) fn runtime_model(body: &str) -> Option<String> {
    response_optional_string_field(body, "runtime_model")
}

pub(super) fn runtime_uncertainty_signal(body: &str) -> bool {
    response_bool_field(body, "runtime_uncertainty_signal")
}

pub(super) fn rust_check_checked(body: &str) -> bool {
    response_bool_field(body, "rust_check_checked")
}

pub(super) fn rust_check_passed(body: &str) -> bool {
    response_bool_field(body, "rust_check_passed")
}

pub(super) fn self_improve_checked(body: &str) -> bool {
    response_bool_field(body, "self_improve_checked")
}

pub(super) fn self_improve_passed(body: &str) -> bool {
    response_bool_field(body, "self_improve_passed")
}

pub(super) fn runtime_tokens(body: &str) -> u64 {
    cycle_runtime_tokens(body)
}

pub(super) fn external_feedbacks(body: &str) -> u64 {
    cycle_external_feedbacks(body)
}

pub(super) fn feedback_memory_updates(body: &str) -> u64 {
    cycle_feedback_memory_updates(body)
}

pub(super) fn replay_rust_check_passed(body: &str) -> u64 {
    cycle_replay_rust_check_passed(body)
}

pub(super) fn live_memory_feedback_applied(body: &str) -> u64 {
    cycle_live_memory_feedback_applied(body)
}

pub(super) fn live_evolution_items(body: &str) -> u64 {
    cycle_live_evolution_items(body)
}
