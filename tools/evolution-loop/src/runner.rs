use std::fs;
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use norion_eval::{ContextRotGate, ContextRotSignal};

use crate::args::Config;
use crate::http;
use crate::json::{
    json_array_field, json_bool_field, json_f64_field, json_object_field, json_string,
    json_string_array, json_string_field, json_u64_field, parse_json_object_array, preview_text,
};
use crate::ledger::{RoundRecord, append_record, next_round, read_ledger_hygiene};
use crate::pool_artifacts;
use crate::pool_dispatch;
use crate::pool_lease;
use crate::pool_request::PoolRequestPlan;
use crate::pool_stage::{self, PoolStageDispatchPlan};
use crate::pool_stage_call::{self, PoolStageCallInput, PoolStageCallResult};
use crate::prompts::{
    approximate_prompt_tokens, load_base_prompts, prompt_with_current_context_limited,
};
use crate::remote_chain;
use crate::report;
use crate::validation;

const HEALTH_GATE_METADATA_ATTEMPTS: usize = 6;
const HEALTH_GATE_METADATA_RETRY_SECS: u64 = 3;
const RUNTIME_REQUEST_REPAIR_FACTOR: &str = "runtime_request_splice";
const RUNTIME_REPAIR_TIMEOUT_REASON: &str = "evolution_loop_stream_timeout";
const RUNTIME_REPAIR_STREAM_ERROR_REASON: &str = "evolution_loop_stream_error";
const RUNTIME_REPAIR_HTTP_TIMEOUT_SECS: u64 = 10;
const MAX_POOL_STAGE_CALL_ANSWER_PREVIEW_CHARS: usize =
    crate::helper_feedback::MAX_HELPER_STAGE_FEEDBACK_CHARS;

#[derive(Debug, Clone, PartialEq)]
struct RoundOutcome {
    success: bool,
    error: Option<String>,
    runtime_tokens: Option<u64>,
    runtime_model: Option<String>,
    answer: Option<String>,
    elapsed_ms: Option<u64>,
    business_cycle_passed: Option<bool>,
    feedback_applied: Option<u64>,
    rust_check_checked: Option<bool>,
    rust_check_passed: Option<bool>,
    rust_check_feedback_applied: Option<u64>,
    self_improve_passed: Option<bool>,
    state_gate_checked: Option<bool>,
    state_gate_passed: Option<bool>,
    trace_gate_checked: Option<bool>,
    trace_gate_passed: Option<bool>,
    delta_chars: usize,
    stages: Vec<String>,
    meta: Vec<String>,
    final_json: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ValidationCommandPlan {
    command: String,
    source: &'static str,
    safety: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ValidationGateEvidence {
    phase: String,
    command_source: String,
    command_safety: String,
    command_preview: String,
    status_code: Option<i32>,
    elapsed_ms: u64,
    stdout_tail: String,
    stderr_tail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ValidationGateFailure {
    message: String,
    evidence: Option<ValidationGateEvidence>,
}

impl ValidationGateEvidence {
    fn meta(&self) -> String {
        format!(
            "validation_gate phase={} source={} safety={} status={} elapsed_ms={} stdout_tail={} stderr_tail={}",
            self.phase,
            self.command_source,
            self.command_safety,
            option_i32_text(self.status_code),
            self.elapsed_ms,
            dash_if_empty(&self.stdout_tail),
            dash_if_empty(&self.stderr_tail)
        )
    }
}

pub(crate) fn run(config: Config) -> Result<(), String> {
    let base_prompts = load_base_prompts(&config)?;
    let rust_check_code = load_rust_check_code(&config)?;
    println!("SmartSteam evolution-loop");
    println!("backend: {}", config.backend);
    println!("ledger: {}", config.ledger_path.display());
    println!(
        "rounds: {}",
        config
            .rounds
            .map(|rounds| rounds.to_string())
            .unwrap_or_else(|| "forever".to_owned())
    );
    println!(
        "max_tokens: {} self_improve_limit: {}",
        config.max_tokens, config.self_improve_limit
    );
    println!(
        "budgets: max_total_tokens={} max_runtime_secs={} max_no_feedback_rounds={}",
        option_u64_text(config.max_total_tokens),
        option_u64_text(config.max_runtime_secs),
        option_usize_text(config.max_no_feedback_rounds)
    );
    println!(
        "rust_check: {}",
        if rust_check_code.is_some() {
            "enabled"
        } else {
            "disabled"
        }
    );
    if let Some(command) = &config.validation_command {
        println!(
            "validation_gate: enabled phase={} timeout_secs={} workdir={} command={}",
            validation_phase_text(config.validation_phase),
            config.validation_timeout_secs,
            config
                .validation_workdir
                .as_deref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| ".".to_owned()),
            preview_text(command, 120)
        );
    } else if config.use_test_gate_validation_command {
        println!(
            "validation_gate: enabled phase={} timeout_secs={} source=test-gate-safe-command",
            validation_phase_text(config.validation_phase),
            config.validation_timeout_secs
        );
    }
    if config.state_consistency_gate {
        println!("state_consistency_gate: enabled");
    }
    if config.require_pool_route {
        println!("pool_route_gate: enabled");
    }
    if config.remote_chain_gate {
        let Some(path) = &config.remote_chain_status_json_path else {
            return Err("--remote-chain-gate requires --remote-chain-status-json".to_owned());
        };
        println!("remote_chain_gate: enabled path={}", path.display());
    }
    if config.pool_capacity_gate {
        let Some(path) = &config.pool_status_json_path else {
            return Err("--pool-capacity-gate requires --pool-status-json".to_owned());
        };
        println!("pool_capacity_gate: enabled path={}", path.display());
    }
    if config.pool_alignment_gate {
        let Some(manifest_path) = &config.pool_manifest_json_path else {
            return Err("--pool-alignment-gate requires --pool-manifest-json".to_owned());
        };
        let Some(status_path) = &config.pool_status_json_path else {
            return Err("--pool-alignment-gate requires --pool-status-json".to_owned());
        };
        let Some(route_path) = &config.pool_route_json_path else {
            return Err("--pool-alignment-gate requires --pool-route-json".to_owned());
        };
        println!(
            "pool_alignment_gate: enabled manifest={} status={} route={} stage_task_kinds={}",
            manifest_path.display(),
            status_path.display(),
            route_path.display(),
            pool_stage::task_kinds_text(&config.pool_stage_route_task_kinds)
        );
    }
    if config.pool_budget_fairness_gate {
        let Some(path) = &config.pool_budget_fairness_json_path else {
            return Err(
                "--pool-budget-fairness-gate requires --pool-budget-fairness-json".to_owned(),
            );
        };
        println!("pool_budget_fairness_gate: enabled path={}", path.display());
    }
    if config.pool_stage_route_gate {
        if config.pool_stage_route_task_kinds.is_empty() {
            return Err(
                "--pool-stage-route-gate requires --pool-stage-route-task-kinds".to_owned(),
            );
        }
        println!(
            "pool_stage_route_gate: enabled task_kinds={}",
            pool_stage::task_kinds_text(&config.pool_stage_route_task_kinds)
        );
    }
    if config.execute_pool_stage_calls {
        if config.pool_stage_route_task_kinds.is_empty() {
            return Err(
                "--execute-pool-stage-calls requires --pool-stage-route-task-kinds".to_owned(),
            );
        }
        println!(
            "pool_stage_calls: enabled task_kinds={} endpoint=/v1/model-pool/call",
            pool_stage::task_kinds_text(&config.pool_stage_route_task_kinds)
        );
    }
    if config.refresh_pool_artifacts {
        println!(
            "pool_artifact_refresh: enabled task_kind={} stage_task_kinds={} manifest_json={} status_json={} route_json={}",
            config.pool_route_task_kind,
            pool_stage::task_kinds_text(&config.pool_stage_route_task_kinds),
            config
                .pool_manifest_json_path
                .as_deref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "none".to_owned()),
            config
                .pool_status_json_path
                .as_deref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "none".to_owned()),
            config
                .pool_route_json_path
                .as_deref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "none".to_owned())
        );
    }
    if let Some(dir) = &config.pool_lease_dir {
        if !config.require_pool_route {
            return Err("--pool-lease-dir requires --require-pool-route".to_owned());
        }
        println!(
            "pool_lease: enabled dir={} ttl_secs={} wait_secs={} poll_secs={} busy_policy={:?}",
            dir.display(),
            config.pool_lease_ttl_secs,
            config.pool_lease_wait_secs,
            config.pool_lease_poll_secs,
            config.pool_lease_busy_policy
        );
    }
    let start_round = next_round(&config.ledger_path)?;
    println!("next_round: {start_round}");

    let mut completed = 0_usize;
    let mut consecutive_failures = 0_usize;
    let mut consecutive_pool_lease_skips = 0_usize;
    let mut budget = BudgetState::default();
    loop {
        if let Some(rounds) = config.rounds
            && completed >= rounds
        {
            println!("completed {completed} round(s)");
            return Ok(());
        }
        if let Some(reason) = pre_round_budget_stop_reason(&config, &budget) {
            println!("stopping: {reason}");
            return Ok(());
        }

        if config.remote_chain_gate {
            run_remote_chain_gate(&config)?;
        }
        if config.refresh_pool_artifacts {
            refresh_pool_artifacts(&config)?;
        }
        if config.pool_capacity_gate {
            run_pool_capacity_gate(&config)?;
        }
        if config.pool_stage_route_gate {
            run_pool_stage_route_gate(&config)?;
        }
        if config.pool_alignment_gate {
            run_pool_alignment_gate(&config)?;
        }
        if config.pool_budget_fairness_gate {
            run_pool_budget_fairness_gate(&config)?;
        }
        if config.state_consistency_gate {
            run_state_consistency_gate(&config)?;
        }
        let dispatch_decision = pool_dispatch::preflight(&config)?;
        if let Some(decision) = &dispatch_decision {
            println!(
                "pool_route_gate: passed selected_role={} port={} endpoint={} context_window={} default_max_tokens={} low_priority={} evidence={}",
                decision.selected_role,
                option_u64_text(decision.selected_port),
                decision.selected_base_url.as_deref().unwrap_or("none"),
                option_u64_text(decision.context_window),
                option_u64_text(decision.default_max_tokens),
                decision.can_accept_low_priority_task,
                decision.evidence
            );
        }
        let validation_plan = effective_validation_command(&config)?;
        let mut validation_evidence = None::<ValidationGateEvidence>;
        let validation_checked = validation_plan.is_some().then_some(true);
        let mut validation_passed = validation_plan.is_some().then_some(true);
        let mut pre_round_stages = Vec::new();
        let mut pre_round_meta = Vec::new();
        if let Some(validation_plan) = &validation_plan
            && config.validation_phase.runs_pre()
        {
            pre_round_stages.push("validation:pre:done".to_owned());
            let evidence = run_validation_gate(&config, "pre", validation_plan)
                .map_err(|failure| failure.message)?;
            pre_round_meta.push(evidence.meta());
            validation_evidence = Some(evidence);
        }
        if config.require_health {
            match backend_health_action(&config)? {
                HealthAction::Ready => {}
                HealthAction::Wait(reason) => {
                    println!(
                        "backend {reason}; waiting {}s before retrying health gate",
                        config.busy_wait_secs
                    );
                    thread::sleep(Duration::from_secs(config.busy_wait_secs));
                    continue;
                }
            }
        }
        if config.experience_audit_gate {
            run_experience_audit_gate(&config)?;
        }
        let allocation_evidence = load_allocation_evidence(&config)?;

        let round = start_round + completed;
        let case_name = format!("{}-{round:04}", config.case_prefix);
        let started_unix = unix_seconds();
        let pool_lease = if let Some(decision) = &dispatch_decision {
            match pool_lease::acquire(&config, decision, round, &case_name, started_unix)? {
                pool_lease::PoolLeaseAcquire::Disabled => None,
                pool_lease::PoolLeaseAcquire::Acquired(lease) => Some(lease),
                pool_lease::PoolLeaseAcquire::Skipped { reason } => {
                    consecutive_pool_lease_skips += 1;
                    println!("[round {round}] pool_lease: skipped {reason}");
                    if let Some(stop_reason) =
                        pool_lease_skip_stop_reason(&config, consecutive_pool_lease_skips, &reason)
                    {
                        return Err(stop_reason);
                    }
                    if config.busy_wait_secs > 0 {
                        thread::sleep(Duration::from_secs(config.busy_wait_secs));
                    }
                    continue;
                }
            }
        } else {
            None
        };
        let pool_request_plan = dispatch_decision
            .as_ref()
            .map(|decision| PoolRequestPlan::from_decision(&config, decision));
        if let Some(plan) = &pool_request_plan {
            println!("[round {round}] {}", plan.meta());
        }
        let pool_stage_dispatch_plans = pool_stage::dispatch_plans(&config)?;
        for plan in &pool_stage_dispatch_plans {
            println!("[round {round}] {}", plan.meta());
        }
        consecutive_pool_lease_skips = 0;
        let pool_lease_summary = pool_lease.as_ref().map(|lease| lease.summary().to_owned());
        if let Some(summary) = pool_lease_summary.as_deref() {
            println!("[round {round}] pool_lease: acquired {summary}");
        }
        let base_prompt = &base_prompts[(round - 1) % base_prompts.len()];
        let prompt_context_limit = pool_prompt_context_char_limit(pool_request_plan.as_ref());
        println!("[round {round}] stage prompt_context:start");
        let prompt =
            prompt_with_current_context_limited(&config, base_prompt, prompt_context_limit)?;
        println!("[round {round}] stage prompt_context:done");
        println!();
        println!("[round {round}] case={case_name}");
        println!("[round {round}] prompt={}", preview_text(&prompt, 160));
        if let Some(limit) = prompt_context_limit {
            println!(
                "[round {round}] prompt_context_budget: max_context_chars={} prompt_chars={} approx_prompt_tokens={}",
                limit,
                prompt.chars().count(),
                approximate_prompt_tokens(&prompt)
            );
        }

        let mut outcome = run_round(
            &config,
            rust_check_code.as_deref(),
            round,
            &case_name,
            &prompt,
            pool_request_plan.as_ref(),
            &pool_stage_dispatch_plans,
        );
        drop(pool_lease);
        if let Some(plan) = &pool_request_plan {
            outcome.stages.push("pool_dispatch:selected".to_owned());
            outcome.meta.push(plan.meta());
        }
        if let Some(summary) = pool_lease_summary {
            outcome.stages.push("pool_lease:acquired".to_owned());
            outcome.meta.push(format!("pool_lease {summary}"));
        }
        if !pool_stage_dispatch_plans.is_empty() {
            outcome
                .stages
                .push("pool_stage_dispatch:planned".to_owned());
            outcome.meta.extend(
                pool_stage_dispatch_plans
                    .iter()
                    .map(PoolStageDispatchPlan::meta),
            );
        }
        if config.execute_pool_stage_calls && outcome.success {
            match execute_pool_stage_calls(
                &config,
                round,
                &case_name,
                started_unix,
                validation_evidence.as_ref(),
                &prompt,
                &mut outcome,
                &pool_stage_dispatch_plans,
            ) {
                Ok(Some(meta)) => {
                    outcome.stages.push("pool_stage_call:executed".to_owned());
                    outcome.meta.push(meta);
                }
                Ok(None) => {}
                Err(error) => {
                    outcome.success = false;
                    outcome.error = Some(error.clone());
                    outcome.stages.push("pool_stage_call:failed".to_owned());
                    outcome.meta.push(error);
                }
            }
        }
        if outcome.success
            && let Some(validation_plan) = &validation_plan
            && config.validation_phase.runs_post()
            && let Some(meta) = match run_validation_gate(&config, "post", validation_plan) {
                Ok(evidence) => {
                    let meta = evidence.meta();
                    validation_evidence = Some(evidence);
                    Some(meta)
                }
                Err(failure) => {
                    outcome.success = false;
                    outcome.error = Some(failure.message.clone());
                    validation_passed = Some(false);
                    outcome.stages.push("validation:post:failed".to_owned());
                    if let Some(evidence) = failure.evidence {
                        outcome.meta.push(evidence.meta());
                        validation_evidence = Some(evidence);
                    }
                    outcome.meta.push(failure.message);
                    None
                }
            }
        {
            outcome.stages.push("validation:post:done".to_owned());
            outcome.meta.push(meta);
        }
        if !pre_round_stages.is_empty() {
            pre_round_stages.extend(outcome.stages);
            outcome.stages = pre_round_stages;
            pre_round_meta.extend(outcome.meta);
            outcome.meta = pre_round_meta;
        }
        let finished_unix = unix_seconds();
        let round_wall_ms = finished_unix
            .saturating_sub(started_unix)
            .saturating_mul(1000);
        if let Some(worker_event_meta) = append_pool_worker_event(
            &config,
            round,
            &case_name,
            &outcome,
            pool_request_plan.as_ref(),
            round_wall_ms,
        )? {
            outcome.stages.push("model_worker_v1:recorded".to_owned());
            outcome.meta.push(worker_event_meta);
        }
        if let Some(stage_event_meta) =
            append_pool_stage_worker_events(&config, round, &case_name, &pool_stage_dispatch_plans)?
        {
            outcome
                .stages
                .push("model_worker_v1:stage_planned".to_owned());
            outcome.meta.push(stage_event_meta);
        }
        let record = RoundRecord {
            round,
            case_name,
            prompt,
            started_unix,
            finished_unix,
            success: outcome.success,
            error: outcome.error.clone(),
            runtime_tokens: outcome.runtime_tokens,
            runtime_model: outcome.runtime_model,
            answer: outcome.answer,
            elapsed_ms: outcome.elapsed_ms,
            business_cycle_passed: outcome.business_cycle_passed,
            feedback_applied: outcome.feedback_applied,
            rust_check_checked: outcome.rust_check_checked,
            rust_check_passed: outcome.rust_check_passed,
            rust_check_feedback_applied: outcome.rust_check_feedback_applied,
            validation_checked,
            validation_passed,
            validation_command_source: validation_plan.as_ref().map(|plan| plan.source.to_owned()),
            validation_command_safety: validation_plan.as_ref().map(|plan| plan.safety.to_owned()),
            validation_command_preview: validation_plan.as_ref().map(|plan| plan.command.clone()),
            validation_phase: validation_evidence
                .as_ref()
                .map(|evidence| evidence.phase.clone()),
            validation_status_code: validation_evidence
                .as_ref()
                .and_then(|evidence| evidence.status_code),
            validation_elapsed_ms: validation_evidence
                .as_ref()
                .map(|evidence| evidence.elapsed_ms),
            validation_stdout_tail: validation_evidence
                .as_ref()
                .map(|evidence| evidence.stdout_tail.clone()),
            validation_stderr_tail: validation_evidence
                .as_ref()
                .map(|evidence| evidence.stderr_tail.clone()),
            self_improve_passed: outcome.self_improve_passed,
            state_gate_checked: outcome.state_gate_checked,
            state_gate_passed: outcome.state_gate_passed,
            trace_gate_checked: outcome.trace_gate_checked,
            trace_gate_passed: outcome.trace_gate_passed,
            delta_chars: outcome.delta_chars,
            stages: outcome.stages,
            meta: outcome.meta,
            allocation_evidence,
            final_json: outcome.final_json,
        };
        println!("[round {round}] stage ledger_append:start");
        append_record(&config.ledger_path, &record)?;
        println!("[round {round}] stage ledger_append:done");
        budget.record(&record, round_wall_ms);

        if let Some(report_path) = config.run_report_json_path.as_deref() {
            println!("[round {round}] stage report_refresh:start");
            match report::write_run_report_json(
                &config,
                report_path,
                config.run_report_gate,
                config.run_report_continuation_gate,
            ) {
                Ok(refresh) => {
                    println!(
                        "[round {round}] stage report_refresh:done path={} rounds={} gate={} failures={}",
                        report_path.display(),
                        refresh.rounds,
                        refresh.gate_label.as_deref().unwrap_or("none"),
                        refresh.gate_failure_count
                    );
                }
                Err(error) => {
                    println!(
                        "[round {round}] stage report_refresh:failed error={}",
                        preview_text(&error, 160)
                    );
                    return Err(error);
                }
            }
        }

        if record.success {
            consecutive_failures = 0;
            println!(
                "[round {round}] ok runtime_tokens={} elapsed_ms={}",
                option_u64_text(record.runtime_tokens),
                option_u64_text(record.elapsed_ms)
            );
        } else {
            consecutive_failures += 1;
            println!(
                "[round {round}] failed ({}/{}): {}",
                consecutive_failures,
                config.max_failures,
                record.error.as_deref().unwrap_or("unknown error")
            );
            if consecutive_failures >= config.max_failures {
                return Err(format!(
                    "stopped after {} consecutive failure(s)",
                    consecutive_failures
                ));
            }
        }

        completed += 1;
        if let Some(reason) = budget_stop_reason(&config, &budget) {
            println!("stopping: {reason}");
            return Ok(());
        }
        if config.interval_secs > 0 {
            thread::sleep(Duration::from_secs(config.interval_secs));
        }
    }
}

