use std::{fs, io};

use super::evolution_candidate_status::{
    candidate_backlog_path, candidate_backlog_status_json, daemon_start_gate_status_json,
    read_candidate_backlog_summary,
};
use super::evolution_clean_room_handoff_status::clean_room_handoff_report_json;
use super::evolution_daemon_log_tail_status::daemon_log_tail_status_json;
use super::evolution_helper_stage_repair_panel::helper_stage_repair_panel_json;
use super::evolution_readiness_start_status::{ReadinessStartStatus, readiness_start_gate_json};
use super::evolution_report_gate_status::{read_report_gate_status, report_gate_preflight_json};
use super::evolution_self_improve_proposal_panel::self_improve_proposal_panel_json;
use super::evolution_start_plan_status::{
    unattended_start_next_step_with_readiness, unattended_start_plan_json_with_readiness,
};
use super::evolution_unified_status::unified_status_json;
use super::evolution_worker_window_status::{
    context_hygiene_status_json, daemon_round_transition_status_json,
    next_round_decision_status_json, next_round_decision_status_lines,
    next_round_downstream_status_consumers_json, next_round_downstream_status_consumers_lines,
    worker_window_replacement_report_json, worker_window_status_json,
};
use super::status_json::{json_object_field, json_string_field, json_string_literal};

pub(super) fn render_enriched_evolution_status_json(
    status: &str,
    work_dir: &str,
) -> io::Result<String> {
    let path = candidate_backlog_path(work_dir);
    let candidate_summary = read_candidate_backlog_summary(&path)?;
    let candidate_json = candidate_backlog_status_json(&path, candidate_summary.as_ref());
    let daemon_gate_json = daemon_start_gate_status_json(candidate_summary.as_ref());
    let loop_status = json_object_field(status, "loop");
    let ledger = loop_status.and_then(|loop_status| json_object_field(loop_status, "ledger"));
    let backend_endpoint = loop_status
        .and_then(|loop_status| json_string_field(loop_status, "backend_endpoint"))
        .unwrap_or_default();
    let daemon = json_object_field(status, "daemon").unwrap_or("{}");
    let report_gate_status = read_report_gate_status(loop_status);
    let readiness_status = ReadinessStartStatus::from_loop_status(loop_status);
    let readiness_gate_json = readiness_start_gate_json(&readiness_status);
    let remote_chain =
        loop_status.and_then(|loop_status| json_object_field(loop_status, "remote_chain"));
    let effective_next_step = unattended_start_next_step_with_readiness(
        daemon,
        work_dir,
        &backend_endpoint,
        candidate_summary.as_ref(),
        &report_gate_status,
        remote_chain,
        Some(&readiness_status),
    );
    let start_plan_json = unattended_start_plan_json_with_readiness(
        daemon,
        work_dir,
        &backend_endpoint,
        candidate_summary.as_ref(),
        &report_gate_status,
        remote_chain,
        Some(&readiness_status),
    );
    let report_gate_preflight_json =
        report_gate_preflight_json(&report_gate_status, work_dir, &backend_endpoint);
    let daemon_log_tail_json = daemon_log_tail_status_json(daemon, ledger);
    let daemon_round_transition_json = daemon_round_transition_status_json(loop_status);
    let context_hygiene_json = context_hygiene_status_json(loop_status);
    let worker_window_report_json = if report_gate_status.report_path().trim().is_empty() {
        None
    } else {
        fs::read_to_string(report_gate_status.report_path()).ok()
    };
    let next_round_decision_json = if !next_round_decision_status_lines(loop_status).is_empty() {
        next_round_decision_status_json(loop_status)
    } else if !next_round_decision_status_lines(Some(status)).is_empty() {
        next_round_decision_status_json(Some(status))
    } else {
        next_round_decision_status_json(worker_window_report_json.as_deref())
    };
    let next_round_downstream_status_consumers_json =
        if !next_round_downstream_status_consumers_lines(loop_status).is_empty() {
            next_round_downstream_status_consumers_json(loop_status)
        } else {
            next_round_downstream_status_consumers_json(Some(status))
        };
    let worker_window_status_json = worker_window_status_json(loop_status);
    let worker_window_replacement_report_json =
        worker_window_replacement_report_json(worker_window_report_json.as_deref());
    let clean_room_handoff_report_json =
        clean_room_handoff_report_json(worker_window_report_json.as_deref());
    let self_improve_proposal_panel_json =
        self_improve_proposal_panel_json(worker_window_report_json.as_deref());
    let helper_stage_repair_panel_json =
        helper_stage_repair_panel_json(worker_window_report_json.as_deref());
    let unified_status_json = unified_status_json(
        status,
        &worker_window_replacement_report_json,
        &clean_room_handoff_report_json,
        &self_improve_proposal_panel_json,
        &helper_stage_repair_panel_json,
    );

    Ok(format!(
        "{{\"schema\":\"smartsteam.forge.evolution_status.v1\",\"read_only\":true,\"starts_process\":false,\"sends_prompt\":false,\"work_dir\":{},\"evolution_status\":{},\"daemon_log_tail\":{},\"daemon_round_transition_status\":{},\"context_hygiene_status\":{},\"next_round_decision_status\":{},\"next_round_downstream_status_consumers\":{},\"report_gate_status\":{},\"report_gate_preflight\":{},\"report_gate_start_gate\":{},\"candidate_backlog\":{},\"daemon_start_gate\":{},\"readiness_start_gate\":{},\"worker_window_status\":{},\"worker_window_replacement_report\":{},\"clean_room_handoff_report\":{},\"self_improve_proposal_panel\":{},\"helper_stage_repair_panel\":{},\"unified_status\":{},\"unattended_start_plan\":{},\"next_step\":{}}}",
        json_string_literal(work_dir),
        status.trim(),
        daemon_log_tail_json,
        daemon_round_transition_json,
        context_hygiene_json,
        next_round_decision_json,
        next_round_downstream_status_consumers_json,
        report_gate_status.to_json(),
        report_gate_preflight_json,
        report_gate_preflight_json,
        candidate_json,
        daemon_gate_json,
        readiness_gate_json,
        worker_window_status_json,
        worker_window_replacement_report_json,
        clean_room_handoff_report_json,
        self_improve_proposal_panel_json,
        helper_stage_repair_panel_json,
        unified_status_json,
        start_plan_json,
        json_string_literal(&effective_next_step)
    ))
}
