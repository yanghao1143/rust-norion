use std::env;
use std::path::PathBuf;

const DEFAULT_BACKEND: &str = "127.0.0.1:7979";
const DEFAULT_ROUNDS: usize = 5;
const DEFAULT_MAX_TOKENS: usize = 4096;
const DEFAULT_SELF_IMPROVE_LIMIT: usize = 1;
const DEFAULT_LEDGER: &str = "target/evolution/evolution-ledger.jsonl";
const DEFAULT_POOL_MANIFEST_JSON: &str = "target/evolution/pool-manifest.json";
const DEFAULT_POOL_STATUS_JSON: &str = "target/evolution/pool-status.json";
const DEFAULT_POOL_ROUTE_JSON: &str = "target/evolution/pool-route-review.json";
const DEFAULT_POOL_ROUTE_TASK_KIND: &str = "review";
const DEFAULT_CASE_PREFIX: &str = "smartsteam-evolution-loop";
const DEFAULT_MIN_INDEX_QUALITY_SCORE: f64 = 0.92;
const DEFAULT_TENANT_ID: &str = "local";
const DEFAULT_WORKSPACE_ID: &str = "default";
const DEFAULT_SESSION_ID: &str = "evolution-loop";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Config {
    pub(crate) backend: String,
    pub(crate) rounds: Option<usize>,
    pub(crate) interval_secs: u64,
    pub(crate) busy_wait_secs: u64,
    pub(crate) max_failures: usize,
    pub(crate) max_total_tokens: Option<u64>,
    pub(crate) max_runtime_secs: Option<u64>,
    pub(crate) max_no_feedback_rounds: Option<usize>,
    pub(crate) max_tokens: usize,
    pub(crate) self_improve_limit: usize,
    pub(crate) profile: String,
    pub(crate) feedback_amount: f32,
    pub(crate) case_prefix: String,
    pub(crate) tenant_id: String,
    pub(crate) workspace_id: String,
    pub(crate) session_id: String,
    pub(crate) ledger_path: PathBuf,
    pub(crate) pool_manifest_json_path: Option<PathBuf>,
    pub(crate) pool_status_json_path: Option<PathBuf>,
    pub(crate) pool_route_json_path: Option<PathBuf>,
    pub(crate) pool_budget_fairness_json_path: Option<PathBuf>,
    pub(crate) remote_chain_status_json_path: Option<PathBuf>,
    pub(crate) worker_window_status_json_path: Option<PathBuf>,
    pub(crate) clean_room_batch_status_json_path: Option<PathBuf>,
    pub(crate) memory_startup_admission_json_path: Option<PathBuf>,
    pub(crate) agent_clean_room_replacement_plan_json_path: Option<PathBuf>,
    pub(crate) remote_chain_gate: bool,
    pub(crate) pool_budget_fairness_gate: bool,
    pub(crate) require_pool_budget_policy: bool,
    pub(crate) pool_capacity_gate: bool,
    pub(crate) pool_alignment_gate: bool,
    pub(crate) refresh_pool_artifacts: bool,
    pub(crate) pool_route_task_kind: String,
    pub(crate) pool_stage_route_task_kinds: Vec<String>,
    pub(crate) pool_stage_route_gate: bool,
    pub(crate) execute_pool_stage_calls: bool,
    pub(crate) required_helper_stage_roles: Vec<String>,
    pub(crate) required_latest_helper_stage_roles: Vec<String>,
    pub(crate) require_useful_latest_helper_stage_feedback: bool,
    pub(crate) require_complete_latest_helper_stage_feedback: bool,
    pub(crate) require_clean_helper_stage_feedback: bool,
    pub(crate) require_final_json_pool_stage_dispatch: bool,
    pub(crate) require_pool_route: bool,
    pub(crate) pool_lease_dir: Option<PathBuf>,
    pub(crate) pool_lease_ttl_secs: u64,
    pub(crate) pool_lease_wait_secs: u64,
    pub(crate) pool_lease_poll_secs: u64,
    pub(crate) pool_lease_busy_policy: PoolLeaseBusyPolicy,
    pub(crate) max_pool_lease_skips: Option<usize>,
    pub(crate) prompt: Option<String>,
    pub(crate) prompt_file: Option<PathBuf>,
    pub(crate) report_context: bool,
    pub(crate) profile_outcome_log_path: Option<PathBuf>,
    pub(crate) profile_outcome_min_samples: usize,
    pub(crate) rust_check_code: Option<String>,
    pub(crate) rust_check_file: Option<PathBuf>,
    pub(crate) rust_check_edition: String,
    pub(crate) rust_check_case: Option<String>,
    pub(crate) validation_command: Option<String>,
    pub(crate) validation_workdir: Option<PathBuf>,
    pub(crate) validation_timeout_secs: u64,
    pub(crate) validation_phase: ValidationPhase,
    pub(crate) use_test_gate_validation_command: bool,
    pub(crate) timeout_secs: u64,
    pub(crate) show_delta: bool,
    pub(crate) business_gate: bool,
    pub(crate) trace_gate: bool,
    pub(crate) require_health: bool,
    pub(crate) min_runtime_context_window: Option<u64>,
    pub(crate) state_consistency_gate: bool,
    pub(crate) experience_audit_gate: bool,
    pub(crate) experience_audit_limit: usize,
    pub(crate) max_index_overlong_records: Option<u64>,
    pub(crate) max_index_overlong_without_clean_gist: u64,
    pub(crate) max_index_record_chars: Option<u64>,
    pub(crate) max_index_noisy_records: u64,
    pub(crate) max_index_noise_penalty: f64,
    pub(crate) min_index_quality_score: f64,
    pub(crate) require_index_retrieval_ready: bool,
    pub(crate) max_quarantine_candidates: u64,
    pub(crate) max_repairable_legacy_records: u64,
    pub(crate) max_legacy_metadata_without_clean_gist: u64,
    pub(crate) report: bool,
    pub(crate) report_gate: bool,
    pub(crate) report_continuation_gate: bool,
    pub(crate) report_json_path: Option<PathBuf>,
    pub(crate) run_report_json_path: Option<PathBuf>,
    pub(crate) run_report_gate: bool,
    pub(crate) run_report_continuation_gate: bool,
    pub(crate) newapi_live_smoke: bool,
    pub(crate) newapi_live_smoke_min_successes: usize,
    pub(crate) newapi_live_smoke_json_path: Option<PathBuf>,
    pub(crate) min_report_rounds: usize,
    pub(crate) min_success_rate: Option<f32>,
    pub(crate) min_feedback_total: Option<u64>,
    pub(crate) min_rust_checks: Option<usize>,
    pub(crate) min_rust_feedback_total: Option<u64>,
    pub(crate) max_stream_truncations: usize,
    pub(crate) max_missing_final: usize,
    pub(crate) max_runtime_response_failures: usize,
    pub(crate) strict_ledger_hygiene: bool,
    pub(crate) require_round_wall_clock_evidence: bool,
    pub(crate) require_test_gate_pass: bool,
    pub(crate) require_safe_test_gate_validation_command: bool,
    pub(crate) require_configured_validation_run: bool,
    pub(crate) require_test_gate_validation_run: bool,
    pub(crate) require_last_success: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ParseOutcome {
    Help,
    ListModels,
    MvpDemo(Config),
    NewApiLiveSmoke(Config),
    Report(Config),
    Run(Config),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ValidationPhase {
    Pre,
    Post,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PoolLeaseBusyPolicy {
    Fail,
    Wait,
    SkipLowPriority,
}

impl ValidationPhase {
    pub(crate) fn runs_pre(self) -> bool {
        matches!(self, Self::Pre | Self::Both)
    }

    pub(crate) fn runs_post(self) -> bool {
        matches!(self, Self::Post | Self::Both)
    }
}

pub(crate) fn parse_env() -> Result<ParseOutcome, String> {
    parse_args(env::args().skip(1))
}

pub(crate) fn parse_args<I, S>(args: I) -> Result<ParseOutcome, String>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut config = Config::default();
    let mut list_models = false;
    let mut mvp_demo = false;
    let mut args = args.into_iter().map(Into::into).peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => return Ok(ParseOutcome::Help),
            "--list-models" => list_models = true,
            "--mvp-demo" => mvp_demo = true,
            "--newapi-live-smoke" | "--model-pool-live-smoke" => config.newapi_live_smoke = true,
            "--min-newapi-live-models" | "--min-model-pool-live-models" => {
                config.newapi_live_smoke_min_successes = parse_usize(
                    &value(&mut args, "--min-newapi-live-models")?,
                    "--min-newapi-live-models",
                )?
                .max(1);
            }
            "--newapi-live-smoke-json" | "--model-pool-live-smoke-json" => {
                config.newapi_live_smoke_json_path =
                    Some(PathBuf::from(value(&mut args, "--newapi-live-smoke-json")?));
            }
            "--report" => config.report = true,
            "--report-json" => {
                config.report = true;
                config.report_json_path = Some(PathBuf::from(value(&mut args, "--report-json")?));
            }
            "--run-report-json" => {
                config.run_report_json_path =
                    Some(PathBuf::from(value(&mut args, "--run-report-json")?));
            }
            "--worker-window-status-json" => {
                config.worker_window_status_json_path = Some(PathBuf::from(value(
                    &mut args,
                    "--worker-window-status-json",
                )?));
            }
            "--clean-room-batch-status-json" => {
                config.clean_room_batch_status_json_path = Some(PathBuf::from(value(
                    &mut args,
                    "--clean-room-batch-status-json",
                )?));
            }
            "--memory-startup-admission-json" => {
                config.memory_startup_admission_json_path = Some(PathBuf::from(value(
                    &mut args,
                    "--memory-startup-admission-json",
                )?));
            }
            "--agent-clean-room-replacement-plan-json" => {
                config.agent_clean_room_replacement_plan_json_path = Some(PathBuf::from(value(
                    &mut args,
                    "--agent-clean-room-replacement-plan-json",
                )?));
            }
            "--run-report-gate" => config.run_report_gate = true,
            "--run-report-continuation-gate" => config.run_report_continuation_gate = true,
            "--report-gate" => {
                config.report = true;
                config.report_gate = true;
            }
            "--report-continuation-gate" => {
                config.report = true;
                config.report_continuation_gate = true;
            }
            "--min-report-rounds" => {
                config.min_report_rounds = parse_usize(
                    &value(&mut args, "--min-report-rounds")?,
                    "--min-report-rounds",
                )?
                .max(1);
            }
            "--min-success-rate" => {
                config.min_success_rate = Some(
                    parse_f32(
                        &value(&mut args, "--min-success-rate")?,
                        "--min-success-rate",
                    )?
                    .clamp(0.0, 100.0),
                );
            }
            "--min-feedback-total" => {
                config.min_feedback_total = parse_optional_u64(
                    &value(&mut args, "--min-feedback-total")?,
                    "--min-feedback-total",
                )?;
            }
            "--min-rust-checks" => {
                config.min_rust_checks = parse_optional_usize(
                    &value(&mut args, "--min-rust-checks")?,
                    "--min-rust-checks",
                )?;
            }
            "--min-rust-feedback-total" => {
                config.min_rust_feedback_total = parse_optional_u64(
                    &value(&mut args, "--min-rust-feedback-total")?,
                    "--min-rust-feedback-total",
                )?;
            }
            "--max-stream-truncations" => {
                config.max_stream_truncations = parse_usize(
                    &value(&mut args, "--max-stream-truncations")?,
                    "--max-stream-truncations",
                )?;
            }
            "--max-missing-final" => {
                config.max_missing_final = parse_usize(
                    &value(&mut args, "--max-missing-final")?,
                    "--max-missing-final",
                )?;
            }
            "--max-runtime-response-failures" => {
                config.max_runtime_response_failures = parse_usize(
                    &value(&mut args, "--max-runtime-response-failures")?,
                    "--max-runtime-response-failures",
                )?;
            }
            "--require-helper-stage-roles" => {
                config.required_helper_stage_roles =
                    parse_helper_stage_roles(&value(&mut args, "--require-helper-stage-roles")?)?;
            }
            "--require-latest-helper-stage-roles" => {
                config.required_latest_helper_stage_roles = parse_helper_stage_roles(&value(
                    &mut args,
                    "--require-latest-helper-stage-roles",
                )?)?;
            }
            "--require-useful-latest-helper-stage-feedback" => {
                config.require_useful_latest_helper_stage_feedback = true
            }
            "--require-complete-latest-helper-stage-feedback" => {
                config.require_complete_latest_helper_stage_feedback = true
            }
            "--require-clean-helper-stage-feedback" => {
                config.require_clean_helper_stage_feedback = true
            }
            "--require-final-json-pool-stage-dispatch" => {
                config.require_final_json_pool_stage_dispatch = true
            }
            "--require-test-gate-pass" => config.require_test_gate_pass = true,
            "--require-safe-test-gate-validation-command" => {
                config.require_safe_test_gate_validation_command = true
            }
            "--require-configured-validation-run" => {
                config.require_configured_validation_run = true
            }
            "--require-test-gate-validation-run" => config.require_test_gate_validation_run = true,
            "--require-round-wall-clock-evidence" => {
                config.require_round_wall_clock_evidence = true
            }
            "--strict-ledger-hygiene" => config.strict_ledger_hygiene = true,
            "--allow-last-failure" => config.require_last_success = false,
            "--backend" => config.backend = value(&mut args, "--backend")?,
            "--rounds" => {
                let rounds = parse_usize(&value(&mut args, "--rounds")?, "--rounds")?;
                config.rounds = Some(rounds.max(1));
            }
            "--forever" => config.rounds = None,
            "--interval-secs" => {
                config.interval_secs =
                    parse_u64(&value(&mut args, "--interval-secs")?, "--interval-secs")?;
            }
            "--busy-wait-secs" => {
                config.busy_wait_secs =
                    parse_u64(&value(&mut args, "--busy-wait-secs")?, "--busy-wait-secs")?;
            }
            "--max-failures" => {
                config.max_failures =
                    parse_usize(&value(&mut args, "--max-failures")?, "--max-failures")?.max(1);
            }
            "--max-total-tokens" => {
                config.max_total_tokens = parse_optional_u64(
                    &value(&mut args, "--max-total-tokens")?,
                    "--max-total-tokens",
                )?;
            }
            "--max-runtime-secs" => {
                config.max_runtime_secs = parse_optional_u64(
                    &value(&mut args, "--max-runtime-secs")?,
                    "--max-runtime-secs",
                )?;
            }
            "--max-no-feedback-rounds" => {
                config.max_no_feedback_rounds = parse_optional_usize(
                    &value(&mut args, "--max-no-feedback-rounds")?,
                    "--max-no-feedback-rounds",
                )?;
            }
            "--max-tokens" | "--max" => {
                config.max_tokens =
                    parse_usize(&value(&mut args, "--max-tokens")?, "--max-tokens")?
                        .clamp(1, 262_144);
            }
            "--self-improve-limit" | "--limit" => {
                config.self_improve_limit = parse_usize(
                    &value(&mut args, "--self-improve-limit")?,
                    "--self-improve-limit",
                )?
                .max(1);
            }
            "--profile" => config.profile = value(&mut args, "--profile")?,
            "--feedback" | "--feedback-amount" => {
                config.feedback_amount =
                    parse_f32(&value(&mut args, "--feedback")?, "--feedback")?.clamp(0.0, 1.0);
            }
            "--case-prefix" => {
                let case_prefix = value(&mut args, "--case-prefix")?;
                if case_prefix.trim().is_empty() {
                    return Err("--case-prefix must not be empty".to_owned());
                }
                config.case_prefix = case_prefix;
            }
            "--tenant-id" => config.tenant_id = non_empty_value(&mut args, "--tenant-id")?,
            "--workspace-id" => config.workspace_id = non_empty_value(&mut args, "--workspace-id")?,
            "--session-id" => config.session_id = non_empty_value(&mut args, "--session-id")?,
            "--ledger" => config.ledger_path = PathBuf::from(value(&mut args, "--ledger")?),
            "--pool-manifest-json" | "--model-pool-manifest-json" => {
                config.pool_manifest_json_path =
                    Some(PathBuf::from(value(&mut args, "--pool-manifest-json")?));
            }
            "--pool-status-json" => {
                config.pool_status_json_path =
                    Some(PathBuf::from(value(&mut args, "--pool-status-json")?));
            }
            "--pool-route-json" => {
                config.pool_route_json_path =
                    Some(PathBuf::from(value(&mut args, "--pool-route-json")?));
            }
            "--pool-budget-fairness-json" => {
                config.pool_budget_fairness_json_path = Some(PathBuf::from(value(
                    &mut args,
                    "--pool-budget-fairness-json",
                )?));
            }
            "--remote-chain-status-json" => {
                config.remote_chain_status_json_path = Some(PathBuf::from(value(
                    &mut args,
                    "--remote-chain-status-json",
                )?));
            }
            "--remote-chain-gate" => config.remote_chain_gate = true,
            "--pool-budget-fairness-gate" => config.pool_budget_fairness_gate = true,
            "--require-pool-budget-policy" => config.require_pool_budget_policy = true,
            "--pool-capacity-gate" => config.pool_capacity_gate = true,
            "--pool-alignment-gate" => config.pool_alignment_gate = true,
            "--refresh-pool-artifacts" => config.refresh_pool_artifacts = true,
            "--pool-route-task-kind" => {
                config.pool_route_task_kind =
                    parse_pool_route_task_kind(&value(&mut args, "--pool-route-task-kind")?)?;
            }
            "--pool-stage-route-task-kinds" => {
                config.pool_stage_route_task_kinds = parse_pool_stage_route_task_kinds(&value(
                    &mut args,
                    "--pool-stage-route-task-kinds",
                )?)?;
            }
            "--pool-stage-route-gate" => config.pool_stage_route_gate = true,
            "--execute-pool-stage-calls" => config.execute_pool_stage_calls = true,
            "--require-pool-route" => config.require_pool_route = true,
            "--pool-lease-dir" => {
                config.pool_lease_dir = Some(PathBuf::from(value(&mut args, "--pool-lease-dir")?));
            }
            "--pool-lease-ttl-secs" => {
                config.pool_lease_ttl_secs = parse_u64(
                    &value(&mut args, "--pool-lease-ttl-secs")?,
                    "--pool-lease-ttl-secs",
                )?
                .max(1);
            }
            "--pool-lease-wait-secs" => {
                config.pool_lease_wait_secs = parse_u64(
                    &value(&mut args, "--pool-lease-wait-secs")?,
                    "--pool-lease-wait-secs",
                )?;
            }
            "--pool-lease-poll-secs" => {
                config.pool_lease_poll_secs = parse_u64(
                    &value(&mut args, "--pool-lease-poll-secs")?,
                    "--pool-lease-poll-secs",
                )?
                .max(1);
            }
            "--pool-lease-busy-policy" => {
                config.pool_lease_busy_policy =
                    parse_pool_lease_busy_policy(&value(&mut args, "--pool-lease-busy-policy")?)?;
            }
            "--max-pool-lease-skips" => {
                config.max_pool_lease_skips = parse_optional_usize(
                    &value(&mut args, "--max-pool-lease-skips")?,
                    "--max-pool-lease-skips",
                )?;
            }
            "--prompt" => config.prompt = Some(value(&mut args, "--prompt")?),
            "--prompt-file" => {
                config.prompt_file = Some(PathBuf::from(value(&mut args, "--prompt-file")?))
            }
            "--no-report-context" => config.report_context = false,
            "--profile-outcome-log" => {
                config.profile_outcome_log_path =
                    Some(PathBuf::from(value(&mut args, "--profile-outcome-log")?));
            }
            "--profile-outcome-min-samples" => {
                config.profile_outcome_min_samples = parse_usize(
                    &value(&mut args, "--profile-outcome-min-samples")?,
                    "--profile-outcome-min-samples",
                )?
                .max(1);
            }
            "--rust-check-code" => {
                let code = value(&mut args, "--rust-check-code")?;
                if code.trim().is_empty() {
                    return Err("--rust-check-code must not be empty".to_owned());
                }
                config.rust_check_code = Some(code);
            }
            "--rust-check-file" => {
                config.rust_check_file =
                    Some(PathBuf::from(value(&mut args, "--rust-check-file")?));
            }
            "--rust-check-edition" => {
                let edition = value(&mut args, "--rust-check-edition")?;
                if !matches!(edition.as_str(), "2015" | "2018" | "2021" | "2024") {
                    return Err("--rust-check-edition must be 2015, 2018, 2021, or 2024".to_owned());
                }
                config.rust_check_edition = edition;
            }
            "--rust-check-case" => {
                let case = value(&mut args, "--rust-check-case")?;
                if case.trim().is_empty() {
                    return Err("--rust-check-case must not be empty".to_owned());
                }
                config.rust_check_case = Some(case);
            }
            "--validation-command" | "--check-command" => {
                let command = value(&mut args, "--validation-command")?;
                if command.trim().is_empty() {
                    return Err("--validation-command must not be empty".to_owned());
                }
                config.validation_command = Some(command);
            }
            "--validation-workdir" | "--check-workdir" => {
                config.validation_workdir =
                    Some(PathBuf::from(value(&mut args, "--validation-workdir")?));
            }
            "--validation-timeout-secs" | "--check-timeout-secs" => {
                config.validation_timeout_secs = parse_u64(
                    &value(&mut args, "--validation-timeout-secs")?,
                    "--validation-timeout-secs",
                )?
                .max(1);
            }
            "--validation-phase" | "--check-phase" => {
                config.validation_phase =
                    parse_validation_phase(&value(&mut args, "--validation-phase")?)?;
            }
            "--use-test-gate-validation-command" => config.use_test_gate_validation_command = true,
            "--timeout-secs" => {
                config.timeout_secs =
                    parse_u64(&value(&mut args, "--timeout-secs")?, "--timeout-secs")?.max(1);
            }
            "--show-delta" => config.show_delta = true,
            "--business-gate" => config.business_gate = true,
            "--trace-gate" => config.trace_gate = true,
            "--no-health-gate" => config.require_health = false,
            "--min-runtime-context" | "--min-runtime-context-window" => {
                config.min_runtime_context_window = parse_optional_u64(
                    &value(&mut args, "--min-runtime-context")?,
                    "--min-runtime-context",
                )?;
            }
            "--state-consistency-gate" | "--ledger-state-gate" => {
                config.state_consistency_gate = true
            }
            "--experience-audit-gate" => config.experience_audit_gate = true,
            "--experience-audit-limit" => {
                config.experience_audit_limit = parse_usize(
                    &value(&mut args, "--experience-audit-limit")?,
                    "--experience-audit-limit",
                )?
                .max(1);
            }
            "--max-index-overlong-records" => {
                config.max_index_overlong_records = Some(parse_u64(
                    &value(&mut args, "--max-index-overlong-records")?,
                    "--max-index-overlong-records",
                )?);
            }
            "--max-index-overlong-without-clean-gist" => {
                config.max_index_overlong_without_clean_gist = parse_u64(
                    &value(&mut args, "--max-index-overlong-without-clean-gist")?,
                    "--max-index-overlong-without-clean-gist",
                )?;
            }
            "--max-index-record-chars" => {
                config.max_index_record_chars = Some(parse_u64(
                    &value(&mut args, "--max-index-record-chars")?,
                    "--max-index-record-chars",
                )?);
            }
            "--max-index-noisy-records" => {
                config.max_index_noisy_records = parse_u64(
                    &value(&mut args, "--max-index-noisy-records")?,
                    "--max-index-noisy-records",
                )?;
            }
            "--max-index-noise-penalty" => {
                config.max_index_noise_penalty = parse_f64(
                    &value(&mut args, "--max-index-noise-penalty")?,
                    "--max-index-noise-penalty",
                )?
                .max(0.0);
            }
            "--min-index-quality-score" => {
                config.min_index_quality_score = parse_f64(
                    &value(&mut args, "--min-index-quality-score")?,
                    "--min-index-quality-score",
                )?
                .clamp(0.0, 1.0);
            }
            "--allow-index-retrieval-not-ready" | "--no-require-index-retrieval-ready" => {
                config.require_index_retrieval_ready = false;
            }
            "--max-quarantine-candidates" => {
                config.max_quarantine_candidates = parse_u64(
                    &value(&mut args, "--max-quarantine-candidates")?,
                    "--max-quarantine-candidates",
                )?;
            }
            "--max-repairable-legacy-records" => {
                config.max_repairable_legacy_records = parse_u64(
                    &value(&mut args, "--max-repairable-legacy-records")?,
                    "--max-repairable-legacy-records",
                )?;
            }
            "--max-legacy-metadata-without-clean-gist" => {
                config.max_legacy_metadata_without_clean_gist = parse_u64(
                    &value(&mut args, "--max-legacy-metadata-without-clean-gist")?,
                    "--max-legacy-metadata-without-clean-gist",
                )?;
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    if config.refresh_pool_artifacts {
        if config.pool_manifest_json_path.is_none() {
            config.pool_manifest_json_path = Some(PathBuf::from(DEFAULT_POOL_MANIFEST_JSON));
        }
        if config.pool_status_json_path.is_none() {
            config.pool_status_json_path = Some(PathBuf::from(DEFAULT_POOL_STATUS_JSON));
        }
        if config.pool_route_json_path.is_none() {
            config.pool_route_json_path = Some(PathBuf::from(DEFAULT_POOL_ROUTE_JSON));
        }
    }
    if config.run_report_json_path.is_some()
        && (config.report_gate || config.report_continuation_gate)
    {
        return Err(
            "--run-report-json cannot be combined with --report-gate or --report-continuation-gate; use --run-report-gate or --run-report-continuation-gate for run-mode gates".to_owned(),
        );
    }
    if mvp_demo {
        Ok(ParseOutcome::MvpDemo(config))
    } else if config.newapi_live_smoke {
        Ok(ParseOutcome::NewApiLiveSmoke(config))
    } else if list_models {
        Ok(ParseOutcome::ListModels)
    } else if config.report {
        Ok(ParseOutcome::Report(config))
    } else {
        Ok(ParseOutcome::Run(config))
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            backend: DEFAULT_BACKEND.to_owned(),
            rounds: Some(DEFAULT_ROUNDS),
            interval_secs: 5,
            busy_wait_secs: 15,
            max_failures: 3,
            max_total_tokens: None,
            max_runtime_secs: None,
            max_no_feedback_rounds: Some(3),
            max_tokens: DEFAULT_MAX_TOKENS,
            self_improve_limit: DEFAULT_SELF_IMPROVE_LIMIT,
            profile: "coding".to_owned(),
            feedback_amount: 0.5,
            case_prefix: DEFAULT_CASE_PREFIX.to_owned(),
            tenant_id: DEFAULT_TENANT_ID.to_owned(),
            workspace_id: DEFAULT_WORKSPACE_ID.to_owned(),
            session_id: DEFAULT_SESSION_ID.to_owned(),
            ledger_path: PathBuf::from(DEFAULT_LEDGER),
            pool_manifest_json_path: None,
            pool_status_json_path: None,
            pool_route_json_path: None,
            pool_budget_fairness_json_path: None,
            remote_chain_status_json_path: None,
            worker_window_status_json_path: None,
            clean_room_batch_status_json_path: None,
            memory_startup_admission_json_path: None,
            agent_clean_room_replacement_plan_json_path: None,
            remote_chain_gate: false,
            pool_budget_fairness_gate: false,
            require_pool_budget_policy: false,
            pool_capacity_gate: false,
            pool_alignment_gate: false,
            refresh_pool_artifacts: false,
            pool_route_task_kind: DEFAULT_POOL_ROUTE_TASK_KIND.to_owned(),
            pool_stage_route_task_kinds: Vec::new(),
            pool_stage_route_gate: false,
            execute_pool_stage_calls: false,
            required_helper_stage_roles: Vec::new(),
            required_latest_helper_stage_roles: Vec::new(),
            require_useful_latest_helper_stage_feedback: false,
            require_complete_latest_helper_stage_feedback: false,
            require_clean_helper_stage_feedback: false,
            require_final_json_pool_stage_dispatch: false,
            require_pool_route: false,
            pool_lease_dir: None,
            pool_lease_ttl_secs: 1800,
            pool_lease_wait_secs: 0,
            pool_lease_poll_secs: 5,
            pool_lease_busy_policy: PoolLeaseBusyPolicy::Wait,
            max_pool_lease_skips: Some(3),
            prompt: None,
            prompt_file: None,
            report_context: true,
            profile_outcome_log_path: None,
            profile_outcome_min_samples: 2,
            rust_check_code: None,
            rust_check_file: None,
            rust_check_edition: "2021".to_owned(),
            rust_check_case: None,
            validation_command: None,
            validation_workdir: None,
            validation_timeout_secs: 300,
            validation_phase: ValidationPhase::Pre,
            use_test_gate_validation_command: false,
            timeout_secs: 900,
            show_delta: false,
            business_gate: false,
            trace_gate: false,
            require_health: true,
            min_runtime_context_window: None,
            state_consistency_gate: false,
            experience_audit_gate: false,
            experience_audit_limit: 25,
            max_index_overlong_records: None,
            max_index_overlong_without_clean_gist: 0,
            max_index_record_chars: None,
            max_index_noisy_records: 0,
            max_index_noise_penalty: 0.0,
            min_index_quality_score: DEFAULT_MIN_INDEX_QUALITY_SCORE,
            require_index_retrieval_ready: true,
            max_quarantine_candidates: 0,
            max_repairable_legacy_records: 0,
            max_legacy_metadata_without_clean_gist: 0,
            report: false,
            report_gate: false,
            report_continuation_gate: false,
            report_json_path: None,
            run_report_json_path: None,
            run_report_gate: false,
            run_report_continuation_gate: false,
            newapi_live_smoke: false,
            newapi_live_smoke_min_successes: 2,
            newapi_live_smoke_json_path: None,
            min_report_rounds: 1,
            min_success_rate: None,
            min_feedback_total: Some(1),
            min_rust_checks: None,
            min_rust_feedback_total: None,
            max_stream_truncations: 0,
            max_missing_final: 0,
            max_runtime_response_failures: 0,
            strict_ledger_hygiene: false,
            require_round_wall_clock_evidence: false,
            require_test_gate_pass: false,
            require_safe_test_gate_validation_command: false,
            require_configured_validation_run: false,
            require_test_gate_validation_run: false,
            require_last_success: true,
        }
    }
}

pub(crate) fn help_text() -> &'static str {
    "SmartSteam evolution-loop\n\
\n\
Runs an unattended business-cycle stream loop against rust-norion and writes a JSONL ledger.\n\
\n\
Usage:\n\
  cargo run --manifest-path tools/evolution-loop/Cargo.toml -- [options]\n\
\n\
Options:\n\
  --backend HOST:PORT              rust-norion backend address (default 127.0.0.1:7979)\n\
  --rounds N                       finite number of rounds (default 5)\n\
  --forever                        run until stopped with Ctrl+C\n\
  --interval-secs N                sleep after each completed round (default 5)\n\
  --busy-wait-secs N               sleep while backend is busy (default 15)\n\
  --max-failures N                 stop after consecutive failures (default 3)\n\
  --max-total-tokens N             stop after total runtime tokens, 0 disables\n\
  --max-runtime-secs N             stop after observed runtime seconds, 0 disables\n\
  --max-no-feedback-rounds N       stop after N rounds with no feedback updates, 0 disables\n\
  --max-tokens N                   per-request generation budget (default 4096)\n\
  --self-improve-limit N           replay items per round (default 1)\n\
  --profile coding|general|writing|long\n\
  --feedback-amount N              feedback amount 0.0..1.0 (default 0.5)\n\
  --case-prefix TEXT               case prefix for generated rounds\n\
  --tenant-id TEXT                  tenant scope for backend calls (default local)\n\
  --workspace-id TEXT               workspace scope for backend calls (default default)\n\
  --session-id TEXT                 session scope for backend calls (default evolution-loop)\n\
  --ledger PATH                    JSONL ledger path\n\
  --list-models                    print the built-in model registry and exit without backend calls\n\
  --mvp-demo                       run the offline M0-M4 model-pool demo and exit without backend calls\n\
  --newapi-live-smoke              call every env-allowed NewAPI/model-pool model once and require live successes\n\
  --min-newapi-live-models N       live smoke minimum successful models (default 2)\n\
  --newapi-live-smoke-json PATH    write secret-free live smoke JSON evidence\n\
  --pool-manifest-json PATH        read gemma-chain pool-manifest -JsonStatus artifact into reports and prompt context\n\
  --pool-status-json PATH          read gemma-chain pool-status -JsonStatus artifact into reports and prompt context\n\
  --pool-route-json PATH           read gemma-chain pool-route-plan -JsonStatus artifact into reports and prompt context\n\
  --pool-budget-fairness-json PATH write model_worker_v1 events during runs and read them into reports/report gate\n\
  --remote-chain-status-json PATH  read status-remote-gemma-chain -JsonStatus artifact into reports and prompt context\n\
  --worker-window-status-json PATH read Codex worker-window paused/polluted/replacement status into report JSON only\n\
  --clean-room-batch-status-json PATH read R24/R25 clean-room batch closure status into report JSON only\n\
  --memory-startup-admission-json PATH read memory startup admission status into report JSON only\n\
  --agent-clean-room-replacement-plan-json PATH read agent clean-room replacement plan into report JSON only\n\
  --remote-chain-gate              fail before each prompt, and in --report-gate, unless --remote-chain-status-json readiness.ready=true\n\
  --pool-budget-fairness-gate      fail before each prompt when --pool-budget-fairness-json reports budget_fairness_blocked=true\n\
  --require-pool-budget-policy     report gate requires quality budget preserved and low-priority helper clamp evidence\n\
  --pool-capacity-gate             fail before each prompt, and in --report-gate, when --pool-status-json capacity.expansion_allowed is missing or false\n\
  --pool-alignment-gate            fail before each prompt unless manifest/status/route roles and helper routes align\n\
  --refresh-pool-artifacts         refresh read-only pool manifest/status/route JSON from backend before each round\n\
  --pool-route-task-kind KIND      task kind for refreshed pool route artifact: summary|router|review|index|test-gate|quality|spare|auto\n\
  --pool-stage-route-task-kinds KINDS refresh extra comma-separated route artifacts, e.g. summary,router,review,index,test-gate\n\
  --pool-stage-route-gate          fail before each prompt unless extra stage route artifacts are allowed and ready\n\
  --execute-pool-stage-calls       after a successful primary round, send explicit helper prompts to ready stage workers through /v1/model-pool/call\n\
  --require-pool-route             fail before prompting unless --pool-route-json allows a ready selected role\n\
  --pool-lease-dir PATH            acquire a local selected-worker lease before prompting; requires --require-pool-route\n\
  --pool-lease-ttl-secs N          local worker lease TTL seconds (default 1800)\n\
  --pool-lease-wait-secs N         wait up to N seconds for a busy worker lease (default 0)\n\
  --pool-lease-poll-secs N         busy lease polling interval while waiting (default 5)\n\
  --pool-lease-busy-policy POLICY  fail|wait|skip-low-priority (default wait)\n\
  --max-pool-lease-skips N         stop after N consecutive skipped low-priority leases, 0 disables (default 3)\n\
  --prompt TEXT                    use one prompt for every round\n\
  --prompt-file PATH               use non-empty lines as rotating prompts\n\
  --no-report-context              do not inject previous ledger summary into prompts\n\
  --rust-check-code TEXT           attach Rust code for compiler feedback\n\
  --rust-check-file PATH           attach Rust code from file for compiler feedback\n\
  --rust-check-edition EDITION     Rust check edition 2015|2018|2021|2024\n\
  --rust-check-case TEXT           Rust check case name\n\
  --validation-command TEXT        run a local shell command as a validation gate\n\
  --validation-workdir PATH        working directory for validation command\n\
  --validation-timeout-secs N      validation command timeout (default 300)\n\
  --validation-phase pre|post|both when to run validation command (default pre)\n\
  --use-test-gate-validation-command use latest safe test-gate validation_command when --validation-command is not set\n\
  --timeout-secs N                 socket read timeout for long inference (default 900)\n\
  --show-delta                     print streamed model text\n\
  --business-gate                  require strict business-cycle state gate\n\
  --trace-gate                     require trace schema gate when backend configured it\n\
  --no-health-gate                 skip /health preflight and busy checks\n\
  --min-runtime-context N          require Gemma runtime health n_ctx >= N, 0 disables\n\
  --state-consistency-gate         fail before each round if ledger rounds are duplicated, non-monotonic, invalid, or gapped\n\
  --experience-audit-gate          run read-only experience cleanup/index audit before each round\n\
  --experience-audit-limit N       sample limit for audit gate (default 25)\n\
  --max-index-overlong-records N   audit gate maximum overlong index records (default disabled)\n\
  --max-index-overlong-without-clean-gist N audit gate maximum overlong index records missing clean gist (default 0)\n\
  --max-index-record-chars N       audit gate maximum index record chars (default disabled)\n\
  --max-index-noisy-records N      audit gate maximum noisy index records (default 0)\n\
  --max-index-noise-penalty N      audit gate maximum index noise penalty (default 0.0)\n\
  --min-index-quality-score N      audit gate minimum index quality score (default 0.92)\n\
  --allow-index-retrieval-not-ready allow audit gate when index retrieval_ready is false or missing\n\
  --max-quarantine-candidates N    audit gate maximum quarantine candidates (default 0)\n\
  --max-repairable-legacy-records N audit gate maximum repairable legacy records (default 0)\n\
  --max-legacy-metadata-without-clean-gist N audit gate maximum legacy metadata records missing clean gist (default 0)\n\
  --report                         summarize ledger and exit without backend calls\n\
  --report-json PATH               write machine-readable report JSON artifact\n\
  --run-report-json PATH           refresh machine-readable report JSON after each run-mode round\n\
  --run-report-gate                apply report gate to --run-report-json refreshes\n\
  --run-report-continuation-gate   apply continuation gate to --run-report-json refreshes\n\
  --profile-outcome-log PATH       replay request outcome JSONL into profile-routing regression report\n\
  --profile-outcome-min-samples N  minimum baseline/candidate replay samples (default 2)\n\
  --report-gate                    fail when ledger or supplied model-pool evidence does not meet gate\n\
  --report-continuation-gate       with --report-gate, fail only when latest/current continuation evidence blocks unattended evolution\n\
  --min-report-rounds N            report gate minimum rounds (default 1)\n\
  --min-success-rate PCT           report gate minimum success rate percent\n\
  --min-feedback-total N           report gate minimum feedback updates, 0 disables\n\
  --min-rust-checks N              report gate minimum Rust check rounds, 0 disables\n\
  --min-rust-feedback-total N      report gate minimum Rust-check feedback updates, 0 disables\n\
  --max-stream-truncations N       report gate maximum stream truncation failures (default 0)\n\
  --max-missing-final N            report gate maximum missing final-event failures (default 0)\n\
  --max-runtime-response-failures N report gate maximum wrapped runtime response failures (default 0)\n\
  --require-helper-stage-roles ROLES report gate requires real helper feedback for comma-separated roles summary|router|review|index|test-gate\n\
  --require-latest-helper-stage-roles ROLES report gate requires latest round helper feedback for comma-separated roles summary|router|review|index|test-gate\n\
  --require-useful-latest-helper-stage-feedback report gate requires latest helper feedback to follow role contracts\n\
  --require-complete-latest-helper-stage-feedback report gate requires latest helper feedback to include required role fields\n\
  --require-clean-helper-stage-feedback report gate requires recent helper feedback to avoid markdown/code-fence wrappers\n\
  --require-final-json-pool-stage-dispatch report gate requires latest final_json.pool_stage_dispatch to include required latest helper roles\n\
  --require-test-gate-pass        report gate requires latest test-gate helper verdict to be pass\n\
  --require-safe-test-gate-validation-command report gate requires latest test-gate validation_command to be a safe cargo validation command\n\
  --require-configured-validation-run report gate requires latest round to run and pass the configured validation command\n\
  --require-test-gate-validation-run report gate requires latest round to run and pass the safe test-gate validation command\n\
  --require-round-wall-clock-evidence report gate requires each ledger record to include started_unix and finished_unix\n\
  --strict-ledger-hygiene          report gate fails on duplicate or non-monotonic rounds\n\
  --allow-last-failure             report gate does not require latest round success\n\
  -h, --help                       show this help\n"
}

fn value<I>(args: &mut std::iter::Peekable<I>, flag: &str) -> Result<String, String>
where
    I: Iterator<Item = String>,
{
    args.next()
        .filter(|value| !value.starts_with("--"))
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn non_empty_value<I>(args: &mut std::iter::Peekable<I>, flag: &str) -> Result<String, String>
where
    I: Iterator<Item = String>,
{
    let value = value(args, flag)?;
    if value.trim().is_empty() {
        return Err(format!("{flag} must not be empty"));
    }
    Ok(value)
}

fn parse_usize(value: &str, flag: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|_| format!("{flag} must be a positive integer"))
}

fn parse_u64(value: &str, flag: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("{flag} must be a positive integer"))
}