fn refresh_pool_artifacts(config: &Config) -> Result<(), String> {
    let manifest_path = config
        .pool_manifest_json_path
        .as_deref()
        .ok_or_else(|| "--refresh-pool-artifacts requires --pool-manifest-json".to_owned())?;
    let manifest_response = http::get(
        &config.backend,
        "/v1/model-pool/manifest",
        config.timeout_secs,
    )
    .map_err(|error| format!("refresh pool manifest failed: {error}"))?;
    write_pool_artifact(
        manifest_path,
        "pool manifest",
        manifest_response.status,
        &manifest_response.body,
    )?;

    let status_path = config
        .pool_status_json_path
        .as_deref()
        .ok_or_else(|| "--refresh-pool-artifacts requires --pool-status-json".to_owned())?;
    let status_response = http::get(
        &config.backend,
        "/v1/model-pool/status",
        config.timeout_secs,
    )
    .map_err(|error| format!("refresh pool status failed: {error}"))?;
    write_pool_artifact(
        status_path,
        "pool status",
        status_response.status,
        &status_response.body,
    )?;

    let route_path = config
        .pool_route_json_path
        .as_deref()
        .ok_or_else(|| "--refresh-pool-artifacts requires --pool-route-json".to_owned())?;
    let route_body = pool_route_refresh_body(&config.pool_route_task_kind, None);
    let route_response = http::post_json(
        &config.backend,
        "/v1/model-pool/route-plan",
        &route_body,
        config.timeout_secs,
    )
    .map_err(|error| format!("refresh pool route failed: {error}"))?;
    write_pool_artifact(
        route_path,
        "pool route",
        route_response.status,
        &route_response.body,
    )?;

    let mut refreshed_stage_routes = Vec::new();
    let mut completed_roles = initial_pool_stage_completed_roles(config);
    for task_kind in pool_stage::task_kinds(config) {
        if pool_stage::is_primary_route_task_kind(config, &task_kind) {
            continue;
        }
        let stage_path = pool_stage::route_path(config, &task_kind);
        if stage_path == route_path {
            continue;
        }
        let stage_body = pool_route_refresh_body(&task_kind, Some(&completed_roles));
        let stage_response = http::post_json(
            &config.backend,
            "/v1/model-pool/route-plan",
            &stage_body,
            config.timeout_secs,
        )
        .map_err(|error| format!("refresh pool stage route {task_kind} failed: {error}"))?;
        write_pool_artifact(
            &stage_path,
            &format!("pool stage route {task_kind}"),
            stage_response.status,
            &stage_response.body,
        )?;
        push_completed_pool_role(&mut completed_roles, &task_kind);
        refreshed_stage_routes.push(format!("{task_kind}={}", stage_path.display()));
    }

    println!(
        "pool_artifact_refresh: wrote manifest={} status={} route={} task_kind={} stage_routes={}",
        manifest_path.display(),
        status_path.display(),
        route_path.display(),
        config.pool_route_task_kind,
        if refreshed_stage_routes.is_empty() {
            "none".to_owned()
        } else {
            refreshed_stage_routes.join(",")
        }
    );
    Ok(())
}

fn pool_route_refresh_body(task_kind: &str, completed_roles: Option<&[String]>) -> String {
    let completed_roles = completed_roles
        .map(|roles| format!(",\"completed_roles\":{}", json_string_array(roles)))
        .unwrap_or_default();
    format!(
        "{{\"task_kind\":{}{}}}",
        json_string(task_kind),
        completed_roles
    )
}

fn write_pool_artifact(path: &Path, label: &str, status: u16, body: &str) -> Result<(), String> {
    if !(200..300).contains(&status) {
        return Err(format!(
            "refresh {label} returned HTTP {status}: {}",
            body.trim()
        ));
    }
    let failures = pool_contract_failures(label, body);
    if !failures.is_empty() {
        return Err(format!(
            "refresh {label} failed safe contract: {}",
            failures.join("; ")
        ));
    }
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create {} parent dir failed: {error}", path.display()))?;
    }
    fs::write(path, format!("{}\n", body.trim_end()))
        .map_err(|error| format!("write {} failed: {error}", path.display()))
}

fn pool_contract_failures(label: &str, body: &str) -> Vec<String> {
    let mut failures = Vec::new();
    if json_bool_field(body, "read_only") != Some(true) {
        failures.push("read_only is not true".to_owned());
    }
    if json_bool_field(body, "launches_process") != Some(false) {
        failures.push("launches_process is not false".to_owned());
    }
    if json_bool_field(body, "sends_prompt") != Some(false) {
        failures.push("sends_prompt is not false".to_owned());
    }
    if label == "pool manifest" {
        match json_object_field(body, "advice") {
            Some(advice) => {
                if json_string_field(&advice, "decision_source").as_deref()
                    != Some("model-pool-advice-core")
                {
                    failures.push(
                        "manifest advice.decision_source is not model-pool-advice-core".to_owned(),
                    );
                }
                if json_bool_field(&advice, "safe_to_enable_pool_workers").is_none() {
                    failures
                        .push("manifest advice.safe_to_enable_pool_workers is missing".to_owned());
                }
                if json_string_field(&advice, "next_step").is_none() {
                    failures.push("manifest advice.next_step is missing".to_owned());
                }
                if json_string_field(&advice, "reason").is_none() {
                    failures.push("manifest advice.reason is missing".to_owned());
                }
                if json_bool_field(&advice, "extra_quality_12b_detected").is_none() {
                    failures
                        .push("manifest advice.extra_quality_12b_detected is missing".to_owned());
                }
                match json_object_field(&advice, "worker_shape") {
                    Some(worker_shape) => {
                        if json_u64_field(&worker_shape, "quality").is_none() {
                            failures
                                .push("manifest advice.worker_shape.quality is missing".to_owned());
                        }
                        if json_u64_field(&worker_shape, "helpers_visible").is_none() {
                            failures.push(
                                "manifest advice.worker_shape.helpers_visible is missing"
                                    .to_owned(),
                            );
                        }
                        if json_u64_field(&worker_shape, "helper_target").is_none() {
                            failures.push(
                                "manifest advice.worker_shape.helper_target is missing".to_owned(),
                            );
                        }
                    }
                    None => {
                        failures.push("manifest advice.worker_shape object is missing".to_owned())
                    }
                }
            }
            None => failures.push("manifest advice object is missing".to_owned()),
        }
    }
    failures
}

fn load_allocation_evidence(config: &Config) -> Result<Vec<String>, String> {
    let mut evidence = Vec::new();
    let pool_manifest = if let Some(path) = &config.pool_manifest_json_path {
        let summary = pool_artifacts::load_manifest(Some(path))?;
        if let Some(summary) = summary.as_ref() {
            evidence.push(format!(
                "pool_manifest {}",
                pool_artifacts::manifest_context_text(summary)
            ));
        }
        summary
    } else {
        None
    };
    let pool_status = if let Some(path) = &config.pool_status_json_path {
        let summary = pool_artifacts::load_status(Some(path))?;
        if let Some(summary) = summary.as_ref() {
            evidence.push(format!(
                "pool_status {}",
                pool_artifacts::status_context_text(summary)
            ));
        }
        summary
    } else {
        None
    };
    if let Some(path) = &config.remote_chain_status_json_path
        && let Some(context) = remote_chain::context(path)?
    {
        evidence.push(format!("remote_chain {context}"));
    }
    let mut pool_routes = Vec::new();
    if let Some(path) = &config.pool_route_json_path {
        let summary = pool_artifacts::load_route(Some(path))?;
        if let Some(summary) = summary {
            evidence.push(format!(
                "pool_route {}",
                pool_artifacts::route_context_text(&summary)
            ));
            pool_routes.push(summary);
        }
    }
    for (task_kind, route) in pool_stage::route_summaries(config)? {
        evidence.push(format!(
            "pool_stage_route[{task_kind}] {}",
            pool_artifacts::route_context_text(&route)
        ));
        pool_routes.push(route);
    }
    if pool_manifest.is_some() || pool_status.is_some() || !pool_routes.is_empty() {
        let alignment = pool_artifacts::alignment_summary(
            pool_manifest.as_ref(),
            pool_status.as_ref(),
            &pool_routes,
        );
        evidence.push(format!(
            "pool_alignment {}",
            pool_artifacts::alignment_context_text(&alignment)
        ));
    }
    if let Some(path) = &config.pool_budget_fairness_json_path
        && path.exists()
        && let Some(summary) = pool_artifacts::load_budget_fairness(Some(path))?
    {
        evidence.push(format!(
            "pool_budget_fairness {}",
            pool_artifacts::budget_fairness_context_text(&summary)
        ));
    }
    Ok(evidence)
}

fn append_pool_worker_event(
    config: &Config,
    round: usize,
    case_name: &str,
    outcome: &RoundOutcome,
    pool_request_plan: Option<&PoolRequestPlan>,
    round_wall_ms: u64,
) -> Result<Option<String>, String> {
    let Some(path) = config.pool_budget_fairness_json_path.as_deref() else {
        return Ok(None);
    };
    let Some(plan) = pool_request_plan else {
        return Ok(None);
    };
    let feedback_applied = outcome
        .feedback_applied
        .unwrap_or(0)
        .saturating_add(outcome.rust_check_feedback_applied.unwrap_or(0));
    let runtime_tokens = outcome.runtime_tokens.unwrap_or(0);
    let latency_ms = outcome.elapsed_ms.unwrap_or(round_wall_ms);
    let answer_metrics = answer_size_metrics(outcome.answer.as_deref());
    let blocked_primary_12b = pool_worker_blocks_primary_12b(config, plan);
    let event = pool_artifacts::ModelWorkerEvent {
        round,
        case_name: case_name.to_owned(),
        role: plan.selected_role.clone(),
        worker_port: plan.selected_port,
        worker_base_url: plan.selected_base_url.clone(),
        task_kind: config.pool_route_task_kind.clone(),
        execution_state: "executed".to_owned(),
        success: outcome.success,
        feedback_applied,
        runtime_tokens,
        latency_ms,
        answer_chars: answer_metrics.chars,
        answer_bytes: answer_metrics.bytes,
        answer_approx_tokens: answer_metrics.approx_tokens,
        runtime_backend: plan.runtime_backend.clone(),
        runtime_device: plan.runtime_device.clone(),
        runtime_accelerator: plan.runtime_accelerator.clone(),
        gpu_layers: plan.gpu_layers,
        blocked_primary_12b,
        default_max_tokens: plan.default_max_tokens,
        configured_max_tokens: plan.configured_max_tokens,
        effective_max_tokens: plan.effective_max_tokens,
        max_tokens_clamped: plan.max_tokens_clamped,
        can_accept_low_priority_task: plan.can_accept_low_priority_task,
    };
    pool_artifacts::append_model_worker_event(path, &event)?;
    Ok(Some(format!(
        "model_worker_v1 path={} role={} task_kind={} success={} feedback={} runtime_tokens={} latency_ms={} answer_chars={} answer_approx_tokens={} runtime_backend={} runtime_device={} runtime_accelerator={} gpu_layers={} blocked_primary_12b={}",
        path.display(),
        event.role,
        event.task_kind,
        event.success,
        event.feedback_applied,
        event.runtime_tokens,
        event.latency_ms,
        option_u64_text(event.answer_chars),
        option_u64_text(event.answer_approx_tokens),
        event.runtime_backend.as_deref().unwrap_or("none"),
        event.runtime_device.as_deref().unwrap_or("none"),
        event.runtime_accelerator.as_deref().unwrap_or("none"),
        option_u64_text(event.gpu_layers),
        event.blocked_primary_12b
    )))
}

fn append_pool_stage_worker_events(
    config: &Config,
    round: usize,
    case_name: &str,
    plans: &[PoolStageDispatchPlan],
) -> Result<Option<String>, String> {
    let Some(path) = config.pool_budget_fairness_json_path.as_deref() else {
        return Ok(None);
    };
    if plans.is_empty() {
        return Ok(None);
    }
    let mut recorded = 0usize;
    let mut roles = Vec::new();
    for plan in plans {
        let event = pool_artifacts::ModelWorkerEvent {
            round,
            case_name: case_name.to_owned(),
            role: plan.selected_role.clone(),
            worker_port: plan.selected_port,
            worker_base_url: plan.selected_base_url.clone(),
            task_kind: plan.task_kind.clone(),
            execution_state: "planned".to_owned(),
            success: false,
            feedback_applied: 0,
            runtime_tokens: 0,
            latency_ms: 0,
            answer_chars: None,
            answer_bytes: None,
            answer_approx_tokens: None,
            runtime_backend: plan.runtime_backend.clone(),
            runtime_device: plan.runtime_device.clone(),
            runtime_accelerator: plan.runtime_accelerator.clone(),
            gpu_layers: plan.gpu_layers,
            blocked_primary_12b: false,
            default_max_tokens: plan.default_max_tokens,
            configured_max_tokens: plan.configured_max_tokens,
            effective_max_tokens: plan.effective_max_tokens,
            max_tokens_clamped: plan.max_tokens_clamped,
            can_accept_low_priority_task: plan.can_accept_low_priority_task,
        };
        pool_artifacts::append_model_worker_event(path, &event)?;
        recorded += 1;
        roles.push(format!("{}:{}", plan.task_kind, plan.selected_role));
    }
    Ok(Some(format!(
        "model_worker_v1_stage_plans path={} planned={} roles={}",
        path.display(),
        recorded,
        roles.join(",")
    )))
}

fn execute_pool_stage_calls(
    config: &Config,
    round: usize,
    case_name: &str,
    round_started_unix: u64,
    validation_evidence: Option<&ValidationGateEvidence>,
    prompt: &str,
    outcome: &mut RoundOutcome,
    plans: &[PoolStageDispatchPlan],
) -> Result<Option<String>, String> {
    if plans.is_empty() {
        return Ok(None);
    }
    let mut executed = 0usize;
    let mut skipped = 0usize;
    let mut summaries = Vec::new();
    let mut completed_roles = initial_pool_stage_completed_roles(config);
    for plan in plans {
        if let Some(reason) = memory_pressure_gate_skip_reason(plan, skipped) {
            skipped += 1;
            outcome.meta.push(format!(
                "memory_pressure_gate task_kind={} blocked=true reason={reason}",
                plan.task_kind
            ));
            outcome.meta.push(format!(
                "pool_stage_call_skipped task_kind={} reason=memory_pressure_gate {reason}",
                plan.task_kind
            ));
            summaries.push(format!(
                "{}:{} skipped_memory_pressure",
                plan.task_kind, plan.selected_role
            ));
            continue;
        }
        let stage_lease =
            match pool_lease::acquire_stage(config, plan, round, case_name, unix_seconds())? {
                pool_lease::PoolLeaseAcquire::Disabled => None,
                pool_lease::PoolLeaseAcquire::Acquired(lease) => {
                    let summary = lease.summary().to_owned();
                    outcome.meta.push(format!(
                        "pool_stage_lease task_kind={} acquired {summary}",
                        plan.task_kind
                    ));
                    Some(lease)
                }
                pool_lease::PoolLeaseAcquire::Skipped { reason } => {
                    skipped += 1;
                    outcome.meta.push(format!(
                        "pool_stage_call_skipped task_kind={} {reason}",
                        plan.task_kind
                    ));
                    summaries.push(format!("{}:{} skipped", plan.task_kind, plan.selected_role));
                    continue;
                }
            };
        let stage_validation_evidence =
            validation_evidence.map(|evidence| pool_stage_call::PoolStageValidationEvidence {
                phase: &evidence.phase,
                command_source: &evidence.command_source,
                command_safety: &evidence.command_safety,
                command_preview: &evidence.command_preview,
                status_code: evidence.status_code,
                elapsed_ms: evidence.elapsed_ms,
                stdout_tail: &evidence.stdout_tail,
                stderr_tail: &evidence.stderr_tail,
            });
        let input = PoolStageCallInput {
            task_kind: &plan.task_kind,
            case_name,
            round,
            validation_timestamp_unix: Some(round_started_unix),
            validation_evidence: stage_validation_evidence.as_ref(),
            original_prompt: prompt,
            primary_answer: outcome.answer.as_deref(),
            final_json: outcome.final_json.as_deref(),
            dispatch_plan: Some(plan),
            completed_roles: &completed_roles,
            max_tokens: plan.effective_max_tokens,
        };
        let result = pool_stage_call::call_backend(&config.backend, config.timeout_secs, &input)?;
        drop(stage_lease);
        append_pool_stage_call_worker_event(config, round, case_name, plan, &result)?;
        if !result.ok {
            return Err(format!(
                "pool stage call {} returned ok=false role={} answer_preview={}",
                plan.task_kind,
                result
                    .selected_role
                    .as_deref()
                    .unwrap_or(plan.selected_role.as_str()),
                result
                    .answer
                    .as_deref()
                    .map(|answer| preview_text(answer, 160))
                    .unwrap_or_else(|| "none".to_owned())
            ));
        }
        executed += 1;
        push_completed_pool_role(&mut completed_roles, &result.task_kind);
        if let Some(selected_role) = result.selected_role.as_deref() {
            push_completed_pool_role(&mut completed_roles, selected_role);
        } else {
            push_completed_pool_role(&mut completed_roles, &plan.selected_role);
        }
        if let Some(answer) = result.answer.as_deref() {
            outcome.meta.push(format!(
                "pool_stage_call_answer task_kind={} role={} elapsed_ms={} answer_approx_tokens={} preview={}",
                result.task_kind,
                result
                    .selected_role
                    .as_deref()
                    .unwrap_or(plan.selected_role.as_str()),
                option_u64_text(result.elapsed_ms),
                option_u64_text(result.answer_approx_tokens),
                preview_text(answer, MAX_POOL_STAGE_CALL_ANSWER_PREVIEW_CHARS)
            ));
        }
        summaries.push(format!(
            "{}:{} elapsed_ms={} answer_approx_tokens={}",
            result.task_kind,
            result
                .selected_role
                .as_deref()
                .unwrap_or(plan.selected_role.as_str()),
            option_u64_text(result.elapsed_ms),
            option_u64_text(result.answer_approx_tokens)
        ));
    }
    Ok(Some(format!(
        "pool_stage_call executed={} skipped={} completed_roles={} stages={}",
        executed,
        skipped,
        completed_roles.join(","),
        summaries.join(",")
    )))
}

fn memory_pressure_gate_skip_reason(
    plan: &PoolStageDispatchPlan,
    prior_stage_skips: usize,
) -> Option<String> {
    let task_kind = plan.task_kind.trim().to_ascii_lowercase();
    if task_kind != "test-gate" || prior_stage_skips == 0 {
        return None;
    }
    Some(format!(
        "prior_stage_skips={prior_stage_skips} selected_role={} port={} effective_max_tokens={} max_tokens_clamped={} low_priority={}",
        plan.selected_role,
        option_u64_text(plan.selected_port),
        plan.effective_max_tokens,
        plan.max_tokens_clamped,
        plan.can_accept_low_priority_task
    ))
}

fn initial_pool_stage_completed_roles(config: &Config) -> Vec<String> {
    let mut roles = Vec::new();
    push_completed_pool_role(&mut roles, "quality");
    push_completed_pool_role(&mut roles, &config.pool_route_task_kind);
    roles
}

fn push_completed_pool_role(roles: &mut Vec<String>, role: &str) {
    let role = canonical_pool_role(role);
    if role.is_empty() || matches!(role.as_str(), "auto" | "chat" | "business-cycle") {
        return;
    }
    if !roles.iter().any(|existing| existing == &role) {
        roles.push(role);
    }
}

fn canonical_pool_role(role: &str) -> String {
    match role.trim().to_ascii_lowercase().as_str() {
        "primary" | "generate" | "generation" => "quality".to_owned(),
        "route" | "intent" | "intent-classify" | "preflight" | "tool-call" | "tool_calls"
        | "function" | "function-call" | "function_call" => "router".to_owned(),
        "test" | "gate" => "test-gate".to_owned(),
        "repo-index" | "repository-index" | "spare" => "index".to_owned(),
        "business_cycle" | "business" => "business-cycle".to_owned(),
        other => other.to_owned(),
    }
}

fn append_pool_stage_call_worker_event(
    config: &Config,
    round: usize,
    case_name: &str,
    plan: &PoolStageDispatchPlan,
    result: &PoolStageCallResult,
) -> Result<(), String> {
    let Some(path) = config.pool_budget_fairness_json_path.as_deref() else {
        return Ok(());
    };
    let selected_role = result
        .selected_role
        .clone()
        .unwrap_or_else(|| plan.selected_role.clone());
    let answer_feedback = u64::from(
        result.ok
            && result
                .answer
                .as_deref()
                .is_some_and(|answer| !answer.trim().is_empty()),
    );
    let event = pool_artifacts::ModelWorkerEvent {
        round,
        case_name: case_name.to_owned(),
        role: selected_role.clone(),
        worker_port: result.selected_port.or(plan.selected_port),
        worker_base_url: result
            .selected_base_url
            .clone()
            .or_else(|| plan.selected_base_url.clone()),
        task_kind: result.task_kind.clone(),
        execution_state: "executed".to_owned(),
        success: result.ok,
        feedback_applied: answer_feedback,
        runtime_tokens: result.answer_approx_tokens.unwrap_or(0),
        latency_ms: result.elapsed_ms.unwrap_or(0),
        answer_chars: result.answer_chars,
        answer_bytes: result.answer_bytes,
        answer_approx_tokens: result.answer_approx_tokens,
        runtime_backend: plan.runtime_backend.clone(),
        runtime_device: plan.runtime_device.clone(),
        runtime_accelerator: plan.runtime_accelerator.clone(),
        gpu_layers: plan.gpu_layers,
        blocked_primary_12b: stage_call_blocks_primary_12b(&result.task_kind, &selected_role),
        default_max_tokens: plan.default_max_tokens,
        configured_max_tokens: plan.configured_max_tokens,
        effective_max_tokens: plan.effective_max_tokens,
        max_tokens_clamped: plan.max_tokens_clamped,
        can_accept_low_priority_task: plan.can_accept_low_priority_task,
    };
    pool_artifacts::append_model_worker_event(path, &event)
}

