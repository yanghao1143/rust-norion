use std::{
    io::{self, Write},
    thread,
    time::Duration,
};

use crate::app::provider::ChatProvider;

mod format;
#[cfg(test)]
mod tests;

use format::{watch_error_report, watch_iteration_report};

pub(crate) fn run_model_pool_watch_to<W: Write>(
    provider: &dyn ChatProvider,
    interval: Duration,
    max_iterations: Option<usize>,
    output: &mut W,
) -> io::Result<()> {
    let mut iteration = 0usize;
    loop {
        iteration = iteration.saturating_add(1);
        writeln!(output, "{}", watch_iteration_report(iteration))?;
        match provider.model_pool_status() {
            Ok(summary) => {
                let _ = provider.record_event("model_pool_watch", &summary);
                writeln!(output, "{summary}")?;
            }
            Err(error) => {
                let report = watch_error_report(iteration, &error);
                let _ = provider.record_event("model_pool_watch_error", &report);
                writeln!(output, "{report}")?;
            }
        }
        output.flush()?;

        if max_iterations.is_some_and(|limit| iteration >= limit) {
            return Ok(());
        }
        if !interval.is_zero() {
            thread::sleep(interval);
        }
    }
}
