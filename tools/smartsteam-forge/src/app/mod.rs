mod commands;
mod context_preview;
mod diagnostic;
mod evolution_candidate_backlog;
mod evolution_candidate_events;
mod evolution_candidate_lifecycle;
mod evolution_candidate_model;
mod evolution_candidate_render;
mod evolution_candidate_sources;
mod evolution_candidate_status;
mod evolution_candidate_updates;
mod evolution_candidates;
mod evolution_clean_room_handoff_status;
mod evolution_daemon_args;
mod evolution_daemon_log_tail_status;
mod evolution_daemon_process;
mod evolution_helper_stage_repair_panel;
mod evolution_readiness_start_status;
mod evolution_report_detail_status;
mod evolution_report_gate_status;
mod evolution_self_improve_proposal_panel;
mod evolution_start_check_json;
mod evolution_start_command_preview;
mod evolution_start_plan_status;
mod evolution_status;
mod evolution_status_contract;
mod evolution_status_enriched_json;
mod evolution_status_summary;
#[cfg(test)]
mod evolution_status_tests;
mod evolution_strict_summary;
mod evolution_unified_status;
mod evolution_worker_window_status;
mod health;
mod input_buffer;
mod mock_provider;
mod model_pool;
mod model_pool_index_notes;
mod once;
mod provider;
mod retrieval_preview;
mod runtime_provider;
mod session_cli;
mod state;
mod status_json;

pub use diagnostic::run_diagnostic;
pub use evolution_candidates::{
    run_evolution_candidate_apply_check, run_evolution_candidate_gate,
    run_evolution_candidate_list, run_evolution_candidate_mark, run_evolution_candidate_validate,
    run_evolution_candidates,
};
pub use evolution_daemon_args::{EvolutionDaemonAction, EvolutionDaemonStartOptions};
pub use evolution_status::{
    run_evolution_daemon_control, run_evolution_start_check_json, run_evolution_status,
    run_evolution_status_watch,
};
pub use evolution_strict_summary::run_evolution_strict_summary;
pub use health::{
    require_prompt_preflight, run_experience_cleanup_audit, run_experience_hygiene_check,
    run_experience_hygiene_quarantine_dry_run, run_experience_repair_dry_run, run_health_check,
    run_preflight_check,
};
pub use mock_provider::MockProvider;
pub use model_pool::{
    run_model_pool_advice, run_model_pool_call, run_model_pool_manifest, run_model_pool_route,
    run_model_pool_smoke, run_model_pool_status, run_model_pool_watch,
};
pub use once::run_once;
pub use provider::ChatProvider;
pub use runtime_provider::RuntimeProvider;
pub use session_cli::{SessionCliCommand, run_session_cli};
pub use state::{App, Message, MessageRole};
