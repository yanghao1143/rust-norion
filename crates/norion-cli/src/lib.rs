//! CLI interaction primitives for Norion frontends.

mod config;
mod input;
mod output;
mod status;

pub use config::{CliRuntimeConfig, CliStartupSnapshot, help_text, parse_cli_args};
pub use input::{
    CliInput, CliInputConfig, InputAction, InputActionKind, InputActionSnapshot, InputBufferKind,
    InputCommandPreview, InputControlSnapshot, InputEndpointOptionSnapshot,
    InputPreferenceOptionSnapshot, InputReadinessSnapshot, InputRequestSnapshot,
    InputRoleOptionSnapshot, InputRouteOptionsSnapshot, InputSessionPolicySnapshot,
    InputSubmitMode, KeyInput, SessionConfigUpdate, SessionConfigUpdateSnapshot,
};
pub use output::{
    OutputUpdate, OutputUpdateSource, OutputViewport, RequestPreviewSnapshot, RouteUpdateSnapshot,
    ScrollIntent, StreamOutcomeSnapshot, gate_advice_status, outcome_status,
    request_preview_status, request_preview_status_with_history_limit, route_update_status,
    session_config_status, started_turn_preview_status,
    started_turn_preview_status_with_history_limit,
};
pub use status::{
    CliSmartSteamStatusHostSnapshot, CliStatusSnapshot, CliWorkerHostSnapshot,
    CliWorkersHostSnapshot, cli_model_pool_status_line, cli_model_pool_workers_line,
    cli_status_line, cli_workers_unavailable_line,
};
