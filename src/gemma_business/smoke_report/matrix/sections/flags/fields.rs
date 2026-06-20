use crate::gemma_business::response_json::{response_bool_field, response_object_bool_field};
use crate::gemma_business::smoke_report::matrix::sections::cases::{
    all_case_bodies_pass, case_bodies_passing,
};
use crate::gemma_business::smoke_report::types::GemmaBusinessCycleCaseResult;

pub(super) fn state_gate_passed(case_results: &[GemmaBusinessCycleCaseResult]) -> bool {
    all_case_bodies_pass(case_results, |body| {
        response_object_bool_field(body, "state_gate", "passed")
    })
}

pub(super) fn trace_gate_passed(case_results: &[GemmaBusinessCycleCaseResult]) -> bool {
    all_case_bodies_pass(case_results, |body| {
        response_object_bool_field(body, "trace_gate", "passed")
    })
}

pub(super) fn rust_check_checked(case_results: &[GemmaBusinessCycleCaseResult]) -> bool {
    all_case_bodies_pass(case_results, |body| {
        response_bool_field(body, "rust_check_checked")
    })
}

pub(super) fn rust_check_passed(case_results: &[GemmaBusinessCycleCaseResult]) -> bool {
    all_case_bodies_pass(case_results, |body| {
        response_bool_field(body, "rust_check_passed")
    })
}

pub(super) fn rust_check_passed_cases(case_results: &[GemmaBusinessCycleCaseResult]) -> usize {
    case_bodies_passing(case_results, |body| {
        response_bool_field(body, "rust_check_passed")
    })
}

pub(super) fn self_improve_checked(case_results: &[GemmaBusinessCycleCaseResult]) -> bool {
    all_case_bodies_pass(case_results, |body| {
        response_bool_field(body, "self_improve_checked")
    })
}

pub(super) fn self_improve_passed(case_results: &[GemmaBusinessCycleCaseResult]) -> bool {
    all_case_bodies_pass(case_results, |body| {
        response_bool_field(body, "self_improve_passed")
    })
}

pub(super) fn self_improve_passed_cases(case_results: &[GemmaBusinessCycleCaseResult]) -> usize {
    case_bodies_passing(case_results, |body| {
        response_bool_field(body, "self_improve_passed")
    })
}
