use std::io;

use super::status_json::{
    json_bool_field, json_null_field, json_object_array_field, json_object_field,
    json_string_field, json_top_level_bool_field, json_top_level_number_field,
    json_top_level_string_array_field, json_top_level_string_field,
};

pub(super) fn validate_read_only_status(status: &str) -> io::Result<()> {
    let daemon = json_object_field(status, "daemon")
        .ok_or_else(|| io::Error::other("evolution status missing daemon object"))?;
    let loop_status = json_object_field(status, "loop")
        .ok_or_else(|| io::Error::other("evolution status missing loop object"))?;

    validate_contract_bool(daemon, "daemon", "read_only", true)?;
    validate_contract_bool(daemon, "daemon", "starts_process", false)?;
    validate_contract_bool(daemon, "daemon", "sends_prompt", false)?;
    validate_contract_bool(loop_status, "loop", "read_only", true)?;
    validate_contract_bool(loop_status, "loop", "starts_process", false)?;
    validate_contract_bool(loop_status, "loop", "sends_prompt", false)?;

    if let Some(touches_remote) = json_bool_field(loop_status, "touches_remote")
        && touches_remote
    {
        return Err(io::Error::other(
            "evolution status loop contract unexpectedly touches remote",
        ));
    }

    Ok(())
}

pub(super) fn validate_read_only_enriched_status(status: &str) -> io::Result<()> {
    match json_top_level_string_field(status, "schema").as_deref() {
        Some("smartsteam.forge.evolution_status.v1") => {}
        Some(schema) => {
            return Err(io::Error::other(format!(
                "evolution enriched status unexpected schema={schema}"
            )));
        }
        None => return Err(io::Error::other("evolution enriched status missing schema")),
    }

    validate_top_level_contract_bool(status, "enriched_status", "read_only", true)?;
    validate_top_level_contract_bool(status, "enriched_status", "starts_process", false)?;
    validate_top_level_contract_bool(status, "enriched_status", "sends_prompt", false)?;

    let evolution_status = json_object_field(status, "evolution_status")
        .ok_or_else(|| io::Error::other("evolution enriched status missing evolution_status"))?;
    validate_read_only_status(evolution_status)?;

    for section in [
        "daemon_log_tail",
        "daemon_round_transition_status",
        "context_hygiene_status",
        "next_round_decision_status",
        "next_round_downstream_status_consumers",
        "report_gate_status",
        "report_gate_preflight",
        "report_gate_start_gate",
        "candidate_backlog",
        "daemon_start_gate",
        "readiness_start_gate",
        "worker_window_status",
        "worker_window_replacement_report",
        "clean_room_handoff_report",
        "self_improve_proposal_panel",
        "helper_stage_repair_panel",
        "unified_status",
        "unattended_start_plan",
    ] {
        validate_read_only_object(status, "evolution enriched status", section)?;
    }
    let worker_window_status =
        json_object_field(status, "worker_window_status").ok_or_else(|| {
            io::Error::other("evolution enriched status missing worker_window_status object")
        })?;
    let daemon_round_transition_status =
        json_object_field(status, "daemon_round_transition_status").ok_or_else(|| {
            io::Error::other(
                "evolution enriched status missing daemon_round_transition_status object",
            )
        })?;
    let next_round_decision_status = json_object_field(status, "next_round_decision_status")
        .ok_or_else(|| {
            io::Error::other("evolution enriched status missing next_round_decision_status object")
        })?;
    let next_round_downstream_status_consumers =
        json_object_field(status, "next_round_downstream_status_consumers").ok_or_else(|| {
            io::Error::other(
                "evolution enriched status missing next_round_downstream_status_consumers object",
            )
        })?;
    for field in [
        "starts_daemon",
        "stops_daemon",
        "touches_remote",
        "sends_prompt",
        "starts_stream",
        "replays_prompt",
        "mutates_active_round",
        "writes_ndkv",
    ] {
        validate_contract_bool(
            daemon_round_transition_status,
            "daemon_round_transition_status",
            field,
            false,
        )?;
    }
    for field in [
        "starts_daemon",
        "stops_daemon",
        "touches_remote",
        "sends_prompt",
        "starts_stream",
        "replays_prompt",
        "writes_ndkv",
    ] {
        validate_contract_bool(
            next_round_decision_status,
            "next_round_decision_status",
            field,
            false,
        )?;
    }
    for field in [
        "side_effects",
        "starts_daemon",
        "stops_daemon",
        "touches_remote",
        "starts_stream",
        "replays_prompt",
        "writes_ndkv",
    ] {
        validate_contract_bool(
            next_round_downstream_status_consumers,
            "next_round_downstream_status_consumers",
            field,
            false,
        )?;
    }
    validate_contract_bool(
        worker_window_status,
        "worker_window_status",
        "starts_clean_room_replacement",
        false,
    )?;
    validate_contract_bool(
        worker_window_status,
        "worker_window_status",
        "mutates_worker_window_status",
        false,
    )?;
    let worker_window_replacement_report =
        json_object_field(status, "worker_window_replacement_report").ok_or_else(|| {
            io::Error::other(
                "evolution enriched status missing worker_window_replacement_report object",
            )
        })?;
    validate_contract_bool(
        worker_window_replacement_report,
        "worker_window_replacement_report",
        "starts_clean_room_replacement",
        false,
    )?;
    validate_contract_bool(
        worker_window_replacement_report,
        "worker_window_replacement_report",
        "mutates_worker_window_status",
        false,
    )?;
    let unified_status = json_object_field(status, "unified_status").ok_or_else(|| {
        io::Error::other("evolution enriched status missing unified_status object")
    })?;
    for field in [
        "starts_daemon",
        "stops_daemon",
        "touches_remote",
        "downloads_model",
        "warms_model_cache",
        "starts_stream",
        "replays_prompt",
    ] {
        validate_contract_bool(unified_status, "unified_status", field, false)?;
    }
    validate_nested_read_only_object(unified_status, "unified_status", "memory_startup_admission")?;
    validate_nested_read_only_object(unified_status, "unified_status", "clean_room_handoff")?;
    validate_nested_read_only_object(unified_status, "unified_status", "self_improve_proposal")?;
    validate_nested_read_only_object(unified_status, "unified_status", "helper_stage_repair")?;
    if let Some(unified_self_improve) = json_object_field(unified_status, "self_improve_proposal")
        && json_object_field(unified_self_improve, "action_assignment").is_some()
    {
        validate_nested_read_only_object(
            unified_self_improve,
            "unified_status.self_improve_proposal",
            "action_assignment",
        )?;
        let action_assignment =
            json_object_field(unified_self_improve, "action_assignment").expect("checked above");
        validate_contract_bool(
            action_assignment,
            "unified_status.self_improve_proposal.action_assignment",
            "report_only",
            true,
        )?;
        validate_contract_bool(
            action_assignment,
            "unified_status.self_improve_proposal.action_assignment",
            "auto_apply",
            false,
        )?;
    }
    let clean_room_handoff_report = json_object_field(status, "clean_room_handoff_report")
        .ok_or_else(|| {
            io::Error::other("evolution enriched status missing clean_room_handoff_report object")
        })?;
    validate_clean_room_handoff_contract(clean_room_handoff_report)?;
    let self_improve_proposal_panel = json_object_field(status, "self_improve_proposal_panel")
        .ok_or_else(|| {
            io::Error::other("evolution enriched status missing self_improve_proposal_panel object")
        })?;
    validate_self_improve_proposal_panel_contract(self_improve_proposal_panel)?;
    let helper_stage_repair_panel = json_object_field(status, "helper_stage_repair_panel")
        .ok_or_else(|| {
            io::Error::other("evolution enriched status missing helper_stage_repair_panel object")
        })?;
    validate_helper_stage_repair_panel_contract(helper_stage_repair_panel)?;
    let start_plan = json_object_field(status, "unattended_start_plan").ok_or_else(|| {
        io::Error::other("evolution enriched status missing unattended_start_plan object")
    })?;
    validate_unattended_start_plan_consistency(start_plan)?;
    validate_enriched_next_step_matches_start_plan(status, start_plan)?;

    Ok(())
}

