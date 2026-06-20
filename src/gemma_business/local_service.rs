use std::thread::JoinHandle;

use rust_norion::{NoironEngine, RuntimeBackend};

use crate::Args;
use crate::model_service::http::reserve_model_service_loopback_addr;
use crate::model_service::server::run_model_service_for_args;

pub(super) struct GemmaLocalService {
    pub(super) args: Args,
    pub(super) bind: String,
    handle: JoinHandle<std::io::Result<()>>,
}

pub(super) fn start_gemma_local_model_service(
    engine: NoironEngine,
    args: &Args,
    max_requests: usize,
    missing_runtime_message: &'static str,
) -> std::io::Result<GemmaLocalService> {
    let mut service_args = args.clone();
    service_args.serve_bind = reserve_model_service_loopback_addr()?;
    service_args.serve_max_requests = Some(max_requests);
    let bind = service_args.serve_bind.clone();
    let server_args = service_args.clone();

    let handle = std::thread::spawn(move || -> std::io::Result<()> {
        let mut engine = engine;
        let runtime = server_args.command_runtime().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, missing_runtime_message)
        })?;
        let mut backend = RuntimeBackend::new(runtime);
        run_model_service_for_args(&mut engine, &mut backend, &server_args)
    });

    Ok(GemmaLocalService {
        args: service_args,
        bind,
        handle,
    })
}

pub(super) fn finish_gemma_local_model_service(
    service: GemmaLocalService,
    panic_message: &'static str,
    failures: &mut Vec<String>,
) -> std::io::Result<Args> {
    let server_result = service
        .handle
        .join()
        .map_err(|_| std::io::Error::other(panic_message))?;
    if let Err(error) = server_result {
        push_service_server_failure(&error.to_string(), failures);
    }
    Ok(service.args)
}

fn push_service_server_failure(error: &str, failures: &mut Vec<String>) {
    failures.push(format!("service server failed: {error}"));
}

#[cfg(test)]
mod tests {
    use super::push_service_server_failure;

    #[test]
    fn push_service_server_failure_formats_runtime_error() {
        let mut failures = Vec::new();

        push_service_server_failure("runtime missing", &mut failures);

        assert_eq!(failures, ["service server failed: runtime missing"]);
    }
}
