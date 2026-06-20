mod input;

use crate::gemma_business::smoke_report::render_parts::{
    BUSINESS_CYCLE_GATE, BUSINESS_CYCLE_SCHEMA, ContractJson, contract_json, feedback_json,
    files_json, generate_json, http_json, replay_json, single_check_json, state_json, trace_json,
};
use crate::model_service::json::{service_json_string, service_json_string_array};

pub(super) use input::SingleReportRender;

pub(super) fn render_single_report_json(input: SingleReportRender<'_>) -> String {
    let files = files_json(
        input.trace_path_json,
        input.memory_path_json,
        input.experience_path_json,
        input.adaptive_path_json,
        input.response_path_json,
    );
    let http = http_json(
        input.health,
        input.business_cycle_ok,
        input.business_cycle_passed,
        input.state_gate_passed,
        input.trace_gate_passed,
    );
    let generate = generate_json(
        input.runtime_model,
        input.runtime_token_count,
        input.runtime_uncertainty_signal,
        service_json_string(input.answer_preview),
    );
    let contract = contract_json(ContractJson {
        passed: input.answer_audit.passed(),
        required_signals: input.answer_audit.required_signals,
        matched_signals: input.answer_audit.matched_signals,
        missing_signals: &input.answer_audit.missing_signals,
        runtime_model_experiences: input.answer_audit.has_runtime_model_experiences,
        protocol_leak: input.answer_audit.protocol_leak,
        substituted_runtime_model_experiences: input
            .answer_audit
            .substituted_runtime_model_experiences,
        evasive_denial: input.answer_audit.evasive_denial,
        handling_signal: input.answer_audit.handling_signal,
    });
    let feedback = feedback_json(input.feedback_applied, input.rust_check_feedback_applied);
    let state = state_json(
        input.runtime_tokens,
        input.external_feedbacks,
        input.feedback_memory_updates,
        input.replay_rust_check_passed,
    );
    let replay = replay_json(
        input.live_memory_feedback_applied,
        input.live_evolution_items,
    );
    let trace = trace_json(input.checked_trace_lines);
    let rust_check = single_check_json(input.rust_check_checked, input.rust_check_passed);
    let self_improve = single_check_json(input.self_improve_checked, input.self_improve_passed);

    format!(
        "{{\"schema\":{},\"passed\":{},\"bind\":{},\"business_case\":{},\"gate\":{},\"files\":{},\"http\":{},\"generate\":{},\"contract\":{},\"feedback\":{},\"rust_check\":{},\"self_improve\":{},\"state\":{},\"replay\":{},\"trace\":{},\"failures\":{}}}",
        service_json_string(BUSINESS_CYCLE_SCHEMA),
        input.passed,
        service_json_string(input.bind),
        service_json_string(input.business_case_name),
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
        service_json_string_array(input.failures)
    )
}
