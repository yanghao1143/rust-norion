use std::io::{self, Write};

use crate::app::provider::ChatProvider;

use super::command_output::{evented_error, record_and_write_summary};
use super::pin_model_pool_index_note;

#[cfg(test)]
mod tests;

pub(crate) fn run_model_pool_route_to<W: Write>(
    provider: &dyn ChatProvider,
    task_kind: &str,
    output: &mut W,
) -> io::Result<()> {
    match provider.model_pool_route(task_kind) {
        Ok(summary) => {
            record_and_write_summary(provider, "model_pool_route", &summary, output)?;
            output.flush()
        }
        Err(error) => Err(evented_error(
            provider,
            "model_pool_route_error",
            "route",
            &error,
        )),
    }
}

pub(crate) fn run_model_pool_call_to<W: Write>(
    provider: &dyn ChatProvider,
    task_kind: &str,
    prompt: &str,
    output: &mut W,
) -> io::Result<()> {
    match provider.model_pool_call(task_kind, prompt) {
        Ok(summary) => {
            record_and_write_summary(provider, "model_pool_call", &summary, output)?;
            if let Some(status) = pin_model_pool_index_note(provider, task_kind, prompt, &summary) {
                writeln!(output, "{status}")?;
            }
            output.flush()
        }
        Err(error) => Err(evented_error(
            provider,
            "model_pool_call_error",
            "call",
            &error,
        )),
    }
}
