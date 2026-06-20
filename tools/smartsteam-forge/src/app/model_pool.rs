use std::{io, time::Duration};

mod advice;
mod alignment;
mod command_output;
mod dispatch;
mod index_note;
mod read_only;
mod smoke;
mod watch;

use super::provider::ChatProvider;
pub(super) use advice::model_pool_advice;
#[cfg(test)]
pub(super) use advice::validate_model_pool_advice_report;
pub(super) use command_output::model_pool_error_summary;
use dispatch::{run_model_pool_call_to, run_model_pool_route_to};
pub(crate) use index_note::pin_model_pool_index_note;
use read_only::{run_model_pool_advice_to, run_model_pool_manifest_to, run_model_pool_status_to};
#[cfg(test)]
pub(super) use smoke::validate_model_pool_smoke_report;
pub(crate) use smoke::{
    model_pool_smoke, model_pool_smoke_error_events, model_pool_smoke_events,
    model_pool_smoke_status, run_model_pool_smoke_to,
};
use watch::run_model_pool_watch_to;

pub fn run_model_pool_status(provider: &dyn ChatProvider) -> io::Result<()> {
    let mut stdout = io::stdout();
    run_model_pool_status_to(provider, &mut stdout)
}

pub fn run_model_pool_manifest(provider: &dyn ChatProvider) -> io::Result<()> {
    let mut stdout = io::stdout();
    run_model_pool_manifest_to(provider, &mut stdout)
}

pub fn run_model_pool_advice(provider: &dyn ChatProvider) -> io::Result<()> {
    let mut stdout = io::stdout();
    run_model_pool_advice_to(provider, &mut stdout)
}

pub fn run_model_pool_smoke(provider: &dyn ChatProvider) -> io::Result<()> {
    let mut stdout = io::stdout();
    run_model_pool_smoke_to(provider, &mut stdout)
}

pub fn run_model_pool_route(provider: &dyn ChatProvider, task_kind: &str) -> io::Result<()> {
    let mut stdout = io::stdout();
    run_model_pool_route_to(provider, task_kind, &mut stdout)
}

pub fn run_model_pool_call(
    provider: &dyn ChatProvider,
    task_kind: &str,
    prompt: &str,
) -> io::Result<()> {
    let mut stdout = io::stdout();
    run_model_pool_call_to(provider, task_kind, prompt, &mut stdout)
}

pub fn run_model_pool_watch(
    provider: &dyn ChatProvider,
    interval: Duration,
    max_iterations: Option<usize>,
) -> io::Result<()> {
    let mut stdout = io::stdout();
    run_model_pool_watch_to(provider, interval, max_iterations, &mut stdout)
}
