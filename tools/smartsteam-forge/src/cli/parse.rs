use std::iter::Peekable;

use smartsteam_forge::{SessionFilter, StreamEndpoint};

use super::CliConfig;
use super::ModelPoolWatchConfig;
use crate::app;

const DEFAULT_MODEL_POOL_WATCH_INTERVAL_SECS: u64 = 5;
const DEFAULT_EVOLUTION_CANDIDATES_LIMIT: usize = 5;
const DEFAULT_SESSION_LIST_LIMIT: usize = 50;

impl CliConfig {
    pub(crate) fn parse<I>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = String>,
    {
        let mut config = Self {
            backend: "127.0.0.1:7878".to_owned(),
            backend_overridden: false,
            mock: false,
            prompt: None,
            endpoint: None,
            request_timeout_secs: None,
            connect_timeout_ms: None,
            read_timeout_ms: None,
            context_messages: None,
            max_tokens: None,
            health_check: false,
            experience_hygiene: false,
            experience_hygiene_quarantine: false,
            experience_hygiene_limit: 20,
            experience_repair: false,
            experience_repair_limit: 20,
            experience_cleanup_audit: false,
            experience_cleanup_audit_limit: 20,
            model_pool_status: false,
            model_pool_manifest: false,
            model_pool_advice: false,
            model_pool_smoke: false,
            model_pool_route: None,
            model_pool_call: None,
            model_pool_watch: None,
            evolution_status: false,
            evolution_status_json: false,
            evolution_strict_summary: false,
            evolution_strict_summary_json: false,
            evolution_strict_summary_path: None,
            evolution_start: false,
            evolution_stop: false,
            evolution_check_only: false,
            evolution_start_check_json: false,
            evolution_watch: None,
            evolution_candidates: false,
            evolution_candidate_list: false,
            evolution_candidate_gate: false,
            evolution_candidates_limit: DEFAULT_EVOLUTION_CANDIDATES_LIMIT,
            evolution_candidates_save: false,
            evolution_candidates_backlog: None,
            evolution_candidate_mark: None,
            evolution_candidate_apply_check: None,
            evolution_candidate_validate: None,
            evolution_candidate_validation_command: None,
            evolution_candidate_validation_status: None,
            evolution_candidate_status: None,
            evolution_candidate_note: None,
            evolution_work_dir: "target\\evolution\\daemon".to_owned(),
            evolution_interval_secs: None,
            evolution_max_tokens: None,
            evolution_max_total_tokens: None,
            evolution_max_runtime_secs: None,
            evolution_max_failures: None,
            evolution_max_no_feedback_rounds: None,
            evolution_timeout_secs: None,
            doctor: false,
            preflight_check: false,
            require_health: false,
            require_safe_device: false,
            session_command: None,
            session_limit: DEFAULT_SESSION_LIST_LIMIT,
            help: false,
        };
        let mut candidate_limit_seen = false;
        let mut args = args.into_iter().peekable();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--backend" => {
                    let value = take_value("--backend", &mut args)?;
                    config.backend = trim_http_prefix(&value).to_owned();
                    config.backend_overridden = true;
                }
                "--mock" => config.mock = true,
                "--prompt" | "--once" => {
                    config.prompt = Some(take_value(&arg, &mut args)?);
                }
                "--smoke" => {
                    config.prompt = Some(
                        "Reply with one short SmartSteam Forge smoke-test sentence.".to_owned(),
                    );
                }
                "--mode" => {
                    let value = take_value("--mode", &mut args)?;
                    config.endpoint = Some(parse_endpoint(&value)?);
                }
                "--timeout-secs" | "--request-timeout-secs" => {
                    let value = take_value(&arg, &mut args)?;
                    config.request_timeout_secs = Some(parse_positive_u64(&arg, &value)?);
                }
                "--connect-timeout-ms" => {
                    let value = take_value("--connect-timeout-ms", &mut args)?;
                    config.connect_timeout_ms =
                        Some(parse_positive_u64("--connect-timeout-ms", &value)?);
                }
                "--read-timeout-ms" => {
                    let value = take_value("--read-timeout-ms", &mut args)?;
                    config.read_timeout_ms = Some(parse_positive_u64("--read-timeout-ms", &value)?);
                }
                "--context-messages" | "--context-window" | "--max-context-messages" => {
                    let value = take_value(&arg, &mut args)?;
                    config.context_messages = Some(parse_positive_usize(&arg, &value)?);
                }
                "--max-tokens" | "--max-output-tokens" => {
                    let value = take_value(&arg, &mut args)?;
                    config.max_tokens = Some(parse_optional_max_tokens(&arg, &value)?);
                }
                "--health" | "--check" => {
                    config.health_check = true;
                }
                "--hygiene" | "--experience-hygiene" => {
                    config.experience_hygiene = true;
                }
                "--hygiene-quarantine"
                | "--experience-hygiene-quarantine"
                | "--hygiene-dry-run" => {
                    config.experience_hygiene = true;
                    config.experience_hygiene_quarantine = true;
                }
                "--hygiene-limit" | "--experience-hygiene-limit" => {
                    let value = take_value(&arg, &mut args)?;
                    config.experience_hygiene_limit = parse_positive_usize(&arg, &value)?;
                }
                "--repair"
                | "--experience-repair"
                | "--repair-dry-run"
                | "--experience-repair-dry-run" => {
                    config.experience_repair = true;
                }
                "--repair-limit" | "--experience-repair-limit" => {
                    let value = take_value(&arg, &mut args)?;
                    config.experience_repair = true;
                    config.experience_repair_limit = parse_positive_usize(&arg, &value)?;
                }
                "--audit" | "--cleanup-audit" | "--experience-cleanup-audit" => {
                    config.experience_cleanup_audit = true;
                }
                "--audit-limit" | "--cleanup-audit-limit" | "--experience-cleanup-audit-limit" => {
                    let value = take_value(&arg, &mut args)?;
                    config.experience_cleanup_audit = true;
                    config.experience_cleanup_audit_limit = parse_positive_usize(&arg, &value)?;
                }
                "--pool-status" | "--model-pool-status" => {
                    config.model_pool_status = true;
                }
                "--pool-manifest" | "--model-pool-manifest" | "--apple-pool-manifest" => {
                    config.model_pool_manifest = true;
                }
                "--pool-advice"
                | "--model-pool-advice"
                | "--apple-pool-advice"
                | "--pool-capacity" => {
                    config.model_pool_advice = true;
                }
                "--pool-smoke" | "--model-pool-smoke" | "--apple-pool-smoke" | "--smoke-pool" => {
                    config.model_pool_smoke = true;
                }
                "--pool-watch" | "--model-pool-watch" | "--watch-pool" => {
                    let interval_secs = take_optional_value(&mut args)
                        .map(|value| parse_positive_u64(&arg, &value))
                        .transpose()?
                        .unwrap_or(DEFAULT_MODEL_POOL_WATCH_INTERVAL_SECS);
                    config.model_pool_watch = Some(ModelPoolWatchConfig {
                        interval_secs,
                        max_iterations: config
                            .model_pool_watch
                            .and_then(|watch| watch.max_iterations),
                    });
                }
                "--pool-watch-count" | "--model-pool-watch-count" | "--watch-count" => {
                    let value = take_value(&arg, &mut args)?;
                    let max_iterations = parse_positive_usize(&arg, &value)?;
                    let watch = config.model_pool_watch.get_or_insert(ModelPoolWatchConfig {
                        interval_secs: DEFAULT_MODEL_POOL_WATCH_INTERVAL_SECS,
                        max_iterations: None,
                    });
                    watch.max_iterations = Some(max_iterations);
                }
                "--pool-route" | "--model-pool-route" | "--route-plan" => {
                    let value = take_value(&arg, &mut args)?;
                    config.model_pool_route = Some(parse_pool_task_kind(&arg, &value)?);
                }
                "--pool-call" | "--model-pool-call" => {
                    let value = take_value(&arg, &mut args)?;
                    config.model_pool_call = Some(parse_pool_task_kind(&arg, &value)?);
                }
                "--evolution-status" | "--daemon-status" => {
                    config.evolution_status = true;
                }
                "--evolution-status-json" | "--evolution-json" | "--daemon-status-json" => {
                    config.evolution_status = true;
                    config.evolution_status_json = true;
                }
                "--evolution-strict-summary"
                | "--strict-evolution-summary"
                | "--strict-summary" => {
                    config.evolution_strict_summary = true;
                }
                "--evolution-strict-summary-json"
                | "--strict-evolution-summary-json"
                | "--strict-summary-json" => {
                    config.evolution_strict_summary = true;
                    config.evolution_strict_summary_json = true;
                }
                "--evolution-strict-summary-path" | "--strict-summary-path" => {
                    config.evolution_strict_summary = true;
                    config.evolution_strict_summary_path = Some(take_value(&arg, &mut args)?);
                }
                "--evolution-start" | "--daemon-start" => {
                    config.evolution_start = true;
                }
                "--evolution-stop" | "--daemon-stop" => {
                    config.evolution_stop = true;
                }
                "--evolution-check-only" | "--daemon-check-only" => {
                    config.evolution_check_only = true;
                }
                "--evolution-start-check" | "--daemon-start-check" => {
                    config.evolution_start = true;
                    config.evolution_check_only = true;
                }
                "--evolution-start-check-json" | "--daemon-start-check-json" => {
                    config.evolution_start = true;
                    config.evolution_check_only = true;
                    config.evolution_start_check_json = true;
                }
                "--evolution-stop-check" | "--daemon-stop-check" => {
                    config.evolution_stop = true;
                    config.evolution_check_only = true;
                }
                "--evolution-watch" | "--daemon-watch" => {
                    let interval_secs = take_optional_value(&mut args)
                        .map(|value| parse_positive_u64(&arg, &value))
                        .transpose()?
                        .unwrap_or(DEFAULT_MODEL_POOL_WATCH_INTERVAL_SECS);
                    config.evolution_watch = Some(ModelPoolWatchConfig {
                        interval_secs,
                        max_iterations: config
                            .evolution_watch
                            .and_then(|watch| watch.max_iterations),
                    });
                }
                "--evolution-watch-count" | "--daemon-watch-count" => {
                    let value = take_value(&arg, &mut args)?;
                    let max_iterations = parse_positive_usize(&arg, &value)?;
                    let watch = config.evolution_watch.get_or_insert(ModelPoolWatchConfig {
                        interval_secs: DEFAULT_MODEL_POOL_WATCH_INTERVAL_SECS,
                        max_iterations: None,
                    });
                    watch.max_iterations = Some(max_iterations);
                }
                "--evolution-candidates" | "--daemon-candidates" => {
                    config.evolution_candidates = true;
                }
                "--evolution-candidate-list" | "--daemon-candidate-list" => {
                    config.evolution_candidate_list = true;
                }
                "--evolution-candidate-gate" | "--daemon-candidate-gate" => {
                    config.evolution_candidate_gate = true;
                }
                "--evolution-candidates-save" | "--daemon-candidates-save" => {
                    config.evolution_candidates = true;
                    config.evolution_candidates_save = true;
                }
                "--evolution-candidates-backlog" | "--daemon-candidates-backlog" => {
                    let value = take_value(&arg, &mut args)?;
                    config.evolution_candidates_backlog = Some(value);
                }
                "--evolution-candidate-mark" | "--daemon-candidate-mark" => {
                    config.evolution_candidate_mark = Some(take_value(&arg, &mut args)?);
                }
                "--evolution-candidate-apply-check" | "--daemon-candidate-apply-check" => {
                    config.evolution_candidate_apply_check = Some(take_value(&arg, &mut args)?);
                }
                "--evolution-candidate-validate" | "--daemon-candidate-validate" => {
                    config.evolution_candidate_validate = Some(take_value(&arg, &mut args)?);
                }
                "--evolution-candidate-validation-command"
                | "--daemon-candidate-validation-command" => {
                    config.evolution_candidate_validation_command =
                        Some(take_value(&arg, &mut args)?);
                }
                "--evolution-candidate-validation-status"
                | "--daemon-candidate-validation-status" => {
                    config.evolution_candidate_validation_status =
                        Some(take_value(&arg, &mut args)?);
                }
                "--evolution-candidate-status" | "--daemon-candidate-status" => {
                    config.evolution_candidate_status = Some(take_value(&arg, &mut args)?);
                }
                "--evolution-candidate-note" | "--daemon-candidate-note" => {
                    config.evolution_candidate_note = Some(take_value(&arg, &mut args)?);
                }
                "--evolution-candidates-limit" | "--daemon-candidates-limit" => {
                    let value = take_value(&arg, &mut args)?;
                    candidate_limit_seen = true;
                    config.evolution_candidates_limit = parse_positive_usize(&arg, &value)?;
                }
                "--evolution-work-dir" | "--daemon-work-dir" => {
                    config.evolution_work_dir = take_value(&arg, &mut args)?;
                }
                "--evolution-interval-secs" | "--daemon-interval-secs" => {
                    let value = take_value(&arg, &mut args)?;
                    config.evolution_interval_secs = Some(parse_positive_u64(&arg, &value)?);
                }
                "--evolution-max-tokens" | "--daemon-max-tokens" => {
                    let value = take_value(&arg, &mut args)?;
                    config.evolution_max_tokens = Some(parse_positive_u64(&arg, &value)?);
                }
                "--evolution-max-total-tokens" | "--daemon-max-total-tokens" => {
                    let value = take_value(&arg, &mut args)?;
                    config.evolution_max_total_tokens = Some(parse_nonnegative_u64(&arg, &value)?);
                }
                "--evolution-max-runtime-secs" | "--daemon-max-runtime-secs" => {
                    let value = take_value(&arg, &mut args)?;
                    config.evolution_max_runtime_secs = Some(parse_nonnegative_u64(&arg, &value)?);
                }
                "--evolution-max-failures" | "--daemon-max-failures" => {
                    let value = take_value(&arg, &mut args)?;
                    config.evolution_max_failures = Some(parse_positive_u64(&arg, &value)?);
                }
                "--evolution-max-no-feedback-rounds" | "--daemon-max-no-feedback-rounds" => {
                    let value = take_value(&arg, &mut args)?;
                    config.evolution_max_no_feedback_rounds =
                        Some(parse_nonnegative_u64(&arg, &value)?);
                }
                "--evolution-timeout-secs" | "--daemon-timeout-secs" => {
                    let value = take_value(&arg, &mut args)?;
                    config.evolution_timeout_secs = Some(parse_positive_u64(&arg, &value)?);
                }
                "--doctor" | "--diagnose" | "--diagnostic" => {
                    config.doctor = true;
                }
                "--preflight" | "--ready" => {
                    config.preflight_check = true;
                }
                "--require-health" => {
                    config.require_health = true;
                }
                "--require-safe-device" | "--safe-device" | "--device-guard" => {
                    config.require_safe_device = true;
                }
                "--sessions" | "--history" => {
                    let filter = take_optional_session_filter(&mut args)?;
                    config.session_command = Some(app::SessionCliCommand::List {
                        filter,
                        limit: config.session_limit,
                    });
                }
                "--summary" | "--summarize" => {
                    let selector = take_optional_value(&mut args).unwrap_or_default();
                    config.session_command = Some(app::SessionCliCommand::Summary { selector });
                }
                "--session-limit" => {
                    let value = take_value("--session-limit", &mut args)?;
                    config.session_limit = parse_positive_usize("--session-limit", &value)?;
                    if let Some(app::SessionCliCommand::List { limit, .. }) =
                        &mut config.session_command
                    {
                        *limit = config.session_limit;
                    }
                }
                "-h" | "--help" => config.help = true,
                _ => return Err(format!("unknown argument: {arg}")),
            }
        }
        if config.evolution_candidates_backlog.is_some()
            && !has_explicit_evolution_action(&config)
            && config.evolution_candidate_mark.is_none()
            && config.evolution_candidate_apply_check.is_none()
            && config.evolution_candidate_validate.is_none()
            && !config.evolution_candidate_gate
            && !config.evolution_candidate_list
        {
            config.evolution_candidates = true;
            config.evolution_candidates_save = true;
        }
        if candidate_limit_seen && !config.evolution_candidate_list {
            config.evolution_candidates = true;
        }
        validate_evolution_daemon_action(&config)?;
        Ok(config)
    }
}

