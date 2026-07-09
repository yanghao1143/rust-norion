use std::env;
use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Instant;

use model_pool_advice_core::{
    ModelCallCandidate, ModelCallFailureClass, ModelFallbackPolicy, ModelPoolLiveSmokePolicy,
    ModelTaskKind, evaluate_live_model_pool_smoke, model_fallback_plan_after_failure,
    model_pool_evidence_is_sanitized,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    if help_requested(env::args().skip(1)) {
        print_help();
        return Ok(());
    }

    let endpoint = required_env("NORION_MODEL_POOL_ENDPOINT")?;
    let api_key = required_env("NORION_MODEL_POOL_API_KEY")?;
    let models = required_env("NORION_MODEL_POOL_MODELS")?
        .split(',')
        .map(str::trim)
        .filter(|model| !model.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if models.is_empty() {
        return Err("NORION_MODEL_POOL_MODELS is empty".to_owned());
    }

    let timeout_seconds = env_u64("NORION_MODEL_POOL_TIMEOUT_SECONDS", 60);
    let max_latency_ms = env_u64("NORION_MODEL_POOL_MAX_LATENCY_MS", 60_000);
    let min_available_models = env_usize("NORION_MODEL_POOL_MIN_AVAILABLE_MODELS", 1);
    let mut candidates = Vec::new();
    let mut rows = Vec::new();
    let policy = ModelPoolLiveSmokePolicy {
        min_available_models,
        max_latency_ms,
        require_code_capable: true,
    };

    match call_provider_preflight(&endpoint, &api_key, timeout_seconds) {
        Ok(()) => rows.push(provider_preflight_json_line("passed", None)),
        Err(ModelCallFailureClass::Unauthorized) => {
            rows.push(provider_preflight_json_line(
                "failed",
                Some(ModelCallFailureClass::Unauthorized),
            ));
            let report = evaluate_live_model_pool_smoke(&candidates, policy);
            let fallback = model_fallback_plan_after_failure(
                "provider_preflight",
                ModelCallFailureClass::Unauthorized,
                &candidates,
                ModelTaskKind::Code,
                ModelFallbackPolicy::default(),
            );
            let fallback_evidence = fallback.evidence_line();
            rows.push(format!(
                "{{\"evidence\":\"{}\",\"reason\":\"provider_auth_failed\"}}",
                json_escape(&fallback_evidence)
            ));
            write_artifact(&mut rows, &report.evidence_line)?;

            println!("provider_preflight=provider_auth_failed failure_class=unauthorized");
            println!("{fallback_evidence}");
            println!("{}", report.evidence_line);
            println!("artifact=target/model-pool-live-smoke.jsonl");
            return Err("model pool provider preflight failed: unauthorized".to_owned());
        }
        Err(_) => rows.push(provider_preflight_json_line("skipped", None)),
    }

    for model in models {
        let start = Instant::now();
        let candidate = match call_chat_completion(&endpoint, &api_key, &model, timeout_seconds) {
            Ok(smoke) => ModelCallCandidate::success(
                model.clone(),
                "code",
                start.elapsed().as_millis() as u64,
                smoke.completion_tokens,
            )
            .with_prompt_tokens(smoke.prompt_tokens)
            .with_code_capability(),
            Err(failure) => {
                ModelCallCandidate::failed(model.clone(), "code", failure).with_code_capability()
            }
        };
        rows.push(candidate_json_line(&candidate));
        candidates.push(candidate);
    }

    let report = evaluate_live_model_pool_smoke(&candidates, policy);

    write_artifact(&mut rows, &report.evidence_line)?;

    println!("{}", report.evidence_line);
    println!("artifact=target/model-pool-live-smoke.jsonl");
    if report.passed {
        Ok(())
    } else {
        Err(format!(
            "model pool live smoke failed: {:?}",
            report.failures
        ))
    }
}

fn help_requested(args: impl IntoIterator<Item = String>) -> bool {
    args.into_iter()
        .any(|arg| matches!(arg.as_str(), "-h" | "--help"))
}

fn print_help() {
    println!(
        "model-pool-live-gate\n\nRequired env:\n  NORION_MODEL_POOL_ENDPOINT\n  NORION_MODEL_POOL_API_KEY\n  NORION_MODEL_POOL_MODELS\n\nOptional env:\n  NORION_MODEL_POOL_TIMEOUT_SECONDS\n  NORION_MODEL_POOL_MAX_LATENCY_MS\n  NORION_MODEL_POOL_MIN_AVAILABLE_MODELS"
    );
}

fn write_artifact(rows: &mut Vec<String>, evidence_line: &str) -> Result<(), String> {
    if !model_pool_evidence_is_sanitized(evidence_line)
        || rows
            .iter()
            .any(|row| !model_pool_evidence_is_sanitized(row))
    {
        return Err("refusing to write unsanitized model-pool evidence".to_owned());
    }

    fs::create_dir_all("target").map_err(|error| format!("create target failed: {error}"))?;
    let artifact = "target/model-pool-live-smoke.jsonl";
    rows.push(format!(
        "{{\"evidence\":\"{}\"}}",
        json_escape(evidence_line)
    ));
    fs::write(artifact, rows.join("\n"))
        .map_err(|error| format!("write {artifact} failed: {error}"))
}

struct SmokeSuccess {
    prompt_tokens: u64,
    completion_tokens: u64,
}

fn call_chat_completion(
    endpoint: &str,
    api_key: &str,
    model: &str,
    timeout_seconds: u64,
) -> Result<SmokeSuccess, ModelCallFailureClass> {
    let base_url = endpoint.trim_end_matches('/');
    let url = if base_url.ends_with("/v1") {
        format!("{base_url}/chat/completions")
    } else {
        format!("{base_url}/v1/chat/completions")
    };
    let body = format!(
        "{{\"model\":\"{}\",\"messages\":[{{\"role\":\"user\",\"content\":\"write one tiny Rust function that adds two u32 values\"}}],\"max_tokens\":96,\"temperature\":0}}",
        json_escape(model)
    );
    let config = format!(
        "url = \"{}\"\nrequest = \"POST\"\nheader = \"content-type: application/json\"\nheader = \"authorization: Bearer {}\"\ndata = \"{}\"\nmax-time = {}\nsilent\nshow-error\nwrite-out = \"\\n__NORION_HTTP_STATUS:%{{http_code}}__\\n\"\n",
        curl_escape(&url),
        curl_escape(api_key),
        curl_escape(&body),
        timeout_seconds.max(1),
    );

    let mut child = Command::new("curl")
        .arg("--config")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| ModelCallFailureClass::Unavailable)?;
    child
        .stdin
        .as_mut()
        .ok_or(ModelCallFailureClass::Unavailable)?
        .write_all(config.as_bytes())
        .map_err(|_| ModelCallFailureClass::Unavailable)?;
    let output = child
        .wait_with_output()
        .map_err(|_| ModelCallFailureClass::Unavailable)?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() && stderr.to_ascii_lowercase().contains("timed out") {
        return Err(ModelCallFailureClass::Timeout);
    }
    let (body, status) = split_curl_status(&stdout)?;
    match status {
        200 => {
            let completion_tokens = json_u64_field(body, "completion_tokens")
                .or_else(|| {
                    if response_looks_code_like(body) {
                        Some(1)
                    } else {
                        None
                    }
                })
                .ok_or(ModelCallFailureClass::MalformedResponse)?;
            if completion_tokens == 0 {
                return Err(ModelCallFailureClass::EmptyOutput);
            }
            Ok(SmokeSuccess {
                prompt_tokens: json_u64_field(body, "prompt_tokens").unwrap_or(0),
                completion_tokens,
            })
        }
        401 | 403 => Err(ModelCallFailureClass::Unauthorized),
        404 => Err(ModelCallFailureClass::ProviderNotFound),
        _ if body.to_ascii_lowercase().contains("model")
            && body.to_ascii_lowercase().contains("not") =>
        {
            Err(ModelCallFailureClass::ProviderNotFound)
        }
        _ => Err(ModelCallFailureClass::MalformedResponse),
    }
}

