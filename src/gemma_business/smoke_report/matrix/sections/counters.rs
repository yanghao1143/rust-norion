mod fields;

use fields::{
    external_feedbacks, feedback_memory_updates, live_evolution_items,
    live_memory_feedback_applied, replay_rust_check_passed, runtime_tokens,
};

pub(super) struct MatrixReportCounters {
    pub(super) runtime_tokens: u64,
    pub(super) external_feedbacks: u64,
    pub(super) feedback_memory_updates: u64,
    pub(super) replay_rust_check_passed: u64,
    pub(super) live_memory_feedback_applied: u64,
    pub(super) live_evolution_items: u64,
}

impl MatrixReportCounters {
    pub(super) fn from_body(body: &str) -> Self {
        Self {
            runtime_tokens: runtime_tokens(body),
            external_feedbacks: external_feedbacks(body),
            feedback_memory_updates: feedback_memory_updates(body),
            replay_rust_check_passed: replay_rust_check_passed(body),
            live_memory_feedback_applied: live_memory_feedback_applied(body),
            live_evolution_items: live_evolution_items(body),
        }
    }
}