fn has_explicit_evolution_action(config: &CliConfig) -> bool {
    config.evolution_status
        || config.evolution_status_json
        || config.evolution_strict_summary
        || config.evolution_strict_summary_json
        || config.evolution_start
        || config.evolution_stop
        || config.evolution_watch.is_some()
        || config.evolution_candidates
}

fn validate_evolution_daemon_action(config: &CliConfig) -> Result<(), String> {
    if config.evolution_watch.is_some() && config.evolution_status_json {
        return Err("--evolution-watch cannot be combined with --evolution-status-json".to_owned());
    }
    let action_count = [
        config.evolution_status,
        config.evolution_strict_summary,
        config.evolution_start,
        config.evolution_stop,
        config.evolution_watch.is_some(),
        config.evolution_candidates,
        config.evolution_candidate_list,
        config.evolution_candidate_gate,
        config.evolution_candidate_mark.is_some(),
        config.evolution_candidate_apply_check.is_some(),
        config.evolution_candidate_validate.is_some(),
    ]
    .into_iter()
    .filter(|enabled| *enabled)
    .count();
    if action_count > 1 {
        return Err(
            "choose only one evolution daemon action: status, strict-summary, candidates, candidate-list, candidate-gate, candidate-apply-check, candidate-validate, start, stop, or watch"
                .to_owned(),
        );
    }
    if config.evolution_check_only && !config.evolution_start && !config.evolution_stop {
        return Err(
            "--evolution-check-only requires --evolution-start or --evolution-stop".to_owned(),
        );
    }
    if has_evolution_start_options(config) && !config.evolution_start {
        return Err(
            "evolution daemon budget options require --evolution-start or --evolution-start-check"
                .to_owned(),
        );
    }
    if config.evolution_candidate_mark.is_some() && config.evolution_candidate_status.is_none() {
        return Err("--evolution-candidate-mark requires --evolution-candidate-status".to_owned());
    }
    if config.evolution_candidate_status.is_some()
        && config.evolution_candidate_mark.is_none()
        && !config.evolution_candidate_list
    {
        return Err("--evolution-candidate-status requires --evolution-candidate-mark or --evolution-candidate-list".to_owned());
    }
    if config.evolution_candidate_note.is_some()
        && config.evolution_candidate_mark.is_none()
        && config.evolution_candidate_validate.is_none()
    {
        return Err(
            "--evolution-candidate-note requires --evolution-candidate-mark or --evolution-candidate-validate"
                .to_owned(),
        );
    }
    if config.evolution_candidate_validate.is_some()
        && config.evolution_candidate_validation_command.is_none()
    {
        return Err(
            "--evolution-candidate-validate requires --evolution-candidate-validation-command"
                .to_owned(),
        );
    }
    if config.evolution_candidate_validate.is_some()
        && config.evolution_candidate_validation_status.is_none()
    {
        return Err(
            "--evolution-candidate-validate requires --evolution-candidate-validation-status"
                .to_owned(),
        );
    }
    if config.evolution_candidate_validation_command.is_some()
        && config.evolution_candidate_validate.is_none()
    {
        return Err(
            "--evolution-candidate-validation-command requires --evolution-candidate-validate"
                .to_owned(),
        );
    }
    if config.evolution_candidate_validation_status.is_some()
        && config.evolution_candidate_validate.is_none()
    {
        return Err(
            "--evolution-candidate-validation-status requires --evolution-candidate-validate"
                .to_owned(),
        );
    }
    Ok(())
}

