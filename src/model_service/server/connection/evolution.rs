use std::net::TcpStream;

use rust_norion::{
    BenchmarkGateReport, NoironEngine, SelfEvolutionAdmissionEvidence, SelfEvolutionAdmissionGate,
    SelfEvolutionAdmissionReport, StateInspectionReport, append_rust_check_trace_jsonl,
    append_self_evolution_admission_trace_jsonl, stable_redaction_digest,
};

use super::super::super::feedback::{
    annotate_model_service_feedback_experience,
    annotate_model_service_feedback_experience_with_source,
    annotate_model_service_rust_check_experience, apply_model_service_behavior_feedback,
    apply_model_service_feedback, model_service_feedback_memory_ids,
    model_service_rust_check_feedback_request,
};
use super::super::super::gates::{
    model_service_state_gate_report_for_request, model_service_trace_gate_report_for_request,
};
use super::super::super::json::{service_error_json, service_json_string, write_http_json};
use super::super::super::request::{
    ModelServiceEvolutionAction, ModelServiceEvolutionRequest, ModelServiceFeedbackRequest,
    ModelServiceReplayRequest, ModelServiceRustCheckRequest, ModelServiceSelfImproveRequest,
};
use super::super::super::response::{
    model_service_feedback_response_json, model_service_replay_response_json,
    model_service_rust_check_response_json, model_service_self_improve_response_json,
};
use super::super::super::rust_check::model_service_rust_check_report;
use super::super::state::{ModelServiceEvolutionTokenError, ModelServiceServerState};
use crate::Args;

pub(super) fn handle_evolution(
    engine: &mut NoironEngine,
    state: &ModelServiceServerState,
    args: &Args,
    stream: &mut TcpStream,
    request_id: usize,
    request: ModelServiceEvolutionRequest,
) -> std::io::Result<()> {
    match request.action {
        ModelServiceEvolutionAction::Apply => {
            let lease = match state
                .consume_evolution_candidate(&request.token, &request.tenant_scope)
            {
                Ok(lease) => lease,
                Err(error) => {
                    return write_evolution_token_error(stream, request_id, request.action, error);
                }
            };
            let approval_ref = stable_redaction_digest([
                "model-service-explicit-genome-apply-v1",
                request.token.as_str(),
                lease.prompt_digest.as_str(),
                lease.preview.candidate_digest.as_str(),
            ]);
            let genome_state_before = engine.genome_runtime_state.clone();
            let (receipt, writer_gate_decision, apply_plan_decision) = match engine
                .apply_genome_evolution_preview(
                    &lease.preview,
                    &approval_ref,
                    &request.tenant_scope,
                ) {
                Ok(report) => (
                    report.receipt,
                    report.writer_gate.decision.as_str(),
                    report.apply_plan.decision.as_str(),
                ),
                Err(receipt) => (receipt, "hold", "held_for_candidate_state"),
            };
            if receipt.applied {
                persist_genome_state_or_restore(engine, args, genome_state_before)?;
            }
            let rollback_token = receipt.applied.then(|| {
                state.register_evolution_rollback(
                    request_id,
                    &request.tenant_scope,
                    receipt.profile,
                    receipt.generation_after,
                    &lease.preview.candidate_digest,
                )
            });
            let body = evolution_response_json(
                request_id,
                request.action,
                &lease.preview.candidate_digest,
                writer_gate_decision,
                apply_plan_decision,
                &receipt,
                rollback_token.as_deref(),
            );
            if receipt.applied {
                write_http_json(stream, 200, "OK", &body)
            } else {
                write_http_json(stream, 409, "Conflict", &body)
            }
        }
        ModelServiceEvolutionAction::Rollback => {
            let lease = match state
                .consume_evolution_rollback(&request.token, &request.tenant_scope)
            {
                Ok(lease) => lease,
                Err(error) => {
                    return write_evolution_token_error(stream, request_id, request.action, error);
                }
            };
            let approval_ref = stable_redaction_digest([
                "model-service-explicit-genome-rollback-v1",
                request.token.as_str(),
                lease.candidate_digest.as_str(),
            ]);
            let genome_state_before = engine.genome_runtime_state.clone();
            let receipt = engine.rollback_genome_evolution(
                lease.profile,
                lease.expected_generation,
                &approval_ref,
            );
            if receipt.applied {
                persist_genome_state_or_restore(engine, args, genome_state_before)?;
            }
            let body = evolution_response_json(
                request_id,
                request.action,
                &lease.candidate_digest,
                if receipt.applied {
                    "rollback_ready"
                } else {
                    "hold"
                },
                if receipt.applied {
                    "rollback_applied"
                } else {
                    "held_for_candidate_state"
                },
                &receipt,
                None,
            );
            if receipt.applied {
                write_http_json(stream, 200, "OK", &body)
            } else {
                write_http_json(stream, 409, "Conflict", &body)
            }
        }
    }
}

