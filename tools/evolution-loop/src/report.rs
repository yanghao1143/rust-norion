use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::Path;

use norion_eval::{
    HelperStageContractSummary, HelperStageHygieneFinding, LedgerGateReport, LedgerSummary,
    ReportGate, SelfImproveProposalPromptGuidance, TestGateValidationRunEvidence,
    TestGateValidationRunFailure, ValidationCommandCoverageEvidence,
    helper_stage_contract_is_useful_for_role, helper_stage_feedback_hygiene_findings,
    helper_stage_missing_complete_fields, helper_stage_placeholder_fields,
    helper_stage_test_gate_verdict as eval_test_gate_verdict,
    test_gate_validation_run_failure as eval_test_gate_validation_run_failure,
    validation_run_failure as eval_validation_run_failure,
};

use crate::args::Config;
use crate::clean_room_batch_status::{self, CleanRoomBatchStatusSummary};
use crate::clean_room_handoff::{self, CleanRoomHandoffSummary};
use crate::helper_feedback;
use crate::helper_stage_repair::{self, HelperStageRepairStatus};
use crate::json::{
    json_array_field, json_bool_field, json_i32_field, json_object_field, json_string,
    json_string_array, json_string_field, json_u64_field, parse_json_object_array,
    parse_json_string_array, parse_json_string_array_map, preview_text,
};
use crate::ledger::ledger_hygiene;
use crate::pool_artifacts::{
    self, PoolBudgetFairnessSummary, PoolManifestSummary, PoolRouteSummary, PoolStatusSummary,
};
use crate::pool_stage;
use crate::remote_chain::{self, RemoteChainStatusSummary};
use crate::self_improve_proposal_artifact::{self, SelfImproveProposalArtifact};
use crate::validation;
use crate::worker_window_status::{self, WorkerWindowStatusSummary};

const MAX_HELPER_STAGE_FEEDBACK_ITEMS: usize = 3;
const MIN_USEFUL_HELPER_STAGE_FEEDBACK_CHARS: usize = 24;
const RECENT_FAILURE_WINDOW_RECORDS: usize = 5;
const RECENT_REPEATED_SUCCESSFUL_ANSWER_THRESHOLD: usize = 3;
const MIN_REPEATED_ADVICE_KEY_CHARS: usize = 24;
const RECENT_COMPLETED_CHANGE_REQUEST_RECORDS: usize = 8;
const REQUIRED_FINAL_JSON_POOL_STAGE_DISPATCH_TASK_KINDS: &[&str] =
    &["summary", "router", "review", "index", "test-gate"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReportJsonRefresh {
    pub(crate) rounds: usize,
    pub(crate) gate_label: Option<String>,
    pub(crate) gate_failure_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
struct ReportRecord {
    round: Option<u64>,
    case_name: Option<String>,
    started_unix: Option<u64>,
    finished_unix: Option<u64>,
    success: bool,
    error: Option<String>,
    runtime_tokens: Option<u64>,
    runtime_model: Option<String>,
    answer: Option<String>,
    final_json: Option<String>,
    final_json_pool_stage_dispatch: Option<String>,
    elapsed_ms: Option<u64>,
    feedback_applied: Option<u64>,
    rust_check_checked: Option<bool>,
    rust_check_passed: Option<bool>,
    rust_check_feedback_applied: Option<u64>,
    validation_checked: Option<bool>,
    validation_passed: Option<bool>,
    validation_command_source: Option<String>,
    validation_command_safety: Option<String>,
    validation_command_preview: Option<String>,
    validation_phase: Option<String>,
    validation_status_code: Option<i32>,
    validation_elapsed_ms: Option<u64>,
    validation_stdout_tail: Option<String>,
    validation_stderr_tail: Option<String>,
    self_improve_passed: Option<bool>,
    state_gate_checked: Option<bool>,
    state_gate_passed: Option<bool>,
    trace_gate_checked: Option<bool>,
    trace_gate_passed: Option<bool>,
    eval_json: Option<String>,
    eval_report_only: Option<bool>,
    eval_failure_kind: Option<String>,
    meta: Vec<String>,
    structured_helper_stage_feedback_by_role: BTreeMap<String, Vec<String>>,
    structured_helper_stage_contract_fields_by_role: BTreeMap<String, BTreeMap<String, String>>,
    allocation_evidence: Vec<String>,
}

impl ReportRecord {
    fn round_wall_elapsed_ms(&self) -> Option<u64> {
        let started = self.started_unix?;
        let finished = self.finished_unix?;
        Some(finished.checked_sub(started)?.saturating_mul(1000))
    }

    fn has_round_wall_clock_evidence(&self) -> bool {
        self.round_wall_elapsed_ms().is_some()
    }

    fn has_stream_truncation_error(&self) -> bool {
        self.error
            .as_deref()
            .is_some_and(|error| error.contains("stream truncated"))
    }

    fn has_missing_final_error(&self) -> bool {
        self.error
            .as_deref()
            .is_some_and(|error| error.contains("stream ended without final event"))
    }

    fn has_runtime_response_failure(&self) -> bool {
        let zero_runtime_tokens = self.runtime_tokens == Some(0);
        let backend_error_answer = self.answer.as_deref().is_some_and(|answer| {
            answer
                .to_ascii_lowercase()
                .contains("runtime backend error")
        });
        let missing_runtime_model_after_success =
            self.success && self.runtime_tokens.is_some() && self.runtime_model.is_none();
        zero_runtime_tokens || backend_error_answer || missing_runtime_model_after_success
    }

    fn helper_stage_feedback(&self) -> Vec<String> {
        helper_feedback::meta_entries(&self.meta)
    }

    fn helper_stage_feedback_by_role(&self) -> BTreeMap<String, Vec<String>> {
        let mut feedback_by_role = helper_feedback::sanitize_feedback_by_role(
            self.structured_helper_stage_feedback_by_role.clone(),
        );
        for (role, feedback_items) in helper_feedback::feedback_by_role_from_meta(&self.meta) {
            let role_feedback = feedback_by_role.entry(role).or_default();
            for feedback in feedback_items {
                if !role_feedback.iter().any(|existing| existing == &feedback) {
                    role_feedback.push(feedback);
                }
            }
        }
        feedback_by_role
    }

    fn helper_stage_contract_fields_by_role(&self) -> BTreeMap<String, BTreeMap<String, String>> {
        self.structured_helper_stage_contract_fields_by_role.clone()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TestGateSummary {
    pub(crate) latest_verdict: Option<String>,
    pub(crate) latest_validation_command: Option<String>,
    pub(crate) latest_validation_command_safety: String,
    pub(crate) latest_failure_kind: Option<String>,
    pub(crate) latest_fields: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
struct ReportSummary {
    total: usize,
    unique_rounds: usize,
    duplicate_rounds: usize,
    non_monotonic_rounds: usize,
    missing_rounds: usize,
    round_gaps: usize,
    max_round: Option<usize>,
    success: usize,
    failure: usize,
    runtime_tokens: u64,
    runtime_token_items: usize,
    elapsed_ms: u64,
    elapsed_items: usize,
    round_wall_elapsed_ms: u64,
    round_wall_elapsed_items: usize,
    feedback_applied: u64,
    feedback_items: usize,
    rust_check_passed: usize,
    rust_check_checked: usize,
    rust_check_feedback_applied: u64,
    rust_check_feedback_items: usize,
    validation_passed: usize,
    validation_checked: usize,
    self_improve_passed: usize,
    self_improve_checked: usize,
    state_gate_passed: usize,
    state_gate_checked: usize,
    trace_gate_passed: usize,
    trace_gate_checked: usize,
    eval_records: usize,
    eval_report_only_records: usize,
    eval_failure_kinds: BTreeMap<String, usize>,
    stream_truncation_failures: usize,
    missing_final_failures: usize,
    runtime_response_failures: usize,
    recent_failure_window_records: usize,
    recent_stream_truncation_failures: usize,
    recent_missing_final_failures: usize,
    recent_runtime_response_failures: usize,
    recent_repeated_successful_answer: Option<RepeatedAnswerSummary>,
    completed_change_requests: Vec<String>,
    invalid_change_requests: Vec<String>,
    validation_command_coverage_evidence: ValidationCommandCoverageEvidence,
    helper_stage_feedback: Vec<String>,
    helper_stage_feedback_by_role: BTreeMap<String, Vec<String>>,
    helper_stage_hygiene_by_role: BTreeMap<String, Vec<HelperStageHygieneFinding>>,
    helper_stage_contract_by_role: BTreeMap<String, HelperStageContractSummary>,
    test_gate: TestGateSummary,
    last: Option<ReportRecord>,
    recent_failures: Vec<ReportRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RepeatedAnswerSummary {
    count: usize,
    window_records: usize,
    preview: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PromptStaleHelperRecovery {
    current_quality_context_required_tokens: String,
    latest_success_round: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PromptHelperFilters {
    stale_context: Option<PromptStaleHelperRecovery>,
    completed_change_topics: Vec<String>,
    invalid_change_topics: Vec<String>,
    satisfied_role_budgets: BTreeMap<String, String>,
    underutilized_role_budgets: BTreeMap<String, PromptRoleTokenHeadroom>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PromptRoleTokenHeadroom {
    used_tokens: u64,
    max_tokens: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct PromptHelperOmissionCounts {
    stale_context: usize,
    completed_change_topic: usize,
    invalid_change_topic: usize,
    generic_noop_proposal: usize,
    satisfied_role_budget: usize,
    underutilized_budget_increase: usize,
}

impl PromptHelperOmissionCounts {
    fn add(&mut self, other: PromptHelperOmissionCounts) {
        self.stale_context += other.stale_context;
        self.completed_change_topic += other.completed_change_topic;
        self.invalid_change_topic += other.invalid_change_topic;
        self.generic_noop_proposal += other.generic_noop_proposal;
        self.satisfied_role_budget += other.satisfied_role_budget;
        self.underutilized_budget_increase += other.underutilized_budget_increase;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PromptHelperOmitReason {
    StaleContext,
    CompletedChangeTopic,
    InvalidChangeTopic,
    GenericNoopProposal,
    SatisfiedRoleBudget,
    UnderutilizedBudgetIncrease,
}

pub(crate) fn run(config: Config) -> Result<(), String> {
    let text = fs::read_to_string(&config.ledger_path).map_err(|error| {
        format!(
            "read ledger {} failed: {error}",
            config.ledger_path.display()
        )
    })?;
    let summary = summarize_ledger(&text);
    let self_improve_proposal_artifact = self_improve_proposal_artifact::from_ledger_text(&text);
    let pool_manifest = pool_artifacts::load_manifest(config.pool_manifest_json_path.as_deref())?;
    let pool_status = pool_artifacts::load_status(config.pool_status_json_path.as_deref())?;
    let pool_route = pool_artifacts::load_route(config.pool_route_json_path.as_deref())?;
    let pool_budget_fairness =
        pool_artifacts::load_budget_fairness(config.pool_budget_fairness_json_path.as_deref())?;
    let remote_chain_status =
        remote_chain::load_status(config.remote_chain_status_json_path.as_deref())?;
    let worker_window_status =
        worker_window_status::load_status(config.worker_window_status_json_path.as_deref())?;
    let clean_room_batch_status =
        clean_room_batch_status::load_status(config.clean_room_batch_status_json_path.as_deref())?;
    let clean_room_handoff = clean_room_handoff::load_status(
        config.memory_startup_admission_json_path.as_deref(),
        config
            .agent_clean_room_replacement_plan_json_path
            .as_deref(),
    )?;
    print_report(
        &config,
        &summary,
        remote_chain_status.as_ref(),
        pool_manifest.as_ref(),
        pool_status.as_ref(),
        pool_route.as_ref(),
        pool_budget_fairness.as_ref(),
        worker_window_status.as_ref(),
        clean_room_batch_status.as_ref(),
        clean_room_handoff.as_ref(),
        &self_improve_proposal_artifact,
    );
    let gate_requested = config.report_gate || config.report_continuation_gate;
    let ledger_gate_failures = if gate_requested {
        report_gate_threshold_failures(&summary, &config)
    } else {
        Vec::new()
    };
    let strict_gate_failures = if gate_requested {
        report_gate_failures(
            &summary,
            &config,
            pool_status.as_ref(),
            pool_budget_fairness.as_ref(),
        )
    } else {
        Vec::new()
    };
    let continuation_gate_failures = if gate_requested {
        report_gate_continuation_failures(
            &summary,
            &config,
            pool_status.as_ref(),
            pool_budget_fairness.as_ref(),
        )
    } else {
        Vec::new()
    };
    let failures = if config.report_continuation_gate {
        continuation_gate_failures.clone()
    } else if config.report_gate {
        strict_gate_failures.clone()
    } else {
        Vec::new()
    };
    if let Some(path) = &config.report_json_path {
        write_report_json(
            path,
            &summary,
            remote_chain_status.as_ref(),
            pool_manifest.as_ref(),
            pool_status.as_ref(),
            pool_route.as_ref(),
            pool_budget_fairness.as_ref(),
            worker_window_status.as_ref(),
            clean_room_batch_status.as_ref(),
            clean_room_handoff.as_ref(),
            &self_improve_proposal_artifact,
            &config.required_latest_helper_stage_roles,
            &ledger_gate_failures,
            &strict_gate_failures,
            &continuation_gate_failures,
            &failures,
        )?;
        println!("report_json: {}", path.display());
    }
    if config.report_gate || config.report_continuation_gate {
        let gate_label = if config.report_continuation_gate {
            "report_continuation_gate"
        } else {
            "report_gate"
        };
        if failures.is_empty() {
            println!("{gate_label}: passed");
        } else {
            println!("{gate_label}: failed");
            for failure in &failures {
                println!("  {failure}");
            }
            return Err(format!(
                "{} failed: {}",
                gate_label.replace('_', " "),
                failures.join("; ")
            ));
        }
    }
    Ok(())
}

pub(crate) fn write_run_report_json(
    config: &Config,
    path: &Path,
    report_gate: bool,
    report_continuation_gate: bool,
) -> Result<ReportJsonRefresh, String> {
    let text = fs::read_to_string(&config.ledger_path).map_err(|error| {
        format!(
            "read ledger {} failed: {error}",
            config.ledger_path.display()
        )
    })?;
    let summary = summarize_ledger(&text);
    let self_improve_proposal_artifact = self_improve_proposal_artifact::from_ledger_text(&text);
    let pool_manifest = pool_artifacts::load_manifest(config.pool_manifest_json_path.as_deref())?;
    let pool_status = pool_artifacts::load_status(config.pool_status_json_path.as_deref())?;
    let pool_route = pool_artifacts::load_route(config.pool_route_json_path.as_deref())?;
    let pool_budget_fairness =
        pool_artifacts::load_budget_fairness(config.pool_budget_fairness_json_path.as_deref())?;
    let remote_chain_status =
        remote_chain::load_status(config.remote_chain_status_json_path.as_deref())?;
    let worker_window_status =
        worker_window_status::load_status(config.worker_window_status_json_path.as_deref())?;
    let clean_room_batch_status =
        clean_room_batch_status::load_status(config.clean_room_batch_status_json_path.as_deref())?;
    let clean_room_handoff = clean_room_handoff::load_status(
        config.memory_startup_admission_json_path.as_deref(),
        config
            .agent_clean_room_replacement_plan_json_path
            .as_deref(),
    )?;

    let gate_requested = report_gate || report_continuation_gate;
    let ledger_gate_failures = if gate_requested {
        report_gate_threshold_failures(&summary, config)
    } else {
        Vec::new()
    };
    let strict_gate_failures = if gate_requested {
        report_gate_failures(
            &summary,
            config,
            pool_status.as_ref(),
            pool_budget_fairness.as_ref(),
        )
    } else {
        Vec::new()
    };
    let continuation_gate_failures = if gate_requested {
        report_gate_continuation_failures(
            &summary,
            config,
            pool_status.as_ref(),
            pool_budget_fairness.as_ref(),
        )
    } else {
        Vec::new()
    };
    let failures = if report_continuation_gate {
        continuation_gate_failures.clone()
    } else if report_gate {
        strict_gate_failures.clone()
    } else {
        Vec::new()
    };

    write_report_json(
        path,
        &summary,
        remote_chain_status.as_ref(),
        pool_manifest.as_ref(),
        pool_status.as_ref(),
        pool_route.as_ref(),
        pool_budget_fairness.as_ref(),
        worker_window_status.as_ref(),
        clean_room_batch_status.as_ref(),
        clean_room_handoff.as_ref(),
        &self_improve_proposal_artifact,
        &config.required_latest_helper_stage_roles,
        &ledger_gate_failures,
        &strict_gate_failures,
        &continuation_gate_failures,
        &failures,
    )?;

    let refresh = ReportJsonRefresh {
        rounds: summary.total,
        gate_label: if report_continuation_gate {
            Some("report_continuation_gate".to_owned())
        } else if report_gate {
            Some("report_gate".to_owned())
        } else {
            None
        },
        gate_failure_count: failures.len(),
    };

    if let Some(label) = refresh.gate_label.as_deref()
        && !failures.is_empty()
    {
        return Err(format!(
            "{} failed: {}",
            label.replace('_', " "),
            failures.join("; ")
        ));
    }

    Ok(refresh)
}

pub(crate) fn prompt_context(path: &Path) -> Result<Option<String>, String> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "read ledger {} for report context failed: {error}",
                path.display()
            ));
        }
    };
    if text.trim().is_empty() {
        return Ok(None);
    }
    let summary = summarize_ledger(&text);
    if summary.total == 0 {
        return Ok(None);
    }
    let self_improve_proposal_artifact = self_improve_proposal_artifact::from_ledger_text(&text);
    Ok(Some(prompt_context_text_with_self_improve_proposals(
        &summary,
        Some(&self_improve_proposal_artifact),
    )))
}

pub(crate) fn latest_test_gate_summary(path: &Path) -> Result<Option<TestGateSummary>, String> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "read ledger {} for test-gate summary failed: {error}",
                path.display()
            ));
        }
    };
    if text.trim().is_empty() {
        return Ok(None);
    }
    let summary = summarize_ledger(&text);
    Ok(has_test_gate_summary(&summary.test_gate).then_some(summary.test_gate))
}

fn summarize_ledger(text: &str) -> ReportSummary {
    let records = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(parse_record)
        .collect::<Vec<_>>();
    let hygiene = ledger_hygiene(
        records
            .iter()
            .map(|record| record.round.and_then(|round| usize::try_from(round).ok())),
    );
    let helper_stage_feedback_by_role = recent_helper_stage_feedback_by_role(&records);
    let helper_stage_contract_by_role =
        recent_helper_stage_contract_summaries_by_role(&records, &helper_stage_feedback_by_role);
    let latest_contract_fields_by_role = records
        .last()
        .map(ReportRecord::helper_stage_contract_fields_by_role)
        .unwrap_or_default();
    let test_gate = test_gate_summary(
        &helper_stage_feedback_by_role,
        &latest_contract_fields_by_role,
    );
    let recent_failure_window_records = records.len().min(RECENT_FAILURE_WINDOW_RECORDS);
    let recent_stream_truncation_failures = records
        .iter()
        .rev()
        .take(RECENT_FAILURE_WINDOW_RECORDS)
        .filter(|record| record.has_stream_truncation_error())
        .count();
    let recent_missing_final_failures = records
        .iter()
        .rev()
        .take(RECENT_FAILURE_WINDOW_RECORDS)
        .filter(|record| record.has_missing_final_error())
        .count();
    let recent_runtime_response_failures = records
        .iter()
        .rev()
        .take(RECENT_FAILURE_WINDOW_RECORDS)
        .filter(|record| record.has_runtime_response_failure())
        .count();
    let recent_repeated_successful_answer = recent_repeated_successful_answer(&records);
    let completed_change_requests = recent_completed_change_requests(&records);
    let validation_command_coverage_evidence = validation_command_coverage_evidence(&records);
    let has_coverage_evidence =
        validation_command_coverage_evidence.coverage_tooling_or_report_evidence_present();
    let invalid_change_requests = recent_invalid_change_requests(&records, has_coverage_evidence);
    let helper_stage_hygiene_by_role = helper_stage_hygiene_by_role(&helper_stage_feedback_by_role);
    let mut summary = ReportSummary {
        total: records.len(),
        unique_rounds: hygiene.unique_rounds,
        duplicate_rounds: hygiene.duplicate_rounds,
        non_monotonic_rounds: hygiene.non_monotonic_rounds,
        missing_rounds: hygiene.missing_rounds,
        round_gaps: hygiene.round_gaps,
        max_round: hygiene.max_round,
        success: 0,
        failure: 0,
        runtime_tokens: 0,
        runtime_token_items: 0,
        elapsed_ms: 0,
        elapsed_items: 0,
        round_wall_elapsed_ms: 0,
        round_wall_elapsed_items: 0,
        feedback_applied: 0,
        feedback_items: 0,
        rust_check_passed: 0,
        rust_check_checked: 0,
        rust_check_feedback_applied: 0,
        rust_check_feedback_items: 0,
        validation_passed: 0,
        validation_checked: 0,
        self_improve_passed: 0,
        self_improve_checked: 0,
        state_gate_passed: 0,
        state_gate_checked: 0,
        trace_gate_passed: 0,
        trace_gate_checked: 0,
        eval_records: 0,
        eval_report_only_records: 0,
        eval_failure_kinds: BTreeMap::new(),
        stream_truncation_failures: 0,
        missing_final_failures: 0,
        runtime_response_failures: 0,
        recent_failure_window_records,
        recent_stream_truncation_failures,
        recent_missing_final_failures,
        recent_runtime_response_failures,
        recent_repeated_successful_answer,
        completed_change_requests,
        invalid_change_requests,
        validation_command_coverage_evidence,
        helper_stage_feedback: recent_helper_stage_feedback(&records),
        helper_stage_feedback_by_role,
        helper_stage_hygiene_by_role,
        helper_stage_contract_by_role,
        test_gate,
        last: records.last().cloned(),
        recent_failures: records
            .iter()
            .rev()
            .filter(|record| !record.success)
            .take(3)
            .cloned()
            .collect(),
    };

    for record in records {
        if record.success {
            summary.success += 1;
        } else {
            summary.failure += 1;
        }
        if record.has_stream_truncation_error() {
            summary.stream_truncation_failures += 1;
        }
        if record.has_missing_final_error() {
            summary.missing_final_failures += 1;
        }
        if record.has_runtime_response_failure() {
            summary.runtime_response_failures += 1;
        }
        if let Some(tokens) = record.runtime_tokens {
            summary.runtime_tokens += tokens;
            summary.runtime_token_items += 1;
        }
        if let Some(elapsed_ms) = record.elapsed_ms {
            summary.elapsed_ms += elapsed_ms;
            summary.elapsed_items += 1;
        }
        if let Some(round_wall_elapsed_ms) = record.round_wall_elapsed_ms() {
            summary.round_wall_elapsed_ms += round_wall_elapsed_ms;
            summary.round_wall_elapsed_items += 1;
        }
        if let Some(feedback_applied) = record.feedback_applied {
            summary.feedback_applied += feedback_applied;
            summary.feedback_items += 1;
        }
        if let Some(rust_check_passed) = record.rust_check_passed
            && record.rust_check_checked.unwrap_or(true)
        {
            summary.rust_check_checked += 1;
            if rust_check_passed {
                summary.rust_check_passed += 1;
            }
        }
        if let Some(rust_feedback_applied) = record.rust_check_feedback_applied {
            summary.rust_check_feedback_applied += rust_feedback_applied;
            summary.rust_check_feedback_items += 1;
        }
        if let Some(validation_passed) = record.validation_passed
            && record.validation_checked.unwrap_or(true)
        {
            summary.validation_checked += 1;
            if validation_passed {
                summary.validation_passed += 1;
            }
        }
        if let Some(self_improve_passed) = record.self_improve_passed {
            summary.self_improve_checked += 1;
            if self_improve_passed {
                summary.self_improve_passed += 1;
            }
        }
        if let Some(state_gate_passed) = record.state_gate_passed
            && record.state_gate_checked.unwrap_or(true)
        {
            summary.state_gate_checked += 1;
            if state_gate_passed {
                summary.state_gate_passed += 1;
            }
        }
        if let Some(trace_gate_passed) = record.trace_gate_passed
            && record.trace_gate_checked.unwrap_or(true)
        {
            summary.trace_gate_checked += 1;
            if trace_gate_passed {
                summary.trace_gate_passed += 1;
            }
        }
        if record.eval_json.is_some() {
            summary.eval_records += 1;
        }
        if record.eval_report_only == Some(true) {
            summary.eval_report_only_records += 1;
        }
        if let Some(failure_kind) = record.eval_failure_kind.as_deref() {
            *summary
                .eval_failure_kinds
                .entry(failure_kind.to_owned())
                .or_insert(0) += 1;
        }
    }
    summary
}

fn recent_repeated_successful_answer(records: &[ReportRecord]) -> Option<RepeatedAnswerSummary> {
    let window_records = records.len().min(RECENT_FAILURE_WINDOW_RECORDS);
    let mut answers = Vec::<(String, usize, String)>::new();
    for answer in records
        .iter()
        .rev()
        .take(RECENT_FAILURE_WINDOW_RECORDS)
        .filter(|record| record.success)
        .filter_map(|record| record.answer.as_deref())
    {
        let advice = extract_advice_text(answer);
        let key = normalized_advice_key(&advice);
        if key.chars().count() < MIN_REPEATED_ADVICE_KEY_CHARS {
            continue;
        }
        if let Some((_, count, preview)) = answers
            .iter_mut()
            .find(|(existing_key, _, _)| existing_key == &key)
        {
            *count += 1;
            *preview = preview_text(&advice, 240);
        } else {
            answers.push((key, 1, preview_text(&advice, 240)));
        }
    }

    answers
        .into_iter()
        .filter(|(_, count, _)| *count >= RECENT_REPEATED_SUCCESSFUL_ANSWER_THRESHOLD)
        .max_by_key(|(_, count, _)| *count)
        .map(|(_, count, preview)| RepeatedAnswerSummary {
            count,
            window_records,
            preview,
        })
}

fn extract_advice_text(answer: &str) -> String {
    let lower = answer.to_ascii_lowercase();
    let mut text = answer;
    if let Some(start) = lower.find("improvement") {
        let after_start = start + "improvement".len();
        text = &answer[after_start..];
        let lower_text = text.to_ascii_lowercase();
        if let Some(end) = lower_text
            .find("verifiable evidence")
            .or_else(|| lower_text.find("verification"))
            .or_else(|| lower_text.find("evidence"))
        {
            text = &text[..end];
        }
    }
    compact_advice_text(text)
}

fn compact_advice_text(text: &str) -> String {
    text.trim_matches(|character: char| {
        matches!(
            character,
            '*' | '`' | ':' | '-' | '/' | ' ' | '\n' | '\r' | '\t'
        )
    })
    .split_whitespace()
    .collect::<Vec<_>>()
    .join(" ")
}

fn normalized_advice_key(text: &str) -> String {
    let mut normalized = String::new();
    let mut previous_space = true;
    for character in text.chars().flat_map(char::to_lowercase) {
        if character.is_alphanumeric() {
            normalized.push(character);
            previous_space = false;
        } else if !previous_space {
            normalized.push(' ');
            previous_space = true;
        }
    }
    normalized.trim().to_owned()
}

fn repeated_successful_answer_blocked_topic(preview: &str) -> String {
    if change_request_has_redundant_max_iterations_flag(preview) {
        "evolution-loop.max-iterations".to_owned()
    } else if change_request_has_unproven_strict_coverage_control(preview) {
        "evolution-loop.strict-coverage".to_owned()
    } else if change_request_has_unproven_test_seed_control(preview) {
        "evolution-loop.test-deterministic-seed".to_owned()
    } else {
        "recent-repeated-successful-answer".to_owned()
    }
}

fn recent_helper_stage_feedback(records: &[ReportRecord]) -> Vec<String> {
    let mut feedback = records
        .iter()
        .flat_map(ReportRecord::helper_stage_feedback)
        .collect::<Vec<_>>();
    let keep_from = feedback
        .len()
        .saturating_sub(MAX_HELPER_STAGE_FEEDBACK_ITEMS);
    feedback.drain(..keep_from);
    feedback
}

fn recent_helper_stage_feedback_by_role(records: &[ReportRecord]) -> BTreeMap<String, Vec<String>> {
    let mut feedback_by_role = BTreeMap::<String, Vec<String>>::new();
    for record in records {
        for (role, feedback) in record.helper_stage_feedback_by_role() {
            feedback_by_role.entry(role).or_default().extend(feedback);
        }
    }
    for feedback in feedback_by_role.values_mut() {
        let keep_from = feedback
            .len()
            .saturating_sub(MAX_HELPER_STAGE_FEEDBACK_ITEMS);
        feedback.drain(..keep_from);
    }
    feedback_by_role
}

fn recent_completed_change_requests(records: &[ReportRecord]) -> Vec<String> {
    let mut completed = BTreeSet::new();
    for record in records
        .iter()
        .rev()
        .filter(|record| record.success)
        .take(RECENT_COMPLETED_CHANGE_REQUEST_RECORDS)
    {
        let fields_by_role = record.helper_stage_contract_fields_by_role();
        let Some(review_fields) = fields_by_role.get("review") else {
            continue;
        };
        let Some(change_request) = review_fields.get("change_request") else {
            continue;
        };
        if completed_final_json_pool_stage_dispatch(change_request, record) {
            completed.insert(
                "review.change_request requested final_json.pool_stage_dispatch; latest_successful_final_json_pool_stage_dispatch_has_required_task_kinds=true"
                    .to_owned(),
            );
        }
        if let Some(index_fields) = fields_by_role.get("index") {
            for marker in helper_feedback::contract_markers("index") {
                if completed_index_contract_marker(change_request, index_fields, marker) {
                    completed.insert(format!(
                        "review.change_request requested index.{marker}; latest_successful_index_contract_has_{marker}=true"
                    ));
                }
            }
        }
    }
    completed.into_iter().collect()
}

fn recent_invalid_change_requests(
    records: &[ReportRecord],
    has_coverage_evidence: bool,
) -> Vec<String> {
    let mut invalid = BTreeSet::new();
    for record in records
        .iter()
        .rev()
        .filter(|record| record.success)
        .take(RECENT_COMPLETED_CHANGE_REQUEST_RECORDS)
    {
        let fields_by_role = record.helper_stage_contract_fields_by_role();
        if let Some(review_fields) = fields_by_role.get("review")
            && let Some(change_request) = review_fields.get("change_request")
            && change_request_has_invalid_cargo_test_strict_flag(change_request)
        {
            invalid.insert(invalid_cargo_test_strict_flag_summary());
        }
        if let Some(review_fields) = fields_by_role.get("review")
            && let Some(change_request) = review_fields.get("change_request")
            && change_request_has_redundant_max_iterations_flag(change_request)
        {
            invalid.insert(invalid_redundant_max_iterations_summary());
        }
        if let Some(review_fields) = fields_by_role.get("review")
            && let Some(change_request) = review_fields.get("change_request")
            && !has_coverage_evidence
            && change_request_has_unproven_strict_coverage_control(change_request)
        {
            invalid.insert(invalid_unproven_strict_coverage_summary());
        }
        if let Some(review_fields) = fields_by_role.get("review")
            && let Some(change_request) = review_fields.get("change_request")
            && change_request_has_unproven_test_seed_control(change_request)
        {
            invalid.insert(invalid_unproven_test_seed_summary());
        }
        if let Some(review_feedback) = record.helper_stage_feedback_by_role().get("review") {
            for feedback in review_feedback {
                if change_request_has_invalid_cargo_test_strict_flag(feedback) {
                    invalid.insert(invalid_cargo_test_strict_flag_summary());
                }
                if change_request_has_redundant_max_iterations_flag(feedback) {
                    invalid.insert(invalid_redundant_max_iterations_summary());
                }
                if !has_coverage_evidence
                    && change_request_has_unproven_strict_coverage_control(feedback)
                {
                    invalid.insert(invalid_unproven_strict_coverage_summary());
                }
                if change_request_has_unproven_test_seed_control(feedback) {
                    invalid.insert(invalid_unproven_test_seed_summary());
                }
            }
        }
    }
    invalid.into_iter().collect()
}

fn invalid_cargo_test_strict_flag_summary() -> String {
    "review.change_request requested unsupported cargo.test.strict-flag; cargo_test_has_no_strict_flag=true"
        .to_owned()
}

fn invalid_redundant_max_iterations_summary() -> String {
    "review.change_request requested redundant evolution-loop.max-iterations flag; evolution_loop_already_has_rounds_forever_and_budget_stop_controls=true"
        .to_owned()
}

fn invalid_unproven_strict_coverage_summary() -> String {
    "review.change_request requested unproven evolution-loop.strict-coverage control; require_existing_coverage_tooling_or_coverage_report_before_strict_coverage_work=true"
        .to_owned()
}

fn invalid_unproven_test_seed_summary() -> String {
    "review.change_request requested unproven evolution-loop.test-deterministic-seed control; require_flaky_test_or_randomness_evidence_before_seed_work=true"
        .to_owned()
}

fn change_request_has_invalid_cargo_test_strict_flag(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower
        .split(|character| matches!(character, '\n' | '\r' | '/' | ';'))
        .any(|segment| {
            let mentions_cargo_test = segment.contains("cargo test");
            let mentions_strict_flag =
                segment.contains("--strict") || segment.contains("strict flag");
            mentions_cargo_test && mentions_strict_flag
        })
}

fn validation_command_coverage_evidence(
    records: &[ReportRecord],
) -> ValidationCommandCoverageEvidence {
    let strict_coverage_requested = latest_validation_command_requests_strict_coverage(records);
    let coverage_tooling_evidence = records
        .iter()
        .flat_map(record_coverage_tooling_evidence)
        .collect::<Vec<_>>();
    let coverage_report_evidence = records
        .iter()
        .flat_map(record_coverage_report_evidence)
        .collect::<Vec<_>>();

    ValidationCommandCoverageEvidence::from_observations([])
        .with_strict_coverage_requested(strict_coverage_requested)
        .with_coverage_tooling_evidence(coverage_tooling_evidence)
        .with_coverage_report_evidence(coverage_report_evidence)
}

fn latest_validation_command_requests_strict_coverage(records: &[ReportRecord]) -> bool {
    records
        .iter()
        .rev()
        .find(|record| record.validation_checked.is_some())
        .and_then(|record| record.validation_command_preview.as_deref())
        .is_some_and(validation_command_requests_strict_coverage)
}

fn validation_command_requests_strict_coverage(command: &str) -> bool {
    let lower = command.to_ascii_lowercase();
    let normalized = normalized_advice_key(command);
    lower.contains("--strict-coverage")
        || normalized.contains("strict coverage")
        || normalized.contains("coverage enforcement")
}

fn record_coverage_tooling_evidence(record: &ReportRecord) -> Vec<String> {
    let mut evidence = Vec::new();
    if record.validation_checked == Some(true) && record.validation_passed == Some(true) {
        for text in [
            record.validation_command_preview.as_deref(),
            record.validation_stdout_tail.as_deref(),
            record.validation_stderr_tail.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            if text_mentions_coverage_tooling(text) {
                evidence.push(text.to_owned());
            }
        }
    }

    for fields in record
        .structured_helper_stage_contract_fields_by_role
        .values()
    {
        for (field, value) in fields {
            if helper_stage_field_is_coverage_tooling_evidence(field, value) {
                evidence.push(value.to_owned());
            }
        }
    }
    evidence
}

fn record_coverage_report_evidence(record: &ReportRecord) -> Vec<String> {
    let mut evidence = Vec::new();
    let validation_passed =
        record.validation_checked == Some(true) && record.validation_passed == Some(true);
    if validation_passed {
        for text in [
            record.validation_command_preview.as_deref(),
            record.validation_stdout_tail.as_deref(),
            record.validation_stderr_tail.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            if text_has_coverage_report_artifact(text) {
                evidence.push(text.to_owned());
            }
        }
    }

    if let Some(eval_json) = record
        .eval_json
        .as_deref()
        .filter(|text| text_has_coverage_report_artifact(text))
    {
        evidence.push(eval_json.to_owned());
    }
    for entry in &record.meta {
        if text_has_coverage_report_artifact(entry) {
            evidence.push(entry.to_owned());
        }
    }
    for fields in record
        .structured_helper_stage_contract_fields_by_role
        .values()
    {
        for (field, value) in fields {
            if helper_stage_field_is_coverage_report_evidence(field, value) {
                evidence.push(value.to_owned());
            }
        }
    }
    evidence
}

fn helper_stage_field_is_coverage_tooling_evidence(field: &str, value: &str) -> bool {
    let normalized_field = normalized_advice_key(field);
    !value.trim().is_empty()
        && (normalized_field.contains("coverage tooling evidence")
            || (normalized_field.contains("coverage") && text_mentions_coverage_tooling(value)))
}

fn helper_stage_field_is_coverage_report_evidence(field: &str, value: &str) -> bool {
    let normalized_field = normalized_advice_key(field);
    !value.trim().is_empty()
        && (normalized_field.contains("coverage report evidence")
            || (normalized_field.contains("coverage") && text_has_coverage_report_artifact(value)))
}

fn text_mentions_coverage_tooling(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let normalized = normalized_advice_key(text);
    lower.contains("llvm-cov")
        || normalized.contains("llvm cov")
        || normalized.contains("cargo tarpaulin")
        || normalized.contains("tarpaulin")
        || normalized.contains("grcov")
        || normalized.contains("lcov")
}

fn text_has_coverage_report_artifact(text: &str) -> bool {
    let normalized = normalized_advice_key(text);
    normalized.contains("validation command coverage report v1")
        || normalized.contains("coverage report v1")
        || (normalized.contains("coverage report")
            && (text.contains('%')
                || normalized.contains("line coverage")
                || normalized.contains("branch coverage")
                || normalized.contains("function coverage")
                || normalized.contains("region coverage")
                || normalized.contains("report path")
                || normalized.contains("total lines")))
}

fn change_request_has_redundant_max_iterations_flag(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let normalized = normalized_advice_key(text);
    let mentions_max_iterations =
        lower.contains("--max-iterations") || normalized.contains("max iterations");
    if !mentions_max_iterations {
        return false;
    }
    let mentions_evolution_loop = normalized.contains("evolution loop")
        || normalized.contains("number of rounds")
        || normalized.contains("runaway loop")
        || normalized.contains("rounds");
    let requests_new_control = normalized.contains("add")
        || normalized.contains("introduce")
        || normalized.contains("implement")
        || normalized.contains("accept")
        || normalized.contains("utilize")
        || normalized.contains("cap")
        || normalized.contains("limit");
    mentions_evolution_loop && requests_new_control
}

fn change_request_has_unproven_strict_coverage_control(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let normalized = normalized_advice_key(text);
    let mentions_strict_coverage = lower.contains("--strict-coverage")
        || normalized.contains("strict coverage")
        || normalized.contains("coverage enforcement")
        || normalized.contains("coverage report")
        || normalized.contains("coverage gate")
        || (normalized.contains("report gate") && normalized.contains("coverage"))
        || normalized.contains("coverage threshold")
        || normalized.contains("coverage validation")
        || normalized.contains("enforce 100 line coverage")
        || normalized.contains("enforce 100 percent line coverage")
        || normalized.contains("100 line coverage")
        || normalized.contains("100 percent line coverage");
    if !mentions_strict_coverage {
        return false;
    }
    let mentions_evolution_loop = normalized.contains("evolution loop")
        || normalized.contains("evolution loop rs")
        || normalized.contains("test harness")
        || normalized.contains("cargo test")
        || normalized.contains("validation command")
        || normalized.contains("report gate")
        || normalized.contains("report json")
        || normalized.contains("ledger report");
    let requests_new_control = normalized.contains("add")
        || normalized.contains("enforce")
        || normalized.contains("implement")
        || normalized.contains("introduce")
        || normalized.contains("require")
        || normalized.contains("gate")
        || normalized.contains("threshold")
        || normalized.contains("strict");
    mentions_evolution_loop && requests_new_control
}

fn change_request_has_unproven_test_seed_control(text: &str) -> bool {
    let normalized = normalized_advice_key(text);
    let mentions_seed = normalized.contains("deterministic seed")
        || normalized.contains("fixed seed")
        || normalized.contains("known random seed")
        || normalized.contains("random seed");
    if !mentions_seed {
        return false;
    }
    let mentions_evolution_tests = normalized.contains("evolution loop")
        && (normalized.contains("test harness")
            || normalized.contains("test execution")
            || normalized.contains("test output")
            || normalized.contains("cargo test")
            || normalized.contains("validation command"));
    let requests_new_control = normalized.contains("add")
        || normalized.contains("initialize")
        || normalized.contains("modify")
        || normalized.contains("set")
        || normalized.contains("ensure")
        || normalized.contains("implement");
    mentions_evolution_tests && requests_new_control
}

fn completed_final_json_pool_stage_dispatch(change_request: &str, record: &ReportRecord) -> bool {
    let normalized_change_request = normalized_advice_key(change_request);
    if !change_request_requests_final_json_pool_stage_dispatch(&normalized_change_request) {
        return false;
    }
    let task_kinds = final_json_pool_stage_dispatch_task_kinds(record);
    REQUIRED_FINAL_JSON_POOL_STAGE_DISPATCH_TASK_KINDS
        .iter()
        .all(|required| task_kinds.iter().any(|task_kind| task_kind == required))
}

fn change_request_requests_final_json_pool_stage_dispatch(normalized_change_request: &str) -> bool {
    let mentions_dispatch = normalized_change_request.contains("final json pool stage dispatch")
        || normalized_change_request.contains("pool stage dispatch");
    if !mentions_dispatch {
        return false;
    }
    normalized_change_request.contains("strict")
        || normalized_change_request.contains("gate")
        || normalized_change_request.contains("require")
        || normalized_change_request.contains("required")
        || normalized_change_request.contains("complete")
        || normalized_change_request.contains("completeness")
        || normalized_change_request.contains("missing")
        || normalized_change_request.contains("persist")
        || normalized_change_request.contains("project")
}

fn completed_index_contract_marker(
    change_request: &str,
    index_fields: &BTreeMap<String, String>,
    marker: &str,
) -> bool {
    let normalized_change_request = normalized_advice_key(change_request);
    if !change_request_explicitly_requests_field(&normalized_change_request, marker) {
        return false;
    }
    let Some(value) = index_fields.get(marker).map(String::as_str) else {
        return false;
    };
    if value.trim().is_empty() || value.eq_ignore_ascii_case("none") {
        return false;
    }
    if marker == "source_origin" {
        return index_fields
            .get("tags")
            .is_some_and(|tags| tags.contains("source_origin="));
    }
    true
}

fn change_request_explicitly_requests_field(normalized_change_request: &str, marker: &str) -> bool {
    let marker = normalized_advice_key(marker);
    normalized_change_request.contains(&format!("{marker} field"))
        || normalized_change_request.contains(&format!("field {marker}"))
        || normalized_change_request.contains(&format!("mandate {marker}"))
        || normalized_change_request.contains(&format!("{marker} is required"))
        || normalized_change_request.contains(&format!("presence of {marker}"))
        || normalized_change_request.contains(&format!("confirm {marker}"))
}

fn helper_stage_hygiene_by_role(
    feedback_by_role: &BTreeMap<String, Vec<String>>,
) -> BTreeMap<String, Vec<HelperStageHygieneFinding>> {
    feedback_by_role
        .iter()
        .filter_map(|(role, feedback)| {
            let findings = helper_stage_feedback_hygiene_findings(role, feedback);
            (!findings.is_empty()).then(|| (role.clone(), findings))
        })
        .collect()
}

fn parse_record(line: &str) -> ReportRecord {
    let final_preview = json_string_field(line, "final_preview");
    let final_json = final_preview.clone().unwrap_or_default();
    let eval_json =
        json_object_field(line, "eval").or_else(|| json_object_field(&final_json, "eval"));
    let eval_report_only = eval_json
        .as_deref()
        .and_then(|eval_json| json_bool_field(eval_json, "report_only"));
    let eval_failure_kind = eval_json
        .as_deref()
        .and_then(|eval_json| json_string_field(eval_json, "failure_kind"));
    let allocation_evidence = json_array_field(line, "allocation_evidence")
        .map(|array| parse_json_string_array(&array))
        .unwrap_or_default();
    let meta = json_array_field(line, "meta")
        .map(|array| parse_json_string_array(&array))
        .unwrap_or_default();
    let structured_helper_stage_feedback_by_role =
        json_object_field(line, "helper_stage_feedback_by_role")
            .map(|object| parse_json_string_array_map(&object))
            .unwrap_or_default();
    let structured_helper_stage_contract_fields_by_role =
        json_object_field(line, "helper_stage_contract_by_role")
            .map(|object| helper_feedback::contract_fields_by_role_from_json(&object))
            .unwrap_or_default();
    ReportRecord {
        round: json_u64_field(line, "round"),
        case_name: json_string_field(line, "case"),
        started_unix: json_u64_field(line, "started_unix"),
        finished_unix: json_u64_field(line, "finished_unix"),
        success: json_bool_field(line, "success").unwrap_or(false),
        error: json_string_field(line, "error"),
        runtime_tokens: json_u64_field(line, "runtime_tokens")
            .or_else(|| json_u64_field(&final_json, "runtime_token_count")),
        runtime_model: json_string_field(line, "runtime_model")
            .or_else(|| json_string_field(&final_json, "runtime_model")),
        answer: json_string_field(line, "answer")
            .or_else(|| json_string_field(&final_json, "answer")),
        final_json: final_preview,
        final_json_pool_stage_dispatch: json_array_field(line, "final_json_pool_stage_dispatch"),
        elapsed_ms: json_u64_field(line, "elapsed_ms")
            .or_else(|| json_u64_field(&final_json, "elapsed_ms")),
        feedback_applied: json_u64_field(line, "feedback_applied")
            .or_else(|| json_u64_field(&final_json, "feedback_applied")),
        rust_check_checked: json_bool_field(line, "rust_check_checked")
            .or_else(|| json_bool_field(&final_json, "rust_check_checked")),
        rust_check_passed: json_bool_field(line, "rust_check_passed")
            .or_else(|| json_bool_field(&final_json, "rust_check_passed")),
        rust_check_feedback_applied: json_u64_field(line, "rust_check_feedback_applied")
            .or_else(|| json_u64_field(&final_json, "rust_check_feedback_applied")),
        validation_checked: json_bool_field(line, "validation_checked"),
        validation_passed: json_bool_field(line, "validation_passed"),
        validation_command_source: json_string_field(line, "validation_command_source"),
        validation_command_safety: json_string_field(line, "validation_command_safety"),
        validation_command_preview: json_string_field(line, "validation_command_preview"),
        validation_phase: json_string_field(line, "validation_phase"),
        validation_status_code: json_i32_field(line, "validation_status_code"),
        validation_elapsed_ms: json_u64_field(line, "validation_elapsed_ms"),
        validation_stdout_tail: json_string_field(line, "validation_stdout_tail"),
        validation_stderr_tail: json_string_field(line, "validation_stderr_tail"),
        self_improve_passed: json_bool_field(line, "self_improve_passed")
            .or_else(|| json_bool_field(&final_json, "self_improve_passed")),
        state_gate_checked: json_bool_field(line, "state_gate_checked")
            .or_else(|| json_bool_field(&final_json, "state_gate_checked")),
        state_gate_passed: json_bool_field(line, "state_gate_passed")
            .or_else(|| json_bool_field(&final_json, "state_gate_passed")),
        trace_gate_checked: json_bool_field(line, "trace_gate_checked")
            .or_else(|| json_bool_field(&final_json, "trace_gate_checked")),
        trace_gate_passed: json_bool_field(line, "trace_gate_passed")
            .or_else(|| json_bool_field(&final_json, "trace_gate_passed")),
        eval_json,
        eval_report_only,
        eval_failure_kind,
        meta,
        structured_helper_stage_feedback_by_role,
        structured_helper_stage_contract_fields_by_role,
        allocation_evidence,
    }
}

fn print_report(
    config: &Config,
    summary: &ReportSummary,
    remote_chain_status: Option<&RemoteChainStatusSummary>,
    pool_manifest: Option<&PoolManifestSummary>,
    pool_status: Option<&PoolStatusSummary>,
    pool_route: Option<&PoolRouteSummary>,
    pool_budget_fairness: Option<&PoolBudgetFairnessSummary>,
    worker_window_status: Option<&WorkerWindowStatusSummary>,
    clean_room_batch_status: Option<&CleanRoomBatchStatusSummary>,
    clean_room_handoff: Option<&CleanRoomHandoffSummary>,
    self_improve_proposal_artifact: &SelfImproveProposalArtifact,
) {
    println!("SmartSteam evolution report");
    println!("ledger: {}", config.ledger_path.display());
    println!("rounds: {}", summary.total);
    println!(
        "ledger_hygiene: unique_rounds={} duplicate_rounds={} non_monotonic_rounds={} missing_rounds={} round_gaps={}",
        summary.unique_rounds,
        summary.duplicate_rounds,
        summary.non_monotonic_rounds,
        summary.missing_rounds,
        summary.round_gaps
    );
    println!(
        "success: {}/{} ({:.1}%)",
        summary.success,
        summary.total,
        percent(summary.success, summary.total)
    );
    println!("failures: {}", summary.failure);
    println!(
        "stream_failures: truncated={} missing_final={}",
        summary.stream_truncation_failures, summary.missing_final_failures
    );
    println!(
        "runtime_response_failures: {}",
        summary.runtime_response_failures
    );
    println!(
        "runtime_tokens: total={} avg={}",
        summary.runtime_tokens,
        average_text(summary.runtime_tokens, summary.runtime_token_items)
    );
    println!(
        "elapsed_ms: total={} avg={}",
        summary.elapsed_ms,
        average_text(summary.elapsed_ms, summary.elapsed_items)
    );
    println!(
        "round_wall_elapsed_ms: total={} avg={}",
        summary.round_wall_elapsed_ms,
        average_text(
            summary.round_wall_elapsed_ms,
            summary.round_wall_elapsed_items
        )
    );
    println!(
        "feedback_applied: total={} avg={}",
        summary.feedback_applied,
        average_text(summary.feedback_applied, summary.feedback_items)
    );
    println!(
        "rust_check_passed: {}/{}",
        summary.rust_check_passed, summary.rust_check_checked
    );
    println!(
        "rust_check_feedback_applied: total={} avg={}",
        summary.rust_check_feedback_applied,
        average_text(
            summary.rust_check_feedback_applied,
            summary.rust_check_feedback_items
        )
    );
    println!(
        "validation_passed: {}/{}",
        summary.validation_passed, summary.validation_checked
    );
    println!(
        "self_improve_passed: {}/{}",
        summary.self_improve_passed, summary.self_improve_checked
    );
    println!(
        "state_gate_passed: {}/{}",
        summary.state_gate_passed, summary.state_gate_checked
    );
    println!(
        "trace_gate_passed: {}/{}",
        summary.trace_gate_passed, summary.trace_gate_checked
    );
    println!(
        "eval_report_only: records={} report_only={} failure_kinds={}",
        summary.eval_records,
        summary.eval_report_only_records,
        eval_failure_kinds_text(&summary.eval_failure_kinds)
    );
    let report_helper_stage_feedback_by_role =
        filtered_report_helper_stage_feedback_by_role(&summary.helper_stage_feedback_by_role);
    if !report_helper_stage_feedback_by_role.is_empty() {
        println!(
            "helper_stage_feedback_by_role: {}",
            helper_stage_feedback_by_role_text(&report_helper_stage_feedback_by_role)
        );
    }
    if !summary.helper_stage_hygiene_by_role.is_empty() {
        println!(
            "helper_stage_hygiene_by_role: {}",
            helper_stage_hygiene_by_role_text(&summary.helper_stage_hygiene_by_role)
        );
    }
    let report_helper_stage_contract_by_role =
        filtered_report_helper_stage_contract_by_role(&summary.helper_stage_contract_by_role);
    if !report_helper_stage_contract_by_role.is_empty() {
        println!(
            "helper_stage_contract_by_role: {}",
            helper_stage_contract_by_role_text(&report_helper_stage_contract_by_role)
        );
    }
    let helper_stage_repair_status =
        helper_stage_repair_status(summary, &config.required_latest_helper_stage_roles);
    println!(
        "helper_stage_repair_status_report_v1: {}",
        helper_stage_repair::context_text(&helper_stage_repair_status)
    );
    if has_test_gate_summary(&summary.test_gate) {
        println!("test_gate: {}", test_gate_context_text(&summary.test_gate));
    }
    if let Some(remote_chain_status) = remote_chain_status {
        println!(
            "remote_chain: {}",
            remote_chain::context_text(remote_chain_status)
        );
    }
    if let Some(pool_manifest) = pool_manifest {
        println!(
            "model_pool_manifest: {}",
            pool_artifacts::manifest_context_text(pool_manifest)
        );
    }
    if let Some(pool_status) = pool_status {
        println!(
            "model_pool: {}",
            pool_artifacts::status_context_text(pool_status)
        );
    }
    if let Some(pool_route) = pool_route {
        println!(
            "model_pool_route: {}",
            pool_artifacts::route_context_text(pool_route)
        );
    }
    if let Some(pool_alignment) = pool_alignment_summary(pool_manifest, pool_status, pool_route) {
        println!(
            "model_pool_alignment: {}",
            pool_artifacts::alignment_context_text(&pool_alignment)
        );
    }
    if let Some(pool_budget_fairness) = pool_budget_fairness {
        println!(
            "model_pool_budget_fairness_report_v1: {}",
            pool_artifacts::budget_fairness_context_text(pool_budget_fairness)
        );
    }
    if let Some(worker_window_status) = worker_window_status {
        println!(
            "worker_window_replacement_report_v1: windows={} paused={} polluted={} replacements={} replacement_required={} blocked_originals={} side_effects_allowed={}",
            worker_window_status.window_count,
            worker_window_status.paused_count,
            worker_window_status.polluted_count,
            worker_window_status.clean_room_replacement_count,
            worker_window_status.replacement_required_count,
            worker_window_status.blocked_original_count,
            option_bool_json(worker_window_status.side_effects_allowed)
        );
    }
    if let Some(clean_room_batch_status) = clean_room_batch_status {
        println!(
            "clean_room_batch_status_report_v1: r24_completed={} r24_workers={} r25_replacements_open={} r25_workers={} old_windows_blocked={} blocked_old_windows={} main_runtime_owner={} worker_runtime_owner_allowed={} side_effects_allowed={}",
            clean_room_batch_status.r24_completed,
            clean_room_batch_status.r24_completed_worker_ids.len(),
            clean_room_batch_status.r25_clean_room_replacements_open,
            clean_room_batch_status
                .r25_clean_room_replacement_worker_ids
                .len(),
            clean_room_batch_status.old_polluted_windows_blocked,
            clean_room_batch_status.blocked_old_window_ids.len(),
            clean_room_batch_status.main_window_runtime_owner,
            clean_room_batch_status.worker_runtime_ownership_allowed,
            option_bool_json(clean_room_batch_status.side_effects_allowed)
        );
    }
    if let Some(clean_room_handoff) = clean_room_handoff {
        let memory = clean_room_handoff.memory_startup_admission.as_ref();
        let agent = clean_room_handoff.agent_replacement_plan.as_ref();
        println!(
            "clean_room_handoff_report_v1: memory_loaded={} agent_plan_loaded={} memory_admission_decisions={} memory_store_mutations={} agent_prompt_tasks={} agent_reason_codes={}",
            memory.is_some(),
            agent.is_some(),
            memory
                .map(|status| status.admission_decision_count)
                .unwrap_or_default(),
            memory
                .map(|status| status.store_mutation_count)
                .unwrap_or_default(),
            agent.map(|plan| plan.task_ids.len()).unwrap_or_default(),
            agent
                .map(|plan| plan.reason_codes.len())
                .unwrap_or_default()
        );
    }
    println!(
        "self_improve_proposal_artifact_v1: candidates_total={} projected={} candidate_only=true auto_apply=false",
        self_improve_proposal_artifact.total_candidate_count,
        self_improve_proposal_artifact.proposals.len()
    );
    let self_improve_proposal_acceptance_summary =
        self_improve_proposal_artifact.acceptance_summary_report();
    println!(
        "self_improve_proposal_acceptance_summary_v1: projected={} accepted_memory={} evidence_backed_business={} advisory_only={} repair_required={} accepted_without_business_evidence={} only_advisory_or_repair={}",
        self_improve_proposal_acceptance_summary.projected_report_count,
        self_improve_proposal_acceptance_summary.memory_admission_accepted_count,
        self_improve_proposal_acceptance_summary.evidence_backed_business_improvement_count,
        self_improve_proposal_acceptance_summary.advisory_only_count,
        self_improve_proposal_acceptance_summary.require_repair_count,
        self_improve_proposal_acceptance_summary.accepted_without_business_evidence_count,
        self_improve_proposal_acceptance_summary.only_advisory_or_repair()
    );
    let self_improve_proposal_action_assignment =
        self_improve_proposal_artifact.acceptance_action_assignment();
    let first_action_target = self_improve_proposal_action_assignment.first_target_digest();
    let first_action_target_id = first_action_target
        .as_ref()
        .map(|target| target.proposal_id.as_str())
        .unwrap_or("-");
    let first_action_target_missing = first_action_target
        .as_ref()
        .map(|target| target.missing_requirements.join(","))
        .unwrap_or_else(|| "-".to_owned());
    println!(
        "self_improve_proposal_action_assignment_v1: action_required={} primary_action={} targets={} first_target={} first_missing={}",
        self_improve_proposal_action_assignment.action_required,
        self_improve_proposal_action_assignment.primary_action,
        self_improve_proposal_action_assignment.target_count,
        first_action_target_id,
        first_action_target_missing
    );
    let self_improve_proposal_action_closure =
        self_improve_proposal_artifact.action_closure_report();
    let first_closure_target = self_improve_proposal_action_closure
        .first_target_id
        .as_deref()
        .unwrap_or("-");
    let first_closure_kind = self_improve_proposal_action_closure
        .first_target_closure_kind
        .as_deref()
        .unwrap_or("-");
    println!(
        "self_improve_proposal_action_closure_report_v1: targets={} closed={} open={} first_target={} first_closed={} first_kind={} first_still_requires_memory_admission={}",
        self_improve_proposal_action_closure.target_count,
        self_improve_proposal_action_closure.closed_target_count,
        self_improve_proposal_action_closure.open_target_count,
        first_closure_target,
        self_improve_proposal_action_closure.first_target_closed,
        first_closure_kind,
        self_improve_proposal_action_closure.first_target_still_requires_memory_admission
    );
    let self_improve_proposal_memory_admission_readiness =
        self_improve_proposal_artifact.memory_admission_readiness_report();
    let first_readiness_target = self_improve_proposal_memory_admission_readiness
        .first_target_id
        .as_deref()
        .unwrap_or("-");
    println!(
        "self_improve_proposal_memory_admission_readiness_report_v1: targets={} ready={} blocked={} first_target={} first_ready={} all_closed_targets_ready={} memory_store_write_allowed={} ndkv_write_allowed={}",
        self_improve_proposal_memory_admission_readiness.target_count,
        self_improve_proposal_memory_admission_readiness.ready_count,
        self_improve_proposal_memory_admission_readiness.blocked_count,
        first_readiness_target,
        self_improve_proposal_memory_admission_readiness.first_target_ready,
        self_improve_proposal_memory_admission_readiness.all_closed_targets_ready,
        self_improve_proposal_memory_admission_readiness.memory_store_write_allowed,
        self_improve_proposal_memory_admission_readiness.ndkv_write_allowed
    );
    let self_improve_proposal_memory_admission_request =
        self_improve_proposal_artifact.memory_admission_request_report();
    let first_request_candidate = self_improve_proposal_memory_admission_request
        .first_candidate_id
        .as_deref()
        .unwrap_or("-");
    println!(
        "self_improve_proposal_memory_admission_request_report_v1: targets={} requests={} blocked={} first_candidate={} first_ready={} all_ready_targets_requested={} writer_required={} auto_apply={} memory_store_write_allowed={} ndkv_write_allowed={}",
        self_improve_proposal_memory_admission_request.target_count,
        self_improve_proposal_memory_admission_request.request_count,
        self_improve_proposal_memory_admission_request.blocked_count,
        first_request_candidate,
        self_improve_proposal_memory_admission_request.first_candidate_ready,
        self_improve_proposal_memory_admission_request.all_ready_targets_requested,
        self_improve_proposal_memory_admission_request.writer_required,
        self_improve_proposal_memory_admission_request.auto_apply,
        self_improve_proposal_memory_admission_request.memory_store_write_allowed,
        self_improve_proposal_memory_admission_request.ndkv_write_allowed
    );
    let self_improve_proposal_memory_admission_decision =
        self_improve_proposal_artifact.memory_admission_decision_report();
    let first_decision_candidate = self_improve_proposal_memory_admission_decision
        .first_candidate_id
        .as_deref()
        .unwrap_or("-");
    println!(
        "self_improve_proposal_memory_admission_decision_report_v1: targets={} requests={} blocked={} first_candidate={} writer_required={} preflight_passed={} explicit_writer_invocation_required={} admission_write_authorized={} gate_blocked={} failure_reasons={} auto_apply={} memory_store_write_allowed={} ndkv_write_allowed={}",
        self_improve_proposal_memory_admission_decision.target_count,
        self_improve_proposal_memory_admission_decision.request_count,
        self_improve_proposal_memory_admission_decision.blocked_count,
        first_decision_candidate,
        self_improve_proposal_memory_admission_decision.writer_required,
        self_improve_proposal_memory_admission_decision.admission_writer_preflight_passed,
        self_improve_proposal_memory_admission_decision.explicit_writer_invocation_required,
        self_improve_proposal_memory_admission_decision.admission_write_authorized,
        self_improve_proposal_memory_admission_decision.gate_blocked,
        self_improve_proposal_memory_admission_decision
            .failure_reasons
            .join(","),
        self_improve_proposal_memory_admission_decision.auto_apply,
        self_improve_proposal_memory_admission_decision.memory_store_write_allowed,
        self_improve_proposal_memory_admission_decision.ndkv_write_allowed
    );
    let self_improve_proposal_memory_admission_writer_plan =
        self_improve_proposal_artifact.memory_admission_writer_plan();
    let first_writer_plan_item = self_improve_proposal_memory_admission_writer_plan
        .first_plan_item_id
        .as_deref()
        .unwrap_or("-");
    println!(
        "self_improve_proposal_memory_admission_writer_plan_report_v1: targets={} requests={} plan_items={} ready={} blocked={} first_item={} writer_plan_ready={} explicit_writer_invocation_required={} experiment_required={} rollback_required={} validation_required={} admission_write_authorized={} auto_apply={} memory_store_write_allowed={} ndkv_write_allowed={}",
        self_improve_proposal_memory_admission_writer_plan.target_count,
        self_improve_proposal_memory_admission_writer_plan.request_count,
        self_improve_proposal_memory_admission_writer_plan.writer_plan_item_count,
        self_improve_proposal_memory_admission_writer_plan.ready_plan_count,
        self_improve_proposal_memory_admission_writer_plan.blocked_count,
        first_writer_plan_item,
        self_improve_proposal_memory_admission_writer_plan.writer_plan_ready,
        self_improve_proposal_memory_admission_writer_plan.explicit_writer_invocation_required,
        self_improve_proposal_memory_admission_writer_plan.experiment_required,
        self_improve_proposal_memory_admission_writer_plan.rollback_required,
        self_improve_proposal_memory_admission_writer_plan.validation_required,
        self_improve_proposal_memory_admission_writer_plan.admission_write_authorized,
        self_improve_proposal_memory_admission_writer_plan.auto_apply,
        self_improve_proposal_memory_admission_writer_plan.memory_store_write_allowed,
        self_improve_proposal_memory_admission_writer_plan.ndkv_write_allowed
    );
    let self_improve_proposal_memory_admission_writer_dry_run =
        self_improve_proposal_artifact.memory_admission_writer_dry_run();
    let first_writer_dry_run_item = self_improve_proposal_memory_admission_writer_dry_run
        .first_dry_run_item_id
        .as_deref()
        .unwrap_or("-");
    println!(
        "self_improve_proposal_memory_admission_writer_dry_run_report_v1: targets={} requests={} plan_items={} dry_run_items={} ready={} blocked={} first_item={} dry_run_ready={} explicit_writer_invocation_required={} dry_run_required={} experiment_required={} rollback_required={} validation_required={} admission_write_authorized={} auto_apply={} memory_store_write_allowed={} ndkv_write_allowed={}",
        self_improve_proposal_memory_admission_writer_dry_run.target_count,
        self_improve_proposal_memory_admission_writer_dry_run.request_count,
        self_improve_proposal_memory_admission_writer_dry_run.writer_plan_item_count,
        self_improve_proposal_memory_admission_writer_dry_run.dry_run_item_count,
        self_improve_proposal_memory_admission_writer_dry_run.ready_dry_run_count,
        self_improve_proposal_memory_admission_writer_dry_run.blocked_count,
        first_writer_dry_run_item,
        self_improve_proposal_memory_admission_writer_dry_run.dry_run_ready,
        self_improve_proposal_memory_admission_writer_dry_run.explicit_writer_invocation_required,
        self_improve_proposal_memory_admission_writer_dry_run.dry_run_required,
        self_improve_proposal_memory_admission_writer_dry_run.experiment_required,
        self_improve_proposal_memory_admission_writer_dry_run.rollback_required,
        self_improve_proposal_memory_admission_writer_dry_run.validation_required,
        self_improve_proposal_memory_admission_writer_dry_run.admission_write_authorized,
        self_improve_proposal_memory_admission_writer_dry_run.auto_apply,
        self_improve_proposal_memory_admission_writer_dry_run.memory_store_write_allowed,
        self_improve_proposal_memory_admission_writer_dry_run.ndkv_write_allowed
    );
    let self_improve_proposal_memory_admission_writer_dry_run_receipt =
        self_improve_proposal_artifact.memory_admission_writer_dry_run_receipt();
    let first_writer_dry_run_receipt_item =
        self_improve_proposal_memory_admission_writer_dry_run_receipt
            .first_receipt_item_id
            .as_deref()
            .unwrap_or("-");
    println!(
        "self_improve_proposal_memory_admission_writer_dry_run_receipt_report_v1: targets={} requests={} dry_run_items={} receipt_items={} succeeded={} blocked={} first_item={} dry_run_receipt_ready={} explicit_writer_invocation_required={} commit_allowed={} validation_required={} rollback_required={} admission_write_authorized={} auto_apply={} memory_store_write_allowed={} ndkv_write_allowed={}",
        self_improve_proposal_memory_admission_writer_dry_run_receipt.target_count,
        self_improve_proposal_memory_admission_writer_dry_run_receipt.request_count,
        self_improve_proposal_memory_admission_writer_dry_run_receipt.dry_run_item_count,
        self_improve_proposal_memory_admission_writer_dry_run_receipt.receipt_item_count,
        self_improve_proposal_memory_admission_writer_dry_run_receipt.succeeded_receipt_count,
        self_improve_proposal_memory_admission_writer_dry_run_receipt.blocked_count,
        first_writer_dry_run_receipt_item,
        self_improve_proposal_memory_admission_writer_dry_run_receipt.dry_run_receipt_ready,
        self_improve_proposal_memory_admission_writer_dry_run_receipt
            .explicit_writer_invocation_required,
        self_improve_proposal_memory_admission_writer_dry_run_receipt.commit_allowed,
        self_improve_proposal_memory_admission_writer_dry_run_receipt.validation_required,
        self_improve_proposal_memory_admission_writer_dry_run_receipt.rollback_required,
        self_improve_proposal_memory_admission_writer_dry_run_receipt.admission_write_authorized,
        self_improve_proposal_memory_admission_writer_dry_run_receipt.auto_apply,
        self_improve_proposal_memory_admission_writer_dry_run_receipt.memory_store_write_allowed,
        self_improve_proposal_memory_admission_writer_dry_run_receipt.ndkv_write_allowed
    );
    let self_improve_proposal_memory_admission_commit_record_stage =
        self_improve_proposal_artifact.memory_admission_commit_record_stage();
    let first_commit_record_stage_item = self_improve_proposal_memory_admission_commit_record_stage
        .first_commit_record_item_id
        .as_deref()
        .unwrap_or("-");
    println!(
        "self_improve_proposal_memory_admission_commit_record_stage_report_v1: targets={} requests={} receipt_items={} commit_record_items={} staged={} blocked={} first_item={} commit_record_stage_ready={} explicit_writer_invocation_required={} validation_required={} rollback_required={} commit_allowed={} admission_write_authorized={} auto_apply={} memory_store_write_allowed={} ndkv_write_allowed={}",
        self_improve_proposal_memory_admission_commit_record_stage.target_count,
        self_improve_proposal_memory_admission_commit_record_stage.request_count,
        self_improve_proposal_memory_admission_commit_record_stage.receipt_item_count,
        self_improve_proposal_memory_admission_commit_record_stage.commit_record_item_count,
        self_improve_proposal_memory_admission_commit_record_stage.staged_commit_record_count,
        self_improve_proposal_memory_admission_commit_record_stage.blocked_count,
        first_commit_record_stage_item,
        self_improve_proposal_memory_admission_commit_record_stage.commit_record_stage_ready,
        self_improve_proposal_memory_admission_commit_record_stage
            .explicit_writer_invocation_required,
        self_improve_proposal_memory_admission_commit_record_stage.validation_required,
        self_improve_proposal_memory_admission_commit_record_stage.rollback_required,
        self_improve_proposal_memory_admission_commit_record_stage.commit_allowed,
        self_improve_proposal_memory_admission_commit_record_stage.admission_write_authorized,
        self_improve_proposal_memory_admission_commit_record_stage.auto_apply,
        self_improve_proposal_memory_admission_commit_record_stage.memory_store_write_allowed,
        self_improve_proposal_memory_admission_commit_record_stage.ndkv_write_allowed
    );
    let self_improve_proposal_memory_admission_commit_approval_request =
        self_improve_proposal_artifact.memory_admission_commit_approval_request();
    let first_commit_approval_request_item =
        self_improve_proposal_memory_admission_commit_approval_request
            .first_approval_request_item_id
            .as_deref()
            .unwrap_or("-");
    println!(
        "self_improve_proposal_memory_admission_commit_approval_request_report_v1: targets={} requests={} commit_record_items={} approval_request_items={} requested={} blocked={} first_item={} commit_approval_request_ready={} explicit_commit_approval_required={} validation_required={} rollback_required={} commit_allowed={} admission_write_authorized={} auto_apply={} memory_store_write_allowed={} ndkv_write_allowed={}",
        self_improve_proposal_memory_admission_commit_approval_request.target_count,
        self_improve_proposal_memory_admission_commit_approval_request.request_count,
        self_improve_proposal_memory_admission_commit_approval_request.commit_record_item_count,
        self_improve_proposal_memory_admission_commit_approval_request.approval_request_item_count,
        self_improve_proposal_memory_admission_commit_approval_request
            .requested_commit_approval_count,
        self_improve_proposal_memory_admission_commit_approval_request.blocked_count,
        first_commit_approval_request_item,
        self_improve_proposal_memory_admission_commit_approval_request
            .commit_approval_request_ready,
        self_improve_proposal_memory_admission_commit_approval_request
            .explicit_commit_approval_required,
        self_improve_proposal_memory_admission_commit_approval_request.validation_required,
        self_improve_proposal_memory_admission_commit_approval_request.rollback_required,
        self_improve_proposal_memory_admission_commit_approval_request.commit_allowed,
        self_improve_proposal_memory_admission_commit_approval_request.admission_write_authorized,
        self_improve_proposal_memory_admission_commit_approval_request.auto_apply,
        self_improve_proposal_memory_admission_commit_approval_request.memory_store_write_allowed,
        self_improve_proposal_memory_admission_commit_approval_request.ndkv_write_allowed
    );
    let self_improve_proposal_memory_admission_commit_approval_decision =
        self_improve_proposal_artifact.memory_admission_commit_approval_decision();
    let first_commit_approval_decision_item =
        self_improve_proposal_memory_admission_commit_approval_decision
            .first_approval_decision_item_id
            .as_deref()
            .unwrap_or("-");
    println!(
        "self_improve_proposal_memory_admission_commit_approval_decision_report_v1: targets={} requests={} approval_request_items={} approval_decision_items={} recorded={} approved={} pending={} blocked={} first_item={} commit_approval_decision_ready={} explicit_commit_approval_required={} validation_required={} rollback_required={} commit_allowed={} admission_write_authorized={} auto_apply={} memory_store_write_allowed={} ndkv_write_allowed={}",
        self_improve_proposal_memory_admission_commit_approval_decision.target_count,
        self_improve_proposal_memory_admission_commit_approval_decision.request_count,
        self_improve_proposal_memory_admission_commit_approval_decision.approval_request_item_count,
        self_improve_proposal_memory_admission_commit_approval_decision
            .approval_decision_item_count,
        self_improve_proposal_memory_admission_commit_approval_decision
            .recorded_approval_decision_count,
        self_improve_proposal_memory_admission_commit_approval_decision.approved_commit_count,
        self_improve_proposal_memory_admission_commit_approval_decision.pending_approval_count,
        self_improve_proposal_memory_admission_commit_approval_decision.blocked_count,
        first_commit_approval_decision_item,
        self_improve_proposal_memory_admission_commit_approval_decision
            .commit_approval_decision_ready,
        self_improve_proposal_memory_admission_commit_approval_decision
            .explicit_commit_approval_required,
        self_improve_proposal_memory_admission_commit_approval_decision.validation_required,
        self_improve_proposal_memory_admission_commit_approval_decision.rollback_required,
        self_improve_proposal_memory_admission_commit_approval_decision.commit_allowed,
        self_improve_proposal_memory_admission_commit_approval_decision.admission_write_authorized,
        self_improve_proposal_memory_admission_commit_approval_decision.auto_apply,
        self_improve_proposal_memory_admission_commit_approval_decision.memory_store_write_allowed,
        self_improve_proposal_memory_admission_commit_approval_decision.ndkv_write_allowed
    );
    let self_improve_proposal_memory_admission_commit_approval_review_packet =
        self_improve_proposal_artifact.memory_admission_commit_approval_review_packet();
    let first_commit_approval_review_packet_item =
        self_improve_proposal_memory_admission_commit_approval_review_packet
            .first_review_packet_item_id
            .as_deref()
            .unwrap_or("-");
    println!(
        "self_improve_proposal_memory_admission_commit_approval_review_packet_report_v1: targets={} requests={} approval_decision_items={} review_packet_items={} ready={} pending={} blocked={} first_item={} approval_review_packet_ready={} explicit_operator_approval_required={} validation_required={} rollback_required={} commit_allowed={} admission_write_authorized={} auto_apply={} memory_store_write_allowed={} ndkv_write_allowed={}",
        self_improve_proposal_memory_admission_commit_approval_review_packet.target_count,
        self_improve_proposal_memory_admission_commit_approval_review_packet.request_count,
        self_improve_proposal_memory_admission_commit_approval_review_packet
            .approval_decision_item_count,
        self_improve_proposal_memory_admission_commit_approval_review_packet
            .review_packet_item_count,
        self_improve_proposal_memory_admission_commit_approval_review_packet
            .ready_review_packet_count,
        self_improve_proposal_memory_admission_commit_approval_review_packet.pending_approval_count,
        self_improve_proposal_memory_admission_commit_approval_review_packet.blocked_count,
        first_commit_approval_review_packet_item,
        self_improve_proposal_memory_admission_commit_approval_review_packet
            .approval_review_packet_ready,
        self_improve_proposal_memory_admission_commit_approval_review_packet
            .explicit_operator_approval_required,
        self_improve_proposal_memory_admission_commit_approval_review_packet.validation_required,
        self_improve_proposal_memory_admission_commit_approval_review_packet.rollback_required,
        self_improve_proposal_memory_admission_commit_approval_review_packet.commit_allowed,
        self_improve_proposal_memory_admission_commit_approval_review_packet
            .admission_write_authorized,
        self_improve_proposal_memory_admission_commit_approval_review_packet.auto_apply,
        self_improve_proposal_memory_admission_commit_approval_review_packet
            .memory_store_write_allowed,
        self_improve_proposal_memory_admission_commit_approval_review_packet.ndkv_write_allowed
    );
    if let Some(last) = &summary.last {
        println!(
            "last: round={} case={} success={} runtime_tokens={} feedback_applied={} round_wall_elapsed_ms={}",
            option_u64_text(last.round),
            last.case_name.as_deref().unwrap_or("?"),
            last.success,
            option_u64_text(last.runtime_tokens),
            option_u64_text(last.feedback_applied),
            option_u64_text(last.round_wall_elapsed_ms())
        );
        if let Some(source) = last.validation_command_source.as_deref() {
            println!(
                "last_validation_command: source={} safety={} command={}",
                source,
                last.validation_command_safety
                    .as_deref()
                    .unwrap_or("unknown"),
                last.validation_command_preview.as_deref().unwrap_or("none")
            );
        }
        if let Some(phase) = last.validation_phase.as_deref() {
            println!(
                "last_validation_result: phase={} status={} elapsed_ms={} stdout_tail={} stderr_tail={}",
                phase,
                option_i32_text(last.validation_status_code),
                option_u64_text(last.validation_elapsed_ms),
                last.validation_stdout_tail.as_deref().unwrap_or("-"),
                last.validation_stderr_tail.as_deref().unwrap_or("-")
            );
        }
    }
    if !summary.recent_failures.is_empty() {
        println!("recent_failures:");
        for failure in &summary.recent_failures {
            println!(
                "  round={} case={} error={}",
                option_u64_text(failure.round),
                failure.case_name.as_deref().unwrap_or("?"),
                failure.error.as_deref().unwrap_or("final gate failed")
            );
        }
    }
}

fn report_gate_failures(
    summary: &ReportSummary,
    config: &Config,
    pool_status: Option<&PoolStatusSummary>,
    pool_budget_fairness: Option<&PoolBudgetFairnessSummary>,
) -> Vec<String> {
    let mut failures = report_gate_threshold_failures(summary, config);
    if config.require_round_wall_clock_evidence {
        failures.extend(round_wall_clock_evidence_failures(summary));
    }
    failures.extend(report_gate_operational_failures(
        summary,
        config,
        pool_status,
        pool_budget_fairness,
        true,
    ));
    failures
}

fn report_gate_continuation_failures(
    summary: &ReportSummary,
    config: &Config,
    pool_status: Option<&PoolStatusSummary>,
    pool_budget_fairness: Option<&PoolBudgetFairnessSummary>,
) -> Vec<String> {
    let mut failures = latest_round_continuation_failures(summary, config);
    failures.extend(report_gate_operational_failures(
        summary,
        config,
        pool_status,
        pool_budget_fairness,
        false,
    ));
    failures
}

fn latest_round_continuation_failures(summary: &ReportSummary, config: &Config) -> Vec<String> {
    let Some(last) = summary.last.as_ref() else {
        return vec!["ledger has no rounds".to_owned()];
    };

    let mut failures = Vec::new();
    let round = option_u64_text(last.round);
    let case = last.case_name.as_deref().unwrap_or("?");
    if config.require_last_success && !last.success {
        failures.push(format!("latest round failed: round={round} case={case}"));
    }
    if let Some(minimum) = config.min_feedback_total.filter(|minimum| *minimum > 0) {
        let actual = last.feedback_applied.unwrap_or(0);
        if actual < minimum {
            failures.push(format!(
                "latest feedback_applied {actual} below minimum {minimum}"
            ));
        }
    }
    if let Some(minimum) = config
        .min_rust_checks
        .filter(|minimum| *minimum > 0)
        .map(|minimum| minimum.min(1))
    {
        let checked = usize::from(
            last.rust_check_passed.is_some() && last.rust_check_checked.unwrap_or(true),
        );
        if checked < minimum {
            failures.push(format!(
                "latest rust_check checked {checked} below minimum {minimum}"
            ));
        }
    }
    if let Some(minimum) = config
        .min_rust_feedback_total
        .filter(|minimum| *minimum > 0)
        .map(|minimum| minimum.min(1))
    {
        let actual = last.rust_check_feedback_applied.unwrap_or(0);
        if actual < minimum {
            failures.push(format!(
                "latest rust_check_feedback_applied {actual} below minimum {minimum}"
            ));
        }
    }
    if last.has_stream_truncation_error() {
        failures.push(format!(
            "latest round has stream truncation failure: round={round} case={case}"
        ));
    }
    if last.has_missing_final_error() {
        failures.push(format!(
            "latest round has missing final-event failure: round={round} case={case}"
        ));
    }
    if last.has_runtime_response_failure() {
        failures.push(format!(
            "latest round has runtime response failure: round={round} case={case}"
        ));
    }
    if config.require_round_wall_clock_evidence && !last.has_round_wall_clock_evidence() {
        failures.push(format!(
            "latest round wall-clock evidence missing: round={round} case={case}"
        ));
    }
    failures
}

fn report_gate_operational_failures(
    summary: &ReportSummary,
    config: &Config,
    pool_status: Option<&PoolStatusSummary>,
    pool_budget_fairness: Option<&PoolBudgetFairnessSummary>,
    block_budget_fairness_share: bool,
) -> Vec<String> {
    let mut failures = Vec::new();
    if !config.required_helper_stage_roles.is_empty() {
        let missing_roles = missing_helper_stage_roles(
            &config.required_helper_stage_roles,
            &summary.helper_stage_feedback_by_role,
        );
        if !missing_roles.is_empty() {
            failures.push(format!(
                "helper stage feedback missing required roles: {}",
                missing_roles.join(",")
            ));
        }
    }
    if !config.required_latest_helper_stage_roles.is_empty() {
        let latest_feedback_by_role = summary
            .last
            .as_ref()
            .map(ReportRecord::helper_stage_feedback_by_role)
            .unwrap_or_default();
        let latest_contract_fields_by_role = summary
            .last
            .as_ref()
            .map(ReportRecord::helper_stage_contract_fields_by_role)
            .unwrap_or_default();
        let missing_roles = missing_helper_stage_roles(
            &config.required_latest_helper_stage_roles,
            &latest_feedback_by_role,
        );
        if !missing_roles.is_empty() {
            failures.push(format!(
                "latest round helper stage feedback missing required roles: {}",
                missing_roles.join(",")
            ));
        }
        if config.require_useful_latest_helper_stage_feedback {
            failures.extend(usefulness_failures_for_helper_stage_roles(
                &config.required_latest_helper_stage_roles,
                &latest_feedback_by_role,
            ));
        }
        if config.require_complete_latest_helper_stage_feedback {
            failures.extend(completeness_failures_for_helper_stage_roles(
                &config.required_latest_helper_stage_roles,
                &latest_feedback_by_role,
                &latest_contract_fields_by_role,
            ));
        }
    }
    if config.require_clean_helper_stage_feedback {
        failures.extend(helper_stage_hygiene_failures(
            &summary.helper_stage_hygiene_by_role,
        ));
    }
    failures.extend(validation_command_coverage_guard_failures(summary));
    if config.require_final_json_pool_stage_dispatch {
        failures.extend(final_json_pool_stage_dispatch_failures(
            summary.last.as_ref(),
            &config.required_latest_helper_stage_roles,
        ));
    }
    if config.require_test_gate_pass {
        match latest_test_gate_verdict(summary) {
            Some("pass") => {}
            Some(verdict) => failures.push(format!(
                "latest test-gate helper verdict is {verdict}, expected pass"
            )),
            None => failures
                .push("test-gate helper verdict missing; expected latest verdict pass".to_owned()),
        }
    }
    if config.require_safe_test_gate_validation_command
        && summary.test_gate.latest_validation_command_safety != "safe"
    {
        failures.push(format!(
            "latest test-gate validation_command is {}, expected safe cargo validation command",
            summary.test_gate.latest_validation_command_safety
        ));
    }
    if config.require_test_gate_validation_run
        && let Some(failure) = test_gate_validation_run_failure(summary.last.as_ref())
    {
        failures.push(failure);
    }
    if config.require_configured_validation_run
        && let Some(failure) = configured_validation_run_failure(summary.last.as_ref())
    {
        failures.push(failure);
    }
    if config.remote_chain_gate {
        match remote_chain::load_status(config.remote_chain_status_json_path.as_deref()) {
            Ok(Some(summary)) => {
                if let Some(failure) = remote_chain::gate_failure(&summary) {
                    failures.push(format!("remote chain not ready: {failure}"));
                }
            }
            Ok(None) => {
                failures.push("remote chain gate requested but status is missing".to_owned())
            }
            Err(error) => failures.push(format!("remote chain status unreadable: {error}")),
        }
    }
    if config.pool_capacity_gate {
        match pool_status {
            Some(pool_status) => {
                if let Some(failure) = pool_artifacts::capacity_gate_failure(pool_status) {
                    failures.push(format!("model pool capacity blocked expansion: {failure}"));
                }
            }
            None => failures
                .push("model pool capacity gate requested but pool status is missing".to_owned()),
        }
    }
    if let Some(failure) = report_gate_pool_alignment_failure(config, pool_status) {
        failures.push(failure);
    }
    if let Some(pool_budget_fairness) = pool_budget_fairness
        && pool_budget_fairness.budget_fairness_blocked
        && block_budget_fairness_share
    {
        let reasons = if pool_budget_fairness.failure_reasons.is_empty() {
            "unknown".to_owned()
        } else {
            pool_budget_fairness.failure_reasons.join("; ")
        };
        failures.push(format!(
            "model pool budget fairness blocked expansion: {reasons}"
        ));
    }
    if config.require_pool_budget_policy
        && let Some(failure) = pool_artifacts::budget_policy_gate_failure(pool_budget_fairness)
    {
        failures.push(failure);
    }
    failures
}

fn validation_command_coverage_guard_failures(summary: &ReportSummary) -> Vec<String> {
    let evidence = &summary.validation_command_coverage_evidence;
    if validation_command_coverage_is_blocked(evidence) {
        vec![
            "validation command coverage guard blocked report gate: evolution-loop.strict-coverage requires coverage tooling/report evidence before --strict-coverage, coverage report, or coverage gate work"
                .to_owned(),
        ]
    } else {
        Vec::new()
    }
}

fn validation_command_coverage_is_blocked(evidence: &ValidationCommandCoverageEvidence) -> bool {
    evidence.strict_coverage_is_requested()
        && !evidence.coverage_tooling_or_report_evidence_present()
}

fn final_json_pool_stage_dispatch_failures(
    latest: Option<&ReportRecord>,
    required_roles: &[String],
) -> Vec<String> {
    let Some(record) = latest else {
        return vec![
            "latest final_json.pool_stage_dispatch missing: ledger has no rounds".to_owned(),
        ];
    };
    let round = option_u64_text(record.round);
    let case = record.case_name.as_deref().unwrap_or("?");
    let required = required_roles
        .iter()
        .map(String::as_str)
        .filter(|role| !role.trim().is_empty())
        .collect::<Vec<_>>();
    if required.is_empty() {
        return vec![
            "latest final_json.pool_stage_dispatch check has no required roles; set --require-latest-helper-stage-roles"
                .to_owned(),
        ];
    }
    let task_kinds = final_json_pool_stage_dispatch_task_kinds(record);
    if task_kinds.is_empty() {
        return vec![format!(
            "latest final_json.pool_stage_dispatch missing: round={round} case={case} field absent"
        )];
    };
    let missing = required
        .iter()
        .filter(|role| !task_kinds.contains(**role))
        .map(|role| (*role).to_owned())
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Vec::new()
    } else {
        vec![format!(
            "latest final_json.pool_stage_dispatch missing required task_kinds: {} round={round} case={case}",
            missing.join(",")
        )]
    }
}

fn final_json_pool_stage_dispatch_task_kinds(record: &ReportRecord) -> BTreeSet<String> {
    if let Some(dispatch_json) = record.final_json_pool_stage_dispatch.as_deref() {
        let task_kinds = pool_stage_dispatch_task_kinds_from_json(dispatch_json);
        if !task_kinds.is_empty() {
            return task_kinds;
        }
    }
    if let Some(final_json) = record.final_json.as_deref()
        && let Some(dispatch_json) = json_array_field(final_json, "pool_stage_dispatch")
    {
        let task_kinds = pool_stage_dispatch_task_kinds_from_json(&dispatch_json);
        if !task_kinds.is_empty() {
            return task_kinds;
        }
    }
    pool_stage_dispatch_task_kinds_from_meta(&record.meta)
}

fn pool_stage_dispatch_task_kinds_from_json(dispatch_json: &str) -> BTreeSet<String> {
    parse_json_object_array(dispatch_json)
        .into_iter()
        .filter_map(|object| json_string_field(&object, "task_kind"))
        .collect()
}

fn pool_stage_dispatch_task_kinds_from_meta(meta: &[String]) -> BTreeSet<String> {
    meta.iter()
        .filter(|item| item.starts_with("pool_stage_dispatch "))
        .filter_map(|item| whitespace_value(item, "task_kind="))
        .collect()
}

fn whitespace_value(item: &str, key: &str) -> Option<String> {
    item.split_whitespace()
        .find_map(|part| part.strip_prefix(key))
        .map(|value| {
            value
                .trim_matches(|character: char| matches!(character, ',' | ';' | '"' | '\''))
                .to_owned()
        })
        .filter(|value| !value.is_empty())
}

fn report_gate_pool_alignment_failure(
    config: &Config,
    pool_status: Option<&PoolStatusSummary>,
) -> Option<String> {
    if !config.pool_alignment_gate {
        return None;
    }
    match load_pool_alignment_for_report_gate(config, pool_status) {
        Ok(alignment) => pool_artifacts::alignment_gate_failure(&alignment).map(|failure| {
            format!(
                "model pool alignment failed: {failure}; {}",
                pool_artifacts::alignment_context_text(&alignment)
            )
        }),
        Err(error) => Some(format!("model pool alignment unavailable: {error}")),
    }
}

fn load_pool_alignment_for_report_gate(
    config: &Config,
    pool_status: Option<&PoolStatusSummary>,
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

    let status = if let Some(pool_status) = pool_status {
        pool_status.clone()
    } else {
        let status_path = config
            .pool_status_json_path
            .as_deref()
            .ok_or_else(|| "--pool-alignment-gate requires --pool-status-json".to_owned())?;
        pool_artifacts::load_status(Some(status_path))?.ok_or_else(|| {
            format!(
                "pool alignment gate failed: status artifact is empty ({})",
                status_path.display()
            )
        })?
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

fn report_gate_threshold_failures(summary: &ReportSummary, config: &Config) -> Vec<String> {
    let gate = eval_report_gate_from_config(config);
    let eval_summary = eval_ledger_summary(summary);
    let breakdown = gate.evaluate_breakdown(&eval_summary);
    let mut failures = Vec::new();

    if let Some(breach) = breakdown.rounds {
        failures.push(format!(
            "rounds {} below minimum {}",
            breach.actual, breach.minimum
        ));
    }
    if let Some(breach) = breakdown.success_rate {
        failures.push(format!(
            "success rate {:.1}% below minimum {:.1}%",
            breach.actual * 100.0,
            breach.minimum * 100.0
        ));
    }
    if let Some(breach) = breakdown.feedback_applied {
        failures.push(format!(
            "feedback_applied {} below minimum {}",
            breach.actual, breach.minimum
        ));
    }
    if let Some(breach) = breakdown.rust_checks {
        failures.push(format!(
            "rust_check checked {} below minimum {}",
            breach.actual, breach.minimum
        ));
    }
    if let Some(breach) = breakdown.rust_check_feedback_applied {
        failures.push(format!(
            "rust_check_feedback_applied {} below minimum {}",
            breach.actual, breach.minimum
        ));
    }
    if let Some(breach) = breakdown.stream_truncations {
        failures.push(format!(
            "stream truncation failures {} above maximum {}",
            breach.actual, breach.maximum
        ));
    }
    if let Some(breach) = breakdown.missing_final_failures {
        failures.push(format!(
            "missing final-event failures {} above maximum {}",
            breach.actual, breach.maximum
        ));
    }
    if let Some(breach) = breakdown.runtime_response_failures {
        failures.push(format!(
            "runtime response failures {} above maximum {}",
            breach.actual, breach.maximum
        ));
    }
    if let Some(count) = breakdown.duplicate_rounds {
        failures.push(format!("ledger has {count} duplicate round record(s)"));
    }
    if let Some(count) = breakdown.non_monotonic_rounds {
        failures.push(format!("ledger has {count} non-monotonic round record(s)"));
    }
    if let Some(count) = breakdown.missing_rounds {
        failures.push(format!("ledger has {count} record(s) without round"));
    }
    if let Some(count) = breakdown.round_gaps {
        failures.push(format!("ledger has {count} missing round number(s)"));
    }
    if breakdown.latest_round_failed {
        match &summary.last {
            Some(last) => failures.push(format!(
                "latest round failed: round={} case={}",
                option_u64_text(last.round),
                last.case_name.as_deref().unwrap_or("?")
            )),
            None => failures.push("ledger has no rounds".to_owned()),
        }
    }

    failures
}

fn round_wall_clock_evidence_failures(summary: &ReportSummary) -> Vec<String> {
    let missing = summary
        .total
        .saturating_sub(summary.round_wall_elapsed_items);
    if missing == 0 {
        return Vec::new();
    }

    let mut examples = Vec::new();
    if let Some(last) = summary.last.as_ref()
        && !last.has_round_wall_clock_evidence()
    {
        examples.push(format!(
            "latest round={} case={}",
            option_u64_text(last.round),
            last.case_name.as_deref().unwrap_or("?")
        ));
    }
    let detail = if examples.is_empty() {
        "expected started_unix <= finished_unix on every record".to_owned()
    } else {
        format!(
            "expected started_unix <= finished_unix on every record; {}",
            examples.join("; ")
        )
    };

    vec![format!(
        "round wall-clock evidence missing for {missing} of {} record(s): {detail}",
        summary.total
    )]
}

fn eval_report_gate_from_config(config: &Config) -> ReportGate {
    ReportGate {
        min_rounds: config.min_report_rounds,
        min_success_rate: config
            .min_success_rate
            .map(|rate| f64::from(rate) / 100.0)
            .unwrap_or(0.0),
        min_validation_pass_rate: 0.0,
        min_self_improve_pass_rate: 0.0,
        min_feedback_total: config.min_feedback_total,
        min_rust_checks: config.min_rust_checks,
        min_rust_feedback_total: config.min_rust_feedback_total,
        max_stream_truncations: config.max_stream_truncations,
        max_missing_final_failures: config.max_missing_final,
        max_runtime_response_failures: config.max_runtime_response_failures,
        max_state_gate_failures: usize::MAX,
        max_trace_gate_failures: usize::MAX,
        max_context_noise_penalty: f64::INFINITY,
        require_strict_ledger_hygiene: config.strict_ledger_hygiene,
        require_last_success: config.require_last_success,
    }
}

fn eval_ledger_summary(summary: &ReportSummary) -> LedgerSummary {
    LedgerSummary {
        total_rounds: summary.total,
        unique_rounds: summary.unique_rounds,
        duplicate_rounds: summary.duplicate_rounds,
        non_monotonic_rounds: summary.non_monotonic_rounds,
        missing_rounds: summary.missing_rounds,
        round_gaps: summary.round_gaps,
        max_round: summary
            .max_round
            .and_then(|round| u64::try_from(round).ok()),
        successful_rounds: summary.success,
        failed_rounds: summary.failure,
        runtime_tokens_total: summary.runtime_tokens,
        runtime_token_items: summary.runtime_token_items,
        elapsed_ms_total: summary.elapsed_ms,
        elapsed_ms_items: summary.elapsed_items,
        feedback_applied_total: summary.feedback_applied,
        feedback_items: summary.feedback_items,
        rust_check_checked: summary.rust_check_checked,
        rust_check_passed: summary.rust_check_passed,
        rust_check_feedback_applied_total: summary.rust_check_feedback_applied,
        rust_check_feedback_items: summary.rust_check_feedback_items,
        validation_checked: summary.validation_checked,
        validation_passed: summary.validation_passed,
        self_improve_checked: summary.self_improve_checked,
        self_improve_passed: summary.self_improve_passed,
        state_gate_checked: summary.state_gate_checked,
        state_gate_passed: summary.state_gate_passed,
        trace_gate_checked: summary.trace_gate_checked,
        trace_gate_passed: summary.trace_gate_passed,
        runtime_response_failures: summary.runtime_response_failures,
        stream_truncations: summary.stream_truncation_failures,
        missing_final_failures: summary.missing_final_failures,
        missing_runtime_models: 0,
        zero_runtime_tokens: 0,
        context_noise_penalty_total: 0.0,
        context_noise_penalty_max: 0.0,
        last_success: summary.last.as_ref().map(|record| record.success),
    }
}

fn ledger_gate_report_json(summary: &ReportSummary, ledger_gate_failures: &[String]) -> String {
    let eval_summary = eval_ledger_summary(summary);
    let report =
        LedgerGateReport::from_summary_and_failure_reasons(&eval_summary, ledger_gate_failures);

    format!(
        "{{\"schema\":\"ledger_gate_report_v1\",\"total_rounds\":{},\"success_rate\":{:.3},\"validation_pass_rate\":{:.3},\"rust_check_checked\":{},\"rust_check_passed\":{},\"rust_check_feedback_applied_total\":{},\"runtime_response_failures\":{},\"stream_truncations\":{},\"missing_final_failures\":{},\"duplicate_rounds\":{},\"round_gaps\":{},\"state_gate_pass_rate\":{:.3},\"trace_gate_pass_rate\":{:.3},\"context_noise_penalty_max\":{:.3},\"last_success\":{},\"gate_blocked\":{},\"failure_reasons\":{},\"allow_next_round\":{}}}",
        report.total_rounds,
        report.success_rate,
        report.validation_pass_rate,
        report.rust_check_checked,
        report.rust_check_passed,
        report.rust_check_feedback_applied_total,
        report.runtime_response_failures,
        report.stream_truncations,
        report.missing_final_failures,
        report.duplicate_rounds,
        report.round_gaps,
        report.state_gate_pass_rate,
        report.trace_gate_pass_rate,
        report.context_noise_penalty_max,
        option_bool_json(report.last_success),
        report.gate_blocked,
        string_array_json(&report.failure_reasons),
        report.allow_next_round
    )
}

fn strict_report_gate_json(strict_gate_failures: &[String]) -> String {
    format!(
        "{{\"passed\":{},\"failures\":{}}}",
        strict_gate_failures.is_empty(),
        string_array_json(strict_gate_failures)
    )
}

fn continuation_gate_report_json(
    summary: &ReportSummary,
    pool_budget_fairness: Option<&PoolBudgetFairnessSummary>,
    strict_gate_failures: &[String],
    continuation_gate_failures: &[String],
) -> String {
    let budget_fairness_blocked =
        pool_budget_fairness.map(|summary| summary.budget_fairness_blocked);
    let budget_fairness_reasons = pool_budget_fairness
        .map(|summary| summary.failure_reasons.as_slice())
        .unwrap_or(&[]);

    format!(
        "{{\"schema\":\"continuation_gate_report_v1\",\"allow_unattended_continuation\":{},\"gate_blocked\":{},\"failure_reasons\":{},\"strict_report_gate_passed\":{},\"strict_failure_reasons\":{},\"historical_failures\":{},\"historical_runtime_response_failures\":{},\"latest_round\":{},\"latest_success\":{},\"latest_runtime_response_failure\":{},\"budget_fairness_blocked\":{},\"budget_fairness_advisory_reasons\":{}}}",
        continuation_gate_failures.is_empty(),
        !continuation_gate_failures.is_empty(),
        string_array_json(continuation_gate_failures),
        strict_gate_failures.is_empty(),
        string_array_json(strict_gate_failures),
        summary.failure,
        summary.runtime_response_failures,
        option_u64_json(summary.last.as_ref().and_then(|record| record.round)),
        option_bool_json(summary.last.as_ref().map(|record| record.success)),
        option_bool_json(
            summary
                .last
                .as_ref()
                .map(ReportRecord::has_runtime_response_failure)
        ),
        option_bool_json(budget_fairness_blocked),
        string_array_json(budget_fairness_reasons)
    )
}

fn adapter_closure_bundle_report_json(
    summary: &ReportSummary,
    ledger_gate_failures: &[String],
    strict_gate_failures: &[String],
    continuation_gate_failures: &[String],
    gate_failures: &[String],
) -> String {
    let source_report_keys = vec![
        "ledger_gate_report_v1".to_owned(),
        "strict_report_gate".to_owned(),
        "continuation_gate_report_v1".to_owned(),
        "validation_command_coverage_report_v1".to_owned(),
        "report_gate".to_owned(),
    ];
    let helper_feedback_roles = summary
        .helper_stage_feedback_by_role
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    let helper_stage_contract_roles = summary
        .helper_stage_contract_by_role
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    let coverage_evidence = &summary.validation_command_coverage_evidence;

    format!(
        "{{\"schema\":\"adapter_closure_bundle_report_v1\",\"consumer_surface\":\"adapter_closure_unattended_continuation\",\"pure_data_bundle\":true,\"source_report_keys\":{},\"consumer_decision\":{{\"report_gate_passed\":{},\"report_gate_failure_count\":{},\"ledger_gate_allow_next_round\":{},\"strict_report_gate_passed\":{},\"allow_unattended_continuation\":{},\"continuation_gate_blocked\":{},\"latest_round\":{},\"latest_success\":{},\"latest_runtime_response_failure\":{}}},\"closure_evidence\":{{\"rounds\":{},\"success\":{},\"failures\":{},\"feedback_applied_total\":{},\"rust_check_checked\":{},\"rust_check_passed\":{},\"validation_checked\":{},\"validation_passed\":{},\"self_improve_checked\":{},\"self_improve_passed\":{},\"state_gate_checked\":{},\"state_gate_passed\":{},\"trace_gate_checked\":{},\"trace_gate_passed\":{},\"runtime_response_failures\":{},\"stream_truncations\":{},\"missing_final_failures\":{}}},\"validation_command_coverage\":{{\"strict_coverage_requested\":{},\"coverage_tooling_evidence_count\":{},\"coverage_report_evidence_count\":{},\"coverage_tooling_or_report_evidence_present\":{}}},\"adapter_surfaces\":{{\"helper_feedback_roles\":{},\"helper_stage_contract_roles\":{},\"test_gate_latest_verdict\":{},\"test_gate_latest_validation_command_safety\":{}}}}}",
        string_array_json(&source_report_keys),
        gate_failures.is_empty(),
        gate_failures.len(),
        ledger_gate_failures.is_empty(),
        strict_gate_failures.is_empty(),
        continuation_gate_failures.is_empty(),
        !continuation_gate_failures.is_empty(),
        option_u64_json(summary.last.as_ref().and_then(|record| record.round)),
        option_bool_json(summary.last.as_ref().map(|record| record.success)),
        option_bool_json(
            summary
                .last
                .as_ref()
                .map(ReportRecord::has_runtime_response_failure)
        ),
        summary.total,
        summary.success,
        summary.failure,
        summary.feedback_applied,
        summary.rust_check_checked,
        summary.rust_check_passed,
        summary.validation_checked,
        summary.validation_passed,
        summary.self_improve_checked,
        summary.self_improve_passed,
        summary.state_gate_checked,
        summary.state_gate_passed,
        summary.trace_gate_checked,
        summary.trace_gate_passed,
        summary.runtime_response_failures,
        summary.stream_truncation_failures,
        summary.missing_final_failures,
        coverage_evidence.strict_coverage_requested,
        coverage_evidence.coverage_tooling_evidence.len(),
        coverage_evidence.coverage_report_evidence.len(),
        coverage_evidence.coverage_tooling_or_report_evidence_present(),
        string_array_json(&helper_feedback_roles),
        string_array_json(&helper_stage_contract_roles),
        option_str_json(summary.test_gate.latest_verdict.as_deref()),
        json_string(&summary.test_gate.latest_validation_command_safety)
    )
}

fn helper_stage_repair_status(
    summary: &ReportSummary,
    required_latest_helper_stage_roles: &[String],
) -> HelperStageRepairStatus {
    let latest_round = summary.last.as_ref().and_then(|record| record.round);
    let fields_by_role = summary
        .last
        .as_ref()
        .map(latest_helper_stage_contract_fields_by_role)
        .unwrap_or_default();
    helper_stage_repair::from_latest_contract_fields_with_required_roles(
        latest_round,
        fields_by_role,
        required_latest_helper_stage_roles,
    )
}

fn latest_helper_stage_contract_fields_by_role(
    record: &ReportRecord,
) -> BTreeMap<String, BTreeMap<String, String>> {
    let structured_fields_by_role = record.helper_stage_contract_fields_by_role();
    let feedback_by_role = record.helper_stage_feedback_by_role();
    let roles = structured_fields_by_role
        .keys()
        .chain(feedback_by_role.keys())
        .cloned()
        .collect::<BTreeSet<_>>();

    roles
        .into_iter()
        .map(|role| {
            let mut fields = feedback_by_role
                .get(role.as_str())
                .map(|feedback| helper_feedback::contract_fields(&role, feedback))
                .unwrap_or_default();
            if let Some(structured_fields) = structured_fields_by_role.get(role.as_str()) {
                fields.extend(structured_fields.clone());
            }
            (role, fields)
        })
        .collect()
}

fn missing_helper_stage_roles(
    required_roles: &[String],
    feedback_by_role: &BTreeMap<String, Vec<String>>,
) -> Vec<String> {
    required_roles
        .iter()
        .filter(|role| {
            feedback_by_role
                .get(role.as_str())
                .map_or(true, Vec::is_empty)
        })
        .cloned()
        .collect()
}

fn helper_stage_hygiene_failures(
    hygiene_by_role: &BTreeMap<String, Vec<HelperStageHygieneFinding>>,
) -> Vec<String> {
    hygiene_by_role
        .values()
        .flat_map(|findings| findings.iter())
        .map(|finding| {
            format!(
                "helper stage feedback hygiene violation role={} kind={} preview={}",
                finding.role, finding.kind, finding.preview
            )
        })
        .collect()
}

fn usefulness_failures_for_helper_stage_roles(
    required_roles: &[String],
    feedback_by_role: &BTreeMap<String, Vec<String>>,
) -> Vec<String> {
    required_roles
        .iter()
        .filter_map(|role| {
            feedback_by_role
                .get(role.as_str())
                .and_then(|feedback| helper_stage_feedback_usefulness_failure(role, feedback))
        })
        .collect()
}

fn completeness_failures_for_helper_stage_roles(
    required_roles: &[String],
    feedback_by_role: &BTreeMap<String, Vec<String>>,
    contract_fields_by_role: &BTreeMap<String, BTreeMap<String, String>>,
) -> Vec<String> {
    required_roles
        .iter()
        .filter_map(|role| {
            let fields = contract_fields_by_role
                .get(role.as_str())
                .cloned()
                .or_else(|| {
                    feedback_by_role
                        .get(role.as_str())
                        .map(|feedback| helper_feedback::contract_fields(role, feedback))
                })?;
            let missing_fields = missing_complete_helper_stage_fields(role, &fields);
            (!missing_fields.is_empty()).then(|| {
                format!(
                    "latest round helper stage feedback for {role} missing required fields: {}",
                    missing_fields.join(",")
                )
            })
        })
        .collect()
}

fn missing_complete_helper_stage_fields(
    role: &str,
    fields: &BTreeMap<String, String>,
) -> Vec<String> {
    let expected_markers = helper_feedback::contract_markers(role)
        .iter()
        .map(|marker| (*marker).to_owned())
        .collect::<Vec<_>>();
    helper_stage_missing_complete_fields(role, fields, &expected_markers)
}

fn helper_stage_feedback_usefulness_failure(role: &str, feedback: &[String]) -> Option<String> {
    let fields = helper_feedback::contract_fields(role, feedback);
    let placeholder_fields = helper_stage_placeholder_fields(role, &fields);
    if !placeholder_fields.is_empty() {
        return Some(format!(
            "latest round helper stage feedback for {role} contains placeholder fields: {}",
            placeholder_fields.join(",")
        ));
    }
    let previews = feedback
        .iter()
        .map(|item| helper_feedback::feedback_preview(item).to_owned())
        .collect::<Vec<_>>();
    (!helper_stage_contract_is_useful_for_role(
        role,
        &fields,
        &previews,
        MIN_USEFUL_HELPER_STAGE_FEEDBACK_CHARS,
    ))
    .then(|| format!("latest round helper stage feedback for {role} is not actionable"))
}

fn helper_stage_contract_summaries_by_role(
    feedback_by_role: &BTreeMap<String, Vec<String>>,
) -> BTreeMap<String, HelperStageContractSummary> {
    feedback_by_role
        .iter()
        .map(|(role, feedback)| {
            let previews = feedback
                .iter()
                .map(|item| helper_feedback::feedback_preview(item).to_owned())
                .collect::<Vec<_>>();
            let fields = helper_feedback::contract_fields(role, feedback);
            let mut summary = HelperStageContractSummary::from_parts(
                &previews,
                helper_feedback::latest_feedback_preview(feedback),
                fields,
                helper_feedback::matched_contract_markers(role, feedback),
                helper_feedback::contract_markers(role)
                    .iter()
                    .map(|marker| (*marker).to_owned())
                    .collect(),
            );
            summary.useful = helper_stage_contract_is_useful_for_role(
                role,
                &summary.fields,
                &previews,
                MIN_USEFUL_HELPER_STAGE_FEEDBACK_CHARS,
            );
            (role.clone(), summary)
        })
        .collect()
}

fn recent_helper_stage_contract_summaries_by_role(
    records: &[ReportRecord],
    feedback_by_role: &BTreeMap<String, Vec<String>>,
) -> BTreeMap<String, HelperStageContractSummary> {
    let mut summaries = helper_stage_contract_summaries_by_role(feedback_by_role);
    for record in records {
        for (role, fields) in record.helper_stage_contract_fields_by_role() {
            if fields.is_empty() {
                continue;
            }
            let summary = summaries
                .entry(role.clone())
                .or_insert_with(|| empty_helper_stage_contract_summary(&role));
            summary.fields.extend(fields);
            summary.matched_markers = helper_feedback::contract_markers(&role)
                .iter()
                .filter(|marker| summary.fields.contains_key(**marker))
                .map(|marker| (*marker).to_owned())
                .collect();
            summary.useful = !summary.fields.is_empty()
                && helper_stage_placeholder_fields(&role, &summary.fields).is_empty();
        }
    }
    summaries
}

fn empty_helper_stage_contract_summary(role: &str) -> HelperStageContractSummary {
    HelperStageContractSummary::empty(
        helper_feedback::contract_markers(role)
            .iter()
            .map(|marker| (*marker).to_owned())
            .collect(),
    )
}

fn latest_test_gate_verdict(summary: &ReportSummary) -> Option<&str> {
    summary.test_gate.latest_verdict.as_deref()
}

fn test_gate_validation_run_failure(last: Option<&ReportRecord>) -> Option<String> {
    let evidence = validation_run_evidence(last);
    eval_test_gate_validation_run_failure(evidence.as_ref())
        .map(|failure| validation_run_failure_text(failure, "test-gate", "test-gate", true))
}

fn configured_validation_run_failure(last: Option<&ReportRecord>) -> Option<String> {
    let evidence = validation_run_evidence(last);
    eval_validation_run_failure(evidence.as_ref(), "configured", false)
        .map(|failure| validation_run_failure_text(failure, "configured", "configured", false))
}

fn validation_run_evidence(last: Option<&ReportRecord>) -> Option<TestGateValidationRunEvidence> {
    last.map(|last| TestGateValidationRunEvidence {
        command_source: last.validation_command_source.clone(),
        command_safety: last.validation_command_safety.clone(),
        checked: last.validation_checked,
        passed: last.validation_passed,
        status_code: last.validation_status_code,
    })
}

fn validation_run_failure_text(
    failure: TestGateValidationRunFailure,
    label: &str,
    expected_source: &str,
    require_safe_command: bool,
) -> String {
    match failure {
        TestGateValidationRunFailure::Missing => {
            format!("latest {label} validation result missing")
        }
        TestGateValidationRunFailure::SourceMismatch(source) => {
            format!("latest validation command source is {source}, expected {expected_source}")
        }
        TestGateValidationRunFailure::SourceMissing => {
            format!("latest validation command source missing, expected {expected_source}")
        }
        TestGateValidationRunFailure::SafetyMismatch(safety) => {
            if require_safe_command {
                format!("latest {label} validation command safety is {safety}, expected safe")
            } else {
                format!("latest {label} validation command safety is {safety}")
            }
        }
        TestGateValidationRunFailure::SafetyMissing => {
            if require_safe_command {
                format!("latest {label} validation command safety missing, expected safe")
            } else {
                format!("latest {label} validation command safety missing")
            }
        }
        TestGateValidationRunFailure::NotChecked => {
            format!("latest {label} validation was not checked")
        }
        TestGateValidationRunFailure::Failed { status_code } => format!(
            "latest {label} validation failed: status={}",
            option_i32_text(status_code)
        ),
    }
}

fn test_gate_verdict(text: &str) -> Option<&'static str> {
    eval_test_gate_verdict(text)
}

fn test_gate_summary(
    feedback_by_role: &BTreeMap<String, Vec<String>>,
    contract_fields_by_role: &BTreeMap<String, BTreeMap<String, String>>,
) -> TestGateSummary {
    let latest_feedback = feedback_by_role
        .get("test-gate")
        .and_then(|feedback| feedback.last());
    let mut latest_fields = latest_feedback
        .map(|feedback| helper_feedback::contract_fields("test-gate", &[feedback.to_owned()]))
        .unwrap_or_default();
    if let Some(structured_fields) = contract_fields_by_role.get("test-gate") {
        latest_fields.extend(structured_fields.clone());
    }

    if latest_feedback.is_none() && latest_fields.is_empty() {
        return TestGateSummary {
            latest_verdict: None,
            latest_validation_command: None,
            latest_validation_command_safety: "missing".to_owned(),
            latest_failure_kind: None,
            latest_fields: BTreeMap::new(),
        };
    }

    let latest_validation_command = test_gate_field_from_contract_or_feedback(
        &latest_fields,
        "validation_command",
        latest_feedback,
    );
    let latest_validation_command_safety =
        validation::test_gate_validation_command_safety(latest_validation_command.as_deref())
            .to_owned();
    TestGateSummary {
        latest_verdict: test_gate_verdict_from_contract_or_feedback(
            &latest_fields,
            latest_feedback,
        ),
        latest_validation_command,
        latest_validation_command_safety,
        latest_failure_kind: test_gate_field_from_contract_or_feedback(
            &latest_fields,
            "failure_kind",
            latest_feedback,
        ),
        latest_fields,
    }
}

fn test_gate_verdict_from_contract_or_feedback(
    fields: &BTreeMap<String, String>,
    feedback: Option<&String>,
) -> Option<String> {
    if fields.contains_key("verdict") {
        return test_gate_contract_field(fields, "verdict")
            .and_then(|verdict| test_gate_verdict(&verdict).map(str::to_owned));
    }
    feedback.and_then(|text| test_gate_verdict(text).map(str::to_owned))
}

fn test_gate_field_from_contract_or_feedback(
    fields: &BTreeMap<String, String>,
    field: &str,
    feedback: Option<&String>,
) -> Option<String> {
    if fields.contains_key(field) {
        return test_gate_contract_field(fields, field);
    }
    feedback.and_then(|text| test_gate_field(text, field))
}

fn test_gate_contract_field(fields: &BTreeMap<String, String>, field: &str) -> Option<String> {
    let value = fields.get(field)?.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("none") {
        None
    } else {
        Some(value.to_owned())
    }
}

fn test_gate_field(text: &str, field: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    let field_start = lower.find(field)?;
    let after_field = text.get(field_start + field.len()..)?;
    let value = after_field
        .trim_start_matches(|character: char| {
            character.is_whitespace() || matches!(character, ':' | '=' | '-' | '"' | '\'')
        })
        .split(" / ")
        .next()
        .unwrap_or_default()
        .split(" ; ")
        .next()
        .unwrap_or_default()
        .trim()
        .trim_matches(|character| matches!(character, '"' | '\''));
    if value.is_empty() || value.eq_ignore_ascii_case("none") {
        None
    } else {
        Some(value.to_owned())
    }
}

fn selected_context_evidence_summary(allocation_evidence: &[String]) -> Option<String> {
    let mut summaries = Vec::new();
    for item in allocation_evidence {
        if !item.contains("selected_context_") {
            continue;
        }
        let task_kind = allocation_task_kind(item);
        let required = allocation_value(item, "selected_context_required_tokens:")
            .unwrap_or_else(|| "?".to_owned());
        let buffer = allocation_value(item, "selected_context_buffer_tokens:")
            .unwrap_or_else(|| "?".to_owned());
        let sufficient = allocation_value(item, "selected_context_sufficient:")
            .unwrap_or_else(|| "?".to_owned());
        let reason = allocation_value(item, "selected_context_block_reason:")
            .unwrap_or_else(|| "none".to_owned());
        summaries.push(format!(
            "{} required:{} buffer:{} sufficient:{} reason:{}",
            task_kind, required, buffer, sufficient, reason
        ));
    }
    if summaries.is_empty() {
        None
    } else {
        Some(summaries.join(" | "))
    }
}

fn allocation_task_kind(item: &str) -> String {
    if let Some(after_prefix) = item.strip_prefix("pool_stage_route[")
        && let Some((task_kind, _)) = after_prefix.split_once(']')
    {
        return task_kind.to_owned();
    }
    allocation_value(item, "task_kind:").unwrap_or_else(|| "?".to_owned())
}

fn allocation_value(item: &str, key: &str) -> Option<String> {
    item.split_whitespace()
        .find_map(|part| part.strip_prefix(key))
        .map(|value| {
            value
                .trim_matches(|character: char| matches!(character, ',' | ';' | '"' | '\''))
                .to_owned()
        })
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
fn prompt_context_text(summary: &ReportSummary) -> String {
    prompt_context_text_with_self_improve_proposals(summary, None)
}

fn prompt_context_text_with_self_improve_proposals(
    summary: &ReportSummary,
    self_improve_proposal_artifact: Option<&SelfImproveProposalArtifact>,
) -> String {
    let helper_filters = prompt_helper_filters(summary);
    let (prompt_helper_stage_feedback, omitted_helper_feedback) =
        prompt_helper_stage_feedback(summary, &helper_filters);
    let (prompt_helper_stage_feedback_by_role, omitted_helper_feedback_by_role) =
        prompt_helper_stage_feedback_by_role(summary, &helper_filters);
    let (prompt_helper_stage_contract_by_role, omitted_helper_contracts) =
        prompt_helper_stage_contract_by_role(summary, &helper_filters);
    let mut helper_omissions = omitted_helper_feedback;
    helper_omissions.add(omitted_helper_feedback_by_role);
    helper_omissions.add(omitted_helper_contracts);
    let mut lines = vec![
        format!(
            "previous_rounds={} success_rate={:.1}% feedback_total={} self_improve={}/{} validation={}/{}",
            summary.total,
            percent(summary.success, summary.total),
            summary.feedback_applied,
            summary.self_improve_passed,
            summary.self_improve_checked,
            summary.validation_passed,
            summary.validation_checked
        ),
        format!(
            "ledger_hygiene=unique_rounds:{} duplicate_rounds:{} non_monotonic_rounds:{} missing_rounds:{} round_gaps:{} stream_truncated:{} missing_final:{} runtime_response_failures:{}",
            summary.unique_rounds,
            summary.duplicate_rounds,
            summary.non_monotonic_rounds,
            summary.missing_rounds,
            summary.round_gaps,
            summary.stream_truncation_failures,
            summary.missing_final_failures,
            summary.runtime_response_failures
        ),
        format!(
            "recent_failure_window=records:{} stream_truncated:{} missing_final:{} runtime_response_failures:{}",
            summary.recent_failure_window_records,
            summary.recent_stream_truncation_failures,
            summary.recent_missing_final_failures,
            summary.recent_runtime_response_failures
        ),
        format!(
            "runtime_tokens_total={} runtime_tokens_avg={} elapsed_ms_avg={} round_wall_elapsed_ms_avg={}",
            summary.runtime_tokens,
            average_text(summary.runtime_tokens, summary.runtime_token_items),
            average_text(summary.elapsed_ms, summary.elapsed_items),
            average_text(
                summary.round_wall_elapsed_ms,
                summary.round_wall_elapsed_items
            )
        ),
        format!(
            "rust_check={}/{} rust_feedback_total={} state_gate={}/{} trace_gate={}/{}",
            summary.rust_check_passed,
            summary.rust_check_checked,
            summary.rust_check_feedback_applied,
            summary.state_gate_passed,
            summary.state_gate_checked,
            summary.trace_gate_passed,
            summary.trace_gate_checked
        ),
    ];

    if let Some(artifact) = self_improve_proposal_artifact
        && artifact.total_candidate_count > 0
    {
        let acceptance = artifact.acceptance_summary_report();
        let assignment = artifact.acceptance_action_assignment();
        let closure_report = if assignment.target_count > 0 {
            Some(artifact.action_closure_report())
        } else {
            None
        };
        let all_action_targets_closed = closure_report
            .as_ref()
            .is_some_and(|report| report.all_targets_closed());
        let guidance = SelfImproveProposalPromptGuidance::from_summary(
            artifact.total_candidate_count,
            &acceptance,
        );
        lines.push(format!(
            "self_improve_proposal_acceptance=source:ledger_artifact candidates_total:{} projected:{} evidence_backed_business:{} advisory_only:{} repair_required:{} accepted_without_business_evidence:{}",
            guidance.total_candidate_count,
            guidance.projected_report_count,
            guidance.evidence_backed_business_improvement_count,
            guidance.advisory_only_count,
            guidance.require_repair_count,
            guidance.accepted_without_business_evidence_count
        ));
        if guidance.should_convert_advisory_to_evidence_backed_business_improvement
            && !all_action_targets_closed
        {
            lines.push(
                "next_self_improve_should_convert_advisory_to_evidence_backed_business_improvement:true"
                    .to_owned(),
            );
        }
        if all_action_targets_closed {
            lines.push(
                "next_self_improve_should_prepare_memory_admission_for_closed_action:true"
                    .to_owned(),
            );
        }
        if guidance.should_repair_unvalidated_or_unaccepted_proposals {
            lines.push(
                "next_self_improve_should_repair_unvalidated_or_unaccepted_proposals:true"
                    .to_owned(),
            );
        }
        if guidance.requires_checked_passed_validation_and_accepted_memory_admission {
            lines.push(
                "next_self_improve_requires_checked_passed_validation_and_accepted_memory_admission:true"
                    .to_owned(),
            );
        }
        if assignment.target_count > 0 {
            let first_target = assignment.first_target_digest();
            let first_target_id = first_target
                .as_ref()
                .map(|target| target.proposal_id.as_str())
                .unwrap_or("none");
            let first_missing = first_target
                .as_ref()
                .map(|target| target.missing_requirements.join(","))
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "none".to_owned());
            let first_round = first_target
                .as_ref()
                .map(|target| option_u64_text(target.source_round))
                .unwrap_or_else(|| "none".to_owned());
            let first_evidence_ids = first_target
                .as_ref()
                .map(|target| target.evidence_ids.join(","))
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "none".to_owned());
            let first_memory_admission = first_target
                .as_ref()
                .map(|target| target.current_memory_admission_decision.as_str())
                .unwrap_or("none");
            let first_validation_checked = first_target
                .as_ref()
                .map(|target| target.validation_checked)
                .unwrap_or(false);
            let first_validation_passed = first_target
                .as_ref()
                .map(|target| target.validation_passed)
                .unwrap_or(false);
            let first_memory_accepted = first_target
                .as_ref()
                .map(|target| target.memory_admission_accepted)
                .unwrap_or(false);
            let first_business_evidence = first_target
                .as_ref()
                .map(|target| target.evidence_backed_business_improvement)
                .unwrap_or(false);
            let first_advisory_only = first_target
                .as_ref()
                .map(|target| target.advisory_only)
                .unwrap_or(false);
            let first_require_repair = first_target
                .as_ref()
                .map(|target| target.require_repair)
                .unwrap_or(false);
            lines.push(format!(
                "self_improve_action_assignment=primary:{} targets:{} first_target:{} first_round:{} first_evidence_ids:{} first_memory_admission:{} first_validation_checked:{} first_validation_passed:{} first_memory_accepted:{} first_business_evidence:{} first_advisory_only:{} first_require_repair:{} first_missing:{}",
                assignment.primary_action,
                assignment.target_count,
                first_target_id,
                first_round,
                first_evidence_ids,
                first_memory_admission,
                first_validation_checked,
                first_validation_passed,
                first_memory_accepted,
                first_business_evidence,
                first_advisory_only,
                first_require_repair,
                first_missing
            ));
            let closure_report = closure_report.unwrap_or_else(|| artifact.action_closure_report());
            let first_closure_target = closure_report.first_target_id.as_deref().unwrap_or("none");
            let first_closure_kind = closure_report
                .first_target_closure_kind
                .as_deref()
                .unwrap_or("none");
            lines.push(format!(
                "self_improve_action_closure=targets:{} closed:{} open:{} first_target:{} first_closed:{} first_kind:{} first_still_requires_memory_admission:{}",
                closure_report.target_count,
                closure_report.closed_target_count,
                closure_report.open_target_count,
                first_closure_target,
                closure_report.first_target_closed,
                first_closure_kind,
                closure_report.first_target_still_requires_memory_admission
            ));
            let readiness = artifact.memory_admission_readiness_report();
            let first_readiness_target = readiness.first_target_id.as_deref().unwrap_or("none");
            lines.push(format!(
                "self_improve_memory_admission_readiness=targets:{} ready:{} blocked:{} first_target:{} first_ready:{} all_closed_targets_ready:{} memory_store_write_allowed:{} ndkv_write_allowed:{}",
                readiness.target_count,
                readiness.ready_count,
                readiness.blocked_count,
                first_readiness_target,
                readiness.first_target_ready,
                readiness.all_closed_targets_ready,
                readiness.memory_store_write_allowed,
                readiness.ndkv_write_allowed
            ));
            let request = artifact.memory_admission_request_report();
            let first_request_candidate = request.first_candidate_id.as_deref().unwrap_or("none");
            lines.push(format!(
                "self_improve_memory_admission_request=targets:{} requests:{} blocked:{} first_candidate:{} first_ready:{} all_ready_targets_requested:{} writer_required:{} auto_apply:{} memory_store_write_allowed:{} ndkv_write_allowed:{}",
                request.target_count,
                request.request_count,
                request.blocked_count,
                first_request_candidate,
                request.first_candidate_ready,
                request.all_ready_targets_requested,
                request.writer_required,
                request.auto_apply,
                request.memory_store_write_allowed,
                request.ndkv_write_allowed
            ));
            let decision = artifact.memory_admission_decision_report();
            let first_decision_candidate = decision.first_candidate_id.as_deref().unwrap_or("none");
            let decision_failures = if decision.failure_reasons.is_empty() {
                "none".to_owned()
            } else {
                decision.failure_reasons.join(",")
            };
            lines.push(format!(
                "self_improve_memory_admission_decision=targets:{} requests:{} blocked:{} first_candidate:{} writer_required:{} preflight_passed:{} explicit_writer_invocation_required:{} admission_write_authorized:{} gate_blocked:{} failure_reasons:{} auto_apply:{} memory_store_write_allowed:{} ndkv_write_allowed:{}",
                decision.target_count,
                decision.request_count,
                decision.blocked_count,
                first_decision_candidate,
                decision.writer_required,
                decision.admission_writer_preflight_passed,
                decision.explicit_writer_invocation_required,
                decision.admission_write_authorized,
                decision.gate_blocked,
                decision_failures,
                decision.auto_apply,
                decision.memory_store_write_allowed,
                decision.ndkv_write_allowed
            ));
            if closure_report.first_target_closed {
                lines.push("next_self_improve_should_not_repeat_closed_action:true".to_owned());
            }
            if closure_report.all_targets_closed() {
                lines
                    .push("next_self_improve_action_assignment_all_targets_closed:true".to_owned());
            }
            if readiness.all_closed_targets_ready {
                lines.push(
                    "next_self_improve_memory_admission_all_closed_targets_ready:true".to_owned(),
                );
            }
            if request.all_ready_targets_requested {
                lines
                    .push("next_self_improve_should_emit_memory_admission_request:true".to_owned());
            }
            if decision.admission_writer_preflight_passed {
                lines.push("next_self_improve_admission_writer_preflight_passed:true".to_owned());
            }
            let writer_plan = artifact.memory_admission_writer_plan();
            let first_writer_plan_item =
                writer_plan.first_plan_item_id.as_deref().unwrap_or("none");
            let writer_plan_failures = if writer_plan.failure_reasons.is_empty() {
                "none".to_owned()
            } else {
                writer_plan.failure_reasons.join(",")
            };
            lines.push(format!(
                "self_improve_memory_admission_writer_plan=targets:{} requests:{} plan_items:{} ready:{} blocked:{} first_item:{} writer_plan_ready:{} explicit_writer_invocation_required:{} experiment_required:{} rollback_required:{} validation_required:{} admission_write_authorized:{} failure_reasons:{} auto_apply:{} memory_store_write_allowed:{} ndkv_write_allowed:{}",
                writer_plan.target_count,
                writer_plan.request_count,
                writer_plan.writer_plan_item_count,
                writer_plan.ready_plan_count,
                writer_plan.blocked_count,
                first_writer_plan_item,
                writer_plan.writer_plan_ready,
                writer_plan.explicit_writer_invocation_required,
                writer_plan.experiment_required,
                writer_plan.rollback_required,
                writer_plan.validation_required,
                writer_plan.admission_write_authorized,
                writer_plan_failures,
                writer_plan.auto_apply,
                writer_plan.memory_store_write_allowed,
                writer_plan.ndkv_write_allowed
            ));
            if writer_plan.writer_plan_ready {
                lines.push(
                    "next_self_improve_should_invoke_explicit_memory_admission_writer:true"
                        .to_owned(),
                );
            }
            let writer_dry_run = artifact.memory_admission_writer_dry_run();
            let first_dry_run_item = writer_dry_run
                .first_dry_run_item_id
                .as_deref()
                .unwrap_or("none");
            let writer_dry_run_failures = if writer_dry_run.failure_reasons.is_empty() {
                "none".to_owned()
            } else {
                writer_dry_run.failure_reasons.join(",")
            };
            lines.push(format!(
                "self_improve_memory_admission_writer_dry_run=targets:{} requests:{} plan_items:{} dry_run_items:{} ready:{} blocked:{} first_item:{} dry_run_ready:{} explicit_writer_invocation_required:{} dry_run_required:{} experiment_required:{} rollback_required:{} validation_required:{} admission_write_authorized:{} failure_reasons:{} auto_apply:{} memory_store_write_allowed:{} ndkv_write_allowed:{}",
                writer_dry_run.target_count,
                writer_dry_run.request_count,
                writer_dry_run.writer_plan_item_count,
                writer_dry_run.dry_run_item_count,
                writer_dry_run.ready_dry_run_count,
                writer_dry_run.blocked_count,
                first_dry_run_item,
                writer_dry_run.dry_run_ready,
                writer_dry_run.explicit_writer_invocation_required,
                writer_dry_run.dry_run_required,
                writer_dry_run.experiment_required,
                writer_dry_run.rollback_required,
                writer_dry_run.validation_required,
                writer_dry_run.admission_write_authorized,
                writer_dry_run_failures,
                writer_dry_run.auto_apply,
                writer_dry_run.memory_store_write_allowed,
                writer_dry_run.ndkv_write_allowed
            ));
            if writer_dry_run.dry_run_ready {
                lines.push(
                    "next_self_improve_should_dry_run_explicit_memory_admission_writer:true"
                        .to_owned(),
                );
            }
            let writer_dry_run_receipt = artifact.memory_admission_writer_dry_run_receipt();
            let first_receipt_item = writer_dry_run_receipt
                .first_receipt_item_id
                .as_deref()
                .unwrap_or("none");
            let writer_dry_run_receipt_failures =
                if writer_dry_run_receipt.failure_reasons.is_empty() {
                    "none".to_owned()
                } else {
                    writer_dry_run_receipt.failure_reasons.join(",")
                };
            lines.push(format!(
                "self_improve_memory_admission_writer_dry_run_receipt=targets:{} requests:{} dry_run_items:{} receipt_items:{} succeeded:{} blocked:{} first_item:{} dry_run_receipt_ready:{} explicit_writer_invocation_required:{} commit_allowed:{} validation_required:{} rollback_required:{} admission_write_authorized:{} failure_reasons:{} auto_apply:{} memory_store_write_allowed:{} ndkv_write_allowed:{}",
                writer_dry_run_receipt.target_count,
                writer_dry_run_receipt.request_count,
                writer_dry_run_receipt.dry_run_item_count,
                writer_dry_run_receipt.receipt_item_count,
                writer_dry_run_receipt.succeeded_receipt_count,
                writer_dry_run_receipt.blocked_count,
                first_receipt_item,
                writer_dry_run_receipt.dry_run_receipt_ready,
                writer_dry_run_receipt.explicit_writer_invocation_required,
                writer_dry_run_receipt.commit_allowed,
                writer_dry_run_receipt.validation_required,
                writer_dry_run_receipt.rollback_required,
                writer_dry_run_receipt.admission_write_authorized,
                writer_dry_run_receipt_failures,
                writer_dry_run_receipt.auto_apply,
                writer_dry_run_receipt.memory_store_write_allowed,
                writer_dry_run_receipt.ndkv_write_allowed
            ));
            if writer_dry_run_receipt.dry_run_receipt_ready {
                lines.push(
                    "next_self_improve_should_record_memory_admission_writer_dry_run_receipt:true"
                        .to_owned(),
                );
            }
            let commit_record_stage = artifact.memory_admission_commit_record_stage();
            let first_commit_record_item = commit_record_stage
                .first_commit_record_item_id
                .as_deref()
                .unwrap_or("none");
            let commit_record_stage_failures = if commit_record_stage.failure_reasons.is_empty() {
                "none".to_owned()
            } else {
                commit_record_stage.failure_reasons.join(",")
            };
            lines.push(format!(
                "self_improve_memory_admission_commit_record_stage=targets:{} requests:{} receipt_items:{} commit_record_items:{} staged:{} blocked:{} first_item:{} commit_record_stage_ready:{} explicit_writer_invocation_required:{} validation_required:{} rollback_required:{} commit_allowed:{} admission_write_authorized:{} failure_reasons:{} auto_apply:{} memory_store_write_allowed:{} ndkv_write_allowed:{}",
                commit_record_stage.target_count,
                commit_record_stage.request_count,
                commit_record_stage.receipt_item_count,
                commit_record_stage.commit_record_item_count,
                commit_record_stage.staged_commit_record_count,
                commit_record_stage.blocked_count,
                first_commit_record_item,
                commit_record_stage.commit_record_stage_ready,
                commit_record_stage.explicit_writer_invocation_required,
                commit_record_stage.validation_required,
                commit_record_stage.rollback_required,
                commit_record_stage.commit_allowed,
                commit_record_stage.admission_write_authorized,
                commit_record_stage_failures,
                commit_record_stage.auto_apply,
                commit_record_stage.memory_store_write_allowed,
                commit_record_stage.ndkv_write_allowed
            ));
            if commit_record_stage.commit_record_stage_ready {
                lines.push(
                    "next_self_improve_should_stage_memory_admission_commit_record:true".to_owned(),
                );
            }
            let commit_approval_request = artifact.memory_admission_commit_approval_request();
            let first_approval_request_item = commit_approval_request
                .first_approval_request_item_id
                .as_deref()
                .unwrap_or("none");
            let commit_approval_request_failures =
                if commit_approval_request.failure_reasons.is_empty() {
                    "none".to_owned()
                } else {
                    commit_approval_request.failure_reasons.join(",")
                };
            lines.push(format!(
                "self_improve_memory_admission_commit_approval_request=targets:{} requests:{} commit_record_items:{} approval_request_items:{} requested:{} blocked:{} first_item:{} commit_approval_request_ready:{} explicit_commit_approval_required:{} validation_required:{} rollback_required:{} commit_allowed:{} admission_write_authorized:{} failure_reasons:{} auto_apply:{} memory_store_write_allowed:{} ndkv_write_allowed:{}",
                commit_approval_request.target_count,
                commit_approval_request.request_count,
                commit_approval_request.commit_record_item_count,
                commit_approval_request.approval_request_item_count,
                commit_approval_request.requested_commit_approval_count,
                commit_approval_request.blocked_count,
                first_approval_request_item,
                commit_approval_request.commit_approval_request_ready,
                commit_approval_request.explicit_commit_approval_required,
                commit_approval_request.validation_required,
                commit_approval_request.rollback_required,
                commit_approval_request.commit_allowed,
                commit_approval_request.admission_write_authorized,
                commit_approval_request_failures,
                commit_approval_request.auto_apply,
                commit_approval_request.memory_store_write_allowed,
                commit_approval_request.ndkv_write_allowed
            ));
            if commit_approval_request.commit_approval_request_ready {
                lines.push(
                    "next_self_improve_should_request_memory_admission_commit_approval:true"
                        .to_owned(),
                );
            }
            let commit_approval_decision = artifact.memory_admission_commit_approval_decision();
            let first_approval_decision_item = commit_approval_decision
                .first_approval_decision_item_id
                .as_deref()
                .unwrap_or("none");
            let commit_approval_decision_failures =
                if commit_approval_decision.failure_reasons.is_empty() {
                    "none".to_owned()
                } else {
                    commit_approval_decision.failure_reasons.join(",")
                };
            lines.push(format!(
                "self_improve_memory_admission_commit_approval_decision=targets:{} requests:{} approval_request_items:{} approval_decision_items:{} recorded:{} approved:{} pending:{} blocked:{} first_item:{} commit_approval_decision_ready:{} explicit_commit_approval_required:{} validation_required:{} rollback_required:{} commit_allowed:{} admission_write_authorized:{} failure_reasons:{} auto_apply:{} memory_store_write_allowed:{} ndkv_write_allowed:{}",
                commit_approval_decision.target_count,
                commit_approval_decision.request_count,
                commit_approval_decision.approval_request_item_count,
                commit_approval_decision.approval_decision_item_count,
                commit_approval_decision.recorded_approval_decision_count,
                commit_approval_decision.approved_commit_count,
                commit_approval_decision.pending_approval_count,
                commit_approval_decision.blocked_count,
                first_approval_decision_item,
                commit_approval_decision.commit_approval_decision_ready,
                commit_approval_decision.explicit_commit_approval_required,
                commit_approval_decision.validation_required,
                commit_approval_decision.rollback_required,
                commit_approval_decision.commit_allowed,
                commit_approval_decision.admission_write_authorized,
                commit_approval_decision_failures,
                commit_approval_decision.auto_apply,
                commit_approval_decision.memory_store_write_allowed,
                commit_approval_decision.ndkv_write_allowed
            ));
            if commit_approval_decision.commit_approval_decision_ready {
                lines.push(
                    "next_self_improve_should_record_memory_admission_commit_approval_decision:true"
                        .to_owned(),
                );
            }
            let commit_approval_review = artifact.memory_admission_commit_approval_review_packet();
            let first_review_packet_item = commit_approval_review
                .first_review_packet_item_id
                .as_deref()
                .unwrap_or("none");
            let commit_approval_review_failures =
                if commit_approval_review.failure_reasons.is_empty() {
                    "none".to_owned()
                } else {
                    commit_approval_review.failure_reasons.join(",")
                };
            lines.push(format!(
                "self_improve_memory_admission_commit_approval_review_packet=targets:{} requests:{} approval_decision_items:{} review_packet_items:{} ready:{} pending:{} blocked:{} first_item:{} approval_review_packet_ready:{} explicit_operator_approval_required:{} validation_required:{} rollback_required:{} commit_allowed:{} admission_write_authorized:{} failure_reasons:{} auto_apply:{} memory_store_write_allowed:{} ndkv_write_allowed:{}",
                commit_approval_review.target_count,
                commit_approval_review.request_count,
                commit_approval_review.approval_decision_item_count,
                commit_approval_review.review_packet_item_count,
                commit_approval_review.ready_review_packet_count,
                commit_approval_review.pending_approval_count,
                commit_approval_review.blocked_count,
                first_review_packet_item,
                commit_approval_review.approval_review_packet_ready,
                commit_approval_review.explicit_operator_approval_required,
                commit_approval_review.validation_required,
                commit_approval_review.rollback_required,
                commit_approval_review.commit_allowed,
                commit_approval_review.admission_write_authorized,
                commit_approval_review_failures,
                commit_approval_review.auto_apply,
                commit_approval_review.memory_store_write_allowed,
                commit_approval_review.ndkv_write_allowed
            ));
            if commit_approval_review.approval_review_packet_ready {
                lines.push(
                    "next_self_improve_should_review_memory_admission_commit_approval_packet:true"
                        .to_owned(),
                );
            }
        }
    }

    if summary.eval_records > 0 {
        lines.push(format!(
            "eval_report_only=records:{} report_only:{} failure_kinds:{}",
            summary.eval_records,
            summary.eval_report_only_records,
            eval_failure_kinds_text(&summary.eval_failure_kinds)
        ));
    }
    if let Some(repeated) = &summary.recent_repeated_successful_answer {
        lines.push(format!(
            "recent_repeated_successful_answer=count:{} window_records:{} blocked_topic:{} preview_redacted:true",
            repeated.count,
            repeated.window_records,
            repeated_successful_answer_blocked_topic(&repeated.preview)
        ));
        lines.push("next_advice_should_not_repeat_recent_successful_answer:true".to_owned());
        lines.push("next_advice_must_not_use_repeated_answer_preview_as_evidence:true".to_owned());
    }
    if !summary.completed_change_requests.is_empty() {
        lines.push(format!(
            "completed_change_requests_do_not_repeat={}",
            summary.completed_change_requests.join(" | ")
        ));
        let blocked_topics =
            completed_change_request_blocked_topics(&summary.completed_change_requests);
        if !blocked_topics.is_empty() {
            lines.push(format!(
                "blocked_completed_change_topics={}",
                blocked_topics.join(",")
            ));
        }
        lines.push("completed_change_requests_are_already_done:true".to_owned());
        lines.push("next_advice_must_not_recommend_completed_change_requests:true".to_owned());
        lines.push("next_advice_must_choose_new_uncompleted_change:true".to_owned());
    }
    if !summary.invalid_change_requests.is_empty() {
        lines.push(format!(
            "invalid_change_requests_do_not_repeat={}",
            summary.invalid_change_requests.join(" | ")
        ));
        let blocked_topics =
            invalid_change_request_blocked_topics(&summary.invalid_change_requests);
        if !blocked_topics.is_empty() {
            lines.push(format!(
                "blocked_invalid_change_topics={}",
                blocked_topics.join(",")
            ));
        }
        lines.push("invalid_change_requests_are_rejected:true".to_owned());
        lines.push("next_advice_must_not_recommend_invalid_change_requests:true".to_owned());
        lines.push("next_advice_must_choose_valid_executable_change:true".to_owned());
    }

    if let Some(last) = &summary.last {
        lines.push(format!(
            "last_round={} case={} success={} feedback={} error={}",
            option_u64_text(last.round),
            last.case_name.as_deref().unwrap_or("?"),
            last.success,
            option_u64_text(last.feedback_applied),
            last.error.as_deref().unwrap_or("none")
        ));
        if !last.allocation_evidence.is_empty() {
            if let Some(selected_context) =
                selected_context_evidence_summary(&last.allocation_evidence)
            {
                lines.push(format!("last_selected_context_evidence={selected_context}"));
            }
            lines.push(format!(
                "last_allocation_evidence={}",
                last.allocation_evidence.join(" | ")
            ));
        }
        if let Some(source) = last.validation_command_source.as_deref() {
            lines.push(format!(
                "last_validation_command=source:{} safety:{} command:{}",
                source,
                last.validation_command_safety
                    .as_deref()
                    .unwrap_or("unknown"),
                last.validation_command_preview.as_deref().unwrap_or("none")
            ));
        }
        if let Some(phase) = last.validation_phase.as_deref() {
            lines.push(format!(
                "last_validation_result=phase:{} status:{} elapsed_ms:{} stdout_tail:{} stderr_tail:{}",
                phase,
                option_i32_text(last.validation_status_code),
                option_u64_text(last.validation_elapsed_ms),
                last.validation_stdout_tail.as_deref().unwrap_or("-"),
                last.validation_stderr_tail.as_deref().unwrap_or("-")
            ));
        }
    }
    if !helper_filters.satisfied_role_budgets.is_empty() {
        lines.push(format!(
            "current_role_max_tokens_satisfied={}",
            role_budget_text(&helper_filters.satisfied_role_budgets)
        ));
    }
    if !helper_filters.underutilized_role_budgets.is_empty() {
        lines.push(format!(
            "current_role_token_headroom={}",
            role_token_headroom_text(&helper_filters.underutilized_role_budgets)
        ));
    }
    if !prompt_helper_stage_feedback.is_empty() {
        lines.push(format!(
            "recent_helper_stage_feedback={}",
            prompt_helper_stage_feedback.join(" | ")
        ));
    }
    if !prompt_helper_stage_feedback_by_role.is_empty() {
        lines.push(format!(
            "recent_helper_stage_feedback_by_role={}",
            helper_stage_feedback_by_role_text(&prompt_helper_stage_feedback_by_role)
        ));
    }
    if !prompt_helper_stage_contract_by_role.is_empty() {
        lines.push(format!(
            "recent_helper_stage_contract_by_role={}",
            helper_stage_contract_by_role_text(&prompt_helper_stage_contract_by_role)
        ));
    }
    if helper_omissions.stale_context > 0
        && let Some(recovery) = &helper_filters.stale_context
    {
        lines.push(format!(
            "stale_helper_stage_feedback_omitted=count:{} current_quality_context_required_tokens:{} latest_success_round:{} reason=current_route_evidence_contradicts_stale_context_advice",
            helper_omissions.stale_context,
            recovery.current_quality_context_required_tokens,
            recovery.latest_success_round
        ));
        lines.push("next_advice_should_ignore_stale_helper_context_expansion:true".to_owned());
    }
    if helper_omissions.completed_change_topic > 0 {
        lines.push(format!(
            "completed_change_topic_helper_context_omitted=count:{} topics:{} reason=latest_successful_contract_already_has_completed_topic",
            helper_omissions.completed_change_topic,
            completed_change_topics_text(&helper_filters.completed_change_topics)
        ));
        lines.push("next_advice_should_not_use_completed_topic_feedback:true".to_owned());
    }
    if helper_omissions.invalid_change_topic > 0 {
        lines.push(format!(
            "invalid_change_topic_helper_context_omitted=count:{} topics:{} reason=latest_review_change_request_is_invalid",
            helper_omissions.invalid_change_topic,
            completed_change_topics_text(&helper_filters.invalid_change_topics)
        ));
        lines.push("next_advice_should_not_use_invalid_change_feedback:true".to_owned());
    }
    if helper_omissions.generic_noop_proposal > 0 {
        lines.push(format!(
            "generic_noop_helper_context_omitted=count:{} reason=review_change_request_is_not_actionable",
            helper_omissions.generic_noop_proposal
        ));
    }
    if helper_omissions.satisfied_role_budget > 0 {
        lines.push(format!(
            "satisfied_role_budget_advice_omitted=count:{} roles:{} reason=latest_route_already_has_requested_max_tokens",
            helper_omissions.satisfied_role_budget,
            role_budget_text(&helper_filters.satisfied_role_budgets)
        ));
        lines.push("next_advice_should_not_repeat_satisfied_role_budget:true".to_owned());
    }
    if helper_omissions.underutilized_budget_increase > 0 {
        lines.push(format!(
            "underutilized_role_budget_increase_omitted=count:{} roles:{} reason=latest_helper_output_far_below_current_max_tokens",
            helper_omissions.underutilized_budget_increase,
            role_token_headroom_text(&helper_filters.underutilized_role_budgets)
        ));
        lines.push(
            "next_advice_should_require_truncation_or_high_utilization_before_budget_increase:true"
                .to_owned(),
        );
    }
    if has_test_gate_summary(&summary.test_gate) {
        lines.push(format!(
            "latest_test_gate={}",
            test_gate_context_text(&summary.test_gate)
        ));
    }
    if let Some(recent) = summary.recent_failures.first() {
        if let Some(latest_success_round) =
            stale_failure_recovered_by_latest_success(summary, recent)
        {
            lines.push(format!(
                "most_recent_failure=round {} case {} status=stale_recovered latest_success_round={} error_omitted=true",
                option_u64_text(recent.round),
                recent.case_name.as_deref().unwrap_or("?"),
                latest_success_round
            ));
            lines.push(
                "next_advice_should_use_current_route_evidence_over_stale_failure:true".to_owned(),
            );
        } else {
            lines.push(format!(
                "most_recent_failure=round {} case {} error {}",
                option_u64_text(recent.round),
                recent.case_name.as_deref().unwrap_or("?"),
                recent.error.as_deref().unwrap_or("final gate failed")
            ));
        }
    }
    lines.join("\n")
}

fn prompt_helper_filters(summary: &ReportSummary) -> PromptHelperFilters {
    PromptHelperFilters {
        stale_context: prompt_stale_helper_recovery(summary),
        completed_change_topics: completed_change_request_blocked_topics(
            &summary.completed_change_requests,
        ),
        invalid_change_topics: invalid_change_request_blocked_topics(
            &summary.invalid_change_requests,
        ),
        satisfied_role_budgets: prompt_satisfied_role_budgets(summary),
        underutilized_role_budgets: prompt_underutilized_role_budgets(summary),
    }
}

fn prompt_stale_helper_recovery(summary: &ReportSummary) -> Option<PromptStaleHelperRecovery> {
    let last = summary.last.as_ref()?;
    if !last.success {
        return None;
    }
    let latest_success_round = last.round?;
    let current_quality_context_required_tokens =
        latest_sufficient_quality_context_required_tokens(last)?;
    Some(PromptStaleHelperRecovery {
        current_quality_context_required_tokens,
        latest_success_round,
    })
}

fn latest_sufficient_quality_context_required_tokens(record: &ReportRecord) -> Option<String> {
    record.allocation_evidence.iter().rev().find_map(|item| {
        if allocation_task_kind(item) != "quality" {
            return None;
        }
        if allocation_value(item, "quality_context_sufficient:").as_deref() != Some("true") {
            return None;
        }
        allocation_value(item, "quality_context_required_tokens:")
    })
}

fn prompt_satisfied_role_budgets(summary: &ReportSummary) -> BTreeMap<String, String> {
    let Some(last) = &summary.last else {
        return BTreeMap::new();
    };
    if !last.success {
        return BTreeMap::new();
    }
    let mut budgets = BTreeMap::new();
    for item in last.allocation_evidence.iter().rev() {
        if allocation_value(item, "selected_context_sufficient:").as_deref() != Some("true") {
            continue;
        }
        let role = allocation_task_kind(item);
        if role == "?" || role.is_empty() {
            continue;
        }
        let Some(tokens) = allocation_value(item, "selected_max_tokens:")
            .or_else(|| allocation_value(item, "effective_max_tokens:"))
            .or_else(|| allocation_value(item, "selected_context_required_tokens:"))
        else {
            continue;
        };
        budgets.entry(role).or_insert(tokens);
    }
    budgets
}

fn prompt_underutilized_role_budgets(
    summary: &ReportSummary,
) -> BTreeMap<String, PromptRoleTokenHeadroom> {
    let Some(last) = &summary.last else {
        return BTreeMap::new();
    };
    if !last.success {
        return BTreeMap::new();
    }
    let current_max_tokens = prompt_satisfied_role_budgets(summary)
        .into_iter()
        .filter_map(|(role, tokens)| tokens.parse::<u64>().ok().map(|tokens| (role, tokens)))
        .collect::<BTreeMap<_, _>>();
    if current_max_tokens.is_empty() {
        return BTreeMap::new();
    }

    let mut headroom = BTreeMap::new();
    for (role, feedback) in last.helper_stage_feedback_by_role() {
        let Some(max_tokens) = current_max_tokens.get(&role).copied() else {
            continue;
        };
        let Some(used_tokens) = feedback
            .iter()
            .filter_map(|item| helper_feedback_answer_approx_tokens(item))
            .max()
        else {
            continue;
        };
        if role_token_budget_is_underutilized(used_tokens, max_tokens) {
            headroom.insert(
                role,
                PromptRoleTokenHeadroom {
                    used_tokens,
                    max_tokens,
                },
            );
        }
    }
    headroom
}

fn helper_feedback_answer_approx_tokens(text: &str) -> Option<u64> {
    text.split_whitespace().find_map(|part| {
        part.strip_prefix("answer_approx_tokens=")
            .or_else(|| part.strip_prefix("answer_approx_tokens:"))
            .and_then(parse_trimmed_u64)
    })
}

fn parse_trimmed_u64(value: &str) -> Option<u64> {
    value
        .trim_matches(|character: char| !character.is_ascii_digit())
        .parse::<u64>()
        .ok()
}

fn role_token_budget_is_underutilized(used_tokens: u64, max_tokens: u64) -> bool {
    max_tokens >= 256 && used_tokens.saturating_mul(2) <= max_tokens
}

fn prompt_helper_stage_feedback(
    summary: &ReportSummary,
    filters: &PromptHelperFilters,
) -> (Vec<String>, PromptHelperOmissionCounts) {
    filter_prompt_helper_items(summary.helper_stage_feedback.iter(), filters)
}

fn prompt_helper_stage_feedback_by_role(
    summary: &ReportSummary,
    filters: &PromptHelperFilters,
) -> (BTreeMap<String, Vec<String>>, PromptHelperOmissionCounts) {
    let mut omitted = PromptHelperOmissionCounts::default();
    let mut filtered = BTreeMap::new();
    for (role, feedback) in &summary.helper_stage_feedback_by_role {
        let (items, role_omitted) = filter_prompt_helper_items(feedback.iter(), filters);
        omitted.add(role_omitted);
        if !items.is_empty() {
            filtered.insert(role.clone(), items);
        }
    }
    (filtered, omitted)
}

fn prompt_helper_stage_contract_by_role(
    summary: &ReportSummary,
    filters: &PromptHelperFilters,
) -> (
    BTreeMap<String, HelperStageContractSummary>,
    PromptHelperOmissionCounts,
) {
    let mut omitted = PromptHelperOmissionCounts::default();
    let mut filtered = BTreeMap::new();
    for (role, contract) in &summary.helper_stage_contract_by_role {
        match helper_stage_contract_omit_reason(contract, filters) {
            Some(PromptHelperOmitReason::StaleContext) => omitted.stale_context += 1,
            Some(PromptHelperOmitReason::CompletedChangeTopic) => {
                omitted.completed_change_topic += 1
            }
            Some(PromptHelperOmitReason::InvalidChangeTopic) => omitted.invalid_change_topic += 1,
            Some(PromptHelperOmitReason::GenericNoopProposal) => omitted.generic_noop_proposal += 1,
            Some(PromptHelperOmitReason::SatisfiedRoleBudget) => omitted.satisfied_role_budget += 1,
            Some(PromptHelperOmitReason::UnderutilizedBudgetIncrease) => {
                omitted.underutilized_budget_increase += 1
            }
            None => {
                filtered.insert(role.clone(), contract.clone());
            }
        }
    }
    (filtered, omitted)
}

fn filter_prompt_helper_items<'a>(
    items: impl Iterator<Item = &'a String>,
    filters: &PromptHelperFilters,
) -> (Vec<String>, PromptHelperOmissionCounts) {
    let mut omitted = PromptHelperOmissionCounts::default();
    let mut filtered = Vec::new();
    for item in items {
        match helper_feedback_omit_reason(item, filters) {
            Some(PromptHelperOmitReason::StaleContext) => omitted.stale_context += 1,
            Some(PromptHelperOmitReason::CompletedChangeTopic) => {
                omitted.completed_change_topic += 1
            }
            Some(PromptHelperOmitReason::InvalidChangeTopic) => omitted.invalid_change_topic += 1,
            Some(PromptHelperOmitReason::GenericNoopProposal) => omitted.generic_noop_proposal += 1,
            Some(PromptHelperOmitReason::SatisfiedRoleBudget) => omitted.satisfied_role_budget += 1,
            Some(PromptHelperOmitReason::UnderutilizedBudgetIncrease) => {
                omitted.underutilized_budget_increase += 1
            }
            None => filtered.push(item.clone()),
        }
    }
    (filtered, omitted)
}

fn helper_stage_contract_omit_reason(
    contract: &HelperStageContractSummary,
    filters: &PromptHelperFilters,
) -> Option<PromptHelperOmitReason> {
    contract
        .latest_preview
        .as_deref()
        .and_then(|preview| helper_feedback_omit_reason(preview, filters))
        .or_else(|| {
            contract
                .fields
                .values()
                .find_map(|value| helper_feedback_omit_reason(value, filters))
        })
}

fn helper_feedback_omit_reason(
    text: &str,
    filters: &PromptHelperFilters,
) -> Option<PromptHelperOmitReason> {
    if filters
        .stale_context
        .as_ref()
        .is_some_and(|recovery| helper_feedback_mentions_stale_context_advice(text, recovery))
    {
        return Some(PromptHelperOmitReason::StaleContext);
    }
    if helper_feedback_mentions_completed_change_topic(text, &filters.completed_change_topics) {
        return Some(PromptHelperOmitReason::CompletedChangeTopic);
    }
    if helper_feedback_mentions_invalid_change_topic(text, &filters.invalid_change_topics) {
        return Some(PromptHelperOmitReason::InvalidChangeTopic);
    }
    if helper_feedback_mentions_generic_noop_proposal(text) {
        return Some(PromptHelperOmitReason::GenericNoopProposal);
    }
    if helper_feedback_mentions_satisfied_role_budget_advice(text, &filters.satisfied_role_budgets)
    {
        return Some(PromptHelperOmitReason::SatisfiedRoleBudget);
    }
    helper_feedback_mentions_underutilized_budget_increase(
        text,
        &filters.underutilized_role_budgets,
    )
    .then_some(PromptHelperOmitReason::UnderutilizedBudgetIncrease)
}

fn helper_feedback_mentions_stale_context_advice(
    text: &str,
    recovery: &PromptStaleHelperRecovery,
) -> bool {
    let lower = text.to_ascii_lowercase();
    let mentions_quality_context = lower.contains("quality")
        && (lower.contains("context_window")
            || lower.contains("context window")
            || lower.contains("quality_context_required_tokens")
            || lower.contains("quality_context_window_too_small"));
    if !mentions_quality_context {
        return false;
    }
    if lower.contains("quality_context_window_too_small") {
        return true;
    }
    lower.contains("262144") && recovery.current_quality_context_required_tokens != "262144"
}

fn helper_feedback_mentions_completed_change_topic(text: &str, topics: &[String]) -> bool {
    if topics.is_empty() {
        return false;
    }
    let normalized = normalized_advice_key(text);
    topics.iter().any(|topic| {
        let normalized_topic = normalized_advice_key(topic);
        let normalized_marker = topic
            .rsplit('.')
            .next()
            .map(normalized_advice_key)
            .unwrap_or_else(|| normalized_topic.clone());
        (!normalized_topic.is_empty() && normalized.contains(&normalized_topic))
            || (!normalized_marker.is_empty() && normalized.contains(&normalized_marker))
    })
}

fn helper_feedback_mentions_invalid_change_topic(text: &str, topics: &[String]) -> bool {
    if topics.is_empty() {
        return false;
    }
    topics.iter().any(|topic| {
        if topic == "cargo.test.strict-flag" {
            return change_request_has_invalid_cargo_test_strict_flag(text);
        }
        if topic == "evolution-loop.max-iterations" {
            return change_request_has_redundant_max_iterations_flag(text);
        }
        if topic == "evolution-loop.strict-coverage" {
            return change_request_has_unproven_strict_coverage_control(text);
        }
        topic == "evolution-loop.test-deterministic-seed"
            && change_request_has_unproven_test_seed_control(text)
    })
}

fn helper_feedback_mentions_generic_noop_proposal(text: &str) -> bool {
    let normalized = normalized_advice_key(text);
    normalized.contains("no change suggested in the primary answer")
        || normalized.contains("no change suggested in primary answer")
        || normalized.contains("no small next change is grounded in the same evidence")
        || normalized.contains("no small next change is grounded in same evidence")
        || normalized.contains("small next change grounded in the same evidence")
        || normalized.contains("small next change grounded in same evidence")
        || normalized.contains("small next change is grounded in the same evidence")
        || normalized.contains("small next change is grounded in same evidence")
}

fn helper_feedback_mentions_satisfied_role_budget_advice(
    text: &str,
    role_budgets: &BTreeMap<String, String>,
) -> bool {
    if role_budgets.is_empty() {
        return false;
    }
    let lower = text.to_ascii_lowercase();
    let mentions_budget = lower.contains("default_max_tokens")
        || lower.contains("max_tokens")
        || lower.contains("max token")
        || lower.contains("required tokens")
        || lower.contains("token requirement")
        || lower.contains("token expectations");
    if !mentions_budget {
        return false;
    }
    let asks_for_change = lower.contains("change_request")
        || lower.contains("update")
        || lower.contains("set")
        || lower.contains("explicitly")
        || lower.contains("inconsistent")
        || lower.contains("discrepancy");
    if !asks_for_change {
        return false;
    }
    role_budgets
        .iter()
        .any(|(role, tokens)| lower.contains(&role.to_ascii_lowercase()) && lower.contains(tokens))
}

fn helper_feedback_mentions_underutilized_budget_increase(
    text: &str,
    headroom: &BTreeMap<String, PromptRoleTokenHeadroom>,
) -> bool {
    if headroom.is_empty() {
        return false;
    }
    let lower = text.to_ascii_lowercase();
    if !text_mentions_token_budget(&lower) || !text_asks_for_budget_increase(&lower) {
        return false;
    }
    let numbers = unsigned_numbers_in_text(&lower);
    headroom.iter().any(|(role, evidence)| {
        lower.contains(&role.to_ascii_lowercase())
            && (numbers.iter().any(|number| *number > evidence.max_tokens)
                || lower.contains("headroom"))
    })
}

fn text_mentions_token_budget(lower: &str) -> bool {
    lower.contains("default_max_tokens")
        || lower.contains("selected_max_tokens")
        || lower.contains("max_tokens")
        || lower.contains("max token")
        || lower.contains("required tokens")
        || lower.contains("token requirement")
        || lower.contains("token expectations")
}

fn text_asks_for_budget_increase(lower: &str) -> bool {
    lower.contains("change_request")
        || lower.contains("update")
        || lower.contains("set")
        || lower.contains("explicitly")
        || lower.contains("inconsistent")
        || lower.contains("discrepancy")
        || lower.contains("increase")
        || lower.contains("raise")
        || lower.contains("optimize")
        || lower.contains("headroom")
}

fn unsigned_numbers_in_text(text: &str) -> Vec<u64> {
    text.split(|character: char| !character.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse::<u64>().ok())
        .collect()
}

fn role_budget_text(values: &BTreeMap<String, String>) -> String {
    if values.is_empty() {
        return "none".to_owned();
    }
    values
        .iter()
        .map(|(role, tokens)| format!("{role}:{tokens}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn role_token_headroom_text(values: &BTreeMap<String, PromptRoleTokenHeadroom>) -> String {
    if values.is_empty() {
        return "none".to_owned();
    }
    values
        .iter()
        .map(|(role, evidence)| format!("{role}:{}/{}", evidence.used_tokens, evidence.max_tokens))
        .collect::<Vec<_>>()
        .join(",")
}

fn completed_change_topics_text(topics: &[String]) -> String {
    if topics.is_empty() {
        return "none".to_owned();
    }
    topics.join(",")
}

fn completed_change_request_blocked_topics(completed_change_requests: &[String]) -> Vec<String> {
    completed_change_requests
        .iter()
        .filter_map(|request| {
            if request.starts_with("review.change_request requested final_json.pool_stage_dispatch")
            {
                return Some("final_json.pool_stage_dispatch".to_owned());
            }
            request
                .strip_prefix("review.change_request requested index.")
                .and_then(|rest| rest.split(';').next())
                .map(str::trim)
                .filter(|marker| !marker.is_empty())
                .map(|marker| format!("index.{marker}"))
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn invalid_change_request_blocked_topics(invalid_change_requests: &[String]) -> Vec<String> {
    invalid_change_requests
        .iter()
        .filter_map(|request| {
            if request.contains("cargo.test.strict-flag") {
                return Some("cargo.test.strict-flag".to_owned());
            }
            if request.contains("evolution-loop.max-iterations") {
                return Some("evolution-loop.max-iterations".to_owned());
            }
            if request.contains("evolution-loop.strict-coverage") {
                return Some("evolution-loop.strict-coverage".to_owned());
            }
            request
                .contains("evolution-loop.test-deterministic-seed")
                .then(|| "evolution-loop.test-deterministic-seed".to_owned())
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn stale_failure_recovered_by_latest_success(
    summary: &ReportSummary,
    recent: &ReportRecord,
) -> Option<u64> {
    let last = summary.last.as_ref()?;
    if !last.success {
        return None;
    }
    let last_round = last.round?;
    let failure_round = recent.round?;
    (last_round > failure_round).then_some(last_round)
}

fn write_report_json(
    path: &Path,
    summary: &ReportSummary,
    remote_chain_status: Option<&RemoteChainStatusSummary>,
    pool_manifest: Option<&PoolManifestSummary>,
    pool_status: Option<&PoolStatusSummary>,
    pool_route: Option<&PoolRouteSummary>,
    pool_budget_fairness: Option<&PoolBudgetFairnessSummary>,
    worker_window_status: Option<&WorkerWindowStatusSummary>,
    clean_room_batch_status: Option<&CleanRoomBatchStatusSummary>,
    clean_room_handoff: Option<&CleanRoomHandoffSummary>,
    self_improve_proposal_artifact: &SelfImproveProposalArtifact,
    required_latest_helper_stage_roles: &[String],
    ledger_gate_failures: &[String],
    strict_gate_failures: &[String],
    continuation_gate_failures: &[String],
    gate_failures: &[String],
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "create report directory {} failed: {error}",
                parent.display()
            )
        })?;
    }
    fs::write(
        path,
        report_json_with_remote_chain_and_required_latest_roles(
            summary,
            remote_chain_status,
            pool_manifest,
            pool_status,
            pool_route,
            pool_budget_fairness,
            worker_window_status,
            clean_room_batch_status,
            clean_room_handoff,
            Some(self_improve_proposal_artifact),
            required_latest_helper_stage_roles,
            ledger_gate_failures,
            strict_gate_failures,
            continuation_gate_failures,
            gate_failures,
        ),
    )
    .map_err(|error| format!("write report JSON {} failed: {error}", path.display()))
}

#[cfg(test)]
fn report_json(
    summary: &ReportSummary,
    pool_status: Option<&PoolStatusSummary>,
    pool_route: Option<&PoolRouteSummary>,
    pool_budget_fairness: Option<&PoolBudgetFairnessSummary>,
    gate_failures: &[String],
) -> String {
    report_json_with_remote_chain(
        summary,
        None,
        None,
        pool_status,
        pool_route,
        pool_budget_fairness,
        None,
        None,
        None,
        None,
        gate_failures,
        gate_failures,
        gate_failures,
        gate_failures,
    )
}

#[cfg(test)]
fn report_json_with_required_latest_helper_stage_roles(
    summary: &ReportSummary,
    required_latest_helper_stage_roles: &[String],
) -> String {
    report_json_with_remote_chain_and_required_latest_roles(
        summary,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        required_latest_helper_stage_roles,
        &[],
        &[],
        &[],
        &[],
    )
}

#[cfg(test)]
fn report_json_with_remote_chain(
    summary: &ReportSummary,
    remote_chain_status: Option<&RemoteChainStatusSummary>,
    pool_manifest: Option<&PoolManifestSummary>,
    pool_status: Option<&PoolStatusSummary>,
    pool_route: Option<&PoolRouteSummary>,
    pool_budget_fairness: Option<&PoolBudgetFairnessSummary>,
    worker_window_status: Option<&WorkerWindowStatusSummary>,
    clean_room_batch_status: Option<&CleanRoomBatchStatusSummary>,
    clean_room_handoff: Option<&CleanRoomHandoffSummary>,
    self_improve_proposal_artifact: Option<&SelfImproveProposalArtifact>,
    ledger_gate_failures: &[String],
    strict_gate_failures: &[String],
    continuation_gate_failures: &[String],
    gate_failures: &[String],
) -> String {
    report_json_with_remote_chain_and_required_latest_roles(
        summary,
        remote_chain_status,
        pool_manifest,
        pool_status,
        pool_route,
        pool_budget_fairness,
        worker_window_status,
        clean_room_batch_status,
        clean_room_handoff,
        self_improve_proposal_artifact,
        &[],
        ledger_gate_failures,
        strict_gate_failures,
        continuation_gate_failures,
        gate_failures,
    )
}

fn report_json_with_remote_chain_and_required_latest_roles(
    summary: &ReportSummary,
    remote_chain_status: Option<&RemoteChainStatusSummary>,
    pool_manifest: Option<&PoolManifestSummary>,
    pool_status: Option<&PoolStatusSummary>,
    pool_route: Option<&PoolRouteSummary>,
    pool_budget_fairness: Option<&PoolBudgetFairnessSummary>,
    worker_window_status: Option<&WorkerWindowStatusSummary>,
    clean_room_batch_status: Option<&CleanRoomBatchStatusSummary>,
    clean_room_handoff: Option<&CleanRoomHandoffSummary>,
    self_improve_proposal_artifact: Option<&SelfImproveProposalArtifact>,
    required_latest_helper_stage_roles: &[String],
    ledger_gate_failures: &[String],
    strict_gate_failures: &[String],
    continuation_gate_failures: &[String],
    gate_failures: &[String],
) -> String {
    let pool_alignment = pool_alignment_summary(pool_manifest, pool_status, pool_route);
    format!(
        "{{\"rounds\":{},\"ledger_hygiene\":{{\"unique_rounds\":{},\"duplicate_rounds\":{},\"non_monotonic_rounds\":{},\"missing_rounds\":{},\"round_gaps\":{}}},\"success\":{},\"failures\":{},\"stream_failures\":{{\"truncated\":{},\"missing_final\":{}}},\"runtime_response_failures\":{},\"recent_repeated_successful_answer\":{},\"completed_change_requests\":{{\"items\":{},\"blocked_topics\":{}}},\"invalid_change_requests\":{{\"items\":{},\"blocked_topics\":{}}},\"success_rate\":{:.3},\"runtime_tokens\":{{\"total\":{},\"avg\":{}}},\"elapsed_ms\":{{\"total\":{},\"avg\":{}}},\"round_wall_elapsed_ms\":{{\"total\":{},\"avg\":{}}},\"feedback_applied\":{{\"total\":{},\"avg\":{}}},\"rust_check\":{{\"passed\":{},\"checked\":{},\"feedback_applied\":{{\"total\":{},\"avg\":{}}}}},\"validation\":{{\"passed\":{},\"checked\":{}}},\"validation_command_coverage_report_v1\":{},\"self_improve\":{{\"passed\":{},\"checked\":{}}},\"self_improve_proposal_artifact_v1\":{},\"self_improve_proposal_acceptance_summary_v1\":{},\"self_improve_proposal_action_assignment_v1\":{},\"self_improve_proposal_action_closure_report_v1\":{},\"self_improve_proposal_memory_admission_readiness_report_v1\":{},\"self_improve_proposal_memory_admission_request_report_v1\":{},\"self_improve_proposal_memory_admission_decision_report_v1\":{},\"self_improve_proposal_memory_admission_writer_plan_report_v1\":{},\"self_improve_proposal_memory_admission_writer_dry_run_report_v1\":{},\"self_improve_proposal_memory_admission_writer_dry_run_receipt_report_v1\":{},\"self_improve_proposal_memory_admission_commit_record_stage_report_v1\":{},\"self_improve_proposal_memory_admission_commit_approval_request_report_v1\":{},\"self_improve_proposal_memory_admission_commit_approval_decision_report_v1\":{},\"self_improve_proposal_memory_admission_commit_approval_review_packet_report_v1\":{},\"state_gate\":{{\"passed\":{},\"checked\":{}}},\"trace_gate\":{{\"passed\":{},\"checked\":{}}},\"eval\":{},\"helper_stage_feedback_by_role\":{},\"helper_stage_hygiene_by_role\":{},\"helper_stage_contract_by_role\":{},\"helper_stage_repair_status_report_v1\":{},\"test_gate\":{},\"remote_chain\":{},\"model_pool_manifest\":{},\"model_pool\":{},\"model_pool_route\":{},\"model_pool_alignment\":{},\"model_pool_budget_fairness_report_v1\":{},\"worker_window_replacement_report_v1\":{},\"clean_room_batch_status_report_v1\":{},\"clean_room_handoff_report_v1\":{},\"strict_report_gate\":{},\"continuation_gate_report_v1\":{},\"ledger_gate_report_v1\":{},\"adapter_closure_bundle_report_v1\":{},\"last\":{},\"recent_failures\":{},\"report_gate\":{{\"passed\":{},\"failures\":{}}}}}",
        summary.total,
        summary.unique_rounds,
        summary.duplicate_rounds,
        summary.non_monotonic_rounds,
        summary.missing_rounds,
        summary.round_gaps,
        summary.success,
        summary.failure,
        summary.stream_truncation_failures,
        summary.missing_final_failures,
        summary.runtime_response_failures,
        repeated_answer_json(summary.recent_repeated_successful_answer.as_ref()),
        string_array_json(&summary.completed_change_requests),
        string_array_json(&completed_change_request_blocked_topics(
            &summary.completed_change_requests
        )),
        string_array_json(&summary.invalid_change_requests),
        string_array_json(&invalid_change_request_blocked_topics(
            &summary.invalid_change_requests
        )),
        percent(summary.success, summary.total),
        summary.runtime_tokens,
        option_average_json(summary.runtime_tokens, summary.runtime_token_items),
        summary.elapsed_ms,
        option_average_json(summary.elapsed_ms, summary.elapsed_items),
        summary.round_wall_elapsed_ms,
        option_average_json(
            summary.round_wall_elapsed_ms,
            summary.round_wall_elapsed_items
        ),
        summary.feedback_applied,
        option_average_json(summary.feedback_applied, summary.feedback_items),
        summary.rust_check_passed,
        summary.rust_check_checked,
        summary.rust_check_feedback_applied,
        option_average_json(
            summary.rust_check_feedback_applied,
            summary.rust_check_feedback_items
        ),
        summary.validation_passed,
        summary.validation_checked,
        validation_command_coverage_report_json(&summary.validation_command_coverage_evidence),
        summary.self_improve_passed,
        summary.self_improve_checked,
        self_improve_proposal_artifact::option_artifact_json(self_improve_proposal_artifact),
        self_improve_proposal_artifact::option_acceptance_summary_json(
            self_improve_proposal_artifact
        ),
        self_improve_proposal_artifact::option_action_assignment_report_json(
            self_improve_proposal_artifact
        ),
        self_improve_proposal_artifact::option_action_closure_report_json(
            self_improve_proposal_artifact
        ),
        self_improve_proposal_artifact::option_memory_admission_readiness_report_json(
            self_improve_proposal_artifact
        ),
        self_improve_proposal_artifact::option_memory_admission_request_report_json(
            self_improve_proposal_artifact
        ),
        self_improve_proposal_artifact::option_memory_admission_decision_report_json(
            self_improve_proposal_artifact
        ),
        self_improve_proposal_artifact::option_memory_admission_writer_plan_report_json(
            self_improve_proposal_artifact
        ),
        self_improve_proposal_artifact::option_memory_admission_writer_dry_run_report_json(
            self_improve_proposal_artifact
        ),
        self_improve_proposal_artifact::option_memory_admission_writer_dry_run_receipt_report_json(
            self_improve_proposal_artifact
        ),
        self_improve_proposal_artifact::option_memory_admission_commit_record_stage_report_json(
            self_improve_proposal_artifact
        ),
        self_improve_proposal_artifact::option_memory_admission_commit_approval_request_report_json(
            self_improve_proposal_artifact
        ),
        self_improve_proposal_artifact::option_memory_admission_commit_approval_decision_report_json(
            self_improve_proposal_artifact
        ),
        self_improve_proposal_artifact::option_memory_admission_commit_approval_review_packet_report_json(
            self_improve_proposal_artifact
        ),
        summary.state_gate_passed,
        summary.state_gate_checked,
        summary.trace_gate_passed,
        summary.trace_gate_checked,
        eval_summary_json(summary),
        report_helper_stage_feedback_by_role_json(&summary.helper_stage_feedback_by_role),
        helper_stage_hygiene_map_json(&summary.helper_stage_hygiene_by_role),
        report_helper_stage_contract_map_json(&summary.helper_stage_contract_by_role),
        helper_stage_repair::status_json(&helper_stage_repair_status(
            summary,
            required_latest_helper_stage_roles
        )),
        test_gate_summary_json(&summary.test_gate),
        remote_chain::option_status_json(remote_chain_status),
        pool_artifacts::option_manifest_json(pool_manifest),
        pool_artifacts::option_status_json(pool_status),
        pool_artifacts::option_route_json(pool_route),
        pool_artifacts::option_alignment_json(pool_alignment.as_ref()),
        pool_artifacts::option_budget_fairness_json(pool_budget_fairness),
        worker_window_status::option_status_json(worker_window_status),
        clean_room_batch_status::option_status_json(clean_room_batch_status),
        clean_room_handoff::option_status_json(clean_room_handoff),
        strict_report_gate_json(strict_gate_failures),
        continuation_gate_report_json(
            summary,
            pool_budget_fairness,
            strict_gate_failures,
            continuation_gate_failures
        ),
        ledger_gate_report_json(summary, ledger_gate_failures),
        adapter_closure_bundle_report_json(
            summary,
            ledger_gate_failures,
            strict_gate_failures,
            continuation_gate_failures,
            gate_failures
        ),
        option_record_json(summary.last.as_ref()),
        record_array_json(&summary.recent_failures),
        gate_failures.is_empty(),
        string_array_json(gate_failures)
    )
}

fn pool_alignment_summary(
    pool_manifest: Option<&PoolManifestSummary>,
    pool_status: Option<&PoolStatusSummary>,
    pool_route: Option<&PoolRouteSummary>,
) -> Option<pool_artifacts::PoolAlignmentSummary> {
    if pool_manifest.is_none() && pool_status.is_none() && pool_route.is_none() {
        return None;
    }
    let routes = pool_route.cloned().into_iter().collect::<Vec<_>>();
    Some(pool_artifacts::alignment_summary(
        pool_manifest,
        pool_status,
        &routes,
    ))
}

fn option_record_json(record: Option<&ReportRecord>) -> String {
    record.map(record_json).unwrap_or_else(|| "null".to_owned())
}

fn record_array_json(records: &[ReportRecord]) -> String {
    let items = records
        .iter()
        .map(record_json)
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

fn record_json(record: &ReportRecord) -> String {
    let helper_stage_feedback_by_role = record.helper_stage_feedback_by_role();
    let helper_stage_contract_by_role =
        helper_stage_contract_summaries_by_role(&helper_stage_feedback_by_role);
    format!(
        "{{\"round\":{},\"case\":{},\"success\":{},\"error\":{},\"runtime_tokens\":{},\"runtime_model\":{},\"answer\":{},\"elapsed_ms\":{},\"round_wall_elapsed_ms\":{},\"feedback_applied\":{},\"rust_check_checked\":{},\"rust_check_passed\":{},\"rust_check_feedback_applied\":{},\"validation_checked\":{},\"validation_passed\":{},\"validation_command_source\":{},\"validation_command_safety\":{},\"validation_command_preview\":{},\"validation_phase\":{},\"validation_status_code\":{},\"validation_elapsed_ms\":{},\"validation_stdout_tail\":{},\"validation_stderr_tail\":{},\"self_improve_passed\":{},\"state_gate_checked\":{},\"state_gate_passed\":{},\"trace_gate_checked\":{},\"trace_gate_passed\":{},\"allocation_evidence\":{},\"helper_stage_feedback\":{},\"helper_stage_feedback_by_role\":{},\"helper_stage_contract_by_role\":{},\"eval\":{}}}",
        option_u64_json(record.round),
        option_str_json(record.case_name.as_deref()),
        record.success,
        option_str_json(record.error.as_deref()),
        option_u64_json(record.runtime_tokens),
        option_str_json(record.runtime_model.as_deref()),
        option_str_json(record.answer.as_deref()),
        option_u64_json(record.elapsed_ms),
        option_u64_json(record.round_wall_elapsed_ms()),
        option_u64_json(record.feedback_applied),
        option_bool_json(record.rust_check_checked),
        option_bool_json(record.rust_check_passed),
        option_u64_json(record.rust_check_feedback_applied),
        option_bool_json(record.validation_checked),
        option_bool_json(record.validation_passed),
        option_str_json(record.validation_command_source.as_deref()),
        option_str_json(record.validation_command_safety.as_deref()),
        option_str_json(record.validation_command_preview.as_deref()),
        option_str_json(record.validation_phase.as_deref()),
        option_i32_json(record.validation_status_code),
        option_u64_json(record.validation_elapsed_ms),
        option_str_json(record.validation_stdout_tail.as_deref()),
        option_str_json(record.validation_stderr_tail.as_deref()),
        option_bool_json(record.self_improve_passed),
        option_bool_json(record.state_gate_checked),
        option_bool_json(record.state_gate_passed),
        option_bool_json(record.trace_gate_checked),
        option_bool_json(record.trace_gate_passed),
        json_string_array(&record.allocation_evidence),
        report_helper_stage_feedback_items_json(&record.helper_stage_feedback()),
        report_helper_stage_feedback_by_role_json(&helper_stage_feedback_by_role),
        report_helper_stage_contract_map_json(&helper_stage_contract_by_role),
        option_json_object(record.eval_json.as_deref())
    )
}

fn eval_summary_json(summary: &ReportSummary) -> String {
    format!(
        "{{\"records\":{},\"report_only_records\":{},\"failure_kinds\":{}}}",
        summary.eval_records,
        summary.eval_report_only_records,
        string_usize_map_json(&summary.eval_failure_kinds)
    )
}

fn repeated_answer_json(summary: Option<&RepeatedAnswerSummary>) -> String {
    match summary {
        Some(summary) => format!(
            "{{\"count\":{},\"window_records\":{},\"preview\":{}}}",
            summary.count,
            summary.window_records,
            json_string(&summary.preview)
        ),
        None => "null".to_owned(),
    }
}

fn test_gate_summary_json(summary: &TestGateSummary) -> String {
    format!(
        "{{\"latest_verdict\":{},\"latest_validation_command\":{},\"latest_validation_command_safety\":{},\"latest_failure_kind\":{},\"latest_fields\":{}}}",
        option_str_json(summary.latest_verdict.as_deref()),
        option_str_json(summary.latest_validation_command.as_deref()),
        json_string(&summary.latest_validation_command_safety),
        option_str_json(summary.latest_failure_kind.as_deref()),
        string_string_map_json(&summary.latest_fields)
    )
}

fn validation_command_coverage_report_json(evidence: &ValidationCommandCoverageEvidence) -> String {
    let strict_coverage_requested = evidence.strict_coverage_is_requested();
    let coverage_evidence_present = evidence.coverage_tooling_or_report_evidence_present();
    let coverage_blocked = validation_command_coverage_is_blocked(evidence);
    let coverage_failure_kind = if coverage_blocked {
        "validation_command_coverage"
    } else {
        "none"
    };
    let failure_reasons = if coverage_blocked {
        vec![
            "strict coverage requested without coverage tooling or coverage report evidence"
                .to_owned(),
        ]
    } else {
        Vec::new()
    };

    format!(
        "{{\"schema\":\"validation_command_coverage_report_v1\",\"validation_command\":{{\"strict_coverage_requested\":{},\"coverage_tooling_evidence\":{},\"coverage_report_evidence\":{},\"coverage_tooling_or_report_evidence_present\":{},\"coverage_blocked\":{},\"coverage_failure_kind\":{},\"model_quality_failure_counted\":false,\"failure_reasons\":{},\"allow_next_round\":{}}}}}",
        strict_coverage_requested,
        json_string_array(&evidence.coverage_tooling_evidence),
        json_string_array(&evidence.coverage_report_evidence),
        coverage_evidence_present,
        coverage_blocked,
        json_string(coverage_failure_kind),
        json_string_array(&failure_reasons),
        !coverage_blocked
    )
}

fn eval_failure_kinds_text(values: &BTreeMap<String, usize>) -> String {
    if values.is_empty() {
        return "none".to_owned();
    }
    values
        .iter()
        .map(|(key, value)| format!("{key}:{value}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn string_string_map_text(values: &BTreeMap<String, String>) -> String {
    if values.is_empty() {
        return "none".to_owned();
    }
    values
        .iter()
        .map(|(key, value)| format!("{key}={}", preview_text(value, 120)))
        .collect::<Vec<_>>()
        .join(";")
}

fn helper_stage_feedback_by_role_text(values: &BTreeMap<String, Vec<String>>) -> String {
    if values.is_empty() {
        return "none".to_owned();
    }
    values
        .iter()
        .map(|(role, feedback)| format!("{role}:{}", feedback.join(" ; ")))
        .collect::<Vec<_>>()
        .join(" | ")
}

fn helper_stage_hygiene_by_role_text(
    values: &BTreeMap<String, Vec<HelperStageHygieneFinding>>,
) -> String {
    if values.is_empty() {
        return "none".to_owned();
    }
    values
        .iter()
        .map(|(role, findings)| {
            let finding_text = findings
                .iter()
                .map(|finding| {
                    format!(
                        "{}#{} preview={}",
                        finding.kind, finding.feedback_index, finding.preview
                    )
                })
                .collect::<Vec<_>>()
                .join(" ; ");
            format!("{role}:{finding_text}")
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

fn helper_stage_contract_by_role_text(
    values: &BTreeMap<String, HelperStageContractSummary>,
) -> String {
    if values.is_empty() {
        return "none".to_owned();
    }
    values
        .iter()
        .map(|(role, summary)| {
            format!(
                "{}:useful={} fields={} matched={} latest={}",
                role,
                summary.useful,
                helper_stage_contract_fields_text(summary),
                if summary.matched_markers.is_empty() {
                    "none".to_owned()
                } else {
                    summary.matched_markers.join(",")
                },
                summary.latest_preview.as_deref().unwrap_or("none")
            )
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

fn helper_stage_contract_fields_text(summary: &HelperStageContractSummary) -> String {
    let items = summary
        .expected_markers
        .iter()
        .filter_map(|field| {
            summary
                .fields
                .get(field)
                .map(|value| format!("{field}={}", preview_text(value, 120)))
        })
        .collect::<Vec<_>>();
    if items.is_empty() {
        "none".to_owned()
    } else {
        items.join(";")
    }
}

fn has_test_gate_summary(summary: &TestGateSummary) -> bool {
    summary.latest_verdict.is_some()
        || summary.latest_validation_command.is_some()
        || summary.latest_validation_command_safety != "missing"
        || summary.latest_failure_kind.is_some()
        || !summary.latest_fields.is_empty()
}

fn test_gate_context_text(summary: &TestGateSummary) -> String {
    format!(
        "verdict:{} validation_command:{} validation_command_safety:{} failure_kind:{} fields:{}",
        summary.latest_verdict.as_deref().unwrap_or("unknown"),
        summary
            .latest_validation_command
            .as_deref()
            .unwrap_or("none"),
        summary.latest_validation_command_safety,
        summary.latest_failure_kind.as_deref().unwrap_or("none"),
        string_string_map_text(&summary.latest_fields)
    )
}

fn string_usize_map_json(values: &BTreeMap<String, usize>) -> String {
    let items = values
        .iter()
        .map(|(key, value)| format!("{}:{}", json_string(key), value))
        .collect::<Vec<_>>()
        .join(",");
    format!("{{{items}}}")
}

fn string_vec_map_json(values: &BTreeMap<String, Vec<String>>) -> String {
    let items = values
        .iter()
        .map(|(key, value)| format!("{}:{}", json_string(key), json_string_array(value)))
        .collect::<Vec<_>>()
        .join(",");
    format!("{{{items}}}")
}

fn helper_stage_hygiene_map_json(
    values: &BTreeMap<String, Vec<HelperStageHygieneFinding>>,
) -> String {
    let items = values
        .iter()
        .map(|(role, findings)| {
            let finding_items = findings
                .iter()
                .map(helper_stage_hygiene_finding_json)
                .collect::<Vec<_>>()
                .join(",");
            format!("{}:[{}]", json_string(role), finding_items)
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("{{{items}}}")
}

fn helper_stage_hygiene_finding_json(finding: &HelperStageHygieneFinding) -> String {
    format!(
        "{{\"role\":{},\"feedback_index\":{},\"kind\":{},\"preview\":{}}}",
        json_string(&finding.role),
        finding.feedback_index,
        json_string(&finding.kind),
        json_string(&finding.preview)
    )
}

fn report_helper_stage_feedback_items_json(values: &[String]) -> String {
    let filtered = values
        .iter()
        .filter(|value| !helper_feedback_mentions_generic_noop_proposal(value))
        .cloned()
        .collect::<Vec<_>>();
    json_string_array(&filtered)
}

fn report_helper_stage_feedback_by_role_json(values: &BTreeMap<String, Vec<String>>) -> String {
    string_vec_map_json(&filtered_report_helper_stage_feedback_by_role(values))
}

fn report_helper_stage_contract_map_json(
    values: &BTreeMap<String, HelperStageContractSummary>,
) -> String {
    helper_stage_contract_map_json(&filtered_report_helper_stage_contract_by_role(values))
}

fn filtered_report_helper_stage_feedback_by_role(
    values: &BTreeMap<String, Vec<String>>,
) -> BTreeMap<String, Vec<String>> {
    values
        .iter()
        .filter_map(|(role, feedback)| {
            let role_feedback = feedback
                .iter()
                .filter(|value| !helper_feedback_mentions_generic_noop_proposal(value))
                .cloned()
                .collect::<Vec<_>>();
            (!role_feedback.is_empty()).then(|| (role.clone(), role_feedback))
        })
        .collect()
}

fn filtered_report_helper_stage_contract_by_role(
    values: &BTreeMap<String, HelperStageContractSummary>,
) -> BTreeMap<String, HelperStageContractSummary> {
    values
        .iter()
        .filter(|(_role, summary)| !helper_stage_contract_mentions_generic_noop_proposal(summary))
        .map(|(role, summary)| (role.clone(), summary.clone()))
        .collect()
}

fn helper_stage_contract_mentions_generic_noop_proposal(
    summary: &HelperStageContractSummary,
) -> bool {
    summary
        .latest_preview
        .as_deref()
        .is_some_and(helper_feedback_mentions_generic_noop_proposal)
        || summary
            .fields
            .values()
            .any(|value| helper_feedback_mentions_generic_noop_proposal(value))
}

fn helper_stage_contract_map_json(values: &BTreeMap<String, HelperStageContractSummary>) -> String {
    let items = values
        .iter()
        .map(|(role, summary)| {
            format!(
                "{}:{{\"useful\":{},\"latest_preview\":{},\"fields\":{},\"matched_markers\":{},\"expected_markers\":{}}}",
                json_string(role),
                summary.useful,
                option_str_json(summary.latest_preview.as_deref()),
                string_string_map_json(&summary.fields),
                json_string_array(&summary.matched_markers),
                json_string_array(&summary.expected_markers)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("{{{items}}}")
}

fn string_string_map_json(values: &BTreeMap<String, String>) -> String {
    let items = values
        .iter()
        .map(|(key, value)| format!("{}:{}", json_string(key), json_string(value)))
        .collect::<Vec<_>>()
        .join(",");
    format!("{{{items}}}")
}

fn string_array_json(values: &[String]) -> String {
    let items = values
        .iter()
        .map(|value| json_string(value))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

fn option_average_json(total: u64, count: usize) -> String {
    if count == 0 {
        "null".to_owned()
    } else {
        (total / count as u64).to_string()
    }
}

fn option_u64_json(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

fn option_i32_json(value: Option<i32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

fn option_bool_json(value: Option<bool>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

fn option_str_json(value: Option<&str>) -> String {
    value.map(json_string).unwrap_or_else(|| "null".to_owned())
}

fn option_json_object(value: Option<&str>) -> String {
    value.unwrap_or("null").to_owned()
}

fn percent(value: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        value as f64 * 100.0 / total as f64
    }
}

fn average_text(total: u64, count: usize) -> String {
    if count == 0 {
        "?".to_owned()
    } else {
        (total / count as u64).to_string()
    }
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

#[cfg(test)]
mod tests {
    use crate::pool_artifacts;

    use super::*;

    fn assert_contains_in_order(text: &str, fragments: &[&str]) {
        let mut offset = 0usize;
        for fragment in fragments {
            let Some(relative_index) = text[offset..].find(fragment) else {
                panic!("missing JSON contract fragment after byte {offset}: {fragment}");
            };
            offset += relative_index + fragment.len();
        }
    }

    fn assert_occurrences(text: &str, fragment: &str, expected: usize) {
        let actual = text.match_indices(fragment).count();
        assert_eq!(
            actual, expected,
            "JSON contract fragment occurrence mismatch for {fragment}"
        );
    }

    #[test]
    fn summarizes_structured_records() {
        let text = "{\"round\":1,\"case\":\"a\",\"success\":true,\"runtime_tokens\":10,\"runtime_model\":\"google/gemma\",\"answer\":\"ok\",\"elapsed_ms\":100,\"feedback_applied\":2,\"rust_check_checked\":true,\"rust_check_passed\":true,\"rust_check_feedback_applied\":1,\"validation_checked\":true,\"validation_passed\":true,\"self_improve_passed\":true,\"state_gate_checked\":false,\"state_gate_passed\":true,\"trace_gate_checked\":false,\"trace_gate_passed\":true}\n\
{\"round\":2,\"case\":\"b\",\"success\":false,\"error\":\"bad\",\"runtime_tokens\":20,\"elapsed_ms\":200,\"feedback_applied\":0,\"self_improve_passed\":false}\n";

        let summary = summarize_ledger(text);

        assert_eq!(summary.total, 2);
        assert_eq!(summary.unique_rounds, 2);
        assert_eq!(summary.duplicate_rounds, 0);
        assert_eq!(summary.non_monotonic_rounds, 0);
        assert_eq!(summary.missing_rounds, 0);
        assert_eq!(summary.round_gaps, 0);
        assert_eq!(summary.success, 1);
        assert_eq!(summary.failure, 1);
        assert_eq!(summary.runtime_tokens, 30);
        assert_eq!(summary.feedback_applied, 2);
        assert_eq!(summary.rust_check_checked, 1);
        assert_eq!(summary.rust_check_passed, 1);
        assert_eq!(summary.rust_check_feedback_applied, 1);
        assert_eq!(summary.validation_checked, 1);
        assert_eq!(summary.validation_passed, 1);
        assert_eq!(summary.runtime_response_failures, 0);
        assert_eq!(summary.self_improve_passed, 1);
        assert_eq!(summary.recent_failures[0].error.as_deref(), Some("bad"));
    }

    #[test]
    fn falls_back_to_final_preview_fields() {
        let text = "{\"round\":1,\"case\":\"legacy\",\"success\":true,\"final_preview\":\"{\\\"ok\\\":true,\\\"business_cycle\\\":{\\\"passed\\\":true,\\\"feedback_applied\\\":4,\\\"self_improve_passed\\\":true},\\\"generate\\\":{\\\"runtime_model\\\":\\\"google/gemma\\\",\\\"runtime_token_count\\\":33,\\\"elapsed_ms\\\":44}}\"}\n";

        let summary = summarize_ledger(text);

        assert_eq!(summary.total, 1);
        assert_eq!(summary.runtime_tokens, 33);
        assert_eq!(summary.elapsed_ms, 44);
        assert_eq!(summary.feedback_applied, 4);
        assert_eq!(summary.runtime_response_failures, 0);
        assert_eq!(summary.self_improve_passed, 1);
    }

    #[test]
    fn summarizes_report_only_eval_from_ledger_artifact() {
        let text = "{\"round\":1,\"case\":\"eval\",\"success\":true,\"feedback_applied\":2,\"eval\":{\"report_only\":true,\"failure_kind\":\"chain_not_ready\",\"backend_8686_reachable\":false}}\n";

        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            ..Config::default()
        };
        let gate_failures = report_gate_failures(&summary, &config, None, None);
        let context = prompt_context_text(&summary);
        let json = report_json(&summary, None, None, None, &gate_failures);

        assert_eq!(summary.eval_records, 1);
        assert_eq!(summary.eval_report_only_records, 1);
        assert_eq!(
            summary.eval_failure_kinds.get("chain_not_ready").copied(),
            Some(1)
        );
        assert!(gate_failures.is_empty(), "{gate_failures:?}");
        assert!(context.contains("eval_report_only=records:1 report_only:1"));
        assert!(context.contains("failure_kinds:chain_not_ready:1"));
        assert!(json.contains("\"eval\":{\"records\":1,\"report_only_records\":1"));
        assert!(json.contains("\"failure_kinds\":{\"chain_not_ready\":1}"));
        assert!(json.contains("\"backend_8686_reachable\":false"));
        assert!(json.contains("\"report_gate\":{\"passed\":true"));
    }

    #[test]
    fn falls_back_to_final_preview_eval() {
        let text = "{\"round\":1,\"case\":\"legacy-eval\",\"success\":true,\"feedback_applied\":2,\"final_preview\":\"{\\\"ok\\\":true,\\\"eval\\\":{\\\"report_only\\\":true,\\\"failure_kind\\\":\\\"model_unavailable\\\"}}\"}\n";

        let summary = summarize_ledger(text);

        assert_eq!(summary.eval_records, 1);
        assert_eq!(summary.eval_report_only_records, 1);
        assert_eq!(
            summary.eval_failure_kinds.get("model_unavailable").copied(),
            Some(1)
        );
    }

    #[test]
    fn remote_chain_summary_is_reported_as_read_only_context() {
        let remote = remote_chain::parse_status(
            "{\"contract_version\":\"smartsteam.remote-gemma-chain.status.v1\",\"readiness\":{\"ready\":false,\"model_api\":false,\"backend\":true,\"web_lab\":true},\"model_pool\":{\"available\":false,\"worker_count\":2,\"healthy_worker_count\":1,\"min_context_tokens\":262144,\"capacity\":{\"recommendation\":\"restore_quality_gate_first\"}},\"next_step\":\"start-remote\"}\n",
        );
        let ledger = "{\"round\":1,\"case\":\"ok\",\"success\":true,\"feedback_applied\":2}\n";
        let summary = summarize_ledger(ledger);
        let json = report_json_with_remote_chain(
            &summary,
            Some(&remote),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
        );
        let context = remote_chain::context_text(&remote);

        assert_eq!(remote.ready, Some(false));
        assert_eq!(remote.model_api, Some(false));
        assert_eq!(remote.backend, Some(true));
        assert!(context.contains("ready:false"));
        assert!(context.contains("workers:1/2"));
        assert!(json.contains("\"remote_chain\":{\"contract_version\""));
        assert!(json.contains("\"ready\":false"));
        assert!(json.contains("\"capacity_recommendation\":\"restore_quality_gate_first\""));
    }

    #[test]
    fn report_gate_blocks_remote_chain_when_requested() {
        let path = std::env::temp_dir().join(format!(
            "smartsteam-report-remote-chain-block-{}.json",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        fs::write(
            &path,
            "{\"readiness\":{\"ready\":false,\"model_api\":false,\"backend\":true,\"web_lab\":true},\"model_pool\":{\"available\":false,\"capacity\":{\"recommendation\":\"restore_quality_gate_first\"}},\"next_step\":\"start-remote\"}\n",
        )
        .unwrap();
        let ledger = "{\"round\":1,\"case\":\"ok\",\"success\":true,\"feedback_applied\":2}\n";
        let summary = summarize_ledger(ledger);
        let config = Config {
            report_gate: true,
            remote_chain_status_json_path: Some(path.clone()),
            remote_chain_gate: true,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("remote chain not ready")),
            "{failures:?}"
        );
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("restore_quality_gate_first")),
            "{failures:?}"
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn report_gate_blocks_model_pool_dependency_precheck_failures() {
        let dir = std::env::temp_dir().join(format!(
            "smartsteam-report-pool-dependency-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let manifest_path = dir.join("pool-manifest.json");
        let status_path = dir.join("pool-status.json");
        let route_path = dir.join("pool-route-index.json");
        fs::write(
            &manifest_path,
            "{\"capacity_policy\":{\"policy\":\"one_quality_plus_small_helpers\",\"max_quality_12b_workers\":1},\"workers\":[{\"role\":\"quality\"},{\"role\":\"summary\"},{\"role\":\"index\"}]}\n",
        )
        .unwrap();
        fs::write(
            &status_path,
            "{\"workers\":[{\"role\":\"quality\",\"tcp_reachable\":true,\"health_ok\":true},{\"role\":\"summary\",\"tcp_reachable\":true,\"health_ok\":true},{\"role\":\"index\",\"tcp_reachable\":true,\"health_ok\":true}]}\n",
        )
        .unwrap();
        fs::write(
            &route_path,
            "{\"task_kind\":\"index\",\"route_allowed\":false,\"route_block_reason\":\"dependency_precheck_blocked:missing_required_roles\",\"dependency_precheck\":{\"strategy\":\"role_dependency_graph_v1\",\"checked\":true,\"requested_role\":\"index\",\"allow_dispatch\":false,\"reason\":\"missing_required_roles\",\"required_roles\":[\"summary\",\"router\"],\"completed_roles\":[\"quality\",\"summary\"],\"missing_roles\":[\"router\"]},\"selected_role\":null,\"candidate_workers\":[{\"role\":\"index\",\"health_ok\":true,\"role_ready\":true}]}\n",
        )
        .unwrap();
        let ledger = "{\"round\":1,\"case\":\"ok\",\"success\":true,\"feedback_applied\":2}\n";
        let summary = summarize_ledger(ledger);
        let pool_manifest = pool_artifacts::load_manifest(Some(&manifest_path))
            .unwrap()
            .unwrap();
        let pool_status = pool_artifacts::load_status(Some(&status_path))
            .unwrap()
            .unwrap();
        let pool_route = pool_artifacts::load_route(Some(&route_path))
            .unwrap()
            .unwrap();
        let config = Config {
            report_gate: true,
            pool_alignment_gate: true,
            pool_manifest_json_path: Some(manifest_path),
            pool_status_json_path: Some(status_path),
            pool_route_json_path: Some(route_path),
            pool_route_task_kind: "index".to_owned(),
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, Some(&pool_status), None);
        let json = report_json_with_remote_chain(
            &summary,
            None,
            Some(&pool_manifest),
            Some(&pool_status),
            Some(&pool_route),
            None,
            None,
            None,
            None,
            None,
            &[],
            &failures,
            &failures,
            &failures,
        );

        assert!(
            failures.iter().any(|failure| failure.contains(
                "route_dependency_failures=index:index:missing_required_roles:missing=router"
            )),
            "{failures:?}"
        );
        assert!(
            json.contains("\"dependency_precheck\":{\"strategy\":\"role_dependency_graph_v1\"")
        );
        assert!(json.contains(
            "\"route_dependency_failures\":[\"index:index:missing_required_roles:missing=router\"]"
        ));
        assert!(json.contains("\"report_gate\":{\"passed\":false"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn pool_manifest_summary_is_reported_as_read_only_context() {
        let manifest = pool_artifacts::parse_manifest(
            "{\"contract_version\":\"gemma-chain.v1\",\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"manifest_kind\":\"rust-norion.model-pool\",\"capacity_policy\":{\"policy\":\"one_quality_plus_small_helpers\",\"target_host\":\"apple_silicon\",\"avoid_extra_12b\":true,\"max_quality_12b_workers\":1,\"helper_roles\":[\"summary\",\"router\",\"review\",\"index\",\"test-gate\"],\"recommended_launch_order\":[\"quality\",\"summary\",\"router\",\"review\",\"index\",\"test-gate\"]},\"workers\":[{\"role\":\"quality\",\"port\":8686,\"default_context_tokens\":262144,\"default_max_tokens\":262144,\"runtime_backend\":\"llama.cpp\",\"runtime_device\":\"metal\",\"runtime_accelerator\":\"metal\",\"gpu_layers\":99},{\"role\":\"summary\",\"port\":8687,\"default_context_tokens\":8192,\"default_max_tokens\":768,\"runtime_backend\":\"llama.cpp\",\"runtime_device\":\"metal\",\"runtime_accelerator\":\"metal\",\"gpu_layers\":80}]}\n",
        );
        let ledger = "{\"round\":1,\"case\":\"ok\",\"success\":true,\"feedback_applied\":2}\n";
        let summary = summarize_ledger(ledger);
        let json = report_json_with_remote_chain(
            &summary,
            None,
            Some(&manifest),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
        );
        let context = pool_artifacts::manifest_context_text(&manifest);

        assert_eq!(manifest.worker_count, 2);
        assert!(context.contains("policy:one_quality_plus_small_helpers"));
        assert!(context.contains("avoid_extra_12b:true"));
        assert!(context.contains("max_quality_12b_workers:1"));
        assert!(
            context
                .contains("recommended_launch_order:quality,summary,router,review,index,test-gate")
        );
        assert!(context.contains("quality@8686"));
        assert!(context.contains("summary@8687"));
        assert!(json.contains("\"model_pool_manifest\":{\"contract_version\":\"gemma-chain.v1\""));
        assert!(json.contains("\"manifest_kind\":\"rust-norion.model-pool\""));
        assert!(json.contains("\"avoid_extra_12b\":true"));
        assert!(json.contains("\"runtime_accelerator\":\"metal\""));
    }

    #[test]
    fn pool_status_summary_is_reported_as_read_only_context() {
        let text = "{\"summary\":\"pool\",\"launch_allowed\":false,\"launch_block_reason\":\"quality_worker_down\",\"chain_classification\":\"quality_worker_down\",\"min_context_tokens\":262144,\"workers\":[{\"port\":8686,\"role\":\"quality\",\"tcp_reachable\":false,\"health_ok\":false},{\"port\":8687,\"role\":\"summary\",\"tcp_reachable\":true,\"health_ok\":true}]}\n";
        let pool = pool_artifacts::parse_status(text);
        let ledger = "{\"round\":1,\"case\":\"ok\",\"success\":true,\"feedback_applied\":2}\n";
        let summary = summarize_ledger(ledger);
        let config = Config {
            report_gate: true,
            ..Config::default()
        };
        let gate_failures = report_gate_failures(&summary, &config, None, None);
        let json = report_json(&summary, Some(&pool), None, None, &gate_failures);
        let context = pool_artifacts::status_context_text(&pool);

        assert_eq!(pool.launch_allowed, Some(false));
        assert_eq!(
            pool.chain_classification.as_deref(),
            Some("quality_worker_down")
        );
        assert_eq!(pool.worker_count, 2);
        assert_eq!(pool.reachable_workers, 1);
        assert_eq!(pool.healthy_workers, 1);
        assert_eq!(pool.roles.len(), 2);
        assert_eq!(pool.roles[0].role, "quality");
        assert_eq!(pool.roles[0].tcp_reachable, false);
        assert_eq!(pool.roles[1].role, "summary");
        assert_eq!(pool.roles[1].health_ok, true);
        assert!(gate_failures.is_empty(), "{gate_failures:?}");
        assert!(context.contains("launch_allowed:false"));
        assert!(context.contains("workers_reachable:1/2"));
        assert!(context.contains("roles:quality:unreachable,summary:healthy"));
        assert!(context.contains("available_roles:summary"));
        assert!(context.contains("blocked_roles:quality"));
        assert!(json.contains("\"model_pool\":{\"launch_allowed\":false"));
        assert!(json.contains("\"reachable\":1"));
        assert!(json.contains("\"roles\":[{\"role\":\"quality\""));
        assert!(json.contains("\"status\":\"healthy\""));
    }

    #[test]
    fn pool_route_summary_is_reported_as_read_only_context() {
        let text = "{\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"task_kind\":\"review\",\"route_allowed\":false,\"route_block_reason\":\"model_pool_launch_blocked:quality_worker_down\",\"selected_role\":null,\"role_candidates\":[\"review\",\"quality\"],\"candidate_workers\":[{\"port\":8686,\"role\":\"quality\",\"health_ok\":false,\"role_ready\":false},{\"port\":8688,\"role\":\"review\",\"health_ok\":true,\"role_ready\":true}]}\n";
        let route = pool_artifacts::parse_route(text);
        let ledger = "{\"round\":1,\"case\":\"ok\",\"success\":true,\"feedback_applied\":2}\n";
        let summary = summarize_ledger(ledger);
        let gate_failures = report_gate_failures(&summary, &Config::default(), None, None);
        let json = report_json(&summary, None, Some(&route), None, &gate_failures);
        let context = pool_artifacts::route_context_text(&route);

        assert_eq!(route.task_kind.as_deref(), Some("review"));
        assert_eq!(route.route_allowed, Some(false));
        assert_eq!(route.role_candidates, vec!["review", "quality"]);
        assert_eq!(route.candidate_count, 2);
        assert_eq!(route.healthy_candidates, 1);
        assert_eq!(route.ready_candidates, 1);
        assert!(context.contains("task_kind:review"));
        assert!(context.contains("route_allowed:false"));
        assert!(context.contains("role_candidates:review,quality"));
        assert!(json.contains("\"model_pool_route\":{\"task_kind\":\"review\""));
        assert!(json.contains("\"selected_role\":null"));
        assert!(json.contains("\"ready\":1"));
    }

    #[test]
    fn pool_alignment_summary_is_reported_as_model_pool_context() {
        let manifest = pool_artifacts::parse_manifest(
            "{\"capacity_policy\":{\"policy\":\"one_quality_plus_small_helpers\",\"max_quality_12b_workers\":1,\"quality_role\":\"quality\",\"helper_roles\":[\"summary\",\"review\",\"index\",\"test-gate\"]},\"workers\":[{\"role\":\"quality\",\"port\":8686},{\"role\":\"summary\",\"port\":8687},{\"role\":\"review\",\"port\":8688}]}\n",
        );
        let pool = pool_artifacts::parse_status(
            "{\"workers\":[{\"role\":\"quality\",\"tcp_reachable\":true,\"health_ok\":true},{\"role\":\"summary\",\"tcp_reachable\":true,\"health_ok\":true},{\"role\":\"extra\",\"tcp_reachable\":true,\"health_ok\":true}]}\n",
        );
        let route = pool_artifacts::parse_route(
            "{\"task_kind\":\"review\",\"route_allowed\":false,\"route_block_reason\":\"worker_down\",\"selected_role\":null,\"candidate_workers\":[{\"role\":\"review\",\"health_ok\":false,\"role_ready\":false}]}\n",
        );
        let ledger = "{\"round\":1,\"case\":\"ok\",\"success\":true,\"feedback_applied\":2}\n";
        let summary = summarize_ledger(ledger);
        let json = report_json_with_remote_chain(
            &summary,
            None,
            Some(&manifest),
            Some(&pool),
            Some(&route),
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
        );

        assert!(json.contains("\"model_pool_alignment\":{\"alignment_ok\":false"));
        assert!(json.contains("\"manifest_roles\":[\"quality\",\"summary\",\"review\"]"));
        assert!(json.contains("\"status_roles\":[\"quality\",\"summary\",\"extra\"]"));
        assert!(json.contains("\"missing_status_roles\":[\"review\"]"));
        assert!(json.contains("\"unplanned_status_roles\":[\"extra\"]"));
        assert!(json.contains("\"route_blocked_or_failed\":[\"review\"]"));
    }

    #[test]
    fn pool_budget_fairness_summary_is_reported_as_additive_context() {
        let pool_budget = pool_artifacts::parse_budget_fairness(
            "{\"workers\":[{\"role\":\"summary\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":400,\"latency_ms\":100},{\"role\":\"review\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":300,\"latency_ms\":100},{\"role\":\"test-gate\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":300,\"latency_ms\":100}]}\n",
        );
        let ledger = "{\"round\":1,\"case\":\"ok\",\"success\":true,\"feedback_applied\":2}\n";
        let summary = summarize_ledger(ledger);
        let gate_failures =
            report_gate_failures(&summary, &Config::default(), None, Some(&pool_budget));
        let json = report_json(&summary, None, None, Some(&pool_budget), &gate_failures);
        let context = pool_artifacts::budget_fairness_context_text(&pool_budget);

        assert!(gate_failures.is_empty(), "{gate_failures:?}");
        assert!(pool_budget.allow_pool_expansion);
        assert!(context.contains("allow_pool_expansion:true"));
        assert!(json.contains("\"model_pool_budget_fairness_report_v1\""));
        assert!(json.contains("\"schema\":\"model_pool_budget_fairness_report_v1\""));
        assert!(json.contains("\"allow_pool_expansion\":true"));
        assert!(json.contains("\"role\":\"test-gate\""));
    }

    #[test]
    fn report_gate_blocks_pool_budget_fairness_failures() {
        let pool_budget = pool_artifacts::parse_budget_fairness(
            "{\"workers\":[{\"role\":\"summary\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":800,\"latency_ms\":100},{\"role\":\"review\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":100,\"latency_ms\":100},{\"role\":\"test-gate\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":100,\"latency_ms\":100,\"blocked_primary_12b\":true}]}\n",
        );
        let ledger = "{\"round\":1,\"case\":\"ok\",\"success\":true,\"feedback_applied\":2}\n";
        let summary = summarize_ledger(ledger);
        let config = Config {
            report_gate: true,
            ..Config::default()
        };
        let gate_failures = report_gate_failures(&summary, &config, None, Some(&pool_budget));
        let json = report_json(&summary, None, None, Some(&pool_budget), &gate_failures);

        assert!(pool_budget.budget_fairness_blocked);
        assert!(
            gate_failures
                .iter()
                .any(|failure| failure.contains("model pool budget fairness blocked expansion")),
            "{gate_failures:?}"
        );
        assert!(
            gate_failures
                .iter()
                .any(|failure| failure.contains("blocked primary 12B")),
            "{gate_failures:?}"
        );
        assert!(json.contains("\"report_gate\":{\"passed\":false"));
        assert!(json.contains("\"budget_fairness_blocked\":true"));
    }

    #[test]
    fn continuation_gate_allows_latest_healthy_round_despite_historical_strict_failures() {
        let pool_budget = pool_artifacts::parse_budget_fairness(
            "{\"workers\":[{\"role\":\"quality\",\"success\":true,\"feedback_applied\":0,\"runtime_tokens\":10,\"latency_ms\":100,\"default_max_tokens\":262144,\"configured_max_tokens\":64,\"effective_max_tokens\":64,\"max_tokens_clamped\":false,\"can_accept_low_priority_task\":false},{\"role\":\"summary\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":800,\"latency_ms\":100,\"default_max_tokens\":768,\"configured_max_tokens\":64,\"effective_max_tokens\":64,\"max_tokens_clamped\":false,\"can_accept_low_priority_task\":true},{\"role\":\"review\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":100,\"latency_ms\":100,\"default_max_tokens\":1024,\"configured_max_tokens\":64,\"effective_max_tokens\":64,\"max_tokens_clamped\":false,\"can_accept_low_priority_task\":true},{\"role\":\"test-gate\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":100,\"latency_ms\":100,\"default_max_tokens\":1024,\"configured_max_tokens\":64,\"effective_max_tokens\":64,\"max_tokens_clamped\":false,\"can_accept_low_priority_task\":true}]}\n",
        );
        let ledger = concat!(
            "{\"round\":1,\"case\":\"old-runtime\",\"success\":true,\"runtime_tokens\":0,\"runtime_model\":\"model\",\"feedback_applied\":1}\n",
            "{\"round\":2,\"case\":\"latest-ok\",\"success\":true,\"runtime_tokens\":64,\"runtime_model\":\"model\",\"feedback_applied\":4}\n"
        );
        let summary = summarize_ledger(ledger);
        let config = Config {
            report_gate: true,
            report_continuation_gate: true,
            require_pool_budget_policy: true,
            ..Config::default()
        };

        let ledger_failures = report_gate_threshold_failures(&summary, &config);
        let strict_failures = report_gate_failures(&summary, &config, None, Some(&pool_budget));
        let continuation_failures =
            report_gate_continuation_failures(&summary, &config, None, Some(&pool_budget));
        let json = report_json_with_remote_chain(
            &summary,
            None,
            None,
            None,
            None,
            Some(&pool_budget),
            None,
            None,
            None,
            None,
            &ledger_failures,
            &strict_failures,
            &continuation_failures,
            &continuation_failures,
        );

        assert!(pool_budget.budget_fairness_blocked);
        assert!(
            strict_failures
                .iter()
                .any(|failure| failure.contains("runtime response failures 1")),
            "{strict_failures:?}"
        );
        assert!(
            strict_failures
                .iter()
                .any(|failure| failure.contains("budget fairness blocked")),
            "{strict_failures:?}"
        );
        assert!(
            continuation_failures.is_empty(),
            "{continuation_failures:?}"
        );
        assert!(json.contains("\"strict_report_gate\":{\"passed\":false"));
        assert!(json.contains("\"continuation_gate_report_v1\""));
        assert!(json.contains("\"allow_unattended_continuation\":true"));
        assert!(json.contains("\"strict_report_gate_passed\":false"));
        assert!(json.contains("\"budget_fairness_blocked\":true"));
        assert!(json.contains("\"report_gate\":{\"passed\":true"));
    }

    #[test]
    fn continuation_gate_blocks_latest_runtime_response_failure() {
        let ledger = concat!(
            "{\"round\":1,\"case\":\"old-ok\",\"success\":true,\"runtime_tokens\":64,\"runtime_model\":\"model\",\"feedback_applied\":1}\n",
            "{\"round\":2,\"case\":\"latest-runtime\",\"success\":true,\"runtime_tokens\":0,\"runtime_model\":\"model\",\"feedback_applied\":1}\n"
        );
        let summary = summarize_ledger(ledger);
        let config = Config {
            report_continuation_gate: true,
            ..Config::default()
        };

        let failures = report_gate_continuation_failures(&summary, &config, None, None);

        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("latest round has runtime response failure")),
            "{failures:?}"
        );
    }

    #[test]
    fn report_gate_passes_required_pool_budget_policy_when_evidence_is_complete() {
        let pool_budget = pool_artifacts::parse_budget_fairness(
            "{\"workers\":[{\"role\":\"quality\",\"success\":true,\"feedback_applied\":0,\"runtime_tokens\":262144,\"latency_ms\":5000,\"default_max_tokens\":262144,\"configured_max_tokens\":262144,\"effective_max_tokens\":262144,\"max_tokens_clamped\":false,\"can_accept_low_priority_task\":false},{\"role\":\"summary\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":300,\"latency_ms\":120,\"default_max_tokens\":768,\"configured_max_tokens\":262144,\"effective_max_tokens\":768,\"max_tokens_clamped\":true,\"can_accept_low_priority_task\":true},{\"role\":\"review\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":250,\"latency_ms\":100,\"default_max_tokens\":1024,\"configured_max_tokens\":262144,\"effective_max_tokens\":1024,\"max_tokens_clamped\":true,\"can_accept_low_priority_task\":true},{\"role\":\"test-gate\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":150,\"latency_ms\":80,\"default_max_tokens\":1024,\"configured_max_tokens\":262144,\"effective_max_tokens\":1024,\"max_tokens_clamped\":true,\"can_accept_low_priority_task\":true}]}\n",
        );
        let ledger = "{\"round\":1,\"case\":\"ok\",\"success\":true,\"feedback_applied\":2}\n";
        let summary = summarize_ledger(ledger);
        let config = Config {
            report_gate: true,
            require_pool_budget_policy: true,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, Some(&pool_budget));

        assert!(failures.is_empty(), "{failures:?}");
        assert!(!pool_budget.budget_fairness_blocked);
    }

    #[test]
    fn report_gate_blocks_required_pool_budget_policy_when_summary_is_missing() {
        let ledger = "{\"round\":1,\"case\":\"ok\",\"success\":true,\"feedback_applied\":2}\n";
        let summary = summarize_ledger(ledger);
        let config = Config {
            report_gate: true,
            require_pool_budget_policy: true,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("budget fairness summary is missing")),
            "{failures:?}"
        );
    }

    #[test]
    fn report_gate_blocks_required_pool_budget_policy_without_quality_evidence() {
        let pool_budget = pool_artifacts::parse_budget_fairness(
            "{\"workers\":[{\"role\":\"summary\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":400,\"latency_ms\":100},{\"role\":\"review\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":300,\"latency_ms\":100},{\"role\":\"test-gate\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":300,\"latency_ms\":100}]}\n",
        );
        let ledger = "{\"round\":1,\"case\":\"ok\",\"success\":true,\"feedback_applied\":2}\n";
        let summary = summarize_ledger(ledger);
        let config = Config {
            report_gate: true,
            require_pool_budget_policy: true,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, Some(&pool_budget));

        assert!(!pool_budget.budget_fairness_blocked);
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("missing quality role evidence")),
            "{failures:?}"
        );
    }

    #[test]
    fn report_gate_blocks_required_pool_budget_policy_without_helper_clamp_evidence() {
        let pool_budget = pool_artifacts::parse_budget_fairness(
            "{\"workers\":[{\"role\":\"quality\",\"success\":true,\"feedback_applied\":0,\"runtime_tokens\":262144,\"latency_ms\":5000,\"default_max_tokens\":262144,\"configured_max_tokens\":262144,\"effective_max_tokens\":262144,\"max_tokens_clamped\":false,\"can_accept_low_priority_task\":false},{\"role\":\"summary\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":300,\"latency_ms\":120,\"default_max_tokens\":768,\"configured_max_tokens\":262144,\"effective_max_tokens\":768,\"max_tokens_clamped\":false,\"can_accept_low_priority_task\":true},{\"role\":\"review\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":250,\"latency_ms\":100,\"default_max_tokens\":1024,\"configured_max_tokens\":262144,\"max_tokens_clamped\":false,\"effective_max_tokens\":1024,\"can_accept_low_priority_task\":true},{\"role\":\"test-gate\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":150,\"latency_ms\":80,\"default_max_tokens\":1024,\"configured_max_tokens\":262144,\"effective_max_tokens\":1024,\"max_tokens_clamped\":false,\"can_accept_low_priority_task\":true}]}\n",
        );
        let ledger = "{\"round\":1,\"case\":\"ok\",\"success\":true,\"feedback_applied\":2}\n";
        let summary = summarize_ledger(ledger);
        let config = Config {
            report_gate: true,
            require_pool_budget_policy: true,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, Some(&pool_budget));

        assert!(pool_budget.budget_fairness_blocked);
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("missing clamped low-priority helper evidence")),
            "{failures:?}"
        );
    }

    #[test]
    fn report_gate_allows_low_budget_pool_policy_without_helper_clamp_evidence() {
        let pool_budget = pool_artifacts::parse_budget_fairness(
            "{\"workers\":[{\"role\":\"quality\",\"success\":true,\"feedback_applied\":0,\"runtime_tokens\":64,\"latency_ms\":5000,\"default_max_tokens\":262144,\"configured_max_tokens\":64,\"effective_max_tokens\":64,\"max_tokens_clamped\":false,\"can_accept_low_priority_task\":false},{\"role\":\"summary\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":10,\"latency_ms\":120,\"default_max_tokens\":768,\"configured_max_tokens\":64,\"effective_max_tokens\":64,\"max_tokens_clamped\":false,\"can_accept_low_priority_task\":true},{\"role\":\"review\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":10,\"latency_ms\":100,\"default_max_tokens\":1024,\"configured_max_tokens\":64,\"effective_max_tokens\":64,\"max_tokens_clamped\":false,\"can_accept_low_priority_task\":true},{\"role\":\"test-gate\",\"success\":true,\"feedback_applied\":1,\"runtime_tokens\":10,\"latency_ms\":80,\"default_max_tokens\":1024,\"configured_max_tokens\":64,\"effective_max_tokens\":64,\"max_tokens_clamped\":false,\"can_accept_low_priority_task\":true}]}\n",
        );
        let ledger = "{\"round\":1,\"case\":\"ok\",\"success\":true,\"feedback_applied\":2}\n";
        let summary = summarize_ledger(ledger);
        let config = Config {
            report_gate: true,
            require_pool_budget_policy: true,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, Some(&pool_budget));

        assert!(!pool_budget.budget_fairness_blocked);
        assert!(failures.is_empty(), "{failures:?}");
    }

    #[test]
    fn missing_pool_budget_fairness_summary_is_reported_as_null() {
        let ledger = "{\"round\":1,\"case\":\"ok\",\"success\":true,\"feedback_applied\":2}\n";
        let summary = summarize_ledger(ledger);
        let json = report_json(&summary, None, None, None, &[]);

        assert!(json.contains("\"model_pool_budget_fairness_report_v1\":null"));
    }

    #[test]
    fn summarizes_allocation_evidence_from_ledger_records() {
        let text = "{\"round\":1,\"case\":\"allocation\",\"success\":true,\"feedback_applied\":2,\"allocation_evidence\":[\"pool_route task_kind:review route_allowed:false\",\"pool_status launch_allowed:false\"]}\n";
        let summary = summarize_ledger(text);
        let context = prompt_context_text(&summary);
        let json = report_json(&summary, None, None, None, &[]);

        assert_eq!(summary.last.as_ref().unwrap().allocation_evidence.len(), 2);
        assert!(context.contains("last_allocation_evidence=pool_route task_kind:review"));
        assert!(json.contains(
            "\"allocation_evidence\":[\"pool_route task_kind:review route_allowed:false\""
        ));
    }

    #[test]
    fn summarizes_selected_context_evidence_from_allocation_records() {
        let text = "{\"round\":1,\"case\":\"allocation\",\"success\":true,\"feedback_applied\":2,\"allocation_evidence\":[\"pool_stage_route[test-gate] task_kind:test-gate route_allowed:true selected_context_required_tokens:2816 selected_context_buffer_tokens:2048 selected_context_sufficient:true selected_context_block_reason:none\",\"pool_stage_route[review] task_kind:review route_allowed:true selected_context_required_tokens:1024 selected_context_buffer_tokens:0 selected_context_sufficient:true selected_context_block_reason:none\"]}\n";
        let summary = summarize_ledger(text);
        let context = prompt_context_text(&summary);

        assert!(context.contains(
            "last_selected_context_evidence=test-gate required:2816 buffer:2048 sufficient:true reason:none"
        ));
        assert!(context.contains("review required:1024 buffer:0 sufficient:true reason:none"));
    }

    #[test]
    fn prompt_context_distinguishes_historical_from_recent_runtime_failures() {
        let text = "{\"round\":1,\"case\":\"old-failure\",\"success\":true,\"answer\":\"runtime backend error\",\"runtime_tokens\":0,\"runtime_model\":\"google/gemma\",\"feedback_applied\":0}\n\
{\"round\":2,\"case\":\"ok-2\",\"success\":true,\"runtime_tokens\":64,\"runtime_model\":\"google/gemma\",\"feedback_applied\":4}\n\
{\"round\":3,\"case\":\"ok-3\",\"success\":true,\"runtime_tokens\":64,\"runtime_model\":\"google/gemma\",\"feedback_applied\":4}\n\
{\"round\":4,\"case\":\"ok-4\",\"success\":true,\"runtime_tokens\":64,\"runtime_model\":\"google/gemma\",\"feedback_applied\":4}\n\
{\"round\":5,\"case\":\"ok-5\",\"success\":true,\"runtime_tokens\":64,\"runtime_model\":\"google/gemma\",\"feedback_applied\":4}\n\
{\"round\":6,\"case\":\"ok-6\",\"success\":true,\"runtime_tokens\":64,\"runtime_model\":\"google/gemma\",\"feedback_applied\":4}\n";
        let summary = summarize_ledger(text);
        let context = prompt_context_text(&summary);

        assert_eq!(summary.runtime_response_failures, 1);
        assert_eq!(summary.recent_failure_window_records, 5);
        assert_eq!(summary.recent_runtime_response_failures, 0);
        assert!(context.contains("runtime_response_failures:1"));
        assert!(context.contains(
            "recent_failure_window=records:5 stream_truncated:0 missing_final:0 runtime_response_failures:0"
        ));
    }

    #[test]
    fn prompt_context_omits_stale_failure_error_after_later_success() {
        let text = "{\"round\":1,\"case\":\"old-context-failure\",\"success\":false,\"error\":\"model_pool_launch_blocked:quality_context_window_too_small quality_context_required_tokens:262144\",\"feedback_applied\":0}\n\
{\"round\":2,\"case\":\"recovered\",\"success\":true,\"runtime_tokens\":64,\"runtime_model\":\"google/gemma\",\"feedback_applied\":4,\"allocation_evidence\":[\"pool_route task_kind:quality quality_context_tokens:65536 quality_context_required_tokens:65536 quality_context_sufficient:true selected_max_tokens:4096\"]}\n";
        let summary = summarize_ledger(text);
        let context = prompt_context_text(&summary);
        let json = report_json(&summary, None, None, None, &[]);

        assert_eq!(summary.failure, 1);
        assert_eq!(summary.success, 1);
        assert!(context.contains(
            "most_recent_failure=round 1 case old-context-failure status=stale_recovered latest_success_round=2 error_omitted=true"
        ));
        assert!(
            context
                .contains("next_advice_should_use_current_route_evidence_over_stale_failure:true")
        );
        assert!(
            context.contains("quality_context_required_tokens:65536"),
            "{context}"
        );
        assert!(
            !context.contains("quality_context_required_tokens:262144"),
            "{context}"
        );
        assert!(json.contains("quality_context_required_tokens:262144"));
    }

    #[test]
    fn prompt_context_omits_stale_helper_context_expansion_after_route_recovery() {
        let text = "{\"round\":1,\"case\":\"old-context-failure\",\"success\":false,\"error\":\"model_pool_launch_blocked:quality_context_window_too_small quality_context_required_tokens:262144\",\"feedback_applied\":0}\n\
{\"round\":2,\"case\":\"recovered\",\"success\":true,\"runtime_tokens\":64,\"runtime_model\":\"google/gemma\",\"feedback_applied\":4,\"allocation_evidence\":[\"pool_route task_kind:quality quality_context_tokens:65536 quality_context_required_tokens:65536 quality_context_sufficient:true selected_max_tokens:4096\"],\"helper_stage_feedback_by_role\":{\"review\":[\"task_kind=review preview=risk: stale route advice / change_request: Update the quality worker context_window from 65536 to 262144 / verification: confirm context_window 262144\"],\"summary\":[\"task_kind=summary preview=memory_update: keep current route evidence / next_context: prefer current pool_route / duplicate_guard: do not repeat stale context advice\"]}}\n";
        let summary = summarize_ledger(text);
        let context = prompt_context_text(&summary);
        let json = report_json(&summary, None, None, None, &[]);

        assert!(context.contains("summary:task_kind=summary"));
        assert!(!context.contains("Update the quality worker context_window"));
        assert!(!context.contains("context_window 262144"), "{context}");
        assert!(context.contains("stale_helper_stage_feedback_omitted=count:"));
        assert!(context.contains("current_quality_context_required_tokens:65536"));
        assert!(context.contains("next_advice_should_ignore_stale_helper_context_expansion:true"));
        assert!(json.contains("Update the quality worker context_window from 65536 to 262144"));
    }

    #[test]
    fn prompt_context_omits_stale_helper_context_expansion_without_failure_record() {
        let text = "{\"round\":7,\"case\":\"all-green-but-stale-helper\",\"success\":true,\"runtime_tokens\":64,\"runtime_model\":\"google/gemma\",\"feedback_applied\":4,\"allocation_evidence\":[\"pool_route task_kind:quality quality_context_tokens:65536 quality_context_required_tokens:65536 quality_context_sufficient:true selected_max_tokens:4096\"],\"helper_stage_feedback_by_role\":{\"review\":[\"task_kind=review preview=risk: stale route advice / change_request: Update the quality worker context_window from 65536 to 262144 / verification: confirm context_window 262144\"],\"router\":[\"task_kind=router preview=route_intent: index / tool_call: null / preflight: allow\"]}}\n";
        let summary = summarize_ledger(text);
        let context = prompt_context_text(&summary);
        let json = report_json(&summary, None, None, None, &[]);

        assert_eq!(summary.failure, 0);
        assert!(context.contains("router:task_kind=router"));
        assert!(!context.contains("Update the quality worker context_window"));
        assert!(!context.contains("context_window 262144"), "{context}");
        assert!(context.contains("current_quality_context_required_tokens:65536"));
        assert!(context.contains("next_advice_should_ignore_stale_helper_context_expansion:true"));
        assert!(json.contains("Update the quality worker context_window from 65536 to 262144"));
    }

    #[test]
    fn prompt_context_omits_satisfied_role_budget_advice_after_route_recovery() {
        let text = "{\"round\":8,\"case\":\"all-green-role-budget\",\"success\":true,\"runtime_tokens\":64,\"runtime_model\":\"google/gemma\",\"feedback_applied\":4,\"allocation_evidence\":[\"pool_stage_route[summary] task_kind:summary route_allowed:true selected_context_sufficient:true selected_max_tokens:768 selected_context_required_tokens:768\",\"pool_stage_route[index] task_kind:index route_allowed:true selected_context_sufficient:true selected_max_tokens:512 selected_context_required_tokens:512\"],\"helper_stage_feedback_by_role\":{\"review\":[\"task_kind=review preview=risk: Inconsistent token expectations across components because the index role requires 512 tokens while summary uses 768 / change_request: Update the final_json structure to explicitly set the index task default_max_tokens to 512 / verification: Check the next iteration reflects default_max_tokens: 512 for index\"],\"router\":[\"task_kind=router preview=route_intent: index / tool_call: null / preflight: allow\"]}}\n";
        let summary = summarize_ledger(text);
        let context = prompt_context_text(&summary);
        let json = report_json(&summary, None, None, None, &[]);

        assert_eq!(summary.failure, 0);
        assert!(context.contains("current_role_max_tokens_satisfied="));
        assert!(context.contains("index:512"));
        assert!(context.contains("summary:768"));
        assert!(context.contains("router:task_kind=router"));
        assert!(
            !context.contains("Update the final_json structure"),
            "{context}"
        );
        assert!(!context.contains("default_max_tokens to 512"), "{context}");
        assert!(context.contains("satisfied_role_budget_advice_omitted=count:"));
        assert!(context.contains("next_advice_should_not_repeat_satisfied_role_budget:true"));
        assert!(json.contains("Update the final_json structure to explicitly set the index task"));
    }

    #[test]
    fn prompt_context_omits_underutilized_role_budget_increase_advice() {
        let text = "{\"round\":9,\"case\":\"low-utilization-budget\",\"success\":true,\"runtime_tokens\":64,\"runtime_model\":\"google/gemma\",\"feedback_applied\":4,\"allocation_evidence\":[\"pool_stage_route[index] task_kind:index route_allowed:true selected_context_sufficient:true selected_max_tokens:512 selected_context_required_tokens:512\"],\"helper_stage_feedback_by_role\":{\"index\":[\"task_kind=index elapsed_ms=1200 answer_approx_tokens=102 preview=* clean_gist: Optimize the index worker max_tokens to 1024 to provide more headroom for compact searchable summary fields / * tags: index,budget / * retention: keep\"],\"router\":[\"task_kind=router preview=route_intent: index / tool_call: null / preflight: allow\"]}}\n";
        let summary = summarize_ledger(text);
        let context = prompt_context_text(&summary);
        let json = report_json(&summary, None, None, None, &[]);

        assert!(context.contains("current_role_token_headroom="));
        assert!(context.contains("index:102/512"));
        assert!(context.contains("router:task_kind=router"));
        assert!(!context.contains("max_tokens to 1024"), "{context}");
        assert!(context.contains("underutilized_role_budget_increase_omitted=count:"));
        assert!(context.contains(
            "next_advice_should_require_truncation_or_high_utilization_before_budget_increase:true"
        ));
        assert!(json.contains("Optimize the index worker max_tokens to 1024"));
    }

    #[test]
    fn prompt_context_warns_about_repeated_successful_answers() {
        let repeated_a = "**Improvement:** Increase the `test-gate` role's `selected_max_tokens` from `768` to `1024`. **Verifiable Evidence:** review requested it.";
        let repeated_b = "**Improvement:** Increase the test-gate role's selected_max_tokens from 768 to 1024. **Verifiable Evidence:** route budget shows it.";
        let other = "**Improvement:** Add a report-only ledger summary field. **Verifiable Evidence:** report JSON includes the field.";
        let text = format!(
            "{{\"round\":1,\"case\":\"old\",\"success\":true,\"answer\":{},\"feedback_applied\":4}}\n\
{{\"round\":2,\"case\":\"repeat-1\",\"success\":true,\"answer\":{},\"feedback_applied\":4}}\n\
{{\"round\":3,\"case\":\"repeat-2\",\"success\":true,\"answer\":{},\"feedback_applied\":4}}\n\
{{\"round\":4,\"case\":\"other\",\"success\":true,\"answer\":{},\"feedback_applied\":4}}\n\
{{\"round\":5,\"case\":\"repeat-3\",\"success\":true,\"answer\":{},\"feedback_applied\":4}}\n",
            json_string("old unrelated answer"),
            json_string(repeated_a),
            json_string(repeated_b),
            json_string(other),
            json_string(repeated_a)
        );
        let summary = summarize_ledger(&text);
        let context = prompt_context_text(&summary);
        let json = report_json(&summary, None, None, None, &[]);
        let repeated = summary.recent_repeated_successful_answer.as_ref().unwrap();

        assert_eq!(repeated.count, 3);
        assert_eq!(repeated.window_records, 5);
        assert!(repeated.preview.contains("Increase the"));
        assert!(context.contains("recent_repeated_successful_answer=count:3 window_records:5"));
        assert!(context.contains("blocked_topic:recent-repeated-successful-answer"));
        assert!(context.contains("preview_redacted:true"));
        assert!(!context.contains("Increase the"), "{context}");
        assert!(context.contains("next_advice_should_not_repeat_recent_successful_answer:true"));
        assert!(
            context.contains("next_advice_must_not_use_repeated_answer_preview_as_evidence:true")
        );
        assert!(json.contains("\"recent_repeated_successful_answer\":{\"count\":3"));
        assert!(json.contains("\"window_records\":5"));
    }

    #[test]
    fn summarizes_validation_command_evidence_from_ledger_records() {
        let text = "{\"round\":1,\"case\":\"validation\",\"success\":true,\"feedback_applied\":2,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"safe\",\"validation_command_preview\":\"cargo check --manifest-path tools/evolution-loop/Cargo.toml\",\"validation_phase\":\"pre\",\"validation_status_code\":0,\"validation_elapsed_ms\":123,\"validation_stdout_tail\":\"Finished dev\",\"validation_stderr_tail\":\"warning: none\"}\n";
        let summary = summarize_ledger(text);
        let context = prompt_context_text(&summary);
        let json = report_json(&summary, None, None, None, &[]);
        let last = summary.last.as_ref().unwrap();

        assert_eq!(last.validation_command_source.as_deref(), Some("test-gate"));
        assert_eq!(last.validation_command_safety.as_deref(), Some("safe"));
        assert_eq!(
            last.validation_command_preview.as_deref(),
            Some("cargo check --manifest-path tools/evolution-loop/Cargo.toml")
        );
        assert_eq!(last.validation_phase.as_deref(), Some("pre"));
        assert_eq!(last.validation_status_code, Some(0));
        assert_eq!(last.validation_elapsed_ms, Some(123));
        assert_eq!(last.validation_stdout_tail.as_deref(), Some("Finished dev"));
        assert_eq!(
            last.validation_stderr_tail.as_deref(),
            Some("warning: none")
        );
        assert!(context.contains("last_validation_command=source:test-gate safety:safe"));
        assert!(context.contains("last_validation_result=phase:pre status:0 elapsed_ms:123"));
        assert!(context.contains("stdout_tail:Finished dev"));
        assert!(context.contains("cargo check --manifest-path tools/evolution-loop/Cargo.toml"));
        assert!(json.contains("\"validation_command_source\":\"test-gate\""));
        assert!(json.contains("\"validation_command_safety\":\"safe\""));
        assert!(json.contains(
            "\"validation_command_preview\":\"cargo check --manifest-path tools/evolution-loop/Cargo.toml\""
        ));
        assert!(json.contains("\"validation_phase\":\"pre\""));
        assert!(json.contains("\"validation_status_code\":0"));
        assert!(json.contains("\"validation_elapsed_ms\":123"));
        assert!(json.contains("\"validation_stdout_tail\":\"Finished dev\""));
        assert!(json.contains("\"validation_stderr_tail\":\"warning: none\""));
    }

    #[test]
    fn summarizes_helper_stage_feedback_from_ledger_meta() {
        let text = "{\"round\":1,\"case\":\"helper\",\"success\":true,\"feedback_applied\":2,\"meta\":[\"pool_stage_call_answer task_kind=review role=review elapsed_ms=222 answer_approx_tokens=4 preview=review feedback\",\"pool_stage_call_answer task_kind=index role=index elapsed_ms=111 answer_approx_tokens=6 preview=index feedback\",\"pool_stage_call_answer task_kind=test-gate role=test-gate elapsed_ms=333 answer_approx_tokens=8 preview=test feedback\",\"pool_stage_call_skipped task_kind=test-gate reason=busy\"]}\n";
        let summary = summarize_ledger(text);
        let context = prompt_context_text(&summary);
        let json = report_json(&summary, None, None, None, &[]);
        let last = summary.last.as_ref().unwrap();

        assert_eq!(last.meta.len(), 4);
        assert_eq!(last.helper_stage_feedback().len(), 3);
        assert_eq!(summary.helper_stage_feedback.len(), 3);
        assert_eq!(summary.helper_stage_feedback_by_role.len(), 3);
        assert_eq!(
            summary
                .helper_stage_feedback_by_role
                .get("review")
                .unwrap()
                .len(),
            1
        );
        assert!(context.contains(
            "recent_helper_stage_feedback=pool_stage_call_answer task_kind=review role=review"
        ));
        assert!(context.contains("recent_helper_stage_feedback_by_role="));
        assert!(context.contains("review:task_kind=review"));
        assert!(context.contains("index:task_kind=index"));
        assert!(context.contains("test-gate:task_kind=test-gate"));
        assert!(context.contains("recent_helper_stage_contract_by_role="));
        assert!(context.contains("review:useful=false"));
        assert!(json.contains(
            "\"helper_stage_feedback\":[\"pool_stage_call_answer task_kind=review role=review"
        ));
        assert!(json.contains("\"helper_stage_feedback_by_role\":{\"index\":[\"task_kind=index"));
        assert!(json.contains("\"helper_stage_contract_by_role\":{\"index\":{\"useful\":false"));
        assert!(json.contains("\"review\":[\"task_kind=review"));
        assert!(json.contains("\"test-gate\":[\"task_kind=test-gate"));
        assert!(!context.contains("pool_stage_call_skipped"));
    }

    #[test]
    fn summarizes_structured_helper_stage_feedback_without_meta() {
        let text = "{\"round\":1,\"case\":\"helper\",\"success\":true,\"feedback_applied\":2,\"helper_stage_feedback_by_role\":{\"summary\":[\"task_kind=summary elapsed_ms=111 answer_approx_tokens=4 preview=memory_update: keep Metal evidence\"],\"test-gate\":[\"task_kind=test-gate elapsed_ms=222 answer_approx_tokens=8 preview=validation_command: cargo test\"]}}\n";
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            required_helper_stage_roles: vec!["summary".to_owned(), "test-gate".to_owned()],
            ..Config::default()
        };
        let failures = report_gate_failures(&summary, &config, None, None);
        let context = prompt_context_text(&summary);
        let json = report_json(&summary, None, None, None, &failures);

        assert_eq!(summary.helper_stage_feedback.len(), 0);
        assert_eq!(summary.helper_stage_feedback_by_role.len(), 2);
        assert!(failures.is_empty(), "{failures:?}");
        assert!(context.contains("recent_helper_stage_feedback_by_role="));
        assert!(context.contains("summary:task_kind=summary"));
        assert!(context.contains("memory_update: keep Metal evidence"));
        assert!(context.contains("test-gate:task_kind=test-gate"));
        assert!(json.contains("\"helper_stage_feedback_by_role\":{\"summary\""));
        assert!(json.contains("\"test-gate\":[\"task_kind=test-gate"));
    }

    #[test]
    fn summarizes_helper_stage_contracts_for_prompt_and_json() {
        let text = "{\"round\":1,\"case\":\"helper\",\"success\":true,\"feedback_applied\":2,\"helper_stage_feedback_by_role\":{\"summary\":[\"task_kind=summary preview=memory_update: keep Metal evidence / next_context: prefer small helpers / duplicate_guard: do not retry stale CPU path\"],\"review\":[\"task_kind=review preview=risk: stale helper advice / change_request: feed contract summary into prompt / verification: cargo test evolution-loop\"]}}\n";
        let summary = summarize_ledger(text);
        let context = prompt_context_text(&summary);
        let json = report_json(&summary, None, None, None, &[]);

        let summary_contract = summary
            .helper_stage_contract_by_role
            .get("summary")
            .expect("summary contract");
        let review_contract = summary
            .helper_stage_contract_by_role
            .get("review")
            .expect("review contract");

        assert!(summary_contract.useful);
        assert!(review_contract.useful);
        assert_eq!(
            summary_contract.matched_markers,
            vec![
                "memory_update".to_owned(),
                "next_context".to_owned(),
                "duplicate_guard".to_owned()
            ]
        );
        assert_eq!(
            summary_contract
                .fields
                .get("memory_update")
                .map(String::as_str),
            Some("keep Metal evidence")
        );
        assert_eq!(
            review_contract
                .fields
                .get("change_request")
                .map(String::as_str),
            Some("feed contract summary into prompt")
        );
        assert!(context.contains("recent_helper_stage_contract_by_role="));
        assert!(context.contains("summary:useful=true fields=memory_update=keep Metal evidence"));
        assert!(context.contains("change_request=feed contract summary into prompt"));
        assert!(context.contains("matched=risk,change_request,verification"));
        assert!(json.contains("\"helper_stage_contract_by_role\":{\"review\":{\"useful\":true"));
        assert!(
            json.contains("\"fields\":{\"change_request\":\"feed contract summary into prompt\"")
        );
        assert!(json.contains("\"memory_update\":\"keep Metal evidence\""));
        assert!(
            json.contains("\"matched_markers\":[\"risk\",\"change_request\",\"verification\"]")
        );
        assert!(json.contains(
            "\"expected_markers\":[\"memory_update\",\"next_context\",\"duplicate_guard\"]"
        ));
    }

    #[test]
    fn summarizes_structured_helper_stage_contract_fields_from_ledger() {
        let text = "{\"round\":1,\"case\":\"helper\",\"success\":true,\"feedback_applied\":2,\"helper_stage_contract_by_role\":{\"review\":{\"fields\":{\"risk\":\"stale helper advice\",\"change_request\":\"persist helper fields\",\"verification\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"},\"matched_markers\":[\"risk\",\"change_request\",\"verification\"],\"expected_markers\":[\"risk\",\"change_request\",\"verification\"]}}}\n";
        let summary = summarize_ledger(text);
        let context = prompt_context_text(&summary);
        let json = report_json(&summary, None, None, None, &[]);

        let review_contract = summary
            .helper_stage_contract_by_role
            .get("review")
            .expect("review contract");

        assert!(review_contract.useful);
        assert_eq!(
            review_contract.fields.get("risk").map(String::as_str),
            Some("stale helper advice")
        );
        assert_eq!(
            review_contract
                .fields
                .get("change_request")
                .map(String::as_str),
            Some("persist helper fields")
        );
        assert!(context.contains("recent_helper_stage_contract_by_role="));
        assert!(context.contains("review:useful=true fields="));
        assert!(context.contains("change_request=persist helper fields"));
        assert!(json.contains("\"helper_stage_contract_by_role\":{\"review\":{\"useful\":true"));
        assert!(json.contains(
            "\"verification\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\""
        ));
    }

    #[test]
    fn summarizes_completed_index_change_requests_for_prompt_guard() {
        let text = r#"{"round":12,"case":"completed-source-origin","success":true,"feedback_applied":4,"helper_stage_feedback_by_role":{"review":["task_kind=review preview=risk: index output is hard to trace / change_request: Add a source_origin field to the index clean_gist output schema / verification: check index contract"],"index":["task_kind=index preview=clean_gist: Index round 12 with source evidence / tags: role=index;case=completed-source-origin;round=12;dependency=review.change_request;source_origin=review.change_request;validation_timestamp=1781770123 / dependency_link: review.change_request / source_origin: review.change_request / validation_timestamp: 1781770123 / retention: keep"]},"helper_stage_contract_by_role":{"review":{"fields":{"risk":"index output is hard to trace","change_request":"Add a source_origin field to the index clean_gist output schema","verification":"check index contract"},"matched_markers":["risk","change_request","verification"],"expected_markers":["risk","change_request","verification"]},"index":{"fields":{"clean_gist":"Index round 12 with source evidence","tags":"role=index;case=completed-source-origin;round=12;dependency=review.change_request;source_origin=review.change_request;validation_timestamp=1781770123","dependency_link":"review.change_request","source_origin":"review.change_request","validation_timestamp":"1781770123","retention":"keep"},"matched_markers":["clean_gist","tags","dependency_link","source_origin","validation_timestamp","retention"],"expected_markers":["clean_gist","tags","dependency_link","source_origin","validation_timestamp","retention"]}}}
"#;
        let summary = summarize_ledger(text);
        let context = prompt_context_text(&summary);
        let json = report_json(&summary, None, None, None, &[]);

        assert_eq!(
            summary.completed_change_requests,
            vec![
                "review.change_request requested index.source_origin; latest_successful_index_contract_has_source_origin=true".to_owned()
            ]
        );
        assert!(context.contains("completed_change_requests_do_not_repeat="));
        assert!(context.contains("index.source_origin"));
        assert!(context.contains("blocked_completed_change_topics=index.source_origin"));
        assert!(context.contains("completed_change_requests_are_already_done:true"));
        assert!(context.contains("next_advice_must_not_recommend_completed_change_requests:true"));
        assert!(context.contains("next_advice_must_choose_new_uncompleted_change:true"));
        assert!(context.contains(
            "completed_change_topic_helper_context_omitted=count:4 topics:index.source_origin"
        ));
        assert!(context.contains("next_advice_should_not_use_completed_topic_feedback:true"));
        assert!(!context.contains("recent_helper_stage_feedback_by_role="));
        assert!(!context.contains("recent_helper_stage_contract_by_role="));
        assert!(json.contains("\"source_origin\":\"review.change_request\""));
        assert!(json.contains("\"helper_stage_contract_by_role\":{\"index\""));
    }

    #[test]
    fn completed_change_request_detects_required_index_field_wording() {
        let text = r#"{"round":12,"case":"completed-source-origin-required","success":true,"feedback_applied":4,"helper_stage_contract_by_role":{"review":{"fields":{"risk":"index output is hard to trace","change_request":"Confirm source_origin is required in the index clean_gist schema","verification":"check index contract"},"matched_markers":["risk","change_request","verification"],"expected_markers":["risk","change_request","verification"]},"index":{"fields":{"clean_gist":"Index round 12 with source evidence","tags":"role=index;case=completed-source-origin-required;round=12;dependency=review.change_request;source_origin=review.change_request;validation_timestamp=1781770123","dependency_link":"review.change_request","source_origin":"review.change_request","validation_timestamp":"1781770123","retention":"keep"},"matched_markers":["clean_gist","tags","dependency_link","source_origin","validation_timestamp","retention"],"expected_markers":["clean_gist","tags","dependency_link","source_origin","validation_timestamp","retention"]}}}
"#;
        let summary = summarize_ledger(text);

        assert_eq!(
            summary.completed_change_requests,
            vec![
                "review.change_request requested index.source_origin; latest_successful_index_contract_has_source_origin=true".to_owned()
            ]
        );
    }

    #[test]
    fn completed_change_request_requires_matching_index_evidence() {
        let text = "{\"round\":12,\"case\":\"missing-source-origin\",\"success\":true,\"feedback_applied\":4,\"helper_stage_contract_by_role\":{\"review\":{\"fields\":{\"risk\":\"index output is hard to trace\",\"change_request\":\"Add a source_origin field to the index clean_gist output schema\",\"verification\":\"check index contract\"},\"matched_markers\":[\"risk\",\"change_request\",\"verification\"],\"expected_markers\":[\"risk\",\"change_request\",\"verification\"]},\"index\":{\"fields\":{\"clean_gist\":\"Index round 12 without source evidence\",\"tags\":\"role=index;case=missing-source-origin;round=12;dependency=review.change_request;validation_timestamp=1781770123\",\"dependency_link\":\"review.change_request\",\"validation_timestamp\":\"1781770123\",\"retention\":\"keep\"},\"matched_markers\":[\"clean_gist\",\"tags\",\"dependency_link\",\"validation_timestamp\",\"retention\"],\"expected_markers\":[\"clean_gist\",\"tags\",\"dependency_link\",\"source_origin\",\"validation_timestamp\",\"retention\"]}}}\n";
        let summary = summarize_ledger(text);
        let context = prompt_context_text(&summary);

        assert!(summary.completed_change_requests.is_empty());
        assert!(!context.contains("completed_change_requests_do_not_repeat="));
        assert!(!context.contains("next_advice_must_not_recommend_completed_change_requests:true"));
    }

    #[test]
    fn summarizes_completed_final_json_pool_stage_dispatch_for_prompt_guard() {
        let text = r#"{"round":13,"case":"completed-dispatch","success":true,"feedback_applied":4,"helper_stage_feedback_by_role":{"review":["task_kind=review preview=risk: final json can hide missing helper dispatch evidence / change_request: Add a --strict flag to the validate-evolution-loop test runner that enforces pool_stage_dispatch completeness for all defined stages / verification: cargo test -q --manifest-path tools/evolution-loop/Cargo.toml"],"router":["task_kind=router preview=route_intent: review / tool_call: null / preflight: allow"]},"helper_stage_contract_by_role":{"review":{"fields":{"risk":"final json can hide missing helper dispatch evidence","change_request":"Add a --strict flag to the validate-evolution-loop test runner that enforces pool_stage_dispatch completeness for all defined stages","verification":"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml"},"matched_markers":["risk","change_request","verification"],"expected_markers":["risk","change_request","verification"]}},"final_json_pool_stage_dispatch":[{"task_kind":"summary"},{"task_kind":"router"},{"task_kind":"review"},{"task_kind":"index"},{"task_kind":"test-gate"}]}
"#;
        let summary = summarize_ledger(text);
        let context = prompt_context_text(&summary);
        let json = report_json(&summary, None, None, None, &[]);

        assert_eq!(
            summary.completed_change_requests,
            vec![
                "review.change_request requested final_json.pool_stage_dispatch; latest_successful_final_json_pool_stage_dispatch_has_required_task_kinds=true".to_owned()
            ]
        );
        assert!(context.contains("completed_change_requests_do_not_repeat="));
        assert!(context.contains("final_json.pool_stage_dispatch"));
        assert!(context.contains("blocked_completed_change_topics=final_json.pool_stage_dispatch"));
        assert!(context.contains("next_advice_must_not_recommend_completed_change_requests:true"));
        assert!(context.contains("completed_change_topic_helper_context_omitted=count:"));
        assert!(context.contains("next_advice_should_not_use_completed_topic_feedback:true"));
        assert!(!context.contains("Add a --strict flag to the validate-evolution-loop"));
        assert!(context.contains("router:task_kind=router"));
        assert!(json.contains("\"completed_change_requests\":{\"items\":[\"review.change_request requested final_json.pool_stage_dispatch"));
        assert!(json.contains("\"blocked_topics\":[\"final_json.pool_stage_dispatch\"]"));
    }

    #[test]
    fn completed_final_json_pool_stage_dispatch_requires_all_task_kinds() {
        let text = r#"{"round":13,"case":"incomplete-dispatch","success":true,"feedback_applied":4,"helper_stage_contract_by_role":{"review":{"fields":{"risk":"final json can hide missing helper dispatch evidence","change_request":"Require final_json.pool_stage_dispatch completeness for every helper role","verification":"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml"},"matched_markers":["risk","change_request","verification"],"expected_markers":["risk","change_request","verification"]}},"final_json_pool_stage_dispatch":[{"task_kind":"summary"},{"task_kind":"router"},{"task_kind":"review"},{"task_kind":"test-gate"}]}
"#;
        let summary = summarize_ledger(text);
        let context = prompt_context_text(&summary);

        assert!(summary.completed_change_requests.is_empty());
        assert!(!context.contains("completed_change_requests_do_not_repeat="));
        assert!(
            !context.contains("blocked_completed_change_topics=final_json.pool_stage_dispatch")
        );
        assert!(!context.contains("next_advice_must_not_recommend_completed_change_requests:true"));
    }

    #[test]
    fn summarizes_invalid_cargo_test_strict_flag_for_prompt_guard() {
        let text = r#"{"round":14,"case":"invalid-cargo-strict","success":true,"feedback_applied":4,"helper_stage_feedback_by_role":{"review":["task_kind=review preview=risk: validation can drift / change_request: Modify the test-gate validation command to include --strict-strict when executing cargo test / verification: rerun report gate"],"router":["task_kind=router preview=route_intent: review / tool_call: null / preflight: allow"]},"helper_stage_contract_by_role":{"review":{"fields":{"risk":"validation can drift","change_request":"Modify the test-gate validation command to include --strict-strict when executing cargo test","verification":"rerun report gate"},"matched_markers":["risk","change_request","verification"],"expected_markers":["risk","change_request","verification"]}}}
"#;
        let summary = summarize_ledger(text);
        let context = prompt_context_text(&summary);
        let json = report_json(&summary, None, None, None, &[]);

        assert_eq!(
            summary.invalid_change_requests,
            vec![
                "review.change_request requested unsupported cargo.test.strict-flag; cargo_test_has_no_strict_flag=true"
                    .to_owned()
            ]
        );
        assert!(context.contains("invalid_change_requests_do_not_repeat="));
        assert!(context.contains("blocked_invalid_change_topics=cargo.test.strict-flag"));
        assert!(context.contains("invalid_change_requests_are_rejected:true"));
        assert!(context.contains("next_advice_must_not_recommend_invalid_change_requests:true"));
        assert!(context.contains("invalid_change_topic_helper_context_omitted=count:"));
        assert!(context.contains("topics:cargo.test.strict-flag"));
        assert!(!context.contains("--strict-strict"), "{context}");
        assert!(
            !context.contains("Modify the test-gate validation command"),
            "{context}"
        );
        assert!(context.contains("router:task_kind=router"));
        assert!(json.contains(
            "\"invalid_change_requests\":{\"items\":[\"review.change_request requested unsupported cargo.test.strict-flag"
        ));
        assert!(json.contains("\"blocked_topics\":[\"cargo.test.strict-flag\"]"));
    }

    #[test]
    fn summarizes_redundant_max_iterations_advice_for_prompt_guard() {
        let text = r#"{"round":14,"case":"invalid-max-iterations","success":true,"feedback_applied":4,"helper_stage_feedback_by_role":{"review":["task_kind=review preview=risk: runaway loops could waste model time / change_request: Modify the evolution loop mechanism to accept and utilize a --max-iterations flag to cap the number of rounds / verification: confirm the new flag terminates after N rounds"],"router":["task_kind=router preview=route_intent: review / tool_call: null / preflight: allow"]},"helper_stage_contract_by_role":{"review":{"fields":{"risk":"runaway loops could waste model time","change_request":"Modify the evolution loop mechanism to accept and utilize a --max-iterations flag to cap the number of rounds","verification":"confirm the new flag terminates after N rounds"},"matched_markers":["risk","change_request","verification"],"expected_markers":["risk","change_request","verification"]}}}
"#;
        let summary = summarize_ledger(text);
        let context = prompt_context_text(&summary);
        let json = report_json(&summary, None, None, None, &[]);

        assert_eq!(
            summary.invalid_change_requests,
            vec![
                "review.change_request requested redundant evolution-loop.max-iterations flag; evolution_loop_already_has_rounds_forever_and_budget_stop_controls=true"
                    .to_owned()
            ]
        );
        assert!(context.contains("invalid_change_requests_do_not_repeat="));
        assert!(context.contains("blocked_invalid_change_topics=evolution-loop.max-iterations"));
        assert!(context.contains("invalid_change_requests_are_rejected:true"));
        assert!(context.contains("next_advice_must_not_recommend_invalid_change_requests:true"));
        assert!(context.contains("invalid_change_topic_helper_context_omitted=count:"));
        assert!(context.contains("topics:evolution-loop.max-iterations"));
        assert!(
            !context.contains("Modify the evolution loop mechanism"),
            "{context}"
        );
        assert!(
            !context.contains("confirm the new flag terminates"),
            "{context}"
        );
        assert!(context.contains("router:task_kind=router"));
        assert!(json.contains(
            "\"invalid_change_requests\":{\"items\":[\"review.change_request requested redundant evolution-loop.max-iterations flag"
        ));
        assert!(json.contains("\"blocked_topics\":[\"evolution-loop.max-iterations\"]"));
    }

    #[test]
    fn summarizes_unproven_strict_coverage_advice_for_prompt_guard() {
        let text = r#"{"round":17,"case":"invalid-strict-coverage","success":true,"feedback_applied":4,"helper_stage_feedback_by_role":{"review":["task_kind=review preview=risk: The test suite passes but lacks explicit coverage enforcement for the core module (`evolution_loop.rs`) / change_request: Add a `--strict-coverage` flag to the evolution-loop test harness to enforce 100% line coverage for `evolution_loop.rs` / verification: cargo test -q --manifest-path tools/evolution-loop/Cargo.toml -- --strict-coverage"],"router":["task_kind=router preview=route_intent: review / tool_call: null / preflight: allow"]},"helper_stage_contract_by_role":{"review":{"fields":{"risk":"The test suite passes but lacks explicit coverage enforcement for the core module (`evolution_loop.rs`)","change_request":"Add a `--strict-coverage` flag to the evolution-loop test harness to enforce 100% line coverage for `evolution_loop.rs`","verification":"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml -- --strict-coverage"},"matched_markers":["risk","change_request","verification"],"expected_markers":["risk","change_request","verification"]}}}
"#;
        let summary = summarize_ledger(text);
        let context = prompt_context_text(&summary);
        let json = report_json(&summary, None, None, None, &[]);

        assert_eq!(
            summary.invalid_change_requests,
            vec![
                "review.change_request requested unproven evolution-loop.strict-coverage control; require_existing_coverage_tooling_or_coverage_report_before_strict_coverage_work=true"
                    .to_owned()
            ]
        );
        assert!(context.contains("invalid_change_requests_do_not_repeat="));
        assert!(context.contains("blocked_invalid_change_topics=evolution-loop.strict-coverage"));
        assert!(context.contains("invalid_change_requests_are_rejected:true"));
        assert!(context.contains("next_advice_must_not_recommend_invalid_change_requests:true"));
        assert!(context.contains("invalid_change_topic_helper_context_omitted=count:"));
        assert!(context.contains("topics:evolution-loop.strict-coverage"));
        assert!(!context.contains("--strict-coverage"), "{context}");
        assert!(!context.contains("100% line coverage"), "{context}");
        assert!(context.contains("router:task_kind=router"));
        assert!(json.contains(
            "\"invalid_change_requests\":{\"items\":[\"review.change_request requested unproven evolution-loop.strict-coverage control"
        ));
        assert!(json.contains("\"blocked_topics\":[\"evolution-loop.strict-coverage\"]"));
        assert!(json.contains("\"strict_coverage_requested\":false"));
        assert!(json.contains("\"coverage_blocked\":false"));
        assert!(json.contains("\"allow_next_round\":true"));
    }

    #[test]
    fn report_gate_quarantines_unproven_coverage_advice_without_blocking_continuation() {
        let text = r#"{"round":18,"case":"invalid-coverage-report-gate","success":true,"feedback_applied":4,"helper_stage_contract_by_role":{"review":{"fields":{"risk":"coverage drift can hide gaps","change_request":"Add a coverage report gate to the evolution-loop report JSON before the next unattended run","verification":"run report gate"},"matched_markers":["risk","change_request","verification"],"expected_markers":["risk","change_request","verification"]}}}
"#;
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert_eq!(
            summary.invalid_change_requests,
            vec![invalid_unproven_strict_coverage_summary()]
        );
        assert!(
            !summary
                .validation_command_coverage_evidence
                .strict_coverage_is_requested()
        );
        assert!(
            !summary
                .validation_command_coverage_evidence
                .coverage_tooling_or_report_evidence_present()
        );
        assert!(failures.is_empty(), "{failures:?}");
    }

    #[test]
    fn latest_regular_validation_quarantines_stale_strict_coverage_advice() {
        let text = r#"{"round":18,"case":"stale-strict-coverage-advice","success":true,"feedback_applied":4,"answer":"Improvement: add --strict-coverage to the test command","validation_checked":true,"validation_passed":true,"validation_command_source":"configured","validation_command_safety":"explicit","validation_command_preview":"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml","validation_status_code":0,"helper_stage_contract_by_role":{"review":{"fields":{"risk":"coverage drift can hide gaps","change_request":"Add a --strict-coverage flag to the evolution-loop validation command","verification":"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml -- --strict-coverage"},"matched_markers":["risk","change_request","verification"],"expected_markers":["risk","change_request","verification"]}}}
{"round":19,"case":"latest-regular-validation","success":true,"feedback_applied":4,"validation_checked":true,"validation_passed":true,"validation_command_source":"configured","validation_command_safety":"explicit","validation_command_preview":"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml","validation_status_code":0}
"#;
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            ..Config::default()
        };
        let failures = report_gate_failures(&summary, &config, None, None);

        assert_eq!(
            summary.invalid_change_requests,
            vec![invalid_unproven_strict_coverage_summary()]
        );
        assert!(
            !summary
                .validation_command_coverage_evidence
                .strict_coverage_is_requested()
        );
        assert!(failures.is_empty(), "{failures:?}");
    }

    #[test]
    fn coverage_report_gate_advice_with_passed_coverage_validation_is_not_invalid() {
        let text = r#"{"round":18,"case":"coverage-evidence","success":true,"feedback_applied":4,"validation_checked":true,"validation_passed":true,"validation_command_source":"configured","validation_command_safety":"explicit","validation_command_preview":"cargo llvm-cov --summary-only --manifest-path tools/evolution-loop/Cargo.toml","validation_status_code":0,"validation_stdout_tail":"coverage report: line coverage 82.4% function coverage 90.0%","helper_stage_contract_by_role":{"review":{"fields":{"risk":"coverage drift can hide gaps","change_request":"Add a coverage report gate to the evolution-loop report JSON before the next unattended run","verification":"cargo llvm-cov --summary-only --manifest-path tools/evolution-loop/Cargo.toml"},"matched_markers":["risk","change_request","verification"],"expected_markers":["risk","change_request","verification"]}}}
"#;
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            ..Config::default()
        };
        let failures = report_gate_failures(&summary, &config, None, None);

        assert!(summary.invalid_change_requests.is_empty());
        assert!(
            summary
                .validation_command_coverage_evidence
                .coverage_tooling_evidence
                .iter()
                .any(|evidence| evidence.contains("cargo llvm-cov"))
        );
        assert!(
            summary
                .validation_command_coverage_evidence
                .coverage_report_evidence
                .iter()
                .any(|evidence| evidence.contains("line coverage 82.4%"))
        );
        assert!(
            summary
                .validation_command_coverage_evidence
                .coverage_tooling_or_report_evidence_present()
        );
        assert!(failures.is_empty(), "{failures:?}");
    }

    #[test]
    fn invalid_advice_and_report_gate_share_coverage_evidence_projection() {
        let text = r#"{"round":20,"case":"coverage-report-field-evidence","success":true,"feedback_applied":4,"helper_stage_contract_by_role":{"review":{"fields":{"risk":"coverage drift can hide gaps","change_request":"Add a coverage report gate to the evolution-loop report JSON before the next unattended run","verification":"inspect validation_command.coverage_report_evidence","coverage_report_evidence":"coverage report path target/evolution/coverage/html/index.html; line coverage 82.4%"},"matched_markers":["risk","change_request","verification"],"expected_markers":["risk","change_request","verification"]}}}
"#;
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert!(
            summary
                .validation_command_coverage_evidence
                .coverage_report_evidence
                .iter()
                .any(|evidence| evidence.contains("target/evolution/coverage"))
        );
        assert!(summary.invalid_change_requests.is_empty());
        assert!(failures.is_empty(), "{failures:?}");
    }

    #[test]
    fn report_json_exposes_validation_command_coverage_evidence_surface() {
        let without_evidence = "{\"round\":19,\"case\":\"strict-command-no-evidence\",\"success\":true,\"feedback_applied\":4,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"safe\",\"validation_command_preview\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml -- --strict-coverage\",\"validation_status_code\":0,\"validation_stdout_tail\":\"tests passed\"}\n";
        let without_summary = summarize_ledger(without_evidence);
        let without_json = report_json(&without_summary, None, None, None, &[]);
        let config = Config {
            report_gate: true,
            ..Config::default()
        };
        let without_failures = report_gate_failures(&without_summary, &config, None, None);

        assert!(without_json.contains("\"validation_command_coverage_report_v1\":{\"schema\":\"validation_command_coverage_report_v1\""));
        assert!(without_json.contains("\"strict_coverage_requested\":true"));
        assert!(without_json.contains("\"coverage_tooling_evidence\":[]"));
        assert!(without_json.contains("\"coverage_report_evidence\":[]"));
        assert!(without_json.contains("\"coverage_tooling_or_report_evidence_present\":false"));
        assert!(without_json.contains("\"coverage_blocked\":true"));
        assert!(without_json.contains("\"coverage_failure_kind\":\"validation_command_coverage\""));
        assert!(without_json.contains("\"model_quality_failure_counted\":false"));
        assert!(without_json.contains("\"allow_next_round\":false"));
        assert!(
            without_failures
                .iter()
                .any(|failure| failure.contains("evolution-loop.strict-coverage")),
            "{without_failures:?}"
        );

        let with_evidence = "{\"round\":20,\"case\":\"strict-command-with-coverage-report\",\"success\":true,\"feedback_applied\":4,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"safe\",\"validation_command_preview\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml -- --strict-coverage\",\"validation_status_code\":0,\"validation_stdout_tail\":\"coverage report path target/evolution/coverage/html/index.html; line coverage 82.4%\"}\n";
        let with_summary = summarize_ledger(with_evidence);
        let with_json = report_json(&with_summary, None, None, None, &[]);
        let with_failures = report_gate_failures(&with_summary, &config, None, None);

        assert!(with_json.contains("\"validation_command_coverage_report_v1\":{\"schema\":\"validation_command_coverage_report_v1\""));
        assert!(with_json.contains("\"strict_coverage_requested\":true"));
        assert!(with_json.contains("\"coverage_report_evidence\":[\"coverage report path target/evolution/coverage/html/index.html; line coverage 82.4%\"]"));
        assert!(with_json.contains("\"coverage_tooling_or_report_evidence_present\":true"));
        assert!(with_json.contains("\"coverage_blocked\":false"));
        assert!(with_json.contains("\"coverage_failure_kind\":\"none\""));
        assert!(with_json.contains("\"failure_reasons\":[]"));
        assert!(with_json.contains("\"allow_next_round\":true"));
        assert!(with_failures.is_empty(), "{with_failures:?}");
    }

    #[test]
    fn report_gate_blocks_strict_coverage_validation_command_without_eval_evidence() {
        let text = "{\"round\":19,\"case\":\"strict-command-no-evidence\",\"success\":true,\"feedback_applied\":4,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"safe\",\"validation_command_preview\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml -- --strict-coverage\",\"validation_status_code\":0,\"validation_stdout_tail\":\"tests passed\"}\n";
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert!(summary.invalid_change_requests.is_empty());
        assert!(
            summary
                .validation_command_coverage_evidence
                .strict_coverage_is_requested()
        );
        assert!(
            !summary
                .validation_command_coverage_evidence
                .coverage_tooling_or_report_evidence_present()
        );
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("evolution-loop.strict-coverage")),
            "{failures:?}"
        );
    }

    #[test]
    fn summarizes_unproven_test_seed_advice_for_prompt_guard() {
        let text = r#"{"round":16,"case":"invalid-test-seed","success":true,"feedback_applied":4,"helper_stage_feedback_by_role":{"review":["task_kind=review preview=risk: test output could be inconsistent / change_request: Modify the evolution-loop test execution setup to initialize the test harness with a fixed known random seed / verification: run cargo test twice and observe identical output"],"router":["task_kind=router preview=route_intent: review / tool_call: null / preflight: allow"]},"helper_stage_contract_by_role":{"review":{"fields":{"risk":"test output could be inconsistent","change_request":"Modify the evolution-loop test execution setup to initialize the test harness with a fixed known random seed","verification":"run cargo test twice and observe identical output"},"matched_markers":["risk","change_request","verification"],"expected_markers":["risk","change_request","verification"]}}}
"#;
        let summary = summarize_ledger(text);
        let context = prompt_context_text(&summary);
        let json = report_json(&summary, None, None, None, &[]);

        assert_eq!(
            summary.invalid_change_requests,
            vec![
                "review.change_request requested unproven evolution-loop.test-deterministic-seed control; require_flaky_test_or_randomness_evidence_before_seed_work=true"
                    .to_owned()
            ]
        );
        assert!(context.contains("invalid_change_requests_do_not_repeat="));
        assert!(
            context
                .contains("blocked_invalid_change_topics=evolution-loop.test-deterministic-seed"),
            "{context}"
        );
        assert!(context.contains("invalid_change_requests_are_rejected:true"));
        assert!(context.contains("next_advice_must_not_recommend_invalid_change_requests:true"));
        assert!(context.contains("invalid_change_topic_helper_context_omitted=count:"));
        assert!(context.contains("topics:evolution-loop.test-deterministic-seed"));
        assert!(
            !context.contains("Modify the evolution-loop test execution setup"),
            "{context}"
        );
        assert!(!context.contains("fixed known random seed"), "{context}");
        assert!(context.contains("router:task_kind=router"));
        assert!(json.contains(
            "\"invalid_change_requests\":{\"items\":[\"review.change_request requested unproven evolution-loop.test-deterministic-seed control"
        ));
        assert!(json.contains("\"blocked_topics\":[\"evolution-loop.test-deterministic-seed\"]"));
    }

    #[test]
    fn valid_runner_strict_flag_advice_is_not_invalid_cargo_test_advice() {
        let text = r#"{"round":15,"case":"valid-runner-strict","success":true,"feedback_applied":4,"helper_stage_feedback_by_role":{"review":["task_kind=review preview=risk: final json can hide missing helper dispatch evidence / change_request: Add a --strict flag to the validate-evolution-loop test runner that enforces pool_stage_dispatch completeness for all defined stages / verification: cargo test -q --manifest-path tools/evolution-loop/Cargo.toml"],"router":["task_kind=router preview=route_intent: review / tool_call: null / preflight: allow"]},"helper_stage_contract_by_role":{"review":{"fields":{"risk":"final json can hide missing helper dispatch evidence","change_request":"Add a --strict flag to the validate-evolution-loop test runner that enforces pool_stage_dispatch completeness for all defined stages","verification":"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml"},"matched_markers":["risk","change_request","verification"],"expected_markers":["risk","change_request","verification"]}}}
"#;
        let summary = summarize_ledger(text);
        let context = prompt_context_text(&summary);
        let json = report_json(&summary, None, None, None, &[]);

        assert!(summary.invalid_change_requests.is_empty());
        assert!(!context.contains("invalid_change_requests_do_not_repeat="));
        assert!(!context.contains("blocked_invalid_change_topics="));
        assert!(!context.contains("next_advice_must_not_recommend_invalid_change_requests:true"));
        assert!(context.contains("Add a --strict flag to the validate-evolution-loop"));
        assert!(json.contains("\"invalid_change_requests\":{\"items\":[],\"blocked_topics\":[]}"));
    }

    #[test]
    fn helper_stage_contract_fields_parse_bullet_feedback() {
        let feedback = vec![
            "task_kind=review preview=- risk: stale index feedback\n- change_request: persist helper fields\n- verification: cargo test -q --manifest-path tools/evolution-loop/Cargo.toml"
                .to_owned(),
        ];
        let fields = helper_feedback::contract_fields("review", &feedback);

        assert_eq!(
            fields.get("risk").map(String::as_str),
            Some("stale index feedback")
        );
        assert_eq!(
            fields.get("change_request").map(String::as_str),
            Some("persist helper fields")
        );
        assert_eq!(
            fields.get("verification").map(String::as_str),
            Some("cargo test -q --manifest-path tools/evolution-loop/Cargo.toml")
        );
        assert!(helper_stage_feedback_usefulness_failure("review", &feedback).is_none());
    }

    #[test]
    fn report_gate_passes_when_required_helper_roles_have_feedback() {
        let text = "{\"round\":1,\"case\":\"helper\",\"success\":true,\"feedback_applied\":2,\"meta\":[\"pool_stage_call_answer task_kind=summary role=summary elapsed_ms=111 answer_approx_tokens=4 preview=summary feedback\",\"pool_stage_call_answer task_kind=review role=review elapsed_ms=222 answer_approx_tokens=4 preview=review feedback\",\"pool_stage_call_answer task_kind=test-gate role=test-gate elapsed_ms=333 answer_approx_tokens=8 preview=test feedback\"]}\n";
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            required_helper_stage_roles: vec![
                "summary".to_owned(),
                "review".to_owned(),
                "test-gate".to_owned(),
            ],
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert!(failures.is_empty(), "{failures:?}");
    }

    #[test]
    fn report_gate_blocks_missing_required_helper_roles() {
        let text = "{\"round\":1,\"case\":\"helper\",\"success\":true,\"feedback_applied\":2,\"meta\":[\"pool_stage_call_answer task_kind=summary role=summary elapsed_ms=111 answer_approx_tokens=4 preview=summary feedback\",\"pool_stage_call_answer task_kind=review role=review elapsed_ms=222 answer_approx_tokens=4 preview=review feedback\"]}\n";
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            required_helper_stage_roles: vec![
                "summary".to_owned(),
                "review".to_owned(),
                "test-gate".to_owned(),
            ],
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert!(
            failures.iter().any(|failure| failure
                .contains("helper stage feedback missing required roles: test-gate")),
            "{failures:?}"
        );
    }

    #[test]
    fn summary_sanitizes_helper_stage_markdown_code_fence_wrapper() {
        let text = "{\"round\":1,\"case\":\"helper\",\"success\":true,\"feedback_applied\":2,\"helper_stage_feedback_by_role\":{\"summary\":[\"task_kind=summary preview=```python / memory_update: keep short\"]}}\n";

        let summary = summarize_ledger(text);
        let latest_summary = summary
            .helper_stage_feedback_by_role
            .get("summary")
            .and_then(|feedback| feedback.last())
            .expect("summary feedback");

        assert!(!summary.helper_stage_hygiene_by_role.contains_key("summary"));
        assert!(!latest_summary.contains("```python"));
        assert!(latest_summary.contains("memory_update: keep short"));
    }

    #[test]
    fn summary_reports_index_prompt_echo_hygiene() {
        let text = "{\"round\":1,\"case\":\"helper\",\"success\":true,\"feedback_applied\":2,\"helper_stage_feedback_by_role\":{\"index\":[\"task_kind=index preview=clean_gist: Check the SmartSteam unattended evolution chain. Return one small improvement and one verifiable evidence item. / Previous SmartSteam evolution ledger summary: previous_rounds=246 / tags: role=index;case=helper;round=1;source_origin=review.change_request / dependency_link: review.change_request / source_origin: review.change_request / validation_timestamp: 1781859101 / retention: keep\"]}}\n";

        let summary = summarize_ledger(text);
        let findings = summary
            .helper_stage_hygiene_by_role
            .get("index")
            .expect("index hygiene finding");

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].role, "index");
        assert_eq!(findings[0].feedback_index, 0);
        assert_eq!(findings[0].kind, "prompt_echo");
        assert!(findings[0].preview.contains("clean_gist"));
    }

    #[test]
    fn report_json_includes_helper_stage_hygiene_by_role() {
        let text = "{\"round\":1,\"case\":\"helper\",\"success\":true,\"feedback_applied\":2,\"helper_stage_feedback_by_role\":{\"index\":[\"task_kind=index preview=clean_gist: Check the SmartSteam unattended evolution chain. Return one small improvement and one verifiable evidence item. / Previous SmartSteam evolution ledger summary: previous_rounds=246 / tags: role=index;case=helper;round=1;source_origin=review.change_request / dependency_link: review.change_request / source_origin: review.change_request / validation_timestamp: 1781859101 / retention: keep\"]}}\n";
        let summary = summarize_ledger(text);

        let json = report_json(&summary, None, None, None, &[]);

        assert!(json.contains("\"helper_stage_hygiene_by_role\":{\"index\":[{"));
        assert!(json.contains("\"role\":\"index\""));
        assert!(json.contains("\"kind\":\"prompt_echo\""));
    }

    #[test]
    fn report_gate_blocks_helper_stage_hygiene_only_when_required() {
        let text = "{\"round\":1,\"case\":\"helper\",\"success\":true,\"feedback_applied\":2,\"helper_stage_feedback_by_role\":{\"summary\":[\"task_kind=summary preview=```python / memory_update: keep short\"],\"index\":[\"task_kind=index preview=clean_gist: Check the SmartSteam unattended evolution chain. Return one small improvement and one verifiable evidence item. / Previous SmartSteam evolution ledger summary: previous_rounds=246 / tags: role=index;case=helper;round=1;source_origin=review.change_request / dependency_link: review.change_request / source_origin: review.change_request / validation_timestamp: 1781859101 / retention: keep\"]}}\n";
        let summary = summarize_ledger(text);
        let default_config = Config {
            report_gate: true,
            ..Config::default()
        };
        let strict_config = Config {
            report_gate: true,
            require_clean_helper_stage_feedback: true,
            ..Config::default()
        };

        let default_failures = report_gate_failures(&summary, &default_config, None, None);
        let strict_failures = report_gate_failures(&summary, &strict_config, None, None);

        assert!(default_failures.is_empty(), "{default_failures:?}");
        assert!(
            strict_failures.iter().any(|failure| failure
                .contains("helper stage feedback hygiene violation role=index kind=prompt_echo")),
            "{strict_failures:?}"
        );
    }

    #[test]
    fn report_gate_requires_latest_helper_stage_roles_when_requested() {
        let text = "{\"round\":1,\"case\":\"stale-helper\",\"success\":true,\"feedback_applied\":2,\"helper_stage_feedback_by_role\":{\"summary\":[\"task_kind=summary preview=old summary\"],\"review\":[\"task_kind=review preview=old review\"],\"test-gate\":[\"task_kind=test-gate preview=old test\"]}}\n{\"round\":2,\"case\":\"latest-no-helper\",\"success\":true,\"feedback_applied\":2}\n";
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            required_helper_stage_roles: vec![
                "summary".to_owned(),
                "review".to_owned(),
                "test-gate".to_owned(),
            ],
            required_latest_helper_stage_roles: vec![
                "summary".to_owned(),
                "review".to_owned(),
                "test-gate".to_owned(),
            ],
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert!(
            failures.iter().any(|failure| failure.contains(
                "latest round helper stage feedback missing required roles: summary,review,test-gate"
            )),
            "{failures:?}"
        );
        assert!(
            !failures
                .iter()
                .any(|failure| failure.starts_with("helper stage feedback missing")),
            "{failures:?}"
        );
    }

    #[test]
    fn report_gate_accepts_latest_helper_stage_roles_from_meta() {
        let text = "{\"round\":1,\"case\":\"latest-helper\",\"success\":true,\"feedback_applied\":2,\"meta\":[\"pool_stage_call_answer task_kind=summary role=summary elapsed_ms=111 answer_approx_tokens=4 preview=summary feedback\",\"pool_stage_call_answer task_kind=review role=review elapsed_ms=222 answer_approx_tokens=4 preview=review feedback\",\"pool_stage_call_answer task_kind=test-gate role=test-gate elapsed_ms=333 answer_approx_tokens=8 preview=verdict: pass\"]}\n";
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            required_latest_helper_stage_roles: vec![
                "summary".to_owned(),
                "review".to_owned(),
                "test-gate".to_owned(),
            ],
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert!(failures.is_empty(), "{failures:?}");
    }

    #[test]
    fn report_gate_accepts_latest_final_json_pool_stage_dispatch() {
        let final_json = json_string(
            r#"{"pool_stage_dispatch":[{"task_kind":"summary"},{"task_kind":"router"},{"task_kind":"review"},{"task_kind":"index"},{"task_kind":"test-gate"}]}"#,
        );
        let text = format!(
            "{{\"round\":1,\"case\":\"latest-dispatch\",\"success\":true,\"feedback_applied\":5,\"helper_stage_feedback_by_role\":{{\"summary\":[\"task_kind=summary preview=summary feedback\"],\"router\":[\"task_kind=router preview=router feedback\"],\"review\":[\"task_kind=review preview=review feedback\"],\"index\":[\"task_kind=index preview=index feedback\"],\"test-gate\":[\"task_kind=test-gate preview=verdict: pass\"]}},\"final_preview\":{final_json}}}\n"
        );
        let summary = summarize_ledger(&text);
        let config = Config {
            report_gate: true,
            required_latest_helper_stage_roles: vec![
                "summary".to_owned(),
                "router".to_owned(),
                "review".to_owned(),
                "index".to_owned(),
                "test-gate".to_owned(),
            ],
            require_final_json_pool_stage_dispatch: true,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert!(failures.is_empty(), "{failures:?}");
    }

    #[test]
    fn report_gate_blocks_latest_final_json_pool_stage_dispatch_missing_role() {
        let final_json = json_string(
            r#"{"pool_stage_dispatch":[{"task_kind":"summary"},{"task_kind":"router"},{"task_kind":"review"},{"task_kind":"test-gate"}]}"#,
        );
        let text = format!(
            "{{\"round\":1,\"case\":\"latest-dispatch\",\"success\":true,\"feedback_applied\":5,\"helper_stage_feedback_by_role\":{{\"summary\":[\"task_kind=summary preview=summary feedback\"],\"router\":[\"task_kind=router preview=router feedback\"],\"review\":[\"task_kind=review preview=review feedback\"],\"index\":[\"task_kind=index preview=index feedback\"],\"test-gate\":[\"task_kind=test-gate preview=verdict: pass\"]}},\"final_preview\":{final_json}}}\n"
        );
        let summary = summarize_ledger(&text);
        let config = Config {
            report_gate: true,
            required_latest_helper_stage_roles: vec![
                "summary".to_owned(),
                "router".to_owned(),
                "review".to_owned(),
                "index".to_owned(),
                "test-gate".to_owned(),
            ],
            require_final_json_pool_stage_dispatch: true,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert!(
            failures.iter().any(|failure| failure.contains(
                "latest final_json.pool_stage_dispatch missing required task_kinds: index"
            )),
            "{failures:?}"
        );
    }

    #[test]
    fn report_gate_accepts_legacy_meta_dispatch_when_final_preview_is_truncated() {
        let final_json = json_string(r#"{"pool_stage_dispatch":[{"task_kind":"summary"}..."#);
        let text = format!(
            "{{\"round\":1,\"case\":\"legacy-dispatch\",\"success\":true,\"feedback_applied\":5,\"helper_stage_feedback_by_role\":{{\"summary\":[\"task_kind=summary preview=summary feedback\"],\"router\":[\"task_kind=router preview=router feedback\"],\"review\":[\"task_kind=review preview=review feedback\"],\"index\":[\"task_kind=index preview=index feedback\"],\"test-gate\":[\"task_kind=test-gate preview=verdict: pass\"]}},\"meta\":[\"pool_stage_dispatch task_kind=summary selected_role=summary\",\"pool_stage_dispatch task_kind=router selected_role=router\",\"pool_stage_dispatch task_kind=review selected_role=review\",\"pool_stage_dispatch task_kind=index selected_role=index\",\"pool_stage_dispatch task_kind=test-gate selected_role=test-gate\"],\"final_preview\":{final_json}}}\n"
        );
        let summary = summarize_ledger(&text);
        let config = Config {
            report_gate: true,
            required_latest_helper_stage_roles: vec![
                "summary".to_owned(),
                "router".to_owned(),
                "review".to_owned(),
                "index".to_owned(),
                "test-gate".to_owned(),
            ],
            require_final_json_pool_stage_dispatch: true,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert!(failures.is_empty(), "{failures:?}");
    }

    #[test]
    fn report_gate_prefers_projected_dispatch_over_legacy_meta_fallback() {
        let text = "{\"round\":1,\"case\":\"projected-dispatch\",\"success\":true,\"feedback_applied\":5,\"helper_stage_feedback_by_role\":{\"summary\":[\"task_kind=summary preview=summary feedback\"],\"router\":[\"task_kind=router preview=router feedback\"],\"review\":[\"task_kind=review preview=review feedback\"],\"index\":[\"task_kind=index preview=index feedback\"],\"test-gate\":[\"task_kind=test-gate preview=verdict: pass\"]},\"meta\":[\"pool_stage_dispatch task_kind=summary selected_role=summary\",\"pool_stage_dispatch task_kind=router selected_role=router\",\"pool_stage_dispatch task_kind=review selected_role=review\",\"pool_stage_dispatch task_kind=index selected_role=index\",\"pool_stage_dispatch task_kind=test-gate selected_role=test-gate\"],\"final_json_pool_stage_dispatch\":[{\"task_kind\":\"summary\"},{\"task_kind\":\"router\"},{\"task_kind\":\"review\"},{\"task_kind\":\"test-gate\"}]}\n";
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            required_latest_helper_stage_roles: vec![
                "summary".to_owned(),
                "router".to_owned(),
                "review".to_owned(),
                "index".to_owned(),
                "test-gate".to_owned(),
            ],
            require_final_json_pool_stage_dispatch: true,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert!(
            failures.iter().any(|failure| failure.contains(
                "latest final_json.pool_stage_dispatch missing required task_kinds: index"
            )),
            "{failures:?}"
        );
    }

    #[test]
    fn report_gate_blocks_latest_helper_stage_feedback_that_ignores_contract() {
        let text = "{\"round\":1,\"case\":\"latest-helper\",\"success\":true,\"feedback_applied\":2,\"helper_stage_feedback_by_role\":{\"summary\":[\"task_kind=summary preview=summary feedback only\"],\"review\":[\"task_kind=review preview=review feedback only\"],\"test-gate\":[\"task_kind=test-gate preview=verdict: pass / validation_command: cargo test\"]}}\n";
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            required_latest_helper_stage_roles: vec![
                "summary".to_owned(),
                "review".to_owned(),
                "test-gate".to_owned(),
            ],
            require_useful_latest_helper_stage_feedback: true,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("feedback for summary is not actionable")),
            "{failures:?}"
        );
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("feedback for review is not actionable")),
            "{failures:?}"
        );
        assert!(
            !failures
                .iter()
                .any(|failure| failure.contains("feedback for test-gate is not actionable")),
            "{failures:?}"
        );
    }

    #[test]
    fn report_gate_blocks_latest_review_feedback_with_placeholder_fields() {
        let text = "{\"round\":1,\"case\":\"latest-helper\",\"success\":true,\"feedback_applied\":2,\"helper_stage_feedback_by_role\":{\"review\":[\"task_kind=review preview=risk: Highest concrete code or behavior risk. / change_request: Smallest improvement to make next. / verification: One check that would prove the change.\"]}}\n";
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            required_latest_helper_stage_roles: vec!["review".to_owned()],
            require_useful_latest_helper_stage_feedback: true,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert!(
            failures.iter().any(|failure| failure.contains(
                "latest round helper stage feedback for review contains placeholder fields: change_request,risk,verification"
            )),
            "{failures:?}"
        );
        assert_eq!(
            summary
                .helper_stage_contract_by_role
                .get("review")
                .map(|summary| summary.useful),
            Some(false)
        );
    }

    #[test]
    fn report_gate_accepts_useful_latest_helper_stage_feedback() {
        let text = "{\"round\":1,\"case\":\"latest-helper\",\"success\":true,\"feedback_applied\":2,\"helper_stage_feedback_by_role\":{\"summary\":[\"task_kind=summary preview=memory_update: keep remote Metal evidence / next_context: use one quality worker / duplicate_guard: do not spawn extra 12B\"],\"review\":[\"task_kind=review preview=risk: helper output may be stale / change_request: require latest roles / verification: run evolution-loop tests\"],\"index\":[\"task_kind=index preview=clean_gist: remote model pool is one 12B plus helpers / tags: gemma,model-pool,index / retention: keep\"],\"test-gate\":[\"task_kind=test-gate preview=verdict: pass / validation_command: cargo test --manifest-path tools/evolution-loop/Cargo.toml / failure_kind: none\"]}}\n";
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            required_latest_helper_stage_roles: vec![
                "summary".to_owned(),
                "review".to_owned(),
                "index".to_owned(),
                "test-gate".to_owned(),
            ],
            require_useful_latest_helper_stage_feedback: true,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert!(failures.is_empty(), "{failures:?}");
    }

    #[test]
    fn report_gate_accepts_complete_latest_helper_stage_feedback() {
        let text = "{\"round\":1,\"case\":\"latest-helper\",\"success\":true,\"feedback_applied\":2,\"helper_stage_feedback_by_role\":{\"summary\":[\"task_kind=summary preview=memory_update: keep remote Metal evidence / next_context: use one quality worker / duplicate_guard: do not spawn extra 12B\"],\"review\":[\"task_kind=review preview=risk: helper output may be stale / change_request: require latest roles / verification: run evolution-loop tests\"],\"index\":[\"task_kind=index preview=clean_gist: remote model pool is one 12B plus helpers / tags: role=index;case=latest-helper;round=1;primary=present;final_json=present;dependency=review.change_request;source_origin=review.change_request;validation_timestamp=1781770123 / dependency_link: review.change_request / source_origin: review.change_request / validation_timestamp: 1781770123 / retention: keep\"],\"test-gate\":[\"task_kind=test-gate preview=verdict: pass / validation_command: cargo test --manifest-path tools/evolution-loop/Cargo.toml\"]}}\n";
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            required_latest_helper_stage_roles: vec![
                "summary".to_owned(),
                "review".to_owned(),
                "index".to_owned(),
                "test-gate".to_owned(),
            ],
            require_complete_latest_helper_stage_feedback: true,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert!(failures.is_empty(), "{failures:?}");
    }

    #[test]
    fn report_gate_uses_structured_contract_fields_for_completeness() {
        let text = "{\"round\":1,\"case\":\"latest-helper\",\"success\":true,\"feedback_applied\":2,\"helper_stage_feedback_by_role\":{\"review\":[\"task_kind=review preview=risk: short preview only\"]},\"helper_stage_contract_by_role\":{\"review\":{\"fields\":{\"risk\":\"short preview only\",\"change_request\":\"persist helper fields\",\"verification\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"},\"matched_markers\":[\"risk\",\"change_request\",\"verification\"],\"expected_markers\":[\"risk\",\"change_request\",\"verification\"]}}}\n";
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            required_latest_helper_stage_roles: vec!["review".to_owned()],
            require_complete_latest_helper_stage_feedback: true,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert!(failures.is_empty(), "{failures:?}");
    }

    #[test]
    fn report_gate_accepts_explicit_no_risk_review_feedback() {
        let text = "{\"round\":1,\"case\":\"latest-helper\",\"success\":true,\"feedback_applied\":2,\"helper_stage_feedback_by_role\":{\"review\":[\"task_kind=review preview=risk: None / change_request: keep current worker routing / verification: cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"]}}\n";
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            required_latest_helper_stage_roles: vec!["review".to_owned()],
            require_complete_latest_helper_stage_feedback: true,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert!(failures.is_empty(), "{failures:?}");
    }

    #[test]
    fn report_gate_blocks_incomplete_latest_review_feedback() {
        let text = "{\"round\":1,\"case\":\"latest-helper\",\"success\":true,\"feedback_applied\":2,\"helper_stage_feedback_by_role\":{\"review\":[\"task_kind=review preview=risk: helper output may be stale / change_request: require latest roles\"]}}\n";
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            required_latest_helper_stage_roles: vec!["review".to_owned()],
            require_complete_latest_helper_stage_feedback: true,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert!(
            failures.iter().any(|failure| failure.contains(
                "latest round helper stage feedback for review missing required fields: verification"
            )),
            "{failures:?}"
        );
    }

    #[test]
    fn report_gate_accepts_passing_test_gate_without_failure_kind() {
        let text = "{\"round\":1,\"case\":\"latest-helper\",\"success\":true,\"feedback_applied\":2,\"helper_stage_feedback_by_role\":{\"test-gate\":[\"task_kind=test-gate preview=verdict: pass / validation_command: cargo test --manifest-path tools/evolution-loop/Cargo.toml\"]}}\n";
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            required_latest_helper_stage_roles: vec!["test-gate".to_owned()],
            require_complete_latest_helper_stage_feedback: true,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert!(failures.is_empty(), "{failures:?}");
    }

    #[test]
    fn report_gate_blocks_failing_test_gate_without_failure_kind() {
        let text = "{\"round\":1,\"case\":\"latest-helper\",\"success\":true,\"feedback_applied\":2,\"helper_stage_feedback_by_role\":{\"test-gate\":[\"task_kind=test-gate preview=verdict: fail / validation_command: cargo test --manifest-path tools/evolution-loop/Cargo.toml\"]}}\n";
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            required_latest_helper_stage_roles: vec!["test-gate".to_owned()],
            require_complete_latest_helper_stage_feedback: true,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert!(
            failures.iter().any(|failure| failure.contains(
                "latest round helper stage feedback for test-gate missing required fields: failure_kind"
            )),
            "{failures:?}"
        );
    }

    #[test]
    fn report_gate_does_not_count_skipped_helper_stage_as_feedback() {
        let text = "{\"round\":1,\"case\":\"helper\",\"success\":true,\"feedback_applied\":2,\"meta\":[\"pool_stage_call_answer task_kind=summary role=summary elapsed_ms=111 answer_approx_tokens=4 preview=summary feedback\",\"pool_stage_call_skipped task_kind=test-gate role=test-gate reason=busy\"]}\n";
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            required_helper_stage_roles: vec!["summary".to_owned(), "test-gate".to_owned()],
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert!(
            failures.iter().any(|failure| failure
                .contains("helper stage feedback missing required roles: test-gate")),
            "{failures:?}"
        );
    }

    #[test]
    fn report_gate_passes_when_latest_test_gate_verdict_passes() {
        let text = "{\"round\":1,\"case\":\"helper\",\"success\":true,\"feedback_applied\":2,\"helper_stage_feedback_by_role\":{\"test-gate\":[\"task_kind=test-gate elapsed_ms=111 answer_approx_tokens=4 preview=verdict: pass / validation_command: cargo test\"]}}\n";
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            require_test_gate_pass: true,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);
        let context = prompt_context_text(&summary);
        let json = report_json(&summary, None, None, None, &failures);

        assert!(failures.is_empty(), "{failures:?}");
        assert_eq!(summary.test_gate.latest_verdict.as_deref(), Some("pass"));
        assert_eq!(
            summary.test_gate.latest_validation_command.as_deref(),
            Some("cargo test")
        );
        assert_eq!(
            summary
                .test_gate
                .latest_fields
                .get("validation_command")
                .map(String::as_str),
            Some("cargo test")
        );
        assert_eq!(summary.test_gate.latest_validation_command_safety, "safe");
        assert!(context.contains("latest_test_gate=verdict:pass"));
        assert!(context.contains("validation_command:cargo test"));
        assert!(context.contains("validation_command_safety:safe"));
        assert!(context.contains("fields:validation_command=cargo test;verdict=pass"));
        assert!(json.contains("\"test_gate\":{\"latest_verdict\":\"pass\""));
        assert!(json.contains("\"latest_validation_command\":\"cargo test\""));
        assert!(json.contains("\"latest_validation_command_safety\":\"safe\""));
        assert!(json.contains("\"latest_fields\":{\"validation_command\":\"cargo test\""));
    }

    #[test]
    fn test_gate_summary_uses_structured_fields_from_bullet_feedback() {
        let text = "{\"round\":1,\"case\":\"helper\",\"success\":true,\"feedback_applied\":2,\"helper_stage_feedback_by_role\":{\"test-gate\":[\"task_kind=test-gate preview=- verdict: pass\\n- validation_command: cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\\n- failure_kind: none\"]}}\n";
        let summary = summarize_ledger(text);
        let context = prompt_context_text(&summary);
        let json = report_json(&summary, None, None, None, &[]);

        assert_eq!(summary.test_gate.latest_verdict.as_deref(), Some("pass"));
        assert_eq!(
            summary.test_gate.latest_validation_command.as_deref(),
            Some("cargo test -q --manifest-path tools/evolution-loop/Cargo.toml")
        );
        assert_eq!(summary.test_gate.latest_validation_command_safety, "safe");
        assert_eq!(summary.test_gate.latest_failure_kind, None);
        assert_eq!(
            summary
                .test_gate
                .latest_fields
                .get("verdict")
                .map(String::as_str),
            Some("pass")
        );
        assert!(context.contains("fields:failure_kind=none;validation_command=cargo test -q --manifest-path tools/evolution-loop/Cargo.toml;verdict=pass"));
        assert!(json.contains("\"latest_fields\":{\"failure_kind\":\"none\",\"validation_command\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\""));
    }

    #[test]
    fn test_gate_summary_uses_structured_contract_fields_without_feedback() {
        let text = "{\"round\":1,\"case\":\"helper\",\"success\":true,\"feedback_applied\":2,\"helper_stage_contract_by_role\":{\"test-gate\":{\"fields\":{\"verdict\":\"pass\",\"validation_command\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"},\"matched_markers\":[\"verdict\",\"validation_command\"],\"expected_markers\":[\"verdict\",\"validation_command\",\"failure_kind\"]}}}\n";
        let summary = summarize_ledger(text);
        let context = prompt_context_text(&summary);
        let json = report_json(&summary, None, None, None, &[]);

        assert_eq!(summary.test_gate.latest_verdict.as_deref(), Some("pass"));
        assert_eq!(
            summary.test_gate.latest_validation_command.as_deref(),
            Some("cargo test -q --manifest-path tools/evolution-loop/Cargo.toml")
        );
        assert_eq!(summary.test_gate.latest_validation_command_safety, "safe");
        assert!(context.contains("latest_test_gate=verdict:pass"));
        assert!(context.contains(
            "validation_command:cargo test -q --manifest-path tools/evolution-loop/Cargo.toml"
        ));
        assert!(context.contains("validation_command_safety:safe"));
        assert!(json.contains("\"test_gate\":{\"latest_verdict\":\"pass\""));
        assert!(json.contains("\"latest_validation_command\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\""));
        assert!(json.contains("\"latest_validation_command_safety\":\"safe\""));
        assert!(json.contains("\"latest_fields\":{\"validation_command\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\""));
    }

    #[test]
    fn test_gate_summary_prefers_structured_contract_fields_over_preview() {
        let text = "{\"round\":1,\"case\":\"helper\",\"success\":true,\"feedback_applied\":2,\"helper_stage_feedback_by_role\":{\"test-gate\":[\"task_kind=test-gate preview=verdict: fail / validation_command: cargo run -- rm -rf target / failure_kind: stale_preview\"]},\"helper_stage_contract_by_role\":{\"test-gate\":{\"fields\":{\"verdict\":\"pass\",\"validation_command\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\",\"failure_kind\":\"none\"},\"matched_markers\":[\"verdict\",\"validation_command\",\"failure_kind\"],\"expected_markers\":[\"verdict\",\"validation_command\",\"failure_kind\"]}}}\n";
        let summary = summarize_ledger(text);

        assert_eq!(summary.test_gate.latest_verdict.as_deref(), Some("pass"));
        assert_eq!(
            summary.test_gate.latest_validation_command.as_deref(),
            Some("cargo test -q --manifest-path tools/evolution-loop/Cargo.toml")
        );
        assert_eq!(summary.test_gate.latest_validation_command_safety, "safe");
        assert_eq!(summary.test_gate.latest_failure_kind, None);
    }

    #[test]
    fn report_gate_blocks_warn_or_fail_test_gate_verdict() {
        let warn = summarize_ledger(
            "{\"round\":1,\"case\":\"helper\",\"success\":true,\"feedback_applied\":2,\"helper_stage_feedback_by_role\":{\"test-gate\":[\"task_kind=test-gate preview=verdict: warn / failure_kind: missing_validation\"]}}\n",
        );
        let fail = summarize_ledger(
            "{\"round\":1,\"case\":\"helper\",\"success\":true,\"feedback_applied\":2,\"helper_stage_feedback_by_role\":{\"test-gate\":[\"task_kind=test-gate preview=verdict=fail / failure_kind: compile_error\"]}}\n",
        );
        let config = Config {
            report_gate: true,
            require_test_gate_pass: true,
            ..Config::default()
        };

        let warn_failures = report_gate_failures(&warn, &config, None, None);
        let fail_failures = report_gate_failures(&fail, &config, None, None);

        assert!(
            warn_failures
                .iter()
                .any(|failure| failure.contains("verdict is warn")),
            "{warn_failures:?}"
        );
        assert!(
            fail_failures
                .iter()
                .any(|failure| failure.contains("verdict is fail")),
            "{fail_failures:?}"
        );
    }

    #[test]
    fn report_gate_blocks_missing_test_gate_verdict_when_required() {
        let text = "{\"round\":1,\"case\":\"helper\",\"success\":true,\"feedback_applied\":2,\"helper_stage_feedback_by_role\":{\"test-gate\":[\"task_kind=test-gate elapsed_ms=111 preview=validation_command: cargo test\"]}}\n";
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            require_test_gate_pass: true,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("test-gate helper verdict missing")),
            "{failures:?}"
        );
    }

    #[test]
    fn report_gate_requires_safe_test_gate_validation_command_when_requested() {
        let safe = summarize_ledger(
            "{\"round\":1,\"case\":\"helper\",\"success\":true,\"feedback_applied\":2,\"helper_stage_feedback_by_role\":{\"test-gate\":[\"task_kind=test-gate preview=verdict: pass / validation_command: cargo check --manifest-path tools/evolution-loop/Cargo.toml\"]}}\n",
        );
        let unsafe_command = summarize_ledger(
            "{\"round\":1,\"case\":\"helper\",\"success\":true,\"feedback_applied\":2,\"helper_stage_feedback_by_role\":{\"test-gate\":[\"task_kind=test-gate preview=verdict: pass / validation_command: cargo run -- rm -rf target\"]}}\n",
        );
        let missing = summarize_ledger(
            "{\"round\":1,\"case\":\"helper\",\"success\":true,\"feedback_applied\":2,\"helper_stage_feedback_by_role\":{\"test-gate\":[\"task_kind=test-gate preview=verdict: pass / validation_command: none\"]}}\n",
        );
        let config = Config {
            report_gate: true,
            require_safe_test_gate_validation_command: true,
            ..Config::default()
        };

        let safe_failures = report_gate_failures(&safe, &config, None, None);
        let unsafe_failures = report_gate_failures(&unsafe_command, &config, None, None);
        let missing_failures = report_gate_failures(&missing, &config, None, None);

        assert_eq!(safe.test_gate.latest_validation_command_safety, "safe");
        assert_eq!(
            unsafe_command.test_gate.latest_validation_command_safety,
            "unsafe"
        );
        assert_eq!(
            missing.test_gate.latest_validation_command_safety,
            "missing"
        );
        assert!(safe_failures.is_empty(), "{safe_failures:?}");
        assert!(
            unsafe_failures
                .iter()
                .any(|failure| failure.contains("validation_command is unsafe")),
            "{unsafe_failures:?}"
        );
        assert!(
            missing_failures
                .iter()
                .any(|failure| failure.contains("validation_command is missing")),
            "{missing_failures:?}"
        );
    }

    #[test]
    fn report_gate_passes_when_latest_test_gate_validation_run_passed() {
        let text = "{\"round\":1,\"case\":\"validation\",\"success\":true,\"feedback_applied\":2,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"safe\",\"validation_command_preview\":\"cargo test --manifest-path tools/evolution-loop/Cargo.toml\",\"validation_status_code\":0}\n";
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            require_test_gate_validation_run: true,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert!(failures.is_empty(), "{failures:?}");
    }

    #[test]
    fn report_gate_blocks_test_gate_validation_run_from_configured_source() {
        let text = "{\"round\":1,\"case\":\"validation\",\"success\":true,\"feedback_applied\":2,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"configured\",\"validation_command_safety\":\"safe\",\"validation_command_preview\":\"cargo test --manifest-path tools/evolution-loop/Cargo.toml\",\"validation_status_code\":0}\n";
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            require_test_gate_validation_run: true,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("source is configured, expected test-gate")),
            "{failures:?}"
        );
    }

    #[test]
    fn report_gate_blocks_test_gate_validation_run_with_unsafe_command() {
        let text = "{\"round\":1,\"case\":\"validation\",\"success\":true,\"feedback_applied\":2,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"unsafe\",\"validation_command_preview\":\"cargo run -- rm -rf target\",\"validation_status_code\":0}\n";
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            require_test_gate_validation_run: true,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("command safety is unsafe, expected safe")),
            "{failures:?}"
        );
    }

    #[test]
    fn report_gate_blocks_test_gate_validation_run_that_was_not_checked() {
        let text = "{\"round\":1,\"case\":\"validation\",\"success\":true,\"feedback_applied\":2,\"validation_checked\":false,\"validation_passed\":false,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"safe\",\"validation_command_preview\":\"cargo test --manifest-path tools/evolution-loop/Cargo.toml\",\"validation_status_code\":0}\n";
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            require_test_gate_validation_run: true,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("validation was not checked")),
            "{failures:?}"
        );
    }

    #[test]
    fn report_gate_blocks_test_gate_validation_run_that_failed() {
        let text = "{\"round\":1,\"case\":\"validation\",\"success\":true,\"feedback_applied\":2,\"validation_checked\":true,\"validation_passed\":false,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"safe\",\"validation_command_preview\":\"cargo test --manifest-path tools/evolution-loop/Cargo.toml\",\"validation_status_code\":7}\n";
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            require_test_gate_validation_run: true,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("validation failed: status=7")),
            "{failures:?}"
        );
    }

    #[test]
    fn report_gate_blocks_missing_test_gate_validation_run_evidence() {
        let text =
            "{\"round\":1,\"case\":\"validation\",\"success\":true,\"feedback_applied\":2}\n";
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            require_test_gate_validation_run: true,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("validation result missing")),
            "{failures:?}"
        );
    }

    #[test]
    fn report_gate_passes_when_latest_configured_validation_run_passed() {
        let text = "{\"round\":1,\"case\":\"validation\",\"success\":true,\"feedback_applied\":2,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"configured\",\"validation_command_safety\":\"explicit\",\"validation_command_preview\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\",\"validation_status_code\":0}\n";
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            require_configured_validation_run: true,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert!(failures.is_empty(), "{failures:?}");
    }

    #[test]
    fn report_gate_blocks_configured_validation_run_from_test_gate_source() {
        let text = "{\"round\":1,\"case\":\"validation\",\"success\":true,\"feedback_applied\":2,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"safe\",\"validation_command_preview\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\",\"validation_status_code\":0}\n";
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            require_configured_validation_run: true,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("source is test-gate, expected configured")),
            "{failures:?}"
        );
    }

    #[test]
    fn report_gate_blocks_configured_validation_run_that_was_not_checked() {
        let text = "{\"round\":1,\"case\":\"validation\",\"success\":true,\"feedback_applied\":2,\"validation_checked\":false,\"validation_passed\":false,\"validation_command_source\":\"configured\",\"validation_command_safety\":\"explicit\",\"validation_command_preview\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\",\"validation_status_code\":0}\n";
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            require_configured_validation_run: true,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("configured validation was not checked")),
            "{failures:?}"
        );
    }

    #[test]
    fn test_gate_validation_command_safety_is_conservative() {
        assert_eq!(
            validation::test_gate_validation_command_safety(Some(
                "cargo test -q --manifest-path tools/evolution-loop/Cargo.toml"
            )),
            "safe"
        );
        assert_eq!(
            validation::test_gate_validation_command_safety(Some("cargo check --workspace")),
            "safe"
        );
        assert_eq!(
            validation::test_gate_validation_command_safety(Some(
                "cargo clippy --all-targets -- -D warnings"
            )),
            "safe"
        );
        assert_eq!(
            validation::test_gate_validation_command_safety(Some("cargo fmt --check")),
            "safe"
        );
        assert_eq!(
            validation::test_gate_validation_command_safety(Some("cargo fmt")),
            "unsafe"
        );
        assert_eq!(
            validation::test_gate_validation_command_safety(Some("cargo test; Remove-Item target")),
            "unsafe"
        );
        assert_eq!(
            validation::test_gate_validation_command_safety(Some("cargo clippy --fix")),
            "unsafe"
        );
        assert_eq!(
            validation::test_gate_validation_command_safety(None),
            "missing"
        );
    }

    #[test]
    fn report_gate_uses_latest_test_gate_feedback_for_verdict() {
        let text = "{\"round\":1,\"case\":\"helper\",\"success\":true,\"feedback_applied\":2,\"helper_stage_feedback_by_role\":{\"test-gate\":[\"task_kind=test-gate preview=verdict: pass / validation_command: cargo test\",\"task_kind=test-gate preview=validation_command: cargo test --all\"]}}\n";
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            require_test_gate_pass: true,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert_eq!(summary.test_gate.latest_verdict, None);
        assert_eq!(
            summary.test_gate.latest_validation_command.as_deref(),
            Some("cargo test --all")
        );
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("test-gate helper verdict missing")),
            "{failures:?}"
        );
    }

    #[test]
    fn report_gate_passes_with_default_evidence() {
        let text = "{\"round\":1,\"case\":\"ok\",\"success\":true,\"feedback_applied\":2}\n";
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            ..Config::default()
        };

        assert!(report_gate_failures(&summary, &config, None, None).is_empty());
    }

    #[test]
    fn report_gate_blocks_pool_capacity_when_requested() {
        let text = "{\"round\":1,\"case\":\"ok\",\"success\":true,\"feedback_applied\":2}\n";
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            pool_capacity_gate: true,
            ..Config::default()
        };
        let pool_status = pool_artifacts::parse_status(
            "{\"launch_allowed\":false,\"launch_block_reason\":\"quality_worker_down\",\"capacity\":{\"expansion_allowed\":false,\"recommendation\":\"restore_quality_gate_first\"},\"workers\":[{\"role\":\"quality\",\"tcp_reachable\":false,\"health_ok\":false}]}\n",
        );

        let failures = report_gate_failures(&summary, &config, Some(&pool_status), None);

        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("model pool capacity blocked expansion"))
        );
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("recommendation=restore_quality_gate_first"))
        );
    }

    #[test]
    fn report_gate_names_failed_thresholds() {
        let text = "{\"round\":1,\"case\":\"bad\",\"success\":false,\"feedback_applied\":0,\"rust_check_checked\":false,\"rust_check_feedback_applied\":0}\n\
{\"round\":1,\"case\":\"bad-again\",\"success\":false,\"feedback_applied\":0,\"rust_check_checked\":false,\"rust_check_feedback_applied\":0}\n";
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            min_report_rounds: 3,
            min_success_rate: Some(75.0),
            min_feedback_total: Some(1),
            min_rust_checks: Some(1),
            min_rust_feedback_total: Some(1),
            strict_ledger_hygiene: true,
            require_last_success: true,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert!(failures.iter().any(|failure| failure.contains("rounds 2")));
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("success rate 0.0%"))
        );
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("feedback_applied 0"))
        );
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("rust_check checked 0"))
        );
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("rust_check_feedback_applied 0"))
        );
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("duplicate round"))
        );
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("non-monotonic round"))
        );
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("latest round failed"))
        );
    }

    #[test]
    fn eval_ledger_summary_maps_report_summary_fields_for_report_gate() {
        let text = "{\"round\":1,\"case\":\"ok\",\"success\":true,\"runtime_tokens\":10,\"runtime_model\":\"google/gemma\",\"answer\":\"ok\",\"elapsed_ms\":100,\"feedback_applied\":2,\"rust_check_checked\":true,\"rust_check_passed\":true,\"rust_check_feedback_applied\":1,\"validation_checked\":true,\"validation_passed\":true,\"self_improve_passed\":true,\"state_gate_checked\":true,\"state_gate_passed\":true,\"trace_gate_checked\":true,\"trace_gate_passed\":true}\n\
{\"round\":3,\"case\":\"truncated\",\"success\":false,\"error\":\"/v1/business-cycle-stream stream truncated before terminal event\",\"runtime_tokens\":20,\"elapsed_ms\":200,\"feedback_applied\":0,\"rust_check_checked\":true,\"rust_check_passed\":false,\"rust_check_feedback_applied\":0,\"validation_checked\":true,\"validation_passed\":false,\"self_improve_passed\":false,\"state_gate_checked\":true,\"state_gate_passed\":false,\"trace_gate_checked\":true,\"trace_gate_passed\":false}\n\
{\"round\":3,\"case\":\"missing-final\",\"success\":true,\"error\":\"stream ended without final event\",\"runtime_tokens\":0,\"runtime_model\":\"google/gemma\",\"answer\":\"Runtime backend error: failed to read response\",\"elapsed_ms\":300,\"feedback_applied\":1}\n\
{\"case\":\"missing-round\",\"success\":false}\n";
        let summary = summarize_ledger(text);

        let eval = eval_ledger_summary(&summary);

        assert_eq!(eval.total_rounds, 4);
        assert_eq!(eval.unique_rounds, 2);
        assert_eq!(eval.duplicate_rounds, 1);
        assert_eq!(eval.non_monotonic_rounds, 1);
        assert_eq!(eval.missing_rounds, 1);
        assert_eq!(eval.round_gaps, 1);
        assert_eq!(eval.max_round, Some(3));
        assert_eq!(eval.successful_rounds, 2);
        assert_eq!(eval.failed_rounds, 2);
        assert_eq!(eval.runtime_tokens_total, 30);
        assert_eq!(eval.runtime_token_items, 3);
        assert_eq!(eval.elapsed_ms_total, 600);
        assert_eq!(eval.elapsed_ms_items, 3);
        assert_eq!(eval.feedback_applied_total, 3);
        assert_eq!(eval.feedback_items, 3);
        assert_eq!(eval.rust_check_checked, 2);
        assert_eq!(eval.rust_check_passed, 1);
        assert_eq!(eval.rust_check_feedback_applied_total, 1);
        assert_eq!(eval.rust_check_feedback_items, 2);
        assert_eq!(eval.validation_checked, 2);
        assert_eq!(eval.validation_passed, 1);
        assert_eq!(eval.self_improve_checked, 2);
        assert_eq!(eval.self_improve_passed, 1);
        assert_eq!(eval.state_gate_checked, 2);
        assert_eq!(eval.state_gate_passed, 1);
        assert_eq!(eval.trace_gate_checked, 2);
        assert_eq!(eval.trace_gate_passed, 1);
        assert_eq!(eval.runtime_response_failures, 1);
        assert_eq!(eval.stream_truncations, 1);
        assert_eq!(eval.missing_final_failures, 1);
        assert_eq!(eval.missing_runtime_models, 0);
        assert_eq!(eval.zero_runtime_tokens, 0);
        assert_eq!(eval.context_noise_penalty_total, 0.0);
        assert_eq!(eval.context_noise_penalty_max, 0.0);
        assert_eq!(eval.last_success, Some(false));
    }

    #[test]
    fn report_gate_threshold_adapter_emits_each_eval_breach_once() {
        let text = "{\"round\":1,\"case\":\"bad\",\"success\":false,\"feedback_applied\":0,\"rust_check_checked\":false,\"rust_check_feedback_applied\":0}\n\
{\"round\":1,\"case\":\"bad-again\",\"success\":false,\"feedback_applied\":0,\"rust_check_checked\":false,\"rust_check_feedback_applied\":0}\n";
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            min_report_rounds: 3,
            min_success_rate: Some(75.0),
            min_feedback_total: Some(1),
            min_rust_checks: Some(1),
            min_rust_feedback_total: Some(1),
            strict_ledger_hygiene: true,
            require_last_success: true,
            ..Config::default()
        };

        let failures = report_gate_threshold_failures(&summary, &config);
        let count = |expected: &str| {
            failures
                .iter()
                .filter(|failure| failure.as_str() == expected)
                .count()
        };

        assert_eq!(count("rounds 2 below minimum 3"), 1, "{failures:?}");
        assert_eq!(
            count("success rate 0.0% below minimum 75.0%"),
            1,
            "{failures:?}"
        );
        assert_eq!(
            count("feedback_applied 0 below minimum 1"),
            1,
            "{failures:?}"
        );
        assert_eq!(
            count("rust_check checked 0 below minimum 1"),
            1,
            "{failures:?}"
        );
        assert_eq!(
            count("rust_check_feedback_applied 0 below minimum 1"),
            1,
            "{failures:?}"
        );
        assert_eq!(
            count("ledger has 1 duplicate round record(s)"),
            1,
            "{failures:?}"
        );
        assert_eq!(
            count("ledger has 1 non-monotonic round record(s)"),
            1,
            "{failures:?}"
        );
        assert_eq!(
            count("latest round failed: round=1 case=bad-again"),
            1,
            "{failures:?}"
        );
    }

    #[test]
    fn report_gate_threshold_failures_keep_eval_breakdown_wording() {
        let text = "{\"round\":1,\"case\":\"bad-runtime\",\"success\":true,\"runtime_tokens\":0,\"feedback_applied\":0,\"answer\":\"Runtime backend error: failed to read response\",\"rust_check_checked\":false,\"rust_check_feedback_applied\":0}\n\
{\"round\":2,\"case\":\"truncated\",\"success\":false,\"error\":\"/v1/business-cycle-stream stream truncated before terminal event\",\"feedback_applied\":0,\"rust_check_checked\":false,\"rust_check_feedback_applied\":0}\n\
{\"round\":3,\"case\":\"missing-final\",\"success\":false,\"error\":\"stream ended without final event\",\"feedback_applied\":0,\"rust_check_checked\":false,\"rust_check_feedback_applied\":0}\n";
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            min_feedback_total: Some(1),
            min_rust_checks: Some(1),
            min_rust_feedback_total: Some(1),
            require_last_success: true,
            ..Config::default()
        };

        let failures = report_gate_threshold_failures(&summary, &config);

        assert!(
            failures.contains(&"feedback_applied 0 below minimum 1".to_owned()),
            "{failures:?}"
        );
        assert!(
            failures.contains(&"rust_check checked 0 below minimum 1".to_owned()),
            "{failures:?}"
        );
        assert!(
            failures.contains(&"rust_check_feedback_applied 0 below minimum 1".to_owned()),
            "{failures:?}"
        );
        assert!(
            failures.contains(&"stream truncation failures 1 above maximum 0".to_owned()),
            "{failures:?}"
        );
        assert!(
            failures.contains(&"missing final-event failures 1 above maximum 0".to_owned()),
            "{failures:?}"
        );
        assert!(
            failures.contains(&"runtime response failures 1 above maximum 0".to_owned()),
            "{failures:?}"
        );
        assert!(
            failures.contains(&"latest round failed: round=3 case=missing-final".to_owned()),
            "{failures:?}"
        );
    }

    #[test]
    fn strict_report_gate_blocks_skipped_round_numbers() {
        let text = "{\"round\":1,\"case\":\"ok\",\"success\":true,\"feedback_applied\":1}\n\
{\"round\":3,\"case\":\"gap\",\"success\":true,\"feedback_applied\":1}\n";
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            strict_ledger_hygiene: true,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert_eq!(summary.round_gaps, 1);
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("missing round number"))
        );
    }

    #[test]
    fn report_gate_blocks_stream_terminal_failures_by_default() {
        let text = "{\"round\":1,\"case\":\"truncated\",\"success\":false,\"error\":\"/v1/business-cycle-stream stream truncated before terminal event\"}\n\
{\"round\":2,\"case\":\"missing-final\",\"success\":false,\"error\":\"stream ended without final event\"}\n";
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            require_last_success: false,
            min_feedback_total: None,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("stream truncation failures 1 above maximum 0"))
        );
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("missing final-event failures 1 above maximum 0"))
        );
    }

    #[test]
    fn report_gate_can_allow_known_stream_terminal_failures() {
        let text = "{\"round\":1,\"case\":\"truncated\",\"success\":false,\"error\":\"/v1/business-cycle-stream stream truncated before terminal event\"}\n\
{\"round\":2,\"case\":\"missing-final\",\"success\":false,\"error\":\"stream ended without final event\"}\n";
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            require_last_success: false,
            min_feedback_total: None,
            max_stream_truncations: 1,
            max_missing_final: 1,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert!(failures.is_empty(), "{failures:?}");
    }

    #[test]
    fn report_gate_blocks_wrapped_runtime_backend_errors_by_default() {
        let text = "{\"round\":1,\"case\":\"bad-runtime\",\"success\":true,\"runtime_tokens\":0,\"feedback_applied\":4,\"final_preview\":\"{\\\"generate\\\":{\\\"answer\\\":\\\"Runtime backend error: failed to read response\\\",\\\"runtime_model\\\":null,\\\"runtime_token_count\\\":0}}\"}\n";
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            require_last_success: false,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert_eq!(summary.runtime_response_failures, 1);
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("runtime response failures 1"))
        );
    }

    #[test]
    fn report_json_includes_summary_and_gate_failures() {
        let text = "{\"round\":1,\"case\":\"bad\",\"success\":false,\"feedback_applied\":0}\n";
        let summary = summarize_ledger(text);
        let failures = vec!["latest round failed".to_owned()];
        let json = report_json(&summary, None, None, None, &failures);

        assert!(json.contains("\"rounds\":1"));
        assert!(json.contains("\"ledger_hygiene\""));
        assert!(json.contains("\"unique_rounds\":1"));
        assert!(json.contains("\"round_gaps\":0"));
        assert!(json.contains("\"runtime_response_failures\":0"));
        assert!(json.contains("\"validation\":{\"passed\":0,\"checked\":0}"));
        assert!(json.contains("\"case\":\"bad\""));
        assert!(json.contains("\"ledger_gate_report_v1\":{\"schema\":\"ledger_gate_report_v1\""));
        assert!(json.contains("\"success_rate\":0.000"));
        assert!(json.contains("\"rust_check_checked\":0"));
        assert!(json.contains("\"rust_check_feedback_applied_total\":0"));
        assert!(json.contains("\"allow_next_round\":false"));
        assert!(json.contains("\"report_gate\":{\"passed\":false"));
        assert!(json.contains("\"latest round failed\""));
    }

    #[test]
    fn report_json_keeps_consumer_contract_for_gate_and_coverage_surface() {
        let text = "{\"round\":20,\"case\":\"strict-command-with-coverage-report\",\"success\":true,\"feedback_applied\":4,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"safe\",\"validation_command_preview\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml -- --strict-coverage\",\"validation_status_code\":0,\"validation_stdout_tail\":\"coverage report path target/evolution/coverage/html/index.html; line coverage 82.4%\"}\n";
        let summary = summarize_ledger(text);
        let failures = vec!["existing gate failure".to_owned()];
        let json = report_json(&summary, None, None, None, &failures);

        assert_occurrences(&json, "\"report_gate\":{\"passed\":", 1);
        assert_occurrences(&json, "\"validation_command_coverage_report_v1\":{", 1);
        assert_occurrences(
            &json,
            "\"self_improve_proposal_action_closure_report_v1\":{",
            1,
        );
        assert_occurrences(&json, "\"adapter_closure_bundle_report_v1\":{", 1);
        assert_contains_in_order(
            &json,
            &[
                "\"validation\":{\"passed\":1,\"checked\":1}",
                "\"validation_command_coverage_report_v1\":{",
                "\"self_improve\":{\"passed\":0,\"checked\":0}",
                "\"self_improve_proposal_action_closure_report_v1\":{",
                "\"strict_report_gate\":{\"passed\":false",
                "\"adapter_closure_bundle_report_v1\":{",
                "\"report_gate\":{\"passed\":false",
            ],
        );
        assert_contains_in_order(
            &json,
            &[
                "\"validation_command_coverage_report_v1\":{\"schema\":\"validation_command_coverage_report_v1\"",
                "\"validation_command\":{\"strict_coverage_requested\":true",
                "\"coverage_tooling_evidence\":[]",
                "\"coverage_report_evidence\":[\"coverage report path target/evolution/coverage/html/index.html; line coverage 82.4%\"]",
                "\"coverage_tooling_or_report_evidence_present\":true",
                "\"coverage_blocked\":false",
                "\"coverage_failure_kind\":\"none\"",
                "\"model_quality_failure_counted\":false",
                "\"failure_reasons\":[]",
                "\"allow_next_round\":true",
            ],
        );
        assert!(json.contains(
            "\"report_gate\":{\"passed\":false,\"failures\":[\"existing gate failure\"]}"
        ));
    }

    #[test]
    fn report_json_exposes_adapter_closure_bundle_without_helper_prose_dependency() {
        let text = "{\"round\":21,\"case\":\"adapter-closure\",\"success\":true,\"feedback_applied\":4,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"safe\",\"validation_command_preview\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml -- --strict-coverage\",\"validation_status_code\":0,\"validation_stdout_tail\":\"coverage report path target/evolution/coverage/html/index.html; line coverage 82.4%\",\"self_improve_passed\":true,\"state_gate_checked\":true,\"state_gate_passed\":true,\"trace_gate_checked\":true,\"trace_gate_passed\":true,\"helper_stage_feedback_by_role\":{\"summary\":[\"task_kind=summary preview=summary feedback\"],\"test-gate\":[\"task_kind=test-gate preview=verdict: pass / validation_command: cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"]},\"helper_stage_contract_by_role\":{\"review\":{\"fields\":{\"risk\":\"none\",\"change_request\":\"keep adapter closure data bundle\",\"verification\":\"cargo test --manifest-path tools/evolution-loop/Cargo.toml\"},\"matched_markers\":[\"risk\",\"change_request\",\"verification\"],\"expected_markers\":[\"risk\",\"change_request\",\"verification\"]}}}\n";
        let summary = summarize_ledger(text);
        let json = report_json(&summary, None, None, None, &[]);

        assert_contains_in_order(
            &json,
            &[
                "\"adapter_closure_bundle_report_v1\":{\"schema\":\"adapter_closure_bundle_report_v1\"",
                "\"consumer_surface\":\"adapter_closure_unattended_continuation\"",
                "\"pure_data_bundle\":true",
                "\"source_report_keys\":[\"ledger_gate_report_v1\",\"strict_report_gate\",\"continuation_gate_report_v1\",\"validation_command_coverage_report_v1\",\"report_gate\"]",
                "\"consumer_decision\":{\"report_gate_passed\":true",
                "\"allow_unattended_continuation\":true",
                "\"latest_round\":21",
                "\"latest_success\":true",
                "\"latest_runtime_response_failure\":false",
            ],
        );
        assert!(json.contains("\"closure_evidence\":{\"rounds\":1,\"success\":1,\"failures\":0"));
        assert!(json.contains(
            "\"validation_command_coverage\":{\"strict_coverage_requested\":true,\"coverage_tooling_evidence_count\":0,\"coverage_report_evidence_count\":1,\"coverage_tooling_or_report_evidence_present\":true}"
        ));
        assert!(json.contains("\"helper_feedback_roles\":[\"summary\",\"test-gate\"]"));
        assert!(
            json.contains("\"helper_stage_contract_roles\":[\"review\",\"summary\",\"test-gate\"]")
        );
        assert!(json.contains("\"test_gate_latest_verdict\":\"pass\""));
        assert!(json.contains("\"test_gate_latest_validation_command_safety\":\"safe\""));
    }

    #[test]
    fn report_json_exposes_worker_window_replacement_status_as_read_only_fixture() {
        let text = "{\"round\":22,\"case\":\"worker-window-status\",\"success\":true,\"feedback_applied\":4}\n";
        let summary = summarize_ledger(text);
        let worker_window_status = worker_window_status::load_status(Some(Path::new(
            "worker-window-status-r21.example.json",
        )))
        .unwrap_or_else(|_| {
            Some(
                worker_window_status::WorkerWindowStatusSummary {
                    source_path: "worker-window-status-r21.example.json".to_owned(),
                    source_status_json: "{\"schema\":\"worker_window_status_v1\",\"side_effects_allowed\":false,\"windows\":[{\"window_id\":\"r20-eval-test\",\"status\":\"paused\",\"polluted\":true,\"clean_room_replacement_required\":true,\"original_window_blocks_assignment\":true},{\"window_id\":\"r21-eval-test\",\"status\":\"clean-room-replacement\",\"clean_room_replacement\":true,\"assignment_allowed\":true},{\"window_id\":\"r21-agent\",\"status\":\"clean-room-replacement\",\"assignment_allowed\":true},{\"window_id\":\"r21-service-cli\",\"status\":\"clean-room-replacement\",\"assignment_allowed\":true}]}".to_owned(),
                    window_count: 4,
                    paused_count: 1,
                    polluted_count: 1,
                    clean_room_replacement_count: 3,
                    replacement_required_count: 1,
                    blocked_original_count: 1,
                    side_effects_allowed: Some(false),
                },
            )
        })
        .unwrap();
        let json = report_json_with_remote_chain(
            &summary,
            None,
            None,
            None,
            None,
            None,
            Some(&worker_window_status),
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
        );

        assert_contains_in_order(
            &json,
            &[
                "\"worker_window_replacement_report_v1\":{\"schema\":\"worker_window_replacement_report_v1\"",
                "\"consumer_surface\":\"clean_room_worker_window_replacement_status\"",
                "\"read_only\":true",
                "\"status_loaded\":true",
                "\"source\":\"external_worker_window_status_json\"",
                "\"window_count\":4",
                "\"paused_count\":1",
                "\"polluted_count\":1",
                "\"clean_room_replacement_count\":3",
                "\"replacement_required_count\":1",
                "\"blocked_original_count\":1",
                "\"side_effects_allowed\":false",
                "\"starts_clean_room_replacement\":false",
                "\"mutates_worker_window_status\":false",
                "\"touches_remote\":false",
                "\"sends_prompt\":false",
            ],
        );
        assert_occurrences(&json, "\"worker_window_replacement_report_v1\":{", 1);
    }

    #[test]
    fn report_json_exposes_clean_room_batch_status_as_report_only_fixture() {
        let text = "{\"round\":25,\"case\":\"clean-room-batch-status\",\"success\":true,\"feedback_applied\":4}\n";
        let summary = summarize_ledger(text);
        let clean_room_batch_status = clean_room_batch_status::CleanRoomBatchStatusSummary {
            source_path: "clean-room-batch-status-r25.example.json".to_owned(),
            source_status_json: "{\"schema\":\"clean_room_batch_status_v1\",\"clean_room_batch_status\":{\"report_only\":true,\"side_effects_allowed\":false,\"r24_status\":\"completed\",\"r25_clean_room_replacements_status\":\"opened\",\"old_polluted_windows_assignment_allowed\":false,\"main_window_owns_ssh\":true,\"main_window_owns_daemon\":true,\"main_window_owns_remote_model_pool\":true,\"main_window_owns_runtime_start_stop\":true,\"worker_runtime_ownership_allowed\":false}}".to_owned(),
            report_only: Some(true),
            side_effects_allowed: Some(false),
            r24_completed: true,
            r24_completed_worker_ids: vec![
                "019ee1c3-ec62-7a92-9c04-27b68ac5f4b9".to_owned(),
            ],
            r25_clean_room_replacements_open: true,
            r25_clean_room_replacement_worker_ids: vec!["R25-clean-room-worker-F".to_owned()],
            old_polluted_windows_blocked: true,
            blocked_old_window_ids: vec!["old-polluted-worker-window".to_owned()],
            main_window_runtime_owner: true,
            worker_runtime_ownership_allowed: false,
        };
        let json = report_json_with_remote_chain(
            &summary,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(&clean_room_batch_status),
            None,
            None,
            &[],
            &[],
            &[],
            &[],
        );

        assert_contains_in_order(
            &json,
            &[
                "\"clean_room_batch_status_report_v1\":{\"schema\":\"clean_room_batch_status_report_v1\"",
                "\"consumer_surface\":\"evolution_loop_report_only_batch_status_closure\"",
                "\"status_loaded\":true",
                "\"report_only\":true",
                "\"side_effects_allowed\":false",
                "\"r24_completed\":true",
                "\"r25_clean_room_replacements_open\":true",
                "\"old_polluted_windows_blocked\":true",
                "\"main_window_runtime_owner\":true",
                "\"worker_runtime_ownership_allowed\":false",
                "\"opens_clean_room_replacement\":false",
                "\"reads_old_thread\":false",
                "\"starts_daemon\":false",
                "\"touches_remote\":false",
                "\"starts_forge\":false",
                "\"starts_web_lab\":false",
            ],
        );
        assert_occurrences(&json, "\"clean_room_batch_status_report_v1\":{", 1);
    }

    #[test]
    fn report_json_exposes_clean_room_handoff_inputs_as_read_only_fixture() {
        let text = "{\"round\":24,\"case\":\"clean-room-handoff\",\"success\":true,\"feedback_applied\":4}\n";
        let summary = summarize_ledger(text);
        let clean_room_handoff = clean_room_handoff::CleanRoomHandoffSummary {
            memory_startup_admission: Some(clean_room_handoff::MemoryStartupAdmissionSummary {
                source_path: "memory-startup-admission-r23.example.json".to_owned(),
                source_json: "{\"schema\":\"memory_startup_admission_status_v1\",\"memory_startup_admission_status\":{\"read_only_contract\":true,\"admission_decision_count\":4,\"store_mutation_count\":0}}".to_owned(),
                read_only_contract: Some(true),
                read_only_review_required: Some(true),
                index_quality_blocker_count: 1,
                index_quality_warning_count: 2,
                index_operation_count: 3,
                index_refresh_count: 1,
                context_rot_risk_count: 2,
                admission_decision_count: 4,
                admission_accepted_count: 2,
                admission_risk_rejection_count: 1,
                migration_live_store_targeted_count: 0,
                adapter_live_write_count: 0,
                live_write_phase_request_count: 0,
                live_store_mutation_requested: false,
                store_mutation_count: 0,
                ndkv_write_allowed: false,
                helper_prose_line_count: 2,
                non_contract_line_count: 3,
                admission_expanded_by_non_contract_evidence: false,
            }),
            agent_replacement_plan: Some(
                clean_room_handoff::AgentCleanRoomReplacementPlanSummary {
                    source_path: "agent-clean-room-replacement-plan-r23.example.json".to_owned(),
                    source_json: "{\"schema\":\"agent_window_context_clean_room_replacement_plan_v1\",\"report_only\":true,\"side_effects_allowed\":false}".to_owned(),
                    report_only: Some(true),
                    pure_data_only: Some(true),
                    side_effects_allowed: Some(false),
                    starts_thread: Some(false),
                    sends_message: Some(false),
                    reads_old_window_payload: Some(false),
                    original_window_follow_up_assignment_allowed: Some(false),
                    clean_room_replacement_plan_required: Some(true),
                    clean_room_replacement_available: Some(true),
                    replacement_prompt_ready: Some(true),
                    follow_up_tasks_only_in_replacement_prompt: Some(true),
                    task_ids: vec!["R24-clean-room-worker-A".to_owned()],
                    evidence_result_ids: vec!["handoff-summary:r23-agent".to_owned()],
                    reason_codes: vec![
                        "window_context_polluted".to_owned(),
                        "paused_by_main_window".to_owned(),
                    ],
                },
            ),
        };
        let json = report_json_with_remote_chain(
            &summary,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(&clean_room_handoff),
            None,
            &[],
            &[],
            &[],
            &[],
        );

        assert_contains_in_order(
            &json,
            &[
                "\"clean_room_handoff_report_v1\":{\"schema\":\"clean_room_handoff_report_v1\"",
                "\"consumer_surface\":\"evolution_loop_report_only_clean_room_handoff\"",
                "\"memory_startup_admission\":{\"loaded\":true",
                "\"read_only_contract\":true",
                "\"admission_decision_count\":4",
                "\"store_mutation_count\":0",
                "\"ndkv_write_allowed\":false",
                "\"agent_clean_room_replacement_plan\":{\"loaded\":true",
                "\"report_only\":true",
                "\"side_effects_allowed\":false",
                "\"replacement_prompt_task_count\":1",
                "\"evidence_result_count\":1",
                "\"reason_code_count\":2",
                "\"starts_clean_room_replacement\":false",
                "\"starts_thread\":false",
                "\"sends_prompt\":false",
                "\"expands_memory_admission\":false",
                "\"mutates_memory_store\":false",
                "\"writes_ndkv\":false",
            ],
        );
        assert_occurrences(&json, "\"clean_room_handoff_report_v1\":{", 1);
    }

    #[test]
    fn report_json_exposes_self_improve_proposal_artifact_as_candidate_only() {
        let text = "{\"round\":26,\"case\":\"self-improve-proposal\",\"success\":true,\"feedback_applied\":4,\"self_improve_passed\":true,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"safe\",\"validation_command_preview\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\",\"helper_stage_contract_by_role\":{\"review\":{\"fields\":{\"risk\":\"none\",\"change_request\":\"project typed self-improve proposal artifact\",\"verification\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}},\"test-gate\":{\"fields\":{\"validation_command\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}}}}\n";
        let summary = summarize_ledger(text);
        let artifact = self_improve_proposal_artifact::from_ledger_text(text);
        let json = report_json_with_remote_chain(
            &summary,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(&artifact),
            &[],
            &[],
            &[],
            &[],
        );

        assert_contains_in_order(
            &json,
            &[
                "\"self_improve_proposal_artifact_v1\":{\"schema\":\"self_improve_proposal_artifact_v1\"",
                "\"candidate_only\":true",
                "\"total_candidate_count\":1",
                "\"suggested_action\":\"project typed self-improve proposal artifact\"",
                "\"validation\":{\"command\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"",
                "\"command_safety\":\"safe\"",
                "\"admission\":{\"status\":\"candidate_report_only\"",
                "\"auto_apply\":false",
                "\"business_improvement_acceptance\":{\"schema\":\"self_improve_proposal_acceptance_v1\"",
                "\"memory_admission_decision\":\"quarantined\"",
                "\"evidence_backed_business_improvement\":false",
                "\"advisory_only\":true",
                "\"side_effects\":{\"applies_code\":false",
                "\"calls_model\":false",
                "\"self_improve_proposal_acceptance_summary_v1\":{\"schema\":\"self_improve_proposal_acceptance_summary_v1\"",
                "\"projected_report_count\":1",
                "\"evidence_backed_business_improvement_count\":0",
                "\"advisory_only_count\":1",
                "\"only_advisory_or_repair\":true",
                "\"self_improve_proposal_action_assignment_v1\":{\"schema\":\"self_improve_proposal_action_assignment_v1\"",
                "\"consumer_surface\":\"evolution_loop_report_only_self_improve_action_assignment\"",
                "\"first_target\":{\"proposal_id\":\"self-improve-r26-helper_contract-projecttypedselfimprovep\"",
                "\"source_round\":26",
                "\"evidence_ids\":[\"ledger.round.26.helper_stage_contract.review.change_request\"]",
                "\"current_memory_admission_decision\":\"quarantined\"",
                "\"validation_checked\":true",
                "\"validation_passed\":true",
                "\"memory_admission_accepted\":false",
                "\"evidence_backed_business_improvement\":false",
                "\"advisory_only\":true",
                "\"assignment\":{\"read_only\":true",
                "\"self_improve_proposal_action_closure_report_v1\":{\"schema\":\"self_improve_proposal_action_closure_report_v1\"",
                "\"consumer_surface\":\"evolution_loop_report_only_self_improve_action_closure\"",
                "\"closed_target_count\":0",
                "\"open_target_count\":1",
                "\"side_effects\":{\"applies_code\":false",
                "\"self_improve_proposal_memory_admission_readiness_report_v1\":{\"schema\":\"self_improve_proposal_memory_admission_readiness_report_v1\"",
                "\"consumer_surface\":\"evolution_loop_report_only_self_improve_memory_admission_readiness\"",
                "\"ready_count\":0",
                "\"blocked_count\":1",
                "\"memory_store_write_allowed\":false",
                "\"ndkv_write_allowed\":false",
                "\"self_improve_proposal_memory_admission_request_report_v1\":{\"schema\":\"self_improve_proposal_memory_admission_request_report_v1\"",
                "\"consumer_surface\":\"evolution_loop_report_only_self_improve_memory_admission_request\"",
                "\"request_count\":0",
                "\"writer_required\":false",
                "\"auto_apply\":false",
                "\"memory_store_write_allowed\":false",
                "\"ndkv_write_allowed\":false",
                "\"self_improve_proposal_memory_admission_decision_report_v1\":{\"schema\":\"self_improve_proposal_memory_admission_decision_report_v1\"",
                "\"consumer_surface\":\"evolution_loop_report_only_self_improve_memory_admission_decision\"",
                "\"admission_writer_preflight_passed\":false",
                "\"explicit_writer_invocation_required\":false",
                "\"admission_write_authorized\":false",
                "\"gate_blocked\":true",
                "\"memory admission request count is zero\"",
                "\"self_improve_proposal_memory_admission_writer_plan_report_v1\":{\"schema\":\"self_improve_proposal_memory_admission_writer_plan_report_v1\"",
                "\"consumer_surface\":\"evolution_loop_report_only_self_improve_memory_admission_writer_plan\"",
                "\"ready_plan_count\":0",
                "\"writer_plan_ready\":false",
                "\"admission_write_authorized\":false",
                "\"writer plan has no requested candidates\"",
                "\"self_improve_proposal_memory_admission_writer_dry_run_report_v1\":{\"schema\":\"self_improve_proposal_memory_admission_writer_dry_run_report_v1\"",
                "\"consumer_surface\":\"evolution_loop_report_only_self_improve_memory_admission_writer_dry_run\"",
                "\"ready_dry_run_count\":0",
                "\"dry_run_ready\":false",
                "\"admission_write_authorized\":false",
                "\"writer dry-run requires ready writer plan\"",
                "\"self_improve_proposal_memory_admission_writer_dry_run_receipt_report_v1\":{\"schema\":\"self_improve_proposal_memory_admission_writer_dry_run_receipt_report_v1\"",
                "\"consumer_surface\":\"evolution_loop_report_only_self_improve_memory_admission_writer_dry_run_receipt\"",
                "\"succeeded_receipt_count\":0",
                "\"dry_run_receipt_ready\":false",
                "\"commit_allowed\":false",
                "\"admission_write_authorized\":false",
                "\"writer dry-run receipt requires ready dry-run manifest\"",
                "\"self_improve_proposal_memory_admission_commit_record_stage_report_v1\":{\"schema\":\"self_improve_proposal_memory_admission_commit_record_stage_report_v1\"",
                "\"consumer_surface\":\"evolution_loop_report_only_self_improve_memory_admission_commit_record_stage\"",
                "\"staged_commit_record_count\":0",
                "\"commit_record_stage_ready\":false",
                "\"commit_allowed\":false",
                "\"admission_write_authorized\":false",
                "\"commit record staging requires ready dry-run receipt report\"",
                "\"self_improve_proposal_memory_admission_commit_approval_request_report_v1\":{\"schema\":\"self_improve_proposal_memory_admission_commit_approval_request_report_v1\"",
                "\"consumer_surface\":\"evolution_loop_report_only_self_improve_memory_admission_commit_approval_request\"",
                "\"requested_commit_approval_count\":0",
                "\"commit_approval_request_ready\":false",
                "\"commit_allowed\":false",
                "\"admission_write_authorized\":false",
                "\"commit approval request requires ready commit record stage\"",
                "\"self_improve_proposal_memory_admission_commit_approval_decision_report_v1\":{\"schema\":\"self_improve_proposal_memory_admission_commit_approval_decision_report_v1\"",
                "\"consumer_surface\":\"evolution_loop_report_only_self_improve_memory_admission_commit_approval_decision\"",
                "\"recorded_approval_decision_count\":0",
                "\"approved_commit_count\":0",
                "\"pending_approval_count\":0",
                "\"commit_approval_decision_ready\":false",
                "\"commit_allowed\":false",
                "\"admission_write_authorized\":false",
                "\"commit approval decision requires ready approval request\"",
                "\"self_improve_proposal_memory_admission_commit_approval_review_packet_report_v1\":{\"schema\":\"self_improve_proposal_memory_admission_commit_approval_review_packet_report_v1\"",
                "\"consumer_surface\":\"evolution_loop_report_only_self_improve_memory_admission_commit_approval_review_packet\"",
                "\"ready_review_packet_count\":0",
                "\"approval_review_packet_ready\":false",
                "\"commit_allowed\":false",
                "\"admission_write_authorized\":false",
                "\"commit approval review packet requires ready approval decision\"",
            ],
        );
        assert_occurrences(&json, "\"self_improve_proposal_artifact_v1\":{", 1);
        assert_occurrences(
            &json,
            "\"self_improve_proposal_acceptance_summary_v1\":{",
            1,
        );
        assert_occurrences(&json, "\"self_improve_proposal_action_assignment_v1\":{", 1);
        assert_occurrences(
            &json,
            "\"self_improve_proposal_action_closure_report_v1\":{",
            1,
        );
        assert_occurrences(
            &json,
            "\"self_improve_proposal_memory_admission_readiness_report_v1\":{",
            1,
        );
        assert_occurrences(
            &json,
            "\"self_improve_proposal_memory_admission_request_report_v1\":{",
            1,
        );
        assert_occurrences(
            &json,
            "\"self_improve_proposal_memory_admission_decision_report_v1\":{",
            1,
        );
        assert_occurrences(
            &json,
            "\"self_improve_proposal_memory_admission_writer_plan_report_v1\":{",
            1,
        );
        assert_occurrences(
            &json,
            "\"self_improve_proposal_memory_admission_writer_dry_run_report_v1\":{",
            1,
        );
        assert_occurrences(
            &json,
            "\"self_improve_proposal_memory_admission_writer_dry_run_receipt_report_v1\":{",
            1,
        );
        assert_occurrences(
            &json,
            "\"self_improve_proposal_memory_admission_commit_record_stage_report_v1\":{",
            1,
        );
        assert_occurrences(
            &json,
            "\"self_improve_proposal_memory_admission_commit_approval_request_report_v1\":{",
            1,
        );
        assert_occurrences(
            &json,
            "\"self_improve_proposal_memory_admission_commit_approval_decision_report_v1\":{",
            1,
        );
        assert_occurrences(
            &json,
            "\"self_improve_proposal_memory_admission_commit_approval_review_packet_report_v1\":{",
            1,
        );
    }

    #[test]
    fn prompt_context_feeds_self_improve_proposal_acceptance_summary_from_ledger() {
        let text = "{\"round\":31,\"case\":\"advisory-proposal\",\"success\":true,\"feedback_applied\":4,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"configured\",\"validation_command_safety\":\"explicit\",\"final_preview\":\"{\\\"self_improve_proposal\\\":{\\\"proposal_id\\\":\\\"r31-advisory\\\",\\\"source_round\\\":31,\\\"evidence_id\\\":\\\"review:r31\\\",\\\"suggested_action\\\":\\\"convert advisory proposal to business evidence\\\",\\\"validation_command\\\":\\\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\\\",\\\"admission_status\\\":\\\"proposed\\\"}}\"}\n";
        let dir = std::env::temp_dir();
        let ledger_path = dir.join(format!(
            "smartsteam-prompt-self-improve-proposal-{}.jsonl",
            std::process::id()
        ));
        let _ = fs::remove_file(&ledger_path);
        fs::write(&ledger_path, text).unwrap();

        let context = prompt_context(&ledger_path).unwrap().unwrap();

        assert!(context.contains(
            "self_improve_proposal_acceptance=source:ledger_artifact candidates_total:1 projected:1 evidence_backed_business:0 advisory_only:1 repair_required:0 accepted_without_business_evidence:0"
        ));
        assert!(context.contains(
            "next_self_improve_should_convert_advisory_to_evidence_backed_business_improvement:true"
        ));
        assert!(context.contains(
            "next_self_improve_requires_checked_passed_validation_and_accepted_memory_admission:true"
        ));
        assert!(context.contains(
            "self_improve_action_assignment=primary:convert_advisory_to_evidence_backed_business_improvement targets:1 first_target:r31-advisory first_round:31 first_evidence_ids:review:r31 first_memory_admission:quarantined first_validation_checked:true first_validation_passed:true first_memory_accepted:false first_business_evidence:false first_advisory_only:true first_require_repair:false first_missing:accepted_memory_admission,evidence_backed_business_improvement"
        ));
        assert!(context.contains(
            "self_improve_action_closure=targets:1 closed:0 open:1 first_target:r31-advisory first_closed:false first_kind:none first_still_requires_memory_admission:true"
        ));

        let _ = fs::remove_file(ledger_path);
    }

    #[test]
    fn prompt_context_marks_closed_no_fail_fast_self_improve_action() {
        let text = "{\"round\":392,\"case\":\"no-fail-fast-proposal\",\"success\":true,\"feedback_applied\":4,\"self_improve_passed\":true,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"safe\",\"validation_command_preview\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\",\"helper_stage_contract_by_role\":{\"review\":{\"fields\":{\"risk\":\"validation stops early\",\"change_request\":\"Update the `validation_command` to include `--no-fail-fast` to ensure comprehensive testing\",\"verification\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}},\"test-gate\":{\"fields\":{\"validation_command\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}}}}\n";
        let summary = summarize_ledger(text);
        let artifact = self_improve_proposal_artifact::from_ledger_text(text);
        let context = prompt_context_text_with_self_improve_proposals(&summary, Some(&artifact));

        assert!(context.contains(
            "self_improve_action_closure=targets:1 closed:1 open:0 first_target:self-improve-r392-helper_contract-updatethevalidationcomma first_closed:true first_kind:test_gate_no_fail_fast first_still_requires_memory_admission:true"
        ));
        assert!(context.contains(
            "self_improve_memory_admission_readiness=targets:1 ready:1 blocked:0 first_target:self-improve-r392-helper_contract-updatethevalidationcomma first_ready:true all_closed_targets_ready:true memory_store_write_allowed:false ndkv_write_allowed:false"
        ));
        assert!(context.contains(
            "self_improve_memory_admission_request=targets:1 requests:1 blocked:0 first_candidate:self-improve-r392-helper_contract-updatethevalidationcomma first_ready:true all_ready_targets_requested:true writer_required:true auto_apply:false memory_store_write_allowed:false ndkv_write_allowed:false"
        ));
        assert!(context.contains(
            "self_improve_memory_admission_decision=targets:1 requests:1 blocked:0 first_candidate:self-improve-r392-helper_contract-updatethevalidationcomma writer_required:true preflight_passed:true explicit_writer_invocation_required:true admission_write_authorized:false gate_blocked:false failure_reasons:none auto_apply:false memory_store_write_allowed:false ndkv_write_allowed:false"
        ));
        assert!(context.contains(
            "self_improve_memory_admission_writer_plan=targets:1 requests:1 plan_items:1 ready:1 blocked:0 first_item:self-improve-r392-helper_contract-updatethevalidationcomma writer_plan_ready:true explicit_writer_invocation_required:true experiment_required:true rollback_required:true validation_required:true admission_write_authorized:false failure_reasons:none auto_apply:false memory_store_write_allowed:false ndkv_write_allowed:false"
        ));
        assert!(context.contains(
            "self_improve_memory_admission_writer_dry_run=targets:1 requests:1 plan_items:1 dry_run_items:1 ready:1 blocked:0 first_item:self-improve-r392-helper_contract-updatethevalidationcomma dry_run_ready:true explicit_writer_invocation_required:true dry_run_required:true experiment_required:true rollback_required:true validation_required:true admission_write_authorized:false failure_reasons:none auto_apply:false memory_store_write_allowed:false ndkv_write_allowed:false"
        ));
        assert!(context.contains(
            "self_improve_memory_admission_writer_dry_run_receipt=targets:1 requests:1 dry_run_items:1 receipt_items:1 succeeded:1 blocked:0 first_item:self-improve-r392-helper_contract-updatethevalidationcomma dry_run_receipt_ready:true explicit_writer_invocation_required:true commit_allowed:false validation_required:true rollback_required:true admission_write_authorized:false failure_reasons:none auto_apply:false memory_store_write_allowed:false ndkv_write_allowed:false"
        ));
        assert!(context.contains(
            "self_improve_memory_admission_commit_record_stage=targets:1 requests:1 receipt_items:1 commit_record_items:1 staged:1 blocked:0 first_item:self-improve-r392-helper_contract-updatethevalidationcomma commit_record_stage_ready:true explicit_writer_invocation_required:true validation_required:true rollback_required:true commit_allowed:false admission_write_authorized:false failure_reasons:none auto_apply:false memory_store_write_allowed:false ndkv_write_allowed:false"
        ));
        assert!(context.contains(
            "self_improve_memory_admission_commit_approval_request=targets:1 requests:1 commit_record_items:1 approval_request_items:1 requested:1 blocked:0 first_item:self-improve-r392-helper_contract-updatethevalidationcomma commit_approval_request_ready:true explicit_commit_approval_required:true validation_required:true rollback_required:true commit_allowed:false admission_write_authorized:false failure_reasons:none auto_apply:false memory_store_write_allowed:false ndkv_write_allowed:false"
        ));
        assert!(context.contains(
            "self_improve_memory_admission_commit_approval_decision=targets:1 requests:1 approval_request_items:1 approval_decision_items:1 recorded:1 approved:0 pending:1 blocked:0 first_item:self-improve-r392-helper_contract-updatethevalidationcomma commit_approval_decision_ready:true explicit_commit_approval_required:true validation_required:true rollback_required:true commit_allowed:false admission_write_authorized:false failure_reasons:none auto_apply:false memory_store_write_allowed:false ndkv_write_allowed:false"
        ));
        assert!(context.contains(
            "self_improve_memory_admission_commit_approval_review_packet=targets:1 requests:1 approval_decision_items:1 review_packet_items:1 ready:1 pending:1 blocked:0 first_item:self-improve-r392-helper_contract-updatethevalidationcomma approval_review_packet_ready:true explicit_operator_approval_required:true validation_required:true rollback_required:true commit_allowed:false admission_write_authorized:false failure_reasons:none auto_apply:false memory_store_write_allowed:false ndkv_write_allowed:false"
        ));
        assert!(!context.contains(
            "next_self_improve_should_convert_advisory_to_evidence_backed_business_improvement:true"
        ));
        assert!(
            context.contains(
                "next_self_improve_should_prepare_memory_admission_for_closed_action:true"
            )
        );
        assert!(context.contains("next_self_improve_should_not_repeat_closed_action:true"));
        assert!(context.contains("next_self_improve_action_assignment_all_targets_closed:true"));
        assert!(
            context.contains("next_self_improve_memory_admission_all_closed_targets_ready:true")
        );
        assert!(context.contains("next_self_improve_should_emit_memory_admission_request:true"));
        assert!(context.contains("next_self_improve_admission_writer_preflight_passed:true"));
        assert!(
            context
                .contains("next_self_improve_should_invoke_explicit_memory_admission_writer:true")
        );
        assert!(
            context
                .contains("next_self_improve_should_dry_run_explicit_memory_admission_writer:true")
        );
        assert!(context.contains(
            "next_self_improve_should_record_memory_admission_writer_dry_run_receipt:true"
        ));
        assert!(
            context.contains("next_self_improve_should_stage_memory_admission_commit_record:true")
        );
        assert!(
            context
                .contains("next_self_improve_should_request_memory_admission_commit_approval:true")
        );
        assert!(context.contains(
            "next_self_improve_should_record_memory_admission_commit_approval_decision:true"
        ));
        assert!(context.contains(
            "next_self_improve_should_review_memory_admission_commit_approval_packet:true"
        ));
    }

    #[test]
    fn prompt_context_flags_self_improve_proposal_repairs() {
        let text = "{\"round\":32,\"case\":\"repair-proposal\",\"success\":true,\"feedback_applied\":4,\"validation_checked\":false,\"validation_passed\":false,\"validation_command_source\":\"configured\",\"validation_command_safety\":\"explicit\",\"final_preview\":\"{\\\"self_improve_proposal\\\":{\\\"proposal_id\\\":\\\"r32-repair\\\",\\\"source_round\\\":32,\\\"evidence_id\\\":\\\"suggestion:r32\\\",\\\"suggested_action\\\":\\\"repair accepted-looking proposal without validation\\\",\\\"validation_command\\\":\\\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\\\",\\\"admission_status\\\":\\\"accepted\\\"}}\"}\n";
        let summary = summarize_ledger(text);
        let artifact = self_improve_proposal_artifact::from_ledger_text(text);
        let context = prompt_context_text_with_self_improve_proposals(&summary, Some(&artifact));

        assert!(context.contains(
            "self_improve_proposal_acceptance=source:ledger_artifact candidates_total:1 projected:1 evidence_backed_business:0 advisory_only:0 repair_required:1 accepted_without_business_evidence:1"
        ));
        assert!(
            context.contains(
                "next_self_improve_should_repair_unvalidated_or_unaccepted_proposals:true"
            )
        );
        assert!(!context.contains(
            "next_self_improve_should_convert_advisory_to_evidence_backed_business_improvement:true"
        ));
    }

    #[test]
    fn prompt_context_omits_self_improve_proposal_line_when_no_candidates() {
        let text = "{\"round\":33,\"case\":\"ordinary-round\",\"success\":true,\"feedback_applied\":4,\"validation_checked\":true,\"validation_passed\":true}\n";
        let summary = summarize_ledger(text);
        let artifact = self_improve_proposal_artifact::from_ledger_text(text);
        let context = prompt_context_text_with_self_improve_proposals(&summary, Some(&artifact));

        assert_eq!(artifact.total_candidate_count, 0);
        assert!(!context.contains("self_improve_proposal_acceptance="));
        assert!(!context.contains("next_self_improve_should_"));
    }

    #[test]
    fn prompt_context_omits_self_improve_proposal_line_when_r78_generic_noop_is_filtered() {
        let text = "{\"round\":411,\"case\":\"noop-review\",\"success\":true,\"feedback_applied\":4,\"self_improve_passed\":true,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"safe\",\"validation_command_preview\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\",\"helper_stage_contract_by_role\":{\"review\":{\"fields\":{\"risk\":\"none\",\"change_request\":\"No change suggested in the primary_answer for this round.\",\"verification\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}},\"test-gate\":{\"fields\":{\"validation_command\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}}}}\n\
{\"round\":412,\"case\":\"generic-review\",\"success\":true,\"feedback_applied\":4,\"self_improve_passed\":true,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"safe\",\"validation_command_preview\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\",\"helper_stage_contract_by_role\":{\"review\":{\"fields\":{\"risk\":\"none\",\"change_request\":\"Small next change grounded in the same evidence\",\"verification\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}},\"test-gate\":{\"fields\":{\"validation_command\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}}}}\n\
{\"round\":414,\"case\":\"negative-generic-review\",\"success\":true,\"feedback_applied\":4,\"self_improve_passed\":true,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"safe\",\"validation_command_preview\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\",\"helper_stage_contract_by_role\":{\"review\":{\"fields\":{\"risk\":\"none\",\"change_request\":\"No small next change is grounded in the same evidence; the previous round already identified a change request regarding `--no-fail-fast`.\",\"verification\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}},\"test-gate\":{\"fields\":{\"validation_command\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}}}}\n";
        let summary = summarize_ledger(text);
        let artifact = self_improve_proposal_artifact::from_ledger_text(text);
        let context = prompt_context_text_with_self_improve_proposals(&summary, Some(&artifact));

        assert_eq!(artifact.total_candidate_count, 0);
        assert!(artifact.proposals.is_empty());
        assert!(!context.contains("self_improve_proposal_acceptance="));
        assert!(!context.contains("self_improve_action_assignment="));
        assert!(!context.contains("next_self_improve_should_"));
        assert!(context.contains(
            "generic_noop_helper_context_omitted=count:1 reason=review_change_request_is_not_actionable"
        ));
        assert!(!context.contains("No change suggested in the primary_answer"));
        assert!(!context.contains("Small next change grounded in the same evidence"));
        assert!(!context.contains("No small next change is grounded in the same evidence"));
    }

    #[test]
    fn report_json_exposes_filtered_r78_generic_noop_as_empty_candidate_surface() {
        let text = "{\"round\":411,\"case\":\"noop-review\",\"success\":true,\"feedback_applied\":4,\"self_improve_passed\":true,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"safe\",\"validation_command_preview\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\",\"helper_stage_contract_by_role\":{\"review\":{\"fields\":{\"risk\":\"none\",\"change_request\":\"No change suggested in the primary_answer for this round.\",\"verification\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}},\"test-gate\":{\"fields\":{\"validation_command\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}}}}\n\
{\"round\":412,\"case\":\"generic-review\",\"success\":true,\"feedback_applied\":4,\"self_improve_passed\":true,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"safe\",\"validation_command_preview\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\",\"helper_stage_contract_by_role\":{\"review\":{\"fields\":{\"risk\":\"none\",\"change_request\":\"Small next change grounded in the same evidence\",\"verification\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}},\"test-gate\":{\"fields\":{\"validation_command\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}}}}\n\
{\"round\":414,\"case\":\"negative-generic-review\",\"success\":true,\"feedback_applied\":4,\"self_improve_passed\":true,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"safe\",\"validation_command_preview\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\",\"helper_stage_contract_by_role\":{\"review\":{\"fields\":{\"risk\":\"none\",\"change_request\":\"No small next change is grounded in the same evidence; the previous round already identified a change request regarding `--no-fail-fast`.\",\"verification\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}},\"test-gate\":{\"fields\":{\"validation_command\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}}}}\n";
        let summary = summarize_ledger(text);
        let artifact = self_improve_proposal_artifact::from_ledger_text(text);
        let json = report_json_with_remote_chain(
            &summary,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(&artifact),
            &[],
            &[],
            &[],
            &[],
        );

        assert_eq!(artifact.total_candidate_count, 0);
        assert!(artifact.proposals.is_empty());
        assert!(json.contains("\"total_candidate_count\":0"));
        assert!(json.contains("\"projected_candidate_count\":0"));
        assert!(json.contains("\"proposals\":[]"));
        assert!(json.contains("\"projected_report_count\":0"));
        assert!(json.contains("\"advisory_only_count\":0"));
        assert!(json.contains("\"action_required\":false"));
        assert!(json.contains("\"primary_action\":\"none\""));
        assert!(json.contains("\"target_count\":0"));
        assert!(json.contains("\"first_target\":null"));
        assert!(!json.contains("r78-generic-noop"));
        assert!(!json.contains("self-improve-r411-helper_contract-"));
        assert!(!json.contains("self-improve-r412-helper_contract-"));
        assert!(!json.contains("No change suggested in the primary_answer"));
        assert!(!json.contains("Small next change grounded in the same evidence"));
        assert!(!json.contains("No small next change is grounded in the same evidence"));
        assert!(!json.contains("self-improve-r414-helper_contract-"));
    }

    #[test]
    fn report_json_exposes_helper_stage_repair_status_for_incomplete_latest_role() {
        let text = "{\"round\":28,\"case\":\"helper-repair\",\"success\":true,\"feedback_applied\":4,\"helper_stage_contract_by_role\":{\"review\":{\"fields\":{\"risk\":\"helper output may omit proof\",\"change_request\":\"surface incomplete helper roles\"},\"matched_markers\":[\"risk\",\"change_request\"],\"expected_markers\":[\"risk\",\"change_request\",\"verification\"]}}}\n";
        let summary = summarize_ledger(text);
        let json = report_json(&summary, None, None, None, &[]);

        assert_contains_in_order(
            &json,
            &[
                "\"helper_stage_repair_status_report_v1\":{\"schema\":\"helper_stage_repair_status_report_v1\"",
                "\"report_only\":true",
                "\"latest_round\":28",
                "\"repair_required\":true",
                "\"incomplete_role_count\":1",
                "\"role\":\"review\"",
                "\"status\":\"missing_required_fields\"",
                "\"missing_fields\":[\"verification\"]",
                "\"admission\":{\"status\":\"repair_proposal_report_only\"",
                "\"auto_apply\":false",
                "\"side_effects\":{\"applies_code\":false",
                "\"calls_model\":false",
            ],
        );
        assert_occurrences(&json, "\"helper_stage_repair_status_report_v1\":{", 1);
    }

    #[test]
    fn report_json_exposes_helper_stage_repair_status_for_missing_required_latest_role() {
        let text = "{\"round\":29,\"case\":\"helper-repair-missing-role\",\"success\":true,\"feedback_applied\":4,\"helper_stage_contract_by_role\":{\"summary\":{\"fields\":{\"memory_update\":\"keep report-only repair surface\",\"next_context\":\"require latest review helper\",\"duplicate_guard\":\"single worker\"},\"matched_markers\":[\"memory_update\",\"next_context\",\"duplicate_guard\"],\"expected_markers\":[\"memory_update\",\"next_context\",\"duplicate_guard\"]}}}\n";
        let summary = summarize_ledger(text);
        let json = report_json_with_required_latest_helper_stage_roles(
            &summary,
            &["summary".to_owned(), "review".to_owned()],
        );

        assert_contains_in_order(
            &json,
            &[
                "\"helper_stage_repair_status_report_v1\":{\"schema\":\"helper_stage_repair_status_report_v1\"",
                "\"report_only\":true",
                "\"latest_round\":29",
                "\"repair_required\":true",
                "\"proposal_count\":1",
                "\"role\":\"review\"",
                "\"target_role\":\"review\"",
                "\"missing_role\":true",
                "\"source_round\":29",
                "\"evidence_id\":\"ledger.round.29.required_latest_helper_stage_roles.review.missing\"",
                "\"status\":\"missing_required_role\"",
                "\"missing_fields\":[\"risk\",\"change_request\",\"verification\"]",
                "\"admission\":{\"status\":\"repair_proposal_report_only\"",
                "\"auto_apply\":false",
                "\"side_effects\":{\"applies_code\":false",
                "\"starts_daemon\":false",
                "\"sends_prompt\":false",
                "\"calls_model\":false",
            ],
        );
        assert_occurrences(&json, "\"helper_stage_repair_status_report_v1\":{", 1);
    }

    #[test]
    fn run_report_json_refresh_writes_current_ledger_snapshot() {
        let dir = std::env::temp_dir();
        let unique = format!("smartsteam-run-report-refresh-{}", std::process::id());
        let ledger_path = dir.join(format!("{unique}.jsonl"));
        let report_path = dir.join(format!("{unique}.json"));
        let _ = fs::remove_file(&ledger_path);
        let _ = fs::remove_file(&report_path);
        fs::write(
            &ledger_path,
            "{\"round\":1,\"case\":\"ok-1\",\"success\":true,\"feedback_applied\":2}\n\
{\"round\":2,\"case\":\"ok-2\",\"success\":true,\"feedback_applied\":3}\n",
        )
        .unwrap();
        let config = Config {
            ledger_path: ledger_path.clone(),
            ..Config::default()
        };

        let refresh = write_run_report_json(&config, &report_path, true, false).unwrap();
        let json = fs::read_to_string(&report_path).unwrap();

        assert_eq!(refresh.rounds, 2);
        assert_eq!(refresh.gate_label.as_deref(), Some("report_gate"));
        assert_eq!(refresh.gate_failure_count, 0);
        assert!(json.contains("\"rounds\":2"));
        assert!(json.contains("\"report_gate\":{\"passed\":true"));

        let _ = fs::remove_file(ledger_path);
        let _ = fs::remove_file(report_path);
    }

    #[test]
    fn report_tracks_round_wall_elapsed_separately_from_model_elapsed() {
        let text = "{\"round\":1,\"case\":\"bootstrap\",\"started_unix\":10,\"finished_unix\":15,\"success\":true,\"runtime_tokens\":11,\"elapsed_ms\":100,\"feedback_applied\":2}\n\
{\"round\":2,\"case\":\"validated\",\"started_unix\":20,\"finished_unix\":27,\"success\":true,\"runtime_tokens\":13,\"elapsed_ms\":200,\"feedback_applied\":3}\n";
        let summary = summarize_ledger(text);
        let context = prompt_context_text(&summary);
        let json = report_json(&summary, None, None, None, &[]);

        assert_eq!(summary.elapsed_ms, 300);
        assert_eq!(summary.elapsed_items, 2);
        assert_eq!(summary.round_wall_elapsed_ms, 12_000);
        assert_eq!(summary.round_wall_elapsed_items, 2);
        assert_eq!(
            summary
                .last
                .as_ref()
                .and_then(ReportRecord::round_wall_elapsed_ms),
            Some(7_000)
        );
        assert!(context.contains("elapsed_ms_avg=150"));
        assert!(context.contains("round_wall_elapsed_ms_avg=6000"));
        assert!(json.contains("\"elapsed_ms\":{\"total\":300,\"avg\":150}"));
        assert!(json.contains("\"round_wall_elapsed_ms\":{\"total\":12000,\"avg\":6000}"));
        assert!(json.contains("\"round_wall_elapsed_ms\":7000"));
    }

    #[test]
    fn report_gate_passes_when_round_wall_clock_evidence_is_complete() {
        let text = "{\"round\":1,\"case\":\"bootstrap\",\"started_unix\":10,\"finished_unix\":15,\"success\":true,\"feedback_applied\":2}\n\
{\"round\":2,\"case\":\"validated\",\"started_unix\":20,\"finished_unix\":27,\"success\":true,\"feedback_applied\":3}\n";
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            require_round_wall_clock_evidence: true,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert!(failures.is_empty(), "{failures:?}");
    }

    #[test]
    fn report_gate_blocks_missing_or_invalid_round_wall_clock_evidence() {
        let text = "{\"round\":1,\"case\":\"missing-finish\",\"started_unix\":10,\"success\":true,\"feedback_applied\":2}\n\
{\"round\":2,\"case\":\"finished-before-start\",\"started_unix\":30,\"finished_unix\":27,\"success\":true,\"feedback_applied\":3}\n";
        let summary = summarize_ledger(text);
        let config = Config {
            report_gate: true,
            require_round_wall_clock_evidence: true,
            ..Config::default()
        };

        let failures = report_gate_failures(&summary, &config, None, None);

        assert_eq!(summary.round_wall_elapsed_items, 0);
        assert!(
            failures.iter().any(|failure| {
                failure.contains("round wall-clock evidence missing for 2 of 2 record(s)")
            }),
            "{failures:?}"
        );
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("latest round=2 case=finished-before-start")),
            "{failures:?}"
        );
    }

    #[test]
    fn summarizes_stream_terminal_failures() {
        let text = "{\"round\":1,\"case\":\"truncated\",\"success\":false,\"error\":\"/v1/business-cycle-stream stream truncated before terminal event\"}\n\
{\"round\":2,\"case\":\"missing-final\",\"success\":false,\"error\":\"stream ended without final event\"}\n";

        let summary = summarize_ledger(text);
        let context = prompt_context_text(&summary);
        let json = report_json(&summary, None, None, None, &[]);

        assert_eq!(summary.failure, 2);
        assert_eq!(summary.stream_truncation_failures, 1);
        assert_eq!(summary.missing_final_failures, 1);
        assert!(context.contains("stream_truncated:1"));
        assert!(context.contains("missing_final:1"));
        assert!(context.contains("runtime_response_failures:0"));
        assert!(json.contains("\"stream_failures\":{\"truncated\":1,\"missing_final\":1}"));
    }
}