pub(super) fn validate_read_only_start_check(start_check: &str) -> io::Result<()> {
    match json_top_level_string_field(start_check, "schema").as_deref() {
        Some("smartsteam.forge.evolution_start_check.v1") => {}
        Some(schema) => {
            return Err(io::Error::other(format!(
                "evolution start check unexpected schema={schema}"
            )));
        }
        None => return Err(io::Error::other("evolution start check missing schema")),
    }
    match json_top_level_string_field(start_check, "action").as_deref() {
        Some("start") => {}
        Some(action) => {
            return Err(io::Error::other(format!(
                "evolution start check unexpected action={action}"
            )));
        }
        None => return Err(io::Error::other("evolution start check missing action")),
    }

    validate_top_level_contract_bool(start_check, "start_check", "read_only", true)?;
    validate_top_level_contract_bool(start_check, "start_check", "starts_process", false)?;
    validate_top_level_contract_bool(start_check, "start_check", "sends_prompt", false)?;
    validate_top_level_contract_bool(start_check, "start_check", "check_only", true)?;
    validate_top_level_contract_string(
        start_check,
        "start_check",
        "preview_source",
        "rust_pure_preview",
    )?;
    let command_output = json_top_level_string_field(start_check, "command_output")
        .ok_or_else(|| io::Error::other("evolution start check missing command_output"))?;
    let command_preview = json_top_level_string_field(start_check, "command_preview")
        .ok_or_else(|| io::Error::other("evolution start check missing command_preview"))?;
    validate_contract_text_contains(
        &command_output,
        "start_check.command_output",
        "check_only=true",
    )?;
    validate_contract_text_contains(
        &command_output,
        "start_check.command_output",
        "starts_process=false",
    )?;
    validate_contract_text_contains(
        &command_output,
        "start_check.command_output",
        "sends_prompt=false",
    )?;
    validate_start_check_backend_consistency(start_check, &command_preview)?;
    validate_start_check_command_preview_consistency(&command_output, &command_preview)?;
    validate_start_check_runtime_context_consistency(start_check, &command_output)?;

    for section in [
        "candidate_backlog",
        "daemon_start_gate",
        "report_gate_status",
        "report_gate_start_gate",
        "readiness_start_gate",
    ] {
        validate_read_only_object(start_check, "evolution start check", section)?;
    }
    validate_start_check_gate_consistency(start_check)?;

    Ok(())
}

fn validate_start_check_backend_consistency(
    start_check: &str,
    command_preview: &str,
) -> io::Result<()> {
    let effective_backend = json_top_level_string_field(start_check, "effective_backend")
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| io::Error::other("evolution start check missing effective_backend"))?;
    if let Some(requested_backend) = json_top_level_string_field(start_check, "backend")
        && requested_backend != effective_backend
    {
        return Err(io::Error::other(format!(
            "evolution status start_check.backend mismatch: expected {effective_backend:?}, got {requested_backend:?}"
        )));
    }
    let backend_arg = format!("-Backend {effective_backend}");
    if !command_preview.contains(&backend_arg) {
        return Err(io::Error::other(format!(
            "evolution status start_check.command_preview missing backend argument {backend_arg:?}"
        )));
    }

    Ok(())
}

fn validate_start_check_runtime_context_consistency(
    start_check: &str,
    command_output: &str,
) -> io::Result<()> {
    let min_runtime_context = json_top_level_number_field(start_check, "min_runtime_context")
        .ok_or_else(|| io::Error::other("evolution start check missing min_runtime_context"))?;
    let min_runtime_context_value = min_runtime_context.parse::<u64>().map_err(|_| {
        io::Error::other(format!(
            "evolution status start_check.min_runtime_context is not u64: {min_runtime_context}"
        ))
    })?;
    if min_runtime_context_value == 0 {
        return Err(io::Error::other(
            "evolution status start_check.min_runtime_context must be positive",
        ));
    }
    let min_runtime_context_source =
        json_top_level_string_field(start_check, "min_runtime_context_source").ok_or_else(
            || io::Error::other("evolution start check missing min_runtime_context_source"),
        )?;
    if min_runtime_context_source.trim().is_empty() {
        return Err(io::Error::other(
            "evolution status start_check.min_runtime_context_source is empty",
        ));
    }
    let output_context = command_output_line_value(command_output, "min_runtime_context=")
        .ok_or_else(|| {
            io::Error::other(
                "evolution status missing start_check.command_output min_runtime_context line",
            )
        })?;
    if output_context != min_runtime_context {
        return Err(io::Error::other(format!(
            "evolution status start_check.min_runtime_context mismatch: expected {min_runtime_context}, got {output_context}"
        )));
    }
    let output_source =
        command_output_line_value(command_output, "min_runtime_context_source=").ok_or_else(
            || {
                io::Error::other(
                    "evolution status missing start_check.command_output min_runtime_context_source line",
                )
            },
        )?;
    if output_source != min_runtime_context_source {
        return Err(io::Error::other(format!(
            "evolution status start_check.min_runtime_context_source mismatch: expected {min_runtime_context_source:?}, got {output_source:?}"
        )));
    }

    Ok(())
}

fn validate_start_check_command_preview_consistency(
    command_output: &str,
    command_preview: &str,
) -> io::Result<()> {
    if command_preview.trim().is_empty() {
        return Err(io::Error::other(
            "evolution status start_check.command_preview is empty",
        ));
    }
    let output_command =
        command_output_line_value(command_output, "command=").ok_or_else(|| {
            io::Error::other("evolution status missing start_check.command_output command line")
        })?;
    if output_command != command_preview {
        return Err(io::Error::other(format!(
            "evolution status start_check.command_output command mismatch: expected {:?}, got {:?}",
            command_preview, output_command
        )));
    }

    Ok(())
}

fn command_output_line_value<'a>(command_output: &'a str, prefix: &str) -> Option<&'a str> {
    command_output
        .lines()
        .find_map(|line| line.strip_prefix(prefix))
}

fn validate_start_check_gate_consistency(start_check: &str) -> io::Result<()> {
    let daemon_running =
        read_top_level_contract_bool(start_check, "start_check", "daemon_running")?;
    let candidate_ready =
        read_top_level_contract_bool(start_check, "start_check", "candidate_preflight_ready")?;
    let report_gate_ready =
        read_top_level_contract_bool(start_check, "start_check", "report_gate_preflight_ready")?;
    let readiness_ready =
        read_top_level_contract_bool(start_check, "start_check", "readiness_preflight_ready")?;
    let can_start = read_top_level_contract_bool(start_check, "start_check", "can_start")?;
    let current_state = json_top_level_string_field(start_check, "current_state")
        .ok_or_else(|| io::Error::other("evolution status missing start_check.current_state"))?;
    let block_reasons = json_top_level_string_array_field(start_check, "block_reasons")
        .ok_or_else(|| io::Error::other("evolution status missing start_check.block_reasons"))?;
    let expected_reasons = expected_start_check_block_reasons(
        daemon_running,
        candidate_ready,
        report_gate_ready,
        readiness_ready,
    );
    if block_reasons != expected_reasons {
        return Err(io::Error::other(format!(
            "evolution status start_check.block_reasons mismatch: expected {:?}, got {:?}",
            expected_reasons, block_reasons
        )));
    }
    let expected_can_start = expected_reasons.is_empty();
    if can_start != expected_can_start {
        return Err(io::Error::other(format!(
            "evolution status start_check.can_start={can_start}, expected {expected_can_start}"
        )));
    }
    let expected_current_state = if daemon_running {
        "running"
    } else if expected_can_start {
        "not_running_ready_to_start"
    } else {
        "not_running_blocked"
    };
    if current_state != expected_current_state {
        return Err(io::Error::other(format!(
            "evolution status start_check.current_state={current_state}, expected {expected_current_state}"
        )));
    }

    Ok(())
}

