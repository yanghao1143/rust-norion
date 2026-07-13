use std::collections::HashMap;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::args::Config;
use crate::http;
use crate::json::{
    json_array_field, json_bool_field, json_object_field, json_string, json_string_field,
    json_u64_field, parse_json_object_array, preview_text,
};
use crate::model_policy;
use crate::pool_stage::PoolStageDispatchPlan;
use crate::validation;

pub(crate) const DEFAULT_TEST_GATE_VALIDATION_COMMAND: &str =
    "cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --no-fail-fast";
const DEFAULT_NEWAPI_MODEL_OUTCOMES: &str = "target/evolution/newapi-model-outcomes.jsonl";
const NEWAPI_MODEL_FAILURE_COOLDOWN_SECS: u64 = 6 * 60 * 60;
const MAX_API_KEY_FILE_BYTES: u64 = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PoolStageCallInput<'a> {
    pub(crate) task_kind: &'a str,
    pub(crate) case_name: &'a str,
    pub(crate) round: usize,
    pub(crate) validation_timestamp_unix: Option<u64>,
    pub(crate) validation_evidence: Option<&'a PoolStageValidationEvidence<'a>>,
    pub(crate) original_prompt: &'a str,
    pub(crate) primary_answer: Option<&'a str>,
    pub(crate) final_json: Option<&'a str>,
    pub(crate) dispatch_plan: Option<&'a PoolStageDispatchPlan>,
    pub(crate) completed_roles: &'a [String],
    pub(crate) max_tokens: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PoolStageValidationEvidence<'a> {
    pub(crate) phase: &'a str,
    pub(crate) command_source: &'a str,
    pub(crate) command_safety: &'a str,
    pub(crate) command_preview: &'a str,
    pub(crate) status_code: Option<i32>,
    pub(crate) elapsed_ms: u64,
    pub(crate) stdout_tail: &'a str,
    pub(crate) stderr_tail: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PoolStageCallResult {
    pub(crate) task_kind: String,
    pub(crate) ok: bool,
    pub(crate) selected_role: Option<String>,
    pub(crate) selected_model: Option<String>,
    pub(crate) selected_port: Option<u64>,
    pub(crate) selected_base_url: Option<String>,
    pub(crate) answer: Option<String>,
    pub(crate) elapsed_ms: Option<u64>,
    pub(crate) answer_chars: Option<u64>,
    pub(crate) answer_bytes: Option<u64>,
    pub(crate) answer_approx_tokens: Option<u64>,
    pub(crate) model_attempts: Vec<PoolStageModelAttempt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PoolStageModelAttempt {
    pub(crate) model: String,
    pub(crate) ok: bool,
    pub(crate) reason: Option<String>,
    pub(crate) elapsed_ms: Option<u64>,
    pub(crate) answer_approx_tokens: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NewApiModelOutcome {
    model: String,
    ok: bool,
    reason: Option<String>,
    elapsed_ms: Option<u64>,
    answer_approx_tokens: Option<u64>,
    observed_unix: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NewApiModelPlan {
    models: Vec<String>,
    skipped_cooldown_models: Vec<String>,
}

impl PoolStageModelAttempt {
    fn success(model: &str, elapsed_ms: Option<u64>, answer_approx_tokens: Option<u64>) -> Self {
        Self {
            model: model.to_owned(),
            ok: true,
            reason: None,
            elapsed_ms,
            answer_approx_tokens,
        }
    }

    fn failure(model: &str, error: &str, elapsed_ms: Option<u64>) -> Self {
        Self {
            model: model.to_owned(),
            ok: false,
            reason: Some(newapi_failure_kind(error).to_owned()),
            elapsed_ms,
            answer_approx_tokens: None,
        }
    }

    fn summary(&self) -> String {
        format!(
            "model={} ok={} reason={} elapsed_ms={} answer_approx_tokens={}",
            compact_meta_value(&self.model),
            self.ok,
            self.reason.as_deref().unwrap_or("none"),
            option_u64_text(self.elapsed_ms),
            option_u64_text(self.answer_approx_tokens)
        )
    }

    fn json(&self) -> String {
        format!(
            "{{\"model\":{},\"ok\":{},\"reason\":{},\"elapsed_ms\":{},\"answer_approx_tokens\":{}}}",
            json_string(&self.model),
            self.ok,
            option_str_json(self.reason.as_deref()),
            option_u64_json(self.elapsed_ms),
            option_u64_json(self.answer_approx_tokens)
        )
    }
}

impl PoolStageCallResult {
    pub(crate) fn model_attempts_summary(&self) -> String {
        model_attempts_summary(&self.model_attempts)
    }
}

pub(crate) fn run_newapi_live_smoke(config: &Config) -> Result<(), String> {
    let report = newapi_live_smoke(
        config.timeout_secs,
        config.max_tokens,
        config.newapi_live_smoke_min_successes,
        config.newapi_live_smoke_force_all_models,
    );
    if let Some(path) = config.newapi_live_smoke_json_path.as_deref() {
        write_newapi_live_smoke_report(path, &report)?;
    }
    println!(
        "newapi_live_smoke ok={} success_count={} min_successes={} total_models={} attempts={}",
        report.ok,
        report.success_count,
        report.min_successes,
        report.total_models,
        model_attempts_summary(&report.attempts)
    );
    if report.ok {
        Ok(())
    } else {
        Err(report.failure_reason)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NewApiLiveSmokeReport {
    ok: bool,
    min_successes: usize,
    success_count: usize,
    total_models: usize,
    attempted_models: usize,
    force_all_models: bool,
    selected_order: Vec<String>,
    usable_models: Vec<String>,
    skipped_cooldown_models: Vec<String>,
    quarantined_models: Vec<String>,
    attempts: Vec<PoolStageModelAttempt>,
    failure_reason: String,
    persistence_error: Option<String>,
}

fn newapi_live_smoke(
    timeout_secs: u64,
    max_tokens: usize,
    min_successes: usize,
    force_all_models: bool,
) -> NewApiLiveSmokeReport {
    let min_successes = min_successes.max(1);
    let Some(config) = NewApiConfig::from_env("review") else {
        return NewApiLiveSmokeReport {
            ok: false,
            min_successes,
            success_count: 0,
            total_models: 0,
            attempted_models: 0,
            force_all_models,
            selected_order: Vec::new(),
            usable_models: Vec::new(),
            skipped_cooldown_models: Vec::new(),
            quarantined_models: Vec::new(),
            attempts: Vec::new(),
            failure_reason: "missing NewAPI env: set NORION_NEWAPI_BASE_URL, NORION_NEWAPI_API_KEY or NORION_NEWAPI_API_KEY_FILE, and NORION_NEWAPI_ALLOWED_MODELS; legacy NORION_MODEL_POOL_* aliases remain accepted".to_owned(),
            persistence_error: None,
        };
    };
    let input = newapi_live_smoke_input(max_tokens);
    let plan = plan_newapi_models(&config.allowed_models, force_all_models);
    let mut attempts = Vec::new();
    let mut persistence_error = None;
    for model in &plan.models {
        let started = Instant::now();
        let attempt = match call_newapi_model(&config, model, timeout_secs, &input) {
            Ok(result) => PoolStageModelAttempt::success(
                model,
                result.elapsed_ms,
                result.answer_approx_tokens,
            ),
            Err(error) => PoolStageModelAttempt::failure(
                model,
                &error,
                Some(started.elapsed().as_millis().try_into().unwrap_or(u64::MAX)),
            ),
        };
        if let Err(error) = persist_newapi_model_outcomes(std::slice::from_ref(&attempt)) {
            persistence_error.get_or_insert(error);
        }
        attempts.push(attempt);
    }
    newapi_live_smoke_report(
        min_successes,
        force_all_models,
        plan.models,
        plan.skipped_cooldown_models,
        attempts,
        persistence_error,
    )
}

fn newapi_live_smoke_input(max_tokens: usize) -> PoolStageCallInput<'static> {
    PoolStageCallInput {
        task_kind: "review",
        case_name: "newapi-live-smoke",
        round: 1,
        validation_timestamp_unix: None,
        validation_evidence: None,
        original_prompt: "Live-smoke rust-norion NewAPI model-pool fallback. Return concise review fields only.",
        primary_answer: Some(
            "runtime_model=noiron-local-transformer runtime_tokens=201 self_improve_passed=true",
        ),
        final_json: Some("{\"success\":true,\"self_improve_passed\":true}"),
        dispatch_plan: None,
        completed_roles: &[],
        max_tokens,
    }
}

fn newapi_live_smoke_report(
    min_successes: usize,
    force_all_models: bool,
    selected_order: Vec<String>,
    skipped_cooldown_models: Vec<String>,
    attempts: Vec<PoolStageModelAttempt>,
    persistence_error: Option<String>,
) -> NewApiLiveSmokeReport {
    let success_count = attempts.iter().filter(|attempt| attempt.ok).count();
    let attempted_models = attempts.len();
    let total_models = attempted_models + skipped_cooldown_models.len();
    let usable_models = attempts
        .iter()
        .filter(|attempt| attempt.ok)
        .map(|attempt| attempt.model.clone())
        .collect();
    let quarantined_models = attempts
        .iter()
        .filter(|attempt| attempt_quarantines_model(attempt))
        .map(|attempt| attempt.model.clone())
        .collect();
    let success_gate_ok = success_count >= min_successes;
    let ok = success_gate_ok && persistence_error.is_none();
    let failure_reason = if let Some(error) = &persistence_error {
        format!("NewAPI model outcome persistence failed: {error}")
    } else if success_gate_ok {
        "none".to_owned()
    } else if attempted_models == 0 && !skipped_cooldown_models.is_empty() {
        format!(
            "all NewAPI models skipped by cooldown: {}",
            skipped_cooldown_models.join(",")
        )
    } else if total_models == 0 {
        "no allowed NewAPI models configured".to_owned()
    } else {
        format!(
            "NewAPI live smoke success_count {} below required {}: {}",
            success_count,
            min_successes,
            model_attempts_summary(&attempts)
        )
    };
    NewApiLiveSmokeReport {
        ok,
        min_successes,
        success_count,
        total_models,
        attempted_models,
        force_all_models,
        selected_order,
        usable_models,
        skipped_cooldown_models,
        quarantined_models,
        attempts,
        failure_reason,
        persistence_error,
    }
}

fn write_newapi_live_smoke_report(
    path: &Path,
    report: &NewApiLiveSmokeReport,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create NewAPI live smoke report dir failed: {error}"))?;
    }
    fs::write(path, newapi_live_smoke_report_json(report))
        .map_err(|error| format!("write NewAPI live smoke report failed: {error}"))
}

fn newapi_live_smoke_report_json(report: &NewApiLiveSmokeReport) -> String {
    format!(
        "{{\"schema\":\"norion.newapi_live_smoke.v2\",\"ok\":{},\"min_successes\":{},\"success_count\":{},\"total_models\":{},\"attempted_models\":{},\"force_all_models\":{},\"failure_reason\":{},\"persistence_error\":{},\"selected_order\":{},\"usable_models\":{},\"skipped_cooldown_models\":{},\"quarantined_models\":{},\"attempts\":{}}}\n",
        report.ok,
        report.min_successes,
        report.success_count,
        report.total_models,
        report.attempted_models,
        report.force_all_models,
        json_string(&report.failure_reason),
        option_str_json(report.persistence_error.as_deref()),
        string_array_json(&report.selected_order),
        string_array_json(&report.usable_models),
        string_array_json(&report.skipped_cooldown_models),
        string_array_json(&report.quarantined_models),
        model_attempts_json(&report.attempts)
    )
}

fn plan_newapi_models(allowed_models: &[String], force_all_models: bool) -> NewApiModelPlan {
    let outcomes = read_newapi_model_outcomes(&newapi_model_outcomes_path());
    plan_newapi_models_from_outcomes(allowed_models, &outcomes, unix_now(), force_all_models)
}

fn plan_newapi_models_from_outcomes(
    allowed_models: &[String],
    outcomes: &HashMap<String, NewApiModelOutcome>,
    now_unix: u64,
    force_all_models: bool,
) -> NewApiModelPlan {
    let mut ranked = Vec::new();
    let mut skipped_cooldown_models = Vec::new();
    for (index, model) in allowed_models.iter().enumerate() {
        match outcomes.get(model) {
            Some(outcome) if !force_all_models && outcome_in_cooldown(outcome, now_unix) => {
                skipped_cooldown_models.push(model.clone());
            }
            Some(outcome) if outcome.ok => {
                ranked.push((
                    0u8,
                    outcome.elapsed_ms.unwrap_or(u64::MAX),
                    index,
                    model.clone(),
                ));
            }
            Some(outcome) => {
                ranked.push((
                    2u8,
                    outcome.elapsed_ms.unwrap_or(u64::MAX),
                    index,
                    model.clone(),
                ));
            }
            None => ranked.push((1u8, u64::MAX, index, model.clone())),
        }
    }
    ranked.sort_by_key(|item| (item.0, item.1, item.2));
    NewApiModelPlan {
        models: ranked.into_iter().map(|item| item.3).collect(),
        skipped_cooldown_models,
    }
}

fn outcome_in_cooldown(outcome: &NewApiModelOutcome, now_unix: u64) -> bool {
    if outcome.ok {
        return false;
    }
    now_unix.saturating_sub(outcome.observed_unix) < NEWAPI_MODEL_FAILURE_COOLDOWN_SECS
}

fn attempt_quarantines_model(attempt: &PoolStageModelAttempt) -> bool {
    !attempt.ok
}

fn newapi_model_outcomes_path() -> PathBuf {
    env_value([
        "NORION_NEWAPI_OUTCOMES_PATH",
        "NORION_NEWAPI_MODEL_OUTCOMES_PATH",
        "NORION_MODEL_POOL_OUTCOMES_PATH",
    ])
    .map(PathBuf::from)
    .unwrap_or_else(|| PathBuf::from(DEFAULT_NEWAPI_MODEL_OUTCOMES))
}

fn read_newapi_model_outcomes(path: &Path) -> HashMap<String, NewApiModelOutcome> {
    fs::read_to_string(path)
        .map(|text| parse_newapi_model_outcomes(&text))
        .unwrap_or_default()
}

fn parse_newapi_model_outcomes(text: &str) -> HashMap<String, NewApiModelOutcome> {
    let mut outcomes = HashMap::new();
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let Some(model) = json_string_field(line, "model") else {
            continue;
        };
        let outcome = NewApiModelOutcome {
            model: model.clone(),
            ok: json_bool_field(line, "ok").unwrap_or(false),
            reason: json_string_field(line, "reason"),
            elapsed_ms: json_u64_field(line, "elapsed_ms"),
            answer_approx_tokens: json_u64_field(line, "answer_approx_tokens"),
            observed_unix: json_u64_field(line, "observed_unix").unwrap_or(0),
        };
        outcomes.insert(model, outcome);
    }
    outcomes
}

fn persist_newapi_model_outcomes(attempts: &[PoolStageModelAttempt]) -> Result<(), String> {
    persist_newapi_model_outcomes_to(&newapi_model_outcomes_path(), attempts)
}

fn persist_newapi_model_outcomes_to(
    path: &Path,
    attempts: &[PoolStageModelAttempt],
) -> Result<(), String> {
    if attempts.is_empty() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create NewAPI model outcome dir failed: {error}"))?;
    }
    let observed_unix = unix_now();
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| format!("open NewAPI model outcome file failed: {error}"))?;
    for attempt in attempts {
        file.write_all(newapi_model_outcome_json(attempt, observed_unix).as_bytes())
            .map_err(|error| format!("write NewAPI model outcome failed: {error}"))?;
    }
    Ok(())
}

