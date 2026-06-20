use std::path::Path;
use std::process::{Child, Output};
use std::thread;
use std::time::{Duration, Instant};

use crate::runtime::RuntimeError;

pub(in crate::runtime::command) fn wait_for_command_output(
    mut child: Child,
    timeout: Option<Duration>,
    program: &Path,
) -> Result<Output, RuntimeError> {
    let Some(timeout) = timeout else {
        return child.wait_with_output().map_err(|error| {
            RuntimeError::new(format!("failed to wait for runtime command: {error}"))
        });
    };
    let started = Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(_status)) => {
                return child.wait_with_output().map_err(|error| {
                    RuntimeError::new(format!("failed to wait for runtime command: {error}"))
                });
            }
            Ok(None) if started.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait_with_output();
                return Err(RuntimeError::new(format!(
                    "runtime command {} timed out after {} ms",
                    program.display(),
                    timeout.as_millis()
                )));
            }
            Ok(None) => thread::sleep(Duration::from_millis(10)),
            Err(error) => {
                return Err(RuntimeError::new(format!(
                    "failed to poll runtime command: {error}"
                )));
            }
        }
    }
}
