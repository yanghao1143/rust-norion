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
