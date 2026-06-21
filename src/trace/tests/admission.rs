use super::*;

fn admitted_self_evolution_admission_line() -> String {
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
        "trace-admission",
        EvolutionLedger {
            replay_rust_check_items: 1,
            replay_rust_check_passed: 1,
            replay_rust_check_failed: 0,
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
    let report = SelfEvolutionAdmissionGate::new().evaluate(&evidence);

    assert!(report.admitted_for_human_review);
    report.json_line()
}

fn operator_approval_trace_report(approved: bool) -> SelfEvolutionOperatorApprovalReport {
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
        "operator-approval-trace",
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
    ledger.append_admission_report("operator-approval-experiment", &admission);
    let replay_gate =
        SelfEvolutionRollbackReplayGate::new().evaluate(&ledger.rollback_replay_plan());
    let mut approval_evidence = SelfEvolutionOperatorApprovalEvidence::from_review_packet(
        "maintainer-jy",
        "operator-approval-ticket",
        &replay_gate.review_packet,
        "approved for operator approval trace schema validation",
    );
    if !approved {
        approval_evidence.approval_ticket_id.clear();
    }
    let approval = SelfEvolutionOperatorApprovalGate::new()
        .evaluate(&replay_gate.review_packet, &approval_evidence);

    assert_eq!(approval.operator_approved, approved);
    approval
}

fn operator_approval_trace_line(approved: bool) -> String {
    operator_approval_trace_report(approved).json_line()
}

fn rollback_replay_apply_trace_report(ready: bool) -> SelfEvolutionRollbackReplayApplyReport {
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
        "rollback-replay-apply-trace",
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
    ledger.append_admission_report("rollback-replay-apply-experiment", &admission);
    let replay_gate =
        SelfEvolutionRollbackReplayGate::new().evaluate(&ledger.rollback_replay_plan());
    let mut approval_evidence = SelfEvolutionOperatorApprovalEvidence::from_review_packet(
        "maintainer-jy",
        "rollback-replay-apply-ticket",
        &replay_gate.review_packet,
        "approved for rollback replay apply trace schema validation",
    );
    if !ready {
        approval_evidence.approval_ticket_id.clear();
    }
    let approval = SelfEvolutionOperatorApprovalGate::new()
        .evaluate(&replay_gate.review_packet, &approval_evidence);
    let report = SelfEvolutionRollbackReplayApplyGate::new().evaluate(&replay_gate, &approval);

    assert_eq!(report.ready_for_operator_apply, ready);
    report
}

fn rollback_replay_apply_trace_line(ready: bool) -> String {
    rollback_replay_apply_trace_report(ready).json_line()
}

#[test]
fn self_evolution_admission_trace_schema_accepts_read_only_packet() {
    let line = admitted_self_evolution_admission_line();
    let failures = evaluate_trace_schema_line(&line);

    assert!(line.contains("\"schema\":\"rust-norion-self-evolution-admission-v1\""));
    assert!(line.contains("\"read_only\":true"));
    assert!(line.contains("\"review_packet\":{"));
    assert!(line.contains("\"approval_review_packet_ids\":[\"approval-review:trace-admission\"]"));
    assert!(line.contains("\"approval_tokens_included\":false"));
    assert!(line.contains("\"memory_store_allowed\":false"));
    assert!(failures.is_empty(), "{failures:?}");

    let path = temp_path("self-evolution-admission-trace-schema");
    fs::write(&path, format!("{line}\n")).unwrap();
    let report = evaluate_trace_schema_jsonl(&path).unwrap();

    assert!(report.passed, "{:?}", report.failures);
    assert_eq!(report.checked_lines, 1);
    assert_eq!(report.self_evolution_admission_events, 1);
    assert_eq!(report.self_evolution_admission_admitted, 1);
    assert_eq!(report.self_evolution_admission_blocked, 0);
    assert_eq!(report.self_evolution_admission_review_packets, 1);
    assert_eq!(report.self_evolution_admission_evidence_ids, 4);
    assert_eq!(
        report.self_evolution_admission_missing_review_packet_refs,
        0
    );
    assert!(
        report
            .summary_line()
            .contains("self_evolution_admission_events=1")
    );
    assert!(
        report
            .summary_line()
            .contains("self_evolution_admission_review_packets=1")
    );
    cleanup(path);
}

#[test]
fn self_evolution_rollback_replay_apply_trace_schema_accepts_ready_preflight() {
    let line = rollback_replay_apply_trace_line(true);
    let failures = evaluate_trace_schema_line(&line);

    assert!(line.contains("\"schema\":\"rust-norion-self-evolution-rollback-replay-apply-v1\""));
    assert!(line.contains("\"decision\":\"ready_for_operator_apply\""));
    assert!(line.contains("\"ready_for_operator_apply\":true"));
    assert!(line.contains("\"explicit_apply_required\":true"));
    assert!(line.contains("\"rollback_gate_admitted_for_human_review\":true"));
    assert!(line.contains("\"operator_approved\":true"));
    assert!(line.contains("\"read_only\":true"));
    assert!(line.contains("\"report_only\":true"));
    assert!(line.contains("\"preview_only\":true"));
    assert!(line.contains("\"write_allowed\":false"));
    assert!(line.contains("\"blocked_reasons_count\":0"));
    assert!(line.contains("\"blocked_reasons_digest\":\"fnv64:"));
    assert!(line.contains("\"content_digest\":\"fnv64:"));
    assert!(!line.contains("\"approval_review_packet_ids\":["));
    assert!(!line.contains("\"blocked_reasons\":["));
    assert!(!line.contains("rollback-replay-apply-ticket"));
    assert!(failures.is_empty(), "{failures:?}");

    let path = temp_path("self-evolution-rollback-replay-apply-trace-schema");
    fs::write(&path, format!("{line}\n")).unwrap();
    let report = evaluate_trace_schema_jsonl(&path).unwrap();

    assert!(report.passed, "{:?}", report.failures);
    assert_eq!(report.checked_lines, 1);
    assert_eq!(report.self_evolution_rollback_replay_apply_events, 1);
    assert_eq!(report.self_evolution_rollback_replay_apply_ready, 1);
    assert_eq!(report.self_evolution_rollback_replay_apply_held, 0);
    assert_eq!(report.self_evolution_rollback_replay_apply_items, 1);
    assert_eq!(report.self_evolution_rollback_replay_apply_replayable, 1);
    assert_eq!(report.self_evolution_rollback_replay_apply_blocked, 0);
    assert_eq!(report.self_evolution_rollback_replay_apply_write_allowed, 0);
    assert_eq!(report.self_evolution_rollback_replay_apply_applied, 0);
    assert!(
        report
            .summary_line()
            .contains("self_evolution_rollback_replay_apply_ready=1")
    );
    cleanup(path);
}

#[test]
fn self_evolution_rollback_replay_apply_trace_schema_accepts_hold() {
    let line = rollback_replay_apply_trace_line(false);
    let failures = evaluate_trace_schema_line(&line);

    assert!(line.contains("\"decision\":\"hold\""));
    assert!(line.contains("\"ready_for_operator_apply\":false"));
    assert!(line.contains("\"operator_approved\":false"));
    assert!(line.contains("\"blocked_reasons_count\":"));
    assert!(!line.contains("\"blocked_reasons\":["));
    assert!(!line.contains("rollback-replay-apply-ticket"));
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn self_evolution_rollback_replay_apply_trace_append_is_gate_consumable() {
    let apply_report = rollback_replay_apply_trace_report(true);
    let path = temp_path("self-evolution-rollback-replay-apply-trace-append");

    crate::append_self_evolution_rollback_replay_apply_trace_jsonl(&path, &apply_report).unwrap();
    let gate = evaluate_trace_schema_jsonl(&path).unwrap();

    assert!(gate.passed, "{:?}", gate.failures);
    assert_eq!(gate.checked_lines, 1);
    assert_eq!(gate.self_evolution_rollback_replay_apply_events, 1);
    assert_eq!(gate.self_evolution_rollback_replay_apply_ready, 1);
    assert_eq!(gate.self_evolution_rollback_replay_apply_held, 0);
    assert_eq!(
        gate.self_evolution_rollback_replay_apply_review_packets,
        apply_report.review_packet_count
    );
    assert_eq!(
        gate.self_evolution_rollback_replay_apply_evidence_ids,
        apply_report.evidence_id_count
    );
    assert_eq!(
        gate.self_evolution_rollback_replay_apply_rollback_anchor_ids,
        apply_report.rollback_anchor_count
    );
    assert_eq!(
        gate.self_evolution_rollback_replay_apply_content_digests,
        apply_report.content_digest_count
    );
    assert_eq!(
        gate.self_evolution_rollback_replay_apply_source_report_schemas,
        apply_report.source_report_schema_count
    );
    assert_eq!(gate.self_evolution_rollback_replay_apply_missing_refs, 0);
    assert_eq!(gate.self_evolution_rollback_replay_apply_write_allowed, 0);
    assert_eq!(gate.self_evolution_rollback_replay_apply_applied, 0);
    cleanup(path);
}

#[test]
fn self_evolution_rollback_replay_apply_trace_schema_rejects_write_or_raw_refs() {
    let line = rollback_replay_apply_trace_line(true)
        .replacen("\"write_allowed\":false", "\"write_allowed\":true", 1)
        .replacen(
            "\"review_packet_count\":",
            "\"approval_review_packet_ids\":[\"raw-ref\"],\"review_packet_count\":",
            1,
        );
    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("write_allowed=true")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("approval_review_packet_ids as count/digest only")),
        "{failures:?}"
    );
}

