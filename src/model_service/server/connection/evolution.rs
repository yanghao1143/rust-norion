use std::net::TcpStream;

use rust_norion::{NoironEngine, StateInspectionReport, append_rust_check_trace_jsonl};

use super::super::super::feedback::{
    annotate_model_service_feedback_experience,
    annotate_model_service_feedback_experience_with_source,
    annotate_model_service_rust_check_experience, apply_model_service_feedback,
    model_service_feedback_memory_ids, model_service_rust_check_feedback_request,
};
use super::super::super::gates::{
    model_service_state_gate_report_for_request, model_service_trace_gate_report_for_request,
};
use super::super::super::json::{service_error_json, write_http_json};
use super::super::super::request::{
    ModelServiceFeedbackRequest, ModelServiceReplayRequest, ModelServiceRustCheckRequest,
    ModelServiceSelfImproveRequest,
};
use super::super::super::response::{
    model_service_feedback_response_json, model_service_replay_response_json,
    model_service_rust_check_response_json, model_service_self_improve_response_json,
};
use super::super::super::rust_check::model_service_rust_check_report;
use crate::Args;

pub(super) fn handle_replay(
    engine: &mut NoironEngine,
    args: &Args,
    stream: &mut TcpStream,
    request_id: usize,
    request: ModelServiceReplayRequest,
) -> std::io::Result<()> {
    let report = engine.replay_experience(request.limit);
    engine.save_full_state(
        &args.memory_path,
        &args.experience_path,
        &args.adaptive_path,
    )?;
    let inspection = StateInspectionReport::from_engine(engine, args.inspect_limit);
    let body = model_service_replay_response_json(request_id, request.limit, &report, &inspection);
    write_http_json(stream, 200, "OK", &body)
}

pub(super) fn handle_self_improve(
    engine: &mut NoironEngine,
    args: &Args,
    stream: &mut TcpStream,
    request_id: usize,
    request: ModelServiceSelfImproveRequest,
) -> std::io::Result<()> {
    let report = engine.replay_experience(request.limit);
    engine.save_full_state(
        &args.memory_path,
        &args.experience_path,
        &args.adaptive_path,
    )?;
    let inspection = StateInspectionReport::from_engine(engine, args.inspect_limit);
    let gate_report =
        model_service_state_gate_report_for_request(&request.inspect, &inspection, args);
    let trace_gate_report =
        match model_service_trace_gate_report_for_request(&request.inspect, args) {
            Ok(report) => report,
            Err(error) if error.kind() == std::io::ErrorKind::InvalidInput => {
                let body = service_error_json(&error.to_string());
                return write_http_json(stream, 400, "Bad Request", &body);
            }
            Err(error) => return Err(error),
        };
    let body = model_service_self_improve_response_json(
        request_id,
        &request,
        &report,
        &inspection,
        gate_report.as_ref(),
        trace_gate_report.as_ref(),
    );
    write_http_json(stream, 200, "OK", &body)
}

pub(super) fn handle_feedback(
    engine: &mut NoironEngine,
    args: &Args,
    stream: &mut TcpStream,
    request_id: usize,
    request: ModelServiceFeedbackRequest,
) -> std::io::Result<()> {
    let memory_ids = model_service_feedback_memory_ids(engine, &request);
    if memory_ids.is_empty() {
        let body = service_error_json(
            "feedback requires a known memory_id or an experience_id with stored/used memory",
        );
        return write_http_json(stream, 400, "Bad Request", &body);
    }
    let updates = apply_model_service_feedback(engine, &request, &memory_ids);
    engine.evolution_ledger.record_external_feedback(&updates);
    annotate_model_service_feedback_experience(engine, &request, &updates);
    engine.save_full_state(
        &args.memory_path,
        &args.experience_path,
        &args.adaptive_path,
    )?;
    let inspection = StateInspectionReport::from_engine(engine, args.inspect_limit);
    let body = model_service_feedback_response_json(
        request_id,
        &request,
        &memory_ids,
        &updates,
        &inspection,
    );
    write_http_json(stream, 200, "OK", &body)
}

pub(super) fn handle_rust_check(
    engine: &mut NoironEngine,
    args: &Args,
    stream: &mut TcpStream,
    request_id: usize,
    request: ModelServiceRustCheckRequest,
) -> std::io::Result<()> {
    let report = match model_service_rust_check_report(&request, "model-service-rust-check") {
        Ok(report) => report,
        Err(error) => {
            let body = service_error_json(&error.to_string());
            return write_http_json(stream, 400, "Bad Request", &body);
        }
    };
    let feedback_request = model_service_rust_check_feedback_request(&request, &report);
    let memory_ids = model_service_feedback_memory_ids(engine, &feedback_request);
    if (feedback_request.experience_id.is_some() || feedback_request.memory_id.is_some())
        && memory_ids.is_empty()
    {
        let body = service_error_json(
            "rust_check feedback requires a known memory_id or an experience_id with stored/used memory",
        );
        return write_http_json(stream, 400, "Bad Request", &body);
    }
    let updates = if memory_ids.is_empty() {
        Vec::new()
    } else {
        apply_model_service_feedback(engine, &feedback_request, &memory_ids)
    };
    if !updates.is_empty() {
        engine.evolution_ledger.record_external_feedback(&updates);
        annotate_model_service_feedback_experience_with_source(
            engine,
            &feedback_request,
            &updates,
            "rust_check",
        );
    }
    annotate_model_service_rust_check_experience(engine, &request, &report);
    if let Some(trace_path) = &args.trace_path {
        append_rust_check_trace_jsonl(
            trace_path,
            request.case_name.as_deref(),
            &report,
            feedback_request.action,
            feedback_request.amount,
            request.experience_id,
            request.memory_id,
            &memory_ids,
            &updates,
        )?;
    }
    engine.save_full_state(
        &args.memory_path,
        &args.experience_path,
        &args.adaptive_path,
    )?;
    let inspection = StateInspectionReport::from_engine(engine, args.inspect_limit);
    let body = model_service_rust_check_response_json(
        request_id,
        &request,
        &report,
        &feedback_request,
        &memory_ids,
        &updates,
        &inspection,
    );
    write_http_json(stream, 200, "OK", &body)
}
