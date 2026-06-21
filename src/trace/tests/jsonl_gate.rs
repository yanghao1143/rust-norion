use super::*;

#[test]
fn trace_schema_jsonl_gate_checks_non_empty_records() {
    let path = temp_path("trace-schema");
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace schema jsonl", TaskProfile::General),
        &mut backend,
    );
    fs::write(
        &path,
        format!(
            "\n{}\n",
            trace_json_line("trace schema jsonl", TaskProfile::General, 8, &outcome)
        ),
    )
    .unwrap();

    let report = evaluate_trace_schema_jsonl(&path).unwrap();

    assert!(report.passed, "{:?}", report.failures);
    assert_eq!(report.checked_lines, 1);
    assert_eq!(report.rust_check_events, 0);
    assert_eq!(report.rust_check_feedback_applied, 0);
    assert_eq!(report.runtime_error_events, 0);
    assert_eq!(report.runtime_timeout_events, 0);
    assert!(report.summary_line().contains("passed=true"));
    assert!(report.summary_line().contains("rust_check_events=0"));
    assert!(report.summary_line().contains("runtime_error_events=0"));
    assert!(report.summary_line().contains("runtime_timeout_events=0"));
    cleanup(path);
}

#[test]
fn trace_schema_jsonl_gate_aggregates_memory_admission_and_kv_fusion() {
    let path = temp_path("trace-schema-memory-admission");
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new(
            "trace schema jsonl memory admission for a Rust runtime adapter tool",
            TaskProfile::Coding,
        ),
        &mut backend,
    );
    let line = trace_json_line(
        "trace schema jsonl memory admission for a Rust runtime adapter tool",
        TaskProfile::Coding,
        8,
        &outcome,
    );
    fs::write(&path, format!("{line}\n")).unwrap();

    let report = evaluate_trace_schema_jsonl(&path).unwrap();
    let admission = json_object_after_field(&line, "memory_admission").unwrap();
    let fusion = json_object_after_field(&line, "kv_fusion").unwrap();

    assert!(report.passed, "{:?}", report.failures);
    assert_eq!(report.checked_lines, 1);
    assert_eq!(report.memory_admission_events, 1);
    assert_eq!(
        report.memory_admission_candidates,
        extract_json_usize_field(admission, "candidates").unwrap()
    );
    assert!(report.memory_admission_candidates >= 1);
    assert_eq!(
        report.memory_admission_review_packets,
        report.memory_admission_candidates
    );
    assert_eq!(
        report.memory_admission_ledger_records,
        report.memory_admission_candidates
    );
    assert_eq!(report.memory_admission_ledger_authorized, 0);
    assert_eq!(report.memory_admission_ledger_applied, 0);
    assert_eq!(report.memory_admission_admitted, 0);
    assert_eq!(report.kv_fusion_events, 1);
    assert_eq!(
        report.kv_fusion_candidates,
        extract_json_usize_field(fusion, "candidates").unwrap()
    );
    assert!(report.kv_fusion_candidates >= 1);
    assert_eq!(
        report
            .kv_fusion_retained_tokens
            .saturating_add(report.kv_fusion_saved_tokens),
        report.kv_fusion_input_tokens
    );
    assert!(
        report
            .summary_line()
            .contains("memory_admission_ledger_records=")
    );
    assert!(report.summary_line().contains("kv_fusion_saved_tokens="));
    cleanup(path);
}