fn persist_genome_state_or_restore(
    engine: &mut NoironEngine,
    args: &Args,
    genome_state_before: rust_norion::GenomeRuntimeState,
) -> std::io::Result<()> {
    if let Err(error) = engine.save_full_state(
        &args.memory_path,
        &args.experience_path,
        &args.adaptive_path,
    ) {
        engine.genome_runtime_state = genome_state_before;
        let _ = engine.save_full_state(
            &args.memory_path,
            &args.experience_path,
            &args.adaptive_path,
        );
        return Err(error);
    }
    Ok(())
}

fn write_evolution_token_error(
    stream: &mut TcpStream,
    request_id: usize,
    action: ModelServiceEvolutionAction,
    error: ModelServiceEvolutionTokenError,
) -> std::io::Result<()> {
    let body = format!(
        "{{\"ok\":false,\"request_id\":{},\"action\":{},\"error\":{},\"retryable\":false}}",
        request_id,
        service_json_string(action.as_str()),
        service_json_string(error.as_str()),
    );
    let (status, reason) = match error {
        ModelServiceEvolutionTokenError::Missing => (409, "Conflict"),
        ModelServiceEvolutionTokenError::Expired => (410, "Gone"),
        ModelServiceEvolutionTokenError::ScopeMismatch => (403, "Forbidden"),
    };
    write_http_json(stream, status, reason, &body)
}

fn evolution_response_json(
    request_id: usize,
    action: ModelServiceEvolutionAction,
    candidate_digest: &str,
    writer_gate_decision: &str,
    apply_plan_decision: &str,
    receipt: &rust_norion::GenomeEvolutionApplyReceipt,
    rollback_token: Option<&str>,
) -> String {
    let rollback_token = rollback_token
        .map(service_json_string)
        .unwrap_or_else(|| "null".to_owned());
    format!(
        "{{\"ok\":{},\"request_id\":{},\"action\":{},\"candidate_digest\":{},\"rollback_token\":{},\"norion\":{{\"dna_closed_loop\":{{\"generation_before\":{},\"generation_after\":{},\"active_genome_id_after\":{},\"writer_gate_decision\":{},\"apply_plan_decision\":{},\"mutation_count\":{},\"dual_chain_committed\":{},\"express_chain_records\":{},\"memory_chain_records\":{},\"mutation_applied\":{},\"rollback_applied\":{},\"receipt_reason\":{},\"candidate_digest\":{}}},\"persistent_writes\":{},\"genome_write_allowed\":{},\"self_evolution_write_allowed\":{}}}}}",
        receipt.applied,
        request_id,
        service_json_string(action.as_str()),
        service_json_string(candidate_digest),
        rollback_token,
        receipt.generation_before,
        receipt.generation_after,
        service_json_string(&receipt.genome_id_after),
        service_json_string(writer_gate_decision),
        service_json_string(apply_plan_decision),
        receipt.mutation_count,
        receipt.dual_chain_committed,
        receipt.express_chain_records,
        receipt.memory_chain_records,
        receipt.applied && !receipt.rolled_back,
        receipt.rolled_back,
        service_json_string(&receipt.reason),
        service_json_string(candidate_digest),
        receipt.applied,
        receipt.applied,
        receipt.applied,
    )
}