#[test]
fn self_evolution_rollback_replay_apply_trace_schema_rejects_mismatched_ready_decision() {
    let line = rollback_replay_apply_trace_line(true).replacen(
        "\"ready_for_operator_apply\":true",
        "\"ready_for_operator_apply\":false",
        1,
    );
    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures.iter().any(
            |failure| failure.contains("ready decision requires ready_for_operator_apply=true")
        ),
        "{failures:?}"
    );
}

#[test]
fn self_evolution_operator_approval_trace_schema_accepts_redacted_read_only_approval() {
    let line = operator_approval_trace_line(true);
    let failures = evaluate_trace_schema_line(&line);

    assert!(line.contains("\"schema\":\"rust-norion-self-evolution-operator-approval-v1\""));
    assert!(line.contains("\"decision\":\"approved\""));
    assert!(line.contains("\"operator_approved\":true"));
    assert!(line.contains("\"operator_digest\":\"fnv64:"));
    assert!(line.contains("\"approval_ticket_digest\":\"fnv64:"));
    assert!(line.contains("\"approval_reason_digest\":\"fnv64:"));
    assert!(line.contains("\"approved_review_packet_count\":1"));
    assert!(!line.contains("\"operator_id\":"));
    assert!(!line.contains("\"approval_ticket_id\":"));
    assert!(!line.contains("\"approval_reason\":"));
    assert!(!line.contains("maintainer-jy"));
    assert!(!line.contains("operator-approval-ticket"));
    assert!(failures.is_empty(), "{failures:?}");

    let path = temp_path("self-evolution-operator-approval-trace-schema");
    fs::write(&path, format!("{line}\n")).unwrap();
    let report = evaluate_trace_schema_jsonl(&path).unwrap();

    assert!(report.passed, "{:?}", report.failures);
    assert_eq!(report.checked_lines, 1);
    assert_eq!(report.self_evolution_operator_approval_events, 1);
    assert_eq!(report.self_evolution_operator_approval_approved, 1);
    assert_eq!(report.self_evolution_operator_approval_held, 0);
    assert_eq!(report.self_evolution_operator_approval_review_packets, 1);
    assert!(report.self_evolution_operator_approval_evidence_ids > 0);
    assert_eq!(report.self_evolution_operator_approval_write_allowed, 0);
    assert_eq!(report.self_evolution_operator_approval_applied, 0);
    assert!(
        report
            .summary_line()
            .contains("self_evolution_operator_approval_events=1")
    );
    cleanup(path);
}

