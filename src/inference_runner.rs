use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;

use rust_norion::{
    DraftToken, GenerationContext, InferenceBackend, InferenceDraft, InferenceRequest,
    NoironEngine, ReasoningStep, RuntimeError, TaskProfile, TenantScope, append_trace_jsonl,
    append_trace_jsonl_with_case,
};

use crate::model_service::http::split_http_head_body;
use crate::model_service::json::{json_string_field, service_json_string};
use crate::model_service::types::TimedOutcome;

const MODEL_POOL_CALL_URL_ENV: &str = "NORION_MODEL_POOL_CALL_URL";
const MODEL_POOL_CALL_DEFAULT_PATH: &str = "/v1/model-pool/call";
const MODEL_POOL_CALL_TIMEOUT: Duration = Duration::from_secs(300);
const MODEL_POOL_ROUTE_PLAN_URL_ENV: &str = "NORION_MODEL_POOL_ROUTE_PLAN_URL";
const MODEL_POOL_ROUTE_PLAN_DEFAULT_PATH: &str = "/v1/model-pool/route-plan";
const MODEL_POOL_ROUTE_PLAN_TIMEOUT: Duration = Duration::from_millis(600);

pub(crate) fn run_timed_inference<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    prompt: String,
    profile: TaskProfile,
    trace_path: Option<&PathBuf>,
    case_name: Option<&str>,
) -> std::io::Result<TimedOutcome> {
    run_timed_inference_with_options(
        engine, backend, prompt, profile, None, trace_path, case_name,
    )
}

pub(crate) fn run_timed_inference_with_options<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    prompt: String,
    profile: TaskProfile,
    max_tokens: Option<usize>,
    trace_path: Option<&PathBuf>,
    case_name: Option<&str>,
) -> std::io::Result<TimedOutcome> {
    run_timed_inference_with_scope_options(
        engine, backend, prompt, profile, max_tokens, None, trace_path, case_name,
    )
}

pub(crate) fn run_timed_inference_with_scope_options<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    prompt: String,
    profile: TaskProfile,
    max_tokens: Option<usize>,
    tenant_scope: Option<TenantScope>,
    trace_path: Option<&PathBuf>,
    case_name: Option<&str>,
) -> std::io::Result<TimedOutcome> {
    run_timed_inference_with_scope_and_route_plan_url_options(
        engine,
        backend,
        prompt,
        profile,
        max_tokens,
        tenant_scope,
        trace_path,
        case_name,
        None,
        None,
    )
}

#[cfg(test)]
pub(crate) fn run_timed_inference_with_model_pool_urls<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    prompt: String,
    profile: TaskProfile,
    max_tokens: Option<usize>,
    trace_path: Option<&PathBuf>,
    case_name: Option<&str>,
    route_plan_url: &str,
    call_url: &str,
) -> std::io::Result<TimedOutcome> {
    run_timed_inference_with_scope_and_route_plan_url_options(
        engine,
        backend,
        prompt,
        profile,
        max_tokens,
        None,
        trace_path,
        case_name,
        Some(route_plan_url),
        Some(call_url),
    )
}

fn run_timed_inference_with_scope_and_route_plan_url_options<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    prompt: String,
    profile: TaskProfile,
    max_tokens: Option<usize>,
    tenant_scope: Option<TenantScope>,
    trace_path: Option<&PathBuf>,
    case_name: Option<&str>,
    route_plan_url: Option<&str>,
    call_url: Option<&str>,
) -> std::io::Result<TimedOutcome> {
    let started = Instant::now();
    let request = if let Some(route_plan_url) = route_plan_url {
        inference_request_with_options_and_route_plan_url(
            prompt.clone(),
            profile,
            max_tokens,
            tenant_scope,
            Some(route_plan_url),
        )
    } else {
        inference_request_with_options(prompt.clone(), profile, max_tokens, tenant_scope)
    };
    let call_url_env = if call_url.is_none() {
        std::env::var(MODEL_POOL_CALL_URL_ENV).ok()
    } else {
        None
    };
    let call_url = call_url
        .or(call_url_env.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let outcome = if let Some(call_url) = call_url {
        let mut model_pool_backend = ModelPoolCallBackend {
            fallback: backend,
            call_url,
            configured_max_tokens: max_tokens,
        };
        engine.infer(request, &mut model_pool_backend)
    } else {
        engine.infer(request, backend)
    };
    let elapsed_ms = started.elapsed().as_millis();

    if let Some(trace_path) = trace_path {
        if let Some(case_name) = case_name {
            append_trace_jsonl_with_case(
                trace_path, case_name, &prompt, profile, elapsed_ms, &outcome,
            )?;
        } else {
            append_trace_jsonl(trace_path, &prompt, profile, elapsed_ms, &outcome)?;
        }
    }

    Ok(TimedOutcome {
        outcome,
        elapsed_ms,
    })
}

struct ModelPoolCallBackend<'a, B: InferenceBackend> {
    fallback: &'a mut B,
    call_url: &'a str,
    configured_max_tokens: Option<usize>,
}