fn newapi_model_outcome_json(attempt: &PoolStageModelAttempt, observed_unix: u64) -> String {
    format!(
        "{{\"schema\":\"norion.newapi_model_outcome.v1\",\"observed_unix\":{},\"model\":{},\"ok\":{},\"reason\":{},\"elapsed_ms\":{},\"answer_approx_tokens\":{}}}\n",
        observed_unix,
        json_string(&attempt.model),
        attempt.ok,
        option_str_json(attempt.reason.as_deref()),
        option_u64_json(attempt.elapsed_ms),
        option_u64_json(attempt.answer_approx_tokens)
    )
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

pub(crate) fn model_attempts_summary(attempts: &[PoolStageModelAttempt]) -> String {
    if attempts.is_empty() {
        return "none".to_owned();
    }
    attempts
        .iter()
        .map(PoolStageModelAttempt::summary)
        .collect::<Vec<_>>()
        .join("|")
}

pub(crate) fn call_backend(
    backend: &str,
    timeout_secs: u64,
    input: &PoolStageCallInput<'_>,
) -> Result<PoolStageCallResult, String> {
    if let Some(result) = call_newapi_from_env(timeout_secs, input)? {
        return Ok(result);
    }
    call_local_backend(backend, timeout_secs, input)
}

fn call_local_backend(
    backend: &str,
    timeout_secs: u64,
    input: &PoolStageCallInput<'_>,
) -> Result<PoolStageCallResult, String> {
    let body = request_body(input);
    let response = http::post_json(backend, "/v1/model-pool/call", &body, timeout_secs)
        .map_err(|error| format!("pool stage call {} failed: {error}", input.task_kind))?;
    if !(200..300).contains(&response.status) {
        return Err(format!(
            "pool stage call {} returned HTTP {}: {}",
            input.task_kind,
            response.status,
            response.body.trim()
        ));
    }
    let mut result = parse_response(input.task_kind, &response.body);
    normalize_contract_answer(input, &mut result);
    Ok(result)
}

fn call_newapi_from_env(
    timeout_secs: u64,
    input: &PoolStageCallInput<'_>,
) -> Result<Option<PoolStageCallResult>, String> {
    let Some(config) = NewApiConfig::from_env(input.task_kind) else {
        return Ok(None);
    };
    let plan = plan_newapi_models(&config.allowed_models, false);
    let mut attempts = Vec::new();
    for model in &plan.models {
        let started = Instant::now();
        match call_newapi_model(&config, model, timeout_secs, input) {
            Ok(mut result) => {
                attempts.push(PoolStageModelAttempt::success(
                    model,
                    result.elapsed_ms,
                    result.answer_approx_tokens,
                ));
                let _ = persist_newapi_model_outcomes(&attempts);
                result.selected_model = Some(model.clone());
                result.model_attempts = attempts;
                return Ok(Some(result));
            }
            Err(error) => {
                let elapsed_ms = started.elapsed().as_millis().try_into().unwrap_or(u64::MAX);
                attempts.push(PoolStageModelAttempt::failure(
                    model,
                    &error,
                    Some(elapsed_ms),
                ));
            }
        }
    }
    let _ = persist_newapi_model_outcomes(&attempts);
    if attempts.is_empty() && !plan.skipped_cooldown_models.is_empty() {
        return Err(format!(
            "NewAPI candidate attempts skipped by cooldown for {}: {}",
            input.task_kind,
            plan.skipped_cooldown_models.join(",")
        ));
    }
    Err(format!(
        "NewAPI candidate attempts failed for {}: {}",
        input.task_kind,
        model_attempts_summary(&attempts)
    ))
}

fn call_newapi_model(
    config: &NewApiConfig,
    model: &str,
    timeout_secs: u64,
    input: &PoolStageCallInput<'_>,
) -> Result<PoolStageCallResult, String> {
    let started = Instant::now();
    let body = newapi_chat_completion_body(model, input);
    let response = http::post_json_url_bearer(
        &config.base_url,
        newapi_chat_completions_path(&config.base_url),
        &body,
        &config.api_key,
        timeout_secs,
    )?;
    if !(200..300).contains(&response.status) {
        return Err(format!("NewAPI returned HTTP {}", response.status));
    }
    let answer = newapi_answer(&response.body).ok_or_else(|| {
        format!(
            "NewAPI response for {} did not include choices[0].message.content",
            input.task_kind
        )
    })?;
    if answer.trim().is_empty() {
        return Err("NewAPI response answer is empty".to_owned());
    }
    if !newapi_answer_has_supported_contract(input, &answer) {
        return Err(format!(
            "NewAPI response for {} did not satisfy helper contract",
            input.task_kind
        ));
    }
    let elapsed_ms = started.elapsed().as_millis().try_into().unwrap_or(u64::MAX);
    let mut result = PoolStageCallResult {
        task_kind: input.task_kind.to_owned(),
        ok: true,
        selected_role: Some(newapi_selected_role(input)),
        selected_model: Some(model.to_owned()),
        selected_port: None,
        selected_base_url: Some(config.base_url.clone()),
        answer: None,
        elapsed_ms: Some(elapsed_ms),
        answer_chars: None,
        answer_bytes: None,
        answer_approx_tokens: None,
        model_attempts: Vec::new(),
    };
    set_answer(&mut result, answer);
    normalize_contract_answer(input, &mut result);
    Ok(result)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NewApiConfig {
    base_url: String,
    api_key: String,
    allowed_models: Vec<String>,
}

impl NewApiConfig {
    fn from_env(task_kind: &str) -> Option<Self> {
        let base_url = env_value(["NORION_NEWAPI_BASE_URL", "NORION_MODEL_POOL_ENDPOINT"])?;
        let api_key = api_key_from_sources(
            env_value(["NORION_NEWAPI_API_KEY", "NORION_MODEL_POOL_API_KEY"]),
            env_value(["NORION_NEWAPI_API_KEY_FILE"]).map(PathBuf::from),
        )?;
        let allowed_models =
            env_value(["NORION_NEWAPI_ALLOWED_MODELS", "NORION_MODEL_POOL_MODELS"])
                .map(|value| model_policy::sorted_allowed_models(&value, task_kind))
                .filter(|models| !models.is_empty())?;
        Some(Self {
            base_url,
            api_key,
            allowed_models,
        })
    }
}

fn api_key_from_sources(env_value: Option<String>, file_path: Option<PathBuf>) -> Option<String> {
    let value = if let Some(value) = env_value.filter(|value| !value.trim().is_empty()) {
        value
    } else {
        let path = file_path?;
        let metadata = fs::metadata(&path).ok()?;
        if !metadata.is_file() || metadata.len() > MAX_API_KEY_FILE_BYTES {
            return None;
        }
        fs::read_to_string(path).ok()?
    };
    let value = value
        .trim()
        .trim_start_matches('\u{feff}')
        .trim()
        .to_owned();
    (!value.is_empty() && !value.contains(['\r', '\n'])).then_some(value)
}

fn env_value<const N: usize>(names: [&str; N]) -> Option<String> {
    names.into_iter().find_map(|name| {
        env::var(name)
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
    })
}

fn newapi_chat_completion_body(model: &str, input: &PoolStageCallInput<'_>) -> String {
    format!(
        "{{\"model\":{},\"messages\":[{{\"role\":\"user\",\"content\":{}}}],\"max_tokens\":{},\"temperature\":0.2,\"stream\":false}}",
        json_string(model),
        json_string(&stage_prompt(input)),
        input.max_tokens.max(1)
    )
}

fn newapi_chat_completions_path(base_url: &str) -> &'static str {
    if base_url.trim_end_matches('/').ends_with("/v1") {
        "/chat/completions"
    } else {
        "/v1/chat/completions"
    }
}

fn newapi_answer(body: &str) -> Option<String> {
    let choices = json_array_field(body, "choices")?;
    let first_choice = parse_json_object_array(&choices).into_iter().next()?;
    json_object_field(&first_choice, "message")
        .and_then(|message| json_string_field(&message, "content"))
        .or_else(|| json_string_field(&first_choice, "text"))
}

fn newapi_selected_role(input: &PoolStageCallInput<'_>) -> String {
    input
        .dispatch_plan
        .map(|plan| plan.selected_role.clone())
        .unwrap_or_else(|| input.task_kind.to_owned())
}

pub(crate) fn request_body(input: &PoolStageCallInput<'_>) -> String {
    format!(
        "{{\"task_kind\":{},\"prompt\":{},\"max_tokens\":{},\"completed_roles\":{}}}",
        json_string(input.task_kind),
        json_string(&stage_prompt(input)),
        input.max_tokens.max(1),
        string_array_json(input.completed_roles)
    )
}

pub(crate) fn stage_prompt(input: &PoolStageCallInput<'_>) -> String {
    if input.task_kind == "summary" {
        return summary_stage_prompt(input);
    }
    if input.task_kind == "router" {
        return router_stage_prompt(input);
    }
    if input.task_kind == "review" {
        return review_stage_prompt(input);
    }
    if input.task_kind == "index" {
        return index_stage_prompt(input);
    }
    if input.task_kind == "test-gate" {
        return test_gate_stage_prompt(input);
    }
    format!(
        "SmartSteam evolution-loop helper stage.\ncase: {}\nstage_task_kind: {}\n{}\nrole_contract:\n{}\n{}\nprimary_prompt_preview: {}\nprimary_answer_preview: {}\nfinal_json_preview: {}\n\nOutput exactly one short bullet per role_contract field, in the same order, with the field name unchanged. Keep each field under 160 characters, cite only evidence from structured_facts and the previews, and do not repeat the full primary answer. Do not add prose before or after the bullets.",
        input.case_name,
        input.task_kind,
        structured_facts(input),
        stage_instruction(input.task_kind),
        decision_rules(input.task_kind),
        preview_text(input.original_prompt, 1200),
        input
            .primary_answer
            .map(|answer| preview_text(answer, 4000))
            .unwrap_or_else(|| "none".to_owned()),
        input
            .final_json
            .map(|json| preview_text(json, 2000))
            .unwrap_or_else(|| "none".to_owned())
    )
}

fn index_stage_prompt(input: &PoolStageCallInput<'_>) -> String {
    format!(
        "SmartSteam index helper.\nReturn only these completed lines:\n{}\n\ncase: {}\n{}\n\nEvidence previews:\nprimary_prompt: {}\nprimary_answer: {}\nfinal_json: {}\n\nRules:\n- You are not answering the user directly.\n- Do not output markdown fences, explanations, JSON blocks, or labels other than clean_gist, tags, dependency_link, source_origin, validation_timestamp, retention.\n- tags must be semicolon-separated key=value retrieval labels, not comma-separated prose.\n- tags must include role=index, case, round, primary, final_json, dependency, source_origin, and validation_timestamp labels.\n- dependency_link must name the upstream helper field or primary evidence source behind the index record.\n- source_origin must repeat the concrete upstream helper field or primary evidence source used for the index record.\n- validation_timestamp must be the exact Unix timestamp from structured_facts.\n- clean_gist must mention the smallest searchable behavior or contract fact from the evidence.\n- retention must be keep, compress, or drop with a short evidence-backed reason.\n- Keep exactly six lines and keep the field names unchanged.",
        index_field_defaults(input),
        input.case_name,
        structured_facts(input),
        preview_text(input.original_prompt, 480),
        input
            .primary_answer
            .map(|answer| preview_text(answer, 1400))
            .unwrap_or_else(|| "none".to_owned()),
        input
            .final_json
            .map(|json| preview_text(json, 900))
            .unwrap_or_else(|| "none".to_owned())
    )
}

fn index_field_defaults(input: &PoolStageCallInput<'_>) -> String {
    let primary = if input
        .primary_answer
        .map(|answer| !answer.trim().is_empty())
        .unwrap_or(false)
    {
        "present"
    } else {
        "missing"
    };
    let final_json = if input
        .final_json
        .map(|json| !json.trim().is_empty())
        .unwrap_or(false)
    {
        "present"
    } else {
        "missing"
    };
    let dependency = if input.completed_roles.iter().any(|role| role == "review") {
        "review.change_request"
    } else if input.completed_roles.iter().any(|role| role == "summary") {
        "summary.next_context"
    } else {
        "primary.evidence"
    };
    let worker = input
        .dispatch_plan
        .map(|plan| {
            format!(
                "{}@{}",
                plan.selected_role,
                option_u64_text(plan.selected_port)
            )
        })
        .unwrap_or_else(|| "index worker".to_owned());
    let validation_timestamp = option_u64_text(input.validation_timestamp_unix);
    format!(
        "clean_gist: Index round {round} {case} with {worker}; primary={primary}; final_json={final_json}; dependency={dependency}; validation_timestamp={validation_timestamp}\ntags: role=index;case={case};round={round};primary={primary};final_json={final_json};dependency={dependency};source_origin={dependency};validation_timestamp={validation_timestamp}\ndependency_link: {dependency}\nsource_origin: {dependency}\nvalidation_timestamp: {validation_timestamp}\nretention: keep; compact retrieval evidence for the next evolution round",
        case = input.case_name,
        round = input.round
    )
}

pub(crate) fn request_fingerprint(input: &PoolStageCallInput<'_>, round_window: u64) -> String {
    let prompt_hash = format!("fnv1a64:{:016x}", fnv1a64(stage_prompt(input).as_bytes()));
    let route = input
        .dispatch_plan
        .map(|plan| {
            format!(
                "{}@{}:{}",
                plan.selected_role,
                option_u64_text(plan.selected_port),
                plan.effective_max_tokens
            )
        })
        .unwrap_or_else(|| "none".to_owned());
    let material = format!(
        "task_kind={};prompt_hash={};route={};round_window={}",
        input.task_kind, prompt_hash, route, round_window
    );
    format!("fnv1a64:{:016x}", fnv1a64(material.as_bytes()))
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn review_stage_prompt(input: &PoolStageCallInput<'_>) -> String {
    format!(
        "SmartSteam review helper.\nReturn only these completed lines:\n{}\n\ncase: {}\n{}\n\nEvidence previews:\nprimary_prompt: {}\nprimary_answer: {}\nfinal_json: {}\n\nRules:\n- You are not answering the user directly.\n- Never output placeholder contract descriptions.\n- Do not output these phrases as field values: highest concrete code or behavior risk; smallest improvement to make next; one check that would prove the change.\n- Every field value must cite evidence from structured_facts, primary_answer, or final_json.\n- If evidence is weak, name the concrete limitation instead of using a placeholder.\n- Keep exactly three lines, keep the field names unchanged, and do not add prose before or after the lines.",
        review_field_contract(),
        input.case_name,
        structured_facts(input),
        preview_text(input.original_prompt, 480),
        input
            .primary_answer
            .map(|answer| preview_text(answer, 1400))
            .unwrap_or_else(|| "none".to_owned()),
        input
            .final_json
            .map(|json| preview_text(json, 900))
            .unwrap_or_else(|| "none".to_owned())
    )
}

fn review_field_contract() -> &'static str {
    "risk: concrete risk evidenced by structured_facts or previews\nchange_request: small next change grounded in the same evidence\nverification: executable command or direct log/file check that verifies the change"
}

fn router_stage_prompt(input: &PoolStageCallInput<'_>) -> String {
    let router_fields = router_field_defaults(input);
    format!(
        "SmartSteam router helper.\nReturn only these completed lines:\n{}\n\ncase: {}\n{}\n\nEvidence previews:\nprimary_prompt: {}\nprimary_answer: {}\nfinal_json: {}\n\nRules:\n- You are not answering the user directly.\n- Do not say you cannot perform the task.\n- Do not output markdown fences, explanations, JSON blocks, or labels other than route_intent, tool_call, preflight.\n- Keep exactly three lines and keep the field names unchanged.\n- Use tool_call: null unless the evidence names a concrete safe tool call.",
        router_fields,
        input.case_name,
        structured_facts(input),
        preview_text(input.original_prompt, 360),
        input
            .primary_answer
            .map(|answer| preview_text(answer, 800))
            .unwrap_or_else(|| "none".to_owned()),
        input
            .final_json
            .map(|json| preview_text(json, 360))
            .unwrap_or_else(|| "none".to_owned())
    )
}

fn router_field_defaults(input: &PoolStageCallInput<'_>) -> String {
    let final_json_present = input
        .final_json
        .map(|json| !json.trim().is_empty())
        .unwrap_or(false);
    let route_intent = if final_json_present {
        "index"
    } else {
        "review"
    };
    let worker = input
        .dispatch_plan
        .map(|plan| {
            format!(
                "{}@{}",
                plan.selected_role,
                option_u64_text(plan.selected_port)
            )
        })
        .unwrap_or_else(|| "router worker".to_owned());
    format!(
        "route_intent: {route_intent}\ntool_call: null\npreflight: allow because {worker} is selected and the stage request is read-only."
    )
}

fn normalize_contract_answer(input: &PoolStageCallInput<'_>, result: &mut PoolStageCallResult) {
    match input.task_kind {
        "router" => {
            let has_contract = result
                .answer
                .as_deref()
                .map(router_answer_has_contract)
                .unwrap_or(false);
            if !has_contract {
                set_answer(result, router_field_defaults(input));
            }
        }
        "test-gate" => {
            let has_contract = result
                .answer
                .as_deref()
                .map(|answer| test_gate_answer_has_supported_contract(input, answer))
                .unwrap_or(false);
            if !has_contract {
                set_answer(result, test_gate_field_defaults(input));
            }
        }
        "index" => {
            let has_contract = result
                .answer
                .as_deref()
                .map(index_answer_has_stable_contract)
                .unwrap_or(false);
            if !has_contract {
                set_answer(result, index_field_defaults(input));
            }
        }
        _ => {}
    }
}

fn newapi_answer_has_supported_contract(input: &PoolStageCallInput<'_>, answer: &str) -> bool {
    match input.task_kind {
        "summary" => summary_answer_has_contract(answer),
        "review" => review_answer_has_contract(answer),
        "router" => router_answer_has_contract(answer),
        "test-gate" => test_gate_answer_has_supported_contract(input, answer),
        "index" => index_answer_has_stable_contract(answer),
        _ => true,
    }
}

fn summary_answer_has_contract(answer: &str) -> bool {
    field_has_value(answer, "memory_update")
        && field_has_value(answer, "next_context")
        && field_has_value(answer, "duplicate_guard")
}

fn review_answer_has_contract(answer: &str) -> bool {
    field_has_value(answer, "risk")
        && field_has_value(answer, "change_request")
        && field_has_value(answer, "verification")
        && !review_answer_uses_placeholder_contract(answer)
}

fn field_has_value(answer: &str, field: &str) -> bool {
    extract_field_value(answer, field)
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

fn review_answer_uses_placeholder_contract(answer: &str) -> bool {
    let lower = answer.to_ascii_lowercase();
    lower.contains("highest concrete code or behavior risk")
        || lower.contains("smallest improvement to make next")
        || lower.contains("one check that would prove the change")
}

fn router_answer_has_contract(answer: &str) -> bool {
    let lower = answer.to_ascii_lowercase();
    lower.contains("route_intent") && lower.contains("tool_call") && lower.contains("preflight")
}

fn test_gate_answer_has_supported_contract(input: &PoolStageCallInput<'_>, answer: &str) -> bool {
    let lower = answer.to_ascii_lowercase();
    if !(lower.contains("verdict")
        && lower.contains("validation_command")
        && lower.contains("failure_kind"))
    {
        return false;
    }
    let Some(command) = extract_field_value(answer, "validation_command") else {
        return false;
    };
    if validation::test_gate_validation_command_safety(Some(&command)) != "safe" {
        return false;
    }
    let verdict = extract_field_value(answer, "verdict")
        .and_then(|value| normalized_test_gate_verdict(&value));
    let Some(verdict) = verdict else {
        return false;
    };
    let failure_kind = extract_field_value(answer, "failure_kind")
        .unwrap_or_else(|| "missing_evidence".to_owned())
        .to_ascii_lowercase();
    if test_gate_validation_evidence_supports_pass(input) {
        !(verdict != "pass" && failure_kind == "missing_evidence")
    } else {
        verdict != "pass"
    }
}

fn normalized_test_gate_verdict(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "pass" => Some("pass"),
        "warn" => Some("warn"),
        "fail" => Some("fail"),
        _ => None,
    }
}

fn index_answer_has_stable_contract(answer: &str) -> bool {
    let lower = answer.to_ascii_lowercase();
    if !(lower.contains("clean_gist")
        && lower.contains("tags")
        && lower.contains("dependency_link")
        && lower.contains("source_origin")
        && lower.contains("validation_timestamp")
        && lower.contains("retention"))
    {
        return false;
    }
    let Some(dependency_link) = extract_field_value(answer, "dependency_link") else {
        return false;
    };
    if dependency_link.trim().eq_ignore_ascii_case("none") {
        return false;
    }
    let Some(source_origin) = extract_field_value(answer, "source_origin") else {
        return false;
    };
    if source_origin.trim().eq_ignore_ascii_case("none") {
        return false;
    }
    let Some(validation_timestamp) = extract_field_value(answer, "validation_timestamp") else {
        return false;
    };
    if !is_stable_unix_timestamp(&validation_timestamp) {
        return false;
    }
    let Some(tags) = extract_field_value(answer, "tags") else {
        return false;
    };
    if !index_tags_are_stable(&tags) {
        return false;
    }
    let dependency_matches = index_tag_value(&tags, "dependency")
        .map(|dependency| dependency == dependency_link.trim())
        .unwrap_or(false);
    let timestamp_matches = index_tag_value(&tags, "validation_timestamp")
        .map(|timestamp| timestamp == validation_timestamp.trim())
        .unwrap_or(false);
    let source_origin_matches = index_tag_value(&tags, "source_origin")
        .map(|origin| origin == source_origin.trim())
        .unwrap_or(false);
    dependency_matches && source_origin_matches && timestamp_matches
}

fn index_tags_are_stable(tags: &str) -> bool {
    let labels = tags
        .split(';')
        .map(str::trim)
        .filter(|label| !label.is_empty())
        .collect::<Vec<_>>();
    if labels.len() < 5 {
        return false;
    }
    let mut keys = labels
        .iter()
        .filter_map(|label| label.split_once('='))
        .map(|(key, value)| (key.trim().to_ascii_lowercase(), value.trim()))
        .filter(|(_, value)| !value.is_empty())
        .collect::<Vec<_>>();
    if keys.len() != labels.len() {
        return false;
    }
    keys.sort_by(|left, right| left.0.cmp(&right.0));
    keys.iter()
        .any(|(key, value)| key == "role" && *value == "index")
        && keys.iter().any(|(key, _)| key == "case")
        && keys.iter().any(|(key, _)| key == "round")
        && keys.iter().any(|(key, _)| key == "primary")
        && keys.iter().any(|(key, _)| key == "final_json")
        && keys.iter().any(|(key, _)| key == "dependency")
        && keys.iter().any(|(key, _)| key == "source_origin")
        && keys.iter().any(|(key, _)| key == "validation_timestamp")
}

fn is_stable_unix_timestamp(value: &str) -> bool {
    value.chars().all(|character| character.is_ascii_digit()) && value.len() >= 10
}

fn index_tag_value<'a>(tags: &'a str, target_key: &str) -> Option<&'a str> {
    tags.split(';')
        .map(str::trim)
        .filter_map(|label| label.split_once('='))
        .find_map(|(key, value)| {
            (key.trim().eq_ignore_ascii_case(target_key) && !value.trim().is_empty())
                .then_some(value.trim())
        })
}