#[test]
fn self_evolution_operator_approval_trace_schema_accepts_redacted_hold() {
    let line = operator_approval_trace_line(false);
    let failures = evaluate_trace_schema_line(&line);

    assert!(line.contains("\"decision\":\"hold\""));
    assert!(line.contains("\"operator_approved\":false"));
    assert!(line.contains("\"blocked_reasons_count\":"));
    assert!(!line.contains("\"blocked_reasons\":["));
    assert!(!line.contains("operator-approval-ticket"));
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn self_evolution_operator_approval_trace_append_is_gate_consumable() {
    let approval = operator_approval_trace_report(true);
    let path = temp_path("self-evolution-operator-approval-trace-append");

    crate::append_self_evolution_operator_approval_trace_jsonl(&path, &approval).unwrap();
    let gate = evaluate_trace_schema_jsonl(&path).unwrap();

    assert!(gate.passed, "{:?}", gate.failures);
    assert_eq!(gate.checked_lines, 1);
    assert_eq!(gate.self_evolution_operator_approval_events, 1);
    assert_eq!(gate.self_evolution_operator_approval_approved, 1);
    assert_eq!(gate.self_evolution_operator_approval_held, 0);
    assert_eq!(gate.self_evolution_operator_approval_review_packets, 1);
    assert!(gate.self_evolution_operator_approval_evidence_ids > 0);
    assert!(gate.self_evolution_operator_approval_rollback_anchor_ids > 0);
    assert!(gate.self_evolution_operator_approval_content_digests > 0);
    assert!(gate.self_evolution_operator_approval_source_report_schemas > 0);
    assert_eq!(
        gate.self_evolution_operator_approval_missing_review_packet_refs,
        0
    );
    assert_eq!(gate.self_evolution_operator_approval_write_allowed, 0);
    assert_eq!(gate.self_evolution_operator_approval_applied, 0);
    cleanup(path);
}

#[test]
fn self_evolution_operator_approval_trace_schema_rejects_write_or_raw_payload() {
    let line = operator_approval_trace_line(true)
        .replacen("\"write_allowed\":false", "\"write_allowed\":true", 1)
        .replacen(
            "\"operator_digest\":\"",
            "\"operator_id\":\"maintainer-jy\",\"operator_digest\":\"",
            1,
        );
    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("write_allowed=true")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("must not expose raw operator_id")),
        "{failures:?}"
    );
}

