use std::io;

use super::evolution_daemon_args::EvolutionDaemonStartOptions;
use super::evolution_daemon_process::{evolution_loop_start_script, resolve_repo_path};
use super::status_json::{json_number_field, json_object_field};

const DEFAULT_DAEMON_BACKEND: &str = "127.0.0.1:7979";
const DEFAULT_PROMPT: &str = "Check the SmartSteam unattended evolution chain. Return one small improvement and one verifiable evidence item.";

const DEFAULT_INTERVAL_SECS: u64 = 30;
const DEFAULT_MAX_TOKENS: u64 = 4096;
const DEFAULT_MAX_TOTAL_TOKENS: u64 = 512;
const DEFAULT_MAX_RUNTIME_SECS: u64 = 900;
const DEFAULT_MAX_FAILURES: u64 = 3;
const DEFAULT_MAX_NO_FEEDBACK_ROUNDS: u64 = 3;
const DEFAULT_TIMEOUT_SECS: u64 = 300;
const DEFAULT_MIN_RUNTIME_CONTEXT: u64 = 262_144;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EvolutionStartCommandPreview {
    pub(super) command: String,
    pub(super) command_output: String,
    pub(super) effective_backend: String,
    pub(super) min_runtime_context: u64,
    pub(super) min_runtime_context_source: &'static str,
}