fn stage_call_blocks_primary_12b(task_kind: &str, selected_role: &str) -> bool {
    !task_kind.trim().eq_ignore_ascii_case("quality")
        && matches!(
            selected_role.trim().to_ascii_lowercase().as_str(),
            "quality" | "primary" | "primary-12b" | "remote-quality"
        )
}

fn pool_worker_blocks_primary_12b(config: &Config, plan: &PoolRequestPlan) -> bool {
    let task_kind = config.pool_route_task_kind.trim().to_ascii_lowercase();
    let selected_role = plan.selected_role.trim().to_ascii_lowercase();
    !matches!(task_kind.as_str(), "quality")
        && matches!(
            selected_role.as_str(),
            "quality" | "primary" | "primary-12b" | "remote-quality"
        )
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct AnswerSizeMetrics {
    chars: Option<u64>,
    bytes: Option<u64>,
    approx_tokens: Option<u64>,
}

fn answer_size_metrics(answer: Option<&str>) -> AnswerSizeMetrics {
    let Some(answer) = answer else {
        return AnswerSizeMetrics::default();
    };
    let chars = answer.chars().count() as u64;
    AnswerSizeMetrics {
        chars: Some(chars),
        bytes: Some(answer.len() as u64),
        approx_tokens: Some(approx_tokens_from_chars(chars)),
    }
}

fn approx_tokens_from_chars(chars: u64) -> u64 {
    if chars == 0 { 0 } else { chars.div_ceil(4) }
}

fn run_remote_chain_gate(config: &Config) -> Result<(), String> {
    let Some(path) = config.remote_chain_status_json_path.as_deref() else {
        return Err("--remote-chain-gate requires --remote-chain-status-json".to_owned());
    };
    let Some(summary) = remote_chain::load_status(Some(path))? else {
        return Err(format!(
            "remote chain gate failed: artifact is empty ({})",
            path.display()
        ));
    };
    if let Some(failure) = remote_chain::gate_failure(&summary) {
        Err(format!("remote chain gate failed: {failure}"))
    } else {
        println!(
            "remote_chain_gate: passed {}",
            remote_chain::context_text(&summary)
        );
        Ok(())
    }
}

fn run_pool_capacity_gate(config: &Config) -> Result<(), String> {
    let Some(path) = config.pool_status_json_path.as_deref() else {
        return Err("--pool-capacity-gate requires --pool-status-json".to_owned());
    };
    let Some(summary) = pool_artifacts::load_status(Some(path))? else {
        return Err(format!(
            "pool capacity gate failed: artifact is empty ({})",
            path.display()
        ));
    };
    if let Some(failure) = pool_artifacts::capacity_gate_failure(&summary) {
        Err(format!("pool capacity gate failed: {failure}"))
    } else {
        println!(
            "pool_capacity_gate: passed {}",
            pool_artifacts::status_context_text(&summary)
        );
        Ok(())
    }
}

fn run_pool_stage_route_gate(config: &Config) -> Result<(), String> {
    let failures = pool_stage::gate_failures(config);
    if failures.is_empty() {
        println!(
            "pool_stage_route_gate: passed task_kinds={}",
            pool_stage::task_kinds_text(&config.pool_stage_route_task_kinds)
        );
        Ok(())
    } else {
        Err(format!(
            "pool stage route gate failed: {}",
            failures.join("; ")
        ))
    }
}

fn run_pool_alignment_gate(config: &Config) -> Result<(), String> {
    let alignment = load_pool_alignment_for_gate(config)?;
    let context = pool_artifacts::alignment_context_text(&alignment);
    if let Some(failure) = pool_artifacts::alignment_gate_failure(&alignment) {
        Err(format!("pool alignment gate failed: {failure}; {context}"))
    } else {
        println!("pool_alignment_gate: passed {context}");
        Ok(())
    }
}

fn load_pool_alignment_for_gate(
    config: &Config,
) -> Result<pool_artifacts::PoolAlignmentSummary, String> {
    let manifest_path = config
        .pool_manifest_json_path
        .as_deref()
        .ok_or_else(|| "--pool-alignment-gate requires --pool-manifest-json".to_owned())?;
    let Some(manifest) = pool_artifacts::load_manifest(Some(manifest_path))? else {
        return Err(format!(
            "pool alignment gate failed: manifest artifact is empty ({})",
            manifest_path.display()
        ));
    };

    let status_path = config
        .pool_status_json_path
        .as_deref()
        .ok_or_else(|| "--pool-alignment-gate requires --pool-status-json".to_owned())?;
    let Some(status) = pool_artifacts::load_status(Some(status_path))? else {
        return Err(format!(
            "pool alignment gate failed: status artifact is empty ({})",
            status_path.display()
        ));
    };

    let route_path = config
        .pool_route_json_path
        .as_deref()
        .ok_or_else(|| "--pool-alignment-gate requires --pool-route-json".to_owned())?;
    let Some(primary_route) = pool_artifacts::load_route(Some(route_path))? else {
        return Err(format!(
            "pool alignment gate failed: route artifact is empty ({})",
            route_path.display()
        ));
    };
    let mut routes = vec![primary_route];
    for task_kind in pool_stage::task_kinds(config) {
        if pool_stage::is_primary_route_task_kind(config, &task_kind) {
            continue;
        }
        let path = pool_stage::route_path(config, &task_kind);
        let Some(route) = pool_artifacts::load_route(Some(&path))? else {
            return Err(format!(
                "pool alignment gate failed: stage route {task_kind} artifact is empty ({})",
                path.display()
            ));
        };
        routes.push(route);
    }
    Ok(pool_artifacts::alignment_summary(
        Some(&manifest),
        Some(&status),
        &routes,
    ))
}

fn run_pool_budget_fairness_gate(config: &Config) -> Result<(), String> {
    let Some(path) = config.pool_budget_fairness_json_path.as_deref() else {
        return Err("--pool-budget-fairness-gate requires --pool-budget-fairness-json".to_owned());
    };
    let Some(summary) = pool_artifacts::load_budget_fairness(Some(path))? else {
        return Err(format!(
            "pool budget fairness gate failed: artifact is empty ({})",
            path.display()
        ));
    };
    let context = pool_artifacts::budget_fairness_context_text(&summary);
    if summary.budget_fairness_blocked {
        let reasons = if summary.failure_reasons.is_empty() {
            "unknown".to_owned()
        } else {
            summary.failure_reasons.join("; ")
        };
        return Err(format!(
            "pool budget fairness gate failed: {reasons}; {context}"
        ));
    }
    println!("pool_budget_fairness_gate: passed {context}");
    Ok(())
}

fn pool_lease_skip_stop_reason(
    config: &Config,
    consecutive_pool_lease_skips: usize,
    last_reason: &str,
) -> Option<String> {
    let limit = config.max_pool_lease_skips?;
    (consecutive_pool_lease_skips >= limit).then(|| {
        format!(
            "stopped after {consecutive_pool_lease_skips} consecutive pool lease skip(s) (limit {limit}): {last_reason}"
        )
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct BudgetState {
    runtime_tokens: u64,
    last_runtime_tokens: Option<u64>,
    observed_runtime_ms: u64,
    last_observed_runtime_ms: Option<u64>,
    consecutive_no_feedback_rounds: usize,
}

impl BudgetState {
    fn record(&mut self, record: &RoundRecord, round_wall_ms: u64) {
        if let Some(runtime_tokens) = record.runtime_tokens {
            self.last_runtime_tokens = Some(runtime_tokens);
        }
        self.runtime_tokens = self
            .runtime_tokens
            .saturating_add(record.runtime_tokens.unwrap_or_default());
        let observed_round_ms = record.elapsed_ms.unwrap_or_default().max(round_wall_ms);
        self.last_observed_runtime_ms = Some(observed_round_ms);
        self.observed_runtime_ms = self.observed_runtime_ms.saturating_add(observed_round_ms);
        if record.feedback_applied.unwrap_or_default() == 0 {
            self.consecutive_no_feedback_rounds =
                self.consecutive_no_feedback_rounds.saturating_add(1);
        } else {
            self.consecutive_no_feedback_rounds = 0;
        }
    }
}

fn pre_round_budget_stop_reason(config: &Config, budget: &BudgetState) -> Option<String> {
    if let Some(max_total_tokens) = config.max_total_tokens {
        if budget.runtime_tokens >= max_total_tokens {
            return Some(format!(
                "runtime token budget reached ({}/{})",
                budget.runtime_tokens, max_total_tokens
            ));
        }
        if let Some(last_runtime_tokens) = budget.last_runtime_tokens {
            let projected = budget.runtime_tokens.saturating_add(last_runtime_tokens);
            if projected > max_total_tokens {
                return Some(format!(
                    "runtime token budget would be exceeded by another round ({}+{}>{})",
                    budget.runtime_tokens, last_runtime_tokens, max_total_tokens
                ));
            }
        }
    }
    if let Some(max_runtime_secs) = config.max_runtime_secs {
        let max_runtime_ms = max_runtime_secs.saturating_mul(1000);
        if budget.observed_runtime_ms >= max_runtime_ms {
            return Some(format!(
                "runtime seconds budget reached ({}/{})",
                budget.observed_runtime_ms / 1000,
                max_runtime_secs
            ));
        }
        if let Some(last_observed_runtime_ms) = budget.last_observed_runtime_ms {
            let projected = budget
                .observed_runtime_ms
                .saturating_add(last_observed_runtime_ms);
            if projected > max_runtime_ms {
                return Some(format!(
                    "runtime seconds budget would be exceeded by another round ({}+{}>{})",
                    budget.observed_runtime_ms / 1000,
                    last_observed_runtime_ms / 1000,
                    max_runtime_secs
                ));
            }
        }
    }
    None
}

fn budget_stop_reason(config: &Config, budget: &BudgetState) -> Option<String> {
    if let Some(max_total_tokens) = config.max_total_tokens
        && budget.runtime_tokens >= max_total_tokens
    {
        return Some(format!(
            "runtime token budget reached ({}/{})",
            budget.runtime_tokens, max_total_tokens
        ));
    }
    if let Some(max_runtime_secs) = config.max_runtime_secs
        && budget.observed_runtime_ms >= max_runtime_secs.saturating_mul(1000)
    {
        return Some(format!(
            "runtime seconds budget reached ({}/{})",
            budget.observed_runtime_ms / 1000,
            max_runtime_secs
        ));
    }
    if let Some(max_no_feedback_rounds) = config.max_no_feedback_rounds
        && budget.consecutive_no_feedback_rounds >= max_no_feedback_rounds
    {
        return Some(format!(
            "no feedback updates for {} consecutive round(s)",
            budget.consecutive_no_feedback_rounds
        ));
    }
    None
}

fn run_state_consistency_gate(config: &Config) -> Result<(), String> {
    let hygiene = read_ledger_hygiene(&config.ledger_path)?;
    let failures = hygiene.state_consistency_failures();
    if failures.is_empty() {
        println!(
            "state_consistency_gate: passed records={} unique_rounds={} round_gaps={}",
            hygiene.records, hygiene.unique_rounds, hygiene.round_gaps
        );
        Ok(())
    } else {
        Err(format!(
            "state consistency gate failed: {}",
            failures.join("; ")
        ))
    }
}

fn effective_validation_command(config: &Config) -> Result<Option<ValidationCommandPlan>, String> {
    if let Some(command) = config
        .validation_command
        .as_deref()
        .filter(|command| !command.trim().is_empty())
    {
        return Ok(Some(ValidationCommandPlan {
            command: command.to_owned(),
            source: "configured",
            safety: "explicit",
        }));
    }
    if !config.use_test_gate_validation_command {
        return Ok(None);
    }
    let Some(summary) = report::latest_test_gate_summary(&config.ledger_path)? else {
        return Err(
            "--use-test-gate-validation-command requested but no test-gate feedback exists in ledger"
                .to_owned(),
        );
    };
    let Some(command) = summary
        .latest_validation_command
        .as_deref()
        .filter(|command| !command.trim().is_empty())
    else {
        return Err(
            "--use-test-gate-validation-command requested but latest test-gate validation_command is missing"
                .to_owned(),
        );
    };
    if summary.latest_validation_command_safety != "safe" {
        return Err(format!(
            "--use-test-gate-validation-command requested but latest test-gate validation_command is {}",
            summary.latest_validation_command_safety
        ));
    }
    Ok(Some(ValidationCommandPlan {
        command: command.to_owned(),
        source: "test-gate",
        safety: "safe",
    }))
}

fn run_validation_gate(
    config: &Config,
    phase: &str,
    plan: &ValidationCommandPlan,
) -> Result<ValidationGateEvidence, ValidationGateFailure> {
    println!(
        "validation_gate:{phase}: running command={} source={} safety={} timeout_secs={}",
        preview_text(&plan.command, 120),
        plan.source,
        plan.safety,
        config.validation_timeout_secs
    );
    let result = validation::run_command(
        &plan.command,
        config.validation_workdir.as_deref(),
        config.validation_timeout_secs,
    )
    .map_err(|message| ValidationGateFailure {
        message,
        evidence: None,
    })?;
    let evidence = ValidationGateEvidence {
        phase: phase.to_owned(),
        command_source: plan.source.to_owned(),
        command_safety: plan.safety.to_owned(),
        command_preview: preview_text(&plan.command, 240),
        status_code: result.status_code,
        elapsed_ms: result.elapsed_ms,
        stdout_tail: result.stdout_tail.clone(),
        stderr_tail: result.stderr_tail.clone(),
    };
    if result.status_code == Some(0) {
        println!(
            "validation_gate:{phase}: passed elapsed_ms={} stdout_tail={} stderr_tail={}",
            result.elapsed_ms,
            dash_if_empty(&result.stdout_tail),
            dash_if_empty(&result.stderr_tail)
        );
        Ok(evidence)
    } else {
        Err(ValidationGateFailure {
            message: validation::failure_message(phase, &plan.command, &result),
            evidence: Some(evidence),
        })
    }
}

fn validation_phase_text(phase: crate::args::ValidationPhase) -> &'static str {
    match phase {
        crate::args::ValidationPhase::Pre => "pre",
        crate::args::ValidationPhase::Post => "post",
        crate::args::ValidationPhase::Both => "both",
    }
}

fn dash_if_empty(value: &str) -> &str {
    if value.is_empty() { "-" } else { value }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum HealthAction {
    Ready,
    Wait(&'static str),
}

fn backend_health_action(config: &Config) -> Result<HealthAction, String> {
    for attempt in 1..=HEALTH_GATE_METADATA_ATTEMPTS {
        println!(
            "health_gate: start endpoint=/health attempt={attempt}/{} timeout_secs={}",
            HEALTH_GATE_METADATA_ATTEMPTS, config.timeout_secs
        );
        let response = match http::get(&config.backend, "/health", config.timeout_secs) {
            Ok(response) => response,
            Err(error) => {
                return Err(format!(
                    "health gate failed: endpoint=/health attempt={attempt}/{} timeout_secs={}: {error}",
                    HEALTH_GATE_METADATA_ATTEMPTS, config.timeout_secs
                ));
            }
        };
        if !(200..300).contains(&response.status) {
            return Err(format!(
                "health gate returned HTTP {}: {}",
                response.status,
                response.body.trim()
            ));
        }
        let busy = json_bool_field(&response.body, "engine_busy").unwrap_or(false);
        let active = json_u64_field(&response.body, "active_engine_requests").unwrap_or(0);
        if busy || active > 0 {
            return Ok(HealthAction::Wait("busy"));
        }
        let summary = runtime_health_summary(&response.body);
        if !summary.is_empty() {
            println!("health_gate: {summary}");
        }
        let failures = health_gate_failures(config, &response.body);
        if failures.is_empty() {
            return Ok(HealthAction::Ready);
        }
        if let Some(reason) = health_gate_retry_reason(config, &response.body, &failures)
            && attempt < HEALTH_GATE_METADATA_ATTEMPTS
        {
            println!(
                "health_gate: {reason}; retrying metadata probe ({}/{})",
                attempt + 1,
                HEALTH_GATE_METADATA_ATTEMPTS
            );
            thread::sleep(Duration::from_secs(HEALTH_GATE_METADATA_RETRY_SECS));
            continue;
        }
        return Err(format!("health gate failed: {}", failures.join("; ")));
    }
    Err("health gate failed after metadata retries".to_owned())
}

fn runtime_health_summary(body: &str) -> String {
    let model = json_string_field(body, "gemma_runtime_model");
    let context = json_u64_field(body, "gemma_runtime_context_window");
    let train_context = json_u64_field(body, "gemma_runtime_train_context_window");
    let metadata_error = json_string_field(body, "gemma_runtime_metadata_error");
    let mut parts = Vec::new();
    if let Some(model) = model {
        parts.push(format!("model={model}"));
    }
    if let Some(context) = context {
        if let Some(train_context) = train_context {
            parts.push(format!("n_ctx={context}/{train_context}"));
        } else {
            parts.push(format!("n_ctx={context}"));
        }
    }
    if let Some(metadata_error) = metadata_error {
        parts.push(format!(
            "metadata_error={}",
            preview_text(&metadata_error, 160)
        ));
    }
    parts.join(" ")
}

fn health_gate_failures(config: &Config, body: &str) -> Vec<String> {
    let mut failures = Vec::new();
    if json_bool_field(body, "gemma_runtime_reachable") == Some(false) {
        failures.push("Gemma runtime is not reachable".to_owned());
    }
    if json_bool_field(body, "safe_device_ok") == Some(false) {
        failures.push("safe device check failed".to_owned());
    }
    if json_bool_field(body, "clean") == Some(false) {
        failures.push("experience hygiene is not clean".to_owned());
    }
    if json_bool_field(body, "readiness_ok") == Some(false) {
        failures.push("backend readiness is false".to_owned());
    }
    if let Some(min_context) = config.min_runtime_context_window {
        match json_u64_field(body, "gemma_runtime_context_window") {
            Some(actual) if actual >= min_context => {}
            Some(actual) => failures.push(format!(
                "Gemma runtime n_ctx {actual} below required {min_context}"
            )),
            None => failures.push(format!(
                "Gemma runtime n_ctx is missing; required {min_context}"
            )),
        }
    }
    failures
}

fn health_gate_retry_reason(
    config: &Config,
    body: &str,
    failures: &[String],
) -> Option<&'static str> {
    let min_context_required = config.min_runtime_context_window.is_some();
    let missing_context = json_u64_field(body, "gemma_runtime_context_window").is_none();
    let metadata_error = json_string_field(body, "gemma_runtime_metadata_error").is_some();
    if !(min_context_required && missing_context && metadata_error) {
        return None;
    }
    let only_metadata_failures = failures.iter().all(|failure| {
        failure.starts_with("Gemma runtime n_ctx is missing")
            || failure == "Gemma runtime is not reachable"
            || failure == "backend readiness is false"
    });
    only_metadata_failures.then_some("Gemma runtime metadata is temporarily unavailable")
}

fn run_experience_audit_gate(config: &Config) -> Result<(), String> {
    let body = format!("{{\"limit\":{}}}", config.experience_audit_limit);
    println!(
        "experience_audit_gate: start endpoint=/v1/experience-cleanup-audit limit={} timeout_secs={}",
        config.experience_audit_limit, config.timeout_secs
    );
    let response = http::post_json(
        &config.backend,
        "/v1/experience-cleanup-audit",
        &body,
        config.timeout_secs,
    )?;
    if !(200..300).contains(&response.status) {
        return Err(format!(
            "experience audit gate returned HTTP {}: {}",
            response.status,
            response.body.trim()
        ));
    }
    if let Some(reason) = experience_audit_deferred_reason(&response.body) {
        println!("experience_audit_gate: deferred {reason}");
        return Ok(());
    }

    let failures = experience_audit_failures(config, &response.body);
    if failures.is_empty() {
        let index_report = json_object_field(&response.body, "index_report");
        let quality_score =
            audit_index_f64(&response.body, index_report.as_deref(), "quality_score")
                .unwrap_or(0.0);
        let retrieval_ready =
            audit_index_bool(&response.body, index_report.as_deref(), "retrieval_ready")
                .map(|value| value.to_string())
                .unwrap_or_else(|| "missing".to_owned());
        println!(
            "experience_audit_gate: passed overlong_records={} overlong_without_clean_gist={} max_record_chars={} noisy_records={} max_noise_penalty={:.6} quality_score={:.6} retrieval_ready={} quarantine_candidates={} repairable_legacy_metadata_lessons={} legacy_metadata_without_clean_gist={}",
            audit_index_u64(&response.body, index_report.as_deref(), "overlong_records")
                .unwrap_or(0),
            audit_index_u64(
                &response.body,
                index_report.as_deref(),
                "overlong_without_clean_gist"
            )
            .unwrap_or(0),
            audit_index_u64(&response.body, index_report.as_deref(), "max_record_chars")
                .unwrap_or(0),
            audit_index_u64(&response.body, index_report.as_deref(), "noisy_records").unwrap_or(0),
            audit_index_f64(&response.body, index_report.as_deref(), "max_noise_penalty")
                .unwrap_or(0.0),
            quality_score,
            retrieval_ready,
            json_u64_field(&response.body, "quarantine_candidates").unwrap_or(0),
            json_u64_field(&response.body, "repairable_legacy_metadata_lessons").unwrap_or(0),
            json_u64_field(&response.body, "legacy_metadata_without_clean_gist").unwrap_or(0)
        );
        Ok(())
    } else {
        Err(format!(
            "experience audit gate failed: {}",
            failures.join("; ")
        ))
    }
}

fn experience_audit_deferred_reason(body: &str) -> Option<String> {
    if json_bool_field(body, "checked") == Some(true) {
        return None;
    }
    json_string_field(body, "error").filter(|error| {
        error.starts_with("experience_hygiene_deferred_large_file")
            || error.starts_with("experience_cleanup_audit_deferred_large_file")
    })
}

fn experience_audit_failures(config: &Config, body: &str) -> Vec<String> {
    let mut failures = Vec::new();
    let error = json_string_field(body, "error");
    if json_bool_field(body, "checked") != Some(true) {
        if error.as_deref() == Some("experience_file_missing") {
            return failures;
        }
        failures.push(format!(
            "audit not checked: {}",
            error.as_deref().unwrap_or("unknown error")
        ));
        return failures;
    }

    if let Some(error) = error {
        failures.push(format!("audit error: {error}"));
    }
    let index_report = json_object_field(body, "index_report");
    let index_report = index_report.as_deref();
    let overlong_records = audit_index_u64(body, index_report, "overlong_records").unwrap_or(0);
    if let Some(max_index_overlong_records) = config.max_index_overlong_records
        && overlong_records > max_index_overlong_records
    {
        failures.push(format!(
            "index overlong_records {} above maximum {}",
            overlong_records, max_index_overlong_records
        ));
    }
    let overlong_without_clean_gist =
        audit_index_u64(body, index_report, "overlong_without_clean_gist").unwrap_or(0);
    if overlong_without_clean_gist > config.max_index_overlong_without_clean_gist {
        failures.push(format!(
            "index overlong_without_clean_gist {} above maximum {}",
            overlong_without_clean_gist, config.max_index_overlong_without_clean_gist
        ));
    }
    let max_record_chars = audit_index_u64(body, index_report, "max_record_chars").unwrap_or(0);
    if let Some(max_index_record_chars) = config.max_index_record_chars
        && max_record_chars > max_index_record_chars
    {
        failures.push(format!(
            "index max_record_chars {} above maximum {}",
            max_record_chars, max_index_record_chars
        ));
    }
    let context_rot_signal = context_rot_signal_from_audit(body, index_report);
    let context_rot_gate = context_rot_gate_from_config(config);
    let context_rot_breakdown = context_rot_gate.evaluate_breakdown(&context_rot_signal);
    if let Some(breach) = context_rot_breakdown.noisy_records {
        failures.push(format!(
            "index noisy_records {} above maximum {}",
            breach.actual, breach.maximum
        ));
    }
    if let Some(breach) = context_rot_breakdown.max_noise_penalty {
        failures.push(format!(
            "index max_noise_penalty {:.6} above maximum {:.6}",
            breach.actual, breach.maximum
        ));
    }
    match audit_index_f64(body, index_report, "quality_score") {
        Some(quality_score) if quality_score < config.min_index_quality_score => {
            failures.push(format!(
                "index quality_score {:.6} below minimum {:.6}",
                quality_score, config.min_index_quality_score
            ));
        }
        None if config.min_index_quality_score > 0.0 => {
            failures.push(format!(
                "index quality_score missing; required minimum {:.6}",
                config.min_index_quality_score
            ));
        }
        _ => {}
    }
    if config.require_index_retrieval_ready {
        match audit_index_bool(body, index_report, "retrieval_ready") {
            Some(true) => {}
            Some(false) => failures.push("index retrieval_ready false".to_owned()),
            None => failures.push("index retrieval_ready missing".to_owned()),
        }
    }
    if let Some(breach) = context_rot_breakdown.quarantine_candidates {
        failures.push(format!(
            "quarantine_candidates {} above maximum {}",
            breach.actual, breach.maximum
        ));
    }
    if let Some(breach) = context_rot_breakdown.repairable_legacy_metadata_lessons {
        failures.push(format!(
            "repairable_legacy_metadata_lessons {} above maximum {}",
            breach.actual, breach.maximum
        ));
    }
    if let Some(breach) = context_rot_breakdown.legacy_metadata_without_clean_gist {
        failures.push(format!(
            "legacy_metadata_without_clean_gist {} above maximum {}",
            breach.actual, breach.maximum
        ));
    }
    failures
}

fn context_rot_signal_from_audit(body: &str, index_report: Option<&str>) -> ContextRotSignal {
    ContextRotSignal::clean()
        .with_noisy_records(audit_index_u64(body, index_report, "noisy_records").unwrap_or(0))
        .with_max_noise_penalty(
            audit_index_f64(body, index_report, "max_noise_penalty").unwrap_or(0.0),
        )
        .with_quarantine_candidates(json_u64_field(body, "quarantine_candidates").unwrap_or(0))
        .with_repairable_legacy_metadata_lessons(
            json_u64_field(body, "repairable_legacy_metadata_lessons").unwrap_or(0),
        )
        .with_legacy_metadata_without_clean_gist(
            json_u64_field(body, "legacy_metadata_without_clean_gist").unwrap_or(0),
        )
        .with_duplicate_outputs(
            audit_index_u64(body, index_report, "duplicate_outputs").unwrap_or(0),
        )
}

fn context_rot_gate_from_config(config: &Config) -> ContextRotGate {
    ContextRotGate {
        max_noisy_records: config.max_index_noisy_records,
        max_noise_penalty: config.max_index_noise_penalty,
        max_quarantine_candidates: config.max_quarantine_candidates,
        max_repairable_legacy_metadata_lessons: config.max_repairable_legacy_records,
        max_legacy_metadata_without_clean_gist: config.max_legacy_metadata_without_clean_gist,
        max_duplicate_outputs: u64::MAX,
    }
}

fn audit_index_u64(body: &str, index_report: Option<&str>, field: &str) -> Option<u64> {
    index_report
        .and_then(|index_report| json_u64_field(index_report, field))
        .or_else(|| json_u64_field(body, field))
}

fn audit_index_f64(body: &str, index_report: Option<&str>, field: &str) -> Option<f64> {
    index_report
        .and_then(|index_report| json_f64_field(index_report, field))
        .or_else(|| json_f64_field(body, field))
}

fn audit_index_bool(body: &str, index_report: Option<&str>, field: &str) -> Option<bool> {
    index_report
        .and_then(|index_report| json_bool_field(index_report, field))
        .or_else(|| json_bool_field(body, field))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ActiveRuntimeRequest {
    request_id: u64,
    endpoint: String,
    elapsed_ms: u64,
    cancel_requested: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeRepairAttempt {
    meta: String,
    repair_factor_released: bool,
}

struct RuntimeRepairWatchdog {
    done: Arc<(Mutex<bool>, Condvar)>,
    attempt: Arc<Mutex<Option<RuntimeRepairAttempt>>>,
    handle: thread::JoinHandle<()>,
}

impl RuntimeRepairWatchdog {
    fn start(config: &Config, round: usize) -> Option<Self> {
        let done = Arc::new((Mutex::new(false), Condvar::new()));
        let attempt = Arc::new(Mutex::new(None));
        let done_for_thread = Arc::clone(&done);
        let attempt_for_thread = Arc::clone(&attempt);
        let backend = config.backend.clone();
        let stream_timeout_secs = config.timeout_secs.max(1);
        let repair_timeout_secs = runtime_repair_http_timeout_secs(config.timeout_secs);
        let handle = thread::Builder::new()
            .name(format!("evolution-runtime-repair-{round}"))
            .spawn(move || {
                let (lock, cvar) = &*done_for_thread;
                let Ok(done_guard) = lock.lock() else {
                    return;
                };
                let Ok((done_guard, wait_result)) = cvar.wait_timeout_while(
                    done_guard,
                    Duration::from_secs(stream_timeout_secs),
                    |done| !*done,
                ) else {
                    return;
                };
                if *done_guard || !wait_result.timed_out() {
                    return;
                }
                drop(done_guard);

                let repair = release_runtime_request_repair_factor(
                    &backend,
                    repair_timeout_secs,
                    round,
                    RUNTIME_REPAIR_TIMEOUT_REASON,
                );
                if let Ok(mut slot) = attempt_for_thread.lock() {
                    *slot = Some(repair);
                }
            })
            .ok()?;

        Some(Self {
            done,
            attempt,
            handle,
        })
    }

    fn finish(self) -> Option<RuntimeRepairAttempt> {
        let (lock, cvar) = &*self.done;
        if let Ok(mut done) = lock.lock() {
            *done = true;
            cvar.notify_all();
        }
        let _ = self.handle.join();
        self.attempt
            .lock()
            .ok()
            .and_then(|mut attempt| attempt.take())
    }
}

fn runtime_repair_http_timeout_secs(timeout_secs: u64) -> u64 {
    timeout_secs.max(1).min(RUNTIME_REPAIR_HTTP_TIMEOUT_SECS)
}

fn runtime_repair_stream_error_reason(error: &str) -> &'static str {
    let lower = error.to_ascii_lowercase();
    if lower.contains("timed out") || lower.contains("timeout") {
        RUNTIME_REPAIR_TIMEOUT_REASON
    } else {
        RUNTIME_REPAIR_STREAM_ERROR_REASON
    }
}

fn runtime_repair_retag_label(round: usize) -> String {
    format!("repair_factor:runtime_splice;source=evolution_loop;round={round}")
}

fn runtime_repair_cancel_body(request_id: u64, reason: &str, retag_label: &str) -> String {
    format!(
        "{{\"request_id\":{},\"reason\":{},\"retag_label\":{}}}",
        request_id,
        json_string(reason),
        json_string(retag_label)
    )
}

fn active_runtime_requests(body: &str) -> Vec<ActiveRuntimeRequest> {
    json_array_field(body, "active_requests")
        .map(|array| {
            parse_json_object_array(&array)
                .into_iter()
                .filter_map(|request| {
                    Some(ActiveRuntimeRequest {
                        request_id: json_u64_field(&request, "request_id")?,
                        endpoint: json_string_field(&request, "endpoint")?,
                        elapsed_ms: json_u64_field(&request, "elapsed_ms").unwrap_or(0),
                        cancel_requested: json_bool_field(&request, "cancel_requested")
                            .unwrap_or(false),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn select_runtime_repair_target(
    requests: &[ActiveRuntimeRequest],
) -> Option<&ActiveRuntimeRequest> {
    requests.iter().find(|request| {
        is_business_cycle_runtime_endpoint(&request.endpoint) && !request.cancel_requested
    })
}

fn is_business_cycle_runtime_endpoint(endpoint: &str) -> bool {
    endpoint.trim_matches('/') == "business-cycle-stream"
        || endpoint
            .trim_end_matches('/')
            .ends_with("/business-cycle-stream")
}

fn release_runtime_request_repair_factor(
    backend: &str,
    timeout_secs: u64,
    round: usize,
    reason: &str,
) -> RuntimeRepairAttempt {
    let timeout_secs = runtime_repair_http_timeout_secs(timeout_secs);
    let health = match http::get(backend, "/health", timeout_secs) {
        Ok(response) => response,
        Err(error) => {
            return RuntimeRepairAttempt {
                meta: format!(
                    "runtime_repair_factor release_attempt=false repair_factor={} round={} reason={} status=health_error error={}",
                    RUNTIME_REQUEST_REPAIR_FACTOR,
                    round,
                    reason,
                    preview_text(&error, 160)
                ),
                repair_factor_released: false,
            };
        }
    };
    if !(200..300).contains(&health.status) {
        return RuntimeRepairAttempt {
            meta: format!(
                "runtime_repair_factor release_attempt=false repair_factor={} round={} reason={} status=health_http_{} body={}",
                RUNTIME_REQUEST_REPAIR_FACTOR,
                round,
                reason,
                health.status,
                preview_text(&health.body, 160)
            ),
            repair_factor_released: false,
        };
    }

    let active_requests = active_runtime_requests(&health.body);
    let Some(target) = select_runtime_repair_target(&active_requests) else {
        return RuntimeRepairAttempt {
            meta: format!(
                "runtime_repair_factor release_attempt=false repair_factor={} round={} reason={} status=no_active_business_cycle_request active_requests={}",
                RUNTIME_REQUEST_REPAIR_FACTOR,
                round,
                reason,
                active_requests.len()
            ),
            repair_factor_released: false,
        };
    };

    let retag_label = runtime_repair_retag_label(round);
    let cancel_body = runtime_repair_cancel_body(target.request_id, reason, &retag_label);
    let cancel = match http::post_json(backend, "/v1/requests/cancel", &cancel_body, timeout_secs) {
        Ok(response) => response,
        Err(error) => {
            return RuntimeRepairAttempt {
                meta: format!(
                    "runtime_repair_factor release_attempt=true repair_factor_released=false round={} request_id={} endpoint={} elapsed_ms={} reason={} repair_factor={} retag_label={} status=cancel_error error={}",
                    round,
                    target.request_id,
                    target.endpoint,
                    target.elapsed_ms,
                    reason,
                    RUNTIME_REQUEST_REPAIR_FACTOR,
                    retag_label,
                    preview_text(&error, 160)
                ),
                repair_factor_released: false,
            };
        }
    };
    let repair_factor_released = (200..300).contains(&cancel.status)
        && json_bool_field(&cancel.body, "repair_factor_released").unwrap_or(false);
    let repair_factor = json_string_field(&cancel.body, "repair_factor")
        .unwrap_or_else(|| RUNTIME_REQUEST_REPAIR_FACTOR.to_owned());
    let target_active = json_bool_field(&cancel.body, "target_active").unwrap_or(false);
    let retag_applied = json_bool_field(&cancel.body, "retag_applied").unwrap_or(false);
    let persistent_writes = json_bool_field(&cancel.body, "persistent_writes").unwrap_or(false);

    RuntimeRepairAttempt {
        meta: format!(
            "runtime_repair_factor release_attempt=true repair_factor_released={} round={} request_id={} endpoint={} elapsed_ms={} reason={} repair_factor={} retag_label={} http_status={} target_active={} retag_applied={} persistent_writes={}",
            repair_factor_released,
            round,
            target.request_id,
            target.endpoint,
            target.elapsed_ms,
            reason,
            repair_factor,
            retag_label,
            cancel.status,
            target_active,
            retag_applied,
            persistent_writes
        ),
        repair_factor_released,
    }
}

fn run_round(
    config: &Config,
    rust_check_code: Option<&str>,
    round: usize,
    case_name: &str,
    prompt: &str,
    pool_request_plan: Option<&PoolRequestPlan>,
    pool_stage_dispatch_plans: &[PoolStageDispatchPlan],
) -> RoundOutcome {
    let body = business_cycle_body(
        config,
        rust_check_code,
        case_name,
        prompt,
        pool_request_plan,
        pool_stage_dispatch_plans,
    );
    let mut outcome = RoundOutcome {
        success: false,
        error: None,
        runtime_tokens: None,
        runtime_model: None,
        answer: None,
        elapsed_ms: None,
        business_cycle_passed: None,
        feedback_applied: None,
        rust_check_checked: None,
        rust_check_passed: None,
        rust_check_feedback_applied: None,
        self_improve_passed: None,
        state_gate_checked: None,
        state_gate_passed: None,
        trace_gate_checked: None,
        trace_gate_passed: None,
        delta_chars: 0,
        stages: Vec::new(),
        meta: Vec::new(),
        final_json: None,
    };

    let runtime_repair_watchdog = RuntimeRepairWatchdog::start(config, round);
    let result = http::post_event_stream(
        &config.backend,
        "/v1/business-cycle-stream",
        &body,
        config.timeout_secs,
        &mut |event, data| {
            match event {
                "stage" => {
                    println!("[round {round}] stage {data}");
                    outcome.stages.push(data.to_owned());
                }
                "meta" => {
                    println!("[round {round}] meta {data}");
                    outcome.meta.push(data.to_owned());
                }
                "delta" => {
                    outcome.delta_chars += data.chars().count();
                    if config.show_delta {
                        print!("{data}");
                    }
                }
                "final" => {
                    outcome.runtime_tokens = json_u64_field(data, "runtime_token_count");
                    outcome.runtime_model = json_string_field(data, "runtime_model");
                    outcome.answer = json_string_field(data, "answer");
                    outcome.elapsed_ms = json_u64_field(data, "elapsed_ms");
                    outcome.business_cycle_passed = json_bool_field(data, "passed");
                    outcome.feedback_applied = json_u64_field(data, "feedback_applied");
                    outcome.rust_check_checked = json_bool_field(data, "rust_check_checked");
                    outcome.rust_check_passed = json_bool_field(data, "rust_check_passed");
                    outcome.rust_check_feedback_applied =
                        json_u64_field(data, "rust_check_feedback_applied");
                    outcome.self_improve_passed = json_bool_field(data, "self_improve_passed");
                    outcome.state_gate_checked = json_bool_field(data, "state_gate_checked");
                    outcome.state_gate_passed = json_bool_field(data, "state_gate_passed");
                    outcome.trace_gate_checked = json_bool_field(data, "trace_gate_checked");
                    outcome.trace_gate_passed = json_bool_field(data, "trace_gate_passed");
                    let ok = json_bool_field(data, "ok").unwrap_or(false);
                    let passed = outcome.business_cycle_passed.unwrap_or(false);
                    let runtime_failure = runtime_response_failure_reason(data);
                    outcome.success = ok && passed && runtime_failure.is_none();
                    if outcome.success {
                        outcome.error = None;
                    } else if let Some(reason) = runtime_failure {
                        outcome.error = Some(reason);
                    } else {
                        outcome.error = Some(final_failure_reason(data, ok, passed));
                    }
                    outcome.final_json = Some(data.to_owned());
                }
                "error" => {
                    outcome.error = Some(data.to_owned());
                }
                "done" | "status" => {
                    println!("[round {round}] {event} {data}");
                }
                other => {
                    outcome.meta.push(format!("{other}: {data}"));
                }
            }
            Ok(())
        },
    );

    if config.show_delta && outcome.delta_chars > 0 {
        println!();
    }
    let runtime_repair_watchdog_attempt =
        runtime_repair_watchdog.and_then(RuntimeRepairWatchdog::finish);
    let runtime_repair_watchdog_released = runtime_repair_watchdog_attempt
        .as_ref()
        .is_some_and(|attempt| attempt.repair_factor_released);
    if let Some(attempt) = runtime_repair_watchdog_attempt {
        println!("[round {round}] meta {}", attempt.meta);
        outcome.meta.push(attempt.meta);
    }
    if let Err(error) = result {
        if !runtime_repair_watchdog_released {
            let reason = runtime_repair_stream_error_reason(&error);
            let attempt = release_runtime_request_repair_factor(
                &config.backend,
                config.timeout_secs,
                round,
                reason,
            );
            println!("[round {round}] meta {}", attempt.meta);
            outcome.meta.push(attempt.meta);
        }
        outcome.error = Some(error);
    }
    if outcome.error.is_none() && outcome.final_json.is_none() {
        outcome.error = Some("stream ended without final event".to_owned());
    }
    if outcome.error.is_some() {
        outcome.success = false;
    }
    outcome
}

fn business_cycle_body(
    config: &Config,
    rust_check_code: Option<&str>,
    case_name: &str,
    prompt: &str,
    pool_request_plan: Option<&PoolRequestPlan>,
    pool_stage_dispatch_plans: &[PoolStageDispatchPlan],
) -> String {
    let gate = if config.business_gate {
        "\"gate\":\"business_cycle\","
    } else {
        ""
    };
    let rust_check = rust_check_json_fields(config, rust_check_code, case_name);
    let max_tokens = pool_request_plan
        .map(|plan| plan.effective_max_tokens)
        .unwrap_or(config.max_tokens);
    let pool_dispatch = pool_request_plan
        .map(PoolRequestPlan::request_json_field)
        .unwrap_or_default();
    let pool_stage_dispatch = pool_stage::request_json_field(pool_stage_dispatch_plans);
    format!(
        "{{\"prompt\":{},\"profile\":{},\"max_tokens\":{},\"case\":{},\"feedback_amount\":{:.3},\"self_improve\":true,\"self_improve_limit\":{},{}\"state_gate\":{},\"trace_gate\":{}{}{}{}}}",
        json_string(prompt),
        json_string(&config.profile),
        max_tokens,
        json_string(case_name),
        config.feedback_amount,
        config.self_improve_limit,
        gate,
        config.business_gate,
        config.trace_gate,
        pool_dispatch,
        pool_stage_dispatch,
        rust_check
    )
}

fn pool_prompt_context_char_limit(plan: Option<&PoolRequestPlan>) -> Option<usize> {
    let plan = plan?;
    if !plan.can_accept_low_priority_task || plan.selected_role.eq_ignore_ascii_case("quality") {
        return None;
    }
    let context_window = plan
        .context_window
        .and_then(|value| usize::try_from(value).ok())?;
    if context_window >= 8_192 {
        return None;
    }
    let output_budget = plan.effective_max_tokens.max(1);
    let reserved_tokens = output_budget.saturating_add(512).min(context_window / 2);
    let prompt_token_budget = context_window.saturating_sub(reserved_tokens).max(512);
    Some(
        prompt_token_budget
            .saturating_mul(3)
            .div_ceil(2)
            .clamp(1_200, 6_000),
    )
}

fn load_rust_check_code(config: &Config) -> Result<Option<String>, String> {
    match (&config.rust_check_code, &config.rust_check_file) {
        (Some(_), Some(_)) => {
            Err("use only one of --rust-check-code or --rust-check-file".to_owned())
        }
        (Some(code), None) => Ok(Some(code.clone())),
        (None, Some(path)) => fs::read_to_string(path)
            .map(|code| Some(code))
            .map_err(|error| format!("read Rust check file {} failed: {error}", path.display())),
        (None, None) => Ok(None),
    }
}

fn rust_check_json_fields(
    config: &Config,
    rust_check_code: Option<&str>,
    case_name: &str,
) -> String {
    let Some(code) = rust_check_code.filter(|code| !code.trim().is_empty()) else {
        return String::new();
    };
    let rust_case = config
        .rust_check_case
        .as_deref()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("{case_name}-rust-check"));
    format!(
        ",\"rust_check_code\":{},\"rust_check_edition\":{},\"rust_check_case\":{}",
        json_string(code),
        json_string(&config.rust_check_edition),
        json_string(&rust_case)
    )
}

fn final_failure_reason(data: &str, ok: bool, passed: bool) -> String {
    let mut reasons = Vec::new();
    if !ok {
        reasons.push("ok=false".to_owned());
    }
    if !passed {
        reasons.push("business_cycle.passed=false".to_owned());
    }
    for field in [
        "feedback_passed",
        "rust_check_passed",
        "self_improve_passed",
        "state_gate_passed",
        "trace_gate_passed",
    ] {
        if json_bool_field(data, field) == Some(false) {
            reasons.push(format!("{field}=false"));
        }
    }
    if json_u64_field(data, "feedback_applied") == Some(0) {
        reasons.push("feedback_applied=0".to_owned());
    }
    if reasons.is_empty() {
        "final payload did not pass".to_owned()
    } else {
        reasons.join(", ")
    }
}

fn runtime_response_failure_reason(data: &str) -> Option<String> {
    let mut reasons = Vec::new();
    if json_u64_field(data, "runtime_token_count").unwrap_or(0) == 0 {
        reasons.push("runtime_token_count=0".to_owned());
    }
    if json_string_field(data, "runtime_model").is_none() {
        reasons.push("runtime_model missing".to_owned());
    }
    if json_string_field(data, "answer")
        .as_deref()
        .is_some_and(|answer| {
            answer
                .to_ascii_lowercase()
                .contains("runtime backend error")
        })
    {
        reasons.push("answer contains runtime backend error".to_owned());
    }
    (!reasons.is_empty()).then(|| format!("runtime response gate failed: {}", reasons.join(", ")))
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn option_u64_text(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "?".to_owned())
}

fn option_i32_text(value: Option<i32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unknown".to_owned())
}

fn option_usize_text(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "disabled".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::{Arc, Mutex};

    #[test]
    fn business_cycle_body_carries_budget_and_gates() {
        let config = Config {
            max_tokens: 8192,
            self_improve_limit: 3,
            feedback_amount: 0.7,
            ..Config::default()
        };

        let body = business_cycle_body(&config, None, "case-a", "prompt", None, &[]);

        assert!(body.contains("\"max_tokens\":8192"));
        assert!(body.contains("\"self_improve_limit\":3"));
        assert!(body.contains("\"state_gate\":false"));
        assert!(body.contains("\"trace_gate\":false"));
        assert!(body.contains("\"case\":\"case-a\""));
    }

    #[test]
    fn business_cycle_body_can_request_strict_business_gate() {
        let config = Config {
            business_gate: true,
            trace_gate: true,
            ..Config::default()
        };

        let body = business_cycle_body(&config, None, "case-a", "prompt", None, &[]);

        assert!(body.contains("\"gate\":\"business_cycle\""));
        assert!(body.contains("\"state_gate\":true"));
        assert!(body.contains("\"trace_gate\":true"));
    }

    #[test]
    fn runtime_repair_target_selects_uncancelled_business_cycle_request() {
        let health = r#"{"active_requests":[{"request_id":1,"endpoint":"generate","elapsed_ms":10,"cancel_requested":false},{"request_id":2,"endpoint":"business-cycle-stream","elapsed_ms":901000,"cancel_requested":true},{"request_id":3,"endpoint":"/v1/business-cycle-stream","elapsed_ms":902000,"cancel_requested":false}]}"#;

        let active = active_runtime_requests(health);
        let target = select_runtime_repair_target(&active).expect("repair target");

        assert_eq!(target.request_id, 3);
        assert_eq!(target.elapsed_ms, 902000);
        assert!(!target.cancel_requested);
    }

    #[test]
    fn runtime_repair_factor_release_posts_cancel_and_retags_active_request() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let backend = listener.local_addr().unwrap().to_string();
        let requests = Arc::new(Mutex::new(Vec::<String>::new()));
        let requests_for_server = Arc::clone(&requests);
        let server = thread::spawn(move || {
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().unwrap();
                let request = read_test_http_request(&mut stream);
                let body = if request.starts_with("GET /health") {
                    r#"{"ok":true,"engine_busy":true,"active_engine_requests":1,"active_requests":[{"request_id":34,"endpoint":"business-cycle-stream","elapsed_ms":901234,"cancel_requested":false,"repair_factor":null,"retag_label":null,"cancel_reason":null}]}"#.to_owned()
                } else {
                    r#"{"ok":true,"request_id":34,"target_request_id":34,"target_active":true,"target_endpoint":"business-cycle-stream","repair_factor_released":true,"repair_factor":"runtime_request_splice","retag_applied":true,"retag_label":"repair_factor:runtime_splice;source=evolution_loop;round=748","reason":"evolution_loop_stream_timeout","cooperative_only":true,"persistent_writes":false}"#.to_owned()
                };
                requests_for_server.lock().unwrap().push(request);
                write_test_http_json(&mut stream, &body);
            }
        });

        let attempt =
            release_runtime_request_repair_factor(&backend, 2, 748, RUNTIME_REPAIR_TIMEOUT_REASON);
        server.join().unwrap();

        assert!(attempt.repair_factor_released, "{}", attempt.meta);
        assert!(attempt.meta.contains("request_id=34"));
        assert!(
            attempt
                .meta
                .contains("repair_factor=runtime_request_splice")
        );
        assert!(
            attempt.meta.contains(
                "retag_label=repair_factor:runtime_splice;source=evolution_loop;round=748"
            )
        );
        assert!(attempt.meta.contains("persistent_writes=false"));

        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests[1].starts_with("POST /v1/requests/cancel"));
        assert!(requests[1].contains("\"request_id\":34"));
        assert!(requests[1].contains("\"reason\":\"evolution_loop_stream_timeout\""));
        assert!(requests[1].contains(
            "\"retag_label\":\"repair_factor:runtime_splice;source=evolution_loop;round=748\""
        ));
    }

    #[test]
    fn business_cycle_body_uses_selected_pool_worker_budget() {
        let config = Config {
            max_tokens: 4096,
            ..Config::default()
        };
        let decision = crate::pool_dispatch::PoolDispatchDecision {
            selected_role: "summary".to_owned(),
            selected_port: Some(8687),
            selected_base_url: Some("http://127.0.0.1:8687".to_owned()),
            context_window: Some(8192),
            default_max_tokens: Some(768),
            runtime_backend: Some("llama.cpp".to_owned()),
            runtime_device: Some("metal".to_owned()),
            runtime_accelerator: Some("metal".to_owned()),
            gpu_layers: Some(99),
            can_accept_low_priority_task: true,
            evidence: "route_allowed:true".to_owned(),
        };
        let pool_request_plan = PoolRequestPlan::from_decision(&config, &decision);

        let body = business_cycle_body(
            &config,
            None,
            "case-a",
            "prompt",
            Some(&pool_request_plan),
            &[],
        );

        assert!(body.contains("\"max_tokens\":768"));
        assert!(body.contains("\"pool_dispatch\""));
        assert!(body.contains("\"selected_role\":\"summary\""));
        assert!(body.contains("\"selected_base_url\":\"http://127.0.0.1:8687\""));
        assert!(body.contains("\"runtime_backend\":\"llama.cpp\""));
        assert!(body.contains("\"runtime_device\":\"metal\""));
        assert!(body.contains("\"runtime_accelerator\":\"metal\""));
        assert!(body.contains("\"gpu_layers\":99"));
        assert!(body.contains("\"configured_max_tokens\":4096"));
        assert!(body.contains("\"effective_max_tokens\":768"));
        assert!(body.contains("\"max_tokens_clamped\":true"));
    }

    #[test]
    fn business_cycle_body_can_attach_stage_dispatch_plan() {
        let config = Config {
            max_tokens: 4096,
            ..Config::default()
        };
        let stage_plan = PoolStageDispatchPlan {
            task_kind: "review".to_owned(),
            selected_role: "review".to_owned(),
            selected_port: Some(8688),
            selected_base_url: Some("http://127.0.0.1:8688".to_owned()),
            context_window: Some(8192),
            default_max_tokens: Some(1024),
            runtime_backend: Some("llama.cpp".to_owned()),
            runtime_device: Some("metal".to_owned()),
            runtime_accelerator: Some("metal".to_owned()),
            gpu_layers: Some(99),
            configured_max_tokens: 4096,
            effective_max_tokens: 1024,
            max_tokens_clamped: true,
            can_accept_low_priority_task: true,
        };

        let body = business_cycle_body(&config, None, "case-a", "prompt", None, &[stage_plan]);

        assert!(body.contains("\"max_tokens\":4096"));
        assert!(body.contains("\"pool_stage_dispatch\""));
        assert!(body.contains("\"task_kind\":\"review\""));
        assert!(body.contains("\"selected_role\":\"review\""));
        assert!(body.contains("\"selected_base_url\":\"http://127.0.0.1:8688\""));
        assert!(body.contains("\"effective_max_tokens\":1024"));
        assert!(body.contains("\"max_tokens_clamped\":true"));
    }

    #[test]
    fn low_priority_small_window_primary_route_limits_prompt_context() {
        let mut plan = PoolRequestPlan {
            selected_role: "review".to_owned(),
            selected_port: Some(8688),
            selected_base_url: Some("http://127.0.0.1:8688".to_owned()),
            context_window: Some(4096),
            default_max_tokens: Some(1024),
            runtime_backend: Some("llama.cpp".to_owned()),
            runtime_device: Some("cpu".to_owned()),
            runtime_accelerator: Some("accelerate".to_owned()),
            gpu_layers: Some(0),
            configured_max_tokens: 262_144,
            effective_max_tokens: 96,
            max_tokens_clamped: true,
            can_accept_low_priority_task: true,
        };

        let limited = pool_prompt_context_char_limit(Some(&plan));
        assert!(limited.is_some_and(|limit| (1_200..=6_000).contains(&limit)));

        plan.context_window = Some(8192);
        assert_eq!(pool_prompt_context_char_limit(Some(&plan)), None);

        plan.context_window = Some(4096);
        plan.selected_role = "quality".to_owned();
        plan.can_accept_low_priority_task = false;
        assert_eq!(pool_prompt_context_char_limit(Some(&plan)), None);
    }

    #[test]
    fn final_failure_reason_names_failed_gate_fields() {
        let reason = final_failure_reason(
            "{\"ok\":true,\"business_cycle\":{\"passed\":false,\"feedback_passed\":true,\"state_gate_passed\":false}}",
            true,
            false,
        );

        assert!(reason.contains("business_cycle.passed=false"));
        assert!(reason.contains("state_gate_passed=false"));
    }

    #[test]
    fn runtime_response_gate_accepts_real_runtime_tokens() {
        let reason = runtime_response_failure_reason(
            "{\"ok\":true,\"generate\":{\"answer\":\"ok\",\"runtime_model\":\"google/gemma\",\"runtime_token_count\":12}}",
        );

        assert!(reason.is_none(), "{reason:?}");
    }

    #[test]
    fn runtime_response_gate_blocks_wrapped_backend_errors() {
        let reason = runtime_response_failure_reason(
            "{\"ok\":true,\"generate\":{\"answer\":\"Runtime backend error: failed to read response\",\"runtime_model\":null,\"runtime_token_count\":0}}",
        )
        .unwrap();

        assert!(reason.contains("runtime_token_count=0"));
        assert!(reason.contains("runtime_model missing"));
        assert!(reason.contains("runtime backend error"));
    }

    #[test]
    fn health_gate_waits_when_backend_is_busy() {
        let body = "{\"ok\":true,\"engine_busy\":true,\"active_engine_requests\":1}";
        let busy = json_bool_field(body, "engine_busy").unwrap_or(false)
            || json_u64_field(body, "active_engine_requests").unwrap_or(0) > 0;

        assert!(busy);
    }

    #[test]
    fn health_gate_requires_min_runtime_context_when_configured() {
        let config = Config {
            min_runtime_context_window: Some(262_144),
            ..Config::default()
        };
        let failures = health_gate_failures(
            &config,
            "{\"ok\":true,\"gemma_runtime_reachable\":true,\"safe_device_ok\":true,\"clean\":true,\"readiness_ok\":true,\"gemma_runtime_context_window\":4096}",
        );

        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("n_ctx 4096 below required 262144"))
        );
    }

    #[test]
    fn health_gate_accepts_runtime_context_that_meets_requirement() {
        let config = Config {
            min_runtime_context_window: Some(262_144),
            ..Config::default()
        };
        let failures = health_gate_failures(
            &config,
            "{\"ok\":true,\"gemma_runtime_reachable\":true,\"safe_device_ok\":true,\"clean\":true,\"readiness_ok\":true,\"gemma_runtime_context_window\":262144}",
        );

        assert!(failures.is_empty());
    }

    #[test]
    fn health_gate_retries_missing_context_when_metadata_probe_times_out() {
        let config = Config {
            min_runtime_context_window: Some(262_144),
            ..Config::default()
        };
        let body = "{\"ok\":true,\"gemma_runtime_reachable\":true,\"safe_device_ok\":true,\"clean\":true,\"readiness_ok\":true,\"gemma_runtime_metadata_error\":\"read Gemma metadata response failed: timed out\"}";
        let failures = health_gate_failures(&config, body);

        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("n_ctx is missing"))
        );
        assert_eq!(
            health_gate_retry_reason(&config, body, &failures),
            Some("Gemma runtime metadata is temporarily unavailable")
        );
    }

    #[test]
    fn health_gate_retries_runtime_unreachable_when_metadata_probe_times_out() {
        let config = Config {
            min_runtime_context_window: Some(262_144),
            ..Config::default()
        };
        let body = "{\"ok\":true,\"gemma_runtime_reachable\":false,\"safe_device_ok\":true,\"clean\":true,\"readiness_ok\":false,\"gemma_runtime_metadata_error\":\"connect Gemma runtime metadata endpoint failed: timed out\"}";
        let failures = health_gate_failures(&config, body);

        assert!(
            failures
                .iter()
                .any(|failure| failure == "Gemma runtime is not reachable")
        );
        assert_eq!(
            health_gate_retry_reason(&config, body, &failures),
            Some("Gemma runtime metadata is temporarily unavailable")
        );
    }

    #[test]
    fn health_gate_does_not_retry_real_low_context() {
        let config = Config {
            min_runtime_context_window: Some(262_144),
            ..Config::default()
        };
        let body = "{\"ok\":true,\"gemma_runtime_reachable\":true,\"safe_device_ok\":true,\"clean\":true,\"readiness_ok\":true,\"gemma_runtime_context_window\":4096}";
        let failures = health_gate_failures(&config, body);

        assert!(health_gate_retry_reason(&config, body, &failures).is_none());
    }

    #[test]
    fn health_gate_does_not_retry_hygiene_failure() {
        let config = Config {
            min_runtime_context_window: Some(262_144),
            ..Config::default()
        };
        let body = "{\"ok\":true,\"gemma_runtime_reachable\":true,\"safe_device_ok\":true,\"clean\":false,\"readiness_ok\":true,\"gemma_runtime_metadata_error\":\"read Gemma metadata response failed: timed out\"}";
        let failures = health_gate_failures(&config, body);

        assert!(health_gate_retry_reason(&config, body, &failures).is_none());
    }

    #[test]
    fn runtime_health_summary_shows_model_and_context_window() {
        let summary = runtime_health_summary(
            "{\"gemma_runtime_model\":\"gemma-4-12b-it-Q8_0.gguf\",\"gemma_runtime_context_window\":262144,\"gemma_runtime_train_context_window\":262144}",
        );

        assert!(summary.contains("model=gemma-4-12b-it-Q8_0.gguf"));
        assert!(summary.contains("n_ctx=262144/262144"));
    }

    #[test]
    fn budget_state_tracks_tokens_runtime_and_no_feedback() {
        let mut budget = BudgetState::default();
        let record = RoundRecord {
            round: 1,
            case_name: "case".to_owned(),
            prompt: "prompt".to_owned(),
            started_unix: 1,
            finished_unix: 2,
            success: true,
            error: None,
            runtime_tokens: Some(10),
            runtime_model: Some("google/gemma".to_owned()),
            answer: Some("ok".to_owned()),
            elapsed_ms: Some(500),
            business_cycle_passed: Some(true),
            feedback_applied: Some(0),
            rust_check_checked: Some(false),
            rust_check_passed: Some(true),
            rust_check_feedback_applied: Some(0),
            validation_checked: Some(false),
            validation_passed: Some(true),
            validation_command_source: None,
            validation_command_safety: None,
            validation_command_preview: None,
            validation_phase: None,
            validation_status_code: None,
            validation_elapsed_ms: None,
            validation_stdout_tail: None,
            validation_stderr_tail: None,
            self_improve_passed: Some(true),
            state_gate_checked: Some(false),
            state_gate_passed: Some(true),
            trace_gate_checked: Some(false),
            trace_gate_passed: Some(true),
            delta_chars: 1,
            stages: Vec::new(),
            meta: Vec::new(),
            allocation_evidence: Vec::new(),
            final_json: None,
        };

        budget.record(&record, 1000);

        assert_eq!(budget.runtime_tokens, 10);
        assert_eq!(budget.last_runtime_tokens, Some(10));
        assert_eq!(budget.observed_runtime_ms, 1000);
        assert_eq!(budget.last_observed_runtime_ms, Some(1000));
        assert_eq!(budget.consecutive_no_feedback_rounds, 1);
    }

    #[test]
    fn budget_state_falls_back_to_model_elapsed_when_wall_time_is_missing() {
        let mut budget = BudgetState::default();
        let record = RoundRecord {
            round: 1,
            case_name: "case".to_owned(),
            prompt: "prompt".to_owned(),
            started_unix: 1,
            finished_unix: 2,
            success: true,
            error: None,
            runtime_tokens: Some(7),
            runtime_model: Some("google/gemma".to_owned()),
            answer: Some("ok".to_owned()),
            elapsed_ms: Some(750),
            business_cycle_passed: Some(true),
            feedback_applied: Some(1),
            rust_check_checked: Some(false),
            rust_check_passed: Some(true),
            rust_check_feedback_applied: Some(0),
            validation_checked: Some(false),
            validation_passed: Some(true),
            validation_command_source: None,
            validation_command_safety: None,
            validation_command_preview: None,
            validation_phase: None,
            validation_status_code: None,
            validation_elapsed_ms: None,
            validation_stdout_tail: None,
            validation_stderr_tail: None,
            self_improve_passed: Some(true),
            state_gate_checked: Some(false),
            state_gate_passed: Some(true),
            trace_gate_checked: Some(false),
            trace_gate_passed: Some(true),
            delta_chars: 1,
            stages: Vec::new(),
            meta: Vec::new(),
            allocation_evidence: Vec::new(),
            final_json: None,
        };

        budget.record(&record, 0);

        assert_eq!(budget.observed_runtime_ms, 750);
        assert_eq!(budget.consecutive_no_feedback_rounds, 0);
    }

    #[test]
    fn budget_stop_reason_names_reached_limits() {
        let config = Config {
            max_total_tokens: Some(10),
            ..Config::default()
        };
        let budget = BudgetState {
            runtime_tokens: 10,
            last_runtime_tokens: Some(10),
            observed_runtime_ms: 0,
            last_observed_runtime_ms: None,
            consecutive_no_feedback_rounds: 0,
        };

        assert_eq!(
            budget_stop_reason(&config, &budget).as_deref(),
            Some("runtime token budget reached (10/10)")
        );
    }

    #[test]
    fn pre_round_budget_stop_allows_first_round_without_token_history() {
        let config = Config {
            max_total_tokens: Some(512),
            ..Config::default()
        };
        let budget = BudgetState {
            runtime_tokens: 0,
            last_runtime_tokens: None,
            observed_runtime_ms: 0,
            last_observed_runtime_ms: None,
            consecutive_no_feedback_rounds: 0,
        };

        assert!(pre_round_budget_stop_reason(&config, &budget).is_none());
    }

    #[test]
    fn pre_round_budget_stop_uses_last_round_tokens_as_forecast() {
        let config = Config {
            max_total_tokens: Some(512),
            ..Config::default()
        };
        let mut budget = BudgetState {
            runtime_tokens: 449,
            last_runtime_tokens: Some(63),
            observed_runtime_ms: 0,
            last_observed_runtime_ms: None,
            consecutive_no_feedback_rounds: 0,
        };
        assert!(pre_round_budget_stop_reason(&config, &budget).is_none());

        budget.runtime_tokens = 451;

        assert_eq!(
            pre_round_budget_stop_reason(&config, &budget).as_deref(),
            Some("runtime token budget would be exceeded by another round (451+63>512)")
        );
    }

    #[test]
    fn pre_round_budget_stop_uses_last_round_wall_time_as_forecast() {
        let config = Config {
            max_runtime_secs: Some(120),
            ..Config::default()
        };
        let mut budget = BudgetState {
            runtime_tokens: 0,
            last_runtime_tokens: None,
            observed_runtime_ms: 50_000,
            last_observed_runtime_ms: Some(60_000),
            consecutive_no_feedback_rounds: 0,
        };
        assert!(pre_round_budget_stop_reason(&config, &budget).is_none());

        budget.observed_runtime_ms = 61_000;

        assert_eq!(
            pre_round_budget_stop_reason(&config, &budget).as_deref(),
            Some("runtime seconds budget would be exceeded by another round (61+60>120)")
        );
    }

    #[test]
    fn pool_lease_skip_stop_reason_respects_limit() {
        let config = Config {
            max_pool_lease_skips: Some(2),
            ..Config::default()
        };

        assert!(pool_lease_skip_stop_reason(&config, 1, "busy").is_none());
        assert_eq!(
            pool_lease_skip_stop_reason(&config, 2, "busy")
                .unwrap()
                .as_str(),
            "stopped after 2 consecutive pool lease skip(s) (limit 2): busy"
        );
    }

    #[test]
    fn pool_lease_skip_stop_reason_can_be_disabled() {
        let config = Config {
            max_pool_lease_skips: None,
            ..Config::default()
        };

        assert!(pool_lease_skip_stop_reason(&config, 20, "busy").is_none());
    }

    #[test]
    fn appends_pool_worker_event_when_budget_fairness_path_is_configured() {
        let path = std::env::temp_dir().join(format!(
            "smartsteam-runner-model-worker-v1-{}.json",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        let config = Config {
            pool_budget_fairness_json_path: Some(path.clone()),
            pool_route_task_kind: "summary".to_owned(),
            ..Config::default()
        };
        let outcome = RoundOutcome {
            success: true,
            error: None,
            runtime_tokens: Some(500),
            runtime_model: Some("gemma-small".to_owned()),
            answer: Some("ok".to_owned()),
            elapsed_ms: Some(1200),
            business_cycle_passed: Some(true),
            feedback_applied: Some(1),
            rust_check_checked: Some(true),
            rust_check_passed: Some(true),
            rust_check_feedback_applied: Some(2),
            self_improve_passed: Some(true),
            state_gate_checked: Some(false),
            state_gate_passed: Some(true),
            trace_gate_checked: Some(false),
            trace_gate_passed: Some(true),
            delta_chars: 2,
            stages: Vec::new(),
            meta: Vec::new(),
            final_json: None,
        };
        let plan = PoolRequestPlan {
            selected_role: "summary".to_owned(),
            selected_port: Some(8687),
            selected_base_url: Some("http://127.0.0.1:8687".to_owned()),
            context_window: Some(8192),
            default_max_tokens: Some(768),
            runtime_backend: Some("llama.cpp".to_owned()),
            runtime_device: Some("metal".to_owned()),
            runtime_accelerator: Some("metal".to_owned()),
            gpu_layers: Some(99),
            configured_max_tokens: 4096,
            effective_max_tokens: 768,
            max_tokens_clamped: true,
            can_accept_low_priority_task: true,
        };

        let meta =
            append_pool_worker_event(&config, 7, "case-7", &outcome, Some(&plan), 1300).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        let summary = pool_artifacts::parse_budget_fairness(&text);

        let meta = meta.unwrap();
        assert!(meta.contains("model_worker_v1"));
        assert!(meta.contains("runtime_device=metal"));
        assert!(meta.contains("answer_chars=2"));
        assert!(meta.contains("answer_approx_tokens=1"));
        assert!(text.contains("\"schema\":\"model_worker_v1\""));
        assert!(text.contains("\"role\":\"summary\""));
        assert!(text.contains("\"execution_state\":\"executed\""));
        assert!(text.contains("\"feedback_applied\":3"));
        assert!(text.contains("\"answer_chars\":2"));
        assert!(text.contains("\"answer_bytes\":2"));
        assert!(text.contains("\"answer_approx_tokens\":1"));
        assert!(text.contains("\"runtime_backend\":\"llama.cpp\""));
        assert!(text.contains("\"runtime_device\":\"metal\""));
        assert!(text.contains("\"runtime_accelerator\":\"metal\""));
        assert!(text.contains("\"gpu_layers\":99"));
        assert_eq!(summary.worker_count, 1);
        assert_eq!(summary.total_runtime_tokens, 500);
        assert_eq!(summary.total_latency_ms, 1200);
        assert_eq!(
            summary.roles[0].runtime_backend.as_deref(),
            Some("llama.cpp")
        );
        assert_eq!(summary.roles[0].runtime_device.as_deref(), Some("metal"));
        assert_eq!(
            summary.roles[0].runtime_accelerator.as_deref(),
            Some("metal")
        );
        assert_eq!(summary.roles[0].gpu_layers, Some(99));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn appends_stage_dispatch_plans_as_planned_worker_events() {
        let path = std::env::temp_dir().join(format!(
            "smartsteam-runner-stage-plans-v1-{}.json",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        let config = Config {
            pool_budget_fairness_json_path: Some(path.clone()),
            ..Config::default()
        };
        let plans = vec![
            PoolStageDispatchPlan {
                task_kind: "summary".to_owned(),
                selected_role: "summary".to_owned(),
                selected_port: Some(8687),
                selected_base_url: Some("http://127.0.0.1:8687".to_owned()),
                context_window: Some(8192),
                default_max_tokens: Some(768),
                runtime_backend: Some("llama.cpp".to_owned()),
                runtime_device: Some("metal".to_owned()),
                runtime_accelerator: Some("metal".to_owned()),
                gpu_layers: Some(99),
                configured_max_tokens: 4096,
                effective_max_tokens: 768,
                max_tokens_clamped: true,
                can_accept_low_priority_task: true,
            },
            PoolStageDispatchPlan {
                task_kind: "test-gate".to_owned(),
                selected_role: "test-gate".to_owned(),
                selected_port: Some(8689),
                selected_base_url: Some("http://127.0.0.1:8689".to_owned()),
                context_window: Some(4096),
                default_max_tokens: Some(512),
                runtime_backend: Some("llama.cpp".to_owned()),
                runtime_device: Some("metal".to_owned()),
                runtime_accelerator: Some("metal".to_owned()),
                gpu_layers: Some(80),
                configured_max_tokens: 4096,
                effective_max_tokens: 512,
                max_tokens_clamped: true,
                can_accept_low_priority_task: true,
            },
        ];

        let meta = append_pool_stage_worker_events(&config, 8, "case-8", &plans)
            .unwrap()
            .unwrap();
        let text = fs::read_to_string(&path).unwrap();
        let summary = pool_artifacts::parse_budget_fairness(&text);

        assert!(meta.contains("planned=2"));
        assert!(meta.contains("summary:summary"));
        assert!(meta.contains("test-gate:test-gate"));
        assert!(text.contains("\"execution_state\":\"planned\""));
        assert!(text.contains("\"role\":\"summary\""));
        assert!(text.contains("\"role\":\"test-gate\""));
        assert_eq!(summary.worker_count, 0);
        assert_eq!(summary.total_runtime_tokens, 0);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn appends_executed_stage_call_as_worker_event() {
        let path = std::env::temp_dir().join(format!(
            "smartsteam-runner-stage-call-v1-{}.json",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        let config = Config {
            pool_budget_fairness_json_path: Some(path.clone()),
            ..Config::default()
        };
        let plan = PoolStageDispatchPlan {
            task_kind: "review".to_owned(),
            selected_role: "review".to_owned(),
            selected_port: Some(8688),
            selected_base_url: Some("http://127.0.0.1:8688".to_owned()),
            context_window: Some(8192),
            default_max_tokens: Some(1024),
            runtime_backend: Some("llama.cpp".to_owned()),
            runtime_device: Some("metal".to_owned()),
            runtime_accelerator: Some("metal".to_owned()),
            gpu_layers: Some(99),
            configured_max_tokens: 4096,
            effective_max_tokens: 1024,
            max_tokens_clamped: true,
            can_accept_low_priority_task: true,
        };
        let result = PoolStageCallResult {
            task_kind: "review".to_owned(),
            ok: true,
            selected_role: Some("review".to_owned()),
            selected_port: Some(8688),
            selected_base_url: Some("http://127.0.0.1:8688".to_owned()),
            answer: Some("review feedback".to_owned()),
            elapsed_ms: Some(222),
            answer_chars: Some(15),
            answer_bytes: Some(15),
            answer_approx_tokens: Some(4),
        };

        append_pool_stage_call_worker_event(&config, 9, "case-9", &plan, &result).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        let summary = pool_artifacts::parse_budget_fairness(&text);

        assert!(text.contains("\"execution_state\":\"executed\""));
        assert!(text.contains("\"role\":\"review\""));
        assert!(text.contains("\"runtime_tokens\":4"));
        assert!(text.contains("\"latency_ms\":222"));
        assert!(text.contains("\"answer_chars\":15"));
        assert!(text.contains("\"answer_approx_tokens\":4"));
        assert_eq!(summary.worker_count, 1);
        assert_eq!(summary.successful_worker_count, 1);
        assert_eq!(summary.feedback_worker_count, 1);
        assert_eq!(summary.total_runtime_tokens, 4);
        assert_eq!(summary.total_latency_ms, 222);
        assert_eq!(summary.roles[0].role, "review");
        assert_eq!(summary.roles[0].runtime_device.as_deref(), Some("metal"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn stage_calls_skip_busy_low_priority_worker_before_http() {
        let dir = std::env::temp_dir().join(format!(
            "smartsteam-runner-stage-call-lease-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        let config = Config {
            backend: "127.0.0.1:9".to_owned(),
            pool_lease_dir: Some(dir.clone()),
            pool_lease_busy_policy: crate::args::PoolLeaseBusyPolicy::SkipLowPriority,
            pool_lease_ttl_secs: 30,
            ..Config::default()
        };
        let plan = PoolStageDispatchPlan {
            task_kind: "summary".to_owned(),
            selected_role: "summary".to_owned(),
            selected_port: Some(8687),
            selected_base_url: Some("http://127.0.0.1:8687".to_owned()),
            context_window: Some(8192),
            default_max_tokens: Some(768),
            runtime_backend: Some("llama.cpp".to_owned()),
            runtime_device: Some("metal".to_owned()),
            runtime_accelerator: Some("metal".to_owned()),
            gpu_layers: Some(99),
            configured_max_tokens: 4096,
            effective_max_tokens: 768,
            max_tokens_clamped: true,
            can_accept_low_priority_task: true,
        };
        let _held_lease =
            pool_lease::acquire_stage(&config, &plan, 9, "case-9", unix_seconds()).unwrap();
        let mut outcome = RoundOutcome {
            success: true,
            error: None,
            runtime_tokens: Some(100),
            runtime_model: Some("gemma".to_owned()),
            answer: Some("primary answer".to_owned()),
            elapsed_ms: Some(10),
            business_cycle_passed: Some(true),
            feedback_applied: Some(1),
            rust_check_checked: Some(false),
            rust_check_passed: Some(true),
            rust_check_feedback_applied: Some(0),
            self_improve_passed: Some(true),
            state_gate_checked: Some(false),
            state_gate_passed: Some(true),
            trace_gate_checked: Some(false),
            trace_gate_passed: Some(true),
            delta_chars: 0,
            stages: Vec::new(),
            meta: Vec::new(),
            final_json: Some("{\"ok\":true}".to_owned()),
        };

        let meta = execute_pool_stage_calls(
            &config,
            9,
            "case-9",
            1_781_770_123,
            None,
            "prompt",
            &mut outcome,
            &[plan],
        )
        .unwrap()
        .unwrap();

        assert!(meta.contains("executed=0"));
        assert!(meta.contains("skipped=1"));
        assert!(
            outcome
                .meta
                .iter()
                .any(|item| item.contains("pool_stage_call_skipped task_kind=summary"))
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn memory_pressure_gate_skips_test_gate_after_prior_stage_skip() {
        let dir = std::env::temp_dir().join(format!(
            "smartsteam-runner-memory-pressure-gate-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        let config = Config {
            backend: "127.0.0.1:9".to_owned(),
            pool_lease_dir: Some(dir.clone()),
            pool_lease_busy_policy: crate::args::PoolLeaseBusyPolicy::SkipLowPriority,
            pool_lease_ttl_secs: 30,
            ..Config::default()
        };
        let summary_plan = PoolStageDispatchPlan {
            task_kind: "summary".to_owned(),
            selected_role: "summary".to_owned(),
            selected_port: Some(8687),
            selected_base_url: Some("http://127.0.0.1:8687".to_owned()),
            context_window: Some(8192),
            default_max_tokens: Some(768),
            runtime_backend: Some("llama.cpp".to_owned()),
            runtime_device: Some("metal".to_owned()),
            runtime_accelerator: Some("metal".to_owned()),
            gpu_layers: Some(99),
            configured_max_tokens: 4096,
            effective_max_tokens: 768,
            max_tokens_clamped: true,
            can_accept_low_priority_task: true,
        };
        let test_gate_plan = PoolStageDispatchPlan {
            task_kind: "test-gate".to_owned(),
            selected_role: "test-gate".to_owned(),
            selected_port: Some(8688),
            selected_base_url: Some("http://127.0.0.1:8688".to_owned()),
            context_window: Some(4096),
            default_max_tokens: Some(768),
            runtime_backend: Some("llama.cpp".to_owned()),
            runtime_device: Some("metal".to_owned()),
            runtime_accelerator: Some("metal".to_owned()),
            gpu_layers: Some(99),
            configured_max_tokens: 4096,
            effective_max_tokens: 768,
            max_tokens_clamped: true,
            can_accept_low_priority_task: true,
        };
        let _held_lease =
            pool_lease::acquire_stage(&config, &summary_plan, 9, "case-9", unix_seconds()).unwrap();
        let mut outcome = RoundOutcome {
            success: true,
            error: None,
            runtime_tokens: Some(100),
            runtime_model: Some("gemma".to_owned()),
            answer: Some("primary answer".to_owned()),
            elapsed_ms: Some(10),
            business_cycle_passed: Some(true),
            feedback_applied: Some(1),
            rust_check_checked: Some(false),
            rust_check_passed: Some(true),
            rust_check_feedback_applied: Some(0),
            self_improve_passed: Some(true),
            state_gate_checked: Some(false),
            state_gate_passed: Some(true),
            trace_gate_checked: Some(false),
            trace_gate_passed: Some(true),
            delta_chars: 0,
            stages: Vec::new(),
            meta: Vec::new(),
            final_json: Some("{\"ok\":true}".to_owned()),
        };

        let meta = execute_pool_stage_calls(
            &config,
            9,
            "case-9",
            1_781_770_123,
            None,
            "prompt",
            &mut outcome,
            &[summary_plan, test_gate_plan],
        )
        .unwrap()
        .unwrap();

        assert!(meta.contains("executed=0"));
        assert!(meta.contains("skipped=2"));
        assert!(meta.contains("completed_roles=quality"));
        assert!(meta.contains("summary:summary skipped"));
        assert!(meta.contains("test-gate:test-gate skipped_memory_pressure"));
        assert!(outcome.meta.iter().any(|item| {
            item.contains("memory_pressure_gate task_kind=test-gate blocked=true")
                && item.contains("prior_stage_skips=1")
                && item.contains("selected_role=test-gate")
                && item.contains("port=8688")
                && item.contains("effective_max_tokens=768")
                && item.contains("max_tokens_clamped=true")
                && item.contains("low_priority=true")
        }));
        assert!(outcome.meta.iter().any(|item| {
            item.contains("pool_stage_call_skipped task_kind=test-gate reason=memory_pressure_gate")
        }));
        assert!(
            outcome
                .meta
                .iter()
                .all(|item| !item.contains("pool_stage_lease task_kind=test-gate"))
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn memory_pressure_gate_only_blocks_test_gate_after_prior_skip() {
        let review_plan = PoolStageDispatchPlan {
            task_kind: "review".to_owned(),
            selected_role: "review".to_owned(),
            selected_port: Some(8688),
            selected_base_url: Some("http://127.0.0.1:8688".to_owned()),
            context_window: Some(4096),
            default_max_tokens: Some(1024),
            runtime_backend: Some("llama.cpp".to_owned()),
            runtime_device: Some("metal".to_owned()),
            runtime_accelerator: Some("metal".to_owned()),
            gpu_layers: Some(99),
            configured_max_tokens: 4096,
            effective_max_tokens: 1024,
            max_tokens_clamped: true,
            can_accept_low_priority_task: true,
        };
        let test_gate_plan = PoolStageDispatchPlan {
            task_kind: "test-gate".to_owned(),
            selected_role: "test-gate".to_owned(),
            selected_port: Some(8688),
            selected_base_url: Some("http://127.0.0.1:8688".to_owned()),
            context_window: Some(4096),
            default_max_tokens: Some(768),
            runtime_backend: Some("llama.cpp".to_owned()),
            runtime_device: Some("metal".to_owned()),
            runtime_accelerator: Some("metal".to_owned()),
            gpu_layers: Some(99),
            configured_max_tokens: 4096,
            effective_max_tokens: 768,
            max_tokens_clamped: true,
            can_accept_low_priority_task: true,
        };

        assert!(memory_pressure_gate_skip_reason(&test_gate_plan, 0).is_none());
        assert!(memory_pressure_gate_skip_reason(&review_plan, 1).is_none());

        let reason = memory_pressure_gate_skip_reason(&test_gate_plan, 1).unwrap();
        assert!(reason.contains("prior_stage_skips=1"));
        assert!(reason.contains("selected_role=test-gate"));
        assert!(reason.contains("port=8688"));
        assert!(reason.contains("effective_max_tokens=768"));
        assert!(reason.contains("max_tokens_clamped=true"));
        assert!(reason.contains("low_priority=true"));
    }

    #[test]
    fn initial_pool_stage_completed_roles_include_quality_and_primary_route_role() {
        let config = Config {
            pool_route_task_kind: "review".to_owned(),
            ..Config::default()
        };
        let mut roles = initial_pool_stage_completed_roles(&config);

        assert_eq!(roles, vec!["quality".to_owned(), "review".to_owned()]);

        push_completed_pool_role(&mut roles, "route");
        push_completed_pool_role(&mut roles, "test");
        push_completed_pool_role(&mut roles, "primary");

        assert_eq!(
            roles,
            vec![
                "quality".to_owned(),
                "review".to_owned(),
                "router".to_owned(),
                "test-gate".to_owned()
            ]
        );
    }

    #[test]
    fn pool_worker_event_marks_helper_route_that_blocks_primary_12b() {
        let config = Config {
            pool_route_task_kind: "review".to_owned(),
            ..Config::default()
        };
        let plan = PoolRequestPlan {
            selected_role: "quality".to_owned(),
            selected_port: Some(8686),
            selected_base_url: Some("http://127.0.0.1:8686".to_owned()),
            context_window: Some(262_144),
            default_max_tokens: Some(262_144),
            runtime_backend: None,
            runtime_device: None,
            runtime_accelerator: None,
            gpu_layers: None,
            configured_max_tokens: 4096,
            effective_max_tokens: 4096,
            max_tokens_clamped: false,
            can_accept_low_priority_task: false,
        };

        assert!(pool_worker_blocks_primary_12b(&config, &plan));

        let quality_config = Config {
            pool_route_task_kind: "quality".to_owned(),
            ..Config::default()
        };
        assert!(!pool_worker_blocks_primary_12b(&quality_config, &plan));
    }

    #[test]
    fn pool_capacity_gate_allows_expansion_ready_status() {
        let path = std::env::temp_dir().join(format!(
            "smartsteam-pool-capacity-pass-{}.json",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        fs::write(
            &path,
            "{\"launch_allowed\":true,\"launch_block_reason\":\"none\",\"chain_classification\":\"prompt_ready\",\"min_context_tokens\":262144,\"capacity\":{\"policy\":\"one_quality_plus_small_helpers\",\"expansion_allowed\":true,\"recommendation\":\"add_review_or_index_worker_after_short_smoke\",\"worker_count\":2,\"healthy_worker_count\":2,\"helper_worker_count\":1,\"healthy_helper_worker_count\":1,\"metal_worker_count\":2,\"cpu_worker_count\":0,\"unknown_runtime_worker_count\":0,\"zero_gpu_layer_worker_count\":0,\"quality_runtime_accelerated\":true},\"workers\":[{\"role\":\"quality\",\"tcp_reachable\":true,\"health_ok\":true},{\"role\":\"summary\",\"tcp_reachable\":true,\"health_ok\":true}]}\n",
        )
        .unwrap();
        let config = Config {
            pool_status_json_path: Some(path.clone()),
            pool_capacity_gate: true,
            ..Config::default()
        };

        assert!(run_pool_capacity_gate(&config).is_ok());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn pool_capacity_gate_blocks_expansion_disallowed_status() {
        let path = std::env::temp_dir().join(format!(
            "smartsteam-pool-capacity-block-{}.json",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        fs::write(
            &path,
            "{\"launch_allowed\":false,\"launch_block_reason\":\"quality_worker_down\",\"chain_classification\":\"quality_worker_down\",\"min_context_tokens\":262144,\"capacity\":{\"policy\":\"one_quality_plus_small_helpers\",\"expansion_allowed\":false,\"recommendation\":\"restore_quality_gate_first\",\"worker_count\":2,\"healthy_worker_count\":1,\"helper_worker_count\":1,\"healthy_helper_worker_count\":1,\"metal_worker_count\":1,\"cpu_worker_count\":0,\"unknown_runtime_worker_count\":0,\"zero_gpu_layer_worker_count\":0,\"quality_runtime_accelerated\":null},\"workers\":[{\"role\":\"quality\",\"tcp_reachable\":false,\"health_ok\":false},{\"role\":\"summary\",\"tcp_reachable\":true,\"health_ok\":true}]}\n",
        )
        .unwrap();
        let config = Config {
            pool_status_json_path: Some(path.clone()),
            pool_capacity_gate: true,
            ..Config::default()
        };

        let error = run_pool_capacity_gate(&config).unwrap_err();
        assert!(error.contains("pool capacity gate failed"));
        assert!(error.contains("expansion_allowed=false"));
        assert!(error.contains("recommendation=restore_quality_gate_first"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn pool_capacity_gate_fails_closed_without_capacity_field() {
        let path = std::env::temp_dir().join(format!(
            "smartsteam-pool-capacity-missing-{}.json",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        fs::write(
            &path,
            "{\"launch_allowed\":true,\"workers\":[{\"role\":\"quality\",\"tcp_reachable\":true,\"health_ok\":true}]}\n",
        )
        .unwrap();
        let config = Config {
            pool_status_json_path: Some(path.clone()),
            pool_capacity_gate: true,
            ..Config::default()
        };

        let error = run_pool_capacity_gate(&config).unwrap_err();
        assert!(error.contains("capacity missing"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn pool_alignment_gate_allows_aligned_manifest_status_and_routes() {
        let dir = std::env::temp_dir().join(format!(
            "smartsteam-pool-alignment-pass-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let manifest = dir.join("pool-manifest.json");
        let status = dir.join("pool-status.json");
        let route = dir.join("pool-route-review.json");
        let summary = dir.join("pool-route-summary.json");
        let index = dir.join("pool-route-index.json");
        let test_gate = dir.join("pool-route-test-gate.json");
        fs::write(
            &manifest,
            "{\"capacity_policy\":{\"policy\":\"one_quality_plus_small_helpers\",\"max_quality_12b_workers\":1,\"quality_role\":\"quality\",\"helper_roles\":[\"summary\",\"review\",\"index\",\"test-gate\"]},\"workers\":[{\"role\":\"quality\",\"port\":8686},{\"role\":\"summary\",\"port\":8687},{\"role\":\"review\",\"port\":8688},{\"role\":\"index\",\"port\":8690},{\"role\":\"test-gate\",\"port\":8689}]}\n",
        )
        .unwrap();
        fs::write(
            &status,
            "{\"workers\":[{\"role\":\"quality\",\"tcp_reachable\":true,\"health_ok\":true},{\"role\":\"summary\",\"tcp_reachable\":true,\"health_ok\":true},{\"role\":\"review\",\"tcp_reachable\":true,\"health_ok\":true},{\"role\":\"index\",\"tcp_reachable\":true,\"health_ok\":true},{\"role\":\"test-gate\",\"tcp_reachable\":true,\"health_ok\":true}]}\n",
        )
        .unwrap();
        fs::write(
            &route,
            "{\"task_kind\":\"review\",\"route_allowed\":true,\"selected_role\":\"review\",\"candidate_workers\":[{\"role\":\"review\",\"health_ok\":true,\"role_ready\":true}]}\n",
        )
        .unwrap();
        fs::write(
            &summary,
            "{\"task_kind\":\"summary\",\"route_allowed\":true,\"selected_role\":\"summary\",\"candidate_workers\":[{\"role\":\"summary\",\"health_ok\":true,\"role_ready\":true}]}\n",
        )
        .unwrap();
        fs::write(
            &index,
            "{\"task_kind\":\"index\",\"route_allowed\":true,\"selected_role\":\"index\",\"candidate_workers\":[{\"role\":\"index\",\"health_ok\":true,\"role_ready\":true}]}\n",
        )
        .unwrap();
        fs::write(
            &test_gate,
            "{\"task_kind\":\"test-gate\",\"route_allowed\":true,\"selected_role\":\"test-gate\",\"candidate_workers\":[{\"role\":\"test-gate\",\"health_ok\":true,\"role_ready\":true}]}\n",
        )
        .unwrap();
        let config = Config {
            pool_manifest_json_path: Some(manifest),
            pool_status_json_path: Some(status),
            pool_route_json_path: Some(route),
            pool_stage_route_task_kinds: vec![
                "summary".to_owned(),
                "index".to_owned(),
                "test-gate".to_owned(),
            ],
            pool_alignment_gate: true,
            ..Config::default()
        };

        assert!(run_pool_alignment_gate(&config).is_ok());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn pool_alignment_gate_blocks_role_and_route_mismatch() {
        let dir = std::env::temp_dir().join(format!(
            "smartsteam-pool-alignment-block-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let manifest = dir.join("pool-manifest.json");
        let status = dir.join("pool-status.json");
        let route = dir.join("pool-route-review.json");
        fs::write(
            &manifest,
            "{\"capacity_policy\":{\"policy\":\"one_quality_plus_small_helpers\",\"max_quality_12b_workers\":1,\"quality_role\":\"quality\",\"helper_roles\":[\"summary\",\"review\",\"index\",\"test-gate\"]},\"workers\":[{\"role\":\"quality\",\"port\":8686},{\"role\":\"summary\",\"port\":8687},{\"role\":\"review\",\"port\":8688}]}\n",
        )
        .unwrap();
        fs::write(
            &status,
            "{\"workers\":[{\"role\":\"quality\",\"tcp_reachable\":true,\"health_ok\":true},{\"role\":\"summary\",\"tcp_reachable\":true,\"health_ok\":true},{\"role\":\"extra\",\"tcp_reachable\":true,\"health_ok\":true}]}\n",
        )
        .unwrap();
        fs::write(
            &route,
            "{\"task_kind\":\"review\",\"route_allowed\":false,\"route_block_reason\":\"worker_down\",\"selected_role\":null,\"candidate_workers\":[{\"role\":\"review\",\"health_ok\":false,\"role_ready\":false}]}\n",
        )
        .unwrap();
        let config = Config {
            pool_manifest_json_path: Some(manifest),
            pool_status_json_path: Some(status),
            pool_route_json_path: Some(route),
            pool_alignment_gate: true,
            ..Config::default()
        };

        let error = run_pool_alignment_gate(&config).unwrap_err();

        assert!(error.contains("pool alignment gate failed"));
        assert!(error.contains("missing_status_roles=review"));
        assert!(error.contains("unplanned_status_roles=extra"));
        assert!(error.contains("route_blocked_or_failed=review"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn pool_budget_fairness_gate_allows_balanced_artifact() {
        let path = std::env::temp_dir().join(format!(
            "smartsteam-pool-budget-fairness-pass-{}.json",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        fs::write(
            &path,
            "{\"workers\":[{\"role\":\"summary\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":300,\"latency_ms\":100},{\"role\":\"review\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":300,\"latency_ms\":100},{\"role\":\"test-gate\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":300,\"latency_ms\":100}]}\n",
        )
        .unwrap();
        let config = Config {
            pool_budget_fairness_json_path: Some(path.clone()),
            pool_budget_fairness_gate: true,
            ..Config::default()
        };

        assert!(run_pool_budget_fairness_gate(&config).is_ok());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn pool_budget_fairness_gate_blocks_unfair_artifact() {
        let path = std::env::temp_dir().join(format!(
            "smartsteam-pool-budget-fairness-block-{}.json",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        fs::write(
            &path,
            "{\"workers\":[{\"role\":\"summary\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":800,\"latency_ms\":100},{\"role\":\"review\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":100,\"latency_ms\":100},{\"role\":\"test-gate\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":100,\"latency_ms\":100,\"blocked_primary_12b\":true}]}\n",
        )
        .unwrap();
        let config = Config {
            pool_budget_fairness_json_path: Some(path.clone()),
            pool_budget_fairness_gate: true,
            ..Config::default()
        };

        let error = run_pool_budget_fairness_gate(&config).unwrap_err();

        assert!(error.contains("pool budget fairness gate failed"));
        assert!(error.contains("blocked primary 12B"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn remote_chain_gate_allows_ready_status() {
        let path = std::env::temp_dir().join(format!(
            "smartsteam-remote-chain-pass-{}.json",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        fs::write(
            &path,
            "{\"readiness\":{\"ready\":true,\"model_api\":true,\"backend\":true,\"web_lab\":true,\"quality_worker\":true,\"model_pool_launch_allowed\":true,\"capacity_expansion_allowed\":true},\"model_pool\":{\"available\":true,\"worker_count\":2,\"healthy_worker_count\":2,\"min_context_tokens\":262144,\"capacity\":{\"recommendation\":\"ready\"}},\"next_step\":\"ready\"}\n",
        )
        .unwrap();
        let config = Config {
            remote_chain_status_json_path: Some(path.clone()),
            remote_chain_gate: true,
            ..Config::default()
        };

        assert!(run_remote_chain_gate(&config).is_ok());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn remote_chain_gate_blocks_unready_status() {
        let path = std::env::temp_dir().join(format!(
            "smartsteam-remote-chain-block-{}.json",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        fs::write(
            &path,
            "{\"readiness\":{\"ready\":false,\"model_api\":false,\"backend\":true,\"web_lab\":true},\"model_pool\":{\"available\":false,\"capacity\":{\"recommendation\":\"restore_quality_gate_first\"}},\"next_step\":\"start-remote\"}\n",
        )
        .unwrap();
        let config = Config {
            remote_chain_status_json_path: Some(path.clone()),
            remote_chain_gate: true,
            ..Config::default()
        };

        let error = run_remote_chain_gate(&config).unwrap_err();

        assert!(error.contains("remote chain gate failed"));
        assert!(error.contains("ready=false"));
        assert!(error.contains("restore_quality_gate_first"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn allocation_evidence_summarizes_pool_artifacts() {
        let pool_manifest = std::env::temp_dir().join(format!(
            "smartsteam-allocation-pool-manifest-{}.json",
            std::process::id()
        ));
        let pool_status = std::env::temp_dir().join(format!(
            "smartsteam-allocation-pool-status-{}.json",
            std::process::id()
        ));
        let pool_route = std::env::temp_dir().join(format!(
            "smartsteam-allocation-pool-route-{}.json",
            std::process::id()
        ));
        let pool_budget = std::env::temp_dir().join(format!(
            "smartsteam-allocation-pool-budget-{}.json",
            std::process::id()
        ));
        let remote_chain = std::env::temp_dir().join(format!(
            "smartsteam-allocation-remote-chain-{}.json",
            std::process::id()
        ));
        let _ = fs::remove_file(&pool_manifest);
        let _ = fs::remove_file(&pool_status);
        let _ = fs::remove_file(&pool_route);
        let _ = fs::remove_file(&pool_budget);
        let _ = fs::remove_file(&remote_chain);
        fs::write(
            &pool_manifest,
            "{\"contract_version\":\"gemma-chain.v1\",\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"manifest_kind\":\"rust-norion.model-pool\",\"capacity_policy\":{\"policy\":\"one_quality_plus_small_helpers\",\"target_host\":\"apple_silicon\",\"avoid_extra_12b\":true,\"max_quality_12b_workers\":1,\"helper_roles\":[\"summary\",\"router\",\"review\",\"index\",\"test-gate\"],\"recommended_launch_order\":[\"quality\",\"summary\",\"router\",\"review\",\"index\",\"test-gate\"]},\"workers\":[{\"role\":\"quality\",\"port\":8686,\"default_context_tokens\":262144,\"default_max_tokens\":262144,\"runtime_backend\":\"llama.cpp\",\"runtime_device\":\"metal\",\"runtime_accelerator\":\"metal\",\"gpu_layers\":99},{\"role\":\"summary\",\"port\":8687,\"default_context_tokens\":8192,\"default_max_tokens\":768,\"runtime_backend\":\"llama.cpp\",\"runtime_device\":\"metal\",\"runtime_accelerator\":\"metal\",\"gpu_layers\":80}]}\n",
        )
        .unwrap();
        fs::write(
            &pool_status,
            "{\"launch_allowed\":false,\"launch_block_reason\":\"quality_worker_down\",\"chain_classification\":\"quality_worker_down\",\"min_context_tokens\":262144,\"workers\":[{\"role\":\"quality\",\"tcp_reachable\":false,\"health_ok\":false},{\"role\":\"summary\",\"tcp_reachable\":true,\"health_ok\":true}]}\n",
        )
        .unwrap();
        fs::write(
            &pool_route,
            "{\"task_kind\":\"review\",\"route_allowed\":false,\"route_block_reason\":\"model_pool_launch_blocked:quality_worker_down\",\"selected_role\":null,\"role_candidates\":[\"review\",\"quality\"],\"candidate_workers\":[{\"role\":\"quality\",\"health_ok\":false,\"role_ready\":false},{\"role\":\"review\",\"health_ok\":true,\"role_ready\":true}]}\n",
        )
        .unwrap();
        fs::write(
            &pool_budget,
            "{\"workers\":[{\"role\":\"summary\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":300,\"latency_ms\":100},{\"role\":\"review\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":300,\"latency_ms\":100},{\"role\":\"test-gate\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":300,\"latency_ms\":100}]}\n",
        )
        .unwrap();
        fs::write(
            &remote_chain,
            "{\"readiness\":{\"ready\":false,\"model_api\":false,\"backend\":true,\"web_lab\":true},\"model_pool\":{\"available\":false,\"capacity\":{\"recommendation\":\"restore_quality_gate_first\"}},\"next_step\":\"start-remote\"}\n",
        )
        .unwrap();
        let config = Config {
            pool_manifest_json_path: Some(pool_manifest.clone()),
            pool_status_json_path: Some(pool_status.clone()),
            pool_route_json_path: Some(pool_route.clone()),
            pool_budget_fairness_json_path: Some(pool_budget.clone()),
            remote_chain_status_json_path: Some(remote_chain.clone()),
            ..Config::default()
        };

        let evidence = load_allocation_evidence(&config).unwrap();

        assert_eq!(evidence.len(), 6);
        assert!(evidence[0].contains("pool_manifest contract_version:gemma-chain.v1"));
        assert!(evidence[0].contains("avoid_extra_12b:true"));
        assert!(
            evidence[0]
                .contains("recommended_launch_order:quality,summary,router,review,index,test-gate")
        );
        assert!(evidence[1].contains("pool_status launch_allowed:false"));
        assert!(evidence[1].contains("available_roles:summary"));
        assert!(evidence[2].contains("remote_chain ready:false"));
        assert!(evidence[2].contains("next_step:start-remote"));
        assert!(evidence[3].contains("pool_route task_kind:review"));
        assert!(evidence[3].contains("route_allowed:false"));
        assert!(evidence[4].contains("pool_alignment alignment_ok:false"));
        assert!(evidence[4].contains("route_blocked_or_failed:review"));
        assert!(evidence[5].contains("pool_budget_fairness workers:3"));
        assert!(evidence[5].contains("allow_pool_expansion:true"));
        let _ = fs::remove_file(pool_manifest);
        let _ = fs::remove_file(pool_status);
        let _ = fs::remove_file(pool_route);
        let _ = fs::remove_file(pool_budget);
        let _ = fs::remove_file(remote_chain);
    }

    #[test]
    fn allocation_evidence_summarizes_stage_route_artifacts() {
        let dir =
            std::env::temp_dir().join(format!("smartsteam-stage-routes-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let primary = dir.join("pool-route-review.json");
        let summary = dir.join("pool-route-summary.json");
        let test_gate = dir.join("pool-route-test-gate.json");
        fs::write(
            &primary,
            "{\"task_kind\":\"review\",\"route_allowed\":true,\"selected_role\":\"review\",\"role_candidates\":[\"review\",\"quality\"],\"candidate_workers\":[{\"role\":\"review\",\"health_ok\":true,\"role_ready\":true,\"base_url\":\"http://127.0.0.1:8688\"}]}\n",
        )
        .unwrap();
        fs::write(
            &summary,
            "{\"task_kind\":\"summary\",\"route_allowed\":true,\"selected_role\":\"summary\",\"role_candidates\":[\"summary\",\"quality\"],\"candidate_workers\":[{\"role\":\"summary\",\"health_ok\":true,\"role_ready\":true,\"base_url\":\"http://127.0.0.1:8687\"}]}\n",
        )
        .unwrap();
        fs::write(
            &test_gate,
            "{\"task_kind\":\"test-gate\",\"route_allowed\":false,\"route_block_reason\":\"test_gate_worker_down\",\"selected_role\":null,\"role_candidates\":[\"test-gate\",\"quality\"],\"candidate_workers\":[{\"role\":\"test-gate\",\"health_ok\":false,\"role_ready\":false}]}\n",
        )
        .unwrap();
        let config = Config {
            pool_route_json_path: Some(primary.clone()),
            pool_stage_route_task_kinds: vec![
                "summary".to_owned(),
                "review".to_owned(),
                "test-gate".to_owned(),
            ],
            ..Config::default()
        };

        let evidence = load_allocation_evidence(&config).unwrap();

        assert_eq!(pool_stage::route_path(&config, "summary"), summary);
        assert_eq!(pool_stage::route_path(&config, "test-gate"), test_gate);
        assert!(
            evidence
                .iter()
                .any(|item| item.contains("pool_route task_kind:review"))
        );
        assert!(
            evidence
                .iter()
                .any(|item| item.contains("pool_stage_route[summary] task_kind:summary"))
        );
        assert!(
            evidence
                .iter()
                .any(|item| item.contains("pool_stage_route[test-gate] task_kind:test-gate"))
        );
        assert!(
            evidence
                .iter()
                .any(|item| item.contains("route_allowed:false"))
        );
        assert!(
            evidence
                .iter()
                .any(|item| item.contains("pool_alignment alignment_ok:false")
                    && item.contains("route_blocked_or_failed:test-gate"))
        );
        assert!(
            evidence
                .iter()
                .all(|item| !item.contains("pool_stage_route[review]"))
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn allocation_evidence_preserves_test_gate_context_buffer_policy() {
        let dir = std::env::temp_dir().join(format!(
            "smartsteam-stage-route-buffer-policy-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let primary = dir.join("pool-route-review.json");
        let test_gate = dir.join("pool-route-test-gate.json");
        fs::write(
            &primary,
            "{\"task_kind\":\"review\",\"route_allowed\":true,\"selected_role\":\"review\",\"role_candidates\":[\"review\",\"quality\"],\"candidate_workers\":[{\"role\":\"review\",\"health_ok\":true,\"role_ready\":true,\"base_url\":\"http://127.0.0.1:8688\"}]}\n",
        )
        .unwrap();
        fs::write(
            &test_gate,
            "{\"task_kind\":\"test-gate\",\"route_allowed\":true,\"selected_context_required_tokens\":3328,\"selected_context_buffer_tokens\":2560,\"selected_context_buffer_policy\":{\"strategy\":\"test_gate_dynamic_upstream_buffer_v1\",\"base_tokens\":2048,\"upstream_role_tokens\":256,\"eligible_upstream_roles\":[\"review\",\"index\"],\"completed_upstream_roles\":[\"review\",\"index\"],\"total_tokens\":2560},\"selected_context_sufficient\":true,\"selected_context_block_reason\":\"none\",\"selected_role\":\"test-gate\",\"role_candidates\":[\"test-gate\",\"review\"],\"candidate_workers\":[{\"role\":\"test-gate\",\"health_ok\":true,\"role_ready\":true,\"base_url\":\"http://127.0.0.1:8688\"}]}\n",
        )
        .unwrap();
        let config = Config {
            pool_route_json_path: Some(primary.clone()),
            pool_stage_route_task_kinds: vec!["test-gate".to_owned()],
            ..Config::default()
        };

        let evidence = load_allocation_evidence(&config).unwrap();
        let test_gate_evidence = evidence
            .iter()
            .find(|item| item.contains("pool_stage_route[test-gate]"))
            .expect("test-gate stage evidence");

        assert_eq!(pool_stage::route_path(&config, "test-gate"), test_gate);
        assert!(test_gate_evidence.contains("selected_context_buffer_tokens:2560"));
        assert!(test_gate_evidence.contains(
            "selected_context_buffer_policy:strategy:test_gate_dynamic_upstream_buffer_v1"
        ));
        assert!(test_gate_evidence.contains("base_tokens:2048"));
        assert!(test_gate_evidence.contains("upstream_role_tokens:256"));
        assert!(test_gate_evidence.contains("eligible_upstream_roles:review,index"));
        assert!(test_gate_evidence.contains("completed_upstream_roles:review,index"));
        assert!(test_gate_evidence.contains("total_tokens:2560"));
        assert!(
            evidence
                .iter()
                .any(|item| item.contains("pool_alignment alignment_ok:false")
                    && item.contains("route_dependency_failures:none"))
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn pool_route_refresh_body_uses_task_kind() {
        let body = pool_route_refresh_body("test-gate", None);

        assert_eq!(body, "{\"task_kind\":\"test-gate\"}");
    }

    #[test]
    fn pool_route_refresh_body_can_include_completed_roles() {
        let completed = vec![
            "quality".to_owned(),
            "review".to_owned(),
            "summary".to_owned(),
        ];
        let body = pool_route_refresh_body("router", Some(&completed));

        assert_eq!(
            body,
            "{\"task_kind\":\"router\",\"completed_roles\":[\"quality\",\"review\",\"summary\"]}"
        );
    }

    #[test]
    fn refresh_pool_artifacts_sends_stage_completed_roles_in_order() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let backend = listener.local_addr().unwrap().to_string();
        let requests = Arc::new(Mutex::new(Vec::<String>::new()));
        let requests_for_server = Arc::clone(&requests);
        let server = thread::spawn(move || {
            for _ in 0..6 {
                let (mut stream, _) = listener.accept().unwrap();
                let request = read_test_http_request(&mut stream);
                let body = test_response_for_request(&request);
                requests_for_server.lock().unwrap().push(request);
                write_test_http_json(&mut stream, &body);
            }
        });
        let dir = std::env::temp_dir().join(format!(
            "smartsteam-refresh-pool-artifacts-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let config = Config {
            backend,
            timeout_secs: 2,
            pool_manifest_json_path: Some(dir.join("pool-manifest.json")),
            pool_status_json_path: Some(dir.join("pool-status.json")),
            pool_route_json_path: Some(dir.join("pool-route-review.json")),
            pool_route_task_kind: "review".to_owned(),
            pool_stage_route_task_kinds: vec![
                "summary".to_owned(),
                "router".to_owned(),
                "index".to_owned(),
            ],
            ..Config::default()
        };

        refresh_pool_artifacts(&config).unwrap();
        server.join().unwrap();
        let requests = requests.lock().unwrap();
        let route_posts = requests
            .iter()
            .filter(|request| request.starts_with("POST /v1/model-pool/route-plan"))
            .collect::<Vec<_>>();

        assert_eq!(route_posts.len(), 4);
        assert!(route_posts[0].contains("{\"task_kind\":\"review\"}"));
        assert!(!route_posts[0].contains("completed_roles"));
        assert!(
            route_posts[1].contains(
                "{\"task_kind\":\"summary\",\"completed_roles\":[\"quality\",\"review\"]}"
            )
        );
        assert!(route_posts[2].contains(
            "{\"task_kind\":\"router\",\"completed_roles\":[\"quality\",\"review\",\"summary\"]}"
        ));
        assert!(route_posts[3].contains(
            "{\"task_kind\":\"index\",\"completed_roles\":[\"quality\",\"review\",\"summary\",\"router\"]}"
        ));
        assert!(
            fs::read_to_string(dir.join("pool-route-index.json"))
                .unwrap()
                .contains("\"selected_role\":\"index\"")
        );
        let _ = fs::remove_dir_all(dir);
    }

    fn read_test_http_request(stream: &mut TcpStream) -> String {
        let mut buffer = Vec::new();
        let mut chunk = [0_u8; 1024];
        loop {
            let read = stream.read(&mut chunk).unwrap();
            assert!(read > 0, "request closed before headers");
            buffer.extend_from_slice(&chunk[..read]);
            if let Some(header_end) = find_test_header_end(&buffer) {
                let header = String::from_utf8_lossy(&buffer[..header_end]).to_string();
                let content_length = header
                    .lines()
                    .find_map(|line| {
                        line.split_once(':').and_then(|(name, value)| {
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().ok())
                                .flatten()
                        })
                    })
                    .unwrap_or(0);
                let body_start = header_end + 4;
                while buffer.len().saturating_sub(body_start) < content_length {
                    let read = stream.read(&mut chunk).unwrap();
                    assert!(read > 0, "request closed before body");
                    buffer.extend_from_slice(&chunk[..read]);
                }
                return String::from_utf8_lossy(&buffer).to_string();
            }
        }
    }

    fn find_test_header_end(bytes: &[u8]) -> Option<usize> {
        bytes.windows(4).position(|window| window == b"\r\n\r\n")
    }

    fn test_response_for_request(request: &str) -> String {
        if request.starts_with("GET /v1/model-pool/manifest") {
            return "{\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"advice\":{\"decision_source\":\"model-pool-advice-core\",\"safe_to_enable_pool_workers\":true,\"next_step\":\"run_short_pool_smoke_then_use_evolution_loop_helper_stage_calls\",\"reason\":\"full_helper_pool_visible\",\"extra_quality_12b_detected\":false,\"worker_shape\":{\"quality\":1,\"helpers_visible\":3,\"helper_target\":3}},\"workers\":[{\"role\":\"quality\"},{\"role\":\"summary\"},{\"role\":\"router\"},{\"role\":\"index\"}]}".to_owned();
        }
        if request.starts_with("GET /v1/model-pool/status") {
            return "{\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"workers\":[{\"role\":\"quality\",\"tcp_reachable\":true,\"health_ok\":true},{\"role\":\"summary\",\"tcp_reachable\":true,\"health_ok\":true},{\"role\":\"router\",\"tcp_reachable\":true,\"health_ok\":true},{\"role\":\"index\",\"tcp_reachable\":true,\"health_ok\":true}]}".to_owned();
        }
        let task_kind =
            json_string_field(request, "task_kind").unwrap_or_else(|| "review".to_owned());
        format!(
            "{{\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"task_kind\":{},\"route_allowed\":true,\"selected_role\":{},\"candidate_workers\":[{{\"role\":{},\"health_ok\":true,\"role_ready\":true}}]}}",
            json_string(&task_kind),
            json_string(&task_kind),
            json_string(&task_kind)
        )
    }

    fn write_test_http_json(stream: &mut TcpStream, body: &str) {
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(response.as_bytes()).unwrap();
    }

    #[test]
    fn pool_contract_failures_accept_safe_read_only_contract() {
        let failures = pool_contract_failures(
            "pool status",
            "{\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false}",
        );

        assert!(failures.is_empty());
    }

    #[test]
    fn pool_contract_failures_require_manifest_advice_contract() {
        let missing = pool_contract_failures(
            "pool manifest",
            "{\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false}",
        );
        assert!(
            missing
                .iter()
                .any(|failure| failure.contains("manifest advice object"))
        );

        let ok = pool_contract_failures(
            "pool manifest",
            "{\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"advice\":{\"decision_source\":\"model-pool-advice-core\",\"safe_to_enable_pool_workers\":true,\"next_step\":\"run_short_pool_smoke_then_use_evolution_loop_helper_stage_calls\",\"reason\":\"full_helper_pool_visible\",\"extra_quality_12b_detected\":false,\"worker_shape\":{\"quality\":1,\"helpers_visible\":4,\"helper_target\":4}}}",
        );
        assert!(ok.is_empty());

        let missing_shape = pool_contract_failures(
            "pool manifest",
            "{\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"advice\":{\"decision_source\":\"model-pool-advice-core\",\"safe_to_enable_pool_workers\":true,\"next_step\":\"run_short_pool_smoke_then_use_evolution_loop_helper_stage_calls\",\"reason\":\"full_helper_pool_visible\",\"extra_quality_12b_detected\":false}}",
        );
        assert!(
            missing_shape.iter().any(|failure| {
                failure.contains("manifest advice.worker_shape object is missing")
            })
        );
    }

    #[test]
    fn pool_contract_failures_reject_unsafe_contract() {
        let failures = pool_contract_failures(
            "pool route",
            "{\"read_only\":false,\"launches_process\":true,\"sends_prompt\":true}",
        );

        assert!(failures.iter().any(|failure| failure.contains("read_only")));
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("launches_process"))
        );
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("sends_prompt"))
        );
    }

    #[test]
    fn write_pool_artifact_creates_parent_and_trims_trailing_newlines() {
        let path = std::env::temp_dir()
            .join(format!("smartsteam-pool-artifact-{}", std::process::id()))
            .join("pool-status.json");
        if let Some(parent) = path.parent() {
            let _ = fs::remove_dir_all(parent);
        }

        write_pool_artifact(
            &path,
            "pool status",
            200,
            "{\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false}\n\n",
        )
        .unwrap();

        let written = fs::read_to_string(&path).unwrap();
        assert_eq!(
            written,
            "{\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false}\n"
        );
        if let Some(parent) = path.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }

    #[test]
    fn write_pool_artifact_rejects_unsafe_contract() {
        let path = std::env::temp_dir().join(format!(
            "smartsteam-unsafe-pool-artifact-{}.json",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);

        let error = write_pool_artifact(
            &path,
            "pool route",
            200,
            "{\"read_only\":true,\"launches_process\":false,\"sends_prompt\":true}",
        )
        .unwrap_err();

        assert!(error.contains("safe contract"));
        assert!(!path.exists());
    }

    #[test]
    fn write_pool_manifest_artifact_rejects_missing_advice() {
        let path = std::env::temp_dir().join(format!(
            "smartsteam-manifest-missing-advice-{}.json",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);

        let error = write_pool_artifact(
            &path,
            "pool manifest",
            200,
            "{\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false}",
        )
        .unwrap_err();

        assert!(error.contains("manifest advice object is missing"));
        assert!(!path.exists());
    }

    #[test]
    fn state_consistency_gate_allows_clean_or_missing_ledger() {
        let path = std::env::temp_dir().join(format!(
            "smartsteam-state-consistency-clean-{}.jsonl",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        let config = Config {
            ledger_path: path.clone(),
            ..Config::default()
        };

        assert!(run_state_consistency_gate(&config).is_ok());

        fs::write(
            &path,
            "{\"round\":1,\"success\":true}\n{\"round\":2,\"success\":true}\n",
        )
        .unwrap();
        assert!(run_state_consistency_gate(&config).is_ok());

        let _ = fs::remove_file(path);
    }

    #[test]
    fn state_consistency_gate_blocks_dirty_round_state() {
        let path = std::env::temp_dir().join(format!(
            "smartsteam-state-consistency-dirty-{}.jsonl",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        fs::write(
            &path,
            "{\"round\":1,\"success\":true}\n{\"round\":3,\"success\":true}\n{\"round\":3,\"success\":false}\n",
        )
        .unwrap();
        let config = Config {
            ledger_path: path.clone(),
            ..Config::default()
        };

        let error = run_state_consistency_gate(&config).unwrap_err();

        assert!(error.contains("duplicate round"));
        assert!(error.contains("non-monotonic round"));
        assert!(error.contains("missing round number"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn validation_gate_allows_successful_command() {
        let config = Config {
            validation_command: Some("echo validation-ok".to_owned()),
            validation_timeout_secs: 5,
            ..Config::default()
        };
        let plan = effective_validation_command(&config).unwrap().unwrap();

        let evidence = run_validation_gate(&config, "pre", &plan).unwrap();
        let meta = evidence.meta();

        assert!(meta.contains("validation_gate phase=pre"));
        assert!(meta.contains("source=configured"));
        assert!(meta.contains("validation-ok"));
        assert_eq!(evidence.status_code, Some(0));
        assert_eq!(evidence.command_source, "configured");
    }

    #[test]
    fn validation_gate_blocks_failing_command() {
        let config = Config {
            validation_command: Some("exit 7".to_owned()),
            validation_timeout_secs: 5,
            ..Config::default()
        };
        let plan = effective_validation_command(&config).unwrap().unwrap();

        let failure = run_validation_gate(&config, "pre", &plan).unwrap_err();

        assert!(failure.message.contains("validation gate pre failed"));
        assert!(failure.message.contains("status=7"));
        assert_eq!(
            failure
                .evidence
                .as_ref()
                .and_then(|evidence| evidence.status_code),
            Some(7)
        );
    }

    #[test]
    fn effective_validation_command_can_use_safe_test_gate_feedback() {
        let path = std::env::temp_dir().join(format!(
            "smartsteam-test-gate-validation-command-{}.jsonl",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        fs::write(
            &path,
            "{\"round\":1,\"case\":\"helper\",\"success\":true,\"feedback_applied\":2,\"helper_stage_feedback_by_role\":{\"test-gate\":[\"task_kind=test-gate preview=verdict: pass / validation_command: cargo check --manifest-path tools/evolution-loop/Cargo.toml\"]}}\n",
        )
        .unwrap();
        let config = Config {
            ledger_path: path.clone(),
            use_test_gate_validation_command: true,
            ..Config::default()
        };

        let plan = effective_validation_command(&config).unwrap().unwrap();

        assert_eq!(
            plan.command,
            "cargo check --manifest-path tools/evolution-loop/Cargo.toml"
        );
        assert_eq!(plan.source, "test-gate");
        assert_eq!(plan.safety, "safe");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn effective_validation_command_rejects_unsafe_test_gate_feedback() {
        let path = std::env::temp_dir().join(format!(
            "smartsteam-unsafe-test-gate-validation-command-{}.jsonl",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        fs::write(
            &path,
            "{\"round\":1,\"case\":\"helper\",\"success\":true,\"feedback_applied\":2,\"helper_stage_feedback_by_role\":{\"test-gate\":[\"task_kind=test-gate preview=verdict: pass / validation_command: cargo run -- rm -rf target\"]}}\n",
        )
        .unwrap();
        let config = Config {
            ledger_path: path.clone(),
            use_test_gate_validation_command: true,
            ..Config::default()
        };

        let error = effective_validation_command(&config).unwrap_err();

        assert!(error.contains("validation_command is unsafe"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn configured_validation_command_overrides_test_gate_feedback() {
        let path = std::env::temp_dir().join(format!(
            "smartsteam-configured-overrides-test-gate-{}.jsonl",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        fs::write(
            &path,
            "{\"round\":1,\"case\":\"helper\",\"success\":true,\"feedback_applied\":2,\"helper_stage_feedback_by_role\":{\"test-gate\":[\"task_kind=test-gate preview=verdict: pass / validation_command: cargo check\"]}}\n",
        )
        .unwrap();
        let config = Config {
            ledger_path: path.clone(),
            validation_command: Some("echo configured".to_owned()),
            use_test_gate_validation_command: true,
            ..Config::default()
        };

        let plan = effective_validation_command(&config).unwrap().unwrap();

        assert_eq!(plan.command, "echo configured");
        assert_eq!(plan.source, "configured");
        assert_eq!(plan.safety, "explicit");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn experience_audit_gate_allows_missing_store_as_clean_start() {
        let failures = experience_audit_failures(
            &Config::default(),
            "{\"checked\":false,\"error\":\"experience_file_missing\"}",
        );

        assert!(failures.is_empty());
    }

    #[test]
    fn experience_audit_gate_detects_deferred_large_file() {
        let reason = experience_audit_deferred_reason(
            "{\"checked\":false,\"error\":\"experience_hygiene_deferred_large_file: size_bytes=6875845 max_inline_bytes=1000000\"}",
        )
        .unwrap();

        assert!(reason.contains("size_bytes=6875845"));
    }

    #[test]
    fn experience_audit_gate_accepts_clean_ready_index() {
        let failures = experience_audit_failures(
            &Config::default(),
            "{\"checked\":true,\"report\":{\"legacy_metadata_without_clean_gist\":0},\"index_report\":{\"overlong_records\":1,\"overlong_without_clean_gist\":0,\"max_record_chars\":4096,\"noisy_records\":0,\"max_noise_penalty\":0.0,\"quality_score\":0.92,\"retrieval_ready\":true},\"quarantine_plan\":{\"quarantine_candidates\":0},\"repair_plan\":{\"repairable_legacy_metadata_lessons\":0}}",
        );

        assert!(failures.is_empty());
    }

    #[test]
    fn experience_audit_gate_blocks_overlong_without_clean_gist_by_default() {
        let failures = experience_audit_failures(
            &Config::default(),
            "{\"checked\":true,\"index_report\":{\"overlong_records\":1,\"overlong_without_clean_gist\":1,\"max_record_chars\":4096,\"noisy_records\":0,\"max_noise_penalty\":0.0,\"quality_score\":0.92,\"retrieval_ready\":true}}",
        );

        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("overlong_without_clean_gist 1"))
        );
    }

    #[test]
    fn experience_audit_gate_names_index_and_cleanup_failures() {
        let config = Config {
            max_index_overlong_records: Some(1),
            max_index_record_chars: Some(4000),
            max_index_noisy_records: 1,
            max_index_noise_penalty: 0.1,
            max_quarantine_candidates: 0,
            max_repairable_legacy_records: 0,
            ..Config::default()
        };
        let failures = experience_audit_failures(
            &config,
            "{\"checked\":true,\"report\":{\"legacy_metadata_without_clean_gist\":4},\"index_report\":{\"overlong_records\":2,\"overlong_without_clean_gist\":1,\"max_record_chars\":5000,\"noisy_records\":2,\"max_noise_penalty\":0.18,\"quality_score\":0.5,\"retrieval_ready\":false},\"quarantine_plan\":{\"quarantine_candidates\":1},\"repair_plan\":{\"repairable_legacy_metadata_lessons\":3}}",
        );

        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("overlong_records 2"))
        );
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("overlong_without_clean_gist 1"))
        );
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("max_record_chars 5000"))
        );
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("noisy_records 2"))
        );
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("max_noise_penalty 0.180000"))
        );
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("quality_score 0.500000"))
        );
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("retrieval_ready false"))
        );
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("quarantine_candidates 1"))
        );
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("repairable_legacy_metadata_lessons 3"))
        );
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("legacy_metadata_without_clean_gist 4"))
        );
        assert_eq!(
            failures
                .iter()
                .filter(|failure| failure.contains("index noisy_records 2 above maximum 1"))
                .count(),
            1
        );
        assert_eq!(
            failures
                .iter()
                .filter(|failure| failure
                    .contains("index max_noise_penalty 0.180000 above maximum 0.100000"))
                .count(),
            1
        );
    }

    #[test]
    fn experience_audit_context_rot_report_matches_eval_contract() {
        let config = Config {
            max_index_noisy_records: 1,
            max_index_noise_penalty: 0.1,
            max_quarantine_candidates: 0,
            max_repairable_legacy_records: 0,
            ..Config::default()
        };
        let body = "{\"checked\":true,\"report\":{\"legacy_metadata_without_clean_gist\":4},\"index_report\":{\"noisy_records\":2,\"max_noise_penalty\":0.18,\"duplicate_outputs\":2,\"quality_score\":0.92,\"retrieval_ready\":true},\"quarantine_plan\":{\"quarantine_candidates\":1},\"repair_plan\":{\"repairable_legacy_metadata_lessons\":3}}";
        let index_report = json_object_field(body, "index_report");
        let signal = context_rot_signal_from_audit(body, index_report.as_deref());
        let gate = context_rot_gate_from_config(&config);
        let decision = gate.evaluate(&signal);
        let report = norion_eval::ContextRotReport::from_signal_and_decision(&signal, &decision);

        assert_eq!(report.noisy_records, 2);
        assert_eq!(report.max_noise_penalty, 0.18);
        assert_eq!(report.quarantine_candidates, 1);
        assert_eq!(report.repairable_legacy_metadata_lessons, 3);
        assert_eq!(report.legacy_metadata_without_clean_gist, 4);
        assert_eq!(report.duplicate_outputs, 2);
        assert!(report.gate_blocked);
        assert!(
            report
                .failure_reasons
                .iter()
                .any(|reason| reason.contains("noisy records 2 above maximum 1"))
        );
        assert!(
            report
                .failure_reasons
                .iter()
                .any(|reason| reason.contains("quarantine candidates 1 above maximum 0"))
        );

        let remediation_gate = norion_eval::ContextRotRemediationGate::for_stage(
            norion_eval::RootAdapterRolloutStage::Enforced,
        );
        let remediation_report = norion_eval::ContextRotRemediationReport::from_gate_and_signal(
            &remediation_gate,
            &signal,
        );

        assert_eq!(remediation_report.quarantine_candidates, 1);
        assert_eq!(remediation_report.repairable_legacy_metadata_lessons, 3);
        assert_eq!(remediation_report.legacy_metadata_without_clean_gist, 4);
        assert_eq!(remediation_report.duplicate_outputs, 2);
        assert!(remediation_report.remediation_blocked);
        assert!(
            remediation_report
                .failure_reasons
                .iter()
                .any(|reason| reason.contains("legacy metadata repair incomplete"))
        );
    }

    #[test]
    fn experience_audit_context_rot_trend_report_matches_eval_contract() {
        let signal_from_audit = |body: &str| {
            let index_report = json_object_field(body, "index_report");
            context_rot_signal_from_audit(body, index_report.as_deref())
        };
        let points = vec![
            norion_eval::ContextRotTrendPoint::new(
                10,
                signal_from_audit(
                    "{\"checked\":true,\"index_report\":{\"noisy_records\":2,\"max_noise_penalty\":0.20,\"duplicate_outputs\":1},\"quarantine_plan\":{\"quarantine_candidates\":1},\"repair_plan\":{\"repairable_legacy_metadata_lessons\":1}}",
                ),
            ),
            norion_eval::ContextRotTrendPoint::new(
                11,
                signal_from_audit(
                    "{\"checked\":true,\"index_report\":{\"noisy_records\":1,\"max_noise_penalty\":0.10,\"duplicate_outputs\":0},\"quarantine_plan\":{\"quarantine_candidates\":0},\"repair_plan\":{\"repairable_legacy_metadata_lessons\":0}}",
                ),
            )
            .with_remediation_applied(true),
            norion_eval::ContextRotTrendPoint::new(
                12,
                signal_from_audit(
                    "{\"checked\":true,\"index_report\":{\"noisy_records\":0,\"max_noise_penalty\":0.0,\"duplicate_outputs\":0},\"quarantine_plan\":{\"quarantine_candidates\":0},\"repair_plan\":{\"repairable_legacy_metadata_lessons\":0}}",
                ),
            ),
        ];
        let gate = norion_eval::ContextRotTrendGate {
            max_consecutive_noisy_rounds: 2,
            max_consecutive_duplicate_rounds: 1,
            ..norion_eval::ContextRotTrendGate::strict()
        };
        let report = norion_eval::ContextRotTrendReport::from_points_and_gate(&points, &gate);

        assert_eq!(report.rounds, 3);
        assert_eq!(report.first_round, Some(10));
        assert_eq!(report.last_round, Some(12));
        assert_eq!(report.latest_noisy_records, 0);
        assert_eq!(report.latest_duplicate_outputs, 0);
        assert_eq!(report.noisy_records_delta, -2);
        assert_eq!(report.duplicate_outputs_delta, -1);
        assert_eq!(report.remediation_applied_rounds, 1);
        assert!(report.remediation_improved_noise);
        assert!(report.remediation_improved_duplicates);
        assert!(!report.trend_blocked);
        assert!(report.allow_unattended_continuation);
    }

    #[test]
    fn business_cycle_body_can_attach_rust_check() {
        let config = Config {
            rust_check_edition: "2024".to_owned(),
            rust_check_case: Some("manual-rust-check".to_owned()),
            ..Config::default()
        };

        let body = business_cycle_body(
            &config,
            Some("pub fn ok() {}"),
            "case-a",
            "prompt",
            None,
            &[],
        );

        assert!(body.contains("\"rust_check_code\":\"pub fn ok() {}\""));
        assert!(body.contains("\"rust_check_edition\":\"2024\""));
        assert!(body.contains("\"rust_check_case\":\"manual-rust-check\""));
    }

    #[test]
    fn rust_check_fields_default_case_from_cycle_case() {
        let fields = rust_check_json_fields(&Config::default(), Some("pub fn ok() {}"), "cycle-a");

        assert!(fields.contains("\"rust_check_case\":\"cycle-a-rust-check\""));
    }
}