pub(super) fn handle_replay(
    engine: &mut NoironEngine,
    args: &Args,
    stream: &mut TcpStream,
    request_id: usize,
    request: ModelServiceReplayRequest,
) -> std::io::Result<()> {
    let Some(scope) = request.tenant_scope.as_ref() else {
        let body = service_error_json("replay requires tenant_id, workspace_id, and session_id");
        return write_http_json(stream, 400, "Bad Request", &body);
    };
    let report = engine.replay_experience_scoped(request.limit, scope);
    engine.save_full_state(
        &args.memory_path,
        &args.experience_path,
        &args.adaptive_path,
    )?;
    let inspection = StateInspectionReport::from_engine_scoped(engine, args.inspect_limit, scope);
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
    let Some(scope) = request.inspect.tenant_scope.as_ref() else {
        let body =
            service_error_json("self-improve requires tenant_id, workspace_id, and session_id");
        return write_http_json(stream, 400, "Bad Request", &body);
    };
    let report = engine.replay_experience_scoped(request.limit, scope);
    engine.save_full_state(
        &args.memory_path,
        &args.experience_path,
        &args.adaptive_path,
    )?;
    let inspection = StateInspectionReport::from_engine_scoped(engine, args.inspect_limit, scope);
    let gate_report =
        model_service_state_gate_report_for_request(&request.inspect, &inspection, args);
    let admission_report = self_improve_admission_report(request_id, engine);
    append_self_improve_admission_trace_jsonl(args, &request, &admission_report)?;
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
        &admission_report,
    );
    write_http_json(stream, 200, "OK", &body)
}

fn self_improve_admission_report(
    request_id: usize,
    engine: &NoironEngine,
) -> SelfEvolutionAdmissionReport {
    let benchmark_gate = BenchmarkGateReport {
        passed: false,
        failures: vec![
            "self_evolution_admission_model_service_benchmark_gate_evidence_missing".to_owned(),
        ],
    };
    let evidence = SelfEvolutionAdmissionEvidence::from_benchmark_gate(
        format!("model-service-self-improve-{request_id}"),
        engine.evolution_ledger,
        &benchmark_gate,
    );
    SelfEvolutionAdmissionGate::new().evaluate(&evidence)
}