fn has_evolution_start_options(config: &CliConfig) -> bool {
    config.evolution_interval_secs.is_some()
        || config.evolution_max_tokens.is_some()
        || config.evolution_max_total_tokens.is_some()
        || config.evolution_max_runtime_secs.is_some()
        || config.evolution_max_failures.is_some()
        || config.evolution_max_no_feedback_rounds.is_some()
        || config.evolution_timeout_secs.is_some()
}

fn trim_http_prefix(value: &str) -> &str {
    value
        .strip_prefix("http://")
        .or_else(|| value.strip_prefix("https://"))
        .unwrap_or(value)
}

fn take_value(name: &str, args: &mut impl Iterator<Item = String>) -> Result<String, String> {
    args.next()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{name} requires a value"))
}

fn parse_positive_u64(name: &str, value: &str) -> Result<u64, String> {
    let parsed = value
        .parse::<u64>()
        .map_err(|_| format!("{name} requires a positive integer"))?;
    if parsed == 0 {
        return Err(format!("{name} requires a positive integer"));
    }
    Ok(parsed)
}

fn parse_nonnegative_u64(name: &str, value: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("{name} requires a nonnegative integer"))
}

fn parse_positive_usize(name: &str, value: &str) -> Result<usize, String> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| format!("{name} requires a positive integer"))?;
    if parsed == 0 {
        return Err(format!("{name} requires a positive integer"));
    }
    Ok(parsed)
}