fn call_provider_preflight(
    endpoint: &str,
    api_key: &str,
    timeout_seconds: u64,
) -> Result<(), ModelCallFailureClass> {
    let base_url = endpoint.trim_end_matches('/');
    let url = if base_url.ends_with("/v1") {
        format!("{base_url}/models")
    } else {
        format!("{base_url}/v1/models")
    };
    let config = format!(
        "url = \"{}\"\nrequest = \"GET\"\nheader = \"authorization: Bearer {}\"\nmax-time = {}\nsilent\nshow-error\nwrite-out = \"\\n__NORION_HTTP_STATUS:%{{http_code}}__\\n\"\n",
        curl_escape(&url),
        curl_escape(api_key),
        timeout_seconds.max(1),
    );

    let mut child = Command::new("curl")
        .arg("--config")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| ModelCallFailureClass::Unavailable)?;
    child
        .stdin
        .as_mut()
        .ok_or(ModelCallFailureClass::Unavailable)?
        .write_all(config.as_bytes())
        .map_err(|_| ModelCallFailureClass::Unavailable)?;
    let output = child
        .wait_with_output()
        .map_err(|_| ModelCallFailureClass::Unavailable)?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() && stderr.to_ascii_lowercase().contains("timed out") {
        return Err(ModelCallFailureClass::Timeout);
    }
    let (_, status) = split_curl_status(&stdout)?;
    match status {
        200 => Ok(()),
        401 | 403 => Err(ModelCallFailureClass::Unauthorized),
        404 => Err(ModelCallFailureClass::ProviderNotFound),
        _ => Err(ModelCallFailureClass::MalformedResponse),
    }
}