fn expected_start_check_block_reasons(
    daemon_running: bool,
    candidate_ready: bool,
    report_gate_ready: bool,
    readiness_ready: bool,
) -> Vec<String> {
    let mut reasons = Vec::new();
    if daemon_running {
        reasons.push("already_running".to_owned());
    }
    if !candidate_ready {
        reasons.push("candidate_backlog_not_ready".to_owned());
    }
    if !report_gate_ready {
        reasons.push("report_gate_not_ready".to_owned());
    }
    if !readiness_ready {
        reasons.push("readiness_not_ready".to_owned());
    }
    reasons
}

fn validate_unattended_start_plan_consistency(plan: &str) -> io::Result<()> {
    let candidate_ready =
        read_contract_bool(plan, "unattended_start_plan", "candidate_lifecycle_ready")?;
    let readiness_ready =
        read_contract_bool(plan, "unattended_start_plan", "readiness_start_ready")?;
    let report_blocks = read_contract_bool(
        plan,
        "unattended_start_plan",
        "report_gate_blocks_continuation",
    )?;
    let can_start = read_contract_bool(plan, "unattended_start_plan", "can_start")?;
    let current_state = json_string_field(plan, "current_state").ok_or_else(|| {
        io::Error::other("evolution status missing unattended_start_plan.current_state")
    })?;
    let block_reason =
        read_optional_contract_string(plan, "unattended_start_plan", "block_reason")?;
    let continuation_block_reason =
        read_optional_contract_string(plan, "unattended_start_plan", "continuation_block_reason")?;
    let expected_block_reason = expected_unattended_start_plan_block_reason(
        candidate_ready,
        readiness_ready,
        report_blocks,
        continuation_block_reason.as_deref(),
        current_state.as_str(),
    )?;
    if block_reason.as_deref() != expected_block_reason {
        return Err(io::Error::other(format!(
            "evolution status unattended_start_plan.block_reason mismatch: expected {:?}, got {:?}",
            expected_block_reason, block_reason
        )));
    }
    let expected_can_start = expected_block_reason.is_none();
    if can_start != expected_can_start {
        return Err(io::Error::other(format!(
            "evolution status unattended_start_plan.can_start={can_start}, expected {expected_can_start}"
        )));
    }
    let expected_current_state = if expected_block_reason == Some("already_running") {
        "running"
    } else if expected_can_start {
        "not_running_ready_to_start"
    } else {
        "not_running_blocked"
    };
    if current_state != expected_current_state {
        return Err(io::Error::other(format!(
            "evolution status unattended_start_plan.current_state={current_state}, expected {expected_current_state}"
        )));
    }

    Ok(())
}

fn expected_unattended_start_plan_block_reason<'a>(
    candidate_ready: bool,
    readiness_ready: bool,
    report_blocks: bool,
    continuation_block_reason: Option<&'a str>,
    current_state: &'a str,
) -> io::Result<Option<&'a str>> {
    if !candidate_ready {
        return Ok(Some("candidate_backlog_not_ready"));
    }
    if !readiness_ready {
        return Ok(Some("readiness_not_ready"));
    }
    if report_blocks {
        return Ok(Some(
            continuation_block_reason.unwrap_or("report_gate_not_ready"),
        ));
    }
    if current_state == "running" {
        return Ok(Some("already_running"));
    }
    Ok(None)
}

fn validate_enriched_next_step_matches_start_plan(status: &str, plan: &str) -> io::Result<()> {
    let actual_next_step = json_top_level_string_field(status, "next_step")
        .ok_or_else(|| io::Error::other("evolution enriched status missing top-level next_step"))?;
    let expected_next_step = expected_unattended_next_step(plan)?;
    if actual_next_step != expected_next_step {
        return Err(io::Error::other(format!(
            "evolution status next_step mismatch: expected {:?}, got {:?}",
            expected_next_step, actual_next_step
        )));
    }

    Ok(())
}

fn expected_unattended_next_step(plan: &str) -> io::Result<String> {
    let can_start = read_contract_bool(plan, "unattended_start_plan", "can_start")?;
    let current_state = json_string_field(plan, "current_state").ok_or_else(|| {
        io::Error::other("evolution status missing unattended_start_plan.current_state")
    })?;
    let block_reason =
        read_optional_contract_string(plan, "unattended_start_plan", "block_reason")?;

    if current_state == "running" {
        return Ok("running: monitor JsonStatus; duplicate unattended start is blocked".to_owned());
    }
    if can_start {
        let check_only_command =
            json_string_field(plan, "check_only_command").ok_or_else(|| {
                io::Error::other(
                    "evolution status missing unattended_start_plan.check_only_command",
                )
            })?;
        return Ok(format!(
            "ready_to_start: run StartCheck before Start: {check_only_command}"
        ));
    }

    match block_reason.as_deref() {
        Some("candidate_backlog_not_ready") => {
            Ok("blocked: resolve candidate backlog before unattended evolution".to_owned())
        }
        Some("report_gate_not_ready") => {
            Ok("blocked: fix report gate before unattended evolution".to_owned())
        }
        Some("readiness_not_ready") => {
            let failures = json_string_field(plan, "readiness_blocking_failures")
                .filter(|value| {
                    let trimmed = value.trim();
                    !trimmed.is_empty() && trimmed != "none"
                })
                .unwrap_or_else(|| "unknown".to_owned());
            Ok(format!(
                "blocked: fix readiness before unattended evolution ({failures})"
            ))
        }
        Some(reason) => Ok(format!(
            "blocked: inspect unattended_start_plan reason={reason}"
        )),
        None => Ok("blocked: inspect unattended_start_plan".to_owned()),
    }
}

fn validate_read_only_object(object: &str, owner: &str, section: &str) -> io::Result<()> {
    let nested = json_object_field(object, section)
        .ok_or_else(|| io::Error::other(format!("{owner} missing {section} object")))?;
    validate_contract_bool(nested, section, "read_only", true)?;
    validate_contract_bool(nested, section, "starts_process", false)?;
    validate_contract_bool(nested, section, "sends_prompt", false)?;
    Ok(())
}

fn validate_nested_read_only_object(object: &str, owner: &str, section: &str) -> io::Result<()> {
    validate_read_only_object(object, owner, section)
}

fn validate_clean_room_handoff_contract(report: &str) -> io::Result<()> {
    for field in ["starts_process", "sends_prompt"] {
        validate_contract_bool(report, "clean_room_handoff_report", field, false)?;
    }

    let side_effects = json_object_field(report, "side_effects").ok_or_else(|| {
        io::Error::other("evolution enriched status missing clean_room_handoff_report.side_effects")
    })?;
    for field in [
        "starts_clean_room_replacement",
        "mutates_worker_window_status",
        "starts_daemon",
        "stops_daemon",
        "touches_remote",
        "downloads_model",
        "warms_model_cache",
        "sends_prompt",
        "starts_stream",
        "replays_prompt",
        "starts_thread",
        "sends_message",
        "mutates_memory_store",
        "writes_ndkv",
    ] {
        validate_contract_bool(
            side_effects,
            "clean_room_handoff_report.side_effects",
            field,
            false,
        )?;
    }

    validate_nested_read_only_object(report, "clean_room_handoff_report", "memory_admission")?;
    validate_nested_read_only_object(report, "clean_room_handoff_report", "agent_replacement")?;
    validate_contract_bool(report, "clean_room_handoff_report", "report_only", true)?;

    Ok(())
}

