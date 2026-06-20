mod cases;
mod counters;
mod flags;

use crate::gemma_business::health_status::SmokeHealthStatus;
use crate::gemma_business::smoke_report::types::GemmaBusinessCycleCaseResult;
use counters::MatrixReportCounters;
use flags::MatrixReportCaseFlags;

pub(super) struct MatrixReportSections {
    pub(super) health: SmokeHealthStatus,
    pub(super) state_gate_passed: bool,
    pub(super) trace_gate_passed: bool,
    pub(super) rust_check_checked: bool,
    pub(super) rust_check_passed: bool,
    pub(super) rust_check_passed_cases: usize,
    pub(super) self_improve_checked: bool,
    pub(super) self_improve_passed: bool,
    pub(super) self_improve_passed_cases: usize,
    pub(super) runtime_tokens: u64,
    pub(super) external_feedbacks: u64,
    pub(super) feedback_memory_updates: u64,
    pub(super) replay_rust_check_passed: u64,
    pub(super) live_memory_feedback_applied: u64,
    pub(super) live_evolution_items: u64,
}

impl MatrixReportSections {
    pub(super) fn from_cases(
        health_body: &str,
        final_cycle_body: &str,
        case_results: &[GemmaBusinessCycleCaseResult],
    ) -> Self {
        let counters = MatrixReportCounters::from_body(final_cycle_body);
        let flags = MatrixReportCaseFlags::from_cases(case_results);
        Self {
            health: SmokeHealthStatus::from_body(health_body),
            state_gate_passed: flags.state_gate_passed,
            trace_gate_passed: flags.trace_gate_passed,
            rust_check_checked: flags.rust_check_checked,
            rust_check_passed: flags.rust_check_passed,
            rust_check_passed_cases: flags.rust_check_passed_cases,
            self_improve_checked: flags.self_improve_checked,
            self_improve_passed: flags.self_improve_passed,
            self_improve_passed_cases: flags.self_improve_passed_cases,
            runtime_tokens: counters.runtime_tokens,
            external_feedbacks: counters.external_feedbacks,
            feedback_memory_updates: counters.feedback_memory_updates,
            replay_rust_check_passed: counters.replay_rust_check_passed,
            live_memory_feedback_applied: counters.live_memory_feedback_applied,
            live_evolution_items: counters.live_evolution_items,
        }
    }
}