#[test]
fn self_evolution_operator_approval_trace_schema_rejects_mismatched_decision() {
    let line = operator_approval_trace_line(true).replacen(
        "\"operator_approved\":true",
        "\"operator_approved\":false",
        1,
    );
    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("approved decision requires operator_approved=true")),
        "{failures:?}"
    );
}

#[test]
fn self_evolution_admission_trace_append_is_gate_consumable() {
    let router_preview = RouterThresholdAdjustmentPreviewPlanner::new().preview(
        NoironRouter::new().state(),
        TaskProfile::Coding,
        GenerationMetrics {
            perplexity: f32::NAN,
            semantic_consistency: 0.20,
            contradiction_count: 0,
            token_count: 64,
        },
    );
    let evidence = SelfEvolutionAdmissionEvidence::from_benchmark_gate(
        "trace-admission-append",
        EvolutionLedger::default(),
        &BenchmarkGateReport {
            passed: false,
            failures: vec!["benchmark_gate_missing".to_owned()],
        },
    )
    .with_router_threshold_preview_report(&router_preview);
    let report = SelfEvolutionAdmissionGate::new().evaluate(&evidence);
    assert!(!report.admitted_for_human_review);

    let path = temp_path("self-evolution-admission-trace-append");
    append_self_evolution_admission_trace_jsonl(&path, &report).unwrap();
    let gate = evaluate_trace_schema_jsonl(&path).unwrap();

    assert!(gate.passed, "{:?}", gate.failures);
    assert_eq!(gate.checked_lines, 1);
    assert_eq!(gate.self_evolution_admission_events, 1);
    assert_eq!(gate.self_evolution_admission_admitted, 0);
    assert_eq!(gate.self_evolution_admission_blocked, 1);
    assert_eq!(gate.self_evolution_admission_review_packets, 1);
    assert!(gate.self_evolution_admission_evidence_ids >= 3);
    assert_eq!(gate.self_evolution_admission_missing_review_packet_refs, 0);
    cleanup(path);
}