impl<B: InferenceBackend> InferenceBackend for ModelPoolCallBackend<'_, B> {
    fn configure_generation(&mut self, max_tokens: Option<usize>) {
        self.configured_max_tokens = max_tokens;
        self.fallback.configure_generation(max_tokens);
    }

    fn configure_runtime_endpoint_override(
        &mut self,
        base_url: Option<&str>,
    ) -> Result<bool, String> {
        self.fallback.configure_runtime_endpoint_override(base_url)
    }

    fn runtime_endpoint_override_active(&self) -> Option<&str> {
        self.fallback.runtime_endpoint_override_active()
    }

    fn runtime_native_context_window(&self) -> Option<usize> {
        self.fallback.runtime_native_context_window()
    }

    fn embed_text(&mut self, text: &str) -> Option<Vec<f32>> {
        self.fallback.embed_text(text)
    }

    fn generate(&mut self, context: GenerationContext<'_>) -> InferenceDraft {
        match fetch_model_pool_call_answer(
            self.call_url,
            context.prompt,
            self.configured_max_tokens,
        ) {
            Ok(answer) => model_pool_call_draft(answer, false),
            Err(_) => self.fallback.generate(context),
        }
    }

    fn generate_stream_checked(
        &mut self,
        context: GenerationContext<'_>,
        on_token: &mut dyn FnMut(&DraftToken) -> Result<(), RuntimeError>,
    ) -> InferenceDraft {
        match fetch_model_pool_call_answer(
            self.call_url,
            context.prompt,
            self.configured_max_tokens,
        ) {
            Ok(answer) => {
                let draft = model_pool_call_draft(answer, true);
                if let Some(token) = draft.tokens.first()
                    && let Err(error) = on_token(token)
                {
                    return InferenceDraft::new(
                        format!("Runtime backend error: {}", error.message()),
                        vec![ReasoningStep::new(
                            "runtime_stream_observer_error",
                            error.message(),
                            0.0,
                        )],
                    );
                }
                draft
            }
            Err(_) => self.fallback.generate_stream_checked(context, on_token),
        }
    }
}

fn model_pool_call_draft(answer: String, stream_token: bool) -> InferenceDraft {
    let draft = InferenceDraft::new(
        answer.clone(),
        vec![ReasoningStep::new(
            "model_pool_call",
            "generated draft through model-pool call",
            0.9,
        )],
    );
    if stream_token {
        draft.with_tokens(vec![DraftToken::new(answer)])
    } else {
        draft
    }
}

#[allow(dead_code)]
pub(crate) fn run_timed_inference_stream<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    prompt: String,
    profile: TaskProfile,
    trace_path: Option<&PathBuf>,
    case_name: Option<&str>,
    on_token: &mut dyn FnMut(&DraftToken),
) -> std::io::Result<TimedOutcome> {
    run_timed_inference_stream_with_options(
        engine, backend, prompt, profile, None, trace_path, case_name, on_token,
    )
}

pub(crate) fn run_timed_inference_stream_with_options<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    prompt: String,
    profile: TaskProfile,
    max_tokens: Option<usize>,
    trace_path: Option<&PathBuf>,
    case_name: Option<&str>,
    on_token: &mut dyn FnMut(&DraftToken),
) -> std::io::Result<TimedOutcome> {
    let mut checked = |token: &DraftToken| {
        on_token(token);
        Ok(())
    };
    run_timed_inference_stream_checked_with_options(
        engine,
        backend,
        prompt,
        profile,
        max_tokens,
        trace_path,
        case_name,
        &mut checked,
    )
}

pub(crate) fn run_timed_inference_stream_checked_with_options<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    prompt: String,
    profile: TaskProfile,
    max_tokens: Option<usize>,
    trace_path: Option<&PathBuf>,
    case_name: Option<&str>,
    on_token: &mut dyn FnMut(&DraftToken) -> std::io::Result<()>,
) -> std::io::Result<TimedOutcome> {
    run_timed_inference_stream_checked_with_scope_options(
        engine, backend, prompt, profile, max_tokens, None, trace_path, case_name, on_token,
    )
}

