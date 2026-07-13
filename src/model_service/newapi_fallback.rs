use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use rust_norion::{
    DraftToken, GenerationContext, InferenceBackend, InferenceDraft, ReasoningStep,
    RuntimeDiagnostics, RuntimeError,
};

use crate::model_service::json::{
    json_bool_field, json_string_field, json_u64_field, service_json_string,
};
use crate::path_utils::ensure_parent_dir;

const BASE_URL_ENV: &str = "NORION_NEWAPI_BASE_URL";
const API_KEY_ENV: &str = "NORION_NEWAPI_API_KEY";
const API_KEY_FILE_ENV: &str = "NORION_NEWAPI_API_KEY_FILE";
const MODELS_ENV: &str = "NORION_NEWAPI_ALLOWED_MODELS";
const OUTCOMES_PATH_ENV: &str = "NORION_NEWAPI_OUTCOMES_PATH";
const MODEL_OUTCOMES_PATH_ENV: &str = "NORION_NEWAPI_MODEL_OUTCOMES_PATH";
const TIMEOUT_SECS_ENV: &str = "NORION_NEWAPI_TIMEOUT_SECS";
const COOLDOWN_SECS_ENV: &str = "NORION_NEWAPI_FAILURE_COOLDOWN_SECS";
const MAX_ATTEMPTS_ENV: &str = "NORION_NEWAPI_MAX_ATTEMPTS";
const DEFAULT_OUTCOMES_PATH: &str = "target/evolution/newapi-model-outcomes.jsonl";
const DEFAULT_TIMEOUT_SECS: u64 = 45;
const DEFAULT_COOLDOWN_SECS: u64 = 6 * 60 * 60;
const DEFAULT_MAX_ATTEMPTS: usize = 3;
const MAX_API_KEY_FILE_BYTES: u64 = 4096;
const DEFAULT_ALLOWED_MODELS: &str =
    include_str!("../../tools/evolution-loop/config/newapi-models.txt");

pub(crate) struct NewApiFallbackBackend<'a, B: InferenceBackend> {
    primary: &'a mut B,
    config: Option<NewApiConfig>,
    generation_max_tokens: Option<usize>,
}

impl<'a, B: InferenceBackend> NewApiFallbackBackend<'a, B> {
    pub(crate) fn from_env(primary: &'a mut B) -> Self {
        let config = NewApiConfig::from_env();
        configure_telemetry(config.as_ref());
        Self {
            primary,
            config,
            generation_max_tokens: None,
        }
    }

    fn generate_with_caller<F>(
        &mut self,
        context: GenerationContext<'_>,
        mut caller: F,
    ) -> InferenceDraft
    where
        F: FnMut(&NewApiConfig, &str, &str, usize) -> Result<NewApiCall, NewApiFailure>,
    {
        let prompt = context.prompt;
        let primary = self.primary.generate(context);
        resolve_fallback(
            self.config.as_ref(),
            primary,
            prompt,
            self.generation_max_tokens.unwrap_or(512),
            &mut caller,
        )
    }
}

