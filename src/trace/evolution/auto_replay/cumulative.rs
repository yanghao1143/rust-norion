mod business_contract;
mod consistency;
mod current;
mod live_evolution;
mod live_feedback;
mod recursive;

use super::context::AutoReplayTrace;

pub(super) fn evaluate_cumulative_trace(failures: &mut Vec<String>, trace: &AutoReplayTrace) {
    let expected_memory_updates = consistency::evaluate_cumulative_consistency(failures, trace);

    current::require_current_counters(failures, trace, expected_memory_updates);
    live_feedback::require_live_feedback(failures, trace);
    business_contract::require_business_contract(failures, trace);
    live_evolution::require_live_evolution(failures, trace);
    recursive::require_recursive(failures, trace);
}