fn validate_self_improve_proposal_panel_contract(panel: &str) -> io::Result<()> {
    for field in ["starts_process", "sends_prompt"] {
        validate_contract_bool(panel, "self_improve_proposal_panel", field, false)?;
    }

    for section in [
        "candidate",
        "validated",
        "admitted",
        "quarantined",
        "promoted",
        "repair_required",
    ] {
        validate_nested_read_only_object(panel, "self_improve_proposal_panel", section)?;
    }

    let side_effects = json_object_field(panel, "side_effects").ok_or_else(|| {
        io::Error::other(
            "evolution enriched status missing self_improve_proposal_panel.side_effects",
        )
    })?;
    for field in [
        "starts_daemon",
        "stops_daemon",
        "starts_process",
        "touches_remote",
        "downloads_model",
        "warms_model_cache",
        "sends_prompt",
        "starts_stream",
        "replays_prompt",
        "starts_thread",
        "sends_message",
        "mutates_memory_store",
        "writes_ndkv",
        "promotes_candidate",
        "repairs_artifact",
    ] {
        validate_contract_bool(
            side_effects,
            "self_improve_proposal_panel.side_effects",
            field,
            false,
        )?;
    }
    validate_contract_bool(panel, "self_improve_proposal_panel", "report_only", true)?;
    if json_object_field(panel, "prompt_guidance").is_some() {
        validate_nested_read_only_object(panel, "self_improve_proposal_panel", "prompt_guidance")?;
    }
    if json_object_field(panel, "action_plan").is_some() {
        validate_nested_read_only_object(panel, "self_improve_proposal_panel", "action_plan")?;
        let action_plan = json_object_field(panel, "action_plan").expect("checked above");
        validate_contract_bool(
            action_plan,
            "self_improve_proposal_panel.action_plan",
            "report_only",
            true,
        )?;
        validate_contract_bool(
            action_plan,
            "self_improve_proposal_panel.action_plan",
            "auto_apply",
            false,
        )?;
    }
    if json_object_field(panel, "action_assignment").is_some() {
        validate_nested_read_only_object(
            panel,
            "self_improve_proposal_panel",
            "action_assignment",
        )?;
        let action_assignment =
            json_object_field(panel, "action_assignment").expect("checked above");
        validate_contract_bool(
            action_assignment,
            "self_improve_proposal_panel.action_assignment",
            "report_only",
            true,
        )?;
        validate_contract_bool(
            action_assignment,
            "self_improve_proposal_panel.action_assignment",
            "auto_apply",
            false,
        )?;
    }

    Ok(())
}

fn validate_helper_stage_repair_panel_contract(panel: &str) -> io::Result<()> {
    for field in ["starts_process", "sends_prompt"] {
        validate_contract_bool(panel, "helper_stage_repair_panel", field, false)?;
    }
    validate_contract_bool(panel, "helper_stage_repair_panel", "report_only", true)?;
    validate_contract_bool(panel, "helper_stage_repair_panel", "auto_apply", false)?;

    let side_effects = json_object_field(panel, "side_effects").ok_or_else(|| {
        io::Error::other("evolution enriched status missing helper_stage_repair_panel.side_effects")
    })?;
    validate_helper_stage_repair_side_effects(
        side_effects,
        "helper_stage_repair_panel.side_effects",
    )?;

    for proposal in json_object_array_field(panel, "proposals").unwrap_or_default() {
        validate_contract_bool(
            proposal,
            "helper_stage_repair_panel.proposal",
            "read_only",
            true,
        )?;
        validate_contract_bool(
            proposal,
            "helper_stage_repair_panel.proposal",
            "starts_process",
            false,
        )?;
        validate_contract_bool(
            proposal,
            "helper_stage_repair_panel.proposal",
            "sends_prompt",
            false,
        )?;
        validate_contract_bool(
            proposal,
            "helper_stage_repair_panel.proposal",
            "auto_apply",
            false,
        )?;
        validate_contract_bool(
            proposal,
            "helper_stage_repair_panel.proposal",
            "candidate_only",
            true,
        )?;
        let proposal_side_effects = json_object_field(proposal, "side_effects").ok_or_else(|| {
            io::Error::other(
                "evolution enriched status missing helper_stage_repair_panel.proposal.side_effects",
            )
        })?;
        validate_helper_stage_repair_side_effects(
            proposal_side_effects,
            "helper_stage_repair_panel.proposal.side_effects",
        )?;
    }

    Ok(())
}

fn validate_helper_stage_repair_side_effects(side_effects: &str, label: &str) -> io::Result<()> {
    for field in [
        "applies_code",
        "edits_files",
        "mutates_ledger",
        "mutates_memory_store",
        "writes_ndkv",
        "starts_daemon",
        "stops_daemon",
        "touches_remote",
        "downloads_model",
        "warms_model_cache",
        "starts_forge",
        "starts_web_lab",
        "sends_prompt",
        "starts_stream",
        "replays_prompt",
        "calls_model",
    ] {
        validate_contract_bool(side_effects, label, field, false)?;
    }

    Ok(())
}

fn read_contract_bool(object: &str, section: &str, field: &str) -> io::Result<bool> {
    json_bool_field(object, field)
        .ok_or_else(|| io::Error::other(format!("evolution status missing {section}.{field}")))
}

fn read_top_level_contract_bool(object: &str, section: &str, field: &str) -> io::Result<bool> {
    json_top_level_bool_field(object, field)
        .ok_or_else(|| io::Error::other(format!("evolution status missing {section}.{field}")))
}

fn read_optional_contract_string(
    object: &str,
    section: &str,
    field: &str,
) -> io::Result<Option<String>> {
    if let Some(value) = json_string_field(object, field) {
        return Ok(Some(value));
    }
    if json_null_field(object, field).is_some() {
        return Ok(None);
    }
    Err(io::Error::other(format!(
        "evolution status missing {section}.{field}"
    )))
}

fn validate_top_level_contract_string(
    object: &str,
    section: &str,
    field: &str,
    expected: &str,
) -> io::Result<()> {
    match json_top_level_string_field(object, field) {
        Some(actual) if actual == expected => Ok(()),
        Some(actual) => Err(io::Error::other(format!(
            "evolution status unsafe {section}.{field}={actual}, expected {expected}"
        ))),
        None => Err(io::Error::other(format!(
            "evolution status missing {section}.{field}"
        ))),
    }
}

fn validate_contract_text_contains(text: &str, section: &str, marker: &str) -> io::Result<()> {
    text.contains(marker).then_some(()).ok_or_else(|| {
        io::Error::other(format!(
            "evolution status missing {section} marker {marker}"
        ))
    })
}

fn validate_top_level_contract_bool(
    object: &str,
    section: &str,
    field: &str,
    expected: bool,
) -> io::Result<()> {
    match json_top_level_bool_field(object, field) {
        Some(actual) if actual == expected => Ok(()),
        Some(actual) => Err(io::Error::other(format!(
            "evolution status unsafe {section}.{field}={actual}, expected {expected}"
        ))),
        None => Err(io::Error::other(format!(
            "evolution status missing {section}.{field}"
        ))),
    }
}