#[test]
fn self_evolution_admission_trace_schema_accepts_blocked_unsafe_adaptive_preview() {
    let recall_report = crate::split::agent::AgentRecallOutcomeAttributionReport {
        attributions: vec![crate::split::agent::AgentRecallOutcomeAttribution {
            task_id: "runtime-recall".to_owned(),
            record_id: "runtime_kv:l0h0:0-8".to_owned(),
            source: "runtime_kv".to_owned(),
            action: crate::split::agent::AgentRecallOutcomeAttributionAction::Penalize,
            amount: 0.32,
            reason_codes: vec!["execution_failed".to_owned()],
        }],
        reinforced_count: 0,
        penalized_count: 1,
        skipped_rejected_recall_count: 0,
        skipped_missing_outcome_task_ids: Vec::new(),
        read_only: false,
        memory_store_write_allowed: true,
        telemetry: Vec::new(),
    };
    let reward_preview =
        crate::split::bridge::recall_outcome_attribution_to_kv_fusion_reward_preview(
            &recall_report,
        );
    let kv_preview = crate::split::bridge::kv_fusion_reward_policy_observation_dry_run(
        &reward_preview,
        crate::split::core::ReinforcedKvFusionPolicy::new(0.92, 64),
    );
    let evidence = SelfEvolutionAdmissionEvidence::from_benchmark_gate(
        "trace-blocked-unsafe-kv-preview",
        EvolutionLedger {
            replay_rust_check_items: 1,
            replay_rust_check_passed: 1,
            replay_rust_check_failed: 0,
            ..EvolutionLedger::default()
        },
        &BenchmarkGateReport {
            passed: true,
            failures: Vec::new(),
        },
    )
    .with_kv_fusion_policy_observation_preview_report(&kv_preview);
    let report = SelfEvolutionAdmissionGate::new().evaluate(&evidence);

    assert!(!report.admitted_for_human_review);
    assert!(!report.adaptive_preview_read_only);
    assert!(report.adaptive_preview_write_allowed);

    let line = report.json_line();
    let failures = evaluate_trace_schema_line(&line);

    assert!(line.contains("\"read_only\":false"));
    assert!(line.contains("\"write_allowed\":true"));
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn self_evolution_admission_trace_schema_rejects_write_and_block_mismatch() {
    let line = admitted_self_evolution_admission_line()
        .replacen(
            "\"memory_store_allowed\":false",
            "\"memory_store_allowed\":true",
            1,
        )
        .replacen(
            "\"admitted_for_human_review\":true",
            "\"admitted_for_human_review\":false",
            1,
        );
    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("memory_store_allowed=true")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("requires blocked reasons")),
        "{failures:?}"
    );
}

#[test]
fn self_evolution_admission_trace_schema_rejects_admitted_unsafe_adaptive_preview() {
    let line = replace_in_trace_object(
        &admitted_self_evolution_admission_line(),
        "adaptive_preview",
        "\"write_allowed\":false",
        "\"write_allowed\":true",
    );
    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("requires adaptive_preview write_allowed=false")),
        "{failures:?}"
    );
}

#[test]
fn self_evolution_admission_trace_schema_rejects_missing_review_packet_refs() {
    let line = admitted_self_evolution_admission_line()
        .replacen(
            "\"approval_review_packet_ids\":[\"approval-review:trace-admission\"]",
            "\"approval_review_packet_ids\":[]",
            1,
        )
        .replacen(
            "\"evidence_ids\":[\"rust-check:trace-admission:items-1:passed-1:failed-0\",\"benchmark-gate:trace-admission:passed-true:failures-0\",\"validation:trace-admission:compiler-1/1:0:tests-1/1:0:benchmarks-1/1:0:experiments-1/1:0\",\"adaptive-preview:router-threshold:trace-admission:ready-true\"]",
            "\"evidence_ids\":[]",
            1,
        );
    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("requires review packet ids")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("requires review evidence ids")),
        "{failures:?}"
    );
}
