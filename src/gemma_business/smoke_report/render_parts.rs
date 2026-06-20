mod checks;
mod contract;
mod feedback;
mod files;
mod generate;
mod http;
mod replay;
mod state;
mod trace;

pub(super) use checks::matrix_check_json;
#[cfg(test)]
pub(super) use checks::single_check_json;
pub(super) use contract::{ContractJson, contract_json};
pub(super) use feedback::feedback_json;
pub(super) use files::files_json;
pub(super) use generate::generate_json;
pub(super) use http::http_json;
pub(super) use replay::replay_json;
pub(super) use state::state_json;
pub(super) use trace::trace_json;

pub(super) const BUSINESS_CYCLE_GATE: &str = "gemma_business_cycle";
pub(super) const BUSINESS_CYCLE_SCHEMA: &str = "rust-norion-gemma-business-cycle-smoke-v1";
pub(super) const MATRIX_BUSINESS_CASE: &str = "gemma-business-cycle-matrix";