fn extract_field_value(text: &str, field: &str) -> Option<String> {
    let field = field.to_ascii_lowercase();
    for line in text.lines() {
        for segment in line.split(" / ") {
            let candidate = trim_contract_bullet(segment);
            let lower = candidate.to_ascii_lowercase();
            if !lower.starts_with(&field) {
                continue;
            }
            let Some(after_field) = candidate.get(field.len()..) else {
                continue;
            };
            let after_separator = after_field.trim_start();
            let Some(value_body) = after_separator
                .strip_prefix(':')
                .or_else(|| after_separator.strip_prefix('='))
                .or_else(|| after_separator.strip_prefix('-'))
            else {
                continue;
            };
            let value = value_body
                .split(" ; ")
                .next()
                .unwrap_or_default()
                .trim()
                .trim_matches(|character| matches!(character, '"' | '\''));
            if !value.is_empty() && !value.eq_ignore_ascii_case("none") {
                return Some(value.to_owned());
            }
        }
    }
    None
}

fn trim_contract_bullet(text: &str) -> &str {
    text.trim()
        .strip_prefix("- ")
        .or_else(|| text.trim().strip_prefix("* "))
        .unwrap_or_else(|| text.trim())
}

fn set_answer(result: &mut PoolStageCallResult, answer: String) {
    result.answer_chars = Some(answer.chars().count() as u64);
    result.answer_bytes = Some(answer.len() as u64);
    result.answer_approx_tokens = Some(answer.chars().count().div_ceil(4) as u64);
    result.answer = Some(answer);
}

