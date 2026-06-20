use std::io::{self, Write};

use crate::app::provider::ChatProvider;

use super::command_output::{evented_error, record_and_write_summary};
use super::model_pool_advice;

#[cfg(test)]
mod tests;

pub(crate) fn run_model_pool_status_to<W: Write>(
    provider: &dyn ChatProvider,
    output: &mut W,
) -> io::Result<()> {
    match provider.model_pool_status() {
        Ok(summary) => {
            record_and_write_summary(provider, "model_pool_status", &summary, output)?;
            output.flush()
        }
        Err(error) => Err(evented_error(
            provider,
            "model_pool_status_error",
            "status",
            &error,
        )),
    }
}

pub(crate) fn run_model_pool_manifest_to<W: Write>(
    provider: &dyn ChatProvider,
    output: &mut W,
) -> io::Result<()> {
    match provider.model_pool_manifest() {
        Ok(summary) => {
            record_and_write_summary(provider, "model_pool_manifest", &summary, output)?;
            output.flush()
        }
        Err(error) => Err(evented_error(
            provider,
            "model_pool_manifest_error",
            "manifest",
            &error,
        )),
    }
}

pub(crate) fn run_model_pool_advice_to<W: Write>(
    provider: &dyn ChatProvider,
    output: &mut W,
) -> io::Result<()> {
    match provider.model_pool_status() {
        Ok(summary) => {
            let advice = model_pool_advice(&summary);
            record_and_write_summary(provider, "model_pool_advice", &advice, output)?;
            output.flush()
        }
        Err(error) => Err(evented_error(
            provider,
            "model_pool_advice_error",
            "advice",
            &error,
        )),
    }
}