fn append_self_improve_admission_trace_jsonl(
    args: &Args,
    request: &ModelServiceSelfImproveRequest,
    report: &SelfEvolutionAdmissionReport,
) -> std::io::Result<()> {
    if let Some(trace_path) = &args.trace_path {
        append_self_evolution_admission_trace_jsonl(trace_path, report)?;
    }

    let trace_gate_enabled = request
        .inspect
        .trace_gate
        .unwrap_or_else(|| args.trace_schema_gate_path.is_some());
    let Some(trace_schema_gate_path) = &args.trace_schema_gate_path else {
        return Ok(());
    };
    if trace_gate_enabled && args.trace_path.as_ref() != Some(trace_schema_gate_path) {
        append_self_evolution_admission_trace_jsonl(trace_schema_gate_path, report)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use rust_norion::{NoironEngine, evaluate_trace_schema_jsonl};

    use super::*;
    use crate::model_service::request::ModelServiceInspectRequest;

    #[test]
    fn self_improve_admission_append_writes_distinct_trace_gate_path() {
        let asset_dir = std::env::temp_dir().join(format!(
            "rust-norion-self-improve-admission-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&asset_dir).unwrap();
        let trace_path = asset_dir.join("trace.jsonl");
        let trace_gate_path = asset_dir.join("trace-gate.jsonl");
        let args = Args::parse(vec![
            "--trace".to_owned(),
            trace_path.display().to_string(),
            "--trace-schema-gate".to_owned(),
            trace_gate_path.display().to_string(),
        ]);
        let request = ModelServiceSelfImproveRequest {
            limit: 1,
            require_deep_self_evolution: true,
            inspect: ModelServiceInspectRequest {
                trace_gate: Some(true),
                ..ModelServiceInspectRequest::default()
            },
        };
        let engine = NoironEngine::new();
        let report = self_improve_admission_report(42, &engine);

        append_self_improve_admission_trace_jsonl(&args, &request, &report).unwrap();

        let trace_report = evaluate_trace_schema_jsonl(&trace_path).unwrap();
        let gate_report = evaluate_trace_schema_jsonl(&trace_gate_path).unwrap();
        assert!(trace_report.passed, "{:?}", trace_report.failures);
        assert!(gate_report.passed, "{:?}", gate_report.failures);
        assert_eq!(trace_report.self_evolution_admission_events, 1);
        assert_eq!(gate_report.self_evolution_admission_events, 1);
        assert_eq!(gate_report.self_evolution_admission_review_packets, 1);
        assert_eq!(gate_report.self_evolution_admission_evidence_ids, 2);
        assert_eq!(
            gate_report.self_evolution_admission_missing_review_packet_refs,
            0
        );

        fs::remove_dir_all(asset_dir).unwrap();
    }
}

pub(super) fn handle_feedback(
    engine: &mut NoironEngine,
    state: &ModelServiceServerState,
    args: &Args,
    stream: &mut TcpStream,
    request_id: usize,
    request: ModelServiceFeedbackRequest,
) -> std::io::Result<()> {
    let Some(scope) = request.tenant_scope.as_ref() else {
        let body = service_error_json("feedback requires tenant_id, workspace_id, and session_id");
        return write_http_json(stream, 400, "Bad Request", &body);
    };
    let memory_ids = model_service_feedback_memory_ids(engine, &request);
    let experience_update = if memory_ids.is_empty() {
        let Some(experience_id) = request.experience_id else {
            let body = service_error_json(
                "feedback requires a known memory_id or an authorized behavior experience",
            );
            return write_http_json(stream, 400, "Bad Request", &body);
        };
        if request.source.as_deref() != Some("browser_behavior_validation") {
            let body = service_error_json(
                "experience-only feedback requires source browser_behavior_validation",
            );
            return write_http_json(stream, 400, "Bad Request", &body);
        }
        let Some(token) = request.capability_token.as_deref() else {
            let body = service_error_json("experience-only feedback requires capability_token");
            return write_http_json(stream, 400, "Bad Request", &body);
        };
        if let Err(error) = state.consume_behavior_feedback(token, experience_id, scope) {
            let body = service_error_json(error.as_str());
            return write_http_json(stream, 409, "Conflict", &body);
        }
        let Some(update) = apply_model_service_behavior_feedback(engine, &request) else {
            let body = service_error_json("behavior feedback experience was not found");
            return write_http_json(stream, 400, "Bad Request", &body);
        };
        Some(update)
    } else {
        None
    };
    let updates = if memory_ids.is_empty() {
        Vec::new()
    } else {
        apply_model_service_feedback(engine, &request, &memory_ids)
    };
    if !updates.is_empty() {
        engine.evolution_ledger.record_external_feedback(&updates);
        annotate_model_service_feedback_experience(engine, &request, &updates);
    }
    engine.save_full_state(
        &args.memory_path,
        &args.experience_path,
        &args.adaptive_path,
    )?;
    let inspection = StateInspectionReport::from_engine_scoped(engine, args.inspect_limit, scope);
    let body = model_service_feedback_response_json(
        request_id,
        &request,
        &memory_ids,
        &updates,
        experience_update.as_ref(),
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
    let local_scope;
    let scope = match request.tenant_scope.as_ref() {
        Some(scope) => scope,
        None => {
            local_scope = rust_norion::TenantScope::local_single_user();
            &local_scope
        }
    };
    let inspection = StateInspectionReport::from_engine_scoped(engine, args.inspect_limit, scope);
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
