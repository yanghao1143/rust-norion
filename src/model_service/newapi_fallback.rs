use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use rust_norion::{
    DraftToken, GenerationContext, InferenceBackend, InferenceDraft, ReasoningStep,
    RuntimeDiagnostics, RuntimeError, generated_code_integrity_failure,
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
const MAX_BEHAVIOR_REPAIR_TOKENS: usize = 2048;
const MAX_BEHAVIOR_REPAIR_ATTEMPT_TIMEOUT_SECS: u64 = 20;
const MAX_BEHAVIOR_REPAIR_POOL_BUDGET_SECS: u64 = 30;
const MAX_API_KEY_FILE_BYTES: u64 = 4096;
const BROWSER_BEHAVIOR_REPAIR_MARKER: &str = "[noiron-browser-validation]";
const BROWSER_BEHAVIOR_EXCLUDE_MARKER: &str = "[noiron-browser-validation-exclude]";
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
        if self.config.is_some() && behavior_repair_requested(prompt) {
            let fallback = resolve_fallback(
                self.config.as_ref(),
                behavior_repair_fallback_trigger(),
                prompt,
                behavior_repair_max_tokens(self.generation_max_tokens),
                &mut caller,
            );
            if fallback.runtime_diagnostics.model_fallback_used {
                return fallback;
            }
            let fallback_diagnostics = fallback.runtime_diagnostics;
            let mut primary = self.primary.generate(context);
            primary.runtime_diagnostics.model_fallback_configured = true;
            primary.runtime_diagnostics.model_fallback_attempts =
                fallback_diagnostics.model_fallback_attempts;
            primary.runtime_diagnostics.model_fallback_failures =
                fallback_diagnostics.model_fallback_failures;
            primary.runtime_diagnostics.model_fallback_quarantined =
                fallback_diagnostics.model_fallback_quarantined;
            primary.runtime_diagnostics.model_fallback_cooldown_skipped =
                fallback_diagnostics.model_fallback_cooldown_skipped;
            primary.runtime_diagnostics.model_fallback_all_failed =
                fallback_diagnostics.model_fallback_all_failed;
            primary.trace.push(ReasoningStep::new(
                "newapi_behavior_repair_failed_primary_retry",
                "NewAPI behavior repair candidates failed; retried the primary backend",
                0.0,
            ));
            return primary;
        }
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

fn behavior_repair_fallback_trigger() -> InferenceDraft {
    InferenceDraft::new(
        "Runtime backend error: unavailable browser behavior repair requires model diversity",
        vec![ReasoningStep::new(
            "runtime_behavior_repair_model_diversity_error",
            "browser behavior repair requires a different model candidate",
            0.0,
        )],
    )
}

fn behavior_repair_requested(prompt: &str) -> bool {
    prompt.contains(BROWSER_BEHAVIOR_REPAIR_MARKER)
}

pub(crate) fn newapi_behavior_task_kind(prompt: &str) -> &'static str {
    let prompt = prompt.to_ascii_lowercase();
    if prompt.contains("gomoku") || prompt.contains("五子棋") {
        "gomoku"
    } else if code_task_requested(&prompt) {
        "generated_code"
    } else {
        "general"
    }
}

