mod extract;

use extract::{
    case_count, checked_trace_lines, external_feedbacks, feedback_applied, feedback_memory_updates,
    passed_cases, replay_live_evolution_items, replay_live_memory_feedback_applied,
    replay_rust_check_passed, runtime_token_count, rust_check_feedback_applied, schema,
};

pub(super) struct ReportBodyEvidence {
    pub(super) schema: Option<String>,
    pub(super) case_count: u64,
    pub(super) passed_cases: u64,
    pub(super) runtime_token_count: u64,
    pub(super) feedback_applied: u64,
    pub(super) rust_check_feedback_applied: u64,
    pub(super) external_feedbacks: u64,
    pub(super) feedback_memory_updates: u64,
    pub(super) replay_rust_check_passed: u64,
    pub(super) replay_live_memory_feedback_applied: u64,
    pub(super) replay_live_evolution_items: u64,
    pub(super) checked_trace_lines: u64,
}

impl ReportBodyEvidence {
    pub(super) fn from_body(body: &str) -> Self {
        Self {
            schema: schema(body),
            case_count: case_count(body),
            passed_cases: passed_cases(body),
            runtime_token_count: runtime_token_count(body),
            feedback_applied: feedback_applied(body),
            rust_check_feedback_applied: rust_check_feedback_applied(body),
            external_feedbacks: external_feedbacks(body),
            feedback_memory_updates: feedback_memory_updates(body),
            replay_rust_check_passed: replay_rust_check_passed(body),
            replay_live_memory_feedback_applied: replay_live_memory_feedback_applied(body),
            replay_live_evolution_items: replay_live_evolution_items(body),
            checked_trace_lines: checked_trace_lines(body),
        }
    }
}