#[test]
fn trace_schema_jsonl_gate_aggregates_adaptive_routing_and_task_hierarchy() {
    let path = temp_path("trace-schema-adaptive-routing");
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let first_outcome = engine.infer(
        InferenceRequest::new(
            "trace schema jsonl adaptive routing for Rust code memory",
            TaskProfile::Coding,
        ),
        &mut backend,
    );
    let second_outcome = engine.infer(
        InferenceRequest::new(
            "trace schema jsonl adaptive routing for long document context",
            TaskProfile::LongDocument,
        ),
        &mut backend,
    );
    let first_line = trace_json_line(
        "trace schema jsonl adaptive routing for Rust code memory",
        TaskProfile::Coding,
        8,
        &first_outcome,
    );
    let second_line = trace_json_line(
        "trace schema jsonl adaptive routing for long document context",
        TaskProfile::LongDocument,
        13,
        &second_outcome,
    );
    fs::write(&path, format!("{first_line}\n{second_line}\n")).unwrap();

    let report = evaluate_trace_schema_jsonl(&path).unwrap();
    let first_routing = json_object_after_field(&first_line, "adaptive_routing").unwrap();
    let second_routing = json_object_after_field(&second_line, "adaptive_routing").unwrap();
    let first_task = json_object_after_field(&first_line, "task_hierarchy").unwrap();
    let second_task = json_object_after_field(&second_line, "task_hierarchy").unwrap();

    let expected_candidates = extract_json_usize_field(first_routing, "candidates")
        .unwrap()
        .saturating_add(extract_json_usize_field(second_routing, "candidates").unwrap());
    let expected_include = extract_json_usize_field(first_routing, "include")
        .unwrap()
        .saturating_add(extract_json_usize_field(second_routing, "include").unwrap());
    let expected_compress = extract_json_usize_field(first_routing, "compress")
        .unwrap()
        .saturating_add(extract_json_usize_field(second_routing, "compress").unwrap());
    let expected_defer = extract_json_usize_field(first_routing, "defer")
        .unwrap()
        .saturating_add(extract_json_usize_field(second_routing, "defer").unwrap());
    let expected_skip = extract_json_usize_field(first_routing, "skip")
        .unwrap()
        .saturating_add(extract_json_usize_field(second_routing, "skip").unwrap());
    let expected_input_tokens = extract_json_usize_field(first_routing, "input_tokens")
        .unwrap()
        .saturating_add(extract_json_usize_field(second_routing, "input_tokens").unwrap());
    let expected_retained_tokens = extract_json_usize_field(first_routing, "retained_tokens")
        .unwrap()
        .saturating_add(extract_json_usize_field(second_routing, "retained_tokens").unwrap());
    let expected_saved_tokens = extract_json_usize_field(first_routing, "saved_tokens")
        .unwrap()
        .saturating_add(extract_json_usize_field(second_routing, "saved_tokens").unwrap());
    let expected_mutation_records = extract_json_usize_field(first_task, "mutation_records")
        .unwrap()
        .saturating_add(extract_json_usize_field(second_task, "mutation_records").unwrap());
    let expected_route_pressure_milli =
        trace_milli(extract_json_f32_field(first_task, "route_pressure").unwrap()).saturating_add(
            trace_milli(extract_json_f32_field(second_task, "route_pressure").unwrap()),
        );
    let expected_compute_reduction_milli =
        trace_milli(extract_json_f32_field(first_task, "compute_reduction").unwrap())
            .saturating_add(trace_milli(
                extract_json_f32_field(second_task, "compute_reduction").unwrap(),
            ));

    assert!(report.passed, "{:?}", report.failures);
    assert_eq!(report.checked_lines, 2);
    assert_eq!(report.adaptive_routing_events, 2);
    assert_eq!(report.adaptive_routing_candidates, expected_candidates);
    assert!(report.adaptive_routing_candidates >= 2);
    assert_eq!(report.adaptive_routing_include, expected_include);
    assert_eq!(report.adaptive_routing_compress, expected_compress);
    assert_eq!(report.adaptive_routing_defer, expected_defer);
    assert_eq!(report.adaptive_routing_skip, expected_skip);
    assert_eq!(
        report.adaptive_routing_include
            + report.adaptive_routing_compress
            + report.adaptive_routing_defer
            + report.adaptive_routing_skip,
        report.adaptive_routing_candidates
    );
    assert_eq!(report.adaptive_routing_input_tokens, expected_input_tokens);
    assert_eq!(
        report.adaptive_routing_retained_tokens,
        expected_retained_tokens
    );
    assert_eq!(report.adaptive_routing_saved_tokens, expected_saved_tokens);
    assert_eq!(
        report
            .adaptive_routing_retained_tokens
            .saturating_add(report.adaptive_routing_saved_tokens),
        report.adaptive_routing_input_tokens
    );
    assert_eq!(report.task_hierarchy_events, 2);
    assert_eq!(
        report.task_hierarchy_mutation_records,
        expected_mutation_records
    );
    assert!(report.task_hierarchy_mutation_records >= 2);
    assert_eq!(
        report.task_hierarchy_route_pressure_milli,
        expected_route_pressure_milli
    );
    assert_eq!(
        report.task_hierarchy_compute_reduction_milli,
        expected_compute_reduction_milli
    );
    assert!(
        report
            .summary_line()
            .contains("adaptive_routing_candidates=")
    );
    assert!(
        report
            .summary_line()
            .contains("task_hierarchy_mutation_records=")
    );
    cleanup(path);
}

