use super::gate_flags::BusinessCycleGateFlags;

pub(super) fn business_cycle_gate_json(flags: &BusinessCycleGateFlags) -> String {
    cycle_summary_json(CycleSummaryJsonInput {
        passed: flags.passed,
        generate_passed: true,
        feedback_passed: flags.feedback_passed,
        feedback_applied: flags.feedback_applied,
        rust_check_checked: flags.rust_check_checked,
        rust_check_passed: flags.rust_check_passed,
        rust_check_feedback_applied: flags.rust_check_feedback_applied,
        self_improve_checked: flags.self_improve_checked,
        self_improve_passed: flags.self_improve_passed,
        state_gate_checked: flags.state_gate_checked,
        state_gate_passed: flags.state_gate_passed,
        trace_gate_checked: flags.trace_gate_checked,
        trace_gate_passed: flags.trace_gate_passed,
    })
}

fn cycle_summary_json(input: CycleSummaryJsonInput) -> String {
    format!(
        "{{\"passed\":{},\"generate_passed\":{},\"feedback_passed\":{},\"feedback_applied\":{},\"rust_check_checked\":{},\"rust_check_passed\":{},\"rust_check_feedback_applied\":{},\"self_improve_checked\":{},\"self_improve_passed\":{},\"state_gate_checked\":{},\"state_gate_passed\":{},\"trace_gate_checked\":{},\"trace_gate_passed\":{}}}",
        input.passed,
        input.generate_passed,
        input.feedback_passed,
        input.feedback_applied,
        input.rust_check_checked,
        input.rust_check_passed,
        input.rust_check_feedback_applied,
        input.self_improve_checked,
        input.self_improve_passed,
        input.state_gate_checked,
        input.state_gate_passed,
        input.trace_gate_checked,
        input.trace_gate_passed
    )
}

#[derive(Debug, Clone, Copy)]
struct CycleSummaryJsonInput {
    passed: bool,
    generate_passed: bool,
    feedback_passed: bool,
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
    fn cycle_summary_json_renders_all_gate_fields() {
        let json = cycle_summary_json(CycleSummaryJsonInput {
            passed: false,
            generate_passed: true,
            feedback_passed: true,
            feedback_applied: 2,
            rust_check_checked: true,
            rust_check_passed: false,
            rust_check_feedback_applied: 1,
            self_improve_checked: true,
            self_improve_passed: true,
            state_gate_checked: true,
            state_gate_passed: true,
            trace_gate_checked: true,
            trace_gate_passed: false,
        });

        assert!(json.contains("\"passed\":false"));
        assert!(json.contains("\"generate_passed\":true"));
        assert!(json.contains("\"feedback_passed\":true"));
        assert!(json.contains("\"feedback_applied\":2"));
        assert!(json.contains("\"rust_check_checked\":true"));
        assert!(json.contains("\"rust_check_passed\":false"));
        assert!(json.contains("\"rust_check_feedback_applied\":1"));
        assert!(json.contains("\"self_improve_checked\":true"));
        assert!(json.contains("\"self_improve_passed\":true"));
        assert!(json.contains("\"state_gate_checked\":true"));
        assert!(json.contains("\"state_gate_passed\":true"));
        assert!(json.contains("\"trace_gate_checked\":true"));
        assert!(json.contains("\"trace_gate_passed\":false"));
    }
}