fn summary_stage_prompt(input: &PoolStageCallInput<'_>) -> String {
    let summary_fields = summary_field_defaults(input);
    format!(
        "SmartSteam summary helper.\nReturn only these completed lines:\n{}\n\ncase: {}\n{}\n\nEvidence previews:\nprimary_prompt: {}\nprimary_answer: {}\nfinal_json: {}\n\nRules:\n- You are not writing code.\n- Do not output markdown fences, functions, JSON, explanations, or labels other than memory_update, next_context, duplicate_guard.\n- You may make the values more specific using the evidence, but keep exactly three lines.\n- Never output placeholders.",
        summary_fields,
        input.case_name,
        structured_facts(input),
        preview_text(input.original_prompt, 360),
        input
            .primary_answer
            .map(|answer| preview_text(answer, 800))
            .unwrap_or_else(|| "none".to_owned()),
        input
            .final_json
            .map(|json| preview_text(json, 360))
            .unwrap_or_else(|| "none".to_owned())
    )
}

fn summary_field_defaults(input: &PoolStageCallInput<'_>) -> String {
    let worker = input
        .dispatch_plan
        .map(|plan| {
            format!(
                "{}@{}",
                plan.selected_role,
                option_u64_text(plan.selected_port)
            )
        })
        .unwrap_or_else(|| "summary worker".to_owned());
    let runtime = input
        .dispatch_plan
        .and_then(|plan| {
            let device = plan.runtime_device.as_deref()?;
            let accelerator = plan.runtime_accelerator.as_deref().unwrap_or("none");
            Some(format!("{device}/{accelerator}"))
        })
        .unwrap_or_else(|| "reported runtime".to_owned());
    format!(
        "memory_update: Keep {worker} on {runtime} for short summary memory updates.\nnext_context: Preserve model-pool stage evidence before the next evolution round.\nduplicate_guard: Do not emit code, markdown fences, placeholders, or route summary work to the 12B worker."
    )
}

fn structured_facts(input: &PoolStageCallInput<'_>) -> String {
    let final_json_present = input
        .final_json
        .map(|json| !json.trim().is_empty())
        .unwrap_or(false);
    let primary_answer_present = input
        .primary_answer
        .map(|answer| !answer.trim().is_empty())
        .unwrap_or(false);
    let mut facts = vec![
        "structured_facts:".to_owned(),
        format!("- task_kind: {}", input.task_kind),
        format!("- round: {}", input.round),
        format!(
            "- validation_timestamp: {}",
            option_u64_text(input.validation_timestamp_unix)
        ),
        format!("- primary_answer_present: {}", primary_answer_present),
        format!("- final_json_present: {}", final_json_present),
        format!("- requested_max_tokens: {}", input.max_tokens.max(1)),
    ];
    if let Some(evidence) = input.validation_evidence {
        facts.push("- validation_gate_checked: true".to_owned());
        facts.push(format!(
            "- validation_gate_passed: {}",
            evidence.status_code == Some(0)
        ));
        facts.push(format!("- validation_gate_phase: {}", evidence.phase));
        facts.push(format!(
            "- validation_command_source: {}",
            evidence.command_source
        ));
        facts.push(format!(
            "- validation_command_safety: {}",
            evidence.command_safety
        ));
        facts.push(format!(
            "- validation_command_safe_for_test_gate: {}",
            validation::test_gate_validation_command_safety(Some(evidence.command_preview))
        ));
        facts.push(format!(
            "- validation_command: {}",
            evidence.command_preview
        ));
        facts.push(format!(
            "- validation_status_code: {}",
            option_i32_text(evidence.status_code)
        ));
        facts.push(format!("- validation_elapsed_ms: {}", evidence.elapsed_ms));
        facts.push(format!(
            "- validation_stdout_tail: {}",
            dash_if_empty(evidence.stdout_tail)
        ));
        facts.push(format!(
            "- validation_stderr_tail: {}",
            dash_if_empty(evidence.stderr_tail)
        ));
    } else {
        facts.push("- validation_gate_checked: false".to_owned());
        facts.push("- validation_gate_passed: false".to_owned());
        facts.push("- validation_command_source: none".to_owned());
        facts.push("- validation_command_safe_for_test_gate: missing".to_owned());
        facts.push("- validation_status_code: none".to_owned());
    }
    if let Some(plan) = input.dispatch_plan {
        facts.push(format!("- selected_role: {}", plan.selected_role));
        facts.push(format!(
            "- selected_port: {}",
            option_u64_text(plan.selected_port)
        ));
        facts.push(format!(
            "- selected_base_url: {}",
            plan.selected_base_url.as_deref().unwrap_or("none")
        ));
        facts.push(format!(
            "- context_window: {}",
            option_u64_text(plan.context_window)
        ));
        facts.push(format!(
            "- default_max_tokens: {}",
            option_u64_text(plan.default_max_tokens)
        ));
        facts.push(format!(
            "- runtime_backend: {}",
            plan.runtime_backend.as_deref().unwrap_or("none")
        ));
        facts.push(format!(
            "- runtime_device: {}",
            plan.runtime_device.as_deref().unwrap_or("none")
        ));
        facts.push(format!(
            "- runtime_accelerator: {}",
            plan.runtime_accelerator.as_deref().unwrap_or("none")
        ));
        facts.push(format!(
            "- gpu_layers: {}",
            option_u64_text(plan.gpu_layers)
        ));
        facts.push(format!(
            "- configured_max_tokens: {}",
            plan.configured_max_tokens
        ));
        facts.push(format!(
            "- effective_max_tokens: {}",
            plan.effective_max_tokens
        ));
        facts.push(format!("- max_tokens_clamped: {}", plan.max_tokens_clamped));
        facts.push(format!(
            "- can_accept_low_priority_task: {}",
            plan.can_accept_low_priority_task
        ));
    } else {
        facts.push("- selected_role: unknown".to_owned());
    }
    facts.join("\n")
}

