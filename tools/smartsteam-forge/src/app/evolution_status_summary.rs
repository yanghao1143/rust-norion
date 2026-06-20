use std::{fs, io};

use super::evolution_candidate_status::{
    candidate_backlog_lines, candidate_backlog_path, daemon_start_gate_line,
    read_candidate_backlog_summary,
};
use super::evolution_clean_room_handoff_status::{
    clean_room_handoff_report_json, clean_room_handoff_report_lines,
};
use super::evolution_daemon_log_tail_status::daemon_log_tail_status_line;
use super::evolution_helper_stage_repair_panel::{
    helper_stage_repair_panel_json, helper_stage_repair_panel_lines,
};
use super::evolution_readiness_start_status::ReadinessStartStatus;
use super::evolution_report_detail_status::{latest_model_output_lines, report_detail_lines};
use super::evolution_report_gate_status::read_report_gate_status;
use super::evolution_self_improve_proposal_panel::{
    self_improve_proposal_panel_json, self_improve_proposal_panel_lines,
};
use super::evolution_start_plan_status::{
    unattended_start_next_step_with_readiness, unattended_start_plan_lines_with_readiness,
};
use super::evolution_unified_status::unified_status_lines;
use super::evolution_worker_window_status::{
    context_hygiene_status_lines, daemon_round_transition_status_lines,
    next_round_decision_status_lines, next_round_downstream_status_consumers_lines,
    worker_window_status_lines,
};
use super::status_json::{
    compact_line, json_bool_field, json_object_field, json_string_array_field, json_string_field,
    scalar_value,
};