fn parse_optional_u64(value: &str, flag: &str) -> Result<Option<u64>, String> {
    let parsed = parse_u64(value, flag)?;
    Ok((parsed > 0).then_some(parsed))
}

fn parse_optional_usize(value: &str, flag: &str) -> Result<Option<usize>, String> {
    let parsed = parse_usize(value, flag)?;
    Ok((parsed > 0).then_some(parsed))
}

fn parse_validation_phase(value: &str) -> Result<ValidationPhase, String> {
    match value {
        "pre" => Ok(ValidationPhase::Pre),
        "post" => Ok(ValidationPhase::Post),
        "both" => Ok(ValidationPhase::Both),
        _ => Err("--validation-phase must be pre, post, or both".to_owned()),
    }
}

fn parse_pool_lease_busy_policy(value: &str) -> Result<PoolLeaseBusyPolicy, String> {
    match value {
        "fail" => Ok(PoolLeaseBusyPolicy::Fail),
        "wait" => Ok(PoolLeaseBusyPolicy::Wait),
        "skip-low-priority" => Ok(PoolLeaseBusyPolicy::SkipLowPriority),
        _ => Err("--pool-lease-busy-policy must be fail, wait, or skip-low-priority".to_owned()),
    }
}

fn parse_pool_route_task_kind(value: &str) -> Result<String, String> {
    match value {
        "summary" | "router" | "review" | "index" | "test-gate" | "quality" | "spare"
        | "auto" => {
            Ok(value.to_owned())
        }
        "route" | "intent" | "intent-classify" | "preflight" | "tool-call" | "tool_call"
        | "tool-calls" | "tool_calls" | "function" | "function-call" | "function_call" => {
            Ok("router".to_owned())
        }
        _ => Err("--pool-route-task-kind must be summary, router, review, index, test-gate, quality, spare, auto, or a router alias such as tool-call/preflight".to_owned()),
    }
}