fn decision_rules(task_kind: &str) -> &'static str {
    match task_kind {
        "test-gate" => {
            "decision_rules:\n- verdict must be exactly pass, warn, or fail.\n- use pass only when structured_facts say validation_gate_checked=true, validation_gate_passed=true, validation_command_safe_for_test_gate=safe, and validation_status_code=0.\n- validation_command must copy the safe validation_command from structured_facts when present; otherwise prefer cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --no-fail-fast.\n- use failure_kind: none when verdict is pass; otherwise use a short category such as missing_evidence, unsafe_command, or validation_risk."
        }
        "summary" => {
            "decision_rules:\n- summarize only durable facts that should improve the next round.\n- do not ask the primary 12B model to repeat work already captured in duplicate_guard."
        }
        "router" => {
            "decision_rules:\n- route_intent must be exactly summary, router, review, test-gate, index, quality, or none.\n- tool_call must be a compact JSON object or null; do not invent unavailable tools.\n- preflight must be allow or block with one short reason."
        }
        "review" => {
            "decision_rules:\n- name one concrete risk and one small change request from the evidence.\n- verification should be executable or directly inspectable."
        }
        "index" => {
            "decision_rules:\n- clean_gist should be searchable and compact.\n- tags must be semicolon-separated key=value retrieval labels, not comma-separated prose.\n- dependency_link must point to the upstream helper field or primary evidence source.\n- source_origin must repeat the concrete source used for dependency_link and tags.\n- validation_timestamp must copy structured_facts validation_timestamp exactly.\n- retention must choose keep, compress, or drop."
        }
        _ => {
            "decision_rules:\n- keep the answer evidence-backed and actionable.\n- verification should be specific."
        }
    }
}

fn stage_instruction(task_kind: &str) -> &'static str {
    match task_kind {
        "summary" => {
            "- memory_update: one reusable lesson from this round\n- next_context: one fact the next prompt should remember\n- duplicate_guard: one thing not to repeat"
        }
        "router" => {
            "- route_intent: summary, router, review, test-gate, index, quality, or none\n- tool_call: compact JSON object or null\n- preflight: allow or block with one short reason"
        }
        "review" => review_field_contract(),
        "test-gate" => {
            "- verdict: pass, warn, or fail\n- validation_command: one safe local cargo command to run\n- failure_kind: concise category if verdict is not pass; use none when verdict is pass"
        }
        "index" => {
            "- clean_gist: compact searchable summary\n- tags: role=index;case=<case>;round=<round>;primary=<present|missing>;final_json=<present|missing>;dependency=<source>;source_origin=<source>;validation_timestamp=<unix>\n- dependency_link: upstream helper field or primary evidence source\n- source_origin: same upstream helper field or primary evidence source\n- validation_timestamp: Unix timestamp copied from structured_facts\n- retention: keep, compress, or drop with reason"
        }
        _ => {
            "- observation: one evidence-backed observation\n- next_action: one small next step\n- verification: one way to check it"
        }
    }
}

fn test_gate_stage_prompt(input: &PoolStageCallInput<'_>) -> String {
    format!(
        "SmartSteam test-gate helper.\nReturn only these completed lines:\n{}\n\ncase: {}\n{}\n\nEvidence previews:\nprimary_prompt: {}\nprimary_answer: {}\nfinal_json: {}\n\nRules:\n- You are not answering the user directly.\n- Do not output markdown fences, JSON blocks, explanations, or labels other than verdict, validation_command, failure_kind.\n- If structured_facts show a safe validation command already ran and validation_status_code is 0, verdict must be pass and failure_kind must be none.\n- If validation evidence is missing, verdict must be warn and failure_kind must be missing_evidence.\n- Keep exactly three lines and keep the field names unchanged.",
        test_gate_field_defaults(input),
        input.case_name,
        structured_facts(input),
        preview_text(input.original_prompt, 480),
        input
            .primary_answer
            .map(|answer| preview_text(answer, 1200))
            .unwrap_or_else(|| "none".to_owned()),
        input
            .final_json
            .map(|json| preview_text(json, 900))
            .unwrap_or_else(|| "none".to_owned())
    )
}

fn test_gate_field_defaults(input: &PoolStageCallInput<'_>) -> String {
    let command = test_gate_safe_validation_command(input);
    if test_gate_validation_evidence_supports_pass(input) {
        return format!("verdict: pass\nvalidation_command: {command}\nfailure_kind: none");
    }
    let failure_kind = input
        .validation_evidence
        .map(|evidence| {
            if validation::test_gate_validation_command_safety(Some(evidence.command_preview))
                != "safe"
            {
                "unsafe_command"
            } else {
                "validation_risk"
            }
        })
        .unwrap_or("missing_evidence");
    format!("verdict: warn\nvalidation_command: {command}\nfailure_kind: {failure_kind}")
}

fn test_gate_safe_validation_command(input: &PoolStageCallInput<'_>) -> String {
    input
        .validation_evidence
        .and_then(|evidence| {
            (validation::test_gate_validation_command_safety(Some(evidence.command_preview))
                == "safe")
                .then_some(evidence.command_preview.trim())
        })
        .filter(|command| !command.is_empty())
        .unwrap_or(DEFAULT_TEST_GATE_VALIDATION_COMMAND)
        .to_owned()
}

fn test_gate_validation_evidence_supports_pass(input: &PoolStageCallInput<'_>) -> bool {
    input.validation_evidence.is_some_and(|evidence| {
        evidence.status_code == Some(0)
            && validation::test_gate_validation_command_safety(Some(evidence.command_preview))
                == "safe"
    })
}

fn option_i32_text(value: Option<i32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_owned())
}

fn dash_if_empty(value: &str) -> &str {
    if value.trim().is_empty() {
        "-"
    } else {
        value.trim()
    }
}

fn option_u64_text(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_owned())
}

