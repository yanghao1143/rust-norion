use super::*;
use crate::memory_admission::MemoryVerifierDecision;
use crate::writer_gate::{
    UnifiedWriterGate, UnifiedWriterGateCandidate, UnifiedWriterGateDecision,
    UnifiedWriterGateDomain, UnifiedWriterGateWriteScope,
};

#[test]
fn issue_246_generated_trace_contract_covers_shadow_drift_candidates() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("issue 246 generated trace contract", TaskProfile::Coding),
        &mut backend,
    );
    let line = trace_json_line(
        "issue 246 generated trace contract",
        TaskProfile::Coding,
        5,
        &outcome,
    );

    let failures = evaluate_trace_schema_line(&line);
    assert!(failures.is_empty(), "{failures:?}");

    let admission = json_object_after_field(&line, "memory_admission").unwrap();
    assert_eq!(extract_json_bool_field(admission, "read_only"), Some(true));
    assert_eq!(
        extract_json_bool_field(admission, "write_allowed"),
        Some(false)
    );
    assert_eq!(extract_json_bool_field(admission, "applied"), Some(false));
    assert_eq!(extract_json_usize_field(admission, "admitted"), Some(0));
    assert_eq!(
        extract_json_usize_field(admission, "ledger_authorized"),
        Some(0)
    );
    assert_eq!(
        extract_json_usize_field(admission, "ledger_applied"),
        Some(0)
    );
    assert_nonempty_shadow_summaries(
        extract_json_string_array_field(admission, "candidate_summaries").unwrap(),
    );

    let routing = json_object_after_field(&line, "adaptive_routing").unwrap();
    assert_eq!(
        extract_json_bool_field(routing, "write_allowed"),
        Some(false)
    );
    assert_eq!(extract_json_bool_field(routing, "applied"), Some(false));
    assert_nonempty_shadow_summaries(
        extract_json_string_array_field(routing, "score_summaries").unwrap(),
    );

    let rollback_line = rollback_trace_line();
    let genome = json_object_after_field(&rollback_line, "reasoning_genome").unwrap();
    assert_eq!(
        extract_json_bool_field(genome, "splice_write_allowed"),
        Some(false)
    );
    assert_eq!(
        extract_json_bool_field(genome, "splice_applied"),
        Some(false)
    );
    assert_nonempty_shadow_summaries(
        extract_json_string_array_field(genome, "splice_lifecycle_summaries").unwrap(),
    );

    let runtime_line = runtime_kv_segment_trace_line();
    let runtime = json_object_after_field(&runtime_line, "runtime_diagnostics").unwrap();
    assert_eq!(
        extract_json_bool_field(runtime, "has_runtime_kv_segment_signal"),
        Some(true)
    );
    assert_nonempty_shadow_summaries(
        extract_json_string_array_field(runtime, "runtime_kv_segment_lifecycle_summaries").unwrap(),
    );
}

#[test]
fn issue_246_apply_boundary_requires_operator_and_writer_gates() {
    let approval = issue_246_operator_approval_report();
    let approval_line = approval.json_line();
    let approval_failures = evaluate_trace_schema_line(&approval_line);
    assert!(approval_failures.is_empty(), "{approval_failures:?}");
    assert!(approval.operator_approved);
    assert!(approval_line.contains("\"shadow_state\":\"ready_for_explicit_apply\""));
    assert!(approval_line.contains("\"drift_state\":\"drift_passed\""));
    assert!(approval_line.contains("\"write_allowed\":false"));
    assert!(approval_line.contains("\"applied\":false"));

    let preflight = issue_246_promotion_preflight_report();
    let preflight_line = preflight.json_line();
    let preflight_failures = evaluate_trace_schema_line(&preflight_line);
    assert!(preflight_failures.is_empty(), "{preflight_failures:?}");
    assert!(preflight.ready_for_explicit_promotion);
    assert!(preflight.explicit_promotion_required);
    assert!(preflight_line.contains("\"shadow_state\":\"ready_for_explicit_apply\""));
    assert!(preflight_line.contains("\"drift_state\":\"drift_passed\""));
    assert!(preflight_line.contains("\"write_allowed\":false"));
    assert!(preflight_line.contains("\"applied\":false"));

    let writer = UnifiedWriterGate::new().evaluate([ready_writer_candidate()]);
    assert_eq!(writer.decision, UnifiedWriterGateDecision::PreviewOnly);
    assert!(writer.explicit_apply_required);
    assert!(!writer.durable_write_allowed);
    assert!(!writer.write_allowed);
    assert!(!writer.applied);
    assert!(writer.is_preview_only());
}

