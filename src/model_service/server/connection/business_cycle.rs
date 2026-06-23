use std::net::TcpStream;

use rust_norion::{InferenceBackend, NoironEngine};

use super::super::super::business_cycle::{
    ModelServiceBusinessCycleEvent, run_model_service_business_cycle,
    run_model_service_business_cycle_observed_cancelable,
};
use super::super::super::json::{
    service_error_json, write_http_json, write_http_sse_headers, write_sse_event,
};
use super::super::super::request::ModelServiceBusinessCycleRequest;
use super::super::super::response::model_service_business_cycle_response_json;
use super::super::state::{ModelServiceLastInferenceTelemetry, ModelServiceServerState};
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