fn parse_optional_max_tokens(name: &str, value: &str) -> Result<Option<usize>, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "auto" | "default" | "backend" | "none" | "off" => Ok(None),
        _ => parse_positive_usize(name, value).map(|value| Some(value.min(262_144))),
    }
}

fn parse_pool_task_kind(name: &str, value: &str) -> Result<String, String> {
    let value = value.trim().to_ascii_lowercase();
    match value.as_str() {
        "auto" | "summary" | "review" | "index" | "quality" => Ok(value),
        "spare" => Ok("index".to_owned()),
        "repo-index" | "repository-index" => Ok("index".to_owned()),
        "test-gate" | "test" | "gate" => Ok("test-gate".to_owned()),
        _ => Err(format!(
            "{name} must be one of auto, summary, review, test-gate, index, or quality"
        )),
    }
}

fn take_optional_session_filter<I>(args: &mut Peekable<I>) -> Result<SessionFilter, String>
where
    I: Iterator<Item = String>,
{
    let Some(value) = take_optional_value(args) else {
        return Ok(SessionFilter::All);
    };
    SessionFilter::parse(&value).ok_or_else(|| {
        format!("unsupported --sessions filter: {value}. Use all, passed, or failed")
    })
}

fn take_optional_value<I>(args: &mut Peekable<I>) -> Option<String>
where
    I: Iterator<Item = String>,
{
    match args.peek() {
        Some(value) if !value.starts_with('-') => args.next(),
        _ => None,
    }
}

fn parse_endpoint(value: &str) -> Result<StreamEndpoint, String> {
    match value {
        "chat" => Ok(StreamEndpoint::Chat),
        "generate" => Ok(StreamEndpoint::Generate),
        "business-cycle" | "business" => Ok(StreamEndpoint::BusinessCycle),
        _ => Err(format!(
            "unsupported --mode value: {value}. Use chat, generate, or business-cycle"
        )),
    }
}
