use crate::gemma_business::response_metrics::{
    checked_trace_lines as response_checked_trace_lines,
    feedback_applied as response_feedback_applied, live_evolution_items,
    live_memory_feedback_applied, report_external_feedbacks, report_feedback_memory_updates,
    report_replay_rust_check_passed, runtime_token_count as response_runtime_token_count,
    rust_check_feedback_applied as response_rust_check_feedback_applied,
};

pub(in crate::gemma_business::regression::gate::body::evidence) fn runtime_token_count(
    body: &str,
) -> u64 {
    response_runtime_token_count(body)
}

pub(in crate::gemma_business::regression::gate::body::evidence) fn feedback_applied(
    body: &str,
) -> u64 {
    response_feedback_applied(body)
}

pub(in crate::gemma_business::regression::gate::body::evidence) fn rust_check_feedback_applied(
    body: &str,
) -> u64 {
    response_rust_check_feedback_applied(body)
}

pub(in crate::gemma_business::regression::gate::body::evidence) fn external_feedbacks(
    body: &str,
) -> u64 {
    report_external_feedbacks(body)
}

pub(in crate::gemma_business::regression::gate::body::evidence) fn feedback_memory_updates(
    body: &str,
) -> u64 {
    report_feedback_memory_updates(body)
}

pub(in crate::gemma_business::regression::gate::body::evidence) fn replay_rust_check_passed(
    body: &str,
) -> u64 {
    report_replay_rust_check_passed(body)
}

pub(in crate::gemma_business::regression::gate::body::evidence) fn replay_live_memory_feedback_applied(
    body: &str,
) -> u64 {
    live_memory_feedback_applied(body)
}

pub(in crate::gemma_business::regression::gate::body::evidence) fn replay_live_evolution_items(
    body: &str,
) -> u64 {
    live_evolution_items(body)
}

pub(in crate::gemma_business::regression::gate::body::evidence) fn checked_trace_lines(
    body: &str,
) -> u64 {
    response_checked_trace_lines(body)
}