#[test]
fn trace_schema_jsonl_gate_aggregates_self_evolution_experiments() {
    let path = temp_path("trace-schema-self-evolution-experiments");
    let mut ledger = SelfEvolutionExperimentLedger::new();
    let admitted = ledger.append_admission_report(
        "experiment-pass",
        &self_evolution_experiment_passing_report("candidate-pass"),
    );
    let held = ledger.append_admission_report(
        "experiment-hold",
        &self_evolution_experiment_hold_report("candidate-hold"),
    );
    let rejected = ledger.append_admission_report(
        "experiment-reject",
        &self_evolution_experiment_reject_report("candidate-reject"),
    );
    let rollback = ledger.append_admission_report(
        "experiment-pass",
        &self_evolution_experiment_rollback_report("candidate-rollback"),
    );

    append_self_evolution_experiment_trace_jsonl(&path, &admitted).unwrap();
    append_self_evolution_experiment_trace_jsonl(&path, &held).unwrap();
    append_self_evolution_experiment_trace_jsonl(&path, &rejected).unwrap();
    append_self_evolution_experiment_trace_jsonl(&path, &rollback).unwrap();

    let report = evaluate_trace_schema_jsonl(&path).unwrap();

    assert!(report.passed, "{:?}", report.failures);
    assert_eq!(report.checked_lines, 4);
    assert_eq!(report.self_evolution_experiment_events, 4);
    assert_eq!(report.self_evolution_experiment_admit, 1);
    assert_eq!(report.self_evolution_experiment_hold, 1);
    assert_eq!(report.self_evolution_experiment_reject, 1);
    assert_eq!(report.self_evolution_experiment_rollback, 1);
    assert_eq!(report.self_evolution_experiment_repeated, 1);
    assert_eq!(report.self_evolution_experiment_conflicts, 0);
    assert_eq!(report.self_evolution_experiment_rollback_replayable, 1);
    assert_eq!(report.self_evolution_experiment_active_candidates, 0);
    assert_eq!(report.self_evolution_experiment_write_allowed, 0);
    assert_eq!(report.self_evolution_experiment_applied, 0);
    assert!(
        report
            .summary_line()
            .contains("self_evolution_experiment_events=4")
    );
    assert!(
        report
            .summary_line()
            .contains("self_evolution_experiment_rollback=1")
    );
    cleanup(path);
}

fn self_evolution_experiment_passing_report(candidate_id: &str) -> SelfEvolutionAdmissionReport {
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
        candidate_id,
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
        SelfEvolutionValidationLane::new(2, 2, 0),
        SelfEvolutionValidationLane::new(1, 1, 0),
        SelfEvolutionValidationLane::new(1, 1, 0),
    ))
    .with_router_threshold_preview_report(&router_preview);

    SelfEvolutionAdmissionGate::new().evaluate(&evidence)
}

fn self_evolution_experiment_hold_report(candidate_id: &str) -> SelfEvolutionAdmissionReport {
    let evidence = SelfEvolutionAdmissionEvidence::from_benchmark_gate(
        candidate_id,
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
        SelfEvolutionValidationLane::new(2, 2, 0),
        SelfEvolutionValidationLane::new(1, 1, 0),
        SelfEvolutionValidationLane::new(1, 1, 0),
    ));

    SelfEvolutionAdmissionGate::new().evaluate(&evidence)
}

fn self_evolution_experiment_reject_report(candidate_id: &str) -> SelfEvolutionAdmissionReport {
    let evidence = SelfEvolutionAdmissionEvidence::from_benchmark_gate(
        candidate_id,
        EvolutionLedger {
            replay_rust_check_items: 1,
            replay_rust_check_passed: 0,
            replay_rust_check_failed: 1,
            ..EvolutionLedger::default()
        },
        &BenchmarkGateReport {
            passed: false,
            failures: vec!["trace experiment failed benchmark gate".to_owned()],
        },
    )
    .with_validation_evidence(SelfEvolutionValidationEvidence::from_lanes(
        SelfEvolutionValidationLane::new(1, 0, 1),
        SelfEvolutionValidationLane::new(1, 0, 1),
        SelfEvolutionValidationLane::new(1, 0, 1),
        SelfEvolutionValidationLane::new(1, 0, 1),
    ));

    SelfEvolutionAdmissionGate::new().evaluate(&evidence)
}

fn self_evolution_experiment_rollback_report(candidate_id: &str) -> SelfEvolutionAdmissionReport {
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
        candidate_id,
        EvolutionLedger {
            replay_rust_check_items: 2,
            replay_rust_check_passed: 2,
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
        SelfEvolutionValidationLane::new(2, 2, 0),
        SelfEvolutionValidationLane::new(2, 2, 0),
        SelfEvolutionValidationLane::new(1, 1, 0),
        SelfEvolutionValidationLane::new(1, 1, 0),
    ))
    .with_router_threshold_preview_report(&router_preview);

    SelfEvolutionAdmissionGate::new().evaluate(&evidence)
}

fn trace_milli(value: f32) -> usize {
    if value.is_finite() {
        (value.clamp(0.0, 1.0) * 1000.0).round() as usize
    } else {
        0
    }
}
