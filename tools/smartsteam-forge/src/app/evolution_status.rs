use std::{
    io::{self, Write},
    thread,
    time::Duration,
};

use super::evolution_candidate_status::{
    candidate_backlog_preflight_path, candidate_backlog_status_json,
    candidate_start_preflight_ready, daemon_start_gate_status_json, read_candidate_backlog_summary,
    render_candidate_start_preflight,
};
use super::evolution_daemon_args::{
    EvolutionDaemonAction, EvolutionDaemonStartOptions, evolution_action_name,
};
use super::evolution_daemon_process::{
    invoke_evolution_daemon_control, load_evolution_status_with_backend,
};
use super::evolution_readiness_start_status::{
    readiness_start_preflight_ready, render_readiness_start_preflight,
};
use super::evolution_report_gate_status::{
    read_report_gate_status, render_report_gate_continuation_preflight,
    report_gate_continuation_preflight_ready, report_gate_preflight_json,
};
use super::evolution_start_check_json::{
    StartCheckCandidateSnapshot, StartCheckJsonInput, StartCheckReadinessSnapshot,
    StartCheckReportSnapshot, render_start_check_json,
};
use super::evolution_start_command_preview::build_evolution_start_command_preview;
use super::evolution_status_contract::{
    validate_read_only_enriched_status, validate_read_only_start_check, validate_read_only_status,
};
use super::evolution_status_enriched_json::render_enriched_evolution_status_json;
use super::evolution_status_summary::summarize_evolution_status;
use super::status_json::{bool_value_text, json_object_field};

pub fn run_evolution_status(
    work_dir: &str,
    json_status: bool,
    backend: Option<&str>,
) -> io::Result<()> {
    let mut stdout = io::stdout();
    run_evolution_status_to(work_dir, json_status, backend, &mut stdout)
}

pub fn run_evolution_status_watch(
    work_dir: &str,
    backend: Option<&str>,
    interval: Duration,
    max_iterations: Option<usize>,
) -> io::Result<()> {
    let mut stdout = io::stdout();
    run_evolution_status_watch_to(work_dir, backend, interval, max_iterations, &mut stdout)
}

pub fn run_evolution_daemon_control(
    action: EvolutionDaemonAction,
    work_dir: &str,
    backend: Option<&str>,
    prompt: Option<&str>,
    check_only: bool,
    candidate_backlog_path: Option<&str>,
    start_options: EvolutionDaemonStartOptions,
) -> io::Result<()> {
    let mut stdout = io::stdout();
    run_evolution_daemon_control_to(
        action,
        work_dir,
        backend,
        prompt,
        check_only,
        candidate_backlog_path,
        start_options,
        &mut stdout,
    )
}

pub fn run_evolution_start_check_json(
    work_dir: &str,
    backend: Option<&str>,
    prompt: Option<&str>,
    candidate_backlog_path: Option<&str>,
    start_options: EvolutionDaemonStartOptions,
) -> io::Result<()> {
    let mut stdout = io::stdout();
    run_evolution_start_check_json_to(
        work_dir,
        backend,
        prompt,
        candidate_backlog_path,
        start_options,
        &mut stdout,
    )
}

fn run_evolution_daemon_control_to<W: Write>(
    action: EvolutionDaemonAction,
    work_dir: &str,
    backend: Option<&str>,
    prompt: Option<&str>,
    check_only: bool,
    candidate_backlog_path: Option<&str>,
    start_options: EvolutionDaemonStartOptions,
    output: &mut W,
) -> io::Result<()> {
    if action == EvolutionDaemonAction::Start {
        let preflight = render_candidate_start_preflight(work_dir, candidate_backlog_path)?;
        writeln!(output, "{preflight}")?;
        if !candidate_start_preflight_ready(&preflight) {
            output.flush()?;
            return Err(io::Error::other(format!(
                "candidate lifecycle preflight failed before evolution daemon start: {}",
                compact_multiline(&preflight)
            )));
        }

        let status = load_evolution_status_with_backend(work_dir, backend)?;
        validate_read_only_status(&status)?;
        let report_gate_preflight = render_report_gate_continuation_preflight(&status);
        writeln!(output, "{report_gate_preflight}")?;
        if !report_gate_continuation_preflight_ready(&report_gate_preflight) {
            output.flush()?;
            return Err(io::Error::other(format!(
                "report gate continuation preflight failed before evolution daemon start: {}",
                compact_multiline(&report_gate_preflight)
            )));
        }

        let readiness_preflight = render_readiness_start_preflight(&status);
        writeln!(output, "{readiness_preflight}")?;
        if !readiness_start_preflight_ready(&readiness_preflight) {
            output.flush()?;
            return Err(io::Error::other(format!(
                "readiness preflight failed before evolution daemon start: {}",
                compact_multiline(&readiness_preflight)
            )));
        }
    }

    let command_output = invoke_evolution_daemon_control(
        action,
        work_dir,
        backend,
        prompt,
        check_only,
        start_options,
    )?;
    let action_name = evolution_action_name(action);
    writeln!(output, "SmartSteam evolution daemon {action_name}")?;
    writeln!(
        output,
        "check_only={} starts_process={} sends_prompt={} stops_process={}",
        bool_value_text(check_only),
        bool_value_text(action == EvolutionDaemonAction::Start && !check_only),
        bool_value_text(action == EvolutionDaemonAction::Start && !check_only),
        bool_value_text(action == EvolutionDaemonAction::Stop && !check_only)
    )?;
    write!(output, "{command_output}")?;
    if !command_output.ends_with('\n') {
        writeln!(output)?;
    }
    output.flush()
}