fn behavior_repair_max_tokens(configured: Option<usize>) -> usize {
    configured.unwrap_or(512).min(MAX_BEHAVIOR_REPAIR_TOKENS)
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
        let primary_answer = primary.answer.clone();
        let resolved = resolve_fallback(
            self.config.as_ref(),
            primary,
            prompt,
            self.generation_max_tokens.unwrap_or(512),
            &mut call_newapi_model,
        );
        let tokens = if resolved.runtime_diagnostics.model_fallback_used
            || resolved.answer != primary_answer
        {
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
        Self::new(
            base_url,
            api_key,
            allowed_models_from_env(),
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
        let allowed_models = normalized_allowed_models(models);
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

fn allowed_models_from_env() -> Vec<String> {
    let models = std::env::var(MODELS_ENV).unwrap_or_else(|_| DEFAULT_ALLOWED_MODELS.to_owned());
    normalized_allowed_models(
        models
            .split([',', '\n', '\r'])
            .filter(|model| !model.trim().is_empty())
            .map(str::to_owned),
    )
}

fn normalized_allowed_models(models: impl IntoIterator<Item = String>) -> Vec<String> {
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
    allowed_models
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
    pub(crate) behavior_repair_attempt_timeout_secs: u64,
    pub(crate) behavior_repair_pool_budget_secs: u64,
    pub(crate) last_candidate_pool_elapsed_ms: u64,
    pub(crate) last_behavior_repair_budget_exhausted: bool,
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
        telemetry.behavior_repair_attempt_timeout_secs = config
            .map(|_| MAX_BEHAVIOR_REPAIR_ATTEMPT_TIMEOUT_SECS)
            .unwrap_or_default();
        telemetry.behavior_repair_pool_budget_secs = config
            .map(|_| MAX_BEHAVIOR_REPAIR_POOL_BUDGET_SECS)
            .unwrap_or_default();
        telemetry.last_candidate_pool_elapsed_ms = 0;
        telemetry.last_behavior_repair_budget_exhausted = false;
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
    let Some(primary_failure) = retryable_primary_failure(&primary, prompt) else {
        update_telemetry(|telemetry| {
            telemetry.last_used = false;
            telemetry.last_selected_model = None;
            telemetry.last_failure_kind = None;
        });
        return primary;
    };
    let Some(config) = config else {
        if primary_failure == "output_integrity" {
            primary.answer = "Runtime backend error: generated code failed integrity validation and no fallback is configured".to_owned();
            primary.tokens = answer_tokens(&primary.answer);
            primary.trace.push(ReasoningStep::new(
                "runtime_output_integrity_error",
                "generated code failed integrity validation and no fallback is configured",
                0.0,
            ));
        }
        return primary;
    };

    update_telemetry(|telemetry| {
        telemetry.primary_failures = telemetry.primary_failures.saturating_add(1);
        telemetry.last_used = false;
        telemetry.last_selected_model = None;
        telemetry.last_failure_kind = Some(primary_failure.to_owned());
    });
    let now = unix_now();
    let plan = plan_models_for_prompt(config, now, prompt);
    let pool_started = Instant::now();
    let behavior_repair = behavior_repair_requested(prompt);
    update_telemetry(|telemetry| {
        telemetry.cooldown_skipped = telemetry
            .cooldown_skipped
            .saturating_add(plan.cooldown_skipped.len());
    });

    let mut attempts = 0usize;
    let mut failures = 0usize;
    let mut quarantined = 0usize;
    let mut behavior_repair_budget_exhausted = false;
    for model in plan.models.iter().take(config.max_attempts) {
        let elapsed_ms = elapsed_millis(pool_started);
        let Some(call_timeout_secs) =
            fallback_call_timeout_secs(config.timeout_secs, behavior_repair, elapsed_ms)
        else {
            behavior_repair_budget_exhausted = true;
            break;
        };
        let bounded_config = (call_timeout_secs != config.timeout_secs).then(|| {
            let mut config = config.clone();
            config.timeout_secs = call_timeout_secs;
            config
        });
        let call_config = bounded_config.as_ref().unwrap_or(config);
        attempts = attempts.saturating_add(1);
        update_telemetry(|telemetry| {
            telemetry.fallback_attempts = telemetry.fallback_attempts.saturating_add(1);
        });
        let result = caller(call_config, model, prompt, max_tokens.max(1)).and_then(|call| {
            if generated_code_integrity_failure(prompt, &call.answer).is_some()
                || behavior_repair_integrity_failure(prompt, &call.answer).is_some()
            {
                Err(NewApiFailure {
                    kind: "output_integrity",
                    stop_pool: false,
                })
            } else {
                Ok(call)
            }
        });
        match result {
            Ok(call) => {
                let pool_elapsed_ms = elapsed_millis(pool_started);
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
                    telemetry.last_candidate_pool_elapsed_ms = pool_elapsed_ms;
                    telemetry.last_behavior_repair_budget_exhausted = false;
                });
                return fallback_success_draft(
                    call,
                    attempts,
                    failures,
                    quarantined,
                    plan.cooldown_skipped.len(),
                    pool_elapsed_ms,
                );
            }
            Err(failure) => {
                failures = failures.saturating_add(1);
                let quarantines_model = failure_quarantines_model(failure.kind);
                let persistence_failed = quarantines_model
                    && persist_outcome(
                        &config.outcomes_path,
                        model,
                        false,
                        Some(failure.kind),
                        None,
                        now,
                    )
                    .is_err();
                quarantined = quarantined
                    .saturating_add(usize::from(quarantines_model && !persistence_failed));
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

    let pool_elapsed_ms = elapsed_millis(pool_started);
    behavior_repair_budget_exhausted |= behavior_repair
        && pool_elapsed_ms >= MAX_BEHAVIOR_REPAIR_POOL_BUDGET_SECS.saturating_mul(1_000);
    update_telemetry(|telemetry| {
        telemetry.last_candidate_pool_elapsed_ms = pool_elapsed_ms;
        telemetry.last_behavior_repair_budget_exhausted = behavior_repair_budget_exhausted;
    });

    primary.trace.push(ReasoningStep::new(
        "newapi_fallback_failed",
        format!(
            "primary_failure={} attempts={} failures={} cooldown_skipped={} pool_elapsed_ms={} behavior_repair_budget_exhausted={} all_failed=true",
            primary_failure,
            attempts,
            failures,
            plan.cooldown_skipped.len(),
            pool_elapsed_ms,
            behavior_repair_budget_exhausted,
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
    if primary_failure == "output_integrity" {
        primary.answer = "Runtime backend error: generated code failed integrity validation after all fallback attempts".to_owned();
        primary.tokens = answer_tokens(&primary.answer);
        primary.trace.push(ReasoningStep::new(
            "runtime_output_integrity_error",
            "generated code failed integrity validation after all fallback attempts",
            0.0,
        ));
    }
    primary
}

fn failure_quarantines_model(kind: &str) -> bool {
    matches!(
        kind,
        "timeout" | "rate_limit" | "model_error" | "model_access" | "response_shape"
    )
}

fn fallback_call_timeout_secs(
    configured_timeout_secs: u64,
    behavior_repair: bool,
    elapsed_ms: u64,
) -> Option<u64> {
    let configured_timeout_secs = configured_timeout_secs.max(1);
    if !behavior_repair {
        return Some(configured_timeout_secs);
    }
    let budget_ms = MAX_BEHAVIOR_REPAIR_POOL_BUDGET_SECS.saturating_mul(1_000);
    let remaining_ms = budget_ms.saturating_sub(elapsed_ms);
    let remaining_secs = remaining_ms / 1_000;
    if remaining_secs == 0 {
        return None;
    }
    Some(
        configured_timeout_secs
            .min(MAX_BEHAVIOR_REPAIR_ATTEMPT_TIMEOUT_SECS)
            .min(remaining_secs),
    )
}

fn behavior_repair_integrity_failure(prompt: &str, answer: &str) -> Option<&'static str> {
    if !behavior_repair_requested(prompt) {
        return None;
    }
    let lower = answer.to_ascii_lowercase();
    if lower.contains("<?php") || lower.contains("<%") {
        return Some("server_template_in_browser_artifact");
    }
    None
}

fn fallback_success_draft(
    call: NewApiCall,
    attempts: usize,
    failures: usize,
    quarantined: usize,
    cooldown_skipped: usize,
    pool_elapsed_ms: u64,
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
                "selected_model={} attempts={} failures={} cooldown_skipped={} elapsed_ms={} pool_elapsed_ms={}",
                call.model,
                attempts,
                failures,
                cooldown_skipped,
                call.elapsed_ms,
                pool_elapsed_ms,
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

fn retryable_primary_failure(draft: &InferenceDraft, prompt: &str) -> Option<&'static str> {
    if generated_code_integrity_failure(prompt, &draft.answer).is_some() {
        return Some("output_integrity");
    }
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

fn plan_models_for_prompt(config: &NewApiConfig, now: u64, prompt: &str) -> ModelPlan {
    let outcomes = read_outcomes(&config.outcomes_path);
    let task_outcomes =
        read_task_outcomes(&config.outcomes_path, newapi_behavior_task_kind(prompt));
    plan_models_from_outcomes_and_task_outcomes(
        &config.allowed_models,
        &outcomes,
        &task_outcomes,
        now,
        config.cooldown_secs,
        prompt,
    )
}

fn plan_models_from_outcomes(
    allowed_models: &[String],
    outcomes: &HashMap<String, ModelOutcome>,
    now: u64,
    cooldown_secs: u64,
) -> ModelPlan {
    plan_models_from_outcomes_for_prompt(allowed_models, outcomes, now, cooldown_secs, "")
}

fn plan_models_from_outcomes_for_prompt(
    allowed_models: &[String],
    outcomes: &HashMap<String, ModelOutcome>,
    now: u64,
    cooldown_secs: u64,
    prompt: &str,
) -> ModelPlan {
    plan_models_from_outcomes_and_task_outcomes(
        allowed_models,
        outcomes,
        &HashMap::new(),
        now,
        cooldown_secs,
        prompt,
    )
}

fn plan_models_from_outcomes_and_task_outcomes(
    allowed_models: &[String],
    outcomes: &HashMap<String, ModelOutcome>,
    task_outcomes: &HashMap<String, ModelOutcome>,
    now: u64,
    cooldown_secs: u64,
    prompt: &str,
) -> ModelPlan {
    let mut ranked = Vec::new();
    let mut cooldown_skipped = Vec::new();
    let excluded = excluded_models(prompt);
    for (index, model) in allowed_models.iter().enumerate() {
        if excluded.iter().any(|excluded| excluded == model) {
            continue;
        }
        let task_rank = model_task_rank(model, prompt);
        let task_outcome = task_outcomes.get(model);
        if matches!(
            task_outcome,
            Some(outcome)
                if !outcome.ok
                    && outcome.observed_unix.saturating_add(cooldown_secs) > now
        ) {
            cooldown_skipped.push(model.clone());
            continue;
        }
        let task_evidence_rank = u8::from(!matches!(task_outcome, Some(outcome) if outcome.ok));
        match outcomes.get(model) {
            Some(outcome)
                if !outcome.ok && outcome.observed_unix.saturating_add(cooldown_secs) > now =>
            {
                cooldown_skipped.push(model.clone());
            }
            Some(outcome) if outcome.ok => ranked.push((
                task_evidence_rank,
                task_rank,
                0u8,
                outcome.elapsed_ms.unwrap_or(u64::MAX),
                index,
                model.clone(),
            )),
            _ => ranked.push((
                task_evidence_rank,
                task_rank,
                1u8,
                u64::MAX,
                index,
                model.clone(),
            )),
        }
    }
    ranked.sort_by_key(|entry| (entry.0, entry.1, entry.2, entry.3, entry.4));
    ModelPlan {
        models: ranked.into_iter().map(|entry| entry.5).collect(),
        cooldown_skipped,
    }
}

fn excluded_models(prompt: &str) -> Vec<&str> {
    prompt
        .lines()
        .filter_map(|line| {
            line.trim()
                .strip_prefix(BROWSER_BEHAVIOR_EXCLUDE_MARKER)
                .map(str::trim)
                .filter(|model| valid_model_id(model))
        })
        .collect()
}

fn model_task_rank(model: &str, prompt: &str) -> u8 {
    let model = model.to_ascii_lowercase();
    if [
        "embed",
        "retriever",
        "guard",
        "safety",
        "reward",
        "parse",
        "detector",
        "pii",
        "clip",
        "translate",
        "deplot",
        "calibration",
        "diffusion",
    ]
    .iter()
    .any(|marker| model.contains(marker))
    {
        return 4;
    }

    let code_task = code_task_requested(prompt);
    let code_model = [
        "code",
        "coder",
        "codestral",
        "starcoder",
        "qwen",
        "deepseek",
        "glm",
        "gpt-oss",
        "kimi",
        "minimax",
        "mistral-small-4",
    ]
    .iter()
    .any(|marker| model.contains(marker));
    if code_task && model.contains("mistral-small-4") {
        return 0;
    }
    if code_task && code_model {
        return 1;
    }
    if code_task {
        return 2;
    }
    if code_model || model.contains("instruct") || model.contains("chat") {
        return 0;
    }
    1
}

fn code_task_requested(prompt: &str) -> bool {
    let prompt = prompt.to_ascii_lowercase();
    behavior_repair_requested(&prompt)
        || [
            "code",
            "html",
            "javascript",
            "typescript",
            "rust",
            "python",
            "gomoku",
            "五子棋",
            "代码",
            "网页",
        ]
        .iter()
        .any(|marker| prompt.contains(marker))
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
        return Err(curl_exit_failure(output.status.code()));
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

fn curl_exit_failure(code: Option<i32>) -> NewApiFailure {
    if code == Some(28) {
        NewApiFailure {
            kind: "timeout",
            stop_pool: false,
        }
    } else {
        NewApiFailure {
            kind: "transport",
            stop_pool: true,
        }
    }
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
        401 => NewApiFailure {
            kind: "auth",
            stop_pool: true,
        },
        403 => NewApiFailure {
            kind: "model_access",
            stop_pool: false,
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
    persist_scoped_outcome(path, model, None, ok, reason, elapsed_ms, observed_unix)
}

pub(crate) fn persist_newapi_behavior_outcome_from_env(
    model: &str,
    task_kind: &str,
    ok: bool,
) -> std::io::Result<bool> {
    if !newapi_outcome_env_configured()
        || !matches!(task_kind, "gomoku" | "generated_code")
        || !allowed_models_from_env()
            .iter()
            .any(|allowed| allowed == model)
    {
        return Ok(false);
    }
    let path = outcomes_path_from_sources(
        std::env::var(OUTCOMES_PATH_ENV).ok(),
        std::env::var(MODEL_OUTCOMES_PATH_ENV).ok(),
    );
    persist_scoped_outcome(
        &path,
        model,
        Some(task_kind),
        ok,
        (!ok).then_some("behavior_contract_error"),
        None,
        unix_now(),
    )?;
    Ok(true)
}

fn newapi_outcome_env_configured() -> bool {
    std::env::var(BASE_URL_ENV)
        .ok()
        .is_some_and(|value| !value.trim().is_empty())
        && (std::env::var(API_KEY_ENV)
            .ok()
            .is_some_and(|value| !value.trim().is_empty())
            || std::env::var(API_KEY_FILE_ENV)
                .ok()
                .is_some_and(|value| !value.trim().is_empty()))
}

fn persist_scoped_outcome(
    path: &Path,
    model: &str,
    task_kind: Option<&str>,
    ok: bool,
    reason: Option<&str>,
    elapsed_ms: Option<u64>,
    observed_unix: u64,
) -> std::io::Result<()> {
    ensure_parent_dir(path)?;
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(
        file,
        "{{\"observed_unix\":{},\"task_kind\":{},\"model\":{},\"ok\":{},\"reason\":{},\"elapsed_ms\":{}}}",
        observed_unix,
        task_kind
            .map(service_json_string)
            .unwrap_or_else(|| "null".to_owned()),
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
    parse_outcomes(&text)
}

fn read_task_outcomes(path: &Path, task_kind: &str) -> HashMap<String, ModelOutcome> {
    let Ok(text) = fs::read_to_string(path) else {
        return HashMap::new();
    };
    parse_task_outcomes(&text, task_kind)
}

fn parse_outcomes(text: &str) -> HashMap<String, ModelOutcome> {
    text.lines()
        .filter_map(|line| {
            if json_string_field(line, "task_kind")
                .is_some_and(|task_kind| task_kind != "availability")
            {
                return None;
            }
            let model = json_string_field(line, "model")?;
            let reason = json_string_field(line, "reason");
            if matches!(
                reason.as_deref(),
                Some("contract_error" | "output_integrity")
            ) {
                return None;
            }
            Some((
                model,
                ModelOutcome {
                    ok: json_bool_field(line, "ok").unwrap_or(false),
                    reason,
                    elapsed_ms: json_u64_field(line, "elapsed_ms"),
                    observed_unix: json_u64_field(line, "observed_unix").unwrap_or(0),
                },
            ))
        })
        .collect()
}

fn parse_task_outcomes(text: &str, wanted_task_kind: &str) -> HashMap<String, ModelOutcome> {
    text.lines()
        .filter_map(|line| {
            if json_string_field(line, "task_kind").as_deref() != Some(wanted_task_kind) {
                return None;
            }
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
    fn malformed_primary_html_uses_complete_fallback() {
        let root = test_path("output-integrity-primary");
        let config = config(root.clone());
        let mut calls = 0;
        let draft = resolve_fallback(
            Some(&config),
            InferenceDraft::new(
                "<!doctype html><html><body><script>const board = [];",
                Vec::new(),
            ),
            "生成一个完整的单文件 HTML 五子棋",
            512,
            &mut |_, model, _, _| {
                calls += 1;
                Ok(NewApiCall {
                    model: model.to_owned(),
                    answer:
                        "<!doctype html><html><body><script>const board=[];</script></body></html>"
                            .to_owned(),
                    elapsed_ms: 12,
                })
            },
        );

        assert_eq!(calls, 1);
        assert!(draft.runtime_diagnostics.model_fallback_used);
        assert!(draft.answer.ends_with("</html>"));
        let _ = fs::remove_file(root);
    }

    #[test]
    fn malformed_code_without_config_returns_validation_error() {
        let draft = resolve_fallback(
            None,
            InferenceDraft::new("```html\n<html>", Vec::new()),
            "生成一个完整 HTML 页面",
            512,
            &mut |_, _, _, _| unreachable!(),
        );

        assert!(draft.answer.starts_with("Runtime backend error:"));
        assert!(!draft.runtime_diagnostics.model_fallback_configured);
        assert!(
            draft
                .trace
                .iter()
                .any(|step| step.label == "runtime_output_integrity_error")
        );
    }

    #[test]
    fn malformed_fallback_is_skipped_for_next_complete_model() {
        let root = test_path("output-integrity-candidates");
        let config = config(root.clone());
        let mut attempted = Vec::new();
        let draft = resolve_fallback(
            Some(&config),
            InferenceDraft::new(
                "<!doctype html><html><body><script>const board = [];",
                Vec::new(),
            ),
            "生成一个完整的单文件 HTML 五子棋",
            512,
            &mut |_, model, _, _| {
                attempted.push(model.to_owned());
                Ok(NewApiCall {
                    model: model.to_owned(),
                    answer: if attempted.len() == 1 {
                        "<!doctype html><html><body><script>const board=[];".to_owned()
                    } else {
                        "<!doctype html><html><body><script>const board=[];</script></body></html>"
                            .to_owned()
                    },
                    elapsed_ms: 12,
                })
            },
        );

        assert_eq!(attempted, vec!["slow", "fast"]);
        assert_eq!(
            draft
                .runtime_diagnostics
                .model_fallback_selected_model
                .as_deref(),
            Some("fast")
        );
        assert_eq!(draft.runtime_diagnostics.model_fallback_failures, 1);
        assert_eq!(draft.runtime_diagnostics.model_fallback_quarantined, 0);
        let _ = fs::remove_file(root);
    }

    #[test]
    fn all_malformed_code_candidates_return_validation_error() {
        let root = test_path("output-integrity-all-failed");
        let mut config = config(root.clone());
        config.max_attempts = 2;
        let draft = resolve_fallback(
            Some(&config),
            InferenceDraft::new("```html\n<html>", Vec::new()),
            "生成一个完整 HTML 页面",
            512,
            &mut |_, model, _, _| {
                Ok(NewApiCall {
                    model: model.to_owned(),
                    answer: "```html\n<html>".to_owned(),
                    elapsed_ms: 12,
                })
            },
        );

        assert!(draft.answer.starts_with("Runtime backend error:"));
        assert!(draft.runtime_diagnostics.model_fallback_all_failed);
        assert_eq!(draft.runtime_diagnostics.model_fallback_quarantined, 0);
        assert!(
            draft
                .trace
                .iter()
                .any(|step| step.label == "runtime_output_integrity_error")
        );
        let _ = fs::remove_file(root);
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
    fn code_repair_prefers_capable_generator_over_faster_safety_model() {
        let now = 1_800_000_000;
        let models = [
            "nvidia/nemotron-content-safety-reasoning-4b".to_owned(),
            "meta/llama-3.1-8b-instruct".to_owned(),
            "qwen/qwen3.5-397b-a17b".to_owned(),
            "mistralai/mistral-small-4-119b-2603".to_owned(),
        ];
        let outcomes = HashMap::from([
            (
                models[0].clone(),
                ModelOutcome {
                    ok: true,
                    reason: None,
                    elapsed_ms: Some(100),
                    observed_unix: now,
                },
            ),
            (
                models[1].clone(),
                ModelOutcome {
                    ok: true,
                    reason: None,
                    elapsed_ms: Some(200),
                    observed_unix: now,
                },
            ),
            (
                models[2].clone(),
                ModelOutcome {
                    ok: true,
                    reason: None,
                    elapsed_ms: Some(5_000),
                    observed_unix: now,
                },
            ),
            (
                models[3].clone(),
                ModelOutcome {
                    ok: true,
                    reason: None,
                    elapsed_ms: Some(300),
                    observed_unix: now,
                },
            ),
        ]);

        let plan = plan_models_from_outcomes_for_prompt(
            &models,
            &outcomes,
            now,
            100,
            "[noiron-browser-validation] repair gomoku html",
        );

        assert_eq!(
            plan.models,
            vec![
                "mistralai/mistral-small-4-119b-2603",
                "qwen/qwen3.5-397b-a17b",
                "meta/llama-3.1-8b-instruct",
                "nvidia/nemotron-content-safety-reasoning-4b"
            ]
        );
    }

    #[test]
    fn task_behavior_failure_only_cools_same_task_model() {
        let now = 1_800_000_000;
        let models = ["fast".to_owned(), "slow".to_owned()];
        let availability = HashMap::from([
            (
                "fast".to_owned(),
                ModelOutcome {
                    ok: true,
                    reason: None,
                    elapsed_ms: Some(10),
                    observed_unix: now,
                },
            ),
            (
                "slow".to_owned(),
                ModelOutcome {
                    ok: true,
                    reason: None,
                    elapsed_ms: Some(20),
                    observed_unix: now,
                },
            ),
        ]);
        let gomoku = HashMap::from([(
            "fast".to_owned(),
            ModelOutcome {
                ok: false,
                reason: Some("behavior_contract_error".to_owned()),
                elapsed_ms: None,
                observed_unix: now,
            },
        )]);

        let gomoku_plan = plan_models_from_outcomes_and_task_outcomes(
            &models,
            &availability,
            &gomoku,
            now,
            100,
            "repair gomoku html",
        );
        let general_plan = plan_models_from_outcomes(&models, &availability, now, 100);

        assert_eq!(gomoku_plan.models, vec!["slow"]);
        assert_eq!(gomoku_plan.cooldown_skipped, vec!["fast"]);
        assert_eq!(general_plan.models, vec!["fast", "slow"]);
    }

    #[test]
    fn proven_task_model_precedes_faster_unproven_model() {
        let now = 1_800_000_000;
        let models = [
            "mistralai/mistral-small-4-119b-2603".to_owned(),
            "meta/llama-3.1-8b-instruct".to_owned(),
        ];
        let availability = HashMap::from([
            (
                models[0].clone(),
                ModelOutcome {
                    ok: true,
                    reason: None,
                    elapsed_ms: Some(10),
                    observed_unix: now,
                },
            ),
            (
                models[1].clone(),
                ModelOutcome {
                    ok: true,
                    reason: None,
                    elapsed_ms: Some(20),
                    observed_unix: now,
                },
            ),
        ]);
        let gomoku = HashMap::from([(
            models[1].clone(),
            ModelOutcome {
                ok: true,
                reason: None,
                elapsed_ms: None,
                observed_unix: now,
            },
        )]);

        let plan = plan_models_from_outcomes_and_task_outcomes(
            &models,
            &availability,
            &gomoku,
            now,
            100,
            "repair gomoku html",
        );

        assert_eq!(plan.models, vec![models[1].clone(), models[0].clone()]);
    }

    #[test]
    fn behavior_task_kind_is_server_derived() {
        assert_eq!(newapi_behavior_task_kind("写一个五子棋 HTML"), "gomoku");
        assert_eq!(
            newapi_behavior_task_kind("generate a Rust function"),
            "generated_code"
        );
        assert_eq!(newapi_behavior_task_kind("summarize this"), "general");
    }

    #[test]
    fn browser_repair_generation_budget_is_bounded() {
        assert_eq!(behavior_repair_max_tokens(None), 512);
        assert_eq!(behavior_repair_max_tokens(Some(1024)), 1024);
        assert_eq!(behavior_repair_max_tokens(Some(4096)), 2048);
    }

    #[test]
    fn browser_repair_candidate_timeout_and_pool_budget_are_bounded() {
        assert_eq!(fallback_call_timeout_secs(45, true, 0), Some(20));
        assert_eq!(fallback_call_timeout_secs(45, true, 19_250), Some(10));
        assert_eq!(fallback_call_timeout_secs(45, true, 29_250), None);
        assert_eq!(fallback_call_timeout_secs(45, true, 30_000), None);
        assert_eq!(fallback_call_timeout_secs(45, false, 99_000), Some(45));
    }

    #[test]
    fn browser_repair_passes_bounded_timeout_to_candidate_caller() {
        let root = test_path("browser-repair-timeout");
        let mut config = config(root.clone());
        config.timeout_secs = 45;
        config.max_attempts = 1;
        let mut timeouts = Vec::new();

        let draft = resolve_fallback(
            Some(&config),
            behavior_repair_fallback_trigger(),
            "[noiron-browser-validation] repair gomoku html",
            512,
            &mut |config, model, _, _| {
                timeouts.push(config.timeout_secs);
                Ok(NewApiCall {
                    model: model.to_owned(),
                    answer: "<!doctype html><html><body></body></html>".to_owned(),
                    elapsed_ms: 12,
                })
            },
        );

        assert_eq!(timeouts, vec![20]);
        assert!(draft.runtime_diagnostics.model_fallback_used);
        assert!(
            draft
                .trace
                .iter()
                .any(|step| step.content.contains("pool_elapsed_ms="))
        );
        let _ = fs::remove_file(root);
    }

    #[test]
    fn behavior_repair_excludes_previous_failed_model() {
        let models = [
            "qwen/qwen3.5-397b-a17b".to_owned(),
            "deepseek-ai/deepseek-v4-flash".to_owned(),
        ];
        let plan = plan_models_from_outcomes_for_prompt(
            &models,
            &HashMap::new(),
            1_800_000_000,
            100,
            "[noiron-browser-validation]\n[noiron-browser-validation-exclude] qwen/qwen3.5-397b-a17b\nrepair gomoku",
        );

        assert_eq!(plan.models, vec!["deepseek-ai/deepseek-v4-flash"]);
    }

    #[test]
    fn browser_behavior_repair_marker_routes_to_newapi_candidate() {
        let root = test_path("behavior-repair-model-diversity");
        let config = config(root.clone());
        let mut attempted = Vec::new();
        let draft = resolve_fallback(
            Some(&config),
            behavior_repair_fallback_trigger(),
            "[noiron-browser-validation] repair gomoku",
            512,
            &mut |_, model, _, _| {
                attempted.push(model.to_owned());
                Ok(NewApiCall {
                    model: model.to_owned(),
                    answer: "<!doctype html><html><body>fixed</body></html>".to_owned(),
                    elapsed_ms: 12,
                })
            },
        );

        assert_eq!(attempted, vec!["slow"]);
        assert!(draft.runtime_diagnostics.model_fallback_used);
        assert_eq!(
            draft.runtime_diagnostics.selected_adapter.as_deref(),
            Some("newapi-fallback")
        );
        let _ = fs::remove_file(root);
    }

    #[test]
    fn browser_repair_skips_server_template_candidate() {
        let root = test_path("behavior-repair-server-template");
        let config = config(root.clone());
        let mut attempted = Vec::new();
        let draft = resolve_fallback(
            Some(&config),
            behavior_repair_fallback_trigger(),
            "[noiron-browser-validation] repair gomoku",
            512,
            &mut |_, model, _, _| {
                attempted.push(model.to_owned());
                Ok(NewApiCall {
                    model: model.to_owned(),
                    answer: if attempted.len() == 1 {
                        "<!doctype html><html><body><?php echo 'bad'; ?></body></html>".to_owned()
                    } else {
                        "<!doctype html><html><body><script>const board=[];</script></body></html>"
                            .to_owned()
                    },
                    elapsed_ms: 12,
                })
            },
        );

        assert_eq!(attempted, vec!["slow", "fast"]);
        assert_eq!(
            draft
                .runtime_diagnostics
                .model_fallback_selected_model
                .as_deref(),
            Some("fast")
        );
        let _ = fs::remove_file(root);
    }

    #[test]
    fn browser_behavior_repair_marker_is_detected_inside_runtime_prompt() {
        assert!(behavior_repair_requested(
            "Conversation transcript:\nuser: [noiron-browser-validation] repair gomoku\nassistant:"
        ));
        assert!(!behavior_repair_requested("ordinary generation prompt"));
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
    fn auth_failure_does_not_quarantine_a_model() {
        let root = test_path("auth-global-failure");
        let config = config(root.clone());
        let draft = resolve_fallback(
            Some(&config),
            behavior_repair_fallback_trigger(),
            "[noiron-browser-validation] repair gomoku",
            512,
            &mut |_, _, _, _| {
                Err(NewApiFailure {
                    kind: "auth",
                    stop_pool: true,
                })
            },
        );

        assert_eq!(draft.runtime_diagnostics.model_fallback_failures, 1);
        assert_eq!(draft.runtime_diagnostics.model_fallback_quarantined, 0);
        assert!(!root.exists());
    }

    #[test]
    fn forbidden_model_is_quarantined_without_stopping_pool() {
        let root = test_path("model-access-failure");
        let config = config(root.clone());
        let mut attempted = Vec::new();
        let draft = resolve_fallback(
            Some(&config),
            behavior_repair_fallback_trigger(),
            "[noiron-browser-validation] repair gomoku",
            512,
            &mut |_, model, _, _| {
                attempted.push(model.to_owned());
                if attempted.len() == 1 {
                    Err(http_failure(403))
                } else {
                    Ok(NewApiCall {
                        model: model.to_owned(),
                        answer: "<!doctype html><html><body>fixed</body></html>".to_owned(),
                        elapsed_ms: 12,
                    })
                }
            },
        );

        assert_eq!(attempted, vec!["slow", "fast"]);
        assert_eq!(draft.runtime_diagnostics.model_fallback_quarantined, 1);
        assert_eq!(
            parse_outcomes(&fs::read_to_string(&root).unwrap())["slow"]
                .reason
                .as_deref(),
            Some("model_access")
        );
        let _ = fs::remove_file(root);
    }

    #[test]
    fn http_auth_failure_scope_is_explicit() {
        assert_eq!(
            http_failure(401),
            NewApiFailure {
                kind: "auth",
                stop_pool: true,
            }
        );
        assert_eq!(
            http_failure(403),
            NewApiFailure {
                kind: "model_access",
                stop_pool: false,
            }
        );
    }

    #[test]
    fn curl_timeout_does_not_stop_candidate_pool() {
        assert_eq!(
            curl_exit_failure(Some(28)),
            NewApiFailure {
                kind: "timeout",
                stop_pool: false,
            }
        );
        assert_eq!(
            curl_exit_failure(Some(7)),
            NewApiFailure {
                kind: "transport",
                stop_pool: true,
            }
        );
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
    fn helper_contract_failure_does_not_quarantine_runtime_model() {
        let text = [
            r#"{"observed_unix":10,"task_kind":"availability","model":"usable","ok":true,"reason":null,"elapsed_ms":50}"#,
            r#"{"observed_unix":20,"task_kind":"review","model":"usable","ok":false,"reason":"contract_error","elapsed_ms":60}"#,
        ]
        .join("\n");

        let outcomes = parse_outcomes(&text);

        assert!(outcomes.get("usable").unwrap().ok);
        assert_eq!(outcomes.get("usable").unwrap().elapsed_ms, Some(50));
        let review = parse_task_outcomes(&text, "review");
        assert!(!review.get("usable").unwrap().ok);
        assert_eq!(
            review.get("usable").unwrap().reason.as_deref(),
            Some("contract_error")
        );
    }

    #[test]
    fn scoped_behavior_outcome_roundtrips_without_polluting_availability() {
        let root = test_path("behavior-outcome-scope");
        persist_scoped_outcome(
            &root,
            "model-a",
            Some("gomoku"),
            false,
            Some("behavior_contract_error"),
            None,
            42,
        )
        .unwrap();
        let text = fs::read_to_string(&root).unwrap();

        assert!(parse_outcomes(&text).is_empty());
        let gomoku = parse_task_outcomes(&text, "gomoku");
        assert!(!gomoku["model-a"].ok);
        assert_eq!(gomoku["model-a"].observed_unix, 42);
        let _ = fs::remove_file(root);
    }

    #[test]
    fn apple_runtime_model_is_not_in_newapi_allowlist() {
        let models = normalized_allowed_models(
            DEFAULT_ALLOWED_MODELS
                .split([',', '\n', '\r'])
                .map(str::to_owned),
        );
        assert!(
            !models
                .iter()
                .any(|model| model == "Qwen3.6-14B-A3B-FableVibes-Q4_K_M.gguf")
        );
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