pub(crate) fn run_timed_inference_stream_checked_with_scope_options<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    prompt: String,
    profile: TaskProfile,
    max_tokens: Option<usize>,
    tenant_scope: Option<TenantScope>,
    trace_path: Option<&PathBuf>,
    case_name: Option<&str>,
    on_token: &mut dyn FnMut(&DraftToken) -> std::io::Result<()>,
) -> std::io::Result<TimedOutcome> {
    run_timed_inference_stream_checked_with_scope_and_call_url_options(
        engine,
        backend,
        prompt,
        profile,
        max_tokens,
        tenant_scope,
        trace_path,
        case_name,
        on_token,
        None,
    )
}

#[cfg(test)]
pub(crate) fn run_timed_inference_stream_checked_with_model_pool_call_url<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    prompt: String,
    profile: TaskProfile,
    max_tokens: Option<usize>,
    trace_path: Option<&PathBuf>,
    case_name: Option<&str>,
    on_token: &mut dyn FnMut(&DraftToken) -> std::io::Result<()>,
    call_url: &str,
) -> std::io::Result<TimedOutcome> {
    run_timed_inference_stream_checked_with_scope_and_call_url_options(
        engine,
        backend,
        prompt,
        profile,
        max_tokens,
        None,
        trace_path,
        case_name,
        on_token,
        Some(call_url),
    )
}

fn run_timed_inference_stream_checked_with_scope_and_call_url_options<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    prompt: String,
    profile: TaskProfile,
    max_tokens: Option<usize>,
    tenant_scope: Option<TenantScope>,
    trace_path: Option<&PathBuf>,
    case_name: Option<&str>,
    on_token: &mut dyn FnMut(&DraftToken) -> std::io::Result<()>,
    call_url: Option<&str>,
) -> std::io::Result<TimedOutcome> {
    let started = Instant::now();
    let request = inference_request_with_options(prompt.clone(), profile, max_tokens, tenant_scope);
    let mut observer_error = None;
    let call_url_env = if call_url.is_none() {
        std::env::var(MODEL_POOL_CALL_URL_ENV).ok()
    } else {
        None
    };
    let call_url = call_url
        .or(call_url_env.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let mut outcome = {
        let mut checked = |token: &DraftToken| match on_token(token) {
            Ok(()) => Ok(()),
            Err(error) => {
                let message = error.to_string();
                observer_error = Some(error);
                Err(RuntimeError::new(format!(
                    "stream observer failed: {message}"
                )))
            }
        };
        if let Some(call_url) = call_url {
            let mut model_pool_backend = ModelPoolCallBackend {
                fallback: backend,
                call_url,
                configured_max_tokens: max_tokens,
            };
            engine.infer_stream_checked(request, &mut model_pool_backend, &mut checked)
        } else {
            engine.infer_stream_checked(request, backend, &mut checked)
        }
    };
    if let Some(error) = observer_error.as_ref() {
        let message = format!("stream observer failed: {error}");
        let timeout = matches!(
            error.kind(),
            std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
        ) || message.to_ascii_lowercase().contains("timed out")
            || message.to_ascii_lowercase().contains("timeout");
        let note = format!(
            "runtime_error:label=runtime_stream_observer_error:timeout={timeout}:message_chars={}",
            message.chars().count()
        );
        if !outcome
            .process_reward
            .notes
            .iter()
            .any(|item| item == &note)
        {
            outcome.process_reward.notes.push(note);
        }
    }
    let elapsed_ms = started.elapsed().as_millis();

    let trace_result = if let Some(trace_path) = trace_path {
        if let Some(case_name) = case_name {
            append_trace_jsonl_with_case(
                trace_path, case_name, &prompt, profile, elapsed_ms, &outcome,
            )
        } else {
            append_trace_jsonl(trace_path, &prompt, profile, elapsed_ms, &outcome)
        }
    } else {
        Ok(())
    };

    if let Some(error) = observer_error {
        let _ = trace_result;
        return Err(error);
    }
    trace_result?;

    Ok(TimedOutcome {
        outcome,
        elapsed_ms,
    })
}

fn inference_request_with_options(
    prompt: String,
    profile: TaskProfile,
    max_tokens: Option<usize>,
    tenant_scope: Option<TenantScope>,
) -> InferenceRequest {
    let route_plan_url = std::env::var(MODEL_POOL_ROUTE_PLAN_URL_ENV).ok();
    inference_request_with_options_and_route_plan_url(
        prompt,
        profile,
        max_tokens,
        tenant_scope,
        route_plan_url.as_deref(),
    )
}

fn inference_request_with_options_and_route_plan_url(
    prompt: String,
    profile: TaskProfile,
    max_tokens: Option<usize>,
    tenant_scope: Option<TenantScope>,
    route_plan_url: Option<&str>,
) -> InferenceRequest {
    let request = InferenceRequest::new(prompt, profile).with_max_tokens(max_tokens);
    let request =
        request.with_tenant_scope(tenant_scope.unwrap_or_else(TenantScope::local_single_user));
    let Some(route_plan_url) = route_plan_url
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return request;
    };

    match fetch_model_pool_route_plan_json(route_plan_url, &request.prompt, request.max_tokens) {
        Ok(route_plan_json) => request
            .clone()
            .try_with_agent_team_route_plan_json(&route_plan_json)
            .unwrap_or(request),
        Err(_) => request,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ModelPoolHttpEndpoint {
    host: String,
    port: u16,
    path: String,
}

fn fetch_model_pool_route_plan_json(
    route_plan_url: &str,
    prompt: &str,
    max_tokens: Option<usize>,
) -> Result<String, String> {
    let body = model_pool_route_plan_request_body(prompt, max_tokens);
    post_model_pool_json(
        route_plan_url,
        MODEL_POOL_ROUTE_PLAN_DEFAULT_PATH,
        MODEL_POOL_ROUTE_PLAN_TIMEOUT,
        "model pool route-plan",
        &body,
    )
}

fn fetch_model_pool_call_answer(
    call_url: &str,
    prompt: &str,
    max_tokens: Option<usize>,
) -> Result<String, String> {
    let body = model_pool_call_request_body(prompt, max_tokens);
    let response_body = post_model_pool_json(
        call_url,
        MODEL_POOL_CALL_DEFAULT_PATH,
        MODEL_POOL_CALL_TIMEOUT,
        "model pool call",
        &body,
    )?;
    json_string_field(&response_body, "answer")
        .filter(|answer| !answer.trim().is_empty())
        .ok_or_else(|| "model pool call response missing answer".to_owned())
}

fn post_model_pool_json(
    url: &str,
    default_path: &str,
    timeout: Duration,
    label: &str,
    body: &str,
) -> Result<String, String> {
    let endpoint = ModelPoolHttpEndpoint::parse(url, default_path, label)?;
    let mut stream = TcpStream::connect((endpoint.host.as_str(), endpoint.port))
        .map_err(|error| format!("{label} connect failed: {error}"))?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|error| format!("{label} read timeout setup failed: {error}"))?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|error| format!("{label} write timeout setup failed: {error}"))?;

    let request = format!(
        "POST {} HTTP/1.1\r\nhost: {}:{}\r\ncontent-type: application/json; charset=utf-8\r\naccept: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        endpoint.path,
        endpoint.host,
        endpoint.port,
        body.len(),
        body
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|error| format!("{label} write failed: {error}"))?;
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|error| format!("{label} read failed: {error}"))?;
    model_pool_http_body(&response, label)
}

