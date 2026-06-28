use std::net::TcpStream;

use rust_norion::{InferenceBackend, NoironEngine};

use super::super::super::business_cycle::{
    run_model_service_business_cycle, run_model_service_business_cycle_observed_cancelable,
    ModelServiceBusinessCycleEvent,
};
use super::super::super::json::{
    service_error_json, write_http_json, write_http_sse_headers, write_sse_event,
};
use super::super::super::request::ModelServiceBusinessCycleRequest;
use super::super::super::response::model_service_business_cycle_response_json;
use super::super::state::{ModelServiceLastInferenceTelemetry, ModelServiceServerState};
use super::{
    runtime_state_block_json, runtime_state_block_message, runtime_state_blocking_failures,
    runtime_state_stream_block_json,
};
use crate::Args;

pub(super) fn handle_business_cycle<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    state: &ModelServiceServerState,
    args: &Args,
    stream: &mut TcpStream,
    request_id: usize,
    request: ModelServiceBusinessCycleRequest,
) -> std::io::Result<()> {
    if let Some(failures) = runtime_state_blocking_failures(args) {
        let message = runtime_state_block_message(&failures);
        state.record_inference(ModelServiceLastInferenceTelemetry::error(
            request_id,
            "business-cycle",
            message.clone(),
        ));
        let body = runtime_state_block_json(request_id, "business-cycle", &message, &failures);
        return write_http_json(stream, 409, "Conflict", &body);
    }
    let report = match run_model_service_business_cycle(engine, backend, args, request) {
        Ok(report) => report,
        Err(error) if error.kind() == std::io::ErrorKind::InvalidInput => {
            state.record_inference(ModelServiceLastInferenceTelemetry::error(
                request_id,
                "business-cycle",
                error.to_string(),
            ));
            let body = service_error_json(&error.to_string());
            return write_http_json(stream, 400, "Bad Request", &body);
        }
        Err(error) => {
            state.record_inference(ModelServiceLastInferenceTelemetry::error(
                request_id,
                "business-cycle",
                error.to_string(),
            ));
            return Err(error);
        }
    };
    state.record_inference(ModelServiceLastInferenceTelemetry::from_timed(
        request_id,
        "business-cycle",
        &report.timed,
    ));
    let body = model_service_business_cycle_response_json(request_id, &report);
    write_http_json(stream, 200, "OK", &body)
}

pub(super) fn handle_business_cycle_stream<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    state: &ModelServiceServerState,
    args: &Args,
    stream: &mut TcpStream,
    request_id: usize,
    request: ModelServiceBusinessCycleRequest,
) -> std::io::Result<()> {
    if let Some(failures) = runtime_state_blocking_failures(args) {
        let message = runtime_state_block_message(&failures);
        state.record_inference(ModelServiceLastInferenceTelemetry::error(
            request_id,
            "business-cycle-stream",
            message.clone(),
        ));
        write_http_sse_headers(stream)?;
        write_sse_event(stream, "error", &message)?;
        let final_body = runtime_state_stream_block_json(
            request_id,
            "business-cycle-stream",
            &message,
            &failures,
        );
        write_sse_event(stream, "final", &final_body)?;
        write_sse_event(stream, "done", "[DONE]")?;
        return Ok(());
    }
    write_http_sse_headers(stream)?;
    write_sse_event(
        stream,
        "status",
        "rust-norion business cycle stream connected",
    )?;
    let mut write_error = None;
    let report = {
        let mut observer = |event: ModelServiceBusinessCycleEvent<'_>| {
            if write_error.is_some() {
                return;
            }
            let result = match event {
                ModelServiceBusinessCycleEvent::Stage(stage) => {
                    write_sse_event(stream, "stage", stage)
                }
                ModelServiceBusinessCycleEvent::Token(token) => {
                    write_sse_event(stream, "delta", &token.text)
                }
                ModelServiceBusinessCycleEvent::Meta(meta) => {
                    write_sse_event(stream, "meta", &meta)
                }
            };
            if let Err(error) = result {
                write_error = Some(error);
            }
        };
        let mut should_cancel = || state.is_cancel_requested(request_id);
        run_model_service_business_cycle_observed_cancelable(
            engine,
            backend,
            args,
            request,
            &mut observer,
            &mut should_cancel,
        )
    };
    if let Some(error) = write_error {
        return Err(error);
    }

    let report = match report {
        Ok(report) => report,
        Err(error) if error.kind() == std::io::ErrorKind::InvalidInput => {
            state.record_inference(ModelServiceLastInferenceTelemetry::error(
                request_id,
                "business-cycle-stream",
                error.to_string(),
            ));
            write_sse_event(stream, "error", &error.to_string())?;
            write_sse_event(stream, "done", "[DONE]")?;
            return Ok(());
        }
        Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {
            let cancellation = state.cancellation_intent(request_id);
            let message = cancellation
                .as_ref()
                .map(|cancellation| {
                    format!(
                        "{}; repair_factor={} retag_label={} reason={}",
                        error,
                        cancellation.repair_factor,
                        cancellation.retag_label,
                        cancellation.reason
                    )
                })
                .unwrap_or_else(|| error.to_string());
            state.record_inference(ModelServiceLastInferenceTelemetry::error(
                request_id,
                "business-cycle-stream",
                message.clone(),
            ));
            write_sse_event(stream, "error", &message)?;
            write_sse_event(stream, "done", "[DONE]")?;
            return Ok(());
        }
        Err(error) => {
            state.record_inference(ModelServiceLastInferenceTelemetry::error(
                request_id,
                "business-cycle-stream",
                error.to_string(),
            ));
            return Err(error);
        }
    };
    state.record_inference(ModelServiceLastInferenceTelemetry::from_timed(
        request_id,
        "business-cycle-stream",
        &report.timed,
    ));
    write_sse_event(
        stream,
        "meta",
        &format!(
            "business_cycle runtime_tokens={} elapsed_ms={}",
            report.timed.outcome.runtime_token_metrics.token_count, report.timed.elapsed_ms
        ),
    )?;
    let body = model_service_business_cycle_response_json(request_id, &report);
    write_sse_event(stream, "final", &body)?;
    write_sse_event(stream, "done", "[DONE]")
}