fn run_evolution_start_check_json_to<W: Write>(
    work_dir: &str,
    backend: Option<&str>,
    prompt: Option<&str>,
    candidate_backlog_path: Option<&str>,
    start_options: EvolutionDaemonStartOptions,
    output: &mut W,
) -> io::Result<()> {
    let candidate_preflight = render_candidate_start_preflight(work_dir, candidate_backlog_path)?;
    let candidate_preflight_ready = candidate_start_preflight_ready(&candidate_preflight);
    let candidate_path = candidate_backlog_preflight_path(work_dir, candidate_backlog_path);
    let candidate_summary = read_candidate_backlog_summary(&candidate_path)?;
    let candidate_backlog_json =
        candidate_backlog_status_json(&candidate_path, candidate_summary.as_ref());
    let daemon_start_gate_json = daemon_start_gate_status_json(candidate_summary.as_ref());
    let status = load_evolution_status_with_backend(work_dir, backend)?;
    validate_read_only_status(&status)?;
    let report_gate_preflight = render_report_gate_continuation_preflight(&status);
    let report_gate_preflight_ready =
        report_gate_continuation_preflight_ready(&report_gate_preflight);
    let readiness_preflight = render_readiness_start_preflight(&status);
    let readiness_preflight_ready = readiness_start_preflight_ready(&readiness_preflight);
    let command_preview =
        build_evolution_start_command_preview(&status, work_dir, backend, prompt, start_options)?;
    let report_gate_status = read_report_gate_status(json_object_field(&status, "loop"));
    let report_gate_status_json = report_gate_status.to_json();
    let report_gate_start_gate_json = report_gate_preflight_json(
        &report_gate_status,
        work_dir,
        &command_preview.effective_backend,
    );

    let start_check_json = render_start_check_json(StartCheckJsonInput {
        status: &status,
        work_dir,
        backend,
        candidate: StartCheckCandidateSnapshot {
            preflight: &candidate_preflight,
            preflight_ready: candidate_preflight_ready,
            backlog_json: &candidate_backlog_json,
            start_gate_json: &daemon_start_gate_json,
        },
        report: StartCheckReportSnapshot {
            preflight: &report_gate_preflight,
            preflight_ready: report_gate_preflight_ready,
            status_json: &report_gate_status_json,
            start_gate_json: &report_gate_start_gate_json,
        },
        readiness: StartCheckReadinessSnapshot {
            preflight: &readiness_preflight,
            preflight_ready: readiness_preflight_ready,
        },
        command_preview: &command_preview,
        start_options,
    });
    validate_read_only_start_check(&start_check_json)?;

    writeln!(output, "{start_check_json}")?;
    output.flush()
}

fn run_evolution_status_to<W: Write>(
    work_dir: &str,
    json_status: bool,
    backend: Option<&str>,
    output: &mut W,
) -> io::Result<()> {
    let status = load_evolution_status_with_backend(work_dir, backend)?;
    validate_read_only_status(&status)?;

    if json_status {
        let enriched_status = render_enriched_evolution_status_json(&status, work_dir)?;
        validate_read_only_enriched_status(&enriched_status)?;
        writeln!(output, "{enriched_status}")?;
    } else {
        writeln!(output, "{}", summarize_evolution_status(&status, work_dir)?)?;
    }
    output.flush()
}

fn run_evolution_status_watch_to<W: Write>(
    work_dir: &str,
    backend: Option<&str>,
    interval: Duration,
    max_iterations: Option<usize>,
    output: &mut W,
) -> io::Result<()> {
    let mut iteration = 0usize;
    loop {
        iteration = iteration.saturating_add(1);
        writeln!(output, "evolution_watch iteration={iteration}")?;
        match load_evolution_status_with_backend(work_dir, backend).and_then(|status| {
            validate_read_only_status(&status)?;
            summarize_evolution_status(&status, work_dir)
        }) {
            Ok(summary) => writeln!(output, "{summary}")?,
            Err(error) => writeln!(
                output,
                "evolution_watch_error iteration={iteration} error={error}"
            )?,
        }
        output.flush()?;

        if max_iterations.is_some_and(|limit| iteration >= limit) {
            return Ok(());
        }
        if !interval.is_zero() {
            thread::sleep(interval);
        }
    }
}

fn compact_multiline(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" | ")
}
