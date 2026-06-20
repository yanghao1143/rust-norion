use smartsteam_forge::ForgeProvider;

pub(super) fn status(provider: &ForgeProvider) -> String {
    match provider.health() {
        Ok(health) => health.summary(),
        Err(error) => format!(
            "backend health error: {}",
            attach_provider_diagnostics(provider, error)
        ),
    }
}

pub(super) fn health_check(provider: &ForgeProvider) -> Result<String, String> {
    provider
        .health()
        .map(|health| health.summary())
        .map_err(|error| attach_provider_diagnostics(provider, error))
}

pub(super) fn experience_hygiene(provider: &ForgeProvider) -> Result<String, String> {
    provider.experience_hygiene()
}

pub(super) fn experience_hygiene_quarantine_dry_run(
    provider: &ForgeProvider,
    limit: usize,
) -> Result<String, String> {
    provider.experience_hygiene_quarantine_dry_run(limit)
}

pub(super) fn experience_repair_dry_run(
    provider: &ForgeProvider,
    limit: usize,
) -> Result<String, String> {
    provider.experience_repair_dry_run(limit)
}

pub(super) fn experience_cleanup_audit(
    provider: &ForgeProvider,
    limit: usize,
) -> Result<String, String> {
    provider.experience_cleanup_audit(limit)
}

pub(super) fn experience_retrieval(
    provider: &ForgeProvider,
    prompt: &str,
    profile: &str,
    limit: usize,
    index_context: Option<&str>,
) -> Result<String, String> {
    provider.experience_retrieval_with_index_context(prompt, profile, limit, index_context)
}

pub(super) fn readiness_check(provider: &ForgeProvider) -> Result<String, String> {
    let health = provider
        .health()
        .map_err(|error| attach_provider_diagnostics(provider, error))?;
    health
        .require_ready()
        .map_err(|error| attach_provider_diagnostics(provider, error))?;
    Ok(health.summary())
}

pub(super) fn prompt_preflight(
    provider: &ForgeProvider,
    require_safe_device: bool,
) -> Result<String, String> {
    let health = provider
        .health()
        .map_err(|error| attach_provider_diagnostics(provider, error))?;
    health
        .require_ready()
        .map_err(|error| attach_provider_diagnostics(provider, error))?;
    if require_safe_device {
        health
            .require_safe_device()
            .map_err(|error| attach_provider_diagnostics(provider, error))?;
    }
    Ok(health.summary())
}

pub(super) fn safe_device_check(provider: &ForgeProvider) -> Result<String, String> {
    let health = provider
        .health()
        .map_err(|error| attach_provider_diagnostics(provider, error))?;
    health
        .require_ready()
        .map_err(|error| attach_provider_diagnostics(provider, error))?;
    health
        .require_safe_device()
        .map_err(|error| attach_provider_diagnostics(provider, error))?;
    Ok(health.summary())
}

pub(super) fn health_and_readiness(
    provider: &ForgeProvider,
) -> (Result<String, String>, Result<String, String>) {
    match provider.health() {
        Ok(health) => {
            let summary = health.summary();
            let readiness = health
                .require_ready()
                .map(|()| summary.clone())
                .map_err(|error| attach_provider_diagnostics(provider, error));
            (Ok(summary), readiness)
        }
        Err(error) => {
            let error = attach_provider_diagnostics(provider, error);
            (Err(error.clone()), Err(error))
        }
    }
}

pub(super) fn health_readiness_and_safe_device(
    provider: &ForgeProvider,
) -> (
    Result<String, String>,
    Result<String, String>,
    Result<String, String>,
) {
    match provider.health() {
        Ok(health) => {
            let summary = health.summary();
            let readiness = health
                .require_ready()
                .map(|()| summary.clone())
                .map_err(|error| attach_provider_diagnostics(provider, error));
            let safe_device = match health.require_ready() {
                Ok(()) => health
                    .require_safe_device()
                    .map(|()| summary.clone())
                    .map_err(|error| attach_provider_diagnostics(provider, error)),
                Err(error) => Err(attach_provider_diagnostics(provider, error)),
            };
            (Ok(summary), readiness, safe_device)
        }
        Err(error) => {
            let error = attach_provider_diagnostics(provider, error);
            (Err(error.clone()), Err(error.clone()), Err(error))
        }
    }
}

pub(super) fn diagnostic_target(provider: &ForgeProvider) -> String {
    let config = provider.config();
    format!(
        "backend={} connect_timeout={:?} read_timeout={:?} request_timeout={:?}",
        config.backend, config.connect_timeout, config.read_timeout, config.request_timeout
    )
}

fn attach_provider_diagnostics(provider: &ForgeProvider, error: String) -> String {
    if error.contains("诊断命令") {
        return error;
    }
    format!("{error}\n{}", provider_diagnostic_commands(provider))
}

fn provider_diagnostic_commands(provider: &ForgeProvider) -> String {
    let backend = &provider.config().backend;
    let base_url = backend_base_url(backend);
    format!(
        "诊断命令: cargo run -- --backend {backend} --connect-timeout-ms 500 --read-timeout-ms 500 --doctor; cargo run -- --backend {backend} --connect-timeout-ms 500 --read-timeout-ms 500 --preflight --require-safe-device; curl.exe -s {base_url}/health; nvidia-smi\n说明: --read-timeout-ms 是单次 read 轮询/heartbeat 间隔；真实 Gemma 流式总等待窗口用 --timeout-secs。"
    )
}

fn backend_base_url(backend: &str) -> String {
    let trimmed = backend.trim().trim_end_matches('/');
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_owned()
    } else {
        format!("http://{trimmed}")
    }
}

#[cfg(test)]
mod tests {
    use smartsteam_forge::ProviderConfig;

    use super::*;

    #[test]
    fn provider_diagnostics_explain_read_poll_vs_total_stream_timeout() {
        let provider = ForgeProvider::new(ProviderConfig::rust_norion("127.0.0.1:7979"));

        let commands = provider_diagnostic_commands(&provider);

        assert!(commands.contains("--read-timeout-ms 500"));
        assert!(commands.contains("--read-timeout-ms 是单次 read 轮询/heartbeat 间隔"));
        assert!(commands.contains("真实 Gemma 流式总等待窗口用 --timeout-secs"));
    }
}
