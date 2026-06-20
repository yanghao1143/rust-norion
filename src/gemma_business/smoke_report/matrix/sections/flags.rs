mod fields;

use crate::gemma_business::smoke_report::types::GemmaBusinessCycleCaseResult;
use fields::{
    rust_check_checked, rust_check_passed, rust_check_passed_cases, self_improve_checked,
    self_improve_passed, self_improve_passed_cases, state_gate_passed, trace_gate_passed,
};

pub(super) struct MatrixReportCaseFlags {
    pub(super) state_gate_passed: bool,
    pub(super) trace_gate_passed: bool,
    pub(super) rust_check_checked: bool,
    pub(super) rust_check_passed: bool,
    pub(super) rust_check_passed_cases: usize,
    pub(super) self_improve_checked: bool,
    pub(super) self_improve_passed: bool,
    pub(super) self_improve_passed_cases: usize,
}

impl MatrixReportCaseFlags {
    pub(super) fn from_cases(case_results: &[GemmaBusinessCycleCaseResult]) -> Self {
        Self {
            state_gate_passed: state_gate_passed(case_results),
            trace_gate_passed: trace_gate_passed(case_results),
            rust_check_checked: rust_check_checked(case_results),
            rust_check_passed: rust_check_passed(case_results),
            rust_check_passed_cases: rust_check_passed_cases(case_results),
            self_improve_checked: self_improve_checked(case_results),
            self_improve_passed: self_improve_passed(case_results),
            self_improve_passed_cases: self_improve_passed_cases(case_results),
        }
    }
}