fn parse_pool_stage_route_task_kinds(value: &str) -> Result<Vec<String>, String> {
    let mut kinds = Vec::new();
    for item in value.split(',') {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            continue;
        }
        let kind = parse_pool_route_task_kind(trimmed)?;
        if !kinds.iter().any(|existing| existing == &kind) {
            kinds.push(kind);
        }
    }
    if kinds.is_empty() {
        return Err("--pool-stage-route-task-kinds must include at least one kind".to_owned());
    }
    Ok(kinds)
}

fn parse_helper_stage_roles(value: &str) -> Result<Vec<String>, String> {
    let mut roles = Vec::new();
    for item in value.split(',') {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            continue;
        }
        match trimmed {
            "summary" | "router" | "review" | "index" | "test-gate" => {
                if !roles.iter().any(|existing| existing == trimmed) {
                    roles.push(trimmed.to_owned());
                }
            }
            _ => {
                return Err(
                    "--require-helper-stage-roles must be summary, router, review, index, or test-gate"
                        .to_owned(),
                );
            }
        }
    }
    if roles.is_empty() {
        return Err("--require-helper-stage-roles must include at least one role".to_owned());
    }
    Ok(roles)
}

fn parse_f32(value: &str, flag: &str) -> Result<f32, String> {
    value
        .parse::<f32>()
        .ok()
        .filter(|value| value.is_finite())
        .ok_or_else(|| format!("{flag} must be a finite number"))
}