fn assert_nonempty_shadow_summaries(summaries: Vec<String>) {
    assert!(!summaries.is_empty());
    for summary in summaries {
        for marker in [
            "shadow_state=",
            "drift_state=",
            "source_ids=",
            "expires_after_steps=",
            "score_milli=",
            "drift_gate_domains=",
            "rollback=",
        ] {
            assert!(summary.contains(marker), "{summary}");
        }
        for domain in [
            "golden_fixture:",
            "routing_behavior:",
            "memory_hygiene:",
            "privacy:",
            "trace_schema:",
        ] {
            assert!(summary.contains(domain), "{summary}");
        }
        assert!(
            summary.contains("write_allowed=false") || !summary.contains("write_allowed="),
            "{summary}"
        );
        assert!(
            summary.contains("applied=false") || !summary.contains("applied="),
            "{summary}"
        );
    }
}

fn runtime_kv_segment_trace_line() -> String {
    struct SegmentBackend;

    impl InferenceBackend for SegmentBackend {
        fn generate(&mut self, context: GenerationContext<'_>) -> InferenceDraft {
            let diagnostics = RuntimeDiagnostics {
                model_id: Some("issue-246-runtime-kv-segment".to_owned()),
                selected_adapter: Some("portable-rust".to_owned()),
                runtime_kv_segments_included: 1,
                runtime_kv_segments_skipped: 1,
                runtime_kv_segments_rejected: 1,
                ..RuntimeDiagnostics::default()
            }
            .with_device_execution(
                context.hardware_plan.device.as_str(),
                context.hardware_plan.execution.primary_lane.as_str(),
                context.hardware_plan.execution.fallback_lane.as_str(),
                context.hardware_plan.execution.memory_mode.as_str(),
            )
            .with_kv_precision(
                context.hardware_plan.execution.hot_kv_precision_bits,
                context.hardware_plan.execution.cold_kv_precision_bits,
            );

            InferenceDraft::new(
                "Runtime KV segment shadow evidence is generated.",
                vec![ReasoningStep::new(
                    "runtime_kv_segment",
                    "segment shadow evidence",
                    0.8,
                )],
            )
            .with_runtime_diagnostics(diagnostics)
        }
    }

    let mut engine = NoironEngine::new();
    let mut backend = SegmentBackend;
    let outcome = engine.infer(
        InferenceRequest::new("issue 246 runtime kv segment", TaskProfile::Coding),
        &mut backend,
    );
    trace_json_line(
        "issue 246 runtime kv segment",
        TaskProfile::Coding,
        5,
        &outcome,
    )
}