fn split_curl_status(stdout: &str) -> Result<(&str, u16), ModelCallFailureClass> {
    let marker = "__NORION_HTTP_STATUS:";
    let (body, status_part) = stdout
        .rsplit_once(marker)
        .ok_or(ModelCallFailureClass::MalformedResponse)?;
    let status = status_part
        .trim()
        .trim_end_matches("__")
        .parse::<u16>()
        .map_err(|_| ModelCallFailureClass::MalformedResponse)?;
    Ok((body, status))
}

fn response_looks_code_like(body: &str) -> bool {
    ["fn ", "```rust", "impl ", "pub fn"]
        .iter()
        .any(|needle| body.contains(needle))
}

fn candidate_json_line(candidate: &ModelCallCandidate) -> String {
    let failure_class = candidate
        .status
        .failure_class()
        .map(ModelCallFailureClass::as_str)
        .unwrap_or("none");
    format!(
        "{{\"model\":\"{}\",\"status\":\"{}\",\"failure_class\":\"{}\",\"wall_ms\":{},\"prompt_tokens\":{},\"completion_tokens\":{},\"tokens_per_sec\":{},\"code_capable\":{}}}",
        json_escape(&candidate.model_id),
        candidate.status.as_str(),
        failure_class,
        candidate
            .latency_ms
            .map(|latency| latency.to_string())
            .unwrap_or_else(|| "null".to_owned()),
        candidate.prompt_tokens,
        candidate.completion_tokens,
        candidate
            .tokens_per_second()
            .map(|rate| format!("{rate:.3}"))
            .unwrap_or_else(|| "null".to_owned()),
        candidate.supports_code,
    )
}

fn provider_preflight_json_line(
    status: &str,
    failure_class: Option<ModelCallFailureClass>,
) -> String {
    format!(
        "{{\"provider_preflight\":\"{}\",\"failure_class\":\"{}\",\"reason\":\"{}\"}}",
        json_escape(status),
        failure_class
            .map(ModelCallFailureClass::as_str)
            .unwrap_or("none"),
        if failure_class == Some(ModelCallFailureClass::Unauthorized) {
            "provider_auth_failed"
        } else {
            "none"
        }
    )
}

fn json_u64_field(body: &str, field: &str) -> Option<u64> {
    let needle = format!("\"{field}\"");
    let after_field = body.get(body.find(&needle)? + needle.len()..)?;
    let after_colon = after_field.get(after_field.find(':')? + 1..)?.trim_start();
    let digits = after_colon
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect::<String>();
    digits.parse().ok()
}

fn required_env(name: &str) -> Result<String, String> {
    env::var(name)
        .map(|value| value.trim().to_owned())
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{name} is required"))
}

fn env_u64(name: &str, default: u64) -> u64 {
    env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn env_usize(name: &str, default: usize) -> usize {
    env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn json_escape(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out
}

fn curl_escape(value: &str) -> String {
    json_escape(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_curl_status_without_returning_body() {
        let (body, status) =
            split_curl_status("{\"usage\":{\"completion_tokens\":1}}\n__NORION_HTTP_STATUS:200__")
                .unwrap();

        assert_eq!(status, 200);
        assert!(body.contains("completion_tokens"));
    }

    #[test]
    fn help_flag_does_not_require_env() {
        assert!(help_requested(vec!["--help".to_owned()]));
        assert!(help_requested(vec!["-h".to_owned()]));
        assert!(!help_requested(Vec::<String>::new()));
    }

    #[test]
    fn candidate_json_line_is_sanitized() {
        let candidate = ModelCallCandidate::failed(
            "meta/llama-3.1-8b-instruct",
            "code",
            ModelCallFailureClass::Unauthorized,
        )
        .with_code_capability();

        let line = candidate_json_line(&candidate);

        assert!(line.contains("\"failure_class\":\"unauthorized\""));
        assert!(model_pool_evidence_is_sanitized(&line));
    }

    #[test]
    fn provider_preflight_auth_failure_evidence_is_sanitized() {
        let line =
            provider_preflight_json_line("failed", Some(ModelCallFailureClass::Unauthorized));

        assert!(line.contains("\"reason\":\"provider_auth_failed\""));
        assert!(line.contains("\"failure_class\":\"unauthorized\""));
        assert!(model_pool_evidence_is_sanitized(&line));
    }
}
