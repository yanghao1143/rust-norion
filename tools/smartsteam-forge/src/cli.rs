use smartsteam_forge::StreamEndpoint;

use crate::app;

mod parse;
mod provider;
#[cfg(test)]
mod tests;
mod usage;

pub(crate) use provider::provider_config;
pub(crate) use usage::usage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ModelPoolWatchConfig {
    pub(crate) interval_secs: u64,
    pub(crate) max_iterations: Option<usize>,
}

#[derive(Debug, Clone)]
pub(crate) struct CliConfig {
    pub(crate) backend: String,
    pub(crate) backend_overridden: bool,
    pub(crate) mock: bool,
    pub(crate) prompt: Option<String>,
    pub(crate) endpoint: Option<StreamEndpoint>,
    pub(crate) request_timeout_secs: Option<u64>,
    pub(crate) connect_timeout_ms: Option<u64>,
    pub(crate) read_timeout_ms: Option<u64>,
    pub(crate) context_messages: Option<usize>,
    pub(crate) max_tokens: Option<Option<usize>>,
    pub(crate) health_check: bool,
    pub(crate) experience_hygiene: bool,
    pub(crate) experience_hygiene_quarantine: bool,
    pub(crate) experience_hygiene_limit: usize,
    pub(crate) experience_repair: bool,
    pub(crate) experience_repair_limit: usize,
    pub(crate) experience_cleanup_audit: bool,
    pub(crate) experience_cleanup_audit_limit: usize,
    pub(crate) model_pool_status: bool,
    pub(crate) model_pool_manifest: bool,
    pub(crate) model_pool_advice: bool,
    pub(crate) model_pool_smoke: bool,
    pub(crate) model_pool_route: Option<String>,
    pub(crate) model_pool_call: Option<String>,
    pub(crate) model_pool_watch: Option<ModelPoolWatchConfig>,
    pub(crate) evolution_status: bool,
    pub(crate) evolution_status_json: bool,
    pub(crate) evolution_strict_summary: bool,
    pub(crate) evolution_strict_summary_json: bool,
    pub(crate) evolution_strict_summary_path: Option<String>,
    pub(crate) evolution_start: bool,
    pub(crate) evolution_stop: bool,
    pub(crate) evolution_check_only: bool,
    pub(crate) evolution_start_check_json: bool,
    pub(crate) evolution_watch: Option<ModelPoolWatchConfig>,
    pub(crate) evolution_candidates: bool,
    pub(crate) evolution_candidate_list: bool,
    pub(crate) evolution_candidate_gate: bool,
    pub(crate) evolution_candidates_limit: usize,
    pub(crate) evolution_candidates_save: bool,
    pub(crate) evolution_candidates_backlog: Option<String>,
    pub(crate) evolution_candidate_mark: Option<String>,
    pub(crate) evolution_candidate_apply_check: Option<String>,
    pub(crate) evolution_candidate_validate: Option<String>,
    pub(crate) evolution_candidate_validation_command: Option<String>,
    pub(crate) evolution_candidate_validation_status: Option<String>,
    pub(crate) evolution_candidate_status: Option<String>,
    pub(crate) evolution_candidate_note: Option<String>,
    pub(crate) evolution_work_dir: String,
    pub(crate) evolution_interval_secs: Option<u64>,
    pub(crate) evolution_max_tokens: Option<u64>,
    pub(crate) evolution_max_total_tokens: Option<u64>,
    pub(crate) evolution_max_runtime_secs: Option<u64>,
    pub(crate) evolution_max_failures: Option<u64>,
    pub(crate) evolution_max_no_feedback_rounds: Option<u64>,
    pub(crate) evolution_timeout_secs: Option<u64>,
    pub(crate) doctor: bool,
    pub(crate) preflight_check: bool,
    pub(crate) require_health: bool,
    pub(crate) require_safe_device: bool,
    pub(crate) session_command: Option<app::SessionCliCommand>,
    session_limit: usize,
    pub(crate) help: bool,
}
