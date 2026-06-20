mod connection;
mod health;
mod state;

use std::net::TcpListener;
use std::sync::Mutex;

use rust_norion::{InferenceBackend, NoironEngine};

use self::connection::handle_model_service_connection_concurrent;
use self::state::ModelServiceServerState;
use super::json::{service_error_json, write_http_json};
use crate::Args;
use crate::path_utils::ensure_parent_dir;

pub(crate) fn run_model_service_for_args<B: InferenceBackend + Send>(
    engine: &mut NoironEngine,
    backend: &mut B,
    args: &Args,
) -> std::io::Result<()> {
    if let Some(trace_path) = &args.trace_path {
        ensure_parent_dir(trace_path)?;
    }
    let listener = TcpListener::bind(&args.serve_bind)?;
    let max_requests = args.serve_max_requests.unwrap_or(usize::MAX).max(1);
    println!("Noiron model service");
    println!("serve_bind: {}", args.serve_bind);
    println!("serve_max_requests: {}", max_requests);
    if let Some(path) = &args.model_pool_manifest_path {
        println!("model_pool_manifest: {}", path.display());
    }
    println!("memory_file: {}", args.memory_path.display());
    println!("experience_file: {}", args.experience_path.display());
    println!("adaptive_file: {}", args.adaptive_path.display());
    if let Some(trace_path) = &args.trace_path {
        println!("trace_file: {}", trace_path.display());
    }

    let engine = Mutex::new(engine);
    let backend = Mutex::new(backend);
    let state = ModelServiceServerState::default();
    std::thread::scope(|scope| -> std::io::Result<()> {
        for request_index in 0..max_requests {
            let (mut stream, _) = listener.accept()?;
            let engine = &engine;
            let backend = &backend;
            let state = &state;
            scope.spawn(move || {
                if let Err(error) = handle_model_service_connection_concurrent(
                    engine,
                    backend,
                    state,
                    args,
                    &mut stream,
                    request_index + 1,
                ) {
                    let body = service_error_json(&error.to_string());
                    let _ = write_http_json(&mut stream, 500, "Internal Server Error", &body);
                }
            });
        }

        Ok(())
    })
}