fn validate_contract_bool(
    object: &str,
    section: &str,
    field: &str,
    expected: bool,
) -> io::Result<()> {
    match json_bool_field(object, field) {
        Some(actual) if actual == expected => Ok(()),
        Some(actual) => Err(io::Error::other(format!(
            "evolution status unsafe {section}.{field}={actual}, expected {expected}"
        ))),
        None => Err(io::Error::other(format!(
            "evolution status missing {section}.{field}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const READ_ONLY_STATUS: &str = r#"{
        "daemon": {"read_only": true, "starts_process": false, "sends_prompt": false},
        "loop": {"read_only": true, "starts_process": false, "sends_prompt": false, "touches_remote": false}
    }"#;
    const READ_ONLY_SECTION: &str =
        r#"{"read_only": true, "starts_process": false, "sends_prompt": false}"#;
    const READ_ONLY_START_PLAN: &str = r#"{"read_only": true, "starts_process": false, "sends_prompt": false, "candidate_lifecycle_ready": true, "readiness_start_ready": true, "readiness_blocking_failures": "none", "can_start": true, "current_state": "not_running_ready_to_start", "block_reason": null, "report_gate_blocks_continuation": false, "continuation_block_reason": null, "check_only_command": ".\\tools\\smartsteam-forge\\evolution-daemon.cmd -StartCheck -WorkDir target"}"#;
    const READY_NEXT_STEP: &str = "ready_to_start: run StartCheck before Start: .\\\\tools\\\\smartsteam-forge\\\\evolution-daemon.cmd -StartCheck -WorkDir target";

    #[test]
    fn accepts_safe_read_only_status_contract() {
        validate_read_only_status(READ_ONLY_STATUS).unwrap();
    }

    #[test]
    fn rejects_daemon_status_that_would_start_processes() {
        let unsafe_status =
            READ_ONLY_STATUS.replace("\"starts_process\": false", "\"starts_process\": true");

        let error = validate_read_only_status(&unsafe_status).unwrap_err();

        assert!(error.to_string().contains("daemon.starts_process=true"));
    }

    #[test]
    fn rejects_loop_status_that_would_send_prompt() {
        let unsafe_status = r#"{
            "daemon": {"read_only": true, "starts_process": false, "sends_prompt": false},
            "loop": {"read_only": true, "starts_process": false, "sends_prompt": true, "touches_remote": false}
        }"#;

        let error = validate_read_only_status(unsafe_status).unwrap_err();

        assert!(error.to_string().contains("loop.sends_prompt=true"));
    }

    #[test]
    fn rejects_loop_status_that_touches_remote() {
        let unsafe_status =
            READ_ONLY_STATUS.replace("\"touches_remote\": false", "\"touches_remote\": true");

        let error = validate_read_only_status(&unsafe_status).unwrap_err();

        assert!(error.to_string().contains("touches remote"));
    }

    #[test]
    fn rejects_status_missing_required_contract_bool() {
        let unsafe_status = r#"{
            "daemon": {"read_only": true, "starts_process": false, "sends_prompt": false},
            "loop": {"read_only": true, "sends_prompt": false, "touches_remote": false}
        }"#;

        let error = validate_read_only_status(unsafe_status).unwrap_err();

        assert!(error.to_string().contains("missing loop.starts_process"));
    }

    #[test]
    fn accepts_safe_read_only_enriched_status_contract() {
        validate_read_only_enriched_status(&read_only_enriched_status()).unwrap();
    }

    #[test]
    fn rejects_enriched_status_that_would_start_processes() {
        let unsafe_status = read_only_enriched_status().replacen(
            "\"starts_process\": false",
            "\"starts_process\": true",
            1,
        );

        let error = validate_read_only_enriched_status(&unsafe_status).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("enriched_status.starts_process=true")
        );
    }

    #[test]
    fn rejects_enriched_status_missing_top_level_schema_even_with_nested_schema() {
        let unsafe_status = read_only_enriched_status()
            .replace(
                "                \"schema\": \"smartsteam.forge.evolution_status.v1\",\n",
                "",
            )
            .replace(
                "\"candidate_backlog\": {\"read_only\": true",
                "\"candidate_backlog\": {\"schema\":\"smartsteam.forge.evolution_status.v1\", \"read_only\": true",
            );

        let error = validate_read_only_enriched_status(&unsafe_status).unwrap_err();

        assert!(error.to_string().contains("missing schema"));
    }

    #[test]
    fn rejects_enriched_status_with_unsafe_nested_gate() {
        let unsafe_status = read_only_enriched_status().replace(
            "\"readiness_start_gate\": {\"read_only\": true, \"starts_process\": false",
            "\"readiness_start_gate\": {\"read_only\": true, \"starts_process\": true",
        );

        let error = validate_read_only_enriched_status(&unsafe_status).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("readiness_start_gate.starts_process=true")
        );
    }

    #[test]
    fn rejects_enriched_status_with_unsafe_embedded_status() {
        let unsafe_status = read_only_enriched_status()
            .replace("\"touches_remote\": false", "\"touches_remote\": true");

        let error = validate_read_only_enriched_status(&unsafe_status).unwrap_err();

        assert!(error.to_string().contains("touches remote"));
    }

    #[test]
    fn rejects_enriched_status_with_unsafe_candidate_backlog() {
        let unsafe_status = read_only_enriched_status().replace(
            "\"candidate_backlog\": {\"read_only\": true, \"starts_process\": false",
            "\"candidate_backlog\": {\"read_only\": true, \"starts_process\": true",
        );

        let error = validate_read_only_enriched_status(&unsafe_status).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("candidate_backlog.starts_process=true")
        );
    }

    #[test]
    fn rejects_enriched_clean_room_handoff_that_would_write_ndkv() {
        let unsafe_status = read_only_enriched_status().replace(
            "\"sends_message\": false, \"mutates_memory_store\": false, \"writes_ndkv\": false",
            "\"sends_message\": false, \"mutates_memory_store\": false, \"writes_ndkv\": true",
        );

        let error = validate_read_only_enriched_status(&unsafe_status).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("clean_room_handoff_report.side_effects.writes_ndkv=true")
        );
    }

    #[test]
    fn rejects_enriched_daemon_round_transition_that_would_write_ndkv() {
        let unsafe_status = read_only_enriched_status().replace(
            "\"daemon_round_transition_status\": {\"read_only\": true, \"starts_process\": false, \"sends_prompt\": false, \"report_only\": true, \"observed_round_done\": false, \"latest_round_state\": null, \"round_in_progress\": false, \"active_round\": \"unknown\", \"done_round\": \"unknown\", \"ledger_round\": \"unknown\", \"ledger_commit_pending\": false, \"ledger_lag_rounds\": \"unknown\", \"status\": null, \"activity_reason\": null, \"evidence_ids\": [], \"reason_codes\": [], \"starts_daemon\": false, \"stops_daemon\": false, \"touches_remote\": false, \"starts_stream\": false, \"replays_prompt\": false, \"mutates_active_round\": false, \"writes_ndkv\": false}",
            "\"daemon_round_transition_status\": {\"read_only\": true, \"starts_process\": false, \"sends_prompt\": false, \"report_only\": true, \"observed_round_done\": false, \"latest_round_state\": null, \"round_in_progress\": false, \"active_round\": \"unknown\", \"done_round\": \"unknown\", \"ledger_round\": \"unknown\", \"ledger_commit_pending\": false, \"ledger_lag_rounds\": \"unknown\", \"status\": null, \"activity_reason\": null, \"evidence_ids\": [], \"reason_codes\": [], \"starts_daemon\": false, \"stops_daemon\": false, \"touches_remote\": false, \"starts_stream\": false, \"replays_prompt\": false, \"mutates_active_round\": false, \"writes_ndkv\": true}",
        );

        let error = validate_read_only_enriched_status(&unsafe_status).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("daemon_round_transition_status.writes_ndkv=true")
        );
    }

    #[test]
    fn rejects_enriched_self_improve_proposal_that_would_start_stream() {
        let unsafe_status = read_only_enriched_status().replace(
            "\"starts_stream\": false, \"replays_prompt\": false, \"starts_thread\": false, \"sends_message\": false, \"mutates_memory_store\": false, \"writes_ndkv\": false, \"promotes_candidate\": false",
            "\"starts_stream\": true, \"replays_prompt\": false, \"starts_thread\": false, \"sends_message\": false, \"mutates_memory_store\": false, \"writes_ndkv\": false, \"promotes_candidate\": false",
        );

        let error = validate_read_only_enriched_status(&unsafe_status).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("self_improve_proposal_panel.side_effects.starts_stream=true")
        );
    }

    #[test]
    fn rejects_enriched_self_improve_proposal_that_would_replay_prompt() {
        let unsafe_status = read_only_enriched_status().replace(
            "\"starts_stream\": false, \"replays_prompt\": false, \"starts_thread\": false, \"sends_message\": false, \"mutates_memory_store\": false, \"writes_ndkv\": false, \"promotes_candidate\": false",
            "\"starts_stream\": false, \"replays_prompt\": true, \"starts_thread\": false, \"sends_message\": false, \"mutates_memory_store\": false, \"writes_ndkv\": false, \"promotes_candidate\": false",
        );

        let error = validate_read_only_enriched_status(&unsafe_status).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("self_improve_proposal_panel.side_effects.replays_prompt=true")
        );
    }

    #[test]
    fn rejects_enriched_self_improve_proposal_action_plan_that_would_auto_apply() {
        let unsafe_status = read_only_enriched_status().replace(
            "\"source\": null, \"candidate_count\": 0",
            "\"source\": null, \"action_plan\": {\"read_only\": true, \"starts_process\": false, \"sends_prompt\": false, \"status_loaded\": true, \"report_only\": true, \"safe\": true, \"action_required\": true, \"primary_action\": \"apply_now\", \"actions\": [\"apply_now\"], \"requires_validation_and_memory_admission\": false, \"auto_apply\": true}, \"candidate_count\": 0",
        );

        let error = validate_read_only_enriched_status(&unsafe_status).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("self_improve_proposal_panel.action_plan.auto_apply=true")
        );
    }

    #[test]
    fn rejects_enriched_helper_stage_repair_that_would_start_web_lab() {
        let unsafe_status = read_only_enriched_status().replace(
            "\"starts_forge\": false, \"starts_web_lab\": false, \"sends_prompt\": false",
            "\"starts_forge\": false, \"starts_web_lab\": true, \"sends_prompt\": false",
        );

        let error = validate_read_only_enriched_status(&unsafe_status).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("helper_stage_repair_panel.side_effects.starts_web_lab=true")
        );
    }

    #[test]
    fn rejects_enriched_unattended_start_plan_block_reason_drift() {
        let unsafe_status = read_only_enriched_status().replace(
            "\"candidate_lifecycle_ready\": true",
            "\"candidate_lifecycle_ready\": false",
        );

        let error = validate_read_only_enriched_status(&unsafe_status).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("unattended_start_plan.block_reason mismatch")
        );
    }

    #[test]
    fn rejects_enriched_unattended_start_plan_can_start_drift() {
        let unsafe_status =
            read_only_enriched_status().replace("\"can_start\": true", "\"can_start\": false");

        let error = validate_read_only_enriched_status(&unsafe_status).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("unattended_start_plan.can_start=false")
        );
    }

    #[test]
    fn rejects_enriched_unattended_start_plan_current_state_drift() {
        let unsafe_status = read_only_enriched_status().replace(
            "\"current_state\": \"not_running_ready_to_start\"",
            "\"current_state\": \"not_running_blocked\"",
        );

        let error = validate_read_only_enriched_status(&unsafe_status).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("unattended_start_plan.current_state=not_running_blocked")
        );
    }

    #[test]
    fn rejects_enriched_next_step_drift_when_plan_blocks_candidate() {
        let unsafe_status = read_only_enriched_status()
            .replace(
                "\"candidate_lifecycle_ready\": true",
                "\"candidate_lifecycle_ready\": false",
            )
            .replace("\"can_start\": true", "\"can_start\": false")
            .replace(
                "\"current_state\": \"not_running_ready_to_start\"",
                "\"current_state\": \"not_running_blocked\"",
            )
            .replace(
                "\"block_reason\": null",
                "\"block_reason\": \"candidate_backlog_not_ready\"",
            );

        let error = validate_read_only_enriched_status(&unsafe_status).unwrap_err();

        assert!(error.to_string().contains("next_step mismatch"));
        assert!(
            error
                .to_string()
                .contains("blocked: resolve candidate backlog")
        );
    }

    #[test]
    fn rejects_enriched_next_step_legacy_ready_text() {
        let unsafe_status = read_only_enriched_status().replace(
            "ready_to_start: run StartCheck before Start: .\\\\tools\\\\smartsteam-forge\\\\evolution-daemon.cmd -StartCheck -WorkDir target",
            "ready: run budgeted -Forever or inspect report gate",
        );

        let error = validate_read_only_enriched_status(&unsafe_status).unwrap_err();

        assert!(error.to_string().contains("next_step mismatch"));
        assert!(error.to_string().contains("ready_to_start"));
    }

    #[test]
    fn accepts_safe_read_only_start_check_contract() {
        let start_check = read_only_start_check();

        validate_read_only_start_check(&start_check).unwrap();
    }

    #[test]
    fn rejects_start_check_that_would_start_processes() {
        let start_check = read_only_start_check().replacen(
            "\"starts_process\": false",
            "\"starts_process\": true",
            1,
        );

        let error = validate_read_only_start_check(&start_check).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("start_check.starts_process=true")
        );
    }

    #[test]
    fn rejects_start_check_missing_top_level_read_only_even_with_nested_read_only() {
        let start_check =
            read_only_start_check().replace("                \"read_only\": true,\n", "");

        let error = validate_read_only_start_check(&start_check).unwrap_err();

        assert!(error.to_string().contains("missing start_check.read_only"));
    }

    #[test]
    fn rejects_start_check_nested_gate_that_would_send_prompt() {
        let start_check = read_only_start_check().replace(
            "\"report_gate_start_gate\": {\"read_only\": true, \"starts_process\": false, \"sends_prompt\": false}",
            "\"report_gate_start_gate\": {\"read_only\": true, \"starts_process\": false, \"sends_prompt\": true}",
        );

        let error = validate_read_only_start_check(&start_check).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("report_gate_start_gate.sends_prompt=true")
        );
    }

    #[test]
    fn rejects_start_check_candidate_gate_that_would_start_processes() {
        let start_check = read_only_start_check().replace(
            "\"daemon_start_gate\": {\"read_only\": true, \"starts_process\": false, \"sends_prompt\": false}",
            "\"daemon_start_gate\": {\"read_only\": true, \"starts_process\": true, \"sends_prompt\": false}",
        );

        let error = validate_read_only_start_check(&start_check).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("daemon_start_gate.starts_process=true")
        );
    }

    #[test]
    fn rejects_start_check_candidate_backlog_that_would_start_processes() {
        let start_check = read_only_start_check().replace(
            "\"candidate_backlog\": {\"read_only\": true, \"starts_process\": false, \"sends_prompt\": false}",
            "\"candidate_backlog\": {\"read_only\": true, \"starts_process\": true, \"sends_prompt\": false}",
        );

        let error = validate_read_only_start_check(&start_check).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("candidate_backlog.starts_process=true")
        );
    }

    #[test]
    fn rejects_start_check_non_pure_preview_source() {
        let start_check = read_only_start_check().replace(
            "\"preview_source\": \"rust_pure_preview\"",
            "\"preview_source\": \"script_check_only\"",
        );

        let error = validate_read_only_start_check(&start_check).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("start_check.preview_source=script_check_only")
        );
    }

    #[test]
    fn rejects_start_check_command_output_missing_safe_marker() {
        let start_check = read_only_start_check().replace(
            "\"command_output\": \"check_only=true\\nstarts_process=false\\nsends_prompt=false\\nmin_runtime_context=262144\\nmin_runtime_context_source=fallback\\ncommand=powershell.exe -NoProfile -File start.ps1 -Backend 127.0.0.1:7979\"",
            "\"command_output\": \"check_only=true\\nstarts_process=true\\nsends_prompt=false\\nmin_runtime_context=262144\\nmin_runtime_context_source=fallback\\ncommand=powershell.exe -NoProfile -File start.ps1 -Backend 127.0.0.1:7979\"",
        );

        let error = validate_read_only_start_check(&start_check).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("start_check.command_output marker starts_process=false")
        );
    }

    #[test]
    fn rejects_start_check_min_runtime_context_drift() {
        let start_check = read_only_start_check().replace(
            "\"min_runtime_context\": 262144",
            "\"min_runtime_context\": 65536",
        );

        let error = validate_read_only_start_check(&start_check).unwrap_err();

        assert!(error.to_string().contains("min_runtime_context mismatch"));
    }

    #[test]
    fn rejects_start_check_min_runtime_context_source_drift() {
        let start_check = read_only_start_check().replace(
            "\"min_runtime_context_source\": \"fallback\"",
            "\"min_runtime_context_source\": \"status_model_pool\"",
        );

        let error = validate_read_only_start_check(&start_check).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("min_runtime_context_source mismatch")
        );
    }

    #[test]
    fn rejects_start_check_effective_backend_missing_from_command_preview() {
        let start_check = read_only_start_check().replace(" -Backend 127.0.0.1:7979", "");

        let error = validate_read_only_start_check(&start_check).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("command_preview missing backend argument")
        );
    }

    #[test]
    fn rejects_start_check_requested_backend_drift() {
        let start_check = read_only_start_check().replace(
            "\"backend\": \"127.0.0.1:7979\"",
            "\"backend\": \"127.0.0.1:7878\"",
        );

        let error = validate_read_only_start_check(&start_check).unwrap_err();

        assert!(error.to_string().contains("start_check.backend mismatch"));
    }

    #[test]
    fn rejects_start_check_missing_command_preview() {
        let start_check = read_only_start_check().replace(
            "                \"command_preview\": \"powershell.exe -NoProfile -File start.ps1 -Backend 127.0.0.1:7979\",\n",
            "",
        );

        let error = validate_read_only_start_check(&start_check).unwrap_err();

        assert!(error.to_string().contains("missing command_preview"));
    }

    #[test]
    fn rejects_start_check_command_output_command_drift() {
        let start_check = read_only_start_check().replace(
            "command=powershell.exe -NoProfile -File start.ps1",
            "command=powershell.exe -NoProfile -File other.ps1",
        );

        let error = validate_read_only_start_check(&start_check).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("command_output command mismatch")
        );
    }

    #[test]
    fn rejects_start_check_block_reason_drift() {
        let start_check = read_only_start_check().replace(
            "\"candidate_preflight_ready\": true",
            "\"candidate_preflight_ready\": false",
        );

        let error = validate_read_only_start_check(&start_check).unwrap_err();

        assert!(error.to_string().contains("block_reasons mismatch"));
    }

    #[test]
    fn rejects_start_check_can_start_drift() {
        let start_check =
            read_only_start_check().replace("\"can_start\": true", "\"can_start\": false");

        let error = validate_read_only_start_check(&start_check).unwrap_err();

        assert!(error.to_string().contains("start_check.can_start=false"));
    }

    #[test]
    fn rejects_start_check_current_state_drift() {
        let start_check = read_only_start_check().replace(
            "\"current_state\": \"not_running_ready_to_start\"",
            "\"current_state\": \"running\"",
        );

        let error = validate_read_only_start_check(&start_check).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("start_check.current_state=running")
        );
    }

    fn read_only_enriched_status() -> String {
        format!(
            r#"{{
                "schema": "smartsteam.forge.evolution_status.v1",
                "read_only": true,
                "starts_process": false,
                "sends_prompt": false,
                "evolution_status": {READ_ONLY_STATUS},
                "daemon_log_tail": {READ_ONLY_SECTION},
                "daemon_round_transition_status": {{"read_only": true, "starts_process": false, "sends_prompt": false, "report_only": true, "observed_round_done": false, "latest_round_state": null, "round_in_progress": false, "active_round": "unknown", "done_round": "unknown", "ledger_round": "unknown", "ledger_commit_pending": false, "ledger_lag_rounds": "unknown", "status": null, "activity_reason": null, "evidence_ids": [], "reason_codes": [], "starts_daemon": false, "stops_daemon": false, "touches_remote": false, "starts_stream": false, "replays_prompt": false, "mutates_active_round": false, "writes_ndkv": false}},
                "context_hygiene_status": {{"read_only": true, "starts_process": false, "sends_prompt": false, "report_only": true, "completed_window_evidence_non_actionable": false, "future_work_requires_fresh_clean_room": false, "reads_old_window_payload": false, "reason_codes": []}},
                "next_round_decision_status": {{"read_only": true, "starts_process": false, "sends_prompt": false, "report_only": true, "decision": null, "starts_daemon": false, "stops_daemon": false, "touches_remote": false, "starts_stream": false, "replays_prompt": false, "writes_ndkv": false, "active_round": "unknown", "done_round": "unknown", "ledger_round": "unknown", "reason_codes": [], "evidence_ids": []}},
                "next_round_downstream_status_consumers": {{"read_only": true, "starts_process": false, "sends_prompt": false, "report_only": true, "side_effects": false, "starts_daemon": false, "stops_daemon": false, "touches_remote": false, "starts_stream": false, "replays_prompt": false, "writes_ndkv": false, "active_round": "unknown", "done_round": "unknown", "ledger_round": "unknown", "round_id_evidence": {{"read_only": true, "starts_process": false, "sends_prompt": false, "active_round": "unknown", "done_round": "unknown", "ledger_round": "unknown", "transition_kind": null, "reason_codes": [], "evidence_ids": []}}, "reason_codes": [], "evidence_ids": [], "consumers": []}},
                "report_gate_status": {READ_ONLY_SECTION},
                "report_gate_preflight": {READ_ONLY_SECTION},
                "report_gate_start_gate": {READ_ONLY_SECTION},
                "candidate_backlog": {READ_ONLY_SECTION},
                "daemon_start_gate": {READ_ONLY_SECTION},
                "readiness_start_gate": {READ_ONLY_SECTION},
                "worker_window_status": {{"read_only": true, "starts_process": false, "sends_prompt": false, "starts_clean_room_replacement": false, "mutates_worker_window_status": false, "total": 0, "clean_room_replacement_required_count": 0, "status_counts": "none", "rows": []}},
                "worker_window_replacement_report": {{"read_only": true, "starts_process": false, "sends_prompt": false, "status_loaded": false, "source": null, "source_path": null, "side_effects_allowed": null, "starts_clean_room_replacement": false, "mutates_worker_window_status": false, "window_count": 0, "paused_count": 0, "polluted_count": 0, "clean_room_replacement_count": 0, "replacement_required_count": 0, "blocked_original_count": 0, "rows": []}},
                "clean_room_handoff_report": {{"read_only": true, "starts_process": false, "sends_prompt": false, "status_loaded": false, "report_only": true, "safe": true, "source": null, "memory_admission": {{"read_only": true, "starts_process": false, "sends_prompt": false, "status_loaded": false, "safe": true}}, "agent_replacement": {{"read_only": true, "starts_process": false, "sends_prompt": false, "status_loaded": false, "safe": true}}, "side_effects": {{"starts_clean_room_replacement": false, "mutates_worker_window_status": false, "starts_daemon": false, "stops_daemon": false, "touches_remote": false, "downloads_model": false, "warms_model_cache": false, "sends_prompt": false, "starts_stream": false, "replays_prompt": false, "starts_thread": false, "sends_message": false, "mutates_memory_store": false, "writes_ndkv": false}}}},
                "self_improve_proposal_panel": {{"read_only": true, "starts_process": false, "sends_prompt": false, "status_loaded": false, "report_only": true, "safe": true, "source": null, "candidate_count": 0, "validated_count": 0, "admitted_count": 0, "quarantined_count": 0, "promoted_count": 0, "repair_required_count": 0, "candidate": {{"read_only": true, "starts_process": false, "sends_prompt": false, "count": 0, "ids": [], "reason_codes": []}}, "validated": {{"read_only": true, "starts_process": false, "sends_prompt": false, "count": 0, "ids": [], "reason_codes": []}}, "admitted": {{"read_only": true, "starts_process": false, "sends_prompt": false, "count": 0, "ids": [], "reason_codes": []}}, "quarantined": {{"read_only": true, "starts_process": false, "sends_prompt": false, "count": 0, "ids": [], "reason_codes": []}}, "promoted": {{"read_only": true, "starts_process": false, "sends_prompt": false, "count": 0, "ids": [], "reason_codes": []}}, "repair_required": {{"read_only": true, "starts_process": false, "sends_prompt": false, "count": 0, "ids": [], "reason_codes": []}}, "side_effects": {{"starts_daemon": false, "stops_daemon": false, "starts_process": false, "touches_remote": false, "downloads_model": false, "warms_model_cache": false, "sends_prompt": false, "starts_stream": false, "replays_prompt": false, "starts_thread": false, "sends_message": false, "mutates_memory_store": false, "writes_ndkv": false, "promotes_candidate": false, "repairs_artifact": false}}}},
                "helper_stage_repair_panel": {{"read_only": true, "starts_process": false, "sends_prompt": false, "status_loaded": false, "report_only": true, "safe": true, "source": null, "latest_round": null, "repair_required": false, "total_role_count": 0, "incomplete_role_count": 0, "missing_helper_role_repair_required": false, "missing_helper_role_repair_proposal_count": 0, "missing_helper_roles": [], "proposal_count": 0, "roles": [], "proposal_ids": [], "missing_fields": [], "placeholder_fields": [], "validation_safe": true, "candidate_only": true, "auto_apply": false, "proposals": [], "side_effects": {{"applies_code": false, "edits_files": false, "mutates_ledger": false, "mutates_memory_store": false, "writes_ndkv": false, "starts_daemon": false, "stops_daemon": false, "touches_remote": false, "downloads_model": false, "warms_model_cache": false, "starts_forge": false, "starts_web_lab": false, "sends_prompt": false, "starts_stream": false, "replays_prompt": false, "calls_model": false}}}},
                "unified_status": {{"read_only": true, "starts_process": false, "sends_prompt": false, "starts_daemon": false, "stops_daemon": false, "touches_remote": false, "downloads_model": false, "warms_model_cache": false, "starts_stream": false, "replays_prompt": false, "daemon_healthy": false, "supervisor_healthy": false, "model_pool_healthy": false, "worker_replacement_required": false, "memory_admission_safe": true, "no_live_write": true, "no_ndkv_write": true, "clean_room_handoff_loaded": false, "clean_room_handoff_safe": true, "self_improve_proposal_loaded": false, "self_improve_proposal_safe": true, "helper_stage_repair_loaded": false, "helper_stage_repair_safe": true, "helper_stage_repair_required": false, "memory_startup_admission": {{"read_only": true, "starts_process": false, "sends_prompt": false, "status_loaded": false, "safe": true, "read_only_contract": true, "live_store_mutation_requested": false, "store_mutation_count": 0, "ndkv_write_allowed": false, "admission_expanded_by_non_contract_evidence": false, "no_live_write": true, "no_ndkv_write": true}}, "clean_room_handoff": {{"read_only": true, "starts_process": false, "sends_prompt": false, "status_loaded": false, "report_only": true, "safe": true, "memory_admission_safe": true, "agent_replacement_safe": true, "starts_clean_room_replacement": false, "mutates_worker_window_status": false, "mutates_memory_store": false, "writes_ndkv": false}}, "self_improve_proposal": {{"read_only": true, "starts_process": false, "sends_prompt": false, "status_loaded": false, "report_only": true, "safe": true, "candidate_count": 0, "validated_count": 0, "admitted_count": 0, "quarantined_count": 0, "promoted_count": 0, "repair_required_count": 0, "starts_daemon": false, "starts_stream": false, "replays_prompt": false}}, "helper_stage_repair": {{"read_only": true, "starts_process": false, "sends_prompt": false, "status_loaded": false, "report_only": true, "safe": true, "repair_required": false, "proposal_count": 0, "incomplete_role_count": 0, "missing_helper_role_repair_required": false, "missing_helper_role_repair_proposal_count": 0, "missing_helper_roles": [], "roles": [], "starts_daemon": false, "starts_forge": false, "starts_web_lab": false, "calls_model": false, "starts_stream": false, "replays_prompt": false, "auto_apply": false, "writes_ndkv": false, "mutates_memory_store": false}}}},
                "unattended_start_plan": {READ_ONLY_START_PLAN},
                "next_step": "{READY_NEXT_STEP}"
            }}"#
        )
    }

    fn read_only_start_check() -> String {
        format!(
            r#"{{
                "schema": "smartsteam.forge.evolution_start_check.v1",
                "read_only": true,
                "starts_process": false,
                "sends_prompt": false,
                "check_only": true,
                "action": "start",
                "candidate_preflight_ready": true,
                "report_gate_preflight_ready": true,
                "readiness_preflight_ready": true,
                "daemon_running": false,
                "can_start": true,
                "current_state": "not_running_ready_to_start",
                "block_reasons": [],
                "backend": "127.0.0.1:7979",
                "effective_backend": "127.0.0.1:7979",
                "preview_source": "rust_pure_preview",
                "command_preview": "powershell.exe -NoProfile -File start.ps1 -Backend 127.0.0.1:7979",
                "command_output": "check_only=true\nstarts_process=false\nsends_prompt=false\nmin_runtime_context=262144\nmin_runtime_context_source=fallback\ncommand=powershell.exe -NoProfile -File start.ps1 -Backend 127.0.0.1:7979",
                "min_runtime_context": 262144,
                "min_runtime_context_source": "fallback",
                "candidate_backlog": {READ_ONLY_SECTION},
                "daemon_start_gate": {READ_ONLY_SECTION},
                "report_gate_status": {READ_ONLY_SECTION},
                "report_gate_start_gate": {READ_ONLY_SECTION},
                "readiness_start_gate": {READ_ONLY_SECTION}
            }}"#
        )
    }
}