pub(super) fn build_evolution_start_command_preview(
    status: &str,
    work_dir: &str,
    backend: Option<&str>,
    prompt: Option<&str>,
    start_options: EvolutionDaemonStartOptions,
) -> io::Result<EvolutionStartCommandPreview> {
    let effective_backend = non_empty_value(backend)
        .unwrap_or(DEFAULT_DAEMON_BACKEND)
        .to_owned();
    let prompt = non_empty_value(prompt).unwrap_or(DEFAULT_PROMPT);
    let work_dir_path = resolve_repo_path(work_dir)?;
    let ledger_path = work_dir_path.join("evolution-ledger.jsonl");
    let report_path = work_dir_path.join("report.json");
    let remote_chain_status_path =
        resolve_repo_path("target\\remote-gemma-chain\\status-with-model-cache.json")?;
    let model_cache_status_path =
        resolve_repo_path("target\\remote-gemma-chain\\model-cache-status.json")?;
    let pool_budget_path = work_dir_path.join("model-pool-budget-fairness.json");
    let pool_lease_dir = work_dir_path.join("pool-leases");
    let start_script = evolution_loop_start_script()?;
    let min_context = resolve_min_runtime_context(status);

    let args = vec![
        "-NoProfile".to_owned(),
        "-ExecutionPolicy".to_owned(),
        "Bypass".to_owned(),
        "-File".to_owned(),
        start_script.display().to_string(),
        "-Backend".to_owned(),
        effective_backend.clone(),
        "-Forever".to_owned(),
        "-IntervalSecs".to_owned(),
        effective_u64(start_options.interval_secs, DEFAULT_INTERVAL_SECS).to_string(),
        "-MaxFailures".to_owned(),
        effective_u64(start_options.max_failures, DEFAULT_MAX_FAILURES).to_string(),
        "-MaxTokens".to_owned(),
        effective_u64(start_options.max_tokens, DEFAULT_MAX_TOKENS).to_string(),
        "-MaxTotalTokens".to_owned(),
        effective_u64(start_options.max_total_tokens, DEFAULT_MAX_TOTAL_TOKENS).to_string(),
        "-MaxRuntimeSecs".to_owned(),
        effective_u64(start_options.max_runtime_secs, DEFAULT_MAX_RUNTIME_SECS).to_string(),
        "-MaxNoFeedbackRounds".to_owned(),
        effective_u64(
            start_options.max_no_feedback_rounds,
            DEFAULT_MAX_NO_FEEDBACK_ROUNDS,
        )
        .to_string(),
        "-TimeoutSecs".to_owned(),
        effective_u64(start_options.timeout_secs, DEFAULT_TIMEOUT_SECS).to_string(),
        "-Ledger".to_owned(),
        ledger_path.display().to_string(),
        "-ReportJson".to_owned(),
        report_path.display().to_string(),
        "-PostRunReportGate".to_owned(),
        "-PostRunContinuationGate".to_owned(),
        "-RefreshRemoteChainStatus".to_owned(),
        "-RemoteChainStatusJson".to_owned(),
        remote_chain_status_path.display().to_string(),
        "-ModelCacheStatusJson".to_owned(),
        model_cache_status_path.display().to_string(),
        "-RemoteChainGate".to_owned(),
        "-RefreshPoolArtifacts".to_owned(),
        "-PoolBudgetFairnessJson".to_owned(),
        pool_budget_path.display().to_string(),
        "-PoolRouteTaskKind".to_owned(),
        "quality".to_owned(),
        "-PoolStageRouteTaskKinds".to_owned(),
        "summary,router,review,index,test-gate".to_owned(),
        "-PoolStageRouteGate".to_owned(),
        "-ExecutePoolStageCalls".to_owned(),
        "-RequirePoolBudgetPolicy".to_owned(),
        "-PoolAlignmentGate".to_owned(),
        "-RequirePoolRoute".to_owned(),
        "-PoolLeaseDir".to_owned(),
        pool_lease_dir.display().to_string(),
        "-PoolLeaseBusyPolicy".to_owned(),
        "skip-low-priority".to_owned(),
        "-MinRuntimeContext".to_owned(),
        min_context.value.to_string(),
        "-ExperienceAuditGate".to_owned(),
        "-StateConsistencyGate".to_owned(),
        "-RequireHelperStageRoles".to_owned(),
        "summary,router,review,index,test-gate".to_owned(),
        "-RequireLatestHelperStageRoles".to_owned(),
        "summary,router,review,index,test-gate".to_owned(),
        "-RequireTestGatePass".to_owned(),
        "-Prompt".to_owned(),
        prompt.to_owned(),
    ];
    let command = format!(
        "powershell.exe {}",
        args.iter()
            .map(|arg| quote_command_argument(arg))
            .collect::<Vec<_>>()
            .join(" ")
    );
    let command_output = format!(
        "check_only=true\nstarts_process=false\nsends_prompt=false\npid_file={}\nstdout_log={}\nstderr_log={}\nmin_runtime_context={}\nmin_runtime_context_source={}\ncommand={command}",
        work_dir_path.join("evolution-loop.pid").display(),
        work_dir_path.join("evolution-loop.out.log").display(),
        work_dir_path.join("evolution-loop.err.log").display(),
        min_context.value,
        min_context.source,
    );

    Ok(EvolutionStartCommandPreview {
        command,
        command_output,
        effective_backend,
        min_runtime_context: min_context.value,
        min_runtime_context_source: min_context.source,
    })
}

struct MinRuntimeContext {
    value: u64,
    source: &'static str,
}

fn resolve_min_runtime_context(status: &str) -> MinRuntimeContext {
    let Some(loop_status) = json_object_field(status, "loop") else {
        return fallback_min_runtime_context();
    };
    if let Some(model_pool) = json_object_field(loop_status, "model_pool") {
        for field in [
            "quality_context_required_tokens",
            "quality_context_tokens",
            "min_context_tokens",
        ] {
            if let Some(value) = positive_u64_field(model_pool, field) {
                return MinRuntimeContext {
                    value,
                    source: "status_model_pool",
                };
            }
        }
    }
    if let Some(remote_chain) = json_object_field(loop_status, "remote_chain") {
        for field in [
            "quality_context_required_tokens",
            "quality_context_tokens",
            "min_context_tokens",
        ] {
            if let Some(value) = positive_u64_field(remote_chain, field) {
                return MinRuntimeContext {
                    value,
                    source: "status_remote_chain",
                };
            }
        }
    }
    fallback_min_runtime_context()
}