fn issue_246_operator_approval_report() -> SelfEvolutionOperatorApprovalReport {
    let router_preview = RouterThresholdAdjustmentPreviewPlanner::new().preview(
        NoironRouter::new().state(),
        TaskProfile::Coding,
        GenerationMetrics {
            perplexity: 36.0,
            semantic_consistency: 0.20,
            contradiction_count: 2,
            token_count: 64,
        },
    );
    let evidence = SelfEvolutionAdmissionEvidence::from_benchmark_gate(
        "issue-246-operator-approval",
        EvolutionLedger {
            replay_rust_check_items: 1,
            replay_rust_check_passed: 1,
            replay_rust_check_failed: 0,
            drift_rollbacks: 1,
            rollback_router_threshold_delta: 0.02,
            rollback_hierarchy_weight_delta: 0.03,
            ..EvolutionLedger::default()
        },
        &BenchmarkGateReport {
            passed: true,
            failures: Vec::new(),
        },
    )
    .with_validation_evidence(SelfEvolutionValidationEvidence::from_lanes(
        SelfEvolutionValidationLane::new(1, 1, 0),
        SelfEvolutionValidationLane::new(1, 1, 0),
        SelfEvolutionValidationLane::new(1, 1, 0),
        SelfEvolutionValidationLane::new(1, 1, 0),
    ))
    .with_router_threshold_preview_report(&router_preview);
    let admission = SelfEvolutionAdmissionGate::new().evaluate(&evidence);
    let mut ledger = SelfEvolutionExperimentLedger::new();
    ledger.append_admission_report("issue-246-operator-experiment", &admission);
    let replay_gate =
        SelfEvolutionRollbackReplayGate::new().evaluate(&ledger.rollback_replay_plan());
    let approval_evidence = SelfEvolutionOperatorApprovalEvidence::from_review_packet(
        "maintainer-jy",
        "issue-246-operator-ticket",
        &replay_gate.review_packet,
        "approved for issue 246 contract",
    );

    SelfEvolutionOperatorApprovalGate::new()
        .evaluate(&replay_gate.review_packet, &approval_evidence)
}

fn issue_246_promotion_preflight_report() -> SelfEvolutionPromotionPreflightReport {
    let router_preview = RouterThresholdAdjustmentPreviewPlanner::new().preview(
        NoironRouter::new().state(),
        TaskProfile::Coding,
        GenerationMetrics {
            perplexity: 36.0,
            semantic_consistency: 0.20,
            contradiction_count: 2,
            token_count: 64,
        },
    );
    let evidence = SelfEvolutionAdmissionEvidence::from_benchmark_gate(
        "issue-246-promotion-preflight",
        EvolutionLedger {
            replay_rust_check_items: 2,
            replay_rust_check_passed: 2,
            replay_rust_check_failed: 0,
            ..EvolutionLedger::default()
        },
        &BenchmarkGateReport {
            passed: true,
            failures: Vec::new(),
        },
    )
    .with_validation_evidence(SelfEvolutionValidationEvidence::from_lanes(
        SelfEvolutionValidationLane::new(2, 2, 0),
        SelfEvolutionValidationLane::new(3, 3, 0),
        SelfEvolutionValidationLane::new(1, 1, 0),
        SelfEvolutionValidationLane::new(1, 1, 0),
    ))
    .with_router_threshold_preview_report(&router_preview);
    let admission = SelfEvolutionAdmissionGate::new().evaluate(&evidence);
    let mut ledger = SelfEvolutionExperimentLedger::new();
    let experiment = ledger.append_admission_report("issue-246-promotion-experiment", &admission);
    let approval_evidence = SelfEvolutionOperatorApprovalEvidence::from_review_packet(
        "maintainer-jy",
        "issue-246-promotion-ticket",
        &admission.review_packet,
        "approved for issue 246 promotion contract",
    );
    let approval = SelfEvolutionOperatorApprovalGate::new()
        .evaluate(&admission.review_packet, &approval_evidence);

    SelfEvolutionPromotionPreflightGate::new().evaluate(&admission, &experiment, &approval)
}

fn ready_writer_candidate() -> UnifiedWriterGateCandidate {
    UnifiedWriterGateCandidate::new(
        UnifiedWriterGateDomain::Memory,
        "issue-246-writer-ready",
        [UnifiedWriterGateWriteScope::DurableMemory],
    )
    .with_refs(
        vec!["review:issue-246".to_owned()],
        vec!["evidence:issue-246".to_owned()],
        vec!["rollback:issue-246".to_owned()],
        vec!["fnv64:issue246content".to_owned()],
        vec!["rust-norion-issue-246-contract-v1".to_owned()],
    )
    .with_evidence(true, true, true, true, true)
    .with_verifier_cluster(
        MemoryVerifierDecision::Pass,
        MemoryVerifierDecision::Pass,
        MemoryVerifierDecision::Pass,
        MemoryVerifierDecision::Pass,
    )
    .with_operator_approval(true, true)
}