fn option_u64_json(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

fn option_str_json(value: Option<&str>) -> String {
    value.map(json_string).unwrap_or_else(|| "null".to_owned())
}

fn compact_meta_value(value: &str) -> String {
    value
        .trim()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric()
                || matches!(character, '-' | '_' | '.' | '/' | ':' | '@')
            {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn newapi_failure_kind(error: &str) -> &'static str {
    let lower = error.to_ascii_lowercase();
    if lower.contains("http 401") || lower.contains("http 403") {
        return "auth";
    }
    if lower.contains("timeout")
        || lower.contains("timed out")
        || lower.contains("operation timed out")
    {
        return "timeout";
    }
    if lower.contains("empty") {
        return "empty_answer";
    }
    if lower.contains("choices[0]") || lower.contains("message.content") {
        return "response_shape";
    }
    if lower.contains("did not satisfy helper contract") {
        return "model_error";
    }
    if lower.contains("http 400")
        || lower.contains("http 404")
        || lower.contains("http 409")
        || lower.contains("http 422")
        || lower.contains("http 429")
        || lower.contains("http 500")
        || lower.contains("http 502")
        || lower.contains("http 503")
        || lower.contains("http 504")
    {
        return "model_error";
    }
    "transport"
}

pub(crate) fn model_attempts_json(attempts: &[PoolStageModelAttempt]) -> String {
    let items = attempts
        .iter()
        .map(PoolStageModelAttempt::json)
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

pub(crate) fn parse_model_attempts(body: &str) -> Vec<PoolStageModelAttempt> {
    let Some(array) = json_array_field(body, "model_attempts") else {
        return Vec::new();
    };
    parse_json_object_array(&array)
        .into_iter()
        .filter_map(|object| {
            let model = json_string_field(&object, "model")?;
            Some(PoolStageModelAttempt {
                model,
                ok: json_bool_field(&object, "ok").unwrap_or(false),
                reason: json_string_field(&object, "reason"),
                elapsed_ms: json_u64_field(&object, "elapsed_ms"),
                answer_approx_tokens: json_u64_field(&object, "answer_approx_tokens"),
            })
        })
        .collect()
}

fn string_array_json(values: &[String]) -> String {
    let items = values
        .iter()
        .map(|value| json_string(value))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

pub(crate) fn parse_response(task_kind: &str, body: &str) -> PoolStageCallResult {
    PoolStageCallResult {
        task_kind: json_string_field(body, "task_kind").unwrap_or_else(|| task_kind.to_owned()),
        ok: json_bool_field(body, "ok").unwrap_or(false),
        selected_role: json_string_field(body, "selected_role"),
        selected_model: json_string_field(body, "selected_model"),
        selected_port: json_u64_field(body, "selected_port"),
        selected_base_url: json_string_field(body, "selected_base_url"),
        answer: json_string_field(body, "answer"),
        elapsed_ms: json_u64_field(body, "elapsed_ms"),
        answer_chars: json_u64_field(body, "answer_chars"),
        answer_bytes: json_u64_field(body, "answer_bytes"),
        answer_approx_tokens: json_u64_field(body, "answer_approx_tokens"),
        model_attempts: parse_model_attempts(body),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> PoolStageCallInput<'static> {
        PoolStageCallInput {
            task_kind: "review",
            case_name: "case-1",
            round: 1,
            validation_timestamp_unix: Some(1_781_770_000),
            validation_evidence: None,
            original_prompt: "Improve the Forge UI",
            primary_answer: Some("Changed a Rust module"),
            final_json: Some("{\"ok\":true}"),
            dispatch_plan: None,
            completed_roles: &[],
            max_tokens: 256,
        }
    }

    fn test_gate_plan() -> PoolStageDispatchPlan {
        PoolStageDispatchPlan {
            task_kind: "test-gate".to_owned(),
            selected_role: "test-gate".to_owned(),
            selected_port: Some(8688),
            selected_base_url: Some("http://127.0.0.1:8688".to_owned()),
            context_window: Some(4096),
            default_max_tokens: Some(768),
            runtime_backend: Some("llama.cpp".to_owned()),
            runtime_device: Some("cpu".to_owned()),
            runtime_accelerator: Some("accelerate".to_owned()),
            gpu_layers: Some(0),
            configured_max_tokens: 262_144,
            effective_max_tokens: 768,
            max_tokens_clamped: true,
            can_accept_low_priority_task: true,
        }
    }

    fn index_plan() -> PoolStageDispatchPlan {
        PoolStageDispatchPlan {
            task_kind: "index".to_owned(),
            selected_role: "index".to_owned(),
            selected_port: Some(8690),
            selected_base_url: Some("http://127.0.0.1:8690".to_owned()),
            context_window: Some(4096),
            default_max_tokens: Some(512),
            runtime_backend: Some("llama.cpp".to_owned()),
            runtime_device: Some("cpu".to_owned()),
            runtime_accelerator: Some("accelerate".to_owned()),
            gpu_layers: Some(0),
            configured_max_tokens: 4096,
            effective_max_tokens: 512,
            max_tokens_clamped: true,
            can_accept_low_priority_task: true,
        }
    }

    fn passed_validation_evidence() -> PoolStageValidationEvidence<'static> {
        PoolStageValidationEvidence {
            phase: "pre",
            command_source: "configured",
            command_safety: "explicit",
            command_preview: "cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --target-dir target\\evolution-loop-daemon-check",
            status_code: Some(0),
            elapsed_ms: 7127,
            stdout_tail: "test result: ok. 349 passed; 0 failed",
            stderr_tail: "",
        }
    }

    #[test]
    fn request_body_targets_model_pool_call_contract() {
        let body = request_body(&input());

        assert!(body.contains("\"task_kind\":\"review\""));
        assert!(body.contains("\"max_tokens\":256"));
        assert!(body.contains("\"completed_roles\":[]"));
        assert!(body.contains("SmartSteam review helper"));
        assert!(body.contains("structured_facts"));
        assert!(body.contains("Return only these completed lines"));
        assert!(body.contains("primary_answer"));
        assert!(body.contains("change_request"));
        assert!(body.contains("verification"));
        assert!(!body.contains("role_contract"));
    }

    #[test]
    fn request_fingerprint_is_stable_and_window_scoped() {
        let plan = test_gate_plan();
        let input = PoolStageCallInput {
            task_kind: "test-gate",
            dispatch_plan: Some(&plan),
            max_tokens: plan.effective_max_tokens,
            ..input()
        };

        let first = request_fingerprint(&input, 22);
        let second = request_fingerprint(&input, 22);
        let other_window = request_fingerprint(&input, 23);

        assert_eq!(first, second);
        assert_ne!(first, other_window);
        assert!(first.starts_with("fnv1a64:"));
    }

    #[test]
    fn newapi_body_uses_chat_completions_contract() {
        let body = newapi_chat_completion_body("qwen/qwen3-next-80b-a3b-instruct", &input());

        assert!(body.contains("\"model\":\"qwen/qwen3-next-80b-a3b-instruct\""));
        assert!(body.contains("\"messages\":[{\"role\":\"user\""));
        assert!(body.contains("\"max_tokens\":256"));
        assert!(body.contains("\"stream\":false"));
        assert!(body.contains("SmartSteam review helper"));
        assert!(!body.contains("NORION_NEWAPI_API_KEY"));
    }

    #[test]
    fn newapi_contract_rejects_guardrail_summary_refusal() {
        let mut summary = input();
        summary.task_kind = "summary";

        assert!(!newapi_answer_has_supported_contract(
            &summary,
            "unsafe / S14"
        ));
        assert!(newapi_answer_has_supported_contract(
            &summary,
            "memory_update: keep NewAPI summary fallback\nnext_context: preserve helper evidence\nduplicate_guard: avoid repeated unsafe summary refusals"
        ));
    }

    #[test]
    fn newapi_contract_requires_review_fields_without_placeholders() {
        assert!(!newapi_answer_has_supported_contract(
            &input(),
            "risk: highest concrete code or behavior risk\nchange_request: smallest improvement to make next\nverification: one check that would prove the change"
        ));
        assert!(newapi_answer_has_supported_contract(
            &input(),
            "risk: primary route fallback can skip review helper evidence\nchange_request: use NewAPI fallback when the primary route is blocked\nverification: cargo test --locked --manifest-path tools/evolution-loop/Cargo.toml pool_stage"
        ));
    }

    #[test]
    fn newapi_path_avoids_double_v1_prefix() {
        assert_eq!(
            newapi_chat_completions_path("http://127.0.0.1:3000/v1"),
            "/chat/completions"
        );
        assert_eq!(
            newapi_chat_completions_path("http://127.0.0.1:3000"),
            "/v1/chat/completions"
        );
    }

    #[test]
    fn parses_newapi_chat_completion_answer() {
        let body = r#"{"choices":[{"message":{"role":"assistant","content":"risk: ok\nchange_request: keep\nverification: cargo test"}}],"usage":{"completion_tokens":17}}"#;

        assert_eq!(
            newapi_answer(body).as_deref(),
            Some("risk: ok\nchange_request: keep\nverification: cargo test")
        );
    }

    #[test]
    fn newapi_failure_kind_keeps_secret_safe_categories() {
        assert_eq!(newapi_failure_kind("NewAPI returned HTTP 401"), "auth");
        assert_eq!(newapi_failure_kind("operation timed out"), "timeout");
        assert_eq!(
            newapi_failure_kind("NewAPI response answer is empty"),
            "empty_answer"
        );
        assert_eq!(
            newapi_failure_kind("NewAPI response did not include choices[0].message.content"),
            "response_shape"
        );
        assert_eq!(
            newapi_failure_kind("NewAPI response for summary did not satisfy helper contract"),
            "model_error"
        );
        assert_eq!(
            newapi_failure_kind("NewAPI returned HTTP 429"),
            "model_error"
        );
        assert_eq!(newapi_failure_kind("connect failed"), "transport");
    }

    #[test]
    fn model_attempt_summary_records_fallback_without_secret() {
        let attempts = vec![
            PoolStageModelAttempt::failure("bad model", "NewAPI returned HTTP 401", Some(3)),
            PoolStageModelAttempt::success("qwen/qwen3-next-80b-a3b-instruct", Some(44), Some(17)),
        ];

        let summary = model_attempts_summary(&attempts);

        assert!(summary.contains("model=bad_model ok=false reason=auth elapsed_ms=3"));
        assert!(summary.contains(
            "model=qwen/qwen3-next-80b-a3b-instruct ok=true reason=none elapsed_ms=44 answer_approx_tokens=17"
        ));
        assert!(!summary.contains("Bearer"));
        assert!(!summary.contains("API_KEY"));
    }

    #[test]
    fn parses_model_attempts_from_response_json() {
        let body = r#"{"model_attempts":[{"model":"first","ok":false,"reason":"auth","elapsed_ms":3,"answer_approx_tokens":null},{"model":"second","ok":true,"reason":null,"elapsed_ms":44,"answer_approx_tokens":17}]}"#;

        let attempts = parse_model_attempts(body);

        assert_eq!(attempts.len(), 2);
        assert_eq!(attempts[0].model, "first");
        assert!(!attempts[0].ok);
        assert_eq!(attempts[0].reason.as_deref(), Some("auth"));
        assert_eq!(attempts[1].model, "second");
        assert!(attempts[1].ok);
        assert_eq!(attempts[1].answer_approx_tokens, Some(17));
    }

    #[test]
    fn newapi_outcomes_rank_successes_and_cool_down_failures() {
        let now = 1_800_000_000;
        let text = [
            format!(
                "{{\"observed_unix\":{},\"model\":\"slow\",\"ok\":true,\"reason\":null,\"elapsed_ms\":500,\"answer_approx_tokens\":20}}",
                now - 10
            ),
            format!(
                "{{\"observed_unix\":{},\"model\":\"fast\",\"ok\":true,\"reason\":null,\"elapsed_ms\":50,\"answer_approx_tokens\":15}}",
                now - 9
            ),
            format!(
                "{{\"observed_unix\":{},\"model\":\"timeout\",\"ok\":false,\"reason\":\"timeout\",\"elapsed_ms\":60000,\"answer_approx_tokens\":null}}",
                now - 8
            ),
        ]
        .join("\n");
        let outcomes = parse_newapi_model_outcomes(&text);
        let allowed_models = vec![
            "timeout".to_owned(),
            "slow".to_owned(),
            "unknown".to_owned(),
            "fast".to_owned(),
        ];

        let plan = plan_newapi_models_from_outcomes(&allowed_models, &outcomes, now, false);

        assert_eq!(
            plan.models,
            vec!["fast".to_owned(), "slow".to_owned(), "unknown".to_owned()]
        );
        assert_eq!(plan.skipped_cooldown_models, vec!["timeout".to_owned()]);
    }

    #[test]
    fn newapi_outcomes_force_all_retests_cooldown_failures() {
        let now = 1_800_000_000;
        let text = format!(
            "{{\"observed_unix\":{},\"model\":\"timeout\",\"ok\":false,\"reason\":\"timeout\",\"elapsed_ms\":60000,\"answer_approx_tokens\":null}}",
            now - 8
        );
        let outcomes = parse_newapi_model_outcomes(&text);
        let allowed_models = vec!["timeout".to_owned(), "unknown".to_owned()];

        let plan = plan_newapi_models_from_outcomes(&allowed_models, &outcomes, now, true);

        assert_eq!(
            plan.models,
            vec!["unknown".to_owned(), "timeout".to_owned()]
        );
        assert!(plan.skipped_cooldown_models.is_empty());
    }

    #[test]
    fn newapi_api_key_file_is_bounded_and_env_wins() {
        let path = std::env::temp_dir().join(format!(
            "norion-evolution-loop-key-{}-{}",
            std::process::id(),
            unix_now()
        ));
        fs::write(&path, "\u{feff} file-secret\r\n").unwrap();

        assert_eq!(
            api_key_from_sources(None, Some(path.clone())).as_deref(),
            Some("file-secret")
        );
        assert_eq!(
            api_key_from_sources(Some("env-secret".to_owned()), Some(path.clone())).as_deref(),
            Some("env-secret")
        );

        fs::write(&path, vec![b'x'; MAX_API_KEY_FILE_BYTES as usize + 1]).unwrap();
        assert!(api_key_from_sources(None, Some(path.clone())).is_none());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn newapi_outcomes_append_each_completed_attempt() {
        let path = std::env::temp_dir().join(format!(
            "norion-evolution-loop-outcomes-{}-{}.jsonl",
            std::process::id(),
            unix_now()
        ));
        persist_newapi_model_outcomes_to(
            &path,
            &[PoolStageModelAttempt::success("first", Some(10), Some(4))],
        )
        .unwrap();
        persist_newapi_model_outcomes_to(
            &path,
            &[PoolStageModelAttempt::failure(
                "second",
                "timeout",
                Some(20),
            )],
        )
        .unwrap();

        let text = fs::read_to_string(&path).unwrap();
        assert_eq!(text.lines().count(), 2);
        let outcomes = parse_newapi_model_outcomes(&text);
        assert!(outcomes.get("first").unwrap().ok);
        assert!(!outcomes.get("second").unwrap().ok);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn newapi_live_smoke_report_requires_min_successes() {
        let report = newapi_live_smoke_report(
            2,
            true,
            vec!["first".to_owned(), "second".to_owned()],
            Vec::new(),
            vec![
                PoolStageModelAttempt::failure("first", "NewAPI returned HTTP 401", Some(3)),
                PoolStageModelAttempt::success("second", Some(44), Some(17)),
            ],
            None,
        );

        assert!(!report.ok);
        assert_eq!(report.success_count, 1);
        assert!(report.failure_reason.contains("below required 2"));
        let json = newapi_live_smoke_report_json(&report);
        assert!(json.contains("\"schema\":\"norion.newapi_live_smoke.v2\""));
        assert!(json.contains("\"success_count\":1"));
        assert!(json.contains("\"force_all_models\":true"));
        assert!(json.contains("\"selected_order\":[\"first\",\"second\"]"));
        assert!(json.contains("\"usable_models\":[\"second\"]"));
        assert!(json.contains("\"quarantined_models\":[\"first\"]"));
        assert!(json.contains("\"reason\":\"auth\""));
        assert!(!json.contains("authorization"));
    }

    #[test]
    fn newapi_live_smoke_report_passes_two_models() {
        let report = newapi_live_smoke_report(
            2,
            false,
            vec!["first".to_owned(), "second".to_owned()],
            Vec::new(),
            vec![
                PoolStageModelAttempt::success("first", Some(10), Some(4)),
                PoolStageModelAttempt::success("second", Some(12), Some(5)),
            ],
            None,
        );

        assert!(report.ok);
        assert_eq!(report.failure_reason, "none");
    }

    #[test]
    fn newapi_selected_role_preserves_dispatch_role() {
        let plan = index_plan();
        let input = PoolStageCallInput {
            dispatch_plan: Some(&plan),
            ..input()
        };

        assert_eq!(newapi_selected_role(&input), "index");
    }

    #[test]
    fn stage_prompt_requires_exact_bulleted_contract_output() {
        let mut index = input();
        index.task_kind = "index";
        let prompt = stage_prompt(&index);

        assert!(prompt.contains("SmartSteam index helper"));
        assert!(prompt.contains("Return only these completed lines"));
        assert!(prompt.contains("keep the field names unchanged"));
        assert!(prompt.contains("tags must be semicolon-separated key=value retrieval labels"));
        assert!(prompt.contains("not comma-separated prose"));
        assert!(prompt.contains(
            "tags: role=index;case=case-1;round=1;primary=present;final_json=present;dependency=primary.evidence;source_origin=primary.evidence;validation_timestamp=1781770000"
        ));
        assert!(prompt.contains("dependency_link: primary.evidence"));
        assert!(prompt.contains("source_origin: primary.evidence"));
        assert!(prompt.contains("validation_timestamp: 1781770000"));
        assert!(prompt.contains("Keep exactly six lines"));
        assert!(!prompt.contains("role_contract"));
    }

    #[test]
    fn review_stage_prompt_blocks_placeholder_contract_descriptions() {
        let prompt = stage_prompt(&input());

        assert!(prompt.contains("SmartSteam review helper"));
        assert!(prompt.contains("risk: concrete risk evidenced by structured_facts or previews"));
        assert!(prompt.contains("change_request: small next change grounded in the same evidence"));
        assert!(prompt.contains(
            "verification: executable command or direct log/file check that verifies the change"
        ));
        assert!(prompt.contains("Never output placeholder contract descriptions"));
        assert!(prompt.contains("highest concrete code or behavior risk"));
        assert!(prompt.contains("smallest improvement to make next"));
        assert!(prompt.contains("one check that would prove the change"));
        assert!(prompt.contains(
            "Every field value must cite evidence from structured_facts, primary_answer, or final_json"
        ));
        assert!(prompt.contains("If evidence is weak, name the concrete limitation"));
        assert!(!prompt.contains("role_contract"));
        assert!(!prompt.contains("- risk: highest concrete code or behavior risk"));
        assert!(!prompt.contains("- change_request: smallest improvement to make next"));
        assert!(!prompt.contains("- verification: one check that would prove the change"));
    }

    #[test]
    fn stage_prompt_uses_role_specific_contracts() {
        let mut summary = input();
        summary.task_kind = "summary";
        let summary_prompt = stage_prompt(&summary);
        assert!(summary_prompt.contains("memory_update"));
        assert!(summary_prompt.contains("duplicate_guard"));
        assert!(summary_prompt.contains("You are not writing code"));
        assert!(summary_prompt.contains("Return only these completed lines"));
        assert!(summary_prompt.contains("Do not emit code"));
        assert!(!summary_prompt.contains("role_contract"));
        assert!(!summary_prompt.contains("<one reusable lesson"));

        let mut test_gate = input();
        test_gate.task_kind = "test-gate";
        let test_gate_prompt = stage_prompt(&test_gate);
        assert!(test_gate_prompt.contains("validation_command"));
        assert!(test_gate_prompt.contains("failure_kind"));
        assert!(test_gate_prompt.contains("SmartSteam test-gate helper"));
        assert!(test_gate_prompt.contains("- validation_gate_checked: false"));
        assert!(test_gate_prompt.contains("verdict: warn"));
        assert!(test_gate_prompt.contains("failure_kind: missing_evidence"));
        assert!(
            !test_gate_prompt
                .contains("validation_command: one safe local cargo command to run, or none")
        );
        assert!(test_gate_prompt.contains("If validation evidence is missing"));

        let mut index = input();
        index.task_kind = "index";
        let index_prompt = stage_prompt(&index);
        assert!(index_prompt.contains("clean_gist"));
        assert!(index_prompt.contains("dependency_link"));
        assert!(index_prompt.contains("source_origin"));
        assert!(index_prompt.contains("retention"));
        assert!(index_prompt.contains("role=index;case=case-1;round=1;primary=present"));
        assert!(index_prompt.contains("not comma-separated prose"));

        let review_prompt = stage_prompt(&input());
        assert!(review_prompt.contains("risk: concrete risk evidenced"));
        assert!(review_prompt.contains("Never output placeholder contract descriptions"));
        assert!(!review_prompt.contains("role_contract"));

        let mut router = input();
        router.task_kind = "router";
        let router_prompt = stage_prompt(&router);
        assert!(router_prompt.contains("route_intent"));
        assert!(router_prompt.contains("route_intent: index"));
        assert!(router_prompt.contains("tool_call"));
        assert!(router_prompt.contains("tool_call: null"));
        assert!(router_prompt.contains("preflight"));
        assert!(router_prompt.contains("Return only these completed lines"));
        assert!(router_prompt.contains("Do not say you cannot perform the task"));
        assert!(!router_prompt.contains("role_contract"));
    }

    #[test]
    fn test_gate_prompt_includes_dispatch_facts_for_small_worker_judgment() {
        let plan = test_gate_plan();
        let validation = passed_validation_evidence();
        let input = PoolStageCallInput {
            task_kind: "test-gate",
            case_name: "case-1",
            round: 1,
            validation_timestamp_unix: Some(1_781_770_000),
            validation_evidence: Some(&validation),
            original_prompt: "Check the pool",
            primary_answer: Some("Implemented a small pool-stage prompt change."),
            final_json: Some("{\"ok\":true}"),
            dispatch_plan: Some(&plan),
            completed_roles: &[],
            max_tokens: plan.effective_max_tokens,
        };

        let prompt = stage_prompt(&input);

        assert!(prompt.contains("structured_facts:"));
        assert!(prompt.contains("- round: 1"));
        assert!(prompt.contains("- validation_timestamp: 1781770000"));
        assert!(prompt.contains("- selected_role: test-gate"));
        assert!(prompt.contains("- selected_port: 8688"));
        assert!(prompt.contains("- runtime_device: cpu"));
        assert!(prompt.contains("- runtime_accelerator: accelerate"));
        assert!(prompt.contains("- gpu_layers: 0"));
        assert!(prompt.contains("- configured_max_tokens: 262144"));
        assert!(prompt.contains("- effective_max_tokens: 768"));
        assert!(prompt.contains("- max_tokens_clamped: true"));
        assert!(prompt.contains("- primary_answer_present: true"));
        assert!(prompt.contains("- final_json_present: true"));
        assert!(prompt.contains("- validation_gate_checked: true"));
        assert!(prompt.contains("- validation_gate_passed: true"));
        assert!(prompt.contains("- validation_command_source: configured"));
        assert!(prompt.contains("- validation_command_safety: explicit"));
        assert!(prompt.contains("- validation_command_safe_for_test_gate: safe"));
        assert!(prompt.contains("- validation_status_code: 0"));
        assert!(prompt.contains("test result: ok. 349 passed; 0 failed"));
        assert!(prompt.contains("verdict: pass"));
        assert!(prompt.contains(
            "validation_command: cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --target-dir target\\evolution-loop-daemon-check"
        ));
        assert!(prompt.contains("failure_kind: none"));
    }

    #[test]
    fn router_result_falls_back_to_contract_when_model_refuses() {
        let plan = PoolStageDispatchPlan {
            task_kind: "router".to_owned(),
            selected_role: "router".to_owned(),
            selected_port: Some(8689),
            selected_base_url: Some("http://127.0.0.1:8689".to_owned()),
            context_window: Some(4096),
            default_max_tokens: Some(512),
            runtime_backend: Some("llama.cpp".to_owned()),
            runtime_device: Some("metal".to_owned()),
            runtime_accelerator: Some("metal".to_owned()),
            gpu_layers: Some(999),
            configured_max_tokens: 256,
            effective_max_tokens: 256,
            max_tokens_clamped: false,
            can_accept_low_priority_task: true,
        };
        let completed_roles = vec!["quality".to_owned(), "summary".to_owned()];
        let input = PoolStageCallInput {
            task_kind: "router",
            case_name: "case-1",
            round: 1,
            validation_timestamp_unix: Some(1_781_770_000),
            validation_evidence: None,
            original_prompt: "Route this",
            primary_answer: Some("small improvement"),
            final_json: Some("{\"ok\":true}"),
            dispatch_plan: Some(&plan),
            completed_roles: &completed_roles,
            max_tokens: 256,
        };
        let mut result = PoolStageCallResult {
            task_kind: "router".to_owned(),
            ok: true,
            selected_role: Some("router".to_owned()),
            selected_model: None,
            selected_port: Some(8689),
            selected_base_url: Some("http://127.0.0.1:8689".to_owned()),
            answer: Some("I cannot help with that request.".to_owned()),
            elapsed_ms: Some(7),
            answer_chars: Some(32),
            answer_bytes: Some(32),
            answer_approx_tokens: Some(8),
            model_attempts: Vec::new(),
        };

        normalize_contract_answer(&input, &mut result);

        let answer = result.answer.as_deref().unwrap();
        assert!(answer.contains("route_intent: index"));
        assert!(answer.contains("tool_call: null"));
        assert!(answer.contains("preflight: allow because router@8689"));
        assert_eq!(result.answer_chars, Some(answer.chars().count() as u64));
    }

    #[test]
    fn test_gate_result_falls_back_to_safe_validation_command_when_model_outputs_none() {
        let validation = passed_validation_evidence();
        let mut test_gate = input();
        test_gate.task_kind = "test-gate";
        test_gate.validation_evidence = Some(&validation);
        let mut result = PoolStageCallResult {
            task_kind: "test-gate".to_owned(),
            ok: true,
            selected_role: Some("test-gate".to_owned()),
            selected_model: None,
            selected_port: Some(8688),
            selected_base_url: Some("http://127.0.0.1:8688".to_owned()),
            answer: Some("verdict: pass\nvalidation_command: None\nfailure_kind: none".to_owned()),
            elapsed_ms: Some(7),
            answer_chars: Some(58),
            answer_bytes: Some(58),
            answer_approx_tokens: Some(15),
            model_attempts: Vec::new(),
        };

        normalize_contract_answer(&test_gate, &mut result);

        let answer = result.answer.as_deref().unwrap();
        assert!(answer.contains("verdict: pass"));
        assert!(
            answer.contains(
                "validation_command: cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --target-dir target\\evolution-loop-daemon-check"
            ),
            "{answer}"
        );
        assert!(answer.contains("failure_kind: none"));
        assert_eq!(result.answer_chars, Some(answer.chars().count() as u64));
    }

    #[test]
    fn test_gate_result_does_not_pass_without_validation_evidence() {
        let mut test_gate = input();
        test_gate.task_kind = "test-gate";
        let mut result = PoolStageCallResult {
            task_kind: "test-gate".to_owned(),
            ok: true,
            selected_role: Some("test-gate".to_owned()),
            selected_model: None,
            selected_port: Some(8688),
            selected_base_url: Some("http://127.0.0.1:8688".to_owned()),
            answer: Some(
                "verdict: pass\nvalidation_command: cargo check --manifest-path tools/evolution-loop/Cargo.toml\nfailure_kind: none"
                    .to_owned(),
            ),
            elapsed_ms: Some(7),
            answer_chars: Some(109),
            answer_bytes: Some(109),
            answer_approx_tokens: Some(28),
            model_attempts: Vec::new(),
        };

        normalize_contract_answer(&test_gate, &mut result);

        let answer = result.answer.as_deref().unwrap();
        assert!(answer.contains("verdict: warn"));
        assert!(answer.contains("failure_kind: missing_evidence"));
        assert!(answer.contains(
            "validation_command: cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --no-fail-fast"
        ));
    }

    #[test]
    fn index_stage_prompt_links_tags_to_completed_review_dependency() {
        let plan = index_plan();
        let completed_roles = vec![
            "quality".to_owned(),
            "summary".to_owned(),
            "router".to_owned(),
            "review".to_owned(),
        ];
        let input = PoolStageCallInput {
            task_kind: "index",
            case_name: "case-42",
            round: 42,
            validation_timestamp_unix: Some(1_781_770_123),
            validation_evidence: None,
            original_prompt: "Check index quality",
            primary_answer: Some("Review requested stable tags."),
            final_json: Some("{\"ok\":true}"),
            dispatch_plan: Some(&plan),
            completed_roles: &completed_roles,
            max_tokens: plan.effective_max_tokens,
        };

        let prompt = stage_prompt(&input);

        assert!(prompt.contains("SmartSteam index helper"));
        assert!(prompt.contains("selected_role: index"));
        assert!(prompt.contains("selected_port: 8690"));
        assert!(prompt.contains(
            "tags: role=index;case=case-42;round=42;primary=present;final_json=present;dependency=review.change_request;source_origin=review.change_request;validation_timestamp=1781770123"
        ));
        assert!(prompt.contains("dependency_link: review.change_request"));
        assert!(prompt.contains("source_origin: review.change_request"));
        assert!(prompt.contains("validation_timestamp: 1781770123"));
        assert!(prompt.contains("tags must include role=index, case, round, primary, final_json"));
        assert!(prompt.contains("dependency_link must name the upstream helper field"));
        assert!(prompt.contains("source_origin must repeat the concrete upstream helper field"));
        assert!(!prompt.contains("comma-separated retrieval tags"));
    }

    #[test]
    fn index_result_falls_back_to_stable_tags_when_model_outputs_placeholder_tags() {
        let plan = index_plan();
        let completed_roles = vec![
            "quality".to_owned(),
            "summary".to_owned(),
            "router".to_owned(),
            "review".to_owned(),
        ];
        let input = PoolStageCallInput {
            task_kind: "index",
            case_name: "case-42",
            round: 42,
            validation_timestamp_unix: Some(1_781_770_123),
            validation_evidence: None,
            original_prompt: "Check index quality",
            primary_answer: Some("Review requested stable tags."),
            final_json: Some("{\"ok\":true}"),
            dispatch_plan: Some(&plan),
            completed_roles: &completed_roles,
            max_tokens: plan.effective_max_tokens,
        };
        let mut result = PoolStageCallResult {
            task_kind: "index".to_owned(),
            ok: true,
            selected_role: Some("index".to_owned()),
            selected_model: None,
            selected_port: Some(8690),
            selected_base_url: Some("http://127.0.0.1:8690".to_owned()),
            answer: Some(
                "clean_gist: compact searchable summary\ntags: comma-separated retrieval tags\nretention: keep"
                    .to_owned(),
            ),
            elapsed_ms: Some(9),
            answer_chars: Some(88),
            answer_bytes: Some(88),
            answer_approx_tokens: Some(22),
            model_attempts: Vec::new(),
        };

        normalize_contract_answer(&input, &mut result);

        let answer = result.answer.as_deref().unwrap();
        assert!(answer.contains("clean_gist: Index round 42 case-42 with index@8690"));
        assert!(answer.contains(
            "tags: role=index;case=case-42;round=42;primary=present;final_json=present;dependency=review.change_request;source_origin=review.change_request;validation_timestamp=1781770123"
        ));
        assert!(answer.contains("dependency_link: review.change_request"));
        assert!(answer.contains("source_origin: review.change_request"));
        assert!(answer.contains("validation_timestamp: 1781770123"));
        assert!(answer.contains("retention: keep; compact retrieval evidence"));
        assert_eq!(result.answer_chars, Some(answer.chars().count() as u64));
    }

    #[test]
    fn index_result_keeps_stable_key_value_tags() {
        let plan = index_plan();
        let input = PoolStageCallInput {
            task_kind: "index",
            case_name: "case-42",
            round: 42,
            validation_timestamp_unix: Some(1_781_770_123),
            validation_evidence: None,
            original_prompt: "Check index quality",
            primary_answer: Some("Review requested stable tags."),
            final_json: Some("{\"ok\":true}"),
            dispatch_plan: Some(&plan),
            completed_roles: &[],
            max_tokens: plan.effective_max_tokens,
        };
        let answer = "clean_gist: stable retrieval labels are present\ntags: role=index;case=case-42;round=42;primary=present;final_json=present;dependency=primary.evidence;source_origin=primary.evidence;validation_timestamp=1781770123\ndependency_link: primary.evidence\nsource_origin: primary.evidence\nvalidation_timestamp: 1781770123\nretention: keep; labels are compact";
        let mut result = PoolStageCallResult {
            task_kind: "index".to_owned(),
            ok: true,
            selected_role: Some("index".to_owned()),
            selected_model: None,
            selected_port: Some(8690),
            selected_base_url: Some("http://127.0.0.1:8690".to_owned()),
            answer: Some(answer.to_owned()),
            elapsed_ms: Some(9),
            answer_chars: Some(answer.chars().count() as u64),
            answer_bytes: Some(answer.len() as u64),
            answer_approx_tokens: Some(37),
            model_attempts: Vec::new(),
        };

        normalize_contract_answer(&input, &mut result);

        assert_eq!(result.answer.as_deref(), Some(answer));
        assert_eq!(result.answer_chars, Some(answer.chars().count() as u64));
    }

    #[test]
    fn index_result_keeps_contract_when_clean_gist_mentions_tags() {
        let plan = index_plan();
        let input = PoolStageCallInput {
            task_kind: "index",
            case_name: "case-42",
            round: 42,
            validation_timestamp_unix: Some(1_781_770_123),
            validation_evidence: None,
            original_prompt: "Check index quality",
            primary_answer: Some("Review requested stable tags."),
            final_json: Some("{\"ok\":true}"),
            dispatch_plan: Some(&plan),
            completed_roles: &[],
            max_tokens: plan.effective_max_tokens,
        };
        let answer = "clean_gist: stable tags are present in the compact index contract\ntags: role=index;case=case-42;round=42;primary=present;final_json=present;dependency=primary.evidence;source_origin=primary.evidence;validation_timestamp=1781770123\ndependency_link: primary.evidence\nsource_origin: primary.evidence\nvalidation_timestamp: 1781770123\nretention: keep; labels are compact";
        let mut result = PoolStageCallResult {
            task_kind: "index".to_owned(),
            ok: true,
            selected_role: Some("index".to_owned()),
            selected_model: None,
            selected_port: Some(8690),
            selected_base_url: Some("http://127.0.0.1:8690".to_owned()),
            answer: Some(answer.to_owned()),
            elapsed_ms: Some(9),
            answer_chars: Some(answer.chars().count() as u64),
            answer_bytes: Some(answer.len() as u64),
            answer_approx_tokens: Some(52),
            model_attempts: Vec::new(),
        };

        normalize_contract_answer(&input, &mut result);

        assert_eq!(result.answer.as_deref(), Some(answer));
        assert_eq!(result.answer_chars, Some(answer.chars().count() as u64));
    }

    #[test]
    fn index_result_falls_back_when_dependency_link_disagrees_with_tags() {
        let plan = index_plan();
        let completed_roles = vec![
            "quality".to_owned(),
            "summary".to_owned(),
            "review".to_owned(),
        ];
        let input = PoolStageCallInput {
            task_kind: "index",
            case_name: "case-42",
            round: 42,
            validation_timestamp_unix: Some(1_781_770_123),
            validation_evidence: None,
            original_prompt: "Check index quality",
            primary_answer: Some("Review requested stable tags."),
            final_json: Some("{\"ok\":true}"),
            dispatch_plan: Some(&plan),
            completed_roles: &completed_roles,
            max_tokens: plan.effective_max_tokens,
        };
        let answer = "clean_gist: stable retrieval labels are present\ntags: role=index;case=case-42;round=42;primary=present;final_json=present;dependency=summary.next_context;source_origin=summary.next_context;validation_timestamp=1781770123\ndependency_link: review.change_request\nsource_origin: summary.next_context\nvalidation_timestamp: 1781770123\nretention: keep; labels are compact";
        let mut result = PoolStageCallResult {
            task_kind: "index".to_owned(),
            ok: true,
            selected_role: Some("index".to_owned()),
            selected_model: None,
            selected_port: Some(8690),
            selected_base_url: Some("http://127.0.0.1:8690".to_owned()),
            answer: Some(answer.to_owned()),
            elapsed_ms: Some(9),
            answer_chars: Some(answer.chars().count() as u64),
            answer_bytes: Some(answer.len() as u64),
            answer_approx_tokens: Some(50),
            model_attempts: Vec::new(),
        };

        normalize_contract_answer(&input, &mut result);

        let normalized = result.answer.as_deref().unwrap();
        assert_ne!(normalized, answer);
        assert!(normalized.contains(
            "tags: role=index;case=case-42;round=42;primary=present;final_json=present;dependency=review.change_request;source_origin=review.change_request;validation_timestamp=1781770123"
        ));
        assert!(normalized.contains("dependency_link: review.change_request"));
        assert!(normalized.contains("source_origin: review.change_request"));
        assert_eq!(result.answer_chars, Some(normalized.chars().count() as u64));
    }

    #[test]
    fn test_gate_result_keeps_safe_validation_command() {
        let answer = "verdict: pass\nvalidation_command: cargo check --manifest-path tools/evolution-loop/Cargo.toml\nfailure_kind: none";
        let validation = passed_validation_evidence();
        let mut test_gate = input();
        test_gate.task_kind = "test-gate";
        test_gate.validation_evidence = Some(&validation);
        let mut result = PoolStageCallResult {
            task_kind: "test-gate".to_owned(),
            ok: true,
            selected_role: Some("test-gate".to_owned()),
            selected_model: None,
            selected_port: Some(8688),
            selected_base_url: Some("http://127.0.0.1:8688".to_owned()),
            answer: Some(answer.to_owned()),
            elapsed_ms: Some(7),
            answer_chars: Some(answer.chars().count() as u64),
            answer_bytes: Some(answer.len() as u64),
            answer_approx_tokens: Some(28),
            model_attempts: Vec::new(),
        };

        normalize_contract_answer(&test_gate, &mut result);

        assert_eq!(result.answer.as_deref(), Some(answer));
        assert_eq!(result.answer_chars, Some(answer.chars().count() as u64));
    }

    #[test]
    fn request_body_carries_completed_roles_for_dependency_precheck() {
        let completed_roles = vec!["quality".to_owned(), "summary".to_owned()];
        let input = PoolStageCallInput {
            completed_roles: &completed_roles,
            ..input()
        };

        let body = request_body(&input);

        assert!(body.contains("\"completed_roles\":[\"quality\",\"summary\"]"));
    }

    #[test]
    fn parses_pool_call_execution_metrics() {
        let parsed = parse_response(
            "review",
            "{\"ok\":true,\"task_kind\":\"review\",\"selected_role\":\"review\",\"selected_port\":8688,\"selected_base_url\":\"http://127.0.0.1:8688\",\"elapsed_ms\":123,\"answer_chars\":40,\"answer_bytes\":42,\"answer_approx_tokens\":10,\"answer\":\"looks good\"}",
        );

        assert!(parsed.ok);
        assert_eq!(parsed.task_kind, "review");
        assert_eq!(parsed.selected_role.as_deref(), Some("review"));
        assert_eq!(parsed.selected_port, Some(8688));
        assert_eq!(
            parsed.selected_base_url.as_deref(),
            Some("http://127.0.0.1:8688")
        );
        assert_eq!(parsed.selected_model, None);
        assert_eq!(parsed.elapsed_ms, Some(123));
        assert_eq!(parsed.answer_chars, Some(40));
        assert_eq!(parsed.answer_bytes, Some(42));
        assert_eq!(parsed.answer_approx_tokens, Some(10));
        assert_eq!(parsed.answer.as_deref(), Some("looks good"));
        assert!(parsed.model_attempts.is_empty());
    }
}
