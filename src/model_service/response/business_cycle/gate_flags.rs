use super::super::super::types::ModelServiceBusinessCycleReport;
use super::super::update_stats::memory_update_applied_count;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct BusinessCycleGateFlags {
    pub(super) passed: bool,
    pub(super) feedback_passed: bool,
    pub(super) feedback_applied: usize,
    pub(super) rust_check_checked: bool,
    pub(super) rust_check_passed: bool,
    pub(super) rust_check_feedback_applied: usize,
    pub(super) self_improve_checked: bool,
    pub(super) self_improve_passed: bool,
    pub(super) state_gate_checked: bool,
    pub(super) state_gate_passed: bool,
    pub(super) trace_gate_checked: bool,
    pub(super) trace_gate_passed: bool,
}

impl BusinessCycleGateFlags {
    pub(super) fn from_report(report: &ModelServiceBusinessCycleReport) -> Self {
        Self::evaluate(GateInputs {
            feedback_applied: memory_update_applied_count(&report.feedback_updates),
            rust_check_checked: report.rust_check_report.is_some(),
            rust_check_passed: report
                .rust_check_report
                .as_ref()
                .map(|report| report.passed)
                .unwrap_or(true),
            rust_check_feedback_applied: memory_update_applied_count(&report.rust_check_updates),
            self_improve_checked: report.self_improve_enabled,
            self_improve_passed: if report.self_improve_enabled {
                report
                    .replay_report
                    .as_ref()
                    .map(|report| report.applied > 0)
                    .unwrap_or(false)
            } else {
                true
            },
            state_gate_checked: report.state_gate_report.is_some(),
            state_gate_passed: report
                .state_gate_report
                .as_ref()
                .map(|report| report.passed)
                .unwrap_or(true),
            trace_gate_checked: report.trace_gate_report.is_some(),
            trace_gate_passed: report
                .trace_gate_report
                .as_ref()
                .map(|report| report.passed)
                .unwrap_or(true),
        })
    }

    fn evaluate(inputs: GateInputs) -> Self {
        let feedback_passed = inputs.feedback_applied > 0;
        let passed = feedback_passed
            && inputs.rust_check_passed
            && inputs.self_improve_passed
            && inputs.state_gate_passed
            && inputs.trace_gate_passed;
        Self {
            passed,
            feedback_passed,
            feedback_applied: inputs.feedback_applied,
            rust_check_checked: inputs.rust_check_checked,
            rust_check_passed: inputs.rust_check_passed,
            rust_check_feedback_applied: inputs.rust_check_feedback_applied,
            self_improve_checked: inputs.self_improve_checked,
            self_improve_passed: inputs.self_improve_passed,
            state_gate_checked: inputs.state_gate_checked,
            state_gate_passed: inputs.state_gate_passed,
            trace_gate_checked: inputs.trace_gate_checked,
            trace_gate_passed: inputs.trace_gate_passed,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct GateInputs {
    feedback_applied: usize,
    rust_check_checked: bool,
    rust_check_passed: bool,
    rust_check_feedback_applied: usize,
    self_improve_checked: bool,
    self_improve_passed: bool,
    state_gate_checked: bool,
    state_gate_passed: bool,
    trace_gate_checked: bool,
    trace_gate_passed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passes_when_feedback_applies_and_optional_gates_pass_or_are_unchecked() {
        let flags = BusinessCycleGateFlags::evaluate(GateInputs {
            feedback_applied: 1,
            rust_check_checked: false,
            rust_check_passed: true,
            rust_check_feedback_applied: 0,
            self_improve_checked: false,
            self_improve_passed: true,
            state_gate_checked: false,
            state_gate_passed: true,
            trace_gate_checked: false,
            trace_gate_passed: true,
        });

        assert!(flags.passed);
        assert!(flags.feedback_passed);
        assert_eq!(flags.feedback_applied, 1);
        assert!(!flags.rust_check_checked);
    }

    #[test]
    fn fails_when_required_feedback_did_not_apply() {
        let flags = BusinessCycleGateFlags::evaluate(GateInputs {
            feedback_applied: 0,
            rust_check_checked: false,
            rust_check_passed: true,
            rust_check_feedback_applied: 0,
            self_improve_checked: false,
            self_improve_passed: true,
            state_gate_checked: false,
            state_gate_passed: true,
            trace_gate_checked: false,
            trace_gate_passed: true,
        });

        assert!(!flags.passed);
        assert!(!flags.feedback_passed);
    }

    #[test]
    fn fails_when_checked_rust_or_state_or_trace_gate_fails() {
        for failing_inputs in [
            GateInputs {
                rust_check_checked: true,
                rust_check_passed: false,
                rust_check_feedback_applied: 1,
                ..passing_inputs()
            },
            GateInputs {
                state_gate_checked: true,
                state_gate_passed: false,
                ..passing_inputs()
            },
            GateInputs {
                trace_gate_checked: true,
                trace_gate_passed: false,
                ..passing_inputs()
            },
        ] {
            let flags = BusinessCycleGateFlags::evaluate(failing_inputs);

            assert!(!flags.passed);
        }
    }

    #[test]
    fn fails_when_self_improve_was_requested_without_replay_application() {
        let flags = BusinessCycleGateFlags::evaluate(GateInputs {
            self_improve_checked: true,
            self_improve_passed: false,
            ..passing_inputs()
        });

        assert!(!flags.passed);
        assert!(flags.self_improve_checked);
        assert!(!flags.self_improve_passed);
    }

    fn passing_inputs() -> GateInputs {
        GateInputs {
            feedback_applied: 1,
            rust_check_checked: false,
            rust_check_passed: true,
            rust_check_feedback_applied: 0,
            self_improve_checked: false,
            self_improve_passed: true,
            state_gate_checked: false,
            state_gate_passed: true,
            trace_gate_checked: false,
            trace_gate_passed: true,
        }
    }
}