impl<B: InferenceBackend> InferenceBackend for NewApiFallbackBackend<'_, B> {
    fn configure_generation(&mut self, max_tokens: Option<usize>) {
        self.generation_max_tokens = max_tokens.map(|value| value.max(1));
        self.primary.configure_generation(max_tokens);
    }

    fn configure_runtime_endpoint_override(
        &mut self,
        base_url: Option<&str>,
    ) -> Result<bool, String> {
        self.primary.configure_runtime_endpoint_override(base_url)
    }

    fn runtime_endpoint_override_active(&self) -> Option<&str> {
        self.primary.runtime_endpoint_override_active()
    }

    fn runtime_native_context_window(&self) -> Option<usize> {
        self.primary.runtime_native_context_window()
    }

    fn embed_text(&mut self, text: &str) -> Option<Vec<f32>> {
        self.primary.embed_text(text)
    }

    fn generate(&mut self, context: GenerationContext<'_>) -> InferenceDraft {
        self.generate_with_caller(context, call_newapi_model)
    }

    fn generate_stream_checked(
        &mut self,
        context: GenerationContext<'_>,
        on_token: &mut dyn FnMut(&DraftToken) -> Result<(), RuntimeError>,
    ) -> InferenceDraft {
        if self.config.is_none() {
            return self.primary.generate_stream_checked(context, on_token);
        }
        let prompt = context.prompt;
        let mut primary_tokens = Vec::new();
        let primary = self.primary.generate_stream_checked(context, &mut |token| {
            primary_tokens.push(token.clone());
            Ok(())
        });
        let resolved = resolve_fallback(
            self.config.as_ref(),
            primary,
            prompt,
            self.generation_max_tokens.unwrap_or(512),
            &mut call_newapi_model,
        );
        let tokens = if resolved.runtime_diagnostics.model_fallback_used {
            resolved.tokens.clone()
        } else {
            primary_tokens
        };
        for token in &tokens {
            if let Err(error) = on_token(token) {
                return InferenceDraft::new(
                    format!("Runtime backend error: {}", error.message()),
                    vec![ReasoningStep::new(
                        "runtime_stream_observer_error",
                        error.message(),
                        0.0,
                    )],
                );
            }
        }
        resolved
    }
}

#[derive(Clone)]
struct NewApiConfig {
    base_url: String,
    api_key: String,
    allowed_models: Vec<String>,
    outcomes_path: PathBuf,
    timeout_secs: u64,
    cooldown_secs: u64,
    max_attempts: usize,
}

impl NewApiConfig {
    fn from_env() -> Option<Self> {
        let base_url = std::env::var(BASE_URL_ENV).ok()?;
        let api_key = api_key_from_sources(
            std::env::var(API_KEY_ENV).ok(),
            std::env::var(API_KEY_FILE_ENV).ok().map(PathBuf::from),
        )?;
        let models =
            std::env::var(MODELS_ENV).unwrap_or_else(|_| DEFAULT_ALLOWED_MODELS.to_owned());
        Self::new(
            base_url,
            api_key,
            models
                .split([',', '\n', '\r'])
                .filter(|model| !model.trim().is_empty())
                .map(str::to_owned),
            outcomes_path_from_sources(
                std::env::var(OUTCOMES_PATH_ENV).ok(),
                std::env::var(MODEL_OUTCOMES_PATH_ENV).ok(),
            ),
            env_u64(TIMEOUT_SECS_ENV, DEFAULT_TIMEOUT_SECS),
            env_u64(COOLDOWN_SECS_ENV, DEFAULT_COOLDOWN_SECS),
            env_usize(MAX_ATTEMPTS_ENV, DEFAULT_MAX_ATTEMPTS),
        )
    }

    fn new(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        models: impl IntoIterator<Item = String>,
        outcomes_path: PathBuf,
        timeout_secs: u64,
        cooldown_secs: u64,
        max_attempts: usize,
    ) -> Option<Self> {
        let base_url = normalize_base_url(base_url.into())?;
        let api_key = api_key
            .into()
            .trim()
            .trim_start_matches('\u{feff}')
            .trim()
            .to_owned();
        if api_key.trim().is_empty() || api_key.contains(['\r', '\n']) {
            return None;
        }
        let mut allowed_models = Vec::new();
        for model in models {
            let model = model.trim();
            if valid_model_id(model)
                && !allowed_models.iter().any(|existing| existing == model)
                && allowed_models.len() < 128
            {
                allowed_models.push(model.to_owned());
            }
        }
        if allowed_models.is_empty() {
            return None;
        }
        Some(Self {
            base_url,
            api_key,
            allowed_models,
            outcomes_path,
            timeout_secs: timeout_secs.max(1),
            cooldown_secs: cooldown_secs.max(1),
            max_attempts: max_attempts.clamp(1, 8),
        })
    }
}