pub(super) fn summarize_evolution_status(status: &str, work_dir: &str) -> io::Result<String> {
    let daemon = json_object_field(status, "daemon")
        .ok_or_else(|| io::Error::other("evolution status missing daemon object"))?;
    let loop_status = json_object_field(status, "loop")
        .ok_or_else(|| io::Error::other("evolution status missing loop object"))?;
    let ledger = json_object_field(loop_status, "ledger");
    let report = json_object_field(loop_status, "report");
    let readiness = json_object_field(loop_status, "readiness");
    let backend = json_object_field(loop_status, "backend");
    let remote_chain = json_object_field(loop_status, "remote_chain");
    let model_pool = json_object_field(loop_status, "model_pool");
    let report_exists = report
        .and_then(|value| json_bool_field(value, "exists"))
        .unwrap_or(false);
    let candidate_summary = read_candidate_backlog_summary(&candidate_backlog_path(work_dir))
        .ok()
        .flatten();

    let mut lines = vec![
        "SmartSteam evolution daemon".to_owned(),
        format!(
            "read_only={} starts_process={} sends_prompt={}",
            bool_value(daemon, "read_only"),
            bool_value(daemon, "starts_process"),
            bool_value(daemon, "sends_prompt")
        ),
        format!("work_dir={work_dir}"),
        format!(
            "daemon running={} pid={} pid_file_exists={} stale_pid_file={} stale_pid={}",
            bool_value(daemon, "running"),
            scalar_value(daemon, "pid"),
            bool_value(daemon, "pid_file_exists"),
            bool_value(daemon, "stale_pid_file"),
            scalar_value(daemon, "stale_pid")
        ),
    ];

    if let Some(reason) = json_string_field(daemon, "last_stop_reason")
        && !reason.trim().is_empty()
    {
        lines.push(format!("last_stop_reason={}", compact_line(&reason, 240)));
    }

    if let Some(ledger) = ledger {
        let total_records = scalar_value(ledger, "total_records");
        let success_count = scalar_value(ledger, "success_count");
        let runtime_tokens = scalar_value(ledger, "runtime_tokens_total");
        let feedback_total = scalar_value(ledger, "feedback_applied_total");
        let ready = readiness
            .map(|value| bool_value(value, "ready"))
            .unwrap_or("unknown");
        lines.push(format!(
            "ledger records={total_records} success={success_count}/{total_records} runtime_tokens={runtime_tokens} feedback={feedback_total} ready={ready}"
        ));
        lines.push(format!(
            "ledger hygiene duplicate_rounds={} round_gaps={} invalid_records={}",
            scalar_value(ledger, "duplicate_rounds"),
            scalar_value(ledger, "round_gaps"),
            scalar_value(ledger, "invalid_records")
        ));

        if let Some(latest) = json_object_field(ledger, "latest") {
            let latest_round = scalar_value(latest, "round");
            lines.push(format!(
                "latest round={} case={} success={} runtime_tokens={} feedback={}",
                latest_round,
                string_value(latest, "case"),
                bool_value(latest, "success"),
                scalar_value(latest, "runtime_tokens"),
                scalar_value(latest, "feedback_applied")
            ));
        }

        if !report_exists
            && let Some(ledger_path) = json_string_field(ledger, "path")
            && let Ok(Some(record)) = last_non_empty_line(&ledger_path)
        {
            lines.extend(latest_model_output_lines(&record));
        }
    }

    if let Some(line) = daemon_log_tail_status_line(daemon, ledger) {
        lines.push(line);
    }
    lines.extend(daemon_round_transition_status_lines(Some(loop_status)));
    lines.extend(context_hygiene_status_lines(Some(loop_status)));
    let mut next_round_lines = next_round_decision_status_lines(Some(loop_status));
    if next_round_lines.is_empty() {
        next_round_lines = next_round_decision_status_lines(Some(status));
    }
    lines.extend(next_round_lines);
    let mut downstream_status_consumer_lines =
        next_round_downstream_status_consumers_lines(Some(loop_status));
    if downstream_status_consumer_lines.is_empty() {
        downstream_status_consumer_lines =
            next_round_downstream_status_consumers_lines(Some(status));
    }
    lines.extend(downstream_status_consumer_lines);

    if let Some(report) = report {
        let report_path = string_value(report, "path");
        lines.push(format!(
            "report exists={} path={}",
            bool_value(report, "exists"),
            report_path
        ));
        if json_bool_field(report, "exists") == Some(true)
            && let Ok(report_json) = fs::read_to_string(&report_path)
        {
            lines.extend(report_detail_lines(&report_json));
            lines.extend(clean_room_handoff_report_lines(&report_json));
            lines.extend(self_improve_proposal_panel_lines(&report_json));
            lines.extend(helper_stage_repair_panel_lines(&report_json));
            let worker_window_report_json =
                super::evolution_worker_window_status::worker_window_replacement_report_json(Some(
                    &report_json,
                ));
            let clean_room_handoff_report_json = clean_room_handoff_report_json(Some(&report_json));
            let self_improve_proposal_panel_json =
                self_improve_proposal_panel_json(Some(&report_json));
            let helper_stage_repair_panel_json = helper_stage_repair_panel_json(Some(&report_json));
            lines.extend(unified_status_lines(
                status,
                &worker_window_report_json,
                &clean_room_handoff_report_json,
                &self_improve_proposal_panel_json,
                &helper_stage_repair_panel_json,
            ));
        }
    }

    lines.extend(candidate_backlog_lines(work_dir));

    if let Some(backend) = backend {
        lines.push(format!(
            "backend checked={} ok={} readiness_ok={} safe_device_ok={} engine_busy={} active_requests={} model={} error={}",
            bool_value(backend, "checked"),
            scalar_value(backend, "ok"),
            scalar_value(backend, "readiness_ok"),
            scalar_value(backend, "safe_device_ok"),
            scalar_value(backend, "engine_busy"),
            scalar_value(backend, "active_engine_requests"),
            string_value(backend, "gemma_runtime_model"),
            compact_line(&string_value(backend, "error"), 160)
        ));
    }

    if let Some(supervisor) = json_object_field(loop_status, "supervisor") {
        lines.push(format!(
            "supervisor running={} pid={} healthy={} error={}",
            bool_value(supervisor, "running"),
            scalar_value(supervisor, "pid"),
            scalar_value(supervisor, "healthy"),
            compact_line(&string_value(supervisor, "error"), 160)
        ));
    }

    if let Some(readiness) = readiness {
        lines.push(format!(
            "readiness ready={} failures={}",
            bool_value(readiness, "ready"),
            list_value(json_string_array_field(readiness, "failures"))
        ));
    }

    if let Some(remote_chain) = remote_chain {
        lines.push(format!(
            "remote_chain checked={} ready={} exists={} error={}",
            bool_value(remote_chain, "checked"),
            scalar_value(remote_chain, "ready"),
            bool_value(remote_chain, "exists"),
            compact_line(&string_value(remote_chain, "error"), 160)
        ));
        if let Some(remote_runtime) = json_object_field(remote_chain, "remote_runtime") {
            lines.push(format!(
                "remote_runtime probed={} touches_remote={} workers={} cpu_or_no_gpu={} cpu_or_no_gpu_roles={} backend_metadata_may_differ_roles={} acceleration_ok={} next_step={} error={}",
                bool_value(remote_runtime, "probed"),
                bool_value(remote_runtime, "touches_remote"),
                scalar_value(remote_runtime, "worker_count"),
                scalar_value(remote_runtime, "cpu_or_no_gpu_count"),
                list_value(json_string_array_field(
                    remote_runtime,
                    "cpu_or_no_gpu_roles"
                )),
                list_value(json_string_array_field(
                    remote_runtime,
                    "backend_metadata_may_differ_roles"
                )),
                bool_value(remote_runtime, "acceleration_ok"),
                compact_line(&string_value(remote_runtime, "acceleration_next_step"), 160),
                compact_line(&string_value(remote_runtime, "error"), 160)
            ));
        }
    }

    if let Some(model_pool) = model_pool {
        lines.push(format!(
            "model_pool available={} launch_allowed={} workers={}/{} min_context_tokens={} reason={}",
            bool_value(model_pool, "available"),
            bool_value(model_pool, "launch_allowed"),
            scalar_value(model_pool, "healthy_worker_count"),
            scalar_value(model_pool, "worker_count"),
            scalar_value(model_pool, "min_context_tokens"),
            string_value(model_pool, "reason")
        ));
    }

    lines.extend(worker_window_status_lines(Some(loop_status)));

    if let Some(line) = daemon_start_gate_line(candidate_summary.as_ref()) {
        lines.push(line);
    }

    let backend_endpoint = json_string_field(loop_status, "backend_endpoint").unwrap_or_default();
    let report_gate_status = read_report_gate_status(Some(loop_status));
    let readiness_status = ReadinessStartStatus::from_loop_status(Some(loop_status));
    lines.extend(unattended_start_plan_lines_with_readiness(
        daemon,
        work_dir,
        &backend_endpoint,
        candidate_summary.as_ref(),
        &report_gate_status,
        remote_chain,
        Some(&readiness_status),
    ));

    let next_step = unattended_start_next_step_with_readiness(
        daemon,
        work_dir,
        &backend_endpoint,
        candidate_summary.as_ref(),
        &report_gate_status,
        remote_chain,
        Some(&readiness_status),
    );
    lines.push(format!("next_step={}", compact_line(&next_step, 240)));

    Ok(lines.join("\n"))
}

fn bool_value(object: &str, field: &str) -> &'static str {
    match json_bool_field(object, field) {
        Some(true) => "true",
        Some(false) => "false",
        None => "unknown",
    }
}

fn string_value(object: &str, field: &str) -> String {
    json_string_field(object, field).unwrap_or_else(|| "unknown".to_owned())
}

fn list_value(values: Option<Vec<String>>) -> String {
    values
        .filter(|values| !values.is_empty())
        .map(|values| values.join(","))
        .unwrap_or_else(|| "none".to_owned())
}

fn last_non_empty_line(path: &str) -> io::Result<Option<String>> {
    let content = fs::read_to_string(path)?;
    Ok(content
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_owned))
}
