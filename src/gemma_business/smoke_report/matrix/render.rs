mod input;

use crate::gemma_business::smoke_report::render_parts::{
    BUSINESS_CYCLE_GATE, BUSINESS_CYCLE_SCHEMA, ContractJson, MATRIX_BUSINESS_CASE, contract_json,
    feedback_json, files_json, generate_json, http_json, matrix_check_json, replay_json,
    state_json, trace_json,
};
use crate::model_service::json::{service_json_string, service_json_string_array};

pub(super) use input::MatrixReportRender;

pub(super) fn render_matrix_report_json(input: MatrixReportRender<'_>) -> String {
    let runtime_model = if input.evidence.runtime_model.is_empty() {
        None
    } else {
        Some(input.evidence.runtime_model.as_str())
    };
    let files = files_json(
        &input.files.trace,
        &input.files.memory,
        &input.files.experience,
        &input.files.adaptive,
        &input.files.response,
    );
    let http = http_json(
        &input.sections.health,
        input.summary.all_expected_cases_passed,
        input.summary.all_expected_cases_passed,
        input.sections.state_gate_passed,
        input.sections.trace_gate_passed,
    );
    let generate = generate_json(
        runtime_model,
        input.runtime_token_count,
        input.evidence.any_runtime_uncertainty,
        service_json_string(&input.evidence.answer_preview),
    );
    let contract = contract_json(ContractJson {
        passed: input.evidence.contract_passed(),
        required_signals: input.evidence.contract_required_signals,
        matched_signals: input.evidence.contract_matched_signals,
        missing_signals: &input.evidence.missing_signals,
        runtime_model_experiences: input.contract.runtime_model_experiences,
        protocol_leak: input.contract.protocol_leak,
        substituted_runtime_model_experiences: input.contract.substituted_runtime_model_experiences,
        evasive_denial: input.contract.evasive_denial,
        handling_signal: input.contract.handling_signal,
    });
    let feedback = feedback_json(input.feedback_applied, input.rust_check_feedback_applied);
    let state = state_json(
        input.sections.runtime_tokens,
        input.sections.external_feedbacks,
        input.sections.feedback_memory_updates,
        input.sections.replay_rust_check_passed,
    );
    let replay = replay_json(
        input.sections.live_memory_feedback_applied,
        input.sections.live_evolution_items,
    );
    let trace = trace_json(input.checked_trace_lines);
    let rust_check = matrix_check_json(
        input.sections.rust_check_checked,
        input.sections.rust_check_passed,
        input.evidence.case_count,
        input.sections.rust_check_passed_cases,
    );
    let self_improve = matrix_check_json(
        input.sections.self_improve_checked,
        input.sections.self_improve_passed,
        input.evidence.case_count,
        input.sections.self_improve_passed_cases,
    );

    format!(
        "{{\"schema\":{},\"passed\":{},\"bind\":{},\"business_case\":{},\"business_cases\":{},\"case_count\":{},\"expected_case_count\":{},\"passed_cases\":{},\"runtime_token_count\":{},\"gate\":{},\"files\":{},\"http\":{},\"generate\":{},\"contract\":{},\"feedback\":{},\"rust_check\":{},\"self_improve\":{},\"state\":{},\"replay\":{},\"trace\":{},\"cases\":[{}],\"failures\":{}}}",
        service_json_string(BUSINESS_CYCLE_SCHEMA),
        input.passed,
        service_json_string(input.bind),
        service_json_string(MATRIX_BUSINESS_CASE),
        service_json_string_array(&input.evidence.business_cases),
        input.evidence.case_count,
        input.summary.expected_case_count,
        input.evidence.passed_cases,
        input.runtime_token_count,
        service_json_string(BUSINESS_CYCLE_GATE),
        files,
        http,
        generate,
        contract,
        feedback,
        rust_check,
        self_improve,
        state,
        replay,
        trace,
        input.case_json,
        service_json_string_array(input.failures)
    )
}