fn fallback_min_runtime_context() -> MinRuntimeContext {
    MinRuntimeContext {
        value: DEFAULT_MIN_RUNTIME_CONTEXT,
        source: "fallback",
    }
}

fn positive_u64_field(object: &str, field: &str) -> Option<u64> {
    json_number_field(object, field)
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
}

fn effective_u64(value: Option<u64>, default_value: u64) -> u64 {
    value.unwrap_or(default_value)
}

fn quote_command_argument(value: &str) -> String {
    if value.is_empty() {
        return "\"\"".to_owned();
    }
    if !value
        .chars()
        .any(|character| character.is_whitespace() || character == '"')
    {
        return value.to_owned();
    }
    format!("\"{}\"", value.replace('"', "\\\""))
}

fn non_empty_value(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_preview_is_pure_and_applies_budget_overrides() {
        let status = r#"{
            "loop": {
                "model_pool": {"min_context_tokens": 65536}
            }
        }"#;

        let preview = build_evolution_start_command_preview(
            status,
            "target\\evolution\\daemon",
            Some("127.0.0.1:7979"),
            Some("custom prompt"),
            EvolutionDaemonStartOptions {
                interval_secs: Some(1),
                max_tokens: Some(64),
                max_total_tokens: Some(96),
                max_runtime_secs: Some(0),
                max_failures: Some(1),
                max_no_feedback_rounds: Some(0),
                timeout_secs: Some(300),
            },
        )
        .unwrap();

        assert!(preview.command.starts_with("powershell.exe -NoProfile"));
        assert!(preview.command.contains("start-evolution-loop.ps1"));
        assert!(preview.command.contains("-Backend 127.0.0.1:7979"));
        assert!(preview.command.contains("-IntervalSecs 1"));
        assert!(preview.command.contains("-MaxTokens 64"));
        assert!(preview.command.contains("-MaxTotalTokens 96"));
        assert!(preview.command.contains("-MaxRuntimeSecs 0"));
        assert!(preview.command.contains("-MaxNoFeedbackRounds 0"));
        assert!(preview.command.contains("-TimeoutSecs 300"));
        assert!(preview.command.contains("-MinRuntimeContext 65536"));
        assert!(preview.command.contains("-Prompt \"custom prompt\""));
        assert_eq!(preview.effective_backend, "127.0.0.1:7979");
        assert_eq!(preview.min_runtime_context, 65536);
        assert_eq!(preview.min_runtime_context_source, "status_model_pool");
        assert!(preview.command_output.contains("check_only=true"));
        assert!(preview.command_output.contains("starts_process=false"));
        assert!(preview.command_output.contains("sends_prompt=false"));
        assert!(preview.command_output.contains("command=powershell.exe"));
    }

    #[test]
    fn command_preview_uses_daemon_defaults_without_overrides() {
        let preview = build_evolution_start_command_preview(
            "{}",
            "target\\evolution\\daemon",
            None,
            None,
            EvolutionDaemonStartOptions::default(),
        )
        .unwrap();

        assert!(preview.command.contains("-Backend 127.0.0.1:7979"));
        assert!(preview.command.contains("-IntervalSecs 30"));
        assert!(preview.command.contains("-MaxTokens 4096"));
        assert!(preview.command.contains("-MaxTotalTokens 512"));
        assert!(preview.command.contains("-MaxRuntimeSecs 900"));
        assert!(preview.command.contains("-MaxNoFeedbackRounds 3"));
        assert!(preview.command.contains("-TimeoutSecs 300"));
        assert!(preview.command.contains("-MinRuntimeContext 262144"));
        assert!(
            preview
                .command
                .contains("-Prompt \"Check the SmartSteam unattended evolution chain.")
        );
        assert_eq!(preview.effective_backend, "127.0.0.1:7979");
        assert_eq!(preview.min_runtime_context_source, "fallback");
    }
}
