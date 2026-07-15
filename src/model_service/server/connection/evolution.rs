use std::net::TcpStream;
use std::time::Instant;

use rust_norion::{
    BenchmarkGateReport, GenomeEvolutionApplyReceipt, InferenceBackend, InferenceRequest,
    NoironEngine, SelfEvolutionAdmissionEvidence, SelfEvolutionAdmissionGate,
    SelfEvolutionAdmissionReport, StateInspectionReport, append_rust_check_trace_jsonl,
    append_self_evolution_admission_trace_jsonl, stable_redaction_digest,
};

use super::super::super::feedback::{
    ModelServiceBehaviorModelOutcomeUpdate, annotate_model_service_feedback_experience,
    annotate_model_service_feedback_experience_with_source,
    annotate_model_service_rust_check_experience, apply_model_service_behavior_feedback,
    apply_model_service_feedback, model_service_feedback_memory_ids,
    model_service_rust_check_feedback_request,
};
use super::super::super::gates::{
    model_service_state_gate_report_for_request, model_service_trace_gate_report_for_request,
};
use super::super::super::json::{service_error_json, service_json_string, write_http_json};
use super::super::super::newapi_fallback::persist_newapi_behavior_outcome_from_env;
use super::super::super::request::{
    ModelServiceEvolutionAction, ModelServiceEvolutionRequest, ModelServiceFeedbackRequest,
    ModelServiceReplayRequest, ModelServiceRustCheckRequest, ModelServiceSelfImproveRequest,
};
use super::super::super::response::{
    model_service_feedback_response_json, model_service_gene_residency_json,
    model_service_replay_response_json, model_service_rust_check_response_json,
    model_service_self_improve_response_json,
};
use super::super::super::rust_check::model_service_rust_check_report;
use super::super::state::{
    ModelServiceEvolutionCandidateLease, ModelServiceEvolutionTokenError, ModelServiceServerState,
};
use super::generation::runtime_error_note;
use crate::Args;
use crate::model_service::types::TimedOutcome;

#[derive(Debug, Clone, PartialEq, Eq)]
struct EvolutionBenefitMetrics {
    quality_milli: u16,
    process_reward_milli: i32,
    critical_reflection_issues: usize,
    contradiction_count: usize,
    output_integrity_passed: bool,
    elapsed_ms: u128,
    token_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EvolutionBenefitGateReport {
    executed: bool,
    passed: bool,
    reason: String,
    baseline: EvolutionBenefitMetrics,
    probe: Option<EvolutionBenefitMetrics>,
    probe_runtime_error: bool,
}

pub(super) fn handle_evolution<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
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
            let benefit_gate = evolution_benefit_gate(engine, backend, &lease, &approval_ref);
            if !benefit_gate.passed {
                let profile_state = engine.genome_runtime_state.profile(lease.preview.profile);
                let receipt = GenomeEvolutionApplyReceipt::held(
                    lease.preview.profile,
                    profile_state.generation,
                    profile_state.active.id.clone(),
                    format!("evolution_benefit_gate_failed:{}", benefit_gate.reason),
                );
                let body = evolution_response_json(
                    request_id,
                    request.action,
                    &lease.preview.candidate_digest,
                    "hold",
                    "held_for_benefit_gate",
                    &receipt,
                    &engine
                        .genome_runtime_state
                        .gene_residency_report(lease.preview.profile),
                    None,
                    Some(&benefit_gate),
                );
                return write_http_json(stream, 409, "Conflict", &body);
            }
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
                &engine
                    .genome_runtime_state
                    .gene_residency_report(lease.preview.profile),
                rollback_token.as_deref(),
                Some(&benefit_gate),
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
                &engine
                    .genome_runtime_state
                    .gene_residency_report(lease.profile),
                None,
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

fn evolution_benefit_gate<B: InferenceBackend>(
    engine: &NoironEngine,
    backend: &mut B,
    lease: &ModelServiceEvolutionCandidateLease,
    approval_ref: &str,
) -> EvolutionBenefitGateReport {
    let baseline = EvolutionBenefitMetrics::from_lease(lease);
    let mut shadow = engine.clone();
    let shadow_receipt =
        match shadow.apply_genome_evolution_preview(&lease.preview, approval_ref, &lease.scope) {
            Ok(report) => report.receipt,
            Err(receipt) => receipt,
        };
    if !shadow_receipt.applied {
        return EvolutionBenefitGateReport {
            executed: false,
            passed: false,
            reason: format!("shadow_apply_failed:{}", shadow_receipt.reason),
            baseline,
            probe: None,
            probe_runtime_error: false,
        };
    }

    let started = Instant::now();
    let outcome = shadow.infer(
        InferenceRequest::new(&lease.prompt, lease.preview.profile)
            .with_max_tokens(lease.max_tokens)
            .with_tenant_scope(lease.scope.clone()),
        backend,
    );
    let timed = TimedOutcome {
        outcome,
        elapsed_ms: started.elapsed().as_millis(),
    };
    EvolutionBenefitGateReport::evaluate(
        baseline,
        EvolutionBenefitMetrics::from_timed(&timed),
        runtime_error_note(&timed).is_some(),
    )
}

impl EvolutionBenefitMetrics {
    fn from_lease(lease: &ModelServiceEvolutionCandidateLease) -> Self {
        Self {
            quality_milli: lease.preview.quality_milli,
            process_reward_milli: lease.preview.process_reward_milli,
            critical_reflection_issues: lease.preview.critical_reflection_issues,
            contradiction_count: lease.preview.contradiction_count,
            output_integrity_passed: lease.preview.output_integrity_passed,
            elapsed_ms: lease.baseline_elapsed_ms,
            token_count: lease.baseline_token_count,
        }
    }