fn api_key_from_sources(env_value: Option<String>, file_path: Option<PathBuf>) -> Option<String> {
    if let Some(value) = env_value.filter(|value| !value.trim().is_empty()) {
        return Some(value);
    }
    let path = file_path?;
    let metadata = fs::metadata(&path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_API_KEY_FILE_BYTES {
        return None;
    }
    fs::read_to_string(path).ok()
}

fn outcomes_path_from_sources(runtime_path: Option<String>, smoke_path: Option<String>) -> PathBuf {
    runtime_path
        .filter(|path| !path.trim().is_empty())
        .or_else(|| smoke_path.filter(|path| !path.trim().is_empty()))
        .map(|path| PathBuf::from(path.trim()))
        .unwrap_or_else(|| PathBuf::from(DEFAULT_OUTCOMES_PATH))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NewApiCall {
    model: String,
    answer: String,
    elapsed_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NewApiFailure {
    kind: &'static str,
    stop_pool: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ModelOutcome {
    ok: bool,
    reason: Option<String>,
    elapsed_ms: Option<u64>,
    observed_unix: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ModelPlan {
    models: Vec<String>,
    cooldown_skipped: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct NewApiFallbackTelemetry {
    pub(crate) configured: bool,
    pub(crate) allowed_models: usize,
    pub(crate) max_attempts: usize,
    pub(crate) primary_failures: usize,
    pub(crate) fallback_attempts: usize,
    pub(crate) fallback_successes: usize,
    pub(crate) fallback_failures: usize,
    pub(crate) cooldown_skipped: usize,
    pub(crate) quarantined_models: usize,
    pub(crate) persistence_failures: usize,
    pub(crate) last_used: bool,
    pub(crate) last_selected_model: Option<String>,
    pub(crate) last_failure_kind: Option<String>,
}

static TELEMETRY: OnceLock<Mutex<NewApiFallbackTelemetry>> = OnceLock::new();

pub(crate) fn newapi_fallback_telemetry() -> NewApiFallbackTelemetry {
    TELEMETRY
        .get_or_init(|| Mutex::new(NewApiFallbackTelemetry::default()))
        .lock()
        .map(|telemetry| telemetry.clone())
        .unwrap_or_default()
}

fn configure_telemetry(config: Option<&NewApiConfig>) {
    let quarantined_models = config
        .map(|config| plan_models(config, unix_now()).cooldown_skipped.len())
        .unwrap_or(0);
    update_telemetry(|telemetry| {
        telemetry.configured = config.is_some();
        telemetry.allowed_models = config.map_or(0, |config| config.allowed_models.len());
        telemetry.max_attempts = config.map_or(0, |config| config.max_attempts);
        telemetry.quarantined_models = quarantined_models;
        telemetry.last_used = false;
        telemetry.last_selected_model = None;
        telemetry.last_failure_kind = None;
    });
}

fn resolve_fallback<F>(
    config: Option<&NewApiConfig>,
    mut primary: InferenceDraft,
    prompt: &str,
    max_tokens: usize,
    caller: &mut F,
) -> InferenceDraft
where
    F: FnMut(&NewApiConfig, &str, &str, usize) -> Result<NewApiCall, NewApiFailure>,
{
    primary.runtime_diagnostics.model_fallback_configured = config.is_some();
    let Some(primary_failure) = retryable_primary_failure(&primary) else {
        update_telemetry(|telemetry| {
            telemetry.last_used = false;
            telemetry.last_selected_model = None;
            telemetry.last_failure_kind = None;
        });
        return primary;
    };
    let Some(config) = config else {
        return primary;
    };

    update_telemetry(|telemetry| {
        telemetry.primary_failures = telemetry.primary_failures.saturating_add(1);
        telemetry.last_used = false;
        telemetry.last_selected_model = None;
        telemetry.last_failure_kind = Some(primary_failure.to_owned());
    });
    let now = unix_now();
    let plan = plan_models(config, now);
    update_telemetry(|telemetry| {
        telemetry.cooldown_skipped = telemetry
            .cooldown_skipped
            .saturating_add(plan.cooldown_skipped.len());
    });

    let mut attempts = 0usize;
    let mut failures = 0usize;
    let mut quarantined = 0usize;
    for model in plan.models.iter().take(config.max_attempts) {
        attempts = attempts.saturating_add(1);
        update_telemetry(|telemetry| {
            telemetry.fallback_attempts = telemetry.fallback_attempts.saturating_add(1);
        });
        match caller(config, model, prompt, max_tokens.max(1)) {
            Ok(call) => {
                let persistence_failed = persist_outcome(
                    &config.outcomes_path,
                    model,
                    true,
                    None,
                    Some(call.elapsed_ms),
                    now,
                )
                .is_err();
                let quarantined_models = plan_models(config, now).cooldown_skipped.len();
                update_telemetry(|telemetry| {
                    telemetry.fallback_successes = telemetry.fallback_successes.saturating_add(1);
                    telemetry.persistence_failures = telemetry
                        .persistence_failures
                        .saturating_add(usize::from(persistence_failed));
                    telemetry.quarantined_models = quarantined_models;
                    telemetry.last_used = true;
                    telemetry.last_selected_model = Some(call.model.clone());
                    telemetry.last_failure_kind = None;
                });
                return fallback_success_draft(
                    call,
                    attempts,
                    failures,
                    quarantined,
                    plan.cooldown_skipped.len(),
                );
            }
            Err(failure) => {
                failures = failures.saturating_add(1);
                let persistence_failed = persist_outcome(
                    &config.outcomes_path,
                    model,
                    false,
                    Some(failure.kind),
                    None,
                    now,
                )
                .is_err();
                quarantined = quarantined.saturating_add(usize::from(!persistence_failed));
                let quarantined_models = plan_models(config, now).cooldown_skipped.len();
                update_telemetry(|telemetry| {
                    telemetry.fallback_failures = telemetry.fallback_failures.saturating_add(1);
                    telemetry.quarantined_models = quarantined_models;
                    telemetry.persistence_failures = telemetry
                        .persistence_failures
                        .saturating_add(usize::from(persistence_failed));
                    telemetry.last_failure_kind = Some(failure.kind.to_owned());
                });
                if failure.stop_pool {
                    break;
                }
            }
        }
    }

    primary.trace.push(ReasoningStep::new(
        "newapi_fallback_failed",
        format!(
            "primary_failure={} attempts={} failures={} cooldown_skipped={} all_failed=true",
            primary_failure,
            attempts,
            failures,
            plan.cooldown_skipped.len()
        ),
        0.0,
    ));
    apply_fallback_diagnostics(
        &mut primary.runtime_diagnostics,
        false,
        attempts,
        failures,
        quarantined,
        plan.cooldown_skipped.len(),
        None,
        true,
    );
    primary
}

fn fallback_success_draft(
    call: NewApiCall,
    attempts: usize,
    failures: usize,
    quarantined: usize,
    cooldown_skipped: usize,
) -> InferenceDraft {
    let mut diagnostics = RuntimeDiagnostics {
        model_id: Some(call.model.clone()),
        selected_adapter: Some("newapi-fallback".to_owned()),
        ..RuntimeDiagnostics::default()
    };
    apply_fallback_diagnostics(
        &mut diagnostics,
        true,
        attempts,
        failures,
        quarantined,
        cooldown_skipped,
        Some(call.model.clone()),
        false,
    );
    InferenceDraft::new(
        call.answer.clone(),
        vec![ReasoningStep::new(
            "newapi_fallback",
            format!(
                "selected_model={} attempts={} failures={} cooldown_skipped={} elapsed_ms={}",
                call.model, attempts, failures, cooldown_skipped, call.elapsed_ms
            ),
            0.85,
        )],
    )
    .with_tokens(answer_tokens(&call.answer))
    .with_runtime_diagnostics(diagnostics)
}

fn apply_fallback_diagnostics(
    diagnostics: &mut RuntimeDiagnostics,
    used: bool,
    attempts: usize,
    failures: usize,
    quarantined: usize,
    cooldown_skipped: usize,
    selected_model: Option<String>,
    all_failed: bool,
) {
    diagnostics.model_fallback_configured = true;
    diagnostics.model_fallback_primary_failed = true;
    diagnostics.model_fallback_used = used;
    diagnostics.model_fallback_attempts = attempts;
    diagnostics.model_fallback_failures = failures;
    diagnostics.model_fallback_quarantined = quarantined;
    diagnostics.model_fallback_cooldown_skipped = cooldown_skipped;
    diagnostics.model_fallback_selected_model = selected_model;
    diagnostics.model_fallback_all_failed = all_failed;
}

fn answer_tokens(answer: &str) -> Vec<DraftToken> {
    let characters = answer.chars().collect::<Vec<_>>();
    characters
        .chunks(4)
        .map(|chunk| DraftToken::new(chunk.iter().collect::<String>()))
        .collect()
}

fn retryable_primary_failure(draft: &InferenceDraft) -> Option<&'static str> {
    let lower = draft.answer.to_ascii_lowercase();
    let runtime_error = lower.starts_with("runtime backend error:")
        || draft
            .trace
            .iter()
            .any(|step| step.label.contains("runtime") && step.label.contains("error"));
    if !runtime_error || lower.contains("cancel") || lower.contains("observer") {
        return None;
    }
    if lower.contains("timeout") || lower.contains("timed out") {
        return Some("timeout");
    }
    if [
        "connect",
        "connection",
        "unavailable",
        "broken pipe",
        "reset by peer",
        "http status 5",
        "response missing",
        "response contract",
        "protocol",
        "runtime command failed",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
    {
        return Some("unavailable");
    }
    None
}

fn plan_models(config: &NewApiConfig, now: u64) -> ModelPlan {
    let outcomes = read_outcomes(&config.outcomes_path);
    plan_models_from_outcomes(&config.allowed_models, &outcomes, now, config.cooldown_secs)
}

fn plan_models_from_outcomes(
    allowed_models: &[String],
    outcomes: &HashMap<String, ModelOutcome>,
    now: u64,
    cooldown_secs: u64,
) -> ModelPlan {
    let mut ranked = Vec::new();
    let mut cooldown_skipped = Vec::new();
    for (index, model) in allowed_models.iter().enumerate() {
        match outcomes.get(model) {
            Some(outcome)
                if !outcome.ok && outcome.observed_unix.saturating_add(cooldown_secs) > now =>
            {
                cooldown_skipped.push(model.clone());
            }
            Some(outcome) if outcome.ok => ranked.push((
                0u8,
                outcome.elapsed_ms.unwrap_or(u64::MAX),
                index,
                model.clone(),
            )),
            _ => ranked.push((1u8, u64::MAX, index, model.clone())),
        }
    }
    ranked.sort_by_key(|entry| (entry.0, entry.1, entry.2));
    ModelPlan {
        models: ranked.into_iter().map(|entry| entry.3).collect(),
        cooldown_skipped,
    }
}

fn call_newapi_model(
    config: &NewApiConfig,
    model: &str,
    prompt: &str,
    max_tokens: usize,
) -> Result<NewApiCall, NewApiFailure> {
    let body = format!(
        "{{\"model\":{},\"messages\":[{{\"role\":\"user\",\"content\":{}}}],\"stream\":false,\"max_tokens\":{}}}",
        service_json_string(model),
        service_json_string(prompt),
        max_tokens.max(1),
    );
    let started = Instant::now();
    let response = curl_post_json(
        &config.base_url,
        "/chat/completions",
        &body,
        &config.api_key,
        config.timeout_secs,
    )?;
    if !(200..300).contains(&response.status) {
        return Err(http_failure(response.status));
    }
    let answer = json_string_field(&response.body, "content")
        .or_else(|| json_string_field(&response.body, "text"))
        .filter(|answer| !answer.trim().is_empty())
        .ok_or(NewApiFailure {
            kind: "response_shape",
            stop_pool: false,
        })?;
    Ok(NewApiCall {
        model: model.to_owned(),
        answer,
        elapsed_ms: elapsed_millis(started),
    })
}

struct CurlResponse {
    status: u16,
    body: String,
}

fn curl_post_json(
    base_url: &str,
    path: &str,
    body: &str,
    api_key: &str,
    timeout_secs: u64,
) -> Result<CurlResponse, NewApiFailure> {
    let url = request_url(base_url, path).ok_or(NewApiFailure {
        kind: "configuration",
        stop_pool: true,
    })?;
    let config = curl_config(&url, body, api_key);
    let mut command = Command::new(if cfg!(windows) { "curl.exe" } else { "curl" });
    command
        .args(curl_args(timeout_secs))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|_| NewApiFailure {
        kind: "transport",
        stop_pool: true,
    })?;
    child
        .stdin
        .take()
        .ok_or(NewApiFailure {
            kind: "transport",
            stop_pool: true,
        })?
        .write_all(config.as_bytes())
        .map_err(|_| NewApiFailure {
            kind: "transport",
            stop_pool: true,
        })?;
    let output = child.wait_with_output().map_err(|_| NewApiFailure {
        kind: "transport",
        stop_pool: true,
    })?;
    if !output.status.success() {
        return Err(NewApiFailure {
            kind: "transport",
            stop_pool: true,
        });
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let (body, status) = split_body_status(&stdout).ok_or(NewApiFailure {
        kind: if output.status.success() {
            "response_shape"
        } else {
            "transport"
        },
        stop_pool: false,
    })?;
    Ok(CurlResponse { status, body })
}

fn curl_args(timeout_secs: u64) -> Vec<String> {
    vec![
        "-sS".to_owned(),
        "--show-error".to_owned(),
        "--max-time".to_owned(),
        timeout_secs.max(1).to_string(),
        "--write-out".to_owned(),
        "\n%{http_code}".to_owned(),
        "--config".to_owned(),
        "-".to_owned(),
    ]
}

fn curl_config(url: &str, body: &str, api_key: &str) -> String {
    format!(
        "url = {}\nrequest = \"POST\"\nheader = {}\nheader = \"content-type: application/json; charset=utf-8\"\nheader = \"accept: application/json\"\ndata-binary = {}\n",
        curl_quote(url),
        curl_quote(&format!("authorization: Bearer {api_key}")),
        curl_quote(body)
    )
}

fn curl_quote(value: &str) -> String {
    format!(
        "\"{}\"",
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\r', "\\r")
            .replace('\n', "\\n")
    )
}

fn split_body_status(stdout: &str) -> Option<(String, u16)> {
    let (body, status) = stdout.rsplit_once('\n')?;
    Some((body.to_owned(), status.trim().parse().ok()?))
}

fn http_failure(status: u16) -> NewApiFailure {
    match status {
        401 | 403 => NewApiFailure {
            kind: "auth",
            stop_pool: true,
        },
        408 | 504 => NewApiFailure {
            kind: "timeout",
            stop_pool: false,
        },
        429 => NewApiFailure {
            kind: "rate_limit",
            stop_pool: false,
        },
        400 | 404 | 422 => NewApiFailure {
            kind: "model_error",
            stop_pool: false,
        },
        _ => NewApiFailure {
            kind: "provider_error",
            stop_pool: false,
        },
    }
}

fn persist_outcome(
    path: &Path,
    model: &str,
    ok: bool,
    reason: Option<&str>,
    elapsed_ms: Option<u64>,
    observed_unix: u64,
) -> std::io::Result<()> {
    ensure_parent_dir(path)?;
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(
        file,
        "{{\"observed_unix\":{},\"model\":{},\"ok\":{},\"reason\":{},\"elapsed_ms\":{}}}",
        observed_unix,
        service_json_string(model),
        ok,
        reason
            .map(service_json_string)
            .unwrap_or_else(|| "null".to_owned()),
        elapsed_ms
            .map(|value| value.to_string())
            .unwrap_or_else(|| "null".to_owned())
    )
}

fn read_outcomes(path: &Path) -> HashMap<String, ModelOutcome> {
    let Ok(text) = fs::read_to_string(path) else {
        return HashMap::new();
    };
    text.lines()
        .filter_map(|line| {
            let model = json_string_field(line, "model")?;
            Some((
                model,
                ModelOutcome {
                    ok: json_bool_field(line, "ok").unwrap_or(false),
                    reason: json_string_field(line, "reason"),
                    elapsed_ms: json_u64_field(line, "elapsed_ms"),
                    observed_unix: json_u64_field(line, "observed_unix").unwrap_or(0),
                },
            ))
        })
        .collect()
}

fn update_telemetry(update: impl FnOnce(&mut NewApiFallbackTelemetry)) {
    if let Ok(mut telemetry) = TELEMETRY
        .get_or_init(|| Mutex::new(NewApiFallbackTelemetry::default()))
        .lock()
    {
        update(&mut telemetry);
    }
}

fn normalize_base_url(value: String) -> Option<String> {
    let value = value.trim().trim_end_matches('/');
    if value.contains(['\r', '\n'])
        || !value.starts_with("https://")
        || value
            .split_once("//")
            .map(|(_, rest)| rest.split('/').next().unwrap_or_default().contains('@'))
            .unwrap_or(true)
    {
        return None;
    }
    Some(value.to_owned())
}

fn request_url(base_url: &str, path: &str) -> Option<String> {
    let base_url = normalize_base_url(base_url.to_owned())?;
    Some(format!(
        "{}{}",
        base_url,
        if path.starts_with('/') {
            path.to_owned()
        } else {
            format!("/{path}")
        }
    ))
}

fn valid_model_id(model: &str) -> bool {
    !model.is_empty()
        && model.len() <= 160
        && model
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "/._:-".contains(character))
}

fn env_u64(name: &str, fallback: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(fallback)
}

fn env_usize(name: &str, fallback: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(fallback)
}

fn elapsed_millis(started: Instant) -> u64 {
    started.elapsed().as_millis().try_into().unwrap_or(u64::MAX)
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(path: PathBuf) -> NewApiConfig {
        NewApiConfig::new(
            "https://provider.example/v1",
            "secret-token",
            ["slow".to_owned(), "fast".to_owned(), "fresh".to_owned()],
            path,
            30,
            100,
            3,
        )
        .unwrap()
    }

    fn runtime_error(message: &str) -> InferenceDraft {
        InferenceDraft::new(
            format!("Runtime backend error: {message}"),
            vec![ReasoningStep::new("runtime_error", message, 0.0)],
        )
    }

    #[test]
    fn primary_success_skips_newapi() {
        let mut calls = 0;
        let draft = resolve_fallback(
            Some(&config(PathBuf::from("unused"))),
            InferenceDraft::new("apple ok", Vec::new()),
            "prompt",
            32,
            &mut |_, _, _, _| {
                calls += 1;
                unreachable!()
            },
        );

        assert_eq!(draft.answer, "apple ok");
        assert_eq!(calls, 0);
    }

    #[test]
    fn retryable_primary_failure_uses_ranked_newapi_model() {
        let root = test_path("fallback");
        let config = config(root.clone());
        let now = unix_now();
        persist_outcome(&root, "slow", true, None, Some(500), now).unwrap();
        persist_outcome(&root, "fast", true, None, Some(50), now).unwrap();
        let mut attempted = Vec::new();

        let draft = resolve_fallback(
            Some(&config),
            runtime_error("connection refused"),
            "prompt",
            32,
            &mut |_, model, _, _| {
                attempted.push(model.to_owned());
                Ok(NewApiCall {
                    model: model.to_owned(),
                    answer: "fallback ok".to_owned(),
                    elapsed_ms: 12,
                })
            },
        );

        assert_eq!(attempted, vec!["fast"]);
        assert_eq!(draft.answer, "fallback ok");
        assert_eq!(draft.runtime_diagnostics.model_id.as_deref(), Some("fast"));
        assert_eq!(
            draft.runtime_diagnostics.selected_adapter.as_deref(),
            Some("newapi-fallback")
        );
        assert!(draft.runtime_diagnostics.model_fallback_used);
        let _ = fs::remove_file(root);
    }

    #[test]
    fn non_retryable_primary_failure_does_not_call_newapi() {
        let mut calls = 0;
        let draft = resolve_fallback(
            Some(&config(PathBuf::from("unused"))),
            runtime_error("development evidence blocked from runtime prompt surface"),
            "prompt",
            32,
            &mut |_, _, _, _| {
                calls += 1;
                unreachable!()
            },
        );

        assert!(draft.answer.contains("development evidence blocked"));
        assert_eq!(calls, 0);
    }

    #[test]
    fn failed_model_enters_cooldown_and_is_skipped() {
        let now = 1_800_000_000;
        let outcomes = HashMap::from([
            (
                "slow".to_owned(),
                ModelOutcome {
                    ok: true,
                    reason: None,
                    elapsed_ms: Some(500),
                    observed_unix: now - 5,
                },
            ),
            (
                "fast".to_owned(),
                ModelOutcome {
                    ok: false,
                    reason: Some("timeout".to_owned()),
                    elapsed_ms: None,
                    observed_unix: now - 5,
                },
            ),
        ]);
        let plan = plan_models_from_outcomes(
            &["fast".to_owned(), "fresh".to_owned(), "slow".to_owned()],
            &outcomes,
            now,
            100,
        );

        assert_eq!(plan.models, vec!["slow", "fresh"]);
        assert_eq!(plan.cooldown_skipped, vec!["fast"]);
    }

    #[test]
    fn fallback_attempts_are_bounded() {
        let root = test_path("bounded");
        let mut config = config(root.clone());
        config.max_attempts = 2;
        let mut attempted = Vec::new();

        let draft = resolve_fallback(
            Some(&config),
            runtime_error("connection refused"),
            "prompt",
            32,
            &mut |_, model, _, _| {
                attempted.push(model.to_owned());
                Err(NewApiFailure {
                    kind: "model_error",
                    stop_pool: false,
                })
            },
        );

        assert_eq!(attempted, vec!["slow", "fast"]);
        assert_eq!(draft.runtime_diagnostics.model_fallback_attempts, 2);
        assert!(draft.runtime_diagnostics.model_fallback_all_failed);
        let _ = fs::remove_file(root);
    }

    #[test]
    fn curl_process_args_never_contain_secret() {
        let args = curl_args(30);
        let config = curl_config(
            "https://provider.example/v1/chat/completions",
            "{\"prompt\":\"safe\"}",
            "secret-token",
        );

        assert!(config.contains("authorization: Bearer secret-token"));
        assert!(
            !args
                .iter()
                .any(|argument| argument.contains("secret-token"))
        );
        assert!(!args.iter().any(|argument| argument.contains("Bearer")));
    }

    #[test]
    fn default_allowlist_contains_all_configured_models() {
        let config = NewApiConfig::new(
            "https://provider.example/v1",
            "secret-token",
            DEFAULT_ALLOWED_MODELS
                .lines()
                .filter(|model| !model.trim().is_empty())
                .map(str::to_owned),
            PathBuf::from("unused"),
            30,
            100,
            3,
        )
        .unwrap();

        assert_eq!(config.allowed_models.len(), 122);
        assert_eq!(config.allowed_models.first().unwrap(), "01-ai/yi-large");
        assert_eq!(config.allowed_models.last().unwrap(), "z-ai/glm-5.2");
    }

    #[test]
    fn api_key_file_is_bounded_and_plain_env_wins() {
        let path = test_path("api-key");
        fs::write(&path, "  file-secret\r\n").unwrap();

        let from_file = api_key_from_sources(None, Some(path.clone())).unwrap();
        assert_eq!(from_file.trim(), "file-secret");
        assert_eq!(
            api_key_from_sources(Some("env-secret".to_owned()), Some(path.clone())).unwrap(),
            "env-secret"
        );

        fs::write(&path, vec![b'x'; MAX_API_KEY_FILE_BYTES as usize + 1]).unwrap();
        assert!(api_key_from_sources(None, Some(path.clone())).is_none());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn outcomes_path_prefers_runtime_name_and_accepts_smoke_alias() {
        assert_eq!(
            outcomes_path_from_sources(
                Some("runtime.jsonl".to_owned()),
                Some("smoke.jsonl".to_owned()),
            ),
            PathBuf::from("runtime.jsonl")
        );
        assert_eq!(
            outcomes_path_from_sources(None, Some("smoke.jsonl".to_owned())),
            PathBuf::from("smoke.jsonl")
        );
        assert_eq!(
            outcomes_path_from_sources(Some(" ".to_owned()), Some("smoke.jsonl".to_owned())),
            PathBuf::from("smoke.jsonl")
        );
    }

    fn test_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "rust-norion-newapi-{name}-{}-{}.jsonl",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ))
    }
}
