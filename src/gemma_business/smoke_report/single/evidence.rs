mod fields;

use fields::{
    business_cycle_ok, business_cycle_passed, external_feedbacks, feedback_memory_updates,
    live_evolution_items, live_memory_feedback_applied, replay_rust_check_passed, runtime_model,
    runtime_tokens, runtime_uncertainty_signal, rust_check_checked, rust_check_passed,
    self_improve_checked, self_improve_passed, state_gate_passed, trace_gate_passed,
};

pub(super) struct SingleReportEvidence {
    pub(super) business_cycle_ok: bool,
    pub(super) business_cycle_passed: bool,
    pub(super) state_gate_passed: bool,
    pub(super) trace_gate_passed: bool,
    pub(super) runtime_model: Option<String>,
    pub(super) runtime_uncertainty_signal: bool,
    pub(super) rust_check_checked: bool,
    pub(super) rust_check_passed: bool,
    pub(super) self_improve_checked: bool,
    pub(super) self_improve_passed: bool,
    pub(super) runtime_tokens: u64,
    pub(super) external_feedbacks: u64,
    pub(super) feedback_memory_updates: u64,
    pub(super) replay_rust_check_passed: u64,
    pub(super) live_memory_feedback_applied: u64,
    pub(super) live_evolution_items: u64,
}

impl SingleReportEvidence {
    pub(super) fn from_cycle_body(cycle_body: &str) -> Self {
        Self {
            business_cycle_ok: business_cycle_ok(cycle_body),
            business_cycle_passed: business_cycle_passed(cycle_body),
            state_gate_passed: state_gate_passed(cycle_body),
            trace_gate_passed: trace_gate_passed(cycle_body),
            runtime_model: runtime_model(cycle_body),
            runtime_uncertainty_signal: runtime_uncertainty_signal(cycle_body),
            rust_check_checked: rust_check_checked(cycle_body),
            rust_check_passed: rust_check_passed(cycle_body),
            self_improve_checked: self_improve_checked(cycle_body),
            self_improve_passed: self_improve_passed(cycle_body),
            runtime_tokens: runtime_tokens(cycle_body),
            external_feedbacks: external_feedbacks(cycle_body),
            feedback_memory_updates: feedback_memory_updates(cycle_body),
            replay_rust_check_passed: replay_rust_check_passed(cycle_body),
            live_memory_feedback_applied: live_memory_feedback_applied(cycle_body),
            live_evolution_items: live_evolution_items(cycle_body),
        }
    }
}