    fn from_timed(timed: &TimedOutcome) -> Self {
        let preview = &timed.outcome.genome_evolution_preview;
        Self {
            quality_milli: preview.quality_milli,
            process_reward_milli: preview.process_reward_milli,
            critical_reflection_issues: preview.critical_reflection_issues,
            contradiction_count: preview.contradiction_count,
            output_integrity_passed: preview.output_integrity_passed,
            elapsed_ms: timed.elapsed_ms,
            token_count: timed.outcome.runtime_token_metrics.token_count,
        }
    }
}

impl EvolutionBenefitGateReport {
    fn evaluate(
        baseline: EvolutionBenefitMetrics,
        probe: EvolutionBenefitMetrics,
        probe_runtime_error: bool,
    ) -> Self {
        let latency_regression_margin = (baseline.elapsed_ms / 4).max(100);
        let reason = if probe_runtime_error {
            Some("probe_runtime_error")
        } else if !probe.output_integrity_passed {
            Some("probe_output_integrity_failed")
        } else if probe.critical_reflection_issues > baseline.critical_reflection_issues {
            Some("critical_reflection_regression")
        } else if probe.contradiction_count > baseline.contradiction_count {
            Some("contradiction_regression")
        } else if probe.quality_milli.saturating_add(20) < baseline.quality_milli {
            Some("quality_regression")
        } else if probe.process_reward_milli.saturating_add(20) < baseline.process_reward_milli {
            Some("process_reward_regression")
        } else if probe.elapsed_ms
            > baseline
                .elapsed_ms
                .saturating_add(latency_regression_margin)
        {
            Some("latency_regression")
        } else {
            None
        };
        let latency_gain_margin = (baseline.elapsed_ms / 20).max(25);
        let improvement = if probe.quality_milli > baseline.quality_milli {
            Some("quality_improved")
        } else if probe.process_reward_milli > baseline.process_reward_milli {
            Some("process_reward_improved")
        } else if probe.critical_reflection_issues < baseline.critical_reflection_issues
            || probe.contradiction_count < baseline.contradiction_count
        {
            Some("reflection_improved")
        } else if probe.elapsed_ms.saturating_add(latency_gain_margin) < baseline.elapsed_ms {
            Some("latency_improved")
        } else if probe.token_count < baseline.token_count {
            Some("token_count_improved")
        } else {
            None
        };
        let passed = reason.is_none() && improvement.is_some();
        Self {
            executed: true,
            passed,
            reason: reason
                .or(improvement)
                .unwrap_or("no_measurable_benefit")
                .to_owned(),
            baseline,
            probe: Some(probe),
            probe_runtime_error,
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
    gene_residency: &rust_norion::GeneResidencyReport,
    rollback_token: Option<&str>,
    benefit_gate: Option<&EvolutionBenefitGateReport>,
) -> String {
    let rollback_token = rollback_token
        .map(service_json_string)
        .unwrap_or_else(|| "null".to_owned());
    let error = (!receipt.applied)
        .then(|| service_json_string(&receipt.reason))
        .unwrap_or_else(|| "null".to_owned());
    let benefit_gate = benefit_gate
        .map(evolution_benefit_gate_json)
        .unwrap_or_else(|| "null".to_owned());
    format!(
        "{{\"ok\":{},\"request_id\":{},\"action\":{},\"candidate_digest\":{},\"rollback_token\":{},\"error\":{},\"norion\":{{\"evolution_benefit_gate\":{},\"dna_closed_loop\":{{\"generation_before\":{},\"generation_after\":{},\"active_genome_id_after\":{},\"writer_gate_decision\":{},\"apply_plan_decision\":{},\"mutation_count\":{},\"dual_chain_committed\":{},\"express_chain_records\":{},\"memory_chain_records\":{},\"mutation_applied\":{},\"rollback_applied\":{},\"receipt_reason\":{},\"candidate_digest\":{},{} }},\"persistent_writes\":{},\"genome_write_allowed\":{},\"self_evolution_write_allowed\":{}}}}}",
        receipt.applied,
        request_id,
        service_json_string(action.as_str()),
        service_json_string(candidate_digest),
        rollback_token,
        error,
        benefit_gate,
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
        model_service_gene_residency_json(gene_residency),
        receipt.applied,
        receipt.applied,
        receipt.applied,
    )
}

fn evolution_benefit_gate_json(report: &EvolutionBenefitGateReport) -> String {
    let probe = report
        .probe
        .as_ref()
        .map(evolution_benefit_metrics_json)
        .unwrap_or_else(|| "null".to_owned());
    format!(
        "{{\"executed\":{},\"passed\":{},\"decision\":{},\"reason\":{},\"shadow_only\":true,\"probe_runtime_error\":{},\"baseline\":{},\"probe\":{}}}",
        report.executed,
        report.passed,
        service_json_string(if report.passed { "keep" } else { "hold" }),
        service_json_string(&report.reason),
        report.probe_runtime_error,
        evolution_benefit_metrics_json(&report.baseline),
        probe,
    )
}

fn evolution_benefit_metrics_json(metrics: &EvolutionBenefitMetrics) -> String {
    format!(
        "{{\"quality_milli\":{},\"process_reward_milli\":{},\"critical_reflection_issues\":{},\"contradiction_count\":{},\"output_integrity_passed\":{},\"elapsed_ms\":{},\"token_count\":{}}}",
        metrics.quality_milli,
        metrics.process_reward_milli,
        metrics.critical_reflection_issues,
        metrics.contradiction_count,
        metrics.output_integrity_passed,
        metrics.elapsed_ms,
        metrics.token_count,
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

    fn benefit_metrics(quality: u16, reward: i32, elapsed_ms: u128) -> EvolutionBenefitMetrics {
        EvolutionBenefitMetrics {
            quality_milli: quality,
            process_reward_milli: reward,
            critical_reflection_issues: 0,
            contradiction_count: 0,
            output_integrity_passed: true,
            elapsed_ms,
            token_count: 40,
        }
    }

    #[test]
    fn evolution_benefit_gate_keeps_gain_and_holds_latency_regression() {
        let baseline = benefit_metrics(800, 700, 1_000);
        let improved = EvolutionBenefitGateReport::evaluate(
            baseline.clone(),
            benefit_metrics(810, 700, 980),
            false,
        );
        assert!(improved.passed);
        assert_eq!(improved.reason, "quality_improved");

        let regressed =
            EvolutionBenefitGateReport::evaluate(baseline, benefit_metrics(800, 700, 1_400), false);
        assert!(!regressed.passed);
        assert_eq!(regressed.reason, "latency_regression");
    }

    #[test]
    fn evolution_response_refreshes_gene_residency_after_apply_and_rollback() {
        let engine = NoironEngine::new();
        let profile = rust_norion::TaskProfile::Coding;
        let residency = engine.genome_runtime_state.gene_residency_report(profile);
        let apply = GenomeEvolutionApplyReceipt {
            profile,
            generation_before: 1,
            generation_after: 2,
            genome_id_before: "genome:coding:generation:1".to_owned(),
            genome_id_after: "genome:coding:generation:2".to_owned(),
            mutation_count: 1,
            applied: true,
            rolled_back: false,
            express_chain_records: 7,
            memory_chain_records: 1,
            dual_chain_committed: true,
            reason: "mutation_applied".to_owned(),
        };
        let apply_json = evolution_response_json(
            7,
            ModelServiceEvolutionAction::Apply,
            "redaction-digest:candidate",
            "ready_for_explicit_apply",
            "ready_for_explicit_apply",
            &apply,
            &residency,
            Some("rollback-token"),
            None,
        );
        assert!(apply_json.contains("\"gene_residency\":{"), "{apply_json}");
        assert!(
            apply_json.contains("\"last_transition_reason\":"),
            "{apply_json}"
        );

        let rollback = GenomeEvolutionApplyReceipt {
            rolled_back: true,
            reason: "rollback_applied".to_owned(),
            ..apply
        };
        let rollback_json = evolution_response_json(
            8,
            ModelServiceEvolutionAction::Rollback,
            "redaction-digest:candidate",
            "rollback_ready",
            "rollback_applied",
            &rollback,
            &residency,
            None,
            None,
        );
        assert!(rollback_json.contains("\"rollback_applied\":true"));
        assert!(rollback_json.contains("\"gene_residency\":{"));
    }

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
    let mut behavior_model_outcome = None;
    let behavior_feedback_requested = request.source.as_deref()
        == Some("browser_behavior_validation")
        || request.capability_token.is_some();
    let experience_update = if behavior_feedback_requested {
        let Some(experience_id) = request.experience_id else {
            let body = service_error_json("behavior feedback requires experience_id");
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
        let lease = match state.consume_behavior_feedback(token, experience_id, scope) {
            Ok(lease) => lease,
            Err(error) => {
                let body = service_error_json(error.as_str());
                return write_http_json(stream, 409, "Conflict", &body);
            }
        };
        let Some(update) = apply_model_service_behavior_feedback(engine, &request) else {
            let body = service_error_json("behavior feedback experience was not found");
            return write_http_json(stream, 400, "Bad Request", &body);
        };
        if let Some(model) = lease.runtime_model {
            let ok = request.action.as_str() == "reinforce";
            behavior_model_outcome = Some(
                match persist_newapi_behavior_outcome_from_env(&model, &lease.task_kind, ok) {
                    Ok(applied) => ModelServiceBehaviorModelOutcomeUpdate {
                        applied,
                        ok,
                        model,
                        task_kind: lease.task_kind,
                        error: None,
                    },
                    Err(error) => ModelServiceBehaviorModelOutcomeUpdate {
                        applied: false,
                        ok,
                        model,
                        task_kind: lease.task_kind,
                        error: Some(format!("{:?}", error.kind()).to_ascii_lowercase()),
                    },
                },
            );
        }
        Some(update)
    } else {
        if memory_ids.is_empty() {
            let body = service_error_json(
                "feedback requires a known memory_id or an authorized behavior experience",
            );
            return write_http_json(stream, 400, "Bad Request", &body);
        }
        None
    };
    let updates = apply_model_service_feedback(engine, &request, &memory_ids);
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
        behavior_model_outcome.as_ref(),
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