#[cfg(test)]
mod tests {
    use std::io::Read;
    use std::net::{TcpListener, TcpStream};

    use rust_norion::{GenerationContext, InferenceDraft, RewardAction, TaskProfile};

    use super::*;
    use crate::model_service::request::ModelServiceInspectRequest;

    #[derive(Default)]
    struct PanicBackend {
        generate_calls: usize,
    }

    impl InferenceBackend for PanicBackend {
        fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
            self.generate_calls += 1;
            panic!("runtime state guard should reject before business cycle generation")
        }
    }

    fn tcp_pair() -> (TcpStream, TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (server, _) = listener.accept().unwrap();
        (client, server)
    }

    fn dirty_state_args() -> Args {
        Args::parse(vec![
            "--memory".to_owned(),
            "legacy-memory.ndkv".to_owned(),
            "--experience".to_owned(),
            "legacy-experience.ndkv".to_owned(),
            "--adaptive".to_owned(),
            "legacy-adaptive.ndkv".to_owned(),
        ])
    }

    fn business_cycle_request() -> ModelServiceBusinessCycleRequest {
        ModelServiceBusinessCycleRequest {
            prompt: "do not write dirty runtime state".to_owned(),
            profile: Some(TaskProfile::Coding),
            case_name: None,
            max_tokens: None,
            feedback_action: RewardAction::Reinforce,
            feedback_amount: 0.5,
            rust_check_code: None,
            rust_check_edition: "2021".to_owned(),
            rust_check_case_name: None,
            self_improve: false,
            self_improve_limit: 1,
            pool_dispatch: None,
            pool_stage_dispatch: Vec::new(),
            inspect: ModelServiceInspectRequest::default(),
        }
    }

    #[test]
    fn business_cycle_handler_returns_runtime_state_bucket_block_json() {
        let args = dirty_state_args();
        let state = ModelServiceServerState::default();
        let mut engine = NoironEngine::new();
        let mut backend = PanicBackend::default();
        let (mut client, mut server) = tcp_pair();

        handle_business_cycle(
            &mut engine,
            &mut backend,
            &state,
            &args,
            &mut server,
            51,
            business_cycle_request(),
        )
        .unwrap();
        drop(server);

        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();
        assert_eq!(backend.generate_calls, 0);
        assert!(response.contains("HTTP/1.1 409 Conflict"));
        assert!(response.contains("\"endpoint\":\"business-cycle\""));
        assert!(response.contains("\"blocked_reason\":\"runtime_state_bucket\""));
        assert!(response.contains("\"persistent_writes\":false"));
    }

    #[test]
    fn business_cycle_stream_returns_runtime_state_bucket_block_final() {
        let args = dirty_state_args();
        let state = ModelServiceServerState::default();
        let mut engine = NoironEngine::new();
        let mut backend = PanicBackend::default();
        let (mut client, mut server) = tcp_pair();

        handle_business_cycle_stream(
            &mut engine,
            &mut backend,
            &state,
            &args,
            &mut server,
            52,
            business_cycle_request(),
        )
        .unwrap();
        drop(server);

        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();
        assert_eq!(backend.generate_calls, 0);
        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("event: error"));
        assert!(response.contains("event: final"));
        assert!(response.contains("\"endpoint\":\"business-cycle-stream\""));
        assert!(response.contains("\"stream_state\":\"blocked\""));
        assert!(response.contains("\"persistent_writes\":false"));
        assert!(response.contains("event: done"));
    }
}
