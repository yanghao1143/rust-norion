use std::path::PathBuf;

mod answer_preview;
mod evidence;
mod files;
mod render;

use crate::Args;
use crate::gemma_business::GemmaModelServiceBusinessCase;
use crate::gemma_business::audit::GemmaModelServiceAnswerAudit;
use crate::gemma_business::health_status::SmokeHealthStatus;

use answer_preview::single_report_answer_preview;
use evidence::SingleReportEvidence;
use files::SingleReportFiles;
use render::{SingleReportRender, render_single_report_json};

#[allow(clippy::too_many_arguments)]
pub(crate) fn gemma_business_cycle_smoke_report_json(
    passed: bool,
    bind: &str,
    business_case: &GemmaModelServiceBusinessCase,
    args: &Args,
    response_path: Option<&PathBuf>,
    health_body: &str,
    cycle_body: &str,
    answer_audit: &GemmaModelServiceAnswerAudit,
    failures: &[String],
    runtime_token_count: u64,
    feedback_applied: u64,
    rust_check_feedback_applied: u64,
    checked_trace_lines: u64,
) -> String {
    let answer_preview = single_report_answer_preview(cycle_body);
    let evidence = SingleReportEvidence::from_cycle_body(cycle_body);
    let files = SingleReportFiles::from_args(args, response_path);
    let health = SmokeHealthStatus::from_body(health_body);
    render_single_report_json(SingleReportRender {
        passed,
        bind,
        business_case_name: business_case.name,
        trace_path_json: &files.trace,
        memory_path_json: &files.memory,
        experience_path_json: &files.experience,
        adaptive_path_json: &files.adaptive,
        response_path_json: &files.response,
        health: &health,
        business_cycle_ok: evidence.business_cycle_ok,
        business_cycle_passed: evidence.business_cycle_passed,
        state_gate_passed: evidence.state_gate_passed,
        trace_gate_passed: evidence.trace_gate_passed,
        runtime_model: evidence.runtime_model.as_deref(),
        runtime_token_count,
        runtime_uncertainty_signal: evidence.runtime_uncertainty_signal,
        answer_preview: &answer_preview,
        answer_audit,
        feedback_applied,
        rust_check_feedback_applied,
        rust_check_checked: evidence.rust_check_checked,
        rust_check_passed: evidence.rust_check_passed,
        self_improve_checked: evidence.self_improve_checked,
        self_improve_passed: evidence.self_improve_passed,
        runtime_tokens: evidence.runtime_tokens,
        external_feedbacks: evidence.external_feedbacks,
        feedback_memory_updates: evidence.feedback_memory_updates,
        replay_rust_check_passed: evidence.replay_rust_check_passed,
        live_memory_feedback_applied: evidence.live_memory_feedback_applied,
        live_evolution_items: evidence.live_evolution_items,
        checked_trace_lines,
        failures,
    })
}