fn parse_f64(value: &str, flag: &str) -> Result<f64, String> {
    value
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
        .ok_or_else(|| format!("{flag} must be a finite number"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_list_models_mode() {
        let parsed = parse_args(["--list-models"]).unwrap();

        assert_eq!(parsed, ParseOutcome::ListModels);
    }

    #[test]
    fn parses_forever_and_generation_budget() {
        let parsed = parse_args([
            "--backend",
            "127.0.0.1:7979",
            "--forever",
            "--max",
            "8192",
            "--self-improve-limit",
            "2",
        ])
        .unwrap();

        let ParseOutcome::Run(config) = parsed else {
            panic!("expected run config");
        };
        assert_eq!(config.backend, "127.0.0.1:7979");
        assert_eq!(config.rounds, None);
        assert_eq!(config.max_tokens, 8192);
        assert_eq!(config.self_improve_limit, 2);
    }

    #[test]
    fn parses_unattended_budget_guards() {
        let parsed = parse_args([
            "--max-total-tokens",
            "1000",
            "--max-runtime-secs",
            "60",
            "--max-no-feedback-rounds",
            "0",
            "--min-runtime-context",
            "262144",
            "--state-consistency-gate",
        ])
        .unwrap();
        let ParseOutcome::Run(config) = parsed else {
            panic!("expected run config");
        };

        assert_eq!(config.max_total_tokens, Some(1000));
        assert_eq!(config.max_runtime_secs, Some(60));
        assert_eq!(config.max_no_feedback_rounds, None);
        assert_eq!(config.min_runtime_context_window, Some(262_144));
        assert!(config.state_consistency_gate);
    }

    #[test]
    fn parses_case_prefix() {
        let parsed = parse_args(["--case-prefix", "nightly-evo"]).unwrap();
        let ParseOutcome::Run(config) = parsed else {
            panic!("expected run config");
        };

        assert_eq!(config.case_prefix, "nightly-evo");
    }

    #[test]
    fn defaults_tenant_scope_for_service_calls() {
        let parsed = parse_args(Vec::<String>::new()).unwrap();
        let ParseOutcome::Run(config) = parsed else {
            panic!("expected run config");
        };

        assert_eq!(config.tenant_id, "local");
        assert_eq!(config.workspace_id, "default");
        assert_eq!(config.session_id, "evolution-loop");
    }

    #[test]
    fn parses_tenant_scope() {
        let parsed = parse_args([
            "--tenant-id",
            "tenant-a",
            "--workspace-id",
            "workspace-b",
            "--session-id",
            "session-c",
        ])
        .unwrap();
        let ParseOutcome::Run(config) = parsed else {
            panic!("expected run config");
        };

        assert_eq!(config.tenant_id, "tenant-a");
        assert_eq!(config.workspace_id, "workspace-b");
        assert_eq!(config.session_id, "session-c");
    }

    #[test]
    fn parses_pool_status_json_path() {
        let parsed =
            parse_args(["--pool-status-json", "target/evolution/pool-status.json"]).unwrap();
        let ParseOutcome::Run(config) = parsed else {
            panic!("expected run config");
        };

        assert_eq!(
            config.pool_status_json_path,
            Some(PathBuf::from("target/evolution/pool-status.json"))
        );
    }

    #[test]
    fn parses_pool_manifest_json_path() {
        let parsed = parse_args([
            "--pool-manifest-json",
            "target/gemma-chain/apple-model-pool.generated.json",
        ])
        .unwrap();
        let ParseOutcome::Run(config) = parsed else {
            panic!("expected run config");
        };

        assert_eq!(
            config.pool_manifest_json_path,
            Some(PathBuf::from(
                "target/gemma-chain/apple-model-pool.generated.json"
            ))
        );
    }

    #[test]
    fn parses_pool_route_json_path() {
        let parsed = parse_args([
            "--pool-route-json",
            "target/evolution/pool-route-review.json",
        ])
        .unwrap();
        let ParseOutcome::Run(config) = parsed else {
            panic!("expected run config");
        };

        assert_eq!(
            config.pool_route_json_path,
            Some(PathBuf::from("target/evolution/pool-route-review.json"))
        );
    }

    #[test]
    fn parses_pool_budget_fairness_json_path() {
        let parsed = parse_args([
            "--pool-budget-fairness-json",
            "target/evolution/model-pool-budget-fairness.json",
        ])
        .unwrap();
        let ParseOutcome::Run(config) = parsed else {
            panic!("expected run config");
        };

        assert_eq!(
            config.pool_budget_fairness_json_path,
            Some(PathBuf::from(
                "target/evolution/model-pool-budget-fairness.json"
            ))
        );
    }

    #[test]
    fn parses_pool_budget_fairness_gate() {
        let parsed = parse_args([
            "--pool-budget-fairness-json",
            "target/evolution/model-pool-budget-fairness.json",
            "--pool-budget-fairness-gate",
            "--require-pool-budget-policy",
        ])
        .unwrap();
        let ParseOutcome::Run(config) = parsed else {
            panic!("expected run config");
        };

        assert!(config.pool_budget_fairness_gate);
        assert!(config.require_pool_budget_policy);
        assert_eq!(
            config.pool_budget_fairness_json_path,
            Some(PathBuf::from(
                "target/evolution/model-pool-budget-fairness.json"
            ))
        );
    }

    #[test]
    fn parses_remote_chain_gate() {
        let parsed = parse_args([
            "--remote-chain-status-json",
            "target/evolution/remote-chain-status.json",
            "--remote-chain-gate",
        ])
        .unwrap();
        let ParseOutcome::Run(config) = parsed else {
            panic!("expected run config");
        };

        assert!(config.remote_chain_gate);
        assert_eq!(
            config.remote_chain_status_json_path,
            Some(PathBuf::from("target/evolution/remote-chain-status.json"))
        );
    }

    #[test]
    fn parses_pool_capacity_gate() {
        let parsed = parse_args([
            "--pool-status-json",
            "target/evolution/pool-status.json",
            "--pool-capacity-gate",
        ])
        .unwrap();
        let ParseOutcome::Run(config) = parsed else {
            panic!("expected run config");
        };

        assert!(config.pool_capacity_gate);
        assert_eq!(
            config.pool_status_json_path,
            Some(PathBuf::from("target/evolution/pool-status.json"))
        );
    }

    #[test]
    fn parses_pool_alignment_gate() {
        let parsed = parse_args([
            "--pool-manifest-json",
            "target/evolution/pool-manifest.json",
            "--pool-status-json",
            "target/evolution/pool-status.json",
            "--pool-route-json",
            "target/evolution/pool-route-review.json",
            "--pool-alignment-gate",
        ])
        .unwrap();
        let ParseOutcome::Run(config) = parsed else {
            panic!("expected run config");
        };

        assert!(config.pool_alignment_gate);
        assert_eq!(
            config.pool_manifest_json_path,
            Some(PathBuf::from("target/evolution/pool-manifest.json"))
        );
        assert_eq!(
            config.pool_status_json_path,
            Some(PathBuf::from("target/evolution/pool-status.json"))
        );
        assert_eq!(
            config.pool_route_json_path,
            Some(PathBuf::from("target/evolution/pool-route-review.json"))
        );
    }

    #[test]
    fn parses_required_pool_route_gate() {
        let parsed = parse_args([
            "--pool-route-json",
            "target/evolution/pool-route-review.json",
            "--require-pool-route",
        ])
        .unwrap();
        let ParseOutcome::Run(config) = parsed else {
            panic!("expected run config");
        };

        assert!(config.require_pool_route);
        assert_eq!(
            config.pool_route_json_path,
            Some(PathBuf::from("target/evolution/pool-route-review.json"))
        );
    }

    #[test]
    fn parses_refresh_pool_artifacts_with_default_paths() {
        let parsed = parse_args(["--refresh-pool-artifacts"]).unwrap();
        let ParseOutcome::Run(config) = parsed else {
            panic!("expected run config");
        };

        assert!(config.refresh_pool_artifacts);
        assert_eq!(
            config.pool_manifest_json_path,
            Some(PathBuf::from("target/evolution/pool-manifest.json"))
        );
        assert_eq!(
            config.pool_status_json_path,
            Some(PathBuf::from("target/evolution/pool-status.json"))
        );
        assert_eq!(
            config.pool_route_json_path,
            Some(PathBuf::from("target/evolution/pool-route-review.json"))
        );
        assert_eq!(config.pool_route_task_kind, "review");
    }

    #[test]
    fn parses_refresh_pool_artifacts_with_route_task_kind() {
        let parsed = parse_args([
            "--refresh-pool-artifacts",
            "--pool-route-task-kind",
            "test-gate",
            "--pool-route-json",
            "target/evolution/pool-route-test-gate.json",
        ])
        .unwrap();
        let ParseOutcome::Run(config) = parsed else {
            panic!("expected run config");
        };

        assert!(config.refresh_pool_artifacts);
        assert_eq!(config.pool_route_task_kind, "test-gate");
        assert_eq!(
            config.pool_route_json_path,
            Some(PathBuf::from("target/evolution/pool-route-test-gate.json"))
        );
    }

    #[test]
    fn parses_pool_stage_route_task_kinds() {
        let parsed = parse_args([
            "--pool-stage-route-task-kinds",
            "summary,tool-call,preflight,index,review,router,summary",
        ])
        .unwrap();
        let ParseOutcome::Run(config) = parsed else {
            panic!("expected run config");
        };

        assert_eq!(
            config.pool_stage_route_task_kinds,
            vec![
                "summary".to_owned(),
                "router".to_owned(),
                "index".to_owned(),
                "review".to_owned()
            ]
        );
    }

    #[test]
    fn parses_pool_stage_route_gate() {
        let parsed = parse_args([
            "--pool-stage-route-task-kinds",
            "summary,review,index",
            "--pool-stage-route-gate",
        ])
        .unwrap();
        let ParseOutcome::Run(config) = parsed else {
            panic!("expected run config");
        };

        assert!(config.pool_stage_route_gate);
        assert_eq!(
            config.pool_stage_route_task_kinds,
            vec![
                "summary".to_owned(),
                "review".to_owned(),
                "index".to_owned()
            ]
        );
    }

    #[test]
    fn parses_execute_pool_stage_calls() {
        let parsed = parse_args([
            "--pool-stage-route-task-kinds",
            "summary,preflight,review",
            "--execute-pool-stage-calls",
        ])
        .unwrap();
        let ParseOutcome::Run(config) = parsed else {
            panic!("expected run config");
        };

        assert!(config.execute_pool_stage_calls);
        assert_eq!(
            config.pool_stage_route_task_kinds,
            vec![
                "summary".to_owned(),
                "router".to_owned(),
                "review".to_owned()
            ]
        );
    }

    #[test]
    fn parses_required_helper_stage_roles() {
        let parsed = parse_args([
            "--require-helper-stage-roles",
            "summary,router,review,index,test-gate,summary",
        ])
        .unwrap();
        let ParseOutcome::Run(config) = parsed else {
            panic!("expected run config");
        };

        assert_eq!(
            config.required_helper_stage_roles,
            vec![
                "summary".to_owned(),
                "router".to_owned(),
                "review".to_owned(),
                "index".to_owned(),
                "test-gate".to_owned()
            ]
        );
    }

    #[test]
    fn parses_required_latest_helper_stage_roles() {
        let parsed = parse_args([
            "--require-latest-helper-stage-roles",
            "summary,router,review,index,test-gate,index",
        ])
        .unwrap();
        let ParseOutcome::Run(config) = parsed else {
            panic!("expected run config");
        };

        assert_eq!(
            config.required_latest_helper_stage_roles,
            vec![
                "summary".to_owned(),
                "router".to_owned(),
                "review".to_owned(),
                "index".to_owned(),
                "test-gate".to_owned()
            ]
        );
    }

    #[test]
    fn parses_require_useful_latest_helper_stage_feedback() {
        let parsed = parse_args(["--require-useful-latest-helper-stage-feedback"]).unwrap();
        let ParseOutcome::Run(config) = parsed else {
            panic!("expected run config");
        };

        assert!(config.require_useful_latest_helper_stage_feedback);
    }

    #[test]
    fn parses_require_complete_latest_helper_stage_feedback() {
        let parsed = parse_args(["--require-complete-latest-helper-stage-feedback"]).unwrap();
        let ParseOutcome::Run(config) = parsed else {
            panic!("expected run config");
        };

        assert!(config.require_complete_latest_helper_stage_feedback);
    }

    #[test]
    fn parses_require_clean_helper_stage_feedback() {
        let parsed = parse_args(["--require-clean-helper-stage-feedback"]).unwrap();
        let ParseOutcome::Run(config) = parsed else {
            panic!("expected run config");
        };

        assert!(config.require_clean_helper_stage_feedback);
    }

    #[test]
    fn parses_require_final_json_pool_stage_dispatch() {
        let parsed = parse_args(["--require-final-json-pool-stage-dispatch"]).unwrap();
        let ParseOutcome::Run(config) = parsed else {
            panic!("expected run config");
        };

        assert!(config.require_final_json_pool_stage_dispatch);
    }

    #[test]
    fn parses_require_test_gate_pass() {
        let parsed = parse_args(["--require-test-gate-pass"]).unwrap();
        let ParseOutcome::Run(config) = parsed else {
            panic!("expected run config");
        };

        assert!(config.require_test_gate_pass);
    }

    #[test]
    fn parses_require_safe_test_gate_validation_command() {
        let parsed = parse_args(["--require-safe-test-gate-validation-command"]).unwrap();
        let ParseOutcome::Run(config) = parsed else {
            panic!("expected run config");
        };

        assert!(config.require_safe_test_gate_validation_command);
    }

    #[test]
    fn parses_require_configured_validation_run() {
        let parsed = parse_args(["--require-configured-validation-run"]).unwrap();
        let ParseOutcome::Run(config) = parsed else {
            panic!("expected run config");
        };

        assert!(config.require_configured_validation_run);
    }

    #[test]
    fn parses_require_test_gate_validation_run() {
        let parsed = parse_args(["--require-test-gate-validation-run"]).unwrap();
        let ParseOutcome::Run(config) = parsed else {
            panic!("expected run config");
        };

        assert!(config.require_test_gate_validation_run);
    }

    #[test]
    fn rejects_unknown_pool_route_task_kind() {
        let error = parse_args(["--pool-route-task-kind", "giant-12b"]).unwrap_err();

        assert!(error.contains("--pool-route-task-kind"));
    }

    #[test]
    fn rejects_unknown_pool_stage_route_task_kind() {
        let error = parse_args(["--pool-stage-route-task-kinds", "summary,giant-12b"]).unwrap_err();

        assert!(error.contains("--pool-route-task-kind"));
    }

    #[test]
    fn rejects_unknown_required_helper_stage_role() {
        let error = parse_args(["--require-helper-stage-roles", "summary,quality"]).unwrap_err();

        assert!(error.contains("--require-helper-stage-roles"));
    }

    #[test]
    fn parses_pool_lease_options() {
        let parsed = parse_args([
            "--pool-route-json",
            "target/evolution/pool-route-summary.json",
            "--require-pool-route",
            "--pool-lease-dir",
            "target/evolution/pool-leases",
            "--pool-lease-ttl-secs",
            "60",
            "--pool-lease-wait-secs",
            "15",
            "--pool-lease-poll-secs",
            "3",
            "--pool-lease-busy-policy",
            "skip-low-priority",
            "--max-pool-lease-skips",
            "7",
        ])
        .unwrap();
        let ParseOutcome::Run(config) = parsed else {
            panic!("expected run config");
        };

        assert!(config.require_pool_route);
        assert_eq!(
            config.pool_lease_dir,
            Some(PathBuf::from("target/evolution/pool-leases"))
        );
        assert_eq!(config.pool_lease_ttl_secs, 60);
        assert_eq!(config.pool_lease_wait_secs, 15);
        assert_eq!(config.pool_lease_poll_secs, 3);
        assert_eq!(
            config.pool_lease_busy_policy,
            PoolLeaseBusyPolicy::SkipLowPriority
        );
        assert_eq!(config.max_pool_lease_skips, Some(7));
    }

    #[test]
    fn parses_experience_audit_gate_thresholds() {
        let parsed = parse_args([
            "--experience-audit-gate",
            "--experience-audit-limit",
            "12",
            "--max-index-overlong-records",
            "2",
            "--max-index-overlong-without-clean-gist",
            "0",
            "--max-index-record-chars",
            "12000",
            "--max-index-noisy-records",
            "2",
            "--max-index-noise-penalty",
            "0.25",
            "--min-index-quality-score",
            "0.75",
            "--allow-index-retrieval-not-ready",
            "--max-quarantine-candidates",
            "1",
            "--max-repairable-legacy-records",
            "3",
            "--max-legacy-metadata-without-clean-gist",
            "4",
        ])
        .unwrap();
        let ParseOutcome::Run(config) = parsed else {
            panic!("expected run config");
        };

        assert!(config.experience_audit_gate);
        assert_eq!(config.experience_audit_limit, 12);
        assert_eq!(config.max_index_overlong_records, Some(2));
        assert_eq!(config.max_index_overlong_without_clean_gist, 0);
        assert_eq!(config.max_index_record_chars, Some(12000));
        assert_eq!(config.max_index_noisy_records, 2);
        assert_eq!(config.max_index_noise_penalty, 0.25);
        assert_eq!(config.min_index_quality_score, 0.75);
        assert!(!config.require_index_retrieval_ready);
        assert_eq!(config.max_quarantine_candidates, 1);
        assert_eq!(config.max_repairable_legacy_records, 3);
        assert_eq!(config.max_legacy_metadata_without_clean_gist, 4);
    }

    #[test]
    fn parses_report_mode() {
        let parsed = parse_args(["--report", "--ledger", "x.jsonl"]).unwrap();
        let ParseOutcome::Report(config) = parsed else {
            panic!("expected report config");
        };

        assert_eq!(config.ledger_path, PathBuf::from("x.jsonl"));
    }

    #[test]
    fn parses_rust_check_options() {
        let parsed = parse_args([
            "--rust-check-code",
            "pub fn ok() {}",
            "--rust-check-edition",
            "2024",
            "--rust-check-case",
            "case-a",
        ])
        .unwrap();
        let ParseOutcome::Run(config) = parsed else {
            panic!("expected run config");
        };

        assert_eq!(config.rust_check_code.as_deref(), Some("pub fn ok() {}"));
        assert_eq!(config.rust_check_edition, "2024");
        assert_eq!(config.rust_check_case.as_deref(), Some("case-a"));
    }

    #[test]
    fn parses_validation_gate_options() {
        let parsed = parse_args([
            "--validation-command",
            "cargo test --manifest-path tools/evolution-loop/Cargo.toml",
            "--validation-workdir",
            "D:/rust-norion",
            "--validation-timeout-secs",
            "120",
            "--validation-phase",
            "both",
        ])
        .unwrap();
        let ParseOutcome::Run(config) = parsed else {
            panic!("expected run config");
        };

        assert_eq!(
            config.validation_command.as_deref(),
            Some("cargo test --manifest-path tools/evolution-loop/Cargo.toml")
        );
        assert_eq!(
            config.validation_workdir,
            Some(PathBuf::from("D:/rust-norion"))
        );
        assert_eq!(config.validation_timeout_secs, 120);
        assert_eq!(config.validation_phase, ValidationPhase::Both);
        assert!(config.validation_phase.runs_pre());
        assert!(config.validation_phase.runs_post());
    }

    #[test]
    fn parses_use_test_gate_validation_command() {
        let parsed = parse_args(["--use-test-gate-validation-command"]).unwrap();
        let ParseOutcome::Run(config) = parsed else {
            panic!("expected run config");
        };

        assert!(config.use_test_gate_validation_command);
    }

    #[test]
    fn parses_report_gate_thresholds() {
        let parsed = parse_args([
            "--report-gate",
            "--report-continuation-gate",
            "--min-report-rounds",
            "2",
            "--min-success-rate",
            "75",
            "--min-feedback-total",
            "0",
            "--min-rust-checks",
            "2",
            "--min-rust-feedback-total",
            "1",
            "--max-stream-truncations",
            "1",
            "--max-missing-final",
            "2",
            "--max-runtime-response-failures",
            "3",
            "--require-round-wall-clock-evidence",
            "--strict-ledger-hygiene",
            "--allow-last-failure",
        ])
        .unwrap();
        let ParseOutcome::Report(config) = parsed else {
            panic!("expected report config");
        };

        assert!(config.report_gate);
        assert!(config.report_continuation_gate);
        assert_eq!(config.min_report_rounds, 2);
        assert_eq!(config.min_success_rate, Some(75.0));
        assert_eq!(config.min_feedback_total, None);
        assert_eq!(config.min_rust_checks, Some(2));
        assert_eq!(config.min_rust_feedback_total, Some(1));
        assert_eq!(config.max_stream_truncations, 1);
        assert_eq!(config.max_missing_final, 2);
        assert_eq!(config.max_runtime_response_failures, 3);
        assert!(config.require_round_wall_clock_evidence);
        assert!(config.strict_ledger_hygiene);
        assert!(!config.require_last_success);
    }

    #[test]
    fn parses_report_json_path() {
        let parsed = parse_args(["--report-json", "target/evolution/report.json"]).unwrap();
        let ParseOutcome::Report(config) = parsed else {
            panic!("expected report config");
        };

        assert_eq!(
            config.report_json_path,
            Some(PathBuf::from("target/evolution/report.json"))
        );
    }

    #[test]
    fn parses_mvp_demo_with_report_json_path() {
        let parsed = parse_args([
            "--mvp-demo",
            "--report-json",
            "target/evolution/mvp-demo.json",
        ])
        .unwrap();
        let ParseOutcome::MvpDemo(config) = parsed else {
            panic!("expected mvp demo config");
        };

        assert_eq!(
            config.report_json_path,
            Some(PathBuf::from("target/evolution/mvp-demo.json"))
        );
    }

    #[test]
    fn parses_newapi_live_smoke_options() {
        let parsed = parse_args([
            "--newapi-live-smoke",
            "--min-newapi-live-models",
            "3",
            "--newapi-live-smoke-json",
            "target/evolution/newapi-live-smoke.json",
        ])
        .unwrap();
        let ParseOutcome::NewApiLiveSmoke(config) = parsed else {
            panic!("expected NewAPI live smoke config");
        };

        assert!(config.newapi_live_smoke);
        assert_eq!(config.newapi_live_smoke_min_successes, 3);
        assert_eq!(
            config.newapi_live_smoke_json_path,
            Some(PathBuf::from("target/evolution/newapi-live-smoke.json"))
        );
    }

    #[test]
    fn parses_worker_window_status_json_path_without_forcing_report_mode() {
        let parsed = parse_args([
            "--worker-window-status-json",
            "docs/runbooks/worker-window-status-r21.example.json",
        ])
        .unwrap();
        let ParseOutcome::Run(config) = parsed else {
            panic!("expected run config");
        };

        assert_eq!(
            config.worker_window_status_json_path,
            Some(PathBuf::from(
                "docs/runbooks/worker-window-status-r21.example.json"
            ))
        );
        assert!(!config.report);
        assert!(!config.report_gate);
    }

    #[test]
    fn parses_clean_room_handoff_json_paths_without_forcing_report_mode() {
        let parsed = parse_args([
            "--memory-startup-admission-json",
            "docs/runbooks/smartsteam-evolution-loop-memory-admission-r23.example.json",
            "--agent-clean-room-replacement-plan-json",
            "docs/runbooks/smartsteam-evolution-loop-agent-replacement-plan-r23.example.json",
        ])
        .unwrap();
        let ParseOutcome::Run(config) = parsed else {
            panic!("expected run config");
        };

        assert_eq!(
            config.memory_startup_admission_json_path,
            Some(PathBuf::from(
                "docs/runbooks/smartsteam-evolution-loop-memory-admission-r23.example.json"
            ))
        );
        assert_eq!(
            config.agent_clean_room_replacement_plan_json_path,
            Some(PathBuf::from(
                "docs/runbooks/smartsteam-evolution-loop-agent-replacement-plan-r23.example.json"
            ))
        );
        assert!(!config.report);
        assert!(!config.report_gate);
    }

    #[test]
    fn parses_clean_room_batch_status_json_path_without_forcing_report_mode() {
        let parsed = parse_args([
            "--clean-room-batch-status-json",
            "docs/runbooks/smartsteam-evolution-loop-clean-room-batch-status-r25.example.json",
        ])
        .unwrap();
        let ParseOutcome::Run(config) = parsed else {
            panic!("expected run config");
        };

        assert_eq!(
            config.clean_room_batch_status_json_path,
            Some(PathBuf::from(
                "docs/runbooks/smartsteam-evolution-loop-clean-room-batch-status-r25.example.json"
            ))
        );
        assert!(!config.report);
        assert!(!config.report_gate);
    }

    #[test]
    fn parses_run_report_json_without_entering_report_mode() {
        let parsed = parse_args([
            "--forever",
            "--run-report-json",
            "target/evolution/daemon/report.json",
            "--run-report-gate",
            "--run-report-continuation-gate",
        ])
        .unwrap();
        let ParseOutcome::Run(config) = parsed else {
            panic!("expected run config");
        };

        assert_eq!(
            config.run_report_json_path,
            Some(PathBuf::from("target/evolution/daemon/report.json"))
        );
        assert!(config.run_report_gate);
        assert!(config.run_report_continuation_gate);
        assert!(!config.report);
        assert!(!config.report_gate);
        assert!(!config.report_continuation_gate);
    }

    #[test]
    fn rejects_report_mode_gate_with_run_report_json() {
        let error = parse_args([
            "--run-report-json",
            "target/evolution/daemon/report.json",
            "--report-continuation-gate",
        ])
        .unwrap_err();

        assert!(error.contains("--run-report-json cannot be combined"));
        assert!(error.contains("--run-report-continuation-gate"));
    }

    #[test]
    fn parses_profile_outcome_replay_options_without_forcing_report_mode() {
        let parsed = parse_args([
            "--profile-outcome-log",
            "target/evolution/request-outcomes.jsonl",
            "--profile-outcome-min-samples",
            "5",
        ])
        .unwrap();
        let ParseOutcome::Run(config) = parsed else {
            panic!("expected run config");
        };

        assert_eq!(
            config.profile_outcome_log_path,
            Some(PathBuf::from("target/evolution/request-outcomes.jsonl"))
        );
        assert_eq!(config.profile_outcome_min_samples, 5);
        assert!(!config.report);
        assert!(!config.report_gate);
    }

    #[test]
    fn report_context_is_enabled_by_default_and_can_be_disabled() {
        let ParseOutcome::Run(default_config) = parse_args([] as [&str; 0]).unwrap() else {
            panic!("expected run config");
        };
        assert!(default_config.report_context);

        let ParseOutcome::Run(disabled_config) = parse_args(["--no-report-context"]).unwrap()
        else {
            panic!("expected run config");
        };
        assert!(!disabled_config.report_context);
    }

    #[test]
    fn clamps_large_token_budget_to_model_ceiling() {
        let parsed = parse_args(["--max-tokens", "999999"]).unwrap();
        let ParseOutcome::Run(config) = parsed else {
            panic!("expected run config");
        };

        assert_eq!(config.max_tokens, 262_144);
    }
}
