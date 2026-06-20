use std::net::TcpStream;

use rust_norion::{NoironEngine, StateInspectionReport};

use super::super::super::gates::{
    model_service_state_gate_report_for_request, model_service_trace_gate_report_for_request,
};
use super::super::super::json::{service_error_json, write_http_json};
use super::super::super::request::ModelServiceInspectRequest;
use super::super::super::response::model_service_state_response_json;
use crate::Args;

pub(super) fn handle_state(
    engine: &NoironEngine,
    args: &Args,
    stream: &mut TcpStream,
    request_id: usize,
) -> std::io::Result<()> {
    let inspection = StateInspectionReport::from_engine(engine, args.inspect_limit);
    let body = model_service_state_response_json(request_id, &inspection, None, None);
    write_http_json(stream, 200, "OK", &body)
}

pub(super) fn handle_inspect(
    engine: &NoironEngine,
    args: &Args,
    stream: &mut TcpStream,
    request_id: usize,
    request: ModelServiceInspectRequest,
) -> std::io::Result<()> {
    let inspection = StateInspectionReport::from_engine(engine, args.inspect_limit);
    let gate_report = model_service_state_gate_report_for_request(&request, &inspection, args);
    let trace_gate_report = match model_service_trace_gate_report_for_request(&request, args) {
        Ok(report) => report,
        Err(error) if error.kind() == std::io::ErrorKind::InvalidInput => {
            let body = service_error_json(&error.to_string());
            return write_http_json(stream, 400, "Bad Request", &body);
        }
        Err(error) => return Err(error),
    };
    let body = model_service_state_response_json(
        request_id,
        &inspection,
        gate_report.as_ref(),
        trace_gate_report.as_ref(),
    );
    write_http_json(stream, 200, "OK", &body)
}