fn model_pool_route_plan_request_body(prompt: &str, max_tokens: Option<usize>) -> String {
    let max_tokens = max_tokens
        .map(|value| format!(",\"max_tokens\":{}", value.max(1)))
        .unwrap_or_default();
    format!(
        "{{\"task_kind\":\"auto\",\"prompt\":{}{max_tokens}}}",
        service_json_string(prompt)
    )
}

fn model_pool_call_request_body(prompt: &str, max_tokens: Option<usize>) -> String {
    let max_tokens = max_tokens
        .map(|value| format!(",\"max_tokens\":{}", value.max(1)))
        .unwrap_or_default();
    format!(
        "{{\"task_kind\":\"auto\",\"prompt\":{}{max_tokens}}}",
        service_json_string(prompt)
    )
}

fn model_pool_http_body(response: &str, label: &str) -> Result<String, String> {
    let (head, body) = split_http_head_body(response);
    let status_code = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or_else(|| format!("{label} response missing HTTP status"))?;
    if !(200..300).contains(&status_code) {
        return Err(format!(
            "{label} returned status {status_code}: {}",
            body.trim()
        ));
    }
    Ok(body.to_owned())
}

impl ModelPoolHttpEndpoint {
    fn parse(url: &str, default_path: &str, label: &str) -> Result<Self, String> {
        let trimmed = url.trim().trim_end_matches('/');
        let without_scheme = trimmed.strip_prefix("http://").unwrap_or(trimmed);
        if without_scheme.starts_with("https://") {
            return Err(format!("{label} client only supports http://"));
        }
        let (authority, path) = without_scheme
            .split_once('/')
            .map(|(authority, path)| (authority, format!("/{path}")))
            .unwrap_or((without_scheme, default_path.to_owned()));
        let (host, port) = authority
            .rsplit_once(':')
            .ok_or_else(|| format!("{label} URL must include host:port"))?;
        let port = port
            .parse::<u16>()
            .map_err(|_| format!("{label} URL port must be a u16"))?;
        if host.is_empty() {
            return Err(format!("{label} URL host must not be empty"));
        }

        Ok(Self {
            host: host.to_owned(),
            port,
            path,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct PanicBackend;

    impl InferenceBackend for PanicBackend {
        fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
            panic!("fallback backend should not be called")
        }
    }

    #[test]
    fn inference_request_options_preserve_tenant_scope() {
        let scope = TenantScope::new("tenant-a", "workspace", "session");
        let request = inference_request_with_options(
            "hello".to_owned(),
            TaskProfile::Coding,
            Some(0),
            Some(scope.clone()),
        );

        assert_eq!(request.max_tokens, Some(1));
        assert_eq!(request.tenant_scope, Some(scope));
    }

    #[test]
    fn inference_request_options_default_to_local_single_user_scope() {
        let request =
            inference_request_with_options("hello".to_owned(), TaskProfile::Coding, None, None);

        assert_eq!(request.tenant_scope, Some(TenantScope::local_single_user()));
    }

    #[test]
    fn inference_request_options_fetch_route_plan_proof_when_url_is_set() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0_u8; 4096];
            let read = stream.read(&mut buffer).unwrap();
            let request = String::from_utf8_lossy(&buffer[..read]);
            assert!(request.contains("POST /v1/model-pool/route-plan HTTP/1.1"));
            assert!(request.contains("\"task_kind\":\"auto\""));
            assert!(request.contains("\"prompt\":\"agent team route\""));
            assert!(request.contains("\"max_tokens\":32"));

            let body = r#"{"ok":true,"read_only":true,"launches_process":false,"sends_prompt":false,"route_allowed":true,"reason":"ready","selected_role":"review","agent_model_route_source":{"route_allowed":true,"proof_ready":true,"selected_role":"review","model_registry_id":"registry.review","model_profile_id":"profile.review","inference_backend_id":"backend.review","model_pool_id":"pool.main"}}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
        });

        let request = inference_request_with_options_and_route_plan_url(
            "agent team route".to_owned(),
            TaskProfile::Coding,
            Some(32),
            None,
            Some(&format!("http://{addr}")),
        );

        assert_eq!(
            request
                .agent_team_route_proof
                .as_ref()
                .and_then(|proof| proof.selected_role.as_deref()),
            Some("review")
        );
        server.join().unwrap();
    }

    #[test]
    fn stream_uses_model_pool_call_answer_when_call_url_is_set() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0_u8; 4096];
            let read = stream.read(&mut buffer).unwrap();
            let request = String::from_utf8_lossy(&buffer[..read]);
            assert!(request.contains("POST /v1/model-pool/call HTTP/1.1"));
            assert!(request.contains("\"task_kind\":\"auto\""));
            assert!(request.contains("\"prompt\":\"stream through model pool\""));
            assert!(request.contains("\"max_tokens\":12"));

            let body = r#"{"ok":true,"answer":"stream model-pool answer"}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
        });
        let mut engine = NoironEngine::new();
        let mut backend = PanicBackend;
        let mut tokens = Vec::new();
        let mut on_token = |token: &DraftToken| {
            tokens.push(token.text.clone());
            Ok(())
        };

        let timed = run_timed_inference_stream_checked_with_model_pool_call_url(
            &mut engine,
            &mut backend,
            "stream through model pool".to_owned(),
            TaskProfile::Coding,
            Some(12),
            None,
            None,
            &mut on_token,
            &format!("http://{addr}"),
        )
        .unwrap();

        assert_eq!(tokens, vec!["stream model-pool answer"]);
        assert_eq!(timed.outcome.raw_answer, "stream model-pool answer");
        server.join().unwrap();
    }

    #[test]
    fn route_plan_endpoint_parse_uses_default_path() {
        let endpoint = ModelPoolHttpEndpoint::parse(
            "127.0.0.1:7878",
            MODEL_POOL_ROUTE_PLAN_DEFAULT_PATH,
            "model pool route-plan",
        )
        .unwrap();

        assert_eq!(endpoint.host, "127.0.0.1");
        assert_eq!(endpoint.port, 7878);
        assert_eq!(endpoint.path, "/v1/model-pool/route-plan");
    }
}
